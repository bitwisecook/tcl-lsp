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

//! Introspection inside compiled procedures, against the tclsh captures' codes and result bytes.

use crate::prelude::*;

include!("../../../../../tcl-registry/tests/data/native_introspection_compilation/cases.rs");

#[test]
fn compiled_introspection_preserves_thirty_three_native_original_header_windows() {
    original_windows(false);
}

#[test]
fn compiled_arrays_preserve_forty_eight_native_original_header_and_local_windows() {
    original_windows(true);
}

#[test]
fn compiled_array_make_preserves_the_native_write_array_error_category() {
    for engine in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile(engine));
        let completion = crate::fixture::eval(
            &mut vm,
            "proc p {} {set a scalar; array set a {}}; catch {p} message; set ::errorCode",
        );
        assert_eq!(completion.code, Code::Ok, "{engine}");
        assert_eq!(
            crate::fixture::bytes(&vm, &completion.result),
            b"TCL WRITE ARRAY",
            "{engine}"
        );
    }
}

fn original_windows(arrays: bool) {
    let mut windows = 0;
    for (engine, table) in [
        (
            "tcl8.6",
            include_str!(
                "../../../../../tcl-registry/tests/data/native_introspection_compilation/8.6.18.tsv"
            ),
        ),
        (
            "tcl9.0",
            include_str!(
                "../../../../../tcl-registry/tests/data/native_introspection_compilation/9.0.4.tsv"
            ),
        ),
        (
            "tcl9.1",
            include_str!(
                "../../../../../tcl-registry/tests/data/native_introspection_compilation/9.1.0.tsv"
            ),
        ),
    ] {
        let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile(engine));
        crate::fixture::eval(
            &mut vm,
            "namespace eval ::N {}; namespace eval ::source {proc target {} {}; namespace export target}; namespace eval ::N {namespace import ::source::target}",
        );
        for row in table.lines() {
            let fields: Vec<_> = tcl_test_support::fixture::tsv_fields(row).collect();
            let case = fields[0].parse::<usize>().unwrap();
            if if arrays {
                case < 15
            } else {
                !matches!(case, 0 | 2 | 3 | 4 | 6 | 8 | 9 | 10 | 11 | 12 | 13)
            } {
                continue;
            }
            crate::fixture::eval(
                &mut vm,
                &format!("proc ::N::p {{left right}} {{{}}}", CASES[case]),
            );
            let left = match case {
                2 => "target",
                10 => "0",
                _ => "a",
            };
            let arguments = [Value::string(left), Value::string("k V j W")];
            let completion = crate::fixture::call(&mut vm, "::N::p", &arguments);
            assert_eq!(
                completion.code.as_int().to_string(),
                fields[1],
                "{engine}/{case}"
            );
            assert_eq!(
                crate::fixture::bytes(&vm, &completion.result),
                crate::fixture::unhex(fields[8]),
                "{engine}/{case}"
            );
            windows += 1;
        }
    }
    assert_eq!(windows, if arrays { 48 } else { 33 });
}
