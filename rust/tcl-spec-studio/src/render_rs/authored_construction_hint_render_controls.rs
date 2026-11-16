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

#[test]
fn actual_shipped_descriptor_survives_studio_projection_and_rust_source_rendering() {
    let value = crate::draft::definition_body_block(&tcl_registry::definer::SNIT_GRAMMAR);
    assert_eq!(
        value["bare_word_construction_source_hint"]["values"],
        serde_json::json!(["%AUTO%"])
    );
    assert_eq!(
        value["bare_word_construction_source_hint"]["prefixes"],
        serde_json::json!(["."])
    );
    let rendered = definition_body_expr(&value, "").unwrap();
    assert!(rendered.contains(
        "bare_word_construction_source_hint: Some(crate::definer::BareWordConstructionSourceHint"
    ));
    assert!(rendered.contains("values: &[\"%AUTO%\"]"));
    assert!(rendered.contains("prefixes: &[\".\"]"));
    let mut wrong = value;
    wrong["bare_word_construction_source_hint"]["prefixes"] = serde_json::json!([""]);
    assert!(definition_body_expr(&wrong, "").is_none());
}
