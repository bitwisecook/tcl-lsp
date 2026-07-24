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

//! Value / stream / path-category builtins: `defined` / `in` / `kind` /
//! `path` / `json_parse` (value); `all` / `any` / `halt` / `halt_error` /
//! `inside` / `limit` / `nth` (stream); `basename` / `partition` /
//! `with_partition` (path).
//!
//! Behaviour notes:
//! - `in(key)` is the inverse of `has` and `inside(container)` the inverse
//!   of `contains`; both delegate to the already-registered implementations
//!   via [`crate::builtins::lookup`] with the arguments swapped, so their
//!   coercion / error wording matches the originals exactly.
//! - `all` / `any` flatten one level before applying the truthiness test,
//!   recovering the "did the predicate hold?" semantics for
//!   `any(stream | map(predicate))`.
//! - `halt` / `halt_error` raise `BuiltinError`; `halt_error` reads an
//!   optional integer exit-code argument (default 5) and produces the
//!   `halt_error: query halted (exit_code=N)` text.
//! - `json_parse` parses a JSON string into the value model (objects
//!   preserve key order, integers stay `Int`); the JSON parse-error line /
//!   column wording is custom (divergent), so error cases stay out of the
//!   golden fixture.
//! - `partition` / `basename` / `with_partition` are TMSH path-string
//!   helpers operating purely on the `/`-segmented string.

use crate::builtins::{
    Builtin, BuiltinSpec, as_int, as_sequence, as_str, lookup, plain, type_name,
};
use crate::errors::QueryError;
use crate::value::{Value, truthy};

use super::encoding::json_to_value;

pub(super) fn registrations() -> Vec<(&'static str, BuiltinSpec)> {
    vec![
        // value
        plain("defined", "value", 1, Some(1), false, bi_defined),
        plain("in", "value", 2, Some(2), false, bi_in),
        plain("kind", "value", 1, Some(1), false, bi_kind),
        plain("path", "value", 1, Some(1), false, bi_path),
        plain("json_parse", "value", 1, Some(1), false, bi_json_parse),
        // stream
        plain("all", "stream", 1, Some(1), true, bi_all),
        plain("any", "stream", 1, Some(1), true, bi_any),
        plain("halt", "stream", 0, Some(0), false, bi_halt),
        plain("halt_error", "stream", 0, Some(1), false, bi_halt_error),
        plain("inside", "stream", 2, Some(2), false, bi_inside),
        plain("limit", "stream", 2, Some(2), true, bi_limit),
        plain("nth", "stream", 2, Some(2), true, bi_nth),
        // path
        plain("basename", "path", 1, Some(1), false, bi_basename),
        plain("partition", "path", 1, Some(1), false, bi_partition),
        plain(
            "with_partition",
            "path",
            2,
            Some(2),
            false,
            bi_with_partition,
        ),
    ]
}

fn bi_defined(args: &[Value]) -> Result<Value, QueryError> {
    let defined = match &args[0] {
        Value::Null => false,
        Value::Str(s) => !s.is_empty(),
        Value::PathRef(p) => !p.full_path.is_empty(),
        _ => true,
    };
    Ok(Value::Bool(defined))
}

/// `in(key, value)` == `has(value, key)`.
fn bi_in(args: &[Value]) -> Result<Value, QueryError> {
    call_plain("has", &[args[1].clone(), args[0].clone()])
}

/// `inside(needle, container)` == `contains(container, needle)`.
fn bi_inside(args: &[Value]) -> Result<Value, QueryError> {
    call_plain("contains", &[args[1].clone(), args[0].clone()])
}

/// Dispatch one of the registry's plain builtins by name.
fn call_plain(name: &str, args: &[Value]) -> Result<Value, QueryError> {
    let spec = lookup(name).expect("builtin registered");
    match &spec.imp {
        Builtin::Plain(f) => f(args),
        _ => unreachable!("{name} is a plain builtin"),
    }
}

fn bi_kind(args: &[Value]) -> Result<Value, QueryError> {
    match &args[0] {
        Value::ObjectRef(o) => Ok(Value::Str(o.kind.clone())),
        Value::PathRef(p) => Ok(Value::Str(p.expected_kind.clone())),
        other => Err(QueryError::builtin(format!(
            "kind: argument 1 must be an object, got {}",
            type_name(other)
        ))),
    }
}

fn bi_path(args: &[Value]) -> Result<Value, QueryError> {
    match &args[0] {
        Value::ObjectRef(o) => Ok(Value::Str(o.full_path.clone())),
        Value::PathRef(p) => Ok(Value::Str(p.full_path.clone())),
        other => Err(QueryError::builtin(format!(
            "path: cannot take path of {}",
            type_name(other)
        ))),
    }
}

fn bi_json_parse(args: &[Value]) -> Result<Value, QueryError> {
    let text = as_str(&args[0], "json_parse", 1)?;
    let parsed: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
        // The JSON parse-error wording is custom (divergent); keep error cases
        // out of the fixture. We surface the parse failure with line/column to
        // stay shape-compatible.
        QueryError::builtin(format!(
            "json_parse: invalid JSON ({} at line {} col {})",
            e,
            e.line(),
            e.column()
        ))
    })?;
    Ok(json_to_value(&parsed, 0))
}

fn bi_all(args: &[Value]) -> Result<Value, QueryError> {
    let items = flatten_one_level(&args[0], "all")?;
    Ok(Value::Bool(items.iter().all(truthy)))
}

fn bi_any(args: &[Value]) -> Result<Value, QueryError> {
    let items = flatten_one_level(&args[0], "any")?;
    Ok(Value::Bool(items.iter().any(truthy)))
}

/// Coerce to a sequence, then flatten one level of
/// lists when *every* element is itself a list.
fn flatten_one_level(value: &Value, name: &str) -> Result<Vec<Value>, QueryError> {
    let seq = as_sequence(value, name, 1)?;
    if !seq.is_empty() && seq.iter().all(|item| matches!(item, Value::List(_))) {
        let mut flat = Vec::new();
        for sub in seq {
            if let Value::List(inner) = sub {
                flat.extend(inner);
            }
        }
        Ok(flat)
    } else {
        Ok(seq)
    }
}

fn bi_halt(_args: &[Value]) -> Result<Value, QueryError> {
    Err(QueryError::builtin("halt: query halted by halt()"))
}

fn bi_halt_error(args: &[Value]) -> Result<Value, QueryError> {
    let exit_code = if args.is_empty() {
        5
    } else {
        as_int(&args[0], "halt_error", 1)?
    };
    Err(QueryError::builtin(format!(
        "halt_error: query halted (exit_code={exit_code})"
    )))
}

fn bi_limit(args: &[Value]) -> Result<Value, QueryError> {
    let items = as_sequence(&args[0], "limit", 1)?;
    let count = as_int(&args[1], "limit", 2)?;
    if count <= 0 {
        return Ok(Value::List(Vec::new()));
    }
    let n = (count as usize).min(items.len());
    Ok(Value::List(items[..n].to_vec()))
}

fn bi_nth(args: &[Value]) -> Result<Value, QueryError> {
    let items = as_sequence(&args[0], "nth", 1)?;
    let idx = as_int(&args[1], "nth", 2)?;
    let len = items.len() as i64;
    if -len <= idx && idx < len {
        let resolved = if idx < 0 { idx + len } else { idx } as usize;
        Ok(items[resolved].clone())
    } else {
        Ok(Value::Null)
    }
}

fn bi_partition(args: &[Value]) -> Result<Value, QueryError> {
    let s = as_str(&args[0], "partition", 1)?;
    if !s.starts_with('/') {
        return Ok(Value::Str(String::new()));
    }
    // Split on `/` into at most 3 parts => ["", first, rest?]; index 1 is the
    // first segment after the leading slash.
    let part = s.split('/').nth(1).unwrap_or("");
    Ok(Value::Str(part.to_string()))
}

fn bi_basename(args: &[Value]) -> Result<Value, QueryError> {
    let s = as_str(&args[0], "basename", 1)?;
    let base = s.rsplit('/').next().unwrap_or("");
    Ok(Value::Str(base.to_string()))
}

fn bi_with_partition(args: &[Value]) -> Result<Value, QueryError> {
    let s = as_str(&args[0], "with_partition", 1)?;
    let p = as_str(&args[1], "with_partition", 2)?;
    if p.is_empty() {
        return Err(QueryError::builtin(
            "with_partition: partition must not be empty",
        ));
    }
    let base = s.rsplit('/').next().unwrap_or("");
    Ok(Value::Str(format!("/{p}/{base}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(text: &str) -> Value {
        Value::Str(text.to_owned())
    }

    fn call_bool(f: fn(&[Value]) -> Result<Value, QueryError>, args: &[Value]) -> bool {
        match f(args) {
            Ok(Value::Bool(b)) => b,
            other => panic!("expected Bool, got {other:?}"),
        }
    }

    fn call_str(f: fn(&[Value]) -> Result<Value, QueryError>, args: &[Value]) -> String {
        match f(args) {
            Ok(Value::Str(t)) => t,
            other => panic!("expected Str, got {other:?}"),
        }
    }

    #[test]
    fn all_any_follow_jq_empty_semantics() {
        // jq behaviour: all([]) is true (vacuous), any([]) is false; null/false
        // are falsy.
        assert!(call_bool(bi_all, &[Value::List(vec![])]));
        assert!(!call_bool(bi_any, &[Value::List(vec![])]));
        assert!(call_bool(
            bi_all,
            &[Value::List(vec![Value::Bool(true), Value::Int(1)])]
        ));
        assert!(!call_bool(
            bi_all,
            &[Value::List(vec![Value::Bool(true), Value::Bool(false)])]
        ));
        assert!(call_bool(
            bi_any,
            &[Value::List(vec![Value::Bool(false), Value::Int(1)])]
        ));
        assert!(!call_bool(
            bi_any,
            &[Value::List(vec![Value::Null, Value::Bool(false)])]
        ));
    }

    #[test]
    fn basename_takes_last_path_segment() {
        assert_eq!(call_str(bi_basename, &[s("/Common/web_pool")]), "web_pool");
        assert_eq!(call_str(bi_basename, &[s("noslash")]), "noslash");
    }
}
