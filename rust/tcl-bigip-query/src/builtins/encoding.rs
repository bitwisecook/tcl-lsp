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

//! Encoding-category builtins: `base64` / `base64d` / `html` / `uri` /
//! `tojson` / `fromjson` / `sh`.
//!
//! Behaviour notes:
//! - `base64` / `base64d` use the standard alphabet with padding —
//!   standard base64 encode / strict decode. The
//!   `base64d` failure text is custom (the
//!   Rust `base64` crate has its own error strings), a documented
//!   divergence kept out of the golden fixture.
//! - `uri` percent-encodes every byte outside the unreserved set
//!   `A-Z a-z 0-9 - _ . ~` over the UTF-8 bytes.
//! - `html` HTML-escapes `&`→`&amp;`,
//!   `<`→`&lt;`, `>`→`&gt;`, `"`→`&quot;`, `'`→`&#x27;`.
//! - `tojson` is compact JSON with sorted keys, routed through
//!   `jsonfmt::to_compact_sorted`.
//! - `fromjson` parses JSON into the value model (objects preserve key
//!   order, integers stay `Int`). The JSON parse-error wording is custom
//!   (divergent), a documented divergence kept out of the fixture.
//! - `sh` is jq's `@sh` force-single-quote (not POSIX shell quoting): every value
//!   is wrapped in `'…'` with embedded `'` emitted as `'\''`; a list /
//!   stream becomes space-separated quoted fields.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;

use crate::builtins::{BuiltinSpec, as_str, plain, to_jsonable, type_name};
use crate::errors::QueryError;
use crate::jsonfmt;
use crate::value::{MAX_VALUE_WALK_DEPTH, Value};

pub(super) fn registrations() -> Vec<(&'static str, BuiltinSpec)> {
    vec![
        plain("base64", "string", 1, Some(1), false, bi_base64),
        plain("base64d", "string", 1, Some(1), false, bi_base64d),
        plain("html", "string", 1, Some(1), false, bi_html),
        plain("uri", "string", 1, Some(1), false, bi_uri),
        plain("tojson", "string", 1, Some(1), false, bi_tojson),
        plain("fromjson", "string", 1, Some(1), false, bi_fromjson),
        plain("sh", "string", 1, Some(1), true, bi_sh),
    ]
}

fn bi_base64(args: &[Value]) -> Result<Value, QueryError> {
    let s = as_str(&args[0], "base64", 1)?;
    Ok(Value::Str(STANDARD.encode(s.as_bytes())))
}

fn bi_base64d(args: &[Value]) -> Result<Value, QueryError> {
    let s = as_str(&args[0], "base64d", 1)?;
    // Strict decoding rejects any non-alphabet byte and demands
    // correct padding — `STANDARD` (with no-trailing-bits checks) mirrors it.
    let bytes = STANDARD
        .decode(s.as_bytes())
        .map_err(|e| QueryError::builtin(format!("base64d: invalid Base64 input: {e}")))?;
    let text = String::from_utf8(bytes)
        .map_err(|e| QueryError::builtin(format!("base64d: invalid Base64 input: {e}")))?;
    Ok(Value::Str(text))
}

fn bi_html(args: &[Value]) -> Result<Value, QueryError> {
    let s = as_str(&args[0], "html", 1)?;
    // HTML-escape — `&` first, then the rest.
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#x27;"),
            other => out.push(other),
        }
    }
    Ok(Value::Str(out))
}

fn bi_uri(args: &[Value]) -> Result<Value, QueryError> {
    let s = as_str(&args[0], "uri", 1)?;
    // Percent-encode every byte outside the unreserved set: keep the unreserved
    // set, percent-encode every other byte (upper-case hex) over the UTF-8 encoding.
    let mut out = String::with_capacity(s.len());
    for &byte in s.as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(byte as char);
        } else {
            out.push('%');
            out.push(hex_upper(byte >> 4));
            out.push(hex_upper(byte & 0x0f));
        }
    }
    Ok(Value::Str(out))
}

fn hex_upper(nibble: u8) -> char {
    match nibble {
        0..=9 => (b'0' + nibble) as char,
        _ => (b'A' + (nibble - 10)) as char,
    }
}

fn bi_tojson(args: &[Value]) -> Result<Value, QueryError> {
    Ok(Value::Str(jsonfmt::to_compact_sorted(&to_jsonable(
        &args[0], 0,
    ))))
}

fn bi_fromjson(args: &[Value]) -> Result<Value, QueryError> {
    let s = as_str(&args[0], "fromjson", 1)?;
    let parsed: serde_json::Value = serde_json::from_str(&s)
        .map_err(|e| QueryError::builtin(format!("fromjson: invalid JSON: {e}")))?;
    Ok(json_to_value(&parsed, 0))
}

/// Convert a parsed `serde_json::Value` into the query value model:
/// integers stay `Int`, anything else `Float`, objects preserve
/// key insertion order (the `preserve_order` feature backs this).
///
/// `depth` is the nesting level of this call (0 at the top); past
/// [`MAX_VALUE_WALK_DEPTH`] this returns `Value::Null` in place of the
/// over-deep subtree instead of recursing further (issue #996). Every
/// current caller (`fromjson`, `http_body_json`, `json_load`, `json_parse`,
/// …) builds `j` via `serde_json::from_str`, which already enforces its own
/// ~128-level default recursion limit before `json_to_value` ever sees the
/// result — but that limit lives in `serde_json`, not here, so this guard is
/// what actually protects `json_to_value` itself for defence-in-depth and
/// consistency with every other `Value`-walker in this crate, independent
/// of how any future caller happens to construct its `serde_json::Value`.
pub(crate) fn json_to_value(j: &serde_json::Value, depth: u32) -> Value {
    if MAX_VALUE_WALK_DEPTH.exceeded(depth) {
        return Value::Null;
    }
    match j {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Bool(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Int(i)
            } else if let Some(u) = n.as_u64() {
                Value::Int(i64::try_from(u).unwrap_or(i64::MAX))
            } else {
                Value::Float(n.as_f64().unwrap_or(f64::NAN))
            }
        }
        serde_json::Value::String(s) => Value::Str(s.clone()),
        serde_json::Value::Array(items) => Value::List(
            items
                .iter()
                .map(|item| json_to_value(item, depth + 1))
                .collect(),
        ),
        serde_json::Value::Object(map) => {
            let mut m = indexmap::IndexMap::new();
            for (k, v) in map {
                m.insert(k.clone(), json_to_value(v, depth + 1));
            }
            Value::Object(m)
        }
    }
}

fn bi_sh(args: &[Value]) -> Result<Value, QueryError> {
    match &args[0] {
        Value::List(items) | Value::Stream(items) => {
            let parts: Result<Vec<String>, QueryError> = items
                .iter()
                .map(|item| Ok(sh_quote(&as_str(item, "sh", 1)?)))
                .collect();
            Ok(Value::Str(parts?.join(" ")))
        }
        other => {
            let s = as_str(other, "sh", 1).map_err(|_| {
                QueryError::builtin(format!(
                    "sh: argument 1 must be a string, got {}",
                    type_name(other)
                ))
            })?;
            Ok(Value::Str(sh_quote(&s)))
        }
    }
}

/// jq `@sh` quoting — force single quotes, embedded `'` → `'\''`.
fn sh_quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Call a builtin and unwrap its `Str` result.
    fn call(f: fn(&[Value]) -> Result<Value, QueryError>, args: &[Value]) -> String {
        match f(args) {
            Ok(Value::Str(text)) => text,
            other => panic!("expected Str, got {other:?}"),
        }
    }

    fn s(text: &str) -> Value {
        Value::Str(text.to_owned())
    }

    #[test]
    fn base64_round_trip() {
        assert_eq!(call(bi_base64, &[s("hello")]), "aGVsbG8=");
        assert_eq!(call(bi_base64d, &[s("aGVsbG8=")]), "hello");
    }

    #[test]
    fn uri_percent_encodes_unsafe_characters() {
        // ::test_uri_encodes_url_unsafe_characters
        assert_eq!(call(bi_uri, &[s("hello world")]), "hello%20world");
    }

    #[test]
    fn html_escapes_markup_and_quotes() {
        // ::test_html_and_sh_quote (html half)
        assert_eq!(
            call(bi_html, &[s("<a>&b</a>")]),
            "&lt;a&gt;&amp;b&lt;/a&gt;"
        );
    }

    #[test]
    fn sh_quotes_strings_lists_and_embedded_quotes() {
        // jq `@sh` behaviour: every list element is single-quoted, embedded
        // `'` → the `'\''` dance.
        assert_eq!(call(bi_sh, &[s("hello world")]), "'hello world'");
        assert_eq!(
            call(bi_sh, &[Value::List(vec![s("a"), s("b c")])]),
            "'a' 'b c'"
        );
        assert_eq!(call(bi_sh, &[s("a'b")]), "'a'\\''b'");
    }

    /// A `serde_json::Value` nested `depth` levels deep, wrapping a single
    /// `Number(0)` leaf. Built with a plain loop (no recursion), so
    /// constructing the fixture itself cannot trip `serde_json::from_str`'s
    /// own ~128-level default recursion limit the way parsing equivalent
    /// JSON *text* would — this is exactly the shape that limit does not
    /// protect `json_to_value` against (issue #996).
    fn deep_json_array(depth: usize) -> serde_json::Value {
        let mut v = serde_json::Value::Number(0.into());
        for _ in 0..depth {
            v = serde_json::Value::Array(vec![v]);
        }
        v
    }

    /// Regression coverage for issue #996: `json_to_value` recurses once
    /// per nested JSON array/object level, with no depth cap before this
    /// fix. 5000 is comfortably past `MAX_VALUE_WALK_DEPTH` (64); the
    /// assertion is that it returns at all, not what it returns.
    #[test]
    fn deeply_nested_json_to_value_does_not_crash() {
        let j = deep_json_array(5000);
        let _ = json_to_value(&j, 0);
    }

    /// JSON nested well under `MAX_VALUE_WALK_DEPTH` still converts exactly
    /// as before this fix.
    #[test]
    fn moderately_nested_json_to_value_is_unchanged() {
        let j = deep_json_array(3);
        let v = json_to_value(&j, 0);
        assert_eq!(jsonfmt::to_compact(&v), "[[[0]]]");
    }
}
