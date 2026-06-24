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

#[allow(clippy::too_many_lines)]
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
        0x01 => format!("r{d} = *({} *)(r{s} {:+})", sz(i.op), i.off),
        0x03 => format!("*({} *)(r{d} {:+}) = r{s}", sz(i.op), i.off),
        0x02 => format!("*({} *)(r{d} {:+}) = {}", sz(i.op), i.off, i.imm),
        0x05 => match i.op & 0xf0 {
            0x00 => format!("goto {:+}", i.off),
            0x90 => "exit".to_string(),
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
