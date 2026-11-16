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

use crate::analyser::ResolvedAnalysisInput;
use crate::compilation_unit::{CompilationUnit, UnitBuildOptions};
use crate::registry_invocation::InvocationMetadataContext;
use std::sync::Arc;

fn unit(source: &str, context: &Arc<tcl_registry::model::ContextRegistry>) -> CompilationUnit {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
    let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
    let input = ResolvedAnalysisInput::new(profile, profile, Arc::clone(context), config);
    CompilationUnit::build_with_analysis_input(
        source,
        UnitBuildOptions {
            registry: context.commands(),
            defer_top_level: false,
            config,
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        None,
        &input,
    )
}

#[test]
fn original_module_metadata_preserves_explicit_standalone_without_supplied_fallback() {
    use crate::registry_invocation::OwnedInvocationMetadataContext;
    let profile = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
    // A profiled default build is a fresh-interpreter document (user
    // decision 2026-10-19T14:20Z); explicit Standalone authoring is the
    // dialect-blind build, still profiled through its Registry.
    let fresh = CompilationUnit::build_for_profile("set value ORIGINAL", &registry, false, profile);
    assert!(fresh.ir_module.source_metadata_input.is_some());
    let standalone = CompilationUnit::build_for_with_config(
        "set value ORIGINAL",
        &registry,
        false,
        tcl_lexer::LexerConfig::from_grammar(profile.grammar),
    );
    let module = &standalone.ir_module;
    assert!(module.source_metadata_input.is_none());
    assert!(module.source_entry.metadata_context.is_standalone());
    assert!(module.source_entry.native_entry.is_none());
    let script = &module.top_level;
    let tokens = script
        .retained_source_tokens_for_statement(&script.statements[0])
        .unwrap();
    let binding = tokens.source_binding.as_ref().unwrap();
    let metadata = binding
        .original_invocation_metadata_for_module(tokens, module, &registry)
        .expect("actual retained standalone Module and original point");
    assert!(metadata.is_standalone());
    assert!(metadata.source_analysis_input().is_none());
    assert!(
        crate::registry_invocation::admitted_native_compiler_invocation(&registry, None, tokens)
            .is_none()
    );
    for change in 0..4 {
        let mut changed = module.clone();
        match change {
            0 => changed.source_entry.unknown_entry = true,
            1 => {
                changed.source_entry.metadata_context = OwnedInvocationMetadataContext::Unavailable;
            }
            2 => changed.lexer_config.strict_quoting = !changed.lexer_config.strict_quoting,
            3 => changed.source = tcl_lexer::SourceImage::document("set value CHANGED"),
            _ => unreachable!(),
        }
        assert!(
            binding
                .original_invocation_metadata_for_module(tokens, &changed, &registry)
                .is_none()
        );
    }
    let foreign_profile =
        tcl_registry::model::ingress::resolve_environment("tcl9.1").analyser_profile();
    let foreign =
        tcl_registry::CommandRegistry::build_default().project_for_profile(foreign_profile);
    assert_ne!(
        registry.snapshot().semantic_key(),
        foreign.snapshot().semantic_key()
    );
    assert!(
        binding
            .original_invocation_metadata_for_module(tokens, module, &foreign)
            .is_none()
    );
    let mut changed_tokens = tokens.clone();
    changed_tokens.argv_texts[1] = "changed".into();
    assert!(
        binding
            .original_invocation_metadata_for_module(&changed_tokens, module, &registry)
            .is_none()
    );

    let context =
        tcl_registry::model::ingress::resolve_environment("tcl").default_context_registry();
    let supplied = unit("set value ORIGINAL", &context);
    let script = &supplied.ir_module.top_level;
    let tokens = script
        .retained_source_tokens_for_statement(&script.statements[0])
        .unwrap();
    let binding = tokens.source_binding.as_ref().unwrap();
    let mut missing = supplied.ir_module.clone();
    missing.source_metadata_input = None;
    missing.source_entry.metadata_context = OwnedInvocationMetadataContext::Standalone;
    assert!(
        binding
            .original_invocation_metadata_for_module(tokens, &missing, context.commands())
            .is_none()
    );
}

#[test]
fn original_tagged_point_selection_keeps_supplied_refusal_distinct_from_standalone() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    use crate::registry_invocation::OwnedInvocationMetadataContext;
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    let unit = unit("eval set ::original VALUE", &context);
    let script = &unit.ir_module.top_level;
    let tokens = script
        .retained_source_tokens_for_statement(&script.statements[0])
        .unwrap();
    let binding = tokens.source_binding.as_ref().unwrap();
    let input = unit.ir_module.source_metadata_input.as_ref().unwrap();
    let owner = OwnedInvocationMetadataContext::for_source_input(Some(input));
    let metadata = binding
        .original_invocation_metadata_for_owner(tokens, &owner, context.commands())
        .unwrap()
        .unwrap();
    assert_eq!(metadata.source_analysis_input(), Some(input));
    assert!(
        binding
            .original_invocation_metadata_for_owner(
                tokens,
                &OwnedInvocationMetadataContext::Standalone,
                context.commands()
            )
            .is_err()
    );
    let older = Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(Arc::clone(context.commands())),
    );
    let foreign =
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
    for changed_context in [older, foreign] {
        let changed = ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            changed_context,
            input.lexer_config(),
        );
        assert!(
            binding
                .original_invocation_metadata_for_owner(
                    tokens,
                    &OwnedInvocationMetadataContext::for_source_input(Some(&changed)),
                    context.commands()
                )
                .is_err()
        );
    }
    let mut config = input.lexer_config();
    config.strict_quoting = !config.strict_quoting;
    let changed = ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        Arc::clone(&context),
        config,
    );
    for owner in [
        OwnedInvocationMetadataContext::Unavailable,
        OwnedInvocationMetadataContext::Supplied(Arc::clone(&context)),
        OwnedInvocationMetadataContext::for_source_input(Some(&changed)),
    ] {
        assert!(
            binding
                .original_invocation_metadata_for_owner(tokens, &owner, context.commands())
                .is_err()
        );
    }
    let mut changed = tokens.clone();
    changed.word_exprs.pop();
    assert!(
        binding
            .original_invocation_metadata_for_owner(&changed, &owner, context.commands())
            .is_err()
    );
    let registry = tcl_registry::CommandRegistry::build_default();
    let standalone = CompilationUnit::build_for("missing_original_command VALUE", &registry, false);
    let script = &standalone.ir_module.top_level;
    let tokens = script
        .retained_source_tokens_for_statement(&script.statements[0])
        .unwrap();
    let binding = tokens.source_binding.as_ref().unwrap();
    assert!(matches!(
        binding.original_invocation_metadata_for_owner(
            tokens,
            &OwnedInvocationMetadataContext::Standalone,
            &registry
        ),
        Ok(None)
    ));
    assert!(
        binding
            .original_invocation_metadata_for_owner(
                tokens,
                &OwnedInvocationMetadataContext::Unavailable,
                &registry
            )
            .is_err()
    );
}

fn original_metadata_declaration_tokens(
    registry: &tcl_registry::CommandRegistry,
    profile: &'static tcl_dialect::DialectProfile,
    config: tcl_lexer::LexerConfig,
    source: &str,
    offset: u32,
    context: &Arc<tcl_registry::model::ContextRegistry>,
) -> (crate::ir::CommandTokens, ResolvedAnalysisInput) {
    let input = ResolvedAnalysisInput::new(profile, profile, Arc::clone(context), config);
    let options =
        crate::command_binding::SourceAnalysisOptions::for_logical_source(&input).unwrap();
    let inventory = crate::command_binding::SourceCommandBindings::analyse_with_options(
        source, config, registry, options,
    );
    let origin = inventory.root_origin.as_ref().unwrap();
    let entry = inventory.conditional_body_entry_at(origin, offset).unwrap();
    let tokens = inventory
        .declaration_original_tokens_at(&entry, offset)
        .unwrap();
    (tokens, input)
}

fn original_caller_alias_contexts() -> (
    tcl_registry::CommandRegistry,
    Arc<tcl_registry::model::ContextRegistry>,
    Arc<tcl_registry::model::ContextRegistry>,
    tcl_registry::CommandRegistry,
    Arc<tcl_registry::model::ContextRegistry>,
) {
    let mut registry = tcl_registry::CommandRegistry::build_default();
    let link = registry.get("upvar").unwrap().clone();
    registry.insert(tcl_registry::CommandSpec {
        name: "metadata_link",
        surface: registry.get("dict").unwrap().surface,
        ..link
    });
    let store = registry.snapshot().shared_registry();
    let current = Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl9.0")
            .with_command_store(Arc::clone(&store)),
    );
    let older = Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.4").with_command_store(store),
    );
    assert_eq!(
        current.commands().snapshot().semantic_key(),
        older.commands().snapshot().semantic_key()
    );
    let mut foreign_registry = tcl_registry::CommandRegistry::build_default();
    foreign_registry.insert(tcl_registry::CommandSpec {
        name: "foreign_caller_alias_axis",
        ..foreign_registry.get("set").unwrap().clone()
    });
    let foreign = Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl9.0")
            .with_command_store(foreign_registry.snapshot().shared_registry()),
    );
    assert_ne!(
        current.commands().snapshot().semantic_key(),
        foreign.commands().snapshot().semantic_key()
    );
    (registry, current, older, foreign_registry, foreign)
}

fn assert_original_metadata_and_flow_refused(
    binding: &crate::command_binding::SourceInvocationBinding,
    tokens: &crate::ir::CommandTokens,
    registry: &tcl_registry::CommandRegistry,
    advice: &crate::command_binding::OriginalCompilationLookupAdvice,
) {
    assert!(
        binding
            .original_invocation_metadata_at_point(tokens, registry)
            .is_err()
    );
    assert!(
        crate::registry_invocation::declaration_invocation_flow(registry, tokens, advice).is_none()
    );
}

fn assert_older_original_metadata_flow(
    older_tokens: &crate::ir::CommandTokens,
    older_input: &ResolvedAnalysisInput,
    registry: &tcl_registry::CommandRegistry,
) {
    let older_binding = older_tokens.source_binding.as_ref().unwrap();
    assert_eq!(
        older_binding
            .original_invocation_metadata_at_point(older_tokens, registry)
            .unwrap()
            .unwrap()
            .source_analysis_input(),
        Some(older_input)
    );
    assert!(
        older_binding
            .declaration_operand_layout_advice(older_tokens)
            .and_then(|advice| {
                crate::registry_invocation::declaration_invocation_flow(
                    registry,
                    older_tokens,
                    &advice,
                )
            })
            .is_none()
    );
}

fn assert_older_original_flow_absent(
    older_tokens: &crate::ir::CommandTokens,
    registry: &tcl_registry::CommandRegistry,
) {
    assert!(
        older_tokens
            .source_binding
            .as_ref()
            .unwrap()
            .declaration_operand_layout_advice(older_tokens)
            .and_then(
                |advice| crate::registry_invocation::declaration_invocation_flow(
                    registry,
                    older_tokens,
                    &advice,
                )
            )
            .is_none()
    );
}

fn assert_original_caller_alias_geometry(
    selected: &crate::registry_invocation::DeclarationInvocationFlow,
    tokens: &crate::ir::CommandTokens,
    written: usize,
    captured: bool,
) {
    use crate::registry_invocation::InvocationWordOrigin;
    assert_eq!(
        selected.caller_alias_operands,
        vec![tokens.words()[written].clone()]
    );
    assert_eq!(
        selected.effective.origins[2],
        InvocationWordOrigin::Written(written)
    );
    assert_eq!(
        matches!(
            selected.effective.origins[1],
            InvocationWordOrigin::BindingPrefix(_)
        ),
        captured,
    );
}

fn assert_original_metadata_owner_refused(
    binding: &crate::command_binding::SourceInvocationBinding,
    tokens: &crate::ir::CommandTokens,
    owner: &crate::registry_invocation::OwnedInvocationMetadataContext,
    registry: &tcl_registry::CommandRegistry,
) {
    assert!(
        binding
            .original_invocation_metadata_for_owner(tokens, owner, registry)
            .is_err()
    );
}

#[test]
fn original_declaration_flow_keeps_selected_point_availability_and_supplied_refusal() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Conditional setter metadata supplies no successful store or physical frame.
    use crate::registry_invocation::OwnedInvocationMetadataContext;
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
    assert_eq!(
        current.commands().snapshot().semantic_key(),
        older.commands().snapshot().semantic_key()
    );
    let profile = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
    let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
    let source = "proc p {} {metadata_write output VALUE}; p";
    let offset = u32::try_from(source.find("metadata_write").unwrap()).unwrap();
    let original = |context: &Arc<tcl_registry::model::ContextRegistry>| {
        original_metadata_declaration_tokens(&registry, profile, config, source, offset, context)
    };
    let (tokens, input) = original(&current);
    let binding = tokens.source_binding.as_ref().unwrap();
    let metadata = binding
        .original_invocation_metadata_at_point(&tokens, &registry)
        .unwrap()
        .unwrap();
    assert_eq!(metadata.source_analysis_input(), Some(&input));
    let advice = binding.declaration_operand_layout_advice(&tokens).unwrap();
    let selected =
        crate::registry_invocation::declaration_invocation_flow(&registry, &tokens, &advice)
            .unwrap();
    assert_eq!(
        selected.literal_stores,
        vec![("output".to_owned(), "VALUE".to_owned())]
    );
    let (older_tokens, older_input) = original(&older);
    assert_older_original_metadata_flow(&older_tokens, &older_input, &registry);
    let mut missing = tokens.clone();
    let missing_binding = missing.source_binding.as_mut().unwrap();
    let point = Arc::make_mut(missing_binding.lookup_state.as_mut().unwrap());
    Arc::make_mut(&mut point.state.baseline).metadata_context =
        OwnedInvocationMetadataContext::Unavailable;
    assert_original_metadata_and_flow_refused(
        missing.source_binding.as_ref().unwrap(),
        &missing,
        &registry,
        &advice,
    );
    let mut changed_config = config;
    changed_config.strict_quoting = !changed_config.strict_quoting;
    let changed_input =
        ResolvedAnalysisInput::new(profile, profile, Arc::clone(&current), changed_config);
    let mut stale = tokens.clone();
    let point = Arc::make_mut(
        stale
            .source_binding
            .as_mut()
            .unwrap()
            .lookup_state
            .as_mut()
            .unwrap(),
    );
    Arc::make_mut(&mut point.state.baseline).metadata_context =
        OwnedInvocationMetadataContext::for_source_input(Some(&changed_input));
    assert!(
        stale
            .source_binding
            .as_ref()
            .unwrap()
            .original_invocation_metadata_at_point(&stale, &registry)
            .is_err()
    );
    let mut changed = tokens.clone();
    changed.word_exprs.pop();
    assert_original_metadata_and_flow_refused(binding, &changed, &registry, &advice);
    let foreign =
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
    assert!(
        binding
            .original_invocation_metadata_at_point(&tokens, foreign.commands())
            .is_err()
    );
    for owner in [
        OwnedInvocationMetadataContext::Unavailable,
        OwnedInvocationMetadataContext::Supplied(Arc::clone(&current)),
        OwnedInvocationMetadataContext::for_source_input(Some(&older_input)),
    ] {
        assert_original_metadata_owner_refused(binding, &tokens, &owner, &registry);
    }
}

#[test]
fn original_caller_alias_operands_keep_captured_geometry_and_selected_metadata() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Conditional formal mentions only: no reached upvar or caller cell.
    use crate::command_binding::SourceAnalysisOptions;
    use crate::registry_invocation::OwnedInvocationMetadataContext;
    let (registry, current, older, _foreign_registry, foreign) = original_caller_alias_contexts();
    let profile = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
    let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
    for (source, call, written, captured) in [
        (
            "proc p {name} {metadata_link 1 $name local}; p target",
            "metadata_link 1 $name local",
            2,
            false,
        ),
        (
            "interp alias {} link {} metadata_link 1; proc p {name} {link $name local}; p target",
            "link $name local",
            1,
            true,
        ),
    ] {
        let offset = u32::try_from(source.find(call).unwrap()).unwrap();
        let original = |context: &Arc<tcl_registry::model::ContextRegistry>| {
            original_metadata_declaration_tokens(
                &registry, profile, config, source, offset, context,
            )
        };
        let (tokens, input) = original(&current);
        let binding = tokens.source_binding.as_ref().unwrap();
        let metadata = binding
            .original_invocation_metadata_at_point(&tokens, &registry)
            .unwrap()
            .unwrap();
        assert_eq!(metadata.source_analysis_input(), Some(&input));
        let advice = binding.declaration_operand_layout_advice(&tokens).unwrap();
        let selected =
            crate::registry_invocation::declaration_invocation_flow(&registry, &tokens, &advice)
                .expect("genuine original caller-alias source recipe");
        assert_original_caller_alias_geometry(&selected, &tokens, written, captured);
        assert_eq!(binding.logical_source_name_advice_input(), Some(&input));
        assert!(advice.dialect().native_name_protocol().is_none());
        assert!(
            SourceAnalysisOptions::for_logical_source(&input)
                .unwrap()
                .native_entry
                .is_none()
        );

        let (older_tokens, _) = original(&older);
        assert_older_original_flow_absent(&older_tokens, &registry);
        let changed_grammar = ResolvedAnalysisInput::new(
            profile,
            profile,
            Arc::clone(&current),
            tcl_lexer::LexerConfig {
                strict_quoting: !config.strict_quoting,
                ..config
            },
        );
        let foreign_input =
            ResolvedAnalysisInput::new(profile, profile, Arc::clone(&foreign), config);
        for owner in [
            OwnedInvocationMetadataContext::Unavailable,
            OwnedInvocationMetadataContext::Supplied(Arc::clone(&current)),
            OwnedInvocationMetadataContext::for_source_input(Some(&changed_grammar)),
            OwnedInvocationMetadataContext::for_source_input(Some(&foreign_input)),
        ] {
            let mut changed = tokens.clone();
            let point = Arc::make_mut(
                changed
                    .source_binding
                    .as_mut()
                    .unwrap()
                    .lookup_state
                    .as_mut()
                    .unwrap(),
            );
            Arc::make_mut(&mut point.state.baseline).metadata_context = owner;
            assert_original_metadata_and_flow_refused(
                changed.source_binding.as_ref().unwrap(),
                &changed,
                &registry,
                &advice,
            );
        }
        let mut changed_vector = tokens.clone();
        changed_vector.word_exprs.pop();
        assert!(
            crate::registry_invocation::declaration_invocation_flow(
                &registry,
                &changed_vector,
                &advice,
            )
            .is_none()
        );
    }
}

#[test]
fn warmed_original_lexical_projection_cannot_donate_changed_module_metadata() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    let unit = unit("eval set ::original VALUE", &context);
    let module = &unit.ir_module;
    let script = &module.top_level;
    let tokens = script
        .retained_source_tokens_for_statement(&script.statements[0])
        .unwrap();
    let binding = tokens.source_binding.as_ref().unwrap();
    let site = binding.invocation_site().unwrap();
    let config = binding.original_lexer_config_for_tokens(tokens).unwrap();
    let first = site
        .source
        .original_lexical_command(tokens.words(), site.offset, config)
        .unwrap();
    let count = site.source.original_lexical_projection().capture_count();
    let second = site
        .source
        .original_lexical_command(tokens.words(), site.offset, config)
        .unwrap();
    assert!(Arc::ptr_eq(&first, &second));
    assert_eq!(
        site.source.original_lexical_projection().capture_count(),
        count
    );
    assert!(
        binding
            .original_invocation_metadata_for_module(tokens, module, context.commands())
            .is_some()
    );
    let input = module.source_metadata_input.as_ref().unwrap();
    let older = Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(Arc::clone(context.commands())),
    );
    assert!(Arc::ptr_eq(older.commands(), context.commands()));
    let foreign =
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
    assert!(!Arc::ptr_eq(foreign.commands(), context.commands()));
    for change in 0..5 {
        let mut changed = module.clone();
        match change {
            0 => changed.source_metadata_input = None,
            1 | 2 => {
                changed.source_metadata_input = Some(ResolvedAnalysisInput::new(
                    input.analyser_profile(),
                    input.unit_profile(),
                    if change == 1 {
                        Arc::clone(&older)
                    } else {
                        Arc::clone(&foreign)
                    },
                    input.lexer_config(),
                ));
            }
            3 => changed.lexer_config.strict_quoting = !changed.lexer_config.strict_quoting,
            4 => changed.source = tcl_lexer::SourceImage::document("eval set ::changed VALUE"),
            _ => unreachable!(),
        }
        assert!(
            binding
                .original_invocation_metadata_for_module(tokens, &changed, context.commands())
                .is_none(),
            "changed owner {change}"
        );
    }
}

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
fn original_point_metadata_keeps_unknown_lookup_and_changed_vector_separate() {
    // naming.diagnostic.original-materialized-write-footprint
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-materialized-write-footprint.md
    // Source/API ownership only: metadata does not prove a reached command.
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    let unit = unit("missing_original_command VALUE", &context);
    let module = &unit.ir_module;
    let script = &module.top_level;
    let tokens = script
        .retained_source_tokens_for_statement(&script.statements[0])
        .unwrap();
    let binding = tokens.source_binding.as_ref().unwrap();
    assert!(binding.unknown || binding.may_be_absent);
    assert_eq!(
        binding
            .original_invocation_metadata_for_module(tokens, module, context.commands())
            .unwrap()
            .source_analysis_input(),
        module.source_metadata_input.as_ref(),
    );
    assert!(
        binding
            .original_materialized_footprint_for_module(tokens, module, context.commands())
            .is_none()
    );
    let mut changed = tokens.clone();
    changed.word_exprs.pop();
    assert!(
        binding
            .original_invocation_metadata_for_module(&changed, module, context.commands())
            .is_none()
    );
    let mut missing = module.clone();
    missing.source_metadata_input = None;
    assert!(
        binding
            .original_invocation_metadata_for_module(tokens, &missing, context.commands())
            .is_none()
    );
}

#[test]
fn original_function_point_metadata_retains_input_without_granting_module_or_normality() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Actual FU/point metadata is distinct from whole-Module currency and
    // from a reached handler, successful read, physical frame or result.
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    let unit = unit("missing_original_command VALUE", &context);
    let function = &unit.top_level;
    let script = &unit.ir_module.top_level;
    let tokens = script
        .retained_source_tokens_for_statement(&script.statements[0])
        .unwrap();
    let binding = tokens.source_binding.as_ref().unwrap();
    assert!(binding.unknown || binding.may_be_absent);
    let metadata = binding
        .original_invocation_metadata_for_function(tokens, function, context.commands())
        .unwrap()
        .context()
        .unwrap();
    assert_eq!(
        metadata.source_analysis_input(),
        function.source_metadata_input()
    );
    assert!(
        crate::registry_invocation::normal_representation_invocation_with_metadata_context(
            context.commands(),
            Some(metadata),
            tokens,
        )
        .is_none()
    );
    let mut changed = unit.ir_module.clone();
    changed.top_level_namespace = "::changed".into();
    assert!(
        binding
            .original_invocation_metadata_for_module(tokens, &changed, context.commands())
            .is_none()
    );
    assert!(
        binding
            .original_invocation_metadata_for_function(tokens, function, context.commands())
            .is_some()
    );
}

#[test]
fn original_function_point_metadata_declines_missing_foreign_stale_and_changed_geometry() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Even a same-store availability change cannot replace the point's
    // original full input. Missing supplied input is never Standalone.
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    let unit = unit("missing_original_command VALUE", &context);
    let function = &unit.top_level;
    let script = &unit.ir_module.top_level;
    let tokens = script
        .retained_source_tokens_for_statement(&script.statements[0])
        .unwrap();
    let binding = tokens.source_binding.as_ref().unwrap();
    let input = function.source_metadata_input().unwrap();
    let older = Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(Arc::clone(context.commands())),
    );
    assert!(Arc::ptr_eq(older.commands(), context.commands()));
    for change in 0..4 {
        let mut changed = function.clone();
        match change {
            0 => changed.source_metadata_input = None,
            1 => {
                changed.source_metadata_input = Some(ResolvedAnalysisInput::new(
                    input.analyser_profile(),
                    input.unit_profile(),
                    Arc::clone(&older),
                    input.lexer_config(),
                ));
            }
            2 => {
                changed.source_metadata_input = Some(ResolvedAnalysisInput::new(
                    input.analyser_profile(),
                    input.unit_profile(),
                    tcl_registry::model::ingress::resolve_environment("tcl9.1")
                        .default_context_registry(),
                    input.lexer_config(),
                ));
            }
            3 => changed.source_config.strict_quoting = !changed.source_config.strict_quoting,
            _ => unreachable!(),
        }
        assert!(
            binding
                .original_invocation_metadata_for_function(tokens, &changed, context.commands(),)
                .is_none(),
            "changed facet {change}"
        );
    }
    let mut changed = tokens.clone();
    changed.word_exprs.pop();
    assert!(
        binding
            .original_invocation_metadata_for_function(&changed, function, context.commands(),)
            .is_none()
    );
    let registry = tcl_registry::CommandRegistry::build_default();
    let standalone = CompilationUnit::build_for("missing_original_command VALUE", &registry, false);
    let script = &standalone.ir_module.top_level;
    let tokens = script
        .retained_source_tokens_for_statement(&script.statements[0])
        .unwrap();
    assert!(standalone.top_level.source_metadata_input().is_none());
    assert!(
        tokens
            .source_binding
            .as_ref()
            .unwrap()
            .original_invocation_metadata_for_function(tokens, &standalone.top_level, &registry)
            .is_some()
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
        let entry = super::super::SourceAnalysisEntry {
            invocation_realm: realm,
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
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
#[test]
fn original_literal_body_ownership_keeps_named_purposes_and_literal_roots() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Conditional lexical names only, never installed aliases or current reads.
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    let source = "catch {global ::N::x; set {$literal} VALUE; set {café(open} VALUE; puts $missing; set byname}";
    let unit = unit(source, &context);
    let script = &unit.ir_module.top_level;
    let tokens = script
        .retained_source_tokens_for_statement(script.statements.last().unwrap())
        .unwrap();
    let metadata = InvocationMetadataContext::for_module(context.commands(), &unit.ir_module);
    for (purpose, expected_names, expected_reads) in [
        (
            crate::script_binds::Ownership::Bindings,
            vec!["x", "$literal", "café(open"],
            Vec::<&str>::new(),
        ),
        (
            crate::script_binds::Ownership::BindingsOrNameReads,
            vec!["x", "$literal", "café(open"],
            vec!["byname"],
        ),
        (
            crate::script_binds::Ownership::ScopeAliases,
            vec!["x"],
            Vec::<&str>::new(),
        ),
    ] {
        let names = crate::script_binds::original_literal_body_ownership(
            context.commands(),
            metadata,
            tokens,
            0,
            purpose,
        )
        .unwrap();
        assert!(!names.opaque, "{names:?}");
        assert_eq!(
            names
                .names
                .iter()
                .map(String::as_str)
                .collect::<std::collections::BTreeSet<_>>(),
            expected_names.into_iter().collect()
        );
        assert_eq!(
            names
                .read_names
                .iter()
                .map(String::as_str)
                .collect::<std::collections::BTreeSet<_>>(),
            expected_reads.into_iter().collect()
        );
        assert!(
            !names
                .names
                .iter()
                .chain(&names.read_names)
                .any(|name| name == "missing")
        );
    }
    assert!(
        crate::script_binds::original_literal_body_ownership(
            context.commands(),
            None,
            tokens,
            0,
            crate::script_binds::Ownership::Bindings,
        )
        .is_none()
    );
    assert!(
        crate::script_binds::original_literal_body_ownership(
            context.commands(),
            metadata,
            tokens,
            1,
            crate::script_binds::Ownership::Bindings,
        )
        .is_none()
    );
}

#[test]
fn original_literal_body_aliases_keep_local_names_and_opaque_residuals() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Held alias names/captures and a conditional residual, no frame or execution.
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    for (source, expected, opaque) in [
        (
            "interp alias {} expose {} global ::N::x; catch {expose}",
            Some("x"),
            false,
        ),
        (
            "rename global retired; interp alias {} expose {} retired ::N::x; catch {expose}",
            Some("x"),
            false,
        ),
        (
            "interp alias {} expose {} global ::N::x; rename global {}; catch {expose}",
            None,
            true,
        ),
        ("catch {set known VALUE; eval $script}", None, true),
    ] {
        let unit = unit(source, &context);
        let script = &unit.ir_module.top_level;
        let tokens = script
            .retained_source_tokens_for_statement(script.statements.last().unwrap())
            .unwrap();
        let names = crate::script_binds::original_literal_body_ownership(
            context.commands(),
            InvocationMetadataContext::for_module(context.commands(), &unit.ir_module),
            tokens,
            0,
            crate::script_binds::Ownership::ScopeAliases,
        )
        .unwrap();
        assert_eq!(
            names.names.first().map(String::as_str),
            expected,
            "{source}: {names:?}"
        );
        assert_eq!(names.opaque, opaque, "{source}: {names:?}");
    }
}

#[test]
fn original_literal_body_ownership_keeps_availability_and_source_currency() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Conditional source names only; availability is not actual execution.
    let current =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    let older = Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(Arc::clone(current.commands())),
    );
    for (context, expected_opaque) in [(&current, false), (&older, true)] {
        let unit = unit("catch {throw {ERR} message}", context);
        let script = &unit.ir_module.top_level;
        let tokens = script
            .retained_source_tokens_for_statement(script.statements.last().unwrap())
            .unwrap();
        let metadata = InvocationMetadataContext::for_module(context.commands(), &unit.ir_module);
        let ownership = crate::script_binds::original_literal_body_ownership(
            context.commands(),
            metadata,
            tokens,
            0,
            crate::script_binds::Ownership::Bindings,
        )
        .unwrap();
        assert_eq!(ownership.opaque, expected_opaque, "{ownership:?}");
        let foreign =
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
        assert!(
            crate::script_binds::original_literal_body_ownership(
                foreign.commands(),
                metadata,
                tokens,
                0,
                crate::script_binds::Ownership::Bindings,
            )
            .is_none()
        );
        let mut changed = tokens.clone();
        let crate::ir::WordExpr::BracedLiteral { text, .. } = &mut changed.word_exprs[1] else {
            panic!("original body word");
        };
        *text = "set fabricated VALUE".to_owned();
        assert!(
            crate::script_binds::original_literal_body_ownership(
                context.commands(),
                metadata,
                &changed,
                0,
                crate::script_binds::Ownership::Bindings,
            )
            .is_none()
        );
    }
}

fn logical_script_context() -> Arc<tcl_registry::model::ContextRegistry> {
    let mut registry = tcl_registry::CommandRegistry::build_default();
    registry.insert(tcl_registry::CommandSpec {
        name: "run-script",
        arity: tcl_registry::Arity::exact(1),
        arg_roles: &[(0, tcl_registry::ArgRole::Body)],
        ..tcl_registry::CommandSpec::DEFAULT
    });
    let environment = tcl_registry::model::ingress::resolve_environment("tcl");
    Arc::new(
        tcl_registry::model::context_for_profile(environment.analyser_profile())
            .with_command_store(Arc::new(registry)),
    )
}

fn logical_original_tokens(
    source: &str,
    context: &Arc<tcl_registry::model::ContextRegistry>,
) -> (crate::analyser::AnalysisResult, crate::ir::CommandTokens) {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input = ResolvedAnalysisInput::new(profile, profile, Arc::clone(context), config);
    assert!(input.has_logical_source_name_context());
    let analysis = crate::analyser::Analyser::new()
        .with_resolved_input(input)
        .analyse(source, "tcl");
    let segments = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
    let mut tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::new(source),
        config,
        segments.last().expect("actual final original command"),
    );
    analysis
        .retained_command_realm()
        .expect("actual analysis retains its original source world")
        .stamp_original_tokens(&mut tokens);
    (analysis, tokens)
}

#[test]
fn original_source_realm_keeps_sealed_declaration_and_reached_owners_independent() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Readonly source realm; no reached lookup or body/frame permission.
    let context = logical_script_context();
    let source = "interp alias {} shared {} run-script; shared {set local VALUE}";
    let (analysis, tokens) = logical_original_tokens(source, &context);
    let input = analysis.resolved_input.as_ref().unwrap();
    let binding = tokens.source_binding.as_ref().unwrap();
    assert!(binding.lookup_state.is_none());
    let advice = binding.declaration_operand_layout_advice(&tokens).unwrap();
    assert!(advice.closed_logical_source_lookup());
    assert_eq!(
        binding.original_source_realm_for_input(&tokens, input, context.commands()),
        Some(advice.realm())
    );
    assert!(binding.lookup_state.is_none());
    assert!(advice.dialect().native_name_protocol().is_none());

    let mut missing = tokens.clone();
    missing
        .source_binding
        .as_mut()
        .unwrap()
        .declaration_layout_observations = None;
    assert!(
        missing
            .source_binding
            .as_ref()
            .unwrap()
            .original_source_realm_for_input(&missing, input, context.commands())
            .is_none()
    );
    let mut changed = tokens.clone();
    changed.word_exprs.pop();
    assert!(
        binding
            .original_source_realm_for_input(&changed, input, context.commands())
            .is_none()
    );
    let older_context = Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(Arc::clone(context.commands())),
    );
    assert!(Arc::ptr_eq(context.commands(), older_context.commands()));
    let mut config = input.lexer_config();
    config.strict_quoting = !config.strict_quoting;
    let mut foreign_store = tcl_registry::CommandRegistry::build_default();
    foreign_store.insert(tcl_registry::CommandSpec {
        name: "foreign_source_realm_axis",
        ..tcl_registry::CommandSpec::DEFAULT
    });
    let foreign_context = Arc::new(context.with_command_store(Arc::new(foreign_store)));
    let native = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    for changed_input in [
        ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            older_context,
            input.lexer_config(),
        ),
        ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            Arc::clone(&context),
            config,
        ),
        ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            foreign_context,
            input.lexer_config(),
        ),
        ResolvedAnalysisInput::new(native, native, Arc::clone(&context), input.lexer_config()),
    ] {
        assert!(
            binding
                .original_source_realm_for_input(&tokens, &changed_input, context.commands())
                .is_none()
        );
    }
    // Present reached ownership is terminal even with valid declaration rows.
    let mut invalid = tokens.clone();
    let mut point = advice.snapshot.as_ref().clone();
    Arc::make_mut(&mut point.state.baseline).metadata_context =
        crate::registry_invocation::OwnedInvocationMetadataContext::Unavailable;
    invalid.source_binding.as_mut().unwrap().lookup_state = Some(Arc::new(point));
    assert!(
        invalid
            .source_binding
            .as_ref()
            .unwrap()
            .original_source_realm_for_input(&invalid, input, context.commands())
            .is_none()
    );
}

#[test]
fn original_logical_materialized_names_keep_static_and_captured_source_horizons() {
    // naming.diagnostic.original-materialized-write-footprint
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-materialized-write-footprint.md
    // Conditional MAY names; no reached handler, child entry or physical store.
    // An actual abrupt prefix retains lexical declarations without a source dispatch point.
    let context = logical_script_context();
    for source in [
        "return DONE; run-script {set {$literal} VALUE; set {café(open} VALUE; puts $read; set byname}",
        "interp alias {} shared {} run-script {set {$literal} VALUE; set {café(open} VALUE; puts $read; set byname}; return DONE; shared",
    ] {
        let (analysis, tokens) = logical_original_tokens(source, &context);
        let binding = tokens
            .source_binding
            .as_ref()
            .expect("original source binding");
        assert!(
            binding.lookup_state.is_none(),
            "declaration-only Logical purpose"
        );
        let input = analysis.resolved_input.as_ref().unwrap();
        let advice = binding.declaration_operand_layout_advice(&tokens).unwrap();
        assert!(advice.closed_logical_source_lookup());
        assert!(advice.dialect().native_string_protocol().is_none());
        let words =
            crate::registry_invocation::source_structure::original_registry_words_for_tokens(
                source, &analysis, &tokens,
            )
            .expect("selected original source schema");
        let bodies = words.source_script_bodies_for(
            &context,
            crate::registry_invocation::OriginalSourceScriptPurpose::PotentialEvaluation,
        );
        assert_eq!(
            bodies.len(),
            1,
            "original selected immediate body: {source}"
        );
        let footprint = binding
            .original_materialized_footprint(
                &tokens,
                source,
                context.commands(),
                InvocationMetadataContext::for_analysis_input(context.commands(), input),
            )
            .expect("sealed Logical installer horizon");
        let names = footprint.body_name_ownership(&bodies[0]);
        assert!(!names.opaque, "{source}: {names:?}");
        assert!(names.names.iter().any(|name| name == "$literal"));
        assert!(names.names.iter().any(|name| name == "café(open"));
        assert!(names.read_names.iter().any(|name| name == "byname"));
        assert!(
            !names
                .names
                .iter()
                .chain(&names.read_names)
                .any(|name| name == "read")
        );
        assert!(
            binding.lookup_state.is_none(),
            "query issues no reached point"
        );
    }
}

fn assert_original_materialized_footprint_refused(
    binding: &crate::command_binding::SourceInvocationBinding,
    tokens: &crate::ir::CommandTokens,
    source: &str,
    registry: &tcl_registry::CommandRegistry,
    metadata: Option<InvocationMetadataContext<'_>>,
) {
    assert!(
        binding
            .original_materialized_footprint(tokens, source, registry, metadata)
            .is_none()
    );
}

fn assert_materialized_foreign_owner_refused(
    binding: &crate::command_binding::SourceInvocationBinding,
    tokens: &crate::ir::CommandTokens,
    source: &str,
    input: &ResolvedAnalysisInput,
) -> (
    Arc<tcl_registry::model::ContextRegistry>,
    ResolvedAnalysisInput,
) {
    let mut foreign_registry = tcl_registry::CommandRegistry::build_default();
    foreign_registry.insert(tcl_registry::CommandSpec {
        name: "foreign_materialized_source_axis",
        ..tcl_registry::CommandSpec::DEFAULT
    });
    let foreign_context = Arc::new(
        tcl_registry::model::context_for_profile(input.analyser_profile())
            .with_command_store(Arc::new(foreign_registry)),
    );
    let foreign = ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        Arc::clone(&foreign_context),
        input.lexer_config(),
    );
    assert!(
        binding
            .original_materialized_footprint(
                tokens,
                source,
                foreign_context.commands(),
                InvocationMetadataContext::for_analysis_input(foreign_context.commands(), &foreign),
            )
            .is_none()
    );

    (foreign_context, foreign)
}

fn assert_materialized_native_owner_refused(
    binding: &crate::command_binding::SourceInvocationBinding,
    tokens: &crate::ir::CommandTokens,
    source: &str,
    input: &ResolvedAnalysisInput,
    context: &Arc<tcl_registry::model::ContextRegistry>,
) -> ResolvedAnalysisInput {
    let native_profile =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let native_input = ResolvedAnalysisInput::new(
        native_profile,
        native_profile,
        Arc::clone(context),
        input.lexer_config(),
    );
    assert!(!native_input.has_logical_source_name_context());
    assert!(
        binding
            .original_materialized_footprint(
                tokens,
                source,
                context.commands(),
                InvocationMetadataContext::for_analysis_input(context.commands(), &native_input),
            )
            .is_none()
    );

    native_input
}

fn assert_materialized_missing_owner_refused(
    binding: &crate::command_binding::SourceInvocationBinding,
    tokens: &crate::ir::CommandTokens,
    source: &str,
    context: &tcl_registry::model::ContextRegistry,
    metadata: Option<InvocationMetadataContext<'_>>,
) -> crate::command_binding::SourceInvocationBinding {
    let mut missing = binding.clone();
    missing.declaration_layout_observations = None;
    assert!(
        missing
            .original_materialized_footprint(tokens, source, context.commands(), metadata)
            .is_none()
    );
    missing
}

fn assert_materialized_unavailable_owner_refused(
    binding: &crate::command_binding::SourceInvocationBinding,
    tokens: &crate::ir::CommandTokens,
    source: &str,
    context: &tcl_registry::model::ContextRegistry,
    metadata: Option<InvocationMetadataContext<'_>>,
) -> (
    crate::command_binding::SourceInvocationBinding,
    crate::ir::CommandTokens,
) {
    let mut unavailable = binding.clone();
    let mut rows = unavailable
        .declaration_layout_observations
        .as_deref()
        .unwrap()
        .to_vec();
    for row in &mut rows {
        let mut state = row.snapshot.state.clone();
        Arc::make_mut(&mut state.baseline).metadata_context =
            crate::registry_invocation::OwnedInvocationMetadataContext::Unavailable;
        row.snapshot = Arc::new(super::super::SourceLookupSnapshot::in_realm(
            state,
            row.snapshot.realm,
        ));
    }
    unavailable.declaration_layout_observations = Some(Arc::from(rows));
    let mut unavailable_tokens = tokens.clone();
    unavailable_tokens.source_binding = Some(unavailable.clone());
    assert!(
        unavailable
            .original_materialized_footprint(
                &unavailable_tokens,
                source,
                context.commands(),
                metadata
            )
            .is_none()
    );

    (unavailable, unavailable_tokens)
}

fn assert_materialized_alternatives_owner_refused(
    binding: &crate::command_binding::SourceInvocationBinding,
    tokens: &crate::ir::CommandTokens,
    source: &str,
    context: &tcl_registry::model::ContextRegistry,
    metadata: Option<InvocationMetadataContext<'_>>,
) -> (
    crate::command_binding::SourceInvocationBinding,
    crate::ir::CommandTokens,
) {
    // Matching head targets do not identify a whole installer world.
    // A second original row with a different nested setter table cannot
    // donate the first row's materialized name facts.
    let mut alternatives = binding.clone();
    let original_row = super::super::declaration_layout::original_declaration_layouts(
        binding.declaration_layout_observations.as_deref().unwrap(),
    )
    .unwrap()
    .next()
    .unwrap()
    .clone();
    let mut different = original_row.clone();
    let mut state = different.snapshot.state.clone();
    state.remove("::set");
    different.snapshot = Arc::new(super::super::SourceLookupSnapshot::in_realm(
        state,
        different.snapshot.realm,
    ));
    alternatives.declaration_layout_observations = Some(Arc::from(vec![original_row, different]));
    let mut alternative_tokens = tokens.clone();
    alternative_tokens.source_binding = Some(alternatives.clone());
    assert!(
        alternatives
            .declaration_operand_layout_advice(&alternative_tokens)
            .unwrap()
            .closed_logical_source_lookup()
    );
    assert!(
        alternatives
            .original_materialized_footprint(
                &alternative_tokens,
                source,
                context.commands(),
                metadata
            )
            .is_none()
    );

    (alternatives, alternative_tokens)
}

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

#[test]
fn original_logical_materialized_names_refuse_changed_and_missing_source_owners() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // An actual abrupt prefix retains lexical declarations without a source dispatch point.
    let context = logical_script_context();
    let source = "return DONE; run-script {set x VALUE}";
    let (analysis, tokens) = logical_original_tokens(source, &context);
    let binding = tokens.source_binding.as_ref().unwrap();
    assert!(binding.lookup_state.is_none());
    let input = analysis.resolved_input.as_ref().unwrap();
    let metadata = InvocationMetadataContext::for_analysis_input(context.commands(), input);
    assert!(
        binding
            .original_materialized_footprint(&tokens, source, context.commands(), metadata)
            .is_some()
    );
    assert!(
        binding
            .original_materialized_footprint(&tokens, source, context.commands(), None)
            .is_none()
    );
    assert_original_materialized_footprint_refused(
        binding,
        &tokens,
        "run-script {set x OTHER}",
        context.commands(),
        metadata,
    );

    let older_context = Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(Arc::clone(context.commands())),
    );
    assert!(Arc::ptr_eq(context.commands(), older_context.commands()));
    let older = ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        older_context,
        input.lexer_config(),
    );
    assert!(
        binding
            .original_materialized_footprint(
                &tokens,
                source,
                context.commands(),
                InvocationMetadataContext::for_analysis_input(context.commands(), &older),
            )
            .is_none()
    );
    let changed = ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        Arc::clone(&context),
        tcl_lexer::LexerConfig {
            expand_syntax: !input.lexer_config().expand_syntax,
            ..input.lexer_config()
        },
    );
    assert!(
        binding
            .original_materialized_footprint(
                &tokens,
                source,
                context.commands(),
                InvocationMetadataContext::for_analysis_input(context.commands(), &changed),
            )
            .is_none()
    );
    let (_foreign_context, _foreign) =
        assert_materialized_foreign_owner_refused(binding, &tokens, source, input);
    let _native_input =
        assert_materialized_native_owner_refused(binding, &tokens, source, input, &context);
    let _missing =
        assert_materialized_missing_owner_refused(binding, &tokens, source, &context, metadata);
    let (_unavailable, _unavailable_tokens) =
        assert_materialized_unavailable_owner_refused(binding, &tokens, source, &context, metadata);
    let (_alternatives, _alternative_tokens) = assert_materialized_alternatives_owner_refused(
        binding, &tokens, source, &context, metadata,
    );
    // A retained reached owner is authoritative even when stale. The
    // earlier valid declaration cannot repair this explicit invalid point.
    let mut invalid_point = binding.clone();
    let point = advice_snapshot(binding, &tokens);
    let mut state = point.state.clone();
    Arc::make_mut(&mut state.baseline).metadata_context =
        crate::registry_invocation::OwnedInvocationMetadataContext::Unavailable;
    invalid_point.lookup_state = Some(Arc::new(super::super::SourceLookupSnapshot::in_realm(
        state,
        point.realm,
    )));
    let mut invalid_tokens = tokens.clone();
    invalid_tokens.source_binding = Some(invalid_point.clone());
    assert!(
        invalid_point
            .original_materialized_footprint(&invalid_tokens, source, context.commands(), metadata)
            .is_none()
    );
}

fn advice_snapshot(
    binding: &super::SourceInvocationBinding,
    tokens: &crate::ir::CommandTokens,
) -> Arc<super::super::SourceLookupSnapshot> {
    binding
        .declaration_operand_layout_advice(tokens)
        .unwrap()
        .snapshot
}

#[test]
fn original_logical_materialized_horizon_refuses_dynamic_expanded_and_invalid_vectors() {
    // naming.diagnostic.original-materialized-write-footprint
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-materialized-write-footprint.md
    // An actual abrupt prefix retains lexical declarations without a source dispatch point.
    let context = logical_script_context();
    for (source, expected) in [
        ("return DONE; run-script {set x VALUE}", true),
        ("return DONE; run-script $script", false),
        ("return DONE; run-script [unknown]", false),
        ("return DONE; run-script {*}{set x VALUE}", false),
    ] {
        let (analysis, tokens) = logical_original_tokens(source, &context);
        let binding = tokens.source_binding.as_ref().unwrap();
        let rows = super::super::declaration_layout::original_declaration_layouts(
            binding.declaration_layout_observations.as_deref().unwrap(),
        )
        .expect("actual original declaration row");
        assert!(rows.clone().all(|row| {
            row.snapshot
                .state
                .source_variables
                .invocation_dialect
                .unwrap()
                .native_string_protocol()
                .is_none()
        }));
        assert_eq!(
            rows.clone()
                .all(|row| binding.original_operands_preserve_lookup(
                    &tokens,
                    context.commands(),
                    row,
                    0
                )),
            expected,
            "{source}"
        );
        let input = analysis.resolved_input.as_ref().unwrap();
        assert_eq!(
            binding
                .original_materialized_footprint(
                    &tokens,
                    source,
                    context.commands(),
                    InvocationMetadataContext::for_analysis_input(context.commands(), input),
                )
                .is_some(),
            expected,
            "{source}"
        );
        if expected {
            let mut altered = tokens.clone();
            let crate::ir::WordExpr::BracedLiteral { text, .. } = &mut altered.word_exprs[1] else {
                panic!("original whole braced body");
            };
            *text = "set fabricated VALUE".to_owned();
            assert!(
                rows.clone()
                    .all(|row| !binding.original_operands_preserve_lookup(
                        &altered,
                        context.commands(),
                        row,
                        0
                    ))
            );
            assert!(
                binding
                    .original_materialized_footprint(
                        &altered,
                        source,
                        context.commands(),
                        InvocationMetadataContext::for_analysis_input(context.commands(), input),
                    )
                    .is_none()
            );
        }
    }
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
    let config = super::super::SourceAnalysisOptions {
        native_entry: Some(&entry),
        ..Default::default()
    }
    .native_lexer_config(tcl_lexer::LexerConfig::for_file_grammar(profile.grammar));
    let input = ResolvedAnalysisInput::new(profile, profile, Arc::clone(&context), config);
    let options = super::super::SourceAnalysisOptions {
        metadata_context: crate::registry_invocation::InvocationMetadataInput::SuppliedSource(
            Some(&input),
        ),
        native_entry: Some(&entry),
        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
        native_compilation: crate::environment_ingress::authoring_native_compilation(),
        ..Default::default()
    };
    let image = tcl_lexer::SourceImage::document(source);
    let bindings = super::super::SourceCommandBindings::analyse_image_in_frame_with_options(
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
            && target.kind == super::super::BindingKind::Proc
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
        super::super::SourceRuntimeReachability::Reached
    );
    assert_ne!(
        binding.runtime_reachability(),
        super::super::SourceRuntimeReachability::Conditional
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

#[test]
fn original_logical_materialized_root_keeps_its_actual_source_lookup_point() {
    // naming.diagnostic.original-materialized-write-footprint
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-materialized-write-footprint.md
    // The root source point supplies conditional schema and MAY names;
    // it is distinct from an unentered suffix and grants no handler/store.
    let context = logical_script_context();
    let source = "run-script {set x VALUE}";
    let (analysis, tokens) = logical_original_tokens(source, &context);
    let input = analysis.resolved_input.as_ref().unwrap();
    let binding = tokens.source_binding.as_ref().unwrap();
    let point = binding.lookup_state.as_ref().unwrap();
    assert_eq!(point.state.logical_source_name_advice_input(), Some(input));
    assert!(point.state.baseline.native_entry.is_none());
    assert_eq!(
        binding.runtime_reachability(),
        super::super::SourceRuntimeReachability::Conditional
    );
    let words = crate::registry_invocation::source_structure::original_registry_words_for_tokens(
        source, &analysis, &tokens,
    )
    .expect("same original source point and Registry source role");
    let bodies = words.source_script_bodies_for(
        &context,
        crate::registry_invocation::OriginalSourceScriptPurpose::PotentialEvaluation,
    );
    assert_eq!(bodies.len(), 1);
    let metadata = InvocationMetadataContext::for_analysis_input(context.commands(), input);
    let footprint = binding
        .original_materialized_footprint(&tokens, source, context.commands(), metadata)
        .unwrap();
    let names = footprint.body_name_ownership(&bodies[0]);
    assert!(!names.opaque);
    assert!(names.names.iter().any(|name| name == "x"));
    assert!(binding.proved_execution_target().is_none());
    assert!(binding.proved_handler_target().is_none());
    assert!(
        binding
            .lookup_state
            .as_ref()
            .is_some_and(|retained| Arc::ptr_eq(retained, point))
    );
}
