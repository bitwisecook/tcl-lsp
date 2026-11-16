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

//! Streaming Tcl lexer.
//!
//! Handles every top-level Tcl construct:
//!
//! - **EOF handling** — emits a trailing ghost `EOL` (once) when the
//!   source does not already end with an EOL token.
//! - **SEP** — runs of horizontal whitespace (`' '`, `\t`, `\r`, VT, FF).
//!   A native value's `\r` is horizontal whitespace, not an EOL. A
//!   Document whose dialect's `source` uses `-translation auto` (every C
//!   Tcl release; [`LexerConfig::source_line_endings`]) reads a lone `\r`
//!   as LF ([`crate::SourceChannel::script_byte`]), so there a lone CR ends
//!   the command. The CR of a CRLF pair stays horizontal whitespace.
//! - **EOL** — runs of EOL characters (`\n`, `;`) interleaved with
//!   horizontal whitespace.
//! - **COMMENT** — `#` at command start, scanned to the next `\n`
//!   (exclusive). Backslash-newline continuation inside comments is
//!   not handled yet; an input whose comment contains a `\` is
//!   reported as a `SyntaxError` so the differential harness can
//!   filter it.
//! - **ESC** — runs of characters that are neither whitespace nor EOL
//!   nor one of the special characters. Terminated by
//!   `$` or `[` so that variable and command substitutions
//!   dispatch on the next iteration.
//! - **VAR** — variable substitution in all four Tcl forms:
//!   `$name`, `$ns::var` (namespace-separated), `${name}` (braced),
//!   and `$arr(idx)` (array index with nested parens and embedded
//!   `${...}` support). A bare `$` with no name following is emitted
//!   as an `STR` token whose span covers just the `$`. Unterminated
//!   `${` and `$arr(` tokenise best-effort (warning collection reports
//!   them as diagnostics).
//! - **CMD** — command substitution `[…]`. The scanner tracks four
//!   pieces of state while inside the command body: outer bracket
//!   nesting (`level`), brace nesting (`blevel`), whether we are
//!   inside a `"…"` quoted sub-region (`in_quotes`), and Tcl comment
//!   state (`in_comment` / `at_command_start`). A `[` increments
//!   `level` only when not inside braces, quotes, or a comment; a `]`
//!   closes the command only when the outer bracket is the innermost
//!   nesting and it is not commented out. Backslash escapes use the shared
//!   raw-parser width after the document source-channel seam (CRLF is one
//!   logical newline); `${…}`
//!   sub-scans exist to stop a `)` or `}` inside a braced variable
//!   name from fooling the counter. Unterminated `[` tokenises
//!   best-effort.
//! - **STR** — braced strings `{…}`. Emitted when a `{` appears
//!   at a word boundary (the previous token was `EOL` / `SEP` / `STR`
//!   / `EXPAND`). The body is scanned with balanced `{` / `}` counting;
//!   backslash sequences consume two characters as a pair (backslash is
//!   inert inside braces — the backslash and the following character
//!   are retained literally in the token text). A `{` that is NOT at a
//!   word boundary is a regular word character in the enclosing `ESC`
//!   token. Unterminated `{` tokenises best-effort.
//! - **Quoted ESC** — `"…"` quoted strings emit `ESC` tokens
//!   carrying the `in_quote = true` flag for the duration of the
//!   quoted run. The lexer keeps an `in_quote: bool` field that is
//!   toggled on the opening and closing `"`; while it is set, the
//!   dispatch ignores separators, EOL characters, `#`, and `{` (all
//!   of which become literal content) and handles only the four
//!   active constructs `$`, `[`, `\`, and `"`. The opening `"` is
//!   captured in the first emitted `ESC`'s span (with
//!   `token_text` stripping it), not as a separate token; if the
//!   body begins with `$` or `[` the first `ESC` is an empty-body
//!   token whose span is extended to cover the terminator. Sub-tokens
//!   (`VAR`, `CMD`) emitted inside a quoted run carry
//!   `in_quote = true`; the **last** `ESC` before the closing `"` and
//!   the closing `"` itself (emitted as a possibly empty `ESC`) reset
//!   `in_quote` to `false` before the token is returned. A `"` mid-word
//!   (not at a word boundary) is a regular character inside the
//!   enclosing `ESC`.
//!
//! ### Architecture
//!
//! Tokens are pure data: a `TokenType`, a byte [`Span`], and an
//! `in_quote` flag — nothing more. Text and `(line, character,
//! offset)` positions are resolved on demand via the lexer's
//! [`SourceMap`]. The lexer tracks only a byte `pos: u32` and a small
//! amount of behavioural state (`at_command_start`, `last_kind`,
//! `done`); there is no incremental column bookkeeping.
//!
//! Every positional entity (Tokens now, IR and CFG nodes elsewhere)
//! carries only a span, and a single `SourceMap` per document is the
//! canonical place that resolves spans to text and positions.
//!
//! ### Offsets and columns
//!
//! [`SourcePosition::offset`] is a byte offset. The `character` field
//! is **byte offset within the line** (`col = offset - line_start`).
//! ASCII parity is exact; non-ASCII drifts from the LSP UTF-16
//! contract — multi-byte column parity is not yet handled.
//!
//! ### Not yet implemented
//!
//! - backslash escapes and line continuation; warning collection
//!   (which will turn the best-effort recovery of unterminated
//!   `${` / `$arr(` into proper diagnostics); ghost character
//!   insertion for error recovery. "Ghost" is our term of art
//!   (chosen over "synthetic" / "virtual" to avoid collisions with
//!   Rust vocabulary — `virtual` is a reserved keyword) for tokens
//!   and characters that exist in the token stream without
//!   corresponding bytes in the source buffer.
//! - sub-lexing support for nested constructs; UTF-16 column
//!   parity; `LineIndex::from_rope_slice` adapter
//!
//! [`Span`]: crate::Span
//! [`SourceMap`]: crate::SourceMap
//! [`SourcePosition`]: crate::SourcePosition
//! [`SourcePosition::offset`]: crate::SourcePosition#structfield.offset

use thiserror::Error;

use crate::source_map::SourceMap;
use crate::span::Span;
use crate::tokens::{Token, TokenType};

mod config;
mod delimited_words;
mod jim_rosters;
mod substitutions;

pub use config::{LeadingBom, LexerConfig, UTF8_BOM};

/// Errors produced by the Tcl lexer.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum LexError {
    /// A syntax error detected in strict-quoting mode.
    #[error("{message}")]
    SyntaxError {
        /// Structured issue retained at its lexical creation site.
        kind: crate::ParseIssueKind,
        /// Human-readable message.
        message: String,
    },
}

/// A non-fatal warning collected during lexing. The analyser
/// harvests these to produce LSP diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexWarning {
    /// Structured issue retained independently of its diagnostic presentation.
    pub kind: crate::ParseIssueKind,
    /// Byte offset in the source where the issue was detected.
    pub offset: u32,
    /// Human-readable message.
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TokenProtocol {
    General,
    JimNative,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JimPrecedingToken {
    Other,
    ScalarVariable,
}

/// Streaming Tcl lexer.
///
/// Produces [`Token`]s via the [`Iterator`] impl. Each token carries
/// only a [`Span`]; text and positions are resolved through the
/// lexer's [`SourceMap`], available via [`Lexer::source_map`] and
/// [`Lexer::into_source_map`].
#[derive(Debug)]
pub struct Lexer<'src> {
    source_map: SourceMap<'src>,
    /// Byte offset of the next character to consume.
    pos: u32,
    /// Whether the next token starts a new command. Set on construction
    /// and after every EOL; preserved across SEP tokens.
    at_command_start: bool,
    /// Whether we are currently inside a `"…"` quoted string.
    in_quote: bool,
    /// Ghost SEP token injected by `parse_brace` / `parse_quoted` when
    /// `irules_brace_separator` is set and the close delimiter of a
    /// brace- or quote-started word is immediately followed by a
    /// non-separator (F5 R2, measurements §1).
    pending_sep: Option<Token>,
    token_protocol: TokenProtocol,
    jim_missing: Option<u8>,
    jim_missing_line: Option<u32>,
    jim_token_start: Option<u32>,
    jim_preceding_token: JimPrecedingToken,
    /// Non-fatal warnings collected during lexing (unterminated
    /// braces, extra chars after close-quote, etc.).
    warnings: Vec<LexWarning>,
    /// Maximum balanced-brace level observed by the actual braced-word scanner.
    max_brace_word_nesting: u32,
    /// Kind of the most recently emitted token. Used to decide whether
    /// EOF needs a trailing ghost EOL and to compute
    /// [`Lexer::is_newword`].
    last_kind: TokenType,
    /// Once true, [`Iterator::next`] returns `None`.
    done: bool,
    config: LexerConfig,
    /// Zero-width "ghost" closing delimiters injected for error
    /// recovery, keyed by source byte offset (value is the delimiter
    /// byte, e.g. `b']'`).  When the scanner reaches a ghost offset it
    /// *sees* the ghost byte before the real one; consuming a ghost is
    /// zero-width (it removes the entry without advancing `pos`), so an
    /// unterminated `[foo bar` re-lexes as a terminated command without
    /// shifting any downstream offsets.  Empty on the normal lexing path.
    ghosts: std::collections::BTreeMap<u32, u8>,
}

impl<'src> Lexer<'src> {
    /// Build a lexer over `source`, scanning the source once to build
    /// the internal [`SourceMap`]. Use [`Lexer::with_source_map`]
    /// instead when a `SourceMap` already exists (e.g. cached on a
    /// document buffer) to avoid re-scanning.
    #[must_use]
    pub fn new(source: &'src str) -> Self {
        Self::with_source_map(SourceMap::new(source), LexerConfig::default())
    }

    /// Build a lexer over a whole source buffer with an explicit dialect
    /// configuration.
    #[must_use]
    pub fn with_config(source: &'src str, config: LexerConfig) -> Self {
        Self::with_source_map(SourceMap::new(source), config)
    }

    /// Lex original native value bytes under the supplied grammar and modes.
    #[must_use]
    pub fn with_bytes(source: &'src [u8], config: LexerConfig) -> Self {
        Self::with_source_map(SourceMap::from_bytes(source), config)
    }

    /// Lex an immutable source image with its explicit input channel.
    #[must_use]
    pub fn with_source_image(source: &'src crate::SourceImage, config: LexerConfig) -> Self {
        Self::with_source_map(SourceMap::from_image(source), config)
    }

    /// Build a lexer with a pre-built `SourceMap` and custom config.
    ///
    /// The caller is responsible for ensuring the `SourceMap` was
    /// built from the same source string.
    #[must_use]
    pub fn with_source_map(source_map: SourceMap<'src>, config: LexerConfig) -> Self {
        // A leading byte-order mark is skipped by *starting past it*, so every
        // token keeps its true byte offset. Guarded on the flag
        // as well as the bytes: the same lexer serves the VM's `eval`, where a
        // BOM at the head of a string is ordinary data.
        let pos = if config.leading_bom == LeadingBom::Skip
            && source_map.source_bytes().starts_with(UTF8_BOM.as_bytes())
        {
            u32::try_from(UTF8_BOM.len()).unwrap_or(0)
        } else {
            0
        };
        Self {
            source_map,
            pos,
            at_command_start: true,
            in_quote: false,
            pending_sep: None,
            token_protocol: TokenProtocol::General,
            jim_missing: None,
            jim_missing_line: Some(0),
            jim_token_start: None,
            jim_preceding_token: JimPrecedingToken::Other,
            warnings: Vec::new(),
            max_brace_word_nesting: 0,
            // Start in "last kind was EOL" so an empty source produces
            // zero tokens rather than a lone ghost trailing EOL.
            last_kind: TokenType::Eol,
            done: false,
            config,
            ghosts: std::collections::BTreeMap::new(),
        }
    }

    /// Attach zero-width ghost closing delimiters (offset → delimiter
    /// byte) for error recovery; see the `ghosts` field.  Builder form,
    /// chains after [`Lexer::new`] / [`Lexer::with_source_map`].
    #[must_use]
    pub fn with_ghosts(mut self, ghosts: std::collections::BTreeMap<u32, u8>) -> Self {
        self.ghosts = ghosts;
        self
    }

    /// Treat `source` as already being the *interior* of an open
    /// double-quoted string, from byte 0: only `$`, `[`, and `\` escapes
    /// are special, and everything else — including `{` / `}`,
    /// whitespace, and `#` — is ordinary literal content, exactly like
    /// `parse_quoted`'s own in-quote dispatch. No top-level
    /// word-splitting or brace-quoting is applied at all.
    ///
    /// For scanning already-extracted word/value text (a `set` value, a
    /// proc-arg default, …) whose own enclosing quotes have already been
    /// stripped by whatever produced the text — re-tokenising it with the
    /// ordinary top-level rules would wrongly treat an embedded `{…}` run
    /// as a fresh brace-quoted (non-substituting) word, when it was
    /// actually just literal content the original quoted context already
    /// carried through unchanged. Builder form, chains after
    /// [`Lexer::new`] / [`Lexer::with_source_map`].
    ///
    /// An unterminated quote is expected here (there is no real closing
    /// delimiter to find) and never surfaces as an error: same
    /// best-effort behaviour as any other unterminated quoted string
    /// (see `parse_quoted`).
    #[must_use]
    pub fn as_quoted_body(mut self) -> Self {
        self.in_quote = true;
        self
    }

    /// The ghost delimiter byte active at `offset`, if any.
    fn ghost_at(&self, offset: u32) -> Option<u8> {
        if self.ghosts.is_empty() {
            return None;
        }
        self.ghosts.get(&offset).copied()
    }

    /// Borrow the lexer's source map without consuming the lexer.
    #[must_use]
    pub fn source_map(&self) -> &SourceMap<'src> {
        &self.source_map
    }

    /// Consume the lexer and return its source map — handy when the
    /// caller wants the same `SourceMap` to resolve the tokens later
    /// without rebuilding it.
    #[must_use]
    pub fn into_source_map(self) -> SourceMap<'src> {
        self.source_map
    }

    /// Borrow the warnings collected during lexing.
    #[must_use]
    pub fn warnings(&self) -> &[LexWarning] {
        &self.warnings
    }

    /// Consume the lexer and return its warnings.
    #[must_use]
    pub fn into_warnings(self) -> Vec<LexWarning> {
        self.warnings
    }

    /// Collect every token, including `SEP` and `EOL`, into a `Vec`.
    ///
    /// # Errors
    ///
    /// Returns [`LexError::SyntaxError`] when strict-quoting mode
    /// is active and the input triggers one of the raise-sites
    /// (`parse_var`, `parse_command`, `parse_brace`, `parse_quoted`).
    pub fn tokenise_all(self) -> Result<Vec<Token>, LexError> {
        self.collect()
    }

    /// Consume the remaining token stream while checking the maximum nesting
    /// observed by this lexer's braced-word scanner against a caller's budget.
    /// Escaped braces, comments, quoted text and braces within bare words retain
    /// their actual lexical treatment under the supplied complete configuration.
    /// This bounds source preparation; it identifies no script body, command,
    /// name, frame or evaluation. Other recursive consumers retain their own caps.
    ///
    /// # Errors
    ///
    /// Returns the first [`LexError`] encountered before the budget is exceeded.
    pub fn braced_word_nesting_within(
        mut self,
        limit: tcl_core_types::RecursionLimit,
    ) -> Result<bool, LexError> {
        // Implementation contract: naming.editor.original-source-formatting-budget
        // docs/design/analysis/name-resolution-proofs/editor-original-source-formatting-budget.md
        if limit.exceeded(self.max_brace_word_nesting) {
            return Ok(false);
        }
        while let Some(token) = self.next() {
            token?;
            if limit.exceeded(self.max_brace_word_nesting) {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Collect every token alongside the non-fatal warnings
    /// accumulated during lexing.
    ///
    /// Like `tokenise_all` but also surfaces the `(offset, message)`
    /// warnings, so callers can merge them into their own diagnostics.
    /// Without these, editors lose recoverable-syntax diagnostics
    /// (extra chars after close-brace / close-quote, unterminated
    /// strings).
    ///
    /// # Errors
    ///
    /// Returns the first [`LexError`] encountered while scanning;
    /// in that case no warnings are surfaced (the error itself
    /// subsumes any diagnostics).
    pub fn tokenise_all_with_warnings(mut self) -> Result<(Vec<Token>, Vec<LexWarning>), LexError> {
        let mut tokens = Vec::new();
        for result in self.by_ref() {
            tokens.push(result?);
        }
        Ok((tokens, self.warnings))
    }

    #[inline]
    fn source(&self) -> &'src [u8] {
        self.source_map.source_bytes()
    }

    #[inline]
    fn current_byte(&self) -> Option<u8> {
        // A ghost closing delimiter at `pos` is seen before the real
        // byte.
        if let Some(g) = self.ghost_at(self.pos) {
            return Some(g);
        }
        self.script_byte(self.pos as usize)
    }

    /// The byte the parser reads at original offset `at` under this
    /// source's channel and the dialect's `source` line-ending rule. Every
    /// structural decision reads source bytes through here, so a translated
    /// Document's lone CR ends a command or comment everywhere LF does, and
    /// no offset moves.
    #[inline]
    fn script_byte(&self, at: usize) -> Option<u8> {
        self.source_map
            .channel()
            .script_byte(self.config.source_line_endings, self.source(), at)
    }

    /// Return the character starting at `self.pos`, or `None` at EOF.
    #[inline]
    fn current_char(&self) -> Option<char> {
        if let Some(g) = self.ghost_at(self.pos) {
            return Some(char::from(g));
        }
        self.current_byte().map(char::from)
    }

    /// Emit a warning (non-strict) or return an error (strict).
    /// Called from `parse_brace`, `parse_command`, and
    /// `parse_quoted` for unterminated constructs and
    /// extra-chars-after-close violations.
    fn warn_or_error(&mut self, kind: crate::ParseIssueKind) -> Result<(), LexError> {
        if self.config.strict_quoting {
            Err(LexError::SyntaxError {
                kind,
                message: kind.message().to_owned(),
            })
        } else {
            self.warnings.push(LexWarning {
                kind,
                offset: self.pos,
                message: kind.message().to_owned(),
            });
            Ok(())
        }
    }

    /// Build a token whose span covers `start_offset..self.pos`
    /// with `content_offset = 0` (no prefix delimiter to strip).
    /// Used for `Sep`, `Eol`, `Comment`, and plain `Esc`.
    fn make_token(&self, kind: TokenType, start_offset: u32) -> Token {
        Token::new(kind, Span::new(start_offset, self.pos))
    }

    fn parse_sep(&mut self) -> Token {
        let start_offset = self.pos;
        while let Some(byte) = self.current_byte() {
            if self.token_protocol == TokenProtocol::JimNative && self.source_is_line_continuation()
            {
                self.pos += 2;
                continue;
            }
            if !(is_horizontal_whitespace_byte(byte, self.config.word_separators)
                || self.token_protocol == TokenProtocol::JimNative
                    && byte.is_ascii_whitespace()
                    && byte != b'\n')
            {
                break;
            }
            self.pos += 1; // All SEP characters are ASCII.
        }
        self.make_token(TokenType::Sep, start_offset)
    }

    /// Whether the `\n` at `self.pos` continues the current command under
    /// the F5 N-rules (measurements §2): the axis is on and the next
    /// line's first non-whitespace character is `{`. Purely lexical —
    /// independent of the command's identity, arity, or completeness
    /// (N2). A blank, whitespace-only, or comment line answers `false`
    /// (N4): the byte the horizontal-whitespace scan stops on is then a
    /// `\n`, `#`, or end-of-input rather than `{`.
    fn newline_continues_command(&self) -> bool {
        if !self.config.brace_line_continuation.continues() {
            return false;
        }
        debug_assert_eq!(self.current_byte(), Some(b'\n'));
        let mut index = self.pos as usize + 1;
        while let Some(byte) = self.script_byte(index) {
            if is_horizontal_whitespace_byte(byte, self.config.word_separators) {
                index += 1;
            } else {
                return byte == b'{';
            }
        }
        false
    }

    /// Consume the continuation newline plus the next line's leading
    /// horizontal whitespace as a single SEP token (N1): the command
    /// does not terminate, and the `{` that follows opens an ordinary
    /// braced word at a fresh word boundary.
    fn parse_brace_line_continuation_sep(&mut self) -> Token {
        let start_offset = self.pos;
        self.pos += 1; // the newline
        while let Some(byte) = self.current_byte() {
            if !is_horizontal_whitespace_byte(byte, self.config.word_separators) {
                break;
            }
            self.pos += 1;
        }
        self.make_token(TokenType::Sep, start_offset)
    }

    fn parse_eol(&mut self) -> Token {
        let start_offset = self.pos;
        // Consume a run mixing EOL characters and horizontal
        // whitespace in a single token.
        while let Some(byte) = self.current_byte() {
            if !(is_horizontal_whitespace_byte(byte, self.config.word_separators)
                || is_eol_byte(byte)
                || self.token_protocol == TokenProtocol::JimNative && byte.is_ascii_whitespace())
            {
                break;
            }
            self.pos += 1;
        }
        self.make_token(TokenType::Eol, start_offset)
    }

    /// The document-source form of one backslash escape.
    ///
    /// The shared decoder intentionally models raw Tcl string values, where a
    /// backslash before CR quotes that CR and leaves a following LF alone. A
    /// lexer consumes document text instead: Tcl's source channel translates
    /// CR, LF, and CRLF to one logical LF before parsing, so a written CRLF
    /// must be treated exactly like a written LF here.
    fn source_backslash_escape_end(&self) -> usize {
        let at = self.pos as usize;
        crate::substitution::source_backslash_continuation_end(
            self.source(),
            at,
            self.source_map.channel(),
        )
        .unwrap_or_else(|| {
            crate::substitution::backslash_escape_end_bytes_in(
                self.source(),
                at,
                self.config.escapes,
            )
        })
    }

    /// Whether this original offset starts a selected source continuation.
    fn source_is_line_continuation(&self) -> bool {
        crate::substitution::source_backslash_continuation_end(
            self.source(),
            self.pos as usize,
            self.source_map.channel(),
        )
        .is_some()
    }

    fn parse_comment(&mut self) -> Token {
        let start_offset = self.pos;
        self.pos += 1; // consume the leading '#'
        while let Some(ch) = self.current_char() {
            match ch {
                '\n' => break,
                '\0' if self.token_protocol == TokenProtocol::JimNative => break,
                '\\' => {
                    // Consume backslash + next char as a pair.
                    // `\<newline>` continues the comment to the
                    // next line (matching C Tcl behaviour).
                    let end = self.source_backslash_escape_end();
                    self.pos = u32::try_from(end).expect("source offset fits u32");
                }
                _ => {
                    self.pos += 1;
                }
            }
        }
        self.make_token(TokenType::Comment, start_offset)
    }

    fn jim_parenthesis_fragment(&mut self, start: u32) -> bool {
        if self.token_protocol != TokenProtocol::JimNative {
            return false;
        }
        let special = self.current_byte() == Some(b'(')
            && self.peek_byte(1).is_none_or(|byte| byte == b'$')
            || self.current_byte() == Some(b')')
                && self.jim_preceding_token == JimPrecedingToken::ScalarVariable;
        if special && self.pos == start {
            self.pos += 1;
        }
        special
    }

    fn parse_esc(&mut self) -> Token {
        let start_offset = self.pos;
        while let Some(ch) = self.current_char() {
            if self.jim_parenthesis_fragment(start_offset) {
                break;
            }
            if is_horizontal_whitespace(ch, self.config.word_separators) || is_eol_char(ch) {
                break;
            }
            if ch == '$' || ch == '[' {
                break;
            }
            if ch == '\\' {
                if self.source_is_line_continuation() {
                    // `\<newline>` line continuation (bare-word
                    // context). At word start → emit the
                    // continuation as a SEP. Mid-word → stop;
                    // the iterator re-enters at the backslash.
                    if self.pos == start_offset {
                        return self.parse_backslash_newline_sep();
                    }
                    break;
                }
                if self.source().get((self.pos + 1) as usize).is_some() {
                    // Consume the original backslash and escaped character.
                    self.pos += 1;
                    if self.current_char().is_some() {
                        self.pos += 1;
                    }
                } else {
                    // Trailing backslash at EOF: literal.
                    if self.token_protocol == TokenProtocol::JimNative {
                        self.jim_missing = Some(b'\\');
                    }
                    self.pos += 1;
                }
                continue;
            }
            self.pos += 1;
        }
        self.make_token(TokenType::Esc, start_offset)
    }

    /// Consume a `\<newline>` line-continuation sequence at word
    /// start as a SEP token. A backslash-newline at the very start of
    /// a token emits `TokenType::Sep` so the next token can recognise
    /// `{` or `"` as brace/quote delimiters (a fresh word boundary).
    fn parse_backslash_newline_sep(&mut self) -> Token {
        let start = self.pos;
        self.pos =
            u32::try_from(self.source_backslash_escape_end()).expect("source offset fits u32");
        self.make_token(TokenType::Sep, start)
    }

    /// Return the parser's byte at `self.pos + offset`, if any.
    #[inline]
    fn peek_byte(&self, offset: u32) -> Option<u8> {
        self.script_byte((self.pos + offset) as usize)
    }
}

impl Iterator for Lexer<'_> {
    type Item = Result<Token, LexError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        // Drain any pending ghost SEP (iRules `}{` boundary).
        if let Some(sep) = self.pending_sep.take() {
            self.last_kind = sep.kind;
            return Some(Ok(sep));
        }

        // EOF: emit a trailing ghost EOL (once) then stop.
        if self.pos as usize >= self.source().len() {
            if self.token_protocol == TokenProtocol::JimNative && self.in_quote {
                self.jim_missing = Some(b'"');
            }
            if self.last_kind == TokenType::Eol {
                self.done = true;
                return None;
            }
            self.last_kind = TokenType::Eol;
            return Some(Ok(Token::new(TokenType::Eol, Span::empty(self.pos))));
        }

        let ch = self
            .current_char()
            .expect("source[pos..] is non-empty when pos < len");

        // Dispatch splits on `in_quote`. Inside a quoted string
        // the scanner ignores separators, EOL characters, `#`, and
        // `{` / `}` — they become literal content — and only `$`,
        // `[`, `"`, and `\` are meaningful. Outside a quoted
        // string the dispatch is the full top-level one.
        let result = if self.in_quote {
            // Inside a quoted string: `$`, `[` dispatch to their
            // own parsers; `\` is still a deferred error; every
            // other character (including the closing `"`) is
            // handled by `parse_quoted(false)`, which scans
            // content and consumes the closing `"` when it
            // encounters one.
            match ch {
                '$' => self.parse_var(),
                '[' => self.parse_command(),
                _ => self.parse_quoted(false),
            }
        } else {
            match ch {
                _ if is_horizontal_whitespace(ch, self.config.word_separators) => {
                    Ok(self.parse_sep())
                }
                // F5 N1 (measurements §2): a newline whose next line
                // starts (after horizontal whitespace) with `{` is a word
                // separator, not a command terminator. A `;` always
                // terminates, and a blank/comment line falls through to
                // the ordinary EOL run below (N4).
                '\n' if self.newline_continues_command() => {
                    Ok(self.parse_brace_line_continuation_sep())
                }
                _ if is_eol_char(ch) => Ok(self.parse_eol()),
                '#' if self.at_command_start => Ok(self.parse_comment()),
                '$' => self.parse_var(),
                '[' => self.parse_command(),
                '{' if self.is_newword() => self.parse_brace_or_expand(),
                '"' if self.is_newword() => self.parse_quoted(true),
                _ => Ok(self.parse_esc()),
            }
        };

        match result {
            Ok(mut tok) => {
                match tok.kind {
                    TokenType::Eol => self.at_command_start = true,
                    TokenType::Sep | TokenType::Comment => {
                        // Preserve current value.
                    }
                    _ => self.at_command_start = false,
                }
                if self.token_protocol == TokenProtocol::JimNative && tok.kind != TokenType::Comment
                {
                    self.jim_token_start = Some(tok.span.start());
                    self.jim_preceding_token = if tok.kind == TokenType::Var
                        && (tok.content_offset == 2
                            || !self.source_map.token_bytes(tok).contains(&b'('))
                    {
                        JimPrecedingToken::ScalarVariable
                    } else {
                        JimPrecedingToken::Other
                    };
                }
                self.last_kind = tok.kind;
                // Tag the token with the current `in_quote` state.
                // `parse_quoted` resets `self.in_quote = false`
                // *before* returning when it consumes the closing
                // `"`, so the last ESC of a quoted run picks up
                // `false` here.
                tok.in_quote = self.in_quote;
                Some(Ok(tok))
            }
            Err(err) => {
                // Fuse the iterator on fatal error.
                self.done = true;
                Some(Err(err))
            }
        }
    }
}

/// Whether `ch` is horizontal whitespace under `seps`.
///
/// `JimTcl` omits `\v` from the set its script parser switches on
/// (`jim.c:1338-1341`); every build of the Tcl core includes it.
#[inline]
fn is_horizontal_whitespace(ch: char, seps: tcl_dialect::WordSeparators) -> bool {
    u8::try_from(u32::from(ch)).is_ok_and(|b| seps.is_separator(b))
}

/// Byte twin of [`is_horizontal_whitespace`].
#[inline]
fn is_horizontal_whitespace_byte(byte: u8, seps: tcl_dialect::WordSeparators) -> bool {
    seps.is_separator(byte)
}

#[inline]
fn is_eol_char(ch: char) -> bool {
    ch == '\n' || ch == ';'
}

#[inline]
fn is_eol_byte(byte: u8) -> bool {
    byte == b'\n' || byte == b';'
}

/// Union of `is_horizontal_whitespace_byte` and `is_eol_byte`.
/// Used by the `{*}` expansion-prefix guard to test the byte after `}`.
#[inline]
fn is_separator_byte(byte: u8, seps: tcl_dialect::WordSeparators) -> bool {
    is_horizontal_whitespace_byte(byte, seps) || is_eol_byte(byte)
}

// There is no "deferred special" character set: every valid ASCII
// character in a Tcl source is handled by the lexer.

#[cfg(test)]
mod tests;

/// The `JimTcl` lexical divergences, each asserted against both grammars.
///
/// Every expectation was measured on interpreters built from the upstream
/// tags 0.76-0.84 against tclsh 8.6 / 9.0.
#[cfg(test)]
mod jim_divergence_tests;

/// `LexerConfig::from_grammar` must carry every axis the lexer reads. The
/// `..Self::default()` tail is where an axis silently goes missing — it is
/// how `list_parse` was declared on the grammar and consumed by nothing —
/// so this pins each carried field against a grammar in which every axis
/// differs from the default.
#[cfg(test)]
mod grammar_carriage_tests;
