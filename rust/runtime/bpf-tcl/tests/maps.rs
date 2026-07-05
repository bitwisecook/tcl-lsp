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

//! Maps end-to-end: a program that counts packets in a map, run under rbpf,
//! accumulates across invocations.

use bpf_tcl::run::run_socket_filter_repeated;
use bpf_tcl_codegen::ebpf::emit_program;
use bpf_tcl_ir::{BpfDiag, compile_module};

const COUNTER: &str = "when SOCKET_FILTER {\n\
    map hits hash 8 8 16\n\
    setint key 0\n\
    map_get n hits {$key}\n\
    setint n1 {$n + 1}\n\
    map_set hits {$key} {$n1}\n\
    accept\n}\n";

#[test]
fn map_counter_accumulates() {
    let module = compile_module(COUNTER).expect("counter should compile");
    let program = &module.programs[0].program;
    assert_eq!(program.maps.len(), 1, "one map declared");
    assert_eq!(program.maps[0].name, "hits");

    let obj = emit_program(program).expect("counter should emit");
    assert_eq!(obj.maps.len(), 1);

    let mut pkt = vec![0u8; 32];
    let (verdict, maps) =
        run_socket_filter_repeated(&obj, &mut pkt, 3).expect("counter should run");

    // Each run accepts the whole packet and increments hits[0].
    assert_eq!(verdict, 32);
    assert_eq!(
        maps[0].get(&0).copied(),
        Some(3),
        "hits[0] == 3 after 3 runs"
    );
}

#[test]
fn map_set_then_get_roundtrips() {
    // Store a value under a key, then read it back into the verdict.
    let src = "when SOCKET_FILTER {\n\
        map m hash 8 8 8\n\
        map_set m {7} {99}\n\
        map_get v m {7}\n\
        accept {$v}\n}\n";
    let module = compile_module(src).expect("compile");
    let obj = emit_program(&module.programs[0].program).expect("emit");
    let mut pkt = vec![0u8; 8];
    let (verdict, _) = run_socket_filter_repeated(&obj, &mut pkt, 1).expect("run");
    assert_eq!(verdict, 99);
}

#[test]
fn unknown_map_is_rejected() {
    let err = compile_module("when SOCKET_FILTER { map_get v nope {0}\n accept }\n").unwrap_err();
    assert_eq!(err.code, BpfDiag::UndefinedVar);
}
