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

//! Original word and variable cursor geometry retains incomplete source ownership.

use super::{
    ArrayIndexEnd, BracedVarEnd, LexerConfig, SubstFlags, TemplateVariableSyntax, WordPart,
    braced_var_name_end, scan_array_index_in, scan_var_ref_with_term, shallow_spanned_parts,
};
use crate::{ParseIssueKind, SourceImage, Span, TokenType};

/// Original partial-word geometry unavailable under this source purpose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WordCursorUnavailable {
    /// Cursor, token or group geometry does not belong to the supplied region.
    SourceGeometry,
    /// The actual configured lexical stream is unavailable.
    LexicalStream,
    /// Expansion or welded delimiters do not retain an independent source word.
    WordShape,
}

/// A cursor prefix in one actual source word, or an actual empty argv boundary.
/// Neither the prefix nor its ordinal selects a command or evaluated value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WordCursorPrefix {
    image: SourceImage,
    config: LexerConfig,
    region: Span,
    container: Span,
    partial: Span,
    kind: crate::WordKind,
    command: Option<(usize, Span)>,
    word_index: usize,
    empty_boundary: bool,
}

impl WordCursorPrefix {
    /// Same complete image/channel and all configured lexical grammar axes.
    #[must_use]
    pub fn matches_source(&self, image: &SourceImage, config: LexerConfig) -> bool {
        &self.image == image && self.config == config
    }
    /// Whole original image/channel retained by the lexical issuer.
    #[must_use]
    pub const fn image(&self) -> &SourceImage {
        &self.image
    }
    /// Full original grammar, including source continuation and escape rules.
    #[must_use]
    pub const fn config(&self) -> LexerConfig {
        self.config
    }
    /// Independently selected lexical source phase.
    #[must_use]
    pub const fn region_span(&self) -> Span {
        self.region
    }
    /// Whole original word, including real delimiters and the following suffix.
    /// An empty boundary has exactly the original cursor point as its extent.
    #[must_use]
    pub const fn container_span(&self) -> Span {
        self.container
    }
    /// Original content through the cursor, without a fabricated closer.
    #[must_use]
    pub const fn partial_span(&self) -> Span {
        self.partial
    }
    /// Actual lexer/grouper word kind, never inferred from a decoded value.
    #[must_use]
    pub const fn kind(&self) -> crate::WordKind {
        self.kind
    }
    /// Original argv ordinal in its containing lexical command.
    #[must_use]
    pub const fn word_index(&self) -> usize {
        self.word_index
    }
    /// Containing grouped command ordinal and original source extent, when a
    /// command is present. An empty head boundary creates no command receipt.
    #[must_use]
    pub const fn command_source(&self) -> Option<(usize, Span)> {
        self.command
    }
    /// This source point follows actual delimiter trivia instead of a word.
    #[must_use]
    pub const fn is_empty_boundary(&self) -> bool {
        self.empty_boundary
    }
}

/// Retain actual configured word grouping at a source cursor, including an
/// unfinished quote or brace. No complete word, name, binding or edit permission
/// is issued; literal prefix values and enclosing source phases are separate.
///
/// # Errors
/// Returns unavailable geometry, stream ownership or independent word shape.
pub fn word_cursor_prefix_in(
    image: &SourceImage,
    region: Span,
    cursor: u32,
    config: LexerConfig,
) -> Result<Option<WordCursorPrefix>, WordCursorUnavailable> {
    if region.start() > cursor || cursor > region.end() || region.end() as usize > image.len() {
        return Err(WordCursorUnavailable::SourceGeometry);
    }
    let tokens = crate::source_region_tokens_in(image, region, config)
        .map_err(|_| WordCursorUnavailable::LexicalStream)?;
    if tokens.iter().any(|token| {
        token.kind == TokenType::Comment
            && token.span.start() <= cursor
            && cursor <= token.span.end()
    }) {
        return Ok(None);
    }
    let commands = crate::group_commands_bytes(&tokens, image.bytes(), config);
    for (command_index, command) in commands.iter().enumerate() {
        for (word_index, word) in command.words.iter().enumerate() {
            let container =
                crate::NativeWord::written_span_from_group(image.bytes(), config, &tokens, word)
                    .map_err(|_| WordCursorUnavailable::SourceGeometry)?;
            if container.start() < region.start() || container.end() > region.end() {
                return Err(WordCursorUnavailable::SourceGeometry);
            }
            if !(container.start() <= cursor && cursor <= container.end()) {
                continue;
            }
            if word.expand || word.welded_after_close || word.welded_after_close_quote {
                return Err(WordCursorUnavailable::WordShape);
            }
            if tokens.get(word.tokens.clone()).is_none_or(|fragments| {
                fragments
                    .iter()
                    .any(|token| !matches!(token.kind, TokenType::Esc | TokenType::Str))
            }) {
                return Ok(None);
            }
            let first = tokens
                .get(word.tokens.start)
                .ok_or(WordCursorUnavailable::SourceGeometry)?;
            let start = word
                .span
                .start()
                .checked_add(u32::from(first.content_offset))
                .ok_or(WordCursorUnavailable::SourceGeometry)?;
            let content_end = match word.kind {
                crate::WordKind::Braced | crate::WordKind::Quoted => {
                    crate::word_closer_offset_at(image.bytes(), word.span)
                        .unwrap_or(container.end())
                }
                crate::WordKind::Bare => container.end(),
            };
            if start > cursor || start > content_end {
                return Ok(None);
            }
            return Ok(Some(WordCursorPrefix {
                image: image.clone(),
                config,
                region,
                container,
                partial: Span::new(start, cursor.min(content_end)),
                kind: word.kind,
                command: Some((
                    command_index,
                    command
                        .source_extent(&tokens, region.end() as usize)
                        .ok_or(WordCursorUnavailable::SourceGeometry)?,
                )),
                word_index,
                empty_boundary: false,
            }));
        }
    }
    if tokens.iter().any(|token| {
        token.kind == TokenType::Expand
            && token.span.start() <= cursor
            && cursor <= token.span.end()
    }) {
        return Err(WordCursorUnavailable::WordShape);
    }
    if tokens.iter().any(|token| {
        token.span.start() < cursor
            && cursor < token.span.end()
            && !matches!(token.kind, TokenType::Sep | TokenType::Eol)
    }) {
        return Ok(None);
    }
    let command = commands.iter().enumerate().find_map(|(index, command)| {
        let extent = command.source_extent(&tokens, region.end() as usize)?;
        (extent.start() <= cursor && cursor <= extent.end()).then_some((
            index,
            extent,
            command.words.len(),
        ))
    });
    Ok(Some(WordCursorPrefix {
        image: image.clone(),
        config,
        region,
        container: Span::new(cursor, cursor),
        partial: Span::new(cursor, cursor),
        kind: crate::WordKind::Bare,
        command: command.map(|(index, extent, _)| (index, extent)),
        word_index: command.map_or(0, |(_, _, len)| len),
        empty_boundary: true,
    }))
}

/// A configured original argv cursor. This retains whole source grouping and
/// an honest parse cut, including dynamic/expanded and unfinished arguments.
/// Only the original head is a complete NativeWord. Active arguments retain
/// geometry without an evaluated value, semantic role or entered frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArgumentCursor {
    image: SourceImage,
    config: LexerConfig,
    region: Span,
    command: Span,
    head: crate::NativeWord,
    word_index: usize,
    words: usize,
    empty_boundary: bool,
    cut: Option<crate::ParseCut>,
}
impl ArgumentCursor {
    /// Exact independently supplied lexical source region.
    #[must_use]
    pub const fn region_span(&self) -> Span {
        self.region
    }

    /// Complete original image/channel/configuration and selected lexical region.
    #[must_use]
    pub fn matches_source(&self, image: &SourceImage, region: Span, config: LexerConfig) -> bool {
        self.image == *image && self.region == region && self.config == config
    }
    /// Original full command extent; no synthetic closing delimiter is added.
    #[must_use]
    pub const fn command_span(&self) -> Span {
        self.command
    }
    /// Real whole written head geometry, without selected-command identity.
    #[must_use]
    pub fn head_span(&self) -> Span {
        self.head.span()
    }
    /// Complete original written head; semantic name/lookup owners are separate.
    #[must_use]
    pub const fn head_word(&self) -> &crate::NativeWord {
        &self.head
    }
    /// Original argv ordinal, including dynamic and expanded written words.
    #[must_use]
    pub const fn word_index(&self) -> usize {
        self.word_index
    }
    /// Number of whole grouped written words; expansions do not imply argc.
    #[must_use]
    pub const fn written_word_count(&self) -> usize {
        self.words
    }
    /// The cursor follows actual separating trivia within this command.
    #[must_use]
    pub const fn is_empty_boundary(&self) -> bool {
        self.empty_boundary
    }
    /// Genuine selected-parser cut at or before this lexical command. Missing
    /// cut means no retained syntax cut, never execution or lookup completion.
    #[must_use]
    pub const fn parse_cut(&self) -> Option<crate::ParseCut> {
        self.cut
    }
}

/// Original configured argv geometry at a source cursor. The caller owns the
/// script-region purpose; this lexer facet does not discover executable bodies.
/// Unlike literal completion prefixes, dynamic and expanded source words retain
/// their actual written ordinal. No evaluated or Native formal role is issued.
///
/// # Errors
/// Returns unavailable geometry, token stream or original group extent.
pub fn argument_cursor_in(
    image: &SourceImage,
    region: Span,
    cursor: u32,
    config: LexerConfig,
) -> Result<Option<ArgumentCursor>, WordCursorUnavailable> {
    if region.start() > cursor || cursor > region.end() || region.end() as usize > image.len() {
        return Err(WordCursorUnavailable::SourceGeometry);
    }
    let tokens = crate::source_region_tokens_in(image, region, config)
        .map_err(|_| WordCursorUnavailable::LexicalStream)?;
    if tokens.iter().any(|token| {
        token.kind == TokenType::Comment
            && token.span.start() <= cursor
            && cursor <= token.span.end()
    }) {
        return Ok(None);
    }
    let commands = crate::group_commands_bytes(&tokens, image.bytes(), config);
    let cut = crate::first_parse_cut_image_in_checked(&commands, &tokens, image, config)
        .map_err(|_| WordCursorUnavailable::LexicalStream)?;
    for (index, command) in commands.iter().enumerate() {
        let Some(head_group) = command.words.first() else {
            continue;
        };
        if head_group.expand {
            continue;
        }
        let head = crate::NativeWord::from_group(image.clone(), config, &tokens, head_group)
            .map_err(|_| WordCursorUnavailable::WordShape)?;
        let extent = command
            .source_extent(&tokens, region.end() as usize)
            .ok_or(WordCursorUnavailable::SourceGeometry)?;
        let mut active = None;
        let mut preceding = 0;
        for (ordinal, word) in command.words.iter().enumerate() {
            let span =
                crate::NativeWord::written_span_from_group(image.bytes(), config, &tokens, word)
                    .map_err(|_| WordCursorUnavailable::SourceGeometry)?;
            if span.end() <= cursor {
                preceding += 1;
            }
            if span.start() <= cursor && cursor <= span.end() {
                active = Some((ordinal, false));
                break;
            }
        }
        if active.is_none() && extent.start() <= cursor && cursor <= extent.end() {
            let last = tokens.iter().rev().find(|token| {
                token.span.start() < cursor
                    && token.span.start() >= extent.start()
                    && token.kind != TokenType::Eof
                    && !(token.kind == TokenType::Eol && token.span.is_empty())
            });
            if last.is_some_and(|token| token.kind == TokenType::Sep) {
                active = Some((preceding, true));
            }
        }
        let Some((word_index, empty_boundary)) = active else {
            continue;
        };
        if word_index == 0 {
            return Ok(None);
        }
        return Ok(Some(ArgumentCursor {
            image: image.clone(),
            config,
            region,
            command: extent,
            head,
            word_index,
            words: command.words.len(),
            empty_boundary,
            cut: cut.filter(|cut| cut.command <= index),
        }));
    }
    Ok(None)
}

/// Source cursor geometry that cannot be represented by this variable purpose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableCursorUnavailable {
    /// The source region or cursor does not belong to the supplied image.
    SourceGeometry,
    /// The configured lexical stream is unavailable.
    LexicalStream,
    /// An index contains a command, expression or unavailable value prefix.
    IndexSubstitution,
    /// Nested variable cursor syntax exceeds the shared decomposition budget.
    IndexNesting,
    /// Following source has an error other than this reference's missing closer.
    ReferenceSyntax,
}

/// A variable prefix in an actual configured lexical source region. The
/// enclosing source phase and any name/frame/edit authority remain independent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableCursorPrefix {
    image: SourceImage,
    config: LexerConfig,
    reference: Span,
    container: Span,
    replacement: Span,
    partial: Span,
    root: Span,
    index: Option<Span>,
    braced: bool,
}

impl VariableCursorPrefix {
    /// Same complete source bytes/channel and all configured grammar axes.
    #[must_use]
    pub fn matches_source(&self, image: &SourceImage, config: LexerConfig) -> bool {
        &self.image == image && self.config == config
    }
    /// Original `$` through the cursor, without invented closing syntax.
    #[must_use]
    pub const fn reference_span(&self) -> Span {
        self.reference
    }
    /// Genuine complete reference extent, or the lexer-owned unfinished extent.
    #[must_use]
    pub const fn container_span(&self) -> Span {
        self.container
    }
    /// Original completion range. A complete reference includes its real
    /// closer; a proven missing closer replaces only `$` through the cursor.
    /// This geometry grants no candidate identity or transformation authority.
    #[must_use]
    pub const fn completion_replacement_span(&self) -> Span {
        self.replacement
    }
    /// Original prefix after `$` or `${`, including a separate index if present.
    #[must_use]
    pub const fn partial_span(&self) -> Span {
        self.partial
    }
    /// Original braced name prefix, bare root prefix or complete explicit index root.
    #[must_use]
    pub const fn root_span(&self) -> Span {
        self.root
    }
    /// Original index prefix, excluding its opening parenthesis.
    #[must_use]
    pub const fn index_span(&self) -> Option<Span> {
        self.index
    }
    /// Braced name syntax. Combined-name interpretation needs its independent owner.
    #[must_use]
    pub const fn is_braced(&self) -> bool {
        self.braced
    }
}

fn span(start: usize, end: usize) -> Result<Span, VariableCursorUnavailable> {
    Ok(Span::new(
        u32::try_from(start).map_err(|_| VariableCursorUnavailable::SourceGeometry)?,
        u32::try_from(end).map_err(|_| VariableCursorUnavailable::SourceGeometry)?,
    ))
}

fn terminal_dollar(bytes: &[u8], start: usize, config: LexerConfig) -> Option<usize> {
    let mut at = start;
    while at < bytes.len() {
        if bytes[at] == b'\\' && at + 1 < bytes.len() {
            at = crate::substitution::backslash_escape_end_bytes_in(bytes, at, config.escapes);
        } else if at + 1 == bytes.len() && bytes[at] == b'$' {
            return Some(at);
        } else {
            at += 1;
        }
    }
    None
}

fn prefix_at(
    image: &SourceImage,
    region: Span,
    config: LexerConfig,
    mut at: usize,
    cursor: usize,
    enclosing_end: usize,
) -> Result<Option<VariableCursorPrefix>, VariableCursorUnavailable> {
    let source = image
        .bytes()
        .get(..region.end() as usize)
        .ok_or(VariableCursorUnavailable::SourceGeometry)?;
    let prefix = source
        .get(..cursor)
        .ok_or(VariableCursorUnavailable::SourceGeometry)?;
    for _ in 0..=super::MAX_INDEX_DEPTH {
        if prefix.get(at) != Some(&b'$') {
            return Ok(None);
        }
        let braced = prefix.get(at + 1) == Some(&b'{');
        let name = at + if braced { 2 } else { 1 };
        let mut index = None;
        let root_end;
        if braced {
            if matches!(
                braced_var_name_end(prefix, name, config.braced_var),
                BracedVarEnd::Closed(_)
            ) {
                return Ok(None);
            }
            root_end = cursor;
        } else {
            let end = tcl_core_types::naming::scan_var_name_end_with(
                prefix,
                name,
                config.var_syntax.name_allows_high_bytes(),
            );
            if end == cursor {
                root_end = end;
            } else if prefix.get(end) == Some(&b'(')
                && !(end == name && config.var_syntax.has_expr_sugar())
            {
                root_end = end;
                let scan = if config.var_syntax.index_parens_nest() {
                    let end = crate::ranges::jim_parenthesis_body_end(prefix, end + 1);
                    if prefix.get(end) == Some(&b')') || end < cursor {
                        return Ok(None);
                    }
                    None
                } else {
                    Some(scan_array_index_in(prefix, end, config))
                };
                if scan.is_some_and(|scan| scan.invalid.is_some()) {
                    return Err(VariableCursorUnavailable::IndexSubstitution);
                }
                if scan.is_some_and(|scan| matches!(scan.end, ArrayIndexEnd::Closed(_))) {
                    return Ok(None);
                }
                let start = end + 1;
                let parts = shallow_spanned_parts(
                    &prefix[start..],
                    SubstFlags::default(),
                    config,
                    TemplateVariableSyntax::WrittenWord,
                );
                if let Some(last) = parts.last() {
                    let nested = match &last.part {
                        WordPart::Variable(_) if last.end == cursor - start => {
                            Some(start + last.start)
                        }
                        WordPart::ParseError(_)
                            if matches!(
                                last.error_kind,
                                Some(
                                    ParseIssueKind::MissingVariableBrace
                                        | ParseIssueKind::MissingParenthesis
                                )
                            ) && prefix.get(start + last.start) == Some(&b'$') =>
                        {
                            Some(start + last.start)
                        }
                        WordPart::Text(_) => terminal_dollar(&prefix[start..], last.start, config)
                            .map(|at| start + at),
                        _ => None,
                    };
                    if let Some(nested) = nested {
                        at = nested;
                        continue;
                    }
                }
                if parts
                    .iter()
                    .any(|part| !matches!(part.part, WordPart::Text(_)))
                {
                    return Err(VariableCursorUnavailable::IndexSubstitution);
                }
                index = Some(span(start, cursor)?);
            } else {
                return Ok(None);
            }
        }
        let (container_end, replacement_end) = match scan_var_ref_with_term(source, at, config) {
            Ok(Some(reference)) => (reference.next, reference.next),
            Ok(None) => (at + 1, at + 1),
            Err(error)
                if (braced
                    && error.kind == ParseIssueKind::MissingVariableBrace
                    && error.term == at + 1)
                    || (index.is_some()
                        && error.kind == ParseIssueKind::MissingParenthesis
                        && error.term == root_end) =>
            {
                (enclosing_end, cursor)
            }
            Err(_) => return Err(VariableCursorUnavailable::ReferenceSyntax),
        };
        if container_end < cursor || container_end > region.end() as usize {
            return Err(VariableCursorUnavailable::SourceGeometry);
        }
        return Ok(Some(VariableCursorPrefix {
            image: image.clone(),
            config,
            reference: span(at, cursor)?,
            container: span(at, container_end)?,
            replacement: span(at, replacement_end)?,
            partial: span(name, cursor)?,
            root: span(name, root_end)?,
            index,
            braced,
        }));
    }
    Err(VariableCursorUnavailable::IndexNesting)
}

/// Select an incomplete or complete variable prefix from the actual configured
/// lexical stream of an independently owned source script region. Comments,
/// literal braces and escaped sigils do not create variable cursor prefixes.
/// No completed `NativeWord`, name key, evaluated value or frame is issued.
///
/// # Errors
/// Returns unavailable source geometry, lexical ownership or index-prefix syntax.
pub fn variable_cursor_prefix_in(
    image: &SourceImage,
    region: Span,
    cursor: u32,
    config: LexerConfig,
) -> Result<Option<VariableCursorPrefix>, VariableCursorUnavailable> {
    if region.start() > cursor || cursor > region.end() || region.end() as usize > image.len() {
        return Err(VariableCursorUnavailable::SourceGeometry);
    }
    let tokens = crate::source_region_tokens_in(image, region, config)
        .map_err(|_| VariableCursorUnavailable::LexicalStream)?;
    let candidates = tokens
        .iter()
        .filter(|token| {
            token.span.start() < cursor
                && cursor <= token.span.end()
                && (token.kind == TokenType::Var
                    || (matches!(token.kind, TokenType::Str | TokenType::Esc)
                        && token.span.len() == 1
                        && image.bytes().get(token.span.as_range()) == Some(b"$")))
        })
        .collect::<Vec<_>>();
    let [token] = candidates.as_slice() else {
        return Ok(None);
    };
    prefix_at(
        image,
        region,
        config,
        token.span.start() as usize,
        cursor as usize,
        token.span.end() as usize,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn word_at(marked: &str) -> (SourceImage, Option<WordCursorPrefix>) {
        let cursor = marked.find("<|>").unwrap();
        let source = marked.replacen("<|>", "", 1);
        let image = SourceImage::document(&source);
        let prefix = word_cursor_prefix_in(
            &image,
            Span::new(0, image.len() as u32),
            cursor as u32,
            LexerConfig::default(),
        )
        .unwrap();
        (image, prefix)
    }

    #[test]
    fn original_word_cursor_keeps_partial_grouping_and_whole_replacement_extent() {
        for (source, partial, kind) in [
            (r"some\ name<|>tail", r"some\ name", crate::WordKind::Bare),
            ("{a b<|> suffix}", "a b", crate::WordKind::Braced),
            ("\"a b<|> suffix\"", "a b", crate::WordKind::Quoted),
            ("{unfinished<|>", "unfinished", crate::WordKind::Braced),
            ("\"unfinished<|>", "unfinished", crate::WordKind::Quoted),
        ] {
            let (image, prefix) = word_at(source);
            let prefix = prefix.unwrap();
            assert_eq!(
                &image.bytes()[prefix.partial_span().as_range()],
                partial.as_bytes()
            );
            assert_eq!(prefix.container_span(), Span::new(0, image.len() as u32));
            assert_eq!(prefix.kind(), kind);
            assert_eq!(prefix.word_index(), 0);
            assert!(!prefix.is_empty_boundary());
            assert!(prefix.matches_source(&image, LexerConfig::default()));
            assert!(!prefix.matches_source(
                &SourceImage::native(image.shared_bytes()),
                LexerConfig::default()
            ));
        }
    }

    #[test]
    fn original_word_cursor_distinguishes_head_and_argument_boundaries() {
        for (source, index) in [
            ("<|>", 0),
            ("puts <|>", 1),
            ("puts value; <|>", 0),
            ("puts value\n <|>", 0),
            ("puts value\\\n <|>", 2),
        ] {
            let (_, prefix) = word_at(source);
            let prefix = prefix.unwrap();
            assert_eq!(prefix.word_index(), index);
            assert!(prefix.is_empty_boundary());
            assert!(prefix.container_span().is_empty());
        }
        let (_, prefix) = word_at("puts {ordinary data<|>}");
        assert_eq!(prefix.unwrap().word_index(), 1);
        for source in ["# comment<|>", "name$variable<|>", "name[puts child]<|>"] {
            assert!(word_at(source).1.is_none());
        }
        let image = SourceImage::document("{word}");
        assert_eq!(
            word_cursor_prefix_in(&image, Span::new(0, 5), 5, LexerConfig::default()),
            Err(WordCursorUnavailable::SourceGeometry)
        );
        let image = SourceImage::document("{*}word");
        assert_eq!(
            word_cursor_prefix_in(
                &image,
                Span::new(0, image.len() as u32),
                image.len() as u32,
                LexerConfig::default()
            ),
            Err(WordCursorUnavailable::WordShape)
        );
    }

    #[test]
    fn original_word_cursor_keeps_actual_grammar_and_region_currentness() {
        for dialect in ["tcl8.6", "tcl9.0", "jim"] {
            let config = LexerConfig::for_dialect(dialect);
            let image = SourceImage::document("; café\\ name");
            let region = Span::new(1, image.len() as u32);
            let prefix = word_cursor_prefix_in(&image, region, image.len() as u32, config)
                .unwrap()
                .unwrap();
            assert_eq!(prefix.region_span(), region);
            assert_eq!(prefix.word_index(), 0);
            assert!(prefix.matches_source(&image, config));
            assert!(!prefix.matches_source(&SourceImage::document("; other\\ name"), config));
            let changed = LexerConfig {
                strict_quoting: !config.strict_quoting,
                ..config
            };
            assert!(!prefix.matches_source(&image, changed));
        }
    }

    fn at(marked: &str, config: LexerConfig) -> (SourceImage, Option<VariableCursorPrefix>) {
        let cursor = marked.find("<|>").unwrap();
        let source = marked.replacen("<|>", "", 1);
        let image = SourceImage::document(&source);
        let result = variable_cursor_prefix_in(
            &image,
            Span::new(0, image.len() as u32),
            cursor as u32,
            config,
        )
        .unwrap();
        (image, result)
    }

    #[test]
    fn original_variable_cursor_keeps_braced_names_and_complete_container() {
        let config = LexerConfig::default();
        let (image, prefix) = at("puts ${cash$name<|>}suffix", config);
        let prefix = prefix.unwrap();
        assert!(prefix.is_braced());
        assert_eq!(
            &image.bytes()[prefix.partial_span().as_range()],
            b"cash$name"
        );
        assert_eq!(
            &image.bytes()[prefix.container_span().as_range()],
            b"${cash$name}"
        );
        assert!(prefix.index_span().is_none());
        let (image, prefix) = at("puts ${a b<|>}suffix", config);
        let prefix = prefix.unwrap();
        assert_eq!(&image.bytes()[prefix.partial_span().as_range()], b"a b");
        let (image, prefix) = at("puts ${scalar(open<|>}", config);
        let prefix = prefix.unwrap();
        assert_eq!(
            &image.bytes()[prefix.root_span().as_range()],
            b"scalar(open"
        );
        assert!(prefix.index_span().is_none());
        assert!(prefix.matches_source(&image, config));
        assert!(!prefix.matches_source(&SourceImage::native(image.shared_bytes()), config));
        let changed = LexerConfig {
            strict_quoting: !config.strict_quoting,
            ..config
        };
        assert!(!prefix.matches_source(&image, changed));
    }

    #[test]
    fn original_variable_cursor_reuses_configured_name_and_closer_grammar() {
        let eight = LexerConfig {
            braced_var: crate::BracedVarStyle::FirstClose,
            ..LexerConfig::default()
        };
        let nine = LexerConfig {
            braced_var: crate::BracedVarStyle::Tcl9Nesting,
            ..eight
        };
        for (config, expected) in [
            (eight, b"${a{b}".as_slice()),
            (nine, b"${a{b}c}".as_slice()),
        ] {
            let (image, prefix) = at("puts ${a<|>{b}c}", config);
            assert_eq!(
                &image.bytes()[prefix.unwrap().container_span().as_range()],
                expected
            );
        }
        assert!(at("puts $a:<|>rest", eight).1.is_none());
        assert!(at("puts $caf\u{e9}<|>", eight).1.is_none());
        let jim = LexerConfig {
            var_syntax: tcl_dialect::VarSyntax::Jim,
            ..eight
        };
        let (image, prefix) = at("puts $caf\u{e9}<|>", jim);
        assert_eq!(
            &image.bytes()[prefix.unwrap().partial_span().as_range()],
            "café".as_bytes()
        );
    }

    #[test]
    fn original_variable_cursor_preserves_index_and_nested_reference_geometry() {
        let config = LexerConfig::default();
        let (image, prefix) = at("puts $arr(ke<|>y)", config);
        let prefix = prefix.unwrap();
        assert_eq!(&image.bytes()[prefix.root_span().as_range()], b"arr");
        assert_eq!(
            &image.bytes()[prefix.index_span().unwrap().as_range()],
            b"ke"
        );
        assert_eq!(
            &image.bytes()[prefix.container_span().as_range()],
            b"$arr(key)"
        );
        for marked in ["puts $arr($in<|>ner)", "puts $arr($<|>)"] {
            let (image, prefix) = at(marked, config);
            let prefix = prefix.unwrap();
            assert_eq!(
                image.bytes()[prefix.reference_span().start() as usize],
                b'$'
            );
            assert_eq!(
                &image.bytes()[prefix.container_span().as_range()],
                if marked.contains("$in") {
                    b"$inner".as_slice()
                } else {
                    b"$".as_slice()
                }
            );
            assert!(prefix.index_span().is_none());
        }
        let image = SourceImage::document("puts $arr([set x 1]suffix)");
        assert_eq!(
            variable_cursor_prefix_in(
                &image,
                Span::new(0, image.len() as u32),
                (image.len() - 1) as u32,
                config
            ),
            Err(VariableCursorUnavailable::IndexSubstitution)
        );
    }

    #[test]
    fn completion_replacement_distinguishes_missing_closers_from_original_containers() {
        let config = LexerConfig::default();
        for (marked, replacement) in [
            ("puts $arr(k<|> more stuff\n", "$arr(k"),
            ("puts ${a<|> b\n", "${a"),
            ("puts $arr(k<|>suffix)\n", "$arr(ksuffix)"),
        ] {
            let (image, prefix) = at(marked, config);
            let prefix = prefix.unwrap();
            assert_eq!(
                &image.bytes()[prefix.completion_replacement_span().as_range()],
                replacement.as_bytes()
            );
            if marked.contains("suffix)") {
                assert_eq!(
                    prefix.completion_replacement_span(),
                    prefix.container_span()
                );
            } else {
                assert_eq!(
                    prefix.completion_replacement_span(),
                    prefix.reference_span()
                );
                assert!(prefix.container_span().end() > prefix.reference_span().end());
            }
        }
        for marked in [
            "puts $arr(k<|>[unterminated",
            "puts $arr(k<|>${unterminated",
        ] {
            let cursor = marked.find("<|>").unwrap();
            let image = SourceImage::document(&marked.replacen("<|>", "", 1));
            assert_eq!(
                variable_cursor_prefix_in(
                    &image,
                    Span::new(0, image.len() as u32),
                    cursor as u32,
                    config,
                ),
                Err(VariableCursorUnavailable::ReferenceSyntax)
            );
        }
    }

    #[test]
    fn original_variable_cursor_keeps_inert_and_unavailable_source_distinct() {
        let config = LexerConfig::default();
        for marked in [
            "# $no<|>",
            "puts {$no<|>}",
            "puts \\$no<|>",
            "puts ${closed}<|>",
        ] {
            assert!(at(marked, config).1.is_none(), "{marked}");
        }
        let image = SourceImage::document("puts $name");
        assert_eq!(
            variable_cursor_prefix_in(&image, Span::new(0, 5), 8, config),
            Err(VariableCursorUnavailable::SourceGeometry)
        );
    }
}

#[cfg(test)]
mod argument_cursor_tests {
    use super::*;
    fn at(source: &str, offset: usize) -> Option<ArgumentCursor> {
        let image = SourceImage::document(source);
        argument_cursor_in(
            &image,
            Span::new(0, u32::try_from(source.len()).unwrap()),
            u32::try_from(offset).unwrap(),
            LexerConfig::default(),
        )
        .unwrap()
    }
    #[test]
    fn argument_cursor_retains_dynamic_expanded_and_future_suffix_ordinals() {
        for (source, needle, ordinal) in [
            ("cmd $value tail", "$value", 1),
            ("cmd {*}{alpha beta} tail", "beta", 1),
            ("cmd first   future", "   ", 2),
            ("cmd {ordinary body data} tail", "body", 1),
        ] {
            let cursor = at(source, source.find(needle).unwrap() + 1).unwrap();
            assert_eq!(cursor.word_index(), ordinal, "{source}");
            assert_eq!(cursor.head_word().try_text().unwrap(), "cmd");
            assert_eq!(cursor.written_word_count(), 3);
        }
    }
    #[test]
    fn argument_cursor_keeps_incomplete_source_cut_and_never_adds_a_closer() {
        for source in ["cmd {unfinished", "cmd \"unfinished"] {
            let cursor = at(source, source.len()).unwrap();
            assert_eq!(cursor.word_index(), 1);
            assert_eq!(cursor.command_span().end() as usize, source.len());
            assert_eq!(
                cursor.parse_cut().unwrap().completeness(),
                crate::ParseCutCompleteness::Incomplete
            );
            assert_eq!(cursor.head_word().image().bytes(), source.as_bytes());
        }
    }
    #[test]
    fn argument_cursor_separates_comments_boundaries_foreign_source_and_config() {
        assert!(at("cmd first\n# comment", 14).is_none());
        assert!(at("cmd first\n", 10).is_none());
        assert!(at("cmd first", 2).is_none());
        let source = "cmd first ";
        let cursor = at(source, source.len()).unwrap();
        assert!(cursor.is_empty_boundary());
        assert_eq!(cursor.word_index(), 2);
        let image = SourceImage::document(source);
        let region = Span::new(0, u32::try_from(source.len()).unwrap());
        assert!(cursor.matches_source(&image, region, LexerConfig::default()));
        assert!(!cursor.matches_source(
            &SourceImage::document("cmd other "),
            region,
            LexerConfig::default()
        ));
        let mut changed = LexerConfig::default();
        changed.expand_syntax = !changed.expand_syntax;
        assert!(!cursor.matches_source(&image, region, changed));
        assert_eq!(
            argument_cursor_in(&image, region, region.end() + 1, LexerConfig::default()),
            Err(WordCursorUnavailable::SourceGeometry)
        );
    }
}
