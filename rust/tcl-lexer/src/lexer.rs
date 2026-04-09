//! Streaming Tcl lexer.
//!
//! **L3 skeleton.** First Rust lexer chunk. Handles the five simplest
//! token kinds:
//!
//! - **EOF handling** — emits a trailing synthetic `EOL` (once) when the
//!   source does not already end with an EOL token, matching the
//!   Python lexer's `get_token()` / `tokenise_all()` contract.
//! - **SEP** — runs of horizontal whitespace (`' '`, `\t`, `\r`, VT, FF).
//!   `\r` is horizontal whitespace in Tcl, not an EOL.
//! - **EOL** — runs of EOL characters (`\n`, `;`) interleaved with
//!   horizontal whitespace, mirroring Python's `_parse_eol`.
//! - **COMMENT** — `#` at command start, scanned to the next `\n`
//!   (exclusive). Backslash-newline continuation inside comments is
//!   not handled in L3; an input whose comment contains a `\` is
//!   reported as [`LexError::UnsupportedCharacter`] so the
//!   differential harness can filter it.
//! - **ESC** — runs of characters that are neither whitespace nor EOL
//!   nor one of the "deferred" special characters.
//!
//! The "deferred" set is `$ [ ] { } " \`. Encountering any of them
//! trips [`LexError::UnsupportedCharacter`] carrying the character and
//! its position. Callers filter inputs on that error rather than
//! receiving silently-wrong token streams. Chunks L4–L9 shrink the set
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
//! - L4: variable substitution (`$name`, `${name}`, `$arr(idx)`,
//!   `$ns::var`)
//! - L5: command substitution (`[…]`, possibly nested)
//! - L6: braced strings (`{…}`, possibly nested)
//! - L7: quoted strings (`"…"`)
//! - L8: expansion prefix (`{*}`), `strict_quoting`, `expand_syntax`,
//!   `irules_brace_separator`
//! - L9: backslash escapes and line continuation; warning collection;
//!   virtual character insertion for error recovery
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
    /// EOF needs a trailing synthetic EOL.
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
            // zero tokens rather than a lone synthetic trailing EOL,
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
}

impl Iterator for Lexer<'_> {
    type Item = Result<Token, LexError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        // EOF: emit a trailing synthetic EOL (once) then stop.
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
/// Triggers [`LexError::UnsupportedCharacter`]. Chunks L4–L9
/// incrementally drain this set.
#[inline]
fn is_deferred_special(ch: char) -> bool {
    matches!(ch, '$' | '[' | ']' | '{' | '}' | '"' | '\\')
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
    fn trailing_whitespace_still_emits_synthetic_eol() {
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
        assert!(lexed.tokens[3].span.is_empty()); // synthetic EOL
    }

    #[test]
    fn unsupported_character_dollar_errors() {
        let err = Lexer::new("foo $bar").tokenise_all().unwrap_err();
        match err {
            LexError::UnsupportedCharacter { ch, position } => {
                assert_eq!(ch, '$');
                assert_eq!(position.offset, 4);
            }
        }
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
        let mut lex = Lexer::new("foo $bar");
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

    #[test]
    fn into_source_map_round_trip() {
        let source = "a\nb\nc";
        let lexer = Lexer::new(source);
        let map = lexer.into_source_map();
        assert_eq!(map.line_index().line_count(), 3);
    }
}
