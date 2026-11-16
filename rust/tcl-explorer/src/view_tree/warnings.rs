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

//! Trees for the views that list warnings: bounds, shimmer, taint sinks and the source callouts.

use super::{ViewNode, arr, det, fmt_bound, rng, s, unavailable_source_view};
use serde_json::Value;

pub(super) fn build_bounds(d: &Value) -> Vec<ViewNode> {
    if !d["bounds"].is_array() {
        return unavailable_source_view();
    }
    let mut out = Vec::new();
    for f in arr(d, "bounds") {
        let findings = arr(f, "findings");
        let divzero = arr(f, "divzero");
        if findings.is_empty() && divzero.is_empty() {
            continue;
        }
        let mut items = Vec::new();
        for b in findings {
            let lo = fmt_bound(&b["lo"], false);
            let hi = fmt_bound(&b["hi"], true);
            items.push(ViewNode::leaf(
                format!(
                    "{} {} ${} in [{lo}, {hi}] vs length {}",
                    s(b, "code"),
                    s(b, "command"),
                    s(b, "indexVar"),
                    s(b, "length")
                ),
                vec![
                    det("code", s(b, "code")),
                    det("command", s(b, "command")),
                    det("index var", format!("${}", s(b, "indexVar"))),
                    det("index range", format!("[{lo}, {hi}]")),
                    det("container length", s(b, "length")),
                    det("reason", s(b, "reason")),
                ],
                Some("yellow"),
            ));
        }
        for dz in divzero {
            items.push(ViewNode::leaf(
                format!("{} '{}' divisor provably 0", s(dz, "code"), s(dz, "op")),
                vec![
                    det("code", s(dz, "code")),
                    det("operator", s(dz, "op")),
                    det("reason", "divisor interval is exactly [0, 0]"),
                ],
                Some("yellow"),
            ));
        }
        out.push(ViewNode::branch(
            format!("function {}", s(f, "name")),
            Vec::new(),
            items,
            Some("cyan"),
        ));
    }
    if out.is_empty() {
        vec![ViewNode::note(
            "(no provable out-of-range / divide-by-zero)",
            "dim",
        )]
    } else {
        out
    }
}

pub(super) fn build_shimmer(d: &Value) -> Vec<ViewNode> {
    if !d["shimmer"].is_array() {
        return unavailable_source_view();
    }
    let out: Vec<ViewNode> = arr(d, "shimmer")
        .iter()
        .map(|w| {
            ViewNode::leaf(
                format!("{} {}", s(w, "code"), s(w, "message")),
                vec![
                    det("code", s(w, "code")),
                    det("severity", {
                        let sv = s(w, "severity");
                        if sv.is_empty() {
                            "warning".to_owned()
                        } else {
                            sv
                        }
                    }),
                    det("message", s(w, "message")),
                    det("range", rng(&w["range"])),
                ],
                Some("yellow"),
            )
            .with_range(&w["range"])
        })
        .collect();
    if out.is_empty() {
        vec![ViewNode::note("(no shimmer warnings)", "dim")]
    } else {
        out
    }
}

pub(super) fn build_taint(d: &Value) -> Vec<ViewNode> {
    if !d["taintWarnings"].is_array() {
        return unavailable_source_view();
    }
    let mut out = Vec::new();
    for w in arr(d, "taintWarnings") {
        let mut detail = vec![
            det("code", s(w, "code")),
            det("severity", {
                let sv = s(w, "severity");
                if sv.is_empty() {
                    "warning".to_owned()
                } else {
                    sv
                }
            }),
            det("message", s(w, "message")),
        ];
        let var = s(w, "variable");
        if !var.is_empty() {
            detail.push(det("variable", var));
        }
        let sink = s(w, "sinkCommand");
        if !sink.is_empty() {
            detail.push(det("sink command", sink));
        }
        detail.push(det("range", rng(&w["range"])));
        out.push(
            ViewNode::leaf(
                format!("{} {}", s(w, "code"), s(w, "message")),
                detail,
                Some("red"),
            )
            .with_range(&w["range"]),
        );
    }
    if out.is_empty() {
        vec![ViewNode::note("(no tainted data flows)", "dim")]
    } else {
        out
    }
}

pub(super) fn build_callouts(d: &Value) -> Vec<ViewNode> {
    if !d["annotations"].is_array() {
        return unavailable_source_view();
    }
    let out: Vec<ViewNode> = arr(d, "annotations")
        .iter()
        .map(|a| {
            ViewNode::leaf(
                format!("{}: {}", s(a, "kind"), s(a, "label")),
                vec![
                    det("kind", s(a, "kind")),
                    det("severity", s(a, "severity")),
                    det("label", s(a, "label")),
                    det("range", rng(&a["range"])),
                ],
                None,
            )
            .with_range(&a["range"])
        })
        .collect();
    if out.is_empty() {
        vec![ViewNode::note("(no salient annotations)", "dim")]
    } else {
        out
    }
}
