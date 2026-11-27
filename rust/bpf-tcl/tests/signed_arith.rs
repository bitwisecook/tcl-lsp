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

//! `bpf-tcl run` agrees with a cpu-v4 kernel: `/` and `%` are `BPF_SDIV` /
//! `BPF_SMOD` (truncating), and the VM honours the other offset-qualified v4
//! forms (32-bit signed div/mod, MOVSX, MEMSX, BSWAP, gotol).

use std::collections::HashMap;
use std::process::Command;

use bpf_tcl::run::run_socket_filter;
use bpf_tcl::vm::{Helper, Program};
use bpf_tcl_codegen::ebpf::{Insn, emit_program};
use bpf_tcl_ir::compile_module;

fn run_src(src: &str) -> i64 {
    let module = compile_module(src).expect("compiles");
    let obj = emit_program(&module.programs[0].program).expect("emits");
    run_socket_filter(&obj, &mut [0u8])
        .expect("runs")
        .cast_signed()
}

#[test]
fn cli_run_truncates_negative_division() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/progs/signed-div.bpftcl");
    let out = Command::new(env!("CARGO_BIN_EXE_bpf-tcl"))
        .args(["run", path, "--packet", "00"])
        .output()
        .expect("bpf-tcl runs");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let r0 = (-31i64).cast_unsigned();
    assert!(stdout.contains(&format!("(r0={r0})")), "{stdout}");
}

#[test]
fn library_run_truncates_negative_division() {
    assert_eq!(run_src(include_str!("progs/signed-div.bpftcl")), -31);
}

#[test]
fn division_and_modulo_follow_operand_signs() {
    for (a, b, q, r) in [
        (7, 2, 3, 1),
        (-7, 2, -3, -1),
        (7, -2, -3, 1),
        (-7, -2, 3, -1),
    ] {
        let src = format!(
            "when SOCKET_FILTER {{ setint a {{0 + {a}}}\n setint b {{0 + {b}}}\n \
             setint q {{$a / $b}}\n setint r {{$a % $b}}\n accept {{$q * 100 + $r}} }}\n"
        );
        assert_eq!(run_src(&src), q * 100 + r, "{a} / {b}");
    }
}

const R0: u8 = 0;
const R1: u8 = 1;
const R10: u8 = 10;

fn ins(op: u8, dst: u8, src: u8, off: i16, imm: i32) -> Insn {
    Insn {
        op,
        dst,
        src,
        off,
        imm,
    }
}

fn lddw(dst: u8, v: i64) -> [Insn; 2] {
    let raw = v.to_le_bytes();
    let (lo, hi) = raw.split_at(4);
    let word = |half: &[u8]| i32::from_le_bytes(half.try_into().unwrap());
    [ins(0x18, dst, 0, 0, word(lo)), ins(0, 0, 0, 0, word(hi))]
}

fn exit() -> Insn {
    ins(0x95, 0, 0, 0, 0)
}

fn bytes(insns: &[Insn]) -> Vec<u8> {
    insns.iter().flat_map(|i| i.to_le_bytes()).collect()
}

fn exec(insns: &[Insn]) -> Result<u64, String> {
    exec_on(insns, &mut [])
}

fn exec_on(insns: &[Insn], packet: &mut [u8]) -> Result<u64, String> {
    let helpers: HashMap<i32, Helper> = HashMap::new();
    Program::new(&bytes(insns))?.execute(packet, &helpers)
}

/// `r0 = a; r1 = b; r0 <op> r1; exit`
fn binop(op: u8, off: i16, a: i64, b: i64) -> u64 {
    let mut p = lddw(R0, a).to_vec();
    p.extend(lddw(R1, b));
    p.push(ins(op, R0, R1, off, 0));
    p.push(exit());
    exec(&p).expect("runs")
}

#[test]
fn sdiv64_smod64_edge_cases() {
    assert_eq!(binop(0x3f, 1, -7, 2).cast_signed(), -3);
    assert_eq!(binop(0x9f, 1, -7, 2).cast_signed(), -1);
    assert_eq!(binop(0x3f, 0, -7, 2), (-7i64).cast_unsigned() / 2);
    assert_eq!(binop(0x3f, 1, i64::MIN, -1).cast_signed(), i64::MIN);
    assert_eq!(binop(0x9f, 1, i64::MIN, -1), 0);
    assert_eq!(binop(0x3f, 1, -7, 0), 0);
    assert_eq!(binop(0x9f, 1, -7, 0).cast_signed(), -7);
}

#[test]
fn sdiv64_immediate_operand() {
    let mut p = lddw(R0, -9).to_vec();
    p.push(ins(0x37, R0, 0, 1, -2));
    p.push(exit());
    assert_eq!(exec(&p).unwrap().cast_signed(), 4);
}

#[test]
fn sdiv32_smod32_zero_extend() {
    assert_eq!(binop(0x3c, 1, -7, 2), u64::from((-3i32).cast_unsigned()));
    assert_eq!(binop(0x9c, 1, -7, 2), u64::from((-1i32).cast_unsigned()));
    assert_eq!(binop(0x9c, 1, 0x1_ffff_fff9, 0), 0xffff_fff9);
}

#[test]
fn movsx_sign_extends() {
    assert_eq!(binop(0xbf, 8, 0, 0x80).cast_signed(), -128);
    assert_eq!(binop(0xbf, 16, 0, 0x8000).cast_signed(), -32768);
    assert_eq!(
        binop(0xbf, 32, 0, 0x8000_0000).cast_signed(),
        i64::from(i32::MIN)
    );
    assert_eq!(binop(0xbc, 8, 0, 0xff), 0xffff_ffff);
}

#[test]
fn memsx_loads_sign_extend_from_packet() {
    let mut pkt = [0xfeu8, 0xff, 0xff, 0x7f];
    // r2 = *(u64 *)(r1 + 0) (packet start); r0 = *(s8 *)(r2 + 0)
    let prog = [ins(0x79, 2, R1, 0, 0), ins(0x91, R0, 2, 0, 0), exit()];
    assert_eq!(exec_on(&prog, &mut pkt).unwrap().cast_signed(), -2);
    let prog = [ins(0x79, 2, R1, 0, 0), ins(0x81, R0, 2, 0, 0), exit()];
    assert_eq!(exec_on(&prog, &mut pkt).unwrap(), 0x7fff_fffe);
}

#[test]
fn bswap_is_unconditional() {
    let mut p = lddw(R0, 0x0102_0304_0506_0708).to_vec();
    p.push(ins(0xd7, R0, 0, 0, 16));
    p.push(exit());
    assert_eq!(exec(&p).unwrap(), 0x0807);
}

#[test]
fn gotol_jumps_by_immediate() {
    let prog = [
        ins(0xb7, R0, 0, 0, 1),
        ins(0x06, 0, 0, 0, 1),
        ins(0xb7, R0, 0, 0, 2),
        exit(),
    ];
    assert_eq!(exec(&prog).unwrap(), 1);
}

#[test]
fn stack_round_trip() {
    let prog = [ins(0x7a, R10, 0, -8, -5), ins(0x79, R0, R10, -8, 0), exit()];
    assert_eq!(exec(&prog).unwrap().cast_signed(), -5);
}

#[test]
fn rejects_malformed_programs() {
    assert!(exec(&[ins(0xbf, R0, R1, 0, 0)]).is_err(), "no exit");
    assert!(
        exec(&[ins(0x3f, R0, R1, 2, 0), exit()]).is_err(),
        "bad div offset"
    );
    assert!(
        exec(&[ins(0x37, R0, 0, 1, 0), exit()]).is_err(),
        "div by zero imm"
    );
    assert!(
        exec(&[ins(0xb7, R10, 0, 0, 0), exit()]).is_err(),
        "write r10"
    );
    assert!(
        exec(&[ins(0x05, 0, 0, 5, 0), exit()]).is_err(),
        "jump out of range"
    );
}

#[test]
fn out_of_bounds_access_is_an_error() {
    let prog = [ins(0x79, 2, R1, 0, 0), ins(0x61, R0, 2, 0, 0), exit()];
    let err = exec_on(&prog, &mut [0u8; 2]).unwrap_err();
    assert!(err.contains("out-of-bounds"), "{err}");
}
