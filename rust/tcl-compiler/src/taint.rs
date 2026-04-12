//! Taint analysis — data-flow from tainted sources to dangerous
//! sinks, tracked through a multi-colour lattice.
//!
//! Ported from `core/compiler/taint/` (C29). This strip lands
//! the core types (`TaintColour`, `TaintLattice`, `TaintWarning`),
//! constant colour masks, and a stub `find_taint_warnings` entry
//! point. The detailed interprocedural propagation, path-concat /
//! URI-split heuristics, and sink-specific sub-checks are
//! follow-ups that plug into C28's summaries and the C25 SCCP
//! lattice.

use bitflags::bitflags;

use tcl_lexer::Span;

// ---------------------------------------------------------------------------
// Colour lattice
// ---------------------------------------------------------------------------

bitflags! {
    /// A taint colour — each bit records one safety property or
    /// origin fact about a value.
    ///
    /// A value is "clean" when `TAINTED` is unset; otherwise one
    /// or more mitigating colours may prove it safe for specific
    /// sinks (see `T102_SAFE`, `CRLF_FREE`, `SHELL_ATOM`, …).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct TaintColour: u32 {
        /// Value is known tainted (attacker-influenced).
        const TAINTED            = 1 << 0;
        /// Value has an absolute-path prefix (starts with `/`).
        const PATH_PREFIXED      = 1 << 1;
        /// Value cannot start with `-` (option-injection safe).
        const NON_DASH_PREFIXED  = 1 << 2;
        /// Value contains no CR / LF characters.
        const CRLF_FREE          = 1 << 3;
        /// Value is a shell atom (no unquoted whitespace).
        const SHELL_ATOM         = 1 << 4;
        /// Value is a canonical Tcl list with known structure.
        const LIST_CANONICAL     = 1 << 5;
        /// Value is a literal regex pattern.
        const REGEX_LITERAL      = 1 << 6;
        /// Value is a fully-normalised filesystem path.
        const PATH_NORMALISED    = 1 << 7;
        /// Value is bounded inside a known safe directory.
        const PATH_BOUNDED       = 1 << 8;
        /// Value is a header token (RFC 7230 tchar set).
        const HEADER_TOKEN_SAFE  = 1 << 9;
        /// Value has been HTML-escaped.
        const HTML_ESCAPED       = 1 << 10;
        /// Value has been URL-encoded.
        const URL_ENCODED        = 1 << 11;
        /// Value is a literal IP address.
        const IP_ADDRESS         = 1 << 12;
        /// Value is a literal TCP/UDP port number.
        const PORT               = 1 << 13;
        /// Value is a fully-qualified domain name.
        const FQDN               = 1 << 14;
    }
}

impl TaintColour {
    /// Every colour bit set. Used as the "definitely clean" mask
    /// in set-union lattices where adding a colour can only
    /// sharpen what we know.
    pub const ALL: Self = Self::from_bits_truncate(
        Self::TAINTED.bits()
            | Self::PATH_PREFIXED.bits()
            | Self::NON_DASH_PREFIXED.bits()
            | Self::CRLF_FREE.bits()
            | Self::SHELL_ATOM.bits()
            | Self::LIST_CANONICAL.bits()
            | Self::REGEX_LITERAL.bits()
            | Self::PATH_NORMALISED.bits()
            | Self::PATH_BOUNDED.bits()
            | Self::HEADER_TOKEN_SAFE.bits()
            | Self::HTML_ESCAPED.bits()
            | Self::URL_ENCODED.bits()
            | Self::IP_ADDRESS.bits()
            | Self::PORT.bits()
            | Self::FQDN.bits(),
    );

    /// Colours that prove a value cannot start with `-` and so
    /// is safe against option-injection sinks (T102).
    pub const T102_SAFE: Self = Self::from_bits_truncate(
        Self::PATH_PREFIXED.bits()
            | Self::NON_DASH_PREFIXED.bits()
            | Self::IP_ADDRESS.bits()
            | Self::PORT.bits()
            | Self::FQDN.bits(),
    );

    /// Colours that mitigate CRLF / header / log injection.
    pub const CRLF_SAFE: Self = Self::from_bits_truncate(
        Self::CRLF_FREE.bits()
            | Self::IP_ADDRESS.bits()
            | Self::PORT.bits()
            | Self::FQDN.bits()
            | Self::HEADER_TOKEN_SAFE.bits()
            | Self::HTML_ESCAPED.bits()
            | Self::URL_ENCODED.bits(),
    );
}

/// Per-SSA-value taint lattice element: a bag of colours plus a
/// flag tracking whether any incoming path definitely set
/// `TAINTED`.
///
/// `colours` is the "must-have" intersection at joins — a colour
/// survives only when every incoming edge has it. Taint is a
/// "may-have" — once any path sets it, it sticks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaintLattice {
    /// Bag of colours. `TAINTED` membership means "may be tainted".
    pub colours: TaintColour,
}

impl TaintLattice {
    /// Fresh clean value — no taint, no mitigations proven.
    #[must_use]
    pub const fn clean() -> Self {
        Self {
            colours: TaintColour::empty(),
        }
    }

    /// Fully tainted with no mitigations.
    #[must_use]
    pub const fn tainted() -> Self {
        Self {
            colours: TaintColour::TAINTED,
        }
    }

    /// True when the value is known tainted.
    #[must_use]
    pub const fn is_tainted(self) -> bool {
        self.colours.contains(TaintColour::TAINTED)
    }

    /// Intersect mitigating colours (must-have), union taint bits
    /// (may-have). This implements the standard lattice join for
    /// taint analysis.
    #[must_use]
    pub fn join(self, other: Self) -> Self {
        let taint = (self.colours | other.colours) & TaintColour::TAINTED;
        let mitigations =
            (self.colours & other.colours) & !TaintColour::TAINTED;
        Self {
            colours: taint | mitigations,
        }
    }

    /// Add a colour (typically a mitigation).
    #[must_use]
    pub fn with(self, c: TaintColour) -> Self {
        Self {
            colours: self.colours | c,
        }
    }

    /// Remove `TAINTED` — used by sanitisers.
    #[must_use]
    pub fn sanitised(self) -> Self {
        Self {
            colours: self.colours & !TaintColour::TAINTED,
        }
    }
}

// ---------------------------------------------------------------------------
// Diagnostic type
// ---------------------------------------------------------------------------

/// Tainted data flowing into a dangerous sink.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TaintWarning {
    /// Span of the sink use.
    pub span: Span,
    /// Variable name carrying the taint.
    pub variable: String,
    /// Command that acted as the sink.
    pub sink_command: String,
    /// Diagnostic code (`"T100"` family).
    pub code: String,
    /// Formatted message.
    pub message: String,
}

// ---------------------------------------------------------------------------
// Public API (stub)
// ---------------------------------------------------------------------------

/// Run taint analysis over a module.
///
/// **Current status**: stub returning an empty vector. The full
/// pipeline requires C28 interprocedural summaries, the SSA +
/// memory-SSA graph, path-concat / URI-split heuristics, and a
/// sink-specific check battery. Those land as follow-up strips;
/// this API shape lets downstream callers (the LSP / optimiser /
/// `compiler_checks`) wire in the surface now.
#[must_use]
pub fn find_taint_warnings(_source: &str) -> Vec<TaintWarning> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_and_tainted_constructors() {
        let c = TaintLattice::clean();
        assert!(!c.is_tainted());
        let t = TaintLattice::tainted();
        assert!(t.is_tainted());
    }

    #[test]
    fn join_propagates_taint_intersects_mitigations() {
        let a = TaintLattice {
            colours: TaintColour::TAINTED | TaintColour::CRLF_FREE | TaintColour::PATH_PREFIXED,
        };
        let b = TaintLattice {
            colours: TaintColour::CRLF_FREE | TaintColour::NON_DASH_PREFIXED,
        };
        let j = a.join(b);
        // Taint is sticky once any path has it.
        assert!(j.colours.contains(TaintColour::TAINTED));
        // CRLF_FREE is on both sides → survives.
        assert!(j.colours.contains(TaintColour::CRLF_FREE));
        // PATH_PREFIXED only on `a` → intersects out.
        assert!(!j.colours.contains(TaintColour::PATH_PREFIXED));
        // NON_DASH_PREFIXED only on `b` → intersects out.
        assert!(!j.colours.contains(TaintColour::NON_DASH_PREFIXED));
    }

    #[test]
    fn with_and_sanitised() {
        let v = TaintLattice::tainted().with(TaintColour::CRLF_FREE);
        assert!(v.is_tainted());
        assert!(v.colours.contains(TaintColour::CRLF_FREE));
        let s = v.sanitised();
        assert!(!s.is_tainted());
        assert!(s.colours.contains(TaintColour::CRLF_FREE));
    }

    #[test]
    fn t102_safe_mask_excludes_tainted() {
        assert!(!TaintColour::T102_SAFE.contains(TaintColour::TAINTED));
        assert!(TaintColour::T102_SAFE.contains(TaintColour::PATH_PREFIXED));
    }

    #[test]
    fn crlf_safe_mask_includes_crlf_free() {
        assert!(TaintColour::CRLF_SAFE.contains(TaintColour::CRLF_FREE));
        assert!(TaintColour::CRLF_SAFE.contains(TaintColour::HEADER_TOKEN_SAFE));
    }

    #[test]
    fn find_taint_warnings_is_empty_stub() {
        assert!(find_taint_warnings("set x 1").is_empty());
    }
}
