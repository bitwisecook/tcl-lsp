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

//! SCF shell word and delimiter geometry, independent of embedded Tcl grammar.

use std::{borrow::Cow, ops::Range};

/// Lexical role of a token in the outer SCF stanza.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShellTokenKind {
    /// A shell word, including its quoted or escaped fragments.
    Word,
    /// An unquoted opening body brace.
    Open,
    /// An unquoted closing body brace.
    Close,
    /// A physical, unescaped newline between words.
    Newline,
}

/// A token and its original byte range in an SCF document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShellToken<'a> {
    /// Original UTF-8 byte extent, including quote delimiters and escapes.
    pub range: Range<usize>,
    /// The role selected by the outer SCF tokenizer.
    pub kind: ShellTokenKind,
    /// False for a word with an unfinished quote or final backslash.
    pub complete: bool,
    raw: &'a str,
}

impl<'a> ShellToken<'a> {
    /// The original source spelling, including its delimiters.
    #[must_use]
    pub fn spelling(&self) -> &'a str {
        self.raw
    }

    /// Join quoted and unquoted fragments, removing quoting and backslash escapes.
    #[must_use]
    pub fn word_value(&self) -> Option<Cow<'a, str>> {
        self.word(false)
    }

    /// Join quoted and unquoted fragments but preserve the written escape pair
    /// for callers (e.g. APM expressions) that must retain the escaped bytes.
    #[must_use]
    pub fn word_with_escapes(&self) -> Option<Cow<'a, str>> {
        self.word(true)
    }

    fn word(&self, keep_escape: bool) -> Option<Cow<'a, str>> {
        if self.kind != ShellTokenKind::Word {
            return None;
        }
        if !self.raw.contains(['"', '\\']) {
            return Some(Cow::Borrowed(self.raw));
        }
        let mut out = String::with_capacity(self.raw.len());
        read_word(self.raw, Mode::Value, |piece, escaped| {
            out.push_str(if escaped && !keep_escape {
                &piece[1..]
            } else {
                piece
            });
        });
        Some(Cow::Owned(out))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Stanzas use physical newlines, braces, ASCII spacing and line comments.
    Stanza,
    /// A standalone header has already had its body brace removed.
    Header,
    /// Decode a complete previously delimited word.
    Value,
}

fn delimiter(c: char, mode: Mode) -> bool {
    match mode {
        Mode::Stanza => matches!(c, ' ' | '\t' | '\r' | '\n' | '{' | '}'),
        Mode::Header => c.is_whitespace(),
        Mode::Value => false,
    }
}

/// Returns byte extent, completeness and physical continuation to a new line.
fn read_word(text: &str, mode: Mode, mut part: impl FnMut(&str, bool)) -> (usize, bool, bool) {
    let mut iter = text.char_indices();
    let mut quoted = false;
    while let Some((pos, c)) = iter.next() {
        if c == '"' {
            quoted = !quoted;
        } else if c == '\\' {
            let Some((next, escaped)) = iter.next() else {
                part(&text[pos..], true);
                return (text.len(), false, false);
            };
            let end = next + escaped.len_utf8();
            part(&text[pos..end], true);
            if mode == Mode::Stanza && !quoted && escaped == '\n' {
                return (end, true, true);
            }
        } else if !quoted && delimiter(c, mode) {
            return (pos, true, false);
        } else {
            part(&text[pos..pos + c.len_utf8()], false);
        }
    }
    (text.len(), !quoted, false)
}

/// Typed source extents for the outer shell. Newlines inside quoted words and
/// escaped brace bytes stay in their word. Comments at line start are skipped.
pub fn shell_tokens(source: &str) -> impl Iterator<Item = ShellToken<'_>> {
    tokenize(source, Mode::Stanza)
}

pub(super) fn header_words(header: &str) -> impl Iterator<Item = ShellToken<'_>> {
    tokenize(header, Mode::Header)
}

fn tokenize(source: &str, mode: Mode) -> impl Iterator<Item = ShellToken<'_>> {
    let mut pos = 0;
    let mut blank = true;
    std::iter::from_fn(move || {
        loop {
            let text = source.get(pos..)?;
            let c = text.chars().next()?;
            let start = pos;
            if mode == Mode::Stanza && blank && c == '#' {
                pos += text.find('\n').unwrap_or(text.len());
                continue;
            }
            let (kind, complete) = if mode == Mode::Stanza && matches!(c, '{' | '}' | '\n') {
                pos += 1;
                blank = c == '\n';
                (
                    match c {
                        '{' => ShellTokenKind::Open,
                        '}' => ShellTokenKind::Close,
                        _ => ShellTokenKind::Newline,
                    },
                    true,
                )
            } else if delimiter(c, mode) {
                pos += c.len_utf8();
                continue;
            } else {
                let (extent, complete, next_blank) = read_word(text, mode, |_, _| {});
                pos += extent;
                blank = next_blank;
                (ShellTokenKind::Word, complete)
            };
            return Some(ShellToken {
                range: start..pos,
                raw: &source[start..pos],
                kind,
                complete,
            });
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn comments_literal_braces_quoted_empty_and_joined_words_keep_source_ranges() {
        let source = r#" # {
stat "" pré"fix {x}" end\ \{ } "é"#;
        let tokens: Vec<_> = shell_tokens(source).collect();
        for token in &tokens {
            assert_eq!(token.spelling(), &source[token.range.clone()]);
        }
        let values: Vec<_> = tokens.iter().filter_map(ShellToken::word_value).collect();
        assert_eq!(values, ["stat", "", "préfix {x}", "end {", "é"]);
        assert!(!tokens.last().unwrap().complete);
    }
}
