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

fn foreign_analysis(source: &str) -> tcl_compiler::analyser::AnalysisResult {
    let original = Analyser::new().analyse(source, "tcl");
    let input = original.resolved_input.as_ref().unwrap();
    let mut config = input.lexer_config();
    config.strict_quoting ^= true;
    Analyser::new()
        .with_resolved_input(ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            input.context_registry(),
            config,
        ))
        .analyse(source, "tcl")
}

#[test]
fn library_source_candidates_do_not_mix_same_slots_from_foreign_complete_inputs() {
    let source = "helper";
    let caller = Analyser::new().analyse(source, "tcl");
    let request =
        tcl_compiler::registry_invocation::source_structure::source_logical_command_candidates_at(
            source, &caller, 0,
        )
        .unwrap();
    let target_source = "proc helper {} {}";
    let target = Analyser::new().analyse(target_source, "tcl");
    let foreign = foreign_analysis(target_source);
    let index = WorkspaceIndex::from_documents([("target", &target), ("foreign", &foreign)]);
    let candidates = index.logical_command_candidates(&request, "caller");
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].uri(), "target");
}

#[test]
fn expression_source_candidates_do_not_mix_same_slots_from_foreign_complete_inputs() {
    let source = "expr {Pi()}";
    let caller = Analyser::new().analyse(source, "tcl");
    let offset = u32::try_from(source.find("Pi").unwrap()).unwrap();
    let occurrence = caller.logical_math_function_at(source, offset).unwrap();
    let target_source = "namespace eval ::tcl::mathfunc {proc Pi {} {}}";
    let target = Analyser::new().analyse(target_source, "tcl");
    let foreign = foreign_analysis(target_source);
    let index = WorkspaceIndex::from_documents([("target", &target), ("foreign", &foreign)]);
    let candidates = index
        .logical_math_source_candidates(&occurrence, &caller, "caller")
        .unwrap();
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].0, "target");
}
