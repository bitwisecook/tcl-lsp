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

//! Hosted and Logical structured source: original lookup, case roles and refused owners.

use super::*;

fn assert_hosted_source_shape_keeps_its_receipt(
    registry: &CommandRegistry,
    metadata: InvocationMetadataContext<'_>,
    child: &CommandTokens,
    source: &RegistryInvocationAssistance,
    eligible: bool,
) {
    if eligible {
        let shape = source
            .candidates
            .iter()
            .find(|shape| shape.command == "HTTP::uri")
            .unwrap();
        assert!(shape.hosted_source.is_some());
        assert_eq!(
            shape
                .source_argument_words(registry, Some(metadata), child)
                .unwrap(),
            [] as [crate::registry_invocation::EffectiveInvocationWord; 0]
        );
        assert!(
            shape
                .with_source_schema(registry, metadata, child, |schema| {
                    Some(schema.facts().canonical_command == "HTTP::uri")
                })
                .unwrap()
        );
        let mut stale = child.clone();
        stale.argv_texts[0] = "HTTP::path".to_owned();
        assert!(
            original_registry_invocation_assistance_with_metadata_context(
                registry,
                Some(metadata),
                &stale,
            )
            .is_none()
        );
        assert!(
            shape
                .source_argument_words(registry, Some(metadata), &stale)
                .is_none()
        );
        assert!(shape.source_argument_words(registry, None, child).is_none());
        let mut changed_label = shape.clone();
        changed_label.command = "HTTP::path".to_owned();
        let mut changed_origin = shape.clone();
        changed_origin.effective.origins[0] = InvocationWordOrigin::Written(0);
        let mut changed_metadata = shape.clone();
        changed_metadata.source_may_commit_response = !shape.source_may_commit_response;
        for changed in [changed_label, changed_origin, changed_metadata] {
            assert!(
                changed
                    .source_argument_words(registry, Some(metadata), child)
                    .is_none()
            );
            assert!(
                changed
                    .with_source_schema(registry, metadata, child, |_| Some(()))
                    .is_none()
            );
        }
    }
}

#[test]
fn hosted_original_source_assistance_keeps_metadata_and_execution_uncertainty_separate() {
    let environment = tcl_registry::model::resolve_known_environment("f5-irules").unwrap();
    let context = environment.default_context_registry();
    let registry = context.commands();
    let config = tcl_lexer::LexerConfig::from_grammar(environment.grammar());
    for (prefix, eligible) in [
        ("", true),
        ("rename HTTP::uri {}; ", false),
        ("proc HTTP::uri {} {}; ", false),
    ] {
        let source = format!("{prefix}when HTTP_REQUEST {{set value \"prefix[HTTP::uri]\"}}");
        let input = crate::analyser::ResolvedAnalysisInput::new(
            environment.analyser_profile(),
            environment.unit_profile(),
            context.clone(),
            config,
        );
        let unit = crate::compilation_unit::CompilationUnit::build_with_analysis_input(
            &source,
            crate::compilation_unit::UnitBuildOptions {
                registry,
                defer_top_level: false,
                config,
                dialect: Some(environment.unit_profile()),
                external_call_sites: None,
                declared_commands: None,
            },
            // Hosted catalogue advice answers where prior history is
            // unknown. A supplied document is a fresh rule load whose
            // original table answers first (user decision
            // 2026-10-21T00:10Z), so this contract supplies the explicit
            // unknown-history entry.
            Some(&crate::command_binding::SourceAnalysisEntry::unknown_history_for_test(&input)),
            &input,
        );
        let metadata = InvocationMetadataContext::for_module(registry, &unit.ir_module).unwrap();
        let event = unit.ir_module.irules_event_bodies.keys().next().unwrap();
        let procedure = &unit.ir_module.procedures[event];
        let statement = &procedure.body.statements[0];
        let parent = procedure
            .body
            .retained_source_tokens_for_statement(statement)
            .unwrap();
        let children =
            crate::word_subst::checked_original_argument_lifted_calls(parent, registry, metadata)
                .unwrap();
        let child = children[0].tokens.as_ref().unwrap();
        let reached =
            registry_invocation_assistance_with_metadata_context(registry, Some(metadata), child)
                .unwrap();
        let source = original_registry_invocation_assistance_with_metadata_context(
            registry,
            Some(metadata),
            child,
        )
        .unwrap();
        assert_eq!(source.unknown_residual, reached.unknown_residual);
        assert_eq!(source.may_be_absent, reached.may_be_absent);
        assert!(source.unanimous_command_words().is_none());
        assert_eq!(
            source
                .candidates
                .iter()
                .any(|shape| shape.command == "HTTP::uri"),
            eligible
        );
        assert_hosted_source_shape_keeps_its_receipt(registry, metadata, child, &source, eligible);
    }
}

#[test]
fn logical_source_structure_uses_original_lookup_without_a_physical_frame() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Conditional source roles remain separate from Native handler and frame entry.
    let owner =
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&owner),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    let registry = owner.commands();
    let tokens = logical_source_tokens("if {1} {set value VALUE}", registry, &input);
    let binding = tokens.source_binding.as_ref().unwrap();
    let advice = binding.original_compilation_lookup_advice(&tokens).unwrap();
    assert!(!advice.closed_lookup());
    assert!(advice.closed_logical_source_lookup());
    let metadata = InvocationMetadataContext::for_analysis_input(registry, &input).unwrap();
    let shape =
        logical_structured_invocation_with_metadata_context(registry, metadata, &tokens, None)
            .expect("genuine Logical source body shape");
    assert_eq!(
        shape.lowering_hook(),
        Some(tcl_registry::hooks::LoweringHookId::If)
    );
    assert!(
        shape
            .written_roles()
            .contains(&(1, tcl_registry::ArgRole::Body))
    );
    assert!(
        resolved_handler_invocation_with_metadata_context(registry, Some(metadata), &tokens)
            .is_none()
    );
    assert!(binding.native_compilation_admission.is_none());
}

#[test]
fn conditional_logical_indices_retain_direct_if_and_refuse_case_lists() {
    let owner =
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&owner),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    let registry = owner.commands();
    let metadata = InvocationMetadataContext::for_analysis_input(registry, &input).unwrap();
    let direct = logical_source_tokens("if {1} {set value VALUE}", registry, &input);
    let direct =
        logical_structured_invocation_with_metadata_context(registry, metadata, &direct, None)
            .unwrap();
    assert_eq!(direct.conditional_logical_body_indices(), Some(vec![1]));
    for source in [
        "switch -exact -- $cmd {foo {return FOO} default {return OTHER}}",
        "switch -exact -- $cmd foo {return FOO} default {return OTHER}",
    ] {
        let tokens = logical_source_tokens(source, registry, &input);
        let shape =
            logical_structured_invocation_with_metadata_context(registry, metadata, &tokens, None)
                .unwrap();
        let Some(StructuredSourceRoles::Logical(roles)) = shape.source_roles.as_ref() else {
            panic!("genuine Logical case role owner");
        };
        assert!(roles.case_body_operands().is_some());
        assert!(shape.conditional_logical_body_indices().is_none());
    }
}

#[test]
fn original_logical_case_roles_keep_selection_residuals_out_of_structured_ir() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Possible bodies are source advice; the unknown option/error branch
    // cannot borrow a definitive Switch or complete Native argument roles.
    let owner =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&owner),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    let registry = owner.commands();
    let metadata = InvocationMetadataContext::for_analysis_input(registry, &input).unwrap();
    let closed_source = "switch -exact -- $cmd {foo {return FOO} default {return OTHER}}";
    let closed = logical_source_tokens(closed_source, registry, &input);
    let shape =
        logical_structured_invocation_with_metadata_context(registry, metadata, &closed, None)
            .expect("closed original Logical source-role projection");
    assert!(
        shape
            .written_roles()
            .contains(&(3, tcl_registry::ArgRole::Body))
    );
    assert!(!shape.invocation.facts.arg_roles_complete);
    let Some(StructuredSourceRoles::Logical(roles)) = shape.source_roles.as_ref() else {
        panic!("the original Logical role issuer must retain its own purpose");
    };
    let bodies = roles.case_body_operands().unwrap();
    assert!(!bodies.selection_unknown && !bodies.no_match_possible);
    assert_eq!(bodies.bodies[0].argument, 3);
    assert_eq!(bodies.bodies[0].list_element, Some(1));
    assert!(
        closed
            .source_binding
            .as_ref()
            .unwrap()
            .native_compilation_admission
            .is_none()
    );
    let ambiguous_source = "switch -exact $cmd {foo {return FOO} default {return OTHER}}";
    let ambiguous = logical_source_tokens(ambiguous_source, registry, &input);
    assert!(
        logical_structured_invocation_with_metadata_context(registry, metadata, &ambiguous, None,)
            .is_none()
    );
    let possible =
        original_logical_operation_invocation_with_metadata_context(registry, metadata, &ambiguous)
            .unwrap();
    let realm = ambiguous
        .source_binding
        .as_ref()
        .unwrap()
        .invocation_realm()
        .unwrap();
    let roles = possible
        .with_metadata_schema(registry, metadata, realm, |schema| {
            schema.authored_logical_source_role_projection()
        })
        .unwrap();
    assert!(roles.selection_unknown());
    assert_eq!(roles.case_body_operands().unwrap().bodies[0].argument, 2);
    for (source, switch_expected) in [(closed_source, true), (ambiguous_source, false)] {
        let mut lowerer = crate::lowering::Lowerer::with_config(registry, input.lexer_config())
            .with_resolved_analysis_input(input.clone());
        let script = &lowerer.lower(source).top_level;
        assert_eq!(
            matches!(
                script.statements.last().unwrap(),
                crate::ir::Statement::Switch { .. }
            ),
            switch_expected
        );
    }
}

#[test]
fn original_logical_case_roles_refuse_missing_foreign_stale_and_replaced_owners() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let owner =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&owner),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    let registry = owner.commands();
    let tokens = logical_source_tokens("switch -- $cmd {café {return VALUE}}", registry, &input);
    let current = InvocationMetadataContext::for_analysis_input(registry, &input).unwrap();
    assert!(
        logical_structured_invocation_with_metadata_context(registry, current, &tokens, None)
            .is_some()
    );
    let older = tcl_registry::model::ingress::resolve_environment("tcl8.4")
        .default_context_registry()
        .with_command_store(registry.snapshot().shared_registry());
    let mut changed_config = input.lexer_config();
    changed_config.strict_quoting = !changed_config.strict_quoting;
    let native = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    for changed in [
        crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::new(older),
            input.lexer_config(),
        ),
        crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            input.context_registry(),
            changed_config,
        ),
        crate::analyser::ResolvedAnalysisInput::new(
            native,
            native,
            input.context_registry(),
            input.lexer_config(),
        ),
    ] {
        let context = InvocationMetadataContext::for_analysis_input(registry, &changed).unwrap();
        assert!(
            logical_structured_invocation_with_metadata_context(registry, context, &tokens, None)
                .is_none()
        );
    }
    let foreign =
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
    assert!(
        InvocationMetadataContext::for_analysis_input(
            registry,
            &crate::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                foreign,
                input.lexer_config(),
            )
        )
        .is_none()
    );
    let mut missing = tokens.clone();
    missing.source_binding = None;
    assert!(
        logical_structured_invocation_with_metadata_context(registry, current, &missing, None)
            .is_none()
    );
    let replaced = logical_source_tokens(
        "proc switch args {}; switch -- $cmd {café {return VALUE}}",
        registry,
        &input,
    );
    assert!(
        logical_structured_invocation_with_metadata_context(registry, current, &replaced, None)
            .is_none()
    );
    let captured = logical_source_tokens(
        "interp alias {} dispatch {} switch -exact --; dispatch $cmd {café {return VALUE}}",
        registry,
        &input,
    );
    assert!(
        logical_structured_invocation_with_metadata_context(registry, current, &captured, None)
            .is_none(),
        "captured effective operands are not direct written slots"
    );
}

#[test]
fn logical_source_structure_refuses_old_availability_stale_grammar_and_replacements() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let owner =
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&owner),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    let registry = owner.commands();
    let tokens = logical_source_tokens("try {set value VALUE}", registry, &input);
    let current = InvocationMetadataContext::for_analysis_input(registry, &input).unwrap();
    assert!(original_declared_structured_invocation(registry, current, &tokens).is_some());
    let older = tcl_registry::model::ingress::resolve_environment("tcl8.4")
        .default_context_registry()
        .with_command_store(registry.snapshot().shared_registry());
    let older = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::new(older),
        input.lexer_config(),
    );
    let older = InvocationMetadataContext::for_analysis_input(registry, &older).unwrap();
    assert!(original_declared_structured_invocation(registry, older, &tokens).is_none());
    let mut config = input.lexer_config();
    config.strict_quoting = !config.strict_quoting;
    let stale = crate::analyser::ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        input.context_registry(),
        config,
    );
    let stale = InvocationMetadataContext::for_analysis_input(registry, &stale).unwrap();
    assert!(original_declared_structured_invocation(registry, stale, &tokens).is_none());
    let foreign =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let foreign = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        foreign,
        input.lexer_config(),
    );
    assert!(InvocationMetadataContext::for_analysis_input(registry, &foreign).is_none());
    let replaced = logical_source_tokens(
        "proc try {args} {}; try {set value VALUE}",
        registry,
        &input,
    );
    assert!(original_declared_structured_invocation(registry, current, &replaced).is_none());
    let mut missing = tokens.clone();
    missing.source_binding = None;
    assert!(original_declared_structured_invocation(registry, current, &missing).is_none());
}

#[test]
fn logical_source_shape_cannot_substitute_for_missing_native_entry() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let owner =
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&owner),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    let registry = owner.commands();
    let tokens = logical_source_tokens("if {1} {set value VALUE}", registry, &input);
    let native_profile = tcl_dialect::DialectProfile::find("tcl9.1").unwrap();
    let native = crate::analyser::ResolvedAnalysisInput::new(
        native_profile,
        native_profile,
        std::sync::Arc::clone(&owner),
        input.lexer_config(),
    );
    let native = InvocationMetadataContext::for_analysis_input(registry, &native).unwrap();
    assert!(original_declared_structured_invocation(registry, native, &tokens).is_none());
    assert!(
        resolved_handler_invocation_with_metadata_context(registry, Some(native), &tokens)
            .is_none()
    );
    assert!(
        tokens
            .source_binding
            .as_ref()
            .unwrap()
            .native_compilation_admission
            .is_none()
    );
}
