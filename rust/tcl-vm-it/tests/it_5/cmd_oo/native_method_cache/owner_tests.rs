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

//! Method dispatch after deletion, inherited redefinition and redefinition, compared with tclsh.

use crate::prelude::*;

fn text(vm: &mut Vm, script: &str) -> String {
    let completion = crate::fixture::eval(vm, script);
    assert_eq!(completion.code, Code::Ok, "{script}");
    completion.result.to_str().to_string()
}

fn vm() -> Vm {
    crate::fixture::interpreter(tcl_vm::test_support::profile("tcl8.6"))
}

#[test]
fn original_vm_idle_selector_cache_does_not_retain_deleted_method_records() {
    // tclsh 8.6.18 and 9.0.4.
    let mut vm = vm();
    assert_eq!(
        text(
            &mut vm,
            "oo::class create C {method m {} {return RESULT}}; C create obj; obj m"
        ),
        "RESULT"
    );
    assert_eq!(
        text(
            &mut vm,
            "oo::define C deletemethod m; list [catch {obj m} r] $r"
        ),
        "1 {unknown method \"m\": must be destroy}"
    );
}

#[test]
fn original_vm_entered_chain_keeps_all_records_and_reads_current_same_record_payload() {
    // tclsh 8.6.18 and 9.0.4: `MUTATED MUTATED`.
    let mut vm = vm();
    assert_eq!(
        text(
            &mut vm,
            "oo::class create B {method m {} {return BASE}}; oo::class create C {superclass B; method m {} {oo::define B method m {} {return MUTATED}; next}}; C create obj; list [obj m] [obj m]",
        ),
        "MUTATED MUTATED"
    );
}

#[test]
fn original_vm_cache_world_does_not_adopt_a_fresh_stamp_after_actual_mutation() {
    let mut vm = vm();
    assert_eq!(
        text(
            &mut vm,
            "oo::class create C {method m {} {return FIRST}}; C create obj; obj m"
        ),
        "FIRST"
    );
    assert_eq!(
        text(&mut vm, "oo::define C method m {} {return SECOND}; obj m"),
        "SECOND"
    );
}
