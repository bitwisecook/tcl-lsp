// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Restoration preserves conditional source receipts, never runtime dispatch.

use super::*;
use crate::analyser::ResolvedAnalysisInput;
use crate::ir::{CommandTokens, Module};

fn original_module(source: &str, input: &ResolvedAnalysisInput) -> Module {
    let mut lowerer = crate::lowering::Lowerer::with_config(
        input.borrowed_context_registry().commands(),
        input.lexer_config(),
    )
    .with_resolved_analysis_input(input.clone());
    lowerer.lower(source);
    let module = lowerer.finish_module(source);
    assert_eq!(module.source_metadata_input.as_ref(), Some(input));
    assert_eq!(module.lexer_config, input.lexer_config());
    module
}

fn logical_input() -> ResolvedAnalysisInput {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    ResolvedAnalysisInput::new(
        profile,
        profile,
        tcl_registry::model::ingress::resolve_known_environment("tcl8.6")
            .unwrap()
            .default_context_registry(),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    )
}

fn original_body_parent(
    source: &str,
    input: &ResolvedAnalysisInput,
) -> (SourceCommandBindings, CommandTokens) {
    let module = original_module(source, input);
    let retained = module.retained_source_bindings.as_ref().unwrap();
    let owner = retained
        .positioned_for_module(&module, input.borrowed_context_registry().commands())
        .unwrap();
    let procedures = module.procedures.values().collect::<Vec<_>>();
    let [procedure] = procedures.as_slice() else {
        panic!("one original Lowerer procedure")
    };
    assert!(retained.matches_original_procedure(procedure));
    let source = procedure.body.executed_source.as_ref().unwrap();
    let namespace = procedure.body.namespace_context.as_deref().unwrap();
    let selected = owner.selected_source_in_context(source, namespace).unwrap();
    let [statement] = procedure.body.statements.as_slice() else {
        panic!("one retained original statement")
    };
    (selected, statement.tokens().unwrap().clone())
}

fn selected_child(tokens: &CommandTokens, config: tcl_lexer::LexerConfig) -> CommandTokens {
    let mut calls = crate::word_subst::checked_lifted_calls(tokens, config).unwrap();
    assert_eq!(calls.len(), 1);
    calls.pop().unwrap().tokens.unwrap()
}

fn assert_raw_nested(owner: &SourceCommandBindings, tokens: &CommandTokens) {
    for (offset, binding) in &tokens.nested_bindings {
        assert_eq!(binding, &owner.invocation_at_source("", *offset));
        assert!(binding.original_logical_declaration_targets.is_none());
    }
}

#[test]
fn original_restored_nested_declaration_keeps_actual_child_vector_without_dispatch() {
    // naming.compiler.original-positioned-callee-body-availability
    // docs/design/analysis/name-resolution-proofs/original-positioned-callee-body-availability.md
    // The Lowerer owns the body, selected source and nested inventory. This
    // reaches the same nested restoration worker used by both Script carriers.
    let input = logical_input();
    let (owner, mut parent) =
        original_body_parent("proc subject {} {puts [package require Tcl]}", &input);
    let child = selected_child(&parent, input.lexer_config());
    let offset = child.words()[0].source().span.start();
    assert_eq!(parent.nested_bindings.len(), 1);
    assert_eq!(parent.nested_bindings[0].0, offset);
    assert!(
        parent.nested_bindings[0]
            .1
            .original_logical_declaration_targets
            .is_some()
    );
    let original_parent = parent.source_binding.clone();
    let original_words = parent.word_exprs.clone();
    owner.restore_original_nested_bindings(&mut parent);
    assert_eq!(parent.source_binding, original_parent);
    assert_eq!(parent.word_exprs, original_words);
    assert_eq!(parent.nested_bindings.len(), 1);
    assert_eq!(parent.nested_bindings[0].0, offset);
    let restored = selected_child(&parent, input.lexer_config());
    assert_eq!(restored.words(), child.words());
    let binding = restored.source_binding.as_ref().unwrap();
    assert!(binding.original_logical_declaration_targets.is_some());
    let targets = binding
        .original_logical_declaration_call_targets(
            &restored,
            &input,
            input.borrowed_context_registry().commands(),
        )
        .unwrap();
    assert_eq!(targets.len(), 1);
    assert!(targets[0].registry_backed);
    assert!(targets[0].implementation_allocation.is_none());
    let raw = owner.invocation_at_source("", offset);
    let mut source_only = binding.clone();
    source_only.original_logical_declaration_targets = None;
    assert_eq!(source_only, raw);
    assert!(binding.lookup_state.is_none());
    assert!(binding.unknown);
    assert!(binding.targets.is_empty());
    assert!(binding.proved_target().is_none());
    assert!(binding.proved_execution_target().is_none());
    assert!(binding.proved_handler_target().is_none());
    assert_ne!(
        binding.runtime_reachability(),
        SourceRuntimeReachability::Reached
    );
    assert_ne!(
        binding.runtime_reachability(),
        SourceRuntimeReachability::Conditional
    );
}

#[test]
fn original_restored_nested_declaration_requires_retained_inventory_input_config_and_rows() {
    // naming.compiler.original-positioned-callee-body-availability
    // docs/design/analysis/name-resolution-proofs/original-positioned-callee-body-availability.md
    let input = logical_input();
    let source = "proc subject {} {puts [package require Tcl]}";
    let (owner, parent) = original_body_parent(source, &input);
    let child = selected_child(&parent, input.lexer_config());
    let site = child
        .source_binding
        .as_ref()
        .unwrap()
        .invocation_site()
        .unwrap()
        .clone();
    assert!(
        parent.nested_bindings[0]
            .1
            .original_logical_declaration_targets
            .is_some()
    );
    for axis in 0..7 {
        let mut changed_owner = owner.clone();
        let mut changed = parent.clone();
        match axis {
            0 => changed_owner.lexer_config = None,
            1 => {
                let mut config = input.lexer_config();
                config.leading_bom = match config.leading_bom {
                    tcl_lexer::LeadingBom::Content => tcl_lexer::LeadingBom::Skip,
                    tcl_lexer::LeadingBom::Skip => tcl_lexer::LeadingBom::Content,
                };
                changed_owner.lexer_config = Some(config);
            }
            2 => {
                changed
                    .nested_bindings
                    .push(changed.nested_bindings[0].clone());
            }
            3 => {
                changed.nested_bindings[0].0 += 1;
            }
            4 => {
                changed.word_exprs.pop();
            }
            5 => {
                changed_owner.declaration_layouts.remove(&site);
            }
            6 => {
                let foreign =
                    original_body_parent("proc another {} {puts [package require Tcl]}", &input);
                changed_owner = foreign.0;
            }
            _ => unreachable!(),
        }
        let offsets = changed
            .nested_bindings
            .iter()
            .map(|(offset, _)| *offset)
            .collect::<Vec<_>>();
        changed_owner.restore_original_nested_bindings(&mut changed);
        assert_eq!(
            changed
                .nested_bindings
                .iter()
                .map(|(offset, _)| *offset)
                .collect::<Vec<_>>(),
            offsets
        );
        assert_raw_nested(&changed_owner, &changed);
    }
    let changed_input = ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        tcl_registry::model::ingress::resolve_known_environment("tcl9.0")
            .unwrap()
            .default_context_registry(),
        input.lexer_config(),
    );
    let (changed_owner, _) = original_body_parent(source, &changed_input);
    let mut changed = parent.clone();
    changed_owner.restore_original_nested_bindings(&mut changed);
    assert_raw_nested(&changed_owner, &changed);
    let mut missing = parent.clone();
    missing.nested_bindings.clear();
    owner.restore_original_nested_bindings(&mut missing);
    assert!(missing.nested_bindings.is_empty());
}

#[test]
fn original_script_restoration_keeps_reached_nested_statement_and_consumed_site_bindings() {
    // naming.source.retained-positioned-command-realm
    // docs/design/analysis/name-resolution-proofs/retained-positioned-command-realm.md
    // A held Native entry supplies the unchanged reached route. Conditional
    // declaration restoration grants nothing to this separate execution route.
    let environment = tcl_registry::model::ingress::resolve_known_environment("tcl8.6").unwrap();
    let profile = environment.unit_profile();
    let input = ResolvedAnalysisInput::new(
        profile,
        profile,
        environment.default_context_registry(),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    let captured = crate::environment_ingress::captured_native_entry(profile);
    let source = "puts [list VALUE]; foreach item [list VALUE] {}";
    let mut lowerer = crate::lowering::Lowerer::with_config(
        input.borrowed_context_registry().commands(),
        input.lexer_config(),
    )
    .with_resolved_analysis_input(input.clone());
    lowerer.set_source_analysis_options(SourceAnalysisOptions {
        native_entry: Some(captured.as_entry()),
        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
        native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
            mode: tcl_registry::native_compilation::NativeCompilationMode::BytecodeObject,
            frame: tcl_registry::native_compilation::NativeCompilationFrame::ScriptCode,
            loop_depth: 0,
            catch_depth: Some(0),
        },
        ..Default::default()
    });
    lowerer.lower(source);
    let module = lowerer.finish_module(source);
    let owner = module
        .retained_source_bindings
        .as_ref()
        .unwrap()
        .positioned_for_module(&module, input.borrowed_context_registry().commands())
        .unwrap();
    let mut restored = module.top_level.clone();
    let source_before = restored.executed_source.clone();
    let statement = restored.statements[0].tokens().unwrap();
    assert!(!statement.nested_bindings.is_empty());
    assert!(restored.command_binding_sites.iter().any(|site| {
        site.source_tokens
            .as_deref()
            .is_some_and(|tokens| !tokens.nested_bindings.is_empty())
    }));
    assert!(owner.restore_script_bindings(&mut restored));
    assert_eq!(restored.executed_source, source_before);
    for tokens in restored
        .statements
        .iter()
        .filter_map(|statement| statement.tokens())
        .chain(
            restored
                .command_binding_sites
                .iter()
                .filter_map(|site| site.source_tokens.as_deref()),
        )
    {
        for (offset, binding) in &tokens.nested_bindings {
            assert_eq!(binding, &owner.invocation_at_source("", *offset));
            assert!(binding.lookup_state.is_some());
            assert!(binding.original_logical_declaration_targets.is_none());
        }
    }
    assert_eq!(captured.execution_point, environment.point());
}
