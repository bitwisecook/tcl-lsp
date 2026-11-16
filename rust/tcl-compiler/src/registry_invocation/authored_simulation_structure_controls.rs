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

fn module(source: &str) -> crate::ir::Module {
    let context = tcl_registry::model::ingress::static_context_for("tcl8.6");
    crate::compilation_unit::CompilationUnit::build_for_profile(
        source,
        context.commands(),
        false,
        context.commands().profile().unwrap(),
    )
    .ir_module
}

fn switch_tokens(module: &crate::ir::Module) -> CommandTokens {
    let mut pending = module
        .procedures
        .values()
        .map(|p| &p.body)
        .collect::<Vec<_>>();
    while let Some(script) = pending.pop() {
        for statement in &script.statements {
            if let Some(tokens) = script.retained_source_tokens_for_statement(statement)
                && tokens
                    .argv_texts
                    .first()
                    .is_some_and(|head| head == "switch")
            {
                return tokens.clone();
            }
            pending.extend(statement.child_scripts());
        }
    }
    panic!("actual retained switch command");
}

fn declaration_tokens(module: &crate::ir::Module) -> CommandTokens {
    let mut pending = module
        .procedures
        .values()
        .map(|p| &p.body)
        .collect::<Vec<_>>();
    while let Some(script) = pending.pop() {
        for statement in &script.statements {
            if let Some(tokens) = script.retained_source_tokens_for_statement(statement)
                && tokens.argv_texts.first().is_some_and(|head| head == "proc")
                && tokens.argv_texts.get(1).is_some_and(|name| name == "inner")
            {
                return tokens.clone();
            }
            pending.extend(statement.child_scripts());
        }
    }
    panic!("actual retained inner procedure declaration");
}

#[test]
fn authored_declaration_coverage_keeps_separate_purpose_and_exact_owner() {
    let source = "proc outer {} {proc inner {} {return VALUE}}";
    let module = module(source);
    let tokens = declaration_tokens(&module);
    let registry = module.resolved_registry();
    let context = InvocationMetadataContext::for_module(registry, &module).unwrap();
    let binding = tokens.source_binding.as_ref().unwrap();
    assert_eq!(
        binding.original_source_advice_purpose(&tokens, context, registry),
        Some(crate::command_binding::OriginalInvocationSourcePurpose::AuthoredSimulation)
    );
    assert!(
        logical_structured_invocation_with_metadata_context(registry, context, &tokens, None)
            .is_none()
    );
    let selected =
        original_authored_procedure_declaration_for_source_coverage(registry, context, &tokens)
            .unwrap();
    assert!(
        selected
            .facts
            .traits
            .contains(tcl_registry::Traits::DEFERS_BODY)
    );
    let advice = binding
        .declaration_operand_layout_advice(&tokens)
        .or_else(|| binding.original_compilation_lookup_advice(&tokens))
        .unwrap();
    assert!(!advice.closed_lookup());
    let mut missing = tokens.clone();
    missing.source_binding = None;
    assert!(
        original_authored_procedure_declaration_for_source_coverage(registry, context, &missing)
            .is_none()
    );
    let mut changed = tokens.clone();
    changed.argv_texts[1] = "changed".to_owned();
    assert!(
        original_authored_procedure_declaration_for_source_coverage(registry, context, &changed)
            .is_none()
    );
    let foreign = super::authored_simulation_structure_controls::module(source);
    let other_registry = foreign.resolved_registry();
    let other_context = InvocationMetadataContext::for_module(other_registry, &foreign).unwrap();
    assert!(
        original_authored_procedure_declaration_for_source_coverage(
            other_registry,
            other_context,
            &tokens
        )
        .is_none()
    );
    let opaque = super::authored_simulation_structure_controls::module(
        "proc outer {} {unknown_op; proc inner {} {return VALUE}}",
    );
    let opaque_tokens = declaration_tokens(&opaque);
    let opaque_registry = opaque.resolved_registry();
    let opaque_context = InvocationMetadataContext::for_module(opaque_registry, &opaque).unwrap();
    assert!(
        original_authored_procedure_declaration_for_source_coverage(
            opaque_registry,
            opaque_context,
            &opaque_tokens
        )
        .is_none()
    );
}

#[test]
fn authored_declaration_coverage_keeps_completed_static_original_name() {
    for (source, expected) in [
        (r"proc p\uD800 {} {return VALUE}", true),
        ("proc helper {} {return VALUE}", true),
        ("proc $name {} {return VALUE}", false),
        ("proc [unknown] {} {return VALUE}", false),
        ("proc {*}$names {} {return VALUE}", false),
    ] {
        let analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.4");
        let input = analysis.resolved_input.as_ref().unwrap();
        let registry = input.borrowed_context_registry().commands();
        let config = input.lexer_config();
        let context = InvocationMetadataContext::for_source_input(
            registry,
            input,
            config,
            Some(input.unit_profile()),
        )
        .unwrap();
        let command =
            crate::segmenter::segment_commands_with_offset_and_config(source, 0, config).remove(0);
        let mut tokens =
            CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(source), config, &command);
        analysis
            .retained_command_realm()
            .unwrap()
            .stamp_original_tokens(&mut tokens);
        assert_eq!(
            original_authored_procedure_declaration_for_source_coverage(registry, context, &tokens)
                .is_some(),
            expected,
            "{source}"
        );
        if expected {
            let mut changed = tokens.clone();
            changed.argv_texts[1] = "other".to_owned();
            assert!(
                original_authored_procedure_declaration_for_source_coverage(
                    registry, context, &changed
                )
                .is_none()
            );
        }
    }
}

#[test]
fn authored_simulation_case_structure_retains_source_purpose_without_native_frame() {
    let source = "proc f {x} {while 1 {switch -glob $x {a* {break} default {break}}}; puts $y}";
    let module = module(source);
    let tokens = switch_tokens(&module);
    let registry = module.resolved_registry();
    let context = InvocationMetadataContext::for_module(registry, &module).unwrap();
    let binding = tokens.source_binding.as_ref().unwrap();
    assert_eq!(
        binding.original_source_advice_purpose(&tokens, context, registry),
        Some(crate::command_binding::OriginalInvocationSourcePurpose::AuthoredSimulation)
    );
    let advice = binding
        .declaration_operand_layout_advice(&tokens)
        .or_else(|| binding.original_compilation_lookup_advice(&tokens))
        .unwrap();
    assert!(!advice.closed_lookup());
    assert!(advice.closed_authored_simulation_source_lookup(&tokens, context, registry));
    let selected =
        logical_structured_invocation_with_metadata_context(registry, context, &tokens, None)
            .expect("actual authored source case schema");
    assert!(selected.is_authored_simulation());
    assert!(matches!(
        selected.source_roles.as_ref(),
        Some(StructuredSourceRoles::AuthoredSimulationCase(_))
    ));
    let selected = SourceStructuredInvocation::Logical(Box::new(selected));
    assert!(matches!(
        selected.lowering_identity(),
        SelectedLoweringBindingIdentity::AuthoredSimulationSourceSchema(_)
    ));
    assert_eq!(
        selected.lowering_hook(),
        Some(tcl_registry::hooks::LoweringHookId::Switch)
    );
    assert_eq!(
        selected.written_roles(),
        vec![(2, tcl_registry::ArgRole::Body)]
    );
    assert!(selected.error_context().is_none());
    assert!(module.source_entry.native_entry.is_none());
    assert_eq!(
        binding.native_compilation_admission_selection(),
        tcl_registry::native_compilation::NativeCompilationSelection::Generic
    );
    assert!(admitted_native_compiler_invocation(registry, None, &tokens).is_none());
    assert!(
        normal_representation_invocation_with_metadata_context(registry, Some(context), &tokens)
            .is_none()
    );
    let mut changed = tokens.clone();
    changed.argv_texts[1] = "-exact".to_owned();
    assert!(
        logical_structured_invocation_with_metadata_context(registry, context, &changed, None)
            .is_none()
    );
    changed = tokens.clone();
    changed.source_binding = None;
    assert!(
        logical_structured_invocation_with_metadata_context(registry, context, &changed, None)
            .is_none()
    );
    let input = module.source_metadata_input.as_ref().unwrap();
    let mut changed_config = input.lexer_config();
    changed_config.strict_quoting = !changed_config.strict_quoting;
    let changed_input = crate::analyser::ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        input.context_registry(),
        changed_config,
    );
    let changed_context =
        InvocationMetadataContext::for_analysis_input(registry, &changed_input).unwrap();
    assert!(!advice.closed_authored_simulation_source_lookup(&tokens, changed_context, registry));
    assert!(
        logical_structured_invocation_with_metadata_context(
            registry,
            changed_context,
            &tokens,
            None
        )
        .is_none()
    );
}

#[test]
fn authored_simulation_case_structure_refuses_unknown_shadowed_and_dynamic_source() {
    for source in [
        "proc switch args {}; proc f {x} {switch -glob $x {a* {break} default {break}}}",
        "proc f {x} {unknown_writer; switch -glob $x {a* {break} default {break}}}",
        "proc f {x body} {switch -glob $x a* $body default {break}}",
        "proc f {x cases} {switch -glob $x {*}$cases}",
        "proc f {} {switch -glob [unknown_writer] {a* {break} default {break}}}",
    ] {
        let module = module(source);
        let tokens = switch_tokens(&module);
        let registry = module.resolved_registry();
        let context = InvocationMetadataContext::for_module(registry, &module).unwrap();
        assert!(
            logical_structured_invocation_with_metadata_context(registry, context, &tokens, None)
                .is_none()
        );
    }
}
