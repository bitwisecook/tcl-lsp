// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Empty stock Jim source iteration, without object or execution receipts.

use super::{
    ModuleCommandBindings, SourceBodyEntryPurpose, SourceExecutionContext, SourceNativeInvocation,
    SourceOutcomes,
};
use tcl_syntax::naming::{NamePolicyAuthority, NativeNameProtocol};

/// The original argv and command observers have already run, and the caller
/// has independently checked the current stock handler implementation.
pub(super) fn transfer(
    native: SourceNativeInvocation<'_>,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> Option<SourceOutcomes> {
    let policy = state.baseline.execution_name_policy?.native_recipe()?;
    if policy.authority() != NamePolicyAuthority::AuthoredSimulation
        || policy.recipe() != NativeNameProtocol::Jim084
        || context.body_entry() != SourceBodyEntryPurpose::Selected
        || state.baseline.native_entry.is_some()
        || state.baseline.source_point_entry.is_some()
        || state.baseline.unknown_entry
        || state.has_opaque_domain()
        || state.unclosed_command_world
        || state.has_unnameable_rebinding_subject()
        || state.source_step_observed()
        || state.source_execution_observed(native.target.identity.as_ref())
        || state.runtime_execution_observed(native.target.identity.as_ref())
        || !state.command_observers.is_quiet()
        || state.source_variables.dynamic_traces
        || !state.source_variables.traced.is_empty()
        || !state
            .source_variables
            .possible_trace_registrations
            .is_empty()
        || !state.ordinary_literal_pool.as_ref().is_some_and(
            super::literal_object_pool::SourceOrdinaryLiteralPool::text_effects_current,
        )
        || !native.target.registry_backed
        || native.target.kind != super::BindingKind::Builtin
        || native.target.implementation_generation != 0
        || native.target.implementation_allocation.is_some()
        || !native.target.prepended.is_empty()
    {
        return None;
    }
    let input = state.baseline.metadata_context.source_analysis_input()?;
    let supplied = input.borrowed_context_registry();
    let schema = tcl_registry::model::resolve_invocation_words_in_context(
        context.registry,
        Some(supplied.context()),
        *native.invocation,
    )?;
    let phase = schema.authored_source_jim_empty_iteration()?;
    if phase.policy() != policy || input.lexer_config() != context.config {
        return None;
    }
    let source = state.current_source_origin.as_ref()?;
    let lexical = source.original_lexical_command(
        native.words,
        native.segment.span.start(),
        context.config,
    )?;
    let original = lexical.native_words();
    if original.iter().any(|word| word.group().expand) {
        return None;
    }
    let arguments = native.invocation.arguments();
    let count = arguments.exact_argv_len()?;
    if original.len() != count.checked_add(1)? {
        return None;
    }
    let words = tcl_registry::native_compiler_words::NativeCompilerWords::capture(
        original,
        policy.string_protocol(),
    )
    .ok()?;
    // Every iterator input is original parser-owned Text. A substituted value
    // with the same bytes supplies no source-object or conversion permission.
    for argument in 0..count.checked_sub(1)? {
        if words.literal(argument + 1)? != arguments.literal_at(argument)?.as_bytes() {
            return None;
        }
    }
    // Preserve the post-argument world only. No Normal completion, physical
    // object, binder, compiled child or conditional-body certificate is issued.
    Some(SourceOutcomes::normal(state))
}
