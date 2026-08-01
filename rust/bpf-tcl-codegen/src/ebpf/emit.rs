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
//! registers, computes, and stores back. No register allocation, no calls.
//!
//! The default calling convention uses rbpf's `EbpfVmFixedMbuff`: `r1` is a
//! metadata buffer whose first two
//! 64-bit words are the packet `data` pointer (offset 0) and `data_end` pointer
//! (offset 8). The prologue loads them into the callee-saved `r6` (`data`) and
//! `r7` (`data_end`) so they survive the whole program; the packet length is
//! `data_end - data`.
//!
//! The explicit kernel-XDP target is a separate ABI. Its initial vertical slice
//! supports map-free verdict-only programs and rejects context access until
//! verifier-safe `xdp_md` lowering exists.

use std::collections::HashMap;

use bpf_tcl_ir::ir::{Block, BpfProgram, CmpOp, Inst, IntBinOp, MapDef, ProgType, Term, UnOp};
use bpf_tcl_ir::ty::Width;
use bpf_tcl_ir::{BpfDiag, BpfError};
use tcl_lexer::Span;

use crate::ebpf::insn::{
    ADD, AND, ARSH, DIV, Insn, JEQ, JNE, JSGE, JSGT, JSLE, JSLT, LSH, MOD, MUL, OR, R0, R1, R2, R3,
    R6, R7, R10, SUB, SZ_B, SZ_DW, SZ_H, SZ_W, XOR, alu64_imm, alu64_reg, alu64_reg_off, call,
    exit, ja, jmp_imm, jmp_reg, ldx, mov64_imm, mov64_reg, neg64, st_imm, stx,
};

/// Packet `data` pointer (loaded from the metadata buffer; callee-saved).
const RPTR: u8 = R6;
/// Packet `data_end` pointer (callee-saved).
const REND: u8 = R7;

/// Offset in the metadata buffer (r1) holding the `data` pointer.
const DATA_OFF: i16 = 0;
/// Offset in the metadata buffer (r1) holding the `data_end` pointer.
const DATA_END_OFF: i16 = 8;

/// Helper id for `map_get` (must match the run harness registration).
const MAP_GET_ID: i32 = 1;
/// Helper id for `map_set`.
const MAP_SET_ID: i32 = 2;

/// The execution ABI an emitted instruction stream targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetAbi {
    /// The userspace `rbpf::EbpfVmFixedMbuff` metadata-buffer ABI.
    RbpfFixedMbuff,
    /// Linux `BPF_PROG_TYPE_XDP`. The first vertical slice supports map-free,
    /// verdict-only programs; context access is rejected until its verifier
    /// proof lowering lands.
    KernelXdp,
}

impl TargetAbi {
    /// Stable CLI/display spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RbpfFixedMbuff => "rbpf",
            Self::KernelXdp => "kernel-xdp",
        }
    }
}

/// A compiled eBPF object: the instruction array plus its raw little-endian
/// byte encoding (exactly what `rbpf` executes).
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
    /// Declared maps (so the run harness can size its map store).
    pub maps: Vec<MapDef>,
}

impl EbpfObject {
    fn assemble(
        target_abi: TargetAbi,
        prog_type: ProgType,
        insns: Vec<Insn>,
        maps: Vec<MapDef>,
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
        }
    }
}

/// A not-yet-laid-out instruction. Block jumps carry a target block id resolved
/// to a relative offset in a second pass. Every variant is exactly one slot, so
/// the pending index equals the final instruction index.
enum Pending {
    Ins(Insn),
    /// `ja <block>`
    Ja(u32),
    /// `if reg != 0 goto <block>`
    JmpNeZero {
        reg: u8,
        target: u32,
    },
}

/// Stack offset of a slot: slot `i` lives at `[r10 - 8*(i+1)]`.
fn slot_off(slot: u32) -> i16 {
    // Slots are capped at 64, so `(slot+1)*8 <= 520` fits in `i16`.
    -i16::try_from((slot + 1) * 8).unwrap_or(i16::MAX)
}

/// Emit an eBPF object for a typed program.
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
    let mut pend: Vec<Pending> = Vec::new();

    // The rbpf target exposes `data` / `data_end` as two 64-bit metadata words.
    // A verdict-only kernel-XDP program does not touch its context, so it needs
    // no prologue. Kernel context access is rejected by `validate_target` until
    // correct `xdp_md` loads and verifier proofs are implemented.
    if target_abi == TargetAbi::RbpfFixedMbuff {
        pend.push(Pending::Ins(ldx(SZ_DW, RPTR, R1, DATA_OFF)));
        pend.push(Pending::Ins(ldx(SZ_DW, REND, R1, DATA_END_OFF)));
    }
    // Zero every slot so no value is read before it is written (verifier "init
    // before read").
    for i in 0..prog.num_slots {
        pend.push(Pending::Ins(st_imm(SZ_DW, R10, slot_off(i), 0)));
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

    let mut block_start: HashMap<u32, usize> = HashMap::new();
    for block in &order {
        block_start.insert(block.id.0, pend.len());
        for inst in &block.insts {
            emit_inst(inst, &mut pend)?;
        }
        emit_term(&block.term, &mut pend);
    }

    // Resolve block jumps to relative offsets.
    let mut insns = Vec::with_capacity(pend.len());
    for (idx, p) in pend.iter().enumerate() {
        let insn = match p {
            Pending::Ins(i) => *i,
            Pending::Ja(target) => ja(rel_off(idx, block_start[target])?),
            Pending::JmpNeZero { reg, target } => {
                jmp_imm(JNE, *reg, 0, rel_off(idx, block_start[target])?)
            }
        };
        insns.push(insn);
    }
    Ok(EbpfObject::assemble(
        target_abi,
        prog.prog_type,
        insns,
        prog.maps.clone(),
    ))
}

fn validate_target(prog: &BpfProgram, target_abi: TargetAbi) -> Result<(), BpfError> {
    if target_abi == TargetAbi::RbpfFixedMbuff {
        return Ok(());
    }
    if prog.prog_type != ProgType::Xdp {
        return Err(BpfError::new(
            BpfDiag::OutOfSubset,
            Span::empty(0),
            "the `kernel-xdp` target only accepts `when XDP` programs",
        ));
    }
    if !prog.maps.is_empty() {
        return Err(BpfError::new(
            BpfDiag::OutOfSubset,
            prog.maps[0].span,
            "the first `kernel-xdp` target slice is map-free (kernel map relocations are not implemented yet)",
        ));
    }
    for inst in prog.blocks.iter().flat_map(|block| &block.insts) {
        let (unsupported, span) = match inst {
            Inst::CtxPtr { span, .. } => (Some("`setbuf`"), *span),
            Inst::CtxLen { span, .. } => (Some("`pktlen`"), *span),
            Inst::Load { span, .. } => (Some("packet loads"), *span),
            Inst::MapGet { span, .. } | Inst::MapSet { span, .. } => (Some("map access"), *span),
            _ => (None, Span::empty(0)),
        };
        if let Some(operation) = unsupported {
            return Err(BpfError::new(
                BpfDiag::OutOfSubset,
                span,
                format!(
                    "{operation} is not available in the first `kernel-xdp` target slice; only map-free, verdict-only handlers are kernel-loadable"
                ),
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

fn emit_inst(inst: &Inst, pend: &mut Vec<Pending>) -> Result<(), BpfError> {
    match inst {
        Inst::Const { dst, val, span } => {
            let imm = i32::try_from(*val).map_err(|_| {
                BpfError::new(
                    BpfDiag::BadInt,
                    *span,
                    format!("constant {val} is out of 32-bit range (a v1 limitation)"),
                )
            })?;
            pend.push(Pending::Ins(mov64_imm(R1, imm)));
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
        Inst::Load {
            dst,
            width,
            ptr,
            off,
            span,
        } => {
            let off16 = i16::try_from(*off).map_err(|_| {
                BpfError::new(
                    BpfDiag::BadInt,
                    *span,
                    "load offset out of range (must fit in 16 bits)",
                )
            })?;
            pend.push(Pending::Ins(ldx(SZ_DW, R1, R10, slot_off(ptr.0))));
            pend.push(Pending::Ins(ldx(width_size(*width), R2, R1, off16)));
            pend.push(Pending::Ins(stx(SZ_DW, R10, R2, slot_off(dst.0))));
        }
        Inst::MapGet { dst, map, key, .. } => {
            // r1 = map index, r2 = key; r0 = map_get(...); store r0.
            pend.push(Pending::Ins(mov64_imm(R1, map_imm(*map))));
            pend.push(Pending::Ins(ldx(SZ_DW, R2, R10, slot_off(key.0))));
            pend.push(Pending::Ins(call(MAP_GET_ID)));
            pend.push(Pending::Ins(stx(SZ_DW, R10, R0, slot_off(dst.0))));
        }
        Inst::MapSet { map, key, val, .. } => {
            // r1 = map index, r2 = key, r3 = value; map_set(...).
            pend.push(Pending::Ins(mov64_imm(R1, map_imm(*map))));
            pend.push(Pending::Ins(ldx(SZ_DW, R2, R10, slot_off(key.0))));
            pend.push(Pending::Ins(ldx(SZ_DW, R3, R10, slot_off(val.0))));
            pend.push(Pending::Ins(call(MAP_SET_ID)));
        }
    }
    Ok(())
}

/// A small map index as a 32-bit immediate.
fn map_imm(map: u32) -> i32 {
    i32::try_from(map).unwrap_or(0)
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
/// Tcl integers are signed 64-bit — the CFG lowers comparisons to signed jumps
/// (`JSLT`, …) and uses sign-extending moves — so `/`, `%`, and `>>` must use the
/// **signed** eBPF ops. `BPF_SDIV` / `BPF_SMOD` are encoded as `DIV` / `MOD` with
/// `off == 1`; the plain unsigned `DIV` / `MOD` (off 0) reinterpret a negative
/// operand as a huge unsigned value, giving a catastrophically wrong result
/// silently (`RUST_ISSUE_031`). `>>` likewise lowers to the arithmetic
/// (sign-preserving) `ARSH`, not the logical `RSH`, so `-8 >> 1` is `-4` as Tcl
/// requires rather than a huge positive (`RUST_ISSUE_062` / `097`). Every other
/// op keeps `off == 0`.
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
    fn kernel_xdp_verdict_only_has_no_rbpf_context_prologue() {
        let module = compile_module("when XDP { pass }\n").expect("compiles");
        let obj = emit_program_for_target(&module.programs[0].program, TargetAbi::KernelXdp)
            .expect("emits for the kernel");
        assert_eq!(obj.target_abi, TargetAbi::KernelXdp);
        assert_eq!(obj.insns.len(), 5);
        assert_ne!(obj.insns[0], ldx(SZ_DW, R6, R1, 0));
        assert_eq!(obj.insns.last().unwrap().op, 0x95);
    }

    #[test]
    fn kernel_xdp_rejects_context_access_until_proof_lowering_exists() {
        let module = compile_module("when XDP { setbuf packet ctx\n pass }\n").expect("compiles");
        let err = emit_program_for_target(&module.programs[0].program, TargetAbi::KernelXdp)
            .expect_err("context ABI is not implemented");
        assert_eq!(err.code, BpfDiag::OutOfSubset);
        assert!(err.msg.contains("verdict-only"));
    }

    #[test]
    fn drop_is_mov0_exit() {
        let obj = emit_src("drop\n");
        // Somewhere a `mov r0, <slot>` then exit; verify it ends exit and the
        // verdict load targets r0.
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
