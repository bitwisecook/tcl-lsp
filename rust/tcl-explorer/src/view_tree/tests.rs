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

use super::*;
use crate::{run_pipeline, serialise_result};

fn data(src: &str) -> Value {
    serialise_result(&run_pipeline(src, "tcl8.6"))
}

/// A hint is advice, not an edit, and the row must not read as one.
///
/// A `hintOnly` optimisation carries no replacement — its range spans the
/// whole consuming statement, so the literal is never a valid edit for
/// it. Rendering the usual `code message -> replacement` gives
/// `-> ` with nothing after it, which reads as a rewrite to the empty
/// string: a deletion, and the opposite of what the entry means.
#[test]
fn a_hint_only_optimisation_is_not_rendered_as_a_rewrite() {
    let d = data("set n 7\nputs \"n=$n\"\n");
    let hints: Vec<&Value> = d["optimisations"]
        .as_array()
        .expect("the opt payload is a list")
        .iter()
        .filter(|o| o.get("hintOnly").and_then(Value::as_bool).unwrap_or(false))
        .collect();
    assert!(
        !hints.is_empty(),
        "the fixture must produce a hint-only entry, else this proves nothing: {:?}",
        d["optimisations"],
    );

    for o in hints {
        let node = opt_leaf(o);
        assert!(
            node.label.ends_with("(hint only)"),
            "a hint must say so: {:?}",
            node.label,
        );
        assert!(
            !node.label.contains('\u{2192}'),
            "and must not carry the rewrite arrow: {:?}",
            node.label,
        );
        let replacement = node
            .detail
            .iter()
            .find(|(k, _)| k == "replacement")
            .map(|(_, v)| v.as_str())
            .expect("every optimisation row details its replacement");
        assert!(
            !replacement.is_empty(),
            "the detail row must explain the absence rather than show a blank edit",
        );
    }
}

/// The ordinary case still renders as a rewrite.
#[test]
fn an_applicable_optimisation_still_shows_its_replacement() {
    let o = serde_json::json!({
        "code": "O100",
        "message": "Inline the constant",
        "replacement": "7",
        "hintOnly": false,
        "range": Value::Null,
    });
    let node = opt_leaf(&o);
    assert!(node.label.contains("\u{2192} 7"), "{:?}", node.label);
    assert!(!node.label.contains("hint only"), "{:?}", node.label);
}

/// A row that shows a range must also *carry* it.
///
/// The detail table's `range` row is a formatted string; an editor host
/// navigating to it would otherwise have to parse `line:col  (start…end)`
/// back out. These are the views a host navigates from.
#[test]
fn rows_that_display_a_range_also_carry_the_structured_one() {
    let d = data("proc greet {who} {\n    puts \"hi $who\"\n}\ngreet world\n");
    for view in ["ir", "cfg", "ssa"] {
        let nodes = build_view(view, &d);
        assert!(!nodes.is_empty(), "{view} produced no rows");
        // `rng` renders a missing span as "?" — a terminator-less block
        // shows a range row with nothing behind it, and carrying `None`
        // there is correct.
        let displayed = |n: &ViewNode| {
            n.detail
                .iter()
                .any(|(k, v)| k == "range" && v.as_str() != "?")
        };
        let mut checked = 0usize;
        let mut stack: Vec<&ViewNode> = nodes.iter().collect();
        while let Some(node) = stack.pop() {
            if displayed(node) {
                assert!(
                    node.range.is_some(),
                    "{view} row {:?} shows a range but carries none",
                    node.label
                );
                checked += 1;
            }
            stack.extend(node.children.iter());
        }
        assert!(checked > 0, "{view} showed no range rows at all");
    }
}

/// The structured range agrees with the payload it came from, in both
/// coordinate systems.
#[test]
fn the_structured_range_matches_the_payload() {
    let d = data("set x 1\n");
    let expected = ViewRange::from_value(&d["ir"]["topLevel"][0]["range"])
        .expect("the payload's top-level statement has a range");

    // The IR view nests its statements under a `top-level` branch, so walk.
    let mut found = Vec::new();
    let mut stack: Vec<ViewNode> = build_view("ir", &d);
    while let Some(node) = stack.pop() {
        found.extend(node.range);
        stack.extend(node.children);
    }
    assert!(
        found.contains(&expected),
        "the statement's range must survive into the view model; got {found:?}"
    );
}

#[test]
fn tree_views_are_a_subset_of_view_meta() {
    // Tree views are declared in the canonical descriptor table. A tree
    // id without a builder would otherwise silently render as no data.
    let meta_ids: std::collections::HashSet<&str> =
        crate::views::VIEW_META.iter().map(|view| view.id).collect();
    for id in crate::views::tree_view_ids() {
        assert!(
            meta_ids.contains(id),
            "tree-view id {id:?} is not a known view descriptor id",
        );
    }
}

#[test]
fn every_tree_descriptor_has_a_shared_builder() {
    let d = data("set x 1");
    for descriptor in crate::views::VIEW_META
        .iter()
        .filter(|descriptor| descriptor.render_kind == crate::views::ViewRenderKind::Tree)
    {
        assert!(
            !build_view(descriptor.id, &d).is_empty(),
            "tree descriptor {} has no shared renderer",
            descriptor.id
        );
    }
}

#[test]
fn ir_view_has_top_level_and_proc_nodes() {
    let d = data("proc f {x} { return $x }\nf 1");
    let nodes = build_view("ir", &d);
    assert_eq!(nodes[0].label, "top-level");
    assert!(nodes.iter().any(|n| n.label.starts_with("::f")));
}

#[test]
fn cfg_view_lists_blocks_with_entry_tag() {
    let d = data("if {$x} { puts hi }");
    let nodes = build_view("cfg", &d);
    let func = &nodes[0];
    assert!(func.label.starts_with("function ::top"));
    assert!(func.children.iter().any(|b| b.label.contains("[entry]")));
}

#[test]
fn world_ssa_view_explains_availability_and_transition_policy() {
    let d = data("interp create child\ninterp hide child puts\n");
    let nodes = build_view("worldSsa", &d);
    assert!(nodes[0].label.contains("function ::top"));
    assert!(
        nodes[0]
            .children
            .iter()
            .any(|node| node.label.starts_with("operations"))
    );
    assert!(
        nodes[0]
            .children
            .iter()
            .any(|node| node.label.starts_with("invocation proof"))
    );
}

#[test]
fn loops_view_reports_a_loop() {
    let d = data("for {set i 0} {$i < 3} {incr i} { puts $i }");
    let nodes = build_view("loops", &d);
    // function ::top → one loop header node.
    assert!(
        nodes[0]
            .children
            .iter()
            .any(|n| n.label.starts_with("header"))
    );
}

/// A selected native addition reads as a selection, with what it proved,
/// rather than as an empty list of rejected premises.
#[test]
fn aot_view_names_a_selected_native_add() {
    let d = serde_json::json!({ "aot": {
            "kind": "native-i64-add",
            "operation": "intrinsic",
            "semanticDecline": null,
            "regionPlanStatus": "available",
            "regions": [],
            "nativeDeclines": [],
            "nativeI64Add": {
                "callee": "::add",
                "operands": [2, 4],
                "boundaryOperation": { "kind": "intrinsic", "id": "channel-write" },
                "frameElided": true,
                "closedProgramStatements": 4,
            },
        } });
    let nodes = build_view("aot", &d);
    let labels: Vec<&str> = nodes[0]
        .children
        .iter()
        .map(|child| child.label.as_str())
        .collect();
    assert_eq!(labels, ["regions (0)", "native i64 add: selected"]);
    let selected = &nodes[0].children[1];
    assert!(
        selected
            .detail
            .contains(&("callee".to_owned(), "::add".to_owned()))
    );
    assert!(
        selected
            .detail
            .contains(&("boundary".to_owned(), "channel-write".to_owned()))
    );
}

#[test]
fn unknown_view_is_empty() {
    assert_eq!(
        build_view("does-not-exist", &data("set x 1")),
        [] as [crate::view_tree::ViewNode; 0]
    );
}

#[test]
fn empty_analysis_views_have_a_dim_note() {
    let d = data("set x 1");
    let taint = build_view("taint", &d);
    assert_eq!(taint.len(), 1);
    assert_eq!(taint[0].style.as_deref(), Some("dim"));
}
