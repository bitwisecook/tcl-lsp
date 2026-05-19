//! Tcl backslash escape processing.
//!
//! Ports `core/parsing/substitution.py::backslash_subst` as idiomatic Rust:
//! zero-copy on the fast path (no backslash in the input), a single
//! forward scan of `char_indices` for the slow path, and a clean match
//! table for escape dispatch. The function is callable directly from
//! Rust and exposed to Python via the `tcl-lsp-rust` binding crate.

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
/// - Octal escapes: `\NNN` (1–3 octal digits)
///
/// Any other `\X` passes through as the character `X`.
///
/// Returns [`Cow::Borrowed`] when `text` contains no backslash (no
/// allocation) and [`Cow::Owned`] otherwise. Invalid code points (e.g.
/// `\UFFFFFFFF`, lone surrogates) map to the Unicode replacement
/// character U+FFFD, matching what any valid-UTF-8 sink would ultimately
/// render. The Python reference implementation produces lone-surrogate
/// `str` objects in those edge cases; Rust `String` cannot, so we pick
/// the closest valid-UTF-8 approximation.
///
/// `\u`-escaped high-surrogate codepoints (U+D800–U+DBFF) immediately
/// followed by a `\u`-escaped low surrogate (U+DC00–U+DFFF) combine
/// into the single supplementary-plane codepoint they encode, matching
/// Tcl 9 `tclParse.c::TclParseBackslash`.
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
            'U' => scan_hex_escape(&mut out, text, escape_i, 8, &mut chars),
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
    if first == '\r' {
        if let Some(&(_, '\n')) = chars.peek() {
            chars.next();
        }
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

/// Parse `\uHHHH` (1–4 hex digits) and, when the parsed codepoint is a
/// high surrogate immediately followed by a `\uLLLL` low surrogate,
/// combine the pair into the supplementary-plane codepoint they
/// encode.  Mirrors the Tcl 9 `TclParseBackslash` behaviour the
/// Python reference implementation gained in PR #426.
fn scan_unicode_escape(
    out: &mut String,
    text: &str,
    escape_i: usize,
    chars: &mut CharIndices<'_>,
) {
    let letter = chars.next().expect("caller peeked 'u'").1;
    let digits_start = escape_i + 1;
    let digits_end = scan_digits(text, digits_start, 4, chars, |c| c.is_ascii_hexdigit());
    if digits_end == digits_start {
        out.push(letter);
        return;
    }
    let digits = &text[digits_start..digits_end];
    let value = u32::from_str_radix(digits, 16).expect("hex digits parse");

    // Surrogate-pair combining: if `value` is a high surrogate and the
    // next six bytes are `\u` + 4 hex digits encoding a low surrogate,
    // emit the combined supplementary-plane codepoint.  The lookahead
    // peeks at raw bytes — backslash, `u`, and hex digits are all
    // ASCII so byte-level slicing is on character boundaries.
    if (0xD800..=0xDBFF).contains(&value) && text.len() >= digits_end + 6 {
        let bytes = text.as_bytes();
        let ok_prefix = bytes[digits_end] == b'\\' && bytes[digits_end + 1] == b'u';
        let low_digits = &bytes[digits_end + 2..digits_end + 6];
        let ok_hex = low_digits.iter().all(u8::is_ascii_hexdigit);
        if ok_prefix && ok_hex {
            let low_str = &text[digits_end + 2..digits_end + 6];
            let low = u32::from_str_radix(low_str, 16).expect("hex digits parse");
            if (0xDC00..=0xDFFF).contains(&low) {
                let combined = 0x10000 + ((value - 0xD800) << 10) + (low - 0xDC00);
                out.push(char::from_u32(combined).unwrap_or('\u{FFFD}'));
                // Advance past the consumed `\uLLLL` (6 ASCII bytes = 6 chars).
                for _ in 0..6 {
                    chars.next();
                }
                return;
            }
        }
    }

    out.push(char::from_u32(value).unwrap_or('\u{FFFD}'));
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
    let digits_end = scan_digits(text, digits_start, 3, chars, |c| matches!(c, '0'..='7'));
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
    }

    #[test]
    fn surrogate_pair_combines() {
        // U+1F600 GRINNING FACE = D83D + DE00 (UTF-16 surrogate pair).
        let input = "\\u".to_owned() + "D83D" + "\\u" + "DE00";
        assert_eq!(subst(&input), "\u{1F600}");
        // U+10000 = D800 + DC00 (smallest supplementary codepoint).
        let input = "\\u".to_owned() + "D800" + "\\u" + "DC00";
        assert_eq!(subst(&input), "\u{10000}");
        // U+10FFFF (max valid codepoint) = DBFF + DFFF.
        let input = "\\u".to_owned() + "DBFF" + "\\u" + "DFFF";
        assert_eq!(subst(&input), "\u{10FFFF}");
    }

    #[test]
    fn surrogate_pair_with_surrounding_text() {
        // The combine path needs to advance the iterator past the
        // consumed low-surrogate escape; verify trailing chars
        // still appear in the output.
        let input = "pre".to_owned() + "\\u" + "D83D" + "\\u" + "DE00" + "post";
        assert_eq!(subst(&input), format!("pre{}post", '\u{1F600}'));
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
        // The low-surrogate unit must be exactly 4 hex digits.  Three
        // digits then 'x' means the second \u parses as \uDE0 (a
        // 3-digit unicode escape = U+0DE0, not a surrogate), then 'x'
        // — and the combine condition (exactly 4 trailing hex digits)
        // fails so the high surrogate also degrades to U+FFFD.
        let input = "\\u".to_owned() + "D800" + "\\u" + "DE0x";
        assert_eq!(subst(&input), "\u{FFFD}\u{0DE0}x");
    }

    #[test]
    fn wide_unicode_out_of_range_is_replacement() {
        assert_eq!(subst(r"\UFFFFFFFF"), "\u{FFFD}");
    }

    #[test]
    fn octal_escape() {
        assert_eq!(subst(r"\0"), "\0");
        assert_eq!(subst(r"\101"), "A");
        assert_eq!(subst(r"\7"), "\x07");
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
