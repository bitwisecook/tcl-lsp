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

//! Trees for the control-flow views: the pre-SSA and post-SSA graphs, dominators and loops.

use super::value_lattices::unreached_arm_leaf;
use super::{ViewNode, arr, det, fmt_usedef, join_str_array, jstr, per_function, pystr, rng, s};
use serde_json::Value;

// --- CFG (pre-/post-SSA) ---

fn term_label(t: &Value) -> String {
    if !t.is_object() {
        return "term <none>".to_owned();
    }
    match s(t, "type").as_str() {
        "goto" => format!("term goto {}", s(t, "target")),
        "branch" => format!(
            "term branch {} → {}/{}",
            s(t, "condition"),
            s(t, "trueTarget"),
            s(t, "falseTarget")
        ),
        "return" => {
            let v = s(t, "value");
            if v.is_empty() {
                "term return".to_owned()
            } else {
                format!("term return {v}")
            }
        }
        _ => "term".to_owned(),
    }
}

// One flat walk building the nested CFG view (functions -> blocks -> items);
// the per-level nesting is clearer inline than threaded through helpers.
#[allow(clippy::too_many_lines)]
pub(super) fn build_cfg(funcs: &[Value], post: bool) -> Vec<ViewNode> {
    let mut out = Vec::new();
    for f in funcs {
        let mut blocks = Vec::new();
        for b in arr(f, "blocks") {
            let mut items: Vec<ViewNode> = Vec::new();
            if post {
                for p in arr(b, "phis") {
                    let inc = p["incoming"].as_object().map_or_default(|o| {
                        o.iter()
                            .map(|(k, v)| format!("{k}:{}", jstr(v)))
                            .collect::<Vec<_>>()
                            .join(", ")
                    });
                    items.push(ViewNode::leaf(
                        format!("phi {}#{} ← {inc}", s(p, "name"), s(p, "version")),
                        vec![
                            det("type", {
                                let t = s(p, "type");
                                if t.is_empty() { "?".to_owned() } else { t }
                            }),
                            det("incoming", inc),
                        ],
                        Some("magenta"),
                    ));
                }
            }
            for st in arr(b, "statements") {
                let mut detail = vec![
                    det("summary", s(st, "summary")),
                    det("range", rng(&st["range"])),
                ];
                if post {
                    detail.push(det("uses", fmt_usedef(&st["uses"])));
                    detail.push(det("defs", fmt_usedef(&st["defs"])));
                }
                items.push(ViewNode::leaf(s(st, "summary"), detail, None).with_range(&st["range"]));
            }
            let term = &b["terminator"];
            items.push(
                ViewNode::leaf(
                    term_label(term),
                    vec![det("range", rng(&term["range"]))],
                    Some("blue"),
                )
                .with_range(&term["range"]),
            );
            let mut tags = String::new();
            if b["isEntry"].as_bool().unwrap_or(false) {
                tags.push_str(" [entry]");
            }
            if b["isUnreachable"].as_bool().unwrap_or(false) {
                tags.push_str(" [unreachable]");
            }
            blocks.push(ViewNode::branch(
                format!("block {}{tags}", s(b, "name")),
                Vec::new(),
                items,
                Some("bold"),
            ));
        }
        if post && f["analysis"].is_object() {
            let a = &f["analysis"];
            let mut asub = Vec::new();
            for br in arr(a, "constantBranches") {
                asub.push(if s(br, "kind") == "selected" {
                    unreached_arm_leaf(br)
                } else {
                    ViewNode::leaf(
                        format!(
                            "const branch {}: always {}",
                            s(br, "block"),
                            pystr(&br["value"])
                        ),
                        vec![
                            det("condition", s(br, "condition")),
                            det("take", s(br, "takenTarget")),
                        ],
                        Some("blue"),
                    )
                });
            }
            for ds in arr(a, "deadStores") {
                asub.push(ViewNode::leaf(
                    format!(
                        "dead store {} {}#{}",
                        s(ds, "block"),
                        s(ds, "variable"),
                        s(ds, "version")
                    ),
                    vec![
                        det("block", s(ds, "block")),
                        det("stmt index", s(ds, "stmtIndex")),
                    ],
                    Some("yellow"),
                ));
            }
            let unreachable = arr(a, "unreachableBlocks");
            if !unreachable.is_empty() {
                let joined = join_str_array(&a["unreachableBlocks"]);
                asub.push(ViewNode::leaf(
                    format!("unreachable: {joined}"),
                    vec![det("blocks", joined)],
                    Some("magenta"),
                ));
            }
            if let Some(types) = a["inferredTypes"].as_object() {
                for (name, ty) in types {
                    asub.push(ViewNode::leaf(
                        format!("{name}: {}", jstr(ty)),
                        vec![det("value", name.clone()), det("type", jstr(ty))],
                        Some("green"),
                    ));
                }
            }
            if !asub.is_empty() {
                blocks.push(ViewNode::branch("analysis", Vec::new(), asub, Some("bold")));
            }
        }
        out.push(ViewNode::branch(
            format!(
                "function {} (entry={}, {} blocks)",
                s(f, "name"),
                s(f, "entry"),
                s(f, "blockCount")
            ),
            Vec::new(),
            blocks,
            Some("cyan"),
        ));
    }
    if out.is_empty() {
        vec![ViewNode::note("(no functions)", "dim")]
    } else {
        out
    }
}

pub(super) fn build_dominators(d: &Value) -> Vec<ViewNode> {
    let mut out = Vec::new();
    for f in arr(d, "dominators") {
        let blocks = arr(f, "blocks")
            .iter()
            .map(|b| {
                ViewNode::leaf(
                    format!(
                        "{} idom={}",
                        s(b, "block"),
                        if b["idom"].is_null() {
                            "—".to_owned()
                        } else {
                            s(b, "idom")
                        }
                    ),
                    vec![
                        det("frontier", join_str_array(&b["frontier"])),
                        det("children", join_str_array(&b["children"])),
                    ],
                    Some("blue"),
                )
            })
            .collect();
        out.push(ViewNode::branch(
            format!("function {} (entry {})", s(f, "name"), s(f, "entry")),
            Vec::new(),
            blocks,
            Some("cyan"),
        ));
    }
    if out.is_empty() {
        vec![ViewNode::note("(no dominator data)", "dim")]
    } else {
        out
    }
}

pub(super) fn build_loops(d: &Value) -> Vec<ViewNode> {
    per_function(arr(d, "loops"), "loops", "(no natural loops)", |lp| {
        ViewNode::leaf(
            format!(
                "header {} (depth {}, {} blocks)",
                s(lp, "header"),
                s(lp, "depth"),
                s(lp, "blockCount")
            ),
            vec![
                det("header block", s(lp, "header")),
                det("nesting depth", s(lp, "depth")),
                det("blocks", join_str_array(&lp["blocks"])),
                det("latches", {
                    let l = join_str_array(&lp["latches"]);
                    if l.is_empty() { "—".to_owned() } else { l }
                }),
            ],
            Some("blue"),
        )
    })
}
