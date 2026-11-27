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

//! Failed class definitions and constructors leave no class or object behind, compared with tclsh.

use crate::prelude::*;

fn text(script: &str) -> String {
    let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile("tcl9.1"));
    let completion = crate::fixture::eval(&mut vm, script);
    assert_eq!(completion.code, Code::Ok, "{script}");
    completion.result.to_str().to_string()
}

#[test]
fn original_class_definition_failure_tears_down_only_its_allocated_original_record() {
    // tclsh 8.6.18 and 9.0.4: `1 DEFINITION {}`.
    assert_eq!(
        text("oo::class create Kept; list [catch {oo::class create K {error DEFINITION}} r] $r [info commands K] [info object isa class Kept]"),
        "1 DEFINITION {} 1"
    );
}

#[test]
fn original_failed_procedure_constructor_keeps_child_state_and_retires_original_object() {
    // tclsh 8.6.18 and 9.0.4: `1 CONSTRUCTOR {} 1`.
    assert_eq!(
        text("oo::class create F {constructor {} {error CONSTRUCTOR}}; list [catch {F create f} r] $r [info commands f] [info object isa class F]"),
        "1 CONSTRUCTOR {} 1"
    );
}
