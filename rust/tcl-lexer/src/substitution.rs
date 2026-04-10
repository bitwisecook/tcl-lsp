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
            'u' => scan_hex_escape(&mut out, text, escape_i, 4, &mut chars),
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
