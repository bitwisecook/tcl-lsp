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

//! Original procedure argument roles and argv: native and Logical owners and their refusals.

use super::*;

fn assert_original_procedure_roles_retained(
    binding: &crate::command_binding::SourceInvocationBinding,
    tokens: &crate::ir::CommandTokens,
    input: &ResolvedAnalysisInput,
    registry: &tcl_registry::CommandRegistry,
) {
    assert!(
        binding
            .original_source_procedure_argument_roles(tokens, input, registry)
            .is_some()
    );
}

fn assert_original_procedure_axis_refused(
    binding: &crate::command_binding::SourceInvocationBinding,
    tokens: &crate::ir::CommandTokens,
    input: &ResolvedAnalysisInput,
    registry: &tcl_registry::CommandRegistry,
    axis: i32,
) {
    assert!(
        binding
            .original_source_procedure_argument_roles(tokens, input, registry)
            .is_none(),
        "axis {axis}"
    );
}

fn assert_original_procedure_roles_refused(
    binding: &crate::command_binding::SourceInvocationBinding,
    tokens: &crate::ir::CommandTokens,
    input: &ResolvedAnalysisInput,
    registry: &tcl_registry::CommandRegistry,
) {
    assert!(
        binding
            .original_source_procedure_argument_roles(tokens, input, registry)
            .is_none()
    );
}

fn original_native_procedure_argument_tokens(
    source: &str,
) -> (
    tcl_vm::Vm,
    Arc<tcl_registry::model::ContextRegistry>,
    ResolvedAnalysisInput,
    crate::ir::CommandTokens,
) {
    let environment = tcl_registry::model::ingress::resolve_known_environment("tcl9.0").unwrap();
    let context = environment.default_context_registry();
    let profile = environment.unit_profile();
    let (owner, entry) = crate::environment_ingress::captured_native_entry_with_owner(profile);
    let config = super::super::super::SourceAnalysisOptions {
        native_entry: Some(&entry),
        ..Default::default()
    }
    .native_lexer_config(tcl_lexer::LexerConfig::for_file_grammar(profile.grammar));
    let input = ResolvedAnalysisInput::new(profile, profile, Arc::clone(&context), config);
    let options = super::super::super::SourceAnalysisOptions {
        metadata_context: crate::registry_invocation::InvocationMetadataInput::SuppliedSource(
            Some(&input),
        ),
        native_entry: Some(&entry),
        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
        native_compilation: crate::environment_ingress::authoring_native_compilation(),
        ..Default::default()
    };
    let image = tcl_lexer::SourceImage::document(source);
    let bindings = super::super::super::SourceCommandBindings::analyse_image_in_frame_with_options(
        &image,
        &crate::var_resolve::VariableExecutionFrame::Global,
        config,
        context.commands(),
        options,
    )
    .unwrap();
    let segments =
        crate::segmenter::segment_commands_image_with_offset_and_config(&image, 0, config).unwrap();
    let mut tokens = crate::ir::CommandTokens::from_segmented(
        &image.source_map(),
        config,
        segments.last().unwrap(),
    );
    bindings.stamp_original_tokens(&mut tokens);
    (owner, context, input, tokens)
}

#[test]
fn original_procedure_argument_roles_keep_native_entry_and_logical_source_owners_separate() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Software-owned source argv roles, independently of entered calls and Normal.
    for source in [
        "proc selected {value} {}; selected {[inert CHILD]}",
        "proc selected {value} {}; rename selected relocated; relocated VALUE",
        "proc selected {prefix value} {}; interp alias {} forwarded {} selected PREFIX; forwarded VALUE",
    ] {
        let (_owner, context, input, tokens) = original_native_procedure_argument_tokens(source);
        let binding = tokens.source_binding.as_ref().unwrap();
        assert!(
            binding
                .original_source_procedure_argument_roles(&tokens, &input, context.commands())
                .is_some(),
            "{source}"
        );
        assert!(!input.has_logical_source_name_context());
        assert!(
            binding
                .lookup_state
                .as_ref()
                .unwrap()
                .state
                .baseline
                .native_entry
                .is_some()
        );
        let advice = binding.declaration_call_layout_advice(&tokens).unwrap();
        assert!(advice.source_targets_are_closed());
        assert!(
            advice
                .targets()
                .iter()
                .all(|target| target.implementation_allocation.is_some())
        );
    }
    let context = logical_script_context();
    // The original declaration suffix remains lexical advice after the actual return.
    let source = "proc selected {value} {}; return DONE; selected {[inert CHILD]}";
    let (analysis, tokens) = logical_original_tokens(source, &context);
    let input = analysis.resolved_input.as_ref().unwrap();
    let binding = tokens.source_binding.as_ref().unwrap();
    assert!(
        binding
            .original_source_procedure_argument_roles(&tokens, input, context.commands())
            .is_some()
    );
    assert!(input.has_logical_source_name_context());
    assert!(binding.lookup_state.is_none());
}

#[test]
fn original_procedure_argument_roles_refuse_changed_missing_and_open_owners() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let source = "proc selected {value} {}; selected VALUE";
    let (_owner, context, input, tokens) = original_native_procedure_argument_tokens(source);
    let binding = tokens.source_binding.as_ref().unwrap();
    assert!(
        binding
            .original_source_procedure_argument_roles(&tokens, &input, context.commands())
            .is_some()
    );
    let changed_input = ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        input.context_registry(),
        tcl_lexer::LexerConfig {
            strict_quoting: !input.lexer_config().strict_quoting,
            ..input.lexer_config()
        },
    );
    assert!(
        binding
            .original_source_procedure_argument_roles(&tokens, &changed_input, context.commands())
            .is_none()
    );
    let foreign = tcl_registry::model::ingress::resolve_known_environment("tcl8.4")
        .unwrap()
        .default_context_registry();
    assert!(
        binding
            .original_source_procedure_argument_roles(&tokens, &input, foreign.commands())
            .is_none()
    );
    for axis in 0..7 {
        let mut changed = tokens.clone();
        match axis {
            0 => changed.argv_texts[1] = "DIFFERENT".into(),
            1 => changed.synthetic = Some(crate::ir::SyntheticMarker::EvaluatedArguments),
            2 => changed.source_binding.as_mut().unwrap().unknown = true,
            3 => changed.source_binding.as_mut().unwrap().may_be_absent = true,
            4 => {
                changed.source_binding.as_mut().unwrap().targets[0].implementation_allocation =
                    None;
            }
            5 => {
                let point = Arc::make_mut(
                    changed
                        .source_binding
                        .as_mut()
                        .unwrap()
                        .lookup_state
                        .as_mut()
                        .unwrap(),
                );
                Arc::make_mut(&mut point.state.baseline).native_entry = None;
            }
            6 => {
                let point = Arc::make_mut(
                    changed
                        .source_binding
                        .as_mut()
                        .unwrap()
                        .lookup_state
                        .as_mut()
                        .unwrap(),
                );
                Arc::make_mut(&mut point.state.baseline).metadata_context =
                    crate::registry_invocation::OwnedInvocationMetadataContext::Unavailable;
            }
            _ => unreachable!(),
        }
        assert!(
            changed
                .source_binding
                .as_ref()
                .unwrap()
                .original_source_procedure_argument_roles(&changed, &input, context.commands())
                .is_none(),
            "axis {axis}"
        );
    }
    for source in [
        "list VALUE",
        "proc selected {value} {}; unknown; selected VALUE",
    ] {
        let (_owner, context, input, tokens) = original_native_procedure_argument_tokens(source);
        assert!(
            tokens
                .source_binding
                .as_ref()
                .unwrap()
                .original_source_procedure_argument_roles(&tokens, &input, context.commands())
                .is_none()
        );
    }
}

#[test]
fn original_logical_procedure_roles_keep_declaration_layout_separate_from_allocation_advice() {
    // naming.compiler.original-positioned-callee-body-availability
    // docs/design/analysis/name-resolution-proofs/original-positioned-callee-body-availability.md
    // naming.source.original-point-operand-projection
    // docs/design/analysis/name-resolution-proofs/original-point-operand-projection.md
    let context = logical_script_context();
    for source in [
        "proc selected {value} {}; selected VALUE",
        "proc selected {prefix value} {}; interp alias {} forwarded {} selected PREFIX; forwarded VALUE",
        "proc selected {value} {}; return DONE; selected {[inert CHILD]}",
    ] {
        let (analysis, tokens) = logical_original_tokens(source, &context);
        let input = analysis.resolved_input.as_ref().unwrap();
        assert!(input.has_logical_source_name_context());
        let binding = tokens.source_binding.as_ref().unwrap();
        assert!(binding.declaration_call_layout_advice(&tokens).is_none());
        let declared = binding
            .original_logical_declaration_call_targets(&tokens, input, context.commands())
            .expect("complete Logical source declaration");
        assert_ne!(
            declared,
            [] as [crate::command_binding::SourceCommandTarget; 0]
        );
        assert!(declared.iter().all(|target| !target.registry_backed
            && target.kind == super::super::super::BindingKind::Proc
            && target.implementation_allocation.is_none()));
        assert!(
            binding
                .original_source_procedure_argument_roles(&tokens, input, context.commands(),)
                .is_some()
        );
        assert!(binding.proved_execution_target().is_none());
        assert!(binding.proved_handler_target().is_none());
        assert!(binding.native_compilation_admission.is_none());
        let mut changed = tokens.clone();
        changed.synthetic = Some(crate::ir::SyntheticMarker::EvaluatedArguments);
        assert!(
            binding
                .original_source_procedure_argument_roles(&changed, input, context.commands(),)
                .is_none()
        );
        let mut altered_config = input.lexer_config();
        altered_config.strict_quoting = !altered_config.strict_quoting;
        let altered = ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            input.context_registry(),
            altered_config,
        );
        assert!(
            binding
                .original_source_procedure_argument_roles(&tokens, &altered, context.commands(),)
                .is_none()
        );
    }
}

#[test]
fn original_native_source_procedure_roles_keep_the_genuine_allocation_purpose() {
    // naming.compiler.original-positioned-callee-body-availability
    // docs/design/analysis/name-resolution-proofs/original-positioned-callee-body-availability.md
    // The actual held software Native entry remains distinct from the
    // positive Logical source model. No external Native execution is observed.
    let (_owner, context, input, tokens) =
        original_native_procedure_argument_tokens("proc selected {value} {}; selected VALUE");
    let binding = tokens.source_binding.as_ref().unwrap();
    assert!(!input.has_logical_source_name_context());
    let advice = binding.declaration_call_layout_advice(&tokens).unwrap();
    assert!(
        advice
            .targets()
            .iter()
            .all(|target| target.implementation_allocation.is_some())
    );
    assert!(
        binding
            .original_logical_declaration_call_targets(&tokens, &input, context.commands(),)
            .is_none()
    );
    assert!(
        binding
            .original_source_procedure_argument_roles(&tokens, &input, context.commands(),)
            .is_some()
    );
}

#[test]
fn original_logical_procedure_argv_keeps_unentered_alias_prefix_and_literal_child() {
    // naming.compiler.original-positioned-callee-body-availability
    // docs/design/analysis/name-resolution-proofs/original-positioned-callee-body-availability.md
    // Lexical argv roles retain the real declaration prefix, not a call frame.
    let context = logical_script_context();
    let source = "proc café {prefix value} {}; interp alias {} forwarded {} café PREFIX; return DONE; forwarded {[inert CHILD]}";
    let (analysis, tokens) = logical_original_tokens(source, &context);
    let input = analysis.resolved_input.as_ref().unwrap();
    let binding = tokens.source_binding.as_ref().unwrap();
    assert!(binding.lookup_state.is_none());
    assert!(binding.unknown);
    assert_eq!(
        binding.targets,
        [] as [crate::command_binding::SourceCommandTarget; 0]
    );
    let declared = binding
        .original_logical_declaration_call_targets(&tokens, input, context.commands())
        .unwrap();
    assert_eq!(declared.len(), 1);
    assert_eq!(declared[0].prepended.len(), 1);
    assert_eq!(
        declared[0].prepended[0].literal_bytes(),
        Some(b"PREFIX".as_slice())
    );
    assert!(declared[0].implementation_allocation.is_none());
    assert!(
        binding
            .original_source_procedure_argument_roles(&tokens, input, context.commands())
            .is_some()
    );
    assert_eq!(
        crate::word_subst::checked_argument_lifted_calls(&tokens, input.lexer_config(),).unwrap(),
        [] as [crate::word_subst::LiftedCall; 0]
    );
    assert!(binding.declaration_call_layout_advice(&tokens).is_none());
    assert!(binding.proved_execution_target().is_none());
    assert!(binding.proved_handler_target().is_none());
    assert_ne!(
        binding.runtime_reachability(),
        super::super::super::SourceRuntimeReachability::Reached
    );
    assert_ne!(
        binding.runtime_reachability(),
        super::super::super::SourceRuntimeReachability::Conditional
    );
}

#[test]
fn original_logical_procedure_argv_refuses_replaced_receipt_vector_and_source_world() {
    // naming.compiler.original-positioned-callee-body-availability
    // docs/design/analysis/name-resolution-proofs/original-positioned-callee-body-availability.md
    let context = logical_script_context();
    let source = "proc selected {value} {}; return DONE; selected {LITERAL}";
    let (analysis, tokens) = logical_original_tokens(source, &context);
    let input = analysis.resolved_input.as_ref().unwrap();
    let binding = tokens.source_binding.as_ref().unwrap();
    assert_original_procedure_roles_retained(binding, &tokens, input, context.commands());
    let (_, foreign) = logical_original_tokens(
        "proc different {value} {}; return DONE; different {LITERAL}",
        &context,
    );
    for axis in 0..4 {
        let mut changed = tokens.clone();
        match axis {
            0 => changed.argv_texts[1] = "CHANGED".into(),
            1 => {
                changed
                    .source_binding
                    .as_mut()
                    .unwrap()
                    .original_logical_declaration_targets = foreign
                    .source_binding
                    .as_ref()
                    .unwrap()
                    .original_logical_declaration_targets
                    .clone();
            }
            2 => {
                changed
                    .source_binding
                    .as_mut()
                    .unwrap()
                    .declaration_layout_observations = foreign
                    .source_binding
                    .as_ref()
                    .unwrap()
                    .declaration_layout_observations
                    .clone();
            }
            3 => {
                changed.source_binding.as_mut().unwrap().targets = binding
                    .original_logical_procedure_call_targets(&tokens, input)
                    .unwrap();
            }
            _ => unreachable!(),
        }
        assert_original_procedure_axis_refused(
            changed.source_binding.as_ref().unwrap(),
            &changed,
            input,
            context.commands(),
            axis,
        );
    }
    let (alias_analysis, mut alias_tokens) = logical_original_tokens(
        "proc selected {prefix value} {}; interp alias {} forwarded {} selected PREFIX; forwarded VALUE",
        &context,
    );
    let alias_input = alias_analysis.resolved_input.as_ref().unwrap();
    let alias_binding = alias_tokens.source_binding.as_ref().unwrap();
    assert_original_procedure_roles_retained(
        alias_binding,
        &alias_tokens,
        alias_input,
        context.commands(),
    );
    alias_tokens.source_binding.as_mut().unwrap().targets[0].prepended[0] =
        crate::registry_invocation::EffectiveInvocationWord::Literal("CHANGED".into());
    assert_original_procedure_roles_refused(
        alias_tokens.source_binding.as_ref().unwrap(),
        &alias_tokens,
        alias_input,
        context.commands(),
    );
    let mut config = input.lexer_config();
    config.strict_quoting = !config.strict_quoting;
    let changed_input = ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        input.context_registry(),
        config,
    );
    assert_original_procedure_roles_refused(binding, &tokens, &changed_input, context.commands());
    let foreign_registry = tcl_registry::model::ingress::resolve_known_environment("tcl8.4")
        .unwrap()
        .default_context_registry();
    assert_original_procedure_roles_refused(binding, &tokens, input, foreign_registry.commands());
}

#[test]
fn original_logical_procedure_argv_cannot_repair_open_or_registry_targets() {
    // naming.compiler.original-positioned-callee-body-availability
    // docs/design/analysis/name-resolution-proofs/original-positioned-callee-body-availability.md
    let context = logical_script_context();
    for source in [
        "proc selected {value} {}; unknown; selected VALUE",
        "proc selected {prefix value} {}; interp alias {} forwarded {} selected [unknown]; forwarded VALUE",
        "proc selected {value} {}; return DONE; list VALUE",
    ] {
        let (analysis, tokens) = logical_original_tokens(source, &context);
        let input = analysis.resolved_input.as_ref().unwrap();
        let binding = tokens.source_binding.as_ref().unwrap();
        assert!(
            binding
                .original_source_procedure_argument_roles(&tokens, input, context.commands(),)
                .is_none(),
            "{source}"
        );
        assert!(binding.proved_execution_target().is_none());
        assert!(binding.proved_handler_target().is_none());
    }
}
