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

//! The successful continuation of one statement or resolved invocation: outputs, contents writes, transitions and destroyed roots.

use super::OriginalVariableInvocation;
use super::alias_binding::{bind_alias, bind_alias_with_compiled_local};
use super::cell_lifetimes::{
    destroy_captured_cell, destroy_root, detach_retained_destructions, establish_written_lifetime,
    invalidate_literal_writes,
};
use super::namespace_retirement::apply_namespace;
use super::source_places::{
    invocation_write_trace_operation, source_variable_definitions_with_original_operands,
    source_variable_read_places_with_original_operands_impl,
    source_variable_write_places_with_output_order_and_original_operands_impl,
    variable_operand_access, variable_output_operand_access_with_original_operands_impl,
};
use super::variable_traces::{apply_trace, apply_trace_with_original};
use crate::ir::Statement;
use crate::place;
use crate::place::{CellGeneration, Place, PlaceKind};
use crate::var_resolve::{ResolveContext, cell_key, project_access, resolve_literal_access};
use tcl_registry::model::semantic::SemanticContext;
use tcl_registry::{
    CommandRegistry, NamespaceTransition, StateTransition, StateTransitionDomain,
    StateTransitionKnowledge, Traits,
};

/// Apply the successful continuation of one statement. Cell operations with
/// active traces lose binding precision because callbacks can re-enter Tcl.
pub fn transfer_statement(
    state: &mut ResolveContext,
    statement: &Statement,
    registry: &CommandRegistry,
) {
    if statement.has_opaque_native_accesses() {
        state.widen();
        return;
    }
    if crate::dictionary_bindings::transfer_scope_marker(state, statement, registry) {
        return;
    }
    if crate::place_bridge::read_places(statement, state, registry)
        .iter()
        .any(|place| place.observed)
    {
        state.widen();
    }
    match statement {
        Statement::AssignConst {
            name, name_braced, ..
        }
        | Statement::AssignValue {
            name, name_braced, ..
        }
        | Statement::AssignExpr {
            name, name_braced, ..
        }
        | Statement::Incr {
            name, name_braced, ..
        } => {
            if state.execution_name_policy.is_some() {
                transfer_invocation(state, statement, statement.tokens(), registry);
                return;
            }
            let target = crate::var_resolve::resolve_target_access(
                name,
                *name_braced,
                state,
                registry,
                tcl_registry::TraceOperation::Write,
            );
            state.record_contents_write(&target, statement.span().start(), false);
            if target.observed {
                state.widen();
            } else {
                establish_written_lifetime(state, &target, statement.span().start());
                if target.is_global() && !target.dynamic {
                    state.namespace_cells.present.insert(cell_key(&target));
                }
            }
        }
        Statement::Block { body, .. } => {
            for inner in &body.statements {
                transfer_statement(state, inner, registry);
            }
        }
        Statement::Call {
            tokens: Some(tokens),
            ..
        } if matches!(
            tokens.synthetic,
            Some(crate::ir::SyntheticMarker::IterationBindings(_))
        ) =>
        {
            let targets = crate::place_bridge::def_places(statement, state, registry);
            invalidate_literal_writes(state, &targets);
            for target in targets {
                state.record_contents_write(&target, statement.span().start(), false);
                if target.observed {
                    state.widen();
                } else {
                    establish_written_lifetime(state, &target, statement.span().start());
                    if target.is_global() && !target.dynamic {
                        state.namespace_cells.present.insert(cell_key(&target));
                    }
                }
            }
        }
        Statement::Call { tokens, .. } | Statement::Barrier { tokens, .. } => {
            transfer_invocation(state, statement, tokens.as_ref(), registry);
        }
        Statement::UpFrame { .. } => state.widen(),
        _ => {}
    }
}

fn transfer_invocation(
    state: &mut ResolveContext,
    statement: &Statement,
    tokens: Option<&crate::ir::CommandTokens>,
    registry: &CommandRegistry,
) {
    let Some(tokens) = tokens else {
        state.widen();
        return;
    };
    if tokens.synthetic == Some(crate::ir::SyntheticMarker::EvaluatedArguments) {
        return;
    }
    let context = registry.profile().map(SemanticContext::for_profile);
    let Some(invocation) =
        crate::registry_invocation::normal_transfer_invocation(registry, context, tokens)
    else {
        state.widen();
        return;
    };
    // The selected invocation supplies the original source instance. A
    // represented offset or a reporting spelling cannot attest this store.
    state.set_contents_write_source(
        tokens
            .source_binding
            .as_ref()
            .and_then(|binding| binding.invocation_site())
            .map(|site| std::sync::Arc::clone(&site.source)),
    );
    invocation.transfer_variables(state, statement, tokens, registry);
}

pub(crate) fn transfer_captured_variable_outputs(
    state: &mut ResolveContext,
    outputs: &[Place],
    source: u32,
) {
    invalidate_literal_writes(state, outputs);
    for target in outputs {
        state.record_contents_write(target, source, false);
        if target.kind != PlaceKind::Unknown && !target.observed {
            establish_written_lifetime(state, target, source);
            if target.is_global() && !target.dynamic {
                state.namespace_cells.present.insert(cell_key(target));
            }
        }
    }
    if outputs
        .iter()
        .any(|target| target.observed || target.kind == PlaceKind::Unknown)
    {
        state.widen();
    }
}

/// Address/effect knowledge after a reached iterator assignment phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceIterationBindingOutcome {
    /// Every selected variable received this iteration's unknown value.
    Entered,
    /// A proved variable-kind conflict stops before entering the body.
    StoreError,
    /// An unknown name, grammar or observer retains a residual obligation.
    Unknown,
}

/// Apply exact frozen iterator values to their sequentially captured outputs.
/// This retains text only; numeric and object representation remain unknown.
pub(crate) fn transfer_finite_iteration_bindings(
    state: &mut ResolveContext,
    values: &[(&[u8], &[u8])],
    registry: &CommandRegistry,
    source_offset: u32,
) -> SourceIterationBindingOutcome {
    use SourceIterationBindingOutcome as Outcome;
    for &(name, value) in values {
        let (Ok(name), Ok(value)) = (std::str::from_utf8(name), std::str::from_utf8(value)) else {
            state.widen();
            return Outcome::Unknown;
        };
        let target = resolve_literal_access(
            name,
            state,
            false,
            registry,
            tcl_registry::TraceOperation::Write,
        );
        if state.native_store_would_error(&target) {
            return Outcome::StoreError;
        }
        if target.observed || target.dynamic || target.kind == PlaceKind::Unknown {
            return Outcome::Unknown;
        }
        transfer_captured_variable_outputs(state, &[target], source_offset);
        state.define_literal(name, value, registry);
    }
    Outcome::Entered
}

/// Apply authored variable-list operands when an iteration actually enters.
/// The caller selects indices from the resolved loop plan, not command text.
/// Arguments have already been frozen. Sequential address lookup preserves
/// earlier writes when a later output fails or an observer changes bindings.
pub fn transfer_source_iteration_bindings(
    state: &mut ResolveContext,
    variable_lists: &[usize],
    arguments: tcl_registry::InvocationArguments<'_>,
    registry: &CommandRegistry,
    source_offset: u32,
) -> SourceIterationBindingOutcome {
    use SourceIterationBindingOutcome as Outcome;
    if variable_lists.is_empty() {
        return Outcome::Entered;
    }
    let Some(dialect) = arguments.dialect() else {
        state.widen();
        return Outcome::Unknown;
    };
    let rules = tcl_syntax::word_rules::WordValueRules::from_grammar(&dialect.lexer_grammar);
    let mut names = Vec::new();
    for &index in variable_lists {
        let Some(value) = arguments.literal_at(index) else {
            state.widen();
            return Outcome::Unknown;
        };
        let Ok(decoded) = rules.split_list(value) else {
            state.widen();
            return Outcome::Unknown;
        };
        if decoded.is_empty() {
            state.widen();
            return Outcome::Unknown;
        }
        names.extend(decoded.into_iter().map(std::borrow::Cow::into_owned));
    }
    let mut outcome = Outcome::Entered;
    for name in names {
        let target = resolve_literal_access(
            &name,
            state,
            false,
            registry,
            tcl_registry::TraceOperation::Write,
        );
        if state.native_store_would_error(&target) {
            return Outcome::StoreError;
        }
        if target.observed || target.dynamic || target.kind == PlaceKind::Unknown {
            outcome = Outcome::Unknown;
        }
        transfer_captured_variable_outputs(state, &[target], source_offset);
    }
    outcome
}

pub(crate) fn transfer_resolved_invocation(
    state: &mut ResolveContext,
    statement: &Statement,
    tokens: &crate::ir::CommandTokens,
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
    registry: &CommandRegistry,
    output: (Option<&[usize]>, Option<&OriginalVariableInvocation>),
) {
    let (output_order, original) = output;
    let operands = if state.execution_name_policy.is_some() {
        let Some(operands) =
            original.filter(|operands| operands.argument_count() == arguments.len())
        else {
            state.widen();
            return;
        };
        Some(operands)
    } else {
        None
    };
    let mut writes = source_variable_write_places_with_output_order_and_original_operands_impl(
        facts,
        arguments,
        state,
        registry,
        output_order,
        operands,
    );
    detach_retained_destructions(
        state,
        facts,
        arguments,
        &mut writes,
        statement.span().start(),
        registry,
        operands,
    );
    invalidate_literal_writes(state, &writes);
    record_resolved_contents_writes(
        state,
        facts,
        arguments,
        registry,
        &writes,
        statement.span().start(),
        operands,
    );
    let operation = invocation_write_trace_operation(facts);
    let trace_callback = crate::place_bridge::def_places(statement, state, registry)
        .into_iter()
        .any(|place| project_access(place, state, operation).observed)
        || source_variable_read_places_with_original_operands_impl(
            facts, arguments, state, registry, operands,
        )
        .iter()
        .any(|place| place.observed);
    transfer_resolved_state_transitions(
        state,
        facts,
        registry,
        tokens.evaluated_body().is_some(),
        statement.span().start(),
        operands,
    );
    if facts.traits.contains(Traits::DESTROYS_VARIABLE) {
        if operands.is_some() {
            for place in &writes {
                destroy_captured_cell(state, place, statement.span().start(), registry);
            }
        } else {
            destroy_invocation_roots(state, facts, arguments, statement.span().start(), registry);
        }
    }
    if let Some(operands) = operands {
        establish_resolved_written_lifetimes(
            state,
            facts,
            arguments,
            registry,
            output_order,
            operands,
            statement.span().start(),
        );
    }
    let precise = precise_cell_effects(facts);
    apply_effects(
        state,
        &facts.effects,
        tokens.evaluated_body().is_some(),
        precise,
        trace_callback,
    );
}

fn record_resolved_contents_writes(
    state: &mut ResolveContext,
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
    registry: &CommandRegistry,
    writes: &[Place],
    source_offset: u32,
    operands: Option<&OriginalVariableInvocation>,
) {
    for place in writes {
        if facts.traits.contains(Traits::DESTROYS_VARIABLE)
            && place
                .index
                .as_ref()
                .is_some_and(|index| index.kind != place::IndexKind::Literal)
        {
            state.record_unknown_element_destruction(place);
            continue;
        }
        if !facts.traits.contains(Traits::DESTROYS_VARIABLE) || place.kind == PlaceKind::Unknown {
            state.record_contents_write(
                place,
                source_offset,
                conditional_contents_write(facts, place, state)
                    && !normal_output_is_written_impl(
                        facts, arguments, place, state, registry, operands,
                    ),
            );
        }
    }
}

fn transfer_resolved_state_transitions(
    state: &mut ResolveContext,
    facts: &tcl_registry::InvocationFacts,
    registry: &CommandRegistry,
    script_interpreted: bool,
    source_offset: u32,
    operands: Option<&OriginalVariableInvocation>,
) {
    match &facts.state_transitions {
        StateTransitionKnowledge::UnknownInvocation => {}
        StateTransitionKnowledge::Declared(transitions) => {
            for fact in transitions.facts() {
                if script_interpreted && matches!(fact.transition, StateTransition::Widen(_)) {
                    continue;
                }
                match (&fact.transition, operands) {
                    (StateTransition::VariableCellAlias(alias), Some(operands)) => {
                        let local = alias
                            .local
                            .argument_index()
                            .and_then(|index| operands.compiled_local(index));
                        bind_alias_with_compiled_local(
                            state,
                            alias,
                            registry,
                            local,
                            Some(operands),
                        );
                    }
                    (StateTransition::Trace(trace), Some(operands)) => {
                        apply_trace_with_original(state, trace, registry, Some(operands));
                    }
                    _ => apply_transition(state, &fact.transition, registry, source_offset),
                }
            }
        }
    }
}

fn establish_resolved_written_lifetimes(
    state: &mut ResolveContext,
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
    registry: &CommandRegistry,
    output_order: Option<&[usize]>,
    operands: &OriginalVariableInvocation,
    source_offset: u32,
) {
    let definitions = source_variable_definitions_with_original_operands(
        facts,
        arguments,
        state,
        registry,
        output_order,
        operands,
    );
    for (_, target) in definitions {
        if conditional_contents_write(facts, &target, state)
            && !normal_output_is_written_impl(
                facts,
                arguments,
                &target,
                state,
                registry,
                Some(operands),
            )
        {
            continue;
        }
        if target.observed || target.kind == PlaceKind::Unknown {
            state.widen();
        } else {
            establish_written_lifetime(state, &target, source_offset);
            if target.is_global() && !target.dynamic {
                state.namespace_cells.present.insert(cell_key(&target));
            }
        }
    }
}

fn destroy_invocation_roots(
    state: &mut ResolveContext,
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
    source: u32,
    registry: &CommandRegistry,
) {
    let mut touched = false;
    for &(index, role) in &facts.arg_roles {
        if role != tcl_registry::ArgRole::VarWrite {
            continue;
        }
        touched = true;
        if let Some(name) = arguments.literal_at(facts.argument_offset + usize::from(index)) {
            destroy_root(state, name, source, registry);
        } else if arguments
            .array_element_root_at(facts.argument_offset + usize::from(index))
            .is_some()
        {
            let target = variable_operand_access(
                arguments,
                facts.argument_offset + usize::from(index),
                state,
                registry,
                tcl_registry::TraceOperation::Unset,
                false,
            );
            if target.observed {
                state.widen();
            } else {
                state.record_unknown_element_destruction(&target);
            }
        } else {
            state.widen();
        }
    }
    if !touched {
        state.widen();
    }
}

fn precise_cell_effects(facts: &tcl_registry::InvocationFacts) -> bool {
    (facts.arg_roles_complete
        && facts
            .arg_roles
            .iter()
            .any(|(_, role)| *role == tcl_registry::ArgRole::VarWrite))
        || facts
            .state_transitions
            .declared()
            .is_some_and(|transitions| {
                transitions.facts().iter().any(|fact| {
                    matches!(
                        fact.transition,
                        StateTransition::VariableCellAlias(_)
                            | StateTransition::Trace(_)
                            | StateTransition::Namespace(NamespaceTransition::Delete { .. })
                    )
                })
            })
}

fn apply_effects(
    state: &mut ResolveContext,
    effects: &tcl_registry::EffectFootprint,
    body_expanded: bool,
    precise: bool,
    trace_callback: bool,
) {
    use tcl_registry::world_effect::{CallbackKinds, EffectAccessMode, WorldStateDomain};
    let callback = effects.callback();
    let trace_only = callback.kinds == CallbackKinds::TRACE;
    if effects.requires_world_barrier() && !body_expanded && (!trace_only || trace_callback) {
        state.widen();
        return;
    }
    for access in effects.accesses() {
        if access.mode == EffectAccessMode::Read {
            continue;
        }
        match access.domain {
            WorldStateDomain::VariableTraces if !precise => {
                state.mark_unenumerated_variable_observers();
            }
            WorldStateDomain::VariableStore if !precise => {
                for generation in state.generations.values_mut() {
                    *generation = CellGeneration::Unknown;
                }
            }
            _ => {}
        }
    }
}

pub(super) fn apply_transition(
    state: &mut ResolveContext,
    transition: &StateTransition,
    registry: &CommandRegistry,
    source_offset: u32,
) {
    match transition {
        StateTransition::VariableCellAlias(alias) => bind_alias(state, alias, registry),
        StateTransition::Trace(trace) => apply_trace(state, trace, registry),
        StateTransition::Namespace(namespace) => apply_namespace(state, namespace, source_offset),
        StateTransition::Widen(widening)
            if widening.domains.iter().any(|domain| {
                matches!(
                    domain,
                    StateTransitionDomain::VariableCells
                        | StateTransitionDomain::Namespaces
                        | StateTransitionDomain::VariableTraces
                )
            }) =>
        {
            state.widen();
        }
        _ => {}
    }
}

pub(super) fn conditional_contents_write(
    facts: &tcl_registry::InvocationFacts,
    place: &Place,
    state: &ResolveContext,
) -> bool {
    facts.traits.contains(Traits::CONDITIONAL_VARIABLE_WRITE)
        && !(facts.successful_handler
            == Some(
                tcl_registry::native_compilation::SuccessfulHandlerSpec::InitialiseEmptyVariable,
            )
            && !place.observed
            && state.contents_presence(place) == crate::var_resolve::ContentsPresence::Undefined)
}

fn normal_output_is_written_impl(
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
    target: &Place,
    state: &ResolveContext,
    registry: &CommandRegistry,
    operands: Option<&OriginalVariableInvocation>,
) -> bool {
    if target.observed || target.dynamic || target.cell.is_none() {
        return false;
    }
    facts
        .successful_variable_output_commitments(arguments)
        .is_some_and(|commitments| {
            commitments.into_iter().any(|(index, commitment)| {
                commitment == tcl_registry::variable_output::VariableOutputCommitment::Written
                    && variable_output_operand_access_with_original_operands_impl(
                        facts, arguments, index, state, registry, operands,
                    ) == *target
            })
        })
}
