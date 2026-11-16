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

//! Trees for the iRules views: flow warnings, the connection scope and event order.

use super::{ViewNode, arr, det, join_str_array, rng, s, unavailable_source_view};
use serde_json::Value;

pub(super) fn build_connection_scope(d: &Value) -> Vec<ViewNode> {
    let scope = &d["connectionScope"];
    if !scope["available"].as_bool().unwrap_or(false) {
        return vec![ViewNode::note("(no connection scope)", "dim")];
    }
    let mut out = vec![ViewNode::leaf(
        "cross-event definitions/imports".to_owned(),
        vec![
            det("definitions", join_str_array(&scope["crossEventDefs"])),
            det("imports", join_str_array(&scope["crossEventImports"])),
            det(
                "racy static definitions",
                join_str_array(&scope["racyStaticDefs"]),
            ),
        ],
        Some("yellow"),
    )];
    for summary in arr(scope, "summaries") {
        out.push(ViewNode::leaf(
            format!("event {}", s(summary, "event")),
            vec![
                det("definitions", join_str_array(&summary["defs"])),
                det(
                    "uses before definition",
                    join_str_array(&summary["usesBeforeDef"]),
                ),
                det("unsets", join_str_array(&summary["unsets"])),
            ],
            Some("blue"),
        ));
    }
    out
}

pub(super) fn build_irules(d: &Value) -> Vec<ViewNode> {
    let mut out = if d["eventOrder"].is_array() && d["irulesFlow"].is_array() {
        Vec::new()
    } else {
        unavailable_source_view()
    };
    for e in arr(d, "eventOrder") {
        let eff =
            e["base_priority"].as_i64().unwrap_or(0) + e["priority_offset"].as_i64().unwrap_or(0);
        out.push(
            ViewNode::leaf(
                format!("{} eff={eff}", s(e, "event")),
                vec![
                    det("event", s(e, "event")),
                    det("effective priority", eff.to_string()),
                    det("base", s(e, "base_priority")),
                    det("offset", s(e, "priority_offset")),
                    det("multiplicity", {
                        let m = s(e, "multiplicity");
                        if m.is_empty() { "once".to_owned() } else { m }
                    }),
                    det("range", rng(&e["range"])),
                ],
                Some("blue"),
            )
            .with_range(&e["range"]),
        );
    }
    for w in arr(d, "irulesFlow") {
        out.push(
            ViewNode::leaf(
                format!("{} {}", s(w, "code"), s(w, "message")),
                vec![
                    det("code", s(w, "code")),
                    det("message", s(w, "message")),
                    det("range", rng(&w["range"])),
                ],
                Some("yellow"),
            )
            .with_range(&w["range"]),
        );
    }
    if out.is_empty() {
        vec![ViewNode::note("(no iRules events)", "dim")]
    } else {
        out
    }
}

pub(super) fn build_event_order(d: &Value) -> Vec<ViewNode> {
    if !d["eventOrder"].is_array() {
        return unavailable_source_view();
    }
    let mut events: Vec<&Value> = arr(d, "eventOrder").iter().collect();
    events.sort_by(|a, b| {
        let ea =
            a["base_priority"].as_i64().unwrap_or(0) + a["priority_offset"].as_i64().unwrap_or(0);
        let eb =
            b["base_priority"].as_i64().unwrap_or(0) + b["priority_offset"].as_i64().unwrap_or(0);
        eb.cmp(&ea).then_with(|| s(a, "event").cmp(&s(b, "event")))
    });
    let out: Vec<ViewNode> = events
        .iter()
        .map(|e| {
            let eff = e["base_priority"].as_i64().unwrap_or(0)
                + e["priority_offset"].as_i64().unwrap_or(0);
            let m = s(e, "multiplicity");
            let mult = if !m.is_empty() && m != "once" {
                format!(" [{m}]")
            } else {
                String::new()
            };
            ViewNode::leaf(
                format!("{} eff={eff}{mult}", s(e, "event")),
                vec![
                    det("event", s(e, "event")),
                    det("effective priority", eff.to_string()),
                    det("base priority", s(e, "base_priority")),
                    det("priority offset", s(e, "priority_offset")),
                    det(
                        "multiplicity",
                        if m.is_empty() { "once".to_owned() } else { m },
                    ),
                    det("range", rng(&e["range"])),
                ],
                Some("blue"),
            )
            .with_range(&e["range"])
        })
        .collect();
    if out.is_empty() {
        vec![ViewNode::note("(no event-order data)", "dim")]
    } else {
        out
    }
}
