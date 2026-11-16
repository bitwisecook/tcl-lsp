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
fn source_optimisation_certificates_retain_unknown_world_hazards() {
    let source = "proc a {} {puts VALUE}\nproc b {} {puts OTHER}\n";
    // Test-input correction (user decision 2026-10-19T14:20Z): a document
    // with no explicit entry is analysed as a fresh interpreter, so the
    // unknown runtime history this test is about must be supplied as an
    // explicit unknown entry. The unit is otherwise built exactly as the
    // document driver builds it; every assertion below is unchanged.
    let unit = {
        let environment = tcl_registry::model::ingress::resolve_environment("tcl8.6");
        let context = environment.default_context_registry();
        let registry = context.commands();
        let config = tcl_lexer::LexerConfig::for_file_grammar(environment.grammar());
        let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            environment.analyser_profile(),
            environment.unit_profile(),
            Arc::clone(&context),
            config,
        );
        let declared =
            tcl_compiler::analyser::utils::document_declared_surface(source, None, "tcl8.6");
        let mut entry = tcl_compiler::command_binding::SourceAnalysisEntry::for_supplied_source(
            registry,
            &input,
            config,
            Some(input.unit_profile()),
        );
        entry.unknown_entry = true;
        Arc::new(
            CompilationUnit::build_with_analysis_input(
                source,
                UnitBuildOptions {
                    registry,
                    defer_top_level: false,
                    config,
                    dialect: Some(input.unit_profile()),
                    external_call_sites: None,
                    declared_commands: Some(&declared),
                },
                Some(&entry),
                &input,
            )
            .with_interprocedural(registry, Some(input.unit_profile())),
        )
    };
    assert!(unit.ir_module.source_entry.unknown_entry);
    let input = unit.ir_module.source_metadata_input.as_ref().unwrap();
    let registry = input.borrowed_context_registry().commands();
    assert!(unit.ir_module.source_entry.native_entry.is_none());
    assert_ne!(
        unit.command_mutations,
        tcl_compiler::command_binding::ModuleCommandMutations::default()
    );
    assert!(module_has_trace_facts(&unit.ir_module));
    assert!(unit.top_level.body_id.is_none());
    let functions = unit
        .ir_module
        .original_native_declaration_body_units
        .keys()
        .map(|id| unit.function_for_body(Some(*id)).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(functions.len(), 2);
    for function in functions {
        let id = function.body_id.unwrap();
        let capsule = tcl_compiler::body_analysis_capsule::BodyAnalysisCapsule::retain(
            Arc::clone(&unit),
            id,
            registry,
        )
        .unwrap();
        assert!(capsule.has_closed_readonly_source_correspondence());
        assert!(!capsule.has_closed_native_correspondence());
        assert_eq!(capsule.unit().command_mutations, unit.command_mutations);
        assert!(module_has_trace_facts(&capsule.unit().ir_module));
        assert!(function.cfg.executed_source.is_none());
        assert!(function.cfg.native_compilation_admission.is_none());
    }
}
