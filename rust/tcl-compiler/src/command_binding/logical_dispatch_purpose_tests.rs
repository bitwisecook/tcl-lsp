// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;
use crate::analyser::ResolvedAnalysisInput;

fn logical_input() -> ResolvedAnalysisInput {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    ResolvedAnalysisInput::new(
        profile,
        profile,
        tcl_registry::model::ingress::resolve_known_environment("tcl8.6")
            .expect("known fixture provider")
            .default_context_registry(),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    )
}

fn logical_original_statement(
    input: &ResolvedAnalysisInput,
    source: &str,
    offset: u32,
) -> crate::ir::CommandTokens {
    let mut lowerer = crate::lowering::Lowerer::with_config(
        input.borrowed_context_registry().commands(),
        input.lexer_config(),
    )
    .with_resolved_analysis_input(input.clone());
    lowerer.lower(source);
    let module = lowerer.finish_module(source);
    assert_eq!(module.source_metadata_input.as_ref(), Some(input));
    assert_eq!(module.lexer_config, input.lexer_config());
    let mut found = Vec::new();
    let mut collect = |statement: &crate::ir::Statement| {
        if statement.span().start() == offset {
            found.push(
                statement
                    .tokens()
                    .expect("original statement vector")
                    .clone(),
            );
        }
    };
    crate::ir::for_each_statement(&module.top_level, &mut collect);
    for procedure in module.procedures.values() {
        crate::ir::for_each_statement(&procedure.body, &mut collect);
    }
    assert_eq!(found.len(), 1);
    found.pop().unwrap()
}

fn assert_deferred_logical_bindings_have_no_execution(
    original: &crate::command_binding::SourceInvocationBinding,
    binding: &crate::command_binding::SourceInvocationBinding,
) {
    assert!(original.lookup_state.is_none());
    assert_ne!(
        original.runtime_reachability(),
        SourceRuntimeReachability::Conditional
    );
    assert_ne!(
        original.runtime_reachability(),
        SourceRuntimeReachability::Reached
    );
    assert!(original.proved_target().is_none());
    assert!(original.unknown);
    assert!(original.targets.is_empty());
    assert!(binding.proved_target().is_none());
    assert!(binding.unknown);
    assert!(binding.targets.is_empty());
    assert!(binding.lookup_state.is_none());
    assert!(binding.logical_source_name_advice_input().is_none());
    assert_ne!(
        binding.runtime_reachability(),
        SourceRuntimeReachability::Conditional
    );
    assert_ne!(
        binding.runtime_reachability(),
        SourceRuntimeReachability::Reached
    );
}

#[test]
fn original_logical_root_and_nested_dispatch_keep_conditional_targets_and_original_owners() {

    let input = logical_input();
    let context = input.context_registry();
    for (source, selected, captured, nested) in [
        ("package require Tcl", "package require Tcl", false, false),
        (
            "interp alias {} ensure {} package require; ensure Tcl",
            "ensure Tcl",
            true,
            false,
        ),
        (
            "proc subject {} {package require Tcl}",
            "package require Tcl",
            false,
            true,
        ),
        (
            "interp alias {} ensure {} package require; proc subject {} {ensure Tcl}",
            "ensure Tcl",
            true,
            true,
        ),
    ] {
        let bindings = SourceCommandBindings::analyse_with_options(
            source,
            input.lexer_config(),
            context.commands(),
            SourceAnalysisOptions::for_logical_source(&input).unwrap(),
        );
        let offset = u32::try_from(source.rfind(selected).unwrap()).unwrap();
        let original = bindings.invocation_at_source("caller label", offset);
        // A declaration-only body uses the actual Lowerer's stamped source
        // vector. Its conditional source target does not denote dispatch.
        let tokens = nested.then(|| logical_original_statement(&input, source, offset));
        let binding = tokens.as_ref().map_or(&original, |tokens| {
            tokens
                .source_binding
                .as_ref()
                .expect("original declaration binding")
        });
        let target = if let Some(tokens) = &tokens {
            let targets = binding
                .original_logical_declaration_call_targets(tokens, &input, context.commands())
                .expect("exact original declaration source target");
            let [target] = targets.as_slice() else {
                panic!("one original source target")
            };
            assert_deferred_logical_bindings_have_no_execution(&original, binding);
            let metadata = binding
                .original_invocation_metadata_for_input(tokens, &input, context.commands())
                .expect("complete original Logical source owner");
            let owner = metadata
                .source_analysis_input()
                .expect("retained source input");
            assert!(owner == &input || owner == &input.for_nested_source());
            target.clone()
        } else {
            assert!(original.lookup_state.is_some());
            assert_eq!(binding.logical_source_name_advice_input(), Some(&input));
            assert_eq!(
                binding.runtime_reachability(),
                SourceRuntimeReachability::Conditional
            );
            binding
                .proved_target()
                .expect("exact original source target")
                .clone()
        };
        assert!(target.registry_backed, "{source}");
        assert_eq!(
            target.prepended,
            if captured {
                vec![crate::registry_invocation::EffectiveInvocationWord::Literal("require".into())]
            } else {
                Vec::new()
            },
            "{source}"
        );
        assert!(binding.execution_is_unknown());
        assert!(binding.proved_execution_target().is_none());
        assert!(binding.proved_handler_target().is_none());
        assert!(binding.native_handler_envelope.is_none());
        assert!(binding.native_compiler_admission.is_none());
        assert!(binding.variable_context.execution_name_policy.is_none());
        assert!(binding.original_recorded_head_name_input().is_none());
        let site = binding.invocation_site().expect("original point");
        assert_eq!(site.offset, offset);
        assert_eq!(
            site.source.source_image(),
            &tcl_lexer::SourceImage::document(source)
        );
    }
}

#[test]
fn original_logical_dispatch_condition_requires_exact_retained_input_and_availability() {

    let input = logical_input();
    let context = input.context_registry();
    let source = "package require Tcl";
    let bindings = SourceCommandBindings::analyse_with_options(
        source,
        input.lexer_config(),
        context.commands(),
        SourceAnalysisOptions::for_logical_source(&input).unwrap(),
    );
    assert_eq!(
        bindings.invocation_at_source("", 0).runtime_reachability(),
        SourceRuntimeReachability::Conditional
    );
    let mut config = input.lexer_config();
    config.strict_quoting = !config.strict_quoting;
    let changed = ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        Arc::clone(&context),
        config,
    );
    let foreign = ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        tcl_registry::model::ingress::resolve_known_environment("tcl9.1")
            .expect("known foreign provider")
            .default_context_registry(),
        input.lexer_config(),
    );
    for (label, owner) in [
        (
            "missing",
            crate::registry_invocation::OwnedInvocationMetadataContext::Unavailable,
        ),
        (
            "standalone",
            crate::registry_invocation::OwnedInvocationMetadataContext::Standalone,
        ),
        (
            "changed config",
            crate::registry_invocation::OwnedInvocationMetadataContext::for_source_input(Some(
                &changed,
            )),
        ),
        (
            "foreign availability",
            crate::registry_invocation::OwnedInvocationMetadataContext::for_source_input(Some(
                &foreign,
            )),
        ),
    ] {
        let mut withdrawn = bindings.clone();
        let indices = withdrawn.dispatch_points.get(&0).unwrap().clone();
        for index in indices {
            let point = &mut withdrawn.points[index];
            Arc::make_mut(&mut point.state.baseline).metadata_context = owner.clone();
            point.lookup_snapshot = std::sync::OnceLock::new();
        }
        let binding = withdrawn.invocation_at_source("", 0);
        assert!(binding.unknown, "{label}");
        assert!(binding.targets.is_empty(), "{label}");
        assert_ne!(
            binding.runtime_reachability(),
            SourceRuntimeReachability::Conditional,
            "{label}"
        );
        assert!(binding.proved_handler_target().is_none(), "{label}");
        assert!(binding.proved_execution_target().is_none(), "{label}");
    }
    let unknown = SourceCommandBindings::analyse_with_options(
        source,
        input.lexer_config(),
        context.commands(),
        SourceAnalysisOptions {
            unknown_entry: true,
            ..Default::default()
        },
    )
    .invocation_at_source("", 0);
    assert!(unknown.logical_source_name_advice_input().is_none());
    assert_ne!(
        unknown.runtime_reachability(),
        SourceRuntimeReachability::Conditional
    );
    assert!(unknown.proved_handler_target().is_none());
}

#[test]
fn original_native_and_explicit_standalone_dispatch_keep_their_reached_purpose() {

    // Live VM capture checks the software owner; no external provider row is inferred.
    let environment = tcl_registry::model::ingress::resolve_known_environment("tcl8.6")
        .expect("known Native fixture provider");
    let profile = environment.analyser_profile();
    let context = environment.default_context_registry();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let (owner, entry) = crate::environment_ingress::captured_native_entry_with_owner(profile);
    let dialect = tcl_registry::InvocationDialect::of_point(entry.execution_point.unwrap());
    let source = "list VALUE";
    let native = SourceCommandBindings::analyse_with_options(
        source,
        config,
        context.commands(),
        SourceAnalysisOptions {
            native_entry: Some(&entry),
            invocation_dialect: Some(dialect),
            native_compilation: crate::environment_ingress::authoring_native_compilation(),
            ..Default::default()
        },
    )
    .invocation_at_source("", 0);
    assert_eq!(
        native.runtime_reachability(),
        SourceRuntimeReachability::Reached
    );
    assert!(native.logical_source_name_advice_input().is_none());
    assert!(native.original_recorded_head_name_input().is_some());
    assert!(native.proved_execution_target().is_some());
    assert!(native.proved_handler_target().is_some());
    let standalone = SourceCommandBindings::analyse(source, config, context.commands())
        .invocation_at_source("", 0);
    assert_eq!(
        standalone.runtime_reachability(),
        SourceRuntimeReachability::Reached
    );
    assert!(standalone.logical_source_name_advice_input().is_none());
    assert!(standalone.proved_execution_target().is_some());
    drop(owner);
}
