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
fn native_consumed_source_heads_do_not_read_unrelated_procedure_summaries() {
    let source = "proc f {x} {set value $x; return $value}\nproc other {} {return OTHER}\n";
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let profile = context.commands().profile().unwrap();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&context),
        config,
    );
    let unit = crate::compilation_unit::CompilationUnit::build_with_analysis_input(
        source,
        crate::compilation_unit::UnitBuildOptions {
            registry: context.commands(),
            defer_top_level: false,
            config,
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        None,
        &input,
    );
    let known = unit
        .ir_module
        .original_source_procedure_bodies()
        .map(|(id, _)| *id)
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(known.len(), 2);
    for &body in &known {
        let function = unit.function_for_body(Some(body)).unwrap();
        assert!(
            function
                .cfg
                .blocks
                .values()
                .flat_map(|block| &block.statements)
                .all(|statement| !matches!(
                    statement,
                    crate::ir::Statement::Call { .. }
                        | crate::ir::Statement::Barrier { .. }
                        | crate::ir::Statement::NativeCall { .. }
                ))
        );
        assert_eq!(
            super::resolved_callees(&unit, Some(body), &known, context.commands()),
            [] as [tcl_runtime_api::ProcedureId; 0]
        );
        assert_eq!(
            super::command_subst_callees(&unit, body, &known, context.commands()),
            [] as [tcl_runtime_api::ProcedureId; 0]
        );
    }
}
