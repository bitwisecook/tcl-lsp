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

//! Compiled ensemble dispatch after its handler or configuration changes, against tclsh.

use crate::prelude::*;

#[test]
fn nested_info_compilation_retains_original_maps_and_late_worker_binding() {
    let script = "oo::class create C
        proc p {} {info class superclasses ::C}
        set r [list [p]]
        rename ::oo::InfoClass::superclasses {}
        proc ::oo::InfoClass::superclasses args {return REPLACED}
        lappend r [p]
        namespace ensemble configure ::oo::InfoClass -prefixes [expr {![namespace ensemble configure ::oo::InfoClass -prefixes]}]
        lappend r [p]
        set r";
    for engine in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile(engine));
        let completion = crate::fixture::eval(&mut vm, script);
        assert_eq!(
            completion.code,
            Code::Ok,
            "{engine}: {:?}",
            completion.result
        );
        assert_eq!(
            completion.result.to_str().as_ref(),
            "::oo::object REPLACED REPLACED",
            "{engine}"
        );
    }
}

#[test]
fn emitted_array_operation_retains_actual_ensemble_compiler() {
    for engine in ["tcl9.0", "tcl9.1"] {
        let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile(engine));
        let completion = crate::fixture::eval(
            &mut vm,
            "proc p {} {array set ::payload {}}; list [p] [array exists payload]",
        );
        assert_eq!(completion.code, Code::Ok, "{engine}");
        assert_eq!(completion.result.to_str().as_ref(), "{} 1", "{engine}");
    }
}
