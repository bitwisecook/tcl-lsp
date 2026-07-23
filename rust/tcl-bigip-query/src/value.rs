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

//! Runtime value model for the query DSL.
//!
//! Covers the runtime values plus the scalar semantics the evaluator and
//! builtins share (`_truthy`, `_eq`, `_sort_key`, scalar coercion) and a
//! serialiser matching the canonical JSON byte format.
//!
//! Most values flow as the plain JSON-shaped variants — `Null` / `Bool` /
//! `Int` / `Float` / `Str` / `List` / `Object` — which keeps the evaluator
//! small. A handful of specialised wrappers carry the extra information the
//! rest of the pipeline needs:
//!
//! - [`Stream`](Value::Stream) — a flat sequence produced by the `[]`
//!   operator or any builtin that returns multiple values. Distinct from a
//!   `List` so the output formatter knows when to emit one-per-line.
//! - [`PathRef`] — a string-valued reference to another BIG-IP object by
//!   full-path. Field access transparently dereferences into the target
//!   object; arithmetic / string predicates treat it as a string.
//! - [`ObjectRef`] — a parsed BIG-IP object (virtual, pool, …) with its
//!   user-visible fields and the byte-ranges the edit planner rewrites.
//! - [`Drop`](Value::Drop) — the sentinel `select` returns to mean "drop
//!   the current value"; `flatten` removes it from streams.
//!
//! `ObjectRef` / `PathRef` are populated by the projection layer; the plain
//! variants alone drive the entire jq-flavoured core.

use std::cmp::Ordering;
use std::rc::Rc;

use indexmap::IndexMap;
use tcl_core_types::RecursionLimit;

use crate::projection::Container;

/// Maximum nesting depth every recursive `Value`-tree walker in this crate
/// enforces (issue #996) — `py_eq` here, plus `to_jsonable`, `walk_paths`,
/// `set_at_path`, `delete_at_path`, and `flatten_go` in
/// [`crate::builtins`], the special-form `walk` builtin in
/// [`crate::special`], the SCF-splice renderer's `format_value` in
/// [`crate::edit_plan`], and `json_to_value` in
/// [`crate::builtins::encoding`]. Each recurses natively once per nesting
/// level of a runtime `Value` (or, for `set_at_path`/`delete_at_path`, once
/// per element of a query-built path — itself unbounded, since `range()`
/// alone can build a path list far deeper than any real document ever
/// nests) with no cap before this fix.
///
/// This crate builds for the wasm32 query console (like `parser.rs`'s
/// `MAX_PARSE_DEPTH` and `eval.rs`'s `MAX_EVAL_DEPTH`), whose
/// host-controlled stack budget this repo does not control, so all of these
/// need a conservative cap independent of the ambient thread's stack. 64
/// mirrors `MAX_PARSE_DEPTH` (`parser.rs`) rather than the more generous
/// `MAX_EVAL_DEPTH` (256): these are all plain structural match-and-recurse
/// walkers with no interpreter dispatch overhead per frame — the same shape
/// and per-frame cost class as the parser — so its already-conservative,
/// wasm-safe figure carries over directly rather than needing its own
/// independent measurement. See
/// `docs/design/compiler/recursive-descent-depth-limits.md`.
pub(crate) const MAX_VALUE_WALK_DEPTH: RecursionLimit = RecursionLimit(64);

/// A single property value's byte location in the source text.
///
/// `range` covers just the value half of `key value` — assigning a new value
/// rewrites this span and leaves the key, indentation, and surrounding
/// stanza untouched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldSlot {
    pub source_uri: String,
    pub start: usize,
    pub end: usize,
    pub raw_text: String,
}

/// A string reference to another BIG-IP object.
///
/// String-like in every context that expects a scalar (the underlying
/// full-path), but the evaluator follows `.field` access through to the
/// target object when one resolves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathRef {
    pub full_path: String,
    /// The kind of object this path is expected to resolve to (`"ltm pool"`,
    /// …). Empty means "any matching kind".
    pub expected_kind: String,
}

impl PathRef {
    #[must_use]
    pub fn new(full_path: impl Into<String>, expected_kind: impl Into<String>) -> Self {
        PathRef {
            full_path: full_path.into(),
            expected_kind: expected_kind.into(),
        }
    }
}

/// A BIG-IP object exposed to the query DSL.
///
/// `kind` is the TMSH module + object type (`"ltm virtual"`). `fields` are
/// the user-visible property names with their current values. `field_slots`
/// maps the same keys to byte ranges so the edit planner can rewrite a
/// single property in place; the whole-stanza `stanza_slot` is captured
/// separately for SCF output and identity-field writes.
#[derive(Debug, Clone)]
pub struct ObjectRef {
    pub kind: String,
    pub full_path: String,
    pub fields: IndexMap<String, Value>,
    pub field_slots: IndexMap<String, FieldSlot>,
    pub stanza_slot: Option<FieldSlot>,
    /// Back-pointer URI used by `refs()` / `referenced_by()`.
    pub config_uri: String,
}

impl ObjectRef {
    /// The object's short name — the last `/`-delimited segment of its
    /// full-path.
    #[must_use]
    pub fn name(&self) -> &str {
        self.full_path.rsplit('/').next().unwrap_or(&self.full_path)
    }

    /// The object's partition — the first segment of a `/partition/...`
    /// path, or empty.
    #[must_use]
    pub fn partition(&self) -> &str {
        if let Some(rest) = self.full_path.strip_prefix('/') {
            // Equivalent to `full_path.split("/", 2)[1]`.
            return rest.split('/').next().unwrap_or("");
        }
        ""
    }
}

/// The runtime value type. The plain variants model the standard JSON-shaped
/// values; the rest are the DSL's specialised wrappers.
#[derive(Debug, Clone)]
pub enum Value {
    Null,
    Bool(bool),
    /// An integer.
    Int(i64),
    /// A float.
    Float(f64),
    Str(String),
    /// An explicit list (a literal array or a builtin result like `keys`).
    List(Vec<Value>),
    /// An object — insertion-ordered, like a dict.
    Object(IndexMap<String, Value>),
    /// A flat sequence produced by `[]` / `map(...)` / `select(...)`.
    Stream(Vec<Value>),
    /// A string reference to another BIG-IP object.
    PathRef(Rc<PathRef>),
    /// A projected BIG-IP object.
    ObjectRef(Rc<ObjectRef>),
    /// A navigable namespace / kind container projected from a
    /// `BigipConfig`.
    Container(Rc<Container>),
    /// The `select` "drop this value" sentinel.
    Drop,
}

impl Value {
    /// Build an object value from key/value pairs, preserving order.
    #[must_use]
    pub fn object(entries: impl IntoIterator<Item = (String, Value)>) -> Value {
        Value::Object(entries.into_iter().collect())
    }

    /// A short human description used in error messages.
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Value::Null => "null".to_string(),
            Value::Container(c) => format!("container({})", c.kind),
            Value::ObjectRef(o) => format!("object({})", o.kind),
            Value::PathRef(p) => format!(
                "path-ref({})",
                if p.full_path.is_empty() {
                    "<empty>"
                } else {
                    &p.full_path
                }
            ),
            Value::Stream(items) => format!("stream(len={})", items.len()),
            Value::List(items) => format!("list(len={})", items.len()),
            // Falls back to the type name.
            Value::Bool(_) => "bool".to_string(),
            Value::Int(_) => "int".to_string(),
            Value::Float(_) => "float".to_string(),
            Value::Str(_) => "str".to_string(),
            Value::Object(_) => "dict".to_string(),
            Value::Drop => "Drop".to_string(),
        }
    }

    /// The type name, used by a few builtin
    /// error messages (`flatten`, `combinations`, …) and the `type` builtin
    /// is handled separately.
    #[must_use]
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "NoneType",
            Value::Bool(_) => "bool",
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::Str(_) => "str",
            Value::List(_) => "list",
            Value::Object(_) => "dict",
            Value::Stream(_) => "Stream",
            Value::PathRef(_) => "PathRef",
            Value::ObjectRef(_) => "ObjectRef",
            Value::Container(_) => "Container",
            Value::Drop => "Drop",
        }
    }
}

/// Truthiness.
///
/// `null` / `false` / `""` / empty collections / `0` are falsy; everything
/// else is truthy. A `PathRef` is truthy when its full-path is non-empty;
/// `ObjectRef` is always truthy.
#[must_use]
pub fn truthy(value: &Value) -> bool {
    match value {
        Value::Null | Value::Drop => false,
        Value::Bool(b) => *b,
        Value::Str(s) => !s.is_empty(),
        Value::List(items) | Value::Stream(items) => !items.is_empty(),
        Value::PathRef(p) => !p.full_path.is_empty(),
        // `bool(value)` fall-through: 0 / 0.0 falsy, empty dict falsy.
        Value::Int(i) => *i != 0,
        Value::Float(f) => *f != 0.0,
        Value::Object(map) => !map.is_empty(),
        // `Container` (like `ObjectRef`) is always truthy.
        Value::ObjectRef(_) | Value::Container(_) => true,
    }
}

/// Equality semantics: a bool equals the matching int (`True == 1`,
/// `False == 0`) and numbers compare across int/float/bool by value.
///
/// Reproduces the quirks that matter to the DSL: `1 == 1.0`, `True == 1`,
/// `False == 0`, deep list / object comparison (object equality is
/// order-independent), and `PathRef == PathRef` by full-path.
///
/// `depth` is the nesting level of this call (0 at the top); past
/// [`MAX_VALUE_WALK_DEPTH`] this stops descending and reports the pair as
/// unequal rather than recursing further — issue #996. A conservative
/// "not equal" is the safe default here: it can only make an
/// astronomically-nested `==`/`!=`/`contains`/`index` comparison (never
/// reachable from a real document) report "different" instead of silently
/// claiming two values are equal when they were never actually compared.
#[must_use]
pub fn py_eq(lhs: &Value, rhs: &Value, depth: u32) -> bool {
    if MAX_VALUE_WALK_DEPTH.exceeded(depth) {
        return false;
    }
    match (lhs, rhs) {
        (Value::Null, Value::Null) => true,
        // A `bool` counts as the matching `int`, so `True == 1`, and
        // numbers compare across int/float/bool by numeric value.
        _ if is_number_like(lhs) && is_number_like(rhs) => num_value(lhs) == num_value(rhs),
        (Value::Str(a), Value::Str(b)) => a == b,
        (Value::PathRef(a), Value::PathRef(b)) => a.full_path == b.full_path,
        (Value::List(a), Value::List(b)) | (Value::Stream(a), Value::Stream(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(x, y)| py_eq(x, y, depth + 1))
        }
        (Value::Object(a), Value::Object(b)) => {
            a.len() == b.len()
                && a.iter()
                    .all(|(k, v)| b.get(k).is_some_and(|w| py_eq(v, w, depth + 1)))
        }
        (Value::ObjectRef(a), Value::ObjectRef(b)) => {
            Rc::ptr_eq(a, b)
                || (a.kind == b.kind
                    && a.full_path == b.full_path
                    && a.fields.len() == b.fields.len()
                    && a.fields
                        .iter()
                        .all(|(k, v)| b.fields.get(k).is_some_and(|w| py_eq(v, w, depth + 1))))
        }
        _ => false,
    }
}

fn is_number_like(v: &Value) -> bool {
    matches!(v, Value::Bool(_) | Value::Int(_) | Value::Float(_))
}

/// The numeric value of a `bool`/`int`/`float` (a `bool` counting as the
/// matching `int`), as `f64` for cross-type comparison.
// i64→f64 only to order numbers cross-type; a rounding tie past 2^53 can at most
// make two near-equal numbers compare equal, never reorder distinct buckets.
#[allow(clippy::cast_precision_loss)]
fn num_value(v: &Value) -> f64 {
    match v {
        Value::Bool(b) => f64::from(u8::from(*b)),
        Value::Int(i) => *i as f64,
        Value::Float(f) => *f,
        _ => f64::NAN,
    }
}

/// jq cross-type ordering.
///
/// Orders `null < false < true < numbers < strings < arrays < objects`,
/// with lexicographic / numeric ordering inside each family. `PathRef`
/// sorts in the string bucket by full-path; `ObjectRef` in the object
/// bucket by its sorted `(key, value)` pairs.
#[must_use]
pub fn sort_cmp(a: &Value, b: &Value) -> Ordering {
    let (ta, tb) = (sort_tag(a), sort_tag(b));
    if ta != tb {
        return ta.cmp(&tb);
    }
    match (a, b) {
        (Value::Bool(x), Value::Bool(y)) => x.cmp(y),
        _ if ta == 2 => num_value(a)
            .partial_cmp(&num_value(b))
            .unwrap_or(Ordering::Equal),
        _ if ta == 3 => string_repr(a).cmp(&string_repr(b)),
        (Value::List(x), Value::List(y)) => cmp_seq(x, y),
        _ if ta == 5 => cmp_object_keys(a, b),
        _ => Ordering::Equal,
    }
}

fn sort_tag(v: &Value) -> u8 {
    match v {
        Value::Null => 0,
        Value::Bool(_) => 1,
        Value::Int(_) | Value::Float(_) => 2,
        Value::Str(_) | Value::PathRef(_) => 3,
        Value::List(_) => 4,
        Value::Object(_) | Value::ObjectRef(_) => 5,
        // Stream / Container / Drop fall into `(6, str(value))`
        // bucket.
        Value::Stream(_) | Value::Container(_) | Value::Drop => 6,
    }
}

fn string_repr(v: &Value) -> String {
    match v {
        Value::Str(s) => s.clone(),
        Value::PathRef(p) => p.full_path.clone(),
        _ => String::new(),
    }
}

fn cmp_seq(x: &[Value], y: &[Value]) -> Ordering {
    for (a, b) in x.iter().zip(y) {
        let c = sort_cmp(a, b);
        if c != Ordering::Equal {
            return c;
        }
    }
    x.len().cmp(&y.len())
}

fn cmp_object_keys(a: &Value, b: &Value) -> Ordering {
    let ka = sorted_object_pairs(a);
    let kb = sorted_object_pairs(b);
    for ((k1, v1), (k2, v2)) in ka.iter().zip(&kb) {
        let kc = k1.cmp(k2);
        if kc != Ordering::Equal {
            return kc;
        }
        let vc = sort_cmp(v1, v2);
        if vc != Ordering::Equal {
            return vc;
        }
    }
    ka.len().cmp(&kb.len())
}

fn sorted_object_pairs(v: &Value) -> Vec<(String, Value)> {
    let mut pairs: Vec<(String, Value)> = match v {
        Value::Object(map) => map
            .iter()
            .map(|(k, val)| (k.clone(), val.clone()))
            .collect(),
        Value::ObjectRef(o) => o
            .fields
            .iter()
            .map(|(k, val)| (k.clone(), val.clone()))
            .collect(),
        _ => Vec::new(),
    };
    pairs.sort_by(|x, y| x.0.cmp(&y.0));
    pairs
}

/// Resolve a possibly-negative index against a length; `None` if out of
/// range. The slice index check is `-len <= i < len`.
// `len as i64` can't wrap for any real slice; the `as usize` results are guarded
// non-negative by the `i >= -len_i` / `i >= 0` checks, so no sign is lost.
#[must_use]
#[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
pub fn resolve_index_pub(i: i64, len: usize) -> Option<usize> {
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

/// Coerce a `PathRef` to its full-path string for scalar operators — port
/// of the evaluator's `_coerce_scalar`.
#[must_use]
pub fn coerce_scalar(value: Value) -> Value {
    match value {
        Value::PathRef(p) => Value::Str(p.full_path.clone()),
        other => other,
    }
}

#[cfg(test)]
mod recursion_tests {
    use super::*;

    /// A list nested `depth` levels deep, wrapping a single `Int(0)` leaf —
    /// `[[[...[0]...]]]`.
    fn deep_nested_list(depth: usize) -> Value {
        let mut v = Value::Int(0);
        for _ in 0..depth {
            v = Value::List(vec![v]);
        }
        v
    }

    /// Run *f* on a worker thread with a 256 MiB stack (mirroring
    /// `tests/hardening.rs`'s `with_big_stack`). Building — and, at scope
    /// end, dropping — a several-thousand-level-deep `Value` costs one
    /// native stack frame per level, since `Value`'s derived `Clone`/`Drop`
    /// have no depth cap of their own (unlike `py_eq` under test here), so
    /// the fixture itself needs more room than the harness's default ~2 MiB
    /// per-test thread provides — independently of whether the guard is
    /// correct.
    fn with_big_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
        std::thread::Builder::new()
            .stack_size(256 * 1024 * 1024)
            .spawn(f)
            .expect("spawn worker thread")
            .join()
            .expect("worker thread did not panic / overflow")
    }

    /// Regression coverage for issue #996: `py_eq` recurses once per nested
    /// `List`/`Object`/`ObjectRef` level, with no depth cap before this fix
    /// — reachable from the `==`/`!=` operators, `IN`, and every builtin
    /// that coerces through `contains`/`index` equality on
    /// generator-controlled `Value` trees (e.g. deeply nested `fromjson`
    /// input compared against itself). 5000 is comfortably past
    /// `MAX_VALUE_WALK_DEPTH` (64); the assertion is that `py_eq` returns at
    /// all, and — since the cap's documented fallback is a definite
    /// "unequal" — that it returns `false` rather than hanging or looping.
    #[test]
    fn deeply_nested_lists_compare_unequal_past_the_cap_without_crashing() {
        with_big_stack(|| {
            const DEPTH: usize = 5000;
            let a = deep_nested_list(DEPTH);
            let b = deep_nested_list(DEPTH);
            assert!(
                !py_eq(&a, &b, 0),
                "past the cap, py_eq must report unequal rather than never returning"
            );
        });
    }

    /// Lists nested well under `MAX_VALUE_WALK_DEPTH` still compare exactly
    /// as before this fix: equal when structurally equal, unequal on any
    /// difference — the safety net must not perturb realistic comparisons.
    #[test]
    fn moderately_nested_lists_still_compare_structurally() {
        let a = deep_nested_list(5);
        let b = deep_nested_list(5);
        assert!(
            py_eq(&a, &b, 0),
            "structurally identical lists must be equal"
        );

        let mut c = Value::Int(1); // differs from `deep_nested_list`'s Int(0) leaf
        for _ in 0..5 {
            c = Value::List(vec![c]);
        }
        assert!(
            !py_eq(&a, &c, 0),
            "lists differing only in the leaf value must be unequal"
        );
    }
}
