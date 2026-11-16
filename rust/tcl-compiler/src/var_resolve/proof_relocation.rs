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

//! `VariableProofRelocation`: alpha-renaming proved allocations and source sites, and relocating a whole `ResolveContext` through it.

use super::cell_key::{VariableCellKey, VariableCellKeyQuery, VariableCellTable};
use super::{
    ContentsOrigin, PendingNamespaceRetirement, PossibleVariableTraceRegistration, ResolveContext,
    VariableExecutionFrame,
};
use crate::place::{CellGeneration, CellOwner, Place};
use std::collections::HashMap;

/// Exact alpha-renaming of proved activation allocations and source write sites.
/// Unmapped external identities remain unchanged and therefore retain cache dependencies.
/// A reusable template must use an injective mapping and preserve its inverse.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VariableProofRelocation {
    /// Actual activation allocation identity to template or instantiated identity.
    pub activations: std::collections::BTreeMap<String, String>,
    /// Exact allocation/store source offsets to their relocated offsets.
    pub source_offsets: std::collections::BTreeMap<u32, u32>,
    /// Canonical key mappings derived from retained typed cell addresses.
    pub storage_keys: HashMap<VariableCellKey, VariableCellKey>,
    /// Exact source instances mapped by an authorised body-template owner.
    pub source_origins: std::collections::BTreeMap<
        std::sync::Arc<crate::command_binding::SourceOriginId>,
        std::sync::Arc<crate::command_binding::SourceOriginId>,
    >,
}

impl VariableProofRelocation {
    /// Invert an injective alpha-renaming; ambiguous mappings cannot instantiate a proof.
    #[must_use]
    pub fn inverse(&self) -> Option<Self> {
        let activations: std::collections::BTreeMap<_, _> = self
            .activations
            .iter()
            .map(|(from, to)| (to.clone(), from.clone()))
            .collect();
        let source_offsets: std::collections::BTreeMap<_, _> = self
            .source_offsets
            .iter()
            .map(|(&from, &to)| (to, from))
            .collect();
        let source_origins: std::collections::BTreeMap<_, _> = self
            .source_origins
            .iter()
            .map(|(from, to)| (std::sync::Arc::clone(to), std::sync::Arc::clone(from)))
            .collect();
        let storage_keys: HashMap<_, _> = self
            .storage_keys
            .iter()
            .map(|(from, to)| (to.clone(), from.clone()))
            .collect();
        (storage_keys.len() == self.storage_keys.len()
            && activations.len() == self.activations.len()
            && source_offsets.len() == self.source_offsets.len()
            && source_origins.len() == self.source_origins.len())
        .then_some(Self {
            activations,
            source_offsets,
            storage_keys,
            source_origins,
        })
    }

    /// Relocate an exact source instance, retaining unmapped external origins.
    #[must_use]
    pub fn source_origin(
        &self,
        source: &std::sync::Arc<crate::command_binding::SourceOriginId>,
    ) -> std::sync::Arc<crate::command_binding::SourceOriginId> {
        self.source_origins
            .get(source)
            .cloned()
            .unwrap_or_else(|| std::sync::Arc::clone(source))
    }

    /// Relocate one exact source site, retaining external sites as dependencies.
    #[must_use]
    pub fn source_offset(&self, source: u32) -> u32 {
        self.source_offsets.get(&source).copied().unwrap_or(source)
    }

    /// Relocate a source-authored lifetime while retaining incoming or unknown identity.
    #[must_use]
    pub fn cell_generation(&self, generation: CellGeneration) -> CellGeneration {
        match generation {
            CellGeneration::After(source) => CellGeneration::After(self.source_offset(source)),
            other => other,
        }
    }

    /// Relocate a typed physical cell address and its name/index dependencies.
    #[must_use]
    pub fn place(&self, place: &Place) -> Place {
        let mut relocated = place.clone();
        if let Some(cell) = &mut relocated.cell {
            if let CellOwner::NamespaceIdentity(identity) = &mut cell.owner {
                **identity = self.namespace_key(identity);
            }
            if let CellOwner::AllocatedInstance(allocation) = &mut cell.owner {
                **allocation = crate::allocated_instance::relocate_allocation(allocation, self);
            }
            if let CellOwner::RetainedSlot(identity) = &mut cell.owner {
                **identity = identity.relocated(self);
            }
            if let CellOwner::Activation(identity) = &mut cell.owner
                && let Some(replacement) = self.activations.get(identity)
            {
                identity.clone_from(replacement);
            }
            if let CellGeneration::After(source) = &mut cell.generation {
                *source = self.source_offset(*source);
            }
        }
        for dependency in &mut relocated.name_reads {
            *dependency = self.place(dependency);
        }
        for index in relocated.index.iter_mut().chain(relocated.keys.iter_mut()) {
            for dependency in &mut index.read_places {
                *dependency = self.place(dependency);
            }
        }
        relocated
    }

    /// Relocate an exact storage slot without parsing its presentation.
    #[must_use]
    pub fn cell_key(&self, key: &VariableCellKey) -> VariableCellKey {
        if let Some(replacement) = self.storage_keys.get(key) {
            return replacement.clone();
        }
        match key {
            VariableCellKey::Lifetime { source, cell } => self
                .cell_key(cell)
                .with_lifetime(self.source_offset(*source)),
            VariableCellKey::Element { cell, index } => {
                self.cell_key(cell).with_index(index.clone())
            }
            VariableCellKey::RetainedSlot(slot) => {
                VariableCellKey::RetainedSlot(Box::new(slot.relocated(self)))
            }
            VariableCellKey::Activation { identity, simple } => VariableCellKey::Activation {
                identity: self
                    .activations
                    .get(identity)
                    .cloned()
                    .unwrap_or_else(|| identity.clone()),
                simple: simple.clone(),
            },
            VariableCellKey::AllocatedInstance { allocation, simple } => {
                VariableCellKey::AllocatedInstance {
                    allocation: Box::new(crate::allocated_instance::relocate_allocation(
                        allocation, self,
                    )),
                    simple: simple.clone(),
                }
            }
            VariableCellKey::Namespace { identity, simple } => VariableCellKey::Namespace {
                identity: self.namespace_key(identity),
                simple: simple.clone(),
            },
            key => key.clone(),
        }
    }

    /// Relocate an exact source-created namespace; native and authored owners persist.
    #[must_use]
    pub fn namespace_key(
        &self,
        key: &crate::command_binding::SourceNamespaceKey,
    ) -> crate::command_binding::SourceNamespaceKey {
        use crate::command_binding::{CommandAllocationSite, SourceNamespaceKey};
        match key {
            SourceNamespaceKey::Allocated {
                site,
                incarnation,
                path,
            } => SourceNamespaceKey::Allocated {
                site: CommandAllocationSite {
                    source: self.source_origin(&site.source),
                    offset: self.source_offset(site.offset),
                },
                incarnation: *incarnation,
                path: path.clone(),
            },
            key => key.clone(),
        }
    }

    /// Text queries explicitly select only authored storage.
    #[must_use]
    pub fn storage_key<Q: VariableCellKeyQuery + ?Sized>(&self, key: &Q) -> VariableCellKey {
        self.cell_key(key.variable_cell_key().as_ref())
    }

    fn storage_map<T: Clone>(&self, values: &VariableCellTable<T>) -> VariableCellTable<T> {
        values
            .iter()
            .map(|(key, value)| (self.cell_key(key), value.clone()))
            .collect()
    }

    fn place_map(
        &self,
        values: &VariableCellTable<std::sync::Arc<Place>>,
    ) -> VariableCellTable<std::sync::Arc<Place>> {
        values
            .iter()
            .map(|(key, place)| (self.cell_key(key), std::sync::Arc::new(self.place(place))))
            .collect()
    }

    /// Relocate a typed selected activation while retaining external frame selections.
    #[must_use]
    pub fn frame(&self, frame: &VariableExecutionFrame) -> VariableExecutionFrame {
        if let VariableExecutionFrame::NamespaceIdentity { namespace, frame } = frame {
            return self
                .frame(frame)
                .with_namespace_identity(self.namespace_key(namespace));
        }
        let mut frame = frame.clone();
        if let VariableExecutionFrame::Procedure { identity, .. }
        | VariableExecutionFrame::NamespaceActivation { identity, .. }
        | VariableExecutionFrame::ReceiverMethod { identity } = &mut frame
            && let Some(replacement) = self.activations.get(identity)
        {
            identity.clone_from(replacement);
        }
        frame
    }

    pub(super) fn contents_origin(&self, origin: &ContentsOrigin) -> ContentsOrigin {
        match origin {
            ContentsOrigin::WrittenAt(source) => {
                ContentsOrigin::WrittenAt(self.source_offset(*source))
            }
            ContentsOrigin::Alternatives { incoming, writes } => {
                let mut writes: Vec<_> = writes
                    .iter()
                    .map(|source| self.source_offset(*source))
                    .collect();
                writes.sort_unstable();
                writes.dedup();
                ContentsOrigin::Alternatives {
                    incoming: *incoming,
                    writes,
                }
            }
            other => other.clone(),
        }
    }
}

impl PossibleVariableTraceRegistration {
    pub(super) fn relocated(&self, relocation: &VariableProofRelocation) -> Self {
        Self {
            target: relocation.place(&self.target),
            operations: self.operations.clone(),
            prefix: self.prefix.clone(),
        }
    }
}

impl ResolveContext {
    /// Relocate the entire binding and contents proof, including caller views and trace cells.
    /// No dialect, existence, observer, or unknown-world field is erased for memoisation.
    #[must_use]
    pub fn relocated(&self, relocation: &VariableProofRelocation) -> Self {
        let expanded = crate::allocated_instance::relocation_with_cell_keys(self, relocation);
        let relocation = &expanded;
        let mut context = self.clone();
        context.closed_observer_allocations = self
            .closed_observer_allocations
            .iter()
            .map(|allocation| {
                crate::allocated_instance::relocate_allocation(allocation, relocation)
            })
            .collect();
        context.namespace_identity = self
            .namespace_identity
            .as_ref()
            .map(|key| relocation.namespace_key(key));
        context.namespace_objects = self
            .namespace_objects
            .iter()
            .map(|(key, object)| (relocation.namespace_key(key), object.clone()))
            .collect();
        context.pending_namespace_retirements = self
            .pending_namespace_retirements
            .iter()
            .map(|(key, pending)| {
                (
                    relocation.namespace_key(key),
                    PendingNamespaceRetirement {
                        namespaces: pending
                            .namespaces
                            .iter()
                            .map(|key| relocation.namespace_key(key))
                            .collect(),
                        source: relocation.source_offset(pending.source),
                        conditional: pending.conditional,
                    },
                )
            })
            .collect();
        context.namespace_cells.present = self
            .namespace_cells
            .present
            .iter()
            .map(|key| relocation.cell_key(key))
            .collect();
        context.namespace_cells.possible = self
            .namespace_cells
            .possible
            .iter()
            .map(|key| relocation.cell_key(key))
            .collect();
        context.namespace_identities = self
            .namespace_identities
            .iter()
            .map(|key| relocation.namespace_key(key))
            .collect();
        context.namespace_addressable_identities = self
            .namespace_addressable_identities
            .iter()
            .map(|key| relocation.namespace_key(key))
            .collect();
        context.outward_namespace_destructions = self
            .outward_namespace_destructions
            .iter()
            .map(|key| relocation.namespace_key(key))
            .collect();
        context.contents_unknown_namespaces = self
            .contents_unknown_namespaces
            .iter()
            .map(|key| relocation.namespace_key(key))
            .collect();
        context.raw_bindings = self.raw_bindings.relocated(relocation);
        context.captured_cells = self.captured_cells.relocated(relocation);
        context.possible_trace_registrations = self
            .possible_trace_registrations
            .iter()
            .map(|registration| registration.relocated(relocation))
            .collect();
        context.dictionary_scopes = self
            .dictionary_scopes
            .iter()
            .map(|(source, activation)| {
                (
                    source.relocated(relocation),
                    std::sync::Arc::new(activation.relocated(relocation)),
                )
            })
            .collect();
        if let Some(identity) = &mut context.activation
            && let Some(replacement) = relocation.activations.get(identity)
        {
            identity.clone_from(replacement);
        }
        context.original_receiver_variable_candidates = self
            .original_receiver_variable_candidates
            .as_ref()
            .map(|candidates| std::sync::Arc::new(candidates.relocated(relocation)));
        context.relocate_storage_addresses(relocation);
        context.relocate_contents_and_observers(relocation);
        context
    }

    fn relocate_storage_addresses(&mut self, relocation: &VariableProofRelocation) {
        for map in [&mut self.alias_bindings, &mut self.name_alias_bindings] {
            *map = map
                .iter()
                .map(|(key, value)| (relocation.cell_key(key), relocation.place(value)))
                .collect();
        }
        for map in [
            &mut self.namespace_alias_bindings,
            &mut self.namespace_name_alias_bindings,
        ] {
            *map = map
                .iter()
                .map(|(key, value)| (relocation.storage_key(key), relocation.place(value)))
                .collect();
        }
        self.generations = self
            .generations
            .iter()
            .map(|(key, &generation)| {
                (
                    relocation.storage_key(key),
                    relocation.cell_generation(generation),
                )
            })
            .collect();
    }

    fn relocate_contents_and_observers(&mut self, relocation: &VariableProofRelocation) {
        self.value_representations = relocation.storage_map(&self.value_representations);
        self.constant_values = relocation.storage_map(&self.constant_values);
        self.closed_literal_contents = self
            .closed_literal_contents
            .iter()
            .map(|(key, contents)| (relocation.storage_key(key), contents.relocated(relocation)))
            .collect();
        self.original_name_values = self
            .original_name_values
            .iter()
            .map(|(key, value)| (relocation.storage_key(key), value.relocated(relocation)))
            .collect();
        self.contents_source_proofs = self.contents_source_proofs.relocated(relocation);
        self.contents_presence = relocation.storage_map(&self.contents_presence);
        self.contents_presence_slots = relocation.place_map(&self.contents_presence_slots);
        self.contents_kinds = relocation.storage_map(&self.contents_kinds);
        self.contents_origins = self
            .contents_origins
            .iter()
            .map(|(key, origin)| {
                (
                    relocation.storage_key(key),
                    relocation.contents_origin(origin),
                )
            })
            .collect();
        self.contents_unknown_arrays = self
            .contents_unknown_arrays
            .iter()
            .map(|key| relocation.storage_key(key))
            .collect();
        self.closed_array_roots = self
            .closed_array_roots
            .iter()
            .map(|key| relocation.storage_key(key))
            .collect();
        self.namespace_cells.closed_namespaces = self
            .namespace_cells
            .closed_namespaces
            .iter()
            .map(|key| relocation.namespace_key(key))
            .collect();
        self.trace_registrations = relocation.storage_map(&self.trace_registrations);
        self.trace_registration_receivers =
            relocation.place_map(&self.trace_registration_receivers);
        for names in [
            &mut self.traced,
            &mut self.untracked_traces,
            &mut self.unknown_bindings,
        ] {
            *names = names
                .iter()
                .map(|key| relocation.storage_key(key))
                .collect();
        }
        if let Some(caller) = &self.caller {
            self.caller = Some(std::sync::Arc::new(caller.relocated(relocation)));
        }
    }
}
