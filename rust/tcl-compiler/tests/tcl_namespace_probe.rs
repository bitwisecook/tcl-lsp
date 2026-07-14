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

//! Probes for the `::tcl` namespace surfaces: the prefix-form operator
//! commands (`::tcl::mathop::*`), user-extensible math functions
//! (`::tcl::mathfunc::*`), and `tcl::unsupported` — none may draw a W123
//! unknown-command (they are real commands in every Tcl 8.5+), and a
//! workspace-defined mathfunc proc must be a legitimate definition.

use tcl_compiler::analyser::Analyser;

fn codes(source: &str) -> Vec<String> {
    let mut a = Analyser::new();
    let result = a.analyse(source, "tcl9.0");
    result
        .diagnostics
        .iter()
        .map(|d| format!("{:?}", d.code))
        .collect()
}

#[test]
fn mathop_prefix_commands_resolve() {
    // tclsh: `::tcl::mathop::+ 1 2` → 3; both spellings are real commands.
    for src in [
        "set x [::tcl::mathop::+ 1 2]\nputs $x\n",
        "set x [tcl::mathop::+ 1 2]\nputs $x\n",
        "set ops [list ::tcl::mathop::* ::tcl::mathop::-]\nputs $ops\n",
    ] {
        let got = codes(src);
        assert!(
            !got.iter().any(|c| c.contains("W123")),
            "mathop command must be known for {src:?}: {got:?}"
        );
    }
}

#[test]
fn mathfunc_proc_definition_extends_expr() {
    // tclsh: `proc ::tcl::mathfunc::double2 {x} {expr {$x*2}}` makes
    // `expr {double2(4)}` valid — the definition must not be flagged, and
    // the proc must count as used/known.
    let src = "proc ::tcl::mathfunc::double2 {x} { expr {$x * 2} }\nputs [expr {double2(4)}]\n";
    let got = codes(src);
    assert!(
        !got.iter().any(|c| c.contains("W123") || c.contains("E00")),
        "user mathfunc must be clean: {got:?}"
    );
}

#[test]
fn tcl_unsupported_representation_is_not_unknown() {
    // `tcl::unsupported::representation` exists in 8.6+; the analyser must
    // stay conservative (no W123 hard-claim) even if the registry lacks a
    // spec for it.
    let got = codes("puts [tcl::unsupported::representation abc]\n");
    assert!(
        !got.iter().any(|c| c.contains("W123")),
        "tcl::unsupported must not be flagged unknown: {got:?}"
    );
}
