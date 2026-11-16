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

#[test]
fn actual_cu_null_fields_do_not_render_as_empty_analysis() {
    // naming.source.retained-explorer-consumers
    // docs/design/analysis/name-resolution-proofs/retained-explorer-consumers.md
    for (view, field) in [
        ("ssa", "cfgPostSsa"),
        ("shimmer", "shimmer"),
        ("gvn", "gvn"),
        ("taint", "taintWarnings"),
        ("liveness", "liveness"),
        ("bounds", "bounds"),
    ] {
        let mut unavailable = json!({});
        unavailable[field] = Value::Null;
        assert_eq!(
            build_view(view, &unavailable),
            unavailable_source_view(),
            "{view}"
        );
        let missing = json!({});
        assert_eq!(
            build_view(view, &missing),
            unavailable_source_view(),
            "{view}"
        );
        unavailable[field] = json!([]);
        assert_ne!(
            build_view(view, &unavailable),
            unavailable_source_view(),
            "{view}"
        );
    }
}

#[test]
fn unavailable_irules_warning_section_preserves_available_event_rows() {
    let data = json!({
        "eventOrder": [{"event": "HTTP_REQUEST", "base_priority": 500, "priority_offset": 0}],
        "irulesFlow": null,
    });
    let rows = build_view("irules", &data);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0], unavailable_source_view()[0]);
    assert_eq!(rows[1].label, "HTTP_REQUEST eff=500");
    let empty = json!({"eventOrder": [], "irulesFlow": []});
    assert_eq!(build_view("irules", &empty)[0].label, "(no iRules events)");
}
