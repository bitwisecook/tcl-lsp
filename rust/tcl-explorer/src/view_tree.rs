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

//! Data-driven view models for the explorer TUI.
//!
//! Every analysis view is
//! an interactive tree of [`ViewNode`]s built from the *same* serialised
//! result the GUI consumes ([`crate::serialise::serialise_result`]). Each
//! node has a one-line `label`, a `detail` table revealed when highlighted,
//! and optional `children`. Keeping these builders next to the one
//! serialisation means the GUI and TUI show the same fields.
//!
//! `asm` / `wasm` deliberately stay on the text renderer — they
//! are specialised disassembly, navigated rather than expanded.

use serde_json::Value;

mod aot;
mod cfg;
mod def_use;
mod front_end;
mod interprocedural;
mod irules;
mod optimiser;
mod value_lattices;
mod warnings;
mod world_ssa;

use aot::{build_aot, build_semantic};
use cfg::{build_cfg, build_dominators, build_loops};
use def_use::{build_dataflow, build_liveness};
use front_end::{build_ir, build_source_map, build_structural_index};
use interprocedural::{build_interproc, build_unit_scope};
use irules::{build_connection_scope, build_event_order, build_irules};
#[cfg(test)]
use optimiser::opt_leaf;
use optimiser::{build_gvn, build_opt, build_optimiser_passes};
use value_lattices::{build_intervals, build_rendered, build_sccp, build_taint_facts, build_types};
use warnings::{build_bounds, build_callouts, build_shimmer, build_taint};
use world_ssa::build_world_ssa;

/// The source span a view row points at, in the two coordinate systems its
/// consumers need.
///
/// Byte offsets are what the web front-end slices the source string with;
/// UTF-16 line/column is what an editor host places a caret with. Both come
/// straight from [`crate::formatters::range_dict`], so neither is re-derived.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewRange {
    /// 0-based start line.
    pub start_line: u32,
    /// 0-based start column, in UTF-16 code units.
    pub start_col_utf16: u32,
    /// 0-based end line.
    pub end_line: u32,
    /// Exclusive end column, in UTF-16 code units.
    pub end_col_utf16: u32,
    /// Start byte offset.
    pub start_offset: u32,
    /// Exclusive end byte offset.
    pub end_offset: u32,
}

impl ViewRange {
    /// Read a range dict, or `None` when the value is absent or not a range
    /// (`range_or_null` emits `null` for a statement with no span).
    #[must_use]
    pub fn from_value(v: &Value) -> Option<Self> {
        if !v.is_object() {
            return None;
        }
        let field = |k: &str| v[k].as_u64().map(|n| u32::try_from(n).unwrap_or(u32::MAX));
        Some(Self {
            start_line: field("startLine")?,
            start_col_utf16: field("startColUtf16")?,
            end_line: field("endLine")?,
            end_col_utf16: field("endColUtf16")?,
            start_offset: field("startOffset")?,
            end_offset: field("endOffset")?,
        })
    }
}

/// One expandable row: a summary `label`, a `detail` table, and children.
///
/// `Serialize` is for the differential test harness (compared against
/// `ViewNode`), not part of the explorer JSON contract.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize)]
pub struct ViewNode {
    /// One-line summary shown in the tree.
    pub label: String,
    /// Key/value rows shown when the node is highlighted.
    pub detail: Vec<(String, String)>,
    /// Nested rows.
    pub children: Vec<ViewNode>,
    /// Optional style hint (a colour name).
    pub style: Option<String>,
    /// Where this row points in the source, when it points anywhere.
    ///
    /// The `detail` table already carries a human-readable `range` row, but
    /// that is a formatted string — a consumer that wants to *navigate* had to
    /// parse `line:col  (start…end)` back out of it. This keeps the structured
    /// value beside it so navigation reads a field instead.
    pub range: Option<ViewRange>,
}

impl ViewNode {
    /// Attach the source span this row points at, from its range dict.
    #[must_use]
    fn with_range(mut self, range: &Value) -> Self {
        self.range = ViewRange::from_value(range);
        self
    }

    fn leaf(label: impl Into<String>, detail: Vec<(String, String)>, style: Option<&str>) -> Self {
        Self {
            label: label.into(),
            detail,
            children: Vec::new(),
            style: style.map(str::to_owned),
            range: None,
        }
    }

    fn branch(
        label: impl Into<String>,
        detail: Vec<(String, String)>,
        children: Vec<ViewNode>,
        style: Option<&str>,
    ) -> Self {
        Self {
            label: label.into(),
            detail,
            children,
            style: style.map(str::to_owned),
            range: None,
        }
    }

    fn note(label: impl Into<String>, style: &str) -> Self {
        Self::leaf(label, Vec::new(), Some(style))
    }
}

// Scalar/JSON accessors.

fn s(v: &Value, key: &str) -> String {
    v.get(key).map_or_default(jstr)
}

fn jstr(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// Render a JSON scalar as a display string for label interpolation.
///
/// The only divergence from [`jstr`] is booleans: they render as
/// `True`/`False` (capitalised) where Rust's `bool` `Display` emits
/// `true`/`false`. The explorer's SSA-view const-branch label interpolates a
/// JSON bool, so it is capitalised here to stay byte-identical.
fn pystr(v: &Value) -> String {
    match v {
        Value::Bool(true) => "True".to_owned(),
        Value::Bool(false) => "False".to_owned(),
        other => jstr(other),
    }
}

fn arr<'a>(v: &'a Value, key: &str) -> &'a [Value] {
    v.get(key)
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

fn det(k: &str, v: impl Into<String>) -> (String, String) {
    (k.to_owned(), v.into())
}

fn yn(v: &Value) -> &'static str {
    if v.as_bool().unwrap_or(false) {
        "yes"
    } else {
        "no"
    }
}

/// An interval endpoint: `null` is ±infinity.
fn fmt_bound(v: &Value, positive: bool) -> String {
    if v.is_null() {
        if positive { "+inf" } else { "-inf" }.to_owned()
    } else {
        jstr(v)
    }
}

/// A range dict → `line:col  (start…end)`.
fn rng(r: &Value) -> String {
    if !r.is_object() {
        return "?".to_owned();
    }
    let line = r["startLine"].as_i64().unwrap_or(0) + 1;
    let col = r["startCol"].as_i64().unwrap_or(0) + 1;
    let so = r["startOffset"].as_i64().unwrap_or(0);
    let eo = r["endOffset"].as_i64().unwrap_or(0);
    format!("{line}:{col}  ({so}…{eo})")
}

/// Proc summary flags.
fn flags(p: &Value) -> String {
    let mut f = Vec::new();
    if p["hasBarrier"].as_bool().unwrap_or(false) {
        f.push("barrier");
    }
    if p["hasUnknownCalls"].as_bool().unwrap_or(false) {
        f.push("unknown_calls");
    }
    if p["writesGlobal"].as_bool().unwrap_or(false) {
        f.push("writes_global");
    }
    if f.is_empty() {
        "—".to_owned()
    } else {
        f.join(", ")
    }
}

/// `{name#ver=lattice:type, …}` for a uses/defs map.
fn fmt_usedef(m: &Value) -> String {
    let Some(obj) = m.as_object() else {
        return "{}".to_owned();
    };
    let mut names: Vec<&String> = obj.keys().collect();
    names.sort();
    let parts: Vec<String> = names
        .iter()
        .map(|name| {
            let info = &obj[*name];
            if info.is_object() {
                let mut piece = format!("{name}#{}", jstr(&info["version"]));
                let lat = s(info, "lattice");
                if !lat.is_empty() {
                    piece.push('=');
                    piece.push_str(&lat);
                }
                let ty = s(info, "type");
                if !ty.is_empty() {
                    piece.push(':');
                    piece.push_str(&ty);
                }
                piece
            } else {
                format!("{name}#{}", jstr(info))
            }
        })
        .collect();
    format!("{{{}}}", parts.join(", "))
}

fn join_str_array(v: &Value) -> String {
    arr_to_strings(v).join(", ")
}

fn arr_to_strings(v: &Value) -> Vec<String> {
    v.as_array()
        .map_or_default(|a| a.iter().map(jstr).collect())
}

// --- Per-function tables ---

/// Build per-function nodes from a list, skipping functions with no
/// entries, with a fallback note. Shared by loops/types/intervals/rendered.
fn per_function<F>(
    funcs: &[Value],
    entries_key: &str,
    empty_note: &str,
    mut row: F,
) -> Vec<ViewNode>
where
    F: FnMut(&Value) -> ViewNode,
{
    let mut out = Vec::new();
    for f in funcs {
        let entries = arr(f, entries_key);
        if entries.is_empty() {
            continue;
        }
        let rows: Vec<ViewNode> = entries.iter().map(&mut row).collect();
        out.push(ViewNode::branch(
            format!("function {}", s(f, "name")),
            Vec::new(),
            rows,
            Some("cyan"),
        ));
    }
    if out.is_empty() {
        vec![ViewNode::note(empty_note, "dim")]
    } else {
        out
    }
}

fn unavailable_source_view() -> Vec<ViewNode> {
    vec![ViewNode::note(
        "This view is unavailable for this analysis",
        "dim",
    )]
}

/// Build the [`ViewNode`] forest for `view` from serialised `data`.
/// An unknown view id yields an empty forest.
#[must_use]
pub fn build_view(view: &str, data: &Value) -> Vec<ViewNode> {
    match view {
        "ir" => build_ir(data),
        "cfg" => build_cfg(arr(data, "cfgPreSsa"), false),
        "ssa" => {
            if data["cfgPostSsa"].is_array() {
                build_cfg(arr(data, "cfgPostSsa"), true)
            } else {
                unavailable_source_view()
            }
        }
        "worldSsa" => build_world_ssa(data),
        "aot" => build_aot(data),
        "dominators" => build_dominators(data),
        "sccp" => build_sccp(data),
        "liveness" => build_liveness(data),
        "semantic" => build_semantic(data),
        "loops" => build_loops(data),
        "types" => build_types(data),
        "intervals" => build_intervals(data),
        "bounds" => build_bounds(data),
        "dataflow" => build_dataflow(data),
        "interproc" => build_interproc(data),
        "unitScope" => build_unit_scope(data),
        "rendered" => build_rendered(data),
        "structuralIndex" => build_structural_index(data),
        "sourceMap" => build_source_map(data),
        "opt" => build_opt(data),
        "optimiserPasses" => build_optimiser_passes(data),
        "gvn" => build_gvn(data),
        "shimmer" => build_shimmer(data),
        "taint" => build_taint(data),
        "taintFacts" => build_taint_facts(data),
        "irules" => build_irules(data),
        "connectionScope" => build_connection_scope(data),
        "eventOrder" => build_event_order(data),
        "callouts" => build_callouts(data),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod retained_source_view_tests;

#[cfg(test)]
mod actual_cu_unavailable_view_tests;
