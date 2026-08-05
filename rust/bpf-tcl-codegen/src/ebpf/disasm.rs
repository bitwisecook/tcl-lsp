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

//! A small disassembler for emitted eBPF — used by `bpf-tcl compile --emit asm`
//! and for human inspection. (The compile path never depends on this; it is
//! output-only.)

use std::fmt::Write as _;

use crate::ebpf::insn::Insn;

/// Render instructions to a readable, one-per-line listing.
#[must_use]
pub fn disasm(insns: &[Insn]) -> String {
    let mut out = String::new();
    for (i, ins) in insns.iter().enumerate() {
        let _ = writeln!(out, "{i:4}: {}", one(*ins));
    }
    out
}

fn sz(op: u8) -> &'static str {
    match op & 0x18 {
        0x00 => "u32",
        0x08 => "u16",
        0x10 => "u8",
        _ => "u64",
    }
}

fn one(i: Insn) -> String {
    let class = i.op & 0x07;
    let d = i.dst;
    let s = i.src;
    match class {
        0x07 => {
            let src_reg = i.op & 0x08 != 0;
            let rhs = if src_reg {
                format!("r{s}")
            } else {
                i.imm.to_string()
            };
            match i.op & 0xf0 {
                0xb0 => format!("r{d} = {rhs}"),
                0x00 => format!("r{d} += {rhs}"),
                0x10 => format!("r{d} -= {rhs}"),
                0x20 => format!("r{d} *= {rhs}"),
                0x30 => format!("r{d} /= {rhs}"),
                0x40 => format!("r{d} |= {rhs}"),
                0x50 => format!("r{d} &= {rhs}"),
                0x60 => format!("r{d} <<= {rhs}"),
                0x70 => format!("r{d} >>= {rhs}"),
                0x80 => format!("r{d} = -r{d}"),
                0x90 => format!("r{d} %= {rhs}"),
                0xa0 => format!("r{d} ^= {rhs}"),
                _ => raw(i),
            }
        }
        0x00 if i.op == 0x18 => {
            // Wide-immediate load (`lddw`): a pseudo map-fd load carries the
            // map source register; its immediate is patched by a relocation.
            if i.src == crate::ebpf::insn::PSEUDO_MAP_FD {
                format!("r{d} = map_fd(reloc)")
            } else {
                format!("r{d} = lddw(imm={})", i.imm)
            }
        }
        0x00 if i.op == 0x00 => format!("<lddw high imm={}>", i.imm),
        0x01 => format!("r{d} = *({} *)(r{s} {:+})", sz(i.op), i.off),
        0x03 => format!("*({} *)(r{d} {:+}) = r{s}", sz(i.op), i.off),
        0x02 => format!("*({} *)(r{d} {:+}) = {}", sz(i.op), i.off, i.imm),
        0x05 => match i.op & 0xf0 {
            0x00 => format!("goto {:+}", i.off),
            0x90 => "exit".to_string(),
            0x80 => format!("call {}", i.imm),
            op => {
                let src_reg = i.op & 0x08 != 0;
                let rhs = if src_reg {
                    format!("r{s}")
                } else {
                    i.imm.to_string()
                };
                let cc = match op {
                    0x10 => "==",
                    0x50 => "!=",
                    0x60 => "s>",
                    0x70 => "s>=",
                    0xc0 => "s<",
                    0xd0 => "s<=",
                    _ => "?",
                };
                format!("if r{d} {cc} {rhs} goto {:+}", i.off)
            }
        },
        _ => raw(i),
    }
}

fn raw(i: Insn) -> String {
    format!(
        "op=0x{:02x} dst=r{} src=r{} off={} imm={}",
        i.op, i.dst, i.src, i.off, i.imm
    )
}
