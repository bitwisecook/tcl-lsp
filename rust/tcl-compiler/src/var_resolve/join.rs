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

//! Joining incoming binding states at a control-flow merge.

use super::canonical_keys::binding_key_in_namespace;
use super::cell_key::{VariableCellKey, VariableCellTable};
use super::{
    ActivationObserverInventory, ContentsOrigin, ContentsPresence, ContentsWorld, ResolveContext,
    SourceReadConfiguration,
};
use crate::place;
use crate::place::{CellGeneration, Place, PlaceKind};
use std::collections::HashSet;

impl ResolveContext {
    /// Join incoming binding states. A binding is precise only when all
    /// executable predecessors agree on the same immutable cell address.
    pub fn join(&mut self, other: &Self) {
        if self.source_read_configuration != other.source_read_configuration {
            self.source_read_configuration = SourceReadConfiguration::Unavailable;
        }
        let execution_disagrees = self.execution != other.execution
            || self.interpreter != other.interpreter
            || self.hosted_execution_context != other.hosted_execution_context;
        let integer_contents = self.joined_integer_contents(other);
        // Equal current stamps with differing issuance histories cannot issue
        // a future stamp safely, including repeated abstract loop allocations.
        if self.original_contents_epoch_high_water != other.original_contents_epoch_high_water {
            self.original_contents_epoch_high_water = None;
            self.original_contents_epoch = None;
        }
        if self.original_contents_epoch != other.original_contents_epoch {
            self.original_contents_epoch = None;
        }
        self.join_literal_contents(other);
        self.join_original_name_values(other);
        if self.original_formal_topology != other.original_formal_topology {
            self.original_formal_topology = None;
        }
        if self.original_native_compiler_formals != other.original_native_compiler_formals {
            self.original_native_compiler_formals = None;
        }
        if self.original_receiver_variable_candidates != other.original_receiver_variable_candidates
        {
            self.original_receiver_variable_candidates = None;
            self.dynamic_bindings = true;
        }
        // Differing issuance histories at a join have no bounded next stamp.
        // In particular, abstract loop iterations must not mint an unbounded
        // succession of receipts for one repeated allocation family.
        if self.representation_epoch_high_water != other.representation_epoch_high_water {
            self.representation_epoch_high_water = None;
            self.representation_epoch = None;
        }
        if self.representation_epoch != other.representation_epoch {
            self.representation_epoch = None;
        }
        self.join_alias_bindings(other);
        self.outward_namespace_destructions
            .extend(other.outward_namespace_destructions.iter().cloned());
        self.raw_bindings.join(&other.raw_bindings);
        self.captured_cells.join(&other.captured_cells);
        self.dictionary_scopes
            .retain(|source, activation| other.dictionary_scopes.get(source) == Some(activation));
        if !self.contents_kinds.shares(&other.contents_kinds) {
            self.contents_kinds
                .retain_where(|key, kind| other.contents_kinds.get(key) == Some(kind));
        }
        self.join_contents_presence(other);
        for (key, slot) in &other.contents_presence_slots {
            self.contents_presence_slots
                .entry(key.clone())
                .or_insert_with(|| std::sync::Arc::clone(slot));
        }
        self.join_contents_origins(other);
        if !self.constant_values.shares(&other.constant_values) {
            self.constant_values
                .retain_where(|name, value| other.constant_values.get(name) == Some(value));
        }
        self.join_value_representations(other, &integer_contents);
        match (&self.caller, &other.caller) {
            (Some(left), Some(right)) if left != right => {
                let mut joined = (**left).clone();
                joined.join(right);
                self.caller = Some(std::sync::Arc::new(joined));
            }
            (Some(_), None) => self.caller = None,
            _ => {}
        }
        self.unknown_bindings
            .extend(other.unknown_bindings.iter().cloned());
        if self.interpreter != other.interpreter {
            self.interpreter = None;
        }
        if self.execution != other.execution {
            self.execution = None;
        }
        if self.hosted_execution_context != other.hosted_execution_context {
            self.hosted_execution_context = None;
        }
        if self.activation != other.activation || self.selected_frame != other.selected_frame {
            self.activation = None;
            self.selected_frame = None;
            self.dynamic_bindings = true;
        }
        self.join_namespace_and_observer_world(other);
        if execution_disagrees {
            // Equal byte names and values in different interpreters, workers,
            // connections or reload epochs do not select one reaching cell.
            self.widen();
        }
    }

    fn join_namespace_and_observer_world(&mut self, other: &Self) {
        self.namespace_known &= other.namespace_known
            && self.namespace == other.namespace
            && self.namespace_identity == other.namespace_identity;
        if self.namespace_identity != other.namespace_identity {
            self.namespace_identity = None;
        }
        self.namespace_identities
            .retain(|identity| other.namespace_identities.contains(identity));
        self.namespace_inventory = self.namespace_inventory.joined(
            other.namespace_inventory,
            self.namespace_addressable_identities == other.namespace_addressable_identities,
        );
        self.namespace_addressable_identities
            .retain(|identity| other.namespace_addressable_identities.contains(identity));
        if self.namespace_name_protocol != other.namespace_name_protocol {
            self.namespace_name_protocol = None;
        }
        if self.execution_name_policy != other.execution_name_policy {
            self.execution_name_policy = None;
        }
        if self.expression_parser != other.expression_parser {
            self.expression_parser = None;
        }
        self.namespace_objects
            .retain(|key, object| other.namespace_objects.get(key) == Some(object));
        if self.authored_tmm_static != other.authored_tmm_static {
            self.authored_tmm_static = None;
            self.mark_unenumerated_variable_observers();
        }
        for (key, pending) in &mut self.pending_namespace_retirements {
            if other.pending_namespace_retirements.get(key) != Some(pending) {
                pending.conditional = true;
            }
        }
        for (key, pending) in &other.pending_namespace_retirements {
            self.pending_namespace_retirements
                .entry(key.clone())
                .or_insert_with(|| {
                    let mut pending = pending.clone();
                    pending.conditional = true;
                    pending
                });
        }
        self.namespace_cells.join(&other.namespace_cells);
        self.dynamic_bindings |= other.dynamic_bindings;
        self.dynamic_traces |= other.dynamic_traces;
        self.join_trace_registrations(other);
        for registration in &other.possible_trace_registrations {
            let required = other
                .possible_trace_registrations
                .iter()
                .filter(|item| *item == registration)
                .count();
            let present = self
                .possible_trace_registrations
                .iter()
                .filter(|item| *item == registration)
                .count();
            self.possible_trace_registrations
                .extend(std::iter::repeat_n(
                    registration.clone(),
                    required.saturating_sub(present),
                ));
        }
        self.known_namespaces
            .retain(|name| other.known_namespaces.contains(name));
        let keys: HashSet<_> = self
            .generations
            .keys()
            .chain(other.generations.keys())
            .cloned()
            .collect();
        for key in keys {
            let mine = self.generations.get(&key).copied().unwrap_or_default();
            let theirs = other.generations.get(&key).copied().unwrap_or_default();
            if mine != theirs {
                self.generations.insert(key, CellGeneration::Unknown);
            }
        }
    }

    fn joined_integer_contents(
        &self,
        other: &Self,
    ) -> VariableCellTable<crate::native_numeric::SourceIntegerConvertibleContents> {
        // Contents/conversion alternatives use each predecessor's still-live
        // bytes and sealed stock/numeric provenance, before value intersection.
        self.value_representations
            .iter()
            .filter_map(|(key, left)| {
                let right = other.value_representations.get(key)?;
                let left_proof = left.integer_contents_in(
                    self.constant_values.get(key).map(String::as_str),
                    self.invocation_dialect,
                )?;
                let right_proof = right.integer_contents_in(
                    other.constant_values.get(key).map(String::as_str),
                    other.invocation_dialect,
                )?;
                (left_proof == right_proof).then(|| (key.clone(), left_proof))
            })
            .collect()
    }

    fn join_value_representations(
        &mut self,
        other: &Self,
        integer_contents: &VariableCellTable<
            crate::native_numeric::SourceIntegerConvertibleContents,
        >,
    ) {
        self.value_representations.retain(|name, value| {
            let Some(joined) = other
                .value_representations
                .get(name)
                .and_then(|right| {
                    if value == right
                        && (!matches!(value, crate::native_numeric::StoredNativeRepresentation::StockLiteralObject(_))
                            || self.constant_values.contains_key(name)) {
                        value.joined(right)
                    } else if let Some(joined) = value.joined_current_numeric(right) {
                        Some(joined)
                    } else if let Some(proof) = integer_contents.get(name) {
                        Some(crate::native_numeric::StoredNativeRepresentation::IntegerConvertibleContents(*proof))
                    } else {
                        value.joined(right)
                    }
                })
            else {
                return false;
            };
            *value = joined;
            true
        });
    }

    fn join_contents_presence(&mut self, other: &Self) {
        // One shared table holds the same presence for every cell on both
        // sides; walking it would copy it for no change.
        if self.contents_presence.shares(&other.contents_presence) {
            return;
        }
        let keys: HashSet<_> = self
            .contents_presence
            .keys()
            .chain(other.contents_presence.keys())
            .cloned()
            .collect();
        for key in keys {
            let left = self
                .contents_presence
                .get(&key)
                .copied()
                .unwrap_or_else(|| {
                    other
                        .contents_presence_slots
                        .get(&key)
                        .map_or(ContentsPresence::Unknown, |slot| {
                            self.contents_presence(slot)
                        })
                });
            let right = other
                .contents_presence
                .get(&key)
                .copied()
                .unwrap_or_else(|| {
                    self.contents_presence_slots
                        .get(&key)
                        .map_or(ContentsPresence::Unknown, |slot| {
                            other.contents_presence(slot)
                        })
                });
            let joined = if left == right {
                left
            } else if left != ContentsPresence::Unknown && right != ContentsPresence::Unknown {
                ContentsPresence::DefinedOrUndefined
            } else {
                ContentsPresence::Unknown
            };
            if self.contents_presence.get(&key) != Some(&joined) {
                self.contents_presence.insert(key, joined);
            }
        }
    }

    pub(super) fn array_contents_unknown(&self, key: &VariableCellKey) -> bool {
        self.contents_unknown_arrays
            .iter()
            .any(|root| key.is_member_of(root))
    }

    pub(super) fn contents_world_unknown_for_key(&self, key: &VariableCellKey) -> bool {
        if let VariableCellKey::Activation { identity, .. } = key.root()
            && self.activation.as_ref() == Some(identity)
            && let Some(world) = self.activation_contents_world
        {
            return world.is_unknown();
        }
        self.contents_world.is_unknown()
    }

    pub(super) fn withdraw_activation_contents_closure(&mut self) {
        self.activation_observer_inventory = ActivationObserverInventory::Open;
        self.closed_observer_allocations.clear();
        if self.activation_contents_world.is_some() {
            self.activation_contents_world = Some(ContentsWorld::Unknown);
        }
    }

    fn incoming_contents_origin(&self, key: &VariableCellKey) -> ContentsOrigin {
        if self.contents_world_unknown_for_key(key)
            || self.array_contents_unknown(key)
            || self
                .contents_unknown_namespaces
                .iter()
                .any(|namespace| binding_key_in_namespace(key, namespace))
        {
            ContentsOrigin::Unknown
        } else {
            ContentsOrigin::Incoming
        }
    }

    fn join_contents_origins(&mut self, other: &Self) {
        self.contents_source_proofs
            .joined(&other.contents_source_proofs);
        let names: HashSet<_> = if self.contents_origins.shares(&other.contents_origins) {
            HashSet::new()
        } else {
            self.contents_origins
                .keys()
                .chain(other.contents_origins.keys())
                .cloned()
                .collect()
        };
        for name in names {
            let left = self
                .contents_origins
                .get(&name)
                .cloned()
                .unwrap_or_else(|| self.incoming_contents_origin(&name));
            let right = other
                .contents_origins
                .get(&name)
                .cloned()
                .unwrap_or_else(|| other.incoming_contents_origin(&name));
            let joined = left.joined(&right);
            if self.contents_origins.get(&name) != Some(&joined) {
                self.contents_origins.insert(name, joined);
            }
        }
        self.closed_array_roots
            .retain(|key| other.closed_array_roots.contains(key));
        self.contents_world = self.contents_world.joined(other.contents_world);
        self.activation_contents_world = match (
            self.activation_contents_world,
            other.activation_contents_world,
        ) {
            (Some(left), Some(right)) => Some(left.joined(right)),
            _ => None,
        };
        self.activation_observer_inventory = if self.activation_observers_closed()
            && other.activation_observers_closed()
            && self.activation == other.activation
        {
            ActivationObserverInventory::Closed
        } else {
            ActivationObserverInventory::Open
        };
        self.closed_observer_allocations
            .retain(|allocation| other.closed_observer_allocations.contains(allocation));
        self.contents_unknown_namespaces
            .extend(other.contents_unknown_namespaces.iter().cloned());
        self.contents_unknown_arrays
            .extend(other.contents_unknown_arrays.iter().cloned());
    }

    fn join_alias_bindings(&mut self, other: &Self) {
        let keys: HashSet<_> = if self.alias_bindings.shares(&other.alias_bindings) {
            HashSet::new()
        } else {
            self.alias_bindings
                .keys()
                .chain(other.alias_bindings.keys())
                .cloned()
                .collect()
        };
        for key in keys {
            if self.name_alias_bindings.contains_key(&key)
                && self.name_alias_bindings.get(&key) == other.name_alias_bindings.get(&key)
            {
                continue;
            }
            if self.alias_bindings.get(&key) != other.alias_bindings.get(&key) {
                if let Some(joined) = self
                    .alias_bindings
                    .get(&key)
                    .zip(other.alias_bindings.get(&key))
                    .and_then(|(left, right)| join_alias_target(left, right))
                {
                    self.alias_bindings.insert(key, joined);
                } else {
                    self.alias_bindings.remove(&key);
                    self.unknown_bindings.insert(key);
                }
            }
        }
        self.join_name_aliases(other);
        let namespaces: HashSet<_> = self
            .namespace_alias_bindings
            .keys()
            .chain(other.namespace_alias_bindings.keys())
            .cloned()
            .collect();
        for name in namespaces {
            if self.namespace_name_alias_bindings.contains_key(&name)
                && self.namespace_name_alias_bindings.get(&name)
                    == other.namespace_name_alias_bindings.get(&name)
            {
                continue;
            }
            if self.namespace_alias_bindings.get(&name) != other.namespace_alias_bindings.get(&name)
            {
                let joined = self
                    .namespace_alias_bindings
                    .get(&name)
                    .zip(other.namespace_alias_bindings.get(&name))
                    .and_then(|(left, right)| join_alias_target(left, right))
                    .unwrap_or_else(place::unknown_top);
                self.namespace_alias_bindings.insert(name, joined);
            }
        }
    }

    fn join_name_aliases(&mut self, other: &Self) {
        let named: HashSet<_> = self
            .name_alias_bindings
            .keys()
            .chain(other.name_alias_bindings.keys())
            .cloned()
            .collect();
        for name in named {
            if self.name_alias_bindings.get(&name) != other.name_alias_bindings.get(&name) {
                self.name_alias_bindings.remove(&name);
                self.unknown_bindings.insert(name);
            }
        }
        let namespace_names: HashSet<_> = self
            .namespace_name_alias_bindings
            .keys()
            .chain(other.namespace_name_alias_bindings.keys())
            .cloned()
            .collect();
        for name in namespace_names {
            if self.namespace_name_alias_bindings.get(&name)
                != other.namespace_name_alias_bindings.get(&name)
            {
                self.namespace_name_alias_bindings
                    .insert(name, place::unknown_top());
            }
        }
    }

    fn join_trace_registrations(&mut self, other: &Self) {
        self.trace_registration_receivers
            .retain(|key, receiver| other.trace_registration_receivers.get(key) == Some(receiver));
        for target in &self.traced {
            if !self.trace_registrations.contains_key(target) {
                self.untracked_traces.insert(target.clone());
            }
        }
        for target in &other.traced {
            if !other.trace_registrations.contains_key(target) {
                self.untracked_traces.insert(target.clone());
            }
        }
        self.traced.extend(other.traced.iter().cloned());
        self.untracked_traces
            .extend(other.untracked_traces.iter().cloned());
        for (target, registrations) in &other.trace_registrations {
            let reaching = self.trace_registrations.entry(target.clone()).or_default();
            let mut matched = Vec::new();
            for registration in registrations {
                let existing = reaching.iter().enumerate().position(|(index, item)| {
                    !matched.contains(&index)
                        && item.0 == registration.0
                        && item.1.same_data(&registration.1)
                });
                if let Some(index) = existing {
                    matched.push(index);
                    if let Some(joined) = reaching[index].1.joined(&registration.1) {
                        reaching[index].1 = joined;
                    } else {
                        self.untracked_traces.insert(target.clone());
                    }
                } else {
                    matched.push(reaching.len());
                    reaching.push(registration.clone());
                }
            }
        }
    }
}

// A scalar name slot survives Tcl unset. Its contents may be fresh while
// aliases still select the same slot. Element links require a proved root
// lifetime: joining different generations must never revive a dangling link.
fn join_alias_target(left: &Place, right: &Place) -> Option<Place> {
    let mut joined = left.clone();
    let mut candidate = right.clone();
    let observed = joined.observed || candidate.observed;
    joined.observed = observed;
    candidate.observed = observed;
    if matches!(left.kind, PlaceKind::Scalar | PlaceKind::ArrayWhole)
        && let (Some(left_cell), Some(right_cell)) = (&mut joined.cell, &mut candidate.cell)
        && left_cell.generation != right_cell.generation
    {
        left_cell.generation = CellGeneration::Unknown;
        right_cell.generation = CellGeneration::Unknown;
    }
    (joined == candidate).then_some(joined)
}
