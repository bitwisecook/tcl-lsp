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

//! Formatting the bodies nested in a command: case lists, script bodies and lambda bodies.

use super::super::config::FormatterConfig;
use super::{CommandArg, FormattingRegion, FormattingSourceLayout, ParsedCommand, format_body};
use tcl_lexer::Token;
use tcl_registry::{CaseListSpec, CommandRegistry};

// Case-list body formatting

/// Format a registry-declared case list, recursing only into elements the
/// shared descriptor parser proved are action bodies.
pub(super) fn format_case_list_body(
    body_text: &str,
    body_source_offset: u32,
    case_list: &CaseListSpec,
    config: &FormatterConfig,
    registry: &CommandRegistry,
    identities: &FormattingSourceLayout,
    indent_level: usize,
) -> String {
    let shape = tcl_syntax::case_list::CaseListShape {
        clause_flags: case_list.clause_flags,
        clause_value_flags: case_list.clause_value_flags,
    };
    let clauses = tcl_syntax::case_list::split_case_list_with_syntax(
        body_text,
        &shape,
        config.lexer_config().list_parse,
    );
    let mut elements = clauses
        .iter()
        .flat_map(|clause| {
            clause
                .flags
                .iter()
                .copied()
                .chain(clause.pattern)
                .chain(clause.body)
        })
        .collect::<Vec<_>>();
    elements.sort_unstable_by_key(|element| element.start);
    elements.dedup_by_key(|element| element.start);
    if elements.is_empty() {
        return body_text.to_owned();
    }
    let action_starts = clauses
        .iter()
        .filter(|clause| clause.valid)
        .filter_map(|clause| clause.body.map(|body| body.start))
        .collect::<std::collections::BTreeSet<_>>();

    let indent = config.make_indent(indent_level);
    let inner_level = indent_level + 1;
    let mut lines: Vec<String> = Vec::new();
    let mut prefix = Vec::new();
    for element in elements {
        // `case_list::Element` includes braces in `start`, but (like Tcl's
        // list parser) reports the interior range for a quoted element.  A
        // formatter must reconstruct the original list *word*, not merely
        // its value: dropping quotes can split one pattern into several
        // elements and change the clause pairing.
        let quoted = !element.braced
            && element.start > 0
            && body_text.as_bytes().get(element.start - 1) == Some(&b'"')
            && body_text.as_bytes().get(element.end) == Some(&b'"');
        let raw_start = element.start - usize::from(quoted);
        let raw_end = element.end + usize::from(element.braced || quoted);
        let Some(raw) = body_text.get(raw_start..raw_end) else {
            return body_text.to_owned();
        };
        if action_starts.contains(&element.start) {
            if element.braced {
                let content_range = element.content_range();
                let Some(content) = body_text.get(content_range.clone()) else {
                    return body_text.to_owned();
                };
                if content == case_list.fallthrough_body.unwrap_or("\0") {
                    prefix.push(raw.to_owned());
                    lines.push(format!("{indent}{}", prefix.join(" ")));
                } else {
                    let action_source_offset = body_source_offset
                        .saturating_add(u32::try_from(content_range.start).unwrap_or(u32::MAX));
                    let formatted = format_body(
                        content,
                        config,
                        registry,
                        identities,
                        FormattingRegion::body(config, action_source_offset, inner_level),
                    );
                    let head = prefix.join(" ");
                    if formatted.trim().is_empty() {
                        lines.push(format!("{indent}{head} {{}}"));
                    } else {
                        lines.push(format!("{indent}{head} {{"));
                        lines.push(formatted);
                        lines.push(format!("{indent}}}"));
                    }
                }
            } else {
                prefix.push(raw.to_owned());
                lines.push(format!("{indent}{}", prefix.join(" ")));
            }
            prefix.clear();
        } else {
            prefix.push(raw.to_owned());
        }
    }
    if !prefix.is_empty() {
        lines.push(format!("{indent}{}", prefix.join(" ")));
    }
    lines.join("\n")
}

fn token_content_source_offset(source_offset: u32, token: Token) -> u32 {
    source_offset
        .saturating_add(token.span.start())
        .saturating_add(u32::from(token.content_offset))
}

fn lambda_body_source_offset(source: &str, source_offset: u32, token: Token) -> u32 {
    // For a braced lambda body this is its exact source position. A decoded
    // bare/quoted element can contract escapes, but the element's source start
    // still keeps outer binding facts on the correct side of the containing
    // command.
    tcl_compiler::lambda_literal::split_lambda_literal(source, token)
        .and_then(|elements| elements.body)
        .map_or_else(
            || token_content_source_offset(source_offset, token),
            |body| source_offset.saturating_add(body.start()),
        )
}

pub(super) fn format_plain_body_argument(
    arg: &mut CommandArg,
    case_list_body: Option<(usize, CaseListSpec)>,
    config: &FormatterConfig,
    registry: &CommandRegistry,
    identities: &FormattingSourceLayout,
    position: (usize, u32, usize),
) {
    let (a, source_offset, inner_level) = position;
    let body_text = arg.text.clone();
    let body_source_offset = arg.tokens.first().map_or(source_offset, |&token| {
        token_content_source_offset(source_offset, token)
    });
    let formatted = if let Some((_, case_list)) = case_list_body.filter(|(index, _)| *index == a) {
        format_case_list_body(
            &body_text,
            body_source_offset,
            &case_list,
            config,
            registry,
            identities,
            inner_level,
        )
    } else {
        format_body(
            &body_text,
            config,
            registry,
            identities,
            FormattingRegion::body(config, body_source_offset, inner_level),
        )
    };
    arg.formatted_body = Some(formatted);
}

fn original_formatted_lambda_body(
    arg: &CommandArg,
    identities: &FormattingSourceLayout,
    body_text: &str,
) -> Option<tcl_lexer::Span> {
    arg.original_word.as_ref().and_then(|word| {
        tcl_compiler::lambda_literal::split_original_lambda_literal(word)
            .and_then(|elements| elements.braced_body())
            .filter(|span| {
                word.image()
                    .try_text()
                    .ok()
                    .and_then(|text| text.get(span.as_range()))
                    == Some(body_text)
                    || identities.original.as_ref().and_then(|layout| {
                        layout
                            .presentation_span(*span)
                            .and_then(|span| layout.text().get(span.as_range()))
                    }) == Some(body_text)
            })
    })
}

pub(super) fn format_lambda_body_argument(
    selected: (&ParsedCommand, usize),
    position: (&str, u32, Token, usize),
    config: &FormatterConfig,
    registry: &CommandRegistry,
    identities: &FormattingSourceLayout,
    body_text: &str,
) -> (
    String,
    Option<std::sync::Arc<tcl_compiler::command_binding::OriginalConditionalLambdaFormattingBody>>,
) {
    let (command, arg_index) = selected;
    let (source, source_offset, tok, inner_level) = position;
    // Format only the real, *decoded* body element —
    // `render_lambda_literal_arg` re-derives the parameter list /
    // namespace elements from the original source at
    // reconstruction time and reassembles them. Decoding first
    // (rather than reformatting the raw source spelling) matters
    // for a non-literal (bare/quoted) body: its backslash escapes
    // must be collapsed before the result is parsed as a script,
    // exactly as Tcl's own list-then-script evaluation would.
    let original_body =
        original_formatted_lambda_body(&command.args[arg_index], identities, body_text);
    let presented_body = original_body.and_then(|span| identities.presentation_span(span));
    let lambda_body_offset = presented_body.map_or_else(
        || {
            if command.args[arg_index].original_word.is_some() {
                u32::MAX
            } else {
                lambda_body_source_offset(source, source_offset, tok)
            }
        },
        tcl_lexer::Span::start,
    );
    let presented_text = identities
        .original
        .as_ref()
        .and_then(|layout| layout.text().get(presented_body?.as_range()));
    let phase = original_body
        .is_none()
        .then(|| {
            let parent = command.args[arg_index].original_word.as_ref()?;
            command
                .source_words
                .as_ref()?
                .decoded_lambda_formatting_body(&identities.input, &identities.realm, parent)
        })
        .flatten()
        .filter(|phase| phase.text() == Some(body_text));
    let conditional_phase = original_body
        .is_none()
        .then(|| {
            let parent = command.args[arg_index].original_word.as_ref()?;
            if let Some(current) = &identities.conditional_phase {
                current.child(parent, &identities.input)
            } else {
                tcl_compiler::command_binding::OriginalConditionalLambdaFormattingBody::capture(
                    &identities.realm,
                    &identities.input,
                    parent,
                )
            }
        })
        .flatten()
        .filter(|phase| phase.image().try_text().ok() == Some(body_text));
    if let Some(phase) = phase {
        let base = phase.source_base();
        let phase_layout = FormattingSourceLayout {
            realm: phase.realm().clone(),
            analysis: None,
            input: identities.input.clone(),
            original: None,
            decoded_phase: Some(phase),
            conditional_phase: None,
        };
        (
            format_body(
                body_text,
                config,
                registry,
                &phase_layout,
                FormattingRegion::body(config, base, inner_level),
            ),
            conditional_phase,
        )
    } else if let Some(phase) = conditional_phase {
        let phase_layout = FormattingSourceLayout {
            realm: identities.realm.clone(),
            analysis: None,
            input: identities.input.clone(),
            original: None,
            decoded_phase: None,
            conditional_phase: Some(phase),
        };
        (
            format_body(
                body_text,
                config,
                registry,
                &phase_layout,
                FormattingRegion::body(config, 0, inner_level),
            ),
            None,
        )
    } else {
        (
            format_body(
                presented_text.unwrap_or(body_text),
                config,
                registry,
                identities,
                FormattingRegion::body(config, lambda_body_offset, inner_level),
            ),
            None,
        )
    }
}
