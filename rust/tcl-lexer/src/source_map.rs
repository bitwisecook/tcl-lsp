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

//! Source text + position lookup, bundled.
//!
//! A [`SourceMap`] pairs original source bytes with a [`LineIndex`]
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
//! borrowed slice and the line index is an `Arc<[u32]>` whose clone is
//! a refcount bump.
//!
//! See `docs/design/rust/engineering-guide.md` for the broader "source map threaded
//! throughout" design.

use crate::line_index::LineIndex;
use crate::span::Span;
use crate::tokens::{ByteCol, SourcePosition, Token, TokenType};

/// How original script bytes reached the parser.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SourceChannel {
    /// An original Tcl value passed to evaluation. CR is not translated.
    NativeValue,
    /// Document text whose source-channel continuations include CR and CRLF.
    Document,
}

/// Immutable original script bytes and their input-channel semantics.
///
/// Equality includes the channel. Grammar, interpreter identity and compilation
/// authority remain independently supplied by the caller's retained entry.
/// Hashing caches the immutable byte digest. Derived line positions are shared
/// across clones and excluded from equality, hashing and ordering. These still
/// compare the complete original bytes and channel, including hash collisions.
#[derive(Debug, Clone)]
pub struct SourceImage {
    bytes: std::sync::Arc<[u8]>,
    channel: SourceChannel,
    geometry: std::sync::Arc<SourceImageGeometry>,
}

#[derive(Debug)]
struct SourceImageGeometry {
    byte_hash: u64,
    line_index: Option<LineIndex>,
}

impl PartialEq for SourceImage {
    fn eq(&self, other: &Self) -> bool {
        self.channel == other.channel && self.bytes == other.bytes
    }
}
impl Eq for SourceImage {}
impl PartialOrd for SourceImage {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for SourceImage {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.bytes
            .cmp(&other.bytes)
            .then_with(|| self.channel.cmp(&other.channel))
    }
}
impl std::hash::Hash for SourceImage {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::hash::Hash::hash(&self.geometry.byte_hash, state);
        std::hash::Hash::hash(&self.channel, state);
    }
}

/// LF presentation of one authentic Document image, with checked source
/// coordinates. This supplies document geometry only: original values,
/// grammar, command identity and execution retain their independent owners.
#[derive(Debug, Clone)]
pub struct DocumentLineEndingProjection {
    original: SourceImage,
    normalised: String,
    collapsed: Vec<(u32, u32)>,
}

impl DocumentLineEndingProjection {
    /// Select Document newline translation. Native value images and source
    /// beyond the shared offset range have no document projection.
    #[must_use]
    pub fn new(original: SourceImage) -> Option<Self> {
        if original.channel() != SourceChannel::Document {
            return None;
        }
        let text = original.try_text().ok()?;
        u32::try_from(text.len()).ok()?;
        let mut normalised = String::with_capacity(text.len());
        let mut collapsed = Vec::new();
        let mut start = 0;
        let mut cursor = 0;
        while cursor < text.len() {
            if text.as_bytes()[cursor] != b'\r' {
                cursor += 1;
                continue;
            }
            normalised.push_str(text.get(start..cursor)?);
            if text.as_bytes().get(cursor + 1) == Some(&b'\n') {
                collapsed.push((
                    u32::try_from(cursor + 1).ok()?,
                    u32::try_from(normalised.len()).ok()?,
                ));
                cursor += 1;
            }
            normalised.push('\n');
            cursor += 1;
            start = cursor;
        }
        normalised.push_str(text.get(start..)?);
        Some(Self {
            original,
            normalised,
            collapsed,
        })
    }

    /// Original complete image; LF presentation never replaces its identity.
    #[must_use]
    pub const fn original(&self) -> &SourceImage {
        &self.original
    }

    /// Document presentation after CRLF and lone CR translation to LF.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.normalised
    }

    /// Consume the derived Document presentation without transferring original
    /// source, value, command, grammar or execution authority to the text.
    #[must_use]
    pub fn into_text(self) -> String {
        self.normalised
    }

    /// Map a Unicode byte boundary to the original document. The LF before
    /// and after a collapsed CRLF map to the complete pair's two boundaries.
    #[must_use]
    pub fn original_offset(&self, offset: u32) -> Option<u32> {
        self.normalised
            .is_char_boundary(usize::try_from(offset).ok()?)
            .then_some(())?;
        let removed = self.collapsed.partition_point(|&(_, at)| at < offset);
        let original = offset.checked_add(u32::try_from(removed).ok()?)?;
        self.original
            .try_text()
            .ok()?
            .is_char_boundary(usize::try_from(original).ok()?)
            .then_some(original)
    }

    /// Map an original Unicode byte boundary to LF presentation. The boundary
    /// between CR and LF has no image and cannot supply a fabricated extent.
    #[must_use]
    pub fn normalised_offset(&self, offset: u32) -> Option<u32> {
        self.original
            .try_text()
            .ok()?
            .is_char_boundary(usize::try_from(offset).ok()?)
            .then_some(())?;
        let removed = self.collapsed.partition_point(|&(at, _)| at < offset);
        if self
            .collapsed
            .get(removed)
            .is_some_and(|&(at, _)| at == offset)
        {
            return None;
        }
        let normalised = offset.checked_sub(u32::try_from(removed).ok()?)?;
        self.normalised
            .is_char_boundary(usize::try_from(normalised).ok()?)
            .then_some(normalised)
    }

    /// Whole source extent addressed by an LF presentation span.
    #[must_use]
    pub fn original_span(&self, span: Span) -> Option<Span> {
        (span.start() <= span.end()).then_some(())?;
        Some(Span::new(
            self.original_offset(span.start())?,
            self.original_offset(span.end())?,
        ))
    }

    /// LF presentation extent of a complete original span.
    #[must_use]
    pub fn normalised_span(&self, span: Span) -> Option<Span> {
        (span.start() <= span.end()).then_some(())?;
        Some(Span::new(
            self.normalised_offset(span.start())?,
            self.normalised_offset(span.end())?,
        ))
    }
}

impl SourceImage {
    /// Retain original native value bytes without decoding or rewriting them.
    #[must_use]
    pub fn native(bytes: impl Into<std::sync::Arc<[u8]>>) -> Self {
        Self::from_bytes(bytes, SourceChannel::NativeValue)
    }

    /// Retain original bytes with an explicitly selected input channel.
    #[must_use]
    pub fn from_bytes(bytes: impl Into<std::sync::Arc<[u8]>>, channel: SourceChannel) -> Self {
        use std::hash::{Hash, Hasher};
        let bytes = bytes.into();
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        bytes.hash(&mut hash);
        let geometry = std::sync::Arc::new(SourceImageGeometry {
            byte_hash: hash.finish(),
            line_index: u32::try_from(bytes.len())
                .is_ok()
                .then(|| LineIndex::from_bytes(&bytes)),
        });
        Self {
            bytes,
            channel,
            geometry,
        }
    }

    /// Retain Unicode document bytes with source-channel continuation rules.
    #[must_use]
    pub fn document(text: &str) -> Self {
        Self::from_bytes(text.as_bytes(), SourceChannel::Document)
    }

    /// Borrow the exact original script bytes.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Length of the original source in bytes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Whether the original source contains no bytes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// Share the exact immutable buffer with an origin or executable artifact.
    #[must_use]
    pub fn shared_bytes(&self) -> std::sync::Arc<[u8]> {
        self.bytes.clone()
    }

    /// Borrow the input-channel policy without inferring it from contents.
    #[must_use]
    pub const fn channel(&self) -> SourceChannel {
        self.channel
    }

    /// Project an unchanged Unicode view when the original bytes permit it.
    ///
    /// # Errors
    /// Returns the original decoding error for opaque byte scripts.
    pub fn try_text(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(&self.bytes)
    }

    /// Index this image in its original byte coordinates.
    #[must_use]
    pub fn source_map(&self) -> SourceMap<'_> {
        SourceMap::from_image(self)
    }
}

impl Default for SourceImage {
    fn default() -> Self {
        Self::document("")
    }
}
impl AsRef<[u8]> for SourceImage {
    fn as_ref(&self) -> &[u8] {
        self.bytes()
    }
}
impl std::ops::Deref for SourceImage {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        self.bytes()
    }
}
impl From<&str> for SourceImage {
    fn from(text: &str) -> Self {
        Self::document(text)
    }
}
impl From<String> for SourceImage {
    fn from(text: String) -> Self {
        Self::from_bytes(text.into_bytes(), SourceChannel::Document)
    }
}
impl From<std::sync::Arc<str>> for SourceImage {
    fn from(text: std::sync::Arc<str>) -> Self {
        Self::document(&text)
    }
}
impl PartialEq<str> for SourceImage {
    fn eq(&self, text: &str) -> bool {
        self.bytes() == text.as_bytes()
    }
}
impl PartialEq<&str> for SourceImage {
    fn eq(&self, text: &&str) -> bool {
        self == *text
    }
}
impl PartialEq<String> for SourceImage {
    fn eq(&self, text: &String) -> bool {
        self == text.as_str()
    }
}

/// A source buffer paired with its line index.
///
/// The primary lookup surface for anyone holding a [`Span`] and
/// wanting to know what text it covers or where it sits in (line,
/// character) space.
#[derive(Debug, Clone)]
pub struct SourceMap<'src> {
    source: &'src [u8],
    channel: SourceChannel,
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
            source: source.as_bytes(),
            channel: SourceChannel::Document,
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
            source: source.as_bytes(),
            channel: SourceChannel::Document,
            line_index,
            base_offset: 0,
            base_line: 0,
            base_col: 0,
        }
    }

    /// Borrow original bytes and channel with their already retained line index.
    /// The caller supplies an index for these exact bytes; no Unicode view is
    /// required and source positions remain byte based.
    #[must_use]
    pub fn from_bytes_with_line_index(
        source: &'src [u8],
        channel: SourceChannel,
        line_index: LineIndex,
    ) -> Self {
        Self {
            source,
            channel,
            line_index,
            base_offset: 0,
            base_line: 0,
            base_col: 0,
        }
    }

    /// Set sub-lexing base offsets. These are added to every
    /// `SourcePosition` returned by [`Self::position_at`] and
    /// [`Self::range_positions`] so positions resolved from a
    /// sub-lexed fragment are reported relative to the parent source.
    #[must_use]
    pub fn with_base(mut self, base_offset: u32, base_line: u32, base_col: u32) -> Self {
        self.base_offset = base_offset;
        self.base_line = base_line;
        self.base_col = base_col;
        self
    }

    /// The document offset this map's first byte sits at — the
    /// `base_offset` given to [`Self::with_base`], and `0` for a map over a
    /// whole document.
    ///
    /// A caller that holds spans in the *parent* span space (a sub-lex
    /// segmented at an offset) subtracts this to index [`Self::text`] /
    /// [`Self::token_text`], which take spans local to the buffer.
    #[must_use]
    pub const fn base_offset(&self) -> u32 {
        self.base_offset
    }

    /// Borrow an unchanged Unicode view of the underlying source buffer.
    ///
    /// # Panics
    /// Panics for opaque byte source; native consumers use `source_bytes` or
    /// `try_source` instead.
    #[must_use]
    pub fn source(&self) -> &'src str {
        self.try_source()
            .expect("Unicode source accessor requires original UTF-8 bytes")
    }

    /// Borrow the original bytes without decoding or materialisation.
    #[must_use]
    pub fn source_bytes(&self) -> &'src [u8] {
        self.source
    }

    /// Check whether the original source has a Unicode presentation.
    ///
    /// # Errors
    /// Returns the original UTF-8 decoding failure; it never repairs bytes.
    pub fn try_source(&self) -> Result<&'src str, std::str::Utf8Error> {
        std::str::from_utf8(self.source)
    }

    /// The input channel whose continuation rules apply to this source.
    #[must_use]
    pub const fn channel(&self) -> SourceChannel {
        self.channel
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
    /// Callers that want the "human-readable" content of a token
    /// should use [`Self::token_text`] instead.
    ///
    /// # Panics
    ///
    /// Panics if `span` is not a valid byte range in the source
    /// (either out of bounds or not on a UTF-8 character boundary).
    #[must_use]
    pub fn text(&self, span: Span) -> &'src str {
        std::str::from_utf8(self.bytes(span))
            .expect("Unicode source slice requires original UTF-8 bytes")
    }

    /// Return the "human-readable" text of a token — the same thing
    /// `Token.text` field contains.
    ///
    /// For most kinds this is identical to `self.text(tok.span)`.
    /// For `VAR` tokens, the leading `$` (and the `{` of a `${…}`
    /// braced form) is stripped so the result is the variable name
    /// alone. Every wrapper-style token's stripping rule lives here:
    /// this is the **one place** in the codebase that encodes the
    /// convention of "position range spans the full token, text field
    /// is the inner content".
    #[must_use]
    pub fn token_text(&self, tok: Token) -> &'src str {
        std::str::from_utf8(self.token_bytes(tok))
            .expect("Unicode token accessor requires original UTF-8 bytes")
    }

    /// Borrow a raw span from the original source, including delimiters.
    ///
    /// # Panics
    /// Panics if the span is outside this source buffer.
    #[must_use]
    pub fn bytes(&self, span: Span) -> &'src [u8] {
        &self.source[span.as_range()]
    }

    /// Borrow a token's original content using the shared delimiter rules.
    #[must_use]
    pub fn token_bytes(&self, tok: Token) -> &'src [u8] {
        token_bytes_in(self.source, tok)
    }

    /// Index a native string-value script without changing its bytes.
    #[must_use]
    pub fn from_bytes(source: &'src [u8]) -> Self {
        Self::from_bytes_with_channel(source, SourceChannel::NativeValue)
    }

    /// Index original bytes under an explicit input-channel policy.
    #[must_use]
    pub fn from_bytes_with_channel(source: &'src [u8], channel: SourceChannel) -> Self {
        Self {
            source,
            channel,
            line_index: LineIndex::from_bytes(source),
            base_offset: 0,
            base_line: 0,
            base_col: 0,
        }
    }

    /// Index an immutable source image with its original channel policy.
    /// The exact image owns its reusable derived line geometry; no grammar,
    /// context, source correspondence or execution authority is cached here.
    #[must_use]
    pub fn from_image(source: &'src SourceImage) -> Self {
        Self::from_bytes_with_line_index(
            source.bytes(),
            source.channel(),
            source
                .geometry
                .line_index
                .clone()
                .unwrap_or_else(|| LineIndex::from_bytes(source.bytes())),
        )
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
                ByteCol::new(self.base_col + raw.character.get())
            } else {
                raw.character
            },
            self.base_offset + raw.offset,
        )
    }

    /// Resolve a span to `(start, end)` positions, where `start` is
    /// the position of the first byte and `end` is the position of
    /// the **last** byte (inclusive), following the lexer's
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

/// [`SourceMap::token_text`] without the map: the inner text of `tok` read
/// straight from `source`, for a caller that holds the source but no line
/// index — the boundary grouper, which must not build one per call. The
/// stripping and empty-clamp rules live here and nowhere else; the method
/// above is this function plus the map's own buffer.
pub(crate) fn token_bytes_in(source: &[u8], tok: Token) -> &[u8] {
    let raw = &source[tok.span.as_range()];
    // Strip the lexer-computed prefix (`$`, `${`, `[`, `{`, `"`,
    // etc.) to get to the content.
    let stripped = &raw[tok.content_offset as usize..];
    // Kind-specific trailing-strip or empty-clamp rules.
    match tok.kind {
        TokenType::Var => {
            // `${}` degenerate: the span is extended by one byte to
            // cover the closing `}`, so the only remainder that is a
            // bare `}` is that empty-name case. A non-degenerate braced
            // name may legitimately *end* with a `}` — `${a{b}}` names
            // the variable `a{b}` (brace-nesting), `${a\}b}` names
            // `a\}b` — and the closing `}` is already excluded from the
            // span. So, like the `Cmd` / `Str` arms, clear only the
            // exact 1-character `}` remainder rather than stripping
            // unconditionally.
            if stripped == b"}" { b"" } else { stripped }
        }
        TokenType::Cmd => {
            // `[]` degenerate: span extended by one to cover
            // the `]`. Non-empty nested commands like
            // `[+ 1 [inner]]` also end with `]` as a
            // legitimate inner bracket, so we must NOT
            // unconditionally strip — check for the exact
            // 1-character `]` remainder instead.
            if stripped == b"]" { b"" } else { stripped }
        }
        TokenType::ExprSugar => {
            // `$()` retains the closer in its empty-content lexer span.
            if stripped == b")" { b"" } else { stripped }
        }
        TokenType::Str => {
            // `{}` degenerate: same shape as `[]`.
            if stripped == b"}" { b"" } else { stripped }
        }
        TokenType::Esc => {
            // Empty-content clamp for quoted sub-tokens.  When the quoted
            // scanner stops with zero content, the span is extended by one
            // byte over the terminator — the opening `"` extended over a
            // following `$` / `[` substitution introducer, or a mid-string
            // `$` / `[` fragment.  After stripping the opening delimiter
            // (via `content_offset`), a one-character remainder that is a
            // terminator (`"` / `$` / `[`) is that empty-body case and
            // clamps to `""`.
            //
            // The clamp fires only for genuine quoted/wrapper tokens: those
            // that stripped an opening delimiter (`content_offset != 0`) or
            // sit inside a quoted run (`in_quote`, e.g. a mid-string
            // introducer before `$var`, or the bare closing quote which
            // `parse_quoted` marks with `content_offset == 1`).  A *literal*
            // `"` / `$` / `[` in a bare word is emitted by `parse_esc` with
            // `content_offset == 0` and `in_quote == false`, so it is left
            // as its own text — `set x $a"` resolves the trailing `"`, not
            // `""`.
            if (tok.content_offset != 0 || tok.in_quote)
                && stripped.len() == 1
                && matches!(stripped.first(), Some(b'"' | b'$' | b'['))
            {
                b""
            } else {
                stripped
            }
        }
        _ => stripped,
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
        assert_eq!(
            map.position_at(0),
            SourcePosition::new(0, ByteCol::new(0), 0)
        );
        assert_eq!(
            map.position_at(4),
            SourcePosition::new(1, ByteCol::new(0), 4)
        );
    }

    #[test]
    fn range_positions_for_non_empty_span() {
        let map = SourceMap::new("abc def");
        // span covering "abc" — start at (0,0,0), end at (0,2,2) for 'c'
        let (start, end) = map.range_positions(Span::new(0, 3));
        assert_eq!(start, SourcePosition::new(0, ByteCol::new(0), 0));
        assert_eq!(end, SourcePosition::new(0, ByteCol::new(2), 2));
    }

    #[test]
    fn range_positions_for_empty_span_point_at_start() {
        let map = SourceMap::new("abc");
        let (start, end) = map.range_positions(Span::empty(3));
        assert_eq!(start, SourcePosition::new(0, ByteCol::new(3), 3));
        assert_eq!(end, SourcePosition::new(0, ByteCol::new(3), 3));
    }

    #[test]
    fn range_positions_across_newline() {
        let map = SourceMap::new("ab\ncd");
        // Span covering just the '\n' at offset 2
        let (start, end) = map.range_positions(Span::new(2, 3));
        assert_eq!(start, SourcePosition::new(0, ByteCol::new(2), 2));
        assert_eq!(end, SourcePosition::new(0, ByteCol::new(2), 2));
        // Span covering "cd" on line 1
        let (start, end) = map.range_positions(Span::new(3, 5));
        assert_eq!(start, SourcePosition::new(1, ByteCol::new(0), 3));
        assert_eq!(end, SourcePosition::new(1, ByteCol::new(1), 4));
    }

    #[test]
    fn token_text_literal_trailing_quote_is_not_empty_clamped() {
        // `set x $a"` — the trailing `"` lexes as a 1-byte ESC with
        // content_offset == 0 (no opening quote was stripped), so its text is
        // the literal `"`, not an empty quoted body.
        let src = "set x $a\"";
        let toks = crate::Lexer::new(src).tokenise_all().unwrap();
        let map = SourceMap::new(src);
        let quote = toks
            .iter()
            .find(|t| t.kind == TokenType::Esc && map.text(t.span) == "\"")
            .expect("trailing literal quote token");
        assert_eq!(quote.content_offset, 0);
        assert_eq!(map.token_text(*quote), "\"");
    }

    #[test]
    fn token_text_empty_quoted_word_still_clamps() {
        // A genuine empty quoted word `""` has content_offset == 1 (the
        // opening quote is stripped); the empty-clamp must still fire.
        let src = "\"\"";
        let toks = crate::Lexer::new(src).tokenise_all().unwrap();
        let map = SourceMap::new(src);
        let word = toks
            .iter()
            .find(|t| t.kind == TokenType::Esc)
            .expect("quoted word token");
        assert_ne!(word.content_offset, 0);
        assert_eq!(map.token_text(*word), "");
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

#[cfg(test)]
mod native_byte_tests {
    use super::{SourceChannel, SourceImage, SourceMap};
    use crate::{Lexer, LexerConfig, Span, TokenType, first_parse_cut_bytes, group_commands_bytes};

    #[test]
    fn original_bytes_survive_token_content_and_nested_parse_cut() {
        let image = SourceImage::native(b"set \xff \x80; list [set \xfe {a}b]".as_slice());
        let config = LexerConfig::for_dialect("tcl9.0");
        let map = image.source_map();
        let tokens = Lexer::with_source_image(&image, config)
            .tokenise_all()
            .unwrap();
        assert!(image.try_text().is_err());
        let contents: Vec<_> = tokens
            .iter()
            .filter(|t| t.kind != TokenType::Sep && t.kind != TokenType::Eol)
            .map(|t| map.token_bytes(*t))
            .collect();
        assert_eq!(&contents[..3], &[b"set".as_slice(), b"\xff", b"\x80"]);
        assert_eq!(map.bytes(Span::new(4, 5)), b"\xff");
        let commands = group_commands_bytes(&tokens, image.bytes(), config);
        assert_eq!(commands.len(), 2);
        let cut = first_parse_cut_bytes(image.bytes(), config).unwrap();
        assert_eq!(cut.command, 1);
        assert_eq!(cut.message, "extra characters after close-brace");
    }

    #[test]
    fn native_value_and_document_continuations_have_distinct_source_receipts() {
        let bytes = b"set a A\\\r\nset b B";
        let native = SourceImage::native(bytes.as_slice());
        let document = SourceImage::document(std::str::from_utf8(bytes).unwrap());
        assert_eq!(native.bytes(), document.bytes());
        assert_ne!(native, document);
        assert_eq!(native.channel(), SourceChannel::NativeValue);
        let config = LexerConfig::default();
        let n = Lexer::with_source_image(&native, config)
            .tokenise_all()
            .unwrap();
        let d = Lexer::with_source_image(&document, config)
            .tokenise_all()
            .unwrap();
        assert_eq!(group_commands_bytes(&n, bytes, config).len(), 2);
        assert_eq!(group_commands_bytes(&d, bytes, config).len(), 1);
    }

    #[test]
    fn opaque_names_use_the_selected_variable_grammar_and_byte_positions() {
        let bytes = b"set \xff V\nlist $\xff ${\xff}";
        for name in ["tcl8.4", "tcl9.0", "jimtcl"] {
            let config = LexerConfig::for_dialect(name);
            let map = SourceMap::from_bytes(bytes).with_base(40, 7, 3);
            let tokens = Lexer::with_source_map(map.clone(), config)
                .tokenise_all()
                .unwrap();
            let vars: Vec<_> = tokens
                .iter()
                .filter(|t| t.kind == TokenType::Var)
                .map(|t| map.token_bytes(*t))
                .collect();
            assert_eq!(vars.last(), Some(&b"\xff".as_slice()));
            assert_eq!(map.position_at(8).line, 8);
            assert_eq!(map.position_at(8).offset, 48);
        }
    }
}

#[cfg(test)]
mod source_image_hash_tests {
    use super::*;
    use std::hash::{Hash, Hasher};

    fn hash(image: &SourceImage) -> u64 {
        let mut state = std::collections::hash_map::DefaultHasher::new();
        image.hash(&mut state);
        state.finish()
    }

    #[test]
    fn source_image_cached_hash_preserves_full_equality_order_and_channel() {
        // Implementation contract: naming.source.cached-image-hash (docs/design/analysis/name-resolution-proofs/cached-image-hash.md).
        let native = SourceImage::native(b"p\xff\0tail".as_slice());
        let equal = SourceImage::from_bytes(native.shared_bytes(), SourceChannel::NativeValue);
        assert_eq!(native, equal);
        assert_eq!(hash(&native), hash(&equal));
        assert_eq!(native.cmp(&equal), std::cmp::Ordering::Equal);
        let document = SourceImage::from_bytes(native.shared_bytes(), SourceChannel::Document);
        assert_ne!(native, document);
        assert_ne!(hash(&native), hash(&document));
        assert!(native < document);
        assert!(SourceImage::native(b"a".as_slice()) < SourceImage::native(b"b".as_slice()));
    }

    #[test]
    fn source_image_digest_collision_does_not_merge_map_or_order_identity() {
        // Implementation contract: naming.source.cached-image-hash (docs/design/analysis/name-resolution-proofs/cached-image-hash.md).
        let first = SourceImage::native(b"first\xff".as_slice());
        let mut second = SourceImage::native(b"second\0".as_slice());
        second.geometry = std::sync::Arc::new(super::SourceImageGeometry {
            byte_hash: first.geometry.byte_hash,
            line_index: second.geometry.line_index.clone(),
        });
        assert_eq!(hash(&first), hash(&second));
        assert_ne!(first, second);
        assert_ne!(first.cmp(&second), std::cmp::Ordering::Equal);
        let mut values = std::collections::HashMap::new();
        values.insert(first.clone(), 1);
        values.insert(second.clone(), 2);
        assert_eq!(values.len(), 2);
        assert_eq!(values.get(&first), Some(&1));
        assert_eq!(values.get(&second), Some(&2));
    }
}

#[cfg(test)]
mod immutable_image_geometry_tests {
    use super::{SourceChannel, SourceImage, SourceMap};
    use std::hash::{Hash, Hasher};

    fn hash(image: &SourceImage) -> u64 {
        let mut state = std::collections::hash_map::DefaultHasher::new();
        image.hash(&mut state);
        state.finish()
    }

    #[test]
    fn original_image_positions_reuse_immutable_geometry_without_changing_identity() {
        // naming.source.original-immutable-image-geometry
        // docs/design/analysis/name-resolution-proofs/original-immutable-image-geometry.md
        let text = "first\r\né\n😀\0last";
        let original = SourceImage::document(text);
        let clone = original.clone();
        let independent = SourceImage::document(text);
        let before = hash(&original);
        assert!(original.geometry.line_index.is_some());
        let direct = SourceMap::new(text);
        for offset in 0..=u32::try_from(text.len()).unwrap() {
            assert_eq!(
                original.source_map().position_at(offset),
                direct.position_at(offset)
            );
            assert_eq!(
                clone.source_map().position_at(offset),
                direct.position_at(offset)
            );
        }
        assert!(std::sync::Arc::ptr_eq(&original.geometry, &clone.geometry));
        assert!(!std::sync::Arc::ptr_eq(
            &original.geometry,
            &independent.geometry
        ));
        assert!(independent.geometry.line_index.is_some());
        assert_eq!(original, independent);
        assert_eq!(original.cmp(&independent), std::cmp::Ordering::Equal);
        assert_eq!(before, hash(&original));
        assert_eq!(before, hash(&independent));
        let native = SourceImage::native(text.as_bytes());
        assert_ne!(original, native);
        assert_eq!(native.source_map().channel(), SourceChannel::NativeValue);
        let changed = SourceImage::document("first\nlast");
        assert_ne!(original, changed);
        assert_ne!(
            original.source_map().position_at(8),
            changed.source_map().position_at(8)
        );
    }

    #[test]
    fn original_native_byte_positions_need_no_unicode_or_document_channel() {
        // naming.source.original-immutable-image-geometry
        // docs/design/analysis/name-resolution-proofs/original-immutable-image-geometry.md
        let bytes = b"\xff\r\n\xed\xa0\x80\0\nx";
        let original = SourceImage::native(bytes.as_slice());
        assert!(original.try_text().is_err());
        let direct = SourceMap::from_bytes_with_channel(bytes, SourceChannel::NativeValue);
        for offset in 0..=u32::try_from(bytes.len()).unwrap() {
            assert_eq!(
                original.source_map().position_at(offset),
                direct.position_at(offset)
            );
        }
        assert_eq!(original.source_map().source_bytes(), bytes);
    }
}

#[cfg(test)]
mod document_line_ending_projection_tests {
    use super::*;

    #[test]
    fn document_projection_round_trips_whole_unicode_words_across_mixed_endings() {
        // naming.source.original-document-line-ending-projection
        // docs/design/analysis/name-resolution-proofs/source-original-document-line-ending-projection.md
        // Source geometry only; no native value, command or source-entry authority.
        let original = "set café {α\r\nβ}\rset γ 3\n";
        let projection =
            DocumentLineEndingProjection::new(SourceImage::document(original)).unwrap();
        assert_eq!(projection.text(), "set café {α\nβ}\nset γ 3\n");
        assert_eq!(projection.original().bytes(), original.as_bytes());
        let start = u32::try_from(projection.text().find('{').unwrap()).unwrap();
        let end = u32::try_from(projection.text().find('}').unwrap() + 1).unwrap();
        let span = Span::new(start, end);
        let source_span = projection.original_span(span).unwrap();
        assert_eq!(&original[source_span.as_range()], "{α\r\nβ}");
        assert_eq!(projection.normalised_span(source_span), Some(span));
        for offset in 0..=u32::try_from(projection.text().len()).unwrap() {
            if projection.text().is_char_boundary(offset as usize) {
                let source = projection.original_offset(offset).unwrap();
                assert_eq!(projection.normalised_offset(source), Some(offset));
            } else {
                assert!(projection.original_offset(offset).is_none());
            }
        }
    }

    #[test]
    fn document_projection_refuses_collapsed_boundary_native_values_and_non_source_extents() {
        // naming.source.original-document-line-ending-projection
        // docs/design/analysis/name-resolution-proofs/source-original-document-line-ending-projection.md
        // The missing boundary is an actual mapping refusal, not a shortened word.
        let original = "é\r\nnext\r\n";
        let projection =
            DocumentLineEndingProjection::new(SourceImage::document(original)).unwrap();
        assert!(projection.normalised_offset(3).is_none());
        assert!(projection.normalised_span(Span::new(0, 3)).is_none());
        assert!(projection.original_offset(1).is_none());
        assert!(projection.normalised_offset(1).is_none());
        assert!(projection.original_offset(100).is_none());
        assert!(projection.normalised_offset(100).is_none());
        assert!(
            DocumentLineEndingProjection::new(SourceImage::native(original.as_bytes())).is_none()
        );
        assert!(
            DocumentLineEndingProjection::new(SourceImage::from_bytes(
                [0xff].as_slice(),
                SourceChannel::Document,
            ))
            .is_none()
        );
        for source in ["", "set value 1\n", "set value 1\r", "set value 1\r\n"] {
            let projection =
                DocumentLineEndingProjection::new(SourceImage::document(source)).unwrap();
            let whole = Span::new(0, u32::try_from(source.len()).unwrap());
            assert_eq!(
                projection.original_span(projection.normalised_span(whole).unwrap()),
                Some(whole)
            );
        }
    }
}
