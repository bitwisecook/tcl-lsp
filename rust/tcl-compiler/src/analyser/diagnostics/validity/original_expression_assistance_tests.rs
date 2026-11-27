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

fn input(braced_var: tcl_dialect::BracedVarStyle) -> crate::analyser::ResolvedAnalysisInput {
    let selected = tcl_dialect::DialectProfile::find("tcl8.4").expect("authored Tcl 8.4 grammar");
    let mut profile = tcl_dialect::DialectProfile::plain_tcl().clone();
    profile.grammar = selected.grammar;
    profile.expr_grammar_base = selected.expr_grammar_base;
    let profile = profile.intern();
    let config = tcl_lexer::LexerConfig {
        braced_var,
        ..tcl_lexer::LexerConfig::for_file_grammar(profile.grammar)
    };
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        tcl_registry::model::ingress::resolve_environment("tcl8.4").default_context_registry(),
        config,
    );
    assert!(input.has_logical_source_name_context());
    input
}

#[test]
fn original_operator_assistance_preserves_retained_variable_grammar() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Recognition and warning geometry only; no Native syntax acceptance.
    let source = "expr {${a{b}c} in {λ}}";
    for (style, count) in [
        (tcl_dialect::BracedVarStyle::FirstClose, 0),
        (tcl_dialect::BracedVarStyle::Tcl9Nesting, 1),
    ] {
        let result = Analyser::new()
            .with_resolved_input(input(style))
            .analyse(source, "tcl");
        let warnings = result
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == DiagCode::W003)
            .collect::<Vec<_>>();
        assert_eq!(warnings.len(), count, "{style:?}: {:?}", result.diagnostics);
        if let [warning] = warnings.as_slice() {
            assert_eq!(source.get(warning.span.as_range()), Some("in"));
        }
    }
}

#[test]
fn original_function_assistance_keeps_lexer_overlays_and_missing_owner_refusals() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Original source references only, independently of dispatch/frames.
    let source = "expr {max(${a{b}c}, 1)}";
    for (style, count) in [
        (tcl_dialect::BracedVarStyle::FirstClose, 0),
        (tcl_dialect::BracedVarStyle::Tcl9Nesting, 1),
    ] {
        let actual = input(style);
        let mut analyser = Analyser::new().with_resolved_input(actual.clone());
        let result = analyser.analyse(source, "tcl");
        let functions = result
            .command_invocations
            .iter()
            .filter(|invocation| invocation.is_mathfunc_call && invocation.name == "max")
            .collect::<Vec<_>>();
        assert_eq!(functions.len(), count, "{style:?}");
        if let [function] = functions.as_slice() {
            assert_eq!(source.get(function.range.as_range()), Some("max"));
            assert_eq!(function.argc, Some(2));
        }
        let command = tcl_source::segment::segment_commands_with_offset_and_config(
            source,
            0,
            actual.lexer_config(),
        )
        .pop()
        .unwrap();
        let expression = command.argv[1];
        analyser.result.resolved_input = None;
        assert_eq!(
            analyser.expr_function_calls(expression),
            [] as [(std::string::String, tcl_lexer::Span, usize); 0]
        );
        analyser.result.resolved_input = Some(crate::analyser::ResolvedAnalysisInput::new(
            actual.analyser_profile(),
            actual.unit_profile(),
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry(),
            actual.lexer_config(),
        ));
        assert_eq!(
            analyser.expr_function_calls(expression),
            [] as [(std::string::String, tcl_lexer::Span, usize); 0]
        );
        let mut stale = actual;
        stale.config.strict_quoting = !stale.config.strict_quoting;
        analyser.result.resolved_input = Some(stale);
        assert_eq!(
            analyser.expr_function_calls(expression),
            [] as [(std::string::String, tcl_lexer::Span, usize); 0]
        );
    }
}
