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
    /// Build a `LineIndex` by scanning `source` once for line breaks.
    ///
    /// A line break is `\n` **only**: the byte immediately after each
    /// `\n` begins the next line. A CRLF therefore breaks on its LF,
    /// and a *lone* CR is **not** a line break — in Tcl a bare `\r` is
    /// horizontal whitespace (a `Sep`), not an end-of-line.
    ///
    /// This matches every line index on the Python side
    /// (`compiler/parsing/lexer.py::_build_line_starts` and
    /// `shared/source_map.py`, both `\n`-only) and the red CST overlay's
    /// own `build_line_starts`. Keeping the rule identical across the
    /// lexer and the CST is what makes their token positions agree for
    /// old-Mac (bare-CR) input — the position-equivalence invariant
    /// restored upstream in #537 (SYNC-JUN08), where a CR-counting index
    /// reported a token after a lone CR one line below its own end (a
    /// backwards range).
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
        let mut starts = Vec::with_capacity(bytes.len() / 32 + 1);
        starts.push(0_u32);
        for (i, &b) in bytes.iter().enumerate() {
            if b == b'\n' {
                starts.push(u32::try_from(i + 1).expect("offset fits u32"));
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

    /// Inverse of [`Self::position_at_utf16`]: resolve an LSP
    /// `(line, character)` position — where `character` is a count of
    /// UTF-16 code units from the line start — to a byte offset into
    /// `source`.
    ///
    /// Used to apply LSP ranged edits (incremental document sync). Out-of
    /// -range inputs clamp: a `line` past the end maps to `source.len()`,
    /// and a `character` past the line's content maps to the line's
    /// terminating newline (or `source.len()` for the last line). A
    /// `character` landing mid-code-point rounds up to the next char
    /// boundary so the result is always a valid byte index.
    #[must_use]
    pub fn offset_at_utf16(&self, line: u32, character: u32, source: &str) -> u32 {
        let line_count = self.line_starts.len();
        let line_idx = line as usize;
        if line_idx >= line_count {
            return u32::try_from(source.len()).expect("source length fits u32");
        }
        let line_start = self.line_starts[line_idx] as usize;
        // The line's content ends at the next line's start (or EOF for
        // the last line), excluding the trailing line terminator so a
        // `character` at/after the content end maps before the newline.
        let raw_end = if line_idx + 1 < line_count {
            self.line_starts[line_idx + 1] as usize
        } else {
            source.len()
        };
        let bytes = source.as_bytes();
        let mut content_end = raw_end;
        if content_end > line_start && bytes[content_end - 1] == b'\n' {
            content_end -= 1;
        }
        if content_end > line_start && bytes[content_end - 1] == b'\r' {
            content_end -= 1;
        }
        let line_text = &source[line_start..content_end];
        // Walk chars accumulating UTF-16 units until we reach `character`.
        let mut utf16 = 0u32;
        for (byte_off, ch) in line_text.char_indices() {
            if utf16 >= character {
                return u32::try_from(line_start + byte_off).expect("offset fits u32");
            }
            utf16 += u32::try_from(ch.len_utf16()).expect("len_utf16 fits u32");
        }
        // `character` is at or past the line's content end.
        u32::try_from(content_end).expect("offset fits u32")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offset_at_utf16_roundtrips_and_clamps() {
        let src = "ab\ncde\nf";
        let idx = LineIndex::new(src);
        // (line, char) -> byte offset.
        assert_eq!(idx.offset_at_utf16(0, 0, src), 0); // 'a'
        assert_eq!(idx.offset_at_utf16(0, 2, src), 2); // end of line 0 (the '\n')
        assert_eq!(idx.offset_at_utf16(1, 0, src), 3); // 'c'
        assert_eq!(idx.offset_at_utf16(1, 3, src), 6); // end of line 1 (the '\n')
        assert_eq!(idx.offset_at_utf16(2, 1, src), 8); // end of last line (EOF)
                                                       // Clamps: char past content -> line content end; line past EOF -> len.
        assert_eq!(idx.offset_at_utf16(0, 99, src), 2);
        assert_eq!(
            idx.offset_at_utf16(9, 0, src),
            u32::try_from(src.len()).unwrap()
        );
        // Round-trips with position_at_utf16 at char boundaries.
        for off in [0u32, 1, 2, 3, 5, 6, 7, 8] {
            let p = idx.position_at_utf16(off, src);
            assert_eq!(
                idx.offset_at_utf16(p.line, p.character, src),
                off,
                "off {off}"
            );
        }
    }

    #[test]
    fn offset_at_utf16_counts_surrogate_pairs() {
        // 'a' + U+1F600 (2 UTF-16 units, 4 bytes) + 'b'.
        let src = "a\u{1F600}b";
        let idx = LineIndex::new(src);
        assert_eq!(idx.offset_at_utf16(0, 0, src), 0); // 'a'
        assert_eq!(idx.offset_at_utf16(0, 1, src), 1); // emoji start
        assert_eq!(idx.offset_at_utf16(0, 3, src), 5); // 'b' (after 2 units)
    }

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
    fn bare_cr_is_not_a_line_break() {
        // A lone CR is horizontal whitespace in Tcl, not an EOL — and
        // post-#537 main counts only `\n` in its line index, so a bare
        // `\r` does not start a new line.
        let idx = LineIndex::new("abc\rdef");
        assert_eq!(idx.line_count(), 1);
        // The lone CR at offset 3 is itself on line 0, column 3.
        assert_eq!(idx.position_at(3), SourcePosition::new(0, 3, 3));
        // ``def`` (offset 4) stays on line 0, column 4 — not line 1.
        assert_eq!(idx.position_at(4), SourcePosition::new(0, 4, 4));
    }

    #[test]
    fn mixed_line_endings() {
        // Only ``\n`` (incl. the LF of a CRLF) breaks a line; a bare
        // ``\r`` does not.
        let src = "a\nb\r\nc\rd";
        let idx = LineIndex::new(src);
        assert_eq!(idx.line_count(), 3);
        // Lines: "a" (0..2), "b\r" (2..5), "c\rd" (5..8).
        assert_eq!(idx.line_start(0), 0);
        assert_eq!(idx.line_start(1), 2);
        assert_eq!(idx.line_start(2), 5);
        // The bare ``\r`` at offset 6 stays on line 2, not a new line.
        assert_eq!(idx.position_at(6), SourcePosition::new(2, 1, 6));
        assert_eq!(idx.position_at(7), SourcePosition::new(2, 2, 7));
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
