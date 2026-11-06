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

use tcl_compiler::semantic_optimisation::SemanticOptimisationConfig;

/// The `native-tier` group and `WasmCompileOptions::native_tier` are two
/// spellings of one set; a pass added to the option and not the group
/// would make the CLI and the Explorer quietly weaker than the harness.
#[test]
fn native_tier_group_matches_the_wasm_option() {
    let group = SemanticOptimisationConfig::from_names("native-tier").unwrap();
    let option = crate::codegen::wasm::WasmCompileOptions::hosted()
        .native_tier()
        .semantic_optimisations();
    assert_eq!(group, option);
}
