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

//! One tokenised command as words: reconstructed text, written geometry and whether the value comes from run-time data.

use crate::ir::{CommandTokens, WordExpr, WordPart};
use crate::segmenter::SegmentedCommand;
use tcl_lexer::{LexerConfig, SourceMap, TokenType};
use tcl_registry::InvocationWord;

#[derive(Debug, Clone)]
pub(super) enum WrittenWordGeometry {
    Missing,
    Present,
}

/// One word of a tokenised command, in both spellings its consumers need.
///
/// The segmenter's reconstructed word text canonicalises a substitution to its
/// braced spelling (`$x` → `${x}`) and de-braces a braced word (`{s}` → `s`),
/// which is what a name-harvesting caller wants but which erases the
/// distinction that decides whether a word *is* a name: `set {$x} 1` names a
/// variable literally called `$x`.  Carrying the written spelling plus the two
/// quoting facts lets every consumer ask its own question of one tokenisation.
#[derive(Debug, Clone)]
pub(crate) struct CommandWord {
    /// The segmenter's reconstructed word text — `$x` → `${x}`, `{s}` → `s`.
    /// The same spelling [`crate::ir::Statement`] arguments carry, so a
    /// consumer reads one text convention whichever side it arrives from.
    pub text: String,
    /// The word's **verbatim** source spelling, its whole written span — `$x`
    /// stays `$x`; original delimiters and expansion markers remain complete.
    /// Missing written geometry is unavailable, never a static literal.
    pub raw: String,
    pub(super) geometry: WrittenWordGeometry,
    /// Some constituent fragment is a `$` / `[` substitution, so the word's
    /// value is not its source text.
    pub substituted: bool,
    /// Tcl `{*}` expands this source word into zero or more argv entries.
    /// This is an argv-shape fact independent of whether the wrapped word
    /// itself substitutes (`{*}$x`) or is literal (`{*}{a b}`).
    pub expanded: bool,
    /// The word is a single brace-quoted token (`{…}`): Tcl leaves its
    /// content literal, so a `$` inside is source text, not a substitution.
    pub braced_literal: bool,
}

impl CommandWord {
    pub(super) fn has_source_geometry(&self) -> bool {
        matches!(self.geometry, WrittenWordGeometry::Present)
    }

    /// Project this source word into the registry's value-conservative
    /// invocation vocabulary. Expansion takes precedence because it changes
    /// argv shape even when the wrapped value is otherwise literal.
    #[must_use]
    pub(crate) fn invocation_word(&self) -> InvocationWord<'_> {
        if !self.has_source_geometry() {
            InvocationWord::Dynamic
        } else if self.expanded {
            InvocationWord::Expanded
        } else if self.substituted {
            InvocationWord::Dynamic
        } else {
            InvocationWord::Literal(&self.text)
        }
    }

    /// The single, statically-known Tcl value contributed by this word.
    #[must_use]
    pub(crate) fn literal(&self) -> Option<&str> {
        self.invocation_word().literal()
    }
}

/// Tokenise source text into a list of commands, each a list of words.
///
/// One splitter, one dialect: this is
/// [`crate::segmenter::segment_commands_with_offset_and_config`] mapped onto
/// the four facts the name-harvesting consumers ask for.  `config` decides
/// where the word boundaries fall — `{*}` expands only where the dialect has
/// it (never under Tcl 8.4 or iRules), and only iRules treats `}{` as a word
/// separator, so `cmd {a}{b}` is two words in Tcl and three in an iRule.  A
/// second, differently-configured tokenisation would disagree with the IR
/// about *which* argument sits in a variable-name position, which is a wrong
/// fact rather than a coarse one for a caller such as
/// [`crate::dynamic_names`]. These callers supply an extracted command or
/// materialized body, so local parsing selects `NestedScript` while command
/// words and value rules retain the complete caller configuration.
pub(crate) fn tokenise_command_words(source: &str, config: LexerConfig) -> Vec<Vec<CommandWord>> {
    let sm = SourceMap::new(source);
    crate::segmenter::segment_commands_with_offset_and_config(
        source,
        0,
        tcl_lexer::SourceSegmentationPurpose::NestedScript.config(config),
    )
    .iter()
    .filter(|command| !command.argv.is_empty())
    .map(|command| command_words(&sm, config, command))
    .collect()
}

/// Map one segmented command onto its per-word [`CommandWord`] facts.
pub(super) fn command_written_spans(
    sm: &SourceMap<'_>,
    config: LexerConfig,
    command: &SegmentedCommand,
) -> Option<Vec<tcl_lexer::Span>> {
    if !command.word_views_aligned() {
        return None;
    }
    let source = std::str::from_utf8(sm.source_bytes()).ok()?;
    for token in &command.all_tokens {
        let raw = source.get(token.span.as_range())?;
        raw.get(usize::from(token.content_offset)..)?;
    }
    let groups = tcl_lexer::group_commands_bytes(&command.all_tokens, sm.source_bytes(), config);
    let groups = groups
        .iter()
        .flat_map(|command| &command.words)
        .collect::<Vec<_>>();
    if groups.len() != command.argv.len() {
        return None;
    }
    groups
        .iter()
        .enumerate()
        .map(|(index, group)| {
            let fragments = command.word_fragments.get(index)?;
            let first = fragments.first()?.token;
            let expected = tcl_lexer::Token {
                span: group.span,
                ..first
            };
            let actual = command.all_tokens.get(group.tokens.clone())?;
            if command.argv[index] != expected
                || actual.len() != fragments.len()
                || !actual.iter().zip(fragments).all(|(token, fragment)| {
                    *token == fragment.token
                        && fragment.text == crate::segmenter::word_piece(sm, *token)
                })
                || command.texts[index]
                    != fragments
                        .iter()
                        .map(|fragment| fragment.text.as_str())
                        .collect::<String>()
                || command.single_token_word[index] != group.is_single_token()
                || command
                    .expand_word
                    .as_ref()
                    .is_some_and(|expand| expand[index])
                    != group.expand
            {
                return None;
            }
            let span = tcl_lexer::NativeWord::written_span_from_group(
                sm.source_bytes(),
                config,
                &command.all_tokens,
                group,
            )
            .ok()?;
            source.get(span.as_range())?;
            Some(span)
        })
        .collect()
}

pub(super) fn command_words(
    sm: &SourceMap<'_>,
    config: LexerConfig,
    command: &SegmentedCommand,
) -> Vec<CommandWord> {
    let written = command_written_spans(sm, config, command);
    let tokens = written
        .as_ref()
        .map(|_| CommandTokens::from_segmented(sm, config, command));
    command
        .argv
        .iter()
        .enumerate()
        .map(|(idx, token)| CommandWord {
            text: command.texts.get(idx).cloned().unwrap_or_default(),
            raw: written
                .as_ref()
                .and_then(|spans| spans.get(idx))
                .map_or_else(String::new, |span| sm.text(*span).to_owned()),
            geometry: if written.is_some() {
                WrittenWordGeometry::Present
            } else {
                WrittenWordGeometry::Missing
            },
            substituted: written.is_none()
                || tokens
                    .as_ref()
                    .and_then(|tokens| tokens.words().get(idx))
                    .is_some_and(word_substitutes),
            expanded: command
                .expand_word
                .as_ref()
                .and_then(|markers| markers.get(idx))
                .copied()
                .unwrap_or(false),
            // `{…}` and nothing welded to it — the same question
            // `CommandTokens::arg_is_braced_literal` answers for an IR
            // statement's arguments, asked here in whole-argv indexing.
            braced_literal: written.is_some()
                && token.kind == TokenType::Str
                && command.single_token_word.get(idx).copied().unwrap_or(false),
        })
        .collect()
}

/// Whether a word's value comes from run-time data rather than its own source
/// text — the [`WordExpr`] forms that substitute.
///
/// `Template` is the compound case (`a$b`, `{a}[c]`): one substituting part is
/// enough, exactly as the whole word's value stops being its written text.
/// `Expand` wraps the word `{*}` expands and answers for that word.
fn word_substitutes(word: &WordExpr) -> bool {
    match word {
        WordExpr::Variable { .. } | WordExpr::CommandSubstitution { .. } => true,
        WordExpr::Template { parts, .. } => parts.iter().any(|part| {
            matches!(
                part,
                WordPart::Variable { .. } | WordPart::CommandSubstitution { .. }
            )
        }),
        WordExpr::Expand { word, .. } => word_substitutes(word),
        WordExpr::Literal { .. } | WordExpr::BracedLiteral { .. } | WordExpr::Opaque { .. } => {
            false
        }
    }
}
