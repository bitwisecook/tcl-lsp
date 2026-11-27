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

//! What a read sees: contents origins, literal values, presence, and whether a read completes or is definitely missing.

use super::canonical_keys::{
    binding_key_in_namespace, canonical_binding_value_key, cell_key, physical_array_key,
};
use super::place_resolution::{resolve_literal_access, resolve_substitution_access};
use super::{
    ContentsOrigin, ContentsPresence, ResolveContext, RootContentsKind, VariableReadCompletion,
};
use crate::place;
use crate::place::{CellGeneration, CellOwner, IndexKind, Place, PlaceKind};
use tcl_registry::CommandRegistry;
use tcl_registry::native_dictionary::NativeDictionaryDialect;

impl ResolveContext {
    /// Last proved contents write to this binding, independently of root recreation.
    #[must_use]
    pub fn contents_origin(&self, place: &Place) -> ContentsOrigin {
        let Some(key) = canonical_binding_value_key(place) else {
            return ContentsOrigin::Unknown;
        };
        self.contents_origins.get(&key).cloned().unwrap_or_else(|| {
            if self.contents_world_unknown_for_key(&key)
                || self.array_contents_unknown(&key)
                || self
                    .contents_unknown_namespaces
                    .iter()
                    .any(|namespace| binding_key_in_namespace(&key, namespace))
            {
                ContentsOrigin::Unknown
            } else {
                ContentsOrigin::Incoming
            }
        })
    }

    /// Contents provenance visible to a read, accounting for implicit native observers.
    #[must_use]
    pub fn read_contents_origin(
        &self,
        place: &Place,
        registry: &CommandRegistry,
    ) -> ContentsOrigin {
        if self.implicit_read_changes_value(place, registry) {
            ContentsOrigin::Unknown
        } else {
            self.contents_origin(place)
        }
    }

    pub(crate) fn implicit_read_changes_value(
        &self,
        place: &Place,
        registry: &CommandRegistry,
    ) -> bool {
        if place.index.is_some() || place.ns != "::" || !matches!(place.kind, PlaceKind::Scalar) {
            return false;
        }
        let dialect = self.invocation_dialect.or_else(|| {
            registry
                .profile()
                .map(tcl_native_model::InvocationDialect::of_profile)
        });
        let Some(dialect) = dialect else {
            return false;
        };
        let variable = tcl_syntax::naming::qualify(&place.ns, &place.name);
        tcl_registry::special_vars::SPECIAL_VARS
            .iter()
            .filter_map(|spec| spec.runtime_hook)
            .any(|hook| {
                hook.changes_read_value() && hook.name_in(dialect) == Some(variable.as_str())
            })
    }

    /// Read a literal value only from the selected proved binding slot.
    #[must_use]
    pub fn literal_value(&self, variable: &str, registry: &CommandRegistry) -> Option<&str> {
        let access = resolve_literal_access(
            variable,
            self,
            false,
            registry,
            tcl_registry::TraceOperation::Read,
        );
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_VARIABLE_TRANSFER").is_some() {
            let key = canonical_binding_value_key(&access);
            eprintln!(
                "ORIGINAL_LITERAL_CONTENTS name_len={} kind={:?} observed={} dynamic={} presence={:?} key_present={} literal_present={} generation_current={} dynamic_bindings={} dynamic_traces={}",
                variable.len(),
                access.kind,
                access.observed,
                access.dynamic,
                self.contents_presence(&access),
                key.is_some(),
                key.as_ref()
                    .is_some_and(|key| self.constant_values.contains_key(key)),
                access
                    .cell
                    .as_ref()
                    .is_some_and(|cell| cell.generation != CellGeneration::Unknown
                        && self
                            .generations
                            .get(&cell_key(&access))
                            .copied()
                            .unwrap_or_default()
                            == cell.generation),
                self.dynamic_bindings,
                self.dynamic_traces
            );
        }
        self.literal_contents_at(&access, registry)
    }

    /// Read proved contents of an already selected physical cell. This never
    /// resolves the source spelling again or executes a read observer.
    #[must_use]
    pub fn literal_contents_at<'a>(
        &'a self,
        access: &Place,
        registry: &CommandRegistry,
    ) -> Option<&'a str> {
        self.literal_contents_for_access_at(access, registry, tcl_registry::TraceOperation::Read)
    }

    /// Read stored contents at an already selected operation boundary. A write
    /// result does not execute the implicit read observer used by later getters.
    #[must_use]
    pub fn literal_contents_for_access_at(
        &self,
        access: &Place,
        registry: &CommandRegistry,
        operation: tcl_registry::TraceOperation,
    ) -> Option<&str> {
        if access.observed
            || (operation == tcl_registry::TraceOperation::Read
                && self.implicit_read_changes_value(access, registry))
            || self.contents_presence(access) != ContentsPresence::Defined
        {
            return None;
        }
        if let Some(cell) = &access.cell {
            let current = self
                .generations
                .get(&cell_key(access))
                .copied()
                .unwrap_or_default();
            if cell.generation == CellGeneration::Unknown || cell.generation != current {
                return None;
            }
        }
        let name = canonical_binding_value_key(access)?;
        self.constant_values.get(&name).map(String::as_str)
    }

    /// Resolve a substitution's literal value using its original spelling and read callbacks.
    #[must_use]
    pub fn substitution_literal_value(
        &self,
        spelling: &str,
        registry: &CommandRegistry,
    ) -> Option<&str> {
        let access = resolve_substitution_access(
            spelling,
            self,
            registry,
            tcl_registry::TraceOperation::Read,
        );
        self.literal_contents_at(&access, registry)
    }

    /// Query contents existence without equating allocated cells with defined values.
    #[must_use]
    pub fn contents_presence(&self, place: &Place) -> ContentsPresence {
        if place.observed {
            return ContentsPresence::Unknown;
        }
        self.captured_contents_presence(place)
    }

    /// Actual presence in a captured post-operation world. This does not run a
    /// later observer; callers must retain the successful physical continuation.
    pub(crate) fn captured_contents_presence(&self, place: &Place) -> ContentsPresence {
        // A slot that holds a link has no contents of its own.
        if place.cell.is_some() && !place.is_global() {
            let slot = cell_key(place);
            if self.alias_bindings.contains_key(&slot)
                || self.name_alias_bindings.contains_key(&slot)
            {
                return ContentsPresence::Unknown;
            }
        }
        let key = canonical_binding_value_key(place);
        if let Some(presence) = key.as_ref().and_then(|key| self.contents_presence.get(key)) {
            return if *presence == ContentsPresence::DefinedOrUndefined {
                ContentsPresence::Unknown
            } else {
                *presence
            };
        }
        if place.index.is_some() {
            let mut root = place.clone();
            root.index = None;
            root.kind = PlaceKind::ArrayWhole;
            if self.captured_contents_presence(&root) == ContentsPresence::Undefined {
                return ContentsPresence::Undefined;
            }
        }
        let Some(key) = key else {
            return ContentsPresence::Unknown;
        };
        if place.is_global() && self.namespace_cells.captured_table_absent(&cell_key(place)) {
            return ContentsPresence::Undefined;
        }
        if place
            .index
            .as_ref()
            .is_some_and(|index| index.kind == place::IndexKind::Literal)
            && physical_array_key(place).is_some_and(|root| self.closed_array_roots.contains(&root))
            && !self.dynamic_bindings
            && !self.array_contents_unknown(&key)
            && !self
                .contents_unknown_namespaces
                .iter()
                .any(|namespace| binding_key_in_namespace(&key, namespace))
        {
            return ContentsPresence::Undefined;
        }
        if self.contents_world_unknown_for_key(&key)
            || self.array_contents_unknown(&key)
            || self
                .contents_unknown_namespaces
                .iter()
                .any(|namespace| binding_key_in_namespace(&key, namespace))
        {
            return ContentsPresence::Unknown;
        }
        if place.is_global() && self.namespace_cells.absent(&cell_key(place)) {
            return ContentsPresence::Undefined;
        }
        if matches!(
            place.cell.as_ref().map(|cell| &cell.owner),
            Some(CellOwner::Activation(identity)) if self.activation.as_ref() == Some(identity)
        ) && place.index.is_none()
            && !self.dynamic_bindings
            && !self.unknown_bindings.contains(&cell_key(place))
        {
            return ContentsPresence::Undefined;
        }
        ContentsPresence::Unknown
    }

    /// Closed contents presence for a purpose-only alternatives query.
    /// A mixed result licenses a possible missing-read diagnostic, never a
    /// value version or a definite native error. Opaque worlds remain absent.
    #[must_use]
    pub fn closed_contents_presence(&self, place: &Place) -> Option<ContentsPresence> {
        if place.observed {
            return None;
        }
        let presence = canonical_binding_value_key(place)
            .and_then(|key| self.contents_presence.get(&key).copied())
            .unwrap_or_else(|| self.contents_presence(place));
        (presence != ContentsPresence::Unknown).then_some(presence)
    }

    /// Query the originally selected cell after the native existence read boundary.
    /// Unknown callbacks and lifetimes cannot become a Boolean value proof.
    #[must_use]
    pub fn existence_after_read_at(
        &self,
        place: &Place,
        registry: &CommandRegistry,
    ) -> Option<bool> {
        if place.observed || self.implicit_read_changes_value(place, registry) {
            return None;
        }
        let cell = place.cell.as_ref()?;
        let current = self
            .generations
            .get(&cell_key(place))
            .copied()
            .unwrap_or_default();
        if cell.generation == CellGeneration::Unknown || cell.generation != current {
            return None;
        }
        match self.contents_presence(place) {
            ContentsPresence::Defined => Some(true),
            ContentsPresence::Undefined => Some(false),
            ContentsPresence::Unknown | ContentsPresence::DefinedOrUndefined => None,
        }
    }

    /// Prove a missing unobserved read without mistaking unknown values for absence.
    /// Defined array roots still need a selected scalar/array operation to prove success.
    #[must_use]
    pub fn read_completion(&self, place: &Place) -> VariableReadCompletion {
        if self.contents_presence(place) == ContentsPresence::Undefined
            || self.store_would_error(place)
        {
            VariableReadCompletion::Error
        } else {
            VariableReadCompletion::Unknown
        }
    }

    // A loop can lose the incarnation of a direct local slot while a later
    // successful scalar store still defines its current lookup contents. This
    // does not identify a captured cell or an element in a retired container.
    pub(super) fn current_scalar_slot_contents(&self, place: &Place) -> bool {
        !self.dynamic_bindings
            && !place.dynamic
            && !place.observed
            && place.kind == PlaceKind::Scalar
            && place.index.is_none()
            && place.cell.as_ref().is_some_and(|cell| {
                cell.generation == CellGeneration::Unknown
                    && matches!(&cell.owner, CellOwner::Activation(identity)
                        if self.activation.as_ref() == Some(identity))
                    && self.generations.get(&cell_key(place)) == Some(&CellGeneration::Unknown)
            })
            && self.contents_presence(place) == ContentsPresence::Defined
            && matches!(self.contents_origin(place), ContentsOrigin::WrittenAt(_))
            && matches!(
                self.contents_kinds.get(&cell_key(place)),
                Some(RootContentsKind::Scalar | RootContentsKind::ConstantScalar(_))
            )
    }

    pub(super) fn current_contents_generation(&self, place: &Place) -> bool {
        place.cell.as_ref().is_some_and(|cell| {
            (cell.generation != CellGeneration::Unknown
                && cell.generation
                    == self
                        .generations
                        .get(&cell_key(place))
                        .copied()
                        .unwrap_or_default())
                || self.current_scalar_slot_contents(place)
        })
    }

    /// A missing selected value before an unobserved native read. This requires
    /// a current physical address and closed absence/observer facts; unknown
    /// tables, dynamic selections and implicit producers retain alternatives.
    #[must_use]
    pub(crate) fn read_is_definitely_missing(
        &self,
        place: &Place,
        registry: &CommandRegistry,
    ) -> bool {
        if self
            .invocation_dialect
            .is_none_or(|dialect| dialect.variable_lookup_policy.is_none())
            || place.observed
            || place.dynamic
            || place
                .cell
                .as_ref()
                .is_none_or(|cell| matches!(cell.owner, CellOwner::SelectedFrame(_)))
            || !self.current_contents_generation(place)
            || self.contents_presence(place) != ContentsPresence::Undefined
            || self.implicit_read_changes_value(place, registry)
        {
            return false;
        }
        let observers =
            self.variable_observers_at(place, tcl_registry::TraceOperation::Read, registry);
        !observers.unknown_residual
            && observers.callbacks.is_empty()
            && observers.possible_callbacks.is_empty()
    }

    /// Whether this already selected native read is guaranteed to produce a
    /// value before any observer. Presence and SSA addressability alone cannot
    /// prove scalar/array compatibility or dictionary element lookup.
    #[must_use]
    pub fn read_produces_value(&self, place: &Place, registry: &CommandRegistry) -> bool {
        use tcl_dialect::VariableContainerModel;
        let Some(dialect) = self.invocation_dialect else {
            return false;
        };
        if dialect.variable_lookup_policy.is_none()
            || place.observed
            || place.dynamic
            || place
                .cell
                .as_ref()
                .is_none_or(|cell| matches!(cell.owner, CellOwner::SelectedFrame(_)))
            || self.implicit_read_changes_value(place, registry)
        {
            return false;
        }
        let dictionary_element = dialect.variable_container_model
            == Some(VariableContainerModel::DictionaryValue)
            && place.kind == PlaceKind::ArrayElem;
        if (!dictionary_element && self.contents_presence(place) != ContentsPresence::Defined)
            || !self.current_contents_generation(place)
        {
            return false;
        }
        let observers =
            self.variable_observers_at(place, tcl_registry::TraceOperation::Read, registry);
        if observers.unknown_residual
            || !observers.callbacks.is_empty()
            || !observers.possible_callbacks.is_empty()
        {
            return false;
        }
        match (dialect.variable_container_model, place.kind) {
            (Some(VariableContainerModel::DistinctArray), PlaceKind::Scalar) => {
                place.index.is_none()
                    && self.root_contents_kind(place) == Some(RootContentsKind::Scalar)
            }
            (Some(VariableContainerModel::DistinctArray), PlaceKind::ArrayElem) => {
                self.root_contents_kind(place) == Some(RootContentsKind::Array)
                    && place
                        .index
                        .as_ref()
                        .is_some_and(|index| index.kind == IndexKind::Literal)
            }
            (Some(VariableContainerModel::DictionaryValue), PlaceKind::Scalar) => {
                place.index.is_none()
            }
            (Some(VariableContainerModel::DictionaryValue), PlaceKind::ArrayElem) => {
                self.dictionary_element_read_produces_value(place, registry, dialect)
            }
            _ => false,
        }
    }

    fn dictionary_element_read_produces_value(
        &self,
        place: &Place,
        registry: &CommandRegistry,
        dialect: tcl_native_model::InvocationDialect,
    ) -> bool {
        let Some(index) = place
            .index
            .as_ref()
            .filter(|index| index.kind == IndexKind::Literal)
        else {
            return false;
        };
        let mut root = place.base();
        root.kind = PlaceKind::Scalar;
        if !self.read_produces_value(&root, registry) {
            return false;
        }
        if let Some(read) = self.original_name_read_result(&root, registry) {
            return dialect
                .dictionary_variable_value_ordinal(read.value().bytes(), index.value.as_bytes())
                .is_some();
        }
        // Explicit logical contents remain an authored compatibility adapter;
        // original produced values above always use their retained native bytes.
        if let Some(value) = self.literal_contents_at(&root, registry) {
            return dialect
                .dictionary_variable_value_ordinal(value.as_bytes(), index.value.as_bytes())
                .is_some();
        }
        matches!(self.array_contents_inventory(&place.base()),
            crate::array_destruction::ArrayContentsInventory::Known(members)
                if members.iter().any(|member| member.cell == place.cell && member.index == place.index))
    }
}
