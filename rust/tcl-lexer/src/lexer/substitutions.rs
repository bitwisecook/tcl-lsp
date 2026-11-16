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

//! Variable and command substitution tokens: `$name`, `${name}`, array indices and `[...]`.

use super::{LexError, Lexer, TokenProtocol};
use crate::span::Span;
use crate::tokens::{Token, TokenType};

impl Lexer<'_> {
    /// Parse a variable substitution starting at the current `$`.
    ///
    /// Handles all four Tcl forms:
    ///
    /// - `$name`, `$ns::var` — identifier scan accepting Unicode
    ///   alphanumerics, underscores, and `::` namespace separators.
    /// - `${name}` — braced scan; the closing `}` is consumed but
    ///   NOT included in the token span (`_end = pos - 1` before the
    ///   `}` advance).
    /// - `$arr(idx)` — array indexing with balanced `(`/`)` and
    ///   embedded `${…}` support. The `)` IS included in the span.
    /// - bare `$` — emitted as an `STR` token whose span covers
    ///   just the `$`.
    ///
    /// **Span convention.** The span always starts at the `$`
    /// position so the resolved start/end `SourcePosition`s include
    /// the dollar sign. The "human-readable" content (variable name
    /// without the leading `$` or `${`) is accessed via
    /// [`SourceMap::token_text`] rather than `SourceMap::text(span)`.
    ///
    /// Never fails. Unterminated `${` and `$arr(` tokenise
    /// best-effort, emitting non-fatal warnings once warning
    /// collection is in place.
    pub(super) fn parse_var(&mut self) -> Result<Token, LexError> {
        let dollar_pos = self.pos;
        self.pos += 1; // skip '$'

        // `JimTcl` expression sugar `$(…)`: with an *empty* variable name the
        // parenthesised text is an `expr` body, not a dict index. Jim reaches
        // this through the command-substitution scanner and retags the token
        // (`JimParseVar`, jim.c:1728-1732), so the body nests and terminates
        // like `[…]` — but it is an expression, so it gets its own kind.
        //
        // No build of the Tcl core does this: there `$(` starts an empty-name array
        // reference, which is why the axis is a grammar knob rather than a rule.
        if self.config.var_syntax.has_expr_sugar() && self.current_byte() == Some(b'(') {
            self.pos += 1; // skip '('
            let content_start = self.pos;
            self.skip_jim_paren_body();
            let content_empty = self.pos == content_start;
            let has_close = self.current_byte() == Some(b')');
            let span_end = if content_empty && has_close {
                self.pos + 1
            } else {
                self.pos
            };
            if has_close {
                self.pos += 1;
            } else if self.token_protocol != TokenProtocol::JimNative {
                self.warn_or_error(crate::ParseIssueKind::MissingExpressionParenthesis)?;
            }
            return Ok(Token::with_content_offset(
                TokenType::ExprSugar,
                Span::new(dollar_pos, span_end),
                2,
            ));
        }

        // `${name}` braced form.
        //
        // Per Tcl 9.0.3's parser (`tclParse.c::Tcl_ParseVarName`), this
        // is *not* a literal scan-to-first-`}`: the parser tracks inner
        // `{…}` with brace counting and consumes `\X` (a backslash plus
        // the following char) as part of the name — so `${a\}b}` reads
        // var `a\}b` and `${a{b}c}` reads var `a{b}c`. The Tcl(n) man
        // page's "no further substitution or modification" claim refers
        // only to `$` / `[` substitution; backslashes and inner braces
        // ARE recognised as syntax.'s
        // `_parse_var` braced branch.
        if self.current_byte() == Some(b'{') {
            self.pos += 1; // skip '{'
            let content_start = self.pos;
            self.skip_braced_var_name_body();
            let content_empty = self.pos == content_start;
            let has_close_brace = self.current_byte() == Some(b'}');
            let span_end = if content_empty && has_close_brace {
                self.pos + 1
            } else {
                self.pos
            };
            if has_close_brace {
                self.pos += 1;
            } else if self.token_protocol != TokenProtocol::JimNative {
                self.warn_or_error(crate::ParseIssueKind::MissingVariableBrace)?;
            }
            return Ok(Token::with_content_offset(
                TokenType::Var,
                Span::new(dollar_pos, span_end),
                2,
            ));
        }

        // `$name` or `$ns::var` identifier form. Bareword characters are the
        // ASCII `[0-9A-Za-z_]` ONLY (`TclIsBareword`, identical in 8.6.14 and
        // 9.0.1; the man page: "Letters and digits are only the standard
        // ASCII ones") — `$café` names the variable `caf` and `é` is ordinary
        // word text, and a `$` before a non-ASCII letter is a literal `$`.
        let name_start = self.pos;
        let name_end = tcl_core_types::naming::scan_var_name_end_with(
            self.source(),
            self.pos as usize,
            self.config.var_syntax.name_allows_high_bytes(),
        );
        self.pos = u32::try_from(name_end).expect("source offset fits u32");
        // `$arr(idx)` array-index form
        if self.current_byte() == Some(b'(') {
            if self.config.var_syntax.index_parens_nest() {
                // Jim counts nested parens with its own scanner, so
                // `$d(a(b))` indexes by the whole `a(b)`; C Tcl stops at the
                // first `)` at token level. Jim has no array-index byte
                // restriction either, so it does not go through
                // `scan_array_index_body`'s `ArrayIndexSyntax` gate.
                self.pos += 1; // skip '('
                self.skip_jim_paren_body();
                if self.current_byte() == Some(b')') {
                    self.pos += 1;
                } else if self.token_protocol != TokenProtocol::JimNative {
                    self.warn_or_error(crate::ParseIssueKind::MissingArrayIndexParenthesis)?;
                }
            } else {
                self.scan_array_index_body()?;
            }
            return Ok(Token::with_content_offset(
                TokenType::Var,
                Span::new(dollar_pos, self.pos),
                1,
            ));
        }

        // Bare `$`
        if self.pos == name_start {
            return Ok(Token::new(
                if self.token_protocol == TokenProtocol::JimNative {
                    TokenType::Esc
                } else {
                    TokenType::Str
                },
                Span::new(dollar_pos, dollar_pos + 1),
            ));
        }

        Ok(Token::with_content_offset(
            TokenType::Var,
            Span::new(dollar_pos, self.pos),
            1,
        ))
    }

    /// Consume a `JimTcl` parenthesised body, from just inside the opening
    /// `(` to the matching `)`, leaving `self.pos` **at** that `)` when the
    /// body is balanced.
    ///
    /// This is Jim's one paren scanner, shared by `$(expr)` sugar and by
    /// `$name(index)` (`JimParseVar`, jim.c:1698-1727). It counts nesting and
    /// lets a backslash hide the next byte; unlike C Tcl's index scanner it
    /// does **not** recognise `[...]`, `{...}` or quotes, so a `)` inside a
    /// nested command substitution still closes the body.
    ///
    /// Jim's own recovery is reproduced: if the input runs out while still
    /// nested but some `)` was seen, the body is taken to end just after the
    /// **last** `)` — so `$a((b)` is the index `(b`, not a run to end of
    /// source. When no `)` was seen at all the scan stops at end of input and
    /// the caller reports the missing close.
    fn skip_jim_paren_body(&mut self) {
        self.pos = u32::try_from(crate::ranges::jim_parenthesis_body_end(
            self.source(),
            self.pos as usize,
        ))
        .expect("source offset fits u32");
    }

    /// Consume a `(…)` array-index body starting at the `(`.
    ///
    /// C Tcl (`Tcl_ParseVarName` → `Tcl_ParseTokens` with a `)` terminator)
    /// does **not** count paren nesting: the index ends at the first `)` at
    /// token level. A literal `(` is plain text; a `)` is only kept in the
    /// index when it is escaped (`\)`) or sits inside a command substitution
    /// (`[…]`) or a nested variable reference (`${…}` / `$name(…)`) — whose
    /// tokens are scanned so their inner `)` are not mistaken for the
    /// terminator. Paren-counting made `$a((b)` never close (swallowing the
    /// rest of the source) and made `$a(x\)y)` end at the escaped `)`.
    /// Advances `self.pos` past the closing `)` (or to EOF
    /// for unterminated input).
    ///
    /// The structural scan is delegated to [`crate::scan_array_index`], the
    /// iterative owner shared with expression lexing and both runtimes.
    pub(super) fn scan_array_index_body(&mut self) -> Result<(), LexError> {
        debug_assert_eq!(self.current_byte(), Some(b'('));
        let scan = crate::scan_array_index_in(self.source(), self.pos as usize, self.config);
        if let Some(invalid) = scan.invalid {
            self.pos = u32::try_from(invalid).expect("source offset fits u32");
            self.warn_or_error(crate::ParseIssueKind::InvalidArrayIndexCharacter)?;
        }
        match scan.end {
            crate::ArrayIndexEnd::Closed(end) => {
                self.pos = u32::try_from(end).expect("source offset fits u32");
            }
            crate::ArrayIndexEnd::Unterminated => {
                self.pos = u32::try_from(self.source().len()).expect("source length fits u32");
                self.warn_or_error(crate::ParseIssueKind::MissingParenthesis)?;
            }
        }
        Ok(())
    }

    /// Advance from the first byte of a `${…}` variable **name** to the `}`
    /// that closes it (or to end-of-input when the form is unterminated),
    /// leaving `pos` *on* the closer.
    ///
    /// The release-aware close rule itself lives in
    /// [`crate::ranges::braced_var_name_end`], the one owner both `subst`
    /// engines resolve `${…}` through as well.
    ///
    /// An unterminated form is C's `missing close-brace for variable name`
    /// error, but this lexer is the *tokeniser* — it must keep producing tokens
    /// for half-typed source, so it takes the documented lenient recovery and
    /// runs the name to end-of-input. The evaluating engines raise instead.
    fn skip_braced_var_name_body(&mut self) {
        let end = match crate::ranges::braced_var_name_end(
            self.source(),
            self.pos as usize,
            self.config.braced_var,
        ) {
            crate::ranges::BracedVarEnd::Closed(end) => end,
            crate::ranges::BracedVarEnd::Unterminated => self.source().len(),
        };
        self.pos = u32::try_from(end).expect("source offset fits u32");
    }

    /// Finish a `[...]` token after its body scan stopped at a real/ghost
    /// closer or EOF. Empty command bodies include their real closer so the
    /// inner-end span still lands on it; recovery ghosts remain zero-width.
    fn finish_command_token(
        &mut self,
        bracket_pos: u32,
        content_start: u32,
    ) -> Result<Token, LexError> {
        let content_empty = self.pos == content_start;
        let close_is_ghost = self.ghost_at(self.pos) == Some(b']');
        let has_close_bracket = self.current_byte() == Some(b']');
        let span_end = if content_empty && has_close_bracket && !close_is_ghost {
            self.pos + 1
        } else {
            self.pos
        };
        if close_is_ghost {
            self.ghosts.remove(&self.pos);
        } else if has_close_bracket {
            self.pos += 1;
        } else {
            if self.token_protocol == TokenProtocol::JimNative && self.jim_missing != Some(b'"') {
                self.jim_missing = Some(b'[');
                self.jim_missing_line = Some(bracket_pos);
            }
            self.warn_or_error(crate::ParseIssueKind::MissingBracket)?;
        }
        Ok(Token::with_content_offset(
            TokenType::Cmd,
            Span::new(bracket_pos, span_end),
            1,
        ))
    }

    fn scan_jim_command_body(&mut self, bracket: u32) {
        // These modes extend the same source scanner with Jim's bracket grammar.
        // Quotes are structural at word start; braces are structural everywhere.
        let mut modes = vec![(b'[', true)];
        while let Some(byte) = self.current_byte() {
            let Some(&(mode, start_word)) = modes.last() else {
                break;
            };
            match (mode, byte) {
                (_, b'\\') => {
                    self.pos += 1;
                    if self.current_byte().is_some() {
                        self.pos += 1;
                    }
                }
                (b'{', b'{') => {
                    modes.push((b'{', false));
                    self.pos += 1;
                }
                (b'{', b'}') | (b'"', b'"') => {
                    modes.pop();
                    if let Some(last) = modes.last_mut() {
                        last.1 = false;
                    }
                    self.pos += 1;
                }
                (b'{', _) => self.pos += 1,
                (b'[', b']') => {
                    modes.pop();
                    if modes.is_empty() {
                        return;
                    }
                    if let Some(last) = modes.last_mut() {
                        last.1 = false;
                    }
                    self.pos += 1;
                }
                (b'[' | b'"', b'[') => {
                    if let Some(last) = modes.last_mut() {
                        last.1 = false;
                    }
                    modes.push((b'[', true));
                    self.pos += 1;
                }
                (b'[', b'"') if start_word => {
                    modes.push((b'"', false));
                    self.pos += 1;
                }
                (b'[', b'{') => {
                    if let Some(last) = modes.last_mut() {
                        last.1 = false;
                    }
                    modes.push((b'{', false));
                    self.pos += 1;
                }
                _ => {
                    if let Some(last) = modes.last_mut() {
                        last.1 = byte.is_ascii_whitespace();
                    }
                    self.pos += 1;
                }
            }
        }
        let marker = if modes.iter().any(|&(mode, _)| mode == b'"') {
            b'"'
        } else {
            b'['
        };
        self.jim_missing = Some(marker);
        self.jim_missing_line = Some(bracket);
    }

    /// Lex a `[…]` command substitution. Where it ends is the shared
    /// scanner's answer ([`crate::ranges::command_substitution_end`]): the
    /// nested script read word by word as `Tcl_ParseCommand` reads it, over
    /// the document's line continuations, with a recovery ghost `]` closing
    /// the innermost substitution open where it sits.
    pub(super) fn parse_command(&mut self) -> Result<Token, LexError> {
        let bracket_pos = self.pos;
        let content_start = self.pos + 1;
        if self.token_protocol == TokenProtocol::JimNative {
            self.pos = content_start; // skip '['
            self.scan_jim_command_body(bracket_pos);
            return self.finish_command_token(bracket_pos, content_start);
        }
        // The document channel continues a line at a backslash before a CR
        // or CRLF as well; an original value continues it only at
        // backslash-newline.
        let continuation = if self.source_map.channel() == crate::SourceChannel::Document {
            crate::ranges::Continuation::Document
        } else {
            crate::ranges::Continuation::Script
        };
        let mut nested_ghosts = Vec::new();
        let ghosts = &self.ghosts;
        let end = crate::ranges::scan_command_substitution(
            self.source(),
            bracket_pos as usize,
            continuation,
            self.config.braced_var,
            &mut |pos| {
                let offset = u32::try_from(pos).expect("source offset fits u32");
                let seen = ghosts.get(&offset) == Some(&b']');
                if seen {
                    nested_ghosts.push(offset);
                }
                seen
            },
        );
        let stop = match end {
            crate::ranges::SubstitutionEnd::Closed(end) => end - 1,
            crate::ranges::SubstitutionEnd::Ghost(at) => {
                nested_ghosts.retain(|&offset| offset as usize != at);
                at
            }
            crate::ranges::SubstitutionEnd::Unterminated => self.source().len(),
        };
        for offset in nested_ghosts {
            self.ghosts.remove(&offset);
        }
        self.pos = u32::try_from(stop).expect("source offset fits u32");
        self.finish_command_token(bracket_pos, content_start)
    }
}
