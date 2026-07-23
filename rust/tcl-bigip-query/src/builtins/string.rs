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

//! String-category builtins.
//!
//! This module covers the non-regex string builtins; the regex family
//! (`match` / `sub` / `gsub` / `test` / `scan` / `capture` / `splits`) lives
//! in [`crate::builtins::regex_str`] atop `safe_regex_compile`.

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
    // Equality coerces both sides' PathRef to full_path.
    let eq = |item: &Value, target: &Value| -> bool {
        let a = match item {
            Value::PathRef(p) => Value::Str(p.full_path.clone()),
            other => other.clone(),
        };
        let b = match target {
            Value::PathRef(p) => Value::Str(p.full_path.clone()),
            other => other.clone(),
        };
        value::py_eq(&a, &b, 0)
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
    // `.upper()` is full-Unicode; the DSL's ascii_upcase / upcase
    // both delegate to it.
    Ok(Value::Str(as_str(&args[0], "upcase", 1)?.to_uppercase()))
}

fn bi_lower(args: &[Value]) -> Result<Value, QueryError> {
    Ok(Value::Str(as_str(&args[0], "downcase", 1)?.to_lowercase()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(text: &str) -> Value {
        Value::Str(text.to_owned())
    }

    fn call_str(f: fn(&[Value]) -> Result<Value, QueryError>, args: &[Value]) -> String {
        match f(args) {
            Ok(Value::Str(t)) => t,
            other => panic!("expected Str, got {other:?}"),
        }
    }

    fn call_bool(f: fn(&[Value]) -> Result<Value, QueryError>, args: &[Value]) -> bool {
        match f(args) {
            Ok(Value::Bool(b)) => b,
            other => panic!("expected Bool, got {other:?}"),
        }
    }

    fn call_strlist(f: fn(&[Value]) -> Result<Value, QueryError>, args: &[Value]) -> Vec<String> {
        match f(args) {
            Ok(Value::List(items)) => items
                .iter()
                .map(|v| match v {
                    Value::Str(t) => t.clone(),
                    other => panic!("expected Str element, got {other:?}"),
                })
                .collect(),
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn ascii_case_conversions() {
        assert_eq!(call_str(bi_upper, &[s("abc")]), "ABC");
        assert_eq!(call_str(bi_lower, &[s("ABC")]), "abc");
    }

    #[test]
    fn predicates_split_and_join() {
        // startswith / endswith / contains / split / join — exercised by the
        // f5-query battery via `select(.name | startswith(…))`-style
        // expressions; covered here directly.
        assert!(call_bool(bi_startswith, &[s("vs_prod"), s("vs_")]));
        assert!(!call_bool(bi_startswith, &[s("api_vs"), s("vs_")]));
        assert!(call_bool(bi_endswith, &[s("web_pool"), s("_pool")]));
        assert!(call_bool(bi_contains, &[s("foobar"), s("oba")]));
        assert!(!call_bool(bi_contains, &[s("foobar"), s("xyz")]));
        assert_eq!(
            call_strlist(bi_split, &[s("a,b,c"), s(",")]),
            ["a", "b", "c"]
        );
        assert_eq!(
            call_str(
                bi_join,
                &[Value::List(vec![s("a"), s("b"), s("c")]), s("-")]
            ),
            "a-b-c"
        );
    }
}
