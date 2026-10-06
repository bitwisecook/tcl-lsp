// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

fn original(
    source: &str,
    command: &str,
    offset: usize,
) -> (SourceCommandBindings, crate::ir::CommandTokens) {
    let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
    let profile = registry.profile().unwrap();
    let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
    let bindings = SourceCommandBindings::analyse_with_options(
        source,
        config,
        registry,
        SourceAnalysisOptions {
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
            native_compilation: crate::environment_ingress::authoring_native_compilation(),
            ..Default::default()
        },
    );
    let source_part = command;
    let segments = crate::segmenter::segment_commands_with_offset_and_config(
        source_part,
        u32::try_from(offset).unwrap(),
        config,
    );
    let mut tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::new(source),
        config,
        &segments[0],
    );
    bindings.stamp_original_tokens(&mut tokens);
    (bindings, tokens)
}

#[test]
fn conditional_expression_template_retains_grammar_without_actual_entry() {
    let source = "proc f {} {expr {7 << 1}}";
    let (_, tokens) = original(source, "expr {7 << 1}", source.find("expr").unwrap());
    let binding = tokens.source_binding.as_ref().unwrap();
    let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
    let receipt = binding
        .conditional_expression_evaluation(registry, &tokens)
        .expect("exact conditional expression receipt");
    assert_eq!(
        receipt.normal_result_representation(|_| None),
        Some(tcl_registry::TclType::Int)
    );
    assert_eq!(
        binding.runtime_reachability(),
        SourceRuntimeReachability::Conditional
    );
    assert!(binding.execution_is_unknown());
    assert!(binding.proved_execution_target().is_none());
    assert!(binding.original_normal_result(&tokens).is_none());
    assert!(!binding.original_invocation_completes_normally(&tokens));
    assert!(binding.expression_preparation(&[]).is_none());
}

#[test]
fn conditional_expression_pool_representation_is_withdrawn_by_shared_getters() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
    for source in [
        "set held 14; llength $held; expr {7 << 1}",
        "unknown_writer; expr {7 << 1}",
    ] {
        let (_, tokens) = original(
            source,
            &source[source.find("expr").unwrap()..],
            source.find("expr").unwrap(),
        );
        let answer = tokens
            .source_binding
            .as_ref()
            .and_then(|binding| binding.conditional_expression_evaluation(registry, &tokens))
            .and_then(|receipt| receipt.normal_result_representation(|_| None));
        assert!(answer.is_none(), "{source}");
    }
}

#[test]
fn conditional_expression_source_and_replaced_handler_do_not_borrow_receipts() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
    let (_, tokens) = original("expr {7 << 1}", "expr {7 << 1}", 0);
    let binding = tokens.source_binding.as_ref().unwrap();
    assert!(
        binding
            .conditional_expression_evaluation(registry, &tokens)
            .is_some()
    );
    let (_, foreign) = original("expr {8 << 1}", "expr {8 << 1}", 0);
    assert!(
        binding
            .conditional_expression_evaluation(registry, &foreign)
            .is_none()
    );
    let source = "proc expr args {return arbitrary}; expr {7 << 1}";
    let (_, replaced) = original(source, "expr {7 << 1}", source.rfind("expr").unwrap());
    assert!(
        replaced
            .source_binding
            .as_ref()
            .unwrap()
            .conditional_expression_evaluation(registry, &replaced)
            .is_none()
    );
}
