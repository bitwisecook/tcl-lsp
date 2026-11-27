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

//! Compiled control and expression commands in procedure bodies, against the tclsh and jimsh captures.

use crate::prelude::*;

fn decode(hex: &str) -> Vec<u8> {
    crate::fixture::unhex(hex)
}

fn interpreter(version: &str) -> Vm {
    crate::fixture::interpreter(tcl_vm::test_support::profile(match version {
        "8.4.20" => "tcl8.4",
        "8.5.19" => "tcl8.5",
        "8.6.18" => "tcl8.6",
        "9.0.4" => "tcl9.0",
        "9.1.0" => "tcl9.1",
        "jim" => "jim",
        _ => panic!("native version"),
    }))
}

/// `proc p {x} body` from object words.
fn define(vm: &mut Vm, params: &[u8], body: &[u8]) {
    let words = [b"p".as_slice(), params, body].map(Value::new_native_string_bytes);
    assert_eq!(crate::fixture::call(vm, "proc", &words).code, Code::Ok);
}

fn run_outputs(table: &str, expected: usize) {
    let mut compared = 0;
    for row in table.lines() {
        let f = tcl_test_support::fixture::tsv_fields(row).collect::<Vec<_>>();
        let mut vm = interpreter(f[0]);
        define(&mut vm, b"x", &decode(f[2]));
        let args = [Value::new_native_string_bytes(f[5].as_bytes())];
        let completion = crate::fixture::call(&mut vm, "p", &args);
        assert_eq!(
            completion.code,
            Code::from_int(f[3].parse().unwrap()),
            "{}/{}: {completion:?}",
            f[0],
            f[1]
        );
        assert_eq!(
            crate::fixture::bytes(&vm, &completion.result),
            decode(f[4]),
            "{}/{}",
            f[0],
            f[1]
        );
        compared += 1;
    }
    assert_eq!(compared, expected);
}

#[test]
fn registered_control_instructions_match_75_native_controls() {
    run_outputs(
        include_str!("../../../../tcl-registry/tests/data/native_control_expression/control75.tsv"),
        75,
    );
}

#[test]
fn compound_expression_programs_match_40_native_controls() {
    run_outputs(
        include_str!(
            "../../../../tcl-registry/tests/data/native_control_expression/compound40.tsv"
        ),
        40,
    );
}

/// Each row's completion code for `p 3`.
fn run_expression_codes(table: &str, expected_count: usize) {
    let mut compared = 0;
    for row in table.lines() {
        let f = tcl_test_support::fixture::tsv_fields(row).collect::<Vec<_>>();
        let mut vm = interpreter(f[0]);
        define(&mut vm, b"x", &decode(f[2]));
        assert_eq!(f[3], "0", "native definition");
        let completion = crate::fixture::call(&mut vm, "p", &[Value::string("3")]);
        assert_eq!(
            completion.code,
            Code::from_int(f[4].parse().unwrap()),
            "{}/{}",
            f[0],
            f[1]
        );
        compared += 1;
    }
    assert_eq!(compared, expected_count);
}

#[test]
fn compiled_expression_storage_matches_40_original_native_windows() {
    run_expression_codes(
        include_str!("../../../../tcl-registry/tests/data/native_control_expression/storage40.tsv"),
        40,
    );
}

#[test]
fn folded_logical_literal_ownership_matches_eight_native_collisions() {
    run_expression_codes(
        include_str!(
            "../../../../tcl-registry/tests/data/native_control_expression/folded-boolean8.tsv"
        ),
        8,
    );
}

#[test]
fn c84_logical_storage_and_short_circuit_match_eight_native_controls() {
    run_expression_codes(
        include_str!("../../../../tcl-registry/tests/data/native_control_expression/logical84.tsv"),
        8,
    );
}

#[test]
fn compiled_catch_store_order_matches_four_native_alias_controls() {
    let mut compared = 0;
    for row in
        include_str!("../../../../tcl-registry/tests/data/native_control_expression/catch4.tsv")
            .lines()
    {
        let fields = tcl_test_support::fixture::tsv_fields(row).collect::<Vec<_>>();
        let mut vm = interpreter(fields[0]);
        define(&mut vm, b"", &decode(fields[1]));
        let completion = crate::fixture::call(&mut vm, "p", &[]);
        assert_eq!(
            completion.code,
            Code::from_int(fields[2].parse().unwrap()),
            "{}: {completion:?}",
            fields[0]
        );
        assert_eq!(
            crate::fixture::bytes(&vm, &completion.result),
            decode(fields[3]),
            "{}",
            fields[0]
        );
        compared += 1;
    }
    assert_eq!(compared, 4);
}

#[test]
fn catch_publication_and_store_order_match_six_native_engines() {
    let mut compared = 0;
    for row in include_str!(
        "../../../../tcl-registry/tests/data/native_control_expression/catch-publication6.tsv"
    )
    .lines()
    {
        let fields = tcl_test_support::fixture::tsv_fields(row).collect::<Vec<_>>();
        let mut vm = interpreter(fields[0]);
        let completion = vm.try_eval_source_bytes(&decode(fields[1])).unwrap();
        assert_eq!(
            completion.code,
            Code::from_int(fields[2].parse().unwrap()),
            "{}: {completion:?}",
            fields[0]
        );
        assert_eq!(
            crate::fixture::bytes(&vm, &completion.result),
            decode(fields[3]),
            "{} original publication callbacks and variable stores",
            fields[0]
        );
        compared += 1;
    }
    assert_eq!(compared, 6);
}
