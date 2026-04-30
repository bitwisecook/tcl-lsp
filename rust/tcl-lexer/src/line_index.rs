//! Shared line-start index for offset → (line, character) lookups.
//!
//! Equivalent to `DocumentBuffer.line_starts` on the Python side. The
//! index is computed once per document and reused across every lexer
//! invocation, future sub-lexings (command substitutions, expressions),
//! and any Rust consumer that needs to translate between byte offsets
//! and `SourcePosition`s. Callers who already hold one should build
//! their [`Lexer`] via `Lexer::with_line_index(...)` rather than
//! `Lexer::new(...)` so the source is scanned for newlines only once.
//!
//! Matches the Python contracts documented in
//! `docs/kcs/kcs-core-lsp-shared-utility-contracts.md`: O(log n) offset
//! lookup via binary search on a sorted start-offset array.
//!
//! [`Lexer`]: crate::lexer::Lexer

use crate::tokens::SourcePosition;

/// Sorted index of line-start byte offsets for a source string.
///
/// `line_starts[i]` is the byte offset of the first character on the
/// 0-based `i`-th line. `line_starts[0]` is always `0`, and
/// `line_starts[i]` for `i > 0` is the byte immediately after the `i`-th
/// `\n`. Offsets past the last newline resolve to the final line.
///
/// Cheap to clone: the backing storage is a `Box<[u32]>`, so cloning
/// is one allocation plus a byte copy. Future chunks may switch to
/// `Arc<[u32]>` if genuine sharing turns up in profiles.
#[derive(Debug, Clone)]
pub struct LineIndex {
    line_starts: Box<[u32]>,
}

impl LineIndex {
    /// Build a `LineIndex` by scanning `source` once for line
    /// terminators. Recognises `\n`, `\r\n`, and bare `\r` —
    /// the three Tcl-relevant line endings (the bare-`\r` case
    /// matches the Python lexer's incremental counter, which
    /// also advances on `\r` inside backslash continuations).
    ///
    /// # Panics
    ///
    /// Panics if `source.len()` does not fit in a `u32`. The lexer
    /// budgets 4 GiB for any single source, well above any realistic
    /// Tcl/iRules input; the limit is enforced here as the last
    /// checked conversion between `usize` and `u32`.
    #[must_use]
    pub fn new(source: &str) -> Self {
        assert!(
            u32::try_from(source.len()).is_ok(),
            "source longer than 4 GiB cannot be indexed",
        );
        let bytes = source.as_bytes();
        let len = bytes.len();
        let mut starts = Vec::with_capacity(len / 32 + 1);
        starts.push(0_u32);
        let mut i = 0;
        while i < len {
            match bytes[i] {
                b'\r' => {
                    // ``\r\n`` counts as one line break; bare ``\r``
                    // also counts as one. The next-line offset is
                    // immediately after the terminator.
                    let consumed = if i + 1 < len && bytes[i + 1] == b'\n' {
                        2
                    } else {
                        1
                    };
                    starts.push(u32::try_from(i + consumed).expect("offset fits u32"));
                    i += consumed;
                }
                b'\n' => {
                    starts.push(u32::try_from(i + 1).expect("offset fits u32"));
                    i += 1;
                }
                _ => i += 1,
            }
        }
        Self {
            line_starts: starts.into_boxed_slice(),
        }
    }

    /// Number of lines in the index. Always ≥ 1 for any `LineIndex`
    /// built from a non-`None` source (even an empty string contains
    /// one line).
    #[must_use]
    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    /// Byte offset of the first character on `line`. Panics if
    /// `line >= self.line_count()`.
    #[must_use]
    pub fn line_start(&self, line: u32) -> u32 {
        self.line_starts[line as usize]
    }

    /// Resolve a byte offset into the source to a [`SourcePosition`].
    ///
    /// Uses binary search in O(log n).
    ///
    /// `character` is the **byte** offset from the start of the line,
    /// not a UTF-16 code unit count. This matches the Python lexer's
    /// actual behaviour (`col = offset - line_start`), which is exact
    /// for ASCII input and drifts for supplementary-plane characters.
    /// Use [`Self::position_at_utf16`] for an LSP-compliant UTF-16
    /// column.
    #[must_use]
    pub fn position_at(&self, offset: u32) -> SourcePosition {
        let line_idx = self
            .line_starts
            .partition_point(|&start| start <= offset)
            .saturating_sub(1);
        let line_start = self.line_starts[line_idx];
        SourcePosition::new(
            u32::try_from(line_idx).expect("line count fits u32"),
            offset - line_start,
            offset,
        )
    }

    /// LSP-compliant variant of [`Self::position_at`] that returns
    /// a `character` column counted in UTF-16 code units (per the
    /// LSP specification's "Position" type, which defines
    /// `character` as zero-based offsets into a UTF-16-encoded
    /// line).
    ///
    /// Requires `source` so we can count UTF-16 code units within
    /// the line up to *offset*. ASCII / BMP characters cost one
    /// code unit; supplementary-plane characters (`U+10000`+) cost
    /// two (a surrogate pair). For ASCII input the answer is
    /// identical to [`Self::position_at`].
    ///
    /// # Panics
    ///
    /// Panics if *offset* falls inside a UTF-8 multi-byte sequence
    /// (the resulting position would be unrepresentable). Callers
    /// should align *offset* to a `char` boundary before calling.
    #[must_use]
    pub fn position_at_utf16(&self, offset: u32, source: &str) -> SourcePosition {
        let line_idx = self
            .line_starts
            .partition_point(|&start| start <= offset)
            .saturating_sub(1);
        let line_start = self.line_starts[line_idx];
        let prefix_start = line_start as usize;
        let prefix_end = offset as usize;
        // Count UTF-16 code units in the line slice up to *offset*.
        // ``str::encode_utf16`` is the canonical conversion; the
        // alternative (summing ``ch.len_utf16()``) is equivalent
        // but allocates per char.
        let line_prefix = &source[prefix_start..prefix_end];
        let col_utf16 = line_prefix.encode_utf16().count();
        SourcePosition::new(
            u32::try_from(line_idx).expect("line count fits u32"),
            u32::try_from(col_utf16).expect("UTF-16 column fits u32"),
            offset,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_source_has_one_line() {
        let idx = LineIndex::new("");
        assert_eq!(idx.line_count(), 1);
        assert_eq!(idx.line_start(0), 0);
        assert_eq!(idx.position_at(0), SourcePosition::new(0, 0, 0));
    }

    #[test]
    fn single_line_no_newline() {
        let idx = LineIndex::new("hello");
        assert_eq!(idx.line_count(), 1);
        assert_eq!(idx.position_at(0), SourcePosition::new(0, 0, 0));
        assert_eq!(idx.position_at(3), SourcePosition::new(0, 3, 3));
    }

    #[test]
    fn two_lines() {
        let idx = LineIndex::new("abc\ndef");
        assert_eq!(idx.line_count(), 2);
        assert_eq!(idx.line_start(0), 0);
        assert_eq!(idx.line_start(1), 4);
        // 'a' at offset 0 → (0, 0, 0)
        assert_eq!(idx.position_at(0), SourcePosition::new(0, 0, 0));
        // 'c' at offset 2 → (0, 2, 2)
        assert_eq!(idx.position_at(2), SourcePosition::new(0, 2, 2));
        // '\n' at offset 3 → (0, 3, 3) — still on line 0
        assert_eq!(idx.position_at(3), SourcePosition::new(0, 3, 3));
        // 'd' at offset 4 → (1, 0, 4) — first char of line 1
        assert_eq!(idx.position_at(4), SourcePosition::new(1, 0, 4));
        // 'f' at offset 6 → (1, 2, 6)
        assert_eq!(idx.position_at(6), SourcePosition::new(1, 2, 6));
    }

    #[test]
    fn many_lines() {
        let idx = LineIndex::new("a\nb\nc\nd\n");
        assert_eq!(idx.line_count(), 5);
        assert_eq!(idx.position_at(0), SourcePosition::new(0, 0, 0));
        assert_eq!(idx.position_at(2), SourcePosition::new(1, 0, 2));
        assert_eq!(idx.position_at(4), SourcePosition::new(2, 0, 4));
        assert_eq!(idx.position_at(6), SourcePosition::new(3, 0, 6));
        // Past the final '\n' — empty 5th line at offset 8.
        assert_eq!(idx.position_at(8), SourcePosition::new(4, 0, 8));
    }

    #[test]
    fn consecutive_newlines_yield_empty_lines() {
        let idx = LineIndex::new("a\n\nb");
        // line 0: "a"
        // line 1: ""
        // line 2: "b"
        assert_eq!(idx.line_count(), 3);
        assert_eq!(idx.position_at(0), SourcePosition::new(0, 0, 0));
        assert_eq!(idx.position_at(2), SourcePosition::new(1, 0, 2));
        assert_eq!(idx.position_at(3), SourcePosition::new(2, 0, 3));
    }

    #[test]
    fn line_index_is_cloneable() {
        let idx = LineIndex::new("x\ny");
        let other = idx.clone();
        assert_eq!(other.line_count(), idx.line_count());
        assert_eq!(other.position_at(2), idx.position_at(2));
    }

    #[test]
    fn crlf_counts_as_single_line_break() {
        let idx = LineIndex::new("abc\r\ndef");
        assert_eq!(idx.line_count(), 2);
        // First line spans bytes 0..5 (``abc\r\n``); second line
        // starts at offset 5.
        assert_eq!(idx.line_start(1), 5);
        assert_eq!(idx.position_at(5), SourcePosition::new(1, 0, 5));
        // Within line 1, offset 7 is column 2 (``f``).
        assert_eq!(idx.position_at(7), SourcePosition::new(1, 2, 7));
    }

    #[test]
    fn bare_cr_counts_as_line_break() {
        let idx = LineIndex::new("abc\rdef");
        assert_eq!(idx.line_count(), 2);
        assert_eq!(idx.line_start(1), 4);
        assert_eq!(idx.position_at(4), SourcePosition::new(1, 0, 4));
    }

    #[test]
    fn mixed_line_endings() {
        // ``\n``, ``\r\n``, bare ``\r`` all count once.
        let src = "a\nb\r\nc\rd";
        let idx = LineIndex::new(src);
        assert_eq!(idx.line_count(), 4);
        // Lines: "a" (0..2), "b" (2..5), "c" (5..7), "d" (7..8).
        assert_eq!(idx.line_start(0), 0);
        assert_eq!(idx.line_start(1), 2);
        assert_eq!(idx.line_start(2), 5);
        assert_eq!(idx.line_start(3), 7);
    }

    #[test]
    fn utf16_position_matches_byte_position_for_ascii() {
        let src = "hello\nworld";
        let idx = LineIndex::new(src);
        // ASCII — UTF-16 column equals byte column.
        for offset in [0u32, 1, 2, 5, 6, 7, 11] {
            let byte_pos = idx.position_at(offset);
            let utf16_pos = idx.position_at_utf16(offset, src);
            assert_eq!(byte_pos.line, utf16_pos.line);
            assert_eq!(byte_pos.character, utf16_pos.character);
        }
    }

    #[test]
    fn utf16_position_counts_one_unit_per_bmp_character() {
        // ``é`` is U+00E9 — 2 bytes in UTF-8, 1 code unit in UTF-16.
        let src = "aé\nb";
        let idx = LineIndex::new(src);
        // ``é`` ends at byte offset 3 (``a`` = 1 + ``é`` = 2).
        // Byte column at offset 3 = 3; UTF-16 column = 2.
        assert_eq!(idx.position_at(3).character, 3);
        assert_eq!(idx.position_at_utf16(3, src).character, 2);
    }

    #[test]
    fn utf16_position_counts_two_units_for_supplementary_plane() {
        // ``😀`` is U+1F600 — 4 bytes in UTF-8, 2 code units in
        // UTF-16 (a surrogate pair).
        let src = "a😀b";
        let idx = LineIndex::new(src);
        // ``😀`` ends at byte offset 5 (1 + 4).
        // Byte column at offset 5 = 5; UTF-16 column = 3 (1 ASCII
        // + 2 surrogates).
        assert_eq!(idx.position_at(5).character, 5);
        assert_eq!(idx.position_at_utf16(5, src).character, 3);
    }

    #[test]
    fn utf16_position_handles_offset_at_line_boundary() {
        let src = "héllo\nwörld";
        let idx = LineIndex::new(src);
        // ``é`` is 2 bytes, so ``hello\n`` line spans 0..7. UTF-16
        // chars on line 0: h, é, l, l, o = 5 code units.
        // The terminating ``\n`` is at byte offset 6.
        assert_eq!(idx.position_at_utf16(6, src).character, 5);
        // First char of line 1 (``w``) at byte offset 7.
        assert_eq!(idx.position_at_utf16(7, src), SourcePosition::new(1, 0, 7));
    }
}
