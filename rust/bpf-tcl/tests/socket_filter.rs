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

//! End-to-end: a `.bpftcl` source → typed IR → eBPF bytes → executed under the
//! userspace `rbpf` VM over a synthetic packet → asserted verdict.

use bpf_tcl::run::run_socket_filter;
use bpf_tcl_codegen::ebpf::emit_program;
use bpf_tcl_ir::compile_module;

fn run_src(src: &str, packet: &mut [u8]) -> u64 {
    let module = compile_module(src).expect("source should compile");
    let obj = emit_program(&module.programs[0].program).expect("program should emit");
    run_socket_filter(&obj, packet).expect("program should run")
}

#[test]
fn accept_all_returns_packet_len() {
    let mut pkt = vec![0u8; 64];
    assert_eq!(
        run_src("when SOCKET_FILTER { setbuf pkt ctx\n accept }\n", &mut pkt),
        64
    );
}

#[test]
fn drop_all_returns_zero() {
    let mut pkt = vec![0u8; 64];
    assert_eq!(run_src("drop\n", &mut pkt), 0);
}

#[test]
fn arithmetic_verdict() {
    // `accept {3 * 4 + 2}` → 14 (exercises Const/Bin/verdict end-to-end).
    let mut pkt = vec![0u8; 8];
    assert_eq!(
        run_src("when SOCKET_FILTER { accept {3 * 4 + 2} }\n", &mut pkt),
        14
    );
}

#[test]
fn signed_shift_and_32bit_widths() {
    let mut pkt = vec![0u8; 8];
    // `>>` is arithmetic (sign-preserving): -8 >> 1 == -4 (RUST_ISSUE_062/097).
    assert_eq!(
        run_src(
            "when SOCKET_FILTER { setint x {0 - 8}\n accept {$x >> 1} }\n",
            &mut pkt
        ),
        (-4i64).cast_unsigned()
    );
    // seti32 sign-extends the low 32 bits: (1<<31) → i32::MIN (RUST_ISSUE_172).
    assert_eq!(
        run_src(
            "when SOCKET_FILTER { seti32 x {1 << 31}\n accept {$x} }\n",
            &mut pkt
        ),
        i64::from(i32::MIN).cast_unsigned()
    );
    // setu32 zero-extends: (1<<31) stays 0x8000_0000 (RUST_ISSUE_172).
    assert_eq!(
        run_src(
            "when SOCKET_FILTER { setu32 x {1 << 31}\n accept {$x} }\n",
            &mut pkt
        ),
        0x8000_0000
    );
}

const BLOCK_SSH: &str = "when SOCKET_FILTER priority 50 {\n\
    setbuf pkt ctx\n\
    pktlen len pkt\n\
    if {$len < 38} { drop }\n\
    load16 dport pkt 36\n\
    if {$dport == 22} { drop }\n\
    accept\n}\n";

#[test]
fn block_ssh_drops_port_22() {
    let mut pkt = vec![0u8; 40];
    // native-endian u16 at offset 36 == 22  →  bytes 0x16 0x00
    pkt[36] = 0x16;
    pkt[37] = 0x00;
    assert_eq!(run_src(BLOCK_SSH, &mut pkt), 0);
}

#[test]
fn block_ssh_accepts_port_80() {
    let mut pkt = vec![0u8; 40];
    pkt[36] = 0x50; // 80
    pkt[37] = 0x00;
    assert_eq!(run_src(BLOCK_SSH, &mut pkt), 40);
}

#[test]
fn block_ssh_drops_short_packet() {
    // len (20) < 38  → drop before the load.
    let mut pkt = vec![0u8; 20];
    assert_eq!(run_src(BLOCK_SSH, &mut pkt), 0);
}

#[test]
fn example_programs_compile_and_run() {
    // The shipped example files compile and run.
    let accept = include_str!("progs/accept-all.bpftcl");
    let drop = include_str!("progs/drop-all.bpftcl");
    let mut pkt = vec![0u8; 100];
    assert_eq!(run_src(accept, &mut pkt), 100);
    assert_eq!(run_src(drop, &mut pkt), 0);
}
