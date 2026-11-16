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

//! Formatting a line selection inside the script region that holds it.

use super::super::config::FormatterConfig;
use super::argument_roles::identify_body_args;
use super::command_parsing::parse_commands;
use super::command_reconstruction::case_list_body_index;
use super::{ArgKind, FormattingRegion, FormattingSourceLayout, MAX_FORMAT_DEPTH, ParsedCommand};
use tcl_lexer::{Lexer, SourceMap};
use tcl_registry::CommandRegistry;

/// Script regions selected by the same source presentation used for whole-file
/// formatting. Whole original words supply geometry; schema roles supply only
/// readonly applicability, independently from handler execution.
fn presentation_children(
    command: &ParsedCommand,
    words: &[tcl_lexer::NativeWord],
    source_offset: u32,
    config: &FormatterConfig,
    registry: &CommandRegistry,
    identities: &FormattingSourceLayout,
) -> Option<Vec<tcl_lexer::Span>> {
    // naming.editor.original-source-whitespace-geometry
    // docs/design/analysis/name-resolution-proofs/original-source-whitespace-geometry.md
    use tcl_lexer::{ExecutablePart, Span, WordKind};
    let mut children = Vec::new();
    for word in words {
        for part in word.executable_parts().all_parts() {
            if let ExecutablePart::Command { body } = part.part {
                children.push(body);
            }
        }
    }
    let case = case_list_body_index(command, config, registry);
    if let Some(source) = &command.source_words {
        let context = identities.input.context_registry();
        for body in source.source_script_bodies(&context) {
            let parent = body.original_container();
            if parent.group().kind != WordKind::Braced
                || parent.group().expand
                || !body.matches_source(parent.image(), config.lexer_config())
                || !body.matches_context(&context)
            {
                continue;
            }
            let written = command.args.iter().position(|argument| {
                argument.tokens.first().is_some_and(|first| {
                    source_offset
                        .checked_add(first.span.start())
                        .and_then(|offset| identities.original_offset(offset))
                        == Some(parent.span().start())
                })
            });
            if let Some(written) = written
                && (command.args[written].kind == ArgKind::Body
                    || case.is_some_and(|(ordinal, _)| ordinal == written))
            {
                children.push(identities.presentation_span(body.content_span())?);
            }
        }
    }
    for (written, argument) in command.args.iter().enumerate().skip(1) {
        let Some(first) = argument.tokens.first() else {
            continue;
        };
        let start = source_offset.checked_add(first.span.start())?;
        let Some(word) = words.iter().find(|word| word.span().start() == start) else {
            continue;
        };
        if word.group().kind != WordKind::Braced || word.group().expand {
            continue;
        }
        if command.source_words.is_some() {
            if argument.kind == ArgKind::LambdaLiteral
                && let Some(original) = argument.original_word.as_ref()
                && let Some(body) =
                    tcl_compiler::lambda_literal::split_original_lambda_literal(original)
                        .and_then(|lambda| lambda.braced_body())
            {
                children.push(identities.presentation_span(body)?);
            }
            continue;
        }
        if let Some((_, descriptor)) = case.filter(|(ordinal, _)| *ordinal == written) {
            let content = word.content_span().ok()?;
            let source = word.image().try_text().ok()?;
            let text = source.get(content.as_range())?;
            let shape = tcl_syntax::case_list::CaseListShape {
                clause_flags: descriptor.clause_flags,
                clause_value_flags: descriptor.clause_value_flags,
            };
            for clause in tcl_syntax::case_list::split_case_list_with_syntax(
                text,
                &shape,
                config.lexer_config().list_parse,
            ) {
                if !clause.valid {
                    continue;
                }
                let Some(body) = clause.body.filter(|body| body.braced) else {
                    continue;
                };
                let range = body.content_range();
                children.push(Span::new(
                    content
                        .start()
                        .checked_add(u32::try_from(range.start).ok()?)?,
                    content
                        .start()
                        .checked_add(u32::try_from(range.end).ok()?)?,
                ));
            }
        } else if argument.kind == ArgKind::Body {
            children.push(word.content_span().ok()?);
        } else if argument.kind == ArgKind::LambdaLiteral
            && let Some(body) = tcl_compiler::lambda_literal::split_original_lambda_literal(word)
                .and_then(|lambda| lambda.braced_body())
        {
            children.push(body);
        }
    }
    Some(children)
}

/// Format a line selection only inside a genuine selected script region and
/// across complete lexical words. Data words and unavailable bodies are kept.
pub(crate) fn formatting_range_region(
    source: &str,
    selection: tcl_lexer::Span,
    config: &FormatterConfig,
    registry: &CommandRegistry,
    identities: &FormattingSourceLayout,
) -> Option<FormattingRegion> {
    // naming.editor.original-source-whitespace-geometry
    // docs/design/analysis/name-resolution-proofs/original-source-whitespace-geometry.md
    use tcl_lexer::{SourceImage, Span};
    let image = SourceImage::document(source);
    let end = u32::try_from(source.len()).ok()?;
    if selection.end() > end
        || identities.input.lexer_config() != config.lexer_config()
        || identities
            .input
            .context_registry()
            .commands()
            .snapshot()
            .semantic_key()
            != registry.snapshot().semantic_key()
        || identities.original.as_ref().map_or_else(
            || {
                identities.analysis.as_ref().is_some_and(|analysis| {
                    !analysis.matches_original_source_image(&image, config.lexer_config())
                })
            },
            |original| original.text() != source,
        )
    {
        return None;
    }
    let mut region = Span::new(0, end);
    let mut depth = 0;
    loop {
        if MAX_FORMAT_DEPTH.exceeded(depth) {
            return None;
        }
        let plan = super::super::source_layout::words_in(&image, region, config.lexer_config())?;
        let text = source.get(region.as_range())?;
        let map = SourceMap::new(text);
        let local_config = if region == Span::new(0, end) {
            config.lexer_config()
        } else {
            config.lexer_config().nested()
        };
        let tokens = Lexer::with_config(text, local_config).tokenise_all().ok()?;
        let (mut commands, _) = parse_commands(text, &map, &tokens, local_config)?;
        let mut children = Vec::new();
        for command in &mut commands {
            identify_body_args(command, registry, identities, region.start(), config, text);
            let start = region.start().checked_add(command.head_start)?;
            let Some(words) = plan.commands.iter().find(|row| row.span.start() == start) else {
                continue;
            };
            children.extend(presentation_children(
                command,
                &words.words,
                region.start(),
                config,
                registry,
                identities,
            )?);
        }
        let child = children
            .into_iter()
            .filter(|child| {
                child.start() <= selection.start()
                    && selection.end() <= child.end()
                    && *child != region
            })
            .min_by_key(|child| child.end() - child.start());
        if let Some(child) = child {
            region = child;
            depth = depth.checked_add(1)?;
        } else {
            if !super::super::source_layout::selection_keeps_whole_words(&plan, selection) {
                return None;
            }
            return Some(FormattingRegion {
                source_offset: selection.start(),
                indent_level: usize::try_from(depth).ok()?,
                local_config: if region == Span::new(0, end) && selection.start() == region.start()
                {
                    config.lexer_config()
                } else {
                    config.lexer_config().nested()
                },
            });
        }
    }
}
