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

//! Signed division and modulo follow kernel `BPF_SDIV` / `BPF_SMOD`
//! (truncation toward zero), not unsigned division and not Tcl floor division,
//! along with the other cpu-v4 offset-qualified instructions.

use std::process::Command;

use bpf_tcl::run::run_socket_filter;
use bpf_tcl::vm;
use bpf_tcl_codegen::ebpf::emit_program;
use bpf_tcl_codegen::ebpf::insn::{
    ALU, ALU64, DIV, Insn, K, MOD, MOV, R0, R1, X, exit, lddw, mov64_imm, mov64_reg,
};
use bpf_tcl_ir::compile_module;

const DIV_PROGRAM: &str = "when SOCKET_FILTER {
    setint x {0 - 7}
    setint q {$x / 2}
    setint r {$x % 2}
    accept {$q * 10 + $r}
}
";

fn run_src(src: &str) -> i64 {
    let module = compile_module(src).expect("compiles");
    let obj = emit_program(&module.programs[0].program).expect("emits");
    run_socket_filter(&obj, &mut [0u8])
        .expect("runs")
        .cast_signed()
}

fn run_insns(insns: &[Insn]) -> u64 {
    let raw: Vec<u8> = insns.iter().flat_map(|i| i.to_le_bytes()).collect();
    vm::execute(&raw, &mut [], &mut |_, _| None).expect("runs")
}

fn op(op: u8, dst: u8, src: u8, off: i16, imm: i32) -> Insn {
    Insn {
        op,
        dst,
        src,
        off,
        imm,
    }
}

#[test]
fn cli_run_matches_kernel_signed_division() {
    let path = std::env::temp_dir().join(format!("bpf-tcl-sdiv-{}.bpftcl", std::process::id()));
    std::fs::write(&path, DIV_PROGRAM).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_bpf-tcl"))
        .arg("run")
        .arg(&path)
        .args(["--packet", "00"])
        .output()
        .unwrap();
    let _ = std::fs::remove_file(&path);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "run failed: {out:?}");
    let expected = format!("(r0={})", (-31i64).cast_unsigned());
    assert!(stdout.contains(&expected), "want q=-3 r=-1 (-31): {stdout}");
}

#[test]
fn signed_division_truncates_toward_zero() {
    assert_eq!(run_src(DIV_PROGRAM), -31);
    let cases = [
        ("7", "2", 3, 1),
        ("0 - 7", "2", -3, -1),
        ("7", "0 - 2", -3, 1),
        ("0 - 7", "0 - 2", 3, -1),
    ];
    for (a, b, q, r) in cases {
        let src = format!(
            "when SOCKET_FILTER {{ setint a {{{a}}}\n setint b {{{b}}}\n accept {{($a / $b) * 100 + $a % $b}} }}\n"
        );
        assert_eq!(run_src(&src), q * 100 + r, "{a} / {b}");
    }
}

#[test]
fn signed_division_by_zero_and_overflow() {
    // Kernel: x / 0 = 0, x % 0 = x, INT_MIN / -1 = INT_MIN, INT_MIN % -1 = 0.
    let [lo, hi] = lddw(R0, i64::MIN);
    let sdiv = |dst, src| op(ALU64 | X | DIV, dst, src, 1, 0);
    let smod = |dst, src| op(ALU64 | X | MOD, dst, src, 1, 0);
    let p = [mov64_imm(R0, -7), mov64_imm(R1, 0), sdiv(R0, R1), exit()];
    assert_eq!(run_insns(&p), 0);
    let p = [mov64_imm(R0, -7), mov64_imm(R1, 0), smod(R0, R1), exit()];
    assert_eq!(run_insns(&p).cast_signed(), -7);
    let p = [lo, hi, mov64_imm(R1, -1), sdiv(R0, R1), exit()];
    assert_eq!(run_insns(&p).cast_signed(), i64::MIN);
    let p = [lo, hi, mov64_imm(R1, -1), smod(R0, R1), exit()];
    assert_eq!(run_insns(&p), 0);
}

#[test]
fn alu32_signed_division_zero_extends() {
    // w0 = -7 s/ 2 = -3, s% 2 = -1, both as 32-bit values zero-extended.
    let p = [mov64_imm(R0, -7), op(ALU | K | DIV, R0, 0, 1, 2), exit()];
    assert_eq!(run_insns(&p), u64::from((-3i32).cast_unsigned()));
    let p = [
        mov64_imm(R0, -7),
        mov64_imm(R1, 2),
        op(ALU | X | MOD, R0, R1, 1, 0),
        exit(),
    ];
    assert_eq!(run_insns(&p), u64::from((-1i32).cast_unsigned()));
    // Unsigned 32-bit divide ignores the upper half.
    let p = [mov64_imm(R0, -8), op(ALU | K | DIV, R0, 0, 0, 2), exit()];
    assert_eq!(run_insns(&p), 0x7fff_fffc);
}

#[test]
fn movsx_sign_extends() {
    let p = [
        mov64_imm(R1, 0x80),
        op(ALU64 | X | MOV, R0, R1, 8, 0),
        exit(),
    ];
    assert_eq!(run_insns(&p).cast_signed(), -128);
    let p = [
        mov64_imm(R1, 0x8000),
        op(ALU | X | MOV, R0, R1, 16, 0),
        exit(),
    ];
    assert_eq!(run_insns(&p), 0xffff_8000);
    let p = [mov64_imm(R1, 0x80), mov64_reg(R0, R1), exit()];
    assert_eq!(run_insns(&p), 0x80);
}

#[test]
fn sign_extending_load() {
    // *(s8 *)(r10 - 1) after storing 0xff yields -1.
    let p = [
        op(0x72, 10, 0, -1, 0xff), // ST MEM B
        op(0x91, R0, 10, -1, 0),   // LDX MEMSX B
        exit(),
    ];
    assert_eq!(run_insns(&p).cast_signed(), -1);
}

#[test]
fn invalid_offset_is_rejected() {
    let raw: Vec<u8> = [mov64_imm(R0, 1), op(ALU64 | X | DIV, R0, R1, 2, 0), exit()]
        .iter()
        .flat_map(|i| i.to_le_bytes())
        .collect();
    let err = vm::execute(&raw, &mut [], &mut |_, _| None).unwrap_err();
    assert!(err.contains("unsupported opcode"), "{err}");
}
