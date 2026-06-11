//! Source-position types mirroring the Python `shared.diagnostic.Range`
//! / `shared.tokens.SourcePosition` so reconstructed BIG-IP objects
//! carry byte-identical spans.
//!
//! A [`Position`] is `(line, character, offset)` where `character` is a
//! 0-based UTF-16 code-unit column (LSP convention) and `offset` is the
//! byte offset into the source. [`Range`] pairs an inclusive start/end
//! exactly as the Python parser's `DocumentBuffer.range_from_offsets`
//! produces them.

use tcl_lexer::LineIndex;

/// A position in source text. Mirrors Python `SourcePosition`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Position {
    /// 0-based line number.
    pub line: u32,
    /// 0-based column in UTF-16 code units (LSP spec).
    pub character: u32,
    /// Byte offset into the source string.
    pub offset: u32,
}

/// A span in source text. Mirrors Python `shared.diagnostic.Range`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Range {
    /// Start position (inclusive).
    pub start: Position,
    /// End position (inclusive — the last covered byte, matching the
    /// Python `range_from_offsets(start, end_inclusive)` contract).
    pub end: Position,
}

impl Range {
    /// A zero-length range at `(0, 0, 0)`, mirroring `Range.zero()`.
    #[must_use]
    pub const fn zero() -> Self {
        let pos = Position {
            line: 0,
            character: 0,
            offset: 0,
        };
        Self {
            start: pos,
            end: pos,
        }
    }

    /// Build a [`Range`] from inclusive source byte offsets, mirroring
    /// `DocumentBuffer.range_from_offsets`: empty source yields the zero
    /// range, and out-of-bounds offsets clamp to the last byte.
    #[must_use]
    pub fn from_offsets(
        source: &str,
        line_index: &LineIndex,
        start: usize,
        end_inclusive: usize,
    ) -> Self {
        if source.is_empty() {
            return Self::zero();
        }
        let max_end = source.len() - 1;
        let safe_start = start.min(max_end);
        let mut safe_end = end_inclusive.min(max_end);
        if safe_end < safe_start {
            safe_end = safe_start;
        }
        Self {
            start: position_at(source, line_index, safe_start),
            end: position_at(source, line_index, safe_end),
        }
    }
}

/// Resolve a byte offset to a [`Position`] via the line index, matching
/// Python `offset_to_position` (UTF-16 column).
fn position_at(source: &str, line_index: &LineIndex, offset: usize) -> Position {
    let off = u32::try_from(offset).unwrap_or(u32::MAX);
    let sp = line_index.position_at_utf16(off, source);
    // Python `SourcePosition.offset` is a code-point index (Python str
    // indexing), not a byte offset — count code points up to the byte
    // offset so non-ASCII sources still match the Python parser.
    let codepoint_offset = source
        .get(..offset.min(source.len()))
        .map_or(0, |s| s.chars().count());
    Position {
        line: sp.line,
        character: sp.character,
        offset: u32::try_from(codepoint_offset).unwrap_or(u32::MAX),
    }
}
