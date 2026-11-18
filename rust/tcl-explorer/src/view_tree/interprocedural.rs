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

//! Trees for the `interproc` and `unitScope` views.

use super::{ViewNode, arr, det, flags, join_str_array, s, yn};
use serde_json::Value;

pub(super) fn build_interproc(d: &Value) -> Vec<ViewNode> {
    let mut out = Vec::new();
    for p in arr(d, "interprocedural") {
        if s(p, "kind") == "method" {
            let mut detail = vec![
                det("method kind", {
                    let m = s(p, "methodKind");
                    if m.is_empty() { "method".to_owned() } else { m }
                }),
                det("pure", yn(&p["pure"])),
                det("calls", {
                    let c = join_str_array(&p["calls"]);
                    if c.is_empty() { "—".to_owned() } else { c }
                }),
            ];
            let wiv = arr(p, "writesInstanceVars");
            if !wiv.is_empty() {
                detail.push(det(
                    "writes instance vars",
                    join_str_array(&p["writesInstanceVars"]),
                ));
            }
            detail.push(det("flags", flags(p)));
            out.push(ViewNode::leaf(
                format!("{} (method)", s(p, "name")),
                detail,
                Some("cyan"),
            ));
        } else {
            let mut detail = vec![
                det("arity", s(p, "arity")),
                det("pure", yn(&p["pure"])),
                det("foldable", yn(&p["foldable"])),
                det("return shape", s(p, "returnShape")),
                det("calls", {
                    let c = join_str_array(&p["calls"]);
                    if c.is_empty() { "—".to_owned() } else { c }
                }),
            ];
            // Advice only: what every call in this file passes. Shown only
            // when the calls agree.
            let passes = join_str_array(&p["everyCallPasses"]);
            if !passes.is_empty() {
                detail.push(det("every call in this file passes", passes));
            }
            detail.push(det(
                "transfer",
                p["transfer"].as_str().map_or_else(
                    || "none — a call is a barrier".to_owned(),
                    ToOwned::to_owned,
                ),
            ));
            detail.push(det("flags", flags(p)));
            out.push(ViewNode::leaf(
                format!("{} arity={}", s(p, "name"), s(p, "arity")),
                detail,
                Some("cyan"),
            ));
        }
    }
    if out.is_empty() {
        vec![ViewNode::note("(no procedures)", "dim")]
    } else {
        out
    }
}

/// Why the interprocedural constant seed fired (or declined) for each
/// procedure: the registry-declared unit boundaries the file crosses, whether
/// the host supplied a cross-file view, and the merged call-site evidence
/// per callee.
pub(super) fn build_unit_scope(d: &Value) -> Vec<ViewNode> {
    let scope = &d["unitScope"];
    let boundaries = join_str_array(&scope["boundaries"]);
    let mut out = vec![ViewNode::leaf(
        "unit".to_owned(),
        vec![
            det(
                "registry boundaries",
                if boundaries.is_empty() {
                    "none".to_owned()
                } else {
                    boundaries
                },
            ),
            det("cross-file view", yn(&scope["hasCrossFileEvidence"])),
            det("seeding", s(scope, "seeding")),
        ],
        Some("cyan"),
    )];
    for callee in arr(scope, "callees") {
        let detail = arr(callee, "positions")
            .iter()
            .map(|p| det("arg", format!("{}: {}", s(p, "index"), s(p, "verdict"))))
            .chain(std::iter::once(det(
                "argument counts",
                join_str_array(&callee["argCounts"]),
            )))
            .collect();
        out.push(ViewNode::leaf(s(callee, "name"), detail, Some("green")));
    }
    if out.len() == 1 {
        out.push(ViewNode::note("(no resolvable call sites)", "dim"));
    }
    out
}
