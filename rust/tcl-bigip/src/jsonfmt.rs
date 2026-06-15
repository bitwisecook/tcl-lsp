//! Minimal JSON helpers for the hand-built reports (graph / stats / cleanup),
//! matching Python's `json.dumps(..., indent=2)` exactly — including
//! `ensure_ascii=True`, which escapes every non-ASCII char as `\uXXXX`
//! (surrogate pairs for astral code points). `serde_json` emits raw UTF-8, so a
//! bespoke escaper is needed for byte-parity.

/// A `json.dumps(ensure_ascii=True)`-compatible quoted string literal.
#[must_use]
pub fn json_string(s: &str) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c if c.is_ascii() => out.push(c),
            c => {
                let mut buf = [0u16; 2];
                for unit in c.encode_utf16(&mut buf) {
                    let _ = write!(out, "\\u{unit:04x}");
                }
            }
        }
    }
    out.push('"');
    out
}
