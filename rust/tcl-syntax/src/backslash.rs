//! Tcl backslash-escape decoding — the canonical decoder.
//!
//! Re-exports [`tcl_lexer::backslash_subst`] (the one byte-exact implementation
//! of reference Tcl 9.0's `TclParseBackslash`, shared with the LSP/compiler) and
//! adds a byte-slice convenience for the runtime, which holds Tcl string reps as
//! UTF-8 bytes. There is intentionally **no** second decoder: the runtime's old
//! hand-rolled `bs.rs` (which emitted a raw `0xFF` byte for `\xff`, invalid
//! UTF-8) is retired in favour of this one (which yields `U+00FF`, matching
//! Tcl 9 and the UTF-8-internal-rep invariant).

use std::borrow::Cow;

pub use tcl_lexer::backslash_subst as decode;

/// Decode Tcl backslash escapes in a byte slice that is a valid UTF-8 Tcl string
/// rep (the runtime invariant). Borrows when there is nothing to decode (no
/// backslash, or — defensively — non-UTF-8 input, which cannot occur for a
/// well-formed internal rep). Otherwise returns freshly decoded bytes.
#[must_use]
pub fn decode_bytes(raw: &[u8]) -> Cow<'_, [u8]> {
    let Ok(s) = core::str::from_utf8(raw) else {
        return Cow::Borrowed(raw);
    };
    match decode(s) {
        Cow::Borrowed(b) => Cow::Borrowed(b.as_bytes()),
        Cow::Owned(o) => Cow::Owned(o.into_bytes()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_decode_matches_tcl9() {
        assert_eq!(&*decode_bytes(b"a\\tb"), b"a\tb");
        // `\xff` is U+00FF ⇒ two UTF-8 bytes (the old bs.rs emitted one raw byte).
        assert_eq!(&*decode_bytes(b"\\xff"), "\u{FF}".as_bytes());
        assert_eq!(&*decode_bytes(b"\\u00e9"), "é".as_bytes());
        // no backslash ⇒ borrowed, byte-identical.
        assert!(matches!(decode_bytes(b"plain"), Cow::Borrowed(_)));
    }
}
