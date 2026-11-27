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

//! `renamemethod` of a missing method and to an integer name, compared with tclsh.

use crate::prelude::*;

fn text(vm: &mut Vm, script: &str) -> String {
    let completion = crate::fixture::eval(vm, script);
    assert_eq!(completion.code, Code::Ok, "{script}");
    completion.result.to_str().to_string()
}

#[test]
fn original_vm_rename_missing_source_does_not_reach_destination_string() {
    let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile("tcl9.0"));
    assert_eq!(
        text(
            &mut vm,
            "oo::class create R {method m {} {return MOVED}}; list [catch {oo::define R renamemethod absent 42} x] $x [info class methods R]",
        ),
        "1 {method absent does not exist} m"
    );
}

#[test]
fn original_vm_rename_moves_same_record_and_formats_an_integer_destination_without_the_host() {
    // tclsh 8.6.18 and 9.0.4.
    let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile("tcl9.0"));
    let destination = Value::int(42);
    text(
        &mut vm,
        "oo::class create R {method m {} {return MOVED}}; R create r",
    );
    let renamed = crate::fixture::call(
        &mut vm,
        "oo::define",
        &[
            Value::string("R"),
            Value::string("renamemethod"),
            Value::string("m"),
            destination,
        ],
    );
    assert_eq!(renamed.code, Code::Ok);
    assert_eq!(
        text(&mut vm, "list [r 42] [catch {r m} x] $x"),
        "MOVED 1 {unknown method \"m\": must be 42 or destroy}"
    );
}
