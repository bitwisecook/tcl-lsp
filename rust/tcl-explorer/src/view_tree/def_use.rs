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

//! Trees for the def-use views: liveness and the data-flow graph.

use super::{ViewNode, arr, det, s, unavailable_source_view, yn};
use serde_json::Value;

pub(super) fn build_liveness(d: &Value) -> Vec<ViewNode> {
    if !d["liveness"].is_array() {
        return unavailable_source_view();
    }
    let mut out = Vec::new();
    for f in arr(d, "liveness") {
        let mut children = Vec::new();
        for chain in arr(f, "chains") {
            let uses = arr(chain, "uses");
            children.push(ViewNode::leaf(
                format!(
                    "{}#{} ({} uses)",
                    s(chain, "variable"),
                    s(chain, "version"),
                    uses.len()
                ),
                vec![
                    det("definition", s(&chain["definition"], "block")),
                    det("kind", s(&chain["definition"], "kind")),
                    det(
                        "use sites",
                        uses.iter()
                            .map(|u| format!("{}:{}", s(u, "block"), s(u, "kind")))
                            .collect::<Vec<_>>()
                            .join(", "),
                    ),
                ],
                Some(if uses.is_empty() { "yellow" } else { "green" }),
            ));
        }
        for dead in arr(f, "deadStores") {
            children.push(ViewNode::leaf(
                format!(
                    "dead store {} {}#{}",
                    s(dead, "block"),
                    s(dead, "variable"),
                    s(dead, "version")
                ),
                vec![det("statement", s(dead, "statementIndex"))],
                Some("yellow"),
            ));
        }
        out.push(ViewNode::branch(
            format!("function {}", s(f, "name")),
            Vec::new(),
            children,
            Some("cyan"),
        ));
    }
    if out.is_empty() {
        vec![ViewNode::note("(no liveness data)", "dim")]
    } else {
        out
    }
}

// --- Data flow / interprocedural ---

pub(super) fn build_dataflow(d: &Value) -> Vec<ViewNode> {
    let df = &d["dataflow"];
    if !df.is_object() || arr(df, "functions").is_empty() {
        return vec![ViewNode::note("(no data-flow information)", "dim")];
    }
    let mut out = Vec::new();
    for f in arr(df, "functions") {
        let mut nodes = Vec::new();
        for a in arr(f, "aliases") {
            nodes.push(ViewNode::leaf(
                format!("alias {} ↔ {}", s(a, "localName"), s(a, "targetName")),
                vec![
                    det("local", s(a, "localName")),
                    det("target", s(a, "targetName")),
                    det("reason", s(a, "reason")),
                ],
                Some("orange"),
            ));
        }
        for n in arr(f, "nodes") {
            let dead = if n["isDead"].as_bool().unwrap_or(false) {
                " (DEAD)"
            } else {
                ""
            };
            let label = format!(
                "{}#{} [{} in {}]{dead}",
                s(n, "name"),
                s(n, "version"),
                s(n, "defKind"),
                s(n, "block")
            );
            let mut detail = vec![
                det("ssa value", format!("{}#{}", s(n, "name"), s(n, "version"))),
                det("def kind", s(n, "defKind")),
                det("block", s(n, "block")),
                det("dead", yn(&n["isDead"])),
            ];
            if !n["useCount"].is_null() {
                detail.push(det("uses", s(n, "useCount")));
            }
            let lat = s(n, "lattice");
            if !lat.is_empty() {
                detail.push(det("lattice", lat));
            }
            let ti = s(n, "typeInfo");
            if !ti.is_empty() {
                detail.push(det("type", ti));
            }
            nodes.push(ViewNode::leaf(
                label,
                detail,
                Some(if n["isDead"].as_bool().unwrap_or(false) {
                    "yellow"
                } else {
                    "green"
                }),
            ));
        }
        let sm = &f["summary"];
        out.push(ViewNode::branch(
            format!(
                "function {} ({} defs, {} uses)",
                s(f, "name"),
                sm["totalDefs"].as_i64().unwrap_or(0),
                sm["totalUses"].as_i64().unwrap_or(0)
            ),
            Vec::new(),
            nodes,
            Some("cyan"),
        ));
    }
    out
}
