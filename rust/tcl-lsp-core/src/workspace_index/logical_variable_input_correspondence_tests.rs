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
use tcl_compiler::analyser::{Analyser, ResolvedAnalysisInput};

#[test]
fn same_variable_geometry_cannot_join_a_different_complete_input() {
    let source = "set ::shared 1";
    let caller = "puts $::shared";
    let caller_analysis = Analyser::new().analyse(caller, "tcl");
    let target_analysis = Analyser::new().analyse(source, "tcl");
    let retained = target_analysis.resolved_input.as_ref().unwrap();
    let mut config = retained.lexer_config();
    config.strict_quoting ^= true;
    let foreign_input = ResolvedAnalysisInput::new(
        retained.analyser_profile(),
        retained.unit_profile(),
        retained.context_registry(),
        config,
    );
    let foreign = Analyser::new()
        .with_resolved_input(foreign_input)
        .analyse(source, "tcl");
    assert_ne!(
        foreign.original_variable_symbols,
        [] as [tcl_compiler::signature_scan::variable_symbol::SignatureSourceVariableOccurrence; 0]
    );
    let occurrence = caller_analysis
        .original_variable_symbols
        .iter()
        .find(|occurrence| occurrence.symbol().is_namespace())
        .unwrap();
    assert!(
        foreign
            .original_variable_symbols
            .iter()
            .any(|candidate| candidate.symbol() == occurrence.symbol())
    );
    let index = WorkspaceIndex::from_documents([
        ("file:///caller.tcl", &caller_analysis),
        ("file:///target.tcl", &target_analysis),
        ("file:///foreign.tcl", &foreign),
    ]);
    let request = index
        .logical_variable_location_for_occurrence(
            "file:///caller.tcl",
            caller,
            &caller_analysis,
            occurrence,
        )
        .unwrap();
    let locations = index.logical_variable_source_locations(&request);
    assert!(
        locations
            .iter()
            .any(|location| location.uri() == "file:///target.tcl")
    );
    assert!(
        locations
            .iter()
            .all(|location| location.uri() != "file:///foreign.tcl")
    );
}
