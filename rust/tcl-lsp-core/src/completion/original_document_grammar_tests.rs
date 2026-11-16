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
fn original_document_grammar_completion_preserves_full_source_configuration() {
    // Implementation contract: naming.core.original-document-grammar-completion
    // docs/design/analysis/name-resolution-proofs/original-document-grammar-completion.md
    let source = "\u{feff}endpoint www {\n    hsts {\n        \n    }\n}\n";
    let profile = tcl_registry::model::ingress::resolve_environment("sslictcl").analyser_profile();
    let context = tcl_registry::model::ingress::context_for_profile(profile);
    let analyse = |leading_bom| {
        let config = tcl_lexer::LexerConfig {
            leading_bom,
            ..tcl_lexer::LexerConfig::for_file_grammar(profile.grammar)
        };
        let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::clone(&context),
            config,
        );
        tcl_compiler::analyser::Analyser::new()
            .with_resolved_input(input)
            .analyse(source, profile.name)
    };
    let labels = |analysis: &AnalysisResult| {
        completions(source, 2, 8, analysis, None, None, profile)
            .into_iter()
            .map(|item| item.label)
            .collect::<Vec<_>>()
    };
    let skipping = analyse(tcl_lexer::LeadingBom::Skip);
    let nested = labels(&skipping);
    assert!(nested.iter().any(|label| label == "max-age"), "{nested:?}");
    assert!(
        !nested.iter().any(|label| label == "endpoint"),
        "{nested:?}"
    );
    let literal = analyse(tcl_lexer::LeadingBom::Content);
    let root = labels(&literal);
    assert!(root.iter().any(|label| label == "endpoint"), "{root:?}");
    assert!(!root.iter().any(|label| label == "max-age"), "{root:?}");
    let index = tcl_lexer::LineIndex::new(source);
    assert!(
        definition_grammar_at_position(
            &source.replace("www", "zzz"),
            2,
            8,
            &index,
            &skipping,
            context.commands(),
        )
        .is_none()
    );
}
