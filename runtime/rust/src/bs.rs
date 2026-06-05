//! Tcl backslash-escape decoding (T1.2).
//!
//! Ports `runtime/zig/valtypes/tcl_bs.zig`, itself a port of reference Tcl's
//! `TclParseBackslash` / `Tcl_UtfBackslash` (`tmp/tcl9.0.3/generic/tclParse.c`).
//! [`consume_one`] decodes a single `\x` sequence; [`decode_span`] loops it
//! across a whole span.
//!
//! Pure, borrow-based, **`unsafe`-free** — it operates on `&[u8]` and writes to
//! caller slices. (Module-level `forbid(unsafe_code)` enforces that.)
//!
//! Fidelity notes (byte-for-byte with reference Tcl):
//! - `\xNN` (1–2 hex), `\uNNNN` (≤4 hex), `\UNNNNNNNN` (≤8 hex), octal `\NNN`
//!   (≤3 octal), the C-escapes `\n \t \r \a \b \f \v`, and `\<whitespace>`
//!   folding (with `\<newline>` additionally eating following spaces/tabs).
//! - `\X` is **not** `\x` (Tcl is case-sensitive); `\8`/`\9` are **not** octal.
//!   Both fall through to "unknown escape → emit the byte verbatim".
//! - `\u`/`\U` use a **raw** UTF-8 encoder (no surrogate/range validation), so
//!   `\uD800` and out-of-range values encode the same bytes Tcl emits — this is
//!   why we don't use Rust's `char` (which rejects those).

#![forbid(unsafe_code)]

/// Raw UTF-8 encode a codepoint into `out` (≥4 bytes), returning the byte
/// count. Matches Tcl's encoder: **no** validity checks, so surrogate and
/// out-of-range codepoints encode rather than being rejected.
fn encode_utf8(cp: u32, out: &mut [u8]) -> usize {
    if cp < 0x80 {
        out[0] = cp as u8;
        1
    } else if cp < 0x800 {
        out[0] = (0xC0 | (cp >> 6)) as u8;
        out[1] = (0x80 | (cp & 0x3F)) as u8;
        2
    } else if cp < 0x10000 {
        out[0] = (0xE0 | (cp >> 12)) as u8;
        out[1] = (0x80 | ((cp >> 6) & 0x3F)) as u8;
        out[2] = (0x80 | (cp & 0x3F)) as u8;
        3
    } else {
        out[0] = (0xF0 | (cp >> 18)) as u8;
        out[1] = (0x80 | ((cp >> 12) & 0x3F)) as u8;
        out[2] = (0x80 | ((cp >> 6) & 0x3F)) as u8;
        out[3] = (0x80 | (cp & 0x3F)) as u8;
        4
    }
}

fn hex_val(c: u8) -> Option<u32> {
    match c {
        b'0'..=b'9' => Some((c - b'0') as u32),
        b'a'..=b'f' => Some((c - b'a' + 10) as u32),
        b'A'..=b'F' => Some((c - b'A' + 10) as u32),
        _ => None,
    }
}

/// Decode one Tcl backslash escape. `si` is the index of the **first byte
/// after** the backslash (the caller has consumed the `\` and verified
/// `si < src.len()`). Writes up to 4 UTF-8 bytes into `out` (must be ≥4) and
/// returns `(next_si, written)` so the caller can advance both cursors.
pub fn consume_one(src: &[u8], si: usize, out: &mut [u8]) -> (usize, usize) {
    let len = src.len();
    let ch = src[si];
    match ch {
        b'n' => {
            out[0] = b'\n';
            (si + 1, 1)
        }
        b't' => {
            out[0] = b'\t';
            (si + 1, 1)
        }
        b'r' => {
            out[0] = b'\r';
            (si + 1, 1)
        }
        b'a' => {
            out[0] = 0x07;
            (si + 1, 1)
        }
        b'b' => {
            out[0] = 0x08;
            (si + 1, 1)
        }
        b'f' => {
            out[0] = 0x0C;
            (si + 1, 1)
        }
        b'v' => {
            out[0] = 0x0B;
            (si + 1, 1)
        }
        b'x' => {
            // `\xNN` — 1 or 2 hex digits. `\X` is NOT recognised (falls to the
            // default arm). Tcl truncates the accumulated value to one byte.
            let mut p = si + 1;
            let mut val: u32 = 0;
            let mut ndig = 0;
            while ndig < 2 && p < len {
                match hex_val(src[p]) {
                    Some(d) => {
                        val = val * 16 + d;
                        p += 1;
                        ndig += 1;
                    }
                    None => break,
                }
            }
            out[0] = (val & 0xFF) as u8;
            (p, 1)
        }
        b'u' => {
            // `\uNNNN` — up to 4 hex digits → UTF-8.
            let mut p = si + 1;
            let mut cp: u32 = 0;
            let mut ndig = 0;
            while ndig < 4 && p < len {
                match hex_val(src[p]) {
                    Some(d) => {
                        cp = cp * 16 + d;
                        p += 1;
                        ndig += 1;
                    }
                    None => break,
                }
            }
            (p, encode_utf8(cp, out))
        }
        b'U' => {
            // `\UNNNNNNNN` — up to 8 hex digits → UTF-8.
            let mut p = si + 1;
            let mut cp: u32 = 0;
            let mut ndig = 0;
            while ndig < 8 && p < len {
                match hex_val(src[p]) {
                    Some(d) => {
                        cp = cp * 16 + d;
                        p += 1;
                        ndig += 1;
                    }
                    None => break,
                }
            }
            (p, encode_utf8(cp, out))
        }
        b'0'..=b'7' => {
            // `\NNN` — up to 3 octal digits. `\8`/`\9` are not octal (default arm).
            let mut p = si;
            let mut val: u32 = 0;
            let mut ndig = 0;
            while ndig < 3 && p < len && src[p] >= b'0' && src[p] <= b'7' {
                val = val * 8 + (src[p] - b'0') as u32;
                p += 1;
                ndig += 1;
            }
            out[0] = (val & 0xFF) as u8;
            (p, 1)
        }
        b' ' | b'\n' | b'\t' | b'\r' => {
            // `\<whitespace>` → single space; `\<newline>` additionally eats
            // following spaces/tabs.
            out[0] = b' ';
            let mut p = si + 1;
            if ch == b'\n' {
                while p < len && (src[p] == b' ' || src[p] == b'\t') {
                    p += 1;
                }
            }
            (p, 1)
        }
        _ => {
            // Unknown escape — emit the byte verbatim (Tcl behaviour).
            out[0] = ch;
            (si + 1, 1)
        }
    }
}

/// Decode a whole span through the backslash rules, returning the substituted
/// bytes. A trailing lone `\` (no following byte) is emitted verbatim, matching
/// reference Tcl.
pub fn decode_span(src: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(src.len());
    let mut buf = [0u8; 4];
    let mut si = 0;
    while si < src.len() {
        if src[si] == b'\\' && si + 1 < src.len() {
            let (next, written) = consume_one(src, si + 1, &mut buf);
            out.extend_from_slice(&buf[..written]);
            si = next;
        } else {
            out.push(src[si]);
            si += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn c_escapes() {
        assert_eq!(decode_span(b"a\\nb\\tc"), b"a\nb\tc");
        assert_eq!(
            decode_span(b"\\a\\b\\f\\v\\r"),
            [0x07, 0x08, 0x0C, 0x0B, b'\r']
        );
    }

    #[test]
    fn hex_escape_one_and_two_digits() {
        assert_eq!(decode_span(b"\\x41"), b"A");
        assert_eq!(decode_span(b"\\x4"), [0x04]);
        // third hex digit is literal: `\x41` + `1`? No — only 2 digits consumed.
        assert_eq!(decode_span(b"\\x411"), b"A1");
        // capital `\X` is NOT a hex escape — unknown escape emits 'X'.
        assert_eq!(decode_span(b"\\X41"), b"X41");
    }

    #[test]
    fn unicode_escapes() {
        assert_eq!(decode_span(b"\\u0041"), b"A");
        assert_eq!(decode_span(b"\\u00e9"), "é".as_bytes()); // U+00E9 → 2 bytes
        assert_eq!(decode_span(b"\\U0001F600"), "😀".as_bytes()); // 4 bytes
    }

    #[test]
    fn octal_escape_and_non_octal() {
        assert_eq!(decode_span(b"\\101"), b"A"); // 0o101 == 65 == 'A'
        assert_eq!(decode_span(b"\\0"), [0u8]);
        // `\8` is not octal → literal '8'.
        assert_eq!(decode_span(b"\\8"), b"8");
    }

    #[test]
    fn whitespace_folding() {
        // `\<space>` → single space.
        assert_eq!(decode_span(b"a\\ b"), b"a b");
        // `\<newline>` → space, and eats following spaces/tabs.
        assert_eq!(decode_span(b"a\\\n   b"), b"a b");
    }

    #[test]
    fn unknown_and_trailing() {
        assert_eq!(decode_span(b"\\$"), b"$");
        assert_eq!(decode_span(b"\\{"), b"{");
        // trailing lone backslash is literal
        assert_eq!(decode_span(b"ab\\"), b"ab\\");
    }
}
