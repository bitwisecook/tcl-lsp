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

//! A userspace eBPF interpreter with Linux kernel instruction semantics,
//! including the cpu-v4 offset-qualified forms (`BPF_SDIV` / `BPF_SMOD`,
//! `MOVSX`, sign-extending `LDX MEMSX`, unconditional `BSWAP`, `JMP32 JA`).
//!
//! It runs the fixed-metadata-buffer ABI the `rbpf` target emits for: `r1`
//! points at a 16-byte buffer holding the packet start and end pointers at
//! offsets 0 and 8, `r10` is the top of a 512-byte stack. Pointers are
//! virtual addresses into three disjoint regions (metadata, packet, stack);
//! every access is bounds-checked against them.

// eBPF sub-register and narrow-width ops truncate by definition.
#![allow(clippy::cast_possible_truncation)]

/// Stack size in bytes, as in the kernel.
pub const STACK_SIZE: usize = 512;
/// Instruction budget per run, so a malformed backward jump cannot hang.
pub const INSN_BUDGET: u64 = 1 << 20;

const MBUFF_LEN: usize = 16;
const MBUFF_BASE: u64 = 0x1000_0000;
const PACKET_BASE: u64 = 0x2000_0000;
const STACK_BASE: u64 = 0x3000_0000;

const CLS_LD: u8 = 0x00;
const CLS_LDX: u8 = 0x01;
const CLS_ST: u8 = 0x02;
const CLS_STX: u8 = 0x03;
const CLS_ALU: u8 = 0x04;
const CLS_JMP: u8 = 0x05;
const CLS_JMP32: u8 = 0x06;
const CLS_ALU64: u8 = 0x07;

const MODE_IMM: u8 = 0x00;
const MODE_MEM: u8 = 0x60;
const MODE_MEMSX: u8 = 0x80;

const SRC_X: u8 = 0x08;

/// A decoded instruction.
#[derive(Clone, Copy)]
struct Insn {
    op: u8,
    dst: usize,
    src: usize,
    off: i16,
    imm: i32,
}

fn decode(raw: &[u8]) -> Result<Vec<Insn>, String> {
    if !raw.len().is_multiple_of(8) {
        return Err(format!(
            "program length {} is not a multiple of 8",
            raw.len()
        ));
    }
    raw.as_chunks::<8>()
        .0
        .iter()
        .enumerate()
        .map(|(pc, b)| {
            let dst = usize::from(b[1] & 0x0f);
            let src = usize::from(b[1] >> 4);
            if dst > 10 || src > 10 {
                return Err(format!("insn {pc}: invalid register"));
            }
            Ok(Insn {
                op: b[0],
                dst,
                src,
                off: i16::from_le_bytes([b[2], b[3]]),
                imm: i32::from_le_bytes([b[4], b[5], b[6], b[7]]),
            })
        })
        .collect()
}

/// The three addressable regions of one run.
struct Memory<'a> {
    mbuff: [u8; MBUFF_LEN],
    packet: &'a mut [u8],
    stack: [u8; STACK_SIZE],
}

impl Memory<'_> {
    fn slice(&mut self, addr: u64, len: usize, pc: usize) -> Result<&mut [u8], String> {
        let regions: [(u64, &mut [u8]); 3] = [
            (MBUFF_BASE, &mut self.mbuff),
            (PACKET_BASE, &mut *self.packet),
            (STACK_BASE, &mut self.stack),
        ];
        for (base, buf) in regions {
            let Some(rel) = addr.checked_sub(base) else {
                continue;
            };
            let Ok(start) = usize::try_from(rel) else {
                continue;
            };
            if let Some(end) = start.checked_add(len)
                && end <= buf.len()
            {
                return Ok(&mut buf[start..end]);
            }
        }
        Err(format!(
            "insn {pc}: out-of-bounds {len}-byte access at {addr:#x}"
        ))
    }

    fn load(&mut self, addr: u64, len: usize, pc: usize) -> Result<u64, String> {
        let mut b = [0u8; 8];
        b[..len].copy_from_slice(self.slice(addr, len, pc)?);
        Ok(u64::from_le_bytes(b))
    }

    fn store(&mut self, addr: u64, len: usize, val: u64, pc: usize) -> Result<(), String> {
        self.slice(addr, len, pc)?
            .copy_from_slice(&val.to_le_bytes()[..len]);
        Ok(())
    }
}

fn size_bytes(op: u8) -> usize {
    match op & 0x18 {
        0x00 => 4,
        0x08 => 2,
        0x10 => 1,
        _ => 8,
    }
}

fn sign_extend(v: u64, bits: u32) -> u64 {
    let shift = 64 - bits;
    (((v << shift).cast_signed()) >> shift).cast_unsigned()
}

fn offset_addr(base: u64, off: i16) -> u64 {
    base.wrapping_add_signed(i64::from(off))
}

/// 64-bit ALU. `None` for an encoding the kernel rejects.
fn alu64(op: u8, off: i16, dst: u64, src: u64) -> Option<u64> {
    let (sd, ss) = (dst.cast_signed(), src.cast_signed());
    Some(match (op & 0xf0, off) {
        (0x00, 0) => dst.wrapping_add(src),
        (0x10, 0) => dst.wrapping_sub(src),
        (0x20, 0) => dst.wrapping_mul(src),
        (0x30, 0) => dst.checked_div(src).unwrap_or(0),
        (0x30, 1) => {
            if ss == 0 {
                0
            } else {
                sd.wrapping_div(ss).cast_unsigned()
            }
        }
        (0x40, 0) => dst | src,
        (0x50, 0) => dst & src,
        (0x60, 0) => dst.wrapping_shl(src as u32),
        (0x70, 0) => dst.wrapping_shr(src as u32),
        (0x80, 0) => sd.wrapping_neg().cast_unsigned(),
        (0x90, 0) => dst.checked_rem(src).unwrap_or(dst),
        (0x90, 1) => {
            if ss == 0 {
                dst
            } else {
                sd.wrapping_rem(ss).cast_unsigned()
            }
        }
        (0xa0, 0) => dst ^ src,
        (0xb0, 0) => src,
        (0xb0, 8) => sign_extend(src, 8),
        (0xb0, 16) => sign_extend(src, 16),
        (0xb0, 32) => sign_extend(src, 32),
        (0xc0, 0) => sd.wrapping_shr(src as u32).cast_unsigned(),
        _ => return None,
    })
}

/// 32-bit ALU; the result is zero-extended to 64 bits.
fn alu32(op: u8, off: i16, dst: u64, src: u64) -> Option<u64> {
    let (d, s) = (dst as u32, src as u32);
    let (sd, ss) = (d.cast_signed(), s.cast_signed());
    let r: u32 = match (op & 0xf0, off) {
        (0x00, 0) => d.wrapping_add(s),
        (0x10, 0) => d.wrapping_sub(s),
        (0x20, 0) => d.wrapping_mul(s),
        (0x30, 0) => d.checked_div(s).unwrap_or(0),
        (0x30, 1) => {
            if ss == 0 {
                0
            } else {
                sd.wrapping_div(ss).cast_unsigned()
            }
        }
        (0x40, 0) => d | s,
        (0x50, 0) => d & s,
        (0x60, 0) => d.wrapping_shl(s),
        (0x70, 0) => d.wrapping_shr(s),
        (0x80, 0) => sd.wrapping_neg().cast_unsigned(),
        (0x90, 0) => d.checked_rem(s).unwrap_or(d),
        (0x90, 1) => {
            if ss == 0 {
                d
            } else {
                sd.wrapping_rem(ss).cast_unsigned()
            }
        }
        (0xa0, 0) => d ^ s,
        (0xb0, 0) => s,
        (0xb0, 8) => sign_extend(u64::from(s), 8) as u32,
        (0xb0, 16) => sign_extend(u64::from(s), 16) as u32,
        (0xc0, 0) => sd.wrapping_shr(s).cast_unsigned(),
        _ => return None,
    };
    Some(u64::from(r))
}

/// `BPF_END`: `to_be` swaps (on a little-endian model), `to_le` and the
/// width truncate; ALU64 `BSWAP` always swaps.
fn byte_order(swap: bool, width: i32, v: u64) -> Option<u64> {
    Some(match (width, swap) {
        (16, false) => v & 0xffff,
        (16, true) => u64::from((v as u16).swap_bytes()),
        (32, false) => v & 0xffff_ffff,
        (32, true) => u64::from((v as u32).swap_bytes()),
        (64, false) => v,
        (64, true) => v.swap_bytes(),
        _ => return None,
    })
}

fn jump_taken(op: u8, d: u64, s: u64, wide: bool) -> Option<bool> {
    let (d, s) = if wide {
        (d, s)
    } else {
        (u64::from(d as u32), u64::from(s as u32))
    };
    let (sd, ss) = if wide {
        (d.cast_signed(), s.cast_signed())
    } else {
        (
            i64::from((d as u32).cast_signed()),
            i64::from((s as u32).cast_signed()),
        )
    };
    Some(match op & 0xf0 {
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
        0xd0 => sd <= ss,
        _ => return None,
    })
}

/// Run `raw` (little-endian eBPF bytecode) once over `packet`, returning `r0`.
///
/// `helper(id, [r1..r5])` services `call`; `None` means "unknown helper".
///
/// # Errors
/// Malformed or unsupported instructions, out-of-bounds memory access,
/// jumps outside the program, unknown helpers, or an exhausted budget.
pub fn execute(
    raw: &[u8],
    packet: &mut [u8],
    helper: &mut dyn FnMut(i32, [u64; 5]) -> Option<u64>,
) -> Result<u64, String> {
    let prog = decode(raw)?;
    let pkt_end = PACKET_BASE + packet.len() as u64;
    let mut mem = Memory {
        mbuff: [0; MBUFF_LEN],
        packet,
        stack: [0; STACK_SIZE],
    };
    mem.mbuff[..8].copy_from_slice(&PACKET_BASE.to_le_bytes());
    mem.mbuff[8..].copy_from_slice(&pkt_end.to_le_bytes());

    let mut reg = [0u64; 11];
    reg[1] = MBUFF_BASE;
    reg[10] = STACK_BASE + STACK_SIZE as u64;

    let mut pc = 0usize;
    for _ in 0..INSN_BUDGET {
        let Some(&i) = prog.get(pc) else {
            return Err(format!("insn {pc}: execution fell off the program end"));
        };
        let bad = || {
            format!(
                "insn {pc}: unsupported opcode {:#04x} (off {})",
                i.op, i.off
            )
        };
        let src_val = if i.op & SRC_X != 0 {
            reg[i.src]
        } else {
            i64::from(i.imm).cast_unsigned()
        };
        let mut next = pc + 1;
        match i.op & 0x07 {
            CLS_ALU64 | CLS_ALU if i.op & 0xf0 == 0xd0 => {
                // BPF_END / BSWAP: the source bit selects to_be under ALU,
                // ALU64|K is the unconditional v4 BSWAP.
                let swap = i.op & 0x07 == CLS_ALU64 || i.op & SRC_X != 0;
                if i.op == (CLS_ALU64 | SRC_X | 0xd0) {
                    return Err(bad());
                }
                reg[i.dst] = byte_order(swap, i.imm, reg[i.dst]).ok_or_else(bad)?;
            }
            CLS_ALU64 => reg[i.dst] = alu64(i.op, i.off, reg[i.dst], src_val).ok_or_else(bad)?,
            CLS_ALU => reg[i.dst] = alu32(i.op, i.off, reg[i.dst], src_val).ok_or_else(bad)?,
            CLS_LD if i.op == (CLS_LD | MODE_IMM | 0x18) && i.src == 0 => {
                let hi = prog
                    .get(pc + 1)
                    .ok_or_else(|| format!("insn {pc}: truncated lddw"))?;
                reg[i.dst] =
                    u64::from(i.imm.cast_unsigned()) | (u64::from(hi.imm.cast_unsigned()) << 32);
                next = pc + 2;
            }
            CLS_LDX if i.op & 0xe0 == MODE_MEM => {
                let len = size_bytes(i.op);
                reg[i.dst] = mem.load(offset_addr(reg[i.src], i.off), len, pc)?;
            }
            CLS_LDX if i.op & 0xe0 == MODE_MEMSX && size_bytes(i.op) < 8 => {
                let len = size_bytes(i.op);
                let v = mem.load(offset_addr(reg[i.src], i.off), len, pc)?;
                reg[i.dst] = sign_extend(v, len as u32 * 8);
            }
            CLS_ST if i.op & 0xe0 == MODE_MEM => {
                let v = i64::from(i.imm).cast_unsigned();
                mem.store(offset_addr(reg[i.dst], i.off), size_bytes(i.op), v, pc)?;
            }
            CLS_STX if i.op & 0xe0 == MODE_MEM => {
                mem.store(
                    offset_addr(reg[i.dst], i.off),
                    size_bytes(i.op),
                    reg[i.src],
                    pc,
                )?;
            }
            cls @ (CLS_JMP | CLS_JMP32) => {
                let wide = cls == CLS_JMP;
                let rel = match i.op & 0xf0 {
                    0x00 if wide => Some(i32::from(i.off)),
                    0x00 => Some(i.imm),
                    0x80 if wide => {
                        let args = [reg[1], reg[2], reg[3], reg[4], reg[5]];
                        reg[0] = helper(i.imm, args)
                            .ok_or_else(|| format!("insn {pc}: unknown helper {}", i.imm))?;
                        None
                    }
                    0x90 if wide => return Ok(reg[0]),
                    _ => jump_taken(i.op, reg[i.dst], src_val, wide)
                        .ok_or_else(bad)?
                        .then_some(i32::from(i.off)),
                };
                if let Some(rel) = rel {
                    next = isize::try_from(next)
                        .ok()
                        .and_then(|n| n.checked_add(rel as isize))
                        .and_then(|n| usize::try_from(n).ok())
                        .filter(|&n| n < prog.len())
                        .ok_or_else(|| format!("insn {pc}: jump out of program"))?;
                }
            }
            _ => return Err(bad()),
        }
        pc = next;
    }
    Err(format!("instruction budget of {INSN_BUDGET} exhausted"))
}
