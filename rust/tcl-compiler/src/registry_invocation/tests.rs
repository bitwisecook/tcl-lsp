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

use crate::interprocedural::test_body_identity::fixture_id;

use super::*;
use tcl_registry::StateTransition;

use crate::ir::{SourceSite, WordOpacity};
use crate::segmenter::segment_commands;

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

fn logical_source_tokens(
    source: &str,
    registry: &CommandRegistry,
    input: &crate::analyser::ResolvedAnalysisInput,
) -> CommandTokens {
    let image = tcl_lexer::SourceImage::document(source);
    let config = input.lexer_config();
    let bindings =
        crate::command_binding::SourceCommandBindings::analyse_image_in_frame_with_options(
            &image,
            &crate::var_resolve::VariableExecutionFrame::Unknown,
            config,
            registry,
            crate::command_binding::SourceAnalysisOptions::for_logical_source(input).unwrap(),
        )
        .unwrap();
    let segment =
        crate::segmenter::segment_commands_image_with_offset_and_config(&image, 0, config)
            .unwrap()
            .pop()
            .unwrap();
    let mut tokens = CommandTokens::from_segmented(&image.source_map(), config, &segment);
    bindings.stamp_original_tokens(&mut tokens);
    tokens
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

fn test_context() -> SemanticContext {
    SemanticContext::for_environment("tcl8.6")
}

fn literal(text: &str) -> WordExpr {
    WordExpr::Literal {
        text: text.to_owned(),
        source: SourceSite::source(tcl_lexer::Span::new(0, 0)),
    }
}

#[test]
fn original_expression_context_keeps_logical_input_and_written_operands() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let owner =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let mut config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    config.strict_quoting = !config.strict_quoting;
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&owner),
        config,
    );
    let registry = owner.commands();
    for source in [
        "expr {2 + 3}",
        "rename expr arithmetic; arithmetic {2 + 3}",
        "interp alias {} arithmetic {} expr; arithmetic {2 + 3}",
    ] {
        let tokens = logical_source_tokens(source, registry, &input);
        let advice = original_expression_operand_advice(registry, &tokens)
            .expect("the original Logical expression keeps its selected grammar");
        assert_eq!(advice.expression_text, "2 + 3");
        assert!(advice.lookup_closed);
        assert!(matches!(
            advice.frame,
            crate::var_resolve::VariableExecutionFrame::Unknown
        ));
        assert_eq!(
            source.get(
                advice.expression_base as usize
                    ..advice.expression_base as usize + advice.expression_text.len()
            ),
            Some("2 + 3"),
        );
        assert!(
            tokens
                .source_binding
                .as_ref()
                .unwrap()
                .native_compilation_admission
                .is_none()
        );
        let metadata = InvocationMetadataContext::for_analysis_input(registry, &input).unwrap();
        assert!(
            resolved_handler_invocation_with_metadata_context(registry, Some(metadata), &tokens)
                .is_none()
        );
    }
    let replaced = logical_source_tokens(
        "proc expr args {return STUB}; expr {2 + 3}",
        registry,
        &input,
    );
    assert!(original_expression_operand_advice(registry, &replaced).is_none());
    let captured = logical_source_tokens(
        "interp alias {} arithmetic {} expr {2 + 3}; arithmetic",
        registry,
        &input,
    );
    assert!(original_expression_operand_advice(registry, &captured).is_none());
}

#[test]
fn original_expression_context_keeps_availability_and_terminal_point_refusals() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let mut registry = CommandRegistry::build_default();
    let mut descriptor = registry.get("expr").unwrap().clone();
    descriptor.name = "arithmetic";
    descriptor.surface = Some(tcl_dialect::model::SpecSurface::TCL86_PLUS);
    registry.insert(descriptor);
    let registry = std::sync::Arc::new(registry);
    let input_for = |environment| {
        let context = tcl_registry::model::ingress::resolve_environment(environment)
            .default_context_registry();
        crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::new(context.with_command_store(std::sync::Arc::clone(&registry))),
            config,
        )
    };
    let current = input_for("tcl9.1");
    let older = input_for("tcl8.4");
    let source = "arithmetic {2 + 3}";
    let tokens = logical_source_tokens(source, &registry, &current);
    assert!(original_expression_operand_advice(&registry, &tokens).is_some());
    assert_eq!(
        current
            .borrowed_context_registry()
            .commands()
            .snapshot()
            .semantic_key(),
        older
            .borrowed_context_registry()
            .commands()
            .snapshot()
            .semantic_key()
    );
    let unavailable = logical_source_tokens(source, &registry, &older);
    assert!(original_expression_operand_advice(&registry, &unavailable).is_none());
    let mut missing = tokens.clone();
    missing.source_binding = None;
    assert!(original_expression_operand_advice(&registry, &missing).is_none());
    let mut changed = tokens.clone();
    changed.argv_texts[1] = "4 + 5".to_owned();
    changed.word_exprs[1] = literal("4 + 5");
    assert!(original_expression_operand_advice(&registry, &changed).is_none());
    let foreign =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    assert!(original_expression_operand_advice(foreign.commands(), &tokens).is_none());

    let image = tcl_lexer::SourceImage::document(source);
    let segment =
        crate::segmenter::segment_commands_image_with_offset_and_config(&image, 0, config)
            .unwrap()
            .remove(0);
    let mut stale_config = config;
    stale_config.strict_quoting = !stale_config.strict_quoting;
    let stale = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        current.context_registry(),
        stale_config,
    );
    for metadata_context in [
        InvocationMetadataInput::SuppliedSource(None),
        InvocationMetadataInput::SuppliedSource(Some(&stale)),
    ] {
        let mut options =
            crate::command_binding::SourceAnalysisOptions::for_logical_source(&current).unwrap();
        options.metadata_context = metadata_context;
        let bindings =
            crate::command_binding::SourceCommandBindings::analyse_image_in_frame_with_options(
                &image,
                &crate::var_resolve::VariableExecutionFrame::Unknown,
                config,
                &registry,
                options,
            )
            .unwrap();
        let mut refused = CommandTokens::from_segmented(&image.source_map(), config, &segment);
        bindings.stamp_original_tokens(&mut refused);
        assert!(original_expression_operand_advice(&registry, &refused).is_none());
    }
}

#[test]
fn expression_operand_advice_keeps_original_sites_and_replaced_handlers_separate() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    for (source, expected) in [
        ("proc f {i} {set y [expr {$i + 0}]}", 1),
        (
            "interp alias {} arithmetic {} expr; proc f {i} {set y [arithmetic {$i + 0}]}",
            1,
        ),
        (
            "rename expr original_expr; proc expr args {list PLAIN}; proc f {i} {set y [expr {$i + 0}]}",
            0,
        ),
        ("proc f {i} {set y [unknown {$i + 0}]}", 0),
    ] {
        let unit = crate::compilation_unit::CompilationUnit::build_for_profile(
            source,
            registry,
            false,
            registry.profile().unwrap(),
        );
        let mut count = 0;
        crate::ir::for_each_statement(
            &unit
                .ir_module
                .procedures
                .get(&fixture_id(&unit.ir_module.procedures, "::f"))
                .expect("authored fixture body")
                .body,
            &mut |statement| {
                for call in crate::word_subst::lifted_calls(
                    statement.tokens(),
                    tcl_lexer::LexerConfig::for_profile(registry.profile()),
                ) {
                    if let Some(tokens) = call.tokens
                        && let Some(advice) = original_expression_operand_advice(registry, &tokens)
                    {
                        assert!(
                            source
                                .get(advice.span.as_range())
                                .is_some_and(|text| text.contains("$i + 0"))
                        );
                        count += 1;
                    }
                }
            },
        );
        assert_eq!(count, expected, "{source}");
    }
}

#[test]
fn default_return_operand_advice_preserves_generic_calls_and_replacements() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    for (source, expected) in [
        ("proc make {} {return [list VALUE]}", vec!["[list VALUE]"]),
        ("proc make {} {return -code error VALUE}", vec![]),
        (
            "rename return stock_return; proc return args {list PLAIN}; proc make {} {return [list VALUE]}",
            vec![],
        ),
    ] {
        let unit = crate::compilation_unit::CompilationUnit::build_for_profile(
            source,
            registry,
            false,
            registry.profile().unwrap(),
        );
        let procedure = unit
            .ir_module
            .procedures
            .get(&fixture_id(&unit.ir_module.procedures, "::make"))
            .expect("authored fixture body");
        let mut values = Vec::new();
        crate::ir::for_each_statement(&procedure.body, &mut |statement| {
            if let Some(tokens) = procedure
                .body
                .retained_source_tokens_for_statement(statement)
            {
                values.extend(
                    advisory_return_values(registry, tokens)
                        .into_iter()
                        .map(|word| word.legacy_text()),
                );
            }
        });
        assert_eq!(values, expected, "{source}");
    }
}

#[test]
fn decoded_native_byte_words_preserve_cardinality_and_checked_text() {
    let bytes = EffectiveInvocationWord::from_bytes(&[0xff]);
    assert_eq!(bytes.literal_bytes(), Some([0xff].as_slice()));
    // Implementation contract: naming.invocation.known-native-byte-values
    // docs/design/analysis/name-resolution-proofs/known-native-byte-values.md
    assert_eq!(
        bytes.as_registry_word(),
        InvocationWord::KnownBytes(&[0xff])
    );
    assert_eq!(bytes.as_registry_word().literal(), None);
    assert!(bytes.as_registry_word().has_exactly_one_argv_entry());
    let unicode = EffectiveInvocationWord::from_bytes("ÿ".as_bytes());
    assert_eq!(unicode.as_registry_word(), InvocationWord::Literal("ÿ"));
    assert_ne!(bytes, unicode);
}

#[test]
fn static_command_word_decodes_only_original_static_values() {
    let config = tcl_lexer::LexerConfig::for_dialect("tcl8.6");
    let rules = WordValueRules::from_config(&config);
    for (source, expected) in [
        (r"se\x74 x 1", Some("set")),
        (r"{se\x74} x 1", Some(r"se\x74")),
        ("\"set\" x 1", Some("set")),
        ("$head x 1", None),
        ("[gethead] x 1", None),
        ("{*}$head x 1", None),
    ] {
        let segment = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
            .pop()
            .expect("one invocation");
        let tokens =
            CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(source), config, &segment);
        assert_eq!(
            static_command_word(&tokens, config.escapes, rules).as_deref(),
            expected,
            "{source}"
        );
    }
}

fn last_source_tokens(source: &str) -> CommandTokens {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
    let bindings = crate::command_binding::SourceCommandBindings::analyse(source, config, registry);
    let segment = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
        .pop()
        .unwrap();
    CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(source), config, &segment)
        .with_source_binding(
            bindings.invocation_at_source("ignored-authored-head", segment.span.start()),
        )
}

#[test]
fn possible_name_operands_preserve_alias_prefix_and_missing_binding_residual() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let mut tokens = last_source_tokens("interp alias {} store {} set target; store VALUE");
    let possible = possible_variable_name_operands(registry, None, &tokens).unwrap();
    assert!(
        possible
            .operands()
            .any(|(role, word)| role == tcl_registry::ArgRole::VarWrite
                && word == &EffectiveInvocationWord::Literal("target".to_owned()))
    );
    let mut unknown_binding = crate::command_binding::SourceInvocationBinding::default();
    unknown_binding.unknown = true;
    tokens.source_binding = Some(unknown_binding);
    let unknown = possible_variable_name_operands(registry, None, &tokens).unwrap();
    assert!(unknown.unknown_residual());
    assert!(unknown.operands().next().is_none());
    tokens.source_binding = Some(crate::command_binding::SourceInvocationBinding::default());
    let closed = possible_variable_name_operands(registry, None, &tokens).unwrap();
    assert!(!closed.unknown_residual());
    assert!(closed.operands().next().is_none());
    tokens.source_binding = None;
    assert!(possible_variable_name_operands(registry, None, &tokens).is_none());
}

#[test]
fn possible_name_operands_keep_expansion_cardinality_separate_from_name_values() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
    for (command, dynamic, expected_roles) in [
        ("set {*}$n VALUE", true, 2),
        ("set fixed {*}$n", false, 2),
        ("set {*}{fixed} VALUE", false, 1),
        ("set fixed VALUE EXTRA", false, 0),
    ] {
        let source = format!("proc f {{n}} {{{command}; return ok}}");
        let bindings =
            crate::command_binding::SourceCommandBindings::analyse(&source, config, registry);
        let offset = source.find(command).unwrap();
        let offset_u32 = u32::try_from(offset).unwrap();
        let segment = crate::segmenter::segment_commands_with_offset_and_config(
            &source[offset..],
            offset_u32,
            config,
        )
        .remove(0);
        let tokens =
            CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(&source), config, &segment)
                .with_source_binding(
                    bindings.invocation_at_source("unused-written-head", offset_u32),
                );
        let possible = possible_variable_name_operands(registry, None, &tokens).unwrap();
        let mut operands = Vec::new();
        for operand in possible.operands() {
            if !operands.contains(&operand) {
                operands.push(operand);
            }
        }
        assert_eq!(operands.len(), expected_roles, "{command}");
        assert_eq!(
            operands
                .iter()
                .any(|(_, word)| matches!(word, EffectiveInvocationWord::Dynamic)),
            dynamic,
            "{command}"
        );
        if dynamic {
            assert!(normal_transfer_invocation(registry, None, &tokens).is_none());
        }
    }
}

#[test]
fn evaluated_wrapper_retains_only_possible_output_names_and_lookup_phase() {
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let mut tokens = f5_source_tokens("catch {foo} ::err");
    tokens.synthetic = Some(crate::ir::SyntheticMarker::EvaluatedWrapper);
    let possible = possible_variable_name_operands(registry, None, &tokens).unwrap();
    assert!(possible.unknown_residual());
    assert!(possible.phased_operands().any(|(role, word, phase, destroys)| {
            role == tcl_registry::ArgRole::VarWrite
                && !destroys
                && word == &EffectiveInvocationWord::Literal("::err".to_owned())
                && phase
                    == tcl_registry::native_compilation::VariableOperandBindingPhase::NormalContinuation
        }));
    assert!(normal_transfer_invocation(registry, None, &tokens).is_none());
    assert!(resolved_tokens_invocation(registry, None, &tokens).is_none());
    tokens.synthetic = Some(crate::ir::SyntheticMarker::EvaluatedArguments);
    assert!(possible_variable_name_operands(registry, None, &tokens).is_none());

    let mut replaced = f5_source_tokens("proc catch args {return CUSTOM}; catch BODY ::err");
    replaced.synthetic = Some(crate::ir::SyntheticMarker::EvaluatedWrapper);
    assert!(
        possible_variable_name_operands(registry, None, &replaced)
            .unwrap()
            .operands()
            .next()
            .is_none()
    );
}

#[test]
fn possible_method_name_operands_do_not_create_normal_store_proof() {
    use crate::command_binding::{SourceAnalysisOptions, SourceCommandBindings};
    let source = "oo::class create C {method m {} {set state VALUE}}";
    let config = tcl_lexer::LexerConfig::default();
    let registry = CommandRegistry::build_default();
    let bindings = SourceCommandBindings::analyse_with_options(
        source,
        config,
        &registry,
        SourceAnalysisOptions {
            invocation_dialect: Some(tcl_registry::InvocationDialect::for_version(
                tcl_dialect::TclVersion::V9_1,
            )),
            native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                ..Default::default()
            },
            ..SourceAnalysisOptions::default()
        },
    );
    let offset = u32::try_from(source.find("set state").unwrap()).unwrap();
    let segment = crate::segmenter::segment_commands_with_offset_and_config(
        "set state VALUE",
        offset,
        config,
    )
    .pop()
    .unwrap();
    let tokens =
        CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(source), config, &segment)
            .with_source_binding(bindings.invocation_at_source("set", offset));
    let possible = possible_variable_name_operands(&registry, None, &tokens).unwrap();
    assert!(possible.unknown_residual());
    assert!(
        possible
            .operands()
            .any(|(role, word)| role == tcl_registry::ArgRole::VarWrite
                && word == &EffectiveInvocationWord::Literal("state".to_owned()))
    );
    assert!(normal_transfer_invocation(&registry, None, &tokens).is_none());
}

#[test]
fn native_operation_plan_retains_original_words_and_reports_missing_source() {
    let config = tcl_lexer::LexerConfig::for_dialect("tcl8.6");
    let rules = WordValueRules::from_config(&config);
    let source = r"se\x74 value 7";
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let bindings = crate::command_binding::SourceCommandBindings::analyse_with_options(
        source,
        config,
        registry,
        crate::command_binding::SourceAnalysisOptions {
            invocation_dialect: Some(crate::environment_ingress::authoring_invocation_dialect(
                registry,
                registry.profile(),
                config,
            )),
            native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                mode: tcl_registry::native_compilation::NativeCompilationMode::BytecodeObject,
                ..crate::environment_ingress::authoring_native_compilation()
            },
            ..crate::command_binding::SourceAnalysisOptions::default()
        },
    );
    let segment = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
        .pop()
        .unwrap();
    let mut tokens =
        CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(source), config, &segment)
            .with_source_binding(bindings.invocation_at_source("set", 0));
    let plan = native_operation_selection_plan(&tokens, config.escapes, rules)
        .unwrap()
        .unwrap();
    assert_eq!(plan.source, r"se\x74 value 7");
    assert_eq!(plan.requirements[0].name, "set");
    assert_eq!(
        plan.guard,
        tcl_runtime_api::CommandBindingGuard::BeforeArguments
    );
    tokens.argv_texts.pop();
    assert_eq!(
        native_operation_selection_plan(&tokens, config.escapes, rules),
        Err(NativeOperationSelectionDecline::MissingOriginalSource),
    );
    let generic = last_source_tokens("pid");
    assert_eq!(
        native_operation_selection_plan(&generic, config.escapes, rules),
        Ok(None)
    );
}

fn f5_source_tokens(source: &str) -> CommandTokens {
    f5_source_tokens_in_realm(source, tcl_dialect::model::InvocationRealm::RuleLoader)
}

fn f5_source_tokens_in_realm(
    source: &str,
    realm: tcl_dialect::model::InvocationRealm,
) -> CommandTokens {
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let config = tcl_lexer::LexerConfig::for_dialect("f5-irules");
    let analysis = crate::command_binding::SourceCommandBindings::analyse_with_options(
        source,
        config,
        registry,
        crate::command_binding::SourceAnalysisOptions {
            invocation_realm: realm,
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(
                tcl_dialect::DialectProfile::find("f5-irules").expect("selected fixture entry"),
            )),
            native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                mode: tcl_registry::native_compilation::NativeCompilationMode::BytecodeObject,
                frame: tcl_registry::native_compilation::NativeCompilationFrame::ScriptCode,
                catch_depth: Some(0),
                loop_depth: 0,
            },
            ..crate::command_binding::SourceAnalysisOptions::default()
        },
    );
    let segment = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
        .pop()
        .unwrap();
    CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(source), config, &segment)
        .with_source_binding(analysis.invocation_at_source("ignored", segment.span.start()))
}

#[test]
fn normal_ensemble_leaf_requires_the_actual_release_worker() {
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let registry = tcl_registry::model::ingress::static_context_for(profile).commands();
        let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
        for (command, introduced) in [
            ("string toupper hello", "tcl8.5"),
            ("binary format a* hello", "tcl8.6"),
        ] {
            let worker = if command.starts_with("string") {
                "::tcl::string::toupper"
            } else {
                "::tcl::binary::format"
            };
            for replaced in [false, true] {
                let source = if replaced {
                    format!(
                        "namespace eval ::tcl::string {{}}; namespace eval ::tcl::binary {{}}; proc {worker} args {{return CUSTOM}}; {command}"
                    )
                } else {
                    command.to_owned()
                };
                let bindings = crate::command_binding::SourceCommandBindings::analyse(
                    &source, config, registry,
                );
                let segment =
                    crate::segmenter::segment_commands_with_offset_and_config(&source, 0, config)
                        .pop()
                        .unwrap();
                let tokens = CommandTokens::from_segmented(
                    &tcl_lexer::SourceMap::new(&source),
                    config,
                    &segment,
                )
                .with_source_binding(
                    bindings.invocation_at_source("ignored", segment.span.start()),
                );
                let expects_worker = profile != "jim" && profile >= introduced;
                assert_eq!(
                    normal_representation_invocation(registry, None, &tokens).is_some(),
                    !replaced || !expects_worker,
                    "{profile}: {source}"
                );
            }
        }
    }
}

#[test]
fn f5_value_provider_proves_normal_representation_without_c_workers() {
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    for command in ["string toupper hello", "binary format a* hello"] {
        let tokens = f5_source_tokens(command);
        assert!(resolved_tokens_invocation(registry, None, &tokens).is_none());
        assert!(normal_representation_invocation(registry, None, &tokens).is_some());
        let replaced = f5_source_tokens(&format!(
            "proc {} args {{return CUSTOM}}; {command}",
            command.split_whitespace().next().unwrap()
        ));
        assert!(normal_representation_invocation(registry, None, &replaced).is_none());
    }
    let tokens = f5_source_tokens_in_realm(
        "namespace eval ::tcl::string {}; proc ::tcl::string::toupper args {return CUSTOM}; string toupper hello",
        tcl_dialect::model::InvocationRealm::InterpreterRuntime,
    );
    assert!(normal_representation_invocation(registry, None, &tokens).is_some());
    let malformed =
        f5_source_tokens("proc ::tcl::string::toupper args {return CUSTOM}; string toupper hello");
    assert!(normal_representation_invocation(registry, None, &malformed).is_none());
}

#[test]
fn normal_completion_retains_handler_return_without_licensing_unknown_compilation() {
    let f5 = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let tokens = f5_source_tokens("return VALUE");
    assert!(!normal_handler_completion_route(f5, None, &tokens).normal_possible());
    assert!(resolved_tokens_invocation(f5, None, &tokens).is_none());
    assert!(
        tokens
            .source_binding
            .as_ref()
            .unwrap()
            .compiled_execution_unknown()
    );
    let replaced = f5_source_tokens("proc return args {set x CUSTOM}; return VALUE");
    assert!(normal_handler_completion_route(f5, None, &replaced).normal_possible());
    let c = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let tokens = last_source_tokens("return VALUE");
    assert!(!normal_handler_completion_route(c, None, &tokens).normal_possible());
    assert!(resolved_tokens_invocation(c, None, &tokens).is_some());
}

#[test]
fn f5_connection_controls_preserve_following_normal_return_identity() {
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    for command in ["drop", "discard", "reject", "DNS::return"] {
        let tokens = f5_source_tokens(&format!("{command}; return"));
        assert!(
            !normal_handler_completion_route(registry, None, &tokens).normal_possible(),
            "{command}"
        );
        assert!(resolved_tokens_invocation(registry, None, &tokens).is_none());
        assert!(
            tokens
                .source_binding
                .as_ref()
                .unwrap()
                .compiled_execution_unknown()
        );
    }
    let replaced = f5_source_tokens("proc drop {} {proc return args {set x CUSTOM}}; drop; return");
    assert!(normal_handler_completion_route(registry, None, &replaced).normal_possible());
}

#[test]
fn representation_contract_preserves_handler_proof_without_opcode_proof() {
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let tokens = f5_source_tokens("llength {a b}");
    assert!(resolved_tokens_invocation(registry, None, &tokens).is_none());
    let normal = normal_representation_invocation(registry, None, &tokens).unwrap();
    assert_eq!(normal.argument_literal(0).as_deref(), Some("a b"));
    assert_eq!(
        normal.argument_type_hint(0).unwrap().expected,
        Some(tcl_registry::TclType::List)
    );
    assert!(
        tokens
            .source_binding
            .as_ref()
            .unwrap()
            .compiled_execution_unknown()
    );
}

#[test]
fn unavailable_f5_string_member_cannot_donate_a_value_handler_contract() {
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let tokens = f5_source_tokens("string cat a b");
    assert!(normal_representation_invocation(registry, None, &tokens).is_none());
}

#[test]
fn unavailable_f5_binary_member_cannot_donate_a_value_handler_contract() {
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let tokens = f5_source_tokens("binary encode hex DATA");
    assert!(normal_representation_invocation(registry, None, &tokens).is_none());
}

#[test]
fn normal_assignment_keeps_value_words_without_an_opcode_or_custom_handler_proof() {
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let tokens = f5_source_tokens("set p [TCP::payload]");
    let normal = normal_representation_invocation(registry, None, &tokens).unwrap();
    let assignment = normal.value_assignment().unwrap();
    assert_eq!(assignment.name, "p");
    assert_eq!(assignment.value, tokens.words()[2]);
    assert!(resolved_tokens_invocation(registry, None, &tokens).is_none());
    for source in ["set p", "proc set args {return CUSTOM}; set p value"] {
        let tokens = f5_source_tokens(source);
        assert!(
            normal_representation_invocation(registry, None, &tokens)
                .and_then(|normal| normal.value_assignment())
                .is_none()
        );
    }
}

#[test]
fn representation_contract_declines_replaced_and_unproved_handlers() {
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    for source in [
        "proc llength args {return CUSTOM}; llength {a b}",
        "set selected mystery; $selected {a b}",
    ] {
        let tokens = f5_source_tokens(source);
        assert!(normal_representation_invocation(registry, None, &tokens).is_none());
    }
}

#[test]
fn normal_taint_source_properties_require_the_selected_getter() {
    use tcl_registry::taint::TaintColour;
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let getter = f5_source_tokens("HTTP::uri");
    let source = normal_taint_invocation(registry, None, &getter).unwrap();
    let colour = source.source_colour().unwrap();
    assert!(colour.contains(TaintColour::TAINTED | TaintColour::PATH_PREFIXED));
    assert!(resolved_tokens_invocation(registry, None, &getter).is_none());

    let setter = f5_source_tokens("HTTP::uri /changed");
    let colour = normal_taint_invocation(registry, None, &setter)
        .unwrap()
        .source_colour()
        .unwrap();
    assert!(colour.contains(TaintColour::TAINTED));
    assert!(!colour.contains(TaintColour::PATH_PREFIXED));
}

#[test]
fn compiled_dynamic_regexp_pattern_retains_its_original_operand_slot() {
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let registry = tcl_registry::model::ingress::static_context_for(profile).commands();
        let cu = crate::compilation_unit::CompilationUnit::build_for(
            "proc p {pattern subject} {regexp $pattern $subject}",
            registry,
            false,
        );
        let statement = &cu
            .ir_module
            .procedures
            .get(&fixture_id(&cu.ir_module.procedures, "::p"))
            .expect("authored fixture body")
            .body
            .statements[0];
        let tokens = statement.tokens().unwrap();
        let normal = normal_representation_invocation(registry, None, tokens).unwrap();
        assert_eq!(
            normal.pattern_source_argument_index(registry),
            (profile != "tcl8.4").then_some(0),
            "{profile}"
        );
    }
    let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
    let tokens = last_source_tokens("regexp -about aa");
    assert_eq!(
        normal_representation_invocation(registry, None, &tokens)
            .unwrap()
            .pattern_source_argument_index(registry),
        Some(1)
    );
}

#[test]
fn normal_pattern_layout_uses_closed_original_read_values() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
    for (source, expected) in [
        (
            "proc p {outer inner s} {set re seed; if {$outer} {if {$inner} {set re left}}; regsub -command $re $s callback out}",
            Some(1),
        ),
        (
            "proc p {flag s} {set re seed; if {$flag} {set re -command}; regsub $re $s callback out}",
            None,
        ),
        ("proc p {re s} {regsub -command $re $s callback out}", None),
        (
            "proc p {flag s} {set re seed; if {$flag} {set re left}; proc observer args {}; trace add variable re read observer; regsub -command $re $s callback out}",
            None,
        ),
    ] {
        let cu = crate::compilation_unit::CompilationUnit::build_for(source, registry, false);
        let statement = cu
            .ir_module
            .procedures
            .get(&fixture_id(&cu.ir_module.procedures, "::p"))
            .expect("authored fixture body")
            .body
            .statements
            .last()
            .unwrap();
        let normal = normal_representation_invocation(registry, None, statement.tokens().unwrap());
        assert_eq!(
            normal
                .as_ref()
                .and_then(|normal| normal.pattern_source_argument_index(registry)),
            expected,
            "{source}"
        );
        if let Some(normal) = normal
            && matches!(normal.proof, NormalRepresentationProof::PatternLayout(_))
        {
            assert!(normal.argument_type_hint(1).is_none());
            assert_eq!(normal.operand_roles(), []);
            assert_eq!(
                normal.representation_effect(),
                tcl_registry::RepresentationEffect::None
            );
            assert!(normal.value_assignment().is_none());
        }
    }
}

#[test]
fn normal_taint_numeric_results_do_not_require_opcode_proof() {
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let tokens = f5_source_tokens("string length VALUE");
    assert!(resolved_tokens_invocation(registry, None, &tokens).is_none());
    assert!(
        normal_taint_invocation(registry, None, &tokens)
            .unwrap()
            .is_sanitiser()
    );
    let replaced = f5_source_tokens("proc string args {return CUSTOM}; string length VALUE");
    assert!(normal_taint_invocation(registry, None, &replaced).is_none());
}

#[test]
fn normal_taint_transform_requires_frozen_mapping_values() {
    use tcl_registry::taint::TaintColour;
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    for source in [
        r#"string map {"\r" "" "\n" ""} VALUE"#,
        r#"set mapping {"\r" "" "\n" ""}; string map -nocase $mapping VALUE"#,
    ] {
        let tokens = f5_source_tokens(source);
        assert!(resolved_tokens_invocation(registry, None, &tokens).is_none());
        assert_eq!(
            normal_taint_invocation(registry, None, &tokens)
                .unwrap()
                .transform_colour(),
            Some(TaintColour::CRLF_FREE),
            "{source}"
        );
    }
    for source in [
        r#"set mapping [HTTP::uri]; string map $mapping {"\r" "" "\n" ""}"#,
        r#"string map {"\r" "" "\n" "x\ny"} VALUE"#,
        r#"string map {"\r" "" "\n"} VALUE"#,
    ] {
        let tokens = f5_source_tokens(source);
        assert!(
            normal_taint_invocation(registry, None, &tokens)
                .and_then(|normal| normal.transform_colour())
                .is_none(),
            "{source}"
        );
    }
}

#[test]
fn normal_taint_source_properties_decline_replaced_or_unproved_handlers() {
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    for source in [
        "proc HTTP::uri args {return CUSTOM}; HTTP::uri",
        "set selected mystery; $selected",
    ] {
        let tokens = f5_source_tokens(source);
        assert!(normal_taint_invocation(registry, None, &tokens).is_none());
    }
    let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
    let tokens = last_source_tokens("HTTP::uri");
    assert!(normal_taint_invocation(registry, None, &tokens).is_none());
}

#[test]
fn normal_handler_transfer_preserves_unknown_compiler_protocol() {
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let tokens = f5_source_tokens("set connection_value 17");
    assert!(
        tokens
            .source_binding
            .as_ref()
            .unwrap()
            .compiled_execution_unknown()
    );
    assert!(resolved_tokens_invocation(registry, None, &tokens).is_none());
    let normal = normal_transfer_invocation(registry, None, &tokens).unwrap();
    assert_eq!(normal.definition_names(), ["connection_value"]);
    assert_eq!(normal.written_argument(1), Some(1));
    let selected = &tokens.source_binding.as_ref().unwrap().variable_context;
    assert_eq!(
        normal.stored_value_literal(selected, registry).as_deref(),
        Some("17")
    );
    let mut observed = selected.as_ref().clone();
    observed.dynamic_traces = true;
    assert!(normal.stored_value_literal(&observed, registry).is_none());
    let statement = crate::ir::Statement::Call {
        span: tokens.words().first().unwrap().source().span,
        command: "set".into(),
        canonical_command: None,
        args: vec!["connection_value".into(), "17".into()],
        defs: Vec::new(),
        reads: Vec::new(),
        reads_own_defs: false,
        safe_on_uninit: false,
        foreach_groups: None,
        tokens: Some(tokens),
    };
    assert_eq!(
        crate::ssa::defs_of_with_registry(&statement, Some(registry)),
        ["connection_value"]
    );
    assert!(resolved_statement_invocation(registry, None, &statement).is_none());
}

#[test]
fn normal_store_uses_frozen_target_and_post_argv_observers() {
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let tokens = f5_source_tokens("set n x; set x OLD; set $n [set n NEW]");
    let normal = normal_transfer_invocation(registry, None, &tokens).unwrap();
    let selected = &tokens.source_binding.as_ref().unwrap().variable_context;
    assert_eq!(normal.definition_names(), ["x"]);
    assert_eq!(
        normal.stored_value_literal(selected, registry).as_deref(),
        Some("NEW")
    );
    assert!(resolved_tokens_invocation(registry, None, &tokens).is_none());

    let source = "proc observer {args} {set ::x CHANGED}; set x OLD; set x [trace add variable x write observer]";
    let tokens = f5_source_tokens(source);
    assert!(normal_transfer_invocation(registry, None, &tokens).is_none());
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let tokens = last_source_tokens(source);
    let normal = normal_transfer_invocation(registry, None, &tokens).unwrap();
    let selected = &tokens.source_binding.as_ref().unwrap().variable_context;
    assert!(normal.stored_value_operand(selected, registry).is_none());
    assert!(normal.stored_value_literal(selected, registry).is_none());
    assert!(resolved_tokens_invocation(registry, None, &tokens).is_some());
}

#[test]
fn normal_store_resolves_actual_alias_destination_after_arguments() {
    let source = "set x OLD; set y NEW; upvar 0 x a; set a [upvar 0 y a]";
    for dialect in ["f5-irules", "tcl8.6"] {
        let registry = tcl_registry::model::ingress::static_context_for(dialect).commands();
        let tokens = if dialect == "f5-irules" {
            f5_source_tokens(source)
        } else {
            last_source_tokens(source)
        };
        let normal = normal_transfer_invocation(registry, None, &tokens).unwrap();
        let selected = &tokens.source_binding.as_ref().unwrap().variable_context;
        let places = normal.mutation_places(selected, registry);
        assert_eq!(places.len(), 1);
        let target = crate::var_resolve::resolve_place("y", selected, false, registry);
        assert_eq!(places[0].kind, crate::place::PlaceKind::Scalar);
        assert!(!places[0].observed && !places[0].dynamic);
        assert!(places[0].cell.is_some());
        assert_eq!(places[0].cell, target.cell);
        assert_eq!(
            crate::var_resolve::canonical_binding_value_name(&places[0]),
            Some("::y".to_owned())
        );
        assert_eq!(
            normal.stored_value_literal(selected, registry).as_deref(),
            Some("")
        );
        assert_eq!(
            resolved_tokens_invocation(registry, None, &tokens).is_some(),
            dialect == "tcl8.6"
        );
    }
}

#[test]
fn normal_handler_transfer_rejects_mutation_and_body_licences() {
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    for source in [
        "set x [proc set args {return CUSTOM}]",
        "if {$flag} {set x 1}",
        "proc set args {return CUSTOM}; set x 1",
        "set x 1 extra",
    ] {
        let tokens = f5_source_tokens(source);
        assert!(
            normal_transfer_invocation(registry, None, &tokens).is_none(),
            "{source}"
        );
    }
}

#[test]
fn argument_values_are_frozen_before_later_substitutions() {
    let tokens = last_source_tokens("set v OLD; list $v [set v NEW]");
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let invocation = resolved_tokens_invocation(registry, Some(test_context()), &tokens).unwrap();
    assert_eq!(invocation.argument_literal(0).as_deref(), Some("OLD"));
    assert_eq!(invocation.arguments[0].as_deref(), Some("${v}"));
    assert!(matches!(
        invocation.effective.words[1],
        WordExpr::Variable { .. }
    ));
    assert_eq!(
        invocation.effective.origins[1],
        InvocationWordOrigin::Written(1)
    );
    assert_eq!(
        tokens
            .source_binding
            .as_ref()
            .unwrap()
            .variable_context
            .substitution_literal_value("$v", registry),
        Some("NEW")
    );
}

#[test]
fn possible_registry_shapes_do_not_establish_execution_facts() {
    let tokens = last_source_tokens("if {$flag} {proc set args {return CUSTOM}}; set x 1");
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    assert!(resolved_tokens_invocation(registry, Some(test_context()), &tokens).is_none());
    let assistance =
        registry_invocation_assistance(registry, Some(test_context()), &tokens).unwrap();
    assert!(assistance.unknown_residual);
    assert!(
        assistance
            .candidates
            .iter()
            .any(|candidate| candidate.command == "set")
    );
    assert_eq!(
        assistance
            .candidates
            .iter()
            .find(|candidate| candidate.command == "set")
            .unwrap()
            .effective
            .written_argument(0),
        Some(0)
    );
}

#[test]
fn unanimous_advisory_words_keep_all_lookup_residuals() {
    let tokens = last_source_tokens("set target VALUE");
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let original = registry_invocation_assistance(registry, Some(test_context()), &tokens)
        .expect("stock command has advisory candidates");
    let first = original
        .candidates
        .first()
        .expect("stock candidate")
        .clone();
    let mut assistance = RegistryInvocationAssistance {
        candidates: vec![first.clone(), first.clone()],
        unknown_residual: false,
        may_be_absent: false,
    };
    let view = assistance
        .unanimous_command_words()
        .expect("unanimous words");
    assert_eq!(view.command(), first.command);
    assert_eq!(view.effective(), &first.effective);
    // Role agreement is a separate purpose and cannot be borrowed here.
    assistance.candidates[1].roles.clear();
    assistance.candidates[1].roles_complete = false;
    assert!(assistance.unanimous_command_words().is_some());
    for (unknown_residual, may_be_absent) in [(true, false), (false, true), (true, true)] {
        assistance.unknown_residual = unknown_residual;
        assistance.may_be_absent = may_be_absent;
        assert!(assistance.unanimous_command_words().is_none());
    }
    assistance.unknown_residual = false;
    assistance.may_be_absent = false;
    assistance.candidates.clear();
    assert!(assistance.unanimous_command_words().is_none());
    for malformed in [
        EffectiveCommandWords {
            words: Vec::new(),
            ..first.effective.clone()
        },
        EffectiveCommandWords {
            origins: Vec::new(),
            ..first.effective.clone()
        },
        EffectiveCommandWords {
            origins: vec![InvocationWordOrigin::BindingPrefix(0); first.effective.words.len()],
            binding_prefix: Vec::new(),
            ..first.effective.clone()
        },
    ] {
        assistance.candidates.push(RegistryInvocationShape {
            effective: malformed,
            ..first.clone()
        });
        assert!(assistance.unanimous_command_words().is_none());
        assistance.candidates.clear();
    }
}

#[test]
fn unanimous_advisory_words_require_identity_values_and_origins() {
    let tokens = last_source_tokens("set target VALUE");
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let original = registry_invocation_assistance(registry, Some(test_context()), &tokens)
        .expect("stock command has advisory candidates");
    let first = original
        .candidates
        .first()
        .expect("stock candidate")
        .clone();
    let mut alternatives = Vec::new();
    let mut renamed = first.clone();
    renamed.command.push_str("_other");
    alternatives.push(renamed);
    let mut changed_word = first.clone();
    changed_word.effective.words[1] = literal("other_target");
    alternatives.push(changed_word);
    let mut changed_origin = first.clone();
    changed_origin.effective.origins[1] = InvocationWordOrigin::BindingPrefix(0);
    alternatives.push(changed_origin);
    let mut changed_prefix = first.clone();
    changed_prefix
        .effective
        .binding_prefix
        .push(EffectiveInvocationWord::Dynamic);
    alternatives.push(changed_prefix);
    for alternative in alternatives {
        let assistance = RegistryInvocationAssistance {
            candidates: vec![first.clone(), alternative],
            unknown_residual: false,
            may_be_absent: false,
        };
        assert!(assistance.unanimous_command_words().is_none());
    }
}

#[test]
fn role_assistance_maps_alias_prefixes_to_only_original_operands() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let surface = tcl_registry::model::DocumentCommandSurface::new(registry, None);
    for (source, expected) in [
        (
            "proc p {} {}; info body p",
            vec![(1, tcl_registry::ArgRole::CommandName)],
        ),
        (
            "proc p {} {}; interp alias {} inspect {} info; inspect body p",
            vec![(1, tcl_registry::ArgRole::CommandName)],
        ),
        (
            "proc p {} {}; interp alias {} inspect {} info body; inspect p",
            vec![(0, tcl_registry::ArgRole::CommandName)],
        ),
        (
            "proc p {} {}; interp alias {} inspect {} info body p; inspect",
            vec![],
        ),
        (
            "interp alias {} declare {} proc p; declare {x} {}",
            vec![
                (0, tcl_registry::ArgRole::ParamList),
                (1, tcl_registry::ArgRole::Body),
            ],
        ),
        (
            "interp alias {} declare {} proc p {x}; declare {}",
            vec![(0, tcl_registry::ArgRole::Body)],
        ),
        (
            "interp alias {} inspect {} info body; rename inspect {}; proc inspect args {}; inspect p",
            vec![],
        ),
    ] {
        let tokens = last_source_tokens(source);
        let roles =
            invocation_argument_role_assistance(registry, test_context(), &surface, &tokens);
        assert_eq!(roles, expected, "{source}");
    }
}

#[test]
fn body_assistance_keeps_navigation_separate_from_unanimous_context() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let surface = tcl_registry::model::DocumentCommandSurface::new(registry, None);
    let tokens = last_source_tokens("interp alias {} declare {} proc p {x}; declare {}");
    let view = invocation_body_assistance(registry, test_context(), &surface, &tokens);
    assert!(
        view.possible_roles
            .contains(&(0, tcl_registry::ArgRole::Body))
    );
    assert!(
        view.definite_roles
            .contains(&(0, tcl_registry::ArgRole::Body))
    );
    assert!(
        !view
            .possible_roles
            .iter()
            .any(|(_, role)| *role == tcl_registry::ArgRole::ParamList)
    );

    let tokens = last_source_tokens(
        "if {$flag} {rename if original; proc if args {return ordinary}}; if {$other} {set x 1}",
    );
    let view = invocation_body_assistance(registry, test_context(), &surface, &tokens);
    assert!(
        view.possible_roles
            .contains(&(1, tcl_registry::ArgRole::Body))
    );
    assert!(view.unknown_residual);
    assert_eq!(
        view.definite_roles,
        [] as [(usize, tcl_registry::ArgRole); 0]
    );
    assert!(view.definite_traits.is_empty());
    assert!(view.definite_scope.is_none());

    let tokens = last_source_tokens(
        "interp alias {} declare {} proc p; rename declare {}; proc declare args {return ordinary}; declare {} {}",
    );
    let view = invocation_body_assistance(registry, test_context(), &surface, &tokens);
    assert_eq!(
        view.possible_roles,
        [] as [(usize, tcl_registry::ArgRole); 0]
    );

    let tokens = last_source_tokens(
        "rename if original; proc if args {return ordinary}; if {$other} {set x 1}",
    );
    assert!(catalogue_invocation_assistance(registry, test_context(), &tokens).is_none());
    let view = invocation_body_assistance(registry, test_context(), &surface, &tokens);
    assert_eq!(
        view.possible_roles,
        [] as [(usize, tcl_registry::ArgRole); 0]
    );
    assert_eq!(
        view.definite_roles,
        [] as [(usize, tcl_registry::ArgRole); 0]
    );
    assert!(view.definite_invocation.is_none());

    let tokens = last_source_tokens(
        "if {$flag} {rename proc original; original proc args {return ordinary}}; proc p {x} {}",
    );
    assert!(catalogue_invocation_assistance(registry, test_context(), &tokens).is_none());
    let view = invocation_body_assistance(registry, test_context(), &surface, &tokens);
    assert!(
        view.possible_roles
            .contains(&(2, tcl_registry::ArgRole::Body))
    );
    assert_eq!(
        view.definite_roles,
        [] as [(usize, tcl_registry::ArgRole); 0]
    );
    assert!(view.definite_invocation.is_none());
}

#[test]
fn body_assistance_does_not_promote_authoring_facts_without_an_actual_axis() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let surface = tcl_registry::model::DocumentCommandSurface::new(registry, None);
    let original = last_source_tokens("proc p {x} {}");
    let actual = invocation_body_assistance(registry, test_context(), &surface, &original);
    assert!(actual.definite_invocation.is_some());
    let mut binding = original.source_binding.as_ref().unwrap().clone();
    std::sync::Arc::make_mut(&mut binding.variable_context).invocation_dialect = None;
    let tokens = original.with_source_binding(binding);
    let advisory = invocation_body_assistance(registry, test_context(), &surface, &tokens);
    assert!(
        advisory
            .possible_roles
            .contains(&(2, tcl_registry::ArgRole::Body))
    );
    assert_eq!(
        advisory.definite_roles,
        [] as [(usize, tcl_registry::ArgRole); 0]
    );
    assert!(advisory.definite_traits.is_empty());
    assert!(advisory.definite_invocation.is_none());
    assert!(advisory.conditional_invocation.is_none());
}

#[test]
fn prepared_body_views_preserve_residual_and_selected_projection_boundaries() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let surface = tcl_registry::model::DocumentCommandSurface::new(registry, None);
    for source in [
        "proc p {x} {}",
        "interp alias {} declare {} proc p; declare {x} {}",
        "interp alias {} declare {} proc p {x}; declare {}",
        "proc p {} {}; interp alias {} inspect {} info body; inspect p",
        "if {$flag} {rename proc original; original proc args {return ordinary}}; proc p {x} {}",
        "rename proc original; original proc args {return ordinary}; proc p {x} {}",
        "proc p {*}$unknown",
        "external p {x} {}",
    ] {
        let tokens = last_source_tokens(source);
        let view = invocation_body_assistance(registry, test_context(), &surface, &tokens);
        assert_eq!(
            view.possible_roles,
            invocation_argument_role_assistance(registry, test_context(), &surface, &tokens),
            "possible original roles: {source}"
        );
        assert_eq!(
            view.definite_roles,
            invocation_argument_role_consensus(registry, test_context(), &tokens),
            "unanimous original roles: {source}"
        );
        let selected = resolved_tokens_invocation(registry, Some(test_context()), &tokens)
            .filter(|_| !view.unknown_residual && !view.may_be_absent)
            .filter(|invocation| {
                invocation.facts.arg_roles_complete
                    && invocation.facts.arity_accepts_frozen_arguments() == Some(true)
            });
        assert_eq!(
            view.definite_invocation, selected,
            "selected implementation: {source}"
        );
        if view.unknown_residual || view.may_be_absent {
            assert!(view.definite_roles.is_empty(), "{source}");
            assert!(view.definite_traits.is_empty(), "{source}");
            assert!(view.definite_scope.is_none(), "{source}");
        }
    }
}

#[test]
fn packaged_body_assistance_keeps_availability_separate_from_native_entry() {
    let registry = tcl_registry::model::ingress::static_context_for("tk").commands();
    let surface = tcl_registry::model::DocumentCommandSurface::new(registry, None);
    let source = "bind .widget click {return event}";
    let config = tcl_lexer::LexerConfig::for_dialect("tk");
    let bindings = crate::command_binding::SourceCommandBindings::analyse(source, config, registry);
    let command = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
        .pop()
        .unwrap();
    let mut tokens =
        CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(source), config, &command);
    bindings.stamp_original_tokens(&mut tokens);
    let context = body_assistance_context(registry, &tokens).unwrap();
    assert_eq!(
        context,
        SemanticContext::for_profile(registry.profile().unwrap())
    );
    assert!(!context.context().authoring_query().packages.is_empty());
    let view =
        segmented_body_assistance(&surface, &bindings, source, config, &command, 0, None).unwrap();
    assert!(
        view.possible_roles
            .contains(&(2, tcl_registry::ArgRole::Body))
    );
    let opaque =
        tokens.with_source_binding(crate::command_binding::SourceInvocationBinding::unknown());
    let view = invocation_body_assistance(registry, context, &surface, &opaque);
    assert_eq!(
        view.definite_roles,
        [] as [(usize, tcl_registry::ArgRole); 0]
    );
    assert!(view.definite_traits.is_empty());
    assert!(view.definite_scope.is_none());
    assert!(view.definite_invocation.is_none());
}

#[test]
fn unprofiled_body_assistance_uses_retained_entry_without_definite_offset_free_facts() {
    let registry = CommandRegistry::build_default();
    assert!(registry.profile().is_none());
    let surface = tcl_registry::model::DocumentCommandSurface::new(&registry, None);
    let source = "eval {$script}";
    let config = tcl_lexer::LexerConfig::for_dialect("tcl9.0");
    let bindings =
        crate::command_binding::SourceCommandBindings::analyse(source, config, &registry);
    let command = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
        .pop()
        .unwrap();
    let view =
        segmented_body_assistance(&surface, &bindings, source, config, &command, 0, None).unwrap();
    assert!(
        view.possible_roles
            .contains(&(0, tcl_registry::ArgRole::Body))
    );
    assert!(
        view.possible_traits
            .contains(tcl_registry::Traits::DYNAMIC_EVAL_BODY)
    );
    let view =
        unpositioned_body_assistance(&surface, &bindings, source, config, &command, None).unwrap();
    assert!(
        view.possible_roles
            .contains(&(0, tcl_registry::ArgRole::Body))
    );
    assert_eq!(
        view.definite_roles,
        [] as [(usize, tcl_registry::ArgRole); 0]
    );
    assert!(view.definite_traits.is_empty());
    assert!(view.definite_invocation.is_none());
    let unknown = crate::command_binding::SourceCommandBindings::analyse_with_options(
        source,
        config,
        &registry,
        crate::command_binding::SourceAnalysisOptions::default(),
    );
    assert!(
        segmented_body_assistance(&surface, &unknown, source, config, &command, 0, None).is_none()
    );
    assert!(
        unpositioned_body_assistance(&surface, &unknown, source, config, &command, None).is_none()
    );
}

#[test]
fn definite_roles_require_closed_actual_alternatives_and_original_positions() {
    use tcl_registry::ArgRole::{Body, Name, ParamList};
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    for (source, expected) in [
        ("proc p {x} {}", vec![(0, Name), (1, ParamList), (2, Body)]),
        (
            "interp alias {} declare {} proc p; declare {x} {}",
            vec![(0, ParamList), (1, Body)],
        ),
        (
            "interp alias {} declare {} proc p {x}; declare {}",
            vec![(0, Body)],
        ),
        (
            "rename proc original; original proc args {return ordinary}; proc p {{a b c}} {}",
            vec![],
        ),
        (
            "if {$unknown} {rename proc original; original proc args {return ordinary}}; proc p {{a b c}} {}",
            vec![],
        ),
        ("external p {{a b c}} {}", vec![]),
        ("proc p {*}$unknown", vec![]),
    ] {
        let tokens = last_source_tokens(source);
        assert_eq!(
            invocation_argument_role_consensus(registry, test_context(), &tokens),
            expected,
            "{source}"
        );
    }
    let opaque = last_source_tokens("proc p {{a b c}} {}")
        .with_source_binding(crate::command_binding::SourceInvocationBinding::unknown());
    assert_eq!(
        invocation_argument_role_consensus(registry, test_context(), &opaque),
        vec![]
    );
}

#[test]
fn unloaded_catalogue_assistance_never_restores_a_live_implementation() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let tokens = last_source_tokens("::struct::tree tree");
    assert!(resolved_tokens_invocation(registry, Some(test_context()), &tokens).is_none());
    let assistance = catalogue_invocation_assistance(registry, test_context(), &tokens)
        .expect("catalogue metadata at an unchanged qualified slot");
    assert_eq!(assistance.shape.command, "struct::tree");
    let shadowed = last_source_tokens(
        "namespace eval ::struct {}; proc ::struct::tree args {return ordinary}; ::struct::tree tree",
    );
    assert!(catalogue_invocation_assistance(registry, test_context(), &shadowed).is_none());
    let unregistered = last_source_tokens("::external::factory tree");
    assert!(catalogue_invocation_assistance(registry, test_context(), &unregistered).is_none());
}

#[test]
fn explicit_point_uncertainty_never_recovers_written_builtin_facts() {
    let registry = CommandRegistry::build_default();
    let command = segment_commands("set x 1").remove(0);
    let tokens = CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::new("set x 1"),
        tcl_lexer::LexerConfig::default(),
        &command,
    )
    .with_source_binding(crate::command_binding::SourceInvocationBinding::unknown());
    assert!(effective_command_words(&tokens).is_none());
    assert!(matches!(
        resolve_command_tokens(&registry, Some(test_context()), &tokens),
        Ok(RegistryInvocationResolution::Unresolved(
            OwnedInvocationResolutionUnresolved::UnprovedBinding { unknown: true, .. }
        ))
    ));
}

#[test]
fn declared_roles_remain_separate_from_runtime_semantics() {
    use tcl_registry::model::DocumentCommandSurface;
    let registry = CommandRegistry::build_default();
    let config = tcl_lexer::LexerConfig::default();
    let source = "# tcl-lsp: stubs-begin\n# tcl-lsp: stub custom {destination:var}\n# tcl-lsp: stubs-end\ncustom result";
    let declared = crate::analyser::utils::document_declared_surface(source, None, "tcl8.6");
    let bindings = crate::command_binding::SourceCommandBindings::analyse_in_namespace_with_options(
        source,
        "::",
        config,
        &registry,
        crate::command_binding::SourceAnalysisOptions {
            declared_commands: Some(&declared),
            ..Default::default()
        },
    );
    let segment = segment_commands(source).remove(0);
    let tokens =
        CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(source), config, &segment)
            .with_source_binding(bindings.invocation_at_source("custom", segment.span.start()));
    let surface = DocumentCommandSurface::new(&registry, Some(&declared));
    let assistance =
        resolved_declared_assistance(&surface, &tokens).expect("untouched declared lookup slot");
    assert_eq!(assistance.roles, [(0, tcl_registry::ArgRole::VarWrite)]);
    assert!(matches!(
        resolve_command_tokens(&registry, None, &tokens),
        Ok(RegistryInvocationResolution::Unresolved(_))
    ));
    assert!(resolved_tokens_invocation(&registry, None, &tokens).is_none());
}

#[test]
fn nested_dispatch_inheritance_never_recovers_a_missing_point() {
    let source = "set r [expr {1 + 2}]";
    let config = tcl_lexer::LexerConfig::default();
    let segment = segment_commands(source).remove(0);
    let parent =
        CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(source), config, &segment)
            .with_source_binding(crate::command_binding::SourceInvocationBinding::unknown());
    let word = &parent.words()[2];
    let mut nested =
        crate::word_subst::nested_command_words(&word.legacy_text(), word.source(), config)
            .unwrap();
    nested.inherit_nested_bindings(&parent);
    assert!(nested.has_unproved_source_binding());
    assert!(
        resolved_tokens_invocation(
            &CommandRegistry::build_default(),
            Some(test_context()),
            &nested
        )
        .is_none()
    );
}

#[test]
fn alias_composition_preserves_original_evaluation_and_inserted_origins() {
    let registry = CommandRegistry::build_default();
    let module = crate::lowering::lower_to_ir(
        "interp alias {} inner {} list second\ninterp alias {} outer {} inner first\nouter $value",
        &registry,
    );
    let tokens = module
        .top_level
        .statements
        .last()
        .unwrap()
        .tokens()
        .unwrap();
    let effective = effective_command_words(tokens).expect("alias has a unique target");
    assert_eq!(tokens.argv_texts, ["outer", "${value}"]);
    assert_eq!(
        effective
            .argument_spellings(&tokens.argv_texts[1..])
            .unwrap(),
        ["second", "first", "${value}"]
    );
    assert_eq!(
        effective.origins,
        [
            InvocationWordOrigin::ResolvedHead,
            InvocationWordOrigin::BindingPrefix(0),
            InvocationWordOrigin::BindingPrefix(1),
            InvocationWordOrigin::Written(1)
        ]
    );
    assert_eq!(effective.written_argument(0), None);
    assert_eq!(effective.written_argument(2), Some(0));
    assert!(matches!(
        effective.words.last(),
        Some(WordExpr::Variable { .. })
    ));
    assert!(matches!(
        effective.words[1].source().provenance,
        crate::ir::Provenance::Opaque
    ));
}

#[test]
fn document_target_composes_arguments_without_registry_semantics() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let source = "proc custom {prefix value} {return $value}\ninterp alias {} invoke {} custom FIXED\ninvoke $value";
    let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
    let bindings = crate::command_binding::SourceCommandBindings::analyse(source, config, registry);
    let segment = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
        .pop()
        .unwrap();
    let tokens =
        CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(source), config, &segment)
            .with_source_binding(bindings.invocation_at_source("invoke", segment.span.start()));
    let effective =
        effective_command_words(&tokens).expect("unique document procedure through alias");
    assert_eq!(
        tokens
            .source_binding
            .as_ref()
            .unwrap()
            .proved_target()
            .unwrap()
            .command,
        "::custom"
    );
    assert_eq!(
        effective
            .argument_spellings(&tokens.argv_texts[1..])
            .unwrap(),
        ["FIXED", "${value}"]
    );
    assert_eq!(effective.written_argument(0), None);
    assert_eq!(effective.written_argument(1), Some(0));
    assert!(
        resolved_tokens_invocation(registry, Some(test_context()), &tokens).is_none(),
        "composing a document callee is not registry implementation proof"
    );
}

#[test]
fn captured_unknown_prefix_preserves_count_without_literal_presentation() {
    let source = "alias written";
    let config = tcl_lexer::LexerConfig::default();
    let segment = segment_commands(source).remove(0);
    let tokens =
        CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(source), config, &segment);
    let effective = compose_effective_words(
        &tokens,
        "list",
        &[
            EffectiveInvocationWord::Dynamic,
            EffectiveInvocationWord::Literal("known".to_owned()),
        ],
    )
    .unwrap();
    assert_eq!(
        effective.argument_presentations(&tokens.argv_texts[1..]),
        Some(vec![
            None,
            Some("known".to_owned()),
            Some("written".to_owned())
        ])
    );
    assert!(
        effective
            .argument_spellings(&tokens.argv_texts[1..])
            .is_none()
    );
    let words = frozen_argument_words(&tokens, &effective);
    assert_eq!(words[0], EffectiveInvocationWord::Dynamic);
    assert_eq!(words.len(), 3);
}

#[test]
fn effective_argument_projection_declines_inconsistent_source_ownership() {
    let projection = EffectiveCommandWords {
        words: vec![literal("list"), literal("arg")],
        origins: vec![InvocationWordOrigin::Written(0)],
        binding_prefix: Vec::new(),
    };
    assert_eq!(projection.argument_spellings(&["arg".into()]), None);
    let projection = EffectiveCommandWords {
        words: vec![literal("list"), literal("arg")],
        origins: vec![
            InvocationWordOrigin::Written(0),
            InvocationWordOrigin::Written(1),
        ],
        binding_prefix: Vec::new(),
    };
    assert_eq!(projection.argument_spellings(&[]), None);
    assert_eq!(
        projection.argument_spellings(&["arg".into()]),
        Some(vec!["arg".into()])
    );
}

#[test]
fn literal_words_materialise_registry_transition_facts() {
    let words = [literal("global"), literal("::shared")];
    let resolution = resolve_word_exprs(
        &CommandRegistry::build_default(),
        Some(test_context()),
        &words,
    )
    .expect("literal command has a registry outcome");

    let RegistryInvocationResolution::Resolved(facts) = resolution else {
        panic!("literal global must resolve");
    };
    let transitions = facts
        .state_transitions
        .declared()
        .expect("global declares transition facts");
    assert!(matches!(
        transitions.facts(),
        [fact]
            if matches!(
                &fact.transition,
                StateTransition::VariableCellAlias(alias)
                    if alias.local.literal() == Some("shared")
                        && alias.local.argument_index() == Some(0)
            )
    ));
}

#[test]
fn segmented_arguments_preserve_a_static_non_option_template_prefix() {
    let command = segment_commands("regexp \"abc$part.*\" $subject\n")
        .into_iter()
        .next()
        .expect("one command");
    assert_eq!(
        segmented_command_arguments(&command),
        vec![InvocationWord::DynamicNonOption, InvocationWord::Dynamic],
        "the first template remains dynamic but cannot be a leading option"
    );
}

#[test]
fn compiler_word_shape_preserves_quoted_simple_words_without_decoding_escapes() {
    use tcl_registry::native_compilation::NativeCompilationWordShape as Shape;

    for (source, expected) in [
        ("list plain", Shape::Literal),
        ("list \"plain\"", Shape::QuotedLiteral),
        ("list \"\"", Shape::QuotedLiteral),
        ("list {plain}", Shape::BracedLiteral),
        (r"list \p\lain", Shape::BackslashLiteral),
        (r#"list "\160lain""#, Shape::BackslashLiteral),
        ("list \"$value\"", Shape::Substituted),
    ] {
        let segment = segment_commands(source).pop().unwrap();
        let tokens = CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(source),
            tcl_lexer::LexerConfig::default(),
            &segment,
        );
        assert_eq!(
            native_compilation_word_shape(&tokens.words()[1]),
            expected,
            "{source}"
        );
    }
}

#[test]
fn effective_words_keep_braces_and_decode_bare_escapes() {
    let source = SourceSite::source(tcl_lexer::Span::new(0, 0));
    assert_eq!(
        effective_invocation_word(
            &WordExpr::Template {
                parts: vec![WordPart::Text {
                    text: r"\x32".to_owned(),
                    source: source.clone()
                }],
                source: source.clone(),
                rejected: None,
            },
            EscapeSyntax::Tcl86,
            WordValueRules::TCL,
        ),
        EffectiveInvocationWord::Literal("2".to_owned())
    );
    assert_eq!(
        effective_invocation_word(
            &WordExpr::BracedLiteral {
                text: r"\x32".to_owned(),
                source
            },
            EscapeSyntax::Tcl86,
            WordValueRules::TCL,
        ),
        EffectiveInvocationWord::Literal(r"\x32".to_owned())
    );
}

#[test]
fn compound_array_word_facts_require_the_original_configured_operand() {
    let policy =
        tcl_syntax::naming::NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_6);
    let config = tcl_lexer::LexerConfig::default();
    for (source, expected) in [
        ("set array($index) VALUE", Some("array")),
        ("set \"array($index)\" VALUE", Some("array")),
        (r"set array\($index) VALUE", None),
        ("set $root($index) VALUE", None),
        ("set array($index)suffix VALUE", None),
    ] {
        let image = tcl_lexer::SourceImage::document(source);
        let plan = tcl_lexer::native_script_words_in(
            image.clone(),
            tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
            config,
        )
        .unwrap();
        let original = &plan.commands[0].words[1];
        assert_eq!(
            original_array_element_word(original, policy),
            expected.map(|root| EffectiveInvocationWord::ArrayElementName {
                root: root.to_owned()
            }),
            "{source}"
        );
        let segment = segment_commands(source).pop().unwrap();
        let tokens = CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::from_image(&image),
            config,
            &segment,
        );
        assert_eq!(
            effective_invocation_word(
                &tokens.words()[1],
                config.escapes,
                WordValueRules::from_config(&config)
            ),
            EffectiveInvocationWord::Dynamic,
            "a representative template has no original operand owner: {source}"
        );
    }
    let source = "set array($index) VALUE";
    let image = tcl_lexer::SourceImage::document(source);
    let config = tcl_lexer::LexerConfig::from_grammar(
        tcl_dialect::DialectProfile::find("tcl9.0").unwrap().grammar,
    );
    let plan = tcl_lexer::native_script_words_in(
        image,
        tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
        config,
    )
    .unwrap();
    assert!(
        original_array_element_word(&plan.commands[0].words[1], policy).is_none(),
        "mismatching original grammar cannot borrow C8.6 operand semantics"
    );
}

#[test]
fn compiled_local_name_requires_a_direct_source_word() {
    let source = SourceSite::source(tcl_lexer::Span::new(0, 0));
    let raw_braced = WordExpr::BracedLiteral {
        text: r"p\x75b".to_owned(),
        source: source.clone(),
    };
    let bare_escape = WordExpr::Template {
        parts: vec![WordPart::Text {
            text: r"p\x75b".to_owned(),
            source: source.clone(),
        }],
        source: source.clone(),
        rejected: None,
    };
    let quoted_plain = WordExpr::Template {
        parts: vec![WordPart::Text {
            text: "pub".to_owned(),
            source: source.clone(),
        }],
        source: source.clone(),
        rejected: None,
    };
    let dynamic = WordExpr::Variable {
        spelling: "$name".to_owned(),
        source: source.clone(),
    };

    assert_eq!(
        compiled_local_name_word(&raw_braced),
        CompiledLocalNameWord::Direct,
        "braces preserve the backslash as part of the variable name"
    );
    assert_eq!(
        compiled_local_name_value(
            &WordExpr::BracedLiteral {
                text: "{zz}".to_owned(),
                source: source.clone(),
            },
            EscapeSyntax::Tcl90,
            WordValueRules::TCL,
        ),
        Some("{zz}".to_owned()),
        "the source owner removes only the grouping brace pair"
    );
    assert_eq!(
        compiled_local_name_word(&bare_escape),
        CompiledLocalNameWord::Stack,
        "bare escapes decode before variable lookup"
    );
    assert_eq!(
        compiled_local_name_word(&quoted_plain),
        CompiledLocalNameWord::Direct,
        "quotes group a direct name without backslash processing"
    );
    assert_eq!(
        compiled_local_name_word(&dynamic),
        CompiledLocalNameWord::Stack
    );
}

#[test]
fn compiled_local_name_collapses_only_braced_continuations() {
    let source = SourceSite::source(tcl_lexer::Span::new(0, 0));
    let continuation = WordExpr::BracedLiteral {
        text: "p\\\n  ub".to_owned(),
        source: source.clone(),
    };
    let raw_escape = WordExpr::BracedLiteral {
        text: r"p\x75b".to_owned(),
        source,
    };

    assert_eq!(
        compiled_local_name_value(&continuation, EscapeSyntax::Tcl90, WordValueRules::TCL),
        Some("p ub".to_owned())
    );
    assert_eq!(
        compiled_local_name_value(&raw_escape, EscapeSyntax::Tcl90, WordValueRules::TCL),
        Some(r"p\x75b".to_owned())
    );
}

#[test]
fn dynamic_and_expanded_words_never_become_literal_registry_values() {
    let dynamic = [
        literal("global"),
        WordExpr::Variable {
            spelling: "$name".to_owned(),
            source: SourceSite::source(tcl_lexer::Span::new(0, 0)),
        },
    ];
    let RegistryInvocationResolution::Resolved(dynamic_facts) = resolve_word_exprs(
        &CommandRegistry::build_default(),
        Some(test_context()),
        &dynamic,
    )
    .expect("dynamic global has a registry outcome") else {
        panic!("literal global head resolves");
    };
    assert!(
        dynamic_facts
            .state_transitions
            .declared()
            .expect("global declares transition facts")
            .facts()
            .iter()
            .any(|fact| matches!(fact.transition, StateTransition::Widen(_)))
    );

    let expanded = [
        literal("proc"),
        literal("p"),
        WordExpr::Expand {
            source: SourceSite::source(tcl_lexer::Span::new(0, 0)),
            word: Box::new(WordExpr::Opaque {
                text: "args".to_owned(),
                source: SourceSite::opaque(tcl_lexer::Span::new(0, 0)),
                reason: WordOpacity::LossySnapshot,
            }),
        },
        literal("body"),
    ];
    let RegistryInvocationResolution::Resolved(expanded_facts) = resolve_word_exprs(
        &CommandRegistry::build_default(),
        Some(test_context()),
        &expanded,
    )
    .expect("expanded proc has a registry outcome") else {
        panic!("literal proc head resolves");
    };
    assert!(
        expanded_facts
            .state_transitions
            .declared()
            .expect("proc declares transition facts")
            .facts()
            .iter()
            .all(|fact| !matches!(fact.transition, StateTransition::CommandBinding(_)))
    );
}

#[test]
fn dynamic_head_stays_typed_unresolved() {
    let words = [WordExpr::Variable {
        spelling: "$command".to_owned(),
        source: SourceSite::source(tcl_lexer::Span::new(0, 0)),
    }];
    assert!(matches!(
        resolve_word_exprs(
            &CommandRegistry::build_default(),
            Some(test_context()),
            &words,
        ),
        Ok(RegistryInvocationResolution::Unresolved(
            OwnedInvocationResolutionUnresolved::ComputedHead {
                word_kind: InvocationWordKind::Dynamic,
            }
        ))
    ));
}

#[test]
fn original_metadata_context_keeps_actual_availability_and_generation() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let generation = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let registry = generation.commands();
    let old = tcl_registry::model::ingress::static_context_for("tcl8.4")
        .with_command_store(std::sync::Arc::clone(registry));
    let tokens = last_source_tokens("dict for {key value} {} {puts nested}");
    let surface = tcl_registry::model::DocumentCommandSurface::new(registry, None);
    assert_ne!(
        invocation_argument_role_assistance(registry, generation, &surface, &tokens),
        [] as [(usize, tcl_registry::ArgRole); 0]
    );
    assert_eq!(
        invocation_argument_role_assistance(registry, &old, &surface, &tokens),
        [] as [(usize, tcl_registry::ArgRole); 0]
    );
    assert_eq!(
        invocation_argument_role_consensus(registry, &old, &tokens),
        [] as [(usize, tcl_registry::ArgRole); 0]
    );
    let body = invocation_body_assistance(registry, &old, &surface, &tokens);
    assert!(body.possible_roles.is_empty() && body.definite_roles.is_empty());
    assert!(body.unknown_residual && body.definite_invocation.is_none());
    let set = last_source_tokens("set target value");
    assert!(original_variable_write_advice(registry, generation, &set).is_some());
    let foreign =
        generation.with_command_store(std::sync::Arc::new(CommandRegistry::build_default()));
    assert_ne!(
        foreign.commands().snapshot().semantic_key(),
        registry.snapshot().semantic_key()
    );
    assert_eq!(
        invocation_argument_role_assistance(registry, &foreign, &surface, &set),
        [] as [(usize, tcl_registry::ArgRole); 0]
    );
    assert_eq!(
        invocation_argument_role_consensus(registry, &foreign, &set),
        [] as [(usize, tcl_registry::ArgRole); 0]
    );
    assert!(original_variable_write_advice(registry, &foreign, &set).is_none());
    assert!(
        original_symbol_declaration_advice(registry, &foreign, &last_source_tokens("proc p {} {}"))
            .is_none()
    );
}
#[test]
fn retained_script_metadata_keeps_actual_context_and_effective_ordinals() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let generation = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let foreign = tcl_registry::model::ingress::static_context_for("tcl9.1");
    for (source, argument, expected) in [
        (
            "trace add variable v write changed",
            4,
            tcl_registry::ScriptTiming::Deferred,
        ),
        (
            "trace remove variable v write changed",
            4,
            tcl_registry::ScriptTiming::ReferenceOnly,
        ),
        (
            "lsort -command compare {b a}",
            1,
            tcl_registry::ScriptTiming::SameInvocation,
        ),
        (
            "interp alias {} install {} trace add variable v write; install changed",
            4,
            tcl_registry::ScriptTiming::Deferred,
        ),
    ] {
        let tokens = last_source_tokens(source);
        let invocation = resolved_tokens_invocation_in_context(generation, &tokens)
            .expect("authentic selected invocation");
        let realm = tokens
            .source_binding
            .as_ref()
            .unwrap()
            .invocation_realm()
            .unwrap();
        assert_eq!(
            invocation.with_metadata_schema(
                generation.commands(),
                generation.into(),
                realm,
                |selected| selected.authored_source_script_timing_at(argument)
            ),
            Some(expected),
            "{source}"
        );
        assert!(
            invocation
                .with_metadata_schema(generation.commands(), foreign.into(), realm, |selected| {
                    selected.authored_source_script_timing_at(argument)
                })
                .is_none()
        );
    }
}

#[test]
fn original_body_metadata_keeps_supplied_availability_and_generation() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let generation = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let older = tcl_registry::model::ingress::static_context_for("tcl8.4")
        .with_command_store(std::sync::Arc::clone(generation.commands()));
    let source = "dict for {key value} {one two} {set found $value}";
    let config = tcl_lexer::LexerConfig::for_dialect("tcl8.6");
    let bindings = crate::command_binding::SourceCommandBindings::analyse(
        source,
        config,
        generation.commands(),
    );
    let surface = tcl_registry::model::DocumentCommandSurface::new(generation.commands(), None);
    let command =
        crate::segmenter::segment_commands_with_offset_and_config(source, 0, config).remove(0);
    let current = segmented_body_assistance(
        &surface,
        &bindings,
        source,
        config,
        &command,
        0,
        Some(generation),
    )
    .unwrap();
    assert!(
        current
            .possible_roles
            .contains(&(3, tcl_registry::ArgRole::Body))
    );
    let unavailable = segmented_body_assistance(
        &surface,
        &bindings,
        source,
        config,
        &command,
        0,
        Some(&older),
    )
    .unwrap();
    assert_eq!(
        unavailable.possible_roles,
        [] as [(usize, tcl_registry::ArgRole); 0]
    );
    assert!(unavailable.unknown_residual);
    let foreign = tcl_registry::model::ingress::static_context_for("tcl9.1");
    assert!(
        segmented_body_assistance(
            &surface,
            &bindings,
            source,
            config,
            &command,
            0,
            Some(foreign)
        )
        .is_none()
    );
    assert!(
        unpositioned_body_assistance(&surface, &bindings, source, config, &command, Some(foreign))
            .is_none()
    );
}

#[test]
fn retained_invocation_context_keeps_availability_without_donating_handler_proof() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let generation = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let older = tcl_registry::model::ingress::static_context_for("tcl8.4")
        .with_command_store(std::sync::Arc::clone(generation.commands()));
    let tokens = last_source_tokens("dict create key value");
    assert!(resolved_tokens_invocation_in_context(generation, &tokens).is_some());
    assert!(resolved_tokens_invocation_in_context(&older, &tokens).is_none());
    assert!(normal_representation_invocation_in_context(generation, &tokens).is_some());
    assert!(normal_representation_invocation_in_context(&older, &tokens).is_none());
    assert!(normal_transfer_invocation_in_context(generation, &tokens).is_some());
    assert!(normal_transfer_invocation_in_context(&older, &tokens).is_none());
    let names = possible_variable_name_operands_in_context(generation, &tokens).unwrap();
    assert!(!names.candidates.is_empty());
    let unavailable_names = possible_variable_name_operands_in_context(&older, &tokens).unwrap();
    assert!(unavailable_names.candidates.is_empty() && unavailable_names.unknown_residual());
    let assistance = registry_invocation_assistance_in_context(generation, &tokens).unwrap();
    assert_ne!(
        assistance.candidates,
        [] as [crate::registry_invocation::RegistryInvocationShape; 0]
    );
    let unavailable = registry_invocation_assistance_in_context(&older, &tokens).unwrap();
    assert!(unavailable.candidates.is_empty() && unavailable.unknown_residual);
    let replaced = last_source_tokens("proc dict args {}; dict create key value");
    assert!(resolved_tokens_invocation_in_context(generation, &replaced).is_none());
    assert!(normal_representation_invocation_in_context(generation, &replaced).is_none());
    assert!(normal_transfer_invocation_in_context(generation, &replaced).is_none());
    let replaced_names = possible_variable_name_operands_in_context(generation, &replaced).unwrap();
    assert!(replaced_names.candidates.is_empty() && replaced_names.unknown_residual());
    let replaced_assistance =
        registry_invocation_assistance_in_context(generation, &replaced).unwrap();
    assert!(replaced_assistance.candidates.is_empty() && replaced_assistance.unknown_residual);
    assert!(
        logical_structured_invocation_with_metadata_context(
            generation.commands(),
            generation.into(),
            &tokens,
            None
        )
        .is_some()
    );
    assert!(
        logical_structured_invocation_with_metadata_context(
            generation.commands(),
            (&older).into(),
            &tokens,
            None
        )
        .is_none()
    );
    let foreign = tcl_registry::model::ingress::static_context_for("tcl9.1");
    assert!(
        logical_structured_invocation_with_metadata_context(
            generation.commands(),
            foreign.into(),
            &tokens,
            None
        )
        .is_none()
    );
    let mut unproved = tokens;
    unproved.source_binding = None;
    assert!(resolved_tokens_invocation_in_context(generation, &unproved).is_none());
    assert!(normal_representation_invocation_in_context(generation, &unproved).is_none());
    assert!(normal_transfer_invocation_in_context(generation, &unproved).is_none());
    assert!(possible_variable_name_operands_in_context(generation, &unproved).is_none());
    assert!(registry_invocation_assistance_in_context(generation, &unproved).is_none());
    assert!(
        logical_structured_invocation_with_metadata_context(
            generation.commands(),
            generation.into(),
            &unproved,
            None
        )
        .is_none()
    );
    assert!(
        original_callback_invocation_with_metadata_context(
            generation.commands(),
            generation.into(),
            &unproved
        )
        .is_none()
    );
}
fn advisory_metadata_tokens(source: &str, registry: &CommandRegistry) -> CommandTokens {
    let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
    let bindings = crate::command_binding::SourceCommandBindings::analyse(source, config, registry);
    let command = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
        .pop()
        .unwrap();
    CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(source), config, &command)
        .with_source_binding(
            bindings.invocation_at_source("unused-reporting-head", command.span.start()),
        )
}

#[test]
fn advisory_value_metadata_shares_supplied_availability_and_store_refusal() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Registry May candidates only; no actual assignment or returned object.
    let baseline = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let mut registry = baseline
        .commands()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    for name in ["set", "return"] {
        let mut descriptor = registry.get(name).unwrap().clone();
        descriptor.surface = Some(tcl_dialect::model::SpecSurface::TCL86_PLUS);
        registry.insert(descriptor);
    }
    let generation = baseline.with_command_store(std::sync::Arc::new(registry));
    let older = tcl_registry::model::ingress::static_context_for("tcl8.4")
        .with_command_store(std::sync::Arc::clone(generation.commands()));
    let registry = generation.commands();
    assert!(std::sync::Arc::ptr_eq(older.commands(), registry));
    let mut assignment = advisory_metadata_tokens("set target VALUE", registry);
    let mut returning = advisory_metadata_tokens("return VALUE", registry);
    let current = advisory_value_assignments_in_context(&generation, &assignment);
    assert_eq!(current.len(), 1);
    assert_eq!(current[0].name, "target");
    assert_eq!(current[0].value, assignment.words()[2]);
    assert_eq!(
        advisory_return_values_in_context(&generation, &returning),
        vec![returning.words()[1].clone()]
    );
    assert_eq!(
        advisory_value_assignments_in_context(&older, &assignment),
        [] as [crate::registry_invocation::AdvisoryValueAssignment; 0]
    );
    assert_eq!(
        advisory_return_values_in_context(&older, &returning),
        [] as [crate::ir::WordExpr; 0]
    );
    let foreign = tcl_registry::model::ingress::static_context_for("tcl9.1");
    assert_eq!(
        advisory_value_assignments_with_metadata_context(
            registry,
            Some(foreign.into()),
            &assignment
        ),
        [] as [crate::registry_invocation::AdvisoryValueAssignment; 0]
    );
    assert_eq!(
        advisory_return_values_with_metadata_context(registry, Some(foreign.into()), &returning),
        [] as [crate::ir::WordExpr; 0]
    );
    assignment.source_binding = None;
    returning.source_binding = None;
    assert_eq!(
        advisory_value_assignments_in_context(&generation, &assignment),
        [] as [crate::registry_invocation::AdvisoryValueAssignment; 0]
    );
    assert_eq!(
        advisory_return_values_in_context(&generation, &returning),
        [] as [crate::ir::WordExpr; 0]
    );
}
