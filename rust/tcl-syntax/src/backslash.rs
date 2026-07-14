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

//! Tcl backslash-escape decoding — the canonical decoder.
//!
//! Re-exports [`tcl_lexer::backslash_subst`] (the one byte-exact implementation
//! of reference Tcl 9.0's `TclParseBackslash`, shared with the LSP/compiler) as
//! [`decode`], its extent rule [`tcl_lexer::backslash_escape_end`] as
//! [`escape_end`] (so scanners that decode one escape at a time advance by the
//! same widths the decoder consumes), and adds a byte-slice convenience for the
//! runtime, which holds Tcl string reps as UTF-8 bytes. There is intentionally
//! **no** second decoder: the runtime's old hand-rolled `bs.rs` (which emitted
//! a raw `0xFF` byte for `\xff`, invalid UTF-8) is retired in favour of this
//! one (which yields `U+00FF`, matching Tcl 9 and the UTF-8-internal-rep
//! invariant).

use std::borrow::Cow;

pub use tcl_lexer::backslash_escape_end as escape_end;
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

/// Collapse Tcl brace-word line continuations: a backslash immediately followed
/// by a newline — `\<LF>`, `\<CR>`, or `\<CRLF>`, together with any spaces and
/// tabs after that newline — becomes a single space. This is the **only**
/// backslash processing a `{braced}` word undergoes (every other backslash byte
/// stays literal), and it matches C's pre-pass rule for the backslash-newline
/// sequence, which the `Tcl` language summary notes applies *even inside
/// braces*. An escaped backslash (`\\`) before a newline is a literal `\\` and
/// does not start a continuation. Borrows unchanged when the input contains no
/// continuation (the common case).
#[must_use]
pub fn collapse_brace_continuations(raw: &[u8]) -> Cow<'_, [u8]> {
    collapse_continuations(raw, false)
}

/// [`collapse_brace_continuations`] for word-*separator* contexts: the spaces
/// and tabs **preceding** the backslash collapse too, so a whole
/// `<ws>\<newline><ws>` run becomes a single space. Between the words of a
/// command (a command substitution `[…]`, a bare word) the run is one
/// inter-word separator the parser collapses; inside a `{braced}` or
/// `"quoted"` word the preceding whitespace is string data, which is why
/// [`collapse_brace_continuations`] keeps it. Borrows unchanged when the input
/// contains no continuation.
#[must_use]
pub fn collapse_separator_continuations(raw: &[u8]) -> Cow<'_, [u8]> {
    collapse_continuations(raw, true)
}

/// The shared continuation collapse; `trim_preceding` selects the separator
/// rule (drop spaces/tabs already emitted before the backslash).
fn collapse_continuations(raw: &[u8], trim_preceding: bool) -> Cow<'_, [u8]> {
    if !raw
        .windows(2)
        .any(|w| w[0] == b'\\' && matches!(w[1], b'\n' | b'\r'))
    {
        return Cow::Borrowed(raw);
    }
    let mut out = Vec::with_capacity(raw.len());
    let mut i = 0;
    while i < raw.len() {
        if raw[i] == b'\\' {
            match raw.get(i + 1) {
                Some(&nl @ (b'\n' | b'\r')) => {
                    i += 2;
                    // A `\r` pairs with a following `\n` (CRLF) as one newline.
                    if nl == b'\r' && raw.get(i) == Some(&b'\n') {
                        i += 1;
                    }
                    if trim_preceding {
                        while matches!(out.last(), Some(b' ' | b'\t')) {
                            out.pop();
                        }
                    }
                    out.push(b' ');
                    while matches!(raw.get(i), Some(b' ' | b'\t')) {
                        i += 1;
                    }
                    continue;
                }
                // A non-continuation backslash escapes the next byte for
                // scanning purposes, so `\\<newline>` stays a literal `\\` +
                // newline rather than the second backslash starting a
                // continuation.
                Some(&b) => {
                    out.push(b'\\');
                    out.push(b);
                    i += 2;
                    continue;
                }
                None => {
                    out.push(b'\\');
                    i += 1;
                    continue;
                }
            }
        }
        out.push(raw[i]);
        i += 1;
    }
    Cow::Owned(out)
}

/// [`collapse_brace_continuations`] for a `&str`, returning a `Cow<str>`. The
/// collapse only ever rewrites ASCII bytes (`\`, newline, spaces, tabs), so a
/// valid-UTF-8 input always yields valid UTF-8. Borrows when there is no
/// continuation to collapse.
#[must_use]
pub fn collapse_brace_continuations_str(text: &str) -> Cow<'_, str> {
    collapsed_bytes_to_str(text, collapse_brace_continuations(text.as_bytes()))
}

/// [`collapse_separator_continuations`] for a `&str`, returning a `Cow<str>`
/// under the same UTF-8-preservation argument as
/// [`collapse_brace_continuations_str`].
#[must_use]
pub fn collapse_separator_continuations_str(text: &str) -> Cow<'_, str> {
    collapsed_bytes_to_str(text, collapse_separator_continuations(text.as_bytes()))
}

/// Rewrap a byte-level collapse of `text` as `Cow<str>`.
fn collapsed_bytes_to_str<'s>(text: &'s str, collapsed: Cow<'s, [u8]>) -> Cow<'s, str> {
    match collapsed {
        Cow::Borrowed(_) => Cow::Borrowed(text),
        Cow::Owned(bytes) => {
            Cow::Owned(String::from_utf8(bytes).expect("continuation collapse preserves UTF-8"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brace_continuations_collapse_like_tcl9() {
        // `\<newline>` plus following spaces/tabs ⇒ one space.
        assert_eq!(&*collapse_brace_continuations(b"a\\\nb"), b"a b");
        assert_eq!(&*collapse_brace_continuations(b"a\\\n   b"), b"a b");
        assert_eq!(&*collapse_brace_continuations(b"a\\\n\t b"), b"a b");
        // `\<CR>` and `\<CRLF>` collapse the same way (tclsh9.0: length 3 for
        // `{a\<eol>   b}` regardless of the line ending).
        assert_eq!(&*collapse_brace_continuations(b"a\\\rb"), b"a b");
        assert_eq!(&*collapse_brace_continuations(b"a\\\r\n   b"), b"a b");
        assert_eq!(&*collapse_brace_continuations(b"a\\\r\tb"), b"a b");
        // No continuation ⇒ borrowed, byte-identical (including a literal `\n`).
        assert!(matches!(
            collapse_brace_continuations(b"p\\nq"),
            Cow::Borrowed(_)
        ));
        // An escaped backslash before a newline is not a continuation.
        assert_eq!(&*collapse_brace_continuations(b"x\\\\\ny"), b"x\\\\\ny");
        assert_eq!(&*collapse_brace_continuations(b"x\\\\\r\ny"), b"x\\\\\r\ny");
        // Two continuations in a row ⇒ two spaces (each `\<eol>` → one space).
        assert_eq!(&*collapse_brace_continuations(b"m\\\n\\\nn"), b"m  n");
    }

    #[test]
    fn brace_continuations_str_matches_bytes_and_borrows() {
        assert_eq!(&*collapse_brace_continuations_str("a\\\n   b"), "a b");
        assert_eq!(&*collapse_brace_continuations_str("a\\\r\nb"), "a b");
        // No continuation ⇒ borrowed (no allocation), including a literal `\t`.
        assert!(matches!(
            collapse_brace_continuations_str("a\\tb"),
            Cow::Borrowed(_)
        ));
    }

    #[test]
    fn separator_continuations_trim_preceding_whitespace() {
        // Word-separator context: the whole `<ws>\<eol><ws>` run is ONE
        // separator, so the spaces/tabs before the backslash collapse too —
        // for LF, CR, and CRLF endings alike.
        assert_eq!(&*collapse_separator_continuations(b"a \\\n  b"), b"a b");
        assert_eq!(
            &*collapse_separator_continuations(b"a\t \\\r\n\t b"),
            b"a b"
        );
        assert_eq!(&*collapse_separator_continuations(b"a \\\r  b"), b"a b");
        // Contrast: the brace/quote rule keeps the preceding space as data.
        assert_eq!(&*collapse_brace_continuations(b"a \\\n  b"), b"a  b");
        // FP guards: no continuation borrows unchanged (preceding whitespace
        // untouched), and an escaped backslash is not a continuation.
        assert!(matches!(
            collapse_separator_continuations(b"a  b"),
            Cow::Borrowed(_)
        ));
        assert_eq!(
            &*collapse_separator_continuations(b"x \\\\\ny"),
            b"x \\\\\ny"
        );
    }

    #[test]
    fn separator_continuations_str_matches_bytes() {
        assert_eq!(&*collapse_separator_continuations_str("a \\\r\n b"), "a b");
        assert!(matches!(
            collapse_separator_continuations_str("a\\tb"),
            Cow::Borrowed(_)
        ));
    }

    #[test]
    fn bytes_decode_matches_tcl9() {
        assert_eq!(&*decode_bytes(b"a\\tb"), b"a\tb");
        // `\xff` is U+00FF ⇒ two UTF-8 bytes (the old bs.rs emitted one raw byte).
        assert_eq!(&*decode_bytes(b"\\xff"), "\u{FF}".as_bytes());
        assert_eq!(&*decode_bytes(b"\\u00e9"), "é".as_bytes());
        // no backslash ⇒ borrowed, byte-identical.
        assert!(matches!(decode_bytes(b"plain"), Cow::Borrowed(_)));
    }

    #[test]
    fn decode_control_escapes_match_tclsh() {
        // Codepoints verified against tclsh8.6/9.0 (`scan "\X" %c`):
        // \a=7 \b=8 \f=12 \n=10 \r=13 \t=9 \v=11 \\=92.
        assert_eq!(&*decode("\\a"), "\u{07}");
        assert_eq!(&*decode("\\b"), "\u{08}");
        assert_eq!(&*decode("\\f"), "\u{0c}");
        assert_eq!(&*decode("\\n"), "\n");
        assert_eq!(&*decode("\\r"), "\r");
        assert_eq!(&*decode("\\t"), "\t");
        assert_eq!(&*decode("\\v"), "\u{0b}");
        assert_eq!(&*decode("\\\\"), "\\");
    }

    #[test]
    fn decode_numeric_escapes_match_tclsh() {
        // tclsh: \x41=A (exactly 2 hex), \101=A (octal), A=A (1-4 hex).
        assert_eq!(&*decode("\\x41"), "A");
        assert_eq!(&*decode("\\101"), "A");
        assert_eq!(&*decode("\\u0041"), "A");
        // \x consumes exactly two hex digits, so `\x4142` → "A42" (tclsh: the
        // string-length is 3, not 1).
        assert_eq!(&*decode("\\x4142"), "A42");
        // \xff → U+00FF (Tcl 9 / UTF-8 internal rep), not a raw 0xFF byte.
        assert_eq!(&*decode("\\xff"), "\u{FF}");
        // \u with fewer than 4 hex digits still decodes (\u41 → A).
        assert_eq!(&*decode("\\u41"), "A");
    }

    #[test]
    fn decode_unknown_escape_drops_backslash() {
        // tclsh: an unknown escape keeps the character (`\q` → `q`).
        assert_eq!(&*decode("\\q"), "q");
        // Escaped quote/brace are literal.
        assert_eq!(&*decode("\\\""), "\"");
        assert_eq!(&*decode("\\{"), "{");
    }

    #[test]
    fn decode_no_backslash_borrows() {
        // Nothing to decode ⇒ a borrowed slice (no allocation).
        assert!(matches!(decode("plain text"), Cow::Borrowed(_)));
    }

    #[test]
    fn decode_bytes_passes_through_non_utf8() {
        // A non-UTF-8 byte slice cannot be a well-formed internal rep, but the
        // defensive branch borrows it unchanged rather than panicking.
        let raw = [0xff, 0xfe, b'a'];
        assert!(matches!(decode_bytes(&raw), Cow::Borrowed(_)));
        assert_eq!(&*decode_bytes(&raw), &raw);
    }
}
