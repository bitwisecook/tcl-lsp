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

//! Original invocation metadata for a module, function or point, and the owners it refuses.

use super::*;
use crate::command_binding::{SourceOriginLexical, SourceOriginProjection};

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
