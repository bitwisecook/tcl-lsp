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

//! Complete-word metadata for explicitly selected Logical authoring passes.

use super::{MAX_MINIFY_DEPTH, MinifyEnv};
use tcl_lexer::{NativeScriptCommandWords, NativeWord, SourceChannel, SourceImage, Span};
use tcl_registry::{ArgRole, ResolvedInvocation};

/// Same complete words and original before-argv Realm, without Native grants.
pub(super) fn with_command_schema<T>(
    env: MinifyEnv<'_>,
    command: &NativeScriptCommandWords,
    project: impl FnOnce(&ResolvedInvocation<'_, '_>) -> T,
) -> Option<T> {
    // naming.minifier.complete-logical-metadata
    // docs/design/analysis/name-resolution-proofs/minifier-complete-logical-metadata.md
    let words = tcl_compiler::registry_invocation::source_structure::original_logical_registry_words_for_command(
        env.identities, env.input, command,
    )?;
    // The present lexical editing loops address complete written ordinals.
    // Captured alias prefixes require their own effective-origin mapping.
    if words.origins().iter().enumerate().any(|(ordinal, origin)| {
        origin != &tcl_compiler::registry_invocation::InvocationWordOrigin::Written(ordinal)
    }) {
        return None;
    }
    words.with_source_schema(env.context, project)
}

/// Editing needs its own closed source-transition applicability. Readonly
/// roles remain available when an earlier callback or operation is unknown.
pub(super) fn with_editable_command_schema<T>(
    env: MinifyEnv<'_>,
    command: &NativeScriptCommandWords,
    project: impl FnOnce(&ResolvedInvocation<'_, '_>) -> T,
) -> Option<T> {
    // naming.minifier.complete-logical-metadata
    // docs/design/analysis/name-resolution-proofs/minifier-complete-logical-metadata.md
    let words = tcl_compiler::registry_invocation::source_structure::original_logical_registry_words_for_command(
        env.identities, env.input, command,
    )?;
    let tcl_compiler::registry_invocation::source_structure::OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
        return None;
    };
    if advice.logical_source_input() != Some(env.input)
        || advice.obligations().contains(
            &tcl_compiler::command_binding::SourceCommandTransitionObligation::UnknownEarlierMutation,
        )
        || !advice.uncertain_operations().is_empty()
    {
        return None;
    }
    with_command_schema(env, command, project)
}

pub(super) fn ascii_value(word: &NativeWord) -> Option<String> {
    String::from_utf8(tcl_syntax::word_rules::original_static_word_ascii_presentation(word)?).ok()
}

/// Original command regions from selected source roles and executable brackets.
/// Data braces never become scripts merely because they contain Tcl-like text.
pub(super) fn commands(source: &str, env: MinifyEnv<'_>) -> Option<Vec<NativeScriptCommandWords>> {
    // naming.minifier.complete-logical-metadata
    // docs/design/analysis/name-resolution-proofs/minifier-complete-logical-metadata.md
    let image = SourceImage::document(source);
    let mut pending = vec![(Span::new(0, u32::try_from(source.len()).ok()?), 0)];
    let mut visited = rustc_hash::FxHashSet::default();
    let mut commands = Vec::new();
    while let Some((region, depth)) = pending.pop() {
        if MAX_MINIFY_DEPTH.exceeded(depth) {
            return None;
        }
        if !visited.insert(region) {
            continue;
        }
        let plan = tcl_lexer::native_script_words_in(image.clone(), region, env.config).ok()?;
        if plan.fatal_tail.is_some() {
            return None;
        }
        for command in plan.commands {
            for word in &command.words {
                for part in word.executable_parts().all_parts() {
                    if let tcl_lexer::ExecutablePart::Command { body } = part.part {
                        pending.push((body, depth + 1));
                    }
                }
            }
            if let Some(regions) = with_command_schema(env, &command, |schema| {
                nested_regions(source, &command.words, schema, env)
            }) {
                pending.extend(regions.into_iter().map(|region| (region, depth + 1)));
            }
            commands.push(command);
        }
    }
    commands.sort_by_key(|command| command.span.start());
    Some(commands)
}

fn nested_regions(
    source: &str,
    words: &[NativeWord],
    schema: &ResolvedInvocation<'_, '_>,
    env: MinifyEnv<'_>,
) -> Vec<Span> {
    let mut regions = Vec::new();
    let case = schema.authored_source_case_invocation();
    let list = case
        .as_ref()
        .and_then(|(_, layout)| layout.clause_list_index);
    if let Some((spec, layout)) = case {
        if let Some(index) = layout.clause_list_index {
            if let Some(word) = index.checked_add(1).and_then(|index| words.get(index)) {
                regions.extend(case_regions(source, word, &spec));
            }
        } else if let Some(start) = layout.inline_clause_start {
            let values = words.iter().skip(1).map(ascii_value).collect::<Vec<_>>();
            let spellings = values
                .iter()
                .map(Option::as_deref)
                .collect::<Option<Vec<_>>>();
            if let Some(clauses) = spellings
                .as_ref()
                .and_then(|values| spec.inline_clauses(values, start))
            {
                regions.extend(clauses.into_iter().filter_map(|clause| {
                    clause
                        .body_index?
                        .checked_add(1)
                        .and_then(|index| words.get(index))
                        .and_then(unchanged_body)
                }));
            }
        }
    }
    let (roles, complete) = schema.authored_source_argument_roles();
    if !complete {
        return regions;
    }
    for (index, role) in roles {
        let Some(argument) = schema
            .semantics
            .argument_offset
            .checked_add(usize::from(index))
        else {
            continue;
        };
        if list == Some(argument) {
            continue;
        }
        let Some(word) = argument.checked_add(1).and_then(|index| words.get(index)) else {
            continue;
        };
        match role {
            ArgRole::Body => regions.extend(unchanged_body(word)),
            ArgRole::LambdaLiteral if unchanged_body(word).is_some() => {
                regions.extend(
                    tcl_compiler::lambda_literal::split_original_lambda_literal(word)
                        .and_then(|lambda| lambda.braced_body()),
                );
            }
            ArgRole::Expr => regions.extend(expression_regions(source, word, env)),
            _ => {}
        }
    }
    regions
}

fn unchanged_body(word: &NativeWord) -> Option<Span> {
    if word.group().expand || word.image().channel() != SourceChannel::Document {
        return None;
    }
    let content = word.content_span().ok()?;
    let raw = word.image().bytes().get(content.as_range())?;
    match word.group().kind {
        tcl_lexer::WordKind::Braced => (tcl_syntax::backslash::source_braced_word_bytes(
            raw,
            SourceChannel::Document,
            word.config().brace_backslash_newline,
        )
        .as_ref()
            == raw)
            .then_some(content),
        tcl_lexer::WordKind::Quoted
            if word.executable_parts().all_parts().all(|part| {
                matches!(
                    part.part,
                    tcl_lexer::ExecutablePart::Text(tcl_lexer::ExecutableText::Original)
                )
            }) =>
        {
            Some(content)
        }
        _ => None,
    }
}

fn case_regions(source: &str, word: &NativeWord, spec: &tcl_registry::CaseListSpec) -> Vec<Span> {
    let Some(content) =
        unchanged_body(word).filter(|_| word.group().kind == tcl_lexer::WordKind::Braced)
    else {
        return Vec::new();
    };
    let Some(inner) = source.get(content.as_range()) else {
        return Vec::new();
    };
    let shape = tcl_syntax::case_list::CaseListShape {
        clause_flags: spec.clause_flags,
        clause_value_flags: spec.clause_value_flags,
    };
    let clauses =
        tcl_syntax::case_list::split_case_list_with_syntax(inner, &shape, word.config().list_parse);
    if clauses
        .iter()
        .any(|clause| !clause.valid || clause.body.is_none())
    {
        return Vec::new();
    }
    clauses
        .into_iter()
        .filter_map(|clause| {
            let body = clause.body.filter(|body| body.braced)?;
            let range = body.content_range();
            Some(Span::new(
                content
                    .start()
                    .checked_add(u32::try_from(range.start).ok()?)?,
                content
                    .start()
                    .checked_add(u32::try_from(range.end).ok()?)?,
            ))
        })
        .collect()
}

fn expression_regions(source: &str, word: &NativeWord, env: MinifyEnv<'_>) -> Vec<Span> {
    let Some(content) = unchanged_body(word) else {
        return Vec::new();
    };
    let Some(expression) = source.as_bytes().get(content.as_range()) else {
        return Vec::new();
    };
    let (tokens, unknown) = tcl_lexer::tokenise_expr_bytes_checked_with_expression_grammar(
        expression,
        &env.config.grammar_over(env.dialect.grammar),
        env.dialect.expr_grammar_base,
        env.dialect.f5_core_expr_grammar(),
    );
    if unknown {
        return Vec::new();
    }
    let Some(terms) = tcl_lexer::expression_terms(expression, &tokens, env.config) else {
        return Vec::new();
    };
    terms
        .into_iter()
        .filter(|term| term.kind == tcl_lexer::ExprTermKind::Command)
        .filter_map(|term| {
            Some(Span::new(
                content.start().checked_add(term.value.start)?,
                content.start().checked_add(term.value.end)?,
            ))
        })
        .collect()
}
