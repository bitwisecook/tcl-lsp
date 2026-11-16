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

//! The tree for the world-state SSA view.

use super::{ViewNode, arr, det, jstr, s, yn};
use serde_json::Value;

// --- World SSA ---

fn world_location_label(location: &Value) -> String {
    let domain = s(location, "domain");
    match s(location, "kind").as_str() {
        "scoped" => format!("{domain}: {}", jstr(&location["subject"])),
        "any" => "all mutable world state".to_owned(),
        other => format!("{domain} ({other})"),
    }
}

fn world_site_label(site: &Value) -> String {
    match s(site, "kind").as_str() {
        "cfg" => format!("block {} {}", s(site, "block"), jstr(&site["position"])),
        "edge" => format!(
            "edge {} → {} ({})",
            s(site, "predecessor"),
            s(site, "successor"),
            jstr(&site["origin"])
        ),
        "node" => format!("node {}", jstr(&site["path"])),
        other => other.to_owned(),
    }
}

#[allow(clippy::too_many_lines)] // One tree is intentionally assembled in view order.
pub(super) fn build_world_ssa(d: &Value) -> Vec<ViewNode> {
    arr(d, "worldSsa")
        .iter()
        .map(|function| {
            let availability = &function["availability"];
            let mut children = Vec::new();
            let locations: Vec<ViewNode> = arr(function, "locations")
                .iter()
                .map(|location| {
                    ViewNode::leaf(
                        world_location_label(location),
                        vec![
                            det("kind", s(location, "kind")),
                            det("domain", s(location, "domain")),
                            det("interpreter", jstr(&location["interpreter"])),
                            det("namespace", jstr(&location["namespace"])),
                            det("subject", jstr(&location["subject"])),
                        ],
                        None,
                    )
                })
                .collect();
            children.push(ViewNode::branch(
                format!("locations ({})", locations.len()),
                Vec::new(),
                locations,
                Some("cyan"),
            ));

            let operations: Vec<ViewNode> = arr(function, "operations")
                .iter()
                .map(|operation| {
                    let mut detail = vec![
                        det("kind", s(operation, "kind")),
                        det("version", s(operation, "version")),
                        det("site", world_site_label(&operation["site"])),
                        det("location", jstr(&operation["location"])),
                    ];
                    if operation["kind"] == "use" {
                        detail.push(det("reaching version", s(operation, "reachingVersion")));
                    }
                    if operation["kind"] == "phi" {
                        detail.push(det("includes initial", yn(&operation["includesInitial"])));
                    }
                    let incoming = arr(operation, "incoming")
                        .iter()
                        .map(|incoming| {
                            ViewNode::leaf(
                                format!(
                                    "block {}: v{}",
                                    s(incoming, "block"),
                                    s(incoming, "version")
                                ),
                                Vec::new(),
                                Some("dim"),
                            )
                        })
                        .collect();
                    ViewNode::branch(
                        format!(
                            "{} {} v{} @ {}",
                            s(operation, "kind"),
                            world_location_label(&operation["location"]),
                            s(operation, "version"),
                            world_site_label(&operation["site"])
                        ),
                        detail,
                        incoming,
                        match s(operation, "kind").as_str() {
                            "clobber" => Some("yellow"),
                            "phi" => Some("magenta"),
                            _ => None,
                        },
                    )
                })
                .collect();
            children.push(ViewNode::branch(
                format!("operations ({})", operations.len()),
                Vec::new(),
                operations,
                None,
            ));

            let invocations: Vec<ViewNode> = arr(function, "invocations")
                .iter()
                .map(|invoke| {
                    let transitions: Vec<ViewNode> = arr(invoke, "transitions")
                        .iter()
                        .map(|transition| {
                            let intents = arr(transition, "intents")
                                .iter()
                                .map(|intent| {
                                    ViewNode::leaf(
                                        format!(
                                            "{} {}",
                                            s(intent, "kind"),
                                            world_location_label(&intent["location"])
                                        ),
                                        vec![
                                            det("commit", s(intent, "commit")),
                                            det("location", jstr(&intent["location"])),
                                        ],
                                        None,
                                    )
                                })
                                .collect();
                            ViewNode::branch(
                                format!("{} ({})", s(transition, "kind"), s(transition, "commit")),
                                vec![det("abrupt transfer", s(transition, "abruptTransfer"))],
                                intents,
                                Some("green"),
                            )
                        })
                        .collect();
                    let proof = &invoke["proof"];
                    ViewNode::branch(
                        format!("{} invocation", s(invoke, "resolution")),
                        vec![
                            det("command", s(invoke, "command")),
                            det("completion", s(invoke, "completion")),
                            det("transition knowledge", s(invoke, "transitionKnowledge")),
                            det("result stability", jstr(&proof["resultStability"])),
                            det(
                                "dispatch dependencies",
                                jstr(&proof["dispatchDependencies"]),
                            ),
                            det("abstention", s(proof, "abstention")),
                        ],
                        transitions,
                        if s(invoke, "resolution") == "unresolved" {
                            Some("yellow")
                        } else {
                            None
                        },
                    )
                })
                .collect();
            children.push(ViewNode::branch(
                format!("invocation proof ({})", invocations.len()),
                Vec::new(),
                invocations,
                Some("blue"),
            ));

            ViewNode::branch(
                format!(
                    "function {} ({})",
                    s(function, "name"),
                    s(availability, "kind")
                ),
                vec![
                    det("availability", s(availability, "kind")),
                    det("executable IR", yn(&availability["hasExecutableIr"])),
                    det("reason", s(availability, "reasonKind")),
                ],
                children,
                if s(availability, "kind") == "available" {
                    None
                } else {
                    Some("yellow")
                },
            )
        })
        .collect()
}
