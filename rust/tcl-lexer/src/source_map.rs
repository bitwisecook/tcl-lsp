//! Source text + position lookup, bundled.
//!
//! A [`SourceMap`] pairs a `&str` source buffer with a [`LineIndex`]
//! and exposes the operations that every downstream Rust crate needs:
//! slicing text for a [`Span`], resolving a byte offset to a
//! `SourcePosition`, and resolving a full [`Span`] to its (start, end)
//! positions. Tokens, IR nodes, CFG nodes, and diagnostics all carry
//! bare [`Span`]s and ask the `SourceMap` for text or positions on
//! demand.
//!
//! Bundling source and line index into one type makes the "threading"
//! obvious: functions that need to resolve spans take `&SourceMap`,
//! not two separate parameters. The bundle is cheap: the source is a
//! borrowed slice (zero-sized to own) and the line index is a
//! `Box<[u32]>` that clones in one allocation.
//!
//! See `docs/rust-rewrite.md` for the broader "source map threaded
//! throughout" design.

use crate::line_index::LineIndex;
use crate::span::Span;
use crate::tokens::{SourcePosition, Token, TokenType};

/// A source buffer paired with its line index.
///
/// The primary lookup surface for anyone holding a [`Span`] and
/// wanting to know what text it covers or where it sits in (line,
/// character) space.
#[derive(Debug, Clone)]
pub struct SourceMap<'src> {
    source: &'src str,
    line_index: LineIndex,
    /// Sub-lexing base offsets. Added to every resolved position.
    base_offset: u32,
    base_line: u32,
    base_col: u32,
}

impl<'src> SourceMap<'src> {
    /// Build a `SourceMap` by scanning `source` once to populate its
    /// `LineIndex`.
    #[must_use]
    pub fn new(source: &'src str) -> Self {
        let line_index = LineIndex::new(source);
        Self {
            source,
            line_index,
            base_offset: 0,
            base_line: 0,
            base_col: 0,
        }
    }

    /// Build a `SourceMap` from an already-computed line index. The
    /// caller is responsible for ensuring the index was built from
    /// the same source string.
    #[must_use]
    pub fn with_line_index(source: &'src str, line_index: LineIndex) -> Self {
        Self {
            source,
            line_index,
            base_offset: 0,
            base_line: 0,
            base_col: 0,
        }
    }

    /// Set sub-lexing base offsets. These are added to every
    /// `SourcePosition` returned by [`Self::position_at`] and
    /// [`Self::range_positions`], matching Python's
    /// `base_offset` / `base_line` / `base_col` constructor
    /// parameters on `TclLexer`.
    #[must_use]
    pub fn with_base(mut self, base_offset: u32, base_line: u32, base_col: u32) -> Self {
        self.base_offset = base_offset;
        self.base_line = base_line;
        self.base_col = base_col;
        self
    }

    /// Borrow the underlying source buffer.
    #[must_use]
    pub fn source(&self) -> &'src str {
        self.source
    }

    /// Borrow the underlying line index.
    #[must_use]
    pub fn line_index(&self) -> &LineIndex {
        &self.line_index
    }

    /// Return the text slice covered by `span`. O(1).
    ///
    /// Returns the raw contents of the source buffer in the given
    /// byte range — including any syntactic delimiters (`$`, `${`,
    /// `{`, `"`, etc.) that the token's span covers. Pure-Rust
    /// callers that want to inspect raw source should use this.
    /// Callers that want the "human-readable" content matching the
    /// Python `Token.text` field should use [`Self::token_text`]
    /// instead.
    ///
    /// # Panics
    ///
    /// Panics if `span` is not a valid byte range in the source
    /// (either out of bounds or not on a UTF-8 character boundary).
    #[must_use]
    pub fn text(&self, span: Span) -> &'src str {
        &self.source[span.as_range()]
    }

    /// Return the "human-readable" text of a token — the same thing
    /// the Python `Token.text` field contains.
    ///
    /// For most kinds this is identical to `self.text(tok.span)`.
    /// For `VAR` tokens, the leading `$` (and the `{` of a `${…}`
    /// braced form) is stripped so the result is the variable name
    /// alone. As more wrapper-style tokens arrive (`STR` braced
    /// strings in L6, quoted strings in L7), this helper grows
    /// additional stripping rules; it is the **one place** in the
    /// codebase that encodes the Python-API convention of "position
    /// range spans the full token, text field is the inner content".
    ///
    /// The `PyO3` binding uses this helper when constructing
    /// `PyToken.text`; Rust consumers that want parity with the
    /// Python API should use it too.
    #[must_use]
    pub fn token_text(&self, tok: Token) -> &'src str {
        let raw = self.text(tok.span);
        // Strip the lexer-computed prefix (`$`, `${`, `[`, `{`, `"`,
        // etc.) to get to the content.
        let stripped = &raw[tok.content_offset as usize..];
        // Kind-specific trailing-strip or empty-clamp rules.
        match tok.kind {
            TokenType::Var => {
                // `${...}` braced form: the degenerate `${}` case
                // extends the span by one byte to cover the
                // closing `}`. Non-degenerate braced forms have no
                // trailing `}` inside the span.
                stripped.strip_suffix('}').unwrap_or(stripped)
            }
            TokenType::Cmd => {
                // `[]` degenerate: span extended by one to cover
                // the `]`. Non-empty nested commands like
                // `[+ 1 [inner]]` also end with `]` as a
                // legitimate inner bracket, so we must NOT
                // unconditionally strip — check for the exact
                // 1-character `]` remainder instead.
                if stripped == "]" {
                    ""
                } else {
                    stripped
                }
            }
            TokenType::Str => {
                // `{}` degenerate: same shape as `[]`.
                if stripped == "}" {
                    ""
                } else {
                    stripped
                }
            }
            TokenType::Esc => {
                // Quoted-string empty-clamp cases: when the
                // scanner stops with zero content consumed it
                // extends the span by one byte to cover the
                // terminator (`"`, `$`, or `[`). After stripping
                // the opening `"` via `content_offset`, a
                // one-character remainder that is exactly a
                // terminator character indicates an empty-body
                // token — return `""`. A legitimate one-character
                // body like `"a"` has `stripped = "a"` → 'a' is
                // not a terminator → return `"a"`. And a bare
                // word containing `"` characters has
                // `content_offset = 0`, so `stripped == raw`
                // (including any quote marks) and the check
                // doesn't fire.
                if stripped.len() == 1 && matches!(stripped.chars().next(), Some('"' | '$' | '[')) {
                    ""
                } else {
                    stripped
                }
            }
            _ => stripped,
        }
    }

    /// Resolve a byte offset to a full `SourcePosition`. O(log n).
    ///
    /// The returned position has `base_offset` / `base_line` /
    /// `base_col` applied so sub-lexing offsets are reflected.
    #[must_use]
    pub fn position_at(&self, offset: u32) -> SourcePosition {
        let raw = self.line_index.position_at(offset);
        SourcePosition::new(
            self.base_line + raw.line,
            if raw.line == 0 {
                self.base_col + raw.character
            } else {
                raw.character
            },
            self.base_offset + raw.offset,
        )
    }

    /// Resolve a span to `(start, end)` positions, where `start` is
    /// the position of the first byte and `end` is the position of
    /// the **last** byte (inclusive), matching the Python lexer's
    /// `Token.start` / `Token.end` convention. For an empty span,
    /// both positions point at `span.start()`.
    #[must_use]
    pub fn range_positions(&self, span: Span) -> (SourcePosition, SourcePosition) {
        let start = self.position_at(span.start());
        let end_offset = if span.is_empty() {
            span.start()
        } else {
            span.end() - 1
        };
        let end = self.position_at(end_offset);
        (start, end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_slicing() {
        let map = SourceMap::new("hello world");
        assert_eq!(map.text(Span::new(0, 5)), "hello");
        assert_eq!(map.text(Span::new(6, 11)), "world");
        assert_eq!(map.text(Span::new(0, 11)), "hello world");
    }

    #[test]
    fn empty_span_text_is_empty() {
        let map = SourceMap::new("hello");
        assert_eq!(map.text(Span::empty(3)), "");
    }

    #[test]
    fn position_at_start_of_line() {
        let map = SourceMap::new("abc\ndef");
        assert_eq!(map.position_at(0), SourcePosition::new(0, 0, 0));
        assert_eq!(map.position_at(4), SourcePosition::new(1, 0, 4));
    }

    #[test]
    fn range_positions_for_non_empty_span() {
        let map = SourceMap::new("abc def");
        // span covering "abc" — start at (0,0,0), end at (0,2,2) for 'c'
        let (start, end) = map.range_positions(Span::new(0, 3));
        assert_eq!(start, SourcePosition::new(0, 0, 0));
        assert_eq!(end, SourcePosition::new(0, 2, 2));
    }

    #[test]
    fn range_positions_for_empty_span_point_at_start() {
        let map = SourceMap::new("abc");
        let (start, end) = map.range_positions(Span::empty(3));
        assert_eq!(start, SourcePosition::new(0, 3, 3));
        assert_eq!(end, SourcePosition::new(0, 3, 3));
    }

    #[test]
    fn range_positions_across_newline() {
        let map = SourceMap::new("ab\ncd");
        // Span covering just the '\n' at offset 2
        let (start, end) = map.range_positions(Span::new(2, 3));
        assert_eq!(start, SourcePosition::new(0, 2, 2));
        assert_eq!(end, SourcePosition::new(0, 2, 2));
        // Span covering "cd" on line 1
        let (start, end) = map.range_positions(Span::new(3, 5));
        assert_eq!(start, SourcePosition::new(1, 0, 3));
        assert_eq!(end, SourcePosition::new(1, 1, 4));
    }

    #[test]
    fn with_shared_line_index() {
        let source = "alpha\nbeta";
        let idx = LineIndex::new(source);
        let map = SourceMap::with_line_index(source, idx);
        assert_eq!(map.source(), source);
        assert_eq!(map.line_index().line_count(), 2);
    }
}
