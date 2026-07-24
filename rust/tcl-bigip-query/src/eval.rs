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

//! Walk the query AST against a root value.
//!
//! The evaluator is a set of recursive functions that translate each AST
//! node into a [`Value`]. Streams flatten lazily: an expression producing a
//! `Stream` continues to flow through pipes one value at a time.
//!
//! It implements the full jq-flavoured core over the plain value model +
//! `Stream` / `ObjectRef` / `PathRef`, plus the
//! [`Container`](crate::projection::Container) navigation branches
//! (projection over a real `BigipConfig`), and the assignment → edit-plan
//! path.

// Index / length casts between `usize` and `i64` model unbounded
// integer indexing and are intentional throughout the evaluator. The
// arithmetic operators take operands by value to consume them on the
// list/string branches.
#![allow(
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::needless_pass_by_value
)]

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use tcl_bigip::parser::BigipConfig;

use crate::ast::{Expr, LitValue, PathStep};
use crate::builtins::{self, Builtin};
use crate::errors::QueryError;
use crate::projection;
use crate::value::{self, ObjectRef, PathRef, Value};

/// The per-file evaluation root.
///
/// For external-JSON roots `json_value` carries the parsed value and `.`
/// uses native dict/list semantics. BIG-IP roots carry the parsed
/// [`BigipConfig`] + the source text, and the projection layer navigates
/// them lazily through [`Container`](crate::projection::Container) / [`ObjectRef`] / [`PathRef`].
pub struct Root {
    pub uri: String,
    pub source: String,
    pub json_value: Option<Value>,
    /// The parsed BIG-IP configuration (empty for JSON roots).
    pub config: BigipConfig,
    /// Lazily built `(kind, full-path) -> ObjectRef` cache. Port of
    /// `Root._object_cache`; the compound key disambiguates objects that
    /// share a full-path across kinds.
    pub object_cache: RefCell<HashMap<(String, String), Rc<ObjectRef>>>,
    /// Lazily built BIG-IP reference graph, memoised on first use by the
    /// graph builtins (`refs` / `referenced_by` / …) and the rule `.refs`
    /// projection, caching the expensive `build_bigip_object_graph` step so
    /// repeated per-object queries don't rebuild the whole graph each time.
    pub object_graph: RefCell<Option<Rc<tcl_bigip::graph::ObjectGraph>>>,
}

impl std::fmt::Debug for Root {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Root")
            .field("uri", &self.uri)
            .field("is_json", &self.is_json())
            .finish_non_exhaustive()
    }
}

impl Root {
    /// Build a JSON-backed root — `.` resolves to *value*.
    #[must_use]
    pub fn json(uri: impl Into<String>, value: Value) -> Rc<Root> {
        Rc::new(Root {
            uri: uri.into(),
            source: String::new(),
            json_value: Some(value),
            config: BigipConfig::default(),
            object_cache: RefCell::new(HashMap::new()),
            object_graph: RefCell::new(None),
        })
    }

    /// Build a BIG-IP-backed root — `.` resolves to the synthetic
    /// `<root>` container over the parsed *config*. Port of the BIG-IP
    /// branch of `values.Root`.
    #[must_use]
    pub fn bigip(
        uri: impl Into<String>,
        source: impl Into<String>,
        config: BigipConfig,
    ) -> Rc<Root> {
        Rc::new(Root {
            uri: uri.into(),
            source: source.into(),
            json_value: None,
            config,
            object_cache: RefCell::new(HashMap::new()),
            object_graph: RefCell::new(None),
        })
    }

    #[must_use]
    pub fn is_json(&self) -> bool {
        self.json_value.is_some()
    }

    /// Return this root's BIG-IP reference graph, building it on first use
    /// and memoising it for subsequent graph-builtin / projection calls.
    ///
    /// The (expensive) `build_bigip_object_graph` result is cached so a query
    /// touching `refs` / `referenced_by` / `.refs` per object stays cheap.
    #[must_use]
    pub fn graph(&self) -> Rc<tcl_bigip::graph::ObjectGraph> {
        if let Some(g) = self.object_graph.borrow().as_ref() {
            return Rc::clone(g);
        }
        let ctx = tcl_bigip::graph::GraphContext::new();
        let sources = [(self.uri.clone(), self.source.clone())];
        let configs = [(self.uri.clone(), &self.config)];
        let graph = Rc::new(tcl_bigip::graph::build_bigip_object_graph(
            &sources, &configs, &ctx,
        ));
        *self.object_graph.borrow_mut() = Some(Rc::clone(&graph));
        graph
    }
}

/// Return the synthetic top-level input for *root*. JSON roots yield their
/// parsed value directly; BIG-IP roots yield the synthetic `<root>`
/// [`Container`](crate::projection::Container).
#[must_use]
pub fn root_container(root: &Rc<Root>) -> Value {
    projection::root_container(root)
}

/// Reader hook for `ucs_cert`.
///
/// Maps `(config_uri, cache_path)` to an `x509_parse`-shaped [`Value`] dict.
/// `dialects` must not import the `tooling` UCS layer, so the CLI injects this
/// callable `None`
/// means "not wired", and `ucs_cert` raises a clear error.
pub type UcsCertReader = Rc<dyn Fn(&str, &str) -> Result<Value, QueryError>>;

/// A file operation the DSL's forensic file builtins ask a [`FilesReader`] to
/// perform against a source's UCS archive.
pub enum FileOp {
    /// List the archive's forensic members with metadata (`path`, `size`,
    /// `sha256`, `is_text`) and no content — a light inventory.
    Inventory,
    /// Read one member's content as a UTF-8-lossy string.
    Content(String),
}

/// Reader hook for the `files` / `file` / `glob` / `grep` builtins: given a
/// source `config_uri` and a [`FileOp`], returns the requested [`Value`]. The
/// CLI injects this (it holds the archive bytes); `None` means "not wired"
/// (e.g. the in-browser console), and the file builtins raise a clear error.
pub type FilesReader = Rc<dyn Fn(&str, &FileOp) -> Result<Value, QueryError>>;

/// Evaluation context.
pub struct EvalContext {
    pub root: Rc<Root>,
    pub named_roots: HashMap<String, Rc<Root>>,
    pub merge_mode: bool,
    /// Every loaded root, in source order — the merge-mode "active roots".
    /// Graph builtins consult this when `merge_mode` so `refs` / `referenced_by`
    /// walk references across files. Empty / single-element in non-merge
    /// mode, where graph builtins stay scoped to the originating source.
    pub merge_roots: Vec<Rc<Root>>,
    /// Lexical `expr as $name | body` bindings; `$name` lookup checks here
    /// first and falls back to `named_roots`.
    pub bindings: HashMap<String, Value>,
    /// Edit ops queued by assignment statements; applied by the runner after
    /// each statement evaluates.
    pub edits: crate::edit_plan::EditPlan,
    /// Whether live network probes are enabled (the `--enable-probes` flag).
    /// When `false`, every network probe builtin raises the gating error
    /// before touching the network.
    pub probes_enabled: bool,
    /// Optional CA bundle path used by TLS-aware probes (`url_*` /
    /// `tls_handshake`) for chain verification.
    /// `None` falls back to the system trust store.
    pub ca_bundle: Option<String>,
    /// Reader hook for `ucs_cert`. `None` means
    /// no reader is wired (e.g. when driven outside the f5 CLI).
    pub ucs_cert_reader: Option<UcsCertReader>,
    /// Reader hook for the forensic `files` / `file` / `glob` / `grep`
    /// builtins. `None` means no reader is wired (e.g. the in-browser console).
    pub files_reader: Option<FilesReader>,
    /// Lazily built merged reference graph spanning every [`merge_roots`]
    /// entry — built on first graph-builtin call in merge mode and memoised
    /// for the duration of the statement so cross-file `refs` /
    /// `referenced_by` per object stay cheap. `None` until first use; unused
    /// in non-merge mode (graph builtins fall back to `Root::graph`).
    ///
    /// [`merge_roots`]: EvalContext::merge_roots
    pub merge_graph: RefCell<Option<Rc<tcl_bigip::graph::ObjectGraph>>>,
}

impl EvalContext {
    /// Return the merged reference graph spanning every [`merge_roots`] entry,
    /// building it on first use and memoising it on [`merge_graph`].
    ///
    /// Feeds every active root's `(uri, source)` + `(uri, config)` into the
    /// graph builder so a reference in one file resolves into an object in
    /// another. Built once per statement (the runner constructs a fresh
    /// context per root, but the merge graph is identical across them, so each
    /// rebuilds at most once).
    ///
    /// [`merge_roots`]: EvalContext::merge_roots
    /// [`merge_graph`]: EvalContext::merge_graph
    #[must_use]
    pub fn merged_graph(&self) -> Rc<tcl_bigip::graph::ObjectGraph> {
        if let Some(g) = self.merge_graph.borrow().as_ref() {
            return Rc::clone(g);
        }
        let ctx = tcl_bigip::graph::GraphContext::new();
        let sources: Vec<(String, String)> = self
            .merge_roots
            .iter()
            .map(|r| (r.uri.clone(), r.source.clone()))
            .collect();
        let configs: Vec<(String, &BigipConfig)> = self
            .merge_roots
            .iter()
            .map(|r| (r.uri.clone(), &r.config))
            .collect();
        let graph = Rc::new(tcl_bigip::graph::build_bigip_object_graph(
            &sources, &configs, &ctx,
        ));
        *self.merge_graph.borrow_mut() = Some(Rc::clone(&graph));
        graph
    }
}

impl EvalContext {
    #[must_use]
    pub fn new(root: Rc<Root>) -> Self {
        EvalContext {
            root,
            named_roots: HashMap::new(),
            merge_mode: false,
            merge_roots: Vec::new(),
            bindings: HashMap::new(),
            edits: crate::edit_plan::EditPlan::new(),
            probes_enabled: false,
            ca_bundle: None,
            ucs_cert_reader: None,
            files_reader: None,
            merge_graph: RefCell::new(None),
        }
    }
}

// Top-level entry points

/// Evaluate a single statement against `ctx.root`; return the flattened
/// list of values produced.
///
/// # Errors
/// Propagates any [`QueryError`] raised during evaluation.
pub fn evaluate_statement(stmt: &Expr, ctx: &mut EvalContext) -> Result<Vec<Value>, QueryError> {
    let root_input = root_container(&ctx.root);
    let out = eval(stmt, &root_input, ctx)?;
    Ok(flatten(out))
}

/// Evaluate every statement against the root and concatenate results.
///
/// # Errors
/// Propagates any [`QueryError`] raised during evaluation.
pub fn evaluate(
    program: &crate::ast::Program,
    ctx: &mut EvalContext,
) -> Result<Vec<Value>, QueryError> {
    let mut out = Vec::new();
    for stmt in &program.statements {
        out.extend(evaluate_statement(stmt, ctx)?);
    }
    Ok(out)
}

// Core dispatch

/// Recursion-depth ceiling for the evaluator. `eval` spends a native stack
/// frame (plus its per-node helper's frame) per nested AST form, so a
/// pathological program — a hand-built AST, or one that nests deeper at eval
/// than at parse — could otherwise overflow the stack.
///
/// Sized empirically: a debug build overflows a small 2 MiB worker stack near
/// ~700 nested `eval` frames. 256 sits well below that on the tightest stack
/// we run on, yet stays above the parser's `MAX_PARSE_DEPTH` (64) with ample
/// headroom so every tree the parser accepts still evaluates — the guard only
/// ever fires on an abusive (typically hand-built) AST.
const MAX_EVAL_DEPTH: usize = 256;

thread_local! {
    /// Per-thread evaluation recursion depth, reset to zero between
    /// top-level statements. A thread-local keeps the guard off the public
    /// `EvalContext` struct (which callers build with field literals) while
    /// still bounding every `eval` re-entry.
    static EVAL_DEPTH: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// RAII counter for [`EVAL_DEPTH`]: increments on construction, decrements on
/// drop so the depth unwinds correctly through `?` early returns and panics.
struct DepthGuard;

impl DepthGuard {
    /// Enter one evaluation level, or return the eval error at the cap.
    fn enter() -> Result<DepthGuard, QueryError> {
        let depth = EVAL_DEPTH.with(|d| {
            let next = d.get() + 1;
            d.set(next);
            next
        });
        if depth > MAX_EVAL_DEPTH {
            // No `DepthGuard` is constructed on this path, so undo the bump
            // by hand before bailing — the whole evaluation then unwinds.
            EVAL_DEPTH.with(|d| d.set(d.get() - 1));
            return Err(QueryError::eval("query nests too deeply to evaluate"));
        }
        Ok(DepthGuard)
    }
}

impl Drop for DepthGuard {
    fn drop(&mut self) {
        EVAL_DEPTH.with(|d| d.set(d.get().saturating_sub(1)));
    }
}

pub(crate) fn eval(
    node: &Expr,
    current: &Value,
    ctx: &mut EvalContext,
) -> Result<Value, QueryError> {
    let _guard = DepthGuard::enter()?;
    match node {
        Expr::Literal { value, .. } => Ok(lit_to_value(value)),
        Expr::Identity { .. } => Ok(current.clone()),
        Expr::Variable { name, .. } => eval_variable(name, ctx),
        Expr::ListLiteral { inner, .. } => match inner {
            None => Ok(Value::List(Vec::new())),
            Some(expr) => {
                let v = eval(expr, current, ctx)?;
                Ok(Value::List(flatten(v)))
            }
        },
        Expr::ObjectLiteral { entries, .. } => eval_object_literal(entries, current, ctx),
        Expr::PathExpr { steps, head, .. } => eval_path(steps, head.as_deref(), current, ctx),
        Expr::Pipe { lhs, rhs, .. } => {
            let lhs_v = eval(lhs, current, ctx)?;
            pipe_through(lhs_v, rhs, ctx)
        }
        Expr::LetBinding {
            source, name, body, ..
        } => eval_let_binding(source, name, body, current, ctx),
        Expr::Call { name, args, .. } => eval_call(name, args, current, ctx),
        Expr::BinOp { op, lhs, rhs, .. } => eval_binop(op, lhs, rhs, current, ctx),
        Expr::UnaryOp { op, operand, .. } => eval_unop(op, operand, current, ctx),
        Expr::Assignment {
            target,
            op,
            rhs,
            source,
            ..
        } => eval_assignment(target, op, rhs, source.as_deref(), current, ctx),
        Expr::IfThenElse {
            cond,
            then_body,
            elifs,
            else_body,
            ..
        } => eval_if_then_else(cond, then_body, elifs, else_body.as_deref(), current, ctx),
        Expr::CommaStream { parts, .. } => eval_comma_stream(parts, current, ctx),
    }
}

fn lit_to_value(lit: &LitValue) -> Value {
    match lit {
        LitValue::Null => Value::Null,
        LitValue::Bool(b) => Value::Bool(*b),
        LitValue::Int(i) => Value::Int(*i),
        LitValue::Float(f) => Value::Float(*f),
        LitValue::Str(s) => Value::Str(s.clone()),
    }
}

fn eval_comma_stream(
    parts: &[Expr],
    current: &Value,
    ctx: &mut EvalContext,
) -> Result<Value, QueryError> {
    let mut out = Vec::new();
    for part in parts {
        let v = eval(part, current, ctx)?;
        out.extend(flatten(v));
    }
    Ok(Value::Stream(out))
}

fn eval_if_then_else(
    cond: &Expr,
    then_body: &Expr,
    elifs: &[(Expr, Expr)],
    else_body: Option<&Expr>,
    current: &Value,
    ctx: &mut EvalContext,
) -> Result<Value, QueryError> {
    let cond_value = eval(cond, current, ctx)?;
    if let Value::Stream(items) = cond_value {
        let mut out = Vec::new();
        for item in items {
            let picked = pick_if_branch(&item, then_body, elifs, else_body, current, ctx)?;
            out.extend(flatten(picked));
        }
        return Ok(Value::Stream(out));
    }
    pick_if_branch(&cond_value, then_body, elifs, else_body, current, ctx)
}

fn pick_if_branch(
    cond_value: &Value,
    then_body: &Expr,
    elifs: &[(Expr, Expr)],
    else_body: Option<&Expr>,
    current: &Value,
    ctx: &mut EvalContext,
) -> Result<Value, QueryError> {
    if value::truthy(cond_value) {
        return eval(then_body, current, ctx);
    }
    for (elif_cond, elif_body) in elifs {
        let ec = eval(elif_cond, current, ctx)?;
        if let Value::Stream(items) = ec {
            if items.iter().any(value::truthy) {
                return eval(elif_body, current, ctx);
            }
            continue;
        }
        if value::truthy(&ec) {
            return eval(elif_body, current, ctx);
        }
    }
    if let Some(eb) = else_body {
        return eval(eb, current, ctx);
    }
    Ok(current.clone())
}

fn pipe_through(values: Value, rhs: &Expr, ctx: &mut EvalContext) -> Result<Value, QueryError> {
    match values {
        Value::Drop => Ok(Value::Stream(Vec::new())),
        Value::Stream(items) => {
            let mut out = Vec::new();
            for item in items {
                if matches!(item, Value::Drop) {
                    continue;
                }
                let v = eval(rhs, &item, ctx)?;
                out.extend(flatten(v));
            }
            Ok(Value::Stream(out))
        }
        other => eval(rhs, &other, ctx),
    }
}

// Object literals

fn eval_object_literal(
    entries: &[(String, Expr)],
    current: &Value,
    ctx: &mut EvalContext,
) -> Result<Value, QueryError> {
    if entries.is_empty() {
        return Ok(Value::Object(indexmap::IndexMap::new()));
    }
    let mut resolved: Vec<(String, Value)> = Vec::with_capacity(entries.len());
    for (key, value_expr) in entries {
        resolved.push((key.clone(), eval(value_expr, current, ctx)?));
    }
    let stream_indices: Vec<usize> = resolved
        .iter()
        .enumerate()
        .filter(|(_, (_, v))| matches!(v, Value::Stream(_)))
        .map(|(i, _)| i)
        .collect();
    if stream_indices.is_empty() {
        let mut map = indexmap::IndexMap::new();
        for (k, v) in resolved {
            map.insert(k, scalarise_dict_value(v));
        }
        return Ok(Value::Object(map));
    }
    let lengths: std::collections::BTreeSet<usize> = stream_indices
        .iter()
        .map(|&i| match &resolved[i].1 {
            Value::Stream(items) => items.len(),
            _ => 0,
        })
        .collect();
    if lengths.len() > 1 {
        let sorted: Vec<usize> = lengths.into_iter().collect();
        return Err(QueryError::eval(format!(
            "object literal: cannot broadcast streams of differing lengths ({sorted:?}); \
             collect one side with [...] first to keep each field a single list value"
        )));
    }
    let length = lengths.into_iter().next().unwrap_or(0);
    let mut rows = Vec::with_capacity(length);
    for idx in 0..length {
        let mut row = indexmap::IndexMap::new();
        for (i, (k, v)) in resolved.iter().enumerate() {
            let cell = if stream_indices.contains(&i) {
                match v {
                    Value::Stream(items) => items[idx].clone(),
                    _ => v.clone(),
                }
            } else {
                v.clone()
            };
            row.insert(k.clone(), scalarise_dict_value(cell));
        }
        rows.push(Value::Object(row));
    }
    Ok(Value::Stream(rows))
}

fn scalarise_dict_value(value: Value) -> Value {
    match value {
        Value::PathRef(p) => Value::Str(p.full_path.clone()),
        Value::ObjectRef(o) => Value::Str(o.full_path.clone()),
        other => other,
    }
}

// Variables

fn eval_variable(name: &str, ctx: &mut EvalContext) -> Result<Value, QueryError> {
    if let Some(v) = ctx.bindings.get(name) {
        return Ok(v.clone());
    }
    if let Some(root) = ctx.named_roots.get(name) {
        return Ok(root_container(root));
    }
    let mut bound: Vec<String> = ctx
        .named_roots
        .keys()
        .chain(ctx.bindings.keys())
        .cloned()
        .collect();
    bound.sort();
    bound.dedup();
    if bound.is_empty() {
        return Err(QueryError::eval(format!(
            "${name}: no variables are bound — introduce one with \
             ``expr as $name | ...`` or load several configs to auto-bind \
             one per filename stem."
        )));
    }
    let known = bound.join(", ");
    Err(QueryError::eval(format!(
        "${name}: unknown variable (known: {known})"
    )))
}

fn eval_let_binding(
    source: &Expr,
    name: &str,
    body: &Expr,
    current: &Value,
    ctx: &mut EvalContext,
) -> Result<Value, QueryError> {
    let source_value = eval(source, current, ctx)?;
    let (items, is_iterating) = match source_value {
        Value::Stream(items) => (items, true),
        other => (vec![other], false),
    };
    let n = items.len();
    let mut out = Vec::new();
    for item in items {
        let prior = ctx.bindings.insert(name.to_string(), item);
        let result = eval(body, current, ctx);
        match prior {
            Some(p) => {
                ctx.bindings.insert(name.to_string(), p);
            }
            None => {
                ctx.bindings.remove(name);
            }
        }
        out.extend(flatten(result?));
    }
    if is_iterating || n != 1 {
        return Ok(Value::Stream(out));
    }
    match out.len() {
        0 => Ok(Value::Stream(Vec::new())),
        1 => Ok(out.into_iter().next().unwrap()),
        _ => Ok(Value::Stream(out)),
    }
}

// Paths

fn eval_path(
    steps: &[PathStep],
    head: Option<&Expr>,
    current: &Value,
    ctx: &mut EvalContext,
) -> Result<Value, QueryError> {
    let mut is_stream = false;
    let mut values: Vec<Value> = if let Some(head_expr) = head {
        let head_value = eval(head_expr, current, ctx)?;
        match head_value {
            Value::Stream(items) => {
                is_stream = true;
                items
            }
            other => vec![other],
        }
    } else {
        vec![current.clone()]
    };
    let path_input = current.clone();
    for step in steps {
        let mut next_values = Vec::new();
        for value in &values {
            for (produced, produced_is_stream) in step_apply(value, step, &path_input, ctx)? {
                if produced_is_stream {
                    is_stream = true;
                }
                next_values.push(produced);
            }
        }
        values = next_values;
    }
    if is_stream || values.len() != 1 {
        return Ok(Value::Stream(values));
    }
    Ok(values.into_iter().next().unwrap())
}

fn step_apply(
    value: &Value,
    step: &PathStep,
    path_input: &Value,
    ctx: &mut EvalContext,
) -> Result<Vec<(Value, bool)>, QueryError> {
    match step {
        PathStep::Field { name, optional, .. } => match field_step(value, name, ctx) {
            Ok(items) => Ok(items.into_iter().map(|v| (v, false)).collect()),
            Err(e) => {
                if *optional {
                    Ok(Vec::new())
                } else {
                    Err(e)
                }
            }
        },
        PathStep::Subscript {
            stream,
            index,
            regex,
            optional,
            ..
        } => {
            if *stream {
                return match subscript_root(value, ctx) {
                    Ok(root) => Ok(stream_items(root).into_iter().map(|v| (v, true)).collect()),
                    Err(e) => {
                        if *optional {
                            Ok(Vec::new())
                        } else {
                            Err(e)
                        }
                    }
                };
            }
            if let Some(pattern) = regex {
                return match regex_subscript(value, pattern, ctx) {
                    Ok(items) => Ok(items.into_iter().map(|v| (v, true)).collect()),
                    Err(e) => {
                        if *optional {
                            Ok(Vec::new())
                        } else {
                            Err(e)
                        }
                    }
                };
            }
            let idx_val = eval(
                index.as_ref().expect("non-stream subscript has index"),
                path_input,
                ctx,
            )?;
            match subscript_step(value, &idx_val, ctx) {
                Ok(v) => Ok(vec![(v, false)]),
                Err(e) => {
                    if *optional {
                        Ok(Vec::new())
                    } else {
                        Err(e)
                    }
                }
            }
        }
    }
}

fn field_step(value: &Value, name: &str, ctx: &mut EvalContext) -> Result<Vec<Value>, QueryError> {
    match value {
        Value::PathRef(p) => match resolve_pathref(p, ctx) {
            Some(target) => field_step(&Value::ObjectRef(target), name, ctx),
            None => Ok(Vec::new()),
        },
        Value::Container(c) => Ok(vec![c.lookup(name)?]),
        Value::ObjectRef(o) => match o.fields.get(name) {
            Some(v) => Ok(vec![v.clone()]),
            None => Err(QueryError::eval(format!(
                "{}: no field {}",
                o.kind,
                pyr(name)
            ))),
        },
        Value::Object(map) => match map.get(name) {
            Some(v) => Ok(vec![v.clone()]),
            None => Err(QueryError::eval(format!("no field {}", pyr(name)))),
        },
        other => Err(QueryError::eval(format!(
            "cannot read field {name:?} on {}",
            other.describe()
        ))),
    }
}

fn subscript_root(value: &Value, ctx: &mut EvalContext) -> Result<Value, QueryError> {
    match value {
        Value::PathRef(p) => match resolve_pathref(p, ctx) {
            Some(target) => subscript_root(&Value::ObjectRef(target), ctx),
            None => Ok(Value::Stream(Vec::new())),
        },
        Value::Container(c) => {
            // Flatten container → container → object until the entries are
            // no longer all containers.
            let mut entries: Vec<Value> = c.entries().values().cloned().collect();
            while !entries.is_empty() && entries.iter().all(|e| matches!(e, Value::Container(_))) {
                let mut flat = Vec::new();
                for sub in &entries {
                    if let Value::Container(sc) = sub {
                        flat.extend(sc.entries().values().cloned());
                    }
                }
                entries = flat;
            }
            Ok(Value::List(entries))
        }
        Value::List(_) | Value::Stream(_) => Ok(value.clone()),
        Value::Object(map) => Ok(Value::List(map.values().cloned().collect())),
        Value::ObjectRef(o) => Ok(Value::List(o.fields.values().cloned().collect())),
        other => Err(QueryError::eval(format!(
            "cannot iterate {}",
            other.describe()
        ))),
    }
}

fn regex_subscript(
    value: &Value,
    pattern: &str,
    ctx: &mut EvalContext,
) -> Result<Vec<Value>, QueryError> {
    match value {
        Value::PathRef(p) => match resolve_pathref(p, ctx) {
            Some(target) => regex_subscript(&Value::ObjectRef(target), pattern, ctx),
            None => Ok(Vec::new()),
        },
        Value::Container(c) => {
            let entries = c.entries();
            let keys = c.regex_keys(pattern)?;
            Ok(keys
                .iter()
                .filter_map(|k| entries.get(k).cloned())
                .collect())
        }
        Value::Object(map) => {
            let rx = builtins::safe_regex_compile(pattern, "regex subscript")
                .map_err(eval_from_builtin)?;
            Ok(map
                .iter()
                .filter(|(k, _)| rx.is_match(k))
                .map(|(_, v)| v.clone())
                .collect())
        }
        Value::ObjectRef(o) => {
            let rx = builtins::safe_regex_compile(pattern, "regex subscript")
                .map_err(eval_from_builtin)?;
            if rx.is_match(&o.full_path) {
                Ok(vec![value.clone()])
            } else {
                Ok(Vec::new())
            }
        }
        other => Err(QueryError::eval(format!(
            "regex subscript not supported on {}",
            other.describe()
        ))),
    }
}

fn subscript_step(
    value: &Value,
    index: &Value,
    ctx: &mut EvalContext,
) -> Result<Value, QueryError> {
    match value {
        Value::PathRef(p) => match resolve_pathref(p, ctx) {
            Some(target) => subscript_step(&Value::ObjectRef(target), index, ctx),
            None => Err(QueryError::eval(format!(
                "cannot subscript unresolved path {}",
                pyr(&p.full_path)
            ))),
        },
        Value::Container(c) => match index {
            Value::Str(key) => c.lookup(key),
            Value::Int(i) => {
                let entries = c.entries();
                let keys: Vec<&String> = entries.keys().collect();
                match resolve_index(*i, keys.len()) {
                    Some(idx) => Ok(entries[keys[idx]].clone()),
                    None => Err(QueryError::eval(format!(
                        "{}: index {i} out of range",
                        c.kind
                    ))),
                }
            }
            other => Err(QueryError::eval(format!(
                "cannot subscript {} with {}",
                value.describe(),
                other.describe()
            ))),
        },
        Value::List(items) => {
            let i = require_index(index)?;
            let resolved = resolve_index(i, items.len())
                .ok_or_else(|| QueryError::eval(format!("list index {i} out of range")))?;
            Ok(items[resolved].clone())
        }
        Value::Stream(items) => {
            let i = require_index(index)?;
            let resolved = resolve_index(i, items.len())
                .ok_or_else(|| QueryError::eval(format!("list index {i} out of range")))?;
            Ok(items[resolved].clone())
        }
        Value::Object(map) => match index {
            Value::Str(key) => map
                .get(key)
                .cloned()
                .ok_or_else(|| QueryError::eval(format!("no field {}", pyr(key)))),
            Value::Int(i) => {
                let resolved = resolve_index(*i, map.len())
                    .ok_or_else(|| QueryError::eval(format!("object index {i} out of range")))?;
                Ok(map[resolved].clone())
            }
            other => Err(QueryError::eval(format!(
                "cannot subscript {} with {}",
                value.describe(),
                other.describe()
            ))),
        },
        Value::ObjectRef(o) => match index {
            Value::Str(key) => o
                .fields
                .get(key)
                .cloned()
                .ok_or_else(|| QueryError::eval(format!("{}: no field {}", o.kind, pyr(key)))),
            other => Err(QueryError::eval(format!(
                "cannot subscript {} with {}",
                value.describe(),
                other.describe()
            ))),
        },
        other => Err(QueryError::eval(format!(
            "cannot subscript {} with {}",
            other.describe(),
            index.describe()
        ))),
    }
}

fn require_index(index: &Value) -> Result<i64, QueryError> {
    match index {
        Value::Int(i) => Ok(*i),
        other => Err(QueryError::eval(format!(
            "list subscript must be an integer, got {}",
            other.describe()
        ))),
    }
}

/// Resolve a possibly-negative index against a length; `None` if out of
/// range. The index check is `-len <= i < len`.
fn resolve_index(i: i64, len: usize) -> Option<usize> {
    let len_i = len as i64;
    if i >= -len_i && i < len_i {
        Some(if i < 0 {
            (len_i + i) as usize
        } else {
            i as usize
        })
    } else {
        None
    }
}

/// Resolve the `ObjectRef` a `PathRef` points to — delegates to the
/// projection layer against `ctx.root`. JSON roots (which never produce
/// `PathRef`s) yield `None`.
fn resolve_pathref(reference: &Rc<PathRef>, ctx: &mut EvalContext) -> Option<Rc<ObjectRef>> {
    projection::resolve_pathref(reference, &ctx.root)
}

// Calls

fn eval_call(
    name: &str,
    args: &[Expr],
    current: &Value,
    ctx: &mut EvalContext,
) -> Result<Value, QueryError> {
    let spec = builtins::lookup(name)
        .ok_or_else(|| QueryError::eval(format!("unknown function {}", pyr(name))))?;
    let arity = args.len();

    // jq-style implicit `.`: a single-arg builtin called bare gets `current`.
    if arity == 0 && !spec.special_form && spec.min_args == 1 && spec.max_args == Some(1) {
        return call_impl(spec, std::slice::from_ref(current), ctx);
    }

    // jq-style implicit receiver for multi-arg builtins: `.x | f(a)` ≡ `f(.x, a)`.
    if !spec.special_form
        && !spec.with_ctx
        && spec.min_args >= 2
        && arity == spec.min_args - 1
        && spec.max_args.is_none_or(|m| arity < m)
    {
        let mut call_args = Vec::with_capacity(arity + 1);
        call_args.push(current.clone());
        for a in args {
            call_args.push(eval(a, current, ctx)?);
        }
        return call_impl(spec, &call_args, ctx);
    }

    if arity < spec.min_args || spec.max_args.is_some_and(|m| arity > m) {
        let expected = if spec.max_args == Some(spec.min_args) {
            spec.min_args.to_string()
        } else if let Some(m) = spec.max_args {
            format!("{}..{}", spec.min_args, m)
        } else {
            format!(">= {}", spec.min_args)
        };
        return Err(QueryError::eval(format!(
            "{name}: expected {expected} argument(s), got {arity}"
        )));
    }

    if spec.special_form {
        return crate::special::eval_special_form(name, args, current, ctx);
    }

    let mut evaluated = Vec::with_capacity(arity);
    for a in args {
        evaluated.push(eval(a, current, ctx)?);
    }

    if !spec.stream_aware
        && (!spec.with_ctx || spec.broadcasts)
        && let Some(plan) = broadcast_call_args(name, &evaluated)?
    {
        let mut out = Vec::with_capacity(plan.len());
        for tup in plan {
            out.push(call_impl(spec, &tup, ctx)?);
        }
        return Ok(Value::Stream(out));
    }
    call_impl(spec, &evaluated, ctx)
}

fn call_impl(
    spec: &builtins::BuiltinSpec,
    args: &[Value],
    ctx: &mut EvalContext,
) -> Result<Value, QueryError> {
    match &spec.imp {
        Builtin::Plain(f) => f(args),
        Builtin::Ctx(f) => f(args, ctx),
        Builtin::Special => Err(QueryError::eval(format!(
            "{}: special form dispatched as plain call",
            spec.name
        ))),
    }
}

fn broadcast_call_args(name: &str, args: &[Value]) -> Result<Option<Vec<Vec<Value>>>, QueryError> {
    let stream_indices: Vec<usize> = args
        .iter()
        .enumerate()
        .filter(|(_, a)| matches!(a, Value::Stream(_)))
        .map(|(i, _)| i)
        .collect();
    if stream_indices.is_empty() {
        return Ok(None);
    }
    let lengths: std::collections::BTreeSet<usize> = stream_indices
        .iter()
        .map(|&i| match &args[i] {
            Value::Stream(items) => items.len(),
            _ => 0,
        })
        .collect();
    if lengths.len() > 1 {
        let sorted: Vec<usize> = lengths.into_iter().collect();
        return Err(QueryError::eval(format!(
            "{name}: cannot broadcast streams of differing lengths ({sorted:?}); \
             collect one side with [...] first"
        )));
    }
    let length = lengths.into_iter().next().unwrap_or(0);
    let mut plan = Vec::with_capacity(length);
    for idx in 0..length {
        let mut row = Vec::with_capacity(args.len());
        for (i, a) in args.iter().enumerate() {
            if stream_indices.contains(&i) {
                if let Value::Stream(items) = a {
                    row.push(items[idx].clone());
                }
            } else {
                row.push(a.clone());
            }
        }
        plan.push(row);
    }
    Ok(Some(plan))
}

// Operators

fn eval_binop(
    op: &str,
    lhs: &Expr,
    rhs: &Expr,
    current: &Value,
    ctx: &mut EvalContext,
) -> Result<Value, QueryError> {
    if op == "and" {
        let l = eval(lhs, current, ctx)?;
        if !value::truthy(&l) {
            return Ok(Value::Bool(false));
        }
        let r = eval(rhs, current, ctx)?;
        return Ok(Value::Bool(value::truthy(&r)));
    }
    if op == "or" {
        let l = eval(lhs, current, ctx)?;
        if value::truthy(&l) {
            return Ok(Value::Bool(true));
        }
        let r = eval(rhs, current, ctx)?;
        return Ok(Value::Bool(value::truthy(&r)));
    }
    let lhs_raw = eval(lhs, current, ctx)?;
    let rhs_raw = eval(rhs, current, ctx)?;
    if matches!(lhs_raw, Value::Stream(_)) || matches!(rhs_raw, Value::Stream(_)) {
        return broadcast_binop(op, lhs_raw, rhs_raw);
    }
    apply_scalar_binop(
        op,
        value::coerce_scalar(lhs_raw),
        value::coerce_scalar(rhs_raw),
    )
}

pub(crate) fn apply_scalar_binop(op: &str, lhs: Value, rhs: Value) -> Result<Value, QueryError> {
    let lhs = value::coerce_scalar(lhs);
    let rhs = value::coerce_scalar(rhs);
    match op {
        "==" => Ok(Value::Bool(value::py_eq(&lhs, &rhs, 0))),
        "!=" => Ok(Value::Bool(!value::py_eq(&lhs, &rhs, 0))),
        "<" | "<=" | ">" | ">=" => cmp(&lhs, &rhs, op),
        "+" => add(lhs, rhs),
        "-" => sub(lhs, rhs),
        "*" => mul(lhs, rhs),
        "/" => div(lhs, rhs),
        other => Err(QueryError::eval(format!("unsupported operator {other:?}"))),
    }
}

fn broadcast_binop(op: &str, lhs: Value, rhs: Value) -> Result<Value, QueryError> {
    match (lhs, rhs) {
        (Value::Stream(a), Value::Stream(b)) => {
            if a.len() != b.len() {
                return Err(QueryError::eval(format!(
                    "cannot {op:?} two streams of differing lengths ({} vs {}) — \
                     collect one side with [...] first",
                    a.len(),
                    b.len()
                )));
            }
            let mut out = Vec::with_capacity(a.len());
            for (x, y) in a.into_iter().zip(b) {
                out.push(apply_scalar_binop(op, x, y)?);
            }
            Ok(Value::Stream(out))
        }
        (Value::Stream(a), rhs) => {
            let mut out = Vec::with_capacity(a.len());
            for x in a {
                out.push(apply_scalar_binop(op, x, rhs.clone())?);
            }
            Ok(Value::Stream(out))
        }
        (lhs, Value::Stream(b)) => {
            let mut out = Vec::with_capacity(b.len());
            for y in b {
                out.push(apply_scalar_binop(op, lhs.clone(), y)?);
            }
            Ok(Value::Stream(out))
        }
        _ => unreachable!("broadcast_binop called without a stream operand"),
    }
}

fn cmp(lhs: &Value, rhs: &Value, op: &str) -> Result<Value, QueryError> {
    let both_num = matches!(lhs, Value::Int(_) | Value::Float(_) | Value::Bool(_))
        && matches!(rhs, Value::Int(_) | Value::Float(_) | Value::Bool(_));
    let same_kind = std::mem::discriminant(lhs) == std::mem::discriminant(rhs);
    if !same_kind && !both_num {
        return Err(QueryError::eval(format!(
            "cannot compare {} with {}",
            lhs.describe(),
            rhs.describe()
        )));
    }
    let ord = value::sort_cmp(lhs, rhs);
    let result = match op {
        "<" => ord.is_lt(),
        "<=" => ord.is_le(),
        ">" => ord.is_gt(),
        _ => ord.is_ge(),
    };
    Ok(Value::Bool(result))
}

fn num_pair(lhs: &Value, rhs: &Value) -> Option<(NumKind, f64, f64, i64, i64)> {
    let (lk, lf, li) = num_parts(lhs)?;
    let (rk, rf, ri) = num_parts(rhs)?;
    let kind = if lk == NumKind::Float || rk == NumKind::Float {
        NumKind::Float
    } else {
        NumKind::Int
    };
    Some((kind, lf, rf, li, ri))
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum NumKind {
    Int,
    Float,
}

// i64→f64 only for the cross-type numeric compare; the exact `i64` is kept
// alongside, so precision loss past 2^53 never affects an int/int comparison.
#[allow(clippy::cast_precision_loss)]
fn num_parts(v: &Value) -> Option<(NumKind, f64, i64)> {
    match v {
        Value::Bool(b) => Some((NumKind::Int, f64::from(u8::from(*b)), i64::from(*b))),
        Value::Int(i) => Some((NumKind::Int, *i as f64, *i)),
        Value::Float(f) => Some((NumKind::Float, *f, 0)),
        _ => None,
    }
}

/// The runtime error raised when integer arithmetic on untrusted query
/// operands exceeds the `i64` range. Phrased like tclsh's `expr` overflow so
/// the message reads naturally for the DSL's users.
fn int_overflow() -> QueryError {
    QueryError::eval("integer overflow")
}

fn add(lhs: Value, rhs: Value) -> Result<Value, QueryError> {
    if let (Value::Str(a), Value::Str(b)) = (&lhs, &rhs) {
        return Ok(Value::Str(format!("{a}{b}")));
    }
    if let Some((kind, lf, rf, li, ri)) = num_pair(&lhs, &rhs) {
        return Ok(match kind {
            // The operands come from untrusted query input; mirror tclsh's
            // "integer overflow" rather than panicking in debug / silently
            // wrapping in release the way `li + ri` would.
            NumKind::Int => Value::Int(li.checked_add(ri).ok_or_else(int_overflow)?),
            NumKind::Float => Value::Float(lf + rf),
        });
    }
    if let Value::List(mut a) = lhs {
        match rhs {
            Value::List(b) => a.extend(b),
            other => a.push(other),
        }
        return Ok(Value::List(a));
    }
    // str + scalar / scalar + str coercion.
    if let Value::Str(a) = &lhs
        && is_concat_scalar(&rhs)
    {
        return Ok(Value::Str(format!("{a}{}", scalar_to_string(&rhs))));
    }
    if let Value::Str(b) = &rhs
        && is_concat_scalar(&lhs)
    {
        return Ok(Value::Str(format!("{}{b}", scalar_to_string(&lhs))));
    }
    Err(QueryError::eval(format!(
        "cannot add {} and {}",
        lhs.describe(),
        rhs.describe()
    )))
}

fn is_concat_scalar(value: &Value) -> bool {
    matches!(
        value,
        Value::Int(_) | Value::Float(_) | Value::Bool(_) | Value::PathRef(_) | Value::Null
    )
}

fn scalar_to_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(b) => if *b { "true" } else { "false" }.to_string(),
        Value::PathRef(p) => p.full_path.clone(),
        Value::Int(i) => i.to_string(),
        Value::Float(f) => crate::jsonfmt::py_float_repr(*f),
        Value::Str(s) => s.clone(),
        other => other.describe(),
    }
}

fn sub(lhs: Value, rhs: Value) -> Result<Value, QueryError> {
    if let Some((kind, lf, rf, li, ri)) = num_pair(&lhs, &rhs) {
        return Ok(match kind {
            // See `add`: subtraction of two untrusted integers can overflow
            // (e.g. `i64::MIN - 1`), so report it instead of wrapping.
            NumKind::Int => Value::Int(li.checked_sub(ri).ok_or_else(int_overflow)?),
            NumKind::Float => Value::Float(lf - rf),
        });
    }
    if let Value::List(a) = &lhs {
        // Equality coerces both sides' PathRef to its full-path string, exactly
        // as `contains` / `index` / the scalar `==` operator do — otherwise
        // removing config references by path string silently removes nothing
        // (a PathRef never `py_eq`s an equal Str) — RUST_ISSUE_122.
        let eq = |item: &Value, target: &Value| -> bool {
            value::py_eq(
                &value::coerce_scalar(item.clone()),
                &value::coerce_scalar(target.clone()),
                0,
            )
        };
        let removed: Vec<Value> = match &rhs {
            Value::List(b) => a
                .iter()
                .filter(|item| !b.iter().any(|x| eq(item, x)))
                .cloned()
                .collect(),
            other => a.iter().filter(|item| !eq(item, other)).cloned().collect(),
        };
        return Ok(Value::List(removed));
    }
    Err(QueryError::eval(format!(
        "cannot subtract {} from {}",
        rhs.describe(),
        lhs.describe()
    )))
}

fn mul(lhs: Value, rhs: Value) -> Result<Value, QueryError> {
    if let Some((kind, lf, rf, li, ri)) = num_pair(&lhs, &rhs) {
        return Ok(match kind {
            // See `add`: multiplication of two untrusted integers overflows
            // fastest of all, so report it instead of wrapping.
            NumKind::Int => Value::Int(li.checked_mul(ri).ok_or_else(int_overflow)?),
            NumKind::Float => Value::Float(lf * rf),
        });
    }
    Err(QueryError::eval(format!(
        "cannot multiply {} and {}",
        lhs.describe(),
        rhs.describe()
    )))
}

fn div(lhs: Value, rhs: Value) -> Result<Value, QueryError> {
    // True division always yields a float.
    if let Some((_, lf, rf, _, _)) = num_pair(&lhs, &rhs) {
        if rf == 0.0 {
            return Err(QueryError::eval("division by zero"));
        }
        return Ok(Value::Float(lf / rf));
    }
    Err(QueryError::eval(format!(
        "cannot divide {} by {}",
        lhs.describe(),
        rhs.describe()
    )))
}

fn eval_unop(
    op: &str,
    operand: &Expr,
    current: &Value,
    ctx: &mut EvalContext,
) -> Result<Value, QueryError> {
    let val = eval(operand, current, ctx)?;
    match op {
        "not" => Ok(Value::Bool(!value::truthy(&val))),
        "-" => match val {
            Value::Bool(_) => Err(QueryError::eval("cannot negate a boolean")),
            // `checked_neg` so negating `i64::MIN` is a clean error, not an
            // overflow panic in debug / wraparound in release (issue 193).
            Value::Int(i) => i
                .checked_neg()
                .map(Value::Int)
                .ok_or_else(|| QueryError::eval("integer negation overflows i64")),
            Value::Float(f) => Ok(Value::Float(-f)),
            other => Err(QueryError::eval(format!(
                "cannot negate {}",
                other.describe()
            ))),
        },
        other => Err(QueryError::eval(format!(
            "unsupported unary operator {other:?}"
        ))),
    }
}

// Helpers

/// Flatten a value into a list, dropping the `select` sentinel and
/// unwrapping a top-level `Stream`.
#[must_use]
pub fn flatten(value: Value) -> Vec<Value> {
    match value {
        Value::Drop => Vec::new(),
        Value::Stream(items) => items
            .into_iter()
            .filter(|v| !matches!(v, Value::Drop))
            .collect(),
        other => vec![other],
    }
}

/// Reduce a value to a single item, erroring on empty / multi.
pub(crate) fn flatten_one(value: Value) -> Result<Value, QueryError> {
    let mut flat = flatten(value);
    match flat.len() {
        0 => Err(QueryError::eval("empty stream cannot be assigned")),
        1 => Ok(flat.pop().unwrap()),
        _ => Err(QueryError::eval("assignment RHS produced multiple values")),
    }
}

/// A list view of *value*: `Stream`/`List` give their items, scalars give a
/// one-element list.
#[must_use]
pub fn stream_items(value: Value) -> Vec<Value> {
    match value {
        Value::Stream(items) | Value::List(items) => items,
        other => vec![other],
    }
}

/// Map a [`QueryError::Builtin`] raised by a helper into the `EvalError`
/// channel the regex-subscript path uses, carrying the underlying message text.
fn eval_from_builtin(e: QueryError) -> QueryError {
    QueryError::Eval(e.message().to_string())
}

/// `eval_from_builtin` exposed to the projection module's `Container`
/// regex-subscript path.
#[must_use]
pub fn eval_from_builtin_pub(e: QueryError) -> QueryError {
    eval_from_builtin(e)
}

/// Repr-style rendering of a string for `{name!r}`-style error messages.
fn pyr(s: &str) -> String {
    crate::lexer::py_repr_str(s)
}

/// `pyr` exposed to the projection module's error messages.
#[must_use]
pub fn pyr_pub(s: &str) -> String {
    pyr(s)
}

// Assignments
//
// Assignments collect into an edit plan and return the post-edit value. This
// resolves the LHS targets so the error paths (which are all an external-JSON
// query can hit, since JSON has no `ObjectRef`s) match the canonical behaviour exactly.

fn eval_assignment(
    target: &Expr,
    op: &str,
    rhs: &Expr,
    source: Option<&Expr>,
    current: &Value,
    ctx: &mut EvalContext,
) -> Result<Value, QueryError> {
    let path_input = match source {
        Some(src) => eval(src, current, ctx)?,
        None => current.clone(),
    };
    let targets = resolve_assignment_targets(target, &path_input, ctx)?;
    let mut produced: Vec<Value> = Vec::new();
    for target in &targets {
        let new_value = compute_assignment_value(op, rhs, target, current, ctx)?;
        let edit_op = build_edit_op(target, op, new_value.clone(), ctx);
        ctx.edits.add(edit_op);
        produced.push(new_value);
    }
    match produced.len() {
        0 => Ok(Value::Stream(Vec::new())),
        1 => Ok(produced.pop().unwrap()),
        _ => Ok(Value::Stream(produced)),
    }
}

/// A single resolved writable LHS.
struct AssignTarget {
    obj: Rc<ObjectRef>,
    field_name: String,
    current_value: Value,
}

fn resolve_assignment_targets(
    target: &Expr,
    current: &Value,
    ctx: &mut EvalContext,
) -> Result<Vec<AssignTarget>, QueryError> {
    let Expr::PathExpr { steps, head, .. } = target else {
        return Err(QueryError::eval("assignment target must be a path"));
    };
    if steps.is_empty() {
        return Err(QueryError::eval("cannot assign to the identity ('.')"));
    }
    let (final_step, prefix_steps) = steps.split_last().unwrap();
    let PathStep::Field {
        name: final_name, ..
    } = final_step
    else {
        return Err(QueryError::eval(
            "assignment LHS must end in a field access (e.g. '.foo'); \
             subscript-only paths are not writable",
        ));
    };
    let prefix_value = eval_path(prefix_steps, head.as_deref(), current, ctx)?;
    let mut targets = Vec::new();
    for obj in flatten(prefix_value) {
        let obj = match obj {
            Value::PathRef(p) => match resolve_pathref(&p, ctx) {
                Some(o) => Value::ObjectRef(o),
                None => {
                    return Err(QueryError::eval(format!(
                        "cannot assign through unresolved path {}",
                        pyr(&p.full_path)
                    )));
                }
            },
            other => other,
        };
        let Value::ObjectRef(o) = &obj else {
            return Err(QueryError::eval(format!(
                "cannot assign to {} on {} (LHS must resolve to a BIG-IP object)",
                pyr(final_name),
                obj.describe()
            )));
        };
        let Some(current_value) = o.fields.get(final_name) else {
            return Err(QueryError::eval(format!(
                "{}: no field {}",
                o.kind,
                pyr(final_name)
            )));
        };
        targets.push(AssignTarget {
            obj: Rc::clone(o),
            field_name: final_name.clone(),
            current_value: current_value.clone(),
        });
    }
    Ok(targets)
}

/// Compute the new value for one assignment target.
fn compute_assignment_value(
    op: &str,
    rhs: &Expr,
    target: &AssignTarget,
    outer: &Value,
    ctx: &mut EvalContext,
) -> Result<Value, QueryError> {
    match op {
        "=" => flatten_one(eval(rhs, outer, ctx)?),
        "|=" => flatten_one(eval(rhs, &target.current_value, ctx)?),
        "+=" => {
            let rhs_val = flatten_one(eval(rhs, outer, ctx)?)?;
            add(target.current_value.clone(), rhs_val)
        }
        "-=" => {
            let rhs_val = flatten_one(eval(rhs, outer, ctx)?)?;
            sub(target.current_value.clone(), rhs_val)
        }
        other => Err(QueryError::eval(format!(
            "unknown assignment operator: {other}"
        ))),
    }
}

/// Build the [`EditOp`](crate::edit_plan::EditOp) for one resolved target.
fn build_edit_op(
    target: &AssignTarget,
    op: &str,
    new_value: Value,
    ctx: &EvalContext,
) -> crate::edit_plan::EditOp {
    let source_uri = if target.obj.config_uri.is_empty() {
        ctx.root.uri.clone()
    } else {
        target.obj.config_uri.clone()
    };
    crate::edit_plan::EditOp {
        source_uri,
        object_path: target.obj.full_path.clone(),
        object_kind: target.obj.kind.clone(),
        field_name: target.field_name.clone(),
        operator: op.to_owned(),
        new_value,
        field_slot: target.obj.field_slots.get(&target.field_name).cloned(),
        stanza_slot: target.obj.stanza_slot.clone(),
        // Evaluator-driven assignments are strict: a zero-occurrence identity
        // rename raises. Only the tolerant `rename()` builtin sets this false.
        strict: true,
    }
}
