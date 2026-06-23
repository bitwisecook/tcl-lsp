//! Encoding-category builtins: `base64` / `base64d` / `html` / `uri` /
//! `tojson` / `fromjson` / `sh`.
//!
//! Parity notes:
//! - `base64` / `base64d` use the standard alphabet with padding, matching
//!   `base64.b64encode` / `b64decode(validate=True)`. The
//!   `base64d` failure text differs from `CPython`'s `binascii` wording (the
//!   Rust `base64` crate has its own error strings), a documented
//!   divergence kept out of the golden fixture.
//! - `uri` percent-encodes everything outside the unreserved set
//!   `A-Z a-z 0-9 - _ . ~` over the UTF-8 bytes — `urllib.parse.quote(s,
//!   safe="")`.
//! - `html` reproduces `html.escape(s, quote=True)`: `&`→`&amp;`,
//!   `<`→`&lt;`, `>`→`&gt;`, `"`→`&quot;`, `'`→`&#x27;`.
//! - `tojson` is `json.dumps(_to_jsonable(value), separators=(",", ":"),
//!   sort_keys=True)` — compact + sorted keys, routed through
//!   `jsonfmt::to_compact_sorted`.
//! - `fromjson` parses JSON into the value model (objects preserve key
//!   order, integers stay `Int`). Its `JSONDecodeError` wording differs
//!   from `CPython`'s, a documented divergence kept out of the fixture.
//! - `sh` is jq's `@sh` force-single-quote (NOT `shlex.quote`): every value
//!   is wrapped in `'…'` with embedded `'` emitted as `'\''`; a list /
//!   stream becomes space-separated quoted fields.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;

use crate::builtins::{BuiltinSpec, as_str, plain, to_jsonable, type_name};
use crate::errors::QueryError;
use crate::jsonfmt;
use crate::value::Value;

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
    // `validate=True` rejects any non-alphabet byte and demands
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
    // `html.escape(s, quote=True)` — `&` first, then the rest.
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
    // `urllib.parse.quote(s, safe="")`: keep the unreserved set, percent-
    // encode every other byte (upper-case hex) over the UTF-8 encoding.
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
        &args[0],
    ))))
}

fn bi_fromjson(args: &[Value]) -> Result<Value, QueryError> {
    let s = as_str(&args[0], "fromjson", 1)?;
    let parsed: serde_json::Value = serde_json::from_str(&s)
        .map_err(|e| QueryError::builtin(format!("fromjson: invalid JSON: {e}")))?;
    Ok(json_to_value(&parsed))
}

/// Convert a parsed `serde_json::Value` into the query value model, matching
/// `json.loads`: integers stay `Int`, anything else `Float`, objects preserve
/// key insertion order (the `preserve_order` feature backs this).
pub(crate) fn json_to_value(j: &serde_json::Value) -> Value {
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
        serde_json::Value::Array(items) => Value::List(items.iter().map(json_to_value).collect()),
        serde_json::Value::Object(map) => {
            let mut m = indexmap::IndexMap::new();
            for (k, v) in map {
                m.insert(k.clone(), json_to_value(v));
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
        // ::test_html_and_sh_quote (sh half) — jq `@sh` parity: every list
        // element is single-quoted, embedded `'` → the `'\''` dance.
        assert_eq!(call(bi_sh, &[s("hello world")]), "'hello world'");
        assert_eq!(
            call(bi_sh, &[Value::List(vec![s("a"), s("b c")])]),
            "'a' 'b c'"
        );
        assert_eq!(call(bi_sh, &[s("a'b")]), "'a'\\''b'");
    }
}
