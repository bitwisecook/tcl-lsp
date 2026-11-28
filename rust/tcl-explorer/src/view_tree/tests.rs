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
