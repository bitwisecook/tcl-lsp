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

//! Withdrawing proofs: original-contents epochs, native representations, and widening after evaluated code.

use super::{
    ContentsOrigin, ContentsPresence, ContentsWorld, NamespaceInventoryClosure, ResolveContext,
};
use tcl_names::place;
use tcl_names::place::CellGeneration;

impl ResolveContext {
    /// Original byte correspondence does not inherit representation cache currency.
    pub(crate) const fn original_contents_epoch(&self) -> Option<u64> {
        self.original_contents_epoch
    }

    /// Unknown content/binding/observer effects retire every previously frozen
    /// value owner. New exact producers may use the next bounded stamp; no
    /// content is reconstructed or reseeded from display text.
    pub(crate) fn invalidate_original_contents(&mut self) {
        self.original_contents_epoch_high_water = self
            .original_contents_epoch_high_water
            .and_then(|epoch| epoch.checked_add(1));
        self.original_contents_epoch = self.original_contents_epoch_high_water;
    }

    /// Withdraw native representation proofs after an operation may coerce shared objects.
    /// String contents, variable bindings and cell lifetimes remain authoritative.
    pub fn invalidate_shared_representations(&mut self) {
        self.invalidate_representations_for_values(None);
    }

    /// A coercion replaces intreps, never bytes: integer-convertible contents
    /// survive as a contents proof without any current representation. A
    /// builtin coercion of a stock literal leaves a stock-class object.
    pub(crate) fn coerce_representations_for_values(&mut self, values: Option<&[String]>) {
        use crate::native_numeric::StoredNativeRepresentation;
        let retained: Vec<_> = self
            .value_representations
            .iter()
            .filter_map(|(key, representation)| {
                let proof = representation.integer_contents_in(
                    self.constant_values.get(key).map(String::as_str),
                    self.invocation_dialect,
                );
                let kept = match (proof, representation) {
                    (Some(proof), _) => {
                        StoredNativeRepresentation::IntegerConvertibleContents(proof)
                    }
                    (None, StoredNativeRepresentation::StockLiteralObject(object)) => {
                        StoredNativeRepresentation::StockLiteralObject(
                            object.converted(tcl_syntax::value::ValueRepresentation::Unknown),
                        )
                    }
                    (None, _) => return None,
                };
                Some((key.clone(), kept))
            })
            .collect();
        self.invalidate_representations_for_values(values);
        for (key, kept) in retained {
            self.value_representations.entry(key).or_insert(kept);
        }
    }

    /// Objects with different current bytes cannot be the same native object.
    /// Unknown operand bytes withdraw every representation. Exact contents or
    /// an actual normalised numeric result-string recipe can prove disjointness.
    /// Numeric categories alone cannot preserve a representation.
    pub(crate) fn invalidate_representations_for_values(&mut self, values: Option<&[String]>) {
        self.value_representations.retain(|key, representation| {
            values.is_some_and(|values| {
                self.constant_values
                    .get(key)
                    .is_some_and(|contents| !values.contains(contents))
                    || matches!(representation,
                        crate::native_numeric::StoredNativeRepresentation::NumericShape(shape)
                        if values.iter().all(|value| !shape.string_bytes_may_equal(value.as_bytes())))
            })
        });
        if self.representation_epoch.is_some() {
            self.representation_epoch_high_water = self
                .representation_epoch_high_water
                .and_then(|epoch| epoch.checked_add(1));
            self.representation_epoch = self.representation_epoch_high_water;
        }
    }

    /// A possible ordinary-container conversion retains a closed cost domain
    /// for objects that may share the selected input. Strict intrep stays unknown.
    pub(crate) fn coerce_ordinary_container_representations(
        &mut self,
        values: Option<&[String]>,
        target: tcl_syntax::value::ValueRepresentation,
    ) {
        let Some(target_domain) = crate::native_numeric::ClosedContainerRepresentations::of(target)
        else {
            self.invalidate_representations_for_values(values);
            return;
        };
        let possible = self
            .value_representations
            .iter()
            .filter_map(|(key, value)| {
                let domain = value.container_alternatives()?;
                let disjoint = values.is_some_and(|values| {
                    self.constant_values
                        .get(key)
                        .is_some_and(|contents| !values.contains(contents))
                });
                (!disjoint).then(|| (key.clone(), domain.joined(target_domain)))
            })
            .collect::<Vec<_>>();
        let stock = self
            .value_representations
            .iter()
            .filter_map(|(key, value)| {
                let crate::native_numeric::StoredNativeRepresentation::StockLiteralObject(object) =
                    value
                else {
                    return None;
                };
                let disjoint = values.is_some_and(|values| {
                    self.constant_values
                        .get(key)
                        .is_some_and(|contents| !values.contains(contents))
                });
                (!disjoint).then(|| {
                    (
                        key.clone(),
                        crate::native_numeric::StoredNativeRepresentation::StockLiteralObject(
                            object.possibly_converted(target),
                        ),
                    )
                })
            })
            .collect::<Vec<_>>();
        self.invalidate_representations_for_values(values);
        self.value_representations.extend(
            possible
                .into_iter()
                .map(|(key, domain)| (key, domain.stored())),
        );
        self.value_representations.extend(stock);
    }

    /// An activation the module may enter at any later point (a procedure
    /// body walked as a deferred root) starts from no representation fact it
    /// could inherit: every held representation is forgotten, and a fresh
    /// stamp sequence is issued for the receipts its own stores make. An
    /// exhausted sequence at the module's end (a loop join of differing
    /// histories) otherwise denies the body every receipt it would issue.
    pub(crate) fn restart_representation_world(&mut self) {
        if self.representation_epoch_high_water.is_some() {
            return;
        }
        self.value_representations.clear();
        self.representation_epoch_high_water = Some(0);
        self.representation_epoch = Some(0);
    }

    /// A loop's exit state, whose joined stamp histories exhausted the
    /// sequence, resumes in a stamp domain of the loop's own `site`: every
    /// pass of an enclosing fixpoint resumes at the same stamp, and no stamp
    /// issued before the loop, or by another loop, can equal one issued here.
    /// Representations the join kept are equal on both sides, so they hold
    /// in the joined world. An exhaustion the loop did not cause stands.
    pub(crate) fn resume_representation_world_at(&mut self, site: u32, entered: bool) {
        if !entered || self.representation_epoch_high_water.is_some() {
            return;
        }
        let stamp = (1_u64 << 63) | (u64::from(site) << 24);
        self.representation_epoch_high_water = Some(stamp);
        self.representation_epoch = Some(stamp);
    }

    /// A selected successful native constructor can issue a new representation
    /// receipt after an uncertain join. It cannot revive an older receipt.
    pub(crate) fn establish_created_representation_epoch(&mut self) {
        if self.representation_epoch.is_some() {
            return;
        }
        self.value_representations.clear();
        self.representation_epoch_high_water = self
            .representation_epoch_high_water
            .and_then(|epoch| epoch.checked_add(1));
        self.representation_epoch = self.representation_epoch_high_water;
    }

    /// [`Self::widen`] for code declared to install no variable observer (a
    /// document-declared command running in the caller's frame): contents,
    /// bindings and lifetimes widen, the enumerated observers stand.
    pub fn widen_keeping_observers(&mut self) {
        let dynamic_traces = self.dynamic_traces;
        let inventory = self.activation_observer_inventory;
        let closed = self.closed_observer_allocations.clone();
        let caller = self.caller.clone();
        self.widen();
        self.dynamic_traces = dynamic_traces;
        self.activation_observer_inventory = inventory;
        self.closed_observer_allocations = closed;
        self.caller = caller;
    }

    /// Evaluated code may retarget bindings and change traces and lifetimes.
    pub fn widen(&mut self) {
        self.invalidate_original_contents();
        self.closed_array_roots.clear();
        self.namespace_inventory = NamespaceInventoryClosure::Open;
        self.captured_cells.widen();
        for slot in self.raw_bindings.slots.values_mut() {
            slot.contents = crate::raw_binding::RawBindingContents::Unknown;
        }
        self.namespace_cells.widen();
        self.retain_literal_values(|_| false);
        self.invalidate_shared_representations();
        self.contents_world = ContentsWorld::Unknown;
        self.withdraw_activation_contents_closure();
        self.contents_kinds.clear();
        for origin in self.contents_origins.values_mut() {
            *origin = ContentsOrigin::Unknown;
        }
        for presence in self.contents_presence.values_mut() {
            *presence = ContentsPresence::Unknown;
        }

        self.original_receiver_variable_candidates = None;
        self.dynamic_bindings = true;
        self.mark_unenumerated_variable_observers();
        self.unknown_bindings
            .extend(self.alias_bindings.keys().cloned());
        self.alias_bindings.clear();
        self.name_alias_bindings.clear();
        for alias in self.namespace_name_alias_bindings.values_mut() {
            *alias = place::unknown_top();
        }
        for alias in self.namespace_alias_bindings.values_mut() {
            *alias = place::unknown_top();
        }
        self.known_namespaces
            .retain(|name| name == "::" || name == &self.namespace);
        for generation in self.generations.values_mut() {
            *generation = CellGeneration::Unknown;
        }
    }
}
