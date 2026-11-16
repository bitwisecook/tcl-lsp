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

use super::*;

#[test]
fn the_first_record_naming_a_method_decides_external_visibility() {
    // tclsh8.6.18, 9.0.4 and 9.1.0 print
    // `1 {unknown method "foo": must be destroy} 0 F 0 F`.
    let script = "oo::class create A {method foo {} {return F}}
            oo::class create B {superclass A; unexport foo}
            B create b1
            set r [list [catch {b1 foo} m] $m]
            oo::class create D {superclass B; export foo}
            lappend r [catch {[D new] foo} m] $m
            B create b2; oo::objdefine b2 export foo
            lappend r [catch {b2 foo} m] $m";
    for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut vm =
            crate::native_fixture::interpreter(crate::environment::profile_for_dialect(dialect));
        let reached = vm.eval_source(script).unwrap();
        assert_eq!(reached.code, Code::Ok, "{dialect}");
        assert_eq!(
            reached.result.to_str().as_ref(),
            "1 {unknown method \"foo\": must be destroy} 0 F 0 F",
            "{dialect}"
        );
    }
}

#[test]
fn an_abstract_class_hides_its_factories() {
    // tclsh9.0.4 and 9.1.0 print `1 {unknown method "new": must be destroy}`.
    for dialect in ["tcl9.0", "tcl9.1"] {
        let mut vm =
            crate::native_fixture::interpreter(crate::environment::profile_for_dialect(dialect));
        let reached = vm
            .eval_source("oo::abstract create A; list [catch {A new} m] $m")
            .unwrap();
        assert_eq!(reached.code, Code::Ok, "{dialect}");
        assert_eq!(
            reached.result.to_str().as_ref(),
            "1 {unknown method \"new\": must be destroy}",
            "{dialect}"
        );
    }
}
