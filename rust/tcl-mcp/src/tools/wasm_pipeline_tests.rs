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
fn compile_tool_uses_canonical_semantic_plan_without_backend_parameter() {
    let schema = tool_schemas()
        .into_iter()
        .find(|(name, _, _)| *name == "compile_wasm")
        .expect("compile_wasm schema")
        .2;
    assert!(schema["properties"].get("backend").is_none());

    let result = dispatch(
        "compile_wasm",
        &json!({ "source": "string length hello", "dialect": "tcl9.0" }),
    )
    .expect("compile_wasm tool");
    assert_eq!(result["pipeline"], "canonical");
    assert_eq!(result["codegen_plan"], "generic-invoke");
    assert!(result["semantic_decline"].is_null());
    assert!(result["wat"].as_str().unwrap().contains("tcl_invoke_argv"));
}
