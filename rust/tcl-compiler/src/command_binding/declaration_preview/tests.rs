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

fn complete_logical_declaration_tokens(
    source: &str,
    command: &str,
    context: &Arc<tcl_registry::model::ContextRegistry>,
) -> (
    SourceCommandBindings,
    crate::ir::CommandTokens,
    crate::analyser::ResolvedAnalysisInput,
) {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input =
        crate::analyser::ResolvedAnalysisInput::new(profile, profile, Arc::clone(context), config);
    let bindings = SourceCommandBindings::analyse_with_options(
        source,
        config,
        context.commands(),
        super::super::SourceAnalysisOptions::for_logical_source(&input).unwrap(),
    );
    let offset = u32::try_from(source.find(command).unwrap()).unwrap();
    let image = tcl_lexer::SourceImage::document(source);
    let part = tcl_lexer::SourceImage::document(command);
    let segments =
        tcl_source::segment::segment_commands_image_with_offset_and_config(&part, offset, config)
            .unwrap();
    let [segment] = segments.as_slice() else {
        panic!("one original source vector");
    };
    let mut tokens = crate::ir::CommandTokens::from_segmented(&image.source_map(), config, segment);
    bindings.stamp_original_tokens(&mut tokens);
    (bindings, tokens, input)
}

#[test]
fn original_declaration_metadata_keeps_unentered_logical_successor_source() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Original declaration metadata supplies conditional source structure;
    // no reached lookup, Native frame, handler or compiled store is donated.
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let source = "proc f {} {set p literal.txt; open $p}";
    for command in ["set p literal.txt", "open $p"] {
        let (_, tokens, input) = complete_logical_declaration_tokens(source, command, &context);
        let binding = tokens.source_binding.as_ref().unwrap();
        assert!(binding.lookup_state.is_none(), "unentered procedure source");
        assert!(binding.logical_source_name_advice_input().is_none());
        assert!(binding.proved_handler_target().is_none());
        assert!(binding.native_structured_preparation.is_none());
        assert!(
            binding
                .original_invocation_metadata_at_point(&tokens, context.commands())
                .is_err()
        );
        let metadata = binding
            .original_invocation_metadata_for_input(&tokens, &input, context.commands())
            .expect("same original Logical declaration source owner");
        assert_eq!(metadata.source_analysis_input(), Some(&input));
        assert!(crate::registry_invocation::original_logical_operation_invocation_with_metadata_context(
                context.commands(), metadata, &tokens,
            ).is_some(), "{command}");
        let advice = binding
            .declaration_operand_layout_advice(&tokens)
            .expect("the exact original successor layout");
        assert!(advice.closed_logical_source_lookup());
        let mut raw = tokens.clone();
        raw.source_binding = None;
        let raw_metadata = advice
            .original_declaration_metadata_context(context.commands(), &raw)
            .expect("raw preview vector retains its sealed advice")
            .unwrap();
        assert_eq!(raw_metadata.source_analysis_input(), Some(&input));
        let flow = crate::registry_invocation::declaration_invocation_flow(
            context.commands(),
            &raw,
            &advice,
        )
        .expect("source flow needs no entered binding");
        if command.starts_with("set ") {
            assert_eq!(
                flow.literal_stores,
                vec![("p".to_owned(), "literal.txt".to_owned())]
            );
        }
        assert!(raw.source_binding.is_none());
    }
}

fn assert_shadowed_declaration_cannot_supply_logical_operation(
    context: &Arc<tcl_registry::model::ContextRegistry>,
    registry: &tcl_registry::CommandRegistry,
) {
    let (_, shadowed, shadow_input) = complete_logical_declaration_tokens(
        "proc metadata_store args {return CUSTOM}; proc f {} {metadata_store p VALUE}",
        "metadata_store p VALUE",
        context,
    );
    let shadow_binding = shadowed.source_binding.as_ref().unwrap();
    if let Some(metadata) =
        shadow_binding.original_invocation_metadata_for_input(&shadowed, &shadow_input, registry)
    {
        assert!(crate::registry_invocation::original_logical_operation_invocation_with_metadata_context(
                registry, metadata, &shadowed,
            ).is_none());
    }
}

fn assert_original_declaration_metadata_refused(
    binding: &crate::command_binding::SourceInvocationBinding,
    tokens: &crate::ir::CommandTokens,
    input: &crate::analyser::ResolvedAnalysisInput,
    registry: &tcl_registry::CommandRegistry,
) {
    assert!(
        binding
            .original_invocation_metadata_for_input(tokens, input, registry)
            .is_none()
    );
}

#[test]
fn original_declaration_metadata_refuses_missing_foreign_and_changed_source_owners() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Same command store does not identify the availability owner; exact
    // source vectors and every original declaration remain prerequisites.
    let mut registry = tcl_registry::CommandRegistry::build_default();
    let mut selected = registry.get("set").unwrap().clone();
    selected.name = "metadata_store";
    selected.surface = registry.get("dict").unwrap().surface;
    registry.insert(selected);
    let context = Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.6")
            .with_command_store(registry.snapshot().shared_registry()),
    );
    let older = Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(registry.snapshot().shared_registry()),
    );
    let source = "proc f {} {metadata_store p VALUE; open $p}";
    let (_, tokens, input) =
        complete_logical_declaration_tokens(source, "metadata_store p VALUE", &context);
    let binding = tokens.source_binding.as_ref().unwrap();
    let metadata = binding
        .original_invocation_metadata_for_input(&tokens, &input, &registry)
        .unwrap();
    assert!(
        crate::registry_invocation::original_logical_operation_invocation_with_metadata_context(
            &registry, metadata, &tokens,
        )
        .is_some()
    );
    let (_, older_tokens, older_input) =
        complete_logical_declaration_tokens(source, "metadata_store p VALUE", &older);
    let older_binding = older_tokens.source_binding.as_ref().unwrap();
    assert_original_declaration_metadata_refused(binding, &tokens, &older_input, &registry);
    if let Some(older_metadata) =
        older_binding.original_invocation_metadata_for_input(&older_tokens, &older_input, &registry)
    {
        assert!(crate::registry_invocation::original_logical_operation_invocation_with_metadata_context(
                &registry, older_metadata, &older_tokens,
            ).is_none());
    }
    let mut foreign_registry = tcl_registry::CommandRegistry::build_default();
    foreign_registry.insert(tcl_registry::CommandSpec {
        name: "foreign_declaration_axis",
        ..registry.get("set").unwrap().clone()
    });
    let foreign = Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.6")
            .with_command_store(foreign_registry.snapshot().shared_registry()),
    );
    let changed_input = crate::analyser::ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        Arc::clone(&context),
        tcl_lexer::LexerConfig {
            strict_quoting: !input.lexer_config().strict_quoting,
            ..input.lexer_config()
        },
    );
    let foreign_input = crate::analyser::ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        foreign,
        input.lexer_config(),
    );
    for owner in [
        crate::registry_invocation::OwnedInvocationMetadataContext::Unavailable,
        crate::registry_invocation::OwnedInvocationMetadataContext::Supplied(Arc::clone(&context)),
        crate::registry_invocation::OwnedInvocationMetadataContext::for_source_input(Some(
            &changed_input,
        )),
        crate::registry_invocation::OwnedInvocationMetadataContext::for_source_input(Some(
            &foreign_input,
        )),
    ] {
        let mut changed = tokens.clone();
        let binding = changed.source_binding.as_mut().unwrap();
        for row in Arc::make_mut(binding.declaration_layout_observations.as_mut().unwrap()) {
            let state = &mut Arc::make_mut(&mut row.snapshot).state;
            Arc::make_mut(&mut state.baseline).metadata_context = owner.clone();
        }
        let binding = changed.source_binding.as_ref().unwrap();
        assert_original_declaration_metadata_refused(binding, &changed, &input, &registry);
    }
    let mut missing = tokens.clone();
    missing
        .source_binding
        .as_mut()
        .unwrap()
        .declaration_layout_observations = None;
    assert_original_declaration_metadata_refused(
        missing.source_binding.as_ref().unwrap(),
        &missing,
        &input,
        &registry,
    );
    let mut changed_vector = tokens.clone();
    changed_vector.word_exprs.pop();
    assert_original_declaration_metadata_refused(binding, &changed_vector, &input, &registry);
    assert_shadowed_declaration_cannot_supply_logical_operation(&context, &registry);
}

#[test]
fn original_declaration_metadata_cannot_repair_invalid_reached_or_native_owners() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Retained source advice cannot replace a present invalid reached
    // owner, nor issue Native lookup or preparation for an unentered body.
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let (_, tokens, input) =
        complete_logical_declaration_tokens("set p VALUE", "set p VALUE", &context);
    assert!(
        tokens
            .source_binding
            .as_ref()
            .unwrap()
            .lookup_state
            .is_some()
    );
    let mut withdrawn = tokens.clone();
    let binding = withdrawn.source_binding.as_mut().unwrap();
    let state = &mut Arc::make_mut(binding.lookup_state.as_mut().unwrap()).state;
    Arc::make_mut(&mut state.baseline).metadata_context =
        crate::registry_invocation::OwnedInvocationMetadataContext::Unavailable;
    assert!(binding.declaration_layout_observations.is_some());
    let binding = withdrawn.source_binding.as_ref().unwrap();
    assert!(
        binding
            .original_source_metadata_for_advice(&withdrawn, context.commands())
            .is_err()
    );
    assert!(
        binding
            .original_invocation_metadata_for_input(&withdrawn, &input, context.commands())
            .is_none()
    );
    let metadata = crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
        context.commands(),
        &input,
    )
    .unwrap();
    assert!(
        crate::registry_invocation::original_logical_operation_invocation_with_metadata_context(
            context.commands(),
            metadata,
            &withdrawn,
        )
        .is_none()
    );
    let source = "proc f {} {set p VALUE}";
    let (_, unentered, _) = complete_logical_declaration_tokens(source, "set p VALUE", &context);
    let native_profile = tcl_registry::model::ingress::resolve_environment("tcl8.6").unit_profile();
    let native_input = crate::analyser::ResolvedAnalysisInput::new(
        native_profile,
        native_profile,
        Arc::clone(&context),
        tcl_lexer::LexerConfig::for_file_grammar(native_profile.grammar),
    );
    assert!(!native_input.has_logical_source_name_context());
    let binding = unentered.source_binding.as_ref().unwrap();
    assert!(binding.lookup_state.is_none());
    assert!(
        binding
            .original_invocation_metadata_for_input(&unentered, &native_input, context.commands())
            .is_none()
    );
    assert!(binding.proved_execution_target().is_none());
    assert!(binding.native_structured_preparation.is_none());
}

fn inventory(source: &str, command: &str) -> (SourceCommandBindings, crate::ir::CommandTokens) {
    inventory_in_world(source, command, false)
}

fn inventory_in_world(
    source: &str,
    command: &str,
    unknown_entry: bool,
) -> (SourceCommandBindings, crate::ir::CommandTokens) {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let dialect = tcl_registry::InvocationDialect::of_point(
        tcl_dialect::model::DialectPoint::of_dialect_name(Some("tcl8.6")).unwrap(),
    );
    let config = tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar);
    let bindings = SourceCommandBindings::analyse_with_options(
        source,
        config,
        registry,
        super::super::SourceAnalysisOptions {
            invocation_dialect: Some(dialect),
            unknown_entry,
            native_compilation: crate::environment_ingress::authoring_native_compilation(),
            ..Default::default()
        },
    );
    let offset = u32::try_from(source.find(command).unwrap()).unwrap();
    let segment =
        tcl_source::segment::segment_commands_with_offset_and_config(command, offset, config)
            .remove(0);
    let mut tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::new(source),
        config,
        &segment,
    );
    bindings.stamp_original_tokens(&mut tokens);
    (bindings, tokens)
}

#[test]
fn original_registry_advice_uses_uncalled_body_layout_without_normal_receipt() {
    // Implementation contract: naming.source.original-registry-header-advice
    // docs/design/analysis/name-resolution-proofs/original-registry-header-advice.md
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    for (source, command, expected) in [
        (
            "proc p {arg} {set evenWhenWritingCode}",
            "set evenWhenWritingCode",
            "set",
        ),
        ("proc p {} \"puts hi\"", "puts hi", "puts"),
    ] {
        let (_, tokens) = inventory(source, command);
        let binding = tokens.source_binding.as_ref().unwrap();
        assert!(
            binding.original_normal_result(&tokens).is_none(),
            "{source}"
        );
        assert!(
            !binding.original_invocation_completes_normally(&tokens),
            "{source}"
        );
        let transfer_available =
            crate::registry_invocation::normal_transfer_invocation(registry, None, &tokens)
                .is_some();
        let advice = crate::registry_invocation::original_registry_invocation_assistance(
            registry, None, &tokens,
        )
        .expect("the original body layout supplies conditional metadata");
        assert_eq!(
            advice.unanimous_command_words().unwrap().command(),
            expected
        );
        assert_eq!(
            crate::registry_invocation::normal_transfer_invocation(registry, None, &tokens,)
                .is_some(),
            transfer_available,
            "{source}"
        );
        assert!(
            binding.original_normal_result(&tokens).is_none(),
            "{source}"
        );
        assert!(
            !binding.original_invocation_completes_normally(&tokens),
            "{source}"
        );
        let mut synthetic = tokens.clone();
        synthetic.source_binding = None;
        assert!(
            crate::registry_invocation::original_registry_invocation_assistance(
                registry, None, &synthetic,
            )
            .is_none()
        );
    }
}

#[test]
fn original_registry_advice_retains_body_shadowing_and_original_word_currency() {
    // Implementation contract: naming.source.original-registry-header-advice
    // docs/design/analysis/name-resolution-proofs/original-registry-header-advice.md
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let (_, tokens) = inventory(
        "proc set args {return CUSTOM}; proc p {} {set value}",
        "set value",
    );
    assert!(
        crate::registry_invocation::original_registry_invocation_assistance(
            registry, None, &tokens,
        )
        .is_none_or(|advice| advice.unanimous_command_words().is_none())
    );
    let (_, mut changed) = inventory("proc p {} {puts hi}", "puts hi");
    changed.argv_texts[0] = "reporting-counterfactual".to_owned();
    let readonly = crate::registry_invocation::original_registry_invocation_assistance(
        registry, None, &changed,
    )
    .unwrap();
    assert_eq!(
        readonly.unanimous_command_words().unwrap().command(),
        "puts"
    );
    let tcl_source::words::WordExpr::Literal { text, .. } = &mut changed.word_exprs[0] else {
        panic!("a bare literal original head");
    };
    *text = "set".to_owned();
    assert!(
        crate::registry_invocation::original_registry_invocation_assistance(
            registry, None, &changed,
        )
        .is_none_or(|advice| advice.unanimous_command_words().is_none())
    );
}

#[test]
fn original_declaration_procedure_name_keeps_native_operand_identity() {
    // Implementation contract: naming.source.original-declaration-name-publication (docs/design/analysis/name-resolution-proofs/original-declaration-name-publication.md).
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let owner = tcl_registry::model::ingress::static_context_for(engine);
        let registry = owner.commands();
        let config = tcl_lexer::LexerConfig::from_grammar(registry.profile().unwrap().grammar);
        let source = r"proc p\uD800 {value} {set local $value}";
        let bindings = SourceCommandBindings::analyse(source, config, registry);
        let site = CommandAllocationSite {
            source: Arc::clone(bindings.source_origin().unwrap()),
            offset: 0,
        };
        let observations =
            original_declaration_layouts(bindings.declaration_layouts.get(&site).unwrap()).unwrap();
        let observation = observations.clone().next().unwrap();
        let tokens = declaration_tokens(&site, observation).unwrap();
        let recipe = declared_body(&site, &tokens, observation, registry)
            .expect("original native name is not a reporting string");
        let native = crate::registry_invocation::original_native_compiler_words(
            site.source.source_image(),
            tokens.words(),
            site.offset,
            config,
        )
        .unwrap();
        let key = crate::signature_scan::scope::SignatureSourceNameKey::from_original_native_word(
            &native[1],
            tcl_syntax::word_rules::WordValueRules::from_config(&config),
            observation
                .snapshot
                .state
                .source_variables
                .execution_name_policy
                .unwrap()
                .native_recipe()
                .unwrap(),
        )
        .unwrap();
        let input = crate::signature_scan::scope::SignatureSourceNameInput::OriginalWord(key);
        assert_eq!(
            recipe.slot,
            observation
                .snapshot
                .state
                .original_publication_key(
                    &observation.namespace,
                    &input,
                    super::super::namespace_slots::PublicationPurpose::Procedure
                )
                .unwrap()
        );
        assert_eq!(recipe.source.origin, site.source);
        assert!(recipe.original_parameters.is_some());
        let dynamic = r"proc $unknown {value} {set local $value}";
        let bindings = SourceCommandBindings::analyse(dynamic, config, registry);
        let site = CommandAllocationSite {
            source: Arc::clone(bindings.source_origin().unwrap()),
            offset: 0,
        };
        if let Some(observation) = bindings
            .declaration_layouts
            .get(&site)
            .and_then(|rows| original_declaration_layouts(rows))
            .and_then(|mut rows| rows.next())
        {
            let tokens = declaration_tokens(&site, observation).unwrap();
            assert!(declared_body(&site, &tokens, observation, registry).is_none());
        }
    }
}

#[test]
fn conditional_authored_declaration_geometry_refuses_withdrawn_rows_and_invalid_points() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let module = crate::lowering::lower_to_ir(
        "rename $old decl\nproc kept {arg} {list retained}",
        registry,
    );
    let header = module
        .top_level
        .retained_source_tokens_for_statement(module.top_level.statements.last().unwrap())
        .expect("actual original unentered header");
    let binding = header.source_binding.as_ref().unwrap();
    assert!(
        binding
            .original_authored_procedure_source_geometry(header, registry)
            .is_some()
    );
    let mut withdrawn = header.clone();
    withdrawn
        .source_binding
        .as_mut()
        .unwrap()
        .declaration_layout_observations = None;
    assert!(
        withdrawn
            .source_binding
            .as_ref()
            .unwrap()
            .original_authored_procedure_source_geometry(&withdrawn, registry)
            .is_none()
    );
    let row =
        original_declaration_layouts(binding.declaration_layout_observations.as_deref().unwrap())
            .unwrap()
            .next()
            .unwrap();
    let mut state = row.snapshot.state.clone();
    Arc::make_mut(&mut state.baseline).metadata_context =
        crate::registry_invocation::OwnedInvocationMetadataContext::Unavailable;
    let mut invalid_point = header.clone();
    invalid_point.source_binding.as_mut().unwrap().lookup_state = Some(Arc::new(
        SourceLookupSnapshot::in_realm(state, row.snapshot.realm),
    ));
    assert!(
        invalid_point
            .source_binding
            .as_ref()
            .unwrap()
            .original_authored_procedure_source_geometry(&invalid_point, registry)
            .is_none()
    );
}

#[test]
fn conditional_declaration_geometry_survives_unknown_table_without_installation() {
    // Implementation contract: naming.source.original-declaration-name-publication
    // docs/design/analysis/name-resolution-proofs/original-declaration-name-publication.md
    let source = "rename $old decl\nproc wrap {arg} {upvar 1 other local; return $local}";
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let (bindings, tokens) = inventory_in_world(
        source,
        "proc wrap {arg} {upvar 1 other local; return $local}",
        true,
    );
    let site = tokens
        .source_binding
        .as_ref()
        .unwrap()
        .invocation_site()
        .unwrap();
    let observation = original_declaration_layouts(bindings.declaration_layouts.get(site).unwrap())
        .unwrap()
        .next()
        .unwrap();
    assert!(observation.snapshot.state.has_opaque_domain());
    let recipe = declared_body(site, &tokens, observation, registry)
        .expect("genuine declaration geometry is independent of current table existence");
    assert_eq!(recipe.namespace.display().as_deref(), Some("::"));
    assert_eq!(
        recipe
            .original_parameters
            .as_ref()
            .unwrap()
            .advisory_parameters()[0]
            .name,
        "arg"
    );
    assert_eq!(
        observation
            .snapshot
            .state
            .installed_procedure_targets(&recipe.slot, site.offset),
        [] as [crate::command_binding::ResolvedCommandTarget; 0]
    );
    assert!(!bindings.has_actual_procedure_entry(&recipe.source));
    assert!(
        tokens
            .source_binding
            .as_ref()
            .unwrap()
            .proved_execution_target()
            .is_none()
    );
    let mut foreign = observation.clone();
    foreign.snapshot = Arc::new(SourceLookupSnapshot::new(
        super::super::ModuleCommandBindings::initial(registry),
    ));
    assert!(declared_body(site, &tokens, &foreign, registry).is_none());
}

#[test]
fn nested_layout_visits_process_each_exact_source_world_once() {
    // Implementation contract: naming.source.original-declaration-layout-visits
    // docs/design/analysis/name-resolution-proofs/source-original-declaration-layout-visits.md
    // Derive conditional source layouts from one genuine outer observation.
    // All descendants keep that unchanged lookup world; reached condition
    // coercions and their independent snapshots are not substituted here.
    let source = "if {1} {if {1} {if {1} {if {1} {if {1} {if {1} {if {1} {puts done}}}}}}}";
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let (mut cached, outer) = inventory(source, source);
    let outer_site = outer
        .source_binding
        .as_ref()
        .unwrap()
        .invocation_site()
        .unwrap()
        .clone();
    let original =
        original_declaration_layouts(cached.declaration_layouts.get(&outer_site).unwrap())
            .unwrap()
            .next()
            .unwrap()
            .clone();
    cached
        .declaration_layouts
        .retain(|site, _| site == &outer_site);
    assert_eq!(cached.declaration_layouts.len(), 1);
    assert_eq!(cached.retain_nested_declared_layouts(registry), 7);
    assert!(cached.declaration_layouts.values().all(|rows| {
        original_declaration_layouts(rows).unwrap().all(|row| {
            row.snapshot == original.snapshot
                && row.entry == original.entry
                && row.namespace == original.namespace
                && row.config == original.config
        })
    }));
    let mut uncached = cached.clone();
    let cached_visits = cached.retain_nested_declared_layouts(registry);
    let uncached_visits = uncached.retain_nested_declared_layouts_with(registry, |_, _| true);
    assert_eq!(cached_visits, 7);
    assert_eq!(uncached_visits, 28);
    assert_eq!(cached, uncached);
}

#[test]
fn nested_layout_visits_keep_reached_representation_worlds_distinct() {
    // Implementation contract: naming.source.original-declaration-layout-visits
    // docs/design/analysis/name-resolution-proofs/source-original-declaration-layout-visits.md
    // The same written command table does not make reached variable and
    // literal representation worlds interchangeable. This software source
    // control supplies no Native object, body-entry or elapsed-time claim.
    let source = "if {1} {if {1} {if {1} {if {1} {if {1} {if {1} {if {1} {puts done}}}}}}}";
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let (mut cached, _) = inventory(source, "puts done");
    let worlds = cached
        .declaration_layouts
        .values()
        .flat_map(|rows| original_declaration_layouts(rows).unwrap())
        .collect::<Vec<_>>();
    let first = worlds.first().unwrap();
    assert!(worlds.iter().all(|row| {
        row.snapshot.state.original_command_world == first.snapshot.state.original_command_world
            && row.snapshot.state.source_variables.original_contents_epoch
                == first
                    .snapshot
                    .state
                    .source_variables
                    .original_contents_epoch
    }));
    let mut distinct = Vec::new();
    for row in &worlds {
        if !distinct.contains(&&row.snapshot) {
            distinct.push(&row.snapshot);
        }
    }
    assert_eq!(distinct.len(), 8);
    for (index, snapshot) in distinct.iter().enumerate() {
        for other in &distinct[index + 1..] {
            assert_ne!(
                snapshot.state.source_variables.representation_epoch,
                other.state.source_variables.representation_epoch,
            );
            assert_ne!(
                snapshot.state.source_variables,
                other.state.source_variables
            );
            assert_ne!(
                snapshot.state.ordinary_literal_pool,
                other.state.ordinary_literal_pool,
            );
        }
    }
    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_SOURCE_PHASES").is_some() {
        let mut first = None;
        for (site, rows) in &cached.declaration_layouts {
            let Some(originals) = original_declaration_layouts(rows) else {
                continue;
            };
            for original in originals {
                let state = &original.snapshot.state;
                let first = first.get_or_insert(state);
                eprintln!(
                    "NESTED_LAYOUT_ORIGINAL_WORLD offset={} rows={} same_state={} same_variables={} same_literal_pool={} same_command_world={} epoch={:?} epoch_high={:?} contents_epoch={:?} contents_high={:?}",
                    site.offset,
                    rows.len(),
                    *first == state,
                    first.source_variables == state.source_variables,
                    first.ordinary_literal_pool == state.ordinary_literal_pool,
                    first.original_command_world == state.original_command_world,
                    state.source_variables.representation_epoch,
                    state.source_variables.representation_epoch_high_water,
                    state.source_variables.original_contents_epoch,
                    state.source_variables.original_contents_epoch_high_water,
                );
            }
        }
    }
    let mut uncached = cached.clone();
    let cached_visits = cached.retain_nested_declared_layouts(registry);
    let uncached_visits = uncached.retain_nested_declared_layouts_with(registry, |_, _| true);
    assert_eq!(cached_visits, 28);
    assert_eq!(uncached_visits, 28);
    assert_eq!(cached, uncached);
}

#[test]
fn exact_parent_layout_reuse_keeps_guarded_children_and_successor_decisions() {
    // Implementation contract: naming.source.original-declaration-layout-visits
    // docs/design/analysis/name-resolution-proofs/source-original-declaration-layout-visits.md
    // Genuine original observations compare one shared projection with
    // independently rederived children and continuation. No elapsed-time,
    // execution, installed implementation or Native frame claim follows.
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    for (source, command, child_count, successor) in [
        ("if {1} {puts done}", "if {1} {puts done}", 1, false),
        ("list done; list later", "list done", 0, true),
        (
            "set local $incoming; puts later",
            "set local $incoming",
            0,
            false,
        ),
        (
            "rename puts saved; puts later",
            "rename puts saved",
            0,
            false,
        ),
        (
            "if {[rename puts saved; expr 1]} {puts done}",
            "if {[rename puts saved; expr 1]} {puts done}",
            0,
            false,
        ),
        (
            "proc wrap {condition} {if {$condition} {puts done}}",
            "if {$condition} {puts done}",
            1,
            false,
        ),
    ] {
        let (bindings, tokens) = inventory(source, command);
        let site = tokens
            .source_binding
            .as_ref()
            .unwrap()
            .invocation_site()
            .unwrap();
        let parents: Vec<_> =
            original_declaration_layouts(bindings.declaration_layouts.get(site).unwrap())
                .unwrap()
                .cloned()
                .collect();
        assert!(!parents.is_empty(), "{source}");
        let originals = parent_declaration_layouts(site, &tokens, &parents, registry);
        let children = unanimous_nested_layouts_from_parents(
            site,
            &originals,
            registry,
            &super::nested_layouts::ScopeAliasOwnershipShare::default(),
        );
        assert_eq!(
            children,
            unanimous_nested_layouts(
                site,
                &tokens,
                &parents,
                registry,
                &super::nested_layouts::ScopeAliasOwnershipShare::default()
            ),
            "{source}"
        );
        assert_eq!(children.len(), child_count, "{source}");
        let shared = originals.iter().all(|row| {
            row.original
                .as_ref()
                .is_some_and(OriginalDeclarationLayout::retains_successor)
        });
        let repeated = parents.iter().all(|parent| {
            retains_successor_layout(
                site,
                &tokens,
                &parent.snapshot,
                &parent.namespace,
                parent.config,
                registry,
            )
        });
        assert_eq!(shared, repeated, "{source}");
        assert_eq!(shared, successor, "{source}");
    }
}

#[test]
fn exact_parent_layout_reuse_refuses_lost_original_source_or_frame() {
    // Implementation contract: naming.source.original-declaration-layout-visits
    // docs/design/analysis/name-resolution-proofs/source-original-declaration-layout-visits.md
    // These changed-premise API controls cannot manufacture a provider
    // execution result from an original layout's reporting fields.
    let source = "if {1} {puts done}";
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let (bindings, tokens) = inventory(source, source);
    let site = tokens
        .source_binding
        .as_ref()
        .unwrap()
        .invocation_site()
        .unwrap();
    let parents: Vec<_> =
        original_declaration_layouts(bindings.declaration_layouts.get(site).unwrap())
            .unwrap()
            .cloned()
            .collect();
    let original = &parents[0].snapshot.state;
    let mut lost_source = original.clone();
    lost_source.current_source_origin = None;
    let mut lost_frame = original.clone();
    lost_frame.variable_frame = crate::var_resolve::VariableExecutionFrame::Unknown;
    let mut lost_dialect = original.clone();
    Arc::make_mut(&mut lost_dialect.source_variables).invocation_dialect = None;
    for state in [lost_source, lost_frame, lost_dialect] {
        let mut changed = parents.clone();
        changed[0].snapshot = Arc::new(SourceLookupSnapshot::in_realm(
            state,
            changed[0].snapshot.realm,
        ));
        let rows = parent_declaration_layouts(site, &tokens, &changed, registry);
        assert!(rows[0].original.is_none());
        assert_eq!(
            unanimous_nested_layouts_from_parents(
                site,
                &rows,
                registry,
                &super::nested_layouts::ScopeAliasOwnershipShare::default()
            ),
            [] as [(
                crate::command_binding::executed_script_source::ExecutedScriptSource,
                std::vec::Vec<
                    crate::command_binding::declaration_layout::DeclarationLayoutObservation,
                >
            ); 0]
        );
        assert!(!rows.iter().all(|row| {
            row.original
                .as_ref()
                .is_some_and(OriginalDeclarationLayout::retains_successor)
        }));
    }
}

fn nested_layout_key_fixture() -> (ExecutedScriptSource, Vec<DeclarationLayoutObservation>) {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let (bindings, _) = inventory("if {1} {puts done}", "puts done");
    let (site, rows) = bindings.declaration_layouts.iter().next().unwrap();
    let parents: Vec<_> = original_declaration_layouts(rows)
        .unwrap()
        .cloned()
        .collect();
    let tokens = declaration_tokens(site, &parents[0]).unwrap();
    unanimous_nested_layouts(
        site,
        &tokens,
        &parents,
        registry,
        &super::nested_layouts::ScopeAliasOwnershipShare::default(),
    )
    .remove(0)
}

#[test]
fn nested_layout_syntax_shares_only_exact_original_geometry() {
    // naming.source.original-declaration-layout-visits
    // docs/design/analysis/name-resolution-proofs/source-original-declaration-layout-visits.md
    // Pure lexical sharing, independently of any Native lookup/frame/header claim.
    let (source, parents) = nested_layout_key_fixture();
    let config = parents[0].config;
    let mut syntax = NestedLayoutSyntax::default();
    let first = syntax.commands(&source, config).unwrap();
    let second = syntax.commands(&source, config).unwrap();
    assert!(Arc::ptr_eq(&first, &second));
    assert_eq!(syntax.parsed_scripts, 1);
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].site.source, source.origin);
    assert_eq!(first[0].site.offset, source.base());
    assert_eq!(first[0].tokens.argv_texts, ["puts", "done"]);
    assert!(first[0].tokens.source_binding.is_none());
    assert_eq!(first[0].tokens.variable_accesses.len(), 0);
    assert_eq!(
        *first[0].tokens.nested_bindings,
        [] as [(u32, crate::command_binding::SourceInvocationBinding); 0]
    );

    let mut changed_config = config;
    changed_config.strict_quoting = !config.strict_quoting;
    let changed = syntax.commands(&source, changed_config).unwrap();
    assert!(!Arc::ptr_eq(&first, &changed));
    let materialised = ExecutedScriptSource::materialised(
        CommandAllocationSite {
            source: Arc::clone(&source.origin),
            offset: source.base(),
        },
        vec![0],
        source.try_text().unwrap(),
    );
    let changed = syntax.commands(&materialised, config).unwrap();
    assert!(!Arc::ptr_eq(&first, &changed));
    assert_eq!(changed[0].site.source, materialised.origin);
    assert_eq!(changed[0].site.offset, 0);
    assert_eq!(changed[0].tokens.argv[0].start(), 0);
    let mut changed_channel = source.clone();
    changed_channel.text = tcl_lexer::SourceImage::document(source.try_text().unwrap());
    let changed = syntax.commands(&changed_channel, config).unwrap();
    assert!(!Arc::ptr_eq(&first, &changed));
    assert_eq!(syntax.parsed_scripts, 4);
}

#[test]
fn nested_layout_syntax_reuse_keeps_original_owner_refusal() {
    // naming.source.original-declaration-layout-visits
    // docs/design/analysis/name-resolution-proofs/source-original-declaration-layout-visits.md
    // A retained pure tape cannot fill a missing source or frame owner.
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let (source, parents) = nested_layout_key_fixture();
    let mut syntax = NestedLayoutSyntax::default();
    let tape = syntax.commands(&source, parents[0].config).unwrap();
    let command = &tape[0];
    let rows = parent_declaration_layouts(&command.site, &command.tokens, &parents, registry);
    assert!(rows[0].original.is_some());
    let original = &parents[0].snapshot.state;
    let mut lost_source = original.clone();
    lost_source.current_source_origin = None;
    let mut lost_frame = original.clone();
    lost_frame.variable_frame = crate::var_resolve::VariableExecutionFrame::Unknown;
    for state in [lost_source, lost_frame] {
        let mut changed = parents.clone();
        changed[0].snapshot = Arc::new(SourceLookupSnapshot::in_realm(
            state,
            changed[0].snapshot.realm,
        ));
        let rows = parent_declaration_layouts(&command.site, &command.tokens, &changed, registry);
        assert!(rows[0].original.is_none());
    }
    assert_eq!(syntax.parsed_scripts, 1);
}

#[test]
fn nested_layout_syntax_memo_matches_original_parser_world_inventory() {
    // naming.source.original-declaration-layout-visits
    // docs/design/analysis/name-resolution-proofs/source-original-declaration-layout-visits.md
    // Compare complete retained layouts; neither path claims entered execution.
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    for source in [
        "if {1} {if {1} {if {1} {puts done}}}",
        "proc p {condition} {if {$condition} {if {1} {puts done}}}",
        "proc p {condition} {if {[rename puts saved; expr 1]} {puts done}}",
    ] {
        let (mut cached, _) = inventory(source, "puts done");
        let mut original = cached.clone();
        let cached_visits = cached.retain_nested_declared_layouts(registry);
        let mut visited = NestedDeclarationLayoutVisits::default();
        let original_visits = original.retain_nested_declared_layouts_using(
            registry,
            |source, parents| visited.insert(NestedDeclarationLayoutKey::new(source, parents)),
            NestedLayoutSyntax::parse,
        );
        assert_eq!(cached_visits, original_visits, "{source}");
        assert_eq!(cached, original, "{source}");
    }
}

#[test]
fn nested_layout_visit_key_keeps_complete_lookup_and_owner_facets() {
    // Implementation contract: naming.source.original-declaration-layout-visits
    // docs/design/analysis/name-resolution-proofs/source-original-declaration-layout-visits.md
    // Changing a semantic facet is a distinct source-contract question;
    // these key controls do not manufacture an entered Native frame.
    let (source, parents) = nested_layout_key_fixture();
    let mut visited = NestedDeclarationLayoutVisits::default();
    assert!(visited.insert(NestedDeclarationLayoutKey::new(&source, &parents)));
    let mut changed = parents.clone();
    changed[0].words = Arc::from([]);
    assert!(!visited.insert(NestedDeclarationLayoutKey::new(&source, &changed)));
    let mut changed = parents.clone();
    changed[0].issuer = DeclarationLayoutIssuer::EnteredActivation;
    assert!(visited.insert(NestedDeclarationLayoutKey::new(&source, &changed)));
    let mut changed = parents.clone();
    changed[0].namespace = super::super::SourceNamespaceKey::authored("::other");
    assert!(visited.insert(NestedDeclarationLayoutKey::new(&source, &changed)));
    let mut changed = parents.clone();
    changed[0].config.expand_syntax = !changed[0].config.expand_syntax;
    assert!(visited.insert(NestedDeclarationLayoutKey::new(&source, &changed)));
    let mut changed = parents.clone();
    changed[0].snapshot = Arc::new(SourceLookupSnapshot::in_realm(
        changed[0].snapshot.state.clone(),
        tcl_dialect::model::InvocationRealm::InterpreterRuntime,
    ));
    assert!(visited.insert(NestedDeclarationLayoutKey::new(&source, &changed)));
    let mut changed = parents.clone();
    let mut state = changed[0].snapshot.state.clone();
    state.mark_opaque_binding_mutation();
    changed[0].snapshot = Arc::new(SourceLookupSnapshot::in_realm(
        state,
        changed[0].snapshot.realm,
    ));
    assert!(visited.insert(NestedDeclarationLayoutKey::new(&source, &changed)));
    let mut changed = parents.clone();
    changed[0].entry = Arc::new(OriginalDiagnosticFrameEntry::ScriptActivation {
        source: changed[0].entry.source().clone(),
        frame: changed[0].entry.frame().clone(),
        namespace: changed[0].namespace.clone(),
        config: changed[0].config,
    });
    assert!(visited.insert(NestedDeclarationLayoutKey::new(&source, &changed)));
    let materialised = ExecutedScriptSource::materialised(
        CommandAllocationSite {
            source: Arc::clone(&source.origin),
            offset: source.base(),
        },
        vec![0],
        source.try_text().unwrap(),
    );
    assert!(visited.insert(NestedDeclarationLayoutKey::new(&materialised, &parents)));
    assert_eq!(visited.len(), 8);
}

#[test]
fn nested_layout_visit_bucket_collision_keeps_distinct_source_worlds() {
    let (source, parents) = nested_layout_key_fixture();
    let mut changed = parents.clone();
    changed[0].namespace = super::super::SourceNamespaceKey::authored("::other");
    let mut visits = NestedDeclarationLayoutVisits::default();
    // A shared bucket cannot collapse two authentic parent namespaces.
    assert!(visits.insert_in_bucket(0, NestedDeclarationLayoutKey::new(&source, &parents)));
    assert!(visits.insert_in_bucket(0, NestedDeclarationLayoutKey::new(&source, &changed)));
    assert!(!visits.insert_in_bucket(0, NestedDeclarationLayoutKey::new(&source, &parents)));
    assert_eq!(visits.len(), 2);
}

#[test]
fn nested_declared_layout_keeps_original_scope_without_entered_authority() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Alias exclusion requires complete original body ownership, not an
    // installed alias, entered variable frame or successful read.
    // Implementation binding: naming.source.original-registry-header-advice
    // docs/design/analysis/name-resolution-proofs/original-registry-header-advice.md
    // The missing original variable stops argv before the rename and proc.
    // Their unentered suffix keeps the quiet original lookup world, while
    // its conditional declaration supplies no installed/entered frame.
    let source =
        "rename $old decl\nproc wrap {condition} {if {$condition} {info exists condition}}";
    let (bindings, tokens) = inventory(source, "info exists condition");
    let binding = tokens.source_binding.as_ref().unwrap();
    let layout = binding
        .declaration_operand_layout_advice(&tokens)
        .expect("unchanged scalar condition retains original nested grammar");
    assert!(layout.closed_lookup());
    assert!(binding.unknown);
    assert!(binding.proved_execution_target().is_none());
    assert!(binding.compiler_lookup_state.is_none());
    let site = binding.invocation_site().unwrap();
    assert!(
        bindings
            .conditional_body_entry_at(&site.source, site.offset)
            .is_none()
    );
    assert!(!bindings.has_actual_procedure_entry_at(&site.source, site.offset));
    let originals =
        original_declaration_layouts(binding.declaration_layout_observations.as_deref().unwrap())
            .unwrap();
    assert!(originals.clone().all(|observation| {
        !observation.snapshot.state.source_variables.dynamic_traces
            && observation.snapshot.state.baseline.native_entry.is_none()
            && matches!(
                observation.entry.as_ref(),
                OriginalDiagnosticFrameEntry::DeclaredProcedure(_)
            )
            && observation
                .entry
                .parameters()
                .iter()
                .map(|parameter| parameter.name.as_str())
                .eq(["condition"])
            && observation.entry.owns_source(&site.source, site.offset)
            && observation
                .entry
                .owns_original_context(&observation.snapshot.state.source_variables)
    }));
}

#[test]
fn nested_declared_layout_declines_lookup_after_unknown_variable_observers() {
    // Implementation binding: naming.source.original-registry-header-advice
    // docs/design/analysis/name-resolution-proofs/original-registry-header-advice.md
    // SourceAnalysisOptions::unknown_entry widens the original variable
    // world through ModuleCommandBindings::mark_opaque_binding_mutation
    // and ResolveContext::widen. The genuine root condition remains a
    // source layout; its unknown read observers cannot donate child lookup.
    let source = "if {$condition} {info exists condition}";
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let (bindings, condition) = inventory_in_world(source, source, true);
    let binding = condition.source_binding.as_ref().unwrap();
    let site = binding.invocation_site().unwrap();
    let parents = original_declaration_layouts(bindings.declaration_layouts.get(site).unwrap())
        .unwrap()
        .cloned()
        .collect::<Vec<_>>();
    assert_ne!(
        parents,
        [] as [crate::command_binding::declaration_layout::DeclarationLayoutObservation; 0]
    );
    assert!(parents.iter().all(|parent| {
        parent.snapshot.state.source_variables.dynamic_traces
            && parent.snapshot.state.baseline.native_entry.is_none()
            && parent.entry.owns_source(&site.source, site.offset)
    }));
    let originals = parent_declaration_layouts(site, &condition, &parents, registry);
    assert!(originals.iter().all(|parent| parent.original.is_some()));
    assert_eq!(
        unanimous_nested_layouts_from_parents(
            site,
            &originals,
            registry,
            &super::nested_layouts::ScopeAliasOwnershipShare::default()
        ),
        [] as [(
            crate::command_binding::executed_script_source::ExecutedScriptSource,
            std::vec::Vec<crate::command_binding::declaration_layout::DeclarationLayoutObservation>
        ); 0]
    );
    assert!(binding.proved_execution_target().is_none());
    assert!(binding.compiler_lookup_state.is_none());
    let (_, nested) = inventory_in_world(source, "info exists condition", true);
    assert!(
        nested
            .source_binding
            .as_ref()
            .unwrap()
            .declaration_operand_layout_advice(&nested)
            .is_none()
    );
}

#[test]
fn nested_declared_layout_stops_before_condition_callbacks_or_aliases() {
    for body in [
        "if {[rename info saved; expr 1]} {info exists condition}",
        "if {$array(index)} {info exists condition}",
        "if {[expr {1}]} {info exists condition}",
        "upvar 1 outer condition; if {$condition} {info exists condition}",
        "trace add variable condition read replaceInfo; if {$condition} {info exists condition}",
    ] {
        let source = format!("proc wrap {{condition}} {{{body}}}");
        let (_, tokens) = inventory_in_world(&source, "info exists condition", true);
        assert!(
            tokens
                .source_binding
                .as_ref()
                .unwrap()
                .declaration_operand_layout_advice(&tokens)
                .is_none(),
            "condition lookup is unavailable after: {body}"
        );
    }
}

#[test]
fn uninstalled_body_layout_retains_original_candidate_without_runtime_authority() {
    let source = "rename $old decl\nproc wrap {arg} {upvar 1 other local; return $local}";
    let (bindings, tokens) = inventory_in_world(source, "upvar 1 other local", true);
    let binding = tokens.source_binding.as_ref().unwrap();
    let layout = binding
        .declaration_operand_layout_advice(&tokens)
        .expect("conditional original grammar");
    assert!(!layout.closed_lookup());
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let initial = super::super::ModuleCommandBindings::initial(registry);
    let original = super::super::source_binding(
        &initial,
        "upvar",
        &super::super::SourceNamespaceKey::authored("::"),
    );
    assert!(layout.targets().contains(original.proved_target().unwrap()));
    assert!(binding.unknown);
    assert!(binding.proved_execution_target().is_none());
    assert!(binding.compiler_lookup_state.is_none());
    let (_, original_words, original_offset) = binding
        .original_compiler_source(&tokens)
        .expect("unchanged lexical vector is geometry, not compilation admission");
    assert_eq!(original_words, tokens.words());
    assert_eq!(
        original_offset,
        u32::try_from(source.find("upvar").unwrap()).unwrap()
    );
    assert_eq!(
        binding.native_compilation_admission_selection(),
        tcl_registry::native_compilation::NativeCompilationSelection::Unknown
    );
    let site = binding.invocation_site().unwrap();
    let lexical_body = bindings
        .conditional_body_entry_at(&site.source, site.offset)
        .expect("independent unchanged declaration body recipe");
    assert!(lexical_body.owns_source(&site.source, site.offset));
    assert!(lexical_body.matches_parameters(&["arg"]));
    assert!(!bindings.has_actual_procedure_entry_at(&site.source, site.offset));
    let observations = binding.declaration_layout_observations.as_deref().unwrap();
    let entry = &original_declaration_layouts(observations)
        .unwrap()
        .next()
        .unwrap()
        .entry;
    let declaration = match entry.as_ref() {
        OriginalDiagnosticFrameEntry::Body(body) => {
            assert_eq!(body.as_ref(), lexical_body.as_ref());
            &body.allocation().site
        }
        OriginalDiagnosticFrameEntry::DeclaredProcedure(body) => &body.declaration,
        _ => panic!("independent procedure declaration owner"),
    };
    assert_eq!(entry.parameters()[0].name, "arg");
    assert_eq!(
        declaration.offset,
        u32::try_from(source.find("proc wrap").unwrap()).unwrap()
    );
    assert_eq!(
        entry.source().try_text().unwrap(),
        "upvar 1 other local; return $local"
    );
    assert_eq!(
        layout
            .snapshot
            .state
            .source_variables
            .namespace_identity
            .as_ref(),
        lexical_body.namespace_context()
    );
    assert!(entry.owns_original_context(&layout.snapshot.state.source_variables));
    let mut changed = tokens.clone();
    changed.word_exprs.pop();
    assert!(
        binding
            .declaration_operand_layout_advice(&changed)
            .is_none()
    );
}

#[test]
fn abrupt_prefix_retains_declaration_body_geometry_without_installation() {
    let source = "rename $old decl\nproc wrap {arg} {upvar 1 other local; return $local}";
    let (bindings, tokens) = inventory(source, "upvar 1 other local");
    let binding = tokens.source_binding.as_ref().unwrap();
    let layout = binding
        .declaration_operand_layout_advice(&tokens)
        .expect("unchanged abrupt table retains original grammar");
    assert!(
        layout.closed_lookup(),
        "the skipped rename did not change the borrowed table"
    );
    assert!(binding.unknown);
    assert!(binding.proved_execution_target().is_none());
    assert!(binding.compiler_lookup_state.is_none());
    let site = binding.invocation_site().unwrap();
    assert!(
        bindings
            .conditional_body_entry_at(&site.source, site.offset)
            .is_none()
    );
    assert!(
        bindings.entered_scripts.keys().all(
            |parent| parent.offset != u32::try_from(source.find("proc wrap").unwrap()).unwrap()
        )
    );
}

#[test]
fn public_declaration_consumer_retains_original_authored_body_namespace() {
    let source = "rename $old decl\nproc wrap {} {\nupvar 1 other local\nreturn $local\n}\n";
    let registry = tcl_registry::CommandRegistry::build_default();
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let realm = crate::realm::document_realm_bindings(source, profile, &registry);
    let offset = u32::try_from(source.find("upvar").unwrap()).unwrap();
    let segment = tcl_source::segment::segment_commands_with_offset_and_config(
        "upvar 1 other local",
        offset,
        config,
    )
    .remove(0);
    let mut tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::new(source),
        config,
        &segment,
    );
    realm.stamp_original_tokens(&mut tokens);
    let assistance = realm
        .original_declaration_assistance(&tokens, &registry)
        .expect("same original authored namespace and lexical declaration owner");
    assert_eq!(
        assistance.declarations,
        vec![crate::registry_invocation::DeclarationArgument {
            argument: 2,
            name: "local".to_owned(),
        },]
    );
    let binding = tokens.source_binding.as_ref().unwrap();
    assert!(binding.proved_execution_target().is_none());
    let (_, original_words, original_offset) = binding
        .original_compiler_source(&tokens)
        .expect("conditional declaration retains only its original vector");
    assert_eq!(original_words, tokens.words());
    assert_eq!(original_offset, offset);
    assert_eq!(
        binding.native_compilation_admission_selection(),
        tcl_registry::native_compilation::NativeCompilationSelection::Unknown
    );
}

#[test]
fn uninstalled_body_layout_stops_at_original_lookup_changes() {
    for prefix in [
        "rename upvar replacement",
        "proc upvar args {}",
        "unknown_call",
    ] {
        let source = format!("rename $old decl\nproc wrap {{}} {{{prefix}; upvar 1 other local}}");
        let (_, tokens) = inventory(&source, "upvar 1 other local");
        assert!(
            tokens
                .source_binding
                .as_ref()
                .unwrap()
                .declaration_operand_layout_advice(&tokens)
                .is_none(),
            "{prefix}"
        );
    }
}
#[test]
fn original_declared_body_parse_cut_keeps_only_the_complete_original_prefix() {
    // naming.source.original-declaration-layout-visits
    // docs/design/analysis/name-resolution-proofs/source-original-declaration-layout-visits.md
    // These are source lexical cuts, with no entered frame or Native admission.
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    let config = tcl_lexer::LexerConfig {
        base_offset: 700,
        base_line: 9,
        base_col: 4,
        ..tcl_lexer::LexerConfig::for_file_grammar(profile.grammar)
    };
    let body = "# retained comment\n\u{feff}puts pre; puts ${a{b}\nputs after";
    let whole = format!("proc p {{}} {{{body}}}");
    let origin = Arc::new(super::super::SourceOriginId::authored(&Arc::from(
        whole.as_str(),
    )));
    let base = u32::try_from(whole.find(body).unwrap()).unwrap();
    let source = ExecutedScriptSource::contiguous(Arc::clone(&origin), body, base).unwrap();
    let mut syntax = NestedLayoutSyntax::default();
    let prefix = syntax.commands(&source, config).unwrap();
    assert_eq!(prefix.len(), 1);
    assert_eq!(prefix[0].tokens.argv_texts, ["\u{feff}puts", "pre"]);
    assert_eq!(prefix[0].site.source, origin);
    assert_eq!(
        prefix[0].site.offset,
        base + u32::try_from(body.find('\u{feff}').unwrap()).unwrap()
    );
    assert_eq!(prefix[0].tokens.argv[0].start(), prefix[0].site.offset);
    assert!(prefix[0].tokens.source_binding.is_none());
    assert!(Arc::ptr_eq(
        &prefix,
        &syntax.commands(&source, config).unwrap()
    ));
    assert!(syntax.commands.keys().any(|key| key.config == config));
    let first_close = tcl_lexer::LexerConfig {
        braced_var: tcl_lexer::BracedVarStyle::FirstClose,
        ..config
    };
    let complete = syntax.commands(&source, first_close).unwrap();
    assert_eq!(complete.len(), 3);
    assert_eq!(complete[1].tokens.argv_texts, ["puts", "${a{b}"]);
    assert_eq!(complete[2].tokens.argv_texts, ["puts", "after"]);
    assert!(!Arc::ptr_eq(&prefix, &complete));
}

#[test]
fn original_declared_body_parse_cut_uses_geometry_after_f5_segment_merging() {
    // naming.source.original-declaration-layout-visits
    // docs/design/analysis/name-resolution-proofs/source-original-declaration-layout-visits.md
    // The merged source projection supplies no selected command or execution.
    let profile = tcl_dialect::DialectProfile::find("f5-irules").unwrap();
    let config = tcl_lexer::LexerConfig {
        braced_var: tcl_lexer::BracedVarStyle::Tcl9Nesting,
        ..tcl_lexer::LexerConfig::for_file_grammar(profile.grammar)
    };
    assert!(config.brace_line_continuation.continues());
    let parent = CommandAllocationSite {
        source: Arc::new(super::super::SourceOriginId::authored(&Arc::from(
            "eval $body",
        ))),
        offset: 0,
    };
    let source = ExecutedScriptSource::materialised(
        parent.clone(),
        vec![0],
        "if {1} {puts yes}\nelse {puts no}\nputs ${a{b}\nputs after",
    );
    let mut syntax = NestedLayoutSyntax::default();
    let prefix = syntax.commands(&source, config).unwrap();
    assert_eq!(prefix.len(), 1);
    assert_eq!(
        prefix[0].tokens.argv_texts,
        ["if", "1", "puts yes", "else", "puts no"]
    );
    assert_eq!(prefix[0].site.source, source.origin);
    assert_eq!(prefix[0].site.offset, 0);
    let first_close = tcl_lexer::LexerConfig {
        braced_var: tcl_lexer::BracedVarStyle::FirstClose,
        ..config
    };
    let complete = syntax.commands(&source, first_close).unwrap();
    assert_eq!(complete.len(), 3);
    assert_eq!(complete[0].tokens.argv_texts, prefix[0].tokens.argv_texts);
    assert_eq!(complete[2].tokens.argv_texts, ["puts", "after"]);
    let malformed_clause =
        ExecutedScriptSource::materialised(parent, vec![0], "if {1} {puts yes}\nelse ${a{b}");
    let clean_if = syntax.commands(&malformed_clause, config).unwrap();
    assert_eq!(clean_if.len(), 1);
    assert_eq!(clean_if[0].tokens.argv_texts, ["if", "1", "puts yes"]);
}

#[test]
fn original_declared_body_parse_cut_keeps_unavailable_distinct_from_empty() {
    // naming.source.original-declaration-layout-visits
    // docs/design/analysis/name-resolution-proofs/source-original-declaration-layout-visits.md
    // Hard lexical/Unicode refusal is not a complete empty source tape.
    let config = tcl_lexer::LexerConfig::for_dialect("tcl9.0");
    let parent = CommandAllocationSite {
        source: Arc::new(super::super::SourceOriginId::authored(&Arc::from(
            "eval $body",
        ))),
        offset: 0,
    };
    let mut syntax = NestedLayoutSyntax::default();
    for body in ["puts ${a{b}", "puts [list \"unterminated]"] {
        let source = ExecutedScriptSource::materialised(parent.clone(), vec![0], body);
        assert!(
            syntax.commands(&source, config).unwrap().is_empty(),
            "{body}"
        );
    }
    let clean = ExecutedScriptSource::materialised(parent.clone(), vec![0], "# only comment");
    assert!(syntax.commands(&clean, config).unwrap().is_empty());
    let welded = ExecutedScriptSource::materialised(
        parent.clone(),
        vec![0],
        "puts before; puts \"bad\"tail; puts after",
    );
    let prefix = syntax.commands(&welded, config).unwrap();
    assert_eq!(prefix.len(), 1);
    assert_eq!(prefix[0].tokens.argv_texts, ["puts", "before"]);
    let strict = tcl_lexer::LexerConfig {
        strict_quoting: true,
        ..config
    };
    assert!(syntax.commands(&welded, strict).is_none());
    let parsed = syntax.parsed_scripts;
    assert!(syntax.commands(&welded, strict).is_none());
    assert_eq!(syntax.parsed_scripts, parsed);
    let opaque = ExecutedScriptSource::materialised_image(
        parent,
        vec![0],
        tcl_lexer::SourceImage::native(b"puts \xff".as_slice()),
    );
    assert!(syntax.commands(&opaque, config).is_none());
    let mut overflow = clean;
    overflow.mapping = super::super::ExecutedScriptMapping::Contiguous { base: u32::MAX };
    assert!(syntax.commands(&overflow, config).is_none());
}

#[test]
fn original_declared_body_parser_keeps_bom_value_and_complete_parent_cache_key() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    // A decoded original script value owns its derived origin. Lexical
    // layout sharing provides no frame, entry, Native head or body execution.
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    assert_eq!(config.leading_bom, tcl_lexer::LeadingBom::Skip);
    let source = "eval \"\\uFEFFputs done\"";
    let image = tcl_lexer::SourceImage::document(source);
    let segment =
        tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config).remove(0);
    let tokens = super::super::source_command_tokens_boxed(&image, 0, config, &segment);
    let parent = CommandAllocationSite {
        source: Arc::new(super::super::SourceOriginId::authored(&Arc::from(source))),
        offset: 0,
    };
    let word = &tokens.words()[1];
    let value = effective_invocation_word(
        word,
        config.escapes,
        tcl_syntax::word_rules::WordValueRules::from_config(&config),
    );
    let value = std::str::from_utf8(value.literal_bytes().unwrap()).unwrap();
    assert_eq!(value, "\u{feff}puts done");
    let body = ExecutedScriptSource::from_word(parent.clone(), 0, word, value, config);
    assert_eq!(
        body.mapping,
        super::super::ExecutedScriptMapping::Materialised
    );
    assert_eq!(body.base(), 0);
    assert_ne!(body.origin, parent.source);
    let mut syntax = NestedLayoutSyntax::default();
    let first = syntax.commands(&body, config).unwrap();
    assert_eq!(first[0].tokens.argv_texts, ["\u{feff}puts", "done"]);
    assert_eq!(first[0].site.offset, 0);
    assert_eq!(first[0].site.source, body.origin);
    assert!(first[0].tokens.source_binding.is_none());
    assert!(Arc::ptr_eq(
        &first,
        &syntax.commands(&body, config).unwrap()
    ));
    let mut content = config;
    content.leading_bom = tcl_lexer::LeadingBom::Content;
    let second = syntax.commands(&body, content).unwrap();
    assert_eq!(second[0].tokens.argv_texts, first[0].tokens.argv_texts);
    assert!(
        !Arc::ptr_eq(&first, &second),
        "complete parent configurations remain distinct keys"
    );
    assert!(syntax.commands.keys().any(|key| key.config == config));
    assert!(syntax.commands.keys().any(|key| key.config == content));
    let whole =
        tcl_source::segment::segment_commands_image_with_offset_and_config(&body.text, 0, config)
            .unwrap();
    assert_eq!(
        whole[0].texts[0], "puts",
        "the actual whole-file purpose is independent"
    );
    let mut first_close = config;
    first_close.braced_var = tcl_lexer::BracedVarStyle::FirstClose;
    let indexed = ExecutedScriptSource::materialised(parent, vec![0], "puts ${a{b}");
    let selected = syntax.commands(&indexed, first_close).unwrap();
    assert_eq!(selected[0].tokens.argv_texts, ["puts", "${a{b}"]);
    assert!(syntax.commands(&indexed, config).unwrap().is_empty());
}
