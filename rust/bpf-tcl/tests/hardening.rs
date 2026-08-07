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

//! Issue #1202 hardening: strict framework grammar, proof-bearing packet
//! reads, typed maps with capacity and missing-vs-zero, full-width constants,
//! and the liveness-based slot allocator. TP/FP/TN/FN coverage: each malformed
//! input is rejected (true positive), each well-formed counterpart still
//! compiles and runs (true negative / false-positive guard).

use std::fmt::Write as _;

use bpf_tcl::run::{run_socket_filter, run_socket_filter_repeated};
use bpf_tcl_codegen::ebpf::emit_program;
use bpf_tcl_ir::{BpfDiag, MapKind, compile_module};

fn run(src: &str, packet: &mut [u8]) -> u64 {
    let module = compile_module(src).expect("compiles");
    let obj = emit_program(&module.programs[0].program).expect("emits");
    run_socket_filter(&obj, packet).expect("runs")
}

// ── strict `when` header grammar ───────────────────────────

#[test]
fn when_non_integer_priority_is_rejected_not_silently_500() {
    // TP: `priority nope` used to become priority 500 with no diagnostic.
    let err = compile_module("when XDP priority nope { pass }\n").unwrap_err();
    assert_eq!(err.code, BpfDiag::BadArity);
    assert!(err.msg.contains("priority"), "{}", err.msg);
}

#[test]
fn when_unknown_header_keyword_is_rejected() {
    // TP: an extra header word is not silently ignored.
    let err = compile_module("when XDP badword 3 { pass }\n").unwrap_err();
    assert_eq!(err.code, BpfDiag::BadArity);
}

#[test]
fn when_substituted_event_is_rejected() {
    let err = compile_module("when $ev { pass }\n").unwrap_err();
    assert_eq!(err.code, BpfDiag::BadEvent);
}

#[test]
fn when_well_formed_priority_still_compiles() {
    // TN: the supported form is untouched.
    let module = compile_module("when XDP priority 10 { pass }\n").expect("compiles");
    assert_eq!(module.programs[0].priority, 10);
}

// ── verdict-less handlers are an error, never a silent drop ─────────────────

#[test]
fn missing_verdict_on_a_path_is_rejected() {
    // TP: the `if` true-branch drops, but the fall-through has no verdict.
    let err = compile_module("when XDP { setint x 1\n if {$x == 1} { drop } }\n").unwrap_err();
    assert_eq!(err.code, BpfDiag::MissingVerdict);
}

#[test]
fn every_path_terminated_compiles() {
    // TN: both branches carry a verdict.
    compile_module("when XDP { setint x 1\n if {$x == 1} { drop } else { pass } }\n")
        .expect("compiles");
}

// ── full-width constants ───────────────────────────────────

#[test]
fn full_width_64bit_constant_materialises() {
    // A constant beyond 32 bits used to be rejected; now it uses `lddw`.
    let mut pkt = vec![0u8; 8];
    let v = run(
        "when SOCKET_FILTER { setint x {0x1122334455}\n accept {$x} }\n",
        &mut pkt,
    );
    assert_eq!(v, 0x11_2233_4455);
}

// ── network-order packet reads ──────────────────────────────────────────────

#[test]
fn big_endian_load_reads_network_order() {
    // `load16 … be` reads a network-order field: bytes 0x00 0x16 → 22.
    let mut pkt = vec![0u8; 40];
    pkt[36] = 0x00;
    pkt[37] = 0x16;
    let src = "when SOCKET_FILTER {\n\
        setbuf pkt ctx\n\
        pktlen len pkt\n\
        if {$len < 38} { drop }\n\
        load16 dport pkt 36 be\n\
        if {$dport == 22} { drop }\n\
        accept\n}\n";
    assert_eq!(run(src, &mut pkt), 0); // dropped: dport == 22
}

#[test]
fn short_packet_takes_the_out_of_bounds_verdict() {
    // The checked load proves `off + width <= data_end`; a short packet takes
    // the drop verdict instead of trapping.
    let mut pkt = vec![0u8; 4]; // far too short for a load at offset 36
    let src = "when SOCKET_FILTER {\n\
        setbuf pkt ctx\n\
        load16 dport pkt 36 be\n\
        accept\n}\n";
    assert_eq!(run(src, &mut pkt), 0); // drop (OOB)
}

#[test]
fn unknown_byte_order_word_is_rejected() {
    let err =
        compile_module("when SOCKET_FILTER { setbuf p ctx\n load16 d p 0 middle\n accept }\n")
            .unwrap_err();
    assert_eq!(err.code, BpfDiag::OutOfSubset);
}

// ── typed maps: kinds, capacity, missing vs zero ────────────────────────────

#[test]
fn map_kinds_are_type_checked() {
    // TP: an unknown kind is rejected.
    let err = compile_module("when SOCKET_FILTER { map m tree 8 8 4\n accept }\n").unwrap_err();
    assert_eq!(err.code, BpfDiag::BadMap);
    // TN: hash and array both parse.
    let m =
        compile_module("when SOCKET_FILTER { map h hash 8 8 4\n map a array 4 8 4\n accept }\n")
            .expect("compiles");
    let maps = &m.programs[0].program.maps;
    assert_eq!(maps[0].kind, MapKind::Hash);
    assert_eq!(maps[1].kind, MapKind::Array);
}

#[test]
fn array_map_requires_a_four_byte_key() {
    let err = compile_module("when SOCKET_FILTER { map a array 8 8 4\n accept }\n").unwrap_err();
    assert_eq!(err.code, BpfDiag::BadMap);
}

#[test]
fn odd_value_size_is_rejected() {
    let err = compile_module("when SOCKET_FILTER { map m hash 8 3 4\n accept }\n").unwrap_err();
    assert_eq!(err.code, BpfDiag::BadMap);
}

#[test]
fn map_has_distinguishes_missing_from_stored_zero() {
    // Store a zero under key 7; `map_has` still reports it present, while a
    // never-written key 9 reads absent.
    let src = "when SOCKET_FILTER {\n\
        map m hash 8 8 8\n\
        map_set m {7} {0}\n\
        map_has present m {7}\n\
        map_has absent m {9}\n\
        setint r {$present * 10 + $absent}\n\
        accept {$r}\n}\n";
    let mut pkt = vec![0u8; 8];
    // present=1, absent=0  →  10
    assert_eq!(run(src, &mut pkt), 10);
}

#[test]
fn hash_map_capacity_is_enforced() {
    // A hash map with room for one entry: the second distinct key's update is
    // dropped, so only the first key persists.
    let src = "when SOCKET_FILTER {\n\
        map m hash 8 8 1\n\
        map_set m {1} {100}\n\
        map_set m {2} {200}\n\
        accept\n}\n";
    let module = compile_module(src).expect("compiles");
    let obj = emit_program(&module.programs[0].program).expect("emits");
    let mut pkt = vec![0u8; 8];
    let (_, maps) = run_socket_filter_repeated(&obj, &mut pkt, 1).expect("runs");
    assert_eq!(maps[0].get(&1).copied(), Some(100));
    assert_eq!(maps[0].get(&2), None, "over-capacity key must be dropped");
}

#[test]
fn array_map_is_zero_filled_and_present() {
    // An array element is present (value 0) before any store.
    let src = "when SOCKET_FILTER {\n\
        map a array 4 8 4\n\
        map_has p a {2}\n\
        accept {$p}\n}\n";
    let mut pkt = vec![0u8; 8];
    assert_eq!(run(src, &mut pkt), 1);
}

// ── liveness-based slot allocation ──────────────────────────────────────────

#[test]
fn slot_reuse_shrinks_the_stack() {
    // Many short-lived temporaries in sequence: with liveness-based reuse the
    // allocated slot count is far below the number of values computed.
    let mut body = String::from("when SOCKET_FILTER {\n");
    for i in 0..40 {
        let _ = writeln!(body, "setint t{i} {{ {i} + 1 }}");
    }
    body.push_str("accept\n}\n");
    let module = compile_module(&body).expect("compiles");
    let prog = &module.programs[0].program;
    assert!(
        prog.raw_slot_count > prog.num_slots,
        "reuse should shrink slots: raw {} allocated {}",
        prog.raw_slot_count,
        prog.num_slots
    );
    assert!(
        prog.num_slots <= 64,
        "allocated slots must fit the stack: {}",
        prog.num_slots
    );
}

#[test]
fn stack_pressure_is_reported_with_source_context() {
    // A handler that keeps 200 values simultaneously live (each used in the
    // final sum) cannot fit the 64-slot stack even after reuse.
    let mut body = String::from("when SOCKET_FILTER {\n");
    for i in 0..200 {
        let _ = writeln!(body, "setint v{i} {i}");
    }
    body.push_str("setint acc 0\n");
    for i in 0..200 {
        let _ = writeln!(body, "setint acc {{ $acc + $v{i} }}");
    }
    body.push_str("accept {$acc}\n}\n");
    let err = compile_module(&body).unwrap_err();
    assert_eq!(err.code, BpfDiag::StackOverflow);
    assert!(
        err.msg.contains("reuse"),
        "message mentions reuse: {}",
        err.msg
    );
}
