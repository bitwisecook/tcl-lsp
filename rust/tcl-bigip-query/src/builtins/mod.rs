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

//! Builtin function library for the query DSL.
//!
//! This module owns the registry (`BuiltinSpec` + the dispatch metadata the
//! evaluator reads), the shared argument-coercion / value-walking helpers,
//! and the **plain** builtins. The special-form builtins (`select`, `map`,
//! the `paths` / `getpath` family, …) are dispatched by [`crate::special`].
//!
//! Plain builtins raise [`QueryError::Builtin`] for argument-type mistakes so
//! the CLI can map them to `error:` uniformly.

// The DSL models an unbounded `int`; index / length / numeric
// conversions between `usize`, `i64`, and `f64` are intentional and
// pervasive.
#![allow(
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::match_same_arms,
    clippy::match_wildcard_for_single_variants,
    clippy::unnecessary_wraps
)]

use std::collections::HashMap;
use std::sync::OnceLock;

use indexmap::IndexMap;
use regex::Regex;

use crate::errors::QueryError;
use crate::eval::EvalContext;
use crate::jsonfmt;
use crate::value::{self, MAX_VALUE_WALK_DEPTH, Value};

mod encoding;
pub(crate) use encoding::json_to_value;
mod extras;
pub mod f5profile;
mod files;
mod graph;
mod math;
mod net;

pub(crate) use graph::extract_rule_refs;
mod inputs_load;
mod regex_str;
mod rename;
mod string;
mod time_dt;
mod value2;

/// How a builtin is invoked.
pub enum Builtin {
    /// Eager: receives already-evaluated argument values.
    Plain(fn(&[Value]) -> Result<Value, QueryError>),
    /// Receives the evaluator context (for `refs` / edit-registering verbs).
    Ctx(fn(&[Value], &mut EvalContext) -> Result<Value, QueryError>),
    /// Dispatched by [`crate::special`] against the unevaluated AST.
    Special,
}

/// The registry entry for one builtin, minus the doc-only `summary` /
/// `signatures` / `examples` / `details` fields.
///
/// Each flag is an intrinsic dispatch property rather than a state-machine
/// smell, so the several bools are expected here.
#[allow(clippy::struct_excessive_bools)]
pub struct BuiltinSpec {
    pub name: &'static str,
    pub category: &'static str,
    pub min_args: usize,
    pub max_args: Option<usize>,
    pub special_form: bool,
    pub with_ctx: bool,
    pub stream_aware: bool,
    /// Whether stream arguments broadcast element-wise (the scalar-builtin
    /// default). `with_ctx` builtins normally skip broadcast; `refs` /
    /// `referenced_by` are the exception (plain — and so broadcasting —
    /// but need `ctx` for the config here).
    pub broadcasts: bool,
    pub imp: Builtin,
}

static REGISTRY: OnceLock<HashMap<&'static str, BuiltinSpec>> = OnceLock::new();

/// Look up a builtin by name.
#[must_use]
pub fn lookup(name: &str) -> Option<&'static BuiltinSpec> {
    REGISTRY.get_or_init(build_registry).get(name)
}

/// Every registered builtin, sorted by `(category, name)`.
#[must_use]
pub fn all_specs() -> Vec<&'static BuiltinSpec> {
    let mut specs: Vec<&'static BuiltinSpec> =
        REGISTRY.get_or_init(build_registry).values().collect();
    specs.sort_by(|a, b| a.category.cmp(b.category).then_with(|| a.name.cmp(b.name)));
    specs
}

/// Human-readable arity, e.g. `2`, `1..3`, or `1+` (variadic).
fn arity(spec: &BuiltinSpec) -> String {
    match spec.max_args {
        Some(max) if max == spec.min_args => spec.min_args.to_string(),
        Some(max) => format!("{}..{max}", spec.min_args),
        None => format!("{}+", spec.min_args),
    }
}

/// Render the builtin catalogue. With `filter = Some(name)` shows the single
/// matching builtin's metadata; otherwise lists every builtin grouped by
/// category.
///
/// This is the idiomatic equivalent of `f5 query --help-builtins`:
/// the [`BuiltinSpec`] carries the dispatch metadata (name, category, arity,
/// flags) but not the full doc prose, so the catalogue surfaces what the
/// registry actually knows and points at the reference docs for the rest.
#[must_use]
pub fn format_catalogue(filter: Option<&str>) -> String {
    use std::fmt::Write as _;
    let specs = all_specs();

    if let Some(name) = filter {
        let Some(spec) = specs.iter().find(|s| s.name == name) else {
            return format!(
                "f5 query: no builtin named '{name}'\n\n\
                 Run `f5 query --help-builtins` for the full catalogue.\n"
            );
        };
        let mut out = String::new();
        let _ = writeln!(out, "{}", spec.name);
        let _ = writeln!(out, "  category: {}", spec.category);
        let _ = writeln!(out, "  arity:    {} arg(s)", arity(spec));
        let mut flags: Vec<&str> = Vec::new();
        if spec.special_form {
            flags.push("special-form");
        }
        if spec.with_ctx {
            flags.push("config-aware");
        }
        if spec.stream_aware {
            flags.push("stream-aware");
        }
        if !flags.is_empty() {
            let _ = writeln!(out, "  flags:    {}", flags.join(", "));
        }
        return out;
    }

    let mut out = String::new();
    let _ = writeln!(
        out,
        "F5 QUERY DSL — BUILTIN FUNCTIONS ({} total)",
        specs.len()
    );
    out.push('\n');
    out.push_str(
        "Each entry is name (arity). Arity is shown as N, MIN..MAX, or MIN+ for\n\
         variadic. For full signatures, prose, and examples see\n\
         docs/references/f5_query/ or run `f5 query --help-builtins NAME`.\n\n",
    );
    let mut current = "";
    for spec in &specs {
        if spec.category != current {
            if !current.is_empty() {
                out.push('\n');
            }
            let _ = writeln!(out, "[{}]", spec.category);
            current = spec.category;
        }
        let _ = writeln!(out, "  {:<26} ({})", spec.name, arity(spec));
    }
    out
}

pub(crate) fn plain(
    name: &'static str,
    category: &'static str,
    min_args: usize,
    max_args: Option<usize>,
    stream_aware: bool,
    f: fn(&[Value]) -> Result<Value, QueryError>,
) -> (&'static str, BuiltinSpec) {
    (
        name,
        BuiltinSpec {
            name,
            category,
            min_args,
            max_args,
            special_form: false,
            with_ctx: false,
            stream_aware,
            broadcasts: true,
            imp: Builtin::Plain(f),
        },
    )
}

pub(crate) fn ctx(
    name: &'static str,
    category: &'static str,
    min_args: usize,
    max_args: Option<usize>,
    f: fn(&[Value], &mut EvalContext) -> Result<Value, QueryError>,
) -> (&'static str, BuiltinSpec) {
    ctx_spec(name, category, min_args, max_args, false, f)
}

/// Like [`ctx`] but with stream arguments broadcasting element-wise — for the
/// `refs` / `referenced_by` builtins, which are plain (broadcasting)
/// but need `ctx` for config access here.
pub(crate) fn ctx_broadcast(
    name: &'static str,
    category: &'static str,
    min_args: usize,
    max_args: Option<usize>,
    f: fn(&[Value], &mut EvalContext) -> Result<Value, QueryError>,
) -> (&'static str, BuiltinSpec) {
    ctx_spec(name, category, min_args, max_args, true, f)
}

fn ctx_spec(
    name: &'static str,
    category: &'static str,
    min_args: usize,
    max_args: Option<usize>,
    broadcasts: bool,
    f: fn(&[Value], &mut EvalContext) -> Result<Value, QueryError>,
) -> (&'static str, BuiltinSpec) {
    (
        name,
        BuiltinSpec {
            name,
            category,
            min_args,
            max_args,
            special_form: false,
            with_ctx: true,
            stream_aware: false,
            broadcasts,
            imp: Builtin::Ctx(f),
        },
    )
}

pub(crate) fn special(
    name: &'static str,
    category: &'static str,
    min_args: usize,
    max_args: Option<usize>,
) -> (&'static str, BuiltinSpec) {
    (
        name,
        BuiltinSpec {
            name,
            category,
            min_args,
            max_args,
            special_form: true,
            with_ctx: false,
            stream_aware: false,
            broadcasts: false,
            imp: Builtin::Special,
        },
    )
}

fn build_registry() -> HashMap<&'static str, BuiltinSpec> {
    let mut m = HashMap::new();
    for (name, spec) in registrations() {
        m.insert(name, spec);
    }
    m
}

fn registrations() -> Vec<(&'static str, BuiltinSpec)> {
    let mut r = vec![
        // value / stream
        plain("length", "value", 1, Some(1), true, bi_length),
        plain("str", "value", 1, Some(1), false, bi_str),
        plain("type", "value", 1, Some(1), false, bi_type),
        plain("has", "value", 2, Some(2), false, bi_has),
        plain("keys", "stream", 1, Some(1), true, bi_keys),
        plain(
            "keys_unsorted",
            "stream",
            1,
            Some(1),
            true,
            bi_keys_unsorted,
        ),
        plain("values", "stream", 1, Some(1), true, bi_values),
        plain("add", "stream", 1, Some(1), true, bi_add),
        plain("sort", "stream", 1, Some(1), true, bi_sort),
        plain("unique", "stream", 1, Some(1), true, bi_unique),
        plain("dupes", "stream", 1, Some(1), true, bi_dupes),
        plain("reverse", "stream", 1, Some(1), true, bi_reverse),
        plain("first", "stream", 1, Some(1), true, bi_first),
        plain("last", "stream", 1, Some(1), true, bi_last),
        plain("count", "stream", 1, Some(1), true, bi_count),
        plain("min", "stream", 1, Some(1), true, bi_min),
        plain("max", "stream", 1, Some(1), true, bi_max),
        plain("range", "stream", 1, Some(3), false, bi_range),
        plain("empty", "stream", 0, Some(0), false, bi_empty),
        plain("error", "stream", 0, Some(1), false, bi_error),
        plain("to_entries", "value", 1, Some(1), true, bi_to_entries),
        plain("from_entries", "value", 1, Some(1), true, bi_from_entries),
        plain("tonumber", "string", 1, Some(1), false, bi_tonumber),
        plain("tostring", "string", 1, Some(1), false, bi_tostring),
        // math
        plain("floor", "math", 1, Some(1), false, bi_floor),
        plain("ceil", "math", 1, Some(1), false, bi_ceil),
        plain("abs", "math", 1, Some(1), false, bi_abs),
    ];
    r.extend(string::registrations());
    r.extend(math::registrations());
    r.extend(net::registrations());
    r.extend(time_dt::registrations());
    r.extend(regex_str::registrations());
    r.extend(value2::registrations());
    r.extend(inputs_load::registrations());
    r.extend(encoding::registrations());
    r.extend(graph::registrations());
    r.extend(rename::registrations());
    r.extend(extras::registrations());
    r.extend(f5profile::registrations());
    r.extend(files::registrations());
    #[cfg(feature = "probes")]
    r.extend(crate::probes::registrations());
    r.extend(crate::special::registrations());
    r
}

// Shared coercion / walking helpers

/// The type name of a value.
#[must_use]
pub(crate) fn type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Int(_) => "int",
        Value::Float(_) => "float",
        Value::Str(_) => "string",
        Value::PathRef(_) => "path-ref",
        Value::ObjectRef(_) => "object",
        Value::Stream(_) => "stream",
        Value::List(_) => "list",
        Value::Object(_) => "dict",
        // Falls back to the type name for a Container.
        Value::Container(_) => "Container",
        Value::Drop => "Drop",
    }
}

/// Coerce a value to a sequence (list or stream) of elements.
pub(crate) fn as_sequence(v: &Value, name: &str, arg: usize) -> Result<Vec<Value>, QueryError> {
    match v {
        Value::List(items) | Value::Stream(items) => Ok(items.clone()),
        other => Err(QueryError::builtin(format!(
            "{name}: argument {arg} must be a list or stream, got {}",
            type_name(other)
        ))),
    }
}

/// Coerce a value to a string.
pub(crate) fn as_str(v: &Value, name: &str, arg: usize) -> Result<String, QueryError> {
    match v {
        Value::PathRef(p) => Ok(p.full_path.clone()),
        Value::Str(s) => Ok(s.clone()),
        other => Err(QueryError::builtin(format!(
            "{name}: argument {arg} must be a string, got {}",
            type_name(other)
        ))),
    }
}

/// Coerce a value to an integer.
pub(crate) fn as_int(v: &Value, name: &str, arg: usize) -> Result<i64, QueryError> {
    match v {
        Value::Bool(_) => Err(QueryError::builtin(format!(
            "{name}: argument {arg} must be an integer, got bool"
        ))),
        Value::Int(i) => Ok(*i),
        other => Err(QueryError::builtin(format!(
            "{name}: argument {arg} must be an integer, got {}",
            type_name(other)
        ))),
    }
}

/// Coerce a value to a number, preserving int / float.
pub(crate) fn as_number(v: &Value, name: &str, arg: usize) -> Result<Value, QueryError> {
    match v {
        Value::Bool(_) => Err(QueryError::builtin(format!(
            "{name}: argument {arg} must be a number, got bool"
        ))),
        Value::Int(i) => Ok(Value::Int(*i)),
        Value::Float(f) => Ok(Value::Float(*f)),
        other => Err(QueryError::builtin(format!(
            "{name}: argument {arg} must be a number, got {}",
            type_name(other)
        ))),
    }
}

/// Sorted `(key, value)` pairs of an object.
pub(crate) fn object_entries(v: &Value, name: &str) -> Result<Vec<(String, Value)>, QueryError> {
    match v {
        Value::ObjectRef(o) => {
            let mut keys: Vec<&String> = o.fields.keys().collect();
            keys.sort();
            Ok(keys
                .into_iter()
                .map(|k| (k.clone(), o.fields[k].clone()))
                .collect())
        }
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            Ok(keys
                .into_iter()
                .map(|k| (k.clone(), map[k].clone()))
                .collect())
        }
        other => Err(QueryError::builtin(format!(
            "{name}: argument 1 must be an object, got {}",
            type_name(other)
        ))),
    }
}

/// Coerce a value to a JSON-shaped value.
///
/// `depth` is the nesting level of this call (0 at the top); past
/// [`MAX_VALUE_WALK_DEPTH`] this returns `Value::Null` in place of the
/// over-deep subtree instead of recursing further — issue #996.
#[must_use]
pub(crate) fn to_jsonable(v: &Value, depth: u32) -> Value {
    if MAX_VALUE_WALK_DEPTH.exceeded(depth) {
        return Value::Null;
    }
    match v {
        Value::Null | Value::Bool(_) | Value::Int(_) | Value::Float(_) | Value::Str(_) => v.clone(),
        Value::PathRef(p) => Value::Str(p.full_path.clone()),
        Value::List(items) | Value::Stream(items) => Value::List(
            items
                .iter()
                .map(|item| to_jsonable(item, depth + 1))
                .collect(),
        ),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, val)| (k.clone(), to_jsonable(val, depth + 1)))
                .collect(),
        ),
        Value::ObjectRef(o) => {
            let mut keys: Vec<&String> = o.fields.keys().collect();
            keys.sort();
            let mut m = IndexMap::new();
            for k in keys {
                m.insert(k.clone(), to_jsonable(&o.fields[k], depth + 1));
            }
            Value::Object(m)
        }
        // Falls back to `str(value)` for a Container.
        Value::Container(c) => Value::Str(format!("container({})", c.kind)),
        Value::Drop => Value::Str("Drop".to_string()),
    }
}

pub(crate) fn is_container(v: &Value) -> bool {
    matches!(
        v,
        Value::Object(_) | Value::List(_) | Value::Stream(_) | Value::ObjectRef(_)
    )
}

/// `(key, child)` pairs for a composite value.
/// Keys are `Value::Str` (objects, sorted) or `Value::Int` (lists).
#[must_use]
pub(crate) fn value_children(v: &Value) -> Vec<(Value, Value)> {
    match v {
        Value::ObjectRef(o) => {
            let mut keys: Vec<&String> = o.fields.keys().collect();
            keys.sort();
            keys.into_iter()
                .map(|k| (Value::Str(k.clone()), o.fields[k].clone()))
                .collect()
        }
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            keys.into_iter()
                .map(|k| (Value::Str(k.clone()), map[k].clone()))
                .collect()
        }
        Value::List(items) | Value::Stream(items) => items
            .iter()
            .enumerate()
            .map(|(i, child)| (Value::Int(i as i64), child.clone()))
            .collect(),
        _ => Vec::new(),
    }
}

/// All paths through a value (only leaf paths when `only_leaves`).
#[must_use]
pub(crate) fn all_paths(v: &Value, only_leaves: bool) -> Vec<Vec<Value>> {
    let mut out = Vec::new();
    let mut prefix = Vec::new();
    walk_paths(v, &mut prefix, only_leaves, &mut out, 0);
    out
}

/// `depth` is the nesting level of this call (0 at the top); past
/// [`MAX_VALUE_WALK_DEPTH`] this stops descending into `cur`'s children —
/// the paths collected so far still stand — rather than recursing further
/// (issue #996).
fn walk_paths(
    cur: &Value,
    prefix: &mut Vec<Value>,
    only_leaves: bool,
    out: &mut Vec<Vec<Value>>,
    depth: u32,
) {
    if !prefix.is_empty() {
        if only_leaves {
            if !is_container(cur) {
                out.push(prefix.clone());
            }
        } else {
            out.push(prefix.clone());
        }
    }
    if MAX_VALUE_WALK_DEPTH.exceeded(depth) {
        return;
    }
    for (key, child) in value_children(cur) {
        prefix.push(key);
        walk_paths(&child, prefix, only_leaves, out, depth + 1);
        prefix.pop();
    }
}

/// Read the value at a path; `Null` for missing keys.
#[must_use]
pub(crate) fn value_at_path(value: &Value, path: &[Value]) -> Value {
    let mut cur = value.clone();
    for key in path {
        cur = match &cur {
            Value::Null => return Value::Null,
            Value::ObjectRef(o) => match key {
                Value::Str(k) => match o.fields.get(k) {
                    Some(v) => v.clone(),
                    None => return Value::Null,
                },
                _ => return Value::Null,
            },
            Value::Object(map) => match key {
                Value::Str(k) => match map.get(k) {
                    Some(v) => v.clone(),
                    None => return Value::Null,
                },
                _ => return Value::Null,
            },
            Value::List(items) | Value::Stream(items) => match key {
                Value::Int(i) => match value::resolve_index_pub(*i, items.len()) {
                    Some(idx) => items[idx].clone(),
                    None => return Value::Null,
                },
                _ => return Value::Null,
            },
            _ => return Value::Null,
        };
    }
    cur
}

/// Coerce a single path key to its canonical form.
fn coerce_path_key(key: &Value, name: &str, idx: usize) -> Result<Value, QueryError> {
    match key {
        Value::Bool(_) => Err(QueryError::builtin(format!(
            "{name}: path[{idx}] cannot be bool"
        ))),
        Value::Int(_) | Value::Str(_) => Ok(key.clone()),
        Value::PathRef(p) => Ok(Value::Str(p.full_path.clone())),
        other => Err(QueryError::builtin(format!(
            "{name}: path[{idx}] must be a string or integer, got {}",
            type_name(other)
        ))),
    }
}

/// Coerce a path argument to a list of canonical keys.
pub(crate) fn coerce_path_list(
    path: &Value,
    name: &str,
    arg: usize,
) -> Result<Vec<Value>, QueryError> {
    let seq = as_sequence(path, name, arg)?;
    seq.iter()
        .enumerate()
        .map(|(i, k)| coerce_path_key(k, name, i))
        .collect()
}

/// Set the value at a path, creating intermediate containers as needed.
///
/// `depth` is the nesting level of this call (0 at the top), which for this
/// function is the index into `path` rather than `value`'s own nesting —
/// `path` is itself query-built and unbounded (e.g. `range(1000000)` alone
/// builds a path far longer than any real document nests). Past
/// [`MAX_VALUE_WALK_DEPTH`] this returns [`QueryError::Builtin`] instead of
/// descending further (issue #996).
pub(crate) fn set_at_path(
    value: Value,
    path: &[Value],
    new_value: Value,
    depth: u32,
) -> Result<Value, QueryError> {
    if path.is_empty() {
        return Ok(new_value);
    }
    if MAX_VALUE_WALK_DEPTH.exceeded(depth) {
        return Err(QueryError::builtin(
            "setpath: path nests too deeply".to_string(),
        ));
    }
    let head = &path[0];
    let rest = &path[1..];
    let value = match value {
        Value::Null => {
            if matches!(head, Value::Str(_)) {
                Value::Object(IndexMap::new())
            } else {
                Value::List(Vec::new())
            }
        }
        Value::ObjectRef(o) => Value::Object(o.fields.clone()),
        other => other,
    };
    match value {
        Value::Object(mut map) => {
            let key = match head {
                Value::Str(s) => s.clone(),
                other => scalar_key_str(other),
            };
            let sub = map.get(&key).cloned().unwrap_or(Value::Null);
            let placed = set_at_path(sub, rest, new_value, depth + 1)?;
            // `IndexMap::insert` updates an existing key in place (keeping its
            // position) and appends a new one — matching dict insertion order.
            map.insert(key, placed);
            Ok(Value::Object(map))
        }
        Value::List(mut items) | Value::Stream(mut items) => {
            let mut idx = match head {
                Value::Int(i) => *i,
                other => {
                    return Err(QueryError::builtin(format!(
                        "setpath: cannot index list-like value with {} key {}",
                        type_name(other),
                        scalar_key_str(other)
                    )));
                }
            };
            if idx < 0 {
                idx += items.len() as i64;
            }
            while idx >= items.len() as i64 {
                items.push(Value::Null);
            }
            if idx < 0 {
                return Err(QueryError::builtin("setpath: negative index out of range"));
            }
            let u = idx as usize;
            let sub = items[u].clone();
            items[u] = set_at_path(sub, rest, new_value, depth + 1)?;
            Ok(Value::List(items))
        }
        other => Err(QueryError::builtin(format!(
            "setpath: cannot descend into {} at key {}",
            type_name(&other),
            scalar_key_str(head)
        ))),
    }
}

/// Delete the value at a path.
///
/// `depth` is the nesting level of this call (0 at the top), tracking
/// `path`'s index like [`set_at_path`]'s does — `path` is itself
/// query-built and unbounded. Past [`MAX_VALUE_WALK_DEPTH`] this stops
/// descending and returns `value` unchanged rather than recursing further
/// (issue #996): failing to apply a pathologically-deep delete is a safe
/// no-op, never data corruption.
pub(crate) fn delete_at_path(value: Value, path: &[Value], depth: u32) -> Value {
    if path.is_empty() {
        return Value::Null;
    }
    if MAX_VALUE_WALK_DEPTH.exceeded(depth) {
        return value;
    }
    let head = &path[0];
    let rest = &path[1..];
    match value {
        // Re-dispatches on the *same* path element in `Object` form — not a
        // path descent, so `depth` does not advance here.
        Value::ObjectRef(o) => delete_at_path(Value::Object(o.fields.clone()), path, depth),
        Value::Object(mut map) => {
            let key = match head {
                Value::Str(s) => s.clone(),
                other => scalar_key_str(other),
            };
            if !map.contains_key(&key) {
                return Value::Object(map);
            }
            if rest.is_empty() {
                map.shift_remove(&key);
            } else {
                let sub = map.get(&key).cloned().unwrap_or(Value::Null);
                map.insert(key, delete_at_path(sub, rest, depth + 1));
            }
            Value::Object(map)
        }
        Value::List(mut items) | Value::Stream(mut items) => {
            let mut idx = match head {
                Value::Int(i) => *i,
                _ => return Value::List(items),
            };
            if idx < 0 {
                idx += items.len() as i64;
            }
            if idx < 0 || idx >= items.len() as i64 {
                return Value::List(items);
            }
            let u = idx as usize;
            if rest.is_empty() {
                items.remove(u);
            } else {
                let sub = items[u].clone();
                items[u] = delete_at_path(sub, rest, depth + 1);
            }
            Value::List(items)
        }
        other => other,
    }
}

fn scalar_key_str(v: &Value) -> String {
    match v {
        Value::Str(s) => s.clone(),
        Value::Int(i) => i.to_string(),
        Value::Bool(b) => if *b { "True" } else { "False" }.to_string(),
        Value::Null => "None".to_string(),
        other => other.describe(),
    }
}

/// Compute `n`-length combinations of a value's elements.
pub(crate) fn combinations_impl(value: &Value, n: Option<i64>) -> Result<Value, QueryError> {
    let items = as_sequence(value, "combinations", 1)?;
    let sub_lists: Vec<Vec<Value>> = if let Some(n) = n {
        if n < 0 {
            return Err(QueryError::builtin(format!(
                "combinations: n must be non-negative, got {n}"
            )));
        }
        std::iter::repeat_n(items, n as usize).collect()
    } else {
        let mut subs = Vec::new();
        for (i, sub) in items.iter().enumerate() {
            match sub {
                Value::List(s) | Value::Stream(s) => subs.push(s.clone()),
                other => {
                    return Err(QueryError::builtin(format!(
                        "combinations: item {i} must be a list, got {}",
                        type_name(other)
                    )));
                }
            }
        }
        subs
    };
    let mut out = Vec::new();
    cart(&[], &sub_lists, &mut out);
    Ok(Value::Stream(out))
}

fn cart(prefix: &[Value], remaining: &[Vec<Value>], out: &mut Vec<Value>) {
    if remaining.is_empty() {
        out.push(Value::List(prefix.to_vec()));
        return;
    }
    let (head, tail) = remaining.split_first().unwrap();
    for item in head {
        let mut next = prefix.to_vec();
        next.push(item.clone());
        cart(&next, tail, out);
    }
}

/// Flatten a value up to `depth` levels.
pub(crate) fn flatten_value(value: &Value, depth: i64) -> Result<Vec<Value>, QueryError> {
    let items = as_sequence(value, "flatten", 1)?;
    if depth < 0 {
        return Err(QueryError::builtin(format!(
            "flatten: depth must be non-negative, got {depth}"
        )));
    }
    Ok(flatten_go(&items, depth, 0))
}

/// `remaining` is the caller-facing `flatten` argument's own countdown
/// (levels still to flatten); `native_depth` is this call's actual native
/// recursion depth (0 at the top), which does not track `remaining` — the
/// two only coincide when the input nests exactly as deep as the caller
/// asked to flatten. A huge `remaining` (`flatten(1000000)` is a one-line
/// query) does not by itself bound recursion; the *value*'s own nesting
/// does, and that is fully generator-controlled. Past
/// [`MAX_VALUE_WALK_DEPTH`] this stops descending — the untouched subtree
/// is returned as-is, exactly like hitting `remaining == 0` — rather than
/// recursing further (issue #996).
fn flatten_go(seq: &[Value], remaining: i64, native_depth: u32) -> Vec<Value> {
    if remaining == 0 || MAX_VALUE_WALK_DEPTH.exceeded(native_depth) {
        return seq.to_vec();
    }
    let mut out = Vec::new();
    for item in seq {
        match item {
            Value::List(s) | Value::Stream(s) => {
                out.extend(flatten_go(s, remaining - 1, native_depth + 1));
            }
            other => out.push(other.clone()),
        }
    }
    out
}

const MAX_REGEX_PATTERN_LENGTH: usize = 1024;

/// Compile a regex with length + nested-quantifier guards.
/// The pattern dialect is the `regex` crate's, which differs on
/// backreferences / lookaround (documented divergence).
pub(crate) fn safe_regex_compile(pattern: &str, name: &str) -> Result<Regex, QueryError> {
    if pattern.chars().count() > MAX_REGEX_PATTERN_LENGTH {
        return Err(QueryError::builtin(format!(
            "{name}: regex pattern too long ({} chars > {MAX_REGEX_PATTERN_LENGTH} char limit) — \
             the DSL caps pattern length to bound regex compile / match cost; \
             split the search into smaller patterns or pre-filter the input",
            pattern.chars().count()
        )));
    }
    let patho = pathological_regex();
    if patho.is_match(pattern) {
        return Err(QueryError::builtin(format!(
            "{name}: pattern {} contains a nested quantifier shape \
             (``(...+)+``, ``(...*)*``, …) that triggers catastrophic backtracking — \
             refused to keep the engine responsive.  Rewrite to use possessive \
             quantifiers, atomic groups, or a non-nested form",
            crate::lexer::py_repr_str(pattern)
        )));
    }
    // The `regex` crate's parse error wording differs, so
    // the `: {e}` tail is not byte-identical (documented divergence); the
    // `{pattern!r}` spelling still uses repr-style quoting.
    Regex::new(pattern).map_err(|e| {
        QueryError::builtin(format!(
            "{name}: invalid pattern {}: {e}",
            crate::lexer::py_repr_str(pattern)
        ))
    })
}

fn pathological_regex() -> &'static Regex {
    static P: OnceLock<Regex> = OnceLock::new();
    P.get_or_init(|| Regex::new(r"\([^)]*[+*]\)\s*[+*]").expect("valid guard regex"))
}

// Plain builtins

// char / element counts as `i64`; wrapping would need more than `i64::MAX`
// items, impossible for an in-memory value.
#[allow(clippy::cast_possible_wrap)]
fn bi_length(args: &[Value]) -> Result<Value, QueryError> {
    let v = &args[0];
    let n: i64 = match v {
        Value::Null => 0,
        Value::Str(s) => s.chars().count() as i64,
        Value::List(items) | Value::Stream(items) => items.len() as i64,
        Value::PathRef(p) => p.full_path.chars().count() as i64,
        Value::ObjectRef(o) => o.fields.len() as i64,
        Value::Object(map) => map.len() as i64,
        other => {
            return Err(QueryError::builtin(format!(
                "length: cannot take length of {}",
                type_name(other)
            )));
        }
    };
    Ok(Value::Int(n))
}

fn bi_str(args: &[Value]) -> Result<Value, QueryError> {
    match &args[0] {
        Value::Str(s) => Ok(Value::Str(s.clone())),
        Value::PathRef(p) => Ok(Value::Str(p.full_path.clone())),
        Value::Null => Ok(Value::Str("null".to_string())),
        Value::Bool(b) => Ok(Value::Str(if *b { "true" } else { "false" }.to_string())),
        Value::Int(i) => Ok(Value::Str(i.to_string())),
        Value::Float(f) => Ok(Value::Str(jsonfmt::py_float_repr(*f))),
        other => Err(QueryError::builtin(format!(
            "str: cannot stringify {} — pick a scalar field \
             (e.g. ``.name`` / ``.full-path``) or use ``--json`` for full objects",
            type_name(other)
        ))),
    }
}

fn bi_type(args: &[Value]) -> Result<Value, QueryError> {
    Ok(Value::Str(type_name(&args[0]).to_string()))
}

fn bi_has(args: &[Value]) -> Result<Value, QueryError> {
    let value = &args[0];
    let key = &args[1];
    let result = match value {
        Value::ObjectRef(o) => o.fields.contains_key(&as_str(key, "has", 2)?),
        Value::Object(map) => {
            let k = match key {
                Value::PathRef(p) => p.full_path.clone(),
                Value::Str(s) => s.clone(),
                _ => return Ok(Value::Bool(false)),
            };
            map.contains_key(&k)
        }
        Value::List(items) | Value::Stream(items) => {
            let idx = as_int(key, "has", 2)?;
            idx >= 0 && idx < items.len() as i64
        }
        Value::Null => false,
        other => {
            return Err(QueryError::builtin(format!(
                "has: cannot test membership on {}",
                type_name(other)
            )));
        }
    };
    Ok(Value::Bool(result))
}

fn bi_keys(args: &[Value]) -> Result<Value, QueryError> {
    match &args[0] {
        Value::ObjectRef(o) => {
            let mut keys: Vec<String> = o.fields.keys().cloned().collect();
            keys.sort();
            Ok(Value::List(keys.into_iter().map(Value::Str).collect()))
        }
        Value::Object(map) => {
            let mut keys: Vec<String> = map.keys().cloned().collect();
            keys.sort();
            Ok(Value::List(keys.into_iter().map(Value::Str).collect()))
        }
        Value::List(items) | Value::Stream(items) => Ok(Value::List(
            (0..items.len() as i64).map(Value::Int).collect(),
        )),
        other => Err(QueryError::builtin(format!(
            "keys: argument 1 must be an object or list, got {}",
            type_name(other)
        ))),
    }
}

fn bi_keys_unsorted(args: &[Value]) -> Result<Value, QueryError> {
    match &args[0] {
        Value::ObjectRef(o) => Ok(Value::List(
            o.fields.keys().cloned().map(Value::Str).collect(),
        )),
        Value::Object(map) => Ok(Value::List(map.keys().cloned().map(Value::Str).collect())),
        Value::List(items) | Value::Stream(items) => Ok(Value::List(
            (0..items.len() as i64).map(Value::Int).collect(),
        )),
        other => Err(QueryError::builtin(format!(
            "keys_unsorted: argument 1 must be an object or list, got {}",
            type_name(other)
        ))),
    }
}

fn bi_values(args: &[Value]) -> Result<Value, QueryError> {
    let entries = object_entries(&args[0], "values")?;
    Ok(Value::List(entries.into_iter().map(|(_, v)| v).collect()))
}

/// Sum a numeric sequence (`first` is the leading `Int`/`Float`, used only for
/// mixed-type error wording).  Integers accumulate with a *checked* add that
/// stands in for the `+` operator's clean overflow error — never a debug panic
/// or release wraparound (issue 193).  A float anywhere promotes the whole sum
/// to a float; from that point the integer accumulator is dead, so its checked
/// add is skipped rather than allowed to error on an overflow that can no longer
/// reach the result (matching `+`, which never re-narrows to int after
/// promoting — so `[i64::MAX, 1.0, 1] | add` yields a float, review follow-up).
fn bi_add_numeric(items: &[Value], first: &Value) -> Result<Value, QueryError> {
    let mut is_float = false;
    let mut isum: i64 = 0;
    let mut fsum: f64 = 0.0;
    for item in items {
        match item {
            Value::Bool(_) => {
                return Err(QueryError::builtin(
                    "add: cannot mix int with bool".to_string(),
                ));
            }
            Value::Int(i) => {
                if !is_float {
                    isum = isum.checked_add(*i).ok_or_else(|| {
                        QueryError::builtin("add: integer sum overflows i64".to_string())
                    })?;
                }
                fsum += *i as f64;
            }
            Value::Float(f) => {
                is_float = true;
                fsum += f;
            }
            other => {
                return Err(QueryError::builtin(format!(
                    "add: cannot mix {} with {}",
                    type_name(first),
                    type_name(other)
                )));
            }
        }
    }
    Ok(if is_float {
        Value::Float(fsum)
    } else {
        Value::Int(isum)
    })
}

fn bi_add(args: &[Value]) -> Result<Value, QueryError> {
    let items = as_sequence(&args[0], "add", 1)?;
    let Some(first) = items.first() else {
        return Ok(Value::Null);
    };
    match first {
        Value::Bool(_) | Value::Null => Err(QueryError::builtin(format!(
            "add: cannot sum {} values — coerce with ``str`` or ``tonumber`` first",
            type_name(first)
        ))),
        Value::Int(_) | Value::Float(_) => bi_add_numeric(&items, first),
        Value::Str(_) | Value::PathRef(_) => {
            let mut parts = String::new();
            for item in &items {
                match item {
                    Value::PathRef(p) => parts.push_str(&p.full_path),
                    Value::Str(s) => parts.push_str(s),
                    other => {
                        return Err(QueryError::builtin(format!(
                            "add: cannot mix string with {}",
                            type_name(other)
                        )));
                    }
                }
            }
            Ok(Value::Str(parts))
        }
        Value::List(_) | Value::Stream(_) => {
            let mut out = Vec::new();
            for item in &items {
                match item {
                    Value::List(s) | Value::Stream(s) => out.extend(s.clone()),
                    other => {
                        return Err(QueryError::builtin(format!(
                            "add: cannot mix list with {}",
                            type_name(other)
                        )));
                    }
                }
            }
            Ok(Value::List(out))
        }
        Value::Object(_) | Value::ObjectRef(_) => {
            let mut merged = IndexMap::new();
            for item in &items {
                match item {
                    Value::Object(map) => {
                        for (k, v) in map {
                            merged.insert(k.clone(), v.clone());
                        }
                    }
                    other => {
                        return Err(QueryError::builtin(format!(
                            "add: cannot mix dict with {}",
                            type_name(other)
                        )));
                    }
                }
            }
            Ok(Value::Object(merged))
        }
        other => Err(QueryError::builtin(format!(
            "add: cannot sum values of type {}",
            type_name(other)
        ))),
    }
}

fn bi_sort(args: &[Value]) -> Result<Value, QueryError> {
    let mut items = as_sequence(&args[0], "sort", 1)?;
    items.sort_by(value::sort_cmp);
    Ok(Value::List(items))
}

fn bi_unique(args: &[Value]) -> Result<Value, QueryError> {
    let mut items = as_sequence(&args[0], "unique", 1)?;
    items.sort_by(value::sort_cmp);
    let mut out: Vec<Value> = Vec::new();
    for item in items {
        if out
            .last()
            .is_none_or(|prev| value::sort_cmp(prev, &item) != std::cmp::Ordering::Equal)
        {
            out.push(item);
        }
    }
    Ok(Value::List(out))
}

fn bi_dupes(args: &[Value]) -> Result<Value, QueryError> {
    let mut items = as_sequence(&args[0], "dupes", 1)?;
    items.sort_by(value::sort_cmp);
    let mut out = Vec::new();
    let mut i = 0;
    let n = items.len();
    while i < n {
        let mut j = i + 1;
        while j < n && value::sort_cmp(&items[j], &items[i]) == std::cmp::Ordering::Equal {
            j += 1;
        }
        if j - i >= 2 {
            out.push(items[i].clone());
        }
        i = j;
    }
    Ok(Value::List(out))
}

fn bi_reverse(args: &[Value]) -> Result<Value, QueryError> {
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Str(s) => Ok(Value::Str(s.chars().rev().collect())),
        Value::PathRef(p) => Ok(Value::Str(p.full_path.chars().rev().collect())),
        other => {
            let mut items = as_sequence(other, "reverse", 1)?;
            items.reverse();
            Ok(Value::List(items))
        }
    }
}

fn bi_first(args: &[Value]) -> Result<Value, QueryError> {
    let items = as_sequence(&args[0], "first", 1)?;
    Ok(items.into_iter().next().unwrap_or(Value::Null))
}

fn bi_last(args: &[Value]) -> Result<Value, QueryError> {
    let items = as_sequence(&args[0], "last", 1)?;
    Ok(items.into_iter().next_back().unwrap_or(Value::Null))
}

fn bi_count(args: &[Value]) -> Result<Value, QueryError> {
    let items = as_sequence(&args[0], "count", 1)?;
    Ok(Value::Int(items.len() as i64))
}

fn bi_min(args: &[Value]) -> Result<Value, QueryError> {
    let items = as_sequence(&args[0], "min", 1)?;
    Ok(extreme(items, true))
}

fn bi_max(args: &[Value]) -> Result<Value, QueryError> {
    let items = as_sequence(&args[0], "max", 1)?;
    Ok(extreme(items, false))
}

/// `min`/`max` with `key=_sort_key`: first occurrence wins ties.
fn extreme(items: Vec<Value>, want_min: bool) -> Value {
    let mut it = items.into_iter();
    let Some(mut best) = it.next() else {
        return Value::Null;
    };
    for item in it {
        let ord = value::sort_cmp(&item, &best);
        let replace = if want_min { ord.is_lt() } else { ord.is_gt() };
        if replace {
            best = item;
        }
    }
    best
}

fn bi_range(args: &[Value]) -> Result<Value, QueryError> {
    let (start, end, step) = match args.len() {
        1 => (0, as_int(&args[0], "range", 1)?, 1),
        2 => (
            as_int(&args[0], "range", 1)?,
            as_int(&args[1], "range", 2)?,
            1,
        ),
        _ => (
            as_int(&args[0], "range", 1)?,
            as_int(&args[1], "range", 2)?,
            as_int(&args[2], "range", 3)?,
        ),
    };
    if step == 0 {
        return Err(QueryError::builtin("range: step must be non-zero"));
    }
    let mut out = Vec::new();
    let mut cur = start;
    // `checked_add` so a range whose stride steps past the i64 boundary ends
    // cleanly instead of overflow-panicking in debug / wrapping in release
    // (issue 193) — the next value would be unrepresentable, so the range is
    // exhausted.
    if step > 0 {
        while cur < end {
            out.push(Value::Int(cur));
            match cur.checked_add(step) {
                Some(next) => cur = next,
                None => break,
            }
        }
    } else {
        while cur > end {
            out.push(Value::Int(cur));
            match cur.checked_add(step) {
                Some(next) => cur = next,
                None => break,
            }
        }
    }
    Ok(Value::Stream(out))
}

fn bi_empty(_args: &[Value]) -> Result<Value, QueryError> {
    Ok(Value::Stream(Vec::new()))
}

fn bi_error(args: &[Value]) -> Result<Value, QueryError> {
    if args.is_empty() {
        return Err(QueryError::builtin("error: query halted by error()"));
    }
    let text = match &args[0] {
        Value::Str(s) => s.clone(),
        Value::PathRef(p) => p.full_path.clone(),
        other => py_repr(other),
    };
    Err(QueryError::builtin(text))
}

fn py_repr(v: &Value) -> String {
    match v {
        Value::Null => "None".to_string(),
        Value::Bool(b) => if *b { "True" } else { "False" }.to_string(),
        Value::Int(i) => i.to_string(),
        Value::Float(f) => jsonfmt::py_float_repr(*f),
        Value::Str(s) => format!("{s:?}"),
        other => other.describe(),
    }
}

pub(crate) fn bi_to_entries(args: &[Value]) -> Result<Value, QueryError> {
    let entries = object_entries(&args[0], "to_entries")?;
    let rows: Vec<Value> = entries
        .into_iter()
        .map(|(k, v)| {
            let mut m = IndexMap::new();
            m.insert("key".to_string(), Value::Str(k));
            m.insert("value".to_string(), v);
            Value::Object(m)
        })
        .collect();
    Ok(Value::List(rows))
}

pub(crate) fn bi_from_entries(args: &[Value]) -> Result<Value, QueryError> {
    let items = as_sequence(&args[0], "from_entries", 1)?;
    let mut out: IndexMap<String, Value> = IndexMap::new();
    for (i, entry) in items.iter().enumerate() {
        let (key, val) = entry_key_value(entry, i)?;
        let key_str = match key {
            Value::PathRef(p) => p.full_path.clone(),
            Value::Null => String::new(),
            Value::Str(s) => s,
            Value::Int(n) => n.to_string(),
            other => {
                return Err(QueryError::builtin(format!(
                    "from_entries: entry {i} key must be a string or integer, got {}",
                    type_name(&other)
                )));
            }
        };
        out.insert(key_str, val);
    }
    Ok(Value::Object(out))
}

fn entry_key_value(entry: &Value, i: usize) -> Result<(Value, Value), QueryError> {
    let lookup = |map: &IndexMap<String, Value>, keys: &[&str]| -> Option<Value> {
        keys.iter().find_map(|k| map.get(*k).cloned())
    };
    match entry {
        Value::Object(map) => {
            let key = lookup(map, &["key", "k", "name"]).ok_or_else(|| {
                QueryError::builtin(format!(
                    "from_entries: entry {i} missing key (expected ``key``, ``k``, or ``name``)"
                ))
            })?;
            let val = lookup(map, &["value", "v"]).unwrap_or(Value::Null);
            Ok((key, val))
        }
        Value::List(items) | Value::Stream(items) if items.len() == 2 => {
            Ok((items[0].clone(), items[1].clone()))
        }
        Value::ObjectRef(o) => {
            let key = lookup(&o.fields, &["key", "k", "name"]).ok_or_else(|| {
                QueryError::builtin(format!(
                    "from_entries: entry {i} missing key (expected ``key``, ``k``, or ``name``)"
                ))
            })?;
            let val = lookup(&o.fields, &["value", "v"]).unwrap_or(Value::Null);
            Ok((key, val))
        }
        other => Err(QueryError::builtin(format!(
            "from_entries: entry {i} must be an object or 2-element list, got {}",
            type_name(other)
        ))),
    }
}

fn bi_tonumber(args: &[Value]) -> Result<Value, QueryError> {
    match &args[0] {
        Value::Bool(_) => Err(QueryError::builtin(
            "tonumber: cannot convert bool to number",
        )),
        Value::Int(i) => Ok(Value::Int(*i)),
        Value::Float(f) => Ok(Value::Float(*f)),
        Value::Null => Err(QueryError::builtin(
            "tonumber: cannot convert null to number",
        )),
        Value::PathRef(_) | Value::Str(_) => {
            let text = match &args[0] {
                Value::PathRef(p) => p.full_path.clone(),
                Value::Str(s) => s.clone(),
                _ => unreachable!(),
            };
            let stripped = text.trim();
            if let Ok(i) = stripped.parse::<i64>() {
                return Ok(Value::Int(i));
            }
            if let Ok(f) = stripped.parse::<f64>() {
                return Ok(Value::Float(f));
            }
            Err(QueryError::builtin(format!(
                "tonumber: cannot parse {text:?} as a number"
            )))
        }
        other => Err(QueryError::builtin(format!(
            "tonumber: cannot convert {} to number",
            type_name(other)
        ))),
    }
}

fn bi_tostring(args: &[Value]) -> Result<Value, QueryError> {
    match &args[0] {
        Value::Str(s) => Ok(Value::Str(s.clone())),
        Value::PathRef(p) => Ok(Value::Str(p.full_path.clone())),
        Value::Null => Ok(Value::Str("null".to_string())),
        Value::Bool(b) => Ok(Value::Str(if *b { "true" } else { "false" }.to_string())),
        Value::Int(i) => Ok(Value::Str(i.to_string())),
        Value::Float(f) => Ok(Value::Str(jsonfmt::py_float_repr(*f))),
        composite
        @ (Value::List(_) | Value::Stream(_) | Value::Object(_) | Value::ObjectRef(_)) => Ok(
            Value::Str(jsonfmt::to_compact_sorted(&to_jsonable(composite, 0))),
        ),
        other => Err(QueryError::builtin(format!(
            "tostring: cannot stringify {}",
            type_name(other)
        ))),
    }
}

// `f.floor()` is already integral; the `f64 -> i64` is the intended narrowing
// (values beyond the `i64` range saturate, matching jq's `floor`).
#[allow(clippy::cast_possible_truncation)]
fn bi_floor(args: &[Value]) -> Result<Value, QueryError> {
    match as_number(&args[0], "floor", 1)? {
        Value::Int(i) => Ok(Value::Int(i)),
        Value::Float(f) => Ok(Value::Int(f.floor() as i64)),
        _ => unreachable!(),
    }
}

// `f.ceil()` is already integral; the `f64 -> i64` is the intended narrowing
// (values beyond the `i64` range saturate, matching jq's `ceil`).
#[allow(clippy::cast_possible_truncation)]
fn bi_ceil(args: &[Value]) -> Result<Value, QueryError> {
    match as_number(&args[0], "ceil", 1)? {
        Value::Int(i) => Ok(Value::Int(i)),
        Value::Float(f) => Ok(Value::Int(f.ceil() as i64)),
        _ => unreachable!(),
    }
}

fn bi_abs(args: &[Value]) -> Result<Value, QueryError> {
    match as_number(&args[0], "abs", 1)? {
        Value::Int(i) => Ok(Value::Int(i.abs())),
        Value::Float(f) => Ok(Value::Float(f.abs())),
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod catalogue_tests {
    use super::{all_specs, format_catalogue, lookup};

    #[test]
    fn all_specs_sorted_and_complete() {
        let specs = all_specs();
        assert!(specs.len() > 100, "expected a large builtin registry");
        // Sorted by (category, name).
        for w in specs.windows(2) {
            let a = (w[0].category, w[0].name);
            let b = (w[1].category, w[1].name);
            assert!(a <= b, "catalogue not sorted: {a:?} > {b:?}");
        }
        // Every listed name resolves back through `lookup`.
        for s in &specs {
            assert!(lookup(s.name).is_some(), "lookup failed for {}", s.name);
        }
    }

    #[test]
    fn catalogue_lists_categories_and_arity() {
        let cat = format_catalogue(None);
        assert!(cat.starts_with("F5 QUERY DSL — BUILTIN FUNCTIONS"));
        assert!(cat.contains("[math]"));
        assert!(cat.ends_with('\n'));
    }

    #[test]
    fn single_builtin_lookup() {
        let one = format_catalogue(Some("atan2"));
        assert!(one.contains("atan2"));
        assert!(one.contains("category: math"));
        assert!(one.contains("arity:    2"));
    }

    #[test]
    fn unknown_builtin_reports_cleanly() {
        let miss = format_catalogue(Some("definitely-not-a-builtin"));
        assert!(miss.contains("no builtin named 'definitely-not-a-builtin'"));
    }
}

#[cfg(test)]
mod add_tests {
    use super::bi_add;
    use crate::value::Value;

    fn add(items: Vec<Value>) -> Result<Value, super::QueryError> {
        bi_add(&[Value::List(items)])
    }

    fn as_int(v: Result<Value, super::QueryError>) -> i64 {
        match v.expect("expected Ok") {
            Value::Int(i) => i,
            other => panic!("expected an integer, got {other:?}"),
        }
    }

    fn as_float(v: Result<Value, super::QueryError>) -> f64 {
        match v.expect("expected Ok") {
            Value::Float(f) => f,
            other => panic!("expected a float, got {other:?}"),
        }
    }

    // TN: a clean all-integer sum stays an exact integer.
    #[test]
    fn all_int_sum_is_exact_int() {
        assert_eq!(
            as_int(add(vec![Value::Int(1), Value::Int(2), Value::Int(3)])),
            6
        );
    }

    // TP: a genuine all-integer overflow is a clean error, not a panic/wrap —
    // this stands in for the `+` operator's overflow behaviour.
    #[test]
    fn all_int_overflow_errors() {
        let err = add(vec![Value::Int(i64::MAX), Value::Int(1)]).unwrap_err();
        assert!(err.to_string().contains("integer sum overflows"), "{err}");
    }

    // FP guard (the review finding): once a float has been seen the result is a
    // float, so a later integer that would overflow the *dead* integer
    // accumulator must NOT error — the `+` fold returns a float here.
    #[test]
    fn float_promotion_suppresses_dead_int_overflow() {
        let got = as_float(add(vec![
            Value::Int(i64::MAX),
            Value::Float(1.0),
            Value::Int(1),
        ]));
        assert!((got - (i64::MAX as f64 + 2.0)).abs() < 1.0, "{got}");
    }

    // TP still fires before promotion: integers that overflow *before* any float
    // appears error just as the `+` fold would (the overflow is real there).
    #[test]
    fn int_overflow_before_float_still_errors() {
        let err = add(vec![
            Value::Int(i64::MAX),
            Value::Int(i64::MAX),
            Value::Float(1.0),
        ])
        .unwrap_err();
        assert!(err.to_string().contains("integer sum overflows"), "{err}");
    }

    // FN guard: a float anywhere promotes the whole sum to float (never silently
    // dropped back to int).
    #[test]
    fn any_float_makes_the_sum_float() {
        assert!((as_float(add(vec![Value::Int(2), Value::Float(0.5)])) - 2.5).abs() < f64::EPSILON);
    }
}

/// Regression coverage for issue #996: `to_jsonable`, `walk_paths`
/// (`all_paths`), `set_at_path`, `delete_at_path`, and `flatten_go`
/// (`flatten_value`) each recurse once per nested `Value`/path level, with
/// no depth cap before this fix. All are reachable with a fully
/// generator-controlled nesting depth: `tojson`/`debug`/`stderr` on deeply
/// nested `fromjson` input, `paths`/`leaf_paths` on the same, `setpath`
/// with a `range()`-built path far longer than any real document nests,
/// `del`/`delpaths` likewise, and `flatten` with a value nested deeper than
/// its own numeric `depth` argument bounds.
#[cfg(test)]
mod recursion_tests {
    use super::*;

    /// A list nested `depth` levels deep, wrapping a single `Int(0)` leaf.
    fn deep_nested_list(depth: usize) -> Value {
        let mut v = Value::Int(0);
        for _ in 0..depth {
            v = Value::List(vec![v]);
        }
        v
    }

    /// An object nested `depth` levels deep under key `"a"`, wrapping a
    /// single `Int(0)` leaf — `{"a": {"a": ... {"a": 0} ...}}`.
    fn deep_nested_object(depth: usize) -> Value {
        let mut v = Value::Int(0);
        for _ in 0..depth {
            let mut m = IndexMap::new();
            m.insert("a".to_string(), v);
            v = Value::Object(m);
        }
        v
    }

    /// A path of `depth` `Str("a")` keys, matching [`deep_nested_object`]'s
    /// shape one key per level.
    fn deep_path(depth: usize) -> Vec<Value> {
        (0..depth).map(|_| Value::Str("a".to_string())).collect()
    }

    const DEPTH: usize = 5000;

    /// Run *f* on a worker thread with a 256 MiB stack (mirroring
    /// `tests/hardening.rs`'s `with_big_stack`). Building — and, at scope
    /// end, dropping — a several-thousand-level-deep `Value` costs one
    /// native stack frame per level, since `Value`'s derived `Clone`/`Drop`
    /// have no depth cap of their own (unlike the guarded functions under
    /// test here), so the fixture itself needs more room than the harness's
    /// default ~2 MiB per-test thread provides — independently of whether
    /// the guard is correct.
    fn with_big_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
        std::thread::Builder::new()
            .stack_size(256 * 1024 * 1024)
            .spawn(f)
            .expect("spawn worker thread")
            .join()
            .expect("worker thread did not panic / overflow")
    }

    #[test]
    fn deeply_nested_value_to_jsonable_does_not_crash() {
        with_big_stack(|| {
            let v = deep_nested_list(DEPTH);
            let _ = to_jsonable(&v, 0);
        });
    }

    #[test]
    fn moderately_nested_value_to_jsonable_is_unchanged() {
        let v = deep_nested_list(3);
        let out = to_jsonable(&v, 0);
        assert_eq!(jsonfmt::to_compact(&out), "[[[0]]]");
    }

    #[test]
    fn deeply_nested_value_all_paths_does_not_crash() {
        with_big_stack(|| {
            let v = deep_nested_object(DEPTH);
            let paths = all_paths(&v, false);
            // Descent stops at the cap, so far fewer than DEPTH paths come back.
            assert!(paths.len() < DEPTH);
        });
    }

    #[test]
    fn moderately_nested_value_all_paths_is_unchanged() {
        let v = deep_nested_object(3);
        let paths = all_paths(&v, true); // leaf paths only
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].len(), 3);
    }

    #[test]
    fn deeply_nested_setpath_path_returns_error_not_crash() {
        let path = deep_path(DEPTH);
        let result = set_at_path(Value::Null, &path, Value::Int(1), 0);
        assert!(result.is_err(), "expected a clean error past the depth cap");
    }

    #[test]
    fn moderately_nested_setpath_path_is_unchanged() {
        let path = deep_path(3);
        let out = set_at_path(Value::Null, &path, Value::Int(1), 0).expect("well under the cap");
        assert_eq!(jsonfmt::to_compact(&out), r#"{"a":{"a":{"a":1}}}"#);
    }

    #[test]
    fn deeply_nested_delete_at_path_does_not_crash() {
        with_big_stack(|| {
            let value = deep_nested_object(DEPTH);
            let path = deep_path(DEPTH);
            let _ = delete_at_path(value, &path, 0);
        });
    }

    #[test]
    fn moderately_nested_delete_at_path_is_unchanged() {
        let value = deep_nested_object(3);
        let path = deep_path(3);
        let out = delete_at_path(value, &path, 0);
        // The innermost "a": 0 is removed, leaving empty objects nested up.
        assert_eq!(jsonfmt::to_compact(&out), r#"{"a":{"a":{}}}"#);
    }

    #[test]
    fn deeply_nested_flatten_does_not_crash() {
        with_big_stack(|| {
            let v = deep_nested_list(DEPTH);
            // `remaining` (the caller-facing `flatten` depth argument) is
            // huge — the actual recursion is bounded by the value's own
            // nesting, which is what the new native-depth guard protects.
            let _ = flatten_go(std::slice::from_ref(&v), i64::MAX, 0);
        });
    }

    #[test]
    fn moderately_nested_flatten_is_unchanged() {
        let v = Value::List(vec![
            Value::List(vec![Value::Int(1), Value::Int(2)]),
            Value::Int(3),
        ]);
        let out = flatten_value(&v, 1).expect("flatten(1)");
        let rendered: Vec<String> = out.iter().map(jsonfmt::to_compact).collect();
        assert_eq!(rendered, vec!["1", "2", "3"]);
    }
}
