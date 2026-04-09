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
    /// Build a `LineIndex` by scanning `source` once for `\n`.
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
        let mut starts = Vec::with_capacity(source.len() / 32 + 1);
        starts.push(0_u32);
        for (i, b) in source.bytes().enumerate() {
            if b == b'\n' {
                // `i + 1` cannot overflow u32: source.len() fits u32
                // and i < source.len().
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
    /// Non-ASCII column parity is tracked as deferred work in
    /// `docs/rust-rewrite.md`; changing it here is a coordinated
    /// Python-and-Rust fix across the whole position infrastructure,
    /// not a lexer-local concern.
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
}
