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
use crate::analyser::ResolvedAnalysisInput;
use crate::registry_invocation::InvocationMetadataContext;

fn input() -> ResolvedAnalysisInput {
    crate::analyser::Analyser::new()
        .prepare_analysis_input("tcl")
        .unwrap()
}

fn original(source: &str, input: &ResolvedAnalysisInput) -> crate::ir::CommandTokens {
    let image = tcl_lexer::SourceImage::document(source);
    let config = input.lexer_config();
    let bindings = SourceCommandBindings::analyse_image_in_frame_with_options(
        &image,
        &crate::var_resolve::VariableExecutionFrame::Unknown,
        config,
        input.borrowed_context_registry().commands(),
        SourceAnalysisOptions::for_logical_source(input).unwrap(),
    )
    .unwrap();
    let segment =
        crate::segmenter::segment_commands_image_with_offset_and_config(&image, 0, config)
            .unwrap()
            .pop()
            .unwrap();
    let mut tokens =
        crate::ir::CommandTokens::from_segmented(&image.source_map(), config, &segment);
    bindings.stamp_original_tokens(&mut tokens);
    tokens
}

#[test]
fn original_logical_reached_roles_distinguish_literal_and_expression_inventory() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Actual original source roles; no Native parser, compiler or handler grant.
    let input = input();
    let registry = input.borrowed_context_registry().commands();
    let metadata = InvocationMetadataContext::for_analysis_input(registry, &input).unwrap();
    let literal = original("puts {[dict size DATA]}", &input);
    let binding = literal.source_binding.as_ref().unwrap();
    assert!(binding.lookup_state.is_some());
    assert!(binding.compiler_lookup_state.is_none());
    assert!(binding.declaration_layout_observations.is_none());
    assert!(binding.original_compiler_words.is_none());
    assert_eq!(
        binding.variable_frame,
        crate::var_resolve::VariableExecutionFrame::Unknown
    );
    let advice = binding
        .original_conditional_operand_layout_advice(&literal, registry)
        .unwrap();
    assert!(advice.closed_logical_source_lookup());
    assert!(!advice.closed_lookup());
    assert_eq!(advice.targets(), binding.targets.as_slice());
    assert_eq!(
        crate::word_subst::checked_original_lifted_calls_with_metadata_context(
            &literal,
            input.lexer_config(),
            registry,
            metadata,
        )
        .unwrap(),
        [] as [crate::word_subst::LiftedCall; 0]
    );
    let expression = original("expr {[dict size DATA]}", &input);
    let calls = crate::word_subst::checked_original_lifted_calls_with_metadata_context(
        &expression,
        input.lexer_config(),
        registry,
        metadata,
    )
    .unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].command, "dict");
    assert_eq!(calls[0].args, ["size", "DATA"]);
    assert!(binding.proved_execution_target().is_none());
    assert!(binding.proved_handler_target().is_none());
    assert!(binding.native_compiler_admission.is_none());
}

#[test]
fn original_logical_reached_roles_refuse_unknown_observers_and_rebound_tokens() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let input = input();
    let registry = input.borrowed_context_registry().commands();
    for source in [
        "unknown; puts {[dict size DATA]}",
        "rename puts retired; puts {[dict size DATA]}",
        "proc puts {value} {}; puts {[dict size DATA]}",
        "puts [rename puts retired; proc puts {value} {}]",
    ] {
        let tokens = original(source, &input);
        let binding = tokens.source_binding.as_ref().unwrap();
        assert!(binding.lookup_state.is_some(), "{source}");
        assert!(
            binding
                .original_logical_source_operand_layout_advice(&tokens, registry)
                .is_none(),
            "{source}"
        );
        assert!(binding.proved_handler_target().is_none());
    }
}

#[test]
fn original_logical_reached_roles_require_same_scope_source_targets_and_vector() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let input = input();
    let registry = input.borrowed_context_registry().commands();
    let tokens = original("puts {[dict size DATA]}", &input);
    let binding = tokens.source_binding.as_ref().unwrap();
    assert!(
        binding
            .original_logical_source_operand_layout_advice(&tokens, registry)
            .is_some()
    );
    let mut changed = tokens.clone();
    changed.argv_texts[0].push('!');
    assert!(
        binding
            .original_logical_source_operand_layout_advice(&changed, registry)
            .is_none()
    );
    let mut foreign = binding.clone();
    Arc::make_mut(&mut foreign.variable_context).namespace = "::foreign".into();
    assert!(
        foreign
            .original_logical_source_operand_layout_advice(&tokens, registry)
            .is_none()
    );
    let mut namespace = binding.clone();
    namespace.lookup_namespace_key = SourceNamespaceKey::Authored("::foreign".into());
    namespace.lookup_namespace = "::foreign".into();
    assert!(
        namespace
            .original_logical_source_operand_layout_advice(&tokens, registry)
            .is_none()
    );
    let mut missing = binding.clone();
    missing.targets.clear();
    assert!(
        missing
            .original_logical_source_operand_layout_advice(&tokens, registry)
            .is_none()
    );
    let mut source = binding.clone();
    Arc::make_mut(source.lookup_state.as_mut().unwrap())
        .state
        .current_source_origin = None;
    assert!(
        source
            .original_logical_source_operand_layout_advice(&tokens, registry)
            .is_none()
    );
    let mut snapshot = binding.lookup_state.as_ref().unwrap().as_ref().clone();
    snapshot.state.current_source_origin = None;
    let mut refused_compiler = binding.clone();
    refused_compiler.compiler_lookup_state = Some(Arc::new(snapshot));
    assert!(
        refused_compiler
            .original_compilation_lookup_advice(&tokens)
            .is_none()
    );
    assert!(
        refused_compiler
            .original_conditional_operand_layout_advice(&tokens, registry)
            .is_none()
    );
}

#[test]
fn original_logical_reached_roles_keep_full_metadata_and_selected_availability() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let input = input();
    let registry = input.borrowed_context_registry().commands();
    let tokens = original("puts {[dict size DATA]}", &input);
    let current = InvocationMetadataContext::for_analysis_input(registry, &input).unwrap();
    assert!(
        crate::registry_invocation::original_logical_operation_invocation_with_metadata_context(
            registry, current, &tokens,
        )
        .is_some()
    );
    let mut config = input.lexer_config();
    config.strict_quoting = !config.strict_quoting;
    let changed = ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        input.context_registry(),
        config,
    );
    let changed = InvocationMetadataContext::for_analysis_input(registry, &changed).unwrap();
    assert!(
        crate::registry_invocation::original_logical_operation_invocation_with_metadata_context(
            registry, changed, &tokens,
        )
        .is_none()
    );
    let foreign = tcl_registry::model::ingress::resolve_known_environment("tcl8.4")
        .unwrap()
        .default_context_registry();
    let foreign = ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        foreign,
        input.lexer_config(),
    );
    assert!(InvocationMetadataContext::for_analysis_input(registry, &foreign).is_none());
    let unbound = original("missing {[dict size DATA]}", &input);
    assert!(
        unbound
            .source_binding
            .as_ref()
            .unwrap()
            .original_logical_source_operand_layout_advice(&unbound, registry)
            .is_none()
    );
}

#[test]
fn original_native_unknown_frame_cannot_borrow_logical_source_layout() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let environment = tcl_registry::model::ingress::resolve_known_environment("tcl9.0").unwrap();
    let profile = environment.unit_profile();
    let context = environment.default_context_registry();
    let (owner, entry) = crate::environment_ingress::captured_native_entry_with_owner(profile);
    let config = SourceAnalysisOptions {
        native_entry: Some(&entry),
        ..Default::default()
    }
    .native_lexer_config(tcl_lexer::LexerConfig::for_file_grammar(profile.grammar));
    let input = ResolvedAnalysisInput::new(profile, profile, Arc::clone(&context), config);
    assert!(SourceAnalysisOptions::for_logical_source(&input).is_none());
    let image = tcl_lexer::SourceImage::document("puts {[dict size DATA]}");
    let bindings = SourceCommandBindings::analyse_image_in_frame_with_options(
        &image,
        &crate::var_resolve::VariableExecutionFrame::Unknown,
        config,
        context.commands(),
        SourceAnalysisOptions {
            native_entry: Some(&entry),
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
            metadata_context: crate::registry_invocation::InvocationMetadataInput::SuppliedSource(
                Some(&input),
            ),
            ..Default::default()
        },
    )
    .unwrap();
    let segment =
        crate::segmenter::segment_commands_image_with_offset_and_config(&image, 0, config)
            .unwrap()
            .pop()
            .unwrap();
    let mut tokens =
        crate::ir::CommandTokens::from_segmented(&image.source_map(), config, &segment);
    bindings.stamp_original_tokens(&mut tokens);
    let binding = tokens.source_binding.as_ref().unwrap();
    assert!(binding.logical_source_name_advice_input().is_none());
    assert!(
        binding
            .original_logical_source_operand_layout_advice(&tokens, context.commands())
            .is_none()
    );
    assert!(binding.proved_handler_target().is_none());
    assert_eq!(entry.execution_point, environment.point());
    drop(owner);
}
