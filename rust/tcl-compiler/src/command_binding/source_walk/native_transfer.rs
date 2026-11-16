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

//! Declared effects a native handler transfers into the walked state: writes and outputs.

use super::super::module_bindings::apply_declared_binding_transitions;
use super::super::namespace_context::SourceNamespaceKey;
use super::super::{
    ModuleCommandBindings, SourceExecutionContext, SourceNativeInvocation, SourceOutcomes,
    command_publication, original_command_table, original_namespace_deletion,
    original_namespace_ensure, source_command_world,
};
use super::outcomes::boxed_source_branch;
use std::sync::Arc;
use tcl_registry::InvocationFacts;

pub(in super::super) fn select_contents_write_source(state: &mut ModuleCommandBindings) {
    if state.source_variables.contents_write_source() != state.current_source_origin.as_ref() {
        let origin = state.current_source_origin.clone();
        Arc::make_mut(&mut state.source_variables).set_contents_write_source(origin);
    }
}

pub(super) fn transfer_native_invocation(
    native: SourceNativeInvocation<'_>,
    state: &mut ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
    facts: &InvocationFacts,
) -> Option<String> {
    select_contents_write_source(state);
    if matches!(
        native_output_binding_phase(facts),
        tcl_registry::native_compilation::VariableOperandBindingPhase::NormalContinuation
            | tcl_registry::native_compilation::VariableOperandBindingPhase::BodyProtocol
    ) {
        return None;
    }
    let SourceNativeInvocation {
        segment,
        invocation,
        ..
    } = native;
    let SourceExecutionContext { registry, .. } = context;
    let execution =
        crate::ir_helpers::ExecutionNamespace::SourceContext(context.namespace_identity());
    let original_operands =
        original_command_table::OriginalCommandOperands::capture(native, facts, state, context);
    let procedure = original_operands
        .procedure_key(facts, state, &context.namespace_identity())
        .map(|key| ModuleCommandBindings::command_key_label(&key).unwrap_or_default());
    if let Some(procedure) = &procedure {
        state.extend_procedure_bodies([procedure.clone()]);
    }
    let read_projection =
        crate::variable_bindings::source_variable_read_places_with_original_operands(
            facts,
            invocation.arguments(),
            &state.source_variables,
            registry,
            native.original_variable_operands,
        );
    let write_projection =
        crate::variable_bindings::source_variable_write_places_with_original_operands(
            facts,
            invocation.arguments(),
            &state.source_variables,
            registry,
            native.original_variable_operands,
        );
    if read_projection.iter().any(|place| place.observed)
        || write_projection.iter().any(|place| place.observed)
    {
        state.mark_opaque_binding_mutation();
    }
    let publication =
        command_publication::PreservingCommandPublication::capture(native, facts, state, context);
    let fresh_procedure = publication.as_ref().and_then(|publication| {
        command_publication::SourceFreshProcedureTraceAbsence::capture(
            publication,
            native,
            facts,
            state,
            context,
        )
    });
    let original_transfer = source_command_world::capture(native, facts, state, context);
    let namespace_ensures = original_namespace_ensure::OriginalNamespaceEnsureTransfer::capture(
        native, facts, state, context,
    );
    let namespace_deletions =
        original_namespace_deletion::OriginalNamespaceDeletionTransfer::capture(
            native, facts, state, context,
        );
    apply_declared_binding_transitions(
        facts,
        state,
        &execution,
        procedure.as_deref(),
        segment.span.start(),
        publication.as_ref(),
        Some(&original_operands),
    );
    if let Some(transfer) = original_transfer {
        transfer.finish(state);
    } else {
        Arc::make_mut(&mut state.original_command_world).withdraw();
    }
    let namespace_deletions =
        namespace_deletions.and_then(|transfer| transfer.after_command_transfer(state));
    let namespace_ensures =
        namespace_ensures.and_then(|transfer| transfer.after_command_transfer(state));
    transfer_native_namespace_writes(
        native,
        state,
        context,
        facts,
        original_namespace_ensure::OriginalNamespaceCellOperations {
            ensures: namespace_ensures.as_ref(),
            deletions: namespace_deletions.as_ref(),
        },
    );
    if let Some(receipt) = fresh_procedure {
        receipt.retain_after_definition(native, facts, state, context);
    }
    procedure
}

fn transfer_native_namespace_writes(
    native: SourceNativeInvocation<'_>,
    state: &mut ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
    facts: &InvocationFacts,
    namespace_operations: original_namespace_ensure::OriginalNamespaceCellOperations<'_>,
) {
    let registry = context.registry;
    let SourceNativeInvocation {
        segment,
        invocation,
        ..
    } = native;
    let script_interpreted = facts.body_execution.is_some()
        || facts.traits.contains(tcl_registry::Traits::DEFERS_BODY)
        || !matches!(
            tcl_registry::script_body_flow::script_body_flow(facts),
            tcl_registry::script_body_flow::ScriptBodyFlow::None
        );
    state.record_provider_state_writes(
        facts,
        *invocation,
        registry,
        native.original_variable_operands,
    );
    let variables = Arc::make_mut(&mut state.source_variables);
    variables.namespace_identities = state.namespaces.iter().cloned().collect();
    variables.known_namespaces = state
        .namespaces
        .iter()
        .filter_map(SourceNamespaceKey::advisory_key)
        .collect();
    let order = native_output_order(native, facts);
    crate::variable_bindings::transfer_source_namespace_cells_with_input(
        variables,
        facts,
        invocation.arguments(),
        registry,
        crate::variable_bindings::SourceNamespaceTransfer::new(
            script_interpreted,
            segment.span.start(),
            order.as_deref(),
        )
        .with_original_operands(native.original_variable_operands)
        .with_namespace_operations(namespace_operations),
    );
}

#[inline(never)]
pub(super) fn transfer_native_completion_outputs(
    outcomes: &mut SourceOutcomes,
    native: SourceNativeInvocation<'_>,
    facts: &InvocationFacts,
    route: tcl_registry::completion_route::InvocationCompletionRoute,
    context: SourceExecutionContext<'_>,
) {
    if native_output_binding_phase(facts)
        == tcl_registry::native_compilation::VariableOperandBindingPhase::NormalContinuation
        && let Some(normal) = &mut outcomes.normal
    {
        let before_outputs = boxed_source_branch(normal);
        transfer_native_outputs(native, facts, normal, context);
        if route.abrupt_possible() {
            let after_outputs = boxed_source_branch(normal);
            outcomes.add_abrupt(
                tcl_registry::completion_route::InvocationCompletionRoute::Tcl(
                    tcl_registry::completion::CompletionCode::Error,
                ),
                &before_outputs,
            );
            outcomes.add_abrupt(
                tcl_registry::completion_route::InvocationCompletionRoute::Tcl(
                    tcl_registry::completion::CompletionCode::Error,
                ),
                &after_outputs,
            );
        }
    }
}

fn native_output_binding_phase(
    facts: &InvocationFacts,
) -> tcl_registry::native_compilation::VariableOperandBindingPhase {
    facts.successful_handler.map_or(
        tcl_registry::native_compilation::VariableOperandBindingPhase::AfterArguments,
        tcl_registry::native_compilation::SuccessfulHandlerSpec::variable_binding_phase,
    )
}

fn native_output_order(
    native: SourceNativeInvocation<'_>,
    facts: &InvocationFacts,
) -> Option<Vec<usize>> {
    use tcl_registry::native_compilation::VariableOutputLookup;
    match facts
        .successful_handler
        .map(tcl_registry::native_compilation::SuccessfulHandlerSpec::variable_output_lookup)
    {
        Some(VariableOutputLookup::CatchSelected) => {
            let dialect = native.invocation.arguments().dialect()?;
            let tcl_registry::catch_invocation::CatchInvocationSelection::Valid(capture) =
                tcl_registry::catch_invocation::select_catch_invocation(
                    native.invocation.arguments(),
                    dialect,
                )
            else {
                return None;
            };
            Some(
                capture
                    .output_order(dialect, *native.compilation_selection)
                    .indices(capture)?
                    .into_iter()
                    .flatten()
                    .collect(),
            )
        }
        Some(VariableOutputLookup::Sequential | VariableOutputLookup::SingleTarget) => Some(
            facts
                .arg_roles
                .iter()
                .filter_map(|&(index, role)| {
                    (role == tcl_registry::ArgRole::VarWrite)
                        .then_some(facts.argument_offset + usize::from(index))
                })
                .collect(),
        ),
        None | Some(VariableOutputLookup::BodyProtocol) => None,
    }
}

fn transfer_native_outputs(
    native: SourceNativeInvocation<'_>,
    facts: &InvocationFacts,
    state: &mut ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) {
    let registry = context.registry;
    select_contents_write_source(state);
    let order = native_output_order(native, facts);
    let places = crate::variable_bindings::source_variable_write_places_with_output_order_and_original_operands(
        facts,
        native.invocation.arguments(),
        &state.source_variables,
        registry,
        order.as_deref(),
        native.original_variable_operands,
    );
    if places.iter().any(|place| place.observed) {
        state.mark_opaque_binding_mutation();
    }
    state.record_provider_state_writes(
        facts,
        *native.invocation,
        registry,
        native.original_variable_operands,
    );
    let variables = Arc::make_mut(&mut state.source_variables);
    variables.namespace_identities = state.namespaces.iter().cloned().collect();
    variables.known_namespaces = state
        .namespaces
        .iter()
        .filter_map(SourceNamespaceKey::advisory_key)
        .collect();
    crate::variable_bindings::transfer_source_namespace_cells_with_input(
        variables,
        facts,
        native.invocation.arguments(),
        registry,
        crate::variable_bindings::SourceNamespaceTransfer::new(
            true,
            native.segment.span.start(),
            order.as_deref(),
        )
        .with_original_operands(native.original_variable_operands),
    );
}
