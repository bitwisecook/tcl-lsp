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
fn nested_expression_inside_a_quoted_or_index_operand_is_not_redundant_2503() {
    for source in [
        "set progress \"\"\nexpr {$progress eq \"\" ? \"\" : \"[expr {round($progress * 100)}]%\"}",
        "if {$a([expr {1+1}]) > 0} {}",
    ] {
        let result = Analyser::new().analyse(source, "tcl8.6");
        assert!(
            !result
                .diagnostics
                .iter()
                .any(|finding| finding.code == DiagCode::W114),
            "{source}"
        );
    }
    let result = Analyser::new().analyse("expr {[expr {1+1}] + 1}", "tcl8.6");
    assert!(
        result
            .diagnostics
            .iter()
            .any(|finding| finding.code == DiagCode::W114),
        "a whole operand is still redundant"
    );
}

#[test]
fn nested_expression_advice_uses_actual_renamed_descriptor_and_refuses_shadow_and_inert_data() {
    let source = "rename expr calculate\nif {[calculate {1+1}]} {}";
    let result = Analyser::new().analyse(source, "tcl8.6");
    let finding = result
        .diagnostics
        .iter()
        .find(|finding| finding.code == DiagCode::W114)
        .unwrap();
    assert_eq!(
        source.get(finding.span.as_range()),
        Some("[calculate {1+1}]")
    );
    assert_eq!(finding.fixes[0].new_text, "(1+1)");
    for source in [
        "proc expr args {return nope}\nif {[expr {1+1}]} {}",
        "if {{[expr {1+1}]}} {}",
        "if {[list [expr {1+1}]]} {}",
    ] {
        let result = Analyser::new().analyse(source, "tcl8.6");
        assert!(
            !result
                .diagnostics
                .iter()
                .any(|finding| finding.code == DiagCode::W114),
            "{source}"
        );
    }
    let result = Analyser::new().analyse("if [expr {1+1}] {}", "tcl8.6");
    let finding = result
        .diagnostics
        .iter()
        .find(|finding| finding.code == DiagCode::W114)
        .unwrap();
    assert_eq!(finding.fixes, [] as [crate::irules_checks::CodeFix; 0]);
    // tclsh8.6.18/9.0.4: `expr {[expr {42}] + 1}` -> 43.
    let source = "expr {[expr {42}] + 1}";
    let result = Analyser::new().analyse(source, "tcl8.6");
    assert!(
        result
            .diagnostics
            .iter()
            .any(|finding| finding.code == DiagCode::W114),
        "{:?}",
        result.diagnostics
    );
}
