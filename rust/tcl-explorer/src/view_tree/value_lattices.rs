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

//! Trees for the per-value lattice views: SCCP, types, intervals, rendered properties and taint facts.

use super::{ViewNode, arr, det, fmt_bound, join_str_array, per_function, pystr, s};
use serde_json::Value;

/// One selection record's leaf: the arm each member selects, the
/// arm whose body runs, and the statement's line.
fn selection_leaf(selection: &Value) -> ViewNode {
    ViewNode::leaf(
        format!("selection: {}", join_str_array(&selection["selected"])),
        vec![
            det("bodies", join_str_array(&selection["bodies"])),
            det(
                "line",
                selection["range"]["startLine"]
                    .as_u64()
                    .map_or_else(|| "?".to_owned(), |line| (line + 1).to_string()),
            ),
        ],
        Some("blue"),
    )
}

/// The leaf of a `Selected` branch fact: an arm of an opaque `switch` whose
/// body no member of the subject runs — the pattern, and the block holding
/// the statement.
pub(super) fn unreached_arm_leaf(fact: &Value) -> ViewNode {
    ViewNode::leaf(
        format!("arm never selected: {}", s(fact, "condition")),
        vec![det("block", s(fact, "block"))],
        Some("blue"),
    )
}

/// One statement's route record of the SCCP view: the answer, the line, and
/// what the statement stores on each completion path.
fn route_leaf(route: &Value) -> ViewNode {
    let answer = s(route, "answer");
    let colour = if answer.starts_with("evaluated") {
        "green"
    } else if answer.starts_with("pending") {
        "yellow"
    } else {
        "magenta"
    };
    let mut details = vec![
        det("answer", answer),
        det(
            "line",
            route["range"]["startLine"]
                .as_u64()
                .map_or_else(|| "?".to_owned(), |line| (line + 1).to_string()),
        ),
    ];
    for path in arr(route, "paths") {
        let stores = arr(path, "stores")
            .iter()
            .filter_map(|store| store.as_str())
            .collect::<Vec<_>>();
        details.push(det(
            &format!("path {}", s(path, "completion")),
            if stores.is_empty() {
                "no stores".to_owned()
            } else {
                stores.join("; ")
            },
        ));
    }
    ViewNode::leaf(
        format!("route {}: {}", s(route, "command"), s(route, "route")),
        details,
        Some(colour),
    )
}

/// One edge refinement of the SCCP view: the place, the fact, its version,
/// edge and domain.
fn refinement_leaf(refinement: &Value) -> ViewNode {
    ViewNode::leaf(
        format!(
            "refinement {} = {}",
            s(refinement, "variable"),
            s(refinement, "fact")
        ),
        vec![
            det("version", s(refinement, "version")),
            det(
                "edge",
                format!("{} → {}", s(refinement, "from"), s(refinement, "to")),
            ),
            det("domain", s(refinement, "domain")),
        ],
        Some("magenta"),
    )
}

/// One loop the solver ran to its exit: its passes, how it left, the block it
/// leaves to and each value it published.
fn enumerated_loop_leaf(record: &Value) -> ViewNode {
    let mut details = vec![det("exit block", s(record, "exitBlock"))];
    details.extend(
        arr(record, "published")
            .iter()
            .map(|value| det(&s(value, "variable"), s(value, "lattice"))),
    );
    ViewNode::leaf(
        format!(
            "enumerated loop: {} iterations, {}",
            pystr(&record["iterations"]),
            s(record, "exit")
        ),
        details,
        Some("magenta"),
    )
}

pub(super) fn build_sccp(d: &Value) -> Vec<ViewNode> {
    let mut out = Vec::new();
    for f in arr(d, "sccp") {
        let mut children = Vec::new();
        for value in arr(f, "values") {
            let detail = value["type"]
                .as_str()
                .map_or_default(|folded| vec![det("type", folded)]);
            children.push(ViewNode::leaf(
                format!(
                    "{}#{} = {}",
                    s(value, "variable"),
                    s(value, "version"),
                    s(value, "lattice")
                ),
                detail,
                Some("green"),
            ));
        }
        children.push(ViewNode::leaf(
            format!(
                "executable blocks: {}",
                join_str_array(&f["executableBlocks"])
            ),
            vec![det(
                "edges",
                f["executableEdges"]
                    .as_array()
                    .map_or(0, Vec::len)
                    .to_string(),
            )],
            Some("cyan"),
        ));
        let tally = &f["routeTally"];
        children.push(ViewNode::leaf(
            format!(
                "routes entered: direct {} · expression {} · implementation {}",
                tally["direct"].as_u64().unwrap_or(0),
                tally["expression"].as_u64().unwrap_or(0),
                tally["implementation"].as_u64().unwrap_or(0),
            ),
            Vec::new(),
            Some("cyan"),
        ));
        for branch in arr(f, "constantBranches") {
            children.push(if s(branch, "kind") == "selected" {
                unreached_arm_leaf(branch)
            } else {
                ViewNode::leaf(
                    format!("branch {}: {}", s(branch, "block"), pystr(&branch["value"])),
                    vec![
                        det("condition", s(branch, "condition")),
                        det("take", s(branch, "takenTarget")),
                    ],
                    Some("blue"),
                )
            });
        }
        children.extend(arr(f, "refinements").iter().map(refinement_leaf));
        children.extend(arr(f, "enumeratedLoops").iter().map(enumerated_loop_leaf));
        children.extend(arr(f, "selections").iter().map(selection_leaf));
        children.extend(arr(f, "routes").iter().map(route_leaf));
        out.push(ViewNode::branch(
            format!("function {}", s(f, "name")),
            Vec::new(),
            children,
            Some("cyan"),
        ));
    }
    if out.is_empty() {
        vec![ViewNode::note("(no SCCP data)", "dim")]
    } else {
        out
    }
}

pub(super) fn build_types(d: &Value) -> Vec<ViewNode> {
    per_function(arr(d, "types"), "entries", "(no inferred types)", |e| {
        let var = s(e, "variable");
        let label = if var.starts_with('(') {
            format!("{var}: {}", s(e, "type"))
        } else {
            format!("{var}#{}: {}", s(e, "version"), s(e, "type"))
        };
        ViewNode::leaf(
            label,
            vec![
                det("variable", var),
                det("version", s(e, "version")),
                det("kind", s(e, "kind")),
                det("type", s(e, "type")),
            ],
            None,
        )
    })
}

pub(super) fn build_intervals(d: &Value) -> Vec<ViewNode> {
    per_function(arr(d, "intervals"), "entries", "(no bounded ranges)", |e| {
        let lo = fmt_bound(&e["lo"], false);
        let hi = fmt_bound(&e["hi"], true);
        ViewNode::leaf(
            format!("{}#{}: [{lo}, {hi}]", s(e, "variable"), s(e, "version")),
            vec![
                det("variable", s(e, "variable")),
                det("version", s(e, "version")),
                det("lower bound", lo),
                det("upper bound", hi),
            ],
            None,
        )
    })
}

pub(super) fn build_rendered(d: &Value) -> Vec<ViewNode> {
    per_function(
        arr(d, "renderedProperties"),
        "entries",
        "(no rendered-property flags)",
        |e| {
            ViewNode::leaf(
                format!("{}#{}", s(e, "variable"), s(e, "version")),
                vec![
                    det("variable", s(e, "variable")),
                    det("version", s(e, "version")),
                    det("may", {
                        let m = join_str_array(&e["may"]);
                        if m.is_empty() { "—".to_owned() } else { m }
                    }),
                    det("must", {
                        let m = join_str_array(&e["must"]);
                        if m.is_empty() { "—".to_owned() } else { m }
                    }),
                ],
                None,
            )
        },
    )
}

pub(super) fn build_taint_facts(d: &Value) -> Vec<ViewNode> {
    let mut out = Vec::new();
    for f in arr(d, "taintFacts") {
        let entries = arr(f, "entries")
            .iter()
            .map(|entry| {
                ViewNode::leaf(
                    format!(
                        "{}#{}: {}",
                        s(entry, "variable"),
                        s(entry, "version"),
                        s(entry, "taint")
                    ),
                    Vec::new(),
                    None,
                )
            })
            .collect();
        out.push(ViewNode::branch(
            format!("function {}", s(f, "name")),
            Vec::new(),
            entries,
            Some("cyan"),
        ));
    }
    if out.is_empty() {
        vec![ViewNode::note("(no taint lattice data)", "dim")]
    } else {
        out
    }
}
