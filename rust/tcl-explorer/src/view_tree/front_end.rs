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

//! Trees for the front-end views: the IR, the structural index and the source map.

use super::{ViewNode, arr, arr_to_strings, det, rng, s};
use serde_json::Value;

// --- IR ---

fn ir_nodes(nodes: &[Value]) -> Vec<ViewNode> {
    nodes
        .iter()
        .map(|n| {
            let detail = vec![
                det("kind", s(n, "kind")),
                det("summary", s(n, "summary")),
                det("range", rng(&n["range"])),
            ];
            let children = arr(n, "children")
                .iter()
                .map(|c| {
                    ViewNode::branch(
                        format!("{}:", s(c, "label")),
                        Vec::new(),
                        ir_nodes(arr(c, "body")),
                        Some("blue"),
                    )
                })
                .collect();
            ViewNode::branch(s(n, "summary"), detail, children, None).with_range(&n["range"])
        })
        .collect()
}

pub(super) fn build_ir(d: &Value) -> Vec<ViewNode> {
    let ir = &d["ir"];
    let mut nodes = vec![ViewNode::branch(
        "top-level",
        Vec::new(),
        ir_nodes(arr(ir, "topLevel")),
        Some("cyan"),
    )];
    if let Some(procs) = ir["procedures"].as_object() {
        let mut rows: Vec<_> = procs.values().collect();
        rows.sort_by(|left, right| {
            s(left, "name")
                .cmp(&s(right, "name"))
                .then(left["bodyId"].as_u64().cmp(&right["bodyId"].as_u64()))
        });
        for proc in rows {
            let display = s(proc, "name");
            let params = arr_to_strings(&proc["params"]).join(" ");
            let header = format!("{display} {{{params}}}");
            nodes.push(
                ViewNode::branch(
                    header,
                    vec![
                        det("body ID", proc["bodyId"].to_string()),
                        det("range", rng(&proc["range"])),
                    ],
                    ir_nodes(arr(proc, "body")),
                    Some("cyan"),
                )
                .with_range(&proc["range"]),
            );
        }
    }
    nodes
}

pub(super) fn build_structural_index(d: &Value) -> Vec<ViewNode> {
    let si = &d["structuralIndex"];
    if !si.is_object() {
        return vec![ViewNode::note("(structural index unavailable)", "dim")];
    }
    let mut roots = Vec::new();
    match si["scriptComplete"].as_bool() {
        Some(complete) => roots.push(ViewNode::leaf(
            format!("script complete: {complete}"),
            Vec::new(),
            Some(if complete { "green" } else { "yellow" }),
        )),
        None => roots.push(ViewNode::note("script completeness unavailable", "dim")),
    }

    let boundaries = arr(si, "commandBoundaries");
    let cb_children: Vec<ViewNode> = boundaries
        .iter()
        .map(|b| {
            ViewNode::leaf(
                format!(
                    "offset {} ({}:{})",
                    s(b, "offset"),
                    b["line"].as_i64().unwrap_or(0) + 1,
                    b["col"].as_i64().unwrap_or(0) + 1
                ),
                Vec::new(),
                None,
            )
        })
        .collect();
    roots.push(ViewNode::branch(
        format!("command boundaries ({})", boundaries.len()),
        Vec::new(),
        cb_children,
        Some("cyan"),
    ));

    for (key, label) in [("brackets", "brackets [ ]"), ("braces", "braces { }")] {
        let group = &si[key];
        let inert = arr(group, "inertSpans");
        let children: Vec<ViewNode> = inert
            .iter()
            .map(|span| {
                let terminated = span["terminated"].as_bool().unwrap_or(false);
                ViewNode::leaf(
                    format!(
                        "inert {}…{} {}",
                        s(span, "start"),
                        s(span, "end"),
                        if terminated {
                            "(terminated)"
                        } else {
                            "(unterminated)"
                        }
                    ),
                    vec![det("text", s(span, "text"))],
                    Some(if terminated { "dim" } else { "red" }),
                )
            })
            .collect();
        roots.push(ViewNode::branch(
            format!(
                "{label}: {} unterminated, {} inert span(s)",
                s(group, "unterminated"),
                inert.len()
            ),
            vec![det("structural events", s(group, "structuralEvents"))],
            children,
            None,
        ));
    }
    roots
}

pub(super) fn build_source_map(d: &Value) -> Vec<ViewNode> {
    let sm = &d["sourceMap"];
    if !sm.is_object() {
        return vec![ViewNode::note("(source map unavailable)", "dim")];
    }
    let mut roots = vec![ViewNode::leaf(
        format!(
            "{} bytes, {} line(s)",
            s(sm, "byteLength"),
            s(sm, "lineCount")
        ),
        Vec::new(),
        Some("cyan"),
    )];
    for line in arr(sm, "lines") {
        let n = line["line"].as_i64().unwrap_or(0) + 1;
        roots.push(ViewNode::leaf(
            format!("{n}: {}", s(line, "text")),
            vec![
                det(
                    "byte range",
                    format!(
                        "{}…{} ({} bytes)",
                        s(line, "start"),
                        s(line, "end"),
                        s(line, "length")
                    ),
                ),
                det("line", n.to_string()),
            ],
            None,
        ));
    }
    roots
}
