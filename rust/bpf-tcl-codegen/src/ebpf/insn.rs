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

//! eBPF instruction encoding — the classic 64-bit instruction format, emitted
//! directly (no assembler, no LLVM).
//!
//! Layout (little-endian): `op:u8, dst:4|src:4, off:i16, imm:i32`.

// A module of tiny instruction-encoding constructors; per-fn `#[must_use]` on
// every one is noise, so opt the whole encoder module out at once.
#![allow(clippy::must_use_candidate)]

/// One eBPF instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Insn {
    /// Opcode byte (class | source | operation, or class | size | mode).
    pub op: u8,
    /// Destination register (0–10).
    pub dst: u8,
    /// Source register (0–10).
    pub src: u8,
    /// 16-bit signed offset (memory displacement or jump distance).
    pub off: i16,
    /// 32-bit signed immediate.
    pub imm: i32,
}

impl Insn {
    /// Encode to the canonical 8 little-endian bytes.
    pub fn to_le_bytes(self) -> [u8; 8] {
        let regs = (self.dst & 0x0f) | ((self.src & 0x0f) << 4);
        let off = self.off.to_le_bytes();
        let imm = self.imm.to_le_bytes();
        [
            self.op, regs, off[0], off[1], imm[0], imm[1], imm[2], imm[3],
        ]
    }
}

// ── instruction classes (low 3 bits) ───────────────────────────────────────
pub const LD: u8 = 0x00;
pub const LDX: u8 = 0x01;
pub const ST: u8 = 0x02;
pub const STX: u8 = 0x03;
pub const ALU: u8 = 0x04; // 32-bit ALU (used only for the byte-order ops)
pub const JMP: u8 = 0x05;
pub const ALU64: u8 = 0x07;

// ── load mode: 64-bit immediate (`lddw`) ────────────────────────────────────
pub const IMM: u8 = 0x00;
pub const DW: u8 = 0x18;

// ── byte-order (BPF_END) ────────────────────────────────────────────────────
// The `BPF_END` operation converts a register's byte order. With the source
// (`K`) bit clear it converts to little-endian; the `X` bit selects big-endian.
pub const END: u8 = 0xd0;

/// `BPF_PSEUDO_MAP_FD` — the `src_reg` value of a `ld_imm64` whose immediate is
/// a map file descriptor. libbpf rewrites such an instruction's immediate to the
/// real map fd at load time, driven by an ELF relocation against the map symbol.
pub const PSEUDO_MAP_FD: u8 = 1;

// ── kernel helper ids (`BPF_FUNC_*` from the UAPI helper table) ──────────────
/// `bpf_map_lookup_elem(map, key) -> value_ptr_or_null`.
pub const FN_MAP_LOOKUP: i32 = 1;
/// `bpf_map_update_elem(map, key, value, flags) -> 0/err`.
pub const FN_MAP_UPDATE: i32 = 2;
/// `bpf_map_delete_elem(map, key) -> 0/err`.
pub const FN_MAP_DELETE: i32 = 3;

// ── ALU / JMP source ────────────────────────────────────────────────────────
pub const K: u8 = 0x00; // immediate
pub const X: u8 = 0x08; // src register

// ── load/store sizes ──────────────────────────────────────────────────────
pub const SZ_W: u8 = 0x00; // 32-bit
pub const SZ_H: u8 = 0x08; // 16-bit
pub const SZ_B: u8 = 0x10; // 8-bit
pub const SZ_DW: u8 = 0x18; // 64-bit

// ── load/store mode ────────────────────────────────────────────────────────
pub const MEM: u8 = 0x60;

// ── ALU operations (high nibble) ───────────────────────────────────────────
pub const ADD: u8 = 0x00;
pub const SUB: u8 = 0x10;
pub const MUL: u8 = 0x20;
pub const DIV: u8 = 0x30;
pub const OR: u8 = 0x40;
pub const AND: u8 = 0x50;
pub const LSH: u8 = 0x60;
pub const RSH: u8 = 0x70;
pub const NEG: u8 = 0x80;
pub const MOD: u8 = 0x90;
pub const XOR: u8 = 0xa0;
pub const MOV: u8 = 0xb0;
/// Arithmetic (sign-preserving) shift right — Tcl's `>>` on a negative value.
/// Distinct from the logical `RSH`, which zero-fills.
pub const ARSH: u8 = 0xc0;

// ── JMP operations (high nibble) ───────────────────────────────────────────
pub const JA: u8 = 0x00;
pub const JEQ: u8 = 0x10;
/// Unsigned greater-than.
pub const JGT: u8 = 0x20;
pub const JNE: u8 = 0x50;
pub const JSGT: u8 = 0x60;
pub const JSGE: u8 = 0x70;
/// Unsigned less-than-or-equal.
pub const JLE: u8 = 0xb0;
pub const JSLT: u8 = 0xc0;
pub const JSLE: u8 = 0xd0;
pub const EXIT: u8 = 0x90;
pub const CALL: u8 = 0x80;

// ── registers ──────────────────────────────────────────────────────────────
pub const R0: u8 = 0;
pub const R1: u8 = 1;
pub const R2: u8 = 2;
pub const R3: u8 = 3;
pub const R4: u8 = 4;
pub const R6: u8 = 6;
pub const R7: u8 = 7;
pub const R8: u8 = 8;
pub const R10: u8 = 10;

// ── builders ───────────────────────────────────────────────────────────────

/// `dst = imm` (64-bit move; the 32-bit immediate is sign-extended).
pub fn mov64_imm(dst: u8, imm: i32) -> Insn {
    Insn {
        op: ALU64 | K | MOV,
        dst,
        src: 0,
        off: 0,
        imm,
    }
}

/// `dst = src` (64-bit register move).
pub fn mov64_reg(dst: u8, src: u8) -> Insn {
    Insn {
        op: ALU64 | X | MOV,
        dst,
        src,
        off: 0,
        imm: 0,
    }
}

/// `dst = dst <alu_op> imm` (64-bit).
pub fn alu64_imm(alu_op: u8, dst: u8, imm: i32) -> Insn {
    Insn {
        op: ALU64 | K | alu_op,
        dst,
        src: 0,
        off: 0,
        imm,
    }
}

/// `dst = dst <alu_op> src` (64-bit).
pub fn alu64_reg(alu_op: u8, dst: u8, src: u8) -> Insn {
    alu64_reg_off(alu_op, dst, src, 0)
}

/// `dst = dst <alu_op> src` (64-bit) with an explicit `off` field.
///
/// `off` selects the signed variant of `DIV` / `MOD`: `BPF_SDIV` / `BPF_SMOD`
/// are encoded as `DIV` / `MOD` with `off == 1` (the sign-aware division form).
/// For all other ALU ops `off` is 0.
pub fn alu64_reg_off(alu_op: u8, dst: u8, src: u8, off: i16) -> Insn {
    Insn {
        op: ALU64 | X | alu_op,
        dst,
        src,
        off,
        imm: 0,
    }
}

/// `dst = -dst` (64-bit negate).
pub fn neg64(dst: u8) -> Insn {
    Insn {
        op: ALU64 | NEG,
        dst,
        src: 0,
        off: 0,
        imm: 0,
    }
}

/// `dst = *(size *)(base + off)` (zero-extended load).
pub fn ldx(size: u8, dst: u8, base: u8, off: i16) -> Insn {
    Insn {
        op: LDX | MEM | size,
        dst,
        src: base,
        off,
        imm: 0,
    }
}

/// `*(size *)(base + off) = val` (register store).
pub fn stx(size: u8, base: u8, val: u8, off: i16) -> Insn {
    Insn {
        op: STX | MEM | size,
        dst: base,
        src: val,
        off,
        imm: 0,
    }
}

/// `*(size *)(base + off) = imm` (immediate store).
pub fn st_imm(size: u8, base: u8, off: i16, imm: i32) -> Insn {
    Insn {
        op: ST | MEM | size,
        dst: base,
        src: 0,
        off,
        imm,
    }
}

/// Conditional jump comparing `dst` to `imm`: `if (dst <jop> imm) pc += off`.
pub fn jmp_imm(jop: u8, dst: u8, imm: i32, off: i16) -> Insn {
    Insn {
        op: JMP | K | jop,
        dst,
        src: 0,
        off,
        imm,
    }
}

/// Conditional jump comparing `dst` to `src`: `if (dst <jop> src) pc += off`.
pub fn jmp_reg(jop: u8, dst: u8, src: u8, off: i16) -> Insn {
    Insn {
        op: JMP | X | jop,
        dst,
        src,
        off,
        imm: 0,
    }
}

/// Unconditional jump: `pc += off`.
pub fn ja(off: i16) -> Insn {
    Insn {
        op: JMP | JA,
        dst: 0,
        src: 0,
        off,
        imm: 0,
    }
}

/// `exit` (return r0 to the caller).
pub fn exit() -> Insn {
    Insn {
        op: JMP | EXIT,
        dst: 0,
        src: 0,
        off: 0,
        imm: 0,
    }
}

/// `call <helper_id>` — a helper call (`src = 0` selects the helper table; the
/// id is the immediate). Arguments are in r1–r5, the result in r0.
pub fn call(helper_id: i32) -> Insn {
    Insn {
        op: JMP | CALL,
        dst: 0,
        src: 0,
        off: 0,
        imm: helper_id,
    }
}

/// `dst = imm64` — the two-instruction wide immediate load (`lddw`). The low
/// 32 bits ride in the first instruction's `imm`, the high 32 bits in the
/// second's; the second instruction has a zero opcode by convention.
pub fn lddw(dst: u8, imm: i64) -> [Insn; 2] {
    lddw_src(dst, 0, imm)
}

/// `dst = <pseudo imm64>` — a wide-immediate load with an explicit `src_reg`.
///
/// A `src_reg` of [`PSEUDO_MAP_FD`] marks the immediate as a map file
/// descriptor placeholder (zero here), to be rewritten by libbpf from a
/// relocation against the map symbol.
pub fn lddw_src(dst: u8, src: u8, imm: i64) -> [Insn; 2] {
    let lo = (imm & 0xffff_ffff) as i32;
    let hi = ((imm >> 32) & 0xffff_ffff) as i32;
    [
        Insn {
            op: LD | IMM | DW,
            dst,
            src,
            off: 0,
            imm: lo,
        },
        Insn {
            op: 0,
            dst: 0,
            src: 0,
            off: 0,
            imm: hi,
        },
    ]
}

/// `dst = bswap<width>(dst)` selecting host↔big-endian (`bpf_htobe*`). `width`
/// is 16, 32, or 64. Encoded as `BPF_ALU | BPF_X | BPF_END` (the X bit selects
/// the big-endian direction), with the width in the immediate.
pub fn bswap_be(dst: u8, width: i32) -> Insn {
    Insn {
        op: ALU | X | END,
        dst,
        src: 0,
        off: 0,
        imm: width,
    }
}

/// `dst = bswap<width>(dst)` selecting host↔little-endian (`bpf_htole*`).
/// Encoded as `BPF_ALU | BPF_K | BPF_END`.
pub fn bswap_le(dst: u8, width: i32) -> Insn {
    Insn {
        op: ALU | K | END,
        dst,
        src: 0,
        off: 0,
        imm: width,
    }
}
