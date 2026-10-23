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

//! Conditional source reference candidates in one genuine caller-frame body.

use super::OriginalSourceScriptBody;
use super::source_structure::source_registry_words_at;
use crate::analyser::AnalysisResult;
use std::collections::BTreeSet;
use tcl_lexer::{ExecutablePart, NativeWord, SourceImage, Span};

const MAX_REGIONS: usize = 2048;

enum SourceRegion {
    Script(Span),
    Expression(Span),
}
impl SourceRegion {
    fn key(&self) -> (bool, u32, u32) {
        match self {
            Self::Script(span) => (false, span.start(), span.end()),
            Self::Expression(span) => (true, span.start(), span.end()),
        }
    }
}

/// Lexical MAY names in an original body and independently selected inherited
/// bodies. This inventory proves no completed read, formal value or Native cell.
pub(crate) fn original_body_reference_candidates(
    source: &str,
    analysis: &AnalysisResult,
    body: &OriginalSourceScriptBody,
) -> Option<BTreeSet<String>> {
    let input = analysis.resolved_input.as_ref()?;
    let image = SourceImage::document(source);
    let config = input.lexer_config();
    let context = input.context_registry();
    if !analysis.matches_original_source_image(&image, config)
        || !body.matches_source(&image, config)
        || !body.matches_context(&context)
    {
        return None;
    }
    let header = body.source_words().head_source()?.span().start();
    let current = source_registry_words_at(source, analysis, header)?;
    current
        .source_script_bodies(&context)
        .iter()
        .any(|candidate| candidate == body)
        .then_some(())?;
    let mut names = BTreeSet::new();
    let mut parser = tcl_registry::InvocationDialect::of_profile(input.unit_profile())
        .expression_parse_context(Some(input.unit_profile()));
    parser.lexer_grammar = config.grammar_over(parser.lexer_grammar);
    let mut work = vec![SourceRegion::Script(body.content_span())];
    let mut visited = BTreeSet::new();
    while let Some(region) = work.pop() {
        if !visited.insert(region.key()) {
            continue;
        }
        if visited.len() > MAX_REGIONS {
            return None;
        }
        let region = match region {
            SourceRegion::Script(span) => span,
            SourceRegion::Expression(span) => {
                retain_expression_span(&image, span, &parser, config, &mut names, &mut work)?;
                continue;
            }
        };
        let plan = tcl_lexer::native_script_words_in_with_purpose(
            image.clone(),
            region,
            config,
            tcl_lexer::SourceSegmentationPurpose::NestedScript,
        )
        .ok()?;
        if plan.fatal_tail.is_some() || plan.commands.len() > MAX_REGIONS {
            return None;
        }
        for command in plan.commands {
            for word in &command.words {
                retain_word_references(word, &mut names, &mut work);
            }
            let Some(selected) = source_registry_words_at(source, analysis, command.span.start())
            else {
                continue;
            };
            retain_selected_expressions(&selected, input, &parser, &mut names, &mut work);
            for child in selected
                .source_expression_script_bodies(input)
                .unwrap_or_default()
            {
                work.push(SourceRegion::Script(child.content_span()));
            }
            for child in selected.source_script_bodies(&context) {
                if child.inherits_immediate_caller_source_context(&context) {
                    work.push(SourceRegion::Script(child.content_span()));
                }
            }
        }
    }
    Some(names)
}

fn retain_selected_expressions(
    selected: &super::source_structure::OriginalRegistryWords,
    input: &crate::analyser::ResolvedAnalysisInput,
    parser: &tcl_syntax::expr::parser::ExprParseContext,
    names: &mut BTreeSet<String>,
    regions: &mut Vec<SourceRegion>,
) {
    let config = input.lexer_config();
    for &(argument, role) in selected.roles().unwrap_or_default() {
        if role != tcl_registry::ArgRole::Expr
            || !matches!(
                selected.origins().get(argument + 1),
                Some(super::InvocationWordOrigin::Written(_))
            )
        {
            continue;
        }
        let Some(word) = selected.original_argument_word(argument) else {
            continue;
        };
        let Some(value) = tcl_syntax::word_rules::original_static_word_unicode_value(word) else {
            continue;
        };
        let Ok(value) = std::str::from_utf8(&value) else {
            continue;
        };
        let expression = tcl_syntax::expr::parser::parse_expr_with_syntax_context(value, parser);
        names.extend(expression.vars_with_config(config));
        if let Ok(content) = word.content_span()
            && word.image().bytes().get(content.as_range()) == Some(value.as_bytes())
        {
            regions.push(SourceRegion::Expression(content));
        }
    }
}

fn retain_word_references(
    word: &NativeWord,
    names: &mut BTreeSet<String>,
    scripts: &mut Vec<SourceRegion>,
) {
    let arena = word.executable_parts();
    for component in arena.all_parts() {
        match component.part {
            ExecutablePart::Variable { name, .. } => {
                if let Some(name) = arena
                    .bytes(name)
                    .and_then(|name| std::str::from_utf8(name).ok())
                {
                    names.insert(
                        tcl_syntax::naming::split_element_ref(name)
                            .map_or(name, |(root, _)| root)
                            .to_owned(),
                    );
                }
            }
            ExecutablePart::Command { body } => scripts.push(SourceRegion::Script(body)),
            ExecutablePart::Expression { expression } => {
                scripts.push(SourceRegion::Expression(expression));
            }
            ExecutablePart::Text(_) | ExecutablePart::ParseError(_) => {}
        }
    }
}

fn retain_expression_span(
    image: &SourceImage,
    span: Span,
    parser: &tcl_syntax::expr::parser::ExprParseContext,
    config: tcl_lexer::LexerConfig,
    names: &mut BTreeSet<String>,
    work: &mut Vec<SourceRegion>,
) -> Option<()> {
    let bytes = image.bytes().get(span.as_range())?;
    let value = std::str::from_utf8(bytes).ok()?;
    names.extend(
        tcl_syntax::expr::parser::parse_expr_with_syntax_context(value, parser)
            .vars_with_config(config),
    );
    let (tokens, _) = tcl_lexer::tokenise_expr_bytes_with_failures(
        bytes,
        &parser.lexer_grammar,
        parser.expr_grammar_base,
        parser.f5_word_grammar,
    );
    for term in tcl_lexer::expression_terms(bytes, &tokens, config)? {
        let content = Span::new(
            span.start().checked_add(term.value.start)?,
            span.start().checked_add(term.value.end)?,
        );
        match term.kind {
            tcl_lexer::ExprTermKind::Command => work.push(SourceRegion::Script(content)),
            tcl_lexer::ExprTermKind::EscapedString => {
                let arena = tcl_lexer::ExecutablePartArena::decompose(
                    image.clone(),
                    content,
                    tcl_lexer::word_parts::SubstFlags::default(),
                    config,
                )
                .ok()?;
                for part in arena.all_parts() {
                    match part.part {
                        ExecutablePart::Command { body } => work.push(SourceRegion::Script(body)),
                        ExecutablePart::Expression { expression } => {
                            work.push(SourceRegion::Expression(expression));
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(
        body: &str,
        config: tcl_lexer::LexerConfig,
    ) -> (String, AnalysisResult, OriginalSourceScriptBody) {
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let context =
            crate::environment_ingress::resolve_environment("tcl8.6").default_context_registry();
        let input =
            crate::analyser::ResolvedAnalysisInput::new(profile, profile, context.clone(), config);
        let source =
            tcl_syntax::list::join_list(["proc", "fixture", "parameter {odd name} {}", body]);
        let analysis = crate::analyser::Analyser::new()
            .with_resolved_input(input)
            .analyse(&source, "tcl");
        let words = source_registry_words_at(&source, &analysis, 0).unwrap();
        let mut bodies = words.source_script_bodies(&context).into_iter();
        let body = bodies.next().unwrap();
        assert!(bodies.next().is_none());
        (source, analysis, body)
    }

    #[test]
    fn original_body_candidates_use_configured_references_and_selected_inherited_bodies() {
        for dialect in ["tcl8.6", "tcl9.1", "jim"] {
            let config = tcl_lexer::LexerConfig::for_dialect(dialect);
            for (body_text, name, found) in [
                ("puts ${odd name}", "odd name", true),
                ("puts ${}", "", true),
                ("return [expr {$parameter + 1}]", "parameter", true),
                ("if 1 {puts $parameter}", "parameter", true),
                ("set inert {$parameter}", "parameter", false),
                ("puts \\$parameter", "parameter", false),
                ("namespace eval N {puts $parameter}", "parameter", false),
                (
                    "\u{feff}namespace eval N {puts $parameter}",
                    "parameter",
                    false,
                ),
                (
                    "apply {parameter {puts $parameter}} OTHER",
                    "parameter",
                    false,
                ),
                (
                    "proc nested parameter {puts $parameter}",
                    "parameter",
                    false,
                ),
            ] {
                let (source, analysis, body) = fixture(body_text, config);
                let names = original_body_reference_candidates(&source, &analysis, &body).unwrap();
                assert_eq!(names.contains(name), found, "{dialect}: {body_text}");
            }
        }
    }

    #[test]
    fn original_body_candidates_retain_immediate_expression_sugar_and_nested_commands() {
        let config = tcl_lexer::LexerConfig::for_dialect("jim");
        for text in [
            "return $($parameter + 1)",
            "return $([list $parameter] + 1)",
        ] {
            let (source, analysis, body) = fixture(text, config);
            assert!(
                original_body_reference_candidates(&source, &analysis, &body)
                    .unwrap()
                    .contains("parameter"),
                "{text}"
            );
        }
    }

    #[test]
    fn original_body_candidates_refuse_stale_source_input_and_unowned_role_data() {
        let config = tcl_lexer::LexerConfig::default();
        let (source, analysis, body) = fixture("puts $parameter", config);
        assert!(
            original_body_reference_candidates(&source, &analysis, &body)
                .unwrap()
                .contains("parameter")
        );
        assert!(
            original_body_reference_candidates(&format!("{source} "), &analysis, &body).is_none()
        );
        let mut missing = analysis.clone();
        missing.resolved_input = None;
        assert!(original_body_reference_candidates(&source, &missing, &body).is_none());
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let foreign_input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            crate::environment_ingress::resolve_environment("tcl9.1").default_context_registry(),
            config,
        );
        let foreign = crate::analyser::Analyser::new()
            .with_resolved_input(foreign_input)
            .analyse(&source, "tcl");
        assert!(foreign.resolved_input != analysis.resolved_input);
        assert!(foreign.matches_original_source_image(&SourceImage::document(&source), config));
        assert!(original_body_reference_candidates(&source, &foreign, &body).is_none());
        let (source, analysis, body) = fixture("unknown {$parameter}", config);
        assert!(
            !original_body_reference_candidates(&source, &analysis, &body)
                .unwrap()
                .contains("parameter")
        );
    }
}
