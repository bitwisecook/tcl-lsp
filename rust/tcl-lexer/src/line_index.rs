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

//! Shared line-start index for offset → (line, character) lookups.
//!
//! A sorted array of line-start byte offsets. The
//! index is computed once per document and reused across every lexer
//! invocation, future sub-lexings (command substitutions, expressions),
//! and any Rust consumer that needs to translate between byte offsets
//! and `SourcePosition`s. Callers who already hold one should build
//! their [`Lexer`] via `Lexer::with_line_index(...)` rather than
//! `Lexer::new(...)` so the source is scanned for newlines only once.
//!
//! Matches the contracts documented in
//! `docs/design/contracts/shared-utility-contracts-rust.md`: O(log n) offset
//! lookup via binary search on a sorted start-offset array.
//!
//! [`Lexer`]: crate::lexer::Lexer

use crate::tokens::{ByteCol, SourcePosition, Utf16Col, Utf16Position};
use std::borrow::Cow;

/// Rewrite every *lone* `\r` (a `\r` not immediately followed by `\n`) to `\n`,
/// leaving CRLF pairs — and everything else — untouched.
///
/// This is the in-memory equivalent of what a real `tclsh` does to a script
/// file before its parser ever sees it: `Tcl_OpenFileChannel` reads scripts
/// with `-translation auto`, whose `TranslateInputEOL` turns every `\r` into
/// `\n` (swallowing a following `\n`). A lone `\r` in a file on disk is
/// therefore a genuine **command terminator** in Tcl, not the horizontal
/// whitespace the lexer would otherwise make of it — treating it as whitespace
/// invents `E003`/`W210` diagnostics and hides real ones.
///
/// Two properties make this safe to apply to *document* text on the way into
/// analysis, and they are what let the `\n`-only [`LineIndex::new`] the lexer
/// and CST share keep agreeing with the client:
///
/// * It is **byte-length preserving** — one `\r` byte becomes one `\n` byte —
///   so every span, offset and UTF-16 column computed against the normalised
///   text is equally valid against the raw text.
/// * `LineIndex::new(normalise_lone_cr(t))` is byte-identical to
///   `LineIndex::new_lsp(t)` for every `t`, so the lexer/CST line model and the
///   LSP client's line model coincide once the text is normalised.
///
/// CRLF and LF documents contain no lone `\r` at all, so they come back
/// borrowed and bit-identical; only an old-Mac / mixed document allocates.
///
/// This applies to **document text entering static analysis**, never to Tcl
/// string values at run time — `eval`ing a string containing `\r` keeps Tcl's
/// own semantics.
#[must_use]
pub fn normalise_lone_cr(source: &str) -> Cow<'_, str> {
    if !LineIndex::contains_bare_cr(source) {
        return Cow::Borrowed(source);
    }
    let bytes = source.as_bytes();
    let mut out = String::with_capacity(source.len());
    let mut last = 0usize;
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'\r' && bytes.get(i + 1) != Some(&b'\n') {
            out.push_str(&source[last..i]);
            out.push('\n');
            last = i + 1;
        }
    }
    out.push_str(&source[last..]);
    Cow::Owned(out)
}

/// Sorted index of line-start byte offsets for a source string.
///
/// `line_starts[i]` is the byte offset of the first character on the
/// 0-based `i`-th line. `line_starts[0]` is always `0`, and
/// `line_starts[i]` for `i > 0` is the byte immediately after the `i`-th
/// `\n`. Offsets past the last newline resolve to the final line.
///
/// **Free to clone**: the backing storage is an `Arc<[u32]>`, so a clone is a
/// reference-count bump — no allocation, no byte copy — and every clone shares
/// one allocation.
///
/// It was a `Box<[u32]>` (one allocation plus a byte copy per clone) until the
/// sharing this anticipated turned up in profiles: `tcl-lsp-server`'s
/// `read_document` hands every in-flight LSP request its own snapshot of the
/// open document, so `R` concurrent requests against an `L`-line document each
/// copied `4L` bytes of line index (issue #1184). `filetypes.tcl` alone is
/// 85,040 lines — 332 KiB of index — per request.
///
/// Nothing here is shared *mutably*: every mutation ([`Self::apply_edit`])
/// builds a fresh `Vec` and replaces the handle wholesale, so an outstanding
/// clone keeps the pre-edit index it was given rather than observing a
/// half-applied edit. That is what makes an in-flight request safe to finish
/// against the revision it started on.
#[derive(Debug, Clone)]
pub struct LineIndex {
    line_starts: std::sync::Arc<[u32]>,
}

impl LineIndex {
    /// Build a `LineIndex` by scanning `source` once for line breaks.
    ///
    /// A line break is `\n` **only**: the byte immediately after each
    /// `\n` begins the next line. A CRLF therefore breaks on its LF,
    /// and a *lone* CR is **not** a line break — in Tcl a bare `\r` is
    /// horizontal whitespace (a `Sep`), not an end-of-line.
    ///
    /// This `\n`-only rule matches the red CST overlay's own
    /// `build_line_starts`. Keeping the rule identical across the
    /// lexer and the CST is what makes their token positions agree for
    /// old-Mac (bare-CR) input — the position-equivalence invariant
    /// restored upstream in #537, where a CR-counting index
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
            line_starts: starts.into(),
        }
    }

    /// Build a `LineIndex` whose line breaks follow the **LSP** end-of-line
    /// definition: `\n`, `\r\n`, *and a lone `\r`* each start a new line.
    ///
    /// This is deliberately distinct from [`Self::new`], whose `\n`-only rule
    /// is required for lexer/CST token-position equivalence (a bare `\r` is
    /// Tcl horizontal whitespace, not an EOL). The LSP protocol, however,
    /// defines a document's `(line, character)` coordinates over all three
    /// terminators, so a language server's model of an **open document** must
    /// count them the same way the client does — otherwise incremental
    /// `didChange` edits on an old-Mac (bare-`\r`) file resolve to the wrong
    /// byte offset and corrupt the server's shadow buffer, and diagnostic
    /// ranges land on the wrong line. Use this constructor for document-sync
    /// and for lifting analyser spans to client ranges; use [`Self::new`] for
    /// the lexer/CST.
    ///
    /// # Panics
    ///
    /// Panics if `source.len()` does not fit in a `u32` (the 4 GiB budget).
    #[must_use]
    pub fn new_lsp(source: &str) -> Self {
        assert!(
            u32::try_from(source.len()).is_ok(),
            "source longer than 4 GiB cannot be indexed",
        );
        let bytes = source.as_bytes();
        let mut starts = Vec::with_capacity(bytes.len() / 32 + 1);
        starts.push(0_u32);
        let mut i = 0;
        while i < bytes.len() {
            match bytes[i] {
                b'\n' => starts.push(u32::try_from(i + 1).expect("offset fits u32")),
                b'\r' => {
                    // CRLF is a single break at the LF: skip the LF so the pair
                    // yields exactly one new line-start (byte after the `\n`),
                    // agreeing with `new`. A lone `\r` breaks at the byte after
                    // the `\r`.
                    if bytes.get(i + 1) == Some(&b'\n') {
                        starts.push(u32::try_from(i + 2).expect("offset fits u32"));
                        i += 1;
                    } else {
                        starts.push(u32::try_from(i + 1).expect("offset fits u32"));
                    }
                }
                _ => {}
            }
            i += 1;
        }
        Self {
            line_starts: starts.into(),
        }
    }

    /// Patch the index **in place** for an edit that replaces source bytes
    /// `[start, old_end)` with `new_text`, instead of rebuilding from the whole
    /// edited document (SRV-INCREMENTAL Task 1).  After the call the index equals
    /// `LineIndex::new(edited_source)` for the document the same splice produces —
    /// proven byte-identical over a random-edit fuzz corpus
    /// (`apply_edit_matches_rebuild_under_fuzz`).
    ///
    /// The three regions of the new index: line-starts at or before `start` are
    /// unchanged; line-starts inside the replaced span `(start, old_end]` are
    /// dropped; line-starts after `old_end` shift by the byte delta; and one new
    /// line-start is inserted per `\n` in `new_text`.  O(lines after the edit +
    /// newlines inserted), not O(document).
    ///
    /// `start` / `old_end` are byte offsets into the *pre-edit* source and must be
    /// on `char` boundaries and satisfy `start <= old_end`.
    ///
    /// # Panics
    ///
    /// Panics if the resulting offsets do not fit in a `u32` (the 4 GiB source
    /// budget [`Self::new`] enforces).
    pub fn apply_edit(&mut self, start: u32, old_end: u32, new_text: &str) {
        debug_assert!(start <= old_end, "edit start must not exceed old_end");
        let delta = i64::from(u32::try_from(new_text.len()).expect("insertion fits u32"))
            - i64::from(old_end - start);
        let inserted = new_text.bytes().filter(|&b| b == b'\n').count();
        let mut next: Vec<u32> = Vec::with_capacity(self.line_starts.len() + inserted);

        // Region 1 — line-starts at or before the edit: unchanged.
        let mut i = 0;
        while i < self.line_starts.len() && self.line_starts[i] <= start {
            next.push(self.line_starts[i]);
            i += 1;
        }
        // Region 2 — one new line-start just after each `\n` in `new_text`.
        for (j, _) in new_text.bytes().enumerate().filter(|&(_, b)| b == b'\n') {
            next.push(start + u32::try_from(j + 1).expect("offset fits u32"));
        }
        // Drop the line-starts that fell inside the replaced span `(start, old_end]`.
        while i < self.line_starts.len() && self.line_starts[i] <= old_end {
            i += 1;
        }
        // Region 3 — line-starts after the edit: shifted by the byte delta.
        while i < self.line_starts.len() {
            let shifted = i64::from(self.line_starts[i]) + delta;
            next.push(u32::try_from(shifted).expect("shifted offset fits u32"));
            i += 1;
        }
        self.line_starts = next.into();
    }

    /// Whether `source` contains a *lone* `\r` — a `\r` not immediately
    /// followed by `\n`.
    ///
    /// This is the exact ambiguity that separates [`Self::new_lsp`] from
    /// [`Self::new`]: a CRLF pair is a single break at the `\n` in *both*
    /// models, so it is never itself the source of a divergence, but a bare
    /// `\r` is a break under [`Self::new_lsp`] and plain whitespace under
    /// [`Self::new`]. When `source` has none, `new_lsp(source)` and
    /// `new(source)` therefore agree line-start-for-line-start, which is
    /// exactly the condition [`Self::apply_edit`] needs to stay sound when
    /// patching an `new_lsp`-built index (see its fuzz-equivalence test,
    /// which never puts a `\r` in its edit alphabet, and the LSP server's
    /// `did_change`, which uses this to decide whether the incremental patch
    /// remains valid for a given document).
    #[must_use]
    pub fn contains_bare_cr(source: &str) -> bool {
        let bytes = source.as_bytes();
        bytes
            .iter()
            .enumerate()
            .any(|(i, &b)| b == b'\r' && bytes.get(i + 1) != Some(&b'\n'))
    }

    /// Whether `self` and `other` are clones sharing **one** backing
    /// allocation, rather than two indices that merely hold equal offsets.
    ///
    /// This is the sharing contract itself, not a debugging aid: consumers that
    /// hand one index to many concurrent readers ([`LineIndex`]'s own doc
    /// comment, and `tcl-lsp-server`'s document snapshots — issue #1184) depend
    /// on a clone costing a reference-count bump, and a silent regression to a
    /// copy-per-clone is invisible to any equality-based assertion. Comparing
    /// identity is the only way to pin it.
    #[must_use]
    pub fn shares_storage_with(&self, other: &Self) -> bool {
        std::sync::Arc::ptr_eq(&self.line_starts, &other.line_starts)
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
    /// not a UTF-16 code unit count. The column is computed as
    /// `col = offset - line_start`, which is exact
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
            ByteCol::new(offset - line_start),
            offset,
        )
    }

    /// Zero-based line number containing byte `offset`.
    ///
    /// The line index is independent of column encoding (it counts line
    /// breaks, not characters), so this is the right accessor whenever a
    /// caller needs only the line — folding ranges, body-span line
    /// extents — and avoids the choice between the byte
    /// [`Self::position_at`] and the UTF-16 [`Self::position_at_utf16`]
    /// columns, neither of which the caller reads.
    #[must_use]
    pub fn line_at(&self, offset: u32) -> u32 {
        let line_idx = self
            .line_starts
            .partition_point(|&start| start <= offset)
            .saturating_sub(1);
        u32::try_from(line_idx).expect("line count fits u32")
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
    /// An *offset* past the end of *source* is clamped to the end (the
    /// returned `character` is the line's full UTF-16 length); the
    /// `SourcePosition.offset` field keeps the raw *offset*.
    ///
    /// An *offset* that falls inside a UTF-8 multi-byte sequence is snapped
    /// *down* to the enclosing `char` boundary for the column count (the
    /// `offset` field still keeps the raw value).  This makes the function
    /// total: a *stale* span — e.g. a workspace-index span applied against a
    /// document that was edited since (cross-document navigation) — degrades
    /// to a nearby valid position instead of panicking and unwinding the
    /// server (issue 175).
    #[must_use]
    pub fn position_at_utf16(&self, offset: u32, source: &str) -> Utf16Position {
        let line_idx = self
            .line_starts
            .partition_point(|&start| start <= offset)
            .saturating_sub(1);
        let line_start = self.line_starts[line_idx];
        let prefix_start = line_start as usize;
        // Clamp an out-of-range offset to the end of the source rather than
        // panicking on the slice.  A diagnostic span may legitimately end one
        // or two bytes past EOF (e.g. a final unbraced word with no trailing
        // newline), and the server lifts every analyser span through here on a
        // worker thread — a panic there silently drops the whole document's
        // diagnostics.'s
        // `min(offset, len(source))` guard.
        let mut prefix_end = (offset as usize).min(source.len());
        // Snap a mid-multibyte-sequence offset down to the enclosing char
        // boundary so the slice below never panics on a stale / misaligned
        // offset (issue 175). `prefix_start` is a line start (always a char
        // boundary), so the loop terminates.
        while prefix_end > prefix_start && !source.is_char_boundary(prefix_end) {
            prefix_end -= 1;
        }
        // Count UTF-16 code units in the line slice up to *offset*.
        // ``str::encode_utf16`` is the canonical conversion; the
        // alternative (summing ``ch.len_utf16()``) is equivalent
        // but allocates per char.
        let line_prefix = &source[prefix_start..prefix_end];
        let col_utf16 = line_prefix.encode_utf16().count();
        Utf16Position::new(
            u32::try_from(line_idx).expect("line count fits u32"),
            Utf16Col::new(u32::try_from(col_utf16).expect("UTF-16 column fits u32")),
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
    pub fn offset_at_utf16(&self, line: u32, character: Utf16Col, source: &str) -> u32 {
        let character = character.get();
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

    /// SRV-INCREMENTAL Task 1 gate: the in-place [`LineIndex::apply_edit`] patch
    /// must be byte-identical to a full rebuild over a random-edit corpus.  ASCII
    /// edits only, so every byte offset is a `char` boundary; the alphabet is
    /// newline-heavy to exercise insert/delete of line-starts in every region.
    #[test]
    fn apply_edit_matches_rebuild_under_fuzz() {
        let starts = |idx: &LineIndex| {
            (0..u32::try_from(idx.line_count()).unwrap())
                .map(|l| idx.line_start(l))
                .collect::<Vec<_>>()
        };
        // Deterministic xorshift PRNG — reproducible without a dev-dependency.
        let mut rng = 0x9E37_79B9_7F4A_7C15_u64;
        let mut next = move || {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            rng
        };
        let mut source = String::from("alpha\nbeta\n\ngamma\ndelta\n");
        let mut idx = LineIndex::new(&source);
        for step in 0..5000 {
            let len = source.len();
            // Reduce the u64 PRNG output modulo a small bound in u64, then
            // convert — the result is < the bound, so it always fits usize.
            let pick = |bound: usize, r: u64| usize::try_from(r % bound as u64).unwrap();
            let a = pick(len + 1, next());
            let b = a + pick(len - a + 1, next());
            // Build an ASCII replacement, newline-heavy (~1 in 3) and 0..6 long.
            let ins_len = pick(7, next());
            let mut ins = String::with_capacity(ins_len);
            for _ in 0..ins_len {
                ins.push(if next() % 3 == 0 { '\n' } else { 'x' });
            }
            idx.apply_edit(u32::try_from(a).unwrap(), u32::try_from(b).unwrap(), &ins);
            source.replace_range(a..b, &ins);
            assert_eq!(
                starts(&idx),
                starts(&LineIndex::new(&source)),
                "patched != rebuilt at step {step}: edit [{a},{b}) += {ins:?}\nsource={source:?}"
            );
        }
    }

    #[test]
    fn apply_edit_handles_pure_insert_delete_and_boundaries() {
        // Pure insertion of a newline.
        let mut idx = LineIndex::new("abcdef");
        idx.apply_edit(3, 3, "\n");
        assert_eq!(idx.line_count(), 2);
        assert_eq!(idx.line_start(1), 4);
        // Pure deletion spanning a newline collapses two lines into one.
        let mut idx = LineIndex::new("ab\ncd\nef");
        idx.apply_edit(2, 6, ""); // delete "\ncd\n"
        assert_eq!(idx.line_count(), 1);
        // Replace exactly at a line boundary (start == an existing line-start).
        let mut idx = LineIndex::new("a\nb\nc");
        idx.apply_edit(2, 2, "Z\n");
        assert!(starts_eq(&idx, "a\nZ\nb\nc"));
    }

    fn starts_eq(idx: &LineIndex, expected_src: &str) -> bool {
        let want = LineIndex::new(expected_src);
        idx.line_count() == want.line_count()
            && (0..u32::try_from(idx.line_count()).unwrap())
                .all(|l| idx.line_start(l) == want.line_start(l))
    }

    #[test]
    fn position_at_utf16_clamps_offset_past_eof() {
        // A diagnostic span may end one or two bytes past EOF (e.g. a final
        // unbraced `$body` word with no trailing newline).  The conversion
        // must clamp to the line's UTF-16 length rather than panicking on the
        // out-of-range slice — a panic on the server's diagnostic worker
        // silently drops the whole document's diagnostics.
        let src = "foreach name $nameList $body";
        let idx = LineIndex::new(src);
        let len = u32::try_from(src.len()).unwrap();
        let pos = idx.position_at_utf16(len + 2, src);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character.get(), len); // clamped to the line's full length
        assert_eq!(pos.offset, len + 2); // raw offset preserved
    }

    #[test]
    fn position_at_utf16_snaps_mid_multibyte_offset() {
        // A stale span (e.g. a workspace-index offset applied against text
        // edited since) can land inside a multi-byte char. The conversion must
        // snap down to the enclosing char boundary, never panic (issue 175).
        let src = "x=€y"; // '€' is 3 bytes at offsets 2..5
        let idx = LineIndex::new_lsp(src);
        // Offset 3 and 4 are *inside* the euro sign.
        for off in [3u32, 4] {
            let pos = idx.position_at_utf16(off, src);
            assert_eq!(pos.line, 0);
            // Snapped down to the boundary before '€' → 2 UTF-16 units (`x=`).
            assert_eq!(pos.character.get(), 2, "offset {off}");
            assert_eq!(pos.offset, off, "raw offset preserved for {off}");
        }
        // The boundary *after* '€' (offset 5) counts the euro's one UTF-16 unit.
        let after = idx.position_at_utf16(5, src);
        assert_eq!(after.character.get(), 3);
    }

    #[test]
    fn line_at_matches_position_at_line_for_multibyte() {
        // 'á' is two bytes; the line number must not depend on column
        // encoding, so `line_at` agrees with both column variants' `.line`.
        let src = "á\nbc\nd";
        let idx = LineIndex::new(src);
        for off in [0u32, 2, 3, 5, 6] {
            assert_eq!(
                idx.line_at(off),
                idx.position_at(off).line,
                "byte off {off}"
            );
            assert_eq!(
                idx.line_at(off),
                idx.position_at_utf16(off, src).line,
                "utf16 off {off}"
            );
        }
    }

    #[test]
    fn offset_at_utf16_roundtrips_and_clamps() {
        let src = "ab\ncde\nf";
        let idx = LineIndex::new(src);
        // (line, char) -> byte offset.
        assert_eq!(idx.offset_at_utf16(0, Utf16Col::new(0), src), 0); // 'a'
        assert_eq!(idx.offset_at_utf16(0, Utf16Col::new(2), src), 2); // end of line 0 (the '\n')
        assert_eq!(idx.offset_at_utf16(1, Utf16Col::new(0), src), 3); // 'c'
        assert_eq!(idx.offset_at_utf16(1, Utf16Col::new(3), src), 6); // end of line 1 (the '\n')
        assert_eq!(idx.offset_at_utf16(2, Utf16Col::new(1), src), 8); // end of last line (EOF)
        // Clamps: char past content -> line content end; line past EOF -> len.
        assert_eq!(idx.offset_at_utf16(0, Utf16Col::new(99), src), 2);
        assert_eq!(
            idx.offset_at_utf16(9, Utf16Col::new(0), src),
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
        assert_eq!(idx.offset_at_utf16(0, Utf16Col::new(0), src), 0); // 'a'
        assert_eq!(idx.offset_at_utf16(0, Utf16Col::new(1), src), 1); // emoji start
        assert_eq!(idx.offset_at_utf16(0, Utf16Col::new(3), src), 5); // 'b' (after 2 units)
    }

    #[test]
    fn empty_source_has_one_line() {
        let idx = LineIndex::new("");
        assert_eq!(idx.line_count(), 1);
        assert_eq!(idx.line_start(0), 0);
        assert_eq!(
            idx.position_at(0),
            SourcePosition::new(0, ByteCol::new(0), 0)
        );
    }

    #[test]
    fn single_line_no_newline() {
        let idx = LineIndex::new("hello");
        assert_eq!(idx.line_count(), 1);
        assert_eq!(
            idx.position_at(0),
            SourcePosition::new(0, ByteCol::new(0), 0)
        );
        assert_eq!(
            idx.position_at(3),
            SourcePosition::new(0, ByteCol::new(3), 3)
        );
    }

    #[test]
    fn two_lines() {
        let idx = LineIndex::new("abc\ndef");
        assert_eq!(idx.line_count(), 2);
        assert_eq!(idx.line_start(0), 0);
        assert_eq!(idx.line_start(1), 4);
        // 'a' at offset 0 → (0, 0, 0)
        assert_eq!(
            idx.position_at(0),
            SourcePosition::new(0, ByteCol::new(0), 0)
        );
        // 'c' at offset 2 → (0, 2, 2)
        assert_eq!(
            idx.position_at(2),
            SourcePosition::new(0, ByteCol::new(2), 2)
        );
        // '\n' at offset 3 → (0, 3, 3) — still on line 0
        assert_eq!(
            idx.position_at(3),
            SourcePosition::new(0, ByteCol::new(3), 3)
        );
        // 'd' at offset 4 → (1, 0, 4) — first char of line 1
        assert_eq!(
            idx.position_at(4),
            SourcePosition::new(1, ByteCol::new(0), 4)
        );
        // 'f' at offset 6 → (1, 2, 6)
        assert_eq!(
            idx.position_at(6),
            SourcePosition::new(1, ByteCol::new(2), 6)
        );
    }

    #[test]
    fn many_lines() {
        let idx = LineIndex::new("a\nb\nc\nd\n");
        assert_eq!(idx.line_count(), 5);
        assert_eq!(
            idx.position_at(0),
            SourcePosition::new(0, ByteCol::new(0), 0)
        );
        assert_eq!(
            idx.position_at(2),
            SourcePosition::new(1, ByteCol::new(0), 2)
        );
        assert_eq!(
            idx.position_at(4),
            SourcePosition::new(2, ByteCol::new(0), 4)
        );
        assert_eq!(
            idx.position_at(6),
            SourcePosition::new(3, ByteCol::new(0), 6)
        );
        // Past the final '\n' — empty 5th line at offset 8.
        assert_eq!(
            idx.position_at(8),
            SourcePosition::new(4, ByteCol::new(0), 8)
        );
    }

    #[test]
    fn consecutive_newlines_yield_empty_lines() {
        let idx = LineIndex::new("a\n\nb");
        // line 0: "a"
        // line 1: ""
        // line 2: "b"
        assert_eq!(idx.line_count(), 3);
        assert_eq!(
            idx.position_at(0),
            SourcePosition::new(0, ByteCol::new(0), 0)
        );
        assert_eq!(
            idx.position_at(2),
            SourcePosition::new(1, ByteCol::new(0), 2)
        );
        assert_eq!(
            idx.position_at(3),
            SourcePosition::new(2, ByteCol::new(0), 3)
        );
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
        assert_eq!(
            idx.position_at(5),
            SourcePosition::new(1, ByteCol::new(0), 5)
        );
        // Within line 1, offset 7 is column 2 (``f``).
        assert_eq!(
            idx.position_at(7),
            SourcePosition::new(1, ByteCol::new(2), 7)
        );
    }

    #[test]
    fn bare_cr_is_not_a_line_break() {
        // A lone CR is horizontal whitespace in Tcl, not an EOL — and
        // post-#537 main counts only `\n` in its line index, so a bare
        // `\r` does not start a new line.
        let idx = LineIndex::new("abc\rdef");
        assert_eq!(idx.line_count(), 1);
        // The lone CR at offset 3 is itself on line 0, column 3.
        assert_eq!(
            idx.position_at(3),
            SourcePosition::new(0, ByteCol::new(3), 3)
        );
        // ``def`` (offset 4) stays on line 0, column 4 — not line 1.
        assert_eq!(
            idx.position_at(4),
            SourcePosition::new(0, ByteCol::new(4), 4)
        );
    }

    #[test]
    fn contains_bare_cr_distinguishes_lone_cr_from_crlf() {
        assert!(!LineIndex::contains_bare_cr("abc\ndef"));
        assert!(!LineIndex::contains_bare_cr("abc\r\ndef\r\n"));
        assert!(!LineIndex::contains_bare_cr(""));
        assert!(LineIndex::contains_bare_cr("abc\rdef"));
        assert!(LineIndex::contains_bare_cr("abc\r\ndef\r"));
        // A trailing `\r` with nothing after it is bare.
        assert!(LineIndex::contains_bare_cr("abc\r"));
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
        assert_eq!(
            idx.position_at(6),
            SourcePosition::new(2, ByteCol::new(1), 6)
        );
        assert_eq!(
            idx.position_at(7),
            SourcePosition::new(2, ByteCol::new(2), 7)
        );
    }

    #[test]
    fn lsp_index_counts_lone_cr_as_line_break() {
        // For LSP document sync a bare `\r` starts a new line
        // (the client models it that way), unlike the `\n`-only `new`.
        let src = "a\rb\nc";
        let idx = LineIndex::new_lsp(src);
        assert_eq!(idx.line_count(), 3);
        assert_eq!(idx.line_start(0), 0); // "a\r"
        assert_eq!(idx.line_start(1), 2); // "b\n"
        assert_eq!(idx.line_start(2), 4); // "c"
        // The `\n`-only index disagrees (2 lines) — the two models are distinct.
        assert_eq!(LineIndex::new(src).line_count(), 2);
    }

    #[test]
    fn lsp_index_crlf_is_single_break() {
        // A CRLF is one break (at the LF), identical to the `\n`-only model.
        let src = "a\r\nb\r\nc";
        let lsp = LineIndex::new_lsp(src);
        let lf = LineIndex::new(src);
        assert_eq!(lsp.line_count(), 3);
        for l in 0..3 {
            assert_eq!(lsp.line_start(l), lf.line_start(l), "line {l}");
        }
    }

    #[test]
    fn lsp_offset_at_utf16_resolves_bare_cr_lines() {
        // The sync path resolves a client `(line, character)` to a byte offset.
        // On a bare-`\r` file the LSP index must map line 1 char 0 to `b`
        // (offset 2), not to the `\n`-only model's line 1 (`c`, offset 4).
        let src = "a\rb\nc";
        let idx = LineIndex::new_lsp(src);
        assert_eq!(idx.offset_at_utf16(0, Utf16Col::new(0), src), 0); // 'a'
        assert_eq!(idx.offset_at_utf16(1, Utf16Col::new(0), src), 2); // 'b'
        assert_eq!(idx.offset_at_utf16(2, Utf16Col::new(0), src), 4); // 'c'
        // Round-trip position_at_utf16 → offset_at_utf16 at char boundaries.
        for off in [0u32, 2, 3, 4, 5] {
            let p = idx.position_at_utf16(off, src);
            assert_eq!(
                idx.offset_at_utf16(p.line, p.character, src),
                off,
                "off {off}"
            );
        }
    }

    #[test]
    fn normalise_lone_cr_leaves_lf_and_crlf_documents_untouched() {
        for src in ["", "a\nb\n", "a\r\nb\r\n", "no breaks at all"] {
            let out = normalise_lone_cr(src);
            assert!(
                matches!(out, Cow::Borrowed(_)),
                "{src:?} should not allocate"
            );
            assert_eq!(out, src);
        }
    }

    #[test]
    fn normalise_lone_cr_rewrites_only_bare_cr() {
        assert_eq!(normalise_lone_cr("a\rb"), "a\nb");
        assert_eq!(normalise_lone_cr("a\r\nb\rc\nd"), "a\r\nb\nc\nd");
        // A trailing bare `\r` is still a terminator.
        assert_eq!(normalise_lone_cr("a\r"), "a\n");
        // `\r\r\n`: the first `\r` is bare, the second pairs with the `\n`.
        assert_eq!(normalise_lone_cr("a\r\r\nb"), "a\n\r\nb");
    }

    /// The property the whole LSP normalisation boundary rests on: the
    /// `\n`-only lexer/CST index over the normalised text is byte-identical to
    /// the LSP index over the raw text, and the rewrite never changes the byte
    /// length. Fuzzed over a `\r`/`\n`-heavy alphabet.
    #[test]
    fn normalise_lone_cr_reconciles_the_two_line_models_under_fuzz() {
        // `\r` twice so bare CRs and CRLF pairs both turn up often.
        const ALPHABET: [char; 6] = ['\r', '\n', '\r', 'x', ' ', '#'];
        let starts = |idx: &LineIndex| {
            (0..u32::try_from(idx.line_count()).unwrap())
                .map(|l| idx.line_start(l))
                .collect::<Vec<_>>()
        };
        let mut rng = 0x2545_F491_4F6C_DD1D_u64;
        let mut next = move || {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            rng
        };
        for case in 0..20_000 {
            let len = usize::try_from(next() % 24).unwrap();
            let raw: String = (0..len)
                .map(|_| ALPHABET[usize::try_from(next() % 6).unwrap()])
                .collect();
            let normalised = normalise_lone_cr(&raw);
            assert_eq!(
                normalised.len(),
                raw.len(),
                "case {case}: length changed for {raw:?}"
            );
            assert!(
                !LineIndex::contains_bare_cr(&normalised),
                "case {case}: bare CR survived in {normalised:?}"
            );
            assert_eq!(
                starts(&LineIndex::new(&normalised)),
                starts(&LineIndex::new_lsp(&raw)),
                "case {case}: line models disagree for {raw:?}"
            );
        }
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
            assert_eq!(byte_pos.character.get(), utf16_pos.character.get());
        }
    }

    #[test]
    fn utf16_position_counts_one_unit_per_bmp_character() {
        // ``é`` is U+00E9 — 2 bytes in UTF-8, 1 code unit in UTF-16.
        let src = "aé\nb";
        let idx = LineIndex::new(src);
        // ``é`` ends at byte offset 3 (``a`` = 1 + ``é`` = 2).
        // Byte column at offset 3 = 3; UTF-16 column = 2.
        assert_eq!(idx.position_at(3).character.get(), 3);
        assert_eq!(idx.position_at_utf16(3, src).character.get(), 2);
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
        assert_eq!(idx.position_at(5).character.get(), 5);
        assert_eq!(idx.position_at_utf16(5, src).character.get(), 3);
    }

    #[test]
    fn utf16_position_handles_offset_at_line_boundary() {
        let src = "héllo\nwörld";
        let idx = LineIndex::new(src);
        // ``é`` is 2 bytes, so ``hello\n`` line spans 0..7. UTF-16
        // chars on line 0: h, é, l, l, o = 5 code units.
        // The terminating ``\n`` is at byte offset 6.
        assert_eq!(idx.position_at_utf16(6, src).character.get(), 5);
        // First char of line 1 (``w``) at byte offset 7.
        assert_eq!(
            idx.position_at_utf16(7, src),
            Utf16Position::new(1, Utf16Col::new(0), 7)
        );
    }
    #[test]
    fn clone_shares_one_backing_allocation() {
        // The sharing contract (issue #1184): a clone is a reference-count
        // bump, not a copy, so many concurrent readers of one document's index
        // hold one allocation between them rather than one each.
        let idx = LineIndex::new_lsp("a\nb\nc\n");
        let snapshots: Vec<LineIndex> = (0..32).map(|_| idx.clone()).collect();
        for snap in &snapshots {
            assert!(
                idx.shares_storage_with(snap),
                "clone must share the original's storage"
            );
        }
        // Two indices built independently are equal line-for-line but must NOT
        // report sharing — otherwise the assertion above would be vacuous.
        let independent = LineIndex::new_lsp("a\nb\nc\n");
        assert_eq!(independent.line_count(), idx.line_count());
        assert!(!idx.shares_storage_with(&independent));
    }

    #[test]
    fn apply_edit_leaves_outstanding_clones_on_the_old_revision() {
        // Build-and-swap, not mutate-in-place: an index handed to an in-flight
        // reader before an edit must keep describing the text that reader was
        // given, so it can never resolve a position against the wrong offsets.
        let mut idx = LineIndex::new_lsp("one\ntwo\nthree\n");
        let snapshot = idx.clone();
        let before: Vec<u32> = (0..snapshot.line_count())
            .map(|l| snapshot.line_start(u32::try_from(l).expect("small")))
            .collect();

        // Insert two lines at the very start of the document.
        idx.apply_edit(0, 0, "zero\nhalf\n");

        assert!(
            !idx.shares_storage_with(&snapshot),
            "an edit must install new storage, never write through the shared one"
        );
        let after: Vec<u32> = (0..snapshot.line_count())
            .map(|l| snapshot.line_start(u32::try_from(l).expect("small")))
            .collect();
        assert_eq!(after, before, "the snapshot observed a mutation");
        assert_eq!(idx.line_count(), snapshot.line_count() + 2);
    }
}
