//! Pretty JSON serialization for the remote verbs.
//!
//! The output uses two-space indentation, `": "` / `", "` separators, and
//! ASCII-only escaping: every non-ASCII scalar is escaped to `\uXXXX` (astral
//! characters as UTF-16 surrogate pairs). `serde_json::to_string_pretty` does
//! not do this. This module wraps `PrettyFormatter` to produce the ASCII-only
//! output (`preserve_order` keeps insertion order so keys match too).

use serde::Serialize;
use serde_json::Value;
use serde_json::ser::{Formatter, PrettyFormatter, Serializer};
use std::io::{self, Write};

/// A `Formatter` that emits two-space-indented JSON: the standard pretty
/// layout plus ASCII-only escaping of every non-ASCII character.
struct AsciiPretty<'a> {
    inner: PrettyFormatter<'a>,
}

impl AsciiPretty<'_> {
    fn new() -> Self {
        AsciiPretty {
            inner: PrettyFormatter::with_indent(b"  "),
        }
    }
}

/// Delegate one structural `Formatter` method to the wrapped `PrettyFormatter`
/// so indentation / separators stay two-space-indented.
macro_rules! delegate {
    ($name:ident ( $($arg:ident : $ty:ty),* )) => {
        fn $name<W>(&mut self, writer: &mut W $(, $arg: $ty)*) -> io::Result<()>
        where
            W: ?Sized + Write,
        {
            self.inner.$name(writer $(, $arg)*)
        }
    };
}

impl Formatter for AsciiPretty<'_> {
    delegate!(begin_array());
    delegate!(end_array());
    delegate!(begin_array_value(first: bool));
    delegate!(end_array_value());
    delegate!(begin_object());
    delegate!(end_object());
    delegate!(begin_object_key(first: bool));
    delegate!(end_object_key());
    delegate!(begin_object_value());
    delegate!(end_object_value());

    fn write_string_fragment<W>(&mut self, writer: &mut W, fragment: &str) -> io::Result<()>
    where
        W: ?Sized + Write,
    {
        // serde_json hands us runs of characters that need no JSON escaping
        // (control chars / `"` / `\` are emitted via `write_char_escape`).
        // Escape any non-ASCII scalar here to keep the output ASCII-only.
        let mut start = 0;
        let bytes = fragment.as_bytes();
        for (idx, ch) in fragment.char_indices() {
            if ch.is_ascii() {
                continue;
            }
            if start < idx {
                writer.write_all(&bytes[start..idx])?;
            }
            let mut buf = [0u16; 2];
            for unit in ch.encode_utf16(&mut buf) {
                write!(writer, "\\u{unit:04x}")?;
            }
            start = idx + ch.len_utf8();
        }
        if start < bytes.len() {
            writer.write_all(&bytes[start..])?;
        }
        Ok(())
    }
}

/// Serialize `value` as two-space-indented, ASCII-escaped JSON
/// (no trailing newline). The verbs append the `"\n"` themselves.
#[must_use]
pub fn dumps_indent2(value: &Value) -> String {
    let mut buf = Vec::new();
    let mut ser = Serializer::with_formatter(&mut buf, AsciiPretty::new());
    value
        .serialize(&mut ser)
        .expect("serializing serde_json::Value cannot fail");
    String::from_utf8(buf).expect("serde_json emits valid UTF-8")
}
