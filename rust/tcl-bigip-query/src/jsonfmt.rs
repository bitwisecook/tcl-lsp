//! Python-`json.dumps`-faithful serialisation of [`Value`]s.
//!
//! The query DSL emits JSON in two spellings:
//!
//! - **pretty** — `json.dumps(obj, indent=2)` (the `--json` output mode);
//! - **compact** — `json.dumps(obj, separators=(",", ":"))` (`tojson`,
//!   `debug`, table cells).
//!
//! Both default to `ensure_ascii=True`, so non-ASCII code points are escaped
//! as `\uXXXX` (with surrogate pairs above the BMP). The transformation from
//! [`Value`] to JSON mirrors `output._to_json`: an `ObjectRef` becomes
//! `{"kind", "full-path", "fields"}`, a `PathRef` its full-path string, a
//! `Stream` an array, and so on.

use std::fmt::Write as _;

use crate::value::Value;

/// Serialise *value* as `json.dumps(_to_json(value), indent=2)` would.
#[must_use]
pub fn to_pretty(value: &Value) -> String {
    let mut out = String::new();
    write_value(&mut out, value, Some(0), false);
    out
}

/// Serialise *value* as `json.dumps(_to_json(value), separators=(",", ":"))`.
#[must_use]
pub fn to_compact(value: &Value) -> String {
    let mut out = String::new();
    write_value(&mut out, value, None, false);
    out
}

/// Compact serialisation with `sort_keys=True` — the spelling `tostring`
/// uses for composite values.
#[must_use]
pub fn to_compact_sorted(value: &Value) -> String {
    let mut out = String::new();
    write_value(&mut out, value, None, true);
    out
}

fn indent(out: &mut String, level: usize) {
    out.push('\n');
    for _ in 0..level * 2 {
        out.push(' ');
    }
}

fn write_value(out: &mut String, value: &Value, level: Option<usize>, sort_keys: bool) {
    match value {
        Value::Null | Value::Drop => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Int(i) => out.push_str(&i.to_string()),
        Value::Float(f) => out.push_str(&py_float_repr(*f)),
        Value::Str(s) => write_string(out, s),
        Value::PathRef(p) => write_string(out, &p.full_path),
        Value::List(items) | Value::Stream(items) => write_array(out, items, level, sort_keys),
        Value::Object(map) => {
            write_object(
                out,
                map.iter().map(|(k, v)| (k.as_str(), v)),
                level,
                sort_keys,
            );
        }
        Value::Container(c) => {
            // Python `_to_json` falls back to `str(value)` for a Container.
            write_string(out, &format!("container({})", c.kind));
        }
        Value::ObjectRef(o) => {
            // `_to_json(ObjectRef)` → {"kind", "full-path", "fields": {...}}.
            let fields = Value::Object(o.fields.clone());
            let entries: Vec<(&str, Value)> = vec![
                ("kind", Value::Str(o.kind.clone())),
                ("full-path", Value::Str(o.full_path.clone())),
                ("fields", fields),
            ];
            write_object(out, entries.iter().map(|(k, v)| (*k, v)), level, sort_keys);
        }
    }
}

fn write_array(out: &mut String, items: &[Value], level: Option<usize>, sort_keys: bool) {
    if items.is_empty() {
        out.push_str("[]");
        return;
    }
    out.push('[');
    let inner = level.map(|l| l + 1);
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        if let Some(l) = inner {
            indent(out, l);
        }
        write_value(out, item, inner, sort_keys);
    }
    if let Some(l) = level {
        indent(out, l);
    }
    out.push(']');
}

fn write_object<'a>(
    out: &mut String,
    entries: impl Iterator<Item = (&'a str, &'a Value)>,
    level: Option<usize>,
    sort_keys: bool,
) {
    let mut entries: Vec<(&str, &Value)> = entries.collect();
    if sort_keys {
        entries.sort_by(|a, b| a.0.cmp(b.0));
    }
    if entries.is_empty() {
        out.push_str("{}");
        return;
    }
    out.push('{');
    let inner = level.map(|l| l + 1);
    for (i, (key, val)) in entries.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        if let Some(l) = inner {
            indent(out, l);
        }
        write_string(out, key);
        // With `indent` set, Python's key separator is `": "`; compact mode
        // uses `":"`.
        out.push_str(if level.is_some() { ": " } else { ":" });
        write_value(out, val, inner, sort_keys);
    }
    if let Some(l) = level {
        indent(out, l);
    }
    out.push('}');
}

/// Escape a string the way Python's `json.dumps(..., ensure_ascii=True)`
/// does: the JSON short escapes plus `\uXXXX` for every control character
/// and every non-ASCII code point (surrogate pairs above the BMP).
fn write_string(out: &mut String, s: &str) {
    out.push('"');
    for ch in s.chars() {
        match ch {
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
                let cp = c as u32;
                if cp > 0xFFFF {
                    // Encode as a UTF-16 surrogate pair, matching CPython.
                    let v = cp - 0x1_0000;
                    let hi = 0xD800 + (v >> 10);
                    let lo = 0xDC00 + (v & 0x3FF);
                    let _ = write!(out, "\\u{hi:04x}\\u{lo:04x}");
                } else {
                    let _ = write!(out, "\\u{cp:04x}");
                }
            }
        }
    }
    out.push('"');
}

/// Render a float the way Python's `repr()` / `json.dumps` would: a
/// shortest round-tripping decimal, always carrying a `.0` for integral
/// values, with `NaN` / `Infinity` / `-Infinity` for the non-finite cases
/// (the non-standard spelling `CPython`'s `json` emits).
#[must_use]
pub fn py_float_repr(f: f64) -> String {
    if f.is_nan() {
        return "NaN".to_string();
    }
    if f.is_infinite() {
        return if f < 0.0 { "-Infinity" } else { "Infinity" }.to_string();
    }
    let mut s = format!("{f}");
    if !s.contains('.') && !s.contains('e') && !s.contains('E') {
        s.push_str(".0");
    }
    s
}
