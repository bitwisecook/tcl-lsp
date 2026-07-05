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

//! Bounded loops: `loop N VAR { body }` unrolls and runs under rbpf.

use bpf_tcl::run::run_socket_filter_repeated;
use bpf_tcl_codegen::ebpf::emit_program;
use bpf_tcl_ir::{BpfDiag, compile_module};

fn run(src: &str) -> (u64, Vec<std::collections::HashMap<u64, u64>>) {
    let module = compile_module(src).expect("compiles");
    let obj = emit_program(&module.programs[0].program).expect("emits");
    let mut pkt = vec![0u8; 16];
    run_socket_filter_repeated(&obj, &mut pkt, 1).expect("runs")
}

#[test]
fn loop_sums_map_values() {
    // Pre-populate four map slots, then loop to sum them.
    let (verdict, _) = run("when SOCKET_FILTER {\n\
         map m hash 8 8 16\n\
         map_set m {0} {10}\n\
         map_set m {1} {20}\n\
         map_set m {2} {30}\n\
         map_set m {3} {40}\n\
         setint sum {0}\n\
         loop 4 i {\n\
             map_get v m {$i}\n\
             setint sum {$sum + $v}\n\
         }\n\
         accept {$sum}\n}\n");
    assert_eq!(verdict, 100); // 10 + 20 + 30 + 40
}

#[test]
fn loop_populates_map() {
    // m[i] = i*i for i in 0..5.
    let (_, maps) = run("when SOCKET_FILTER {\n\
         map sq hash 8 8 16\n\
         loop 5 i {\n\
             setint v {$i * $i}\n\
             map_set sq {$i} {$v}\n\
         }\n\
         accept\n}\n");
    assert_eq!(maps[0].get(&0).copied(), Some(0));
    assert_eq!(maps[0].get(&3).copied(), Some(9));
    assert_eq!(maps[0].get(&4).copied(), Some(16));
}

#[test]
fn nested_loops_unroll() {
    // m[i*10 + j] = 1 for i in 0..2, j in 0..3 → six entries.
    let (_, maps) = run("when SOCKET_FILTER {\n\
         map m hash 8 8 16\n\
         loop 2 i {\n\
             loop 3 j {\n\
                 setint k {$i * 10 + $j}\n\
                 map_set m {$k} {1}\n\
             }\n\
         }\n\
         accept\n}\n");
    assert_eq!(maps[0].len(), 6);
    assert_eq!(maps[0].get(&0).copied(), Some(1)); // i=0, j=0
    assert_eq!(maps[0].get(&12).copied(), Some(1)); // i=1, j=2
}

#[test]
fn over_cap_loop_is_rejected() {
    let err = compile_module("when SOCKET_FILTER { loop 1000 i { drop } }\n").unwrap_err();
    assert_eq!(err.code, BpfDiag::UnboundedLoop);
}

#[test]
fn non_integer_loop_count_is_rejected() {
    let err = compile_module("when SOCKET_FILTER { loop abc i { drop } }\n").unwrap_err();
    assert_eq!(err.code, BpfDiag::BadInt);
}

#[test]
fn native_while_still_rejected() {
    // `loop` is the only supported loop; native `while` is still a back-edge.
    let err = compile_module(
        "when SOCKET_FILTER { seti32 n {0}\n while {1} { seti32 n {1} }\n accept }\n",
    )
    .unwrap_err();
    assert_eq!(err.code, BpfDiag::UnboundedLoop);
}
