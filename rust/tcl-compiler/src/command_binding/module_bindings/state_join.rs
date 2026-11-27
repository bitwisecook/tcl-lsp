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

//! The join of two binding states: bindings, activation values, lookup support and history.

use super::super::{ModuleCommandBindings, PackageProvision};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use tcl_names::command::SourceCommandKey;

impl ModuleCommandBindings {
    /// Extend the retained-body inventory at the one copy-on-write mutation
    /// seam. Ordinary state forks keep sharing the module's original set;
    /// only a branch that recovers a previously-unseen procedure allocates.
    pub(in super::super) fn extend_procedure_bodies(
        &mut self,
        names: impl IntoIterator<Item = String>,
    ) -> bool {
        let additions: Vec<String> = names
            .into_iter()
            .filter(|name| !self.procedure_bodies.contains(name))
            .collect();
        if additions.is_empty() {
            return false;
        }
        let bodies = Arc::make_mut(&mut self.procedure_bodies);
        bodies.extend(additions);
        true
    }

    /// Join two sparse states sharing one immutable registry baseline.
    ///
    /// The boolean lets callers avoid publishing an observational history
    /// entry when a transition's alternatives made no lattice change.
    pub(in super::super) fn join(&mut self, other: &Self) -> bool {
        debug_assert!(
            Arc::ptr_eq(&self.baseline, &other.baseline),
            "a binding analysis may only join states from one registry baseline"
        );
        let mut changed = self.join_activation_values(other);
        if !Arc::ptr_eq(&self.original_command_world, &other.original_command_world)
            && self.original_command_world != other.original_command_world
        {
            changed |=
                Arc::make_mut(&mut self.original_command_world).join(&other.original_command_world);
        }
        changed |= self.join_logical_alias_prefixes(other);
        changed |= self.join_lookup_support(other);
        changed |= self.join_binding_history(other);
        if !Arc::ptr_eq(&self.bindings, &other.bindings) && self.bindings != other.bindings {
            // A name whose incoming alternatives this side already holds
            // (through one shared value, or by containment) joins to the
            // value it has, which `replace_bindings` leaves in place; only
            // the other names are joined, in the same ascending order.
            let keys: BTreeSet<SourceCommandKey> = self
                .bindings
                .keys_not_sharing_a_value(&other.bindings)
                .filter(|key| {
                    !matches!(
                        (self.bindings.get(*key), other.bindings.get(*key)),
                        (Some(held), Some(incoming)) if incoming.is_subset(held)
                    )
                })
                .cloned()
                .collect();
            let joined = keys.into_iter().map(|key| {
                let mut bindings = self.bindings.get(&key).cloned().unwrap_or_else(|| {
                    Self::unmodified_bindings(&key, self.baseline.semantics.binding_names())
                });
                bindings.extend(other.bindings.get(&key).cloned().unwrap_or_else(|| {
                    Self::unmodified_bindings(&key, self.baseline.semantics.binding_names())
                }));
                (key, bindings)
            });
            changed |= self.replace_bindings(joined.collect::<Vec<_>>());
        }
        changed
    }

    /// Join the activation's reachable values and construction prerequisites.
    fn join_activation_values(&mut self, other: &Self) -> bool {
        let pool = self
            .ordinary_literal_pool
            .as_ref()
            .zip(other.ordinary_literal_pool.as_ref())
            .and_then(|(left, right)| left.joined(right));
        let pool_changed = self.ordinary_literal_pool != pool;
        self.ordinary_literal_pool = pool;
        // naming.command.shared-noop-source-joins
        // docs/design/analysis/name-resolution-proofs/shared-noop-source-joins.md
        // The enclosing join requires one original baseline. Skip only a pure
        // collection self-join authenticated by its actual immutable Arc;
        // the literal-pool currency query above still runs independently.
        let mut changed = pool_changed;
        if !Arc::ptr_eq(&self.object_instances, &other.object_instances) {
            let before_objects = Arc::clone(&self.object_instances);
            Arc::make_mut(&mut self.object_instances).join(&other.object_instances);
            changed |= self.object_instances != before_objects;
        }
        // One shared map holds no count above its own.
        let counts = (!Arc::ptr_eq(&self.allocation_counts, &other.allocation_counts))
            .then_some(&other.allocation_counts);
        for (site, incoming) in counts.into_iter().flat_map(|counts| counts.iter()) {
            let current = self.allocation_counts.get(site).copied().unwrap_or(0);
            if *incoming > current {
                Arc::make_mut(&mut self.allocation_counts).insert(site.clone(), *incoming);
                changed = true;
            }
        }
        let counts = (!Arc::ptr_eq(
            &self.namespace_allocation_counts,
            &other.namespace_allocation_counts,
        ))
        .then_some(&other.namespace_allocation_counts);
        for (site, incoming) in counts.into_iter().flat_map(|counts| counts.iter()) {
            let current = self
                .namespace_allocation_counts
                .get(site)
                .copied()
                .unwrap_or(0);
            if *incoming > current {
                Arc::make_mut(&mut self.namespace_allocation_counts)
                    .insert(site.clone(), *incoming);
                changed = true;
            }
        }
        if !Arc::ptr_eq(&self.class_definitions, &other.class_definitions) {
            let before = self.class_definitions.len();
            Arc::make_mut(&mut self.class_definitions).retain(|identity, definition| {
                other.class_definitions.get(identity) == Some(definition)
            });
            changed |= before != self.class_definitions.len();
        }
        if !Arc::ptr_eq(&self.bounded_constructions, &other.bounded_constructions) {
            let before = self.bounded_constructions.len();
            Arc::make_mut(&mut self.bounded_constructions).retain(|identity, construction| {
                other.bounded_constructions.get(identity) == Some(construction)
            });
            changed |= before != self.bounded_constructions.len();
        }
        if !Arc::ptr_eq(&self.command_observers, &other.command_observers) {
            changed |= Arc::make_mut(&mut self.command_observers).join(&other.command_observers);
        }
        if !Arc::ptr_eq(
            &self.tainted_object_dispatch,
            &other.tainted_object_dispatch,
        ) {
            let before = self.tainted_object_dispatch.len();
            Arc::make_mut(&mut self.tainted_object_dispatch)
                .extend(other.tainted_object_dispatch.iter().cloned());
            changed |= before != self.tainted_object_dispatch.len();
        }
        if self.source_variables != other.source_variables {
            let mut joined = self.source_variables.as_ref().clone();
            joined.join(&other.source_variables);
            if *self.source_variables != joined {
                self.source_variables = Arc::new(joined);
                changed = true;
            }
        }
        if self.variable_frame != other.variable_frame
            && self.variable_frame != tcl_names::variable::VariableExecutionFrame::Unknown
        {
            self.variable_frame = tcl_names::variable::VariableExecutionFrame::Unknown;
            changed = true;
        }
        changed
    }

    /// Lookup depends on command objects, namespace alternatives and provider state.
    fn join_lookup_support(&mut self, other: &Self) -> bool {
        let mut changed = false;
        changed |= join_alternatives(&mut self.objects, &other.objects);
        changed |= self.join_ensemble_dispatches(other);
        if !Arc::ptr_eq(&self.namespace_exports, &other.namespace_exports) {
            changed |= Arc::make_mut(&mut self.namespace_exports)
                .join_with_default(&other.namespace_exports, &Vec::new());
        }
        if !Arc::ptr_eq(&self.namespace_paths, &other.namespace_paths) {
            changed |= Arc::make_mut(&mut self.namespace_paths)
                .join_with_default(&other.namespace_paths, &Vec::new());
        }
        changed |= join_alternatives_with_default(
            &mut self.packages,
            &other.packages,
            PackageProvision::absent(),
        );
        if other.loader_handler_unknown && !self.loader_handler_unknown {
            self.loader_handler_unknown = true;
            changed = true;
        }
        if !Arc::ptr_eq(&self.namespaces, &other.namespaces) {
            let before = self.namespaces.len();
            Arc::make_mut(&mut self.namespaces)
                .retain(|namespace| other.namespaces.contains(namespace));
            changed |= self.namespaces.len() != before;
        }
        for (destination, incoming) in [
            (
                &mut self.unknown_lookup_namespaces,
                &other.unknown_lookup_namespaces,
            ),
            (
                &mut self.unknown_namespace_paths,
                &other.unknown_namespace_paths,
            ),
            (
                &mut self.namespace_unknown_handlers,
                &other.namespace_unknown_handlers,
            ),
            (
                &mut self.ensemble_owner_namespaces,
                &other.ensemble_owner_namespaces,
            ),
            (
                &mut self.unknown_export_namespaces,
                &other.unknown_export_namespaces,
            ),
        ] {
            if !Arc::ptr_eq(destination, incoming) {
                let before = destination.len();
                Arc::make_mut(destination).extend(incoming.iter().cloned());
                changed |= destination.len() != before;
            }
        }
        for (destination, incoming) in [
            (&mut self.revoked_loaders, &other.revoked_loaders),
            (
                &mut self.tainted_provider_state,
                &other.tainted_provider_state,
            ),
        ] {
            if !Arc::ptr_eq(destination, incoming) {
                let before = destination.len();
                Arc::make_mut(destination).extend(incoming.iter().cloned());
                changed |= destination.len() != before;
            }
        }
        changed
    }

    /// Withhold the completed command world after a step with no closed
    /// normal-return receipt. `modelled_for_diagnostics` says the diagnostic
    /// view still knows the step's exact effect; one step it does not know
    /// withholds that view for good.
    pub(in super::super) fn withhold_command_world(&mut self, modelled_for_diagnostics: bool) {
        if !modelled_for_diagnostics {
            self.stock_tclsh_stdout = false;
        }
        self.diagnostic_command_world_closed = modelled_for_diagnostics
            && (!self.unclosed_command_world || self.diagnostic_command_world_closed);
        self.unclosed_command_world = true;
    }

    /// Historical opacity survives restoration of the current lookup state.
    fn join_binding_history(&mut self, other: &Self) -> bool {
        let mut changed = false;
        if self.stock_tclsh_stdout && !other.stock_tclsh_stdout {
            self.stock_tclsh_stdout = false;
            changed = true;
        }
        if other.unmodelled_reentry && !self.unmodelled_reentry {
            self.unmodelled_reentry = true;
            changed = true;
        }
        if other.opaque_domain && !self.opaque_domain {
            self.opaque_domain = true;
            changed = true;
        }
        if other.opaque_binding_mutation && !self.opaque_binding_mutation {
            self.opaque_binding_mutation = true;
            changed = true;
        }
        if other.receiver_frame_opacity && !self.receiver_frame_opacity {
            self.receiver_frame_opacity = true;
            changed = true;
        }
        if other.dynamic_proc_binding && !self.dynamic_proc_binding {
            self.dynamic_proc_binding = true;
            changed = true;
        }
        if other.unnameable_rebinding_subject && !self.unnameable_rebinding_subject {
            self.unnameable_rebinding_subject = true;
            changed = true;
        }
        if other.unclosed_command_world {
            let closed_for_diagnostics = other.diagnostic_command_world_closed
                && (!self.unclosed_command_world || self.diagnostic_command_world_closed);
            if !self.unclosed_command_world
                || self.diagnostic_command_world_closed != closed_for_diagnostics
            {
                self.unclosed_command_world = true;
                self.diagnostic_command_world_closed = closed_for_diagnostics;
                changed = true;
            }
        }
        changed |= self.namespace_resolution.join(&other.namespace_resolution);
        if !Arc::ptr_eq(&self.procedure_bodies, &other.procedure_bodies)
            && self.procedure_bodies != other.procedure_bodies
        {
            changed |= self.extend_procedure_bodies(other.procedure_bodies.iter().cloned());
        }
        // A shared or contained set adds nothing and stays shared.
        if !self.rebound_names.shares(&other.rebound_names)
            && !other.rebound_names.is_subset(&self.rebound_names)
        {
            self.rebound_names
                .extend(other.rebound_names.iter().cloned());
            changed = true;
        }
        let proc_rebound_count = self.proc_rebound_names.len();
        self.proc_rebound_names
            .extend(other.proc_rebound_names.iter().cloned());
        changed |= self.proc_rebound_names.len() != proc_rebound_count;
        if other.has_redefined_procedures && !self.has_redefined_procedures {
            self.has_redefined_procedures = true;
            changed = true;
        }
        changed
    }
}

fn join_alternatives<K: Clone + Ord, V: Clone + Ord>(
    destination: &mut Arc<BTreeMap<K, BTreeSet<V>>>,
    incoming: &Arc<BTreeMap<K, BTreeSet<V>>>,
) -> bool {
    if destination == incoming {
        return false;
    }
    let mut changed = false;
    for (key, values) in incoming.iter() {
        let entry = Arc::make_mut(destination).entry(key.clone()).or_default();
        let previous = entry.len();
        entry.extend(values.iter().cloned());
        changed |= entry.len() != previous;
    }
    changed
}

fn join_alternatives_with_default<K: Clone + Ord, V: Clone + Ord>(
    destination: &mut Arc<BTreeMap<K, BTreeSet<V>>>,
    incoming: &Arc<BTreeMap<K, BTreeSet<V>>>,
    default: V,
) -> bool {
    if destination == incoming {
        return false;
    }
    let keys = destination
        .keys()
        .chain(incoming.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    let fallback = BTreeSet::from([default]);
    let joined = keys
        .into_iter()
        .map(|key| {
            let mut values = destination
                .get(&key)
                .cloned()
                .unwrap_or_else(|| fallback.clone());
            values.extend(
                incoming
                    .get(&key)
                    .cloned()
                    .unwrap_or_else(|| fallback.clone()),
            );
            (key, values)
        })
        .collect::<BTreeMap<_, _>>();
    if **destination == joined {
        return false;
    }
    *destination = Arc::new(joined);
    true
}
