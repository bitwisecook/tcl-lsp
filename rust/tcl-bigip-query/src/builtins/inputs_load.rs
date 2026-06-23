//! File-loading builtins: `json_load` / `jsonl_load` / `csv_load` /
//! `f5log_load` / `cert_load`.
//!
//! Each reads a file path argument and returns the parsed value, reusing
//! the parsers in [`crate::inputs`] so a query can mix CLI-loaded
//! side-inputs and in-query loads from the same code path.
//!
//! Notes:
//! - All five honour `~` tilde expansion and report `file not found:` /
//!   `cannot read` / format-specific parse errors with the exact Python
//!   wording.
//! - `cert_load` is **unsupported**: it reads the file and validates its
//!   shape, then raises a clear error for the X.509 parse rather than
//!   returning a half-shaped dict.

use crate::builtins::{BuiltinSpec, as_str, plain, type_name};
use crate::errors::QueryError;
use crate::inputs::{parse_csv, parse_f5log, parse_jsonl};
use crate::value::Value;

pub(super) fn registrations() -> Vec<(&'static str, BuiltinSpec)> {
    vec![
        plain("json_load", "value", 1, Some(1), false, bi_json_load),
        plain("jsonl_load", "value", 1, Some(1), false, bi_jsonl_load),
        plain("csv_load", "value", 1, Some(2), false, bi_csv_load),
        plain("f5log_load", "value", 1, Some(1), false, bi_f5log_load),
        plain("cert_load", "value", 1, Some(2), false, bi_cert_load),
    ]
}

/// Port of `os.path.expanduser` for the `~` / `~/...` head only (the form
/// the builtins document). `~user` expansion is not reproduced — Python's
/// `expanduser` leaves an unknown `~user` untouched too when the home
/// can't be resolved, so the common `~` / `~/` case is the load-bearing
/// one.
fn expanduser(p: &str) -> String {
    if p == "~" {
        if let Some(home) = home_dir() {
            return home;
        }
        return p.to_string();
    }
    if let Some(rest) = p.strip_prefix("~/")
        && let Some(home) = home_dir()
    {
        return format!("{home}/{rest}");
    }
    p.to_string()
}

fn home_dir() -> Option<String> {
    std::env::var("HOME").ok().filter(|h| !h.is_empty())
}

/// Read a UTF-8 text file, mapping IO errors to the per-builtin Python
/// wording (`{name}: file not found: {path}` / `{name}: cannot read ...`).
fn read_text(name: &str, expanded: &str) -> Result<String, QueryError> {
    match std::fs::read(expanded) {
        Ok(bytes) => match String::from_utf8(bytes) {
            Ok(s) => Ok(s),
            // Python opens with `encoding="utf-8"`; a decode error surfaces
            // as the OSError-shaped `cannot read` path is not hit — Python
            // raises a UnicodeDecodeError, which is not one of the caught
            // exceptions, so it propagates. We surface a clear builtin error.
            Err(e) => Err(QueryError::builtin(format!(
                "{name}: cannot read {expanded}: {e}"
            ))),
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(QueryError::builtin(format!(
            "{name}: file not found: {expanded}"
        ))),
        Err(e) => Err(QueryError::builtin(format!(
            "{name}: cannot read {expanded}: {e}"
        ))),
    }
}

fn bi_json_load(args: &[Value]) -> Result<Value, QueryError> {
    let p = as_str(&args[0], "json_load", 1)?;
    let expanded = expanduser(&p);
    let text = read_text("json_load", &expanded)?;
    let parsed: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
        QueryError::builtin(format!(
            "json_load: {expanded}: invalid JSON ({} at line {} col {})",
            e,
            e.line(),
            e.column()
        ))
    })?;
    Ok(crate::builtins::json_to_value(&parsed))
}

fn bi_jsonl_load(args: &[Value]) -> Result<Value, QueryError> {
    let p = as_str(&args[0], "jsonl_load", 1)?;
    let expanded = expanduser(&p);
    let text = read_text("jsonl_load", &expanded)?;
    // `parse_jsonl` raises with the `{source}: line N: ...` shape; the
    // Python builtin re-wraps as `jsonl_load: {exc}`.
    parse_jsonl(&text, &expanded).map_err(|e| QueryError::builtin(format!("jsonl_load: {e}")))
}

fn bi_csv_load(args: &[Value]) -> Result<Value, QueryError> {
    let p = as_str(&args[0], "csv_load", 1)?;
    let expanded = expanduser(&p);
    let headers: Option<Vec<String>> = if args.len() > 1 {
        let Value::List(items) = &args[1] else {
            return Err(QueryError::builtin(format!(
                "csv_load: argument 2 must be a list of strings, got {}",
                type_name(&args[1])
            )));
        };
        let mut hdrs = Vec::with_capacity(items.len());
        for (i, h) in items.iter().enumerate() {
            let Value::Str(s) = h else {
                return Err(QueryError::builtin(format!(
                    "csv_load: argument 2: header {i} must be a string, got {}",
                    type_name(h)
                )));
            };
            hdrs.push(s.clone());
        }
        Some(hdrs)
    } else {
        None
    };
    let text = read_text("csv_load", &expanded)?;
    parse_csv(&text, headers.as_deref(), &expanded)
        .map_err(|e| QueryError::builtin(format!("csv_load: {e}")))
}

fn bi_f5log_load(args: &[Value]) -> Result<Value, QueryError> {
    let p = as_str(&args[0], "f5log_load", 1)?;
    let expanded = expanduser(&p);
    let text = read_text("f5log_load", &expanded)?;
    Ok(parse_f5log(&text))
}

/// `cert_load` — unsupported. Reads the file and validates the argument
/// shapes, then raises a clear error for the X.509 parse.
fn bi_cert_load(args: &[Value]) -> Result<Value, QueryError> {
    let p = as_str(&args[0], "cert_load", 1)?;
    let expanded = expanduser(&p);
    if args.len() > 1 {
        // Validate the password arg shape so a type error matches Python's
        // `_as_str` before we hit the unsupported-parse error.
        let _ = as_str(&args[1], "cert_load", 2)?;
    }
    // Read order matches Python: missing / unreadable file errors first.
    match std::fs::read(&expanded) {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(QueryError::builtin(format!(
                "cert_load: file not found: {expanded}"
            )));
        }
        Err(e) => {
            return Err(QueryError::builtin(format!(
                "cert_load: cannot read {expanded}: {e}"
            )));
        }
    }
    Err(QueryError::builtin(format!(
        "cert_load: {expanded}: X.509 parsing is not yet supported in the \
         Rust port (deferred to the probes increment)"
    )))
}
