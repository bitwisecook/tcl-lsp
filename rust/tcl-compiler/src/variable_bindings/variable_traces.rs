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

//! Variable-trace registrations: whether one is closed, and applying a trace transition to the retained observers.

use super::OriginalVariableInvocation;
use crate::var_resolve::{ResolveContext, cell_key, resolve_literal_place, trace_key};
use tcl_names::place;
use tcl_names::place::{CellGeneration, CellOwner, Place, PlaceKind};
use tcl_registry::{CommandRegistry, StateTransition, TraceTarget, TraceTransition};

/// A selected variable-trace registration never invokes its stored prefix.
/// Valid removal is a no-op for missing or incompatible variable names; adding
/// still requires a live physical destination with a valid namespace/root kind.
#[cfg(test)]
pub(super) fn variable_trace_registration_is_closed(
    state: &ResolveContext,
    facts: &tcl_registry::InvocationFacts,
    registry: &CommandRegistry,
) -> bool {
    variable_trace_registration_is_closed_impl(state, facts, registry, None)
}

pub(crate) fn variable_trace_registration_is_closed_with_original_operands(
    state: &ResolveContext,
    facts: &tcl_registry::InvocationFacts,
    registry: &CommandRegistry,
    operands: &OriginalVariableInvocation,
) -> bool {
    variable_trace_registration_is_closed_impl(state, facts, registry, Some(operands))
}

fn variable_trace_registration_is_closed_impl(
    state: &ResolveContext,
    facts: &tcl_registry::InvocationFacts,
    registry: &CommandRegistry,
    operands: Option<&OriginalVariableInvocation>,
) -> bool {
    if state.invocation_dialect.is_none_or(|dialect| {
        dialect.tcl_version.is_none()
            || dialect.variable_lookup_policy != Some(tcl_dialect::VariableLookupPolicy::Tcl)
    }) {
        return false;
    }
    let Some(transitions) = facts.state_transitions.declared() else {
        return false;
    };
    let [fact] = transitions.facts() else {
        return false;
    };
    let StateTransition::Trace(trace) = &fact.transition else {
        return false;
    };
    let (target, operations, added) = match trace {
        TraceTransition::Add {
            target, operations, ..
        } => (target, operations, true),
        TraceTransition::Remove {
            target, operations, ..
        } => (target, operations, false),
    };
    let TraceTarget::Variable(variable) = target else {
        return false;
    };
    let tcl_registry::TraceOperationSet::Known(operations) = operations else {
        return false;
    };
    if operations.is_empty() {
        return false;
    }
    if !added {
        return true;
    }
    let mut target = operands.map_or_else(
        || {
            variable.literal().map_or_else(place::unknown_top, |name| {
                resolve_literal_place(name, state, false, registry)
            })
        },
        |operands| {
            variable
                .argument_index()
                .map_or_else(place::unknown_top, |index| {
                    operands.trace_subject_access(index, state, registry)
                })
        },
    );
    target.observed = false;
    // Registering a trace on an array root does not perform a scalar store.
    // An indexed target still requires an actual array receiver.
    if target.dynamic
        || target.kind == PlaceKind::Unknown
        || (target.index.is_some() && state.store_would_error(&target))
    {
        return false;
    }
    if target.index.is_some()
        && state.root_contents_kind(&target).is_none()
        && state.contents_presence(&target.base())
            != crate::var_resolve::ContentsPresence::Undefined
    {
        return false;
    }
    target.cell.as_ref().is_some_and(|cell| {
        cell.generation != CellGeneration::Unknown
            && !matches!(cell.owner, CellOwner::SelectedFrame(_))
            && (!target.is_global()
                || operands.map_or_else(
                    || state.known_namespaces.contains(&target.ns),
                    |_| {
                        state.namespace_footprint(&target).is_some_and(|namespace| {
                            state.namespace_addressable_identities.contains(&namespace)
                        })
                    },
                ))
    })
}

pub(super) fn apply_trace(
    state: &mut ResolveContext,
    trace: &TraceTransition,
    registry: &CommandRegistry,
) {
    apply_trace_with_original(state, trace, registry, None);
}

pub(super) fn apply_trace_with_original(
    state: &mut ResolveContext,
    trace: &TraceTransition,
    registry: &CommandRegistry,
    operands: Option<&OriginalVariableInvocation>,
) {
    let (target, operations, prefix, added) = match trace {
        TraceTransition::Add {
            target,
            operations,
            prefix,
        } => (target, operations, prefix, true),
        TraceTransition::Remove {
            target,
            operations,
            prefix,
        } => (target, operations, prefix, false),
    };
    let TraceTarget::Variable(variable) = target else {
        return;
    };
    let target = operands.map_or_else(
        || {
            variable.literal().map_or_else(place::unknown_top, |name| {
                resolve_literal_place(name, state, false, registry)
            })
        },
        |operands| {
            variable
                .argument_index()
                .map_or_else(place::unknown_top, |index| {
                    operands.trace_subject_access(index, state, registry)
                })
        },
    );
    let prefix = operands.map_or_else(
        || {
            prefix
                .literal()
                .map(|value| crate::var_resolve::VariableTracePrefix::Authored(value.to_owned()))
        },
        |operands| {
            crate::var_resolve::VariableTracePrefix::copied_original(
                operands.subject_input(prefix, state)?,
                state,
            )
        },
    );
    let registration = match (operations, prefix) {
        (tcl_registry::TraceOperationSet::Known(operations), Some(prefix)) => {
            Some((operations.clone(), prefix))
        }
        _ => None,
    };
    if target.kind == PlaceKind::Unknown {
        retain_possible_trace_registration(state, target, registration, added);
        return;
    }
    let key = trace_key(&target);
    if added && target.is_global() && !target.dynamic {
        state.namespace_cells.present.insert(cell_key(&target));
    }
    update_trace_registration(state, &key, registration, added);
    let observed = state.untracked_traces.contains(&key)
        || state
            .trace_registrations
            .get(&key)
            .is_some_and(|registrations| !registrations.is_empty());
    if observed {
        state.traced.insert(key.clone());
        state
            .trace_registration_receivers
            .insert(key.clone(), std::sync::Arc::new(target.clone()));
    } else {
        state.traced.remove(&key);
        state.trace_registration_receivers.remove(&key);
    }
    let aliases = state
        .alias_bindings
        .iter()
        .filter(|(_, alias)| place::overlap(alias, &target))
        .map(|(key, alias)| {
            (
                key.clone(),
                observed || state.unenumerated_observers_may_run(alias),
            )
        })
        .collect::<Vec<_>>();
    for (key, observed) in aliases {
        state
            .alias_bindings
            .get_mut(&key)
            .expect("selected alias")
            .observed = observed;
    }
}

fn retain_possible_trace_registration(
    state: &mut ResolveContext,
    target: Place,
    registration: Option<(
        Vec<tcl_registry::TraceOperation>,
        crate::var_resolve::VariableTracePrefix,
    )>,
    added: bool,
) {
    if added {
        if let Some((operations, prefix)) = registration {
            state.possible_trace_registrations.push(
                crate::var_resolve::PossibleVariableTraceRegistration {
                    target,
                    operations,
                    prefix,
                },
            );
        } else {
            state.mark_unenumerated_variable_observers();
        }
    }
}

fn update_trace_registration(
    state: &mut ResolveContext,
    key: &tcl_names::variable::VariableCellKey,
    registration: Option<(
        Vec<tcl_registry::TraceOperation>,
        crate::var_resolve::VariableTracePrefix,
    )>,
    added: bool,
) {
    if let Some(registration) = registration {
        if state.traced.contains(key) && !state.trace_registrations.contains_key(key) {
            state.untracked_traces.insert(key.clone());
        }
        if added {
            state
                .trace_registrations
                .entry(key.clone())
                .or_default()
                .push(registration);
        } else {
            let selected = state
                .trace_registrations
                .get(key)
                .and_then(|registrations| {
                    registrations.iter().rposition(|item| {
                        item.0 == registration.0 && item.1.removal_matches(&registration.1, state)
                    })
                });
            if let Some(index) = selected {
                state
                    .trace_registrations
                    .get_mut(key)
                    .expect("selected trace inventory")
                    .remove(index);
            }
        }
    } else if added {
        state.untracked_traces.insert(key.clone());
    }
}
