//! Rendered-value property lattice — string content analysis
//! over SSA.
//!
//! Tracks properties of each SSA value *after* Tcl backslash
//! substitution so downstream consumers (primarily the taint
//! engine) can query things like "does this value contain a
//! forward slash?" or "has it been through a partial unescape?"
//! without re-traversing source text.
//!
//! Uses a reduced product of boolean domains split into two
//! join kinds:
//!
//! * **may** properties (union at phi joins): overapproximate. If
//!   *any* incoming edge has `HAS_FORWARD_SLASH`, the merged
//!   value has it.
//! * **must** properties (intersection at phi joins):
//!   underapproximate. `STARTS_WITH_SLASH` only survives when
//!   *all* incoming edges agree.
//!
//! Ported from `core/compiler/rendered_properties.py` (C27c) —
//! focused subset covering the lattice types, join operation,
//! and the literal-text analyser. Integration with the full
//! SSA walk is left for a follow-up once taint is in the tree.

use bitflags::bitflags;

bitflags! {
    /// Properties of a rendered (post-backslash-subst) SSA value.
    ///
    /// Split into two groups — "may" properties (union at joins)
    /// and "must" properties (intersection at joins) — tracked
    /// together in one `u32` bitmask.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct RenderedProperties: u32 {
        /// No properties known.
        const NONE                = 0;

        // -- may properties (union at joins) --
        /// Rendered literal text contains `/`.
        const HAS_FORWARD_SLASH   = 1 << 0;
        /// Rendered literal text contains `\\` (path separator).
        const HAS_BACKSLASH       = 1 << 1;
        /// Rendered literal text contains `\r` or `\n`.
        const HAS_CRLF            = 1 << 2;
        /// Value contains `$var` or `[cmd]` interpolation.
        const HAS_INTERPOLATION   = 1 << 3;
        /// Rendered text contains already-escaped sequences.
        const HAS_DOUBLE_ESCAPE   = 1 << 4;
        /// Rendered text contains `\x00` / null byte.
        const HAS_NULL            = 1 << 5;

        // -- provenance bits (propagated explicitly by commands) --
        /// Value passed through `subst` / `URI::decode` / `b64decode`.
        const WAS_UNESCAPED       = 1 << 6;
        /// Value was already `WAS_UNESCAPED` then unescaped again.
        const DOUBLE_UNESCAPED    = 1 << 7;
        /// Value is fully canonical — no residual encoding
        /// (`-normalized` getters, `file normalize`, …).
        const FULLY_NORMALISED    = 1 << 8;

        // -- must properties (intersection at joins) --
        /// First rendered literal character is `/`.
        const STARTS_WITH_SLASH   = 1 << 9;
        /// First rendered literal character is `-`.
        const STARTS_WITH_DASH    = 1 << 10;
    }
}

impl RenderedProperties {
    /// Mask of "may"-join properties.
    pub const MAY_MASK: Self = Self::from_bits_truncate(
        Self::HAS_FORWARD_SLASH.bits()
            | Self::HAS_BACKSLASH.bits()
            | Self::HAS_CRLF.bits()
            | Self::HAS_INTERPOLATION.bits()
            | Self::HAS_DOUBLE_ESCAPE.bits()
            | Self::HAS_NULL.bits(),
    );

    /// Mask of provenance bits.
    pub const PROVENANCE_MASK: Self = Self::from_bits_truncate(
        Self::WAS_UNESCAPED.bits()
            | Self::DOUBLE_UNESCAPED.bits()
            | Self::FULLY_NORMALISED.bits(),
    );

    /// Mask of "must"-join properties.
    pub const MUST_MASK: Self =
        Self::from_bits_truncate(Self::STARTS_WITH_SLASH.bits() | Self::STARTS_WITH_DASH.bits());
}

/// Rendered string content properties for a single SSA value.
///
/// `may` bits union at joins (overapproximate); `must` bits
/// intersect at joins (underapproximate). The bottom element has
/// empty `may` and all must-bits set (assume every must-property
/// holds until disproven); the top element is the reverse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RenderedValueProps {
    /// Properties that *may* hold (union at joins).
    pub may: RenderedProperties,
    /// Properties that *must* hold (intersection at joins).
    pub must: RenderedProperties,
}

impl RenderedValueProps {
    /// The lattice bottom — no known may-property, every
    /// must-property assumed until proven otherwise.
    #[must_use]
    pub const fn bottom() -> Self {
        Self {
            may: RenderedProperties::NONE,
            must: RenderedProperties::MUST_MASK,
        }
    }

    /// The lattice top — every may-property conservatively
    /// assumed, no must-property known.
    #[must_use]
    pub const fn top() -> Self {
        Self {
            may: RenderedProperties::MAY_MASK,
            must: RenderedProperties::NONE,
        }
    }
}

impl Default for RenderedValueProps {
    fn default() -> Self {
        Self::bottom()
    }
}

/// Join two lattice values.
///
/// `may` bits use union (overapproximate); `must` bits use
/// intersection (underapproximate). Ported from the Python
/// `rendered_join`.
#[must_use]
pub fn rendered_join(a: RenderedValueProps, b: RenderedValueProps) -> RenderedValueProps {
    RenderedValueProps {
        may: a.may | b.may,
        must: a.must & b.must,
    }
}

/// Analyse a rendered literal string for its property bitmask.
///
/// Sets `HAS_FORWARD_SLASH` / `HAS_BACKSLASH` / `HAS_CRLF` /
/// `HAS_NULL` bits when their character appears. Sets
/// `STARTS_WITH_SLASH` / `STARTS_WITH_DASH` based on the first
/// character. Does *not* set `HAS_INTERPOLATION` — that's the
/// caller's responsibility based on source structure before
/// rendering.
#[must_use]
pub fn analyse_literal(text: &str) -> RenderedValueProps {
    let mut may = RenderedProperties::NONE;
    let bytes = text.as_bytes();
    for &b in bytes {
        match b {
            b'/' => may |= RenderedProperties::HAS_FORWARD_SLASH,
            b'\\' => may |= RenderedProperties::HAS_BACKSLASH,
            b'\r' | b'\n' => may |= RenderedProperties::HAS_CRLF,
            0 => may |= RenderedProperties::HAS_NULL,
            _ => {}
        }
    }
    let mut must = RenderedProperties::NONE;
    if let Some(&first) = bytes.first() {
        if first == b'/' {
            must |= RenderedProperties::STARTS_WITH_SLASH;
        }
        if first == b'-' {
            must |= RenderedProperties::STARTS_WITH_DASH;
        }
    }
    RenderedValueProps { may, must }
}

/// Apply an unescape-step transition.
///
/// If the incoming value had `WAS_UNESCAPED`, the outgoing value
/// gains `DOUBLE_UNESCAPED`. Otherwise sets `WAS_UNESCAPED`.
#[must_use]
pub fn apply_unescape(input: RenderedValueProps) -> RenderedValueProps {
    let mut out = input;
    if out.may.contains(RenderedProperties::WAS_UNESCAPED) {
        out.may |= RenderedProperties::DOUBLE_UNESCAPED;
    } else {
        out.may |= RenderedProperties::WAS_UNESCAPED;
    }
    // Must-bits survive unescaping only when we've proved they do;
    // conservatively clear them.
    out.must = RenderedProperties::NONE;
    out
}

/// Apply a `-normalized` getter — strips all encoding, sets
/// `FULLY_NORMALISED`, clears unescape provenance.
#[must_use]
pub fn apply_normalised(mut input: RenderedValueProps) -> RenderedValueProps {
    input.may.remove(RenderedProperties::WAS_UNESCAPED);
    input.may.remove(RenderedProperties::DOUBLE_UNESCAPED);
    input.may |= RenderedProperties::FULLY_NORMALISED;
    input
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bottom_has_all_must_none_may() {
        let b = RenderedValueProps::bottom();
        assert_eq!(b.may, RenderedProperties::NONE);
        assert_eq!(b.must, RenderedProperties::MUST_MASK);
    }

    #[test]
    fn top_is_reverse_of_bottom() {
        let t = RenderedValueProps::top();
        assert_eq!(t.may, RenderedProperties::MAY_MASK);
        assert_eq!(t.must, RenderedProperties::NONE);
    }

    #[test]
    fn default_is_bottom() {
        assert_eq!(RenderedValueProps::default(), RenderedValueProps::bottom());
    }

    #[test]
    fn join_unions_may_intersects_must() {
        let a = RenderedValueProps {
            may: RenderedProperties::HAS_FORWARD_SLASH,
            must: RenderedProperties::STARTS_WITH_SLASH | RenderedProperties::STARTS_WITH_DASH,
        };
        let b = RenderedValueProps {
            may: RenderedProperties::HAS_CRLF,
            must: RenderedProperties::STARTS_WITH_SLASH,
        };
        let j = rendered_join(a, b);
        assert!(j.may.contains(RenderedProperties::HAS_FORWARD_SLASH));
        assert!(j.may.contains(RenderedProperties::HAS_CRLF));
        // Intersection — only STARTS_WITH_SLASH survives.
        assert!(j.must.contains(RenderedProperties::STARTS_WITH_SLASH));
        assert!(!j.must.contains(RenderedProperties::STARTS_WITH_DASH));
    }

    #[test]
    fn analyse_literal_slash() {
        let p = analyse_literal("/etc/hosts");
        assert!(p.may.contains(RenderedProperties::HAS_FORWARD_SLASH));
        assert!(p.must.contains(RenderedProperties::STARTS_WITH_SLASH));
        assert!(!p.must.contains(RenderedProperties::STARTS_WITH_DASH));
    }

    #[test]
    fn analyse_literal_dash() {
        let p = analyse_literal("-flag");
        assert!(p.must.contains(RenderedProperties::STARTS_WITH_DASH));
        assert!(!p.must.contains(RenderedProperties::STARTS_WITH_SLASH));
    }

    #[test]
    fn analyse_literal_crlf_and_null() {
        let p = analyse_literal("line1\r\nline2");
        assert!(p.may.contains(RenderedProperties::HAS_CRLF));
        let p = analyse_literal("a\0b");
        assert!(p.may.contains(RenderedProperties::HAS_NULL));
    }

    #[test]
    fn analyse_literal_plain_string() {
        let p = analyse_literal("hello");
        assert_eq!(p.may, RenderedProperties::NONE);
        assert_eq!(p.must, RenderedProperties::NONE);
    }

    #[test]
    fn apply_unescape_first_time_sets_was_unescaped() {
        let out = apply_unescape(RenderedValueProps::bottom());
        assert!(out.may.contains(RenderedProperties::WAS_UNESCAPED));
        assert!(!out.may.contains(RenderedProperties::DOUBLE_UNESCAPED));
    }

    #[test]
    fn apply_unescape_second_time_sets_double_unescaped() {
        let mut input = RenderedValueProps::bottom();
        input.may |= RenderedProperties::WAS_UNESCAPED;
        let out = apply_unescape(input);
        assert!(out.may.contains(RenderedProperties::WAS_UNESCAPED));
        assert!(out.may.contains(RenderedProperties::DOUBLE_UNESCAPED));
    }

    #[test]
    fn apply_normalised_strips_unescape_provenance() {
        let mut input = RenderedValueProps::bottom();
        input.may |= RenderedProperties::WAS_UNESCAPED;
        input.may |= RenderedProperties::DOUBLE_UNESCAPED;
        let out = apply_normalised(input);
        assert!(!out.may.contains(RenderedProperties::WAS_UNESCAPED));
        assert!(!out.may.contains(RenderedProperties::DOUBLE_UNESCAPED));
        assert!(out.may.contains(RenderedProperties::FULLY_NORMALISED));
    }
}
