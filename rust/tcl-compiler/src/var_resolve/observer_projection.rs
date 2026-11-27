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

//! The trace callbacks an access observes: registration keys and the ordered projection of known callbacks.

use super::canonical_keys::cell_key;
use super::{ActivationObserverInventory, ResolveContext, VariableTracePrefix};
use tcl_names::place;
use tcl_names::place::{CellGeneration, CellOwner, Place, PlaceKind};
use tcl_names::variable::VariableCellKey;
use tcl_registry::CommandRegistry;

/// Registration address of a trace; array elements retain their own indices.
#[must_use]
pub fn trace_key(place: &Place) -> VariableCellKey {
    let mut key = cell_key(place);
    if let Some(index) = &place.index {
        key = key.with_index(index.value.clone());
    }
    key
}

fn variable_observer_keys(
    bound: &Place,
    ctx: &ResolveContext,
    operation: tcl_registry::TraceOperation,
) -> Vec<VariableCellKey> {
    let root = cell_key(bound);
    let mut candidates = vec![root.clone()];
    if let Some(index) = &bound.index {
        if index.kind == place::IndexKind::Literal {
            candidates.push(trace_key(bound));
        } else {
            candidates.extend(
                ctx.traced
                    .iter()
                    .filter(|key| key.is_member_of(&root))
                    .cloned(),
            );
        }
    } else if bound.kind == PlaceKind::ArrayWhole
        || operation == tcl_registry::TraceOperation::Unset
    {
        candidates.extend(
            ctx.traced
                .iter()
                .filter(|key| key.is_member_of(&root))
                .cloned(),
        );
    }
    if bound.kind == PlaceKind::Unknown {
        candidates.extend(
            ctx.traced
                .iter()
                .filter(|key| {
                    bound.ns == place::LOCAL_NS
                        || ctx
                            .namespace_footprint(bound)
                            .is_none_or(|namespace| key.is_in_namespace(&namespace))
                })
                .cloned(),
        );
    }
    candidates
}

/// One registration selected from an actual physical variable access.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VariableObserverCallback {
    pub(crate) key: VariableCellKey,
    pub(crate) prefix: VariableTracePrefix,
}

/// Ordered known callbacks plus obligations that cannot be enumerated safely.
#[derive(Debug, Default)]
pub(crate) struct VariableObserverProjection {
    pub(crate) callbacks: Vec<VariableObserverCallback>,
    pub(crate) possible_callbacks: Vec<VariableTracePrefix>,
    pub(crate) unknown_residual: bool,
}

impl ResolveContext {
    /// Complete typed registration inventory for one proved array root. Missing
    /// receiver provenance cannot become an empty member callback inventory.
    pub(crate) fn array_registration_receivers_known(&self, root: &Place) -> bool {
        variable_observer_keys(root, self, tcl_registry::TraceOperation::Unset)
            .into_iter()
            .all(|key| {
                !self.untracked_traces.contains(&key)
                    && (!self.traced.contains(&key) && !self.trace_registrations.contains_key(&key)
                        || self
                            .trace_registration_receivers
                            .get(&key)
                            .is_some_and(|receiver| receiver.cell == root.cell))
            })
    }

    /// Select only registrations on this captured cell, excluding array members
    /// and containing-root registrations. Staged teardown owns those phases.
    pub(crate) fn variable_observers_on_cell(
        &self,
        access: &Place,
        operation: tcl_registry::TraceOperation,
        registry: &CommandRegistry,
    ) -> VariableObserverProjection {
        self.variable_observers_for_keys(
            access,
            operation,
            registry,
            vec![trace_key(access)],
            false,
        )
    }

    pub(crate) fn variable_observers_at(
        &self,
        access: &Place,
        operation: tcl_registry::TraceOperation,
        registry: &CommandRegistry,
    ) -> VariableObserverProjection {
        let keys = variable_observer_keys(access, self, operation);
        let unordered_destruction = operation == tcl_registry::TraceOperation::Unset
            && access.index.is_none()
            && keys.iter().any(|key| key != &cell_key(access));
        self.variable_observers_for_keys(access, operation, registry, keys, unordered_destruction)
    }

    pub(crate) fn unenumerated_observers_may_run(&self, access: &Place) -> bool {
        let closed_owner = match access.cell.as_ref().map(|cell| &cell.owner) {
            Some(CellOwner::Activation(identity)) => {
                self.activation_observers_closed() && self.activation.as_ref() == Some(identity)
            }
            Some(CellOwner::AllocatedInstance(allocation)) => self
                .closed_observer_allocations
                .contains(allocation.as_ref()),
            _ => false,
        };
        self.dynamic_traces && !closed_owner
    }

    /// An unresolved registration can select an existing caller or receiver.
    /// Revoke every affected fresh-owner receipt, not just the active frame.
    pub(crate) fn mark_unenumerated_variable_observers(&mut self) {
        self.invalidate_original_contents();
        self.dynamic_traces = true;
        self.activation_observer_inventory = ActivationObserverInventory::Open;
        self.closed_observer_allocations.clear();
        if let Some(caller) = &mut self.caller {
            std::sync::Arc::make_mut(caller).mark_unenumerated_variable_observers();
        }
    }

    fn variable_observers_for_keys(
        &self,
        access: &Place,
        operation: tcl_registry::TraceOperation,
        registry: &CommandRegistry,
        keys: Vec<VariableCellKey>,
        unordered_destruction: bool,
    ) -> VariableObserverProjection {
        let uncertain_address = access.kind == PlaceKind::Unknown
            || access
                .cell
                .as_ref()
                .is_none_or(|cell| cell.generation == CellGeneration::Unknown)
            || access
                .index
                .as_ref()
                .is_some_and(|index| index.kind != place::IndexKind::Literal);
        let mut projection = VariableObserverProjection {
            unknown_residual: self.unenumerated_observers_may_run(access)
                || self.authored_static_observer_may_run(access, operation)
                || self.implicit_read_changes_value(access, registry)
                    && operation == tcl_registry::TraceOperation::Read,
            ..VariableObserverProjection::default()
        };
        for key in keys {
            let Some(registrations) = self.trace_registrations.get(&key) else {
                projection.unknown_residual |=
                    self.traced.contains(&key) || self.untracked_traces.contains(&key);
                continue;
            };
            projection.unknown_residual |= self.untracked_traces.contains(&key);
            for (operations, prefix) in registrations.iter().rev() {
                if operations.contains(&operation) {
                    projection.unknown_residual |= uncertain_address || unordered_destruction;
                    projection.callbacks.push(VariableObserverCallback {
                        key: key.clone(),
                        prefix: prefix.clone(),
                    });
                }
            }
        }
        projection.possible_callbacks = self
            .possible_trace_registrations
            .iter()
            .filter(|registration| {
                registration.operations.contains(&operation)
                    && place::overlap(&registration.target, access)
            })
            .map(|registration| registration.prefix.clone())
            .collect();
        if !projection.callbacks.is_empty() || !projection.possible_callbacks.is_empty() {
            let dialect = self.invocation_dialect.or_else(|| {
                registry
                    .profile()
                    .map(tcl_native_model::InvocationDialect::of_profile)
            });
            projection.unknown_residual |=
                dialect.and_then(|dialect| dialect.tcl_version).is_none();
        }
        projection
    }
}

/// Observe only callbacks installed for this variable access operation.
#[must_use]
pub fn project_access(
    mut bound: Place,
    ctx: &ResolveContext,
    operation: tcl_registry::TraceOperation,
) -> Place {
    let candidates = variable_observer_keys(&bound, ctx, operation);
    bound.observed = ctx.unenumerated_observers_may_run(&bound)
        || ctx.authored_static_observer_may_run(&bound, operation)
        || ctx.possible_trace_registrations.iter().any(|registration| {
            registration.operations.contains(&operation)
                && place::overlap(&registration.target, &bound)
        })
        || candidates.iter().any(|key| {
            ctx.untracked_traces.contains(key)
                || ctx.trace_registrations.get(key).map_or_else(
                    || ctx.traced.contains(key),
                    |registrations| {
                        registrations
                            .iter()
                            .any(|(operations, _)| operations.contains(&operation))
                    },
                )
        });
    bound
}
