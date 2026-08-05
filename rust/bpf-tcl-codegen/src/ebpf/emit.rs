// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! BPF-IR → eBPF bytecode. A deliberately simple v1 codegen: every value lives
//! in its own stack slot, and each instruction loads its operands into scratch
//! registers, computes, and stores back. No register allocation, no calls
//! except map helpers.
//!
//! Three execution ABIs share this backend (see [`TargetAbi`]):
//!
//! - `RbpfFixedMbuff` — the userspace [`rbpf`](https://docs.rs/rbpf) simulator.
//!   `r1` is a metadata buffer whose first two 64-bit words are the packet
//!   `data` and `data_end` pointers (offsets 0 and 8). Map helpers pass the map
//!   index and integer key/value by value.
//! - `KernelXdp` — Linux `BPF_PROG_TYPE_XDP`. `r1` is `struct xdp_md`; `data`
//!   and `data_end` are 32-bit context fields the verifier rewrites into a
//!   packet pointer / packet-end pointer.
//! - `KernelSocketFilter` — Linux `BPF_PROG_TYPE_SOCKET_FILTER`. `r1` is
//!   `struct __sk_buff`; direct packet access reads `data` / `data_end` at their
//!   `__sk_buff` offsets.
//!
//! Both kernel targets emit real map ABIs: a `ld_imm64` pseudo map-fd load
//! (relocated by libbpf against the map symbol), a stack-resident key/value, and
//! `bpf_map_lookup_elem` / `bpf_map_update_elem` helper calls. Both emit a
//! dominating `data + off + width <= data_end` proof before every packet
//! dereference, so the kernel verifier accepts the load.

use std::collections::HashMap;

use bpf_tcl_ir::ir::{
    Block, BpfProgram, CmpOp, Inst, IntBinOp, MapDef, OobAction, ProgType, Term, UnOp,
};
use bpf_tcl_ir::ty::{ByteOrder, Width};
use bpf_tcl_ir::{BpfDiag, BpfError};
use tcl_lexer::Span;

use crate::ebpf::insn::{
    ADD, AND, ARSH, DIV, FN_MAP_LOOKUP, FN_MAP_UPDATE, Insn, JEQ, JLE, JNE, JSGE, JSGT, JSLE, JSLT,
    LSH, MOD, MUL, OR, PSEUDO_MAP_FD, R0, R1, R2, R3, R4, R6, R7, R10, SUB, SZ_B, SZ_DW, SZ_H,
    SZ_W, XOR, alu64_imm, alu64_reg, alu64_reg_off, bswap_be, bswap_le, call, exit, ja, jmp_imm,
    jmp_reg, lddw, lddw_src, ldx, mov64_imm, mov64_reg, neg64, st_imm, stx,
};

/// Packet `data` pointer (callee-saved). Under rbpf it is a 64-bit metadata
/// word; under a kernel target it is the verifier's `PTR_TO_PACKET`.
const RPTR: u8 = R6;
/// Packet `data_end` pointer (callee-saved).
const REND: u8 = R7;

/// Helper id for the rbpf simulator's by-value `map_get`.
const RBPF_MAP_GET_ID: i32 = 1;
/// Helper id for the rbpf simulator's by-value `map_set`.
const RBPF_MAP_SET_ID: i32 = 2;
/// Helper id for the rbpf simulator's by-value `map_has`.
const RBPF_MAP_HAS_ID: i32 = 3;

/// The maximum eBPF stack, in bytes (`MAX_BPF_STACK`).
const MAX_STACK: i32 = 512;

/// The execution ABI an emitted instruction stream targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetAbi {
    /// The userspace `rbpf::EbpfVmFixedMbuff` metadata-buffer ABI.
    RbpfFixedMbuff,
    /// Linux `BPF_PROG_TYPE_XDP` (`struct xdp_md` context).
    KernelXdp,
    /// Linux `BPF_PROG_TYPE_SOCKET_FILTER` (`struct __sk_buff` context).
    KernelSocketFilter,
}

impl TargetAbi {
    /// Stable CLI/display spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RbpfFixedMbuff => "rbpf",
            Self::KernelXdp => "kernel-xdp",
            Self::KernelSocketFilter => "kernel-socket",
        }
    }

    /// Whether this target emits a Linux-kernel-loadable ABI (kernel context
    /// fields, verifier proofs, pointer-based map helpers).
    #[must_use]
    pub const fn is_kernel(self) -> bool {
        !matches!(self, Self::RbpfFixedMbuff)
    }

    /// The context layout: `data` / `data_end` field offsets and the load width
    /// used to read them.
    const fn ctx_layout(self) -> CtxLayout {
        match self {
            // The rbpf metadata buffer holds two 64-bit pointers.
            Self::RbpfFixedMbuff => CtxLayout {
                data_off: 0,
                data_end_off: 8,
                load_size: SZ_DW,
            },
            // `struct xdp_md { __u32 data; __u32 data_end; … }`.
            Self::KernelXdp => CtxLayout {
                data_off: 0,
                data_end_off: 4,
                load_size: SZ_W,
            },
            // `struct __sk_buff`: `data` at byte 76, `data_end` at byte 80.
            Self::KernelSocketFilter => CtxLayout {
                data_off: 76,
                data_end_off: 80,
                load_size: SZ_W,
            },
        }
    }
}

/// The `data` / `data_end` context field offsets and the width used to read them.
struct CtxLayout {
    data_off: i16,
    data_end_off: i16,
    load_size: u8,
}

/// A relocation the ELF writer must emit: at instruction `insn_index`, a pseudo
/// `ld_imm64` needs its immediate patched to map `map_index`'s file descriptor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapReloc {
    /// Instruction index of the `ld_imm64` (its byte offset is `index * 8`).
    pub insn_index: u32,
    /// The declared map this instruction references.
    pub map_index: u32,
}

/// A compiled eBPF object: the instruction array plus its raw little-endian
/// byte encoding (exactly what `rbpf` executes or a loader submits).
#[derive(Debug, Clone)]
pub struct EbpfObject {
    /// The context/helper ABI this instruction stream targets.
    pub target_abi: TargetAbi,
    /// The program type (so the object is self-describing for a loader).
    pub prog_type: ProgType,
    /// The decoded instructions.
    pub insns: Vec<Insn>,
    /// The flattened little-endian bytes (`insns.len() * 8`).
    pub raw: Vec<u8>,
    /// Declared maps (so the run harness can size its map store and the ELF
    /// writer can emit their definitions).
    pub maps: Vec<MapDef>,
    /// Map file-descriptor relocations (kernel targets only).
    pub relocations: Vec<MapReloc>,
}

impl EbpfObject {
    fn assemble(
        target_abi: TargetAbi,
        prog_type: ProgType,
        insns: Vec<Insn>,
        maps: Vec<MapDef>,
        relocations: Vec<MapReloc>,
    ) -> Self {
        let mut raw = Vec::with_capacity(insns.len() * 8);
        for i in &insns {
            raw.extend_from_slice(&i.to_le_bytes());
        }
        Self {
            target_abi,
            prog_type,
            insns,
            raw,
            maps,
            relocations,
        }
    }
}

/// A not-yet-laid-out instruction. Block jumps carry a target block id resolved
/// to a relative offset in a second pass. A `Pending` may expand to more than
/// one instruction (`Wide` is two), so pending-index and instruction-index are
/// distinct — a prefix sum maps between them.
enum Pending {
    Ins(Insn),
    /// A two-instruction wide-immediate load (`lddw`).
    Wide([Insn; 2]),
    /// A pseudo map-fd load into `r1` (two instructions), recorded as a
    /// relocation against `map`.
    MapFd {
        map: u32,
    },
    /// `ja <block>`
    Ja(u32),
    /// `if reg != 0 goto <block>`
    JmpNeZero {
        reg: u8,
        target: u32,
    },
}

impl Pending {
    /// How many machine instructions this pending item expands to.
    fn len(&self) -> usize {
        match self {
            Pending::Wide(_) | Pending::MapFd { .. } => 2,
            _ => 1,
        }
    }
}

/// Stack offset of a slot: slot `i` lives at `[r10 - 8*(i+1)]`.
fn slot_off(slot: u32) -> i16 {
    // Slots are capped at 64, so `(slot+1)*8 <= 520` fits in `i16`.
    -i16::try_from((slot + 1) * 8).unwrap_or(i16::MAX)
}

/// The emit-time context threaded through the per-instruction lowering: the
/// target ABI plus the layout facts a kernel map helper needs (where the
/// scratch key/value cells live above the slot region).
#[derive(Clone, Copy)]
struct EmitCtx {
    target: TargetAbi,
    /// Stack offset of the scratch cell holding a map key.
    map_key_off: i16,
    /// Stack offset of the scratch cell holding a map value.
    map_value_off: i16,
}

impl EmitCtx {
    fn new(target: TargetAbi, num_slots: u32) -> Self {
        // The slot region occupies `[r10 - 8*num_slots .. r10]`. The map
        // key/value scratch cells sit just above it.
        let base = -(i32::try_from(num_slots).unwrap_or(0) * 8);
        let map_key_off = i16::try_from(base - 8).unwrap_or(i16::MIN);
        let map_value_off = i16::try_from(base - 16).unwrap_or(i16::MIN);
        Self {
            target,
            map_key_off,
            map_value_off,
        }
    }
}

/// Emit an eBPF object for a typed program (the rbpf simulator ABI).
///
/// # Errors
/// Returns a [`BpfError`] for v1 codegen limits (constant out of 32-bit range,
/// load offset out of 16-bit range, program too large for a 16-bit jump).
pub fn emit_program(prog: &BpfProgram) -> Result<EbpfObject, BpfError> {
    emit_program_for_target(prog, TargetAbi::RbpfFixedMbuff)
}

/// Emit an eBPF object for a typed program and explicit execution ABI.
///
/// # Errors
/// Returns a [`BpfError`] for unsupported target/program combinations and the
/// same codegen limits as [`emit_program`].
pub fn emit_program_for_target(
    prog: &BpfProgram,
    target_abi: TargetAbi,
) -> Result<EbpfObject, BpfError> {
    validate_target(prog, target_abi)?;
    let ctx = EmitCtx::new(target_abi, prog.num_slots);
    let mut pend: Vec<Pending> = Vec::new();

    emit_prologue(prog, target_abi, &mut pend);

    // Zero only the slots that may be read before being written (the
    // liveness-based zero-init set), not every slot — blanket zeroing wastes
    // instructions and verifier state. `assign_slots` computed this set.
    for slot in &prog.zero_init {
        pend.push(Pending::Ins(st_imm(SZ_DW, R10, slot_off(slot.0), 0)));
    }

    // Lay out the entry block first, then the rest in id order.
    let mut order: Vec<&Block> = Vec::with_capacity(prog.blocks.len());
    if let Some(entry) = prog.blocks.iter().find(|b| b.id == prog.entry) {
        order.push(entry);
    }
    for b in &prog.blocks {
        if b.id != prog.entry {
            order.push(b);
        }
    }

    // `block_start` keys are *pending* indices; converted to instruction
    // indices via the prefix sum below.
    let mut block_start: HashMap<u32, usize> = HashMap::new();
    for block in &order {
        block_start.insert(block.id.0, pend.len());
        for inst in &block.insts {
            emit_inst(inst, ctx, &mut pend)?;
        }
        emit_term(&block.term, &mut pend);
    }

    // Prefix sum: instruction index at which each pending item begins.
    let mut insn_index_of: Vec<usize> = Vec::with_capacity(pend.len() + 1);
    let mut acc = 0usize;
    for p in &pend {
        insn_index_of.push(acc);
        acc += p.len();
    }
    insn_index_of.push(acc);
    let block_insn_start = |pend_idx: usize| -> usize { insn_index_of[pend_idx] };

    // Resolve block jumps to relative offsets and record map-fd relocations.
    let mut insns = Vec::with_capacity(acc);
    let mut relocations = Vec::new();
    for (pend_idx, p) in pend.iter().enumerate() {
        let here = insn_index_of[pend_idx];
        match p {
            Pending::Ins(i) => insns.push(*i),
            Pending::Wide(pair) => insns.extend_from_slice(pair),
            Pending::MapFd { map } => {
                relocations.push(MapReloc {
                    insn_index: u32::try_from(here).unwrap_or(u32::MAX),
                    map_index: *map,
                });
                insns.extend_from_slice(&lddw_src(R1, PSEUDO_MAP_FD, 0));
            }
            Pending::Ja(target) => {
                insns.push(ja(rel_off(here, block_insn_start(block_start[target]))?));
            }
            Pending::JmpNeZero { reg, target } => insns.push(jmp_imm(
                JNE,
                *reg,
                0,
                rel_off(here, block_insn_start(block_start[target]))?,
            )),
        }
    }
    Ok(EbpfObject::assemble(
        target_abi,
        prog.prog_type,
        insns,
        prog.maps.clone(),
        relocations,
    ))
}

/// Emit the context prologue that leaves `data` in `r6` and `data_end` in `r7`.
///
/// A verdict-only kernel program (no packet or context access) needs no
/// prologue at all; the rbpf simulator always loads the metadata buffer.
fn emit_prologue(prog: &BpfProgram, target: TargetAbi, pend: &mut Vec<Pending>) {
    let needs_ctx = target == TargetAbi::RbpfFixedMbuff || program_uses_context(prog);
    if !needs_ctx {
        return;
    }
    let layout = target.ctx_layout();
    pend.push(Pending::Ins(ldx(
        layout.load_size,
        RPTR,
        R1,
        layout.data_off,
    )));
    pend.push(Pending::Ins(ldx(
        layout.load_size,
        REND,
        R1,
        layout.data_end_off,
    )));
}

/// Whether any instruction reads the packet context (so the prologue is needed).
fn program_uses_context(prog: &BpfProgram) -> bool {
    prog.blocks.iter().flat_map(|b| &b.insts).any(|inst| {
        matches!(
            inst,
            Inst::CtxPtr { .. } | Inst::CtxLen { .. } | Inst::Load { .. }
        )
    })
}

fn validate_target(prog: &BpfProgram, target_abi: TargetAbi) -> Result<(), BpfError> {
    if !target_abi.is_kernel() {
        return Ok(());
    }
    let expected = match target_abi {
        TargetAbi::KernelXdp => ProgType::Xdp,
        TargetAbi::KernelSocketFilter => ProgType::SocketFilter,
        TargetAbi::RbpfFixedMbuff => return Ok(()),
    };
    if prog.prog_type != expected {
        return Err(BpfError::new(
            BpfDiag::OutOfSubset,
            Span::empty(0),
            format!(
                "the `{}` target only accepts `{}` programs",
                target_abi.as_str(),
                match expected {
                    ProgType::Xdp => "when XDP",
                    ProgType::SocketFilter => "when SOCKET_FILTER",
                }
            ),
        ));
    }
    // The kernel map helpers spill the key and value into two scratch cells
    // above the slot region; guard the 512-byte stack.
    if !prog.maps.is_empty() {
        let used = i32::try_from(prog.num_slots).unwrap_or(i32::MAX) * 8 + 16;
        if used > MAX_STACK {
            return Err(BpfError::new(
                BpfDiag::OutOfSubset,
                prog.maps[0].span,
                "program uses too much stack for kernel map key/value scratch (exceeds 512 bytes)",
            ));
        }
    }
    Ok(())
}

fn rel_off(from: usize, to: usize) -> Result<i16, BpfError> {
    // eBPF jump offsets are relative to the *next* instruction.
    let delta = isize::try_from(to).unwrap_or(0) - isize::try_from(from).unwrap_or(0) - 1;
    i16::try_from(delta).map_err(|_| {
        BpfError::new(
            BpfDiag::Internal,
            Span::empty(0),
            "program too large: jump target exceeds a 16-bit offset",
        )
    })
}

fn emit_inst(inst: &Inst, ctx: EmitCtx, pend: &mut Vec<Pending>) -> Result<(), BpfError> {
    match inst {
        Inst::Const { dst, val, .. } => {
            // A value fitting a sign-extended 32-bit immediate uses the single
            // `mov`; a full 64-bit constant uses the two-instruction `lddw`
            // (`RUST_ISSUE_172`: large constants must materialise correctly).
            if let Ok(imm) = i32::try_from(*val) {
                pend.push(Pending::Ins(mov64_imm(R1, imm)));
            } else {
                pend.push(Pending::Wide(lddw(R1, *val)));
            }
            pend.push(Pending::Ins(stx(SZ_DW, R10, R1, slot_off(dst.0))));
        }
        Inst::Copy { dst, src, .. } => {
            pend.push(Pending::Ins(ldx(SZ_DW, R1, R10, slot_off(src.0))));
            pend.push(Pending::Ins(stx(SZ_DW, R10, R1, slot_off(dst.0))));
        }
        Inst::Bin { dst, op, a, b, .. } => {
            pend.push(Pending::Ins(ldx(SZ_DW, R1, R10, slot_off(a.0))));
            pend.push(Pending::Ins(ldx(SZ_DW, R2, R10, slot_off(b.0))));
            let (alu_op, off) = bin_alu(*op);
            pend.push(Pending::Ins(alu64_reg_off(alu_op, R1, R2, off)));
            pend.push(Pending::Ins(stx(SZ_DW, R10, R1, slot_off(dst.0))));
        }
        Inst::Un { dst, op, a, .. } => {
            pend.push(Pending::Ins(ldx(SZ_DW, R1, R10, slot_off(a.0))));
            match op {
                UnOp::Neg => pend.push(Pending::Ins(neg64(R1))),
                UnOp::BitNot => pend.push(Pending::Ins(alu64_imm(XOR, R1, -1))),
                UnOp::Not => {
                    // r1 = (r1 == 0) ? 1 : 0
                    pend.push(Pending::Ins(mov64_imm(R3, 1)));
                    pend.push(Pending::Ins(jmp_imm(JEQ, R1, 0, 1)));
                    pend.push(Pending::Ins(mov64_imm(R3, 0)));
                    pend.push(Pending::Ins(mov64_reg(R1, R3)));
                }
            }
            pend.push(Pending::Ins(stx(SZ_DW, R10, R1, slot_off(dst.0))));
        }
        Inst::Cmp { dst, op, a, b, .. } => {
            // r3 = (a <op> b) ? 1 : 0
            pend.push(Pending::Ins(ldx(SZ_DW, R1, R10, slot_off(a.0))));
            pend.push(Pending::Ins(ldx(SZ_DW, R2, R10, slot_off(b.0))));
            pend.push(Pending::Ins(mov64_imm(R3, 1)));
            pend.push(Pending::Ins(jmp_reg(cmp_jop(*op), R1, R2, 1)));
            pend.push(Pending::Ins(mov64_imm(R3, 0)));
            pend.push(Pending::Ins(stx(SZ_DW, R10, R3, slot_off(dst.0))));
        }
        Inst::CtxPtr { dst, .. } => {
            pend.push(Pending::Ins(stx(SZ_DW, R10, RPTR, slot_off(dst.0))));
        }
        Inst::CtxLen { dst, .. } => {
            // length = data_end - data
            pend.push(Pending::Ins(mov64_reg(R1, REND)));
            pend.push(Pending::Ins(alu64_reg(SUB, R1, RPTR)));
            pend.push(Pending::Ins(stx(SZ_DW, R10, R1, slot_off(dst.0))));
        }
        load @ Inst::Load { .. } => emit_load(load, pend)?,
        Inst::MapGet { dst, map, key, .. } => emit_map_get(ctx, *dst, *map, *key, pend),
        Inst::MapHas { dst, map, key, .. } => emit_map_has(ctx, *dst, *map, *key, pend),
        Inst::MapSet { map, key, val, .. } => emit_map_set(ctx, *map, *key, *val, pend),
    }
    Ok(())
}

/// Emit a checked packet load honouring its byte order and out-of-bounds
/// action. `r6` holds the packet `data` pointer and `r7` holds `data_end`, so
/// the load first proves the field lies inside the packet:
///
/// ```text
///   r2 = r6                          ; the packet pointer
///   r2 += off + width                ; one past the field
///   if r2 <= r7 goto +2              ; in bounds → skip the OOB return
///   r0 = <oob verdict>; exit         ; short packet → take the OOB action
///   r2 = *(width *)(r6 + off)         ; the load itself (r6 still `data`)
///   r2 = bswap(r2)                    ; byte-order conversion (multi-byte only)
///   store r2
/// ```
///
/// This is the verifier-safe shape: the compared register (`r2 = r6 + C`) and
/// the load base (`r6`) share a packet-pointer id, so proving `r6 + C <=
/// data_end` teaches the verifier that `r6 + off` (with `off < C`) is in bounds.
fn emit_load(load: &Inst, pend: &mut Vec<Pending>) -> Result<(), BpfError> {
    let Inst::Load {
        dst,
        width,
        ptr: _,
        off,
        order,
        oob,
        span,
    } = load
    else {
        return Err(BpfError::new(
            BpfDiag::Internal,
            Span::empty(0),
            "emit_load called with a non-load instruction",
        ));
    };
    let (dst, width, off, order, oob, span) = (*dst, *width, *off, *order, *oob, *span);
    let off16 = i16::try_from(off).map_err(|_| {
        BpfError::new(
            BpfDiag::BadInt,
            span,
            "load offset out of range (must fit in 16 bits)",
        )
    })?;
    let field_end = i32::try_from(width.bytes()).unwrap_or(0) + off;
    let OobAction::ReturnVerdict(verdict) = oob;
    let vimm = i32::try_from(verdict).unwrap_or(0);

    // Bounds proof: r2 = data + field_end; if r2 <= data_end skip the return.
    pend.push(Pending::Ins(mov64_reg(R2, RPTR)));
    pend.push(Pending::Ins(alu64_imm(ADD, R2, field_end)));
    pend.push(Pending::Ins(jmp_reg(JLE, R2, REND, 2)));
    pend.push(Pending::Ins(mov64_imm(R0, vimm)));
    pend.push(Pending::Ins(exit()));

    // The load itself, from the original packet pointer (still `data`).
    pend.push(Pending::Ins(ldx(width_size(width), R2, RPTR, off16)));
    // Byte-order conversion (a single-byte field needs none).
    if width != Width::B8 {
        let bits = i32::try_from(width.bytes()).unwrap_or(0) * 8;
        match order {
            ByteOrder::Big => pend.push(Pending::Ins(bswap_be(R2, bits))),
            ByteOrder::Little => pend.push(Pending::Ins(bswap_le(R2, bits))),
            ByteOrder::Native => {}
        }
    }
    pend.push(Pending::Ins(stx(SZ_DW, R10, R2, slot_off(dst.0))));
    Ok(())
}

/// A small map index as a 32-bit immediate.
fn map_imm(map: u32) -> i32 {
    i32::try_from(map).unwrap_or(0)
}

/// `dst = map_get(map, key)`.
fn emit_map_get(
    ctx: EmitCtx,
    dst: bpf_tcl_ir::ir::SlotId,
    map: u32,
    key: bpf_tcl_ir::ir::SlotId,
    pend: &mut Vec<Pending>,
) {
    if !ctx.target.is_kernel() {
        // rbpf: r1 = map index, r2 = key by value; r0 = map_get(...); store r0.
        pend.push(Pending::Ins(mov64_imm(R1, map_imm(map))));
        pend.push(Pending::Ins(ldx(SZ_DW, R2, R10, slot_off(key.0))));
        pend.push(Pending::Ins(call(RBPF_MAP_GET_ID)));
        pend.push(Pending::Ins(stx(SZ_DW, R10, R0, slot_off(dst.0))));
        return;
    }
    emit_kernel_lookup(ctx, map, key, pend);
    // r0 = value pointer or NULL. Default the result to 0 (absent), then load
    // the value when present. r1 holds the result so a following store works.
    pend.push(Pending::Ins(mov64_imm(R1, 0)));
    pend.push(Pending::Ins(jmp_imm(JEQ, R0, 0, 1)));
    pend.push(Pending::Ins(ldx(SZ_DW, R1, R0, 0)));
    pend.push(Pending::Ins(stx(SZ_DW, R10, R1, slot_off(dst.0))));
}

/// `dst = map_has(map, key)` — 1 when present, 0 when absent.
fn emit_map_has(
    ctx: EmitCtx,
    dst: bpf_tcl_ir::ir::SlotId,
    map: u32,
    key: bpf_tcl_ir::ir::SlotId,
    pend: &mut Vec<Pending>,
) {
    if !ctx.target.is_kernel() {
        pend.push(Pending::Ins(mov64_imm(R1, map_imm(map))));
        pend.push(Pending::Ins(ldx(SZ_DW, R2, R10, slot_off(key.0))));
        pend.push(Pending::Ins(call(RBPF_MAP_HAS_ID)));
        pend.push(Pending::Ins(stx(SZ_DW, R10, R0, slot_off(dst.0))));
        return;
    }
    emit_kernel_lookup(ctx, map, key, pend);
    // r1 = (r0 != NULL) ? 1 : 0.
    pend.push(Pending::Ins(mov64_imm(R1, 0)));
    pend.push(Pending::Ins(jmp_imm(JEQ, R0, 0, 1)));
    pend.push(Pending::Ins(mov64_imm(R1, 1)));
    pend.push(Pending::Ins(stx(SZ_DW, R10, R1, slot_off(dst.0))));
}

/// `map_set(map, key, val)`.
fn emit_map_set(
    ctx: EmitCtx,
    map: u32,
    key: bpf_tcl_ir::ir::SlotId,
    val: bpf_tcl_ir::ir::SlotId,
    pend: &mut Vec<Pending>,
) {
    if !ctx.target.is_kernel() {
        pend.push(Pending::Ins(mov64_imm(R1, map_imm(map))));
        pend.push(Pending::Ins(ldx(SZ_DW, R2, R10, slot_off(key.0))));
        pend.push(Pending::Ins(ldx(SZ_DW, R3, R10, slot_off(val.0))));
        pend.push(Pending::Ins(call(RBPF_MAP_SET_ID)));
        return;
    }
    // Spill key and value into their stack scratch cells.
    pend.push(Pending::Ins(ldx(SZ_DW, R1, R10, slot_off(key.0))));
    pend.push(Pending::Ins(stx(SZ_DW, R10, R1, ctx.map_key_off)));
    pend.push(Pending::Ins(ldx(SZ_DW, R1, R10, slot_off(val.0))));
    pend.push(Pending::Ins(stx(SZ_DW, R10, R1, ctx.map_value_off)));
    // bpf_map_update_elem(map, &key, &value, BPF_ANY).
    pend.push(Pending::MapFd { map });
    pend.push(Pending::Ins(mov64_reg(R2, R10)));
    pend.push(Pending::Ins(alu64_imm(ADD, R2, i32::from(ctx.map_key_off))));
    pend.push(Pending::Ins(mov64_reg(R3, R10)));
    pend.push(Pending::Ins(alu64_imm(
        ADD,
        R3,
        i32::from(ctx.map_value_off),
    )));
    pend.push(Pending::Ins(mov64_imm(R4, 0)));
    pend.push(Pending::Ins(call(FN_MAP_UPDATE)));
}

/// Emit the shared prefix of a kernel map lookup: spill the key to its scratch
/// cell and call `bpf_map_lookup_elem(map, &key)`, leaving the result in `r0`.
fn emit_kernel_lookup(
    ctx: EmitCtx,
    map: u32,
    key: bpf_tcl_ir::ir::SlotId,
    pend: &mut Vec<Pending>,
) {
    pend.push(Pending::Ins(ldx(SZ_DW, R1, R10, slot_off(key.0))));
    pend.push(Pending::Ins(stx(SZ_DW, R10, R1, ctx.map_key_off)));
    pend.push(Pending::MapFd { map });
    pend.push(Pending::Ins(mov64_reg(R2, R10)));
    pend.push(Pending::Ins(alu64_imm(ADD, R2, i32::from(ctx.map_key_off))));
    pend.push(Pending::Ins(call(FN_MAP_LOOKUP)));
}

fn emit_term(term: &Term, pend: &mut Vec<Pending>) {
    match term {
        Term::Goto { target, .. } => pend.push(Pending::Ja(target.0)),
        Term::BranchNz { cond, t, f, .. } => {
            pend.push(Pending::Ins(ldx(SZ_DW, R1, R10, slot_off(cond.0))));
            pend.push(Pending::JmpNeZero {
                reg: R1,
                target: t.0,
            });
            pend.push(Pending::Ja(f.0));
        }
        Term::Return { verdict, .. } => {
            pend.push(Pending::Ins(ldx(SZ_DW, R0, R10, slot_off(verdict.0))));
            pend.push(Pending::Ins(exit()));
        }
    }
}

/// Map an [`IntBinOp`] to its eBPF ALU opcode plus the instruction `off` field.
///
/// **Integer-semantics contract.** BPF-Tcl integers are signed 64-bit, but
/// `/` and `%` follow eBPF's **signed truncated-toward-zero** division
/// (`BPF_SDIV` / `BPF_SMOD`), *not* Tcl's floor division. The two agree for
/// operands of the same sign and diverge only when exactly one operand is
/// negative: `-7 / 2` is `-3` here (truncation) where Tcl yields `-4` (floor),
/// and `-7 % 2` is `-1` here where Tcl yields `1`. This narrower, verifier-
/// native contract is deliberate and documented (issue #1202); the front-end
/// does not silently pretend to be Tcl. `BPF_SDIV` / `BPF_SMOD` are encoded as
/// `DIV` / `MOD` with `off == 1`; the plain unsigned forms (off 0) reinterpret
/// a negative operand as a huge unsigned value, giving a catastrophically
/// wrong result (`RUST_ISSUE_031`). `>>` lowers to the arithmetic
/// (sign-preserving) `ARSH`, not the logical `RSH`, so `-8 >> 1` is `-4`
/// (`RUST_ISSUE_062` / `097`). Every other op keeps `off == 0`.
fn bin_alu(op: IntBinOp) -> (u8, i16) {
    match op {
        IntBinOp::Add => (ADD, 0),
        IntBinOp::Sub => (SUB, 0),
        IntBinOp::Mul => (MUL, 0),
        IntBinOp::Div => (DIV, 1),
        IntBinOp::Mod => (MOD, 1),
        IntBinOp::And => (AND, 0),
        IntBinOp::Or => (OR, 0),
        IntBinOp::Xor => (XOR, 0),
        IntBinOp::Shl => (LSH, 0),
        IntBinOp::Shr => (ARSH, 0),
    }
}

fn cmp_jop(op: CmpOp) -> u8 {
    match op {
        CmpOp::Eq => JEQ,
        CmpOp::Ne => JNE,
        CmpOp::Lt => JSLT,
        CmpOp::Le => JSLE,
        CmpOp::Gt => JSGT,
        CmpOp::Ge => JSGE,
    }
}

fn width_size(w: Width) -> u8 {
    match w {
        Width::B8 => SZ_B,
        Width::B16 => SZ_H,
        Width::B32 => SZ_W,
    }
}

#[cfg(test)]
mod tests {
    use bpf_tcl_ir::compile_module;

    use super::*;

    fn emit_src(src: &str) -> EbpfObject {
        let module = compile_module(src).expect("compiles");
        emit_program(&module.programs[0].program).expect("emits")
    }

    fn emit_kernel(src: &str, target: TargetAbi) -> EbpfObject {
        let module = compile_module(src).expect("compiles");
        emit_program_for_target(&module.programs[0].program, target).expect("emits for the kernel")
    }

    #[test]
    fn accept_all_ends_in_exit() {
        let obj = emit_src("when SOCKET_FILTER { setbuf pkt ctx\n accept }\n");
        assert_eq!(obj.raw.len(), obj.insns.len() * 8);
        // Last instruction is EXIT (0x95).
        assert_eq!(obj.insns.last().unwrap().op, 0x95);
        // Prologue loads data/data_end from the metadata buffer (r1).
        assert_eq!(obj.insns[0], ldx(SZ_DW, R6, R1, 0)); // r6 = data
        assert_eq!(obj.insns[1], ldx(SZ_DW, R7, R1, 8)); // r7 = data_end
    }

    #[test]
    fn kernel_xdp_verdict_only_has_no_context_prologue() {
        let obj = emit_kernel("when XDP { pass }\n", TargetAbi::KernelXdp);
        assert_eq!(obj.target_abi, TargetAbi::KernelXdp);
        // No prologue and no slot zeroing: `mov r1,2; stx; ldx r0; exit`.
        assert_eq!(obj.insns.len(), 4);
        assert_ne!(obj.insns[0], ldx(SZ_DW, R6, R1, 0));
        assert_eq!(obj.insns.last().unwrap().op, 0x95);
    }

    #[test]
    fn kernel_xdp_reads_xdp_md_context_fields() {
        // `struct xdp_md`: data at byte 0, data_end at byte 4, both 32-bit.
        let obj = emit_kernel(
            "when XDP { setbuf p ctx\n load16 dport p 36\n pass }\n",
            TargetAbi::KernelXdp,
        );
        assert_eq!(obj.insns[0], ldx(SZ_W, R6, R1, 0)); // r6 = data (u32)
        assert_eq!(obj.insns[1], ldx(SZ_W, R7, R1, 4)); // r7 = data_end (u32)
    }

    #[test]
    fn kernel_socket_reads_sk_buff_context_fields() {
        // `struct __sk_buff`: data at byte 76, data_end at byte 80.
        let obj = emit_kernel(
            "when SOCKET_FILTER { setbuf p ctx\n load16 dport p 2\n accept }\n",
            TargetAbi::KernelSocketFilter,
        );
        assert_eq!(obj.insns[0], ldx(SZ_W, R6, R1, 76));
        assert_eq!(obj.insns[1], ldx(SZ_W, R7, R1, 80));
    }

    #[test]
    fn kernel_packet_load_has_dominating_bounds_proof() {
        let obj = emit_kernel(
            "when XDP { setbuf p ctx\n load16 dport p 36\n pass }\n",
            TargetAbi::KernelXdp,
        );
        // The load `*(u16*)(r6 + 36)` (op 0x69) must be preceded by a
        // `if r2 <= r7 goto +N` (JLE reg, op 0xbd) comparing against data_end.
        let load_pos = obj
            .insns
            .iter()
            .position(|i| i.op == (SZ_H | 0x61) && i.src == R6 && i.off == 36)
            .expect("a u16 packet load from r6+36");
        let has_proof = obj.insns[..load_pos]
            .iter()
            .any(|i| i.op == (JLE | 0x0d) && i.src == REND);
        assert!(
            has_proof,
            "load must be dominated by a data_end bounds check"
        );
    }

    #[test]
    fn kernel_map_lookup_emits_pseudo_fd_and_helper_call() {
        let obj = emit_kernel(
            "when XDP {\n map hits array 4 8 4\n setint k 0\n map_get n hits {$k}\n pass }\n",
            TargetAbi::KernelXdp,
        );
        // One relocation for the single map-fd load.
        assert_eq!(obj.relocations.len(), 1);
        assert_eq!(obj.relocations[0].map_index, 0);
        // The relocated instruction is a pseudo ld_imm64 (src = PSEUDO_MAP_FD).
        let reloc_at = obj.relocations[0].insn_index as usize;
        assert_eq!(obj.insns[reloc_at].src, PSEUDO_MAP_FD);
        // A bpf_map_lookup_elem call (helper id 1) is present.
        assert!(
            obj.insns
                .iter()
                .any(|i| i.op == 0x85 && i.imm == FN_MAP_LOOKUP)
        );
    }

    #[test]
    fn kernel_map_update_emits_update_helper() {
        let obj = emit_kernel(
            "when XDP {\n map hits array 4 8 4\n setint k 0\n map_set hits {$k} {7}\n pass }\n",
            TargetAbi::KernelXdp,
        );
        assert!(
            obj.insns
                .iter()
                .any(|i| i.op == 0x85 && i.imm == FN_MAP_UPDATE)
        );
    }

    #[test]
    fn kernel_target_rejects_wrong_program_type() {
        let module = compile_module("when SOCKET_FILTER { accept }\n").expect("compiles");
        let err = emit_program_for_target(&module.programs[0].program, TargetAbi::KernelXdp)
            .expect_err("socket filter is not an XDP program");
        assert_eq!(err.code, BpfDiag::OutOfSubset);
    }

    #[test]
    fn drop_is_mov0_exit() {
        let obj = emit_src("drop\n");
        let n = obj.insns.len();
        assert_eq!(obj.insns[n - 1].op, 0x95); // exit
        assert_eq!(obj.insns[n - 2].dst, R0); // ldx r0, [r10-..]
    }

    #[test]
    fn division_uses_signed_ops() {
        // `RUST_ISSUE_031`: `/` and `%` on signed Tcl integers must emit the
        // signed eBPF ops (BPF_SDIV / BPF_SMOD = DIV / MOD with off == 1), not
        // the unsigned forms that mangle negative operands.
        let div_op = alu64_reg_off(DIV, R1, R2, 1).op;
        let mod_op = alu64_reg_off(MOD, R1, R2, 1).op;
        let obj = emit_src(
            "when SOCKET_FILTER {\n\
             setint x {0 - 8}\n\
             setint y {$x / 2}\n\
             setint z {$x % 3}\n\
             accept\n}\n",
        );
        let div = obj
            .insns
            .iter()
            .find(|i| i.op == div_op)
            .expect("a signed div instruction");
        assert_eq!(div.off, 1, "division must be signed (off==1)");
        let modi = obj
            .insns
            .iter()
            .find(|i| i.op == mod_op)
            .expect("a signed mod instruction");
        assert_eq!(modi.off, 1, "modulo must be signed (off==1)");
    }

    #[test]
    fn port_filter_has_branches() {
        let obj = emit_src(
            "when SOCKET_FILTER {\n\
             setbuf pkt ctx\n\
             pktlen len pkt\n\
             if {$len < 36} { accept }\n\
             load16 dport pkt 36\n\
             if {$dport == 22} { drop }\n\
             accept\n}\n",
        );
        // Contains at least one conditional jump (JNE branch dispatch, 0x55) and
        // a signed compare in a Cmp materialisation (JSLT 0xcd / JEQ 0x1d).
        assert!(obj.insns.iter().any(|i| i.op == (super::JNE | 0x05)));
        assert_eq!(obj.insns.last().unwrap().op, 0x95);
    }
}
