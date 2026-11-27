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

//! The original syntax regions of a command that can contribute memo-summary dependencies.

use crate::ir::CommandTokens;
use tcl_lexer::Span;

/// Original syntax regions that may contribute memo-summary dependencies.
/// The caller still owns current Module/target selection and every residual.
pub(crate) struct OriginalSourceDependencyRegions {
    pub(crate) substitutions: Vec<Span>,
    pub(crate) scripts: Vec<Span>,
    pub(crate) unknown: bool,
}

/// Original complete words and selected source roles distinguish evaluated
/// syntax from bracket text in data. No invocation or frame is admitted.
pub(crate) fn original_source_dependency_regions(
    tokens: &CommandTokens,
    words: &[tcl_lexer::NativeWord],
    registry: &tcl_registry::CommandRegistry,
    profile: Option<&tcl_dialect::DialectProfile>,
) -> Option<OriginalSourceDependencyRegions> {
    if tokens.synthetic.is_some() || !tokens.original_words_match_argv_text() {
        return None;
    }
    let first = words.first()?;
    let image = first.image();
    let config = first.config();
    let original = crate::registry_invocation::original_native_compiler_words(
        image,
        tokens.words(),
        first.span().start(),
        config,
    )?;
    (original == words).then_some(())?;
    let mut out = OriginalSourceDependencyRegions {
        substitutions: Vec::new(),
        scripts: Vec::new(),
        unknown: false,
    };
    let mut expressions = Vec::new();
    for word in words {
        dependency_parts(word.executable_parts(), &mut expressions, &mut out);
    }
    let selected = crate::registry_invocation::original_source_name_roles(registry, tokens);
    if words.iter().any(|word| word.group().expand)
        || selected
            .as_ref()
            .is_some_and(|selected| !selected.direct_written_arguments())
    {
        out.unknown = true;
        return Some(out);
    }
    if let Some(selected) = selected.as_ref() {
        out.unknown |= !selected.roles_complete()
            && words
                .iter()
                .skip(1)
                .any(|word| word.bytes().contains(&b'['));
        if selected
            .facts()
            .traits
            .contains(tcl_registry::Traits::PERFORMS_SUBSTITUTION)
        {
            out.unknown = true;
        }
        for &(argument, role) in selected.roles() {
            if !role.carries_script() && role != tcl_registry::ArgRole::CommandPrefix {
                continue;
            }
            let word = words.get(argument.checked_add(1)?)?;
            let content = word.content_span().ok()?;
            let original = image.bytes().get(content.as_range())?;
            let unchanged = tcl_syntax::word_rules::original_static_word_unicode_value(word)
                .is_some_and(|value| value.as_slice() == original);
            if !unchanged || word.group().expand {
                out.unknown = true;
                continue;
            }
            if !original.contains(&b'[') {
                continue;
            }
            out.unknown |= selected.has_unknown_residual();
            match role {
                tcl_registry::ArgRole::Expr => expressions.push(content),
                tcl_registry::ArgRole::Body
                    if selected.facts().body_kind == tcl_registry::BodyKind::Plain
                        && selected.facts().body_interpreter
                            == tcl_registry::BodyInterpreter::Current =>
                {
                    out.scripts.push(content);
                }
                _ => out.unknown = true,
            }
        }
    } else {
        out.unknown |= words
            .iter()
            .skip(1)
            .any(|word| word.bytes().contains(&b'['));
    }
    collect_original_expression_dependency_regions(image, config, profile, expressions, &mut out)?;
    Some(out)
}

pub(super) fn collect_original_expression_dependency_regions(
    image: &tcl_lexer::SourceImage,
    config: tcl_lexer::LexerConfig,
    profile: Option<&tcl_dialect::DialectProfile>,
    mut expressions: Vec<Span>,
    out: &mut OriginalSourceDependencyRegions,
) -> Option<()> {
    expressions.retain(|span| {
        image
            .bytes()
            .get(span.as_range())
            .is_some_and(|bytes| bytes.contains(&b'['))
    });
    let mut depth = 0;
    while !expressions.is_empty() {
        if tcl_source::limits::MAX_BRACKET_TEXT_DEPTH.exceeded(depth) {
            out.unknown = true;
            break;
        }
        let Some(profile) = profile else {
            out.unknown = true;
            break;
        };
        let mut nested = Vec::new();
        for span in expressions {
            let expression = image.bytes().get(span.as_range())?;
            let (tokens, unknown) = tcl_lexer::tokenise_expr_bytes_checked_with_expression_grammar(
                expression,
                &config.grammar_over(profile.grammar),
                profile.expr_grammar_base,
                profile.f5_core_expr_grammar(),
            );
            out.unknown |= unknown;
            let Some(terms) = tcl_lexer::expression_terms(expression, &tokens, config) else {
                out.unknown = true;
                continue;
            };
            for term in terms {
                let relative = match term.kind {
                    tcl_lexer::ExprTermKind::Variable
                    | tcl_lexer::ExprTermKind::IndexedVariable
                    | tcl_lexer::ExprTermKind::Command => term.source,
                    tcl_lexer::ExprTermKind::EscapedString => term.value,
                    tcl_lexer::ExprTermKind::Number
                    | tcl_lexer::ExprTermKind::Boolean
                    | tcl_lexer::ExprTermKind::String => continue,
                };
                let part = Span::new(
                    span.start().checked_add(relative.start)?,
                    span.start().checked_add(relative.end)?,
                );
                let arena = tcl_lexer::ExecutablePartArena::decompose(
                    image.clone(),
                    part,
                    tcl_lexer::word_parts::SubstFlags::default(),
                    config,
                )
                .ok()?;
                dependency_parts(&arena, &mut nested, out);
            }
        }
        expressions = nested;
        depth += 1;
    }
    Some(())
}

fn dependency_parts(
    arena: &tcl_lexer::ExecutablePartArena,
    expressions: &mut Vec<Span>,
    out: &mut OriginalSourceDependencyRegions,
) {
    for part in arena.all_parts() {
        match part.part {
            tcl_lexer::ExecutablePart::Command { body } => out.substitutions.push(body),
            tcl_lexer::ExecutablePart::Expression { expression } => expressions.push(expression),
            tcl_lexer::ExecutablePart::ParseError(_) => out.unknown = true,
            tcl_lexer::ExecutablePart::Text(_) | tcl_lexer::ExecutablePart::Variable { .. } => {}
        }
    }
}
