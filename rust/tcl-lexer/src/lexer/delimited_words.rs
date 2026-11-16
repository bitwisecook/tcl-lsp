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

//! Quoted and braced words and the `{*}` expansion prefix.

use super::{LexError, Lexer, TokenProtocol, is_separator_byte};
use crate::span::Span;
use crate::tokens::{Token, TokenType};

impl Lexer<'_> {
    /// Parse a quoted-string `ESC` token.
    ///
    /// Called from the iterator in two modes:
    ///
    /// - `opening = true`: the iterator has seen a `"` at a word
    ///   boundary (top-level, `!in_quote`, `is_newword()`). Skip
    ///   the opening `"`, set `in_quote = true`, then scan content
    ///   until a terminator.
    /// - `opening = false`: the iterator is already `in_quote`.
    ///   Scan content from the current position until a terminator.
    ///   If the terminator is the closing `"`, consume it and
    ///   reset `in_quote = false`.
    ///
    /// The content scan stops at `$`, `[`, `"`, or `\`, or EOF.
    /// `$` and `[` are left in the stream for the iterator to
    /// dispatch to `parse_var` / `parse_command` on the next
    /// iteration. `\` is left in the stream so the iterator can
    /// surface it as an unsupported character (proper handling is
    /// not yet implemented). Everything else — separators, EOL characters,
    /// `#`, `{`, `}` — is consumed as literal content.
    ///
    /// ### Span convention
    ///
    /// The span starts at `start_offset` (the opening `"` when
    /// `opening`, or the first content byte otherwise) and normally
    /// ends at the position where the scanner stopped. In the
    /// **empty-content** case — where the scanner stopped
    /// immediately without consuming any content — the span is
    /// extended by one byte to cover the stop character itself,
    /// so the end position lands on it, for the degenerate cases:
    ///
    /// - `""` — empty body, stop char is the closing `"`. Span
    ///   covers `""`; `token_text` returns `""`.
    /// - `"$foo"` opening ESC — stop char is `$`. Span covers
    ///   `"$`; `token_text` returns `""` and the next dispatch
    ///   handles the `$`.
    /// - `"[cmd]"` opening ESC — same, stop char is `[`.
    /// - Closing empty ESC after a sub-token (`$foo"` after a
    ///   VAR) — stop char is the closing `"`. Span covers `"`;
    ///   `token_text` returns `""`.
    ///
    /// Never fails. An unterminated quoted string tokenises
    /// best-effort — the scanner consumes everything up to EOF
    /// and returns an `ESC` with `in_quote = true` still set, so
    /// the trailing synthetic EOL inherits the `true` flag too.
    /// The "missing close-quote" warning is not yet emitted.
    pub(super) fn parse_quoted(&mut self, opening: bool) -> Result<Token, LexError> {
        let start_offset = self.pos;
        if opening {
            self.pos += 1; // skip opening `"`
            self.in_quote = true;
            self.jim_missing_line = self.jim_token_start;
        }
        let content_start = self.pos;
        let mut closed = false;

        while let Some(ch) = self.current_char() {
            if self.jim_parenthesis_fragment(content_start) {
                break;
            }
            match ch {
                '"' => {
                    closed = true;
                    break;
                }
                '$' | '[' => break,
                '\\' => {
                    // Inside a quoted string, `\<char>` is consumed
                    // as a literal pair (both the backslash and
                    // the following character stay in the token
                    // text). `\<newline>` inside a quote is NOT a
                    // word break — it's just another pair of
                    // literal bytes.
                    if self.token_protocol == TokenProtocol::JimNative
                        && self.peek_byte(1).is_none()
                    {
                        self.jim_missing = Some(b'\\');
                    }
                    let end = self.source_backslash_escape_end();
                    self.pos = u32::try_from(end).expect("source offset fits u32");
                }
                _ => {
                    self.pos += 1;
                }
            }
        }

        let content_empty = self.pos == content_start;
        // Empty-content clamp: extend the span by one byte so the
        // end position lands on the terminator (closing `"`, or the
        // `$` / `[` that stopped the scan). Don't extend at EOF
        // with no terminator — that would push `span.end` past
        // `source.len()` and make `SourceMap::text` panic on
        // slicing.
        let has_stop = closed || self.current_char().is_some();
        let span_end = if content_empty && has_stop {
            self.pos + 1
        } else {
            self.pos
        };

        if closed {
            self.pos += 1; // advance past closing `"`
            self.in_quote = false;
            // Check for extra characters after close-quote. `JimTcl` has no
            // such check anywhere — it clears `inquote` and keeps scanning,
            // so `puts "abc"def` is the single word `abcdef` rather than a
            // parse error (`JimParseStr`, jim.c:1748).
            if let Some(after) = self
                .current_byte()
                .filter(|_| self.config.quote_termination.is_strict())
            {
                let is_bs_nl = self.source_is_line_continuation();
                if self.config.irules_brace_separator {
                    // F5 R2 (measurements §1): a word that started with
                    // `"` splits exactly as a braced one — `"a"b` →
                    // `a`/`b`, `"a""b"` → `a`/`b`, `"a"}` → `a`/`}` —
                    // with no diagnostic (R6).
                    if !is_separator_byte(after, self.config.word_separators) && !is_bs_nl {
                        let sep_span = Span::empty(self.pos);
                        self.pending_sep = Some(Token::new(TokenType::Sep, sep_span));
                    }
                } else {
                    let ok = is_separator_byte(after, self.config.word_separators)
                        || after == b']'
                        || is_bs_nl;
                    if !ok {
                        self.warn_or_error(crate::ParseIssueKind::ExtraAfterQuote)?;
                    }
                }
            }
        } else if self.current_char().is_none() {
            // EOF without closing quote.
            if self.token_protocol == TokenProtocol::JimNative {
                self.jim_missing = Some(b'"');
            }
            self.warn_or_error(crate::ParseIssueKind::MissingQuote)?;
        }

        let content_offset: u8 = if !opening && content_empty && closed {
            // A bare closing-quote sub-token (the trailing `"` of `"$foo"`,
            // emitted as a 1-byte ESC): the `"` is a leading delimiter of a
            // zero-content token, so `content_offset == 1` makes
            // `SourceMap::token_text` yield `""`. This distinguishes it from a
            // *literal* trailing `"` in a bare word, which `parse_esc` emits
            // with `content_offset == 0` and `in_quote == false`.
            // The opening / mid-string cases keep `opening.into()` so the
            // semantic-tokens fragment logic (which trims an extended `$`/`[`
            // introducer keyed on `content_offset`) is unaffected.
            1
        } else {
            opening.into()
        };
        Ok(Token::with_content_offset(
            TokenType::Esc,
            Span::new(start_offset, span_end),
            content_offset,
        ))
    }

    /// Whether the next token is at a word boundary (so `{` at
    /// `self.pos` starts a braced string rather than being part of a
    /// bare word) — true when the previously emitted token was `Sep`,
    /// `Eol`,
    /// `Str`, or `Expand`. The initial state before any token is
    /// emitted is `Eol`, so the first character of the source is
    /// always at a word boundary.
    #[inline]
    pub(super) fn is_newword(&self) -> bool {
        matches!(
            self.last_kind,
            TokenType::Sep | TokenType::Eol | TokenType::Str | TokenType::Expand
        )
    }

    /// Check for the `{*}` expansion prefix (Tcl 8.5+) at the
    /// current `{`, and dispatch to either `parse_expand` or
    /// `parse_brace`. The condition is:
    ///
    /// ```text
    /// if expand_syntax
    ///     and text[pos+1] == '*'
    ///     and text[pos+2] == '}'
    ///     and text[pos+3] is a non-separator
    /// ```
    pub(super) fn parse_brace_or_expand(&mut self) -> Result<Token, LexError> {
        if self.token_protocol != TokenProtocol::JimNative
            && self.config.expand_syntax
            && self.peek_byte(1) == Some(b'*')
            && self.peek_byte(2) == Some(b'}')
            && let Some(after) = self.peek_byte(3)
            && !is_separator_byte(after, self.config.word_separators)
        {
            return Ok(self.parse_expand());
        }
        self.parse_brace()
    }

    /// Emit an `EXPAND` token for the `{*}` prefix. The token is a
    /// zero-width marker anchored at the `{` position (`span =
    /// [pos, pos)`), so `range_positions` gives start == end there.
    /// The lexer then advances `self.pos` past the full `{*}` so
    /// the next dispatch starts at the word to expand.
    fn parse_expand(&mut self) -> Token {
        let start = self.pos;
        self.pos += 3; // skip `{*}`
        // Emit an empty span anchored at the `{` position.
        Token::new(TokenType::Expand, Span::empty(start))
    }

    /// Parse a braced string starting at the current `{`. The caller
    /// is responsible for checking `is_newword()` before dispatching
    /// here — a `{` in the middle of a bare word is a regular
    /// character in `parse_esc`, not a braced-string opener.
    ///
    /// The scanner counts balanced `{` / `}` pairs (`level` starts
    /// at 1 after skipping the opening `{`). Backslash sequences use the
    /// shared raw-parser escape width, but
    /// the backslash and the following character are preserved
    /// literally in the token text — braces are "verbatim" in Tcl,
    /// so `\}` inside a brace body does NOT count as a close brace
    /// even though the scanner skips over it.
    ///
    /// The span starts at the `{` and normally ends at the last
    /// character of the body (NOT the closing `}`). The
    /// empty-body degenerate `{}` extends the span by one so
    /// `range_positions` reports the `}` as the end position.
    /// `SourceMap::token_text` strips the leading `{` (and the
    /// trailing `}` for the degenerate `{}` case) so callers
    /// see just the inside of the braces.
    ///
    /// Never fails. Unterminated `{` tokenises best-effort. Under the F5
    /// word-break axis (`irules_brace_separator`), a non-separator after
    /// the close brace injects a ghost SEP instead of the "extra
    /// characters after close-brace" warning — see the R2 comment below.
    pub(super) fn parse_brace(&mut self) -> Result<Token, LexError> {
        let brace_pos = self.pos;
        self.pos += 1; // skip opening '{'
        let content_start = self.pos;

        let mut level: u32 = 1;
        self.max_brace_word_nesting = self.max_brace_word_nesting.max(level);
        let mut span_end: u32 = self.pos;

        while let Some(ch) = self.current_char() {
            match ch {
                '\\' => {
                    // Consume the source-channel escape span.
                    let end = self.source_backslash_escape_end();
                    self.pos = u32::try_from(end).expect("source offset fits u32");
                }
                '{' => {
                    level += 1;
                    self.max_brace_word_nesting = self.max_brace_word_nesting.max(level);
                    self.pos += 1;
                }
                '}' => {
                    level -= 1;
                    if level == 0 {
                        // Leave `self.pos` at the closing `}`; the
                        // code after the loop decides whether to
                        // include it in the span and consume it.
                        span_end = self.pos;
                        break;
                    }
                    self.pos += 1;
                }
                _ => {
                    self.pos += 1;
                }
            }
        }

        // At this point `self.pos` is at the closing `}` (level
        // reached zero) or at EOF. `span_end` was set inside the
        // `}` branch; if the loop fell through (EOF), set it now.
        if level > 0 {
            span_end = self.pos;
        }

        let content_empty = span_end == content_start;
        let has_close_brace = self.current_byte() == Some(b'}');
        let final_span_end = if content_empty && has_close_brace {
            span_end + 1 // include the `}` so the end position lands on it
        } else {
            span_end
        };
        if has_close_brace {
            self.pos += 1;
            // Check for extra characters after close-brace.
            if let Some(after) = self.current_byte()
                && !is_separator_byte(after, self.config.word_separators)
            {
                // Backslash-newline after close-brace is fine
                let is_bs_nl = self.source_is_line_continuation();
                if !is_bs_nl {
                    if self.config.irules_brace_separator {
                        // F5 R2 (measurements §1): the word started with
                        // `{`, so ANY non-separator after its close brace
                        // begins a new word — a zero-width ghost SEP,
                        // with no diagnostic (R6). The next word parses
                        // from scratch under ordinary word-start rules
                        // (R3): `{a}{b}` → `a`/`b`, `{a}b` → `a`/`b`,
                        // `{*}$l` → literal `*` word plus `$l` (the
                        // separator wins; expansion does not exist).
                        let sep_span = Span::empty(self.pos);
                        self.pending_sep = Some(Token::new(TokenType::Sep, sep_span));
                    } else {
                        // One spelling, shared with the eval-facing consumer
                        // that turns this recoverable warning into C's hard
                        // error (`runtime/rust`, via
                        // `script::WordSpan::welded_after_close`).
                        self.warn_or_error(crate::ParseIssueKind::ExtraAfterBrace)?;
                    }
                }
            }
        } else {
            if self.token_protocol == TokenProtocol::JimNative {
                self.jim_missing = Some(b'{');
                self.jim_missing_line = Some(brace_pos);
            }
            self.warn_or_error(crate::ParseIssueKind::MissingBrace)?;
        }

        Ok(Token::with_content_offset(
            TokenType::Str,
            Span::new(brace_pos, final_span_end),
            1,
        ))
    }
}
