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
use tcl_compiler::analyser::Analyser;

#[test]
fn logical_class_navigation_candidate_requires_the_selected_original_factory() {
    let caller = "proc caller {} {helper}";
    let analysis = Analyser::new().analyse(caller, "tcl");
    let request =
        tcl_compiler::registry_invocation::source_structure::source_logical_command_candidates_at(
            caller,
            &analysis,
            u32::try_from(caller.find("helper").unwrap()).unwrap(),
        )
        .unwrap();
    let class = Analyser::new().analyse("oo::class create helper {method run {} {}}", "tcl");
    let native = Analyser::new().analyse("oo::class create helper {method run {} {}}", "tcl8.6");
    let dynamic = Analyser::new().analyse("oo::class create $unknown {method run {} {}}", "tcl");
    let index = WorkspaceIndex::from_documents([
        ("class", &class),
        ("native", &native),
        ("dynamic", &dynamic),
    ]);
    let candidates = index.logical_command_candidates(&request, "caller");
    assert_eq!(candidates.len(), 1, "{candidates:?}");
    assert_eq!(candidates[0].uri, "class");
}
