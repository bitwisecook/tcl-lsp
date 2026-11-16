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
use serde_json::json;

// naming.source.retained-explorer-consumers. These are wire/render
// controls; they do not prove Native execution or provider behaviour.
#[test]
fn unavailable_source_fields_are_distinct_from_completed_empty_views() {
    // naming.source.retained-explorer-consumers
    // docs/design/analysis/name-resolution-proofs/retained-explorer-consumers.md
    // Wire/render observation only; no Native execution or provider claim.
    let empty = json!({
        "optimisations": [], "optimiserPasses": [],
        "eventOrder": [], "annotations": [], "irulesFlow": [],
    });
    for (view, field, expected_empty) in [
        ("opt", "optimisations", "(no optimiser rewrites)"),
        (
            "optimiserPasses",
            "optimiserPasses",
            "(optimiser pipeline unavailable)",
        ),
        ("eventOrder", "eventOrder", "(no event-order data)"),
        ("callouts", "annotations", "(no salient annotations)"),
    ] {
        assert_eq!(build_view(view, &empty)[0].label, expected_empty);
        let mut unavailable = empty.clone();
        unavailable[field] = Value::Null;
        let rows = build_view(view, &unavailable);
        assert_eq!(rows, unavailable_source_view(), "{view}");
        assert!(rows[0].range.is_none());
        let mut missing = empty.clone();
        missing.as_object_mut().unwrap().remove(field);
        assert_eq!(
            build_view(view, &missing),
            unavailable_source_view(),
            "{view}"
        );
    }
}

#[test]
fn unavailable_event_section_preserves_independent_irules_warnings() {
    // naming.source.retained-explorer-consumers
    // docs/design/analysis/name-resolution-proofs/retained-explorer-consumers.md
    // Wire/render observation only; no Native execution or provider claim.
    let data = json!({
        "eventOrder": null,
        "irulesFlow": [{"code": "W-test", "message": "independent warning", "range": null}],
    });
    let rows = build_view("irules", &data);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0], unavailable_source_view()[0]);
    assert_eq!(rows[1].label, "W-test independent warning");
    let empty = json!({"eventOrder": [], "irulesFlow": []});
    assert_eq!(build_view("irules", &empty)[0].label, "(no iRules events)");
}
