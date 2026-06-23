//! Special-form builtins.
//!
//! Special forms receive their arguments unevaluated (as AST) plus the
//! evaluator context, so they can re-bind `.` per input value (`select`,
//! `map`, `sort_by`, …) or capture path projections (`pick`). The evaluator
//! dispatches here once arity is checked.

use std::cmp::Ordering;

use indexmap::IndexMap;

use crate::ast::{Expr, PathStep};
use crate::builtins::{
    self, BuiltinSpec, all_paths, bi_from_entries, bi_to_entries, coerce_path_list,
    combinations_impl, delete_at_path, flatten_value, set_at_path, special, to_jsonable,
    value_at_path,
};
use crate::errors::QueryError;
use crate::eval::{EvalContext, eval, flatten, flatten_one, stream_items};
use crate::value::{self, Value};

pub(crate) fn registrations() -> Vec<(&'static str, BuiltinSpec)> {
    vec![
        special("not", "stream", 0, Some(0)),
        special("select", "stream", 1, Some(1)),
        special("map", "stream", 1, Some(1)),
        special("sort_by", "stream", 1, Some(1)),
        special("unique_by", "stream", 1, Some(1)),
        special("min_by", "stream", 1, Some(1)),
        special("max_by", "stream", 1, Some(1)),
        special("group_by", "stream", 1, Some(1)),
        special("with_entries", "value", 1, Some(1)),
        special("flatten", "stream", 0, Some(1)),
        special("paths", "value", 0, Some(1)),
        special("leaf_paths", "value", 0, Some(0)),
        special("getpath", "value", 1, Some(1)),
        special("setpath", "value", 2, Some(2)),
        special("del", "value", 1, Some(1)),
        special("delpaths", "value", 1, Some(1)),
        special("walk", "value", 1, Some(1)),
        special("recurse", "value", 0, Some(2)),
        special("recurse_down", "value", 0, Some(0)),
        special("until", "value", 2, Some(2)),
        special("repeat", "value", 1, Some(1)),
        special("combinations", "stream", 0, Some(1)),
        special("map_values", "stream", 1, Some(1)),
        special("pick", "value", 1, None),
        special("min_max", "stream", 0, Some(1)),
        special("max_min", "stream", 0, Some(1)),
        special("debug", "stream", 0, Some(1)),
        special("stderr", "stream", 0, Some(0)),
        special("IN", "stream", 1, None),
        special("INDEX", "stream", 1, Some(2)),
    ]
}

const ITER_CAP: usize = 100_000;

#[allow(clippy::too_many_lines)]
pub(crate) fn eval_special_form(
    name: &str,
    args: &[Expr],
    current: &Value,
    ctx: &mut EvalContext,
) -> Result<Value, QueryError> {
    match name {
        "not" => Ok(Value::Bool(!value::truthy(current))),
        "select" => {
            let result = eval(&args[0], current, ctx)?;
            if let Value::Stream(items) = result {
                Ok(if items.iter().any(value::truthy) {
                    current.clone()
                } else {
                    Value::Drop
                })
            } else {
                Ok(if value::truthy(&result) {
                    current.clone()
                } else {
                    Value::Drop
                })
            }
        }
        "map" => {
            let mut out = Vec::new();
            for item in stream_items(current.clone()) {
                let v = eval(&args[0], &item, ctx)?;
                out.extend(flatten(v));
            }
            Ok(Value::List(out))
        }
        "sort_by" => {
            let items = stream_items(current.clone());
            let mut keyed = key_items(&args[0], items, ctx)?;
            keyed.sort_by(|a, b| value::sort_cmp(&a.1, &b.1));
            Ok(Value::List(
                keyed.into_iter().map(|(item, _)| item).collect(),
            ))
        }
        "unique_by" => {
            let items = stream_items(current.clone());
            let mut keyed = key_items(&args[0], items, ctx)?;
            keyed.sort_by(|a, b| value::sort_cmp(&a.1, &b.1));
            let mut out: Vec<Value> = Vec::new();
            let mut last: Option<Value> = None;
            for (item, key) in keyed {
                if last
                    .as_ref()
                    .is_none_or(|l| value::sort_cmp(l, &key) != Ordering::Equal)
                {
                    out.push(item);
                    last = Some(key);
                }
            }
            Ok(Value::List(out))
        }
        "min_by" => by_extreme(&args[0], current, ctx, true),
        "max_by" => by_extreme(&args[0], current, ctx, false),
        "group_by" => {
            let items = stream_items(current.clone());
            let keyed = key_items(&args[0], items, ctx)?;
            let mut groups: Vec<(Value, Vec<Value>)> = Vec::new();
            for (item, key) in keyed {
                if let Some(g) = groups
                    .iter_mut()
                    .find(|(k, _)| value::sort_cmp(k, &key) == Ordering::Equal)
                {
                    g.1.push(item);
                } else {
                    groups.push((key, vec![item]));
                }
            }
            groups.sort_by(|a, b| value::sort_cmp(&a.0, &b.0));
            Ok(Value::List(
                groups.into_iter().map(|(_, v)| Value::List(v)).collect(),
            ))
        }
        "with_entries" => {
            let entries = bi_to_entries(std::slice::from_ref(current))?;
            let mut mapped = Vec::new();
            for entry in stream_items(entries) {
                let result = eval(&args[0], &entry, ctx)?;
                mapped.extend(flatten(result));
            }
            bi_from_entries(&[Value::List(mapped)])
        }
        "flatten" => {
            let depth = if args.is_empty() {
                1
            } else {
                match eval(&args[0], current, ctx)? {
                    Value::Int(i) => i,
                    other => {
                        return Err(QueryError::builtin(format!(
                            "flatten: argument 1 must be an integer, got {}",
                            other.type_name()
                        )));
                    }
                }
            };
            Ok(Value::List(flatten_value(current, depth)?))
        }
        "paths" => eval_paths(args, current, ctx, false),
        "leaf_paths" => eval_paths(args, current, ctx, true),
        "getpath" => {
            let path_value = eval(&args[0], current, ctx)?;
            let path = coerce_path_list(&path_value, "getpath", 1)?;
            Ok(value_at_path(current, &path))
        }
        "setpath" => {
            let path_value = eval(&args[0], current, ctx)?;
            let path = coerce_path_list(&path_value, "setpath", 1)?;
            let new_value = match eval(&args[1], current, ctx)? {
                Value::Stream(items) => Value::List(items),
                other => other,
            };
            set_at_path(current.clone(), &path, new_value)
        }
        "del" => {
            let path_value = eval(&args[0], current, ctx)?;
            let path = coerce_path_list(&path_value, "del", 1)?;
            Ok(delete_at_path(current.clone(), &path))
        }
        "delpaths" => {
            let paths_value = eval(&args[0], current, ctx)?;
            let raw = builtins::as_sequence(&paths_value, "delpaths", 1)?;
            let mut paths: Vec<Vec<Value>> = raw
                .iter()
                .map(|p| coerce_path_list(p, "delpaths", 1))
                .collect::<Result<_, _>>()?;
            paths.sort_by_key(|p| std::cmp::Reverse(p.len()));
            let mut out = current.clone();
            for p in &paths {
                out = delete_at_path(out, p);
            }
            Ok(out)
        }
        "walk" => walk(&args[0], current.clone(), ctx),
        "recurse" => eval_recurse(args, current, ctx),
        "recurse_down" => eval_recurse(&[], current, ctx),
        "until" => {
            let mut value = current.clone();
            for _ in 0..ITER_CAP {
                let cond = flatten_one(eval(&args[0], &value, ctx)?)?;
                if value::truthy(&cond) {
                    return Ok(value);
                }
                value = flatten_one(eval(&args[1], &value, ctx)?)?;
            }
            Err(QueryError::eval(
                "until: hit 100,000-iteration cap without satisfying the condition",
            ))
        }
        "repeat" => {
            let mut out = Vec::new();
            let mut value = current.clone();
            for _ in 0..ITER_CAP {
                value = flatten_one(eval(&args[0], &value, ctx)?)?;
                out.push(value.clone());
            }
            Ok(Value::Stream(out))
        }
        "combinations" => {
            let n = if args.is_empty() {
                None
            } else {
                match eval(&args[0], current, ctx)? {
                    Value::Int(i) => Some(i),
                    other => {
                        return Err(QueryError::builtin(format!(
                            "combinations: argument 1 must be an integer, got {}",
                            other.type_name()
                        )));
                    }
                }
            };
            combinations_impl(current, n)
        }
        "map_values" => map_values(&args[0], current, ctx),
        "pick" => eval_pick(args, current, ctx),
        "min_max" | "max_min" => {
            let items = stream_items(current.clone());
            if items.is_empty() {
                return Ok(Value::List(vec![Value::Null, Value::Null]));
            }
            let (mn, mx) = if args.is_empty() {
                (extreme_item(&items, true), extreme_item(&items, false))
            } else {
                let keyed = key_items(&args[0], items, ctx)?;
                (keyed_extreme(&keyed, true), keyed_extreme(&keyed, false))
            };
            Ok(if name == "min_max" {
                Value::List(vec![mn, mx])
            } else {
                Value::List(vec![mx, mn])
            })
        }
        "debug" => {
            let jsonable = to_jsonable(current);
            let payload = if args.is_empty() {
                Value::List(vec![Value::Str("DEBUG:".to_string()), jsonable])
            } else {
                let label = match eval(&args[0], current, ctx)? {
                    Value::Stream(items) => items
                        .into_iter()
                        .next()
                        .unwrap_or(Value::Str(String::new())),
                    other => other,
                };
                Value::List(vec![
                    Value::Str("DEBUG:".to_string()),
                    to_jsonable(&label),
                    jsonable,
                ])
            };
            eprintln!("{}", crate::jsonfmt::to_compact(&payload));
            Ok(current.clone())
        }
        "stderr" => {
            eprintln!("{}", crate::jsonfmt::to_compact(&to_jsonable(current)));
            Ok(current.clone())
        }
        "IN" => {
            let mut candidates = Vec::new();
            for arg in args {
                let v = eval(arg, current, ctx)?;
                candidates.extend(flatten(v));
            }
            Ok(Value::Bool(
                candidates.iter().any(|c| value::py_eq(current, c)),
            ))
        }
        "INDEX" => {
            let (source_items, key_expr) = if args.len() == 1 {
                (stream_items(current.clone()), &args[0])
            } else {
                let source = eval(&args[0], current, ctx)?;
                (flatten(source), &args[1])
            };
            let mut index: IndexMap<String, Value> = IndexMap::new();
            for item in source_items {
                let key = flatten_one(eval(key_expr, &item, ctx)?)?;
                let key_str = match key {
                    Value::PathRef(p) => p.full_path.clone(),
                    Value::Null => String::new(),
                    other => py_str_key(&other),
                };
                index.insert(key_str, item);
            }
            Ok(Value::Object(index))
        }
        other => Err(QueryError::eval(format!(
            "unsupported special form: {other}"
        ))),
    }
}

fn key_items(
    body: &Expr,
    items: Vec<Value>,
    ctx: &mut EvalContext,
) -> Result<Vec<(Value, Value)>, QueryError> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let key = key_of_body(body, &item, ctx)?;
        out.push((item, key));
    }
    Ok(out)
}

fn key_of_body(body: &Expr, item: &Value, ctx: &mut EvalContext) -> Result<Value, QueryError> {
    let result = eval(body, item, ctx)?;
    Ok(match result {
        Value::Stream(items) => Value::List(items),
        other => other,
    })
}

fn by_extreme(
    body: &Expr,
    current: &Value,
    ctx: &mut EvalContext,
    want_min: bool,
) -> Result<Value, QueryError> {
    let items = stream_items(current.clone());
    if items.is_empty() {
        return Ok(Value::Null);
    }
    let keyed = key_items(body, items, ctx)?;
    Ok(keyed_extreme(&keyed, want_min))
}

fn keyed_extreme(keyed: &[(Value, Value)], want_min: bool) -> Value {
    let mut best = &keyed[0];
    for pair in &keyed[1..] {
        let ord = value::sort_cmp(&pair.1, &best.1);
        let replace = if want_min { ord.is_lt() } else { ord.is_gt() };
        if replace {
            best = pair;
        }
    }
    best.0.clone()
}

fn extreme_item(items: &[Value], want_min: bool) -> Value {
    let mut best = &items[0];
    for item in &items[1..] {
        let ord = value::sort_cmp(item, best);
        let replace = if want_min { ord.is_lt() } else { ord.is_gt() };
        if replace {
            best = item;
        }
    }
    best.clone()
}

fn walk(body: &Expr, value: Value, ctx: &mut EvalContext) -> Result<Value, QueryError> {
    let rebuilt = match value {
        Value::ObjectRef(o) => {
            let mut m = IndexMap::new();
            for (k, v) in &o.fields {
                m.insert(k.clone(), walk(body, v.clone(), ctx)?);
            }
            Value::Object(m)
        }
        Value::Object(map) => {
            let mut m = IndexMap::new();
            for (k, v) in map {
                m.insert(k, walk(body, v, ctx)?);
            }
            Value::Object(m)
        }
        Value::List(items) => {
            let mut out = Vec::with_capacity(items.len());
            for v in items {
                out.push(walk(body, v, ctx)?);
            }
            Value::List(out)
        }
        Value::Stream(items) => {
            let mut out = Vec::with_capacity(items.len());
            for v in items {
                out.push(walk(body, v, ctx)?);
            }
            Value::List(out)
        }
        scalar => scalar,
    };
    Ok(flatten_one_value(eval(body, &rebuilt, ctx)?))
}

/// Port of `evaluator._flatten_one` used inside `walk` — there it collapses
/// a single-item stream to its item but does not error on multi (jq's walk
/// keeps the last). `_flatten_one` in the Python `walk` is actually
/// `_flatten_one` which raises on empty/multi; mirror that loosely by taking
/// the first flattened value (walk bodies yield one value).
fn flatten_one_value(value: Value) -> Value {
    let mut flat = flatten(value);
    if flat.is_empty() {
        Value::Null
    } else {
        flat.swap_remove(0)
    }
}

fn map_values(body: &Expr, current: &Value, ctx: &mut EvalContext) -> Result<Value, QueryError> {
    match current {
        Value::ObjectRef(o) => map_values_dict(&o.fields.clone(), body, ctx),
        Value::Object(map) => map_values_dict(&map.clone(), body, ctx),
        Value::List(_) | Value::Stream(_) => {
            let mut out = Vec::new();
            for item in stream_items(current.clone()) {
                out.extend(flatten(eval(body, &item, ctx)?));
            }
            Ok(Value::List(out))
        }
        other => Err(QueryError::eval(format!(
            "map_values: input must be an object or array, got {}",
            other.type_name()
        ))),
    }
}

fn map_values_dict(
    d: &IndexMap<String, Value>,
    body: &Expr,
    ctx: &mut EvalContext,
) -> Result<Value, QueryError> {
    let mut out = IndexMap::new();
    for (k, v) in d {
        let result = eval(body, v, ctx)?;
        match result {
            Value::Drop => {}
            Value::Stream(items) => {
                let filtered: Vec<Value> = items
                    .into_iter()
                    .filter(|x| !matches!(x, Value::Drop))
                    .collect();
                if filtered.is_empty() {
                    return Err(QueryError::builtin(format!(
                        "map_values: body yielded no value for key {k:?} \
                         (use ``with_entries(select(...))`` to drop entries)"
                    )));
                }
                if filtered.len() > 1 {
                    return Err(QueryError::builtin(format!(
                        "map_values: body yielded {} values for key {k:?}; \
                         ``map_values`` keeps the object's shape one-to-one",
                        filtered.len()
                    )));
                }
                out.insert(k.clone(), filtered.into_iter().next().unwrap());
            }
            other => {
                out.insert(k.clone(), other);
            }
        }
    }
    Ok(Value::Object(out))
}

fn eval_paths(
    args: &[Expr],
    current: &Value,
    ctx: &mut EvalContext,
    only_leaves: bool,
) -> Result<Value, QueryError> {
    let every = all_paths(current, only_leaves);
    if only_leaves || args.is_empty() {
        return Ok(Value::Stream(every.into_iter().map(Value::List).collect()));
    }
    let mut out = Vec::new();
    for p in every {
        let value_at = value_at_path(current, &p);
        let result = eval(&args[0], &value_at, ctx)?;
        let keep = match result {
            Value::Stream(items) => items.iter().any(value::truthy),
            other => value::truthy(&other),
        };
        if keep {
            out.push(Value::List(p));
        }
    }
    Ok(Value::Stream(out))
}

fn eval_recurse(
    args: &[Expr],
    current: &Value,
    ctx: &mut EvalContext,
) -> Result<Value, QueryError> {
    if args.is_empty() {
        // DFS pre-order via an explicit stack.
        let mut out = Vec::new();
        let mut stack = vec![current.clone()];
        while let Some(value) = stack.pop() {
            out.push(value.clone());
            if out.len() >= ITER_CAP {
                break;
            }
            let children = builtins::value_children(&value);
            for (_key, child) in children.into_iter().rev() {
                stack.push(child);
            }
        }
        return Ok(Value::Stream(out));
    }
    let mut out = vec![current.clone()];
    let body = &args[0];
    let cond = args.get(1);
    let mut value = current.clone();
    for _ in 0..ITER_CAP {
        let nxt = flatten_one(eval(body, &value, ctx)?)?;
        if let Some(cond) = cond {
            let cond_value = flatten_one(eval(cond, &nxt, ctx)?)?;
            if !value::truthy(&cond_value) {
                break;
            }
        } else if matches!(nxt, Value::Null) {
            break;
        }
        out.push(nxt.clone());
        value = nxt;
    }
    Ok(Value::Stream(out))
}

fn eval_pick(args: &[Expr], current: &Value, ctx: &mut EvalContext) -> Result<Value, QueryError> {
    let mut paths: Vec<Vec<Value>> = Vec::new();
    for arg in args {
        let static_paths = collect_path_projections(arg);
        if !static_paths.is_empty() {
            paths.extend(static_paths);
            continue;
        }
        let result = eval(arg, current, ctx)?;
        for candidate in flatten(result) {
            if let Value::List(keys) = &candidate
                && keys
                    .iter()
                    .all(|k| matches!(k, Value::Str(_) | Value::Int(_)))
            {
                paths.push(keys.clone());
            }
        }
    }
    let mut out = Value::Null;
    for p in &paths {
        let (exists, value_at) = probe_path(current, p);
        if !exists {
            continue;
        }
        out = set_at_path(out, p, value_at)?;
    }
    Ok(out)
}

fn probe_path(value: &Value, path: &[Value]) -> (bool, Value) {
    let mut cur = value.clone();
    for key in path {
        cur = match &cur {
            Value::Null => return (false, Value::Null),
            Value::ObjectRef(o) => match key {
                Value::Str(k) => match o.fields.get(k) {
                    Some(v) => v.clone(),
                    None => return (false, Value::Null),
                },
                _ => return (false, Value::Null),
            },
            Value::Object(map) => match key {
                Value::Str(k) => match map.get(k) {
                    Some(v) => v.clone(),
                    None => return (false, Value::Null),
                },
                _ => return (false, Value::Null),
            },
            Value::List(items) | Value::Stream(items) => match key {
                Value::Int(i) => match value::resolve_index_pub(*i, items.len()) {
                    Some(idx) => items[idx].clone(),
                    None => return (false, Value::Null),
                },
                _ => return (false, Value::Null),
            },
            _ => return (false, Value::Null),
        };
    }
    (true, cur)
}

fn collect_path_projections(node: &Expr) -> Vec<Vec<Value>> {
    let mut paths = Vec::new();
    collect_walk(node, &mut paths);
    paths
}

fn collect_walk(node: &Expr, paths: &mut Vec<Vec<Value>>) {
    match node {
        Expr::CommaStream { parts, .. } => {
            for part in parts {
                collect_walk(part, paths);
            }
        }
        Expr::PathExpr { steps, head, .. } => {
            if head.is_none()
                && let Some(p) = path_from_steps(steps)
            {
                paths.push(p);
            }
        }
        _ => {}
    }
}

fn path_from_steps(steps: &[PathStep]) -> Option<Vec<Value>> {
    let mut out = Vec::new();
    for step in steps {
        match step {
            PathStep::Field { name, .. } => out.push(Value::Str(name.clone())),
            PathStep::Subscript {
                stream,
                index,
                regex,
                ..
            } => {
                if *stream || regex.is_some() || index.is_none() {
                    return None;
                }
                match index.as_deref() {
                    Some(Expr::Literal {
                        value: crate::ast::LitValue::Str(s),
                        ..
                    }) => out.push(Value::Str(s.clone())),
                    Some(Expr::Literal {
                        value: crate::ast::LitValue::Int(i),
                        ..
                    }) => out.push(Value::Int(*i)),
                    _ => return None,
                }
            }
        }
    }
    Some(out)
}

/// Python `str(key)` for INDEX keys (scalar shapes).
fn py_str_key(v: &Value) -> String {
    match v {
        Value::Str(s) => s.clone(),
        Value::Int(i) => i.to_string(),
        Value::Float(f) => crate::jsonfmt::py_float_repr(*f),
        Value::Bool(b) => if *b { "True" } else { "False" }.to_string(),
        Value::Null => "None".to_string(),
        other => other.describe(),
    }
}
