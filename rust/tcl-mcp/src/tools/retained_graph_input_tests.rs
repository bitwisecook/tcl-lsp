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

#[test]
fn memory_alias_tool_keeps_authored_exposure_and_unknown_storage() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let graph = super::memory_aliases(&serde_json::json!({
        "source": "proc aliaser {} {global ::shared; set shared 1}", "dialect": "tcl",
    }));
    let function = graph["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["name"] == "::aliaser")
        .unwrap();
    assert!(
        function["alias_sets"]
            .as_array()
            .unwrap()
            .iter()
            .any(|alias| {
                alias["reason"] == "global-cell"
                    && alias["names"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|name| name == "::shared")
                    && alias["names"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|name| name == "shared")
            })
    );
    assert_eq!(function["has_wildcard_aliasing"].as_bool(), Some(true));
}
