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

use serde_json::json;
use tcl_explorer::serialise_meta;

mod support;

#[test]
fn meta_lists_all_dialects_views_and_severities() {
    support::install_surfaces();
    let meta = serialise_meta();
    // Every environment the registry offers is exposed, and in its order.
    // Derived from the selectable set rather than pinned to a count: the
    // invariant worth holding is "the explorer drops none of them".
    let dialects: Vec<&str> = meta["dialects"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["name"].as_str().unwrap())
        .collect();
    let selectable: Vec<&str> = tcl_dialect::model::EnvironmentRegistry::compiled_selectable()
        .iter()
        .map(|environment| environment.id.as_str())
        .collect();
    assert_eq!(dialects, selectable);
    // Every entry carries its labels, like the `views` entries.
    for entry in meta["dialects"].as_array().unwrap() {
        assert!(entry["displayName"].as_str().is_some_and(|s| !s.is_empty()));
        assert!(entry["shortName"].as_str().is_some_and(|s| !s.is_empty()));
    }
    // The original 27 views, plus World SSA, six durable compiler
    // artefact views, the optimisation-pass toggle surface and the AOT
    // plan.
    assert_eq!(meta["views"].as_array().unwrap().len(), 36);
    assert_eq!(meta["severities"], json!(["error", "warning", "info"]));
    let traits = meta["traits"]
        .as_array()
        .expect("trait presentation metadata");
    assert_eq!(traits.len(), tcl_registry::traits::Trait::ALL.len());
    assert!(traits.iter().all(|item| {
        item["name"].as_str().is_some_and(|text| !text.is_empty())
            && item["summary"]
                .as_str()
                .is_some_and(|text| !text.is_empty())
            && item["group"].as_str().is_some_and(|text| !text.is_empty())
    }));
    // The parse-tree tab is the CST; there is no `greentree` entry.
    assert_eq!(
        meta["views"][0],
        json!({ "id": "cst", "label": "CST", "payload": "cst", "group": "compiler", "renderKind": "frontend" })
    );
    assert!(
        !meta["views"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["id"] == "greentree"),
        "greentree tab must be dropped"
    );
}
