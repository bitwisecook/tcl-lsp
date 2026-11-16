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

fn actions(
    source: &str,
    analysis: &AnalysisResult,
    line: u32,
    style: crate::formatting::DocstringStyle,
) -> Vec<CodeAction> {
    let index = LineIndex::new(source);
    docstring_actions(
        source,
        LspRange {
            start_line: line,
            start_character: 0,
            end_line: line,
            end_character: 0,
        },
        analysis,
        &index,
        style,
    )
}

#[test]
// Implementation contract: naming.core.original-docstring-declaration-actions
// docs/design/analysis/name-resolution-proofs/original-docstring-declaration-actions.md
fn original_docstrings_keep_opaque_redefinitions_without_reporting_maps() {
    let source = "proc p\\uD800 {first} {return $first}\nproc p\\uD801 {second} {return $second}\nproc p\\uD800 {third} {return $third}\n";
    let mut analysis = Analyser::new().analyse(source, "tcl8.6");
    analysis.all_procs.clear();
    analysis.global_scope.procs.clear();
    for (line, formal) in [(0, "first"), (1, "second"), (2, "third")] {
        let found = actions(
            source,
            &analysis,
            line,
            crate::formatting::DocstringStyle::Preceding,
        );
        assert_eq!(found.len(), 1);
        assert!(found[0].edits[0].new_text.contains(formal));
        assert_eq!(found[0].edits[0].range.start_line, line);
    }
    assert_eq!(
        actions(
            &format!("# displaced\n{source}"),
            &analysis,
            1,
            crate::formatting::DocstringStyle::Preceding
        ),
        [] as [crate::code_actions::CodeAction; 0]
    );
    analysis.body_lexer_config = None;
    assert_eq!(
        actions(
            source,
            &analysis,
            0,
            crate::formatting::DocstringStyle::Preceding
        ),
        [] as [crate::code_actions::CodeAction; 0]
    );
}

#[test]
// Implementation contract: naming.core.original-docstring-declaration-actions
// docs/design/analysis/name-resolution-proofs/original-docstring-declaration-actions.md
fn original_body_docstring_requires_its_braced_word_and_rejects_computed_body() {
    let source = "proc p {value} {return $value}\n";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let found = actions(
        source,
        &analysis,
        0,
        crate::formatting::DocstringStyle::Body,
    );
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].edits[0].range.start_line, 0);
    assert_eq!(found[0].edits[0].range.start_character, 16);
    assert_eq!(found[0].edits[0].range.end_line, 0);
    assert_eq!(found[0].edits[0].range.end_character, 16);
    assert!(found[0].edits[0].new_text.contains("value"));
    let computed = "set script {return value}\nproc computed {} $script\n";
    let analysis = Analyser::new().analyse(computed, "tcl8.6");
    assert_eq!(
        actions(
            computed,
            &analysis,
            1,
            crate::formatting::DocstringStyle::Preceding
        )
        .len(),
        1
    );
    assert_eq!(
        actions(
            computed,
            &analysis,
            1,
            crate::formatting::DocstringStyle::Body
        ),
        [] as [crate::code_actions::CodeAction; 0]
    );
    assert_eq!(
        actions(
            computed,
            &analysis,
            1,
            crate::formatting::DocstringStyle::None
        ),
        [] as [crate::code_actions::CodeAction; 0]
    );
}
