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

//! Materialized footprints and installer projections: aliases, availability and stale owners.

use super::*;

#[test]
fn original_installer_module_projection_keeps_snapshot_and_future_scope_refusal() {
    // naming.diagnostic.original-materialized-write-footprint
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-materialized-write-footprint.md
    // Software source snapshot/lookup controls, not callback or frame entry.
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    let unit = unit("eval set ::early VALUE; proc set args {}", &context);
    let module = &unit.ir_module;
    let script = &module.top_level;
    let tokens = script
        .retained_source_tokens_for_statement(&script.statements[0])
        .unwrap();
    let binding = tokens.source_binding.as_ref().unwrap();
    assert_eq!(
        binding
            .original_invocation_metadata_for_module(tokens, module, context.commands())
            .unwrap()
            .source_analysis_input(),
        module.source_metadata_input.as_ref(),
    );
    let footprint = binding
        .original_materialized_footprint_for_module(tokens, module, context.commands())
        .unwrap();
    assert!(std::ptr::eq(
        footprint.bindings(),
        &raw const binding.lookup_state.as_ref().unwrap().state
    ));
    assert_eq!(
        footprint.metadata_context().source_analysis_input(),
        module.source_metadata_input.as_ref()
    );
    assert_eq!(footprint.lexer_config(), module.lexer_config.nested());
    assert!(
        footprint
            .script_writes("set ::observed VALUE")
            .names
            .iter()
            .any(|name| name == "::observed")
    );
    assert!(matches!(
        footprint.deferred_namespace(Some(tcl_registry::ScriptLookupScope::GlobalFrame)),
        Some(crate::ir::ExecutionNamespace::SourceContext(_)),
    ));
    for scope in [
        None,
        Some(tcl_registry::ScriptLookupScope::InvokingFrame),
        Some(tcl_registry::ScriptLookupScope::TriggerFrame),
    ] {
        assert!(footprint.deferred_namespace(scope).is_none());
    }
    let mut refused = module.clone();
    refused.source_metadata_input = None;
    assert!(
        binding
            .original_materialized_footprint_for_module(tokens, &refused, context.commands())
            .is_none()
    );
    refused = module.clone();
    refused.lexer_config.expand_syntax = !refused.lexer_config.expand_syntax;
    assert!(
        binding
            .original_materialized_footprint_for_module(tokens, &refused, context.commands())
            .is_none()
    );
    refused = module.clone();
    refused.source = tcl_lexer::SourceImage::document("eval set ::different VALUE");
    assert!(
        binding
            .original_materialized_footprint_for_module(tokens, &refused, context.commands())
            .is_none()
    );
    let original = module.source_metadata_input.as_ref().unwrap();
    refused = module.clone();
    refused.source_metadata_input = Some(ResolvedAnalysisInput::new(
        original.analyser_profile(),
        original.unit_profile(),
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry(),
        original.lexer_config(),
    ));
    assert!(
        binding
            .original_materialized_footprint_for_module(tokens, &refused, context.commands())
            .is_none()
    );
}

#[test]
fn original_installer_module_projection_withdraws_changed_entry_and_frame_owner() {
    // naming.diagnostic.original-materialized-write-footprint
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-materialized-write-footprint.md
    // Full source-owner reuse controls, independently of physical frame entry.
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    let unit = unit("eval set ::original VALUE", &context);
    let module = &unit.ir_module;
    let owner = module.retained_source_bindings.as_ref().unwrap();
    assert!(owner.matches_module(module, context.commands()));
    let script = &module.top_level;
    let tokens = script
        .retained_source_tokens_for_statement(&script.statements[0])
        .unwrap();
    let binding = tokens.source_binding.as_ref().unwrap();
    assert!(
        binding
            .original_materialized_footprint_for_module(tokens, module, context.commands())
            .is_some()
    );
    for change in 0..7 {
        let mut changed = module.clone();
        match change {
            0 => changed.retained_source_bindings = None,
            1 => changed.source_entry.unknown_entry = !changed.source_entry.unknown_entry,
            2 => {
                changed.source_entry.compilation_scope =
                    tcl_runtime_api::SourceCompilationScope::EnteredSource;
            }
            3 => changed.top_level_namespace = "::other".into(),
            4 => {
                changed.native_namespace =
                    Some(tcl_core_types::ByteNamespacePath::from_segments(["other"]));
            }
            5 => changed.top_level_kind = crate::ir::TopLevelKind::ProcedureBody,
            6 => changed.source_entry.incoming_formals.push("new".into()),
            _ => unreachable!(),
        }
        assert!(InvocationMetadataContext::for_module(context.commands(), &changed).is_some());
        assert!(
            binding
                .original_invocation_metadata_for_module(tokens, &changed, context.commands())
                .is_none()
        );
        assert!(
            binding
                .original_materialized_footprint_for_module(tokens, &changed, context.commands())
                .is_none(),
            "change {change} retained stale source interpretation"
        );
    }
}

#[test]
fn original_installer_module_projection_accepts_only_the_retained_nested_input() {
    // naming.diagnostic.original-materialized-write-footprint
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-materialized-write-footprint.md
    // Independently produced source points retain their explicit nested policy.
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    let source = "eval set ::nested VALUE";
    let root = unit(source, &context);
    let original = root.ir_module.source_metadata_input.as_ref().unwrap();
    let input = original.for_nested_source();
    let nested = CompilationUnit::build_with_analysis_input(
        source,
        UnitBuildOptions {
            registry: context.commands(),
            defer_top_level: false,
            config: input.lexer_config(),
            dialect: Some(input.unit_profile()),
            external_call_sites: None,
            declared_commands: None,
        },
        None,
        &input,
    );
    let script = &nested.ir_module.top_level;
    let tokens = script
        .retained_source_tokens_for_statement(&script.statements[0])
        .unwrap();
    let binding = tokens.source_binding.as_ref().unwrap();
    assert_eq!(
        binding
            .original_invocation_metadata_for_module(tokens, &root.ir_module, context.commands())
            .unwrap()
            .source_analysis_input(),
        Some(&input),
    );
    let footprint = binding
        .original_materialized_footprint_for_module(tokens, &root.ir_module, context.commands())
        .unwrap();
    assert_eq!(
        footprint.metadata_context().source_analysis_input(),
        Some(&input)
    );
    assert!(
        footprint
            .script_writes("set ::observed VALUE")
            .names
            .iter()
            .any(|name| name == "::observed")
    );
    let mut changed = root.ir_module.clone();
    let mut config = original.lexer_config();
    config.expand_syntax = !config.expand_syntax;
    changed.lexer_config = config;
    changed.source_metadata_input = Some(ResolvedAnalysisInput::new(
        original.analyser_profile(),
        original.unit_profile(),
        Arc::clone(&context),
        config,
    ));
    assert!(InvocationMetadataContext::for_module(context.commands(), &changed).is_some());
    assert!(
        binding
            .original_materialized_footprint_for_module(tokens, &changed, context.commands())
            .is_none()
    );
}

#[test]
fn original_materialized_footprint_keeps_alias_targets_and_literal_roots() {
    // naming.diagnostic.original-materialized-write-footprint
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-materialized-write-footprint.md
    // Source/API possible names only; no process result, child argv or store.
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    for (source, text, expected) in [
        ("eval set {} VALUE", "set {} VALUE", Some("")),
        ("eval set {(key)} VALUE", "set {(key)} VALUE", Some("")),
        (
            "interp alias {} put {} set {}; eval put VALUE",
            "put VALUE",
            Some(""),
        ),
        ("proc set args {}; eval set {} VALUE", "set {} VALUE", None),
        (
            "eval set {café(open} VALUE",
            "set {café(open} VALUE",
            Some("café(open"),
        ),
        (
            "interp alias {} put {} set {$literal}; eval put VALUE",
            "put VALUE",
            Some("$literal"),
        ),
        (
            "rename set moved; interp alias {} put {} moved café; eval put VALUE",
            "put VALUE",
            Some("café"),
        ),
        (
            "proc set args {}; eval set hidden VALUE",
            "set hidden VALUE",
            None,
        ),
        (
            "interp alias {} put {} set fixed; rename set moved; eval put VALUE",
            "put VALUE",
            None,
        ),
    ] {
        let unit = unit(source, &context);
        let script = &unit.ir_module.top_level;
        let tokens = script
            .retained_source_tokens_for_statement(script.statements.last().unwrap())
            .unwrap();
        let binding = tokens.source_binding.as_ref().unwrap();
        let metadata = InvocationMetadataContext::for_module(context.commands(), &unit.ir_module);
        let footprint = binding
            .original_materialized_footprint(tokens, source, context.commands(), metadata)
            .expect("original closed installer horizon");
        let writes = footprint.script_writes(text);
        if let Some(expected) = expected {
            assert!(
                writes.names.iter().any(|name| name == expected),
                "{source}: {:?}",
                writes.names
            );
        } else {
            assert!(
                writes.names.is_empty(),
                "known replacement: {:?}",
                writes.names
            );
        }
    }
}

#[test]
fn original_materialized_footprint_keeps_empty_reads_and_unset_targets() {
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    for text in [
        "set {} VALUE; set {}; unset {}",
        "set {(key)} VALUE; set {(key)}; unset {(key)}",
    ] {
        let source = format!("eval {{{text}}}");
        let unit = unit(&source, &context);
        let script = &unit.ir_module.top_level;
        let tokens = script
            .retained_source_tokens_for_statement(script.statements.last().unwrap())
            .unwrap();
        let binding = tokens.source_binding.as_ref().unwrap();
        let metadata = InvocationMetadataContext::for_module(context.commands(), &unit.ir_module);
        let footprint = binding
            .original_materialized_footprint(tokens, &source, context.commands(), metadata)
            .expect("original materialized source owner");
        let effects = footprint.script_writes(text);
        assert!(
            effects.names.iter().any(String::is_empty),
            "{text}: {effects:?}"
        );
        assert!(
            effects.read_names.iter().any(String::is_empty),
            "{text}: {effects:?}"
        );
    }
}

#[test]
fn original_materialized_footprint_uses_actual_availability_on_the_same_store() {
    // naming.diagnostic.original-materialized-write-footprint
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-materialized-write-footprint.md
    // The command generation is identical; only actual availability changes.
    let mut registry = tcl_registry::CommandRegistry::build_default();
    let setter = registry.get("set").unwrap().clone();
    registry.insert(tcl_registry::CommandSpec {
        name: "metadata_write",
        surface: registry.get("dict").unwrap().surface,
        ..setter
    });
    let current = Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl9.0")
            .with_command_store(registry.snapshot().shared_registry()),
    );
    let older = Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(registry.snapshot().shared_registry()),
    );
    let source = "eval metadata_write output VALUE";
    for (context, expected) in [(&current, true), (&older, false)] {
        let unit = unit(source, context);
        let script = &unit.ir_module.top_level;
        let tokens = script
            .retained_source_tokens_for_statement(script.statements.last().unwrap())
            .unwrap();
        let binding = tokens.source_binding.as_ref().unwrap();
        let metadata = InvocationMetadataContext::for_module(&registry, &unit.ir_module);
        let footprint = binding
            .original_materialized_footprint(tokens, source, &registry, metadata)
            .unwrap();
        let writes = footprint.script_writes("metadata_write output VALUE");
        assert_eq!(
            writes.names.iter().any(|name| name == "output"),
            expected,
            "{expected}: {writes:?}"
        );
        if !expected {
            assert!(
                writes.opaque,
                "unavailable source target keeps its residual"
            );
        }
    }
}

#[test]
fn materialized_bodies_keep_the_original_entry_availability_phase() {
    // naming.diagnostic.original-materialized-write-footprint
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-materialized-write-footprint.md
    // Exact source/API phase selection only. The explicit entry has no
    // native interpreter, body-entry, worker or successful store receipt.
    use tcl_dialect::model::InvocationRealm;
    let context =
        tcl_registry::model::ingress::resolve_environment("f5-irules").default_context_registry();
    let registry = context.commands();
    let profile = tcl_dialect::DialectProfile::find("f5-irules").unwrap();
    let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
    let input = ResolvedAnalysisInput::new(profile, profile, Arc::clone(&context), config);
    assert!(context.context().resolve_spec(registry, "time").is_none());
    assert!(
        context
            .context()
            .resolve_spec_in_realm(registry, "time", InvocationRealm::InterpreterRuntime,)
            .is_some()
    );
    let source = "eval time {set output VALUE}";
    for (realm, expected) in [
        (InvocationRealm::RuleLoader, false),
        (InvocationRealm::InterpreterRuntime, true),
    ] {
        let entry = super::super::super::SourceAnalysisEntry {
            invocation_realm: realm,
            invocation_dialect: Some(tcl_native_model::InvocationDialect::of_profile(profile)),
            ..Default::default()
        };
        let unit = CompilationUnit::build_with_analysis_input(
            source,
            UnitBuildOptions {
                registry,
                defer_top_level: false,
                config,
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            Some(&entry),
            &input,
        );
        assert!(entry.native_entry.is_none());
        let script = &unit.ir_module.top_level;
        let tokens = script
            .retained_source_tokens_for_statement(script.statements.last().unwrap())
            .unwrap();
        let binding = tokens.source_binding.as_ref().unwrap();
        assert_eq!(binding.invocation_realm(), Some(realm));
        let footprint = binding
            .original_materialized_footprint(
                tokens,
                source,
                registry,
                InvocationMetadataContext::for_module(registry, &unit.ir_module),
            )
            .unwrap();
        let writes = footprint.script_writes("time {set output VALUE}");
        assert_eq!(
            writes.names.iter().any(|name| name == "output"),
            expected,
            "{realm:?}: {writes:?}"
        );
        if !expected {
            assert!(writes.opaque, "loader refusal keeps the residual");
        }
    }
}

#[test]
fn original_materialized_footprint_refuses_missing_foreign_and_stale_owners() {
    // naming.diagnostic.original-materialized-write-footprint
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-materialized-write-footprint.md
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    let source = "eval set x VALUE";
    let unit = unit(source, &context);
    let script = &unit.ir_module.top_level;
    let tokens = script
        .retained_source_tokens_for_statement(script.statements.last().unwrap())
        .unwrap();
    let binding = tokens.source_binding.as_ref().unwrap();
    let metadata = InvocationMetadataContext::for_module(context.commands(), &unit.ir_module);
    assert!(
        binding
            .original_materialized_footprint(tokens, source, context.commands(), metadata)
            .is_some()
    );
    assert!(
        binding
            .original_materialized_footprint(tokens, source, context.commands(), None)
            .is_none()
    );
    assert!(
        binding
            .original_materialized_footprint(
                tokens,
                "eval set x OTHER",
                context.commands(),
                metadata
            )
            .is_none()
    );
    let input = unit.ir_module.source_metadata_input.as_ref().unwrap();
    let foreign = ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry(),
        input.lexer_config(),
    );
    assert!(
        binding
            .original_materialized_footprint(
                tokens,
                source,
                context.commands(),
                InvocationMetadataContext::for_analysis_input(context.commands(), &foreign)
            )
            .is_none()
    );
    let stale = ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        Arc::clone(&context),
        tcl_lexer::LexerConfig {
            expand_syntax: tcl_lexer::LexerConfig::for_dialect("tcl8.4").expand_syntax,
            ..input.lexer_config()
        },
    );
    assert!(
        binding
            .original_materialized_footprint(
                tokens,
                source,
                context.commands(),
                InvocationMetadataContext::for_analysis_input(context.commands(), &stale)
            )
            .is_none()
    );
}
