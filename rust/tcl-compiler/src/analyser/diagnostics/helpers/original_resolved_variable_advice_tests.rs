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
fn authored_startup_advice_preserves_literal_dollars_and_exact_global_aliases() {
    // naming.diagnostics.original-resolved-variable-name-advice
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-resolved-variable-name-advice.md
    // Authored reporting advice only. These names grant no Native cell,
    // startup value, trace, completed getter or entered frame.
    let empty = HashSet::new();
    for (name, expected) in [
        ("argv", "argv"),
        ("::argv", "argv"),
        ("$argv", "$argv"),
        ("::$argv", "$argv"),
        ("${argv}", "${argv}"),
        ("argv(open", "argv(open"),
        ("$argv(k)", "$argv"),
        ("é(k)", "é"),
    ] {
        let cell = tcl_names::variable::VariableCellKey::Authored(name.to_owned());
        assert_eq!(startup_cell_binding(&cell, true, &empty), (expected, true));
    }
    assert!(!has_global_startup_binding("$::argv", false, &empty));
    assert!(!has_global_startup_binding(
        "$argv",
        false,
        &HashSet::from(["argv".to_owned()])
    ));
    assert!(has_global_startup_binding(
        "$argv",
        false,
        &HashSet::from(["::$argv".to_owned()])
    ));
    assert!(!has_global_startup_binding(
        "argv",
        false,
        &HashSet::from(["::$argv".to_owned()])
    ));
}

#[test]
fn literal_variable_diagnostics_keep_defined_names_and_startup_advice_distinct() {
    // naming.diagnostics.original-resolved-variable-name-advice
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-resolved-variable-name-advice.md
    // Genuine retained Logical analysis, not original Native read or store
    // admission. Check emitted diagnostics and exact read geometry.
    for source in [
        "puts ${$argv}",
        "set {$counter} VALUE; puts ${counter}; puts ${$counter}",
    ] {
        let mut analyser = crate::analyser::Analyser::new();
        let analysis = analyser.analyse(source, "tcl");
        assert!(analysis.allows_retained_logical_declaration_advice());
        let reads: Vec<_> = analysis
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == tcl_core_types::DiagCode::W210)
            .map(|diagnostic| &source[diagnostic.span.as_range()])
            .collect();
        let expected = if source.starts_with("puts") {
            "${$argv}"
        } else {
            "${counter}"
        };
        assert_eq!(reads, vec![expected], "{source:?}");
    }
    let analysis = crate::analyser::Analyser::new().analyse("puts $argv", "tcl");
    assert!(analysis.allows_retained_logical_declaration_advice());
    assert!(
        !analysis
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == tcl_core_types::DiagCode::W210)
    );
}
#[test]
fn literal_scalar_parentheses_do_not_borrow_array_root_parameters() {
    // naming.diagnostics.original-resolved-variable-name-advice
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-resolved-variable-name-advice.md
    // Retained Logical read-before-set advice only: these authored names
    // and spans do not establish a Native array kind or current element.
    for (source, expected) in [
        ("proc f {a} {puts ${a(b}}", vec!["${a(b}"]),
        ("proc f {é} {puts ${é(b}}", vec!["${é(b}"]),
        ("proc f {{$a}} {puts ${$a(b}}", vec!["${$a(b}"]),
        ("proc f {a} {puts ${a(k)}}", vec![]),
        ("proc f {é} {puts ${é(k)}}", vec![]),
        ("proc f {{$a}} {puts ${$a(k)}}", vec![]),
    ] {
        let analysis = crate::analyser::Analyser::new().analyse(source, "tcl");
        assert!(analysis.allows_retained_logical_declaration_advice());
        let reads: Vec<_> = analysis
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == tcl_core_types::DiagCode::W210)
            .map(|diagnostic| &source[diagnostic.span.as_range()])
            .collect();
        assert_eq!(reads, expected, "{source:?}");
    }
}
