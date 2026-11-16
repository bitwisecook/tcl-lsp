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

fn assert_source_definition_owner_withdrawals(
    unit: &crate::compilation_unit::CompilationUnit,
    selected: &[OriginalSourceSsaDefinition<'_>],
    context: &tcl_registry::model::ContextRegistry,
) {
    for stale in [false, true] {
        let mut changed = unit.clone();
        let function = changed
            .body_units
            .get_mut(&selected[0].name().body_id())
            .unwrap();
        if stale {
            function.source_config = tcl_lexer::LexerConfig::for_file_grammar(
                tcl_registry::model::ingress::resolve_environment("tcl8.4")
                    .unit_profile()
                    .grammar,
            );
        } else {
            function.source_metadata_input = None;
        }
        assert!(
            original_source_definition_at_unit(
                &changed,
                selected[0].name().span(),
                context.commands()
            )
            .is_none()
        );
    }
    let mut missing = unit.clone();
    missing.ir_module.source_metadata_input = None;
    assert!(
        original_source_definition_at_unit(&missing, selected[0].name().span(), context.commands())
            .is_none()
    );
}

#[test]
fn native_source_definitions_select_actual_body_symbols_without_physical_store_grants() {
    let source = "proc a {} {set same 1; return $same}\nproc b {} {set same TWO; return $same}\n";
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
    let options = crate::compilation_unit::UnitBuildOptions {
        registry: context.commands(),
        defer_top_level: false,
        config,
        dialect: Some(profile),
        external_call_sites: None,
        declared_commands: None,
    };
    let unit = crate::compilation_unit::CompilationUnit::build_with_analysis_input(
        source, options, None, &input,
    );
    assert_eq!(
        unit.ir_module.original_native_declaration_body_units.len(),
        2
    );
    let offsets = source
        .match_indices("same ")
        .map(|(offset, _)| u32::try_from(offset).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(offsets.len(), 2);
    let selected = offsets
        .iter()
        .map(|&offset| {
            original_source_definition_at_unit(
                &unit,
                tcl_lexer::Span::new(offset, offset + 4),
                context.commands(),
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    assert_ne!(selected[0].name().body_id(), selected[1].name().body_id());
    assert!(!std::ptr::eq(
        selected[0].function(),
        selected[1].function()
    ));
    for definition in &selected {
        assert_eq!(definition.name().original_name_input().bytes(), b"same");
        let function = definition.function();
        let body = definition.name().body_id();
        assert_eq!(function.body_id, Some(body));
        let mut accepted = 0;
        for (&block, statements) in &function.ssa.blocks {
            for (index, statement) in statements.statements.iter().enumerate() {
                if !statement.defs.contains_key(&definition.symbol()) {
                    continue;
                }
                let view = SsaSourceView::at_statement(&function.ssa, block, index);
                if view
                    .original_source_definition_for_body(
                        definition.symbol(),
                        &unit,
                        body,
                        context.commands(),
                    )
                    .is_some()
                {
                    accepted += 1;
                    assert!(
                        view.original_definition_name_with_metadata_context(
                            definition.symbol(),
                            context.commands(),
                            function.invocation_metadata_context_for_module(
                                context.commands(),
                                &unit.ir_module
                            )
                        )
                        .is_none()
                    );
                    let other = selected
                        .iter()
                        .find(|other| other.name().body_id() != body)
                        .unwrap();
                    assert!(
                        view.original_source_definition_for_body(
                            definition.symbol(),
                            &unit,
                            other.name().body_id(),
                            context.commands()
                        )
                        .is_none()
                    );
                }
            }
        }
        assert_eq!(accepted, 1);
    }
    assert_source_definition_owner_withdrawals(&unit, &selected, &context);
}
