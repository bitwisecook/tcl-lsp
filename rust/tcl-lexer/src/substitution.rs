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

/// Process Tcl backslash escapes in `text`.
///
/// Recognises:
///
/// - Simple mappings: `\a \b \f \n \r \t \v \\ \{ \} \[ \] \$ \" \<space> \;`
/// - Line continuation: `\<LF>`, `\<CR>`, or `\<CRLF>` followed by any
///   run of space/tab, collapsed to a single ASCII space.
/// - Hex escapes: `\xNN` (1–2 hex digits)
/// - Unicode escapes: `\uNNNN` (1–4 hex digits)
/// - Wide unicode escapes: `\UNNNNNNNN` (1–8 hex digits)
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
///
#[must_use]
pub fn backslash_subst(text: &str) -> Cow<'_, str> {
    if !text.contains('\\') {
        return Cow::Borrowed(text);
    }

    let mut out = String::with_capacity(text.len());
    let mut chars = text.char_indices().peekable();

    while let Some((_, ch)) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        let Some(&(escape_i, escape_ch)) = chars.peek() else {
            out.push('\\');
            break;
        };

        match escape_ch {
            'a' => push_simple(&mut out, '\x07', &mut chars),
            'b' => push_simple(&mut out, '\x08', &mut chars),
            'f' => push_simple(&mut out, '\x0C', &mut chars),
            'n' => push_simple(&mut out, '\n', &mut chars),
            'r' => push_simple(&mut out, '\r', &mut chars),
            't' => push_simple(&mut out, '\t', &mut chars),
            'v' => push_simple(&mut out, '\x0B', &mut chars),
            '\\' | '{' | '}' | '[' | ']' | '$' | '"' | ' ' | ';' => {
                push_simple(&mut out, escape_ch, &mut chars);
            }
            '\n' | '\r' => consume_line_continuation(&mut out, escape_ch, &mut chars),
            'x' => scan_hex_escape(&mut out, text, escape_i, 2, &mut chars),
            'u' => scan_unicode_escape(&mut out, text, escape_i, &mut chars),
            'U' => scan_wide_unicode_escape(&mut out, &mut chars),
            '0'..='7' => scan_octal_escape(&mut out, text, escape_i, &mut chars),
            _ => {
                // Unknown escape: pass through the char after the backslash.
                out.push(escape_ch);
                chars.next();
            }
        }
    }

    Cow::Owned(out)
}

type CharIndices<'s> = std::iter::Peekable<std::str::CharIndices<'s>>;

#[inline]
fn push_simple(out: &mut String, resolved: char, chars: &mut CharIndices<'_>) {
    out.push(resolved);
    chars.next();
}

fn consume_line_continuation(out: &mut String, first: char, chars: &mut CharIndices<'_>) {
    chars.next(); // consume the LF or CR
    if first == '\r'
        && let Some(&(_, '\n')) = chars.peek()
    {
        chars.next();
    }
    while let Some(&(_, c)) = chars.peek() {
        if c == ' ' || c == '\t' {
            chars.next();
        } else {
            break;
        }
    }
    out.push(' ');
}

/// Parse `\uHHHH` (1–4 hex digits). C Tcl 9.0 keeps surrogate code
/// units as independent string elements; Rust `String` cannot store
/// surrogate scalar values, so they degrade to U+FFFD instead of being
/// combined with a following `\u` escape.
fn scan_unicode_escape(out: &mut String, text: &str, escape_i: usize, chars: &mut CharIndices<'_>) {
    let letter = chars.next().expect("caller peeked 'u'").1;
    let digits_start = escape_i + 1;
    let digits_end = scan_digits(text, digits_start, 4, chars, |c| c.is_ascii_hexdigit());
    if digits_end == digits_start {
        out.push(letter);
        return;
    }
    let digits = &text[digits_start..digits_end];
    let value = u32::from_str_radix(digits, 16).expect("hex digits parse");

    out.push(char::from_u32(value).unwrap_or('\u{FFFD}'));
}

fn scan_wide_unicode_escape(out: &mut String, chars: &mut CharIndices<'_>) {
    let letter = chars.next().expect("caller peeked 'U'").1;
    let mut value = 0u32;
    let mut consumed = 0usize;
    while consumed < 8 {
        let Some(&(_, c)) = chars.peek() else { break };
        let Some(digit) = c.to_digit(16) else { break };
        let Some(next) = value.checked_mul(16).and_then(|v| v.checked_add(digit)) else {
            break;
        };
        if next > 0x0010_FFFF {
            break;
        }
        chars.next();
        value = next;
        consumed += 1;
    }
    if consumed == 0 {
        out.push(letter);
    } else {
        out.push(char::from_u32(value).unwrap_or('\u{FFFD}'));
    }
}

fn scan_hex_escape(
    out: &mut String,
    text: &str,
    escape_i: usize,
    max_digits: usize,
    chars: &mut CharIndices<'_>,
) {
    let letter = chars.next().expect("caller peeked a valid escape letter").1;
    let digits_start = escape_i + 1;
    let digits_end = scan_digits(text, digits_start, max_digits, chars, |c| {
        c.is_ascii_hexdigit()
    });
    if digits_end > digits_start {
        let digits = &text[digits_start..digits_end];
        let value = u32::from_str_radix(digits, 16).expect("hex digits parse");
        out.push(char::from_u32(value).unwrap_or('\u{FFFD}'));
    } else {
        // No hex digits followed → pass the letter through literally.
        out.push(letter);
    }
}

fn scan_octal_escape(out: &mut String, text: &str, escape_i: usize, chars: &mut CharIndices<'_>) {
    // The first octal digit is still in the peek buffer; the helper
    // consumes it.
    let digits_start = escape_i;
    let max_digits = match chars.peek().map(|(_, c)| *c) {
        Some('0'..='3') => 3,
        Some('4'..='7') => 2,
        _ => unreachable!("caller peeked an octal digit"),
    };
    let digits_end = scan_digits(text, digits_start, max_digits, chars, |c| {
        matches!(c, '0'..='7')
    });
    let digits = &text[digits_start..digits_end];
    let value = u32::from_str_radix(digits, 8).expect("octal digits parse");
    out.push(char::from_u32(value).unwrap_or('\u{FFFD}'));
}

fn scan_digits<F>(
    _text: &str,
    start: usize,
    max: usize,
    chars: &mut CharIndices<'_>,
    is_digit: F,
) -> usize
where
    F: Fn(char) -> bool,
{
    let mut end = start;
    let mut count = 0;
    while count < max {
        let Some(&(_, c)) = chars.peek() else { break };
        if !is_digit(c) {
            break;
        }
        chars.next();
        // Digits are ASCII by construction, so each consumed digit
        // advances by exactly one byte.
        end += 1;
        count += 1;
    }
    end
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
    fn crlf_continuation() {
        assert_eq!(subst("hello\\\r\nworld"), "hello world");
    }

    #[test]
    fn cr_continuation() {
        assert_eq!(subst("hello\\\rworld"), "hello world");
    }

    #[test]
    fn continuation_strips_leading_whitespace() {
        assert_eq!(subst("hello\\\r\n   world"), "hello world");
        assert_eq!(subst("hello\\\n\t \tworld"), "hello world");
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

/// The byte index one past the backslash escape starting at `text[i] == '\'`.
///
/// The *span* of an escape, for consumers that highlight it rather than
/// evaluate it.  Widths match [`backslash_subst`] exactly — `\xNN` (1–2 hex),
/// `\uNNNN` (1–4 hex), `\UNNNNNNNN` (1–8 hex), `\NNN` (1–3 octal), else the
/// backslash plus one full character.
///
/// It lives beside the evaluator because it *is* the same rule.  Three separate
/// hand-rolled copies had drifted — one consumed unbounded hex digits, one never
/// recognised `\U`, one assumed every escape was two bytes — so `\x41` was
/// tokenised as an escape `\x` plus a string `41`.  The digits belong to the
/// escape.
///
/// The escaped character may be multi-byte (`\é`, `\你`, `\€`), so the fallback
/// advances by its real UTF-8 width: a fixed `+2` would slice inside the
/// character.
#[must_use]
pub fn backslash_escape_end(text: &str, i: usize) -> usize {
    let b = text.as_bytes();
    debug_assert_eq!(b.get(i), Some(&b'\\'), "caller must point at a backslash");

    let hex_run = |start: usize, max: usize| {
        let mut j = start;
        while j < b.len() && j < start + max && b[j].is_ascii_hexdigit() {
            j += 1;
        }
        // `\x` with no hex digit at all is a literal `x`, not a hex escape.
        (j > start).then_some(j)
    };
    match b.get(i + 1) {
        None => i + 1,
        Some(b'x') => hex_run(i + 2, 2).unwrap_or(i + 2),
        Some(b'u') => hex_run(i + 2, 4).unwrap_or(i + 2),
        Some(b'U') => hex_run(i + 2, 8).unwrap_or(i + 2),
        Some(b'0'..=b'7') => {
            let mut j = i + 2;
            while j < b.len() && j < i + 4 && matches!(b[j], b'0'..=b'7') {
                j += 1;
            }
            j
        }
        Some(_) => i + 1 + text[i + 1..].chars().next().map_or(1, char::len_utf8),
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
/// [`backslash_escape_end`].
#[must_use]
pub fn split_backslash_escapes(text: &str) -> Vec<EscapeSegment> {
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
        let end = backslash_escape_end(text, i);
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
