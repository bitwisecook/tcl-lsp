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

//! Publishing stores: contents writes, literal and unknown definitions, captured receivers, root kinds and representations.

use super::canonical_keys::{
    binding_key_in_namespace, canonical_binding_value_key, canonical_literal_variable_key,
    cell_key, physical_array_key,
};
use super::cell_key::VariableCellKey;
use super::place_resolution::{resolve_literal_access, resolve_literal_place};
use super::{
    ContentsOrigin, ContentsPresence, ContentsWorld, ResolveContext, RootContentsKind,
    contents_presence_after_write,
};
use crate::place;
use crate::place::{CellGeneration, CellOwner, Place, PlaceKind};
use tcl_registry::CommandRegistry;

impl ResolveContext {
    /// Invalidate literal contents selected by a canonical mutation footprint.
    /// Bounded unknown array elements affect only the proved physical root.
    pub fn invalidate_contents_literals(&mut self, place: &Place) {
        if place.observed || (place.kind == PlaceKind::Unknown && place.ns == place::LOCAL_NS) {
            self.invalidate_original_contents();
            self.retain_literal_values(|_| false);
            self.invalidate_shared_representations();
            return;
        }
        if place.kind == PlaceKind::Unknown {
            let namespace = self.namespace_footprint(place);
            if namespace.is_none() {
                self.invalidate_original_contents();
            }
            self.retain_literal_values(|key| {
                namespace
                    .as_ref()
                    .is_some_and(|namespace| !key.is_in_namespace(namespace))
            });
            self.value_representations.retain(|key, _| {
                namespace
                    .as_ref()
                    .is_some_and(|namespace| !key.is_in_namespace(namespace))
            });
            return;
        }
        if let Some(key) = canonical_binding_value_key(place) {
            self.forget_literal_value(&key);
            self.value_representations.remove(&key);
        }
        if place.index.is_none()
            || place
                .index
                .as_ref()
                .is_some_and(|index| index.kind != place::IndexKind::Literal)
        {
            let root = physical_array_key(place);
            if let Some(root) = root {
                self.retain_literal_values(|key| !key.is_member_of(&root));
                self.value_representations
                    .retain(|key, _| !key.is_member_of(&root));
            }
        }
        let mut root = place.clone();
        root.index = None;
        root.kind = PlaceKind::Scalar;
        if let Some(key) = canonical_binding_value_key(&root) {
            self.forget_literal_value(&key);
            self.value_representations.remove(&key);
        }
    }

    /// Publish one normal contents store or a typed unresolved write footprint.
    pub fn record_contents_write(&mut self, place: &Place, source: u32, conditional: bool) {
        let previous_presence = self.closed_contents_presence(place);
        self.record_presence_slot(place);
        self.record_contents_source(place, conditional);
        self.record_root_kind(place, conditional);
        self.invalidate_contents_literals(place);
        if place.observed || (place.kind == PlaceKind::Unknown && place.ns == place::LOCAL_NS) {
            self.closed_array_roots.clear();
            self.namespace_cells.closed_namespaces.clear();
            self.contents_world = ContentsWorld::Unknown;
            self.withdraw_activation_contents_closure();
            for origin in self.contents_origins.values_mut() {
                *origin = ContentsOrigin::Unknown;
            }
            for presence in self.contents_presence.values_mut() {
                *presence = ContentsPresence::Unknown;
            }
        } else if place.kind == PlaceKind::Unknown {
            self.closed_array_roots.clear();
            self.namespace_cells.closed_namespaces.clear();
            let Some(namespace) = self.namespace_footprint(place) else {
                self.contents_world = ContentsWorld::Unknown;
                self.withdraw_activation_contents_closure();
                for origin in self.contents_origins.values_mut() {
                    *origin = ContentsOrigin::Unknown;
                }
                for presence in self.contents_presence.values_mut() {
                    *presence = ContentsPresence::Unknown;
                }
                return;
            };
            self.contents_unknown_namespaces.insert(namespace.clone());
            for (name, presence) in &mut self.contents_presence {
                if binding_key_in_namespace(name, &namespace) {
                    *presence = ContentsPresence::Unknown;
                }
            }
            for (name, origin) in &mut self.contents_origins {
                if binding_key_in_namespace(name, &namespace) {
                    *origin = ContentsOrigin::Unknown;
                }
            }
        } else if place.kind == PlaceKind::ArrayWhole
            || place
                .index
                .as_ref()
                .is_some_and(|index| index.kind != place::IndexKind::Literal)
        {
            let Some(root) = physical_array_key(place) else {
                self.invalidate_original_contents();
                self.contents_world = ContentsWorld::Unknown;
                self.withdraw_activation_contents_closure();
                return;
            };
            self.contents_unknown_arrays.insert(root.clone());
            for (key, origin) in &mut self.contents_origins {
                if key.is_member_of(&root) {
                    *origin = origin.joined(&ContentsOrigin::WrittenAt(source));
                }
            }
            let presence = contents_presence_after_write(previous_presence, conditional);
            self.contents_presence.insert(cell_key(place), presence);
        } else if let Some(key) = canonical_binding_value_key(place) {
            let presence = contents_presence_after_write(previous_presence, conditional);
            self.contents_presence.insert(key.clone(), presence);
            if place.index.is_some() {
                let root = cell_key(place);
                self.contents_presence.insert(root, presence);
            }
            self.contents_origins.insert(
                key,
                if conditional {
                    self.contents_origin(place)
                        .joined(&ContentsOrigin::WrittenAt(source))
                } else {
                    ContentsOrigin::WrittenAt(source)
                },
            );
        }
    }

    /// Record a possible deletion of one unknown element while retaining its array root.
    pub fn record_unknown_element_destruction(&mut self, place: &Place) {
        self.captured_cells
            .destroy(place, Some(RootContentsKind::Array));
        let Some(root) = physical_array_key(place) else {
            self.record_contents_write(&crate::place::unknown_top(), 0, true);
            return;
        };
        self.contents_unknown_arrays.insert(root.clone());
        for (key, origin) in &mut self.contents_origins {
            if key.is_member_of(&root) {
                *origin = ContentsOrigin::Unknown;
            }
        }
        for (key, presence) in &mut self.contents_presence {
            if key.is_member_of(&root) {
                *presence = ContentsPresence::Unknown;
            }
        }
        self.retain_literal_values(|key| !key.is_member_of(&root));
        self.value_representations
            .retain(|key, _| !key.is_member_of(&root));
    }

    /// Define one proven literal parameter or successfully stored value.
    pub fn define_literal(&mut self, variable: &str, value: &str, registry: &CommandRegistry) {
        let target = resolve_literal_place(variable, self, false, registry);
        if let Some(name) = canonical_binding_value_key(&target) {
            self.publish_defined_slot(&target, &name);
            self.value_representations.remove(&name);
            self.store_literal_value(name, value.to_owned());
        }
    }

    /// Publish a successful store whose contents are not known: the slot is
    /// defined and holds no literal. The recorded write origin stays.
    pub fn define_stored_contents(&mut self, variable: &str, registry: &CommandRegistry) {
        let target = resolve_literal_place(variable, self, false, registry);
        if let Some(name) = canonical_binding_value_key(&target) {
            self.publish_defined_slot(&target, &name);
            self.value_representations.remove(&name);
            self.forget_literal_value(&name);
        }
    }

    /// Record the stock class of the object a literal definition stored.
    pub(crate) fn retain_stock_literal_object(
        &mut self,
        variable: &str,
        object: crate::command_binding::SourceStockLiteralObject,
        registry: &CommandRegistry,
    ) {
        let target = resolve_literal_place(variable, self, false, registry);
        if let Some(name) = canonical_binding_value_key(&target) {
            self.value_representations.insert(
                name,
                crate::native_numeric::StoredNativeRepresentation::StockLiteralObject(object),
            );
        }
    }

    /// Withdraw namespace value and lifetime facts at an externally callable
    /// deferred entry. Fresh local slots and formal ingress remain independent;
    /// actual calls must retain their caller's authoritative namespace world.
    pub fn widen_deferred_namespace_inputs(&mut self) {
        self.closed_array_roots
            .retain(|key| !key.is_namespace_storage());
        self.namespace_cells.widen();
        self.contents_unknown_namespaces
            .extend(self.namespace_identities.iter().cloned());
        self.contents_unknown_namespaces.insert("::".to_owned());
        self.retain_literal_values(|key| !key.is_namespace_storage());
        self.value_representations
            .retain(|key, _| !key.is_namespace_storage());
        self.contents_kinds
            .retain(|key, _| !key.is_namespace_storage());
        for (key, origin) in &mut self.contents_origins {
            if key.is_namespace_storage() {
                *origin = ContentsOrigin::Unknown;
            }
        }
        for (key, presence) in &mut self.contents_presence {
            if key.is_namespace_storage() {
                *presence = ContentsPresence::Unknown;
            }
        }
        for (key, generation) in &mut self.generations {
            if key.is_namespace_storage() {
                *generation = CellGeneration::Unknown;
            }
        }
        for target in self
            .alias_bindings
            .values_mut()
            .chain(self.namespace_alias_bindings.values_mut())
        {
            if let Some(cell) = &mut target.cell
                && matches!(
                    cell.owner,
                    CellOwner::Namespace(_) | CellOwner::NamespaceIdentity(_)
                )
            {
                cell.generation = CellGeneration::Unknown;
            }
        }
    }

    /// Capture one active receiver while its physical owner remains distinguishable.
    /// Repeated recursive activations sharing one abstract allocation family do
    /// not establish that a later destruction addresses the same instance.
    pub fn capture_protected_cell(
        &mut self,
        receiver: &Place,
    ) -> Option<crate::captured_cell::CapturedCellLeaseId> {
        let owner = &receiver.cell.as_ref()?.owner;
        match owner {
            CellOwner::Namespace(_) | CellOwner::NamespaceIdentity(_) => {}
            CellOwner::AllocatedInstance(allocation)
                if allocation.incarnation
                    != crate::command_binding::AllocationIncarnation::RepeatedFresh => {}
            CellOwner::Activation(identity) => {
                let mut occurrences = 0;
                let mut frame = Some(&*self);
                while let Some(current) = frame {
                    occurrences += usize::from(current.activation.as_ref() == Some(identity));
                    frame = current.caller.as_deref();
                }
                if occurrences != 1 {
                    return None;
                }
            }
            CellOwner::RetainedSlot(_) if self.raw_bindings.active_array_member(receiver) => {}
            _ => return None,
        }
        self.captured_cells.capture(receiver)
    }

    /// Select a protected live shell after callbacks, without consulting its former alias.
    /// A contents reset may refresh scalar contents generation; retired owners never revive.
    #[must_use]
    pub fn captured_cell_receiver(
        &self,
        id: crate::captured_cell::CapturedCellLeaseId,
    ) -> Option<Place> {
        use crate::captured_cell::CapturedCellState;
        if !matches!(
            self.captured_cells.state(id),
            CapturedCellState::Live | CapturedCellState::ContentsReset
        ) {
            return None;
        }
        let mut receiver = self.captured_cells.receiver(id)?.clone();
        let generation = self
            .generations
            .get(&cell_key(&receiver))
            .copied()
            .unwrap_or_default();
        if generation == CellGeneration::Unknown {
            return None;
        }
        if receiver.index.is_none()
            && let Some(cell) = &mut receiver.cell
        {
            cell.generation = generation;
        }
        receiver.observed = false;
        Some(receiver)
    }

    /// Conversion hooks of contents selected by a completed protected read.
    /// This checks the retained physical shell and current contents generation,
    /// without asking whether a future read would execute observers again.
    #[must_use]
    pub(crate) fn captured_numeric_input_hooks_closed(
        &self,
        id: crate::captured_cell::CapturedCellLeaseId,
    ) -> bool {
        self.captured_cell_receiver(id).is_some_and(|receiver| {
            self.contents_presence(&receiver) == ContentsPresence::Defined
                && self.current_contents_generation(&receiver)
                && self.selected_contents_numeric_hooks_closed(&receiver)
        })
    }

    /// Publish a successful native store to the already captured physical receiver.
    /// The caller owns observer delivery after this store; this does not resolve
    /// a variable spelling or invoke a callback.
    pub(crate) fn publish_captured_store(
        &mut self,
        receiver: &Place,
        value: Option<&str>,
        source: u32,
    ) {
        let mut stored = receiver.clone();
        stored.observed = false;
        let mut root = stored.clone();
        root.index = None;
        root.kind = PlaceKind::ArrayWhole;
        let created_array = self
            .invocation_dialect
            .and_then(|dialect| dialect.variable_container_model)
            == Some(tcl_dialect::VariableContainerModel::DistinctArray)
            && stored.index.as_ref().is_some_and(|index| {
                index.kind == place::IndexKind::Literal
                    && self.contents_presence(&root) == ContentsPresence::Undefined
            });
        self.record_contents_write(&stored, source, false);
        if created_array && let Some(root) = physical_array_key(&root) {
            self.closed_array_roots.insert(root);
        }
        if let Some(name) = canonical_binding_value_key(&stored) {
            self.publish_defined_slot(&stored, &name);
            if let Some(value) = value {
                self.store_literal_value(name, value.to_owned());
            }
        }
    }

    pub(crate) fn record_presence_slot(&mut self, place: &Place) {
        if let Some(key) = canonical_binding_value_key(place) {
            self.contents_presence_slots
                .insert(key, std::sync::Arc::new(place.clone()));
        }
        if place.index.is_some() {
            let mut root = place.clone();
            root.index = None;
            root.kind = PlaceKind::ArrayWhole;
            if let Some(key) = canonical_binding_value_key(&root) {
                self.contents_presence_slots
                    .insert(key, std::sync::Arc::new(root));
            }
        }
    }

    pub(super) fn publish_defined_slot(&mut self, target: &Place, name: &VariableCellKey) {
        self.record_presence_slot(target);
        self.record_root_kind(target, false);
        self.contents_presence
            .insert(name.to_owned(), ContentsPresence::Defined);
        if target.index.is_some() {
            self.contents_presence
                .insert(cell_key(target), ContentsPresence::Defined);
        }
        if target.is_global() && !target.dynamic {
            self.namespace_cells.present.insert(cell_key(target));
        }
    }

    /// Record namespace slot allocation for a link, retaining the undefined
    /// contents of a stable cell independently of the occupied table slot.
    pub(crate) fn allocate_namespace_alias_target(&mut self, target: &Place) {
        if !target.is_global() || target.dynamic {
            return;
        }
        if self
            .invocation_dialect
            .and_then(|dialect| dialect.variable_link_binding)
            == Some(tcl_dialect::VariableLinkBinding::StableCell)
            && self.contents_presence(target) == ContentsPresence::Undefined
            && let Some(key) = canonical_binding_value_key(target)
        {
            self.record_presence_slot(target);
            self.contents_presence
                .insert(key, ContentsPresence::Undefined);
        }
        self.namespace_cells.present.insert(cell_key(target));
    }

    /// Root kind of the same selected live physical cell, before observers run.
    #[must_use]
    pub fn root_contents_kind(&self, place: &Place) -> Option<RootContentsKind> {
        place.cell.as_ref()?;
        let key = cell_key(place);
        if place.observed || !self.current_contents_generation(place) {
            return None;
        }
        self.contents_kinds.get(&key).copied()
    }

    /// Whether an ordinary scalar/element store is proved to fail before writing.
    /// Unknown root kinds and observers cannot donate an error-only route.
    #[must_use]
    pub fn store_would_error(&self, place: &Place) -> bool {
        if let Some(index) = &place.index
            && let Some(CellOwner::RetainedSlot(identity)) =
                place.cell.as_ref().map(|cell| &cell.owner)
            && self
                .raw_bindings
                .retired_array_members
                .get(&identity.storage_key())
                .is_some_and(|members| members.contains(&index.value))
        {
            return true;
        }
        if self
            .invocation_dialect
            .and_then(|dialect| dialect.variable_container_model)
            != Some(tcl_dialect::VariableContainerModel::DistinctArray)
        {
            return false;
        }
        matches!(
            (place.kind, self.root_contents_kind(place)),
            (PlaceKind::Scalar, Some(RootContentsKind::Array))
                | (PlaceKind::ArrayElem, Some(RootContentsKind::Scalar))
        )
    }

    fn record_root_kind(&mut self, place: &Place, conditional: bool) {
        if place.observed || (place.kind == PlaceKind::Unknown && place.ns == place::LOCAL_NS) {
            self.contents_kinds.clear();
            return;
        }
        if place.kind == PlaceKind::Unknown {
            let namespace = self.namespace_footprint(place);
            self.contents_kinds.retain(|key, _| {
                namespace
                    .as_ref()
                    .is_some_and(|namespace| !key.is_in_namespace(namespace))
            });
            return;
        }
        let kind = match place.kind {
            PlaceKind::Scalar | PlaceKind::DictPath => RootContentsKind::Scalar,
            PlaceKind::ArrayElem | PlaceKind::ArrayWhole => RootContentsKind::Array,
            _ => return,
        };
        let key = cell_key(place);
        if conditional {
            if self.contents_kinds.get(&key) != Some(&kind) {
                self.contents_kinds.remove(&key);
            }
        } else {
            self.contents_kinds.insert(key, kind);
        }
    }

    /// Store known text with independently proved native representation.
    pub fn define_literal_with_representation(
        &mut self,
        variable: &str,
        value: &str,
        representation: tcl_syntax::value::ValueRepresentation,
        registry: &CommandRegistry,
    ) {
        self.define_literal(variable, value, registry);
        if let Some(name) = canonical_literal_variable_key(variable, self, registry) {
            self.value_representations
                .insert(name, representation.into());
        }
    }

    /// Representation of the same selected, currently defined physical value.
    #[must_use]
    pub fn contents_representation_at(
        &self,
        place: &Place,
    ) -> tcl_syntax::value::ValueRepresentation {
        let valid = !place.observed
            && self.contents_presence(place) == ContentsPresence::Defined
            && self.current_contents_generation(place);
        if !valid {
            return tcl_syntax::value::ValueRepresentation::Unknown;
        }
        canonical_binding_value_key(place)
            .and_then(|name| self.value_representations.get(&name))
            .map_or_default(crate::native_numeric::StoredNativeRepresentation::representation)
    }

    /// Closed possible ordinary container representations at this live read.
    /// This cost-advice projection never upgrades the strict representation query.
    pub(crate) fn container_representation_alternatives_at(
        &self,
        place: &Place,
    ) -> Option<crate::native_numeric::ClosedContainerRepresentations> {
        if place.observed
            || place.dynamic
            || self.contents_presence(place) != ContentsPresence::Defined
            || !self.current_contents_generation(place)
        {
            return None;
        }
        self.value_representations
            .get(&canonical_binding_value_key(place)?)?
            .container_alternatives()
    }

    pub(crate) fn contents_list_method_provider_at(
        &self,
        place: &Place,
        registry: &CommandRegistry,
    ) -> Option<tcl_registry::native_result::NativeListMethodProvider> {
        if !self.read_produces_value(place, registry) {
            return None;
        }
        match self
            .value_representations
            .get(&canonical_binding_value_key(place)?)?
        {
            crate::native_numeric::StoredNativeRepresentation::ReadOnlyListMethodProvider(
                provider,
            ) => Some(*provider),
            _ => None,
        }
    }

    /// Allocate a definitely bound parameter or store whose contents are unknown.
    /// This retains its cell address without inventing a literal value.
    pub fn define_unknown_contents(&mut self, variable: &str, registry: &CommandRegistry) {
        let target = resolve_literal_access(
            variable,
            self,
            false,
            registry,
            tcl_registry::TraceOperation::Write,
        );
        let Some(name) = canonical_binding_value_key(&target) else {
            return;
        };
        self.forget_literal_value(&name);
        self.value_representations.remove(&name);
        self.publish_defined_slot(&target, &name);
        self.contents_origins.insert(name, ContentsOrigin::Unknown);
    }

    /// Define a bound value with known representation and unknown string contents.
    pub fn define_unknown_contents_with_representation(
        &mut self,
        variable: &str,
        representation: tcl_syntax::value::ValueRepresentation,
        registry: &CommandRegistry,
    ) {
        self.define_unknown_contents(variable, registry);
        if let Some(name) = canonical_literal_variable_key(variable, self, registry) {
            self.value_representations
                .insert(name, representation.into());
        }
    }
}
