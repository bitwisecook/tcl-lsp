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

#[test]
fn original_logical_root_and_nested_dispatch_keep_conditional_targets_and_original_owners() {
    // naming.source.logical-original-operation-transfer
    // docs/design/analysis/name-resolution-proofs/logical-original-operation-transfer.md
    let input = logical_input();
    let context = input.context_registry();
    for (source, selected, captured) in [
        ("package require Tcl", "package require Tcl", false),
        (
            "interp alias {} ensure {} package require; ensure Tcl",
            "ensure Tcl",
            true,
        ),
        (
            "proc subject {} {package require Tcl}",
            "package require Tcl",
            false,
        ),
        (
            "interp alias {} ensure {} package require; proc subject {} {ensure Tcl}",
            "ensure Tcl",
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
        let binding = bindings.invocation_at_source("caller label", offset);
        let target = binding
            .proved_target()
            .expect("exact original source target");
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
        assert_eq!(binding.logical_source_name_advice_input(), Some(&input));
        assert_eq!(
            binding.runtime_reachability(),
            SourceRuntimeReachability::Conditional
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
    // naming.source.logical-original-operation-transfer
    // docs/design/analysis/name-resolution-proofs/logical-original-operation-transfer.md
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
    // naming.source.logical-original-operation-transfer
    // docs/design/analysis/name-resolution-proofs/logical-original-operation-transfer.md
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
