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

//! Same-original range and ordered assignment completion/header windows.
use crate::Value;
include!("../../../tcl-registry/tests/data/native_list_operations/cases.rs");
include!("../../../tcl-registry/tests/data/native_list_operations/tables.rs");
include!("../../../tcl-registry/tests/data/native_list_assignment_order/cases.rs");
fn unhex(value: &str) -> Vec<u8> {
    value
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}
fn header(value: &Value) -> [String; 3] {
    [
        value.native_object_type_name().to_owned(),
        usize::from(value.resident_string_bytes().is_some()).to_string(),
        value.native_object_reference_count().to_string(),
    ]
}
fn run(engine: &str, body: &str, result: &[&str], original: &[&str]) {
    let profile = tcl_registry::model::resolve_environment(engine).unit_profile();
    let mut vm = crate::native_fixture::interpreter(profile);
    vm.try_eval_source(&format!("set ::log {{}}; proc tap {{v}} {{lappend ::log $v;return $v}}; proc p {{name idx}} {{set x $name;set a(1) $name;{body}}}")).unwrap();
    let head = Value::new_native_string_bytes(b"p".as_slice());
    let arguments = [
        Value::new_native_string_bytes(b"{A} B C".as_slice()),
        Value::new_native_string_bytes(b"1".as_slice()),
    ];
    let completion = vm.invoke_host_original_object_vector(&head, &arguments);
    assert!(
        vm.execution_refusal.is_none(),
        "{engine}/{body}: {:?}",
        vm.execution_refusal
    );
    assert_eq!(
        completion.code.as_int().to_string(),
        result[2],
        "{engine}/{body}"
    );
    assert_eq!(
        header(&completion.result).each_ref().map(String::as_str),
        result[3..6],
        "{engine}/{body}: result header"
    );
    assert_eq!(
        completion.result.to_str().as_bytes(),
        unhex(result[6]),
        "{engine}/{body}: result bytes"
    );
    assert_eq!(
        header(&arguments[0]).each_ref().map(String::as_str),
        original[2..5],
        "{engine}/{body}: original header"
    );
    assert_eq!(arguments[0].to_str().as_bytes(), unhex(original[5]));
}
#[test]
fn original_range_and_assignment_match_84_native_completion_and_original_header_pairs() {
    let mut windows = 0;
    for &(engine, list, order) in LIST_TABLES {
        for (rows, bodies, cases) in [
            (list, CASES, &[0usize, 1, 2, 3, 4, 11, 12, 13, 14, 28][..]),
            (order, ORDER_CASES, &[0usize, 1, 2, 3][..]),
        ] {
            for line in rows.lines().filter(|line| line.starts_with("R|")) {
                let result = line.split('|').collect::<Vec<_>>();
                let case = result[1].parse::<usize>().unwrap();
                if !cases.contains(&case) {
                    continue;
                }
                let prefix = format!("O|{case}|");
                let original = rows
                    .lines()
                    .find(|line| line.starts_with(&prefix))
                    .unwrap()
                    .split('|')
                    .collect::<Vec<_>>();
                run(engine, bodies[case], &result, &original);
                windows += 1;
            }
        }
    }
    assert_eq!(windows, 84);
}
