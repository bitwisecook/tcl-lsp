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

//! Userspace eBPF interpreter following the Linux kernel's instruction
//! semantics, cpu v4 included: signed `DIV`/`MOD` (`off == 1`), `MOVSX`,
//! sign-extending `LDX` (`BPF_MEMSX`), unconditional `BSWAP` and `gotol`.
//!
//! The program sees the rbpf "fixed mbuff" ABI: `r1` points at a 16-byte
//! metadata buffer holding the packet start and end pointers at offsets 0 and
//! 8, and `r10` is the top of a 512-byte zeroed stack. Addresses are virtual;
//! every access is bounds-checked against the mbuff, packet and stack.

// Narrowing and sign-reinterpreting casts are the instruction semantics here.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]

use std::collections::HashMap;

/// A helper reachable through `call imm`: receives `r1`–`r5`, returns `r0`.
pub type Helper = fn(u64, u64, u64, u64, u64) -> u64;

pub const STACK_SIZE: usize = 512;
const MBUFF_SIZE: usize = 16;
const MAX_PROG_INSNS: usize = 1_000_000;
const MAX_STEPS: u64 = 1 << 26;

const MBUFF_BASE: u64 = 0x1_0000_0000;
const PACKET_BASE: u64 = 0x2_0000_0000;
const STACK_BASE: u64 = 0x3_0000_0000;

const LD: u8 = 0x00;
const LDX: u8 = 0x01;
const ST: u8 = 0x02;
const STX: u8 = 0x03;
const ALU: u8 = 0x04;
const JMP: u8 = 0x05;
const JMP32: u8 = 0x06;
const ALU64: u8 = 0x07;

const SRC_X: u8 = 0x08;
const MODE_IMM: u8 = 0x00;
const MODE_MEM: u8 = 0x60;
const MODE_MEMSX: u8 = 0x80;
const SIZE_DW: u8 = 0x18;

const OP_DIV: u8 = 0x30;
const OP_LSH: u8 = 0x60;
const OP_RSH: u8 = 0x70;
const OP_NEG: u8 = 0x80;
const OP_MOD: u8 = 0x90;
const OP_MOV: u8 = 0xb0;
const OP_ARSH: u8 = 0xc0;
const OP_END: u8 = 0xd0;

const J_JA: u8 = 0x00;
const J_CALL: u8 = 0x80;
const J_EXIT: u8 = 0x90;

#[derive(Debug, Clone, Copy)]
struct Insn {
    op: u8,
    dst: usize,
    src: usize,
    off: i16,
    imm: i32,
}

impl Insn {
    fn decode(b: [u8; 8]) -> Self {
        Self {
            op: b[0],
            dst: usize::from(b[1] & 0x0f),
            src: usize::from(b[1] >> 4),
            off: i16::from_le_bytes([b[2], b[3]]),
            imm: i32::from_le_bytes([b[4], b[5], b[6], b[7]]),
        }
    }

    fn class(self) -> u8 {
        self.op & 0x07
    }

    fn code(self) -> u8 {
        self.op & 0xf0
    }

    fn reg_src(self) -> bool {
        self.op & SRC_X != 0
    }
}

/// A decoded program that passed the static checks.
pub struct Program {
    insns: Vec<Insn>,
}

impl Program {
    /// Decode and statically check raw eBPF bytecode.
    ///
    /// # Errors
    /// Describes the first malformed or unsupported instruction.
    pub fn new(bytes: &[u8]) -> Result<Self, String> {
        if bytes.is_empty() || !bytes.len().is_multiple_of(8) {
            return Err(format!(
                "program length {} is not a non-zero multiple of 8",
                bytes.len()
            ));
        }
        let insns: Vec<Insn> = bytes
            .as_chunks::<8>()
            .0
            .iter()
            .copied()
            .map(Insn::decode)
            .collect();
        if insns.len() > MAX_PROG_INSNS {
            return Err(format!(
                "program has {} insns, limit {MAX_PROG_INSNS}",
                insns.len()
            ));
        }
        let prog = Self { insns };
        prog.check()?;
        Ok(prog)
    }

    fn check(&self) -> Result<(), String> {
        let n = self.insns.len();
        let mut wide_tail = vec![false; n];
        let mut targets = Vec::new();
        let mut pc = 0;
        while pc < n {
            let i = self.insns[pc];
            let bad = |why: &str| Err(format!("insn #{pc} (op 0x{:02x}): {why}", i.op));
            if i.dst > 10 || i.src > 10 {
                return bad("invalid register");
            }
            let writes_dst = matches!(i.class(), LD | LDX | ALU | ALU64);
            if writes_dst && i.dst == 10 {
                return bad("r10 is read-only");
            }
            match i.class() {
                LD => {
                    if i.op != LD | MODE_IMM | SIZE_DW {
                        return bad("unsupported load");
                    }
                    if pc + 1 >= n || self.insns[pc + 1].op != 0 {
                        return bad("incomplete lddw");
                    }
                    wide_tail[pc + 1] = true;
                    pc += 2;
                    continue;
                }
                LDX => {
                    let mode = i.op & 0xe0;
                    if !(mode == MODE_MEM || (mode == MODE_MEMSX && i.op & 0x18 != SIZE_DW)) {
                        return bad("unsupported load mode");
                    }
                }
                ST | STX => {
                    if i.op & 0xe0 != MODE_MEM {
                        return bad("unsupported store mode");
                    }
                }
                ALU | ALU64 => check_alu(i).or_else(bad)?,
                JMP | JMP32 => match i.code() {
                    J_EXIT | J_CALL if i.class() == JMP32 => return bad("invalid in JMP32"),
                    J_EXIT => {}
                    J_CALL => {
                        if i.src != 0 {
                            return bad("only helper calls are supported");
                        }
                    }
                    0xe0 | 0xf0 => return bad("unknown jump"),
                    _ => {
                        let rel = if i.class() == JMP32 && i.code() == J_JA {
                            i64::from(i.imm)
                        } else {
                            i64::from(i.off)
                        };
                        let target = i64::try_from(pc).unwrap_or(i64::MAX) + 1 + rel;
                        match usize::try_from(target) {
                            Ok(t) if t < n => targets.push((pc, t)),
                            _ => return bad("jump out of range"),
                        }
                    }
                },
                _ => return bad("unknown instruction class"),
            }
            pc += 1;
        }
        if let Some((from, _)) = targets.iter().find(|(_, t)| wide_tail[*t]) {
            return Err(format!("insn #{from}: jump into the middle of lddw"));
        }
        let last = self.insns[n - 1];
        if wide_tail[n - 1] || last.op != JMP | J_EXIT {
            return Err("program does not end in exit".to_owned());
        }
        Ok(())
    }

    /// Execute over `packet`, resolving `call imm` through `helpers`.
    ///
    /// # Errors
    /// Out-of-bounds memory access, an unknown helper, or a runaway program.
    pub fn execute(
        &self,
        packet: &mut [u8],
        helpers: &HashMap<i32, Helper>,
    ) -> Result<u64, String> {
        let mut mem = Memory {
            mbuff: [0; MBUFF_SIZE],
            packet,
            stack: [0; STACK_SIZE],
        };
        let pkt_end = PACKET_BASE + mem.packet.len() as u64;
        mem.mbuff[..8].copy_from_slice(&PACKET_BASE.to_le_bytes());
        mem.mbuff[8..].copy_from_slice(&pkt_end.to_le_bytes());

        let mut reg = [0u64; 11];
        reg[1] = MBUFF_BASE;
        reg[10] = STACK_BASE + STACK_SIZE as u64;

        let mut pc = 0usize;
        let mut steps = 0u64;
        loop {
            steps += 1;
            if steps > MAX_STEPS {
                return Err(format!("exceeded {MAX_STEPS} executed instructions"));
            }
            let i = self.insns[pc];
            pc += 1;
            let src_val = if i.reg_src() {
                reg[i.src]
            } else {
                i64::from(i.imm).cast_unsigned()
            };
            match i.class() {
                LD => {
                    let hi = u64::from(self.insns[pc].imm.cast_unsigned());
                    reg[i.dst] = u64::from(i.imm.cast_unsigned()) | (hi << 32);
                    pc += 1;
                }
                LDX => {
                    let addr = reg[i.src].wrapping_add_signed(i64::from(i.off));
                    let size = access_size(i.op);
                    let raw = mem.load(addr, size, pc - 1)?;
                    reg[i.dst] = if i.op & 0xe0 == MODE_MEMSX {
                        sign_extend(raw, size)
                    } else {
                        raw
                    };
                }
                ST | STX => {
                    let addr = reg[i.dst].wrapping_add_signed(i64::from(i.off));
                    let val = if i.class() == ST {
                        i64::from(i.imm).cast_unsigned()
                    } else {
                        reg[i.src]
                    };
                    mem.store(addr, access_size(i.op), val, pc - 1)?;
                }
                ALU64 => reg[i.dst] = alu64(i, reg[i.dst], src_val),
                ALU => reg[i.dst] = alu32(i, reg[i.dst], src_val),
                _ => match i.code() {
                    J_EXIT => return Ok(reg[0]),
                    J_CALL => {
                        let h = helpers
                            .get(&i.imm)
                            .ok_or_else(|| format!("insn #{}: unknown helper {}", pc - 1, i.imm))?;
                        reg[0] = h(reg[1], reg[2], reg[3], reg[4], reg[5]);
                    }
                    J_JA => {
                        let rel = if i.class() == JMP32 {
                            i64::from(i.imm)
                        } else {
                            i64::from(i.off)
                        };
                        pc = jump(pc, rel);
                    }
                    code => {
                        let taken = if i.class() == JMP32 {
                            cond(
                                code,
                                u64::from(reg[i.dst] as u32),
                                u64::from(src_val as u32),
                                true,
                            )
                        } else {
                            cond(code, reg[i.dst], src_val, false)
                        };
                        if taken {
                            pc = jump(pc, i64::from(i.off));
                        }
                    }
                },
            }
        }
    }
}

fn check_alu(i: Insn) -> Result<(), &'static str> {
    let is64 = i.class() == ALU64;
    match i.code() {
        OP_DIV | OP_MOD => {
            if !matches!(i.off, 0 | 1) {
                return Err("div/mod offset must be 0 or 1");
            }
            if !i.reg_src() && i.imm == 0 {
                return Err("division by zero immediate");
            }
        }
        OP_MOV => {
            let ok = match i.off {
                0 => true,
                8 | 16 => i.reg_src(),
                32 => i.reg_src() && is64,
                _ => false,
            };
            if !ok {
                return Err("invalid mov offset");
            }
        }
        OP_END => {
            if !matches!(i.imm, 16 | 32 | 64) {
                return Err("byte swap width must be 16, 32 or 64");
            }
            if is64 && i.reg_src() {
                return Err("invalid bswap source");
            }
            return if i.off == 0 {
                Ok(())
            } else {
                Err("non-zero offset")
            };
        }
        OP_NEG if i.reg_src() => return Err("neg takes no source"),
        0xe0 | 0xf0 => return Err("unknown ALU op"),
        _ => {}
    }
    if matches!(i.code(), OP_LSH | OP_RSH | OP_ARSH) && !i.reg_src() {
        let width = if is64 { 64 } else { 32 };
        if !(0..width).contains(&i.imm) {
            return Err("shift immediate out of range");
        }
    }
    if !matches!(i.code(), OP_DIV | OP_MOD | OP_MOV) && i.off != 0 {
        return Err("non-zero offset");
    }
    Ok(())
}

fn alu64(i: Insn, d: u64, s: u64) -> u64 {
    let signed = i.off == 1;
    match i.code() {
        0x00 => d.wrapping_add(s),
        0x10 => d.wrapping_sub(s),
        0x20 => d.wrapping_mul(s),
        OP_DIV if s == 0 => 0,
        OP_DIV if signed => d
            .cast_signed()
            .wrapping_div(s.cast_signed())
            .cast_unsigned(),
        OP_DIV => d / s,
        0x40 => d | s,
        0x50 => d & s,
        OP_LSH => d << (s & 63),
        OP_RSH => d >> (s & 63),
        OP_NEG => d.cast_signed().wrapping_neg().cast_unsigned(),
        OP_MOD if s == 0 => d,
        OP_MOD if signed => d
            .cast_signed()
            .wrapping_rem(s.cast_signed())
            .cast_unsigned(),
        OP_MOD => d % s,
        0xa0 => d ^ s,
        OP_MOV => match i.off {
            8 => i64::from(s as i8).cast_unsigned(),
            16 => i64::from(s as i16).cast_unsigned(),
            32 => i64::from(s as i32).cast_unsigned(),
            _ => s,
        },
        OP_ARSH => (d.cast_signed() >> (s & 63)).cast_unsigned(),
        _ => bswap(d, i.imm),
    }
}

fn alu32(i: Insn, d: u64, s: u64) -> u64 {
    if i.code() == OP_END {
        return if i.reg_src() {
            bswap(d, i.imm)
        } else {
            truncate(d, i.imm)
        };
    }
    let (d, s) = (d as u32, s as u32);
    let signed = i.off == 1;
    let r = match i.code() {
        0x00 => d.wrapping_add(s),
        0x10 => d.wrapping_sub(s),
        0x20 => d.wrapping_mul(s),
        OP_DIV if s == 0 => 0,
        OP_DIV if signed => d
            .cast_signed()
            .wrapping_div(s.cast_signed())
            .cast_unsigned(),
        OP_DIV => d / s,
        0x40 => d | s,
        0x50 => d & s,
        OP_LSH => d << (s & 31),
        OP_RSH => d >> (s & 31),
        OP_NEG => d.cast_signed().wrapping_neg().cast_unsigned(),
        OP_MOD if s == 0 => d,
        OP_MOD if signed => d
            .cast_signed()
            .wrapping_rem(s.cast_signed())
            .cast_unsigned(),
        OP_MOD => d % s,
        0xa0 => d ^ s,
        OP_MOV => match i.off {
            8 => i32::from(s as i8).cast_unsigned(),
            16 => i32::from(s as i16).cast_unsigned(),
            _ => s,
        },
        _ => (d.cast_signed() >> (s & 31)).cast_unsigned(),
    };
    u64::from(r)
}

fn bswap(v: u64, width: i32) -> u64 {
    match width {
        16 => u64::from((v as u16).swap_bytes()),
        32 => u64::from((v as u32).swap_bytes()),
        _ => v.swap_bytes(),
    }
}

fn truncate(v: u64, width: i32) -> u64 {
    match width {
        16 => v & 0xffff,
        32 => v & 0xffff_ffff,
        _ => v,
    }
}

fn cond(code: u8, d: u64, s: u64, narrow: bool) -> bool {
    let (sd, ss) = if narrow {
        (i64::from(d as u32 as i32), i64::from(s as u32 as i32))
    } else {
        (d.cast_signed(), s.cast_signed())
    };
    match code {
        0x10 => d == s,
        0x20 => d > s,
        0x30 => d >= s,
        0x40 => d & s != 0,
        0x50 => d != s,
        0x60 => sd > ss,
        0x70 => sd >= ss,
        0xa0 => d < s,
        0xb0 => d <= s,
        0xc0 => sd < ss,
        _ => sd <= ss,
    }
}

fn jump(pc: usize, rel: i64) -> usize {
    pc.wrapping_add_signed(rel as isize)
}

fn access_size(op: u8) -> usize {
    match op & 0x18 {
        0x00 => 4,
        0x08 => 2,
        0x10 => 1,
        _ => 8,
    }
}

fn sign_extend(v: u64, size: usize) -> u64 {
    match size {
        1 => i64::from(v as i8).cast_unsigned(),
        2 => i64::from(v as i16).cast_unsigned(),
        4 => i64::from(v as i32).cast_unsigned(),
        _ => v,
    }
}

struct Memory<'p> {
    mbuff: [u8; MBUFF_SIZE],
    packet: &'p mut [u8],
    stack: [u8; STACK_SIZE],
}

impl Memory<'_> {
    fn region(&mut self, addr: u64, len: usize, pc: usize) -> Result<&mut [u8], String> {
        let regions: [(u64, &mut [u8]); 3] = [
            (MBUFF_BASE, &mut self.mbuff),
            (PACKET_BASE, self.packet),
            (STACK_BASE, &mut self.stack),
        ];
        for (base, buf) in regions {
            let Some(off) = addr.checked_sub(base) else {
                continue;
            };
            let Ok(off) = usize::try_from(off) else {
                continue;
            };
            if off.checked_add(len).is_some_and(|end| end <= buf.len()) {
                return Ok(&mut buf[off..off + len]);
            }
        }
        Err(format!(
            "insn #{pc}: out-of-bounds {len}-byte access at 0x{addr:x}"
        ))
    }

    fn load(&mut self, addr: u64, len: usize, pc: usize) -> Result<u64, String> {
        let mut b = [0u8; 8];
        b[..len].copy_from_slice(self.region(addr, len, pc)?);
        Ok(u64::from_le_bytes(b))
    }

    fn store(&mut self, addr: u64, len: usize, val: u64, pc: usize) -> Result<(), String> {
        self.region(addr, len, pc)?
            .copy_from_slice(&val.to_le_bytes()[..len]);
        Ok(())
    }
}
