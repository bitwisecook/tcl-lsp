// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Attempted original scalar-update syntax, before a rejected operand read.

use super::{
    CommandRegistry, CommandTokens, InvocationMetadataContext, LogicalStructuredInvocation,
};

/// Immutable attempted syntax. This receipt proves neither a completed argv,
/// reached handler, entered frame, Native operand nor a committed store.
pub(crate) struct OriginalOperandEvaluationUpdate {
    selected: LogicalStructuredInvocation,
    hook: tcl_registry::hooks::LoweringHookId,
}

impl OriginalOperandEvaluationUpdate {
    pub(crate) fn descriptor_key(&self) -> tcl_registry::CanonicalCommandDescriptorKey<'_> {
        self.selected.descriptor_key()
    }

    pub(crate) fn lowering_hook(&self) -> tcl_registry::hooks::LoweringHookId {
        self.hook
    }
}

/// Preserve an actual update's operand plan even when an original operand read
/// fails before handler entry. Handler/body admission keeps its separate guard.
/// Only complete direct written layouts with static original write names and
/// no expression, body, frame or deferred-script grammar can issue this view.
pub(crate) fn original_operand_evaluation_update(
    registry: &CommandRegistry,
    metadata: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
    bindings: &crate::command_binding::SourceCommandBindings,
) -> Option<OriginalOperandEvaluationUpdate> {
    use crate::command_binding::OriginalInvocationSourcePurpose;
    use tcl_registry::{ArgRole, BodyInterpreter, BodyKind};
    if !bindings.original_arguments_rejected_before_handler(tokens)
        || tokens
            .source_binding
            .as_ref()?
            .original_source_advice_purpose(tokens, metadata, registry)?
            != OriginalInvocationSourcePurpose::Execution
    {
        return None;
    }
    metadata.source_analysis_input()?;
    // This existing issuer proves original syntax/direct slots only. The
    // authentic failure receipt above deliberately supplies no handler entry.
    let selected = super::logical_structured_invocation_with_metadata_context(
        registry, metadata, tokens, None,
    )?;
    let invocation = &selected.invocation;
    let facts = &invocation.facts;
    let hook = facts.lowering_hook?;
    if facts.frame_effect.is_some()
        || facts.body_execution.is_some()
        || facts.body_kind != BodyKind::Plain
        || facts.body_interpreter != BodyInterpreter::Current
        || !facts.deferred_script_argument_indices()?.is_empty()
        || selected
            .written_roles()
            .iter()
            .any(|(_, role)| matches!(role, ArgRole::Body | ArgRole::Expr | ArgRole::CommandPrefix))
    {
        return None;
    }
    let dialect = invocation.dialect?;
    let mut writes = 0usize;
    for (argument, role) in selected.written_roles() {
        if role != ArgRole::VarWrite {
            continue;
        }
        let word = tokens.words().get(argument.checked_add(1)?)?;
        if !matches!(
            super::effective_invocation_word(
                word,
                dialect.lexer_grammar.escapes,
                dialect.word_values,
            ),
            super::EffectiveInvocationWord::Literal(_)
        ) {
            return None;
        }
        writes = writes.checked_add(1)?;
    }
    (writes != 0).then_some(OriginalOperandEvaluationUpdate { selected, hook })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn failed_nested_expression_keeps_its_original_operand_without_expression_topology() {
        for environment in ["tcl8.6", "tcl9.0"] {
            let fixture = crate::provider_fixtures::original_native_module(
                "set n [expr {$missing+1}]",
                environment,
            );
            let (context, module) = &*fixture;
            let registry = context.commands();
            let statement = module.top_level.statements.last().unwrap();
            let tokens = module
                .top_level
                .retained_source_tokens_for_statement(statement)
                .unwrap();
            let positioned = module
                .retained_source_bindings
                .as_ref()
                .unwrap()
                .positioned_for_module(module, registry)
                .unwrap();
            let metadata = InvocationMetadataContext::for_module(registry, module).unwrap();
            assert!(positioned.original_arguments_rejected_before_handler(tokens));
            let syntax = original_operand_evaluation_update(registry, metadata, tokens, positioned)
                .expect("actual attempted Set operand syntax");
            assert_eq!(
                syntax.lowering_hook(),
                tcl_registry::hooks::LoweringHookId::Set
            );
            assert!(!matches!(
                statement,
                crate::ir::Statement::AssignExpr { .. } | crate::ir::Statement::ExprEval { .. }
            ));
            assert!(
                module
                    .top_level
                    .retained_source_tokens_for_statement(statement)
                    .is_some()
            );
        }
    }

    #[test]
    fn original_update_syntax_preserves_failed_reads_without_handler_or_body_entry() {
        for environment in ["tcl8.6", "tcl9.0"] {
            let resolved =
                tcl_registry::model::ingress::resolve_known_environment(environment).unwrap();
            let context = resolved.default_context_registry();
            let registry = context.commands();
            let profile = resolved.unit_profile();
            let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
            let input = crate::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                Arc::clone(&context),
                config,
            );
            let (_owner, native) =
                crate::environment_ingress::captured_native_entry_with_owner(profile);
            for (source, expected) in [
                ("set n 5; incr n $delta", true),
                ("set n 5; incr n \"$delta\"", true),
                ("set n 5; set delta 2; incr n $delta", false),
                ("set n 5; incr $destination $delta", false),
                ("if $condition {set n 1}", false),
                (
                    "proc incr args {return CUSTOM}; set n 5; incr n $delta",
                    false,
                ),
            ] {
                let bindings = crate::command_binding::SourceCommandBindings::analyse_with_options(
                    source,
                    config,
                    registry,
                    crate::command_binding::SourceAnalysisOptions {
                        metadata_context: super::super::InvocationMetadataInput::SuppliedSource(
                            Some(&input),
                        ),
                        native_entry: Some(&native),
                        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(
                            profile,
                        )),
                        native_compilation:
                            crate::environment_ingress::authoring_native_compilation(),
                        ..Default::default()
                    },
                );
                let segment =
                    crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
                        .into_iter()
                        .last()
                        .unwrap();
                let mut tokens = CommandTokens::from_segmented(
                    &tcl_lexer::SourceMap::new(source),
                    config,
                    &segment,
                );
                bindings.stamp_original_tokens(&mut tokens);
                let metadata =
                    InvocationMetadataContext::for_analysis_input(registry, &input).unwrap();
                let syntax =
                    original_operand_evaluation_update(registry, metadata, &tokens, &bindings);
                assert_eq!(syntax.is_some(), expected, "{environment}: {source}");
                if !expected {
                    continue;
                }
                assert!(bindings.original_arguments_rejected_before_handler(&tokens));
                assert!(
                    super::super::logical_structured_invocation_with_metadata_context(
                        registry,
                        metadata,
                        &tokens,
                        Some(&bindings),
                    )
                    .is_none()
                );
                assert_eq!(
                    syntax.unwrap().lowering_hook(),
                    tcl_registry::hooks::LoweringHookId::Incr
                );
                let mut changed = tokens.clone();
                changed.argv_texts.last_mut().unwrap().push_str("CHANGED");
                assert!(
                    original_operand_evaluation_update(registry, metadata, &changed, &bindings)
                        .is_none()
                );
                let foreign = tcl_registry::model::ingress::resolve_environment("tcl9.1")
                    .default_context_registry();
                assert!(
                    original_operand_evaluation_update(
                        foreign.commands(),
                        metadata,
                        &tokens,
                        &bindings
                    )
                    .is_none()
                );
            }
        }
    }
}
