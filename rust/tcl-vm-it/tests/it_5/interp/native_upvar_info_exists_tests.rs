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

//! Ordinary `upvar` and `info exists` in procedure bodies, against the tclsh
//! captures: completion codes, results and the values each case reaches.

use crate::prelude::*;
use std::cell::RefCell;

include!("../../../../tcl-registry/tests/data/native_upvar_info_exists/cases.rs");
const TABLES: &[(&str, &str)] = &[
    (
        "tcl8.4",
        include_str!("../../../../tcl-registry/tests/data/native_upvar_info_exists/8.4.20.txt"),
    ),
    (
        "tcl8.5",
        include_str!("../../../../tcl-registry/tests/data/native_upvar_info_exists/8.5.19.txt"),
    ),
    (
        "tcl8.6",
        include_str!("../../../../tcl-registry/tests/data/native_upvar_info_exists/8.6.18.txt"),
    ),
    (
        "tcl9.0",
        include_str!("../../../../tcl-registry/tests/data/native_upvar_info_exists/9.0.4.txt"),
    ),
    (
        "tcl9.1",
        include_str!("../../../../tcl-registry/tests/data/native_upvar_info_exists/9.1.0.txt"),
    ),
];
include!("../../../../tcl-registry/tests/data/native_upvar_info_exists/quiet/cases.rs");
const QUIET_TABLES: &[(&str, &str)] = &[
    (
        "tcl8.4",
        include_str!(
            "../../../../tcl-registry/tests/data/native_upvar_info_exists/quiet/8.4.20.txt"
        ),
    ),
    (
        "tcl8.5",
        include_str!(
            "../../../../tcl-registry/tests/data/native_upvar_info_exists/quiet/8.5.19.txt"
        ),
    ),
    (
        "tcl8.6",
        include_str!(
            "../../../../tcl-registry/tests/data/native_upvar_info_exists/quiet/8.6.18.txt"
        ),
    ),
    (
        "tcl9.0",
        include_str!(
            "../../../../tcl-registry/tests/data/native_upvar_info_exists/quiet/9.0.4.txt"
        ),
    ),
    (
        "tcl9.1",
        include_str!(
            "../../../../tcl-registry/tests/data/native_upvar_info_exists/quiet/9.1.0.txt"
        ),
    ),
];
const SETUP: &str = "catch {unset ::alias};set ::x GLOBAL;set ::a(k) GLOBALARRAY;catch {unset ::g};set ::log {}; proc tap {v} {lappend ::log $v;return $v};proc traceRead {n e op} {set ::g CREATED}; proc traceFail {n e op} {error TRACE}";
fn unhex(value: &str) -> Vec<u8> {
    value
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

thread_local! {
    static MARKS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}
fn mark(_vm: &mut Vm, arguments: &[Value]) -> Completion<Value> {
    let mut bytes = String::new();
    for byte in arguments[0].string_bytes().iter() {
        use std::fmt::Write;
        write!(bytes, "{byte:02x}").unwrap();
    }
    MARKS.with(|marks| marks.borrow_mut().push(bytes));
    Completion::new(Code::Ok, Value::empty(), Value::empty())
}
fn compare_native_tables(tables: &[(&str, &str)], cases: &[&str]) -> (usize, usize) {
    let mut results = 0;
    let mut headers = 0;
    for &(engine, table) in tables {
        let profile = tcl_dialect::DialectProfile::find(engine).unwrap();
        let mut vm = crate::fixture::interpreter(profile);
        vm.register("mark", mark);
        for row in table.lines().filter(|row| row.starts_with("R|")) {
            let fields: Vec<_> = row.split('|').collect();
            let case = fields[1].parse::<usize>().unwrap();
            assert_eq!(
                vm.try_eval_source(SETUP).unwrap().code,
                tcl_runtime_api::Code::Ok
            );
            vm.try_eval_source(&format!(
                "proc p {{name idx}} {{set x LOCAL;set a(k) LOCALARRAY;{}}}",
                cases[case]
            ))
            .unwrap();
            MARKS.with(|marks| marks.borrow_mut().clear());
            let arguments = [
                Value::new_native_string_bytes(b"x".as_slice()),
                Value::new_native_string_bytes(b"k".as_slice()),
            ];
            let completion = crate::fixture::call(&mut vm, "p", &arguments);
            assert_eq!(
                completion.code.as_int().to_string(),
                fields[2],
                "{engine}/{case}"
            );
            let value = &completion.result;
            assert_eq!(
                value.string_bytes().as_ref(),
                unhex(fields[6]),
                "{engine}/{case} result"
            );
            let expected = table
                .lines()
                .filter(|row| row.starts_with(&format!("H|{case}|")))
                .map(|row| row.rsplit('|').next().unwrap().to_owned())
                .collect::<Vec<_>>();
            MARKS.with(|marks| {
                assert_eq!(
                    *marks.borrow(),
                    expected,
                    "{engine}/{case} reached originals"
                );
            });
            results += 1;
            headers += expected.len();
        }
    }
    (results, headers)
}

// Native proof: naming.variable.original-upvar-and-exists-completion-and-name-windows
// docs/design/analysis/name-resolution-proofs/variable.original-upvar-and-exists-completion-and-name-windows.md
#[test]
fn original_upvar_and_exists_preserve_140_native_completions_and_20_name_headers() {
    assert_eq!(compare_native_tables(TABLES, CASES), (140, 20));
}
// Native proof: naming.variable.exists-read-trace-quiet-result-purpose
// docs/design/analysis/name-resolution-proofs/variable.exists-read-trace-quiet-result-purpose.md
#[test]
fn original_exists_quiet_trace_semantics_preserve_15_native_result_windows() {
    assert_eq!(compare_native_tables(QUIET_TABLES, QUIET_CASES), (15, 0));
}
