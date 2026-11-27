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

//! Procedure names with colons, compared with the tclsh and jimsh captures.

use crate::prelude::*;

fn actual(engine: &str) -> Vm {
    crate::fixture::interpreter(tcl_vm::test_support::profile(engine))
}

fn text(vm: &mut Vm, script: &str) -> (Code, Vec<u8>) {
    let completion = crate::fixture::eval(vm, script);
    (
        completion.code,
        crate::fixture::bytes(vm, &completion.result),
    )
}

#[test]
fn original_procedure_colon_names_match_all_18_native_home_and_error_controls() {
    // Native proof: naming.procedure-source.colon-namespace-home
    // docs/design/analysis/name-resolution-proofs/procedure-source.colon-namespace-home.md
    // Native proof: naming.procedure-source.colon-local-name
    // docs/design/analysis/name-resolution-proofs/procedure-source.colon-local-name.md
    // Native proof: naming.procedure-source.name-before-malformed-formals
    // docs/design/analysis/name-resolution-proofs/procedure-source.name-before-malformed-formals.md
    let rows = include_str!("../../../../tcl-syntax/tests/data/native_procedure_name/rows.txt");
    let mut count = 0;
    for row in rows.lines() {
        let fields: Vec<_> = tcl_test_support::fixture::tsv_fields(row).collect();
        let mut vm = actual(fields[0]);
        let script = String::from_utf8(crate::fixture::unhex(fields[4])).unwrap();
        let (code, result) = text(&mut vm, &script);
        assert_eq!(
            code.as_int(),
            fields[2].parse::<i64>().unwrap(),
            "{}/{}",
            fields[0],
            fields[1]
        );
        assert_eq!(
            result,
            crate::fixture::unhex(fields[3]),
            "{}/{}",
            fields[0],
            fields[1]
        );
        count += 1;
    }
    assert_eq!(count, 18);
}

#[test]
fn catch_epilogue_publishes_status_in_the_actual_interpreter_result_owner() {
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let mut vm = actual(engine);
        assert_eq!(
            text(&mut vm, "list [catch {set x :} r] $r"),
            (Code::Ok, b"0 :".to_vec()),
            "{engine}"
        );
    }
}

#[test]
fn legacy_name_rejection_precedes_original_formal_default_and_body_access() {
    // tclsh 8.4.20 and 8.5.19.
    for engine in ["tcl8.4", "tcl8.5"] {
        let mut vm = actual(engine);
        assert_eq!(
            text(
                &mut vm,
                "list [catch {namespace eval : {proc : {{x untouched}} untouched}} r] $r"
            ),
            (
                Code::Ok,
                b"1 {can't create procedure \":\" in non-global namespace with name starting with \":\"}"
                    .to_vec()
            ),
            "{engine}"
        );
    }
}
