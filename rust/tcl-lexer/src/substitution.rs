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

//! Tcl backslash escape processing.
//!
//! Backslash-substitution processing:
//! zero-copy on the fast path (no backslash in the input), a single
//! forward scan of `char_indices` for the slow path, and a clean match
//! table for escape dispatch. The function is callable directly from
//! Rust and exposed via the `tcl-lsp-rust` binding crate.

use std::borrow::Cow;

use tcl_dialect::EscapeSyntax;

/// Process Tcl backslash escapes in `text` under **Tcl 9.0's** grammar.
///
/// The release-blind entry point, for the large body of consumers with no
/// dialect in scope (a hover renderer, a manifest reader, a test helper).
/// Consumers that know which release they compile or evaluate for call
/// [`backslash_subst_in`] instead — the grammar is release-variant, and the
/// 9.0 default is the documented `Unknown` posture, not a universal rule.
///
/// See [`backslash_subst_in`] for the recognised forms.
#[must_use]
pub fn backslash_subst(text: &str) -> Cow<'_, str> {
    backslash_subst_in(text, EscapeSyntax::default())
}

/// Process Tcl backslash escapes in `text` under `escapes`.
///
/// Byte-exact with the release's `TclParseBackslash`. Recognises:
///
/// - Simple mappings: `\a \b \f \n \r \t \v \\ \{ \} \[ \] \$ \" \<space> \;`
/// - Line continuation: `\<LF>` followed by any run of space/tab, collapsed
///   to a single ASCII space. A source channel may translate CRLF to LF before
///   parsing, but a raw `\<CR>` or `\<CRLF>` handed to the parser is not a
///   continuation.
/// - Hex escapes: `\xNN` — two hex digits from 8.6 (TIP 388), unbounded
///   before that, keeping the low byte either way
/// - Unicode escapes: `\uNNNN` (1–4 hex digits)
/// - Wide unicode escapes: `\UNNNNNNNN` (1–8 hex digits) — 8.6 onward only
/// - Octal escapes: `\NNN` (1–3 octal digits, capped to Tcl's byte range)
///
/// Any other `\X` passes through as the character `X`.
///
/// Returns [`Cow::Borrowed`] when `text` contains no backslash (no
/// allocation) and [`Cow::Owned`] otherwise. Surrogate code points map
/// to the Unicode replacement character U+FFFD, matching what any
/// valid-UTF-8 sink would ultimately render. A lone surrogate cannot
/// be represented in a Rust `String`, so U+FFFD is the closest
/// valid-UTF-8 approximation for those edge cases.
#[must_use]
pub fn backslash_subst_in(text: &str, escapes: EscapeSyntax) -> Cow<'_, str> {
    if !text.contains('\\') {
        return Cow::Borrowed(text);
    }

    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    // Start of the literal run waiting to be copied in one piece.
    let mut literal = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] != b'\\' {
            i += 1;
            continue;
        }
        out.push_str(&text[literal..i]);
        let (end, resolved) = decode_escape(text, i, escapes);
        out.push(resolved);
        i = end;
        literal = end;
    }
    out.push_str(&text[literal..]);

    Cow::Owned(out)
}

/// One backslash escape starting at `text[i] == '\'`: where it ends, and the
/// single character it resolves to.
///
/// `TclParseBackslash` always yields exactly one code point, so the extent and
/// the value come from one scan and cannot disagree — which matters because
/// both are release-variant and a consumer that measured an escape under one
/// release and decoded it under another would slice mid-escape. Both
/// [`backslash_subst_in`] and [`backslash_escape_end_in`] are this function.
fn decode_escape(text: &str, i: usize, escapes: EscapeSyntax) -> (usize, char) {
    let b = text.as_bytes();
    debug_assert_eq!(b.get(i), Some(&b'\\'), "caller must point at a backslash");
    let Some(&next) = b.get(i + 1) else {
        // A trailing lone backslash is itself.
        return (i + 1, '\\');
    };
    match next {
        // Absolute values, as C uses, so no compiler's idea of `\n` can differ.
        b'a' => (i + 2, '\u{07}'),
        b'b' => (i + 2, '\u{08}'),
        b'f' => (i + 2, '\u{0C}'),
        b'n' => (i + 2, '\u{0A}'),
        b'r' => (i + 2, '\u{0D}'),
        b't' => (i + 2, '\u{09}'),
        b'v' => (i + 2, '\u{0B}'),
        b'\n' => (
            backslash_continuation_end(b, i).expect("caller matched a continuation"),
            ' ',
        ),
        b'x' => {
            let (end, value) = hex_run(b, i + 2, escapes.hex_escape_digits(), false);
            // No hex digit at all means the escape is a literal `x`.
            if end == i + 2 {
                (i + 2, 'x')
            } else {
                // Only the low byte survives, in every release.
                (end, scalar(value & 0xFF, escapes))
            }
        }
        b'u' => {
            let (end, value) = hex_run(b, i + 2, Some(4), true);
            if end == i + 2 {
                (i + 2, 'u')
            } else {
                (end, scalar(value, escapes))
            }
        }
        b'U' if escapes.has_wide_unicode() => {
            let (end, value) = hex_run(b, i + 2, Some(8), true);
            if end == i + 2 {
                (i + 2, 'U')
            } else {
                (end, scalar(value, escapes))
            }
        }
        b'0'..=b'7' => {
            let (end, value) = octal_run(b, i + 1, escapes);
            (end, scalar(value, escapes))
        }
        // Anything else — the punctuation escapes, a pre-8.6 `\U`, an unknown
        // letter — is the character itself. It may be multi-byte (`\é`, `\你`),
        // so advance by its real UTF-8 width rather than a fixed two bytes.
        _ => {
            let ch = text[i + 1..].chars().next().unwrap_or('\u{FFFD}');
            (i + 1 + ch.len_utf8(), ch)
        }
    }
}

/// The decoded scalar for `value`, degraded to U+FFFD when the release cannot
/// hold it: a surrogate (which no Rust `String` can store) in any release, and
/// anything past the basic multilingual plane in a UTF-16-internal one.
fn scalar(value: u32, escapes: EscapeSyntax) -> char {
    if escapes.is_bmp_only() && value > 0xFFFF {
        return '\u{FFFD}';
    }
    char::from_u32(value).unwrap_or('\u{FFFD}')
}

/// True when the escape starting at `text[i] == '\'` is a `\<newline>` line
/// continuation rather than an ordinary escape.
///
/// The classification a scanner needs when a continuation must behave as the
/// whitespace it substitutes to (`TclParseWhiteSpace` gives it `TYPE_SPACE`)
/// while every other escape is content. Release-invariant: no release moves the
/// newline out of the byte straight after the backslash.
pub(crate) fn is_line_continuation(text: &str, i: usize) -> bool {
    backslash_continuation_end(text.as_bytes(), i).is_some()
}

/// End of a raw `\<LF>` line continuation starting at `bytes[i] == b'\\'`.
///
/// The LF and the run of spaces and tabs after it all belong to the escape —
/// the whole span collapses to a single space. C's `TclParseBackslash` has no
/// CR arm: channel-level CRLF translation happens before this raw parser seam,
/// while `\<CR>` and raw `\<CRLF>` remain ordinary escaped data. This rule is
/// release-invariant and is shared with brace-word continuation collapse.
#[must_use]
pub fn backslash_continuation_end(bytes: &[u8], i: usize) -> Option<usize> {
    if bytes.get(i) != Some(&b'\\') || bytes.get(i + 1) != Some(&b'\n') {
        return None;
    }
    let mut j = i + 2;
    while matches!(bytes.get(j), Some(b' ' | b'\t')) {
        j += 1;
    }
    Some(j)
}

/// Scan the hex digits of an escape from `start`: where they end, and their
/// accumulated value.
///
/// `max` is the digit budget ([`None`] for 8.4/8.5's unbounded `\x`, where the
/// accumulation wraps and only the low byte is kept). `capped` applies C's
/// `ParseHex` guard, which refuses a further digit once the value has passed
/// `0x10FFF` (`tcl9.0.4/generic/tclParse.c:735`) — so `\UFFFFFFFF` consumes
/// five digits, not eight, and `\U00110000` consumes seven.
fn hex_run(b: &[u8], start: usize, max: Option<usize>, capped: bool) -> (usize, u32) {
    let budget = max.unwrap_or(usize::MAX);
    let mut end = start;
    let mut value = 0u32;
    let mut count = 0usize;
    while count < budget {
        let Some(&byte) = b.get(end) else { break };
        if !byte.is_ascii_hexdigit() || (capped && value > 0x0001_0FFF) {
            break;
        }
        let digit = char::from(byte).to_digit(16).expect("hex digit");
        value = value.wrapping_mul(16).wrapping_add(digit);
        end += 1;
        count += 1;
    }
    (end, value)
}

/// Scan the octal digits of an escape whose first digit is `b[start]`: where
/// they end, and their value.
///
/// One to three digits. Whether a *third* is taken is release-variant — 8.6
/// onward refuses it once the two-digit value has reached `0x20`
/// (`tcl8.6.16/generic/tclParse.c:977`), 8.4/8.5 always take it and truncate to
/// a byte — so `\400` is `\40` plus a literal `0` from 8.6 and a single NUL
/// before it.
fn octal_run(b: &[u8], start: usize, escapes: EscapeSyntax) -> (usize, u32) {
    let digit = |at: usize| match b.get(at) {
        Some(&c @ b'0'..=b'7') => Some(u32::from(c - b'0')),
        _ => None,
    };
    let first = digit(start).expect("caller peeked an octal digit");
    let Some(second) = digit(start + 1) else {
        return (start + 1, first);
    };
    let value = first * 8 + second;
    if !escapes.octal_takes_third_digit(value) {
        return (start + 2, value);
    }
    let Some(third) = digit(start + 2) else {
        return (start + 2, value);
    };
    (start + 3, (value * 8 + third) & 0xFF)
}

#[cfg(test)]
mod tests {
    use super::backslash_subst;
    use std::borrow::Cow;

    fn subst(s: &str) -> String {
        backslash_subst(s).into_owned()
    }

    #[test]
    fn no_backslash_is_borrowed() {
        let input = "plain ascii and \u{1F600} emoji";
        match backslash_subst(input) {
            Cow::Borrowed(borrowed) => assert_eq!(borrowed, input),
            Cow::Owned(_) => panic!("expected borrowed Cow for backslash-free input"),
        }
    }

    #[test]
    fn simple_letter_escapes() {
        assert_eq!(subst(r"\a"), "\x07");
        assert_eq!(subst(r"\b"), "\x08");
        assert_eq!(subst(r"\f"), "\x0C");
        assert_eq!(subst(r"\n"), "\n");
        assert_eq!(subst(r"\r"), "\r");
        assert_eq!(subst(r"\t"), "\t");
        assert_eq!(subst(r"\v"), "\x0B");
    }

    #[test]
    fn punctuation_escapes() {
        assert_eq!(subst(r"\\"), "\\");
        assert_eq!(subst(r"\{"), "{");
        assert_eq!(subst(r"\}"), "}");
        assert_eq!(subst(r"\["), "[");
        assert_eq!(subst(r"\]"), "]");
        assert_eq!(subst(r"\$"), "$");
        assert_eq!(subst(r#"\""#), "\"");
        assert_eq!(subst("\\ "), " ");
        assert_eq!(subst(r"\;"), ";");
    }

    #[test]
    fn lf_continuation() {
        assert_eq!(subst("hello\\\nworld"), "hello world");
    }

    #[test]
    fn raw_crlf_is_not_a_continuation() {
        assert_eq!(subst("hello\\\r\nworld"), "hello\r\nworld");
    }

    #[test]
    fn raw_cr_is_not_a_continuation() {
        assert_eq!(subst("hello\\\rworld"), "hello\rworld");
    }

    #[test]
    fn continuation_strips_leading_whitespace() {
        assert_eq!(subst("hello\\\n\t \tworld"), "hello world");
        assert_eq!(subst("hello\\\r\n   world"), "hello\r\n   world");
    }

    #[test]
    fn hex_escape_two_digits() {
        assert_eq!(subst(r"\x41"), "A");
        assert_eq!(subst(r"\xff"), "\u{FF}");
    }

    #[test]
    fn hex_escape_one_digit_stops_at_non_hex() {
        assert_eq!(subst(r"\x1g"), "\u{01}g");
    }

    #[test]
    fn hex_escape_no_digits_passes_letter() {
        assert_eq!(subst(r"\xg"), "xg");
    }

    #[test]
    fn unicode_escape_four_digits() {
        assert_eq!(subst(r"\u00E9"), "é");
        assert_eq!(subst(r"\u03A9"), "Ω");
    }

    #[test]
    fn unicode_escape_fewer_digits() {
        assert_eq!(subst(r"\u41"), "A");
        assert_eq!(subst(r"\u9"), "\t");
    }

    #[test]
    fn wide_unicode_escape() {
        assert_eq!(subst(r"\U0001F600"), "\u{1F600}");
        assert_eq!(subst(r"\U0010FFFF"), "\u{10FFFF}");
    }

    #[test]
    fn wide_unicode_escape_stops_before_invalid_scalar() {
        assert_eq!(subst(r"\U00110000"), "\u{11000}0");
        assert_eq!(subst(r"\UFFFFFFFF"), "\u{FFFFF}FFF");
        assert_eq!(subst(r"\U10FFFFF"), "\u{10FFFF}F");
        assert_eq!(subst(r"\U123456"), "\u{12345}6");
        assert_eq!(subst(r"\U00000041"), "A");
    }

    #[test]
    fn surrogate_pair_does_not_combine() {
        // C Tcl 9.0 keeps the two \u escapes as two surrogate code
        // units. Rust strings cannot represent those scalar values,
        // so the closest valid UTF-8 representation is two
        // replacement characters, not one U+1F600 character.
        let input = "\\u".to_owned() + "D83D" + "\\u" + "DE00";
        assert_eq!(subst(&input), "\u{FFFD}\u{FFFD}");

        let input = "\\u".to_owned() + "D800" + "\\u" + "DC00";
        assert_eq!(subst(&input), "\u{FFFD}\u{FFFD}");

        let input = "\\u".to_owned() + "DBFF" + "\\u" + "DFFF";
        assert_eq!(subst(&input), "\u{FFFD}\u{FFFD}");
    }

    #[test]
    fn surrogate_pair_with_surrounding_text_does_not_consume_following_escape() {
        let input = "pre".to_owned() + "\\u" + "D83D" + "\\u" + "DE00" + "post";
        assert_eq!(subst(&input), "pre\u{FFFD}\u{FFFD}post");
    }

    #[test]
    fn lone_high_surrogate_falls_back_to_replacement() {
        // No following \u — first surrogate maps to U+FFFD; the
        // trailing literal text passes through.
        let input = "\\u".to_owned() + "D800x";
        assert_eq!(subst(&input), "\u{FFFD}x");
    }

    #[test]
    fn high_surrogate_followed_by_non_low_surrogate_does_not_combine() {
        // \uD800 (high surrogate) followed by ASCII 'A' — high
        // surrogate is invalid on its own and becomes U+FFFD.
        let input = "\\u".to_owned() + "D800A";
        assert_eq!(subst(&input), "\u{FFFD}A");
    }

    #[test]
    fn high_surrogate_followed_by_unicode_escape_not_a_low_surrogate() {
        // \uD800 + A — the second escape is valid but not a low
        // surrogate, so the pair does not combine.
        let input = "\\u".to_owned() + "D800" + "\\u" + "0041";
        assert_eq!(subst(&input), "\u{FFFD}A");
    }

    #[test]
    fn surrogate_pair_with_short_low_unit_does_not_combine() {
        // Three digits then 'x' means the second \u parses as \uDE0
        // (a 3-digit unicode escape = U+0DE0, not a surrogate), then
        // 'x'. The high surrogate still degrades to U+FFFD.
        let input = "\\u".to_owned() + "D800" + "\\u" + "DE0x";
        assert_eq!(subst(&input), "\u{FFFD}\u{0DE0}x");
    }

    #[test]
    fn wide_unicode_escape_no_digits_passes_letter() {
        assert_eq!(subst(r"\Ug"), "Ug");
    }

    #[test]
    fn octal_escape() {
        assert_eq!(subst(r"\0"), "\0");
        assert_eq!(subst(r"\101"), "A");
        assert_eq!(subst(r"\7"), "\x07");
    }

    #[test]
    fn octal_escape_matches_tcl_byte_ceiling() {
        assert_eq!(subst(r"\377"), "\u{FF}");
        assert_eq!(subst(r"\378"), "\u{1F}8");
        assert_eq!(subst(r"\400"), " 0");
        assert_eq!(subst(r"\477"), "'7");
        assert_eq!(subst(r"\777"), "?7");
    }

    #[test]
    fn octal_escape_stops_at_non_octal() {
        assert_eq!(subst(r"\19"), "\x019");
        assert_eq!(subst(r"\78"), "\x078");
    }

    #[test]
    fn unknown_escape_passes_through() {
        assert_eq!(subst(r"\q"), "q");
        assert_eq!(subst(r"\!"), "!");
    }

    #[test]
    fn unknown_escape_with_multibyte_char() {
        assert_eq!(subst("\\é"), "é");
        assert_eq!(subst("\\\u{1F600}"), "\u{1F600}");
    }

    #[test]
    fn trailing_backslash() {
        assert_eq!(subst(r"foo\"), "foo\\");
    }

    #[test]
    fn mixed_content() {
        assert_eq!(subst(r"path\tis\nhere"), "path\tis\nhere");
        assert_eq!(subst(r"\x48\x65llo, \u0057orld\x21"), "Hello, World!");
    }

    #[test]
    fn multibyte_text_without_escapes_preserved() {
        let input = "résumé café";
        assert_eq!(subst(input), input);
    }

    #[test]
    fn multibyte_text_with_escapes() {
        assert_eq!(subst(r"\n café"), "\n café");
    }
}

/// The byte index one past the backslash escape starting at `text[i] == '\'`,
/// under **Tcl 9.0's** grammar.
///
/// The release-blind twin of [`backslash_escape_end_in`], paired with
/// [`backslash_subst`]. A consumer that knows its release must use the `_in`
/// pair for **both** calls: an escape's width and its value come from the same
/// scan, so mixing releases slices mid-escape.
#[must_use]
pub fn backslash_escape_end(text: &str, i: usize) -> usize {
    backslash_escape_end_in(text, i, EscapeSyntax::default())
}

/// The byte index one past the backslash escape starting at `text[i] == '\'`,
/// under `escapes`.
///
/// The *span* of an escape, for consumers that highlight or decode it one
/// escape at a time.  Widths match [`backslash_subst_in`] exactly — they are
/// literally the same scan — covering `\xNN` (two hex
/// digits from 8.6, unbounded before), `\uNNNN` (1–4 hex), `\UNNNNNNNN` (1–8
/// hex, 8.6 onward), `\NNN` (1–3 octal), the `\<newline>` line continuation
/// (LF, CR, or CRLF plus the run of spaces/tabs it absorbs, per
/// `TclParseBackslash`'s backslash-newline rule), else the backslash plus one
/// full character.
///
/// It lives beside the evaluator because it *is* the same rule.  Separate
/// hand-rolled copies had drifted — one consumed unbounded hex digits, one
/// never recognised `\U`, one assumed every escape was two bytes, one missed
/// the CRLF continuation — so `\x41` was tokenised as an escape `\x` plus a
/// string `41`.  The digits (and the continuation's whitespace) belong to the
/// escape.
///
/// The escaped character may be multi-byte (`\é`, `\你`, `\€`), so the fallback
/// advances by its real UTF-8 width: a fixed `+2` would slice inside the
/// character.
#[must_use]
pub fn backslash_escape_end_in(text: &str, i: usize, escapes: EscapeSyntax) -> usize {
    decode_escape(text, i, escapes).0
}

#[cfg(test)]
mod release_vector_tests {
    use super::{backslash_escape_end_in, backslash_subst_in};
    use tcl_dialect::EscapeSyntax::{self, Tcl84, Tcl86, Tcl90};

    /// One vector: the source, and per release the decoded string plus the
    /// number of bytes the escape at offset 0 consumes.
    struct Vector {
        src: &'static str,
        want: [(EscapeSyntax, &'static str, usize); 3],
    }

    /// Every release-variant (and a few release-invariant) escape, decoded and
    /// measured under each grammar. Value and width are asserted together
    /// because they come from one scan and must not drift apart.
    ///
    /// The 8.6 and 9.0 columns are tclsh-verified (8.6.14, 9.0.4); the 8.4/8.5
    /// column is read from `tcl8.4.20/generic/tclParse.c:553-685` and
    /// `tcl8.5.19/generic/tclParse.c:839-975`, which are identical here.
    const VECTORS: &[Vector] = &[
        // TIP 388: `\x` took every trailing hex digit and kept the low byte.
        Vector {
            src: r"\x41",
            want: [(Tcl84, "A", 4), (Tcl86, "A", 4), (Tcl90, "A", 4)],
        },
        Vector {
            src: r"\x4142",
            want: [(Tcl84, "B", 6), (Tcl86, "A42", 4), (Tcl90, "A42", 4)],
        },
        Vector {
            src: r"\xff",
            want: [
                (Tcl84, "\u{FF}", 4),
                (Tcl86, "\u{FF}", 4),
                (Tcl90, "\u{FF}", 4),
            ],
        },
        // No hex digit at all: a literal `x`, in every release.
        Vector {
            src: r"\xZZ",
            want: [(Tcl84, "xZZ", 2), (Tcl86, "xZZ", 2), (Tcl90, "xZZ", 2)],
        },
        Vector {
            src: r"\x1g",
            want: [
                (Tcl84, "\u{01}g", 3),
                (Tcl86, "\u{01}g", 3),
                (Tcl90, "\u{01}g", 3),
            ],
        },
        // `\U` is TIP 388 too — before it, a literal `U` and plain digits.
        Vector {
            src: r"\U0001F600",
            want: [
                (Tcl84, "U0001F600", 2),
                // 8.6's stock build is UTF-16-internal: the width is 9.0's,
                // the value degrades to U+FFFD.
                (Tcl86, "\u{FFFD}", 10),
                (Tcl90, "\u{1F600}", 10),
            ],
        },
        Vector {
            src: r"\U41",
            want: [(Tcl84, "U41", 2), (Tcl86, "A", 4), (Tcl90, "A", 4)],
        },
        // C's `ParseHex` stops once the accumulated value passes 0x10FFF, so
        // this consumes five digits — not eight — in every release that has it.
        Vector {
            src: r"\UFFFFFFFF",
            want: [
                (Tcl84, "UFFFFFFFF", 2),
                (Tcl86, "\u{FFFD}FFF", 7),
                (Tcl90, "\u{FFFFF}FFF", 7),
            ],
        },
        // `\u` never changed: at most four hex digits, always BMP.
        Vector {
            src: r"\u00E9",
            want: [(Tcl84, "é", 6), (Tcl86, "é", 6), (Tcl90, "é", 6)],
        },
        Vector {
            src: r"\u9",
            want: [(Tcl84, "\t", 3), (Tcl86, "\t", 3), (Tcl90, "\t", 3)],
        },
        // Octal: the third digit's guard is 8.6's.
        Vector {
            src: r"\101",
            want: [(Tcl84, "A", 4), (Tcl86, "A", 4), (Tcl90, "A", 4)],
        },
        Vector {
            src: r"\377",
            want: [
                (Tcl84, "\u{FF}", 4),
                (Tcl86, "\u{FF}", 4),
                (Tcl90, "\u{FF}", 4),
            ],
        },
        Vector {
            src: r"\400",
            // 8.4/8.5 take the third digit and truncate 0o400 to a byte: NUL.
            want: [(Tcl84, "\0", 4), (Tcl86, " 0", 3), (Tcl90, " 0", 3)],
        },
        Vector {
            src: r"\777",
            want: [(Tcl84, "\u{FF}", 4), (Tcl86, "?7", 3), (Tcl90, "?7", 3)],
        },
        Vector {
            src: r"\378",
            want: [
                (Tcl84, "\u{1F}8", 3),
                (Tcl86, "\u{1F}8", 3),
                (Tcl90, "\u{1F}8", 3),
            ],
        },
        // Release-invariant: simple letters, punctuation, a multi-byte escaped
        // character, and the line continuation.
        Vector {
            src: r"\n",
            want: [(Tcl84, "\n", 2), (Tcl86, "\n", 2), (Tcl90, "\n", 2)],
        },
        Vector {
            src: "\\é",
            want: [(Tcl84, "é", 3), (Tcl86, "é", 3), (Tcl90, "é", 3)],
        },
        Vector {
            src: "\\\n   x",
            want: [(Tcl84, " x", 5), (Tcl86, " x", 5), (Tcl90, " x", 5)],
        },
        Vector {
            src: "\\\r\n\t x",
            want: [
                (Tcl84, "\r\n\t x", 2),
                (Tcl86, "\r\n\t x", 2),
                (Tcl90, "\r\n\t x", 2),
            ],
        },
    ];

    #[test]
    fn decoded_value_and_consumed_width_agree_per_release() {
        for vector in VECTORS {
            for &(escapes, decoded, width) in &vector.want {
                assert_eq!(
                    &*backslash_subst_in(vector.src, escapes),
                    decoded,
                    "decode {:?} under {escapes:?}",
                    vector.src
                );
                assert_eq!(
                    backslash_escape_end_in(vector.src, 0, escapes),
                    width,
                    "extent of {:?} under {escapes:?}",
                    vector.src
                );
            }
        }
    }

    /// The extent must be exactly the span the decoder turns into one
    /// character: decoding the measured slice yields a single char, and
    /// decoding the tail separately reproduces the whole result.
    #[test]
    fn the_extent_is_the_span_the_decoder_consumes() {
        for vector in VECTORS {
            for &(escapes, decoded, width) in &vector.want {
                let head = backslash_subst_in(&vector.src[..width], escapes);
                assert_eq!(
                    head.chars().count(),
                    1,
                    "escape slice of {:?} under {escapes:?} is not one char",
                    vector.src
                );
                let tail = backslash_subst_in(&vector.src[width..], escapes);
                assert_eq!(
                    format!("{head}{tail}"),
                    decoded,
                    "split decode of {:?} under {escapes:?}",
                    vector.src
                );
            }
        }
    }

    #[test]
    fn the_release_blind_entry_points_are_the_tcl9_ones() {
        for vector in VECTORS {
            assert_eq!(
                super::backslash_subst(vector.src),
                backslash_subst_in(vector.src, Tcl90),
                "{:?}",
                vector.src
            );
            assert_eq!(
                super::backslash_escape_end(vector.src, 0),
                backslash_escape_end_in(vector.src, 0, Tcl90),
                "{:?}",
                vector.src
            );
        }
    }

    #[test]
    fn split_segments_follow_the_release_widths() {
        // `\x4142` is one six-byte escape under 8.5 and a four-byte escape plus
        // literal `42` under 8.6 — a highlighter must colour them differently.
        let pieces = |escapes| {
            super::split_backslash_escapes_in(r"a\x4142z", escapes)
                .into_iter()
                .map(|s| (&r"a\x4142z"[s.start..s.end], s.is_escape))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            pieces(Tcl84),
            vec![("a", false), (r"\x4142", true), ("z", false)]
        );
        assert_eq!(
            pieces(Tcl90),
            vec![("a", false), (r"\x41", true), ("42z", false)]
        );
    }
}

#[cfg(test)]
mod escape_end_tests {
    use super::backslash_escape_end;

    #[test]
    fn widths_match_the_evaluator() {
        for (src, want) in [
            (r"\n", r"\n"),
            (r"\\", r"\\"),
            (r"\x41", r"\x41"),
            (r"\x4", r"\x4"),
            // Capped at two hex digits — the third is string content.
            (r"\x414", r"\x41"),
            (r"\é", r"\é"),
            (r"\U0001F600", r"\U0001F600"),
            // Octal, capped at three digits.
            (r"\101", r"\101"),
            (r"\1012", r"\101"),
            // `\x` with no hex digit is a literal `x`.
            (r"\xz", r"\x"),
        ] {
            assert_eq!(&src[..backslash_escape_end(src, 0)], want, "for {src:?}");
        }
    }

    #[test]
    fn multibyte_escaped_char_is_not_sliced() {
        for src in [r"\é", r"\你", r"\€"] {
            let end = backslash_escape_end(src, 0);
            assert!(src.is_char_boundary(end), "sliced inside a char: {src:?}");
            assert_eq!(end, src.len());
        }
    }

    #[test]
    fn continuation_spans_newline_and_following_whitespace() {
        // Only a raw `\<LF>` absorbs following spaces/tabs. C's
        // `TclParseBackslash` has no CR arm; a source channel may translate
        // CRLF before parsing, but the raw parser keeps it.
        assert_eq!(backslash_escape_end("\\\n   x", 0), 5);
        assert_eq!(backslash_escape_end("\\\r\n\t x", 0), 2);
        assert_eq!(backslash_escape_end("\\\rx", 0), 2);
        assert_eq!(backslash_escape_end("\\\n", 0), 2);
        // FP guard: an escaped backslash is a 2-byte escape — a newline after
        // it is content, not part of a continuation.
        assert_eq!(backslash_escape_end("\\\\\n  x", 0), 2);
    }
}

/// One piece of a string split around its backslash escapes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EscapeSegment {
    /// Byte offset of the first byte, relative to the text passed in.
    pub start: usize,
    /// Byte offset one past the last byte.
    pub end: usize,
    /// Whether this piece is a backslash escape (rather than a literal run).
    pub is_escape: bool,
}

/// Split `text` into alternating literal runs and backslash escapes.
///
/// Every highlighter that colours a Tcl string has to do this — the Tcl token
/// walker, the APL lexer, the BIG-IP config lexer — and each had grown its own
/// copy. They drifted: one consumed unbounded hex digits, one never recognised
/// `\U`, one assumed every escape was two bytes, so `\x41` was tokenised as an
/// escape `\x` plus a string `41`. One rule, one implementation, beside the
/// [`backslash_subst`] evaluator that defines it.
///
/// Segments are contiguous and cover `text` exactly; a text with no backslash
/// yields a single literal segment (or none, when empty). Widths come from
/// [`backslash_escape_end`], so this splits under Tcl 9.0's grammar; a caller
/// with a dialect in scope uses [`split_backslash_escapes_in`].
#[must_use]
pub fn split_backslash_escapes(text: &str) -> Vec<EscapeSegment> {
    split_backslash_escapes_in(text, EscapeSyntax::default())
}

/// [`split_backslash_escapes`] under `escapes` — the release-aware form, whose
/// escape segments are exactly as wide as that release's decoder consumes.
#[must_use]
pub fn split_backslash_escapes_in(text: &str, escapes: EscapeSyntax) -> Vec<EscapeSegment> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut run = 0usize;
    let mut i = 0usize;

    while i < bytes.len() {
        // A trailing lone backslash is literal — there is nothing to escape.
        if bytes[i] != b'\\' || i + 1 >= bytes.len() {
            i += 1;
            continue;
        }
        let end = backslash_escape_end_in(text, i, escapes);
        if run < i {
            out.push(EscapeSegment {
                start: run,
                end: i,
                is_escape: false,
            });
        }
        out.push(EscapeSegment {
            start: i,
            end,
            is_escape: true,
        });
        i = end;
        run = end;
    }
    if run < bytes.len() {
        out.push(EscapeSegment {
            start: run,
            end: bytes.len(),
            is_escape: false,
        });
    }
    out
}

#[cfg(test)]
mod split_escape_tests {
    use super::{EscapeSegment, split_backslash_escapes};

    fn pieces(text: &str) -> Vec<(&str, bool)> {
        split_backslash_escapes(text)
            .into_iter()
            .map(|s| (&text[s.start..s.end], s.is_escape))
            .collect()
    }

    #[test]
    fn splits_around_escapes_of_every_width() {
        assert_eq!(
            pieces(r"a\x41b\U0001F600c\101d"),
            vec![
                ("a", false),
                (r"\x41", true),
                ("b", false),
                (r"\U0001F600", true),
                ("c", false),
                (r"\101", true),
                ("d", false),
            ]
        );
    }

    #[test]
    fn no_backslash_is_one_literal_run() {
        assert_eq!(pieces("plain"), vec![("plain", false)]);
        assert!(split_backslash_escapes("").is_empty());
    }

    #[test]
    fn a_trailing_lone_backslash_is_literal() {
        assert_eq!(pieces(r"ab\"), vec![(r"ab\", false)]);
    }

    /// Segments must tile the input exactly — no gaps, no overlaps.
    #[test]
    fn segments_cover_the_text_exactly() {
        for text in [r"a\nb", r"\x41", "plain", r"\\", r"a\é b", ""] {
            let segs: Vec<EscapeSegment> = split_backslash_escapes(text);
            let mut at = 0usize;
            for s in &segs {
                assert_eq!(s.start, at, "gap/overlap in {text:?}: {segs:?}");
                at = s.end;
            }
            assert_eq!(at, text.len(), "does not reach the end of {text:?}");
        }
    }
}
