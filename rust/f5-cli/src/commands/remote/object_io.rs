//! Single-object iControl REST round-trip helpers.
//!
//! Endpoint shape (iControl REST):
//!
//! ```text
//!   GET  /mgmt/tm/<module>/<type>/<encoded-name>   -> JSON describing the object
//!   PUT  /mgmt/tm/<module>/<type>/<encoded-name>   -> replace the object
//!   POST /mgmt/tm/<module>/<type>                  -> create a new object
//! ```
//!
//! The `<encoded-name>` for a full-path is `~Common~name` (slashes become
//! tildes).

use serde_json::Value;

use super::auth::Credentials;
use super::rest;

/// Convert `/Common/foo` -> `~Common~foo` (iControl REST format).
fn encode_path(full_path: &str) -> String {
    if full_path.starts_with('/') {
        full_path.replace('/', "~")
    } else {
        // Percent-encode everything but the RFC 3986 unreserved set.
        percent_encode(full_path)
    }
}

/// Percent-encode every byte that is not an unreserved character
/// (`A-Z a-z 0-9 _ . - ~`), encoding all others including `/`.
fn percent_encode(s: &str) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(s.len());
    for &b in s.as_bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'-' | b'~') {
            out.push(b as char);
        } else {
            let _ = write!(out, "%{b:02X}");
        }
    }
    out
}

/// Map a `kind` name to its `(module, type)` URI segments, or an error string
/// for an unknown kind.
fn kind_to_endpoint(kind: &str) -> Result<(&'static str, &'static str), String> {
    match kind {
        "virtual" => Ok(("ltm", "virtual")),
        "pool" => Ok(("ltm", "pool")),
        "node" => Ok(("ltm", "node")),
        "rule" => Ok(("ltm", "rule")),
        "monitor-http" => Ok(("ltm", "monitor/http")),
        "monitor-tcp" => Ok(("ltm", "monitor/tcp")),
        "data-group-internal" => Ok(("ltm", "data-group/internal")),
        other => Err(format!(
            "unsupported object kind '{other}' (supported: ['data-group-internal', 'monitor-http', 'monitor-tcp', 'node', 'pool', 'rule', 'virtual'])"
        )),
    }
}

/// The PUT/POST endpoint a `--dry-run` would target, plus the human verb label.
pub struct DryRunPlan {
    /// `would <verb> <kind> <target>` line printed to stderr.
    pub verb_label: &'static str,
    /// `fullPath` (preferred) or `name` (fallback) or `(unnamed)`.
    pub target: String,
}

/// Compute the `--dry-run` summary: target name (`fullPath`, else `name`, else
/// `(unnamed)`) plus the verb label (`POST (create)` / `PUT (replace)`).
#[must_use]
pub fn dry_run_plan(payload: &Value, create: bool) -> DryRunPlan {
    let target = payload
        .get("fullPath")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .or_else(|| payload.get("name").and_then(Value::as_str))
        .map_or_else(|| "(unnamed)".to_owned(), str::to_owned);
    DryRunPlan {
        verb_label: if create {
            "POST (create)"
        } else {
            "PUT (replace)"
        },
        target,
    }
}

/// GET one object via iControl REST and return its JSON value.
///
/// # Errors
/// Propagates transport / HTTP errors from [`rest`].
pub fn pull_object(
    credentials: &Credentials,
    kind: &str,
    full_path: &str,
    insecure: bool,
    timeout: f64,
) -> Result<Value, String> {
    let (module, otype) = kind_to_endpoint(kind)?;
    let encoded = encode_path(full_path);
    let path = format!("/mgmt/tm/{module}/{otype}/{encoded}");
    let resp = rest::request(credentials, "GET", &path, None, insecure, timeout)?;
    if resp.status >= 400 {
        return Err(format!(
            "GET {full_path}: HTTP {}: {}",
            resp.status,
            rest::repr_bytes(&resp.body, 200)
        ));
    }
    serde_json::from_slice(&resp.body).map_err(|e| e.to_string())
}

/// PUT (default) or POST (`create`) one object via iControl REST.
///
/// # Errors
/// Returns an error when a replace payload lacks both `fullPath` and `name`,
/// or propagates transport / HTTP errors.
pub fn push_object(
    credentials: &Credentials,
    kind: &str,
    payload: &Value,
    create: bool,
    insecure: bool,
    timeout: f64,
) -> Result<Value, String> {
    let (module, otype) = kind_to_endpoint(kind)?;
    let (method, path) = if create {
        ("POST", format!("/mgmt/tm/{module}/{otype}"))
    } else {
        let full_path = payload
            .get("fullPath")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .or_else(|| payload.get("name").and_then(Value::as_str))
            .filter(|s| !s.is_empty());
        let Some(full_path) = full_path else {
            return Err("payload must include fullPath or name".to_owned());
        };
        (
            "PUT",
            format!("/mgmt/tm/{module}/{otype}/{}", encode_path(full_path)),
        )
    };
    // Compact body, using the default `(',', ':')`-style separators (no spaces).
    let body = serde_json::to_vec(payload).map_err(|e| e.to_string())?;
    let resp = rest::request(credentials, method, &path, Some(&body), insecure, timeout)?;
    if resp.status >= 400 {
        return Err(format!(
            "{method} {path}: HTTP {}: {}",
            resp.status,
            rest::repr_bytes(&resp.body, 200)
        ));
    }
    if resp.body.is_empty() {
        Ok(Value::Object(serde_json::Map::new()))
    } else {
        serde_json::from_slice(&resp.body).map_err(|e| e.to_string())
    }
}

/// A JSON string field, treating empty strings as absent.
fn nonempty_str(v: Option<&Value>) -> Option<&str> {
    v.and_then(Value::as_str).filter(|x| !x.is_empty())
}

/// Render an iControl REST object as an SCF stanza (best-effort) for virtuals,
/// pools, nodes, and rules.
#[must_use]
pub fn object_to_scf_stanza(kind: &str, obj: &Value) -> String {
    let s = nonempty_str;
    let name = s(obj.get("fullPath"))
        .or_else(|| obj.get("name").and_then(Value::as_str))
        .unwrap_or("");
    match kind {
        "virtual" => {
            let mut lines = vec![format!("ltm virtual {name} {{")];
            if let Some(d) = s(obj.get("destination")) {
                lines.push(format!("    destination {d}"));
            }
            if let Some(p) = s(obj.get("pool")) {
                lines.push(format!("    pool {p}"));
            }
            if let Some(rules) = obj.get("rules").and_then(Value::as_array)
                && !rules.is_empty()
            {
                lines.push("    rules {".to_owned());
                for r in rules {
                    if let Some(r) = r.as_str() {
                        lines.push(format!("        {r}"));
                    }
                }
                lines.push("    }".to_owned());
            }
            lines.push("}".to_owned());
            lines.join("\n") + "\n"
        }
        "pool" => {
            let mut lines = vec![format!("ltm pool {name} {{")];
            let members = obj
                .get("membersReference")
                .and_then(|m| m.get("items"))
                .and_then(Value::as_array)
                .or_else(|| obj.get("members").and_then(Value::as_array));
            if let Some(members) = members
                && !members.is_empty()
            {
                lines.push("    members {".to_owned());
                for m in members {
                    let mname = s(m.get("fullPath"))
                        .or_else(|| m.get("name").and_then(Value::as_str))
                        .unwrap_or("");
                    lines.push(format!("        {mname} {{"));
                    if let Some(a) = s(m.get("address")) {
                        lines.push(format!("            address {a}"));
                    }
                    lines.push("        }".to_owned());
                }
                lines.push("    }".to_owned());
            }
            if let Some(m) = s(obj.get("monitor")) {
                lines.push(format!("    monitor {m}"));
            }
            if let Some(lb) = s(obj.get("loadBalancingMode")) {
                lines.push(format!("    load-balancing-mode {lb}"));
            }
            lines.push("}".to_owned());
            lines.join("\n") + "\n"
        }
        "node" => {
            let mut lines = vec![format!("ltm node {name} {{")];
            if let Some(a) = s(obj.get("address")) {
                lines.push(format!("    address {a}"));
            }
            lines.push("}".to_owned());
            lines.join("\n") + "\n"
        }
        "rule" => {
            let body = obj
                .get("apiAnonymous")
                .and_then(Value::as_str)
                .unwrap_or("");
            format!("ltm rule {name} {{\n{body}\n}}\n")
        }
        _ => {
            let json = serde_json::to_string(obj).unwrap_or_default();
            format!("# {kind} {name}\n# {json}\n")
        }
    }
}

/// Parse a JSON payload, reproducing the
/// `Expecting value: line L column C (char N)` decoder message for the common
/// no-value / leading-garbage cases. The position is recomputed from the first
/// non-whitespace character (serde reports a different offset), per the parity
/// contract.
///
/// # Errors
/// Returns `invalid JSON payload: <message>` on any parse failure.
pub fn parse_payload(raw: &str) -> Result<Value, String> {
    if let Ok(v) = serde_json::from_str::<Value>(raw) {
        return Ok(v);
    }
    let (line, col, ch) = expecting_value_position(raw);
    Err(format!(
        "invalid JSON payload: Expecting value: line {line} column {col} (char {ch})"
    ))
}

/// Compute the `(line, column, char)` for an `Expecting value` error: the
/// offset of the first non-whitespace byte (or the end of input when the string
/// is empty / all whitespace), with 1-based line / column.
fn expecting_value_position(raw: &str) -> (usize, usize, usize) {
    // Skip the ASCII whitespace set " \t\n\r"; the error points at the first
    // non-whitespace char, or at end-of-string when none remains.
    let bytes = raw.as_bytes();
    let mut idx = 0;
    while idx < bytes.len() && matches!(bytes[idx], b' ' | b'\t' | b'\n' | b'\r') {
        idx += 1;
    }
    char_position(raw, idx)
}

/// Translate a byte offset into a 1-based `(line, column, char)` triple.
/// Inputs here are ASCII-dominated config payloads; the char index counts
/// Unicode scalar values to match the decoder's string indexing.
fn char_position(raw: &str, byte_idx: usize) -> (usize, usize, usize) {
    let mut line = 1usize;
    let mut col = 1usize;
    let mut char_idx = 0usize;
    for (b, c) in raw.char_indices() {
        if b >= byte_idx {
            break;
        }
        if c == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
        char_idx += 1;
    }
    (line, col, char_idx)
}
