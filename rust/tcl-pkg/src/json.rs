//! Canonical JSON emitter.
//!
//! Reproduces Python `json.dumps(..., indent=2, sort_keys=True,
//! ensure_ascii=False)`: object keys sorted alphabetically (recursively),
//! 2-space indent, `": "` key separator, `,` item separator, no trailing
//! whitespace. Callers append the final newline. This keeps the lockfile and
//! `--json` output byte-identical with the Python implementation regardless of
//! whether `serde_json`'s `preserve_order` feature is active workspace-wide.

use serde_json::Value;

/// Serialise `value` as canonical JSON (no trailing newline).
#[must_use]
pub fn canonical_json(value: &Value) -> String {
    let mut out = String::new();
    write_value(value, 0, &mut out);
    out
}

fn push_indent(level: usize, out: &mut String) {
    for _ in 0..level {
        out.push_str("  ");
    }
}

fn write_value(value: &Value, indent: usize, out: &mut String) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => out.push_str(&n.to_string()),
        Value::String(s) => out.push_str(&escape_string(s)),
        Value::Array(items) => {
            if items.is_empty() {
                out.push_str("[]");
                return;
            }
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push('\n');
                push_indent(indent + 1, out);
                write_value(item, indent + 1, out);
            }
            out.push('\n');
            push_indent(indent, out);
            out.push(']');
        }
        Value::Object(map) => {
            if map.is_empty() {
                out.push_str("{}");
                return;
            }
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            out.push('{');
            for (i, key) in keys.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push('\n');
                push_indent(indent + 1, out);
                out.push_str(&escape_string(key));
                out.push_str(": ");
                write_value(&map[key.as_str()], indent + 1, out);
            }
            out.push('\n');
            push_indent(indent, out);
            out.push('}');
        }
    }
}

/// Escape a string the way `serde_json` does, which matches Python's
/// `ensure_ascii=False` for the ASCII identifiers/URLs in our data: `"` and `\`
/// escaped, control characters as short forms or `\u00XX`, non-ASCII passed
/// through, forward slash left unescaped.
fn escape_string(s: &str) -> String {
    serde_json::to_string(s).expect("string serialises")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn sorts_keys_recursively() {
        let v = json!({"b": 1, "a": {"d": 2, "c": 3}});
        assert_eq!(
            canonical_json(&v),
            "{\n  \"a\": {\n    \"c\": 3,\n    \"d\": 2\n  },\n  \"b\": 1\n}"
        );
    }

    #[test]
    fn empty_containers() {
        assert_eq!(canonical_json(&json!({})), "{}");
        assert_eq!(canonical_json(&json!([])), "[]");
        assert_eq!(canonical_json(&json!({"a": []})), "{\n  \"a\": []\n}");
    }

    #[test]
    fn null_and_bool() {
        let v = json!({"x": null, "y": true, "z": false});
        assert_eq!(
            canonical_json(&v),
            "{\n  \"x\": null,\n  \"y\": true,\n  \"z\": false\n}"
        );
    }
}
