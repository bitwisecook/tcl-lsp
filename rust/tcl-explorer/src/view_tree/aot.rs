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

//! Trees for the `semantic` view and the AOT plan the WASM emitter selects over it.

use super::{ViewNode, arr, det, jstr, s};
use serde_json::Value;

pub(super) fn build_semantic(d: &Value) -> Vec<ViewNode> {
    let mut out = Vec::new();
    for f in arr(d, "semantic") {
        let mut children = Vec::new();
        for block in arr(f, "blocks") {
            let instructions = arr(block, "instructions")
                .iter()
                .map(|i| ViewNode::leaf(s(i, "kind"), Vec::new(), None))
                .collect();
            children.push(ViewNode::branch(
                format!("block {} ({})", s(block, "index"), s(block, "terminator")),
                Vec::new(),
                instructions,
                Some("blue"),
            ));
        }
        for decline in arr(f, "opaque") {
            children.push(ViewNode::leaf(
                format!("opaque region {}", s(decline, "range")),
                Vec::new(),
                Some("yellow"),
            ));
        }
        out.push(ViewNode::branch(
            format!("{} [{}]", s(f, "name"), s(f, "status")),
            vec![det("decline/proof", s(f, "decline"))],
            children,
            Some(if s(f, "status") == "available" {
                "green"
            } else {
                "yellow"
            }),
        ));
    }
    if out.is_empty() {
        vec![ViewNode::note("(no executable semantic data)", "dim")]
    } else {
        out
    }
}

// --- AOT plan ---

/// The AOT plan: what the WASM emitter selected, the guarded candidates each
/// invocation considered, and every premise the sealed native addition
/// rejected.
pub(super) fn build_aot(d: &Value) -> Vec<ViewNode> {
    let plan = &d["aot"];
    if !plan.is_object() {
        return Vec::new();
    }
    let mut children = Vec::new();
    let decline = &plan["semanticDecline"];
    if decline.is_object() {
        children.push(ViewNode::leaf(
            format!("semantic decline: {}", s(decline, "kind")),
            vec![det("reason", s(decline, "detailKind"))],
            Some("yellow"),
        ));
    }
    children.push(aot_regions(plan));
    children.push(aot_native_add(plan));
    let detail = [
        ("operation", s(plan, "operation")),
        ("region plan", s(plan, "regionPlanStatus")),
    ]
    .into_iter()
    .filter(|(_, value)| !value.is_empty())
    .map(|(key, value)| det(key, value))
    .collect();
    vec![ViewNode::branch(
        format!("plan: {}", s(plan, "kind")),
        detail,
        children,
        Some("cyan"),
    )]
}

fn aot_regions(plan: &Value) -> ViewNode {
    let regions: Vec<ViewNode> = arr(plan, "regions")
        .iter()
        .map(|region| {
            let candidates: Vec<ViewNode> = arr(region, "candidates")
                .iter()
                .map(aot_candidate)
                .collect();
            let operation = jstr(&region["operation"]["id"]);
            ViewNode::branch(
                format!(
                    "region {} · {}",
                    jstr(&region["node"]),
                    s(region, "selectedKind")
                ),
                if operation.is_empty() {
                    Vec::new()
                } else {
                    vec![det("operation", operation)]
                },
                candidates,
                None,
            )
        })
        .collect();
    ViewNode::branch(
        format!("regions ({})", regions.len()),
        Vec::new(),
        regions,
        Some("blue"),
    )
}

fn aot_candidate(candidate: &Value) -> ViewNode {
    if s(candidate, "decision") == "selected" {
        return ViewNode::leaf(
            format!("{}: selected", s(candidate, "kind")),
            Vec::new(),
            Some("green"),
        );
    }
    ViewNode::leaf(
        format!(
            "{}: declined, {}",
            s(candidate, "kind"),
            s(candidate, "reason")
        ),
        Vec::new(),
        Some("dim"),
    )
}

fn aot_native_add(plan: &Value) -> ViewNode {
    let native = &plan["nativeI64Add"];
    if native.is_object() {
        return ViewNode::leaf(
            "native i64 add: selected",
            vec![
                det("callee", s(native, "callee")),
                det("operands", jstr(&native["operands"])),
                det("boundary", jstr(&native["boundaryOperation"]["id"])),
                det(
                    "closed program statements",
                    s(native, "closedProgramStatements"),
                ),
            ],
            Some("green"),
        );
    }
    let premises: Vec<ViewNode> = arr(plan, "nativeDeclines")
        .iter()
        .map(aot_native_decline)
        .collect();
    ViewNode::branch(
        format!("native i64 add: declined ({} premises)", premises.len()),
        Vec::new(),
        premises,
        Some("yellow"),
    )
}

/// One rejected premise. The label carries the premise, its pass when it is
/// one, and the reason; the rows carry what the label does not.
fn aot_native_decline(decline: &Value) -> ViewNode {
    let detail = &decline["detail"];
    let subject = match detail.get("pass").and_then(Value::as_str) {
        Some(pass) => format!("{} {pass}", s(decline, "premise")),
        None => s(decline, "premise"),
    };
    let mut rows: Vec<(String, String)> = detail
        .as_object()
        .into_iter()
        .flatten()
        .filter(|(key, _)| key.as_str() != "pass")
        .map(|(key, value)| det(key, jstr(value)))
        .collect();
    rows.extend(
        arr(decline, "sites")
            .iter()
            .map(|site| det("site", aot_site_label(site))),
    );
    ViewNode::leaf(
        format!("{subject}: {}", s(decline, "reason")),
        rows,
        Some("yellow"),
    )
}

fn aot_site_label(site: &Value) -> String {
    let nested = match site["nestedArgument"].as_u64() {
        Some(argument) => format!(", argument {argument}"),
        None => String::new(),
    };
    format!(
        "{} block {} statement {}{nested}",
        s(site, "function"),
        s(site, "block"),
        s(site, "statementIndex")
    )
}
