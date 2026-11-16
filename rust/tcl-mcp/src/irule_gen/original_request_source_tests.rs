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
fn original_request_context_keeps_retained_unit_profile_and_current_source() {
    let source = "when HTTP_REQUEST {pool p}";
    let analysis = crate::tools::analyse(source, "f5-irules");
    let current = tcl_irules::OriginalIrulesSourceContext::from_source_analysis(source, &analysis)
        .expect("actual current source analysis owns the source grammar");
    let input = analysis.resolved_input.as_ref().unwrap();
    let config = analysis.body_lexer_config.unwrap();
    let mut expression = ExprParseContext::for_profile(input.unit_profile());
    expression.lexer_grammar = config.grammar_over(expression.lexer_grammar);
    assert_eq!(current.expression_syntax(), (config, expression));
    let mut changed_unit = input.unit_profile().clone();
    changed_unit.expr_grammar_base =
        if changed_unit.expr_grammar_base == Some(tcl_dialect::TclVersion::V9_1) {
            None
        } else {
            Some(tcl_dialect::TclVersion::V9_1)
        };
    let changed_unit = changed_unit.intern();
    let mut changed_analyser = input.analyser_profile().clone();
    changed_analyser.expr_grammar_base =
        if changed_analyser.expr_grammar_base == Some(tcl_dialect::TclVersion::V9_1) {
            None
        } else {
            Some(tcl_dialect::TclVersion::V9_1)
        };
    let changed_analyser = changed_analyser.intern();
    assert!(current.matches_current(source, &analysis, &input.context_registry()));
    for (analyser, unit) in [
        (input.analyser_profile(), changed_unit),
        (changed_analyser, input.unit_profile()),
    ] {
        let changed_input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            analyser,
            unit,
            input.context_registry(),
            config,
        );
        let changed = tcl_compiler::analyser::Analyser::new()
            .with_resolved_input(changed_input)
            .analyse(source, analyser.name);
        assert!(tcl_compiler::source_graph::current_analysis(source, &changed).is_some());
        assert!(!current.matches_current(source, &changed, &input.context_registry()));
    }
    let mut missing = analysis.clone();
    missing.resolved_input = None;
    assert!(
        tcl_irules::OriginalIrulesSourceContext::from_source_analysis(source, &missing).is_none()
    );
    assert!(
        tcl_irules::OriginalIrulesSourceContext::from_source_analysis("# changed", &analysis)
            .is_none()
    );
}

#[test]
fn request_hints_keep_literal_word_and_container_values() {
    let source = "when HTTP_REQUEST {pool p}";
    let registry = crate::environment::store_for_dialect("f5-irules");
    let captured = SourceInventory::capture(source, registry);
    let syntax = captured.context.as_ref().unwrap().expression_syntax();
    let config = syntax.0;
    for (condition, expected) in [
        (r#"[HTTP::uri] eq {"/literal"}"#, "\"/literal\""),
        (r"[HTTP::uri] eq {host[set ::hint 1]}", "host[set ::hint 1]"),
        (r"[HTTP::uri] eq {$literal}", "$literal"),
        (r#"[HTTP::uri] eq "one||two""#, "one||two"),
    ] {
        let condition = json!({"kind":"if", "condition":condition});
        let lines = build_request_setup("HTTP_REQUEST", &[&condition], Some(syntax));
        let line = lines.first().expect("literal request example");
        let parsed = tcl_lexer::native_script_words_in(
            tcl_lexer::SourceImage::document(line),
            tcl_lexer::Span::new(0, u32::try_from(line.len()).unwrap()),
            config.nested(),
        )
        .unwrap();
        assert!(parsed.fatal_tail.is_none());
        assert_eq!(parsed.commands.len(), 1);
        assert_eq!(parsed.commands[0].words.len(), 5);
        let uri = &parsed.commands[0].words[4];
        assert_eq!(
            tcl_syntax::word_rules::original_static_word_unicode_value(uri).as_deref(),
            Some(expected.as_bytes())
        );
        assert!(
            uri.executable_parts()
                .all_parts()
                .all(|part| matches!(part.part, tcl_lexer::ExecutablePart::Text(_)))
        );
    }
    for value in [
        "}",
        "\"host\"",
        "$literal",
        "host[set ::hint 1]",
        "a\\b",
        "a\nb",
    ] {
        let proposal = request_literal_word(value, config).unwrap();
        let word = sole_source_word(&proposal, config.nested()).unwrap();
        assert_eq!(
            tcl_syntax::word_rules::original_static_word_unicode_value(&word).as_deref(),
            Some(value.as_bytes())
        );
        let container = format!("{{{proposal}}}");
        let container = sole_source_word(&container, config.nested()).unwrap();
        assert_eq!(
            tcl_syntax::word_rules::original_static_word_unicode_value(&container).as_deref(),
            Some(proposal.as_bytes())
        );
    }
    let condition = json!({"kind":"if", "condition":r#"[HTTP::uri] eq "$dynamic""#});
    let lines = build_request_setup("HTTP_REQUEST", &[&condition], Some(syntax));
    assert_eq!(
        lines,
        ["::orch::run_http_request -host \"example.com\" -uri \"/\""]
    );
    assert!(build_request_setup("HTTP_REQUEST", &[&condition], None).is_empty());
    assert_eq!(
        cfg_pattern_to_value("\"quoted\"").as_deref(),
        Some("\"quoted\"")
    );
    assert_eq!(
        cfg_pattern_to_value("{literal}").as_deref(),
        Some("{literal}")
    );
    let parsed = tcl_syntax::expr::parser::parse_expr_with_syntax_context(
        r#"[HTTP::header {X Literal}] eq "yes""#,
        &syntax.1,
    );
    let tcl_syntax::expr::ExprNode::Binary { left, .. } = parsed else {
        panic!("original expression topology");
    };
    assert_eq!(
        source_header_hint(&left, config).as_deref(),
        Some("X Literal")
    );
    let parsed = tcl_syntax::expr::parser::parse_expr_with_syntax_context(
        r#"[HTTP::header $dynamic] eq "yes""#,
        &syntax.1,
    );
    let tcl_syntax::expr::ExprNode::Binary { left, .. } = parsed else {
        panic!("original expression topology");
    };
    assert!(source_header_hint(&left, config).is_none());
}
