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

//! Trees for the optimiser's views: rewrites, per-pass output and redundancy hints.

use super::{ViewNode, arr, det, rng, s, unavailable_source_view};
use serde_json::Value;

// --- Optimiser / GVN / shimmer / taint / iRules / callouts ---

/// One optimisation's label and details, shared by the flat rewrite list and
/// the per-pass pipeline.
///
/// A `hintOnly` entry carries no replacement: its range spans the whole
/// consuming statement, so the literal is never a valid edit for it.
/// Rendering the usual `code message -> replacement` for one shows advice as a
/// rewrite to the empty string — a deletion — which is the opposite of what it
/// means, so the arrow is dropped and the row says what it is.
pub(super) fn opt_leaf(o: &Value) -> ViewNode {
    let hint_only = o.get("hintOnly").and_then(Value::as_bool).unwrap_or(false);
    let (tail, replacement) = if hint_only {
        (" (hint only)".to_owned(), "(none — hint only)".to_owned())
    } else {
        (
            format!(" \u{2192} {}", s(o, "replacement")),
            s(o, "replacement"),
        )
    };
    ViewNode::leaf(
        format!("{} {}{tail}", s(o, "code"), s(o, "message")),
        vec![
            det("code", s(o, "code")),
            det("message", s(o, "message")),
            det("replacement", replacement),
            det("range", rng(&o["range"])),
        ],
        Some(if hint_only { "yellow" } else { "green" }),
    )
    .with_range(&o["range"])
}

pub(super) fn build_opt(d: &Value) -> Vec<ViewNode> {
    if !d["optimisations"].is_array() {
        return unavailable_source_view();
    }
    let out: Vec<ViewNode> = arr(d, "optimisations").iter().map(opt_leaf).collect();
    if out.is_empty() {
        vec![ViewNode::note("(no optimiser rewrites)", "dim")]
    } else {
        out
    }
}

pub(super) fn build_optimiser_passes(d: &Value) -> Vec<ViewNode> {
    if !d["optimiserPasses"].is_array() {
        return unavailable_source_view();
    }
    let out: Vec<ViewNode> = arr(d, "optimiserPasses")
        .iter()
        .map(|p| {
            let opts: Vec<ViewNode> = arr(p, "optimisations").iter().map(opt_leaf).collect();
            let count = s(p, "count");
            ViewNode::branch(
                format!("{} ({count})", s(p, "label")),
                vec![det("pass", s(p, "id")), det("rewrites", count)],
                opts,
                Some(if arr(p, "optimisations").is_empty() {
                    "dim"
                } else {
                    "green"
                }),
            )
        })
        .collect();
    if out.is_empty() {
        vec![ViewNode::note("(optimiser pipeline unavailable)", "dim")]
    } else {
        out
    }
}

pub(super) fn build_gvn(d: &Value) -> Vec<ViewNode> {
    if !d["gvn"].is_array() {
        return unavailable_source_view();
    }
    let out: Vec<ViewNode> = arr(d, "gvn")
        .iter()
        .map(|w| {
            ViewNode::leaf(
                format!("{} {}", s(w, "code"), s(w, "expression")),
                vec![
                    det("code", s(w, "code")),
                    det("message", s(w, "message")),
                    det("expression", s(w, "expression")),
                    det("first seen", rng(&w["firstRange"])),
                    det("range", rng(&w["range"])),
                ],
                Some("green"),
            )
            .with_range(&w["range"])
        })
        .collect();
    if out.is_empty() {
        vec![ViewNode::note("(no redundant computations)", "dim")]
    } else {
        out
    }
}
