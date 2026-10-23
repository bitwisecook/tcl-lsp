// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original source advice follows the retained entry purpose.

use super::*;
use crate::analyser::ResolvedAnalysisInput;
use crate::command_binding::{
    OriginalInvocationSourcePurpose, SourceAnalysisEntry, SourceCommandBindings,
};
use std::sync::Arc;

struct Fixture {
    input: ResolvedAnalysisInput,
    entry: SourceAnalysisEntry,
    bindings: SourceCommandBindings,
    tokens: CommandTokens,
    _owner: Option<tcl_vm::Vm>,
}

fn from_entry(
    source: &str,
    input: ResolvedAnalysisInput,
    entry: SourceAnalysisEntry,
    owner: Option<tcl_vm::Vm>,
) -> Fixture {
    let image = tcl_lexer::SourceImage::document(source);
    let config = input.lexer_config();
    let registry = input.borrowed_context_registry().commands();
    let bindings =
        SourceCommandBindings::analyse_with_options(source, config, registry, entry.options());
    let segment =
        crate::segmenter::segment_commands_image_with_offset_and_config(&image, 0, config)
            .unwrap()
            .pop()
            .unwrap();
    let mut tokens = CommandTokens::from_segmented(&image.source_map(), config, &segment);
    bindings.stamp_original_tokens(&mut tokens);
    Fixture {
        input,
        entry,
        bindings,
        tokens,
        _owner: owner,
    }
}

fn native(source: &str, environment: &str) -> Fixture {
    let environment = tcl_registry::model::ingress::resolve_known_environment(environment).unwrap();
    let profile = environment.unit_profile();
    let input = ResolvedAnalysisInput::new(
        profile,
        profile,
        environment.default_context_registry(),
        tcl_lexer::LexerConfig::for_profile(Some(profile)),
    );
    let (owner, held) = crate::environment_ingress::captured_native_entry_with_owner(profile);
    let entry = SourceAnalysisEntry {
        metadata_context: OwnedInvocationMetadataContext::for_source_input(Some(&input)),
        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
        native_compilation: crate::environment_ingress::authoring_native_compilation(),
        native_entry: Some(Arc::new(held)),
        ..Default::default()
    };
    from_entry(source, input, entry, Some(owner))
}

fn logical(source: &str) -> Fixture {
    let context = tcl_registry::model::ingress::resolve_known_environment("tcl9.1")
        .unwrap()
        .default_context_registry();
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let input = ResolvedAnalysisInput::new(
        profile,
        profile,
        context,
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    let entry = SourceAnalysisEntry::for_logical_source(&input).unwrap();
    from_entry(source, input, entry, None)
}

#[test]
fn original_native_jim_source_advice_cannot_borrow_raw_logical_input_eligibility() {
    // naming.compiler.original-analysis-metadata-context

    // Actual held source purpose only, independently of execution permission.
    let fixture = native("if {1} {set value VALUE}", "jimtcl");
    let registry = fixture.input.borrowed_context_registry().commands();
    let metadata = InvocationMetadataContext::for_analysis_input(registry, &fixture.input).unwrap();
    assert!(metadata.permits_logical_source_names());
    let binding = fixture.tokens.source_binding.as_ref().unwrap();
    assert_eq!(
        binding.original_source_advice_purpose(&fixture.tokens, metadata, registry),
        Some(OriginalInvocationSourcePurpose::Execution)
    );
    assert!(
        logical_structured_invocation_with_metadata_context(
            registry,
            metadata,
            &fixture.tokens,
            Some(&fixture.bindings),
        )
        .is_some()
    );
    assert!(
        original_logical_operation_invocation_with_metadata_context(
            registry,
            metadata,
            &fixture.tokens,
        )
        .is_none()
    );
}

#[test]
fn original_logical_source_body_roles_keep_their_real_entry_purpose() {
    // naming.compiler.original-analysis-metadata-context

    // Conditional source roles issue no Native handler or compiler owner.
    let fixture = logical("if {1} {set value VALUE}");
    let registry = fixture.input.borrowed_context_registry().commands();
    let metadata = InvocationMetadataContext::for_analysis_input(registry, &fixture.input).unwrap();
    let binding = fixture.tokens.source_binding.as_ref().unwrap();
    assert_eq!(
        binding.original_source_advice_purpose(&fixture.tokens, metadata, registry),
        Some(OriginalInvocationSourcePurpose::Logical)
    );
    let structure = logical_structured_invocation_with_metadata_context(
        registry,
        metadata,
        &fixture.tokens,
        Some(&fixture.bindings),
    )
    .unwrap();
    assert!(
        structure
            .written_roles()
            .contains(&(1, tcl_registry::ArgRole::Body))
    );
    assert!(
        conditional_body_topology_advice_with_metadata_context(
            registry,
            metadata,
            &fixture.tokens,
            &fixture.bindings,
        )
        .is_some()
    );
    assert!(binding.proved_handler_target().is_none());
    assert!(binding.native_compilation_admission.is_none());
}

#[test]
fn original_native_unentered_declaration_keeps_geometry_without_invocation_permission() {
    // naming.compiler.original-analysis-metadata-context

    // A Native request's lexical layout remains distinct from actual entry.
    let fixture = native("return DONE; if {1} {set value VALUE}", "tcl8.4");
    let registry = fixture.input.borrowed_context_registry().commands();
    let metadata = InvocationMetadataContext::for_analysis_input(registry, &fixture.input).unwrap();
    let binding = fixture.tokens.source_binding.as_ref().unwrap();
    assert!(binding.invocation_site().is_some());
    assert!(binding.lookup_state.is_none());
    assert!(
        binding
            .original_invocation_metadata_for_input(&fixture.tokens, &fixture.input, registry,)
            .is_none()
    );
    assert_eq!(
        binding.original_source_advice_purpose(&fixture.tokens, metadata, registry),
        Some(OriginalInvocationSourcePurpose::Execution)
    );
    assert!(
        logical_structured_invocation_with_metadata_context(
            registry,
            metadata,
            &fixture.tokens,
            Some(&fixture.bindings),
        )
        .is_some()
    );
    assert!(binding.proved_execution_target().is_none());
    assert!(binding.proved_handler_target().is_none());
    assert!(
        normal_transfer_invocation_in_context(
            fixture.input.borrowed_context_registry(),
            &fixture.tokens,
        )
        .is_none()
    );
}

#[test]
fn original_source_advice_purpose_refuses_changed_input_vector_and_missing_owner() {
    // naming.compiler.original-analysis-metadata-context

    // Eligibility flags cannot recover a foreign or absent original owner.
    for fixture in [
        native("if {1} {set value VALUE}", "jimtcl"),
        logical("if {1} {set value VALUE}"),
    ] {
        let registry = fixture.input.borrowed_context_registry().commands();
        let metadata =
            InvocationMetadataContext::for_analysis_input(registry, &fixture.input).unwrap();
        let binding = fixture.tokens.source_binding.as_ref().unwrap();
        let mut changed_config = fixture.input.lexer_config();
        changed_config.strict_quoting = !changed_config.strict_quoting;
        let changed = ResolvedAnalysisInput::new(
            fixture.input.analyser_profile(),
            fixture.input.unit_profile(),
            fixture.input.context_registry(),
            changed_config,
        );
        let changed_metadata =
            InvocationMetadataContext::for_analysis_input(registry, &changed).unwrap();
        assert!(changed_metadata.permits_logical_source_names());
        assert!(
            binding
                .original_source_advice_purpose(&fixture.tokens, changed_metadata, registry)
                .is_none()
        );
        assert!(
            logical_structured_invocation_with_metadata_context(
                registry,
                changed_metadata,
                &fixture.tokens,
                Some(&fixture.bindings),
            )
            .is_none()
        );
        let mut changed_vector = fixture.tokens.clone();
        changed_vector.argv_texts[0].push('!');
        assert!(
            changed_vector
                .source_binding
                .as_ref()
                .unwrap()
                .original_source_advice_purpose(&changed_vector, metadata, registry)
                .is_none()
        );
        assert!(
            logical_structured_invocation_with_metadata_context(
                registry,
                metadata,
                &changed_vector,
                Some(&fixture.bindings),
            )
            .is_none()
        );
        let mut missing = fixture.tokens.clone();
        missing.source_binding = None;
        assert!(
            logical_structured_invocation_with_metadata_context(
                registry,
                metadata,
                &missing,
                Some(&fixture.bindings),
            )
            .is_none()
        );
    }
}

#[test]
fn original_source_advice_retains_explicit_authored_simulation_separately() {
    // naming.compiler.original-analysis-metadata-context

    // An explicit authored recipe is source policy, never a Native entry.
    let environment = tcl_registry::model::ingress::resolve_known_environment("tcl8.4").unwrap();
    let profile = environment.unit_profile();
    let input = ResolvedAnalysisInput::new(
        profile,
        profile,
        environment.default_context_registry(),
        tcl_lexer::LexerConfig::for_profile(Some(profile)),
    );
    let policy = tcl_syntax::naming::ExecutionNamePolicy::NativeRecipe(
        tcl_syntax::naming::NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_4),
    );
    let entry = SourceAnalysisEntry {
        metadata_context: OwnedInvocationMetadataContext::for_source_input(Some(&input)),
        execution_name_policy: Some(policy),
        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
        native_compilation: crate::environment_ingress::authoring_native_compilation(),
        ..Default::default()
    };
    let source = "return DONE; if {1} {set value VALUE}";
    let fixture = from_entry(source, input, entry, None);
    let registry = fixture.input.borrowed_context_registry().commands();
    let metadata = InvocationMetadataContext::for_analysis_input(registry, &fixture.input).unwrap();
    let binding = fixture.tokens.source_binding.as_ref().unwrap();
    let advice = binding
        .declaration_operand_layout_advice(&fixture.tokens)
        .unwrap();
    assert!(binding.lookup_state.is_none());
    assert_eq!(
        advice.original_source_purpose(metadata, registry),
        Some(OriginalInvocationSourcePurpose::AuthoredSimulation)
    );
    assert_eq!(
        binding.original_source_advice_purpose(&fixture.tokens, metadata, registry),
        Some(OriginalInvocationSourcePurpose::AuthoredSimulation)
    );
    assert!(fixture.entry.native_entry.is_none());
    assert!(fixture.entry.logical_source_input.is_none());
    assert_eq!(fixture.entry.execution_name_policy, Some(policy));
    assert!(
        original_logical_operation_invocation_with_metadata_context(
            registry,
            metadata,
            &fixture.tokens,
        )
        .is_none()
    );
    assert!(
        binding
            .original_invocation_source_purpose(&fixture.tokens, metadata, registry)
            .is_none()
    );
    let mut config = fixture.input.lexer_config();
    config.strict_quoting = !config.strict_quoting;
    let changed = ResolvedAnalysisInput::new(
        fixture.input.analyser_profile(),
        fixture.input.unit_profile(),
        fixture.input.context_registry(),
        config,
    );
    let foreign = InvocationMetadataContext::for_analysis_input(registry, &changed).unwrap();
    assert!(advice.original_source_purpose(foreign, registry).is_none());
    let mut missing = fixture.tokens.clone();
    missing.source_binding = None;
    assert!(
        logical_structured_invocation_with_metadata_context(
            registry,
            metadata,
            &missing,
            Some(&fixture.bindings),
        )
        .is_none()
    );
}
