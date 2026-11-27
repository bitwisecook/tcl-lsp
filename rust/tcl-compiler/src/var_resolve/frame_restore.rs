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

//! Returning to the caller activation while keeping a callee's effects on outward namespace storage.

use super::ResolveContext;
use super::canonical_keys::cell_key;
use std::collections::HashSet;
use tcl_names::place::PlaceKind;
use tcl_names::variable::VariableCellKey;

/// Restore the caller activation while retaining effects on outward namespace storage.
#[must_use]
pub fn restore_execution_frame(parent: &ResolveContext, child: &ResolveContext) -> ResolveContext {
    let mut restored = restore_selected_bindings(parent, child);
    restore_namespace_bindings(&mut restored, child);
    let mut parent_activations = HashSet::new();
    let mut frame = Some(parent);
    while let Some(context) = frame {
        if let Some(identity) = &context.activation {
            parent_activations.insert(identity.clone());
        }
        frame = context.caller.as_deref();
    }
    let instance_keys = crate::allocated_instance::outward_cell_keys(parent)
        .union(&crate::allocated_instance::outward_cell_keys(child))
        .cloned()
        .collect::<HashSet<_>>();
    let parent_key = |key: &VariableCellKey| {
        instance_keys.contains(key)
            || match key.root() {
                VariableCellKey::Namespace { .. } | VariableCellKey::RetainedSlot(_) => true,
                VariableCellKey::Activation { identity, .. } => {
                    parent_activations.contains(identity)
                }
                VariableCellKey::Authored(name) => tcl_syntax::naming::is_absolute(name),
                _ => false,
            }
    };
    restore_outward_observers_and_generations(&mut restored, child, &parent_key);
    restore_outward_values(&mut restored, child, &parent_key);
    restored
        .contents_unknown_arrays
        .retain(|key| !parent_key(key));
    restored.contents_unknown_arrays.extend(
        child
            .contents_unknown_arrays
            .iter()
            .filter(|key| parent_key(key))
            .cloned(),
    );
    restored.closed_array_roots.retain(|key| !parent_key(key));
    restored.closed_array_roots.extend(
        child
            .closed_array_roots
            .iter()
            .filter(|key| parent_key(key))
            .cloned(),
    );
    restored.contents_world = restored.contents_world.joined(child.contents_world);
    restored
        .contents_unknown_namespaces
        .extend(child.contents_unknown_namespaces.iter().cloned());
    restored
        .outward_namespace_destructions
        .extend(child.outward_namespace_destructions.iter().cloned());
    restored.dynamic_traces |= child.dynamic_traces;
    restored
        .closed_observer_allocations
        .clone_from(&child.closed_observer_allocations);
    restore_outward_aliases(parent, child, &mut restored);
    if child.dynamic_bindings {
        restored.widen();
    }
    crate::variable_bindings::complete_pending_namespace_retirements(&mut restored);
    restored
}

fn restore_outward_observers_and_generations(
    restored: &mut ResolveContext,
    child: &ResolveContext,
    parent_key: &impl Fn(&VariableCellKey) -> bool,
) {
    restored.generations.retain(|key, _| !parent_key(key));
    restored.generations.extend(
        child
            .generations
            .iter()
            .filter(|(key, _)| parent_key(key))
            .map(|(key, generation)| (key.clone(), *generation)),
    );
    restored.traced.retain(|key| !parent_key(key));
    restored
        .traced
        .extend(child.traced.iter().filter(|key| parent_key(key)).cloned());
    restored
        .trace_registrations
        .retain(|key, _| !parent_key(key));
    restored.trace_registrations.extend(
        child
            .trace_registrations
            .iter()
            .filter(|(key, _)| parent_key(key))
            .map(|(key, registrations)| (key.clone(), registrations.clone())),
    );
    restored
        .trace_registration_receivers
        .retain(|key, _| !parent_key(key));
    restored.trace_registration_receivers.extend(
        child
            .trace_registration_receivers
            .iter()
            .filter(|(key, _)| parent_key(key))
            .map(|(key, value)| (key.clone(), std::sync::Arc::clone(value))),
    );
    restored.untracked_traces.retain(|key| !parent_key(key));
    restored.untracked_traces.extend(
        child
            .untracked_traces
            .iter()
            .filter(|key| parent_key(key))
            .cloned(),
    );
}

fn restore_namespace_bindings(restored: &mut ResolveContext, child: &ResolveContext) {
    restored.raw_bindings.clone_from(&child.raw_bindings);
    restored.captured_cells.clone_from(&child.captured_cells);
    restored
        .possible_trace_registrations
        .clone_from(&child.possible_trace_registrations);
    restored.namespace_cells.clone_from(&child.namespace_cells);
    restored
        .pending_namespace_retirements
        .clone_from(&child.pending_namespace_retirements);
    restored
        .namespace_identities
        .clone_from(&child.namespace_identities);
    restored
        .namespace_addressable_identities
        .clone_from(&child.namespace_addressable_identities);
    restored.namespace_inventory = child.namespace_inventory;
    restored.namespace_name_protocol = child.namespace_name_protocol;
    restored.execution_name_policy = child.execution_name_policy;
    restored.expression_parser = child.expression_parser;
    restored.hosted_execution_context = child.hosted_execution_context;
    let retained_current = restored.namespace_identity.as_ref().and_then(|key| {
        restored
            .namespace_objects
            .get(key)
            .cloned()
            .map(|object| (key.clone(), object))
    });
    restored
        .namespace_objects
        .clone_from(&child.namespace_objects);
    if let Some((key, object)) = retained_current {
        restored.namespace_objects.entry(key).or_insert(object);
    }
    restored
        .authored_tmm_static
        .clone_from(&child.authored_tmm_static);
    restored
        .namespace_alias_bindings
        .clone_from(&child.namespace_alias_bindings);
    restored
        .namespace_name_alias_bindings
        .clone_from(&child.namespace_name_alias_bindings);
    restored
        .known_namespaces
        .clone_from(&child.known_namespaces);
}

fn restore_outward_aliases(
    parent: &ResolveContext,
    child: &ResolveContext,
    restored: &mut ResolveContext,
) {
    for (name, alias) in &mut restored.alias_bindings {
        if let Some(retained) = child.raw_bindings.retained_array_member(alias) {
            *alias = retained;
            continue;
        }
        let deleted = alias.is_global()
            && (child
                .outward_namespace_destructions
                .iter()
                .any(|namespace| cell_key(alias).is_owned_by_namespace(namespace))
                || (cell_key(alias).authored_spelling().is_some()
                    && parent.known_namespaces.contains(&alias.ns)
                    && !child.known_namespaces.contains(&alias.ns)));
        let generation = child
            .generations
            .get(&cell_key(alias))
            .copied()
            .unwrap_or_default();
        let changed = alias
            .cell
            .as_ref()
            .is_some_and(|cell| cell.generation != generation);
        let jim = child
            .invocation_dialect
            .and_then(|dialect| dialect.variable_link_binding)
            == Some(tcl_dialect::VariableLinkBinding::SelectedFrameName);
        if !jim && (deleted || (changed && alias.kind == PlaceKind::ArrayElem)) {
            restored.unknown_bindings.insert(name.clone());
        } else if changed && let Some(cell) = &mut alias.cell {
            cell.generation = generation;
        }
    }
}

fn restore_outward_values(
    restored: &mut ResolveContext,
    child: &ResolveContext,
    parent_key: &impl Fn(&VariableCellKey) -> bool,
) {
    restored
        .constant_values
        .retain(|key, _| !parent_key(key) || child.constant_values.contains_key(key));
    restored.constant_values.extend(
        child
            .constant_values
            .iter()
            .filter(|(key, _)| parent_key(key))
            .map(|(key, value)| (key.clone(), value.clone())),
    );
    restored
        .closed_literal_contents
        .retain(|key, _| !parent_key(key));
    restored.closed_literal_contents.extend(
        child
            .closed_literal_contents
            .iter()
            .filter(|(key, _)| parent_key(key))
            .map(|(key, value)| (key.clone(), value.clone())),
    );
    restored
        .original_name_values
        .retain(|key, _| !parent_key(key));
    restored.original_name_values.extend(
        child
            .original_name_values
            .iter()
            .filter(|(key, _)| parent_key(key))
            .map(|(key, value)| (key.clone(), value.clone())),
    );
    // Representation changes affect shared Tcl objects, including objects whose
    // variable bindings belong to an unselected ancestor frame.
    restored
        .value_representations
        .clone_from(&child.value_representations);
    restored.original_contents_epoch = child.original_contents_epoch;
    restored.original_contents_epoch_high_water = restored
        .original_contents_epoch_high_water
        .zip(child.original_contents_epoch_high_water)
        .map(|(left, right)| left.max(right));
    restored.representation_epoch = child.representation_epoch;
    restored.representation_epoch_high_water = restored
        .representation_epoch_high_water
        .zip(child.representation_epoch_high_water)
        .map(|(left, right)| left.max(right));
    restored.contents_kinds.retain(|key, _| !parent_key(key));
    restored.contents_kinds.extend(
        child
            .contents_kinds
            .iter()
            .filter(|(key, _)| parent_key(key))
            .map(|(key, kind)| (key.clone(), *kind)),
    );
    restored.contents_presence.retain(|key, _| !parent_key(key));
    restored.contents_presence.extend(
        child
            .contents_presence
            .iter()
            .filter(|(key, _)| parent_key(key))
            .map(|(key, &presence)| (key.clone(), presence)),
    );
    restored
        .contents_presence_slots
        .retain(|key, _| !parent_key(key));
    restored.contents_presence_slots.extend(
        child
            .contents_presence_slots
            .iter()
            .filter(|(key, _)| parent_key(key))
            .map(|(key, place)| (key.clone(), std::sync::Arc::clone(place))),
    );
    restored
        .contents_source_proofs
        .restore(&child.contents_source_proofs, parent_key);
    restored.contents_origins.retain(|key, _| !parent_key(key));
    restored.contents_origins.extend(
        child
            .contents_origins
            .iter()
            .filter(|(key, _)| parent_key(key))
            .map(|(key, origin)| (key.clone(), origin.clone())),
    );
}

fn same_execution_frame(left: &ResolveContext, right: &ResolveContext) -> bool {
    left.frame_kind == right.frame_kind
        && left.namespace == right.namespace
        && left.namespace_identity == right.namespace_identity
        && ((left.global_frame() && right.global_frame())
            || (left.activation.is_some() && left.activation == right.activation))
}

fn restore_selected_bindings(parent: &ResolveContext, child: &ResolveContext) -> ResolveContext {
    let mut parents = Vec::new();
    let mut frame = Some(parent);
    while let Some(current) = frame {
        parents.push(current);
        frame = current.caller.as_deref();
    }
    let mut restored = None;
    for parent in parents.into_iter().rev() {
        let mut current = parent.clone();
        let mut candidate = Some(child);
        while let Some(frame) = candidate {
            if same_execution_frame(parent, frame) {
                current.alias_bindings.clone_from(&frame.alias_bindings);
                current
                    .name_alias_bindings
                    .clone_from(&frame.name_alias_bindings);
                current.unknown_bindings.clone_from(&frame.unknown_bindings);
                current.globals.clone_from(&frame.globals);
                current.ns_vars.clone_from(&frame.ns_vars);
                current.upvar_aliases.clone_from(&frame.upvar_aliases);
                current
                    .original_receiver_variable_candidates
                    .clone_from(&frame.original_receiver_variable_candidates);
                current.dynamic_bindings = frame.dynamic_bindings;
                current.activation_contents_world = frame.activation_contents_world;
                current.activation_observer_inventory = frame.activation_observer_inventory;
                break;
            }
            candidate = frame.caller.as_deref();
        }
        current.caller = restored;
        restored = Some(std::sync::Arc::new(current));
    }
    std::sync::Arc::try_unwrap(restored.expect("parent frame was retained"))
        .expect("restored root has a single owner")
}
