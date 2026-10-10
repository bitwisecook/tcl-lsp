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

//! Original byte words captured from the shared lexer and command grouper.

use std::borrow::Cow;

use crate::executable_parts::{
    ExecutableInput, ExecutablePart, ExecutablePartArena, ExecutablePartsUnavailable,
};
use crate::word_parts::{SpannedPart, SubstFlags, WordPart, decompose_spanned_checked};
use crate::{LexerConfig, SourceImage, Span, Token, TokenType, WordKind, WordSpan};

/// A grouped word whose original source and lexical facts remain one owner.
///
/// This is lexical provenance only. It grants no selected command, native
/// compiler admission, object representation, visibility or effect authority.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NativeWord {
    arena: ExecutablePartArena,
    config: LexerConfig,
    group: WordSpan,
    tokens: Vec<Token>,
    written: Span,
    operand: Span,
}

/// An unavailable or malformed original grouped word.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeWordError {
    /// Token indices, token geometry or expansion markers do not match the group.
    MisalignedGroup,
    /// The selected lexer could not publish a complete token stream.
    LexicalStreamUnavailable,
    /// The bounded advisory compatibility projection cannot retain every index.
    SubstitutionDepthUnavailable,
    /// An original extent lies outside the retained image.
    SourceBounds,
    /// The shared syntax owner rejected the original word.
    Parse(&'static str),
    /// A valid concatenating word has several content ranges rather than one.
    NonContiguousContent,
}

impl NativeWord {
    /// Capture a word from the tokens produced for this exact image/configuration.
    /// Spans are local byte offsets into `image`; configured base coordinates
    /// affect position reporting only. The retained group's token range is
    /// normalized to the captured fragment slice. Expansion markers contribute
    /// to `span()` but never to the word's content or substitution parts.
    ///
    /// # Errors
    /// Rejects empty/misaligned groups, missing expansion markers and extents
    /// outside the image, and shared decomposition errors before publishing
    /// immutable word geometry.
    pub fn from_group(
        image: SourceImage,
        config: LexerConfig,
        tokens: &[Token],
        word: &WordSpan,
    ) -> Result<Self, NativeWordError> {
        let fragments = tokens
            .get(word.tokens.clone())
            .filter(|tokens| !tokens.is_empty())
            .ok_or(NativeWordError::MisalignedGroup)?;
        let first = fragments.first().ok_or(NativeWordError::MisalignedGroup)?;
        let last = fragments.last().ok_or(NativeWordError::MisalignedGroup)?;
        if first.span.start() != word.span.start()
            || last.span.end() != word.span.end()
            || fragments.iter().any(|token| {
                matches!(
                    token.kind,
                    TokenType::Sep | TokenType::Eol | TokenType::Comment | TokenType::Expand
                )
            })
            || fragments
                .windows(2)
                .any(|pair| pair[0].span.start() > pair[1].span.start())
        {
            return Err(NativeWordError::MisalignedGroup);
        }
        let operand = complete_group_span(image.bytes(), word.span, *last, config)?;
        let mut start = operand.start();
        if word.expand {
            let mut preceding = word.tokens.start;
            while preceding > 0 {
                let marker = tokens[preceding - 1];
                if marker.kind != TokenType::Expand
                    || marker.span.start().checked_add(3) != Some(start)
                    || image
                        .bytes()
                        .get(marker.span.start() as usize..start as usize)
                        != Some(b"{*}".as_slice())
                {
                    break;
                }
                start = marker.span.start();
                preceding -= 1;
            }
            if start == operand.start() {
                return Err(NativeWordError::MisalignedGroup);
            }
        }
        let written = Span::new(start, operand.end());
        if image.bytes().get(written.as_range()).is_none()
            || fragments
                .iter()
                .any(|token| image.bytes().get(token.span.as_range()).is_none())
        {
            return Err(NativeWordError::SourceBounds);
        }
        let mut group = word.clone();
        group.tokens = 0..fragments.len();
        let mut captured = Self {
            arena: ExecutablePartArena::empty(image, config),
            config,
            group,
            tokens: fragments.to_vec(),
            written,
            operand,
        };
        let inputs = captured.executable_inputs()?;
        let arena = ExecutablePartArena::from_inputs(captured.image().clone(), &inputs, config)
            .map_err(|error| match error {
                ExecutablePartsUnavailable::SourceBounds => NativeWordError::SourceBounds,
                ExecutablePartsUnavailable::SourceGeometry => NativeWordError::MisalignedGroup,
            })?;
        if let Some(message) = arena.all_parts().find_map(|part| match part.part {
            ExecutablePart::ParseError(message) => Some(message),
            _ => None,
        }) {
            return Err(NativeWordError::Parse(message));
        }
        captured.arena = arena;
        Ok(captured)
    }

    /// The exact source buffer and original input channel.
    #[must_use]
    pub fn image(&self) -> &SourceImage {
        self.arena.image()
    }

    /// Authoritative full-depth executable components, with flat index edges.
    /// The arena's spans and literal/name/script bytes belong to `image()`.
    /// Executable consumers traverse its list IDs explicitly; `parts()` is a
    /// bounded compatibility projection, not executable authority.
    #[must_use]
    pub fn executable_parts(&self) -> &ExecutablePartArena {
        &self.arena
    }

    /// The selected lexical configuration, independent of compiler admission.
    #[must_use]
    pub const fn config(&self) -> LexerConfig {
        self.config
    }

    /// Grouping facts with indices into `tokens()`.
    #[must_use]
    pub fn group(&self) -> &WordSpan {
        &self.group
    }

    /// Original content tokens; expansion markers are represented by `span()`.
    #[must_use]
    pub fn tokens(&self) -> &[Token] {
        &self.tokens
    }

    /// Full written extent, including original expansion markers and closers.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.written
    }

    /// Full operand extent, excluding preceding expansion markers.
    #[must_use]
    pub const fn word_span(&self) -> Span {
        self.operand
    }

    /// Original operand bytes, including its braces or quotes.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.image().bytes()[self.operand.as_range()]
    }

    /// Original written bytes, including preceding expansion markers.
    #[must_use]
    pub fn written_bytes(&self) -> &[u8] {
        &self.image().bytes()[self.written.as_range()]
    }

    /// Checked unchanged Unicode operand view for advisory consumers.
    ///
    /// # Errors
    /// Returns the original UTF-8 error; opaque words have no equivalent view.
    pub fn try_text(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(self.bytes())
    }

    /// Original content extent after the shared delimiter owner removes wrappers.
    /// Braced content remains literal and uncollapsed: value formation must use
    /// the selected braced-word continuation rules and original input channel.
    ///
    /// # Errors
    /// Returns the selected shared parse error for missing closers or welded
    /// delimiters.
    pub fn content_span(&self) -> Result<Span, NativeWordError> {
        use crate::EXTRA_AFTER_CLOSE_QUOTE;
        use crate::word_parts::{EXTRA_AFTER_CLOSE_BRACE, MISSING_CLOSE_BRACE};
        if self.group.welded_after_close {
            return Err(NativeWordError::Parse(EXTRA_AFTER_CLOSE_BRACE));
        }
        let start = self.operand.start() as usize;
        let end = self.operand.end() as usize;
        match self.group.kind {
            WordKind::Braced if self.image().bytes().get(start) == Some(&b'{') => {
                let close = crate::word_closer_offset_at(self.image().bytes(), self.group.span)
                    .ok_or(NativeWordError::Parse(MISSING_CLOSE_BRACE))?;
                Ok(Span::new(self.operand.start() + 1, close))
            }
            WordKind::Quoted => {
                let close = crate::quoted_word_close(self.image().bytes(), start)
                    .map_err(NativeWordError::Parse)?;
                if close + 1 < end {
                    return Err(if self.config.quote_termination.is_strict() {
                        NativeWordError::Parse(EXTRA_AFTER_CLOSE_QUOTE)
                    } else {
                        NativeWordError::NonContiguousContent
                    });
                }
                let close = u32::try_from(close).map_err(|_| NativeWordError::SourceBounds)?;
                Ok(Span::new(self.operand.start() + 1, close))
            }
            _ => Ok(self.operand),
        }
    }

    fn executable_inputs(&self) -> Result<Vec<ExecutableInput>, NativeWordError> {
        match self.content_span() {
            Ok(span) => Ok(vec![if self.group.kind == WordKind::Braced {
                ExecutableInput::Literal(span)
            } else {
                ExecutableInput::Substitute {
                    span,
                    flags: SubstFlags::default(),
                }
            }]),
            Err(NativeWordError::NonContiguousContent) => {
                let source = self.image().bytes();
                let mut inputs = Vec::new();
                for token in &self.tokens {
                    let span = if matches!(token.kind, TokenType::Str | TokenType::Esc) {
                        let content = crate::source_map::token_bytes_in(source, *token);
                        let start = content.as_ptr() as usize - source.as_ptr() as usize;
                        Span::new(
                            u32::try_from(start).map_err(|_| NativeWordError::SourceBounds)?,
                            u32::try_from(start + content.len())
                                .map_err(|_| NativeWordError::SourceBounds)?,
                        )
                    } else {
                        complete_token_span(source, *token, self.config)?
                    };
                    if span.is_empty() {
                        continue;
                    }
                    inputs.push(if token.kind == TokenType::Str {
                        ExecutableInput::Literal(span)
                    } else {
                        ExecutableInput::Substitute {
                            span,
                            flags: if token.kind == TokenType::Esc {
                                SubstFlags {
                                    vars: false,
                                    cmds: false,
                                    ..SubstFlags::default()
                                }
                            } else {
                                SubstFlags::default()
                            },
                        }
                    });
                }
                Ok(inputs)
            }
            Err(error) => Err(error),
        }
    }

    /// Checked bounded compatibility projection through the shared scanner.
    /// Executable consumers use `executable_parts()` instead.
    /// Variable names and nested scripts borrow exact bytes from `image()`.
    /// Text escapes use the shared byte decoder. Returned component extents
    /// are local image byte offsets, including original substitution delimiters.
    /// A brace group yields one unchanged literal Text part; this API does not
    /// collapse its line continuations or infer an object type from its bytes.
    ///
    /// # Errors
    /// Returns shared delimiter errors and separately reports unavailable
    /// executable index decomposition before any component can execute.
    pub fn parts(&self) -> Result<Vec<SpannedPart<'_>>, NativeWordError> {
        let content = match self.content_span() {
            Err(NativeWordError::NonContiguousContent) => return self.fragment_parts(),
            other => other?,
        };
        let bytes = &self.image().bytes()[content.as_range()];
        let mut parts = if self.group.kind == WordKind::Braced {
            vec![SpannedPart {
                part: WordPart::Text(Cow::Borrowed(bytes)),
                start: 0,
                end: bytes.len(),
                error_term: None,
            }]
        } else {
            decompose_spanned_checked(bytes, SubstFlags::default(), self.config)
                .map_err(|_| NativeWordError::SubstitutionDepthUnavailable)?
        };
        for part in &mut parts {
            part.start += content.start() as usize;
            part.end += content.start() as usize;
            part.error_term = part.error_term.map(|term| term + content.start() as usize);
        }
        Ok(parts)
    }
    fn fragment_parts(&self) -> Result<Vec<SpannedPart<'_>>, NativeWordError> {
        let source = self.image().bytes();
        let mut parts = Vec::new();
        for token in &self.tokens {
            let literal = matches!(token.kind, TokenType::Str | TokenType::Esc);
            let content = if literal {
                crate::source_map::token_bytes_in(source, *token)
            } else {
                &source[complete_token_span(source, *token, self.config)
                    .expect("captured variable extent was validated")
                    .as_range()]
            };
            if content.is_empty() {
                continue;
            }
            let at = content.as_ptr() as usize - source.as_ptr() as usize;
            if token.kind == TokenType::Str {
                parts.push(SpannedPart {
                    part: WordPart::Text(Cow::Borrowed(content)),
                    start: at,
                    end: at + content.len(),
                    error_term: None,
                });
            } else {
                let flags = if token.kind == TokenType::Esc {
                    SubstFlags {
                        vars: false,
                        cmds: false,
                        ..SubstFlags::default()
                    }
                } else {
                    SubstFlags::default()
                };
                for mut part in decompose_spanned_checked(content, flags, self.config)
                    .map_err(|_| NativeWordError::SubstitutionDepthUnavailable)?
                {
                    part.start += at;
                    part.end += at;
                    part.error_term = part.error_term.map(|term| term + at);
                    parts.push(part);
                }
            }
        }
        Ok(parts)
    }
}

/// Complete token geometry is selected by the shared variable scanner for
/// `${name}`, whose lexical span excludes its closing brace.
pub(crate) fn complete_token_span(
    source: &[u8],
    token: Token,
    config: LexerConfig,
) -> Result<Span, NativeWordError> {
    if token.kind == TokenType::Var {
        if source.get(token.span.start() as usize) != Some(&b'$') {
            return Err(NativeWordError::MisalignedGroup);
        }
        let reference =
            crate::word_parts::scan_var_ref(source, token.span.start() as usize, config)
                .map_err(NativeWordError::Parse)?
                .ok_or(NativeWordError::MisalignedGroup)?;
        return Ok(Span::new(
            token.span.start(),
            u32::try_from(reference.next).map_err(|_| NativeWordError::SourceBounds)?,
        ));
    }
    Ok(crate::ranges::token_word_span(source, token))
}

/// A final escape fragment has no opening quote of its own. Retain both the
/// grouped word's delimiter and a final delimited component's own closer.
pub(crate) fn complete_group_span(
    source: &[u8],
    group: Span,
    last: Token,
    config: LexerConfig,
) -> Result<Span, NativeWordError> {
    Ok(Span::new(
        group.start(),
        complete_token_span(source, last, config)?
            .end()
            .max(crate::word_span_at(source, group).end()),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Lexer, group_commands_bytes};

    fn capture(source: &[u8], index: usize, config: LexerConfig) -> NativeWord {
        let image = SourceImage::native(source.to_vec());
        let tokens = Lexer::with_source_image(&image, config)
            .tokenise_all()
            .unwrap();
        let commands = group_commands_bytes(&tokens, image.bytes(), config);
        NativeWord::from_group(image.clone(), config, &tokens, &commands[0].words[index]).unwrap()
    }

    #[test]
    fn original_opaque_words_keep_shared_parts_and_full_expanded_extent() {
        let word = capture(b"list {*}[set \xff $a(\xfe)]", 1, LexerConfig::default());
        assert!(word.group().expand);
        assert_eq!(word.written_bytes(), b"{*}[set \xff $a(\xfe)]");
        assert_eq!(word.bytes(), b"[set \xff $a(\xfe)]");
        assert!(word.try_text().is_err());
        let parts = word.parts().unwrap();
        assert_eq!(parts.len(), 1);
        assert!(matches!(&parts[0].part, WordPart::Command(body) if *body == b"set \xff $a(\xfe)"));
        assert_eq!(
            &word.image().bytes()[parts[0].start..parts[0].end],
            word.bytes()
        );
    }

    #[test]
    fn opaque_literal_escapes_and_variable_names_do_not_require_unicode() {
        let word = capture(b"list \xff\\n${\xfe}", 1, LexerConfig::default());
        let parts = word.parts().unwrap();
        assert!(matches!(&parts[0].part, WordPart::Text(bytes) if bytes.as_ref() == b"\xff\n"));
        assert!(matches!(&parts[1].part, WordPart::Variable(variable) if variable.name == b"\xfe"));
        assert_eq!(
            &word.image().bytes()[parts[1].start..parts[1].end],
            b"${\xfe}"
        );
    }

    #[test]
    fn quoted_escape_fragments_retain_the_whole_original_word() {
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let config =
                LexerConfig::from_grammar(tcl_dialect::grammar_of_dialect_name(Some(dialect)));
            for source in [
                br#"list "\\m([join $operators |])\\M""#.as_slice(),
                br#"list "[set value]\\""#,
                br#"list "[set value]\n""#,
                br"list prefix-[set value]",
                br#"list """#,
            ] {
                let word = capture(source, 1, config);
                assert_eq!(
                    word.word_span().end() as usize,
                    source.len(),
                    "{dialect}: {source:?}"
                );
                assert_eq!(word.bytes(), &source[5..], "{dialect}: {source:?}");
                assert!(word.parts().is_ok(), "{dialect}: {source:?}");
            }
        }
    }

    #[test]
    fn brace_groups_preserve_literal_bytes_and_empty_group_geometry() {
        for source in [b"list {\xff\\n$x[side]}".as_slice(), b"list {}"] {
            let word = capture(source, 1, LexerConfig::default());
            let parts = word.parts().unwrap();
            assert_eq!(parts.len(), 1);
            assert!(matches!(&parts[0].part, WordPart::Text(bytes)
                if bytes.as_ref() == &source[6..source.len() - 1]));
            assert_eq!(word.word_span().end() as usize, source.len());
        }
    }

    #[test]
    fn selected_jim_quotes_concatenate_without_quote_bytes_or_unicode_names() {
        let config = LexerConfig::from_grammar(tcl_dialect::grammar_of_dialect_name(Some("jim")));
        let word = capture(b"list \"a\"\xff${\xfe}", 1, config);
        assert_eq!(
            word.content_span(),
            Err(NativeWordError::NonContiguousContent)
        );
        let parts = word.parts().unwrap();
        assert!(matches!(&parts[0].part, WordPart::Text(bytes) if bytes.as_ref() == b"a"));
        assert!(matches!(&parts[1].part, WordPart::Text(bytes) if bytes.as_ref() == b"\xff"));
        assert!(matches!(&parts[2].part, WordPart::Variable(variable) if variable.name == b"\xfe"));
    }

    #[test]
    fn executable_words_retain_full_index_depth_without_advisory_literal_fallback() {
        for config in [
            LexerConfig::default(),
            LexerConfig::from_grammar(tcl_dialect::grammar_of_dialect_name(Some("jim"))),
        ] {
            let mut source = b"list ".to_vec();
            for _ in 0..80 {
                source.extend_from_slice(b"$a(");
            }
            source.extend_from_slice(b"[compileMe]");
            source.extend(std::iter::repeat_n(b')', 80));
            let image = SourceImage::native(source);
            let tokens = Lexer::with_source_image(&image, config)
                .tokenise_all()
                .unwrap();
            let commands = group_commands_bytes(&tokens, image.bytes(), config);
            let word =
                NativeWord::from_group(image, config, &tokens, &commands[0].words[1]).unwrap();
            assert_eq!(
                word.parts(),
                Err(NativeWordError::SubstitutionDepthUnavailable)
            );
            assert_eq!(
                word.executable_parts()
                    .all_parts()
                    .filter(|part| matches!(part.part, ExecutablePart::Variable { .. }))
                    .count(),
                80
            );
            assert!(
                word.executable_parts()
                    .all_parts()
                    .any(|part| matches!(part.part, ExecutablePart::Command { body }
                    if word.executable_parts().bytes(body) == Some(b"compileMe".as_slice())))
            );
        }
    }

    #[test]
    fn malformed_groups_and_foreign_token_ranges_are_not_silently_empty() {
        let image = SourceImage::native(b"list {x}y".to_vec());
        let config = LexerConfig::default();
        let tokens = Lexer::with_source_image(&image, config)
            .tokenise_all()
            .unwrap();
        let commands = group_commands_bytes(&tokens, image.bytes(), config);
        assert_eq!(
            NativeWord::from_group(image.clone(), config, &tokens, &commands[0].words[1]),
            Err(NativeWordError::Parse(crate::EXTRA_AFTER_CLOSE_BRACE))
        );
        let image = SourceImage::native(b"list x".to_vec());
        let config = LexerConfig::default();
        let tokens = Lexer::with_source_image(&image, config)
            .tokenise_all()
            .unwrap();
        let commands = group_commands_bytes(&tokens, image.bytes(), config);
        let mut group = commands[0].words[1].clone();
        group.tokens = 0..0;
        assert_eq!(
            NativeWord::from_group(image, config, &tokens, &group),
            Err(NativeWordError::MisalignedGroup)
        );
    }
}
