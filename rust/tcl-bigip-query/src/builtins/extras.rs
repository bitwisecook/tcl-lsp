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

//! Misc builtins outside the other categories:
//! `env`, `source_file`, and the `http_*` HTTP-response accessors.
//!
//! The `http_*` family operates on an HTTP-response **dict** (the shape
//! `url_get` / `url_post` return) — they are pure extractors, NOT live
//! network calls, so they are fully deterministic. `env` reads the process
//! environment; `source_file` reports the source URI an object came from.

use indexmap::IndexMap;

use crate::builtins::{BuiltinSpec, as_str, ctx, plain, type_name};
use crate::errors::QueryError;
use crate::eval::EvalContext;
use crate::value::Value;

pub(super) fn registrations() -> Vec<(&'static str, BuiltinSpec)> {
    vec![
        plain("env", "value", 0, Some(0), false, bi_env),
        ctx("source_file", "value", 1, Some(1), bi_source_file),
        plain("http_status", "net", 1, Some(1), false, bi_http_status),
        plain("http_body", "net", 1, Some(1), false, bi_http_body),
        plain(
            "http_body_json",
            "net",
            1,
            Some(1),
            false,
            bi_http_body_json,
        ),
        plain("http_headers", "net", 1, Some(1), false, bi_http_headers),
        plain("http_header", "net", 2, Some(2), false, bi_http_header),
        plain("http_ok", "net", 1, Some(1), false, bi_http_ok),
        plain("http_redirect", "net", 1, Some(1), false, bi_http_redirect),
        plain(
            "http_client_error",
            "net",
            1,
            Some(1),
            false,
            bi_http_client_error,
        ),
        plain(
            "http_server_error",
            "net",
            1,
            Some(1),
            false,
            bi_http_server_error,
        ),
    ]
}

/// `env()` → an object of the process environment. Values are strings;
/// iteration order is unspecified.
fn bi_env(_args: &[Value]) -> Result<Value, QueryError> {
    let mut m: IndexMap<String, Value> = IndexMap::new();
    for (k, v) in std::env::vars() {
        m.insert(k, Value::Str(v));
    }
    Ok(Value::Object(m))
}

/// `source_file(obj)` → the URI of the source that defined *obj*, or `null`.
/// `ObjectRef` → its `config_uri`; `PathRef`
/// → resolved through the projection to its backing object's `config_uri`.
fn bi_source_file(args: &[Value], ctx: &mut EvalContext) -> Result<Value, QueryError> {
    match &args[0] {
        Value::ObjectRef(o) => Ok(uri_or_null(&o.config_uri)),
        Value::PathRef(p) => match crate::projection::resolve_pathref(p, &ctx.root) {
            Some(target) => Ok(uri_or_null(&target.config_uri)),
            None => Ok(Value::Null),
        },
        _ => Ok(Value::Null),
    }
}

fn uri_or_null(uri: &str) -> Value {
    if uri.is_empty() {
        Value::Null
    } else {
        Value::Str(uri.to_owned())
    }
}

/// Require an HTTP-response dict.
fn http_response<'a>(
    value: &'a Value,
    name: &str,
) -> Result<&'a IndexMap<String, Value>, QueryError> {
    match value {
        Value::Object(map) => Ok(map),
        other => Err(QueryError::builtin(format!(
            "{name}: argument must be an HTTP response dict (got {})",
            type_name(other)
        ))),
    }
}

/// Whether the response status falls within `[low, high]`.
fn status_in_range(value: &Value, low: i64, high: i64, name: &str) -> Result<Value, QueryError> {
    let resp = http_response(value, name)?;
    let ok = match resp.get("status") {
        Some(Value::Int(s)) => low <= *s && *s <= high,
        _ => false,
    };
    Ok(Value::Bool(ok))
}

fn bi_http_status(args: &[Value]) -> Result<Value, QueryError> {
    let resp = http_response(&args[0], "http_status")?;
    Ok(resp.get("status").cloned().unwrap_or(Value::Null))
}

fn bi_http_body(args: &[Value]) -> Result<Value, QueryError> {
    let resp = http_response(&args[0], "http_body")?;
    let body = match resp.get("body") {
        Some(Value::Str(s)) => s.clone(),
        Some(other) => py_str(other),
        None => String::new(),
    };
    Ok(Value::Str(body))
}

fn bi_http_body_json(args: &[Value]) -> Result<Value, QueryError> {
    let resp = http_response(&args[0], "http_body_json")?;
    if let Some(bj) = resp.get("body_json")
        && !matches!(bj, Value::Null)
    {
        return Ok(bj.clone());
    }
    let body = match resp.get("body") {
        Some(Value::Str(s)) => s.clone(),
        Some(other) => py_str(other),
        None => String::new(),
    };
    if body.is_empty() {
        return Ok(Value::Null);
    }
    match serde_json::from_str::<serde_json::Value>(&body) {
        Ok(j) => Ok(crate::builtins::json_to_value(&j, 0)),
        Err(e) => Err(QueryError::builtin(format!(
            "http_body_json: invalid JSON ({e})"
        ))),
    }
}

fn bi_http_headers(args: &[Value]) -> Result<Value, QueryError> {
    let resp = http_response(&args[0], "http_headers")?;
    match resp.get("headers") {
        Some(Value::Object(h)) => Ok(Value::Object(h.clone())),
        _ => Ok(Value::Object(IndexMap::new())),
    }
}

fn bi_http_header(args: &[Value]) -> Result<Value, QueryError> {
    let resp = http_response(&args[0], "http_header")?;
    let key = as_str(&args[1], "http_header", 2)?.to_lowercase();
    match resp.get("headers") {
        Some(Value::Object(h)) => Ok(h.get(&key).cloned().unwrap_or(Value::Null)),
        _ => Ok(Value::Null),
    }
}

fn bi_http_ok(args: &[Value]) -> Result<Value, QueryError> {
    status_in_range(&args[0], 200, 299, "http_ok")
}

fn bi_http_redirect(args: &[Value]) -> Result<Value, QueryError> {
    status_in_range(&args[0], 300, 399, "http_redirect")
}

fn bi_http_client_error(args: &[Value]) -> Result<Value, QueryError> {
    status_in_range(&args[0], 400, 499, "http_client_error")
}

fn bi_http_server_error(args: &[Value]) -> Result<Value, QueryError> {
    status_in_range(&args[0], 500, 599, "http_server_error")
}

/// String coercion of a value for the non-string `body` coercion path.
fn py_str(v: &Value) -> String {
    match v {
        Value::Str(s) => s.clone(),
        Value::Int(i) => i.to_string(),
        Value::Float(f) => crate::jsonfmt::py_float_repr(*f),
        Value::Bool(b) => if *b { "True" } else { "False" }.to_owned(),
        Value::Null => "None".to_owned(),
        other => crate::jsonfmt::to_compact(other),
    }
}
