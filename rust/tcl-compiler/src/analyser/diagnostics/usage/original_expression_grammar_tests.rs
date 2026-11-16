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
fn original_expression_advice_keeps_whole_input_and_body_comment_bom() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Actual parser input and lexical comment geometry; no expression execution grant.
    let mut prepared = Analyser::new();
    let input = prepared.prepare_analysis_input("tcl9.1").unwrap();
    for supplied in [None, Some(input.clone())] {
        let mut analyser = supplied.map_or_else(Analyser::new, |input| {
            Analyser::new().with_resolved_input(input)
        });
        let source = "\u{feff}expr {$name == \"literal\"}";
        analyser.analyse_and_retain_result_for_test(source, "tcl9.1");
        assert!(
            analyser.registry.is_none(),
            "walk-exit scratch holder is cleared"
        );
        assert!(
            analyser
                .head_identities
                .matches_resolved_analysis_input(&input)
        );
        let context = analyser
            .original_expression_parser_context()
            .expect("actual retained whole input");
        assert_eq!(
            context.lexer_grammar.braced_var,
            input.lexer_config().braced_var
        );
        assert_eq!(analyser.result.resolved_input.as_ref(), Some(&input));
        let hints = analyser
            .result
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == DiagCode::W110)
            .collect::<Vec<_>>();
        assert_eq!(hints.len(), 1);
        assert_eq!(source.get(hints[0].span.as_range()), Some("=="));
        let mut changed = input.clone();
        changed.config.strict_quoting = !changed.config.strict_quoting;
        analyser.result.resolved_input = Some(changed);
        assert!(analyser.original_expression_parser_context().is_none());
        analyser.result.resolved_input = None;
        assert!(analyser.original_expression_parser_context().is_none());
    }
    // The BOM in extracted brace content is a command character, so the
    // following # is an argument rather than the start of a comment.
    assert_eq!(
        comment_regions_recursive("\u{feff}# prose", input.lexer_config()),
        [] as [std::ops::Range<usize>; 0]
    );
    let comments = comment_regions_recursive("# prose\n{# nested}", input.lexer_config());
    assert_eq!(comments, [0..7, 9..17]);
}

#[test]
fn original_string_comparison_advice_uses_retained_variable_grammar() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Source parser and warning/edit geometry only; no runtime equivalence.
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    let source = r#"expr {${a{b}c} == "needle"}"#;
    for (braced_var, expected) in [
        (tcl_dialect::BracedVarStyle::FirstClose, 0),
        (tcl_dialect::BracedVarStyle::Tcl9Nesting, 1),
    ] {
        let config = tcl_lexer::LexerConfig {
            braced_var,
            ..tcl_lexer::LexerConfig::from_grammar(profile.grammar)
        };
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::clone(&context),
            config,
        );
        let mut analyser = Analyser::new().with_resolved_input(input);
        let result = analyser.analyse_and_retain_result_for_test(source, "presentation-only");
        let hints: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == DiagCode::W110)
            .collect();
        assert_eq!(
            hints.len(),
            expected,
            "{braced_var:?}: {:?}",
            result.diagnostics
        );
        if let Some(hint) = hints.first() {
            assert_eq!(source.get(hint.span.as_range()), Some("=="));
            assert_eq!(hint.fixes.len(), 1);
            assert_eq!(hint.fixes[0].new_text, "eq");
        }
        assert_eq!(
            analyser
                .original_expression_parser_context()
                .unwrap()
                .lexer_grammar
                .braced_var,
            braced_var
        );
        analyser.result.resolved_input = None;
        assert!(analyser.original_expression_parser_context().is_none());
        analyser.result.resolved_input = Some(crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::resolve_environment("tcl8.4").default_context_registry(),
            config,
        ));
        assert!(analyser.original_expression_parser_context().is_none());
    }
}
