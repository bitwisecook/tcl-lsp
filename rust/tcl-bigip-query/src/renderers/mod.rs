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

//! Pluggable output renderers for the query DSL.
//!
//! Covers the renderer registry plus the three built-in renderer modules
//! (`mermaid`, `gantt`, `ascii-blocks`).
//!
//! Renderers turn the flattened evaluator value list into a human-readable
//! string. They sit alongside the built-in output modes in
//! [`crate::output`] and are reachable from the CLI via
//! `f5 query --render NAME` (with `--render-opt KEY=VALUE` per-renderer
//! options).
//!
//! Renderers are not self-registering; a small static registry is built on
//! first use. The renderer contract is a callable
//! `(values, &opts) -> Result<String, QueryError>` that parses and
//! validates its own options and raises [`QueryError::Renderer`] for
//! caller-visible problems.

use std::collections::BTreeMap;

use crate::errors::QueryError;
use crate::value::Value;

mod ascii_blocks;
mod gantt;
mod mermaid;

/// The implementation signature shared by every renderer.
type RenderFn = fn(&[Value], &BTreeMap<String, String>) -> Result<String, QueryError>;

/// Metadata for one registered renderer.
///
/// `accepts` is a short free-text shape hint shown by `--help-renderers`; the
/// registry does not enforce it.
pub struct RendererSpec {
    pub name: &'static str,
    pub summary: &'static str,
    pub accepts: &'static str,
    pub details: &'static str,
    pub impl_fn: RenderFn,
}

/// The static catalogue of built-in renderers, sorted by name.
///
/// Sorted at definition time so [`list_renderers`] and the registered-names
/// list in error messages is sorted by name.
const REGISTRY: &[RendererSpec] = &[
    RendererSpec {
        name: "ascii-blocks",
        summary: "Unicode line-art block diagram of a nested {title, rows} tree.",
        accepts: "dict {title, rows} or list of such dicts",
        details: concat!(
            "Each dict becomes a bordered box; nested ``rows`` (synonyms: ",
            "``children``, ``items``) become indented child boxes joined to ",
            "the parent with a tree connector.  Picks the box-drawing style ",
            "with --render-opt style=rounded|square|ascii (default rounded)."
        ),
        impl_fn: ascii_blocks::render,
    },
    RendererSpec {
        name: "gantt",
        summary: "ASCII Gantt-style timeline of (timestamp, label, state) transitions.",
        accepts: "rows of (timestamp, label, state) — TSV string, 3-tuples, or dicts",
        details: concat!(
            "Rows may be a TSV string, 3-tuples, or dicts.  ",
            "Renders one row per distinct label, with ``v`` for the DOWN ",
            "transition, ``^`` for UP, and ``#`` for the spans the label was ",
            "marked down.  Use --render-opt unit-minutes=N (a divisor of 60) ",
            "to change the time resolution."
        ),
        impl_fn: gantt::render,
    },
    RendererSpec {
        name: "mermaid",
        summary: "Mermaid flowchart — BIG-IP object graph (for ObjectRef streams) or a generic chain.",
        accepts: "stream of ObjectRef (for the BIG-IP graph mode) or any ordered stream (chain mode)",
        details: concat!(
            "ObjectRef-mode reuses the same engine as ``f5 graph --format mermaid``: ",
            "walks the reference graph from each ObjectRef seed in the query result, ",
            "limited by --render-opt max-depth=N and optionally reversed via ",
            "--render-opt reverse=true.  Generic mode renders the values as a left-to-right ",
            "chain of nodes — handy for ad-hoc ``{stage, next}`` projections."
        ),
        impl_fn: mermaid::render,
    },
];

/// Return the spec for *name*, or `None` if no such renderer.
#[must_use]
pub fn lookup(name: &str) -> Option<&'static RendererSpec> {
    REGISTRY.iter().find(|s| s.name == name)
}

/// Return every registered renderer, sorted by name. The static [`REGISTRY`]
/// is already sorted.
#[must_use]
pub fn list_renderers() -> &'static [RendererSpec] {
    REGISTRY
}

/// Look up *name* and dispatch the renderer with *values* and *opts*.
///
/// # Errors
///
/// Returns [`QueryError::Renderer`] if *name* is not registered, or whatever
/// error the renderer itself raises for bad input / options.
pub fn render(
    name: &str,
    values: &[Value],
    opts: &BTreeMap<String, String>,
) -> Result<String, QueryError> {
    if let Some(spec) = lookup(name) {
        (spec.impl_fn)(values, opts)
    } else {
        let names: Vec<&str> = list_renderers().iter().map(|s| s.name).collect();
        Err(QueryError::Renderer(format!(
            "unknown renderer '{name}' (registered: {})",
            names.join(", ")
        )))
    }
}
