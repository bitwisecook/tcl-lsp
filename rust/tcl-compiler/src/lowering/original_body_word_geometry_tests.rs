// SPDX-License-Identifier: AGPL-3.0-or-later
//! Authentic original body geometry, separately from entered lookup.

use super::*;
use crate::analyser::ResolvedAnalysisInput;
use crate::command_binding::{ExecutedScriptSource, SourceAnalysisEntry, SourceNamespaceKey};
use crate::registry_invocation::{InvocationMetadataContext, OwnedInvocationMetadataContext};
use std::sync::Arc;

struct Fixture {
    context: Arc<tcl_registry::model::ContextRegistry>,
    input: ResolvedAnalysisInput,
    entry: SourceAnalysisEntry,
    bindings: SourceCommandBindings,
    body: ExecutedScriptSource,
    namespace: SourceNamespaceKey,
    _owner: tcl_vm::Vm,
}

fn fixture(environment: &str) -> Fixture {
    fixture_with_source(environment, "if {$condition} {set result VALUE}", 0)
}

fn unentered_fixture(environment: &str) -> Fixture {
    fixture_with_source(environment, "return DONE; if {$condition} {set result VALUE}", 1)
}

fn fixture_with_source(environment: &str, source: &str, parent: usize) -> Fixture {
    let environment = tcl_registry::model::ingress::resolve_known_environment(environment).unwrap();
    let context = environment.default_context_registry();
    let profile = environment.unit_profile();
    let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
    let input = ResolvedAnalysisInput::new(profile, profile, Arc::clone(&context), config);
    let (owner, native) = crate::environment_ingress::captured_native_entry_with_owner(profile);
    let entry = SourceAnalysisEntry {
        metadata_context: OwnedInvocationMetadataContext::for_source_input(Some(&input)),
        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
        native_compilation: crate::environment_ingress::authoring_native_compilation(),
        native_entry: Some(Arc::new(native)),
        ..Default::default()
    };
    let bindings = SourceCommandBindings::analyse_with_options(
        source,
        config,
        context.commands(),
        entry.options(),
    );
    let image = tcl_lexer::SourceImage::document(source);
    let segments =
        crate::segmenter::segment_commands_image_with_offset_and_config(&image, 0, config).unwrap();
    let mut parent = CommandTokens::from_segmented(&image.source_map(), config, &segments[parent]);
    bindings.stamp_original_tokens(&mut parent);
    let binding = parent.source_binding.as_ref().unwrap();
    let body = binding
        .original_body_source_for_word(&parent, parent.words()[2].source().span, config)
        .unwrap();
    let request =
        InvocationMetadataContext::for_analysis_input(context.commands(), &input).unwrap();
    let namespace = binding
        .original_body_namespace_context(context.commands(), request, &parent, &body)
        .unwrap();
    Fixture {
        context,
        input,
        entry,
        bindings,
        body,
        namespace,
        _owner: owner,
    }
}

fn original_body_tokens(fixture: &Fixture) -> CommandTokens {
    let mut tokens = unbound_body_tokens(fixture);
    fixture.bindings.stamp_original_tokens(&mut tokens);
    tokens
}

fn unbound_body_tokens(fixture: &Fixture) -> CommandTokens {
    let config = fixture.input.lexer_config();
    let segments = crate::segmenter::segment_commands_image_with_offset_and_config(
        &fixture.body.text,
        fixture.body.base(),
        config.nested(),
    )
    .unwrap();
    let words = WordSpace::new(
        fixture.body.try_text().unwrap(),
        fixture.body.base(),
        fixture.body.text.channel(),
    );
    let mut tokens = CommandTokens::from_segmented(&words.map(), config, &segments[0]);
    // Construct the original parser vector before any declaration/reached stamp.
    // Explicit unknown is a missing receipt, not a replacement for a real one.
    tokens.source_binding = Some(crate::command_binding::SourceInvocationBinding::unknown());
    tokens
}

#[test]
fn original_conditional_body_retains_written_geometry_without_lookup_or_native_entry() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Actual held VM issuer; abrupt completion keeps this child unentered.
    for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
        let fixture = unentered_fixture(environment);
        let mut lowerer =
            Lowerer::with_config(fixture.context.commands(), fixture.input.lexer_config())
                .with_dialect(Some(fixture.input.unit_profile()))
                .with_resolved_analysis_input(fixture.input.clone());
        lowerer.set_source_analysis_options(fixture.entry.options());
        lowerer.source_bindings = Some(fixture.bindings.clone().into());
        let script = lowerer.lower_original_body(fixture.body.clone(), Some(&fixture.namespace));
        assert_eq!(
            script.namespace_context.as_deref(),
            Some(&fixture.namespace),
            "{environment}"
        );
        assert_eq!(script.statements.len(), 1, "{environment}");
        let tokens = script
            .retained_source_tokens_for_statement(&script.statements[0])
            .unwrap();
        assert_eq!(
            tokens.argv_texts,
            ["set", "result", "VALUE"],
            "{environment}"
        );
        let binding = tokens.source_binding.as_ref().unwrap();
        assert_eq!(
            binding.original_lexer_config_for_tokens(tokens),
            Some(fixture.input.lexer_config()),
            "{environment}"
        );
        assert_eq!(
            binding.invocation_site().unwrap().source,
            fixture.body.origin,
            "{environment}"
        );
        assert!(binding.proved_execution_target().is_none(), "{environment}");
        assert!(binding.lookup_state.is_none(), "{environment}");
        assert!(binding.compiler_lookup_state.is_none(), "{environment}");
        assert!(binding.native_compilation_admission.is_none(), "{environment}");
        assert!(
            crate::registry_invocation::normal_transfer_invocation_in_context(
                &fixture.context, tokens,
            ).is_none(),
            "{environment}"
        );
        assert!(
            binding
                .original_invocation_metadata_for_input(
                    tokens,
                    &fixture.input,
                    fixture.context.commands()
                )
                .is_none(),
            "{environment}"
        );
        assert!(lowerer.original_body_word_source.is_none(), "{environment}");
    }
}

#[test]
fn original_body_word_geometry_refuses_foreign_input_source_config_and_vector() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let fixture = fixture("tcl8.4");
    let config = fixture.input.lexer_config();
    // The geometry-only issuer must receive the actual pre-stamp vector.
    let mut original = unbound_body_tokens(&fixture);
    let owner = fixture
        .bindings
        .original_body_written_word_owner(
            &fixture.body,
            config,
            fixture.context.commands(),
            &fixture.input,
        )
        .unwrap();
    assert!(
        original
            .source_binding
            .as_ref()
            .unwrap()
            .invocation_site()
            .is_none()
    );
    assert!(
        owner
            .stamp_original_tokens(fixture.context.commands(), &fixture.input, &mut original)
            .is_some()
    );
    let binding = original.source_binding.as_ref().unwrap();
    assert_eq!(
        binding.original_lexer_config_for_tokens(&original),
        Some(config)
    );
    assert!(binding.unknown);
    assert!(binding.targets.is_empty());
    assert!(binding.lookup_state.is_none());
    assert!(binding.compiler_lookup_state.is_none());
    assert!(
        binding
            .original_source_word_dialect_for_tokens(&original)
            .is_none()
    );
    let mut changed_config = config;
    changed_config.strict_quoting = !config.strict_quoting;
    let changed_input = ResolvedAnalysisInput::new(
        fixture.input.analyser_profile(),
        fixture.input.unit_profile(),
        fixture.input.context_registry(),
        changed_config,
    );
    let foreign_context = tcl_registry::model::ingress::resolve_known_environment("tcl8.5")
        .unwrap()
        .default_context_registry();
    let foreign_input = ResolvedAnalysisInput::new(
        fixture.input.analyser_profile(),
        fixture.input.unit_profile(),
        Arc::clone(&foreign_context),
        config,
    );
    for (input, registry) in [
        (&changed_input, fixture.context.commands()),
        (&foreign_input, fixture.context.commands()),
        (&fixture.input, foreign_context.commands()),
    ] {
        let mut tokens = unbound_body_tokens(&fixture);
        assert!(
            owner
                .stamp_original_tokens(registry, input, &mut tokens)
                .is_none()
        );
        assert!(
            tokens
                .source_binding
                .as_ref()
                .unwrap()
                .invocation_site()
                .is_none()
        );
        assert!(
            fixture
                .bindings
                .original_body_written_word_owner(&fixture.body, config, registry, input)
                .is_none()
        );
    }
    assert!(
        fixture
            .bindings
            .original_body_written_word_owner(
                &fixture.body,
                changed_config,
                fixture.context.commands(),
                &fixture.input
            )
            .is_none()
    );
    let mut changed_span = unbound_body_tokens(&fixture);
    crate::lattice_rebase::rebase_command_tokens(&mut changed_span, 1);
    let mut changed_vector = unbound_body_tokens(&fixture);
    changed_vector.argv_texts[0].push('!');
    let mut missing = unbound_body_tokens(&fixture);
    missing.source_binding = None;
    for mut tokens in [changed_span, changed_vector, missing] {
        assert!(
            owner
                .stamp_original_tokens(fixture.context.commands(), &fixture.input, &mut tokens)
                .is_none()
        );
    }
    let mut changed_source = fixture.body.clone();
    changed_source.text = tcl_lexer::SourceImage::native(b"set result OTHER".as_slice());
    assert!(
        fixture
            .bindings
            .original_body_written_word_owner(
                &changed_source,
                config,
                fixture.context.commands(),
                &fixture.input
            )
            .is_none()
    );
    let mut changed_channel = fixture.body.clone();
    changed_channel.text = tcl_lexer::SourceImage::document(fixture.body.try_text().unwrap());
    assert!(
        fixture
            .bindings
            .original_body_written_word_owner(
                &changed_channel,
                config,
                fixture.context.commands(),
                &fixture.input
            )
            .is_none()
    );
    let foreign_source = ExecutedScriptSource::contiguous(
        Arc::new(crate::command_binding::SourceOriginId::authored(
            &Arc::from("set result VALUE"),
        )),
        "set result VALUE",
        0,
    )
    .unwrap();
    assert!(
        fixture
            .bindings
            .original_body_written_word_owner(
                &foreign_source,
                config,
                fixture.context.commands(),
                &fixture.input
            )
            .is_none()
    );
}

#[test]
fn original_body_geometry_is_absent_without_namespace_descent_or_retained_owner() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let fixture = fixture("tcl8.4");
    let mut lowerer =
        Lowerer::with_config(fixture.context.commands(), fixture.input.lexer_config())
            .with_dialect(Some(fixture.input.unit_profile()))
            .with_resolved_analysis_input(fixture.input.clone());
    lowerer.set_source_analysis_options(fixture.entry.options());
    lowerer.source_bindings = Some(fixture.bindings.clone().into());
    let missing_namespace = lowerer.lower_original_body(fixture.body.clone(), None);
    assert!(
        matches!(&missing_namespace.statements[0], Statement::Barrier { reason, .. }
        if reason == "original body namespace context is unavailable")
    );
    assert!(lowerer.original_body_word_source.is_none());
    lowerer.source_bindings = None;
    let missing_owner = lowerer.lower_original_body(fixture.body.clone(), Some(&fixture.namespace));
    assert!(
        matches!(&missing_owner.statements[0], Statement::Barrier { reason, .. }
        if reason == "original body source admission is unavailable")
    );
    assert!(lowerer.original_body_word_source.is_none());
    // A structural namespace memo remains insufficient outside authentic body
    // descent: its declared location supplies no entered or compiler point.
    lowerer.source_bindings = Some(fixture.bindings.clone().into());
    let selected = fixture
        .bindings
        .selected_source_in_context(&fixture.body, &fixture.namespace)
        .unwrap();
    let script = lowerer.lower_into_script_with_bindings(
        fixture.body.try_text().unwrap(),
        fixture.body.base(),
        &fixture.namespace.display().unwrap(),
        selected,
    );
    let tokens = script
        .retained_source_tokens_for_statement(&script.statements[0])
        .unwrap();
    let binding = tokens.source_binding.as_ref().unwrap();
    assert!(binding.invocation_site().is_some());
    assert!(binding.declaration_operand_layout_advice(tokens).is_some());
    assert!(binding.lookup_state.is_none());
    assert!(binding.compiler_lookup_state.is_none());
    assert!(binding.proved_execution_target().is_none());
    assert!(binding.proved_handler_target().is_none());
    assert!(!binding.unobserved_native_dispatch());
    assert!(binding.native_compilation_admission.is_none());
    assert!(binding.admitted_inline_invocation().is_none());
    assert!(binding.admitted_named_invocation().is_none());
    assert!(binding.source_origin().is_none());
    assert!(binding.invocation_realm().is_none());
    assert!(
        binding.original_invocation_metadata_for_input(
            tokens, &fixture.input, fixture.context.commands(),
        ).is_none()
    );
    assert!(
        crate::registry_invocation::resolved_handler_invocation(
            fixture.context.commands(), None, tokens,
        ).is_none()
    );
    assert!(
        crate::registry_invocation::normal_transfer_invocation_in_context(
            &fixture.context, tokens,
        ).is_none()
    );
}

#[test]
fn original_body_geometry_cannot_overwrite_an_actual_declaration_owner() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // The real declaration and its location remain intact after geometry refuses.
    let fixture = fixture("tcl8.4");
    let mut original = original_body_tokens(&fixture);
    let retained = original.clone();
    let binding = original.source_binding.as_ref().unwrap();
    assert!(binding.invocation_site().is_some());
    assert!(binding.declaration_operand_layout_advice(&original).is_some());
    let owner = fixture.bindings.original_body_written_word_owner(
        &fixture.body, fixture.input.lexer_config(), fixture.context.commands(),
        &fixture.input,
    ).unwrap();
    assert!(owner.stamp_original_tokens(
        fixture.context.commands(), &fixture.input, &mut original,
    ).is_none());
    assert_eq!(original, retained);
    let binding = original.source_binding.as_ref().unwrap();
    assert!(binding.lookup_state.is_none());
    assert!(binding.compiler_lookup_state.is_none());
    assert!(binding.proved_execution_target().is_none());
    assert!(binding.proved_handler_target().is_none());
    assert!(!binding.unobserved_native_dispatch());
    assert!(binding.native_compilation_admission.is_none());
    assert!(binding.admitted_inline_invocation().is_none());
    assert!(binding.admitted_named_invocation().is_none());
    assert!(crate::registry_invocation::normal_transfer_invocation_in_context(
        &fixture.context, &original,
    ).is_none());
}

#[test]
fn original_jim_body_source_point_remains_without_a_proved_execution_target() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Original held Jim source correspondence is distinct from executable proof.
    let fixture = fixture("jimtcl");
    let mut lowerer = Lowerer::with_config(
        fixture.context.commands(), fixture.input.lexer_config(),
    ).with_dialect(Some(fixture.input.unit_profile()))
        .with_resolved_analysis_input(fixture.input.clone());
    lowerer.set_source_analysis_options(fixture.entry.options());
    lowerer.source_bindings = Some(fixture.bindings.clone().into());
    let script = lowerer.lower_original_body(fixture.body.clone(), Some(&fixture.namespace));
    assert_eq!(script.namespace_context.as_deref(), Some(&fixture.namespace));
    assert_eq!(script.statements.len(), 1);
    let tokens = script.retained_source_tokens_for_statement(&script.statements[0]).unwrap();
    assert_eq!(tokens.argv_texts, ["set", "result", "VALUE"]);
    let binding = tokens.source_binding.as_ref().unwrap();
    assert_eq!(binding.invocation_site().unwrap().source, fixture.body.origin);
    assert_eq!(binding.original_lexer_config_for_tokens(tokens), Some(fixture.input.lexer_config()));
    assert!(binding.lookup_state.is_some());
    assert!(binding.original_invocation_metadata_for_input(
        tokens, &fixture.input, fixture.context.commands(),
    ).is_some());
    assert!(binding.proved_execution_target().is_none());
    assert!(lowerer.original_body_word_source.is_none());
}

fn folded_fixture(environment: &str) -> (Fixture, CommandTokens) {
    let source = "if {$condition} {set result \\\n café}";
    let environment = tcl_registry::model::ingress::resolve_known_environment(environment).unwrap();
    let context = environment.default_context_registry();
    let profile = environment.unit_profile();
    let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
    let input = ResolvedAnalysisInput::new(profile, profile, Arc::clone(&context), config);
    let (owner, native) = crate::environment_ingress::captured_native_entry_with_owner(profile);
    let entry = SourceAnalysisEntry {
        metadata_context: OwnedInvocationMetadataContext::for_source_input(Some(&input)),
        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
        native_compilation: crate::environment_ingress::authoring_native_compilation(),
        native_entry: Some(Arc::new(native)),
        ..Default::default()
    };
    let bindings = SourceCommandBindings::analyse_with_options(
        source,
        config,
        context.commands(),
        entry.options(),
    );
    let image = tcl_lexer::SourceImage::document(source);
    let segments =
        crate::segmenter::segment_commands_image_with_offset_and_config(&image, 0, config).unwrap();
    let mut parent = CommandTokens::from_segmented(&image.source_map(), config, &segments[0]);
    bindings.stamp_original_tokens(&mut parent);
    let binding = parent.source_binding.as_ref().unwrap();
    let body = binding
        .original_body_source_for_word(&parent, parent.words()[2].source().span, config)
        .unwrap();
    let request =
        InvocationMetadataContext::for_analysis_input(context.commands(), &input).unwrap();
    let namespace = binding
        .original_body_namespace_context(context.commands(), request, &parent, &body)
        .unwrap();
    (
        Fixture {
            context,
            input,
            entry,
            bindings,
            body,
            namespace,
            _owner: owner,
        },
        parent,
    )
}

#[test]
fn original_derived_body_geometry_retains_the_exact_parent_word_and_namespace() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Authentic held provider and original source projection; syntax only.
    for environment in ["tcl8.4", "tcl9.0"] {
        let (fixture, parent) = folded_fixture(environment);
        assert!(matches!(
            fixture.body.origin.kind(),
            crate::command_binding::SourceOriginKind::Derived { .. }
        ));
        assert_ne!(fixture.bindings.source_origin(), Some(&fixture.body.origin));
        assert!(
            fixture
                .bindings
                .original_body_written_word_owner(
                    &fixture.body,
                    fixture.input.lexer_config(),
                    fixture.context.commands(),
                    &fixture.input
                )
                .is_none()
        );
        let _geometry_owner = fixture
            .bindings
            .original_body_written_word_owner_at_parent(
                &fixture.body,
                fixture.input.lexer_config(),
                fixture.context.commands(),
                &fixture.input,
                &parent,
            )
            .expect("exact original materialized parent word");
        let mut lowerer =
            Lowerer::with_config(fixture.context.commands(), fixture.input.lexer_config())
                .with_dialect(Some(fixture.input.unit_profile()))
                .with_resolved_analysis_input(fixture.input.clone());
        lowerer.set_source_analysis_options(fixture.entry.options());
        lowerer.source_bindings = Some(fixture.bindings.clone().into());
        lowerer.structured_source_parent = Some(Box::new(parent));
        let script = lowerer.lower_original_body(fixture.body.clone(), Some(&fixture.namespace));
        assert_eq!(
            script.namespace_context.as_deref(),
            Some(&fixture.namespace)
        );
        assert_eq!(script.statements.len(), 1);
        let original = script
            .retained_source_tokens_for_statement(&script.statements[0])
            .unwrap();
        assert_eq!(original.argv_texts, ["set", "result", "café"]);
        let binding = original.source_binding.as_ref().unwrap();
        assert_eq!(
            binding.original_lexer_config_for_tokens(original),
            Some(fixture.input.lexer_config())
        );
        assert_eq!(
            binding.invocation_site().unwrap().source,
            fixture.body.origin
        );
        assert!(binding.lookup_state.is_none());
        assert!(binding.proved_execution_target().is_none());
        assert!(
            binding
                .original_invocation_metadata_for_input(
                    original,
                    &fixture.input,
                    fixture.context.commands()
                )
                .is_none()
        );
        assert!(lowerer.original_body_word_source.is_none());
    }
}

#[test]
fn original_derived_body_geometry_refuses_changed_parent_vector_span_and_input() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let (fixture, parent) = folded_fixture("tcl8.4");
    let config = fixture.input.lexer_config();
    let mut changed_text = parent.clone();
    changed_text.argv_texts[2].push('!');
    let mut changed_word = parent.clone();
    let crate::ir::WordExpr::BracedLiteral { text, .. } = &mut changed_word.word_exprs[2] else {
        panic!("original whole body word")
    };
    text.push('!');
    changed_word.argv_texts[2].push('!');
    let mut changed_span = parent.clone();
    crate::lattice_rebase::rebase_command_tokens(&mut changed_span, 1);
    let mut missing = parent.clone();
    missing.source_binding = None;
    let mut arguments_only = parent.clone();
    arguments_only.synthetic = Some(crate::ir::SyntheticMarker::EvaluatedArguments);
    for changed in [
        changed_text,
        changed_word,
        changed_span,
        missing,
        arguments_only,
    ] {
        assert!(
            fixture
                .bindings
                .original_body_written_word_owner_at_parent(
                    &fixture.body,
                    config,
                    fixture.context.commands(),
                    &fixture.input,
                    &changed
                )
                .is_none()
        );
    }
    let changed_config = tcl_lexer::LexerConfig {
        strict_quoting: !config.strict_quoting,
        ..config
    };
    let changed_input = ResolvedAnalysisInput::new(
        fixture.input.analyser_profile(),
        fixture.input.unit_profile(),
        fixture.input.context_registry(),
        changed_config,
    );
    assert!(
        fixture
            .bindings
            .original_body_written_word_owner_at_parent(
                &fixture.body,
                changed_config,
                fixture.context.commands(),
                &changed_input,
                &parent
            )
            .is_none()
    );
    let foreign_context = tcl_registry::model::ingress::resolve_known_environment("tcl8.5")
        .unwrap()
        .default_context_registry();
    let foreign_input = ResolvedAnalysisInput::new(
        fixture.input.analyser_profile(),
        fixture.input.unit_profile(),
        foreign_context.clone(),
        config,
    );
    assert!(
        fixture
            .bindings
            .original_body_written_word_owner_at_parent(
                &fixture.body,
                config,
                fixture.context.commands(),
                &foreign_input,
                &parent
            )
            .is_none()
    );
    assert!(
        fixture
            .bindings
            .original_body_written_word_owner_at_parent(
                &fixture.body,
                config,
                foreign_context.commands(),
                &fixture.input,
                &parent
            )
            .is_none()
    );
}

#[test]
fn original_derived_body_geometry_refuses_counterfeit_source_ordinal_and_parent_site() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let (fixture, parent) = folded_fixture("tcl8.4");
    let site = parent
        .source_binding
        .as_ref()
        .unwrap()
        .invocation_site()
        .unwrap()
        .clone();
    let mut changed_text = fixture.body.clone();
    changed_text.text = tcl_lexer::SourceImage::native(b"set result OTHER".as_slice());
    let wrong_ordinal =
        ExecutedScriptSource::materialised(site.clone(), vec![2], fixture.body.try_text().unwrap());
    let shifted_parent = crate::command_binding::CommandAllocationSite {
        offset: site.offset + 1,
        ..site.clone()
    };
    let counterfeit_span = ExecutedScriptSource::materialised(
        shifted_parent,
        vec![1],
        fixture.body.try_text().unwrap(),
    );
    let foreign_parent = crate::command_binding::CommandAllocationSite {
        source: Arc::new(crate::command_binding::SourceOriginId::authored(
            &Arc::from("if {$condition} {set result OTHER}"),
        )),
        offset: site.offset,
    };
    let foreign_source = ExecutedScriptSource::materialised(
        foreign_parent,
        vec![1],
        fixture.body.try_text().unwrap(),
    );
    for body in [
        changed_text,
        wrong_ordinal,
        counterfeit_span,
        foreign_source,
    ] {
        assert!(
            fixture
                .bindings
                .original_body_written_word_owner_at_parent(
                    &body,
                    fixture.input.lexer_config(),
                    fixture.context.commands(),
                    &fixture.input,
                    &parent
                )
                .is_none()
        );
    }
}
