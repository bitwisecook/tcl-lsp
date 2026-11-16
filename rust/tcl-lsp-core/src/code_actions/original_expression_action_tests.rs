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
use tcl_compiler::analyser::Analyser;

fn actions(source: &str, analysis: &AnalysisResult) -> Vec<CodeAction> {
    selected_actions(source, analysis, "!(1 && 0)")
}

fn selected_actions(source: &str, analysis: &AnalysisResult, selected: &str) -> Vec<CodeAction> {
    let start = u32::try_from(source.find(selected).unwrap()).unwrap();
    expr_rewrite_actions(
        source,
        LspRange {
            start_line: 0,
            start_character: start,
            end_line: 0,
            end_character: start + u32::try_from(selected.len()).unwrap(),
        },
        analysis,
        &LineIndex::new(source),
    )
}

#[test]
fn literal_native_operator_actions_require_a_whole_normal_unobserved_operand() {
    for (source, selected, replacement) in [
        ("if {!(1 && 0)} {}", "!(1 && 0)", "!1 || !0"),
        ("expr {!1 || !0}", "!1 || !0", "!(1 && 0)"),
        ("expr {1 == 0}", "1 == 0", "1 != 0"),
        ("expr {$a == $b}", "$a == $b", "$a != $b"),
    ] {
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let proposed = selected_actions(source, &analysis, selected);
        assert_eq!(proposed.len(), 1, "{source}");
        assert!(proposed[0].disabled.is_none(), "{source}");
        assert_eq!(proposed[0].edits[0].new_text, replacement, "{source}");
    }
    for (source, selected) in [
        ("if {1 == 1} {} elseif {[incr x]} {}", "1 == 1"),
        ("expr {$a < $b}", "$a < $b"),
        ("expr {1 < 2}", "1 < 2"),
        ("if {!(1 && 0)} {info frame -1}", "!(1 && 0)"),
        ("expr {1 && (2 == 3)}", "2 == 3"),
    ] {
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let proposed = selected_actions(source, &analysis, selected);
        assert_eq!(proposed.len(), 1, "{source}");
        assert!(proposed[0].edits.is_empty(), "{source}");
        assert!(proposed[0].disabled.is_some(), "{source}");
    }
}

#[test]
// Implementation contract: naming.core.original-expression-source-selection
// docs/design/analysis/name-resolution-proofs/original-expression-source-selection.md
fn original_expression_actions_separate_position_from_rewrite_equivalence() {
    let source = "expr {!(1 && 0)}";
    let mut analysis = Analyser::new().analyse(source, "tcl8.6");
    let proposed = actions(source, &analysis);
    assert_eq!(proposed.len(), 1);
    assert_eq!(proposed[0].edits.len(), 1);
    assert_eq!(proposed[0].edits[0].new_text, "!1 || !0");
    assert!(proposed[0].disabled.is_none());
    let body = "if {!(1 && 0)} {puts yes}";
    let proposed = actions(body, &Analyser::new().analyse(body, "tcl8.6"));
    assert_eq!(
        proposed[0].edits.len(),
        1,
        "a reflection-free body: {proposed:?}"
    );
    for unavailable in [
        "expr {!(1 && 0)}; info frame 0",
        "if {!(1 && 0)} {info frame -1}",
        "if {!(1 && 0)} {puts [info frame 0]}",
    ] {
        let analysis = Analyser::new().analyse(unavailable, "tcl8.6");
        let proposed = actions(unavailable, &analysis);
        assert_eq!(proposed.len(), 1, "{unavailable}");
        assert!(proposed[0].edits.is_empty(), "{unavailable}");
        assert!(
            proposed[0]
                .disabled
                .as_ref()
                .unwrap()
                .starts_with("missing-expression-operator-rewrite-permission:")
        );
    }
    analysis.command_invocations.clear();
    assert_eq!(actions(source, &analysis).len(), 1);
    assert_eq!(
        actions(&format!("#{source}"), &analysis),
        [] as [crate::code_actions::CodeAction; 0]
    );
    for source in [
        "set data {!(1 && 0)}",
        "# expr {!(1 && 0)}",
        "proc expr args {}; expr {!(1 && 0)}",
        "trace add execution expr enter callback; expr {!(1 && 0)}",
    ] {
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        assert!(actions(source, &analysis).is_empty(), "{source}");
    }
}
