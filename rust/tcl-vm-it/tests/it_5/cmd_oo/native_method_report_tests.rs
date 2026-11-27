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

//! `self method` in constructors, destructors, `next` and literally named methods, compared with tclsh.

use crate::prelude::*;

#[test]
fn original_foundation_value_headers_follow_constructor_destructor_next_roles() {
    // naming.tcloo.original-selected-method-role-report
    // docs/design/analysis/name-resolution-proofs/tcloo-original-selected-method-role-report.md
    // tclsh 8.6.18 and 9.0.4 print the same rows.
    for profile in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile(profile));
        let completion = crate::fixture::eval(
            &mut vm,
            "proc observe_report_role label {lappend ::rows $label [uplevel 1 {self method}]}; oo::class create ReportBase { constructor {} {observe_report_role ctor-base}; destructor {observe_report_role dtor-base} }; oo::class create ReportDerived { superclass ReportBase; constructor {} {observe_report_role ctor-derived; next}; destructor {observe_report_role dtor-derived; next}; method <constructor> {} {observe_report_role named-literal}; method <destructor> {} {observe_report_role named-destructor}; export <constructor> <destructor> }; ReportDerived create ReportObject; ReportObject <constructor>; ReportObject <destructor>; ReportObject destroy; set rows",
        );
        assert_eq!(completion.code, Code::Ok, "{}", completion.result.to_str());
        assert_eq!(
            completion.result.to_str().as_ref(),
            "ctor-derived <constructor> ctor-base <constructor> named-literal <constructor> named-destructor <destructor> dtor-derived <destructor> dtor-base <destructor>",
            "{profile}"
        );
    }
}
