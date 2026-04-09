//! Streaming Tcl lexer.
//!
//! **L3 + L4 + L5 + L6.** The first four lexer chunks, taken
//! together, handle every non-quoted top-level Tcl construct:
//!
//! - **EOF handling** — emits a trailing ghost `EOL` (once) when the
//!   source does not already end with an EOL token, matching the
//!   Python lexer's `get_token()` / `tokenise_all()` contract.
//! - **SEP** — runs of horizontal whitespace (`' '`, `\t`, `\r`, VT, FF).
//!   `\r` is horizontal whitespace in Tcl, not an EOL.
//! - **EOL** — runs of EOL characters (`\n`, `;`) interleaved with
//!   horizontal whitespace, mirroring Python's `_parse_eol`.
//! - **COMMENT** — `#` at command start, scanned to the next `\n`
//!   (exclusive). Backslash-newline continuation inside comments is
//!   not handled yet; an input whose comment contains a `\` is
//!   reported as [`LexError::UnsupportedCharacter`] so the
//!   differential harness can filter it.
//! - **ESC** — runs of characters that are neither whitespace nor EOL
//!   nor one of the "deferred" special characters. Terminated by
//!   `$` (L4) or `[` (L5) so that variable and command substitutions
//!   dispatch on the next iteration.
//! - **VAR (L4)** — variable substitution in all four Tcl forms:
//!   `$name`, `$ns::var` (namespace-separated), `${name}` (braced),
//!   and `$arr(idx)` (array index with nested parens and embedded
//!   `${...}` support). A bare `$` with no name following is emitted
//!   as an `STR` token whose span covers just the `$`, matching
//!   Python's `_parse_var` fallback. Unterminated `${` and `$arr(`
//!   tokenize best-effort (L9 will add the warning-collection
//!   machinery to report them as diagnostics).
//! - **CMD (L5)** — command substitution `[…]` with Python-parity
//!   nesting rules. The scanner tracks three pieces of state while
//!   inside the command body: outer bracket nesting (`level`),
//!   brace nesting (`blevel`), and whether we are inside a `"…"`
//!   quoted sub-region (`in_quotes`). A `[` increments `level` only
//!   when not inside braces or quotes; a `]` closes the command only
//!   when the outer bracket is the innermost nesting. Backslash
//!   escapes consume two characters (CRLF counted as one line
//!   advance); `${…}` sub-scans exist to stop a `)` or `}` inside a
//!   braced variable name from fooling the counter. Unterminated
//!   `[` tokenizes best-effort (L9 adds the warning).
//! - **STR (L6)** — braced strings `{…}`. Emitted when a `{` appears
//!   at a word boundary (the previous token was `EOL` / `SEP` / `STR`
//!   / `EXPAND`, matching Python's `newword` predicate). The body
//!   is scanned with balanced `{` / `}` counting; backslash
//!   sequences consume two characters as a pair (preserving Python's
//!   "backslash is inert inside braces" semantics — the backslash
//!   and the following character are retained literally in the
//!   token text). A `{` that is NOT at a word boundary is a regular
//!   word character in the enclosing `ESC` token, matching Python's
//!   `_parse_string` fall-through for mid-word braces. Unterminated
//!   `{` tokenizes best-effort (L9 adds the warning). The `}{ ` iRules
//!   word-boundary injection and the "extra characters after
//!   close-brace" warning are deferred to L8 (dialect flags) and
//!   L9 (warning collection) respectively.
//!
//! The "deferred" set is now `" \`. Encountering either of them
//! trips [`LexError::UnsupportedCharacter`] carrying the character
//! and its position. Callers filter inputs on that error rather than
//! receiving silently-wrong token streams. Chunks L7–L9 shrink the
//! set to zero.
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
//! This matches the broader "source map threaded throughout" rewrite
//! design: every positional entity (Tokens now, future IR and CFG
//! nodes later) carries only a span, and a single `SourceMap` per
//! document is the canonical place that resolves spans to text and
//! positions. See `docs/rust-rewrite.md` for the principle and the
//! `tower-lsp` / `ropey` plan that follows it upstream.
//!
//! ### Offsets and columns
//!
//! [`SourcePosition::offset`] is a byte offset. The `character` field
//! is **byte offset within the line** — the same thing the Python
//! lexer produces when it does `col = offset - line_start`. ASCII
//! parity is exact; non-ASCII drifts from the LSP UTF-16 contract.
//! Multi-byte column parity is tracked as deferred work in
//! `docs/rust-rewrite.md` and must be fixed in lock-step across both
//! implementations.
//!
//! ### Not yet implemented
//!
//! Explicit list of deferrals so reviewers can tell what lives where:
//!
//! - ~~L4: variable substitution (`$name`, `${name}`, `$arr(idx)`,~~
//!   ~~`$ns::var`)~~ — landed
//! - ~~L5: command substitution (`[…]`, possibly nested)~~ — landed
//! - ~~L6: braced strings (`{…}`, possibly nested)~~ —
//!   **landed in this chunk**
//! - L7: quoted strings (`"…"`)
//! - L8: expansion prefix (`{*}`), `strict_quoting`, `expand_syntax`,
//!   `irules_brace_separator`
//! - L9: backslash escapes and line continuation; warning collection
//!   (which will turn L4's best-effort recovery of unterminated
//!   `${` / `$arr(` into proper diagnostics); ghost character
//!   insertion for error recovery. "Ghost" is our term of art
//!   (chosen over "synthetic" / "virtual" to avoid collisions with
//!   Rust vocabulary — `virtual` is a reserved keyword) for tokens
//!   and characters that exist in the token stream without
//!   corresponding bytes in the source buffer.
//! - Later: sub-lexing support for nested constructs; UTF-16 column
//!   parity; `LineIndex::from_rope_slice` adapter
//!
//! [`Span`]: crate::Span
//! [`SourceMap`]: crate::SourceMap
//! [`SourcePosition`]: crate::SourcePosition
//! [`SourcePosition::offset`]: crate::SourcePosition#structfield.offset

use thiserror::Error;

use crate::source_map::SourceMap;
use crate::span::Span;
use crate::tokens::{SourcePosition, Token, TokenType};

/// Configuration for the Tcl lexer.
///
/// Empty in L3. Every dialect flag on Python's `TclLexer`
/// (`strict_quoting`, `expand_syntax`, `irules_brace_separator`) gates
/// behaviour the Rust lexer does not yet implement. They become fields
/// on this struct in chunk L8.
#[derive(Debug, Clone, Copy, Default)]
pub struct LexerConfig {}

/// Errors produced by the Tcl lexer.
///
/// L3 has exactly one variant. Future chunks add real error variants
/// (unterminated brace, extra close-quote, etc.) mirroring Python's
/// `TclParseError`. The `UnsupportedCharacter` variant shrinks and
/// eventually disappears.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum LexError {
    /// The lexer reached a character whose handler has not been
    /// ported from Python yet. The character and its position are
    /// preserved so callers — in particular the differential test
    /// harness — can filter inputs to the currently supported subset.
    #[error(
        "unsupported character {ch:?} at line {} character {} offset {} — \
         the Rust lexer has not yet implemented this construct",
        position.line, position.character, position.offset
    )]
    UnsupportedCharacter {
        /// The offending character.
        ch: char,
        /// Its position in the source.
        position: SourcePosition,
    },
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
    /// Kind of the most recently emitted token. Used to decide whether
    /// EOF needs a trailing ghost EOL.
    last_kind: TokenType,
    /// Once true, [`Iterator::next`] returns `None`.
    done: bool,
    _config: LexerConfig,
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

    /// Build a lexer with a pre-built `SourceMap` and custom config.
    ///
    /// The caller is responsible for ensuring the `SourceMap` was
    /// built from the same source string.
    #[must_use]
    pub fn with_source_map(source_map: SourceMap<'src>, config: LexerConfig) -> Self {
        Self {
            source_map,
            pos: 0,
            at_command_start: true,
            // Start in "last kind was EOL" so an empty source produces
            // zero tokens rather than a lone ghost trailing EOL,
            // matching `TclLexer.__init__` in Python.
            last_kind: TokenType::Eol,
            done: false,
            _config: config,
        }
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

    /// Collect every token, including `SEP` and `EOL`, into a `Vec`.
    ///
    /// Matches `TclLexer.tokenise_all` on the Python side.
    ///
    /// # Errors
    ///
    /// Returns [`LexError::UnsupportedCharacter`] on the first
    /// character whose handler has not been ported.
    pub fn tokenise_all(self) -> Result<Vec<Token>, LexError> {
        self.collect()
    }

    #[inline]
    fn source(&self) -> &'src str {
        self.source_map.source()
    }

    #[inline]
    fn current_byte(&self) -> Option<u8> {
        self.source().as_bytes().get(self.pos as usize).copied()
    }

    /// Return the character starting at `self.pos`, or `None` at EOF.
    #[inline]
    fn current_char(&self) -> Option<char> {
        self.source()[self.pos as usize..].chars().next()
    }

    fn position_at(&self, offset: u32) -> SourcePosition {
        self.source_map.position_at(offset)
    }

    /// Build a token whose span covers `start_offset..self.pos`.
    fn make_token(&self, kind: TokenType, start_offset: u32) -> Token {
        Token {
            kind,
            span: Span::new(start_offset, self.pos),
            in_quote: false,
        }
    }

    fn parse_sep(&mut self) -> Token {
        let start_offset = self.pos;
        while let Some(byte) = self.current_byte() {
            if !is_horizontal_whitespace_byte(byte) {
                break;
            }
            self.pos += 1; // All SEP characters are ASCII.
        }
        self.make_token(TokenType::Sep, start_offset)
    }

    fn parse_eol(&mut self) -> Token {
        let start_offset = self.pos;
        // Python's `_parse_eol` consumes a run mixing EOL characters
        // and horizontal whitespace in a single token.
        while let Some(byte) = self.current_byte() {
            if !is_horizontal_whitespace_byte(byte) && !is_eol_byte(byte) {
                break;
            }
            self.pos += 1;
        }
        self.make_token(TokenType::Eol, start_offset)
    }

    fn parse_comment(&mut self) -> Result<Token, LexError> {
        let start_offset = self.pos;
        self.pos += 1; // consume the leading '#'
        while let Some(ch) = self.current_char() {
            match ch {
                '\n' => break,
                '\\' => {
                    return Err(LexError::UnsupportedCharacter {
                        ch,
                        position: self.position_at(self.pos),
                    });
                }
                _ => {
                    self.pos += u32::try_from(ch.len_utf8()).expect("char len fits u32");
                }
            }
        }
        Ok(self.make_token(TokenType::Comment, start_offset))
    }

    fn parse_esc(&mut self) -> Result<Token, LexError> {
        let start_offset = self.pos;
        while let Some(ch) = self.current_char() {
            if is_horizontal_whitespace(ch) || is_eol_char(ch) {
                break;
            }
            // `$` and `[` terminate a bare word so the next iteration
            // dispatches to `parse_var` / `parse_command`. Matches
            // Python `_parse_string`'s `elif ch == "$" or ch == "["`
            // branch.
            if ch == '$' || ch == '[' {
                break;
            }
            if is_deferred_special(ch) {
                if self.pos == start_offset {
                    // Did not consume anything yet; surface the error
                    // rather than emit an empty ESC token. In practice
                    // the top-level dispatch rejects deferred chars
                    // before calling `parse_esc`; this branch is
                    // defensive.
                    return Err(LexError::UnsupportedCharacter {
                        ch,
                        position: self.position_at(self.pos),
                    });
                }
                break;
            }
            self.pos += u32::try_from(ch.len_utf8()).expect("char len fits u32");
        }
        Ok(self.make_token(TokenType::Esc, start_offset))
    }

    /// Parse a variable substitution starting at the current `$`.
    ///
    /// Handles all four Tcl forms:
    ///
    /// - `$name`, `$ns::var` — identifier scan accepting Unicode
    ///   alphanumerics, underscores, and `::` namespace separators.
    /// - `${name}` — braced scan; the closing `}` is consumed but
    ///   NOT included in the token span (matching Python's `_end =
    ///   self.pos - 1` before the `}` advance).
    /// - `$arr(idx)` — array indexing with balanced `(`/`)` and
    ///   embedded `${…}` support, matching the Python lexer's
    ///   `_parse_var` behaviour. The `)` IS included in the span.
    /// - bare `$` — emitted as an `STR` token whose span covers
    ///   just the `$`, matching `_parse_var`'s fallback.
    ///
    /// **Span convention.** The span always starts at the `$`
    /// position so the resolved start/end `SourcePosition`s include
    /// the dollar sign, matching the Python lexer's `Token.start`
    /// behaviour. The "human-readable" content (variable name
    /// without the leading `$` or `${`) is accessed via
    /// [`SourceMap::token_text`] rather than `SourceMap::text(span)`;
    /// the bridge layer uses `token_text` so Python callers see the
    /// same `tok.text` they always have.
    ///
    /// Never fails. Unterminated `${` and `$arr(` tokenize
    /// best-effort; the Python lexer emits non-fatal warnings for
    /// those cases, which the Rust lexer will start reproducing in
    /// L9 when warning collection lands.
    fn parse_var(&mut self) -> Token {
        let dollar_pos = self.pos;
        self.pos += 1; // skip '$'

        // `${name}` braced form.
        if self.current_byte() == Some(b'{') {
            self.pos += 1; // skip '{'
            let content_start = self.pos;
            while let Some(ch) = self.current_char() {
                if ch == '}' {
                    break;
                }
                self.pos += u32::try_from(ch.len_utf8()).expect("char len fits u32");
            }
            // Compute the token's exclusive-end offset. Python's
            // `_parse_var` uses the clamp
            // `end_offset = _end if _end >= _start else _start`,
            // which produces one of three cases:
            //
            // - `${name}`: end is the last char of `name` —
            //   `span_end == self.pos` (before `}` is consumed).
            // - `${}`: `_end < _start` (no content scanned), so the
            //   clamp pins end to `_start`, which is the `}`
            //   position itself. We match by extending
            //   `span_end` to include the `}`.
            // - `${` (unterminated, empty): no content, no `}`. We
            //   stop at the current `self.pos`; this is a minor
            //   parity drift against Python (which emits a past-end
            //   position there) and does not affect any input in
            //   the differential corpus.
            let content_empty = self.pos == content_start;
            let has_close_brace = self.current_byte() == Some(b'}');
            let span_end = if content_empty && has_close_brace {
                self.pos + 1 // include the `}`
            } else {
                self.pos
            };
            if has_close_brace {
                self.pos += 1;
            }
            return Token::new(TokenType::Var, Span::new(dollar_pos, span_end));
        }

        // `$name` or `$ns::var` identifier form.
        let name_start = self.pos;
        while let Some(ch) = self.current_char() {
            if ch.is_alphanumeric() || ch == '_' {
                self.pos += u32::try_from(ch.len_utf8()).expect("char len fits u32");
                continue;
            }
            if ch == ':' && self.peek_byte(1) == Some(b':') {
                self.pos += 2;
                continue;
            }
            break;
        }

        // `$arr(idx)` array-index form. The `(` only counts as an
        // array index when it immediately follows an identifier,
        // matching the Python dispatcher's `if self.remaining and
        // self._cur() == "("` branch inside `_parse_var`.
        if self.current_byte() == Some(b'(') {
            self.scan_array_index_body();
            return Token::new(TokenType::Var, Span::new(dollar_pos, self.pos));
        }

        // Bare `$` — no identifier chars were consumed. Python emits
        // this as an `STR` token whose text is just `$`, not a `VAR`.
        if self.pos == name_start {
            return Token::new(TokenType::Str, Span::new(dollar_pos, dollar_pos + 1));
        }

        Token::new(TokenType::Var, Span::new(dollar_pos, self.pos))
    }

    /// Consume a `(…)` array-index body starting at the `(`,
    /// including balanced nested `(` / `)` and any embedded
    /// `${…}`. Advances `self.pos` past the closing `)` (or to
    /// EOF for unterminated input).
    fn scan_array_index_body(&mut self) {
        debug_assert_eq!(self.current_byte(), Some(b'('));
        self.pos += 1; // skip '('
        let mut depth: u32 = 1;
        while depth > 0 {
            let Some(ch) = self.current_char() else {
                // Unterminated — leave `self.pos` at EOF. L9 adds
                // the warning.
                return;
            };
            match ch {
                '(' => {
                    depth += 1;
                    self.pos += 1;
                }
                ')' => {
                    depth -= 1;
                    self.pos += 1;
                }
                '$' if self.peek_byte(1) == Some(b'{') => {
                    // `${…}` inside an array index — scan to the
                    // matching `}`. Python does this to avoid
                    // mis-counting any `(` or `)` characters inside
                    // the braced name.
                    self.pos += 2; // skip '${'
                    while let Some(inner) = self.current_char() {
                        if inner == '}' {
                            self.pos += 1;
                            break;
                        }
                        self.pos += u32::try_from(inner.len_utf8()).expect("char len fits u32");
                    }
                }
                _ => {
                    self.pos += u32::try_from(ch.len_utf8()).expect("char len fits u32");
                }
            }
        }
    }

    /// Return the byte at `self.pos + offset`, if any.
    #[inline]
    fn peek_byte(&self, offset: u32) -> Option<u8> {
        self.source()
            .as_bytes()
            .get((self.pos + offset) as usize)
            .copied()
    }

    /// Mirror of Python's `_parse_string` `newword` predicate: the
    /// next token is at a word boundary (so `{` at `self.pos`
    /// starts a braced string rather than being part of a bare
    /// word) when the previously emitted token was `Sep`, `Eol`,
    /// `Str`, or `Expand`. The initial state before any token is
    /// emitted is `Eol`, so the first character of the source is
    /// always at a word boundary.
    #[inline]
    fn is_newword(&self) -> bool {
        matches!(
            self.last_kind,
            TokenType::Sep | TokenType::Eol | TokenType::Str | TokenType::Expand
        )
    }

    /// Parse a braced string starting at the current `{`. The caller
    /// is responsible for checking `is_newword()` before dispatching
    /// here — a `{` in the middle of a bare word is a regular
    /// character in `parse_esc`, not a braced-string opener.
    ///
    /// The scanner counts balanced `{` / `}` pairs (`level` starts
    /// at 1 after skipping the opening `{`). Backslash sequences
    /// consume two characters as a pair (CRLF counted as one), but
    /// the backslash and the following character are preserved
    /// literally in the token text — braces are "verbatim" in Tcl,
    /// so `\}` inside a brace body does NOT count as a close brace
    /// even though the scanner skips over it.
    ///
    /// The span starts at the `{` and normally ends at the last
    /// character of the body (NOT the closing `}`), matching
    /// Python's `_end = self.pos - 1` before the `}` advance. The
    /// empty-body degenerate `{}` extends the span by one so
    /// `range_positions` reports the `}` as the end position,
    /// matching Python's `end_offset = _start` clamp.
    /// `SourceMap::token_text` strips the leading `{` (and the
    /// trailing `}` for the degenerate `{}` case) so Python callers
    /// see just the inside of the braces.
    ///
    /// Never fails. Unterminated `{` tokenizes best-effort; the
    /// "extra characters after close-brace" warning and the
    /// iRules `}{` word-boundary injection are deferred to L8/L9.
    fn parse_brace(&mut self) -> Token {
        let brace_pos = self.pos;
        self.pos += 1; // skip opening '{'
        let content_start = self.pos;

        let mut level: u32 = 1;
        let mut span_end: u32 = self.pos;

        while let Some(ch) = self.current_char() {
            match ch {
                '\\' => {
                    // Consume the backslash and the next character
                    // as a pair. Matches Python's `_parse_brace`
                    // which skips the backslash then the escaped
                    // char, with CRLF counted as one character.
                    self.pos += 1;
                    match self.current_char() {
                        Some(esc @ ('\n' | '\r')) => {
                            self.pos += 1;
                            if esc == '\r' && self.current_byte() == Some(b'\n') {
                                self.pos += 1;
                            }
                        }
                        Some(esc) => {
                            self.pos += u32::try_from(esc.len_utf8()).expect("char len fits u32");
                        }
                        None => {
                            // Trailing backslash at EOF — leave as is.
                        }
                    }
                }
                '{' => {
                    level += 1;
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
                    self.pos += u32::try_from(ch.len_utf8()).expect("char len fits u32");
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
        }

        Token::new(TokenType::Str, Span::new(brace_pos, final_span_end))
    }

    /// Parse a command substitution starting at the current `[`.
    ///
    /// Scans the command body until a matching `]`, tracking outer
    /// bracket nesting (`level`), brace nesting (`blevel`), and
    /// whether we are inside a `"…"` sub-region (`in_quotes`). Each
    /// piece of state gates which characters are meaningful:
    ///
    /// - `"` — toggles `in_quotes` when `blevel == 0`.
    /// - `[` — increments `level` when not braced and not quoted.
    /// - `]` — decrements `level` when not braced and not quoted;
    ///   closes the command when `level` reaches zero.
    /// - `\\` — consumes the next character unconditionally (CRLF
    ///   counted as one pair). This makes `\]` and `\"` inside a
    ///   command body inert.
    /// - `$` followed by `{` (outside braces and quotes) — sub-scans
    ///   a `${…}` construct so a `}` or `)` inside a braced
    ///   variable name does not fool the counter.
    /// - `{` / `}` — adjust `blevel` when not quoted.
    ///
    /// The span always starts at the `[` and normally ends at the
    /// last character of the body (NOT the closing `]`), matching
    /// Python's `_end = self.pos - 1`. The empty-command degenerate
    /// case `[]` extends the span by one so `range_positions`
    /// reports the `]` as the end position, matching Python's
    /// `end_offset = _start` clamp. `SourceMap::token_text` strips
    /// the leading `[` (and the trailing `]` from the degenerate
    /// case) so Python callers see just the command body.
    ///
    /// Never fails. An unterminated `[` tokenizes best-effort; the
    /// Python lexer emits a `missing close-bracket` warning, which
    /// L9's warning-collection infrastructure will start
    /// reproducing.
    fn parse_command(&mut self) -> Token {
        let bracket_pos = self.pos;
        self.pos += 1; // skip '['
        let content_start = self.pos;

        let mut level: u32 = 1;
        let mut blevel: u32 = 0;
        let mut in_quotes = false;

        while let Some(ch) = self.current_char() {
            match ch {
                '"' if blevel == 0 => {
                    in_quotes = !in_quotes;
                    self.pos += 1;
                }
                '[' if blevel == 0 && !in_quotes => {
                    level += 1;
                    self.pos += 1;
                }
                ']' if blevel == 0 && !in_quotes => {
                    level -= 1;
                    if level == 0 {
                        // Leave `self.pos` at the closing `]`; the
                        // code after the loop decides whether to
                        // include it in the span and consume it.
                        break;
                    }
                    self.pos += 1;
                }
                '\\' => {
                    // Consume the backslash and the next character
                    // as a pair (CRLF counted as one). This matches
                    // Python's "Skip backslash. Skip escaped char."
                    // logic inside `_parse_command`.
                    self.pos += 1;
                    match self.current_char() {
                        Some(esc @ ('\n' | '\r')) => {
                            self.pos += 1;
                            if esc == '\r' && self.current_byte() == Some(b'\n') {
                                self.pos += 1;
                            }
                        }
                        Some(esc) => {
                            self.pos += u32::try_from(esc.len_utf8()).expect("char len fits u32");
                        }
                        None => {
                            // Trailing backslash at EOF — leave as is.
                        }
                    }
                }
                '$' if !in_quotes && blevel == 0 => {
                    // `${…}` inside a command body: sub-scan to the
                    // matching `}` so any `)` or `]` inside a braced
                    // variable name does not fool the outer counter.
                    if self.peek_byte(1) == Some(b'{') {
                        self.pos += 2; // skip '${'
                        while let Some(inner) = self.current_char() {
                            if inner == '}' {
                                self.pos += 1;
                                break;
                            }
                            self.pos += u32::try_from(inner.len_utf8()).expect("char len fits u32");
                        }
                    } else {
                        self.pos += 1;
                    }
                }
                '{' if !in_quotes => {
                    blevel += 1;
                    self.pos += 1;
                }
                '}' if !in_quotes => {
                    blevel = blevel.saturating_sub(1);
                    self.pos += 1;
                }
                _ => {
                    self.pos += u32::try_from(ch.len_utf8()).expect("char len fits u32");
                }
            }
        }

        // At this point `self.pos` is at the closing `]` or at EOF.
        let content_empty = self.pos == content_start;
        let has_close_bracket = self.current_byte() == Some(b']');
        let span_end = if content_empty && has_close_bracket {
            self.pos + 1 // include the `]` so the end position lands on it
        } else {
            self.pos
        };
        if has_close_bracket {
            self.pos += 1;
        }

        Token::new(TokenType::Cmd, Span::new(bracket_pos, span_end))
    }
}

impl Iterator for Lexer<'_> {
    type Item = Result<Token, LexError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        // EOF: emit a trailing ghost EOL (once) then stop.
        if self.pos as usize >= self.source().len() {
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

        let result = match ch {
            _ if is_horizontal_whitespace(ch) => Ok(self.parse_sep()),
            _ if is_eol_char(ch) => Ok(self.parse_eol()),
            '#' if self.at_command_start => self.parse_comment(),
            '$' => Ok(self.parse_var()),
            '[' => Ok(self.parse_command()),
            '{' if self.is_newword() => Ok(self.parse_brace()),
            _ if is_deferred_special(ch) => Err(LexError::UnsupportedCharacter {
                ch,
                position: self.position_at(self.pos),
            }),
            _ => self.parse_esc(),
        };

        match result {
            Ok(tok) => {
                match tok.kind {
                    TokenType::Eol => self.at_command_start = true,
                    TokenType::Sep | TokenType::Comment => {
                        // Preserve current value.
                    }
                    _ => self.at_command_start = false,
                }
                self.last_kind = tok.kind;
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

#[inline]
fn is_horizontal_whitespace(ch: char) -> bool {
    matches!(ch, ' ' | '\t' | '\r' | '\u{0B}' | '\u{0C}')
}

#[inline]
fn is_horizontal_whitespace_byte(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\r' | 0x0B | 0x0C)
}

#[inline]
fn is_eol_char(ch: char) -> bool {
    ch == '\n' || ch == ';'
}

#[inline]
fn is_eol_byte(byte: u8) -> bool {
    byte == b'\n' || byte == b';'
}

/// Characters whose handling the Rust lexer has not yet implemented.
/// Triggers [`LexError::UnsupportedCharacter`]. Chunks L7–L9
/// incrementally drain this set. L4 removed `$`; L5 removed `[`
/// (now dispatches to `parse_command`) and `]` (now a regular
/// word character outside a command body); L6 removed `{` (now
/// dispatches to `parse_brace` when at a word boundary, otherwise
/// a regular word character) and `}` (now a regular word
/// character outside a brace body).
#[inline]
fn is_deferred_special(ch: char) -> bool {
    matches!(ch, '"' | '\\')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::line_index::LineIndex;

    struct Lexed<'src> {
        source_map: SourceMap<'src>,
        tokens: Vec<Token>,
    }

    impl<'src> Lexed<'src> {
        fn run(source: &'src str) -> Self {
            let lexer = Lexer::new(source);
            let source_map = lexer.source_map().clone();
            let tokens = lexer.tokenise_all().expect("L3 lexer accepts fixture");
            Self { source_map, tokens }
        }

        fn kinds(&self) -> Vec<TokenType> {
            self.tokens.iter().map(|t| t.kind).collect()
        }

        fn texts(&self) -> Vec<&'src str> {
            self.tokens
                .iter()
                .map(|t| self.source_map.text(t.span))
                .collect()
        }

        fn positions(&self, idx: usize) -> (SourcePosition, SourcePosition) {
            self.source_map.range_positions(self.tokens[idx].span)
        }
    }

    #[test]
    fn empty_source_produces_no_tokens() {
        let lexed = Lexed::run("");
        assert!(lexed.tokens.is_empty());
    }

    #[test]
    fn single_word_emits_esc_and_trailing_eol() {
        let lexed = Lexed::run("foo");
        assert_eq!(lexed.tokens.len(), 2);
        assert_eq!(lexed.tokens[0].kind, TokenType::Esc);
        assert_eq!(lexed.source_map.text(lexed.tokens[0].span), "foo");
        assert_eq!(lexed.tokens[1].kind, TokenType::Eol);
        assert_eq!(lexed.source_map.text(lexed.tokens[1].span), "");
    }

    #[test]
    fn two_words_separated_by_space() {
        let lexed = Lexed::run("foo bar");
        assert_eq!(
            lexed.kinds(),
            vec![
                TokenType::Esc,
                TokenType::Sep,
                TokenType::Esc,
                TokenType::Eol,
            ]
        );
        assert_eq!(lexed.texts(), vec!["foo", " ", "bar", ""]);
    }

    #[test]
    fn multiple_spaces_collapse_into_one_sep_token() {
        let lexed = Lexed::run("foo   bar");
        assert_eq!(lexed.texts(), vec!["foo", "   ", "bar", ""]);
    }

    #[test]
    fn tab_separator() {
        let lexed = Lexed::run("foo\tbar");
        assert_eq!(lexed.texts(), vec!["foo", "\t", "bar", ""]);
    }

    #[test]
    fn cr_is_separator_not_eol() {
        // Python: `\r` is in `_SEP_CHARS`, not `_EOL_CHARS`.
        let lexed = Lexed::run("foo\rbar");
        assert_eq!(
            lexed.kinds(),
            vec![
                TokenType::Esc,
                TokenType::Sep,
                TokenType::Esc,
                TokenType::Eol,
            ]
        );
    }

    #[test]
    fn lf_is_eol() {
        let lexed = Lexed::run("foo\nbar");
        assert_eq!(
            lexed.kinds(),
            vec![
                TokenType::Esc,
                TokenType::Eol,
                TokenType::Esc,
                TokenType::Eol,
            ]
        );
    }

    #[test]
    fn semicolon_is_eol() {
        let lexed = Lexed::run("foo;bar");
        assert_eq!(
            lexed.kinds(),
            vec![
                TokenType::Esc,
                TokenType::Eol,
                TokenType::Esc,
                TokenType::Eol,
            ]
        );
    }

    #[test]
    fn mixed_eol_and_whitespace_becomes_single_eol_token() {
        let lexed = Lexed::run("foo\n \t;\nbar");
        assert_eq!(
            lexed.kinds(),
            vec![
                TokenType::Esc,
                TokenType::Eol,
                TokenType::Esc,
                TokenType::Eol,
            ]
        );
        assert_eq!(lexed.source_map.text(lexed.tokens[1].span), "\n \t;\n");
    }

    #[test]
    fn leading_whitespace_before_word() {
        let lexed = Lexed::run("  foo");
        assert_eq!(
            lexed.kinds(),
            vec![TokenType::Sep, TokenType::Esc, TokenType::Eol]
        );
    }

    #[test]
    fn trailing_whitespace_still_emits_ghost_eol() {
        let lexed = Lexed::run("foo  ");
        assert_eq!(
            lexed.kinds(),
            vec![TokenType::Esc, TokenType::Sep, TokenType::Eol]
        );
        assert_eq!(lexed.source_map.text(lexed.tokens[2].span), "");
    }

    #[test]
    fn trailing_newline_does_not_add_second_eol() {
        let lexed = Lexed::run("foo\n");
        assert_eq!(lexed.kinds(), vec![TokenType::Esc, TokenType::Eol]);
        assert_eq!(lexed.source_map.text(lexed.tokens[1].span), "\n");
    }

    #[test]
    fn comment_at_command_start() {
        let lexed = Lexed::run("# hello world");
        assert_eq!(lexed.tokens[0].kind, TokenType::Comment);
        assert_eq!(lexed.source_map.text(lexed.tokens[0].span), "# hello world");
    }

    #[test]
    fn comment_terminated_by_newline() {
        let lexed = Lexed::run("# hello\nfoo");
        assert_eq!(lexed.tokens[0].kind, TokenType::Comment);
        assert_eq!(lexed.source_map.text(lexed.tokens[0].span), "# hello");
        assert_eq!(lexed.tokens[1].kind, TokenType::Eol);
        assert_eq!(lexed.source_map.text(lexed.tokens[1].span), "\n");
        assert_eq!(lexed.tokens[2].kind, TokenType::Esc);
        assert_eq!(lexed.source_map.text(lexed.tokens[2].span), "foo");
    }

    #[test]
    fn comment_after_whitespace_at_command_start() {
        // Leading whitespace preserves `at_command_start`, so `#` is
        // still a comment.
        let lexed = Lexed::run("   # comment");
        assert_eq!(lexed.tokens[0].kind, TokenType::Sep);
        assert_eq!(lexed.tokens[1].kind, TokenType::Comment);
        assert_eq!(lexed.source_map.text(lexed.tokens[1].span), "# comment");
    }

    #[test]
    fn hash_not_at_command_start_is_part_of_word() {
        let lexed = Lexed::run("foo #bar");
        assert_eq!(
            lexed.kinds(),
            vec![
                TokenType::Esc,
                TokenType::Sep,
                TokenType::Esc,
                TokenType::Eol,
            ]
        );
        assert_eq!(lexed.source_map.text(lexed.tokens[2].span), "#bar");
    }

    #[test]
    fn two_commands_separated_by_eol_both_allow_comments() {
        let lexed = Lexed::run("foo\n# comment");
        assert!(lexed.tokens.iter().any(|t| t.kind == TokenType::Comment));
    }

    #[test]
    fn position_tracking_simple_word() {
        let lexed = Lexed::run("foo");
        let (start, end) = lexed.positions(0);
        assert_eq!(start, SourcePosition::new(0, 0, 0));
        assert_eq!(end, SourcePosition::new(0, 2, 2));
    }

    #[test]
    fn position_tracking_across_newline() {
        let lexed = Lexed::run("ab\ncd");
        // ESC "ab" at (0,0)-(0,1)
        let (start, end) = lexed.positions(0);
        assert_eq!(lexed.source_map.text(lexed.tokens[0].span), "ab");
        assert_eq!(start, SourcePosition::new(0, 0, 0));
        assert_eq!(end, SourcePosition::new(0, 1, 1));
        // EOL "\n" at (0,2)-(0,2)
        let (start, end) = lexed.positions(1);
        assert_eq!(lexed.tokens[1].kind, TokenType::Eol);
        assert_eq!(lexed.source_map.text(lexed.tokens[1].span), "\n");
        assert_eq!(start, SourcePosition::new(0, 2, 2));
        assert_eq!(end, SourcePosition::new(0, 2, 2));
        // ESC "cd" at (1,0)-(1,1)
        let (start, end) = lexed.positions(2);
        assert_eq!(lexed.source_map.text(lexed.tokens[2].span), "cd");
        assert_eq!(start, SourcePosition::new(1, 0, 3));
        assert_eq!(end, SourcePosition::new(1, 1, 4));
    }

    #[test]
    fn spans_are_accurate() {
        let lexed = Lexed::run("foo bar");
        assert_eq!(lexed.tokens[0].span, Span::new(0, 3)); // "foo"
        assert_eq!(lexed.tokens[1].span, Span::new(3, 4)); // " "
        assert_eq!(lexed.tokens[2].span, Span::new(4, 7)); // "bar"
        assert!(lexed.tokens[3].span.is_empty()); // ghost EOL
    }

    #[test]
    fn dollar_is_no_longer_an_unsupported_character() {
        // Regression guard: L4 removed `$` from the deferred set.
        // The lexer should accept `$bar` as a VAR token, not error.
        let tokens = Lexer::new("foo $bar").tokenise_all().unwrap();
        let kinds: Vec<TokenType> = tokens.iter().map(|t| t.kind).collect();
        assert_eq!(
            kinds,
            vec![
                TokenType::Esc,
                TokenType::Sep,
                TokenType::Var,
                TokenType::Eol,
            ]
        );
    }

    #[test]
    fn brace_is_no_longer_an_unsupported_character() {
        // Regression guard: L6 removed `{` and `}` from the deferred
        // set. `foo {bar}` should now lex as ESC + SEP + STR + EOL.
        let tokens = Lexer::new("foo {bar}").tokenise_all().unwrap();
        let kinds: Vec<TokenType> = tokens.iter().map(|t| t.kind).collect();
        assert_eq!(
            kinds,
            vec![
                TokenType::Esc,
                TokenType::Sep,
                TokenType::Str,
                TokenType::Eol,
            ]
        );
    }

    #[test]
    fn bracket_is_no_longer_an_unsupported_character() {
        // Regression guard: L5 removed `[` from the deferred set.
        // `[cmd]` should now lex as a CMD token, not error.
        let tokens = Lexer::new("[cmd]").tokenise_all().unwrap();
        let kinds: Vec<TokenType> = tokens.iter().map(|t| t.kind).collect();
        assert_eq!(kinds, vec![TokenType::Cmd, TokenType::Eol]);
    }

    #[test]
    fn unsupported_character_backslash_errors() {
        let err = Lexer::new(r"foo \n bar").tokenise_all().unwrap_err();
        assert!(matches!(
            err,
            LexError::UnsupportedCharacter { ch: '\\', .. }
        ));
    }

    #[test]
    fn unsupported_character_in_comment_errors() {
        let err = Lexer::new("# hello\\ world").tokenise_all().unwrap_err();
        assert!(matches!(
            err,
            LexError::UnsupportedCharacter { ch: '\\', .. }
        ));
    }

    #[test]
    fn after_error_iterator_stops() {
        // Use `"` which is still in the deferred set after L6.
        let mut lex = Lexer::new(r#"foo "bar""#);
        let mut tokens = Vec::new();
        let mut err_seen = false;
        for result in lex.by_ref() {
            if let Ok(tok) = result {
                tokens.push(tok);
            } else {
                err_seen = true;
                break;
            }
        }
        assert!(err_seen);
        assert!(lex.next().is_none());
    }

    #[test]
    fn shared_source_map_constructor() {
        let source = "foo\nbar";
        let map = SourceMap::new(source);
        let via_shared = Lexer::with_source_map(map, LexerConfig::default())
            .tokenise_all()
            .unwrap();
        let via_new = Lexer::new(source).tokenise_all().unwrap();
        assert_eq!(via_shared, via_new);
    }

    #[test]
    fn shared_line_index_via_source_map() {
        let source = "alpha beta";
        let idx = LineIndex::new(source);
        let map = SourceMap::with_line_index(source, idx);
        let tokens = Lexer::with_source_map(map, LexerConfig::default())
            .tokenise_all()
            .unwrap();
        assert!(!tokens.is_empty());
    }

    // ------------------------------------------------------------------
    // L4 — variable substitution
    // ------------------------------------------------------------------

    fn var_token_text(source: &str) -> (Vec<(TokenType, String)>, SourceMap<'_>) {
        let lexer = Lexer::new(source);
        let map = lexer.source_map().clone();
        let tokens = lexer.tokenise_all().expect("L4 lexer accepts fixture");
        // `token_text` strips the leading `$` / `${` for VAR tokens
        // so the assertions mirror Python's `tok.text` field.
        let rows = tokens
            .iter()
            .map(|t| (t.kind, map.token_text(*t).to_owned()))
            .collect();
        (rows, map)
    }

    #[test]
    fn var_simple_identifier() {
        let (rows, _) = var_token_text("$foo");
        assert_eq!(
            rows,
            vec![
                (TokenType::Var, "foo".into()),
                (TokenType::Eol, String::new()),
            ]
        );
    }

    #[test]
    fn var_with_underscore() {
        let (rows, _) = var_token_text("$_private");
        assert_eq!(rows[0], (TokenType::Var, "_private".into()));
    }

    #[test]
    fn var_alphanumeric_accepts_digits_anywhere() {
        assert_eq!(
            var_token_text("$foo1").0[0],
            (TokenType::Var, "foo1".into())
        );
        // Python allows digits at the start of variable names —
        // Tcl uses `$1`, `$2` etc. for regexp backrefs.
        assert_eq!(var_token_text("$1").0[0], (TokenType::Var, "1".into()));
    }

    #[test]
    fn var_uppercase() {
        assert_eq!(var_token_text("$FOO").0[0], (TokenType::Var, "FOO".into()));
    }

    #[test]
    fn var_namespace_separator() {
        let (rows, _) = var_token_text("$ns::var");
        assert_eq!(rows[0], (TokenType::Var, "ns::var".into()));
    }

    #[test]
    fn var_multi_level_namespace() {
        let (rows, _) = var_token_text("$a::b::c");
        assert_eq!(rows[0], (TokenType::Var, "a::b::c".into()));
    }

    #[test]
    fn var_leading_namespace() {
        // `$::global` — starts with `::` (double colon).
        let (rows, _) = var_token_text("$::global");
        assert_eq!(rows[0], (TokenType::Var, "::global".into()));
    }

    #[test]
    fn var_single_colon_terminates_name() {
        // A single `:` is not part of the identifier; it ends the
        // VAR token and the rest becomes an ESC token.
        let (rows, _) = var_token_text("$foo:bar");
        assert_eq!(rows[0], (TokenType::Var, "foo".into()));
        assert_eq!(rows[1], (TokenType::Esc, ":bar".into()));
    }

    #[test]
    fn var_braced_form() {
        let (rows, _) = var_token_text("${name}");
        // The braces are stripped — the token text is the body only.
        assert_eq!(rows[0], (TokenType::Var, "name".into()));
    }

    #[test]
    fn var_braced_empty_body() {
        let (rows, _) = var_token_text("${}");
        assert_eq!(rows[0], (TokenType::Var, String::new()));
    }

    #[test]
    fn var_braced_allows_arbitrary_characters() {
        // Inside `${…}` all characters except `}` are legal, including
        // spaces, `$`, `[`, etc.
        let (rows, _) = var_token_text("${weird name with spaces}");
        assert_eq!(rows[0], (TokenType::Var, "weird name with spaces".into()));
    }

    #[test]
    fn var_braced_unterminated_tokenises_best_effort() {
        // Missing `}` — Python emits a non-fatal warning and
        // tokenises the remaining input as the variable name. L4
        // matches the tokenisation but does not emit the warning
        // (L9 adds warning collection).
        let (rows, _) = var_token_text("${unterminated");
        assert_eq!(rows[0], (TokenType::Var, "unterminated".into()));
    }

    #[test]
    fn var_array_index() {
        let (rows, _) = var_token_text("$arr(idx)");
        // Span covers the whole `arr(idx)` — including the parens —
        // but not the leading `$`, matching Python.
        assert_eq!(rows[0], (TokenType::Var, "arr(idx)".into()));
    }

    #[test]
    fn var_array_index_nested_parens() {
        let (rows, _) = var_token_text("$arr(one(two)three)");
        assert_eq!(rows[0], (TokenType::Var, "arr(one(two)three)".into()));
    }

    #[test]
    fn var_array_index_with_inner_braced_var() {
        // `${key}` inside the index scans to the matching `}` as a
        // unit — the `(` / `)` inside such a braced name would not
        // count against the array-index depth. Python does this so
        // a variable-named-with-parens doesn't fool the index
        // scanner.
        let (rows, _) = var_token_text("$arr(${key})");
        assert_eq!(rows[0], (TokenType::Var, "arr(${key})".into()));
    }

    #[test]
    fn var_array_index_unterminated_tokenises_best_effort() {
        let (rows, _) = var_token_text("$arr(idx");
        assert_eq!(rows[0], (TokenType::Var, "arr(idx".into()));
    }

    #[test]
    fn bare_dollar_is_an_str_token() {
        // Python emits bare `$` as an STR token whose text is the
        // `$` character — not a VAR.
        let (rows, _) = var_token_text("$");
        assert_eq!(rows[0], (TokenType::Str, "$".into()));
    }

    #[test]
    fn bare_dollar_followed_by_space() {
        let (rows, _) = var_token_text("$ foo");
        assert_eq!(rows[0], (TokenType::Str, "$".into()));
        assert_eq!(rows[1], (TokenType::Sep, " ".into()));
        assert_eq!(rows[2], (TokenType::Esc, "foo".into()));
    }

    #[test]
    fn bare_dollar_followed_by_lf() {
        let (rows, _) = var_token_text("$\n");
        assert_eq!(rows[0], (TokenType::Str, "$".into()));
        assert_eq!(rows[1], (TokenType::Eol, "\n".into()));
    }

    #[test]
    fn var_followed_by_word() {
        // `$foo bar` — VAR then SEP then ESC.
        let (rows, _) = var_token_text("$foo bar");
        assert_eq!(rows[0], (TokenType::Var, "foo".into()));
        assert_eq!(rows[1], (TokenType::Sep, " ".into()));
        assert_eq!(rows[2], (TokenType::Esc, "bar".into()));
    }

    #[test]
    fn multiple_vars() {
        let (rows, _) = var_token_text("$a $b $c");
        assert_eq!(rows[0], (TokenType::Var, "a".into()));
        assert_eq!(rows[2], (TokenType::Var, "b".into()));
        assert_eq!(rows[4], (TokenType::Var, "c".into()));
    }

    #[test]
    fn var_resets_at_command_start() {
        // After a VAR token, `#` is no longer a comment opener.
        let (rows, _) = var_token_text("$foo #bar");
        assert_eq!(rows[0], (TokenType::Var, "foo".into()));
        assert_eq!(rows[1], (TokenType::Sep, " ".into()));
        // `#bar` should be an ESC, not a COMMENT.
        assert_eq!(rows[2], (TokenType::Esc, "#bar".into()));
    }

    #[test]
    fn esc_stops_at_dollar() {
        // `foo$bar` — ESC "foo", VAR "bar". The `$` terminates the
        // bare word rather than being consumed as a literal.
        let (rows, _) = var_token_text("foo$bar");
        assert_eq!(rows[0], (TokenType::Esc, "foo".into()));
        assert_eq!(rows[1], (TokenType::Var, "bar".into()));
    }

    #[test]
    fn var_span_positions() {
        // `$foo bar` — the VAR span covers the whole `$foo` (offset
        // 0..4), matching the Python lexer's convention that
        // `Token.start` points at the `$` and `Token.end` at the
        // last char of the name. `token_text` is how you get just
        // the "foo" part.
        let lexer = Lexer::new("$foo bar");
        let map = lexer.source_map().clone();
        let tokens = lexer.tokenise_all().unwrap();
        let var = tokens.iter().find(|t| t.kind == TokenType::Var).unwrap();
        assert_eq!(var.span.start(), 0);
        assert_eq!(var.span.end(), 4);
        let (start, end) = map.range_positions(var.span);
        assert_eq!(start, SourcePosition::new(0, 0, 0));
        assert_eq!(end, SourcePosition::new(0, 3, 3));
        assert_eq!(map.token_text(*var), "foo");
    }

    #[test]
    fn braced_var_span_covers_delimiter_and_name() {
        // `${name}` — span is [0, 6), covering "${name" but NOT the
        // closing `}`. The lexer consumes the `}` so the next
        // dispatch starts at offset 7. `token_text` strips the `${`
        // wrapper so the visible text is "name".
        let lexer = Lexer::new("${name}");
        let map = lexer.source_map().clone();
        let tokens = lexer.tokenise_all().unwrap();
        let var = tokens.iter().find(|t| t.kind == TokenType::Var).unwrap();
        assert_eq!(var.span.start(), 0);
        assert_eq!(var.span.end(), 6);
        assert_eq!(map.text(var.span), "${name");
        assert_eq!(map.token_text(*var), "name");
    }

    #[test]
    fn into_source_map_round_trip() {
        let source = "a\nb\nc";
        let lexer = Lexer::new(source);
        let map = lexer.into_source_map();
        assert_eq!(map.line_index().line_count(), 3);
    }

    // ------------------------------------------------------------------
    // L5 — command substitution
    // ------------------------------------------------------------------

    fn cmd_token_rows(source: &str) -> (Vec<(TokenType, String)>, SourceMap<'_>) {
        let lexer = Lexer::new(source);
        let map = lexer.source_map().clone();
        let tokens = lexer.tokenise_all().expect("L5 lexer accepts fixture");
        let rows = tokens
            .iter()
            .map(|t| (t.kind, map.token_text(*t).to_owned()))
            .collect();
        (rows, map)
    }

    #[test]
    fn cmd_simple_body() {
        let (rows, _) = cmd_token_rows("[+ 1 2]");
        assert_eq!(
            rows,
            vec![
                (TokenType::Cmd, "+ 1 2".into()),
                (TokenType::Eol, String::new()),
            ]
        );
    }

    #[test]
    fn cmd_empty_body() {
        // Degenerate `[]` — exercises the empty-body end-position clamp.
        let (rows, _) = cmd_token_rows("[]");
        assert_eq!(rows[0], (TokenType::Cmd, String::new()));
    }

    #[test]
    fn cmd_nested_brackets() {
        let (rows, _) = cmd_token_rows("[+ 1 [+ 2 3]]");
        assert_eq!(rows[0], (TokenType::Cmd, "+ 1 [+ 2 3]".into()));
    }

    #[test]
    fn cmd_deeply_nested_brackets() {
        let (rows, _) = cmd_token_rows("[a [b [c [d]]]]");
        assert_eq!(rows[0], (TokenType::Cmd, "a [b [c [d]]]".into()));
    }

    #[test]
    fn cmd_followed_by_word() {
        let (rows, _) = cmd_token_rows("[cmd] tail");
        assert_eq!(rows[0], (TokenType::Cmd, "cmd".into()));
        assert_eq!(rows[1], (TokenType::Sep, " ".into()));
        assert_eq!(rows[2], (TokenType::Esc, "tail".into()));
    }

    #[test]
    fn word_then_cmd() {
        // `foo[cmd]` — ESC then CMD. The `[` terminates the bare
        // word.
        let (rows, _) = cmd_token_rows("foo[cmd]");
        assert_eq!(rows[0], (TokenType::Esc, "foo".into()));
        assert_eq!(rows[1], (TokenType::Cmd, "cmd".into()));
    }

    #[test]
    fn cmd_then_word() {
        let (rows, _) = cmd_token_rows("[cmd]tail");
        assert_eq!(rows[0], (TokenType::Cmd, "cmd".into()));
        assert_eq!(rows[1], (TokenType::Esc, "tail".into()));
    }

    #[test]
    fn cmd_with_quoted_substring() {
        // `"..."` inside a command body is NOT a quoted string at
        // the top level — parse_command just tracks it for its
        // bracket-nesting state so that `]` inside the quotes does
        // not close the command.
        let (rows, _) = cmd_token_rows(r#"[puts "hello world"]"#);
        assert_eq!(rows[0], (TokenType::Cmd, r#"puts "hello world""#.into()));
    }

    #[test]
    fn cmd_with_bracket_inside_quotes_does_not_close() {
        let (rows, _) = cmd_token_rows(r#"[puts "a]b"]"#);
        assert_eq!(rows[0], (TokenType::Cmd, r#"puts "a]b""#.into()));
    }

    #[test]
    fn cmd_with_braced_substring() {
        // `{…}` inside a command body adjusts `blevel` so `]`
        // inside the braces does not close the command.
        let (rows, _) = cmd_token_rows("[list {a b c}]");
        assert_eq!(rows[0], (TokenType::Cmd, "list {a b c}".into()));
    }

    #[test]
    fn cmd_with_bracket_inside_braces_does_not_close() {
        let (rows, _) = cmd_token_rows("[list {a ] b}]");
        assert_eq!(rows[0], (TokenType::Cmd, "list {a ] b}".into()));
    }

    #[test]
    fn cmd_with_nested_braces() {
        let (rows, _) = cmd_token_rows("[list {a {nested} b}]");
        assert_eq!(rows[0], (TokenType::Cmd, "list {a {nested} b}".into()));
    }

    #[test]
    fn cmd_with_backslash_escape() {
        // `\]` inside the body is inert — it doesn't close the
        // command. The backslash consumes two bytes as a pair.
        let (rows, _) = cmd_token_rows(r"[a \] b]");
        assert_eq!(rows[0], (TokenType::Cmd, r"a \] b".into()));
    }

    #[test]
    fn cmd_with_backslash_quote() {
        // `\"` inside the body is inert — the quote state does not
        // toggle because the `"` is escaped.
        let (rows, _) = cmd_token_rows(r#"[a \" b]"#);
        assert_eq!(rows[0], (TokenType::Cmd, r#"a \" b"#.into()));
    }

    #[test]
    fn cmd_with_dollar_braced_var_inside() {
        // `${...}` inside the command body is sub-scanned so a `}`
        // inside the braced name does not throw off the counter.
        let (rows, _) = cmd_token_rows("[set ${odd}name value]");
        assert_eq!(rows[0], (TokenType::Cmd, "set ${odd}name value".into()));
    }

    #[test]
    fn cmd_with_plain_dollar_var_inside() {
        let (rows, _) = cmd_token_rows("[expr $a + $b]");
        assert_eq!(rows[0], (TokenType::Cmd, "expr $a + $b".into()));
    }

    #[test]
    fn cmd_multiline_body() {
        let (rows, _) = cmd_token_rows("[a\nb\nc]");
        assert_eq!(rows[0], (TokenType::Cmd, "a\nb\nc".into()));
    }

    #[test]
    fn cmd_unterminated_tokenises_best_effort() {
        let (rows, _) = cmd_token_rows("[unterminated");
        assert_eq!(rows[0], (TokenType::Cmd, "unterminated".into()));
    }

    #[test]
    fn cmd_span_positions() {
        // `[cmd] rest` — the CMD span covers the whole `[cmd]` range
        // (offset 0..4 per Python's convention: start at `[`, end
        // at the last char of the body, `token_text` strips the
        // leading `[`).
        let lexer = Lexer::new("[cmd] rest");
        let map = lexer.source_map().clone();
        let tokens = lexer.tokenise_all().unwrap();
        let cmd = tokens.iter().find(|t| t.kind == TokenType::Cmd).unwrap();
        assert_eq!(cmd.span.start(), 0);
        assert_eq!(cmd.span.end(), 4);
        let (start, end) = map.range_positions(cmd.span);
        assert_eq!(start, SourcePosition::new(0, 0, 0));
        assert_eq!(end, SourcePosition::new(0, 3, 3));
        assert_eq!(map.token_text(*cmd), "cmd");
    }

    #[test]
    fn standalone_closing_bracket_is_part_of_word() {
        // `foo]bar` — `]` is not a deferred character after L5, so
        // it's included in the bare word matching Python's
        // `_parse_string` behaviour.
        let (rows, _) = cmd_token_rows("foo]bar");
        assert_eq!(rows[0], (TokenType::Esc, "foo]bar".into()));
    }

    #[test]
    fn cmd_resets_at_command_start() {
        let (rows, _) = cmd_token_rows("[cmd] #not-a-comment");
        assert_eq!(rows[0], (TokenType::Cmd, "cmd".into()));
        assert_eq!(rows[1], (TokenType::Sep, " ".into()));
        assert_eq!(rows[2], (TokenType::Esc, "#not-a-comment".into()));
    }

    #[test]
    fn cmd_after_eol_allows_comment_before() {
        let (rows, _) = cmd_token_rows("# c\n[cmd]");
        assert!(rows.iter().any(|(k, _)| *k == TokenType::Comment));
        assert!(rows.iter().any(|(k, _)| *k == TokenType::Cmd));
    }

    // ------------------------------------------------------------------
    // L6 — braced strings
    // ------------------------------------------------------------------

    fn str_token_rows(source: &str) -> (Vec<(TokenType, String)>, SourceMap<'_>) {
        let lexer = Lexer::new(source);
        let map = lexer.source_map().clone();
        let tokens = lexer.tokenise_all().expect("L6 lexer accepts fixture");
        let rows = tokens
            .iter()
            .map(|t| (t.kind, map.token_text(*t).to_owned()))
            .collect();
        (rows, map)
    }

    #[test]
    fn braced_simple_body() {
        let (rows, _) = str_token_rows("{hello world}");
        assert_eq!(
            rows,
            vec![
                (TokenType::Str, "hello world".into()),
                (TokenType::Eol, String::new()),
            ]
        );
    }

    #[test]
    fn braced_empty_body() {
        let (rows, _) = str_token_rows("{}");
        assert_eq!(rows[0], (TokenType::Str, String::new()));
    }

    #[test]
    fn braced_nested_once() {
        let (rows, _) = str_token_rows("{a {b c} d}");
        assert_eq!(rows[0], (TokenType::Str, "a {b c} d".into()));
    }

    #[test]
    fn braced_deeply_nested() {
        let (rows, _) = str_token_rows("{a {b {c {d}}}}");
        assert_eq!(rows[0], (TokenType::Str, "a {b {c {d}}}".into()));
    }

    #[test]
    fn braced_after_word() {
        // `proc foo {body}` — SEP after "foo" makes `{body}` a
        // newword, so the `{` starts a braced string.
        let (rows, _) = str_token_rows("proc foo {body}");
        assert_eq!(rows[0], (TokenType::Esc, "proc".into()));
        assert_eq!(rows[2], (TokenType::Esc, "foo".into()));
        assert_eq!(rows[4], (TokenType::Str, "body".into()));
    }

    #[test]
    fn braced_midword_is_regular_character() {
        // `foo{bar}` — the `{` is NOT at a word boundary (the
        // previous token was ESC, not SEP/EOL/STR/EXPAND), so it's
        // a regular character in the bare word. Matches Python's
        // `_parse_string` fall-through.
        let (rows, _) = str_token_rows("foo{bar}");
        assert_eq!(rows[0], (TokenType::Esc, "foo{bar}".into()));
    }

    #[test]
    fn close_brace_midword_is_regular_character() {
        let (rows, _) = str_token_rows("foo}bar");
        assert_eq!(rows[0], (TokenType::Esc, "foo}bar".into()));
    }

    #[test]
    fn braced_multiline_body() {
        let (rows, _) = str_token_rows("{line1\nline2}");
        assert_eq!(rows[0], (TokenType::Str, "line1\nline2".into()));
    }

    #[test]
    fn braced_with_dollar_is_literal() {
        // `{foo $bar}` — `$` inside a braced string is a literal
        // character, not a variable substitution. This happens
        // naturally because the brace scanner only looks for
        // balanced `{` / `}` and backslashes.
        let (rows, _) = str_token_rows("{foo $bar}");
        assert_eq!(rows[0], (TokenType::Str, "foo $bar".into()));
    }

    #[test]
    fn braced_with_brackets_is_literal() {
        let (rows, _) = str_token_rows("{foo [bar]}");
        assert_eq!(rows[0], (TokenType::Str, "foo [bar]".into()));
    }

    #[test]
    fn braced_with_backslash_is_literal_pair() {
        // `{foo\nbar}` — the backslash and `n` are preserved in
        // the token text because braces are verbatim in Tcl. The
        // scanner skips them as a pair so `\}` does not count as
        // a close brace.
        let (rows, _) = str_token_rows(r"{foo\nbar}");
        assert_eq!(rows[0], (TokenType::Str, r"foo\nbar".into()));
    }

    #[test]
    fn braced_with_backslash_close_brace_is_inert() {
        let (rows, _) = str_token_rows(r"{a\}b}");
        assert_eq!(rows[0], (TokenType::Str, r"a\}b".into()));
    }

    #[test]
    fn braced_with_backslash_open_brace_is_inert() {
        let (rows, _) = str_token_rows(r"{a\{b}");
        assert_eq!(rows[0], (TokenType::Str, r"a\{b".into()));
    }

    #[test]
    fn braced_followed_by_word() {
        let (rows, _) = str_token_rows("{foo} bar");
        assert_eq!(rows[0], (TokenType::Str, "foo".into()));
        assert_eq!(rows[1], (TokenType::Sep, " ".into()));
        assert_eq!(rows[2], (TokenType::Esc, "bar".into()));
    }

    #[test]
    fn braced_then_braced() {
        // After a STR token, `newword` is True again (STR is in
        // the `newword` set alongside SEP/EOL/EXPAND), so a second
        // `{` starts another braced string.
        let (rows, _) = str_token_rows("{a}{b}");
        assert_eq!(rows[0], (TokenType::Str, "a".into()));
        assert_eq!(rows[1], (TokenType::Str, "b".into()));
    }

    #[test]
    fn braced_unterminated_tokenises_best_effort() {
        let (rows, _) = str_token_rows("{unterminated");
        assert_eq!(rows[0], (TokenType::Str, "unterminated".into()));
    }

    #[test]
    fn braced_at_command_start() {
        // A `{` at the start of a line (initial at_command_start=
        // true, initial last_kind=Eol → newword=true) is a STR.
        let (rows, _) = str_token_rows("{hello}");
        assert_eq!(rows[0], (TokenType::Str, "hello".into()));
    }

    #[test]
    fn braced_inside_command_substitution() {
        // `[list {a b}]` — the braces inside a CMD body are handled
        // by `parse_command`'s own `blevel` tracking, not by
        // `parse_brace`. The STR stays one CMD token.
        let (rows, _) = str_token_rows("[list {a b}]");
        assert_eq!(rows[0], (TokenType::Cmd, "list {a b}".into()));
    }

    #[test]
    fn braced_span_positions() {
        // `{hello}` — span covers `{hello` (0..6), token_text
        // strips `{`, end position is at the 'o' not the `}`.
        let lexer = Lexer::new("{hello}");
        let map = lexer.source_map().clone();
        let tokens = lexer.tokenise_all().unwrap();
        let brace = tokens.iter().find(|t| t.kind == TokenType::Str).unwrap();
        assert_eq!(brace.span.start(), 0);
        assert_eq!(brace.span.end(), 6);
        let (start, end) = map.range_positions(brace.span);
        assert_eq!(start, SourcePosition::new(0, 0, 0));
        assert_eq!(end, SourcePosition::new(0, 5, 5));
        assert_eq!(map.token_text(*brace), "hello");
    }

    #[test]
    fn braced_resets_at_command_start() {
        // After a STR, `at_command_start` is False so `#` is not
        // a comment.
        let (rows, _) = str_token_rows("{body} #not-a-comment");
        assert_eq!(rows[0], (TokenType::Str, "body".into()));
        assert_eq!(rows[1], (TokenType::Sep, " ".into()));
        assert_eq!(rows[2], (TokenType::Esc, "#not-a-comment".into()));
    }

    #[test]
    fn braced_preserves_newword_for_next_token() {
        // `{a}{b}{c}` — every STR sets `last_kind = Str`, which is
        // in the `newword` set, so subsequent `{`s start new
        // braced strings.
        let (rows, _) = str_token_rows("{a}{b}{c}");
        assert_eq!(rows[0], (TokenType::Str, "a".into()));
        assert_eq!(rows[1], (TokenType::Str, "b".into()));
        assert_eq!(rows[2], (TokenType::Str, "c".into()));
    }
}
