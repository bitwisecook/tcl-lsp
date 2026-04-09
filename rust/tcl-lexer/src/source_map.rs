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
}

impl<'src> SourceMap<'src> {
    /// Build a `SourceMap` by scanning `source` once to populate its
    /// `LineIndex`.
    #[must_use]
    pub fn new(source: &'src str) -> Self {
        let line_index = LineIndex::new(source);
        Self { source, line_index }
    }

    /// Build a `SourceMap` from an already-computed line index. The
    /// caller is responsible for ensuring the index was built from
    /// the same source string.
    #[must_use]
    pub fn with_line_index(source: &'src str, line_index: LineIndex) -> Self {
        Self { source, line_index }
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
        match tok.kind {
            TokenType::Var => {
                if let Some(after_open) = raw.strip_prefix("${") {
                    // For the degenerate `${}` case the lexer
                    // extends the span to cover the closing `}` so
                    // the token's end position matches Python's
                    // clamp. Strip it here; non-degenerate braced
                    // forms have no trailing `}` inside the span.
                    after_open.strip_suffix('}').unwrap_or(after_open)
                } else if let Some(inner) = raw.strip_prefix('$') {
                    inner
                } else {
                    raw
                }
            }
            TokenType::Cmd => {
                // CMD spans start at the `[` and normally end
                // before the matching close bracket, so the span
                // content is `[ + body`. For the degenerate `[]`
                // case the lexer extends the span by one byte so
                // the end position lands on the `]`; handle that
                // with an exact-match check rather than an
                // unconditional `strip_suffix(']')`, because
                // nested commands like `[+ 1 [inner]]` have a
                // legitimate `]` at the last byte of the span
                // (the inner close bracket) that must NOT be
                // stripped.
                if raw == "[]" {
                    ""
                } else {
                    raw.strip_prefix('[').unwrap_or(raw)
                }
            }
            TokenType::Str => {
                // STR spans start at the `{` and normally end
                // before the matching close brace. The degenerate
                // `{}` case extends the span to cover the `}`; use
                // the same exact-match check as `Cmd` so nested
                // braces like `{a {b} c}` keep their inner `}`
                // intact.
                if raw == "{}" {
                    ""
                } else {
                    raw.strip_prefix('{').unwrap_or(raw)
                }
            }
            _ => raw,
        }
    }

    /// Resolve a byte offset to a full `SourcePosition`. O(log n).
    #[must_use]
    pub fn position_at(&self, offset: u32) -> SourcePosition {
        self.line_index.position_at(offset)
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
