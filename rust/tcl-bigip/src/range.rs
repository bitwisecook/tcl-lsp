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

//! Source-position types for BIG-IP objects.
//!
//! A [`Position`] is `(line, character, offset)` where `character` is a
//! 0-based UTF-16 code-unit column (LSP convention) and `offset` is the
//! Unicode scalar index in the canonical model. [`Range`] pairs an inclusive start/end.

use tcl_lexer::LineIndex;

/// A position in source text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Position {
    /// 0-based line number.
    pub line: u32,
    /// 0-based column in UTF-16 code units (LSP spec).
    pub character: u32,
    /// Unicode scalar index in the canonical model.
    pub offset: u32,
}

/// A span in source text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Range {
    /// Start position (inclusive).
    pub start: Position,
    /// End position (inclusive — the last covered byte).
    pub end: Position,
}

impl Range {
    /// A zero-length range at `(0, 0, 0)`.
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

    /// Build a [`Range`] from inclusive source byte offsets: empty source
    /// yields the zero range, and out-of-bounds offsets clamp to the last byte.
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

/// Resolve a byte offset to a [`Position`] via the line index (UTF-16 column).
fn position_at(source: &str, line_index: &LineIndex, offset: usize) -> Position {
    let off = u32::try_from(offset).unwrap_or(u32::MAX);
    let sp = line_index.position_at_utf16(off, source);
    // The `offset` field is a code-point index, not a byte offset — count
    // code points up to the byte offset so non-ASCII sources are handled
    // correctly.
    let codepoint_offset = source
        .get(..offset.min(source.len()))
        .map_or(0, |s| s.chars().count());
    Position {
        line: sp.line,
        character: sp.character.get(),
        offset: u32::try_from(codepoint_offset).unwrap_or(u32::MAX),
    }
}

/// Look up original SCF byte extents by a model object's opening location.
/// The model's serialised scalar offset and inclusive range endpoint cannot
/// be used as UTF-8 byte indexes. Build once for an original config image.
#[derive(Debug, Default)]
pub struct StanzaSourceIndex {
    by_open: std::collections::BTreeMap<(u32, u32), crate::parser::BlockSpan>,
}

impl StanzaSourceIndex {
    /// Index every outer stanza by its opening brace on this source image.
    #[must_use]
    pub fn new(source: &str) -> Self {
        let lines = LineIndex::new(source);
        let by_open = crate::parser::block_spans(source)
            .filter_map(|span| {
                let open = u32::try_from(span.header.end).ok()?;
                let position = lines.position_at_utf16(open, source);
                Some(((position.line, position.character.get()), span))
            })
            .collect();
        Self { by_open }
    }

    /// The original header, inner body, and observed closing delimiter.
    #[must_use]
    pub fn at(&self, opening: Position) -> Option<&crate::parser::BlockSpan> {
        self.by_open.get(&(opening.line, opening.character))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_offsets_resolves_positions_and_clamps() {
        // "abc\ndef": a0 b1 c2 \n3 d4 e5 f6.
        let source = "abc\ndef";
        let li = LineIndex::new(source);

        let first = Range::from_offsets(source, &li, 0, 2);
        assert_eq!((first.start.line, first.start.character), (0, 0));
        assert_eq!((first.end.line, first.end.character), (0, 2));

        let second = Range::from_offsets(source, &li, 4, 6);
        assert_eq!((second.start.line, second.start.character), (1, 0));
        assert_eq!((second.end.line, second.end.character), (1, 2));

        // Out-of-bounds offsets clamp to the last byte.
        let clamped = Range::from_offsets(source, &li, 100, 200);
        assert_eq!(clamped.start.offset, 6);
        assert_eq!(clamped.end.offset, 6);

        // An inverted range (end < start) collapses to the start.
        let inverted = Range::from_offsets(source, &li, 5, 1);
        assert_eq!(inverted.start.offset, inverted.end.offset);

        // Empty source → the zero range.
        let empty_li = LineIndex::new("");
        let zero = Range::from_offsets("", &empty_li, 5, 9);
        assert_eq!(
            (zero.start.line, zero.start.character, zero.start.offset),
            (0, 0, 0)
        );
    }
}
