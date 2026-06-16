//! String-category builtins (port of the `string` section of `builtins.py`).
//!
//! This first batch covers the non-regex string builtins; the regex family
//! (`match` / `sub` / `gsub` / `test` / `scan` / `capture` / `splits`) lands
//! in a later batch atop `safe_regex_compile`.

use crate::builtins::{BuiltinSpec, as_sequence, as_str, plain, type_name};
use crate::errors::QueryError;
use crate::value::{self, Value};

pub(super) fn registrations() -> Vec<(&'static str, BuiltinSpec)> {
    vec![
        plain("startswith", "string", 2, Some(2), false, bi_startswith),
        plain("endswith", "string", 2, Some(2), false, bi_endswith),
        plain("contains", "string", 2, Some(2), false, bi_contains),
        plain("split", "string", 2, Some(2), false, bi_split),
        plain("join", "string", 2, Some(2), true, bi_join),
        plain("ascii_upcase", "string", 1, Some(1), false, bi_upper),
        plain("ascii_downcase", "string", 1, Some(1), false, bi_lower),
        plain("upcase", "string", 1, Some(1), false, bi_upper),
        plain("downcase", "string", 1, Some(1), false, bi_lower),
    ]
}

fn bi_startswith(args: &[Value]) -> Result<Value, QueryError> {
    let value = as_str(&args[0], "startswith", 1)?;
    let prefix = as_str(&args[1], "startswith", 2)?;
    Ok(Value::Bool(value.starts_with(&prefix)))
}

fn bi_endswith(args: &[Value]) -> Result<Value, QueryError> {
    let value = as_str(&args[0], "endswith", 1)?;
    let suffix = as_str(&args[1], "endswith", 2)?;
    Ok(Value::Bool(value.ends_with(&suffix)))
}

fn bi_contains(args: &[Value]) -> Result<Value, QueryError> {
    let value = &args[0];
    let needle = &args[1];
    // `_eq` from the Python impl coerces both sides' PathRef to full_path.
    let eq = |item: &Value, target: &Value| -> bool {
        let a = match item {
            Value::PathRef(p) => Value::Str(p.full_path.clone()),
            other => other.clone(),
        };
        let b = match target {
            Value::PathRef(p) => Value::Str(p.full_path.clone()),
            other => other.clone(),
        };
        value::py_eq(&a, &b)
    };
    match value {
        Value::Str(s) => Ok(Value::Bool(s.contains(&as_str(needle, "contains", 2)?))),
        Value::List(items) | Value::Stream(items) => {
            Ok(Value::Bool(items.iter().any(|item| eq(item, needle))))
        }
        Value::PathRef(p) => Ok(Value::Bool(
            p.full_path.contains(&as_str(needle, "contains", 2)?),
        )),
        other => Err(QueryError::builtin(format!(
            "contains: cannot search inside {}",
            type_name(other)
        ))),
    }
}

fn bi_split(args: &[Value]) -> Result<Value, QueryError> {
    let value = as_str(&args[0], "split", 1)?;
    let sep = as_str(&args[1], "split", 2)?;
    let parts: Vec<Value> = value
        .split(&sep)
        .map(|s| Value::Str(s.to_string()))
        .collect();
    Ok(Value::List(parts))
}

fn bi_join(args: &[Value]) -> Result<Value, QueryError> {
    let items = as_sequence(&args[0], "join", 1)?;
    let sep = as_str(&args[1], "join", 2)?;
    let mut parts = Vec::with_capacity(items.len());
    for item in &items {
        parts.push(as_str(item, "join", 1)?);
    }
    Ok(Value::Str(parts.join(&sep)))
}

fn bi_upper(args: &[Value]) -> Result<Value, QueryError> {
    // Python `.upper()` is full-Unicode; the DSL's ascii_upcase / upcase
    // both delegate to it.
    Ok(Value::Str(as_str(&args[0], "upcase", 1)?.to_uppercase()))
}

fn bi_lower(args: &[Value]) -> Result<Value, QueryError> {
    Ok(Value::Str(as_str(&args[0], "downcase", 1)?.to_lowercase()))
}
