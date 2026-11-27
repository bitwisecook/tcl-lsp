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

//! The cells a resolved source invocation reads, writes or destroys, with the lookup order of its outputs.

use super::OriginalVariableInvocation;
use super::alias_targets::{alias_target, original_alias_target};
use super::namespace_retirement::namespace_destruction_from_facts;
use crate::var_resolve::{
    ResolveContext, project_access, resolve_literal_access, resolve_literal_place,
};
use tcl_names::place;
use tcl_names::place::{Place, PlaceKind};
use tcl_registry::{
    CommandRegistry, NamespaceTransition, StateTransition, TraceTarget, TraceTransition, Traits,
    VariableCellAliasTransition,
};

/// Whether a variable-name role is an actual contents write rather than a
/// registry-owned binding or trace-registration operand.
#[must_use]
pub(crate) fn contents_write_operand(
    facts: &tcl_registry::InvocationFacts,
    literal: Option<&str>,
) -> bool {
    let Some(transitions) = facts.state_transitions.declared() else {
        return true;
    };
    let mut binding_only = false;
    for fact in transitions.facts() {
        match &fact.transition {
            StateTransition::Trace(
                TraceTransition::Add {
                    target: TraceTarget::Variable(variable),
                    ..
                }
                | TraceTransition::Remove {
                    target: TraceTarget::Variable(variable),
                    ..
                },
            ) if variable.literal() == literal => binding_only = true,
            StateTransition::VariableCellAlias(alias) if alias.local.literal() == literal => {
                if alias.writes_value {
                    return true;
                }
                binding_only = true;
            }
            _ => {}
        }
    }
    !binding_only
}

pub(super) fn contents_write_operand_impl(
    facts: &tcl_registry::InvocationFacts,
    literal: Option<&str>,
    index: usize,
    operands: Option<&OriginalVariableInvocation>,
) -> bool {
    if facts.variable_receiver_operand_form(index)
        == Some(tcl_registry::resolved_invocation::VariableReceiverOperandForm::TraceSubject)
    {
        return false;
    }
    if operands.is_none() {
        return contents_write_operand(facts, literal);
    }
    let mut binding_only = false;
    for fact in facts
        .state_transitions
        .declared()
        .into_iter()
        .flat_map(tcl_registry::StateTransitions::facts)
    {
        match &fact.transition {
            StateTransition::Trace(
                TraceTransition::Add {
                    target: TraceTarget::Variable(variable),
                    ..
                }
                | TraceTransition::Remove {
                    target: TraceTarget::Variable(variable),
                    ..
                },
            ) if variable.argument_index() == Some(index) => binding_only = true,
            StateTransition::VariableCellAlias(alias)
                if alias.local.argument_index() == Some(index) =>
            {
                if alias.writes_value {
                    return true;
                }
                binding_only = true;
            }
            _ => {}
        }
    }
    !binding_only
}

/// Named cells read by a proved getter or the read side of a cell update.
/// Literal argument values are already frozen by the caller's invocation owner.
#[must_use]
pub fn source_variable_read_places(
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
    state: &ResolveContext,
    registry: &CommandRegistry,
) -> Vec<Place> {
    source_variable_read_places_with_original_operands_impl(facts, arguments, state, registry, None)
}

pub(crate) fn source_variable_read_places_with_original_operands(
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
    state: &ResolveContext,
    registry: &CommandRegistry,
    operands: &OriginalVariableInvocation,
) -> Vec<Place> {
    source_variable_read_places_with_original_operands_impl(
        facts,
        arguments,
        state,
        registry,
        Some(operands),
    )
}

pub(super) fn source_variable_read_places_with_original_operands_impl(
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
    state: &ResolveContext,
    registry: &CommandRegistry,
    operands: Option<&OriginalVariableInvocation>,
) -> Vec<Place> {
    if facts
        .state_transitions
        .declared()
        .into_iter()
        .flat_map(tcl_registry::StateTransitions::facts)
        .any(|fact| matches!(fact.transition, StateTransition::VariableCellAlias(_)))
    {
        return Vec::new();
    }
    facts
        .arg_roles
        .iter()
        .filter_map(|&(index, role)| {
            if role != tcl_registry::ArgRole::VarRead
                && !(role == tcl_registry::ArgRole::VarWrite
                    && facts.traits.contains(Traits::READS_BEFORE_WRITE))
            {
                return None;
            }
            let index = facts.argument_offset + usize::from(index);
            if facts.variable_receiver_operand_form(index)
                == Some(
                    tcl_registry::resolved_invocation::VariableReceiverOperandForm::TraceSubject,
                )
            {
                return None;
            }
            Some(variable_operand_access_impl(
                arguments,
                index,
                state,
                registry,
                tcl_registry::TraceOperation::Read,
                facts.traits.contains(Traits::WHOLE_ARRAY_ARG),
                operands,
            ))
        })
        .collect()
}

pub(crate) fn variable_output_operand_access(
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
    index: usize,
    state: &ResolveContext,
    registry: &CommandRegistry,
) -> Place {
    variable_output_operand_access_with_original_operands_impl(
        facts, arguments, index, state, registry, None,
    )
}

pub(crate) fn variable_output_operand_access_with_original_operands(
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
    index: usize,
    state: &ResolveContext,
    registry: &CommandRegistry,
    operands: &OriginalVariableInvocation,
) -> Place {
    variable_output_operand_access_with_original_operands_impl(
        facts,
        arguments,
        index,
        state,
        registry,
        Some(operands),
    )
}

/// Actual normal value-definition receivers at their effective operand
/// ordinals. Trace registrations, binding-only declarations and destruction
/// remain independent from value definitions.
pub(crate) fn source_variable_definitions_with_original_operands(
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
    state: &ResolveContext,
    registry: &CommandRegistry,
    order: Option<&[usize]>,
    operands: &OriginalVariableInvocation,
) -> Vec<(usize, Place)> {
    if facts.traits.contains(Traits::DESTROYS_VARIABLE) {
        return Vec::new();
    }
    let uncertain = uncertain_variable_output_addresses_with_original_operands_impl(
        facts,
        arguments,
        state,
        registry,
        order,
        Some(operands),
    );
    facts
        .arg_roles
        .iter()
        .filter_map(|&(index, role)| {
            let index = facts.argument_offset + usize::from(index);
            if role != tcl_registry::ArgRole::VarWrite
                || !contents_write_operand_impl(
                    facts,
                    arguments.literal_at(index),
                    index,
                    Some(operands),
                )
            {
                return None;
            }
            Some((
                index,
                if uncertain.contains(&index) {
                    project_access(
                        place::unknown_top(),
                        state,
                        tcl_registry::TraceOperation::Write,
                    )
                } else {
                    variable_output_operand_access_with_original_operands(
                        facts, arguments, index, state, registry, operands,
                    )
                },
            ))
        })
        .collect()
}

pub(super) fn variable_output_operand_access_with_original_operands_impl(
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
    index: usize,
    state: &ResolveContext,
    registry: &CommandRegistry,
    operands: Option<&OriginalVariableInvocation>,
) -> Place {
    let operation = if facts.traits.contains(Traits::DESTROYS_VARIABLE) {
        tcl_registry::TraceOperation::Unset
    } else {
        tcl_registry::TraceOperation::Write
    };
    let name = arguments.literal_at(index);
    let alias = facts
        .state_transitions
        .declared()
        .into_iter()
        .flat_map(tcl_registry::StateTransitions::facts)
        .find_map(|fact| {
            if let StateTransition::VariableCellAlias(alias) = &fact.transition
                && alias.writes_value
                && operands.map_or_else(
                    || name.is_some() && alias.local.literal() == name,
                    |_| alias.local.argument_index() == Some(index),
                )
            {
                Some(alias)
            } else {
                None
            }
        });
    alias.map_or_else(
        || {
            variable_operand_access_impl(
                arguments,
                index,
                state,
                registry,
                operation,
                facts.traits.contains(Traits::WHOLE_ARRAY_ARG),
                operands,
            )
        },
        |alias| {
            project_access(
                operands.map_or_else(
                    || alias_target(state, &alias.target, registry),
                    |operands| {
                        original_alias_target(state, &alias.target, operands, registry, false)
                    },
                ),
                state,
                operation,
            )
        },
    )
}

/// Effective output indices whose address may change after an earlier observer.
/// `order` is an independently selected native protocol; absence retains every
/// possible ordering. This does not grant dispatch, compiler or value evidence.
#[must_use]
pub fn uncertain_variable_output_addresses(
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
    state: &ResolveContext,
    registry: &CommandRegistry,
    order: Option<&[usize]>,
) -> std::collections::BTreeSet<usize> {
    uncertain_variable_output_addresses_with_original_operands_impl(
        facts, arguments, state, registry, order, None,
    )
}

pub(super) fn uncertain_variable_output_addresses_with_original_operands_impl(
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
    state: &ResolveContext,
    registry: &CommandRegistry,
    order: Option<&[usize]>,
    operands: Option<&OriginalVariableInvocation>,
) -> std::collections::BTreeSet<usize> {
    let outputs: Vec<_> = facts
        .arg_roles
        .iter()
        .filter_map(|&(index, role)| {
            let index = facts.argument_offset + usize::from(index);
            (role == tcl_registry::ArgRole::VarWrite
                && output_commitment(facts, arguments, index)
                    != Some(tcl_registry::variable_output::VariableOutputCommitment::Unchanged)
                && contents_write_operand_impl(facts, arguments.literal_at(index), index, operands))
            .then(|| {
                (
                    index,
                    variable_output_operand_access_with_original_operands_impl(
                        facts, arguments, index, state, registry, operands,
                    ),
                )
            })
        })
        .collect();
    let may_observe = |index: usize| {
        outputs.iter().any(|(argument, place)| {
            *argument == index && (place.observed || place.kind == PlaceKind::Unknown)
        })
    };
    let order = order.filter(|order| outputs.iter().all(|(index, _)| order.contains(index)));
    outputs
        .iter()
        .filter_map(|(index, _)| {
            let exposed = order.map_or_else(
                || {
                    outputs
                        .iter()
                        .any(|(other, _)| other != index && may_observe(*other))
                },
                |order| {
                    order
                        .iter()
                        .take_while(|other| *other != index)
                        .any(|other| may_observe(*other))
                },
            );
            exposed.then_some(*index)
        })
        .collect()
}

pub(super) fn default_variable_output_order(
    facts: &tcl_registry::InvocationFacts,
) -> Option<Vec<usize>> {
    use tcl_registry::native_compilation::VariableOutputLookup;
    let indices: Vec<_> = facts
        .arg_roles
        .iter()
        .filter_map(|&(index, role)| {
            (role == tcl_registry::ArgRole::VarWrite)
                .then_some(facts.argument_offset + usize::from(index))
        })
        .collect();
    let selected = facts
        .successful_handler
        .map(tcl_registry::native_compilation::SuccessfulHandlerSpec::variable_output_lookup);
    (indices.len() <= 1
        || matches!(
            selected,
            Some(VariableOutputLookup::Sequential | VariableOutputLookup::SingleTarget)
        ))
    .then_some(indices)
}

/// Bound changes with an independently selected native output lookup order.
#[must_use]
pub fn source_variable_write_places_with_output_order(
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
    state: &ResolveContext,
    registry: &CommandRegistry,
    order: Option<&[usize]>,
) -> Vec<Place> {
    source_variable_write_places_with_output_order_and_original_operands_impl(
        facts, arguments, state, registry, order, None,
    )
}

pub(crate) fn source_variable_write_places_with_output_order_and_original_operands(
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
    state: &ResolveContext,
    registry: &CommandRegistry,
    order: Option<&[usize]>,
    operands: &OriginalVariableInvocation,
) -> Vec<Place> {
    source_variable_write_places_with_output_order_and_original_operands_impl(
        facts,
        arguments,
        state,
        registry,
        order,
        Some(operands),
    )
}

pub(super) fn source_variable_write_places_with_output_order_and_original_operands_impl(
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
    state: &ResolveContext,
    registry: &CommandRegistry,
    order: Option<&[usize]>,
    operands: Option<&OriginalVariableInvocation>,
) -> Vec<Place> {
    let uncertain = uncertain_variable_output_addresses_with_original_operands_impl(
        facts, arguments, state, registry, order, operands,
    );
    let unknown_targets: Vec<_> = uncertain
        .into_iter()
        .map(|index| {
            variable_output_operand_access_with_original_operands_impl(
                facts, arguments, index, state, registry, operands,
            )
        })
        .collect();
    unordered_source_variable_write_places(facts, arguments, state, registry, operands)
        .into_iter()
        .map(|place| {
            if unknown_targets.contains(&place) {
                project_access(
                    place::unknown_top(),
                    state,
                    tcl_registry::TraceOperation::Write,
                )
            } else {
                place
            }
        })
        .collect()
}

/// Bound cells written or destroyed by one already-resolved source invocation.
/// Role indices refer to the effective argument vector, including alias prefixes.
#[must_use]
pub fn source_variable_write_places(
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
    state: &ResolveContext,
    registry: &CommandRegistry,
) -> Vec<Place> {
    let order = default_variable_output_order(facts);
    source_variable_write_places_with_output_order(
        facts,
        arguments,
        state,
        registry,
        order.as_deref(),
    )
}

pub(crate) fn source_variable_write_places_with_original_operands(
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
    state: &ResolveContext,
    registry: &CommandRegistry,
    operands: &OriginalVariableInvocation,
) -> Vec<Place> {
    let order = default_variable_output_order(facts);
    source_variable_write_places_with_output_order_and_original_operands(
        facts,
        arguments,
        state,
        registry,
        order.as_deref(),
        operands,
    )
}

pub(super) fn raw_static_unset_operand_error(
    arguments: tcl_registry::InvocationArguments<'_>,
    index: usize,
    state: &ResolveContext,
    registry: &CommandRegistry,
    operands: Option<&OriginalVariableInvocation>,
) -> bool {
    operands.map_or_else(
        || {
            arguments
                .literal_at(index)
                .is_some_and(|name| state.raw_static_unset_error(name, registry))
        },
        |operands| {
            operands
                .raw_unset_slot(index, state, registry)
                .is_some_and(|slot| state.raw_static_unset_error_at_slot(&slot))
        },
    )
}

fn unordered_source_variable_write_places(
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
    state: &ResolveContext,
    registry: &CommandRegistry,
    operands: Option<&OriginalVariableInvocation>,
) -> Vec<Place> {
    let mut writes = namespace_destruction_from_facts(facts, state);
    let aliases = invocation_variable_aliases(facts);
    for alias in &aliases {
        if alias.writes_value {
            writes.push(operands.map_or_else(
                || alias_target(state, &alias.target, registry),
                |operands| original_alias_target(state, &alias.target, operands, registry, false),
            ));
        }
    }
    for &(index, role) in &facts.arg_roles {
        if role != tcl_registry::ArgRole::VarWrite {
            continue;
        }
        if output_commitment(facts, arguments, facts.argument_offset + usize::from(index))
            == Some(tcl_registry::variable_output::VariableOutputCommitment::Unchanged)
        {
            continue;
        }
        let index = facts.argument_offset + usize::from(index);
        let literal = arguments.literal_at(index);
        if facts.traits.contains(Traits::DESTROYS_VARIABLE)
            && raw_static_unset_operand_error(arguments, index, state, registry, operands)
        {
            // A direct Jim static fallback is not an own local table entry.
            // Unset reports missing (or succeeds under -nocomplain) without
            // destroying the retained VarVal.
            continue;
        }
        if !contents_write_operand_impl(facts, literal, index, operands) {
            continue;
        }
        if aliases.iter().any(|alias| {
            !alias.writes_value
                && operands.map_or_else(
                    || alias.local.literal() == literal,
                    |_| alias.local.argument_index() == Some(index),
                )
        }) {
            continue;
        }
        let target = source_write_operand_target(
            facts, arguments, index, state, registry, operands, &aliases,
        );
        let target = project_access(
            target,
            state,
            if facts.traits.contains(Traits::DESTROYS_VARIABLE) {
                tcl_registry::TraceOperation::Unset
            } else {
                tcl_registry::TraceOperation::Write
            },
        );
        if !empty_initialisation_writes(facts, &target, state) {
            continue;
        }
        if !writes.contains(&target) {
            writes.push(target);
        }
    }
    if !facts.arg_roles_complete
        && facts.effects.accesses().iter().any(|access| {
            access.domain == tcl_registry::WorldStateDomain::VariableStore
                && access.mode != tcl_registry::EffectAccessMode::Read
        })
    {
        writes.push(place::unknown_top());
    }
    scoped_world_write_places(facts, state, registry, &mut writes);
    let operation = invocation_write_trace_operation(facts);
    writes
        .into_iter()
        .map(|place| project_access(place, state, operation))
        .collect()
}

fn source_write_operand_target(
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
    index: usize,
    state: &ResolveContext,
    registry: &CommandRegistry,
    operands: Option<&OriginalVariableInvocation>,
    aliases: &[&VariableCellAliasTransition],
) -> Place {
    let literal = arguments.literal_at(index);
    if let Some(operands) = operands {
        variable_output_operand_access_with_original_operands_impl(
            facts,
            arguments,
            index,
            state,
            registry,
            Some(operands),
        )
    } else {
        literal.map_or_else(
            || {
                variable_operand_access(
                    arguments,
                    index,
                    state,
                    registry,
                    tcl_registry::TraceOperation::Write,
                    facts.traits.contains(Traits::WHOLE_ARRAY_ARG),
                )
            },
            |name| {
                if let Some(alias) = aliases
                    .iter()
                    .find(|alias| alias.local.literal() == Some(name))
                {
                    alias_target(state, &alias.target, registry)
                } else {
                    resolve_literal_place(
                        name,
                        state,
                        facts.traits.contains(Traits::WHOLE_ARRAY_ARG),
                        registry,
                    )
                }
            },
        )
    }
}

pub(super) fn invocation_write_trace_operation(
    facts: &tcl_registry::InvocationFacts,
) -> tcl_registry::TraceOperation {
    if facts.traits.contains(Traits::DESTROYS_VARIABLE)
        || facts
            .state_transitions
            .declared()
            .is_some_and(|transitions| {
                transitions.facts().iter().any(|fact| {
                    matches!(
                        fact.transition,
                        StateTransition::Namespace(NamespaceTransition::Delete { .. })
                    )
                })
            })
    {
        tcl_registry::TraceOperation::Unset
    } else {
        tcl_registry::TraceOperation::Write
    }
}

fn invocation_variable_aliases(
    facts: &tcl_registry::InvocationFacts,
) -> Vec<&VariableCellAliasTransition> {
    facts
        .state_transitions
        .declared()
        .into_iter()
        .flat_map(tcl_registry::StateTransitions::facts)
        .filter_map(|fact| match &fact.transition {
            StateTransition::VariableCellAlias(alias) => Some(alias),
            _ => None,
        })
        .collect()
}

fn empty_initialisation_writes(
    facts: &tcl_registry::InvocationFacts,
    target: &Place,
    state: &ResolveContext,
) -> bool {
    if facts.successful_handler
        != Some(tcl_registry::native_compilation::SuccessfulHandlerSpec::InitialiseEmptyVariable)
    {
        return true;
    }
    let read = project_access(target.clone(), state, tcl_registry::TraceOperation::Read);
    read.observed || state.contents_presence(&read) != crate::var_resolve::ContentsPresence::Defined
}

/// Resolve an already evaluated variable-name operand, retaining a bounded array shape.
#[must_use]
pub fn variable_operand_access(
    arguments: tcl_registry::InvocationArguments<'_>,
    index: usize,
    context: &ResolveContext,
    registry: &CommandRegistry,
    operation: tcl_registry::TraceOperation,
    whole_array: bool,
) -> Place {
    if let Some(name) = arguments.literal_at(index) {
        resolve_literal_access(name, context, whole_array, registry, operation)
    } else if let Some(root) = arguments.array_element_root_at(index) {
        crate::var_resolve::resolve_array_element_root(root, context, registry, operation)
    } else {
        project_access(place::unknown_top(), context, operation)
    }
}

pub(super) fn variable_operand_access_impl(
    arguments: tcl_registry::InvocationArguments<'_>,
    index: usize,
    context: &ResolveContext,
    registry: &CommandRegistry,
    operation: tcl_registry::TraceOperation,
    whole_array: bool,
    operands: Option<&OriginalVariableInvocation>,
) -> Place {
    operands.map_or_else(
        || variable_operand_access(arguments, index, context, registry, operation, whole_array),
        |operands| operands.access(index, context, registry, operation, whole_array),
    )
}

fn scoped_world_write_places(
    facts: &tcl_registry::InvocationFacts,
    state: &ResolveContext,
    registry: &CommandRegistry,
    writes: &mut Vec<Place>,
) {
    use tcl_registry::world_effect::{
        EffectAccessMode, InterpreterScope, NamespaceScope, SubjectScope, WorldStateDomain,
    };
    for access in facts.effects.accesses() {
        if access.domain != WorldStateDomain::VariableStore || access.mode == EffectAccessMode::Read
        {
            continue;
        }
        let namespace = match &access.namespace {
            NamespaceScope::Named(namespace) => Some(namespace.as_ref()),
            _ => None,
        };
        if let Some(namespace) = namespace {
            let target = if access.interpreter == InterpreterScope::Current {
                if let Some(identity) = state.namespace_identity_for_written(namespace) {
                    match &access.subject {
                        SubjectScope::Named(name) => {
                            state.namespace_place_in_identity(name, &identity, false)
                        }
                        SubjectScope::Wildcard => {
                            let mut target =
                                state.namespace_place_in_identity("", &identity, false);
                            target.kind = PlaceKind::Unknown;
                            target
                        }
                    }
                } else if state.namespace_identity.is_some() {
                    place::unknown_top()
                } else {
                    match &access.subject {
                        SubjectScope::Named(name) => resolve_literal_place(
                            &tcl_syntax::naming::qualify(namespace, name),
                            state,
                            false,
                            registry,
                        ),
                        SubjectScope::Wildcard => place::unknown_namespace(namespace),
                    }
                }
            } else {
                place::unknown_top()
            };
            if !writes.contains(&target) {
                writes.push(target);
            }
        }
    }
}

pub(super) fn output_commitment(
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
    index: usize,
) -> Option<tcl_registry::variable_output::VariableOutputCommitment> {
    facts
        .successful_variable_output_commitments(arguments)?
        .into_iter()
        .find_map(|(argument, commitment)| (argument == index).then_some(commitment))
}
