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
//! of reference Tcl's `TclParseBackslash`, shared with the LSP/compiler) as
//! [`decode`], its extent rule [`tcl_lexer::backslash_escape_end`] as
//! [`escape_end`] (so scanners that decode one escape at a time advance by the
//! same widths the decoder consumes), and adds a byte-slice convenience for the
//! runtime, which holds Tcl string reps as UTF-8 bytes. There is intentionally
//! **no** second decoder: the runtime's old hand-rolled `bs.rs` (which emitted
//! a raw `0xFF` byte for `\xff`, invalid UTF-8) is retired in favour of this
//! one (which yields `U+00FF`, matching Tcl 9 and the UTF-8-internal-rep
//! invariant).
//!
//! The grammar is **release-variant** ([`EscapeSyntax`]): TIP 388 (8.6) capped
//! `\x` at two hex digits, added `\U`, and guarded the octal third digit, so
//! `\x4142` is `B` under 8.5 and `A42` from 8.6. Each entry point comes in two
//! forms — a bare name pinned to Tcl 9.0, for consumers with no release in
//! scope, and an `_in` form taking the release. Decode and extent must always
//! use the *same* form: an escape's width and its value come from one scan.

use std::borrow::Cow;

pub use tcl_dialect::EscapeSyntax;
pub use tcl_lexer::backslash_escape_end as escape_end;
pub use tcl_lexer::backslash_escape_end_in as escape_end_in;
pub use tcl_lexer::backslash_subst as decode;
pub use tcl_lexer::backslash_subst_in as decode_in;

/// Decode Tcl backslash escapes in a byte slice that is a valid UTF-8 Tcl string
/// rep (the runtime invariant), under **Tcl 9.0's** grammar. Borrows when there
/// is nothing to decode (no backslash, or — defensively — non-UTF-8 input, which
/// cannot occur for a well-formed internal rep). Otherwise returns freshly
/// decoded bytes.
///
/// A caller that knows which release it evaluates for uses [`decode_bytes_in`];
/// the grammar is release-variant (see [`EscapeSyntax`]) and 9.0 is the
/// documented default for consumers with no release in scope.
#[must_use]
pub fn decode_bytes(raw: &[u8]) -> Cow<'_, [u8]> {
    decode_bytes_in(raw, EscapeSyntax::default())
}

/// [`decode_bytes`] under `escapes` — the release-aware form.
#[must_use]
pub fn decode_bytes_in(raw: &[u8], escapes: EscapeSyntax) -> Cow<'_, [u8]> {
    let Ok(s) = core::str::from_utf8(raw) else {
        return Cow::Borrowed(raw);
    };
    match decode_in(s, escapes) {
        Cow::Borrowed(b) => Cow::Borrowed(b.as_bytes()),
        Cow::Owned(o) => Cow::Owned(o.into_bytes()),
    }
}

/// Collapse Tcl brace-word line continuations: a backslash immediately followed
/// by LF — `\<LF>` — together with any spaces and tabs after it, becomes a
/// single space. Raw `\<CR>` and `\<CRLF>` are data; a source channel may
/// translate CRLF to LF before this parser seam. This is the **only**
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
        .iter()
        .enumerate()
        .any(|(i, _)| tcl_lexer::backslash_continuation_end(raw, i).is_some())
    {
        return Cow::Borrowed(raw);
    }
    let mut out = Vec::with_capacity(raw.len());
    let mut i = 0;
    while i < raw.len() {
        if raw[i] == b'\\' {
            match tcl_lexer::backslash_continuation_end(raw, i) {
                Some(end) => {
                    i = end;
                    if trim_preceding {
                        while matches!(out.last(), Some(b' ' | b'\t')) {
                            out.pop();
                        }
                    }
                    out.push(b' ');
                    continue;
                }
                // A non-continuation backslash escapes the next byte for
                // scanning purposes, so `\\<newline>` stays a literal `\\` +
                // newline rather than the second backslash starting a
                // continuation.
                None if let Some(&b) = raw.get(i + 1) => {
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

/// [`collapse_brace_continuations`], gated on the dialect's
/// [`BraceBackslashNewline`](tcl_dialect::BraceBackslashNewline) rule.
///
/// Every build of the Tcl core folds — `TclCopyAndCollapse` rewrites the
/// backslash-newline run to one space, so `{a\<newline>b}` is `a b`.
/// `JimTcl` keeps the bytes (`JimParseSubBrace`, jim.c:1444-1485),
/// deliberately, so line numbers survive a braced body.
///
/// Measured: `string length {a\<newline>b}` is 3 under tclsh 8.6 and 9.0,
/// and 4 under every modelled Jim release.
///
/// Under `Literal` this borrows the input unchanged. Downstream *list*
/// parsing still applies its own element escapes, which is why not folding
/// here is enough to reproduce Jim end to end.
#[must_use]
pub fn collapse_brace_continuations_for(
    raw: &[u8],
    rule: tcl_dialect::BraceBackslashNewline,
) -> Cow<'_, [u8]> {
    if rule.folds() {
        collapse_brace_continuations(raw)
    } else {
        Cow::Borrowed(raw)
    }
}

/// [`collapse_brace_continuations_for`] for a `&str`.
#[must_use]
pub fn collapse_brace_continuations_str_for(
    text: &str,
    rule: tcl_dialect::BraceBackslashNewline,
) -> Cow<'_, str> {
    if rule.folds() {
        collapse_brace_continuations_str(text)
    } else {
        Cow::Borrowed(text)
    }
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
        // TclParseBackslash has no CR arm: raw CR and CRLF stay byte-exact.
        assert_eq!(&*collapse_brace_continuations(b"a\\\rb"), b"a\\\rb");
        assert_eq!(
            &*collapse_brace_continuations(b"a\\\r\n   b"),
            b"a\\\r\n   b"
        );
        assert_eq!(&*collapse_brace_continuations(b"a\\\r\tb"), b"a\\\r\tb");
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
        assert_eq!(&*collapse_brace_continuations_str("a\\\r\nb"), "a\\\r\nb");
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
        // for LF. Raw CR/CRLF are ordinary escaped data.
        assert_eq!(&*collapse_separator_continuations(b"a \\\n  b"), b"a b");
        assert_eq!(
            &*collapse_separator_continuations(b"a\t \\\r\n\t b"),
            b"a\t \\\r\n\t b"
        );
        assert_eq!(
            &*collapse_separator_continuations(b"a \\\r  b"),
            b"a \\\r  b"
        );
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
        assert_eq!(
            &*collapse_separator_continuations_str("a \\\r\n b"),
            "a \\\r\n b"
        );
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
    fn decode_bytes_follows_the_release() {
        // The user-visible symptom of issue #1479: a script pinned to 8.4/8.5
        // reads `\x4142` as `B` (all trailing hex digits, low byte), 8.6+ as
        // `A42` (TIP 388's two-digit cap). `\U` exists only from 8.6, and 8.6's
        // stock UTF-16-internal build degrades an astral scalar to U+FFFD.
        assert_eq!(&*decode_bytes_in(b"\\x4142", EscapeSyntax::Tcl84), b"B");
        assert_eq!(&*decode_bytes_in(b"\\x4142", EscapeSyntax::Tcl86), b"A42");
        assert_eq!(&*decode_bytes_in(b"\\x4142", EscapeSyntax::Tcl90), b"A42");
        assert_eq!(
            &*decode_bytes_in(b"\\U0001F600", EscapeSyntax::Tcl84),
            b"U0001F600"
        );
        assert_eq!(
            &*decode_bytes_in(b"\\U0001F600", EscapeSyntax::Tcl86),
            "\u{FFFD}".as_bytes()
        );
        assert_eq!(
            &*decode_bytes_in(b"\\U0001F600", EscapeSyntax::Tcl90),
            "\u{1F600}".as_bytes()
        );
        // The release-blind entry point is the 9.0 one.
        assert_eq!(
            &*decode_bytes(b"\\x4142"),
            &*decode_bytes_in(b"\\x4142", EscapeSyntax::Tcl90)
        );
    }

    #[test]
    fn escape_extent_follows_the_release() {
        // Width and value must agree per release, or a per-escape scanner
        // slices mid-escape.
        assert_eq!(escape_end_in("\\x4142", 0, EscapeSyntax::Tcl84), 6);
        assert_eq!(escape_end_in("\\x4142", 0, EscapeSyntax::Tcl90), 4);
        assert_eq!(escape_end_in("\\U0001F600", 0, EscapeSyntax::Tcl84), 2);
        assert_eq!(escape_end_in("\\U0001F600", 0, EscapeSyntax::Tcl90), 10);
        assert_eq!(
            escape_end("\\x4142", 0),
            escape_end_in("\\x4142", 0, EscapeSyntax::Tcl90)
        );
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
