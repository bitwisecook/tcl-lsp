//! Byte-range spans threaded through the whole Rust rewrite.
//!
//! Every positional entity — [`Token`], future IR nodes, future CFG
//! nodes, future diagnostics — carries a [`Span`] pointing into the
//! source buffer. Text and `(line, character, offset)` positions are
//! resolved on demand via a [`SourceMap`] rather than being stored
//! inline on every entity. This mirrors the design used by
//! rust-analyzer (`TextRange`), tree-sitter (`Range`), and swc
//! (`Span`): cheap `Copy` values, no lifetimes, no per-entity
//! position bloat, and one central place (`SourceMap`) that owns the
//! line index.
//!
//! [`Token`]: crate::Token
//! [`SourceMap`]: crate::SourceMap

/// A byte range within a source string: `[start, end)` (exclusive end).
///
/// 8 bytes, `Copy`, no lifetime. Cheap to store on every IR or CFG
/// node, cheap to move around in diagnostics, and trivially
/// comparable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    start: u32,
    end: u32,
}

impl Span {
    /// Build a span from raw byte offsets.
    ///
    /// Asserts in debug builds that `start <= end`; a `Span` with
    /// `start > end` is nonsensical and would produce
    /// incorrect-looking text when sliced.
    #[must_use]
    pub const fn new(start: u32, end: u32) -> Self {
        debug_assert!(start <= end, "span start must be <= end");
        Self { start, end }
    }

    /// Build an empty span anchored at `offset`.
    #[must_use]
    pub const fn empty(offset: u32) -> Self {
        Self {
            start: offset,
            end: offset,
        }
    }

    /// Byte offset of the first character in the span.
    #[must_use]
    pub const fn start(self) -> u32 {
        self.start
    }

    /// Byte offset one past the last character in the span.
    #[must_use]
    pub const fn end(self) -> u32 {
        self.end
    }

    /// Number of bytes covered by the span.
    #[must_use]
    pub const fn len(self) -> u32 {
        self.end - self.start
    }

    /// True if the span covers zero bytes.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }

    /// Convert the span into a `std::ops::Range<usize>` suitable for
    /// slicing a `&str` directly: `&source[span.as_range()]`.
    #[must_use]
    pub const fn as_range(self) -> std::ops::Range<usize> {
        self.start as usize..self.end as usize
    }
}

impl From<Span> for std::ops::Range<usize> {
    fn from(span: Span) -> Self {
        span.as_range()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_and_accessors() {
        let span = Span::new(3, 10);
        assert_eq!(span.start(), 3);
        assert_eq!(span.end(), 10);
        assert_eq!(span.len(), 7);
        assert!(!span.is_empty());
    }

    #[test]
    fn empty_span_has_zero_len() {
        let span = Span::empty(42);
        assert_eq!(span.start(), 42);
        assert_eq!(span.end(), 42);
        assert_eq!(span.len(), 0);
        assert!(span.is_empty());
    }

    #[test]
    fn as_range_slices_str() {
        let source = "hello world";
        let span = Span::new(6, 11);
        assert_eq!(&source[span.as_range()], "world");
    }

    #[test]
    fn into_std_range() {
        let span = Span::new(0, 5);
        let range: std::ops::Range<usize> = span.into();
        assert_eq!(range, 0..5);
    }

    #[test]
    fn span_is_copy() {
        let a = Span::new(0, 3);
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn span_hash_and_eq() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(Span::new(0, 3));
        set.insert(Span::new(0, 3));
        set.insert(Span::new(0, 4));
        assert_eq!(set.len(), 2);
    }
}
