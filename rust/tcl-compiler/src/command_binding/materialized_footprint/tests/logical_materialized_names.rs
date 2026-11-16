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

//! Materialized names under a Logical declaration: horizons, realms and refused owners.

use super::*;

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
        row.snapshot = Arc::new(super::super::super::SourceLookupSnapshot::in_realm(
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
    let original_row = super::super::super::declaration_layout::original_declaration_layouts(
        binding.declaration_layout_observations.as_deref().unwrap(),
    )
    .unwrap()
    .next()
    .unwrap()
    .clone();
    let mut different = original_row.clone();
    let mut state = different.snapshot.state.clone();
    state.remove("::set");
    different.snapshot = Arc::new(super::super::super::SourceLookupSnapshot::in_realm(
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
    invalid_point.lookup_state = Some(Arc::new(
        super::super::super::SourceLookupSnapshot::in_realm(state, point.realm),
    ));
    let mut invalid_tokens = tokens.clone();
    invalid_tokens.source_binding = Some(invalid_point.clone());
    assert!(
        invalid_point
            .original_materialized_footprint(&invalid_tokens, source, context.commands(), metadata)
            .is_none()
    );
}

fn advice_snapshot(
    binding: &super::super::SourceInvocationBinding,
    tokens: &crate::ir::CommandTokens,
) -> Arc<super::super::super::SourceLookupSnapshot> {
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
        let rows = super::super::super::declaration_layout::original_declaration_layouts(
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
        super::super::super::SourceRuntimeReachability::Conditional
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
