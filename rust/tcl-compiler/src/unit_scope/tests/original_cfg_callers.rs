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

//! The caller inventory of original CFG statements and nested bodies under their retained owners.

use super::*;

fn original_caller_fixture(source: &str, config: tcl_lexer::LexerConfig) -> IrModule {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
    let registry = CommandRegistry::build_default();
    let context = std::sync::Arc::new(
        tcl_registry::model::context_for_profile(profile)
            .with_command_store(std::sync::Arc::new(registry)),
    );
    let input = crate::analyser::ResolvedAnalysisInput::new(profile, profile, context, config);
    assert!(input.has_logical_source_name_context());
    crate::lowering::lower_to_ir_with(
        crate::lowering::Lowerer::with_config(input.borrowed_context_registry().commands(), config)
            .with_dialect(Some(profile))
            .with_resolved_analysis_input(input.clone()),
        source,
    )
}

fn original_caller_cfg_evidence(
    source: &str,
    module: &IrModule,
    cfg: &CfgModule,
) -> CallSiteEvidence {
    let input = module.source_metadata_input.as_ref().unwrap();
    let registry = module.resolved_registry();
    let identities = crate::realm::document_realm_bindings_with_source_entry(
        source,
        module.lexer_config,
        registry,
        &module.source_entry,
    )
    .with_resolved_analysis_input(input.clone());
    collect_call_site_constants(
        cfg,
        &[],
        &module.procedures,
        &module.future_call_sites,
        CallSiteSourceContext {
            registry,
            module,
            declared: None,
            input: Some(input),
            identities: &identities,
        },
    )
}

#[test]
fn original_cfg_caller_inventory_keeps_consumed_expression_source_and_configuration() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Source caller alternatives only; no Native parameter or Normal grant.
    let profile = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let source =
        "proc helper {mode} {return $mode}\nhelper FIXED\nset result [expr {[helper DIFFERENT]}]";
    let module = original_caller_fixture(source, config);
    assert!(fixture_id_if_present(&module.procedures, "::helper").is_some());
    let cfg = crate::cfg_builder::build_cfg(&module, false);
    assert!(
        cfg.top_level.blocks.iter().any(|(&block, data)| {
            data.statements.iter().enumerate().any(|(index, stmt)| {
                matches!(stmt, Statement::AssignExpr { .. })
                    && stmt.tokens().is_none()
                    && cfg.top_level.source_tokens_at(block, index).is_some()
            })
        }),
        "the consumed expression keeps its genuine sidecar"
    );
    let current = original_caller_cfg_evidence(source, &module, &cfg);
    let caller = current.get("::helper").expect("actual caller alternatives");
    assert_eq!(
        caller.slots[&0].values,
        BTreeSet::from(["FIXED".to_owned(), "DIFFERENT".to_owned()])
    );
    assert_eq!(uniform(&current, "::helper", 0), None);
    assert_eq!(module.lexer_config, config);
    assert!(module.source_entry.native_entry.is_none());
}

#[test]
fn original_cfg_caller_inventory_refuses_missing_foreign_and_stale_owners() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let profile = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
    let source = "proc helper {mode} {return $mode}\nhelper FIXED";
    let module = original_caller_fixture(
        source,
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    let cfg = crate::cfg_builder::build_cfg(&module, false);
    let current = original_caller_cfg_evidence(source, &module, &cfg);
    assert_eq!(uniform(&current, "::helper", 0).as_deref(), Some("FIXED"));
    for owner in [
        crate::registry_invocation::OwnedInvocationMetadataContext::Unavailable,
        crate::registry_invocation::OwnedInvocationMetadataContext::Standalone,
    ] {
        let mut unavailable = cfg.clone();
        unavailable.top_level.metadata_context = owner;
        let evidence = original_caller_cfg_evidence(source, &module, &unavailable);
        assert!(evidence.get("::helper").unwrap().opaque_caller);
        assert_eq!(uniform(&evidence, "::helper", 0), None);
    }
    let input = module.source_metadata_input.as_ref().unwrap();
    let mut changed_config = input.lexer_config();
    changed_config.leading_bom = if changed_config.leading_bom == tcl_lexer::LeadingBom::Skip {
        tcl_lexer::LeadingBom::Content
    } else {
        tcl_lexer::LeadingBom::Skip
    };
    let changed = crate::analyser::ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        input.context_registry(),
        changed_config,
    );
    let mut foreign_registry = CommandRegistry::build_default();
    foreign_registry.insert(tcl_registry::CommandSpec {
        name: "foreign_caller_source_store",
        ..tcl_registry::CommandSpec::DEFAULT
    });
    let foreign_context = std::sync::Arc::new(
        tcl_registry::model::context_for_profile(profile)
            .with_command_store(std::sync::Arc::new(foreign_registry)),
    );
    let foreign = supplied_unit_input(input, foreign_context);
    for replacement in [changed, foreign] {
        let mut unavailable = cfg.clone();
        unavailable.top_level.metadata_context =
            crate::registry_invocation::OwnedInvocationMetadataContext::SuppliedSource(Box::new(
                replacement,
            ));
        let evidence = original_caller_cfg_evidence(source, &module, &unavailable);
        assert!(evidence.get("::helper").unwrap().opaque_caller);
        assert_eq!(uniform(&evidence, "::helper", 0), None);
    }
    let mut changed = module.clone();
    changed.lexer_config = changed_config;
    let evidence = original_caller_cfg_evidence(source, &changed, &cfg);
    assert!(evidence.get("::helper").unwrap().opaque_caller);
    assert_eq!(uniform(&evidence, "::helper", 0), None);
}

fn assert_foreign_caller_sidecar(
    tokens: &crate::ir::CommandTokens,
    command: &tcl_lexer::NativeScriptCommandWords,
    ctx: &CallSiteScanCtx<'_, impl std::hash::BuildHasher>,
    caller: &CallerFrame<'_>,
    segment: &crate::segmenter::SegmentedCommand,
) {
    let mut foreign = tokens.clone();
    let offset = command.span.start();
    let unrelated = tokens.source_binding.clone().unwrap();
    foreign.nested_bindings_mut().push((offset, unrelated));
    let mut withdrawn = CallSiteEvidence::default();
    record_call_site_evidence(
        &mut withdrawn,
        ctx,
        caller,
        segment.name(),
        segment.args(),
        Some(&foreign),
        0,
    );
    assert!(withdrawn.get("::helper").unwrap().opaque_caller);
    assert_eq!(uniform(&withdrawn, "::helper", 0), None);
}

#[test]
fn original_body_caller_inventory_requires_each_original_child_sidecar() {
    // naming.source.original-script-region-purpose
    // docs/design/analysis/name-resolution-proofs/original-script-region-purpose.md
    // Whole authored body syntax is separate from dispatch and Native frame entry.
    let profile = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
    let source = "proc helper {mode} {return $mode}\nhelper FIXED\ncatch {helper DIFFERENT}";
    let module = original_caller_fixture(
        source,
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    let cfg = crate::cfg_builder::build_cfg(&module, false);
    let input = module.source_metadata_input.as_ref().unwrap();
    let registry = module.resolved_registry();
    let realm = crate::realm::document_realm_bindings_with_source_entry(
        source,
        module.lexer_config,
        registry,
        &module.source_entry,
    )
    .with_resolved_analysis_input(input.clone());
    let segment =
        crate::segmenter::segment_commands_with_offset_and_config(source, 0, module.lexer_config)
            .pop()
            .unwrap();
    let mut tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::new(source),
        module.lexer_config,
        &segment,
    );
    realm.stamp_original_tokens(&mut tokens);
    let words =
        crate::registry_invocation::source_structure::original_registry_words_for_supplied_tokens(
            &realm, input, &tokens,
        )
        .unwrap();
    let context = input.context_registry();
    let bodies = words
        .source_script_bodies_for_mutation_coverage(&context)
        .unwrap();
    assert_eq!(bodies.len(), 1);
    let commands = bodies[0].original_commands(&context).unwrap();
    assert_eq!(commands.len(), 1);
    assert!(commands[0].span.start() > tokens.argv[0].start());
    let command = &commands[0];
    assert_eq!(command.words[0].bytes(), b"helper");
    let previous = CallSiteEvidence::default();
    let facts = ModuleVarFacts::default();
    let known = HashSet::from(["::helper".to_owned()]);
    let reach = vec!["::helper".to_owned()];
    let ctx = CallSiteScanCtx {
        known: &known,
        registry,
        module: &module,
        source_input: Some(input),
        identities: &realm,
        declared: None,
        var_facts: &facts,
        previous: &previous,
        procedures: &module.procedures,
        unenumerable_reach: &reach,
        unresolved_handler: None,
    };
    let caller = CallerFrame {
        metadata_owner: &cfg.top_level.metadata_context,
        scope: CallerScopeId::TopLevel,
        params: &[],
        callers_tracked: true,
    };
    let mut current = CallSiteEvidence::default();
    record_call_site_evidence(
        &mut current,
        &ctx,
        &caller,
        segment.name(),
        segment.args(),
        Some(&tokens),
        0,
    );
    assert_eq!(
        uniform(&current, "::helper", 0).as_deref(),
        Some("DIFFERENT")
    );
    let mut missing = tokens.clone();
    missing.nested_bindings_mut().clear();
    let mut withdrawn = CallSiteEvidence::default();
    record_call_site_evidence(
        &mut withdrawn,
        &ctx,
        &caller,
        segment.name(),
        segment.args(),
        Some(&missing),
        0,
    );
    assert!(withdrawn.get("::helper").unwrap().opaque_caller);
    assert_eq!(uniform(&withdrawn, "::helper", 0), None);
    assert_foreign_caller_sidecar(&tokens, command, &ctx, &caller, &segment);
}
