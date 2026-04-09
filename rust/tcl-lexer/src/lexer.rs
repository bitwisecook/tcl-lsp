//! Streaming Tcl lexer.
//!
//! **L3 + L4.** The first two lexer chunks, taken together, handle
//! five token kinds plus variable substitution:
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
//!   nor one of the "deferred" special characters.
//! - **VAR (L4)** — variable substitution in all four Tcl forms:
//!   `$name`, `$ns::var` (namespace-separated), `${name}` (braced),
//!   and `$arr(idx)` (array index with nested parens and embedded
//!   `${...}` support). A bare `$` with no name following is emitted
//!   as an `STR` token whose span covers just the `$`, matching
//!   Python's `_parse_var` fallback. Unterminated `${` and `$arr(`
//!   tokenize best-effort (L9 will add the warning-collection
//!   machinery to report them as diagnostics).
//!
//! The "deferred" set is now `[ ] { } " \`. Encountering any of them
//! trips [`LexError::UnsupportedCharacter`] carrying the character and
//! its position. Callers filter inputs on that error rather than
//! receiving silently-wrong token streams. Chunks L5–L9 shrink the set
//! to zero.
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
//!   ~~`$ns::var`)~~ — **landed in this chunk**
//! - L5: command substitution (`[…]`, possibly nested)
//! - L6: braced strings (`{…}`, possibly nested)
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
            // `$` terminates a bare word — the next token is a VAR.
            if ch == '$' {
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
/// Triggers [`LexError::UnsupportedCharacter`]. Chunks L5–L9
/// incrementally drain this set. L4 removed `$`.
#[inline]
fn is_deferred_special(ch: char) -> bool {
    matches!(ch, '[' | ']' | '{' | '}' | '"' | '\\')
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
    fn unsupported_character_brace_errors() {
        let err = Lexer::new("foo {bar}").tokenise_all().unwrap_err();
        assert!(matches!(
            err,
            LexError::UnsupportedCharacter { ch: '{', .. }
        ));
    }

    #[test]
    fn unsupported_character_bracket_errors() {
        let err = Lexer::new("[cmd]").tokenise_all().unwrap_err();
        assert!(matches!(
            err,
            LexError::UnsupportedCharacter { ch: '[', .. }
        ));
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
        // Use `[` which is still in the deferred set after L4.
        let mut lex = Lexer::new("foo [bar]");
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
}
