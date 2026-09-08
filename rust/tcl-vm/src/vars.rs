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
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Stable variable-cell storage shared by frames and namespace tokens.

use std::collections::{BTreeMap, HashMap, HashSet};

use tcl_core_types::VarId;
use tcl_runtime_api::FrameLinkOrigin;

use crate::value::Value;

/// A name table owns bindings, while the arena owns the cells they identify.
/// Removing a binding never makes its `VarId` identify a later variable.
pub(crate) type VarTable = BTreeMap<String, VarId>;

/// The mutable state carried by one Tcl `Var` cell.
pub(crate) enum VarState {
    /// Materialised but unset (for `variable` and `trace add variable`).
    Undefined,
    /// A scalar value.
    Scalar(Value),
    /// An array whose element names bind stable cells of their own.
    Array(VarTable),
    /// An `upvar`/`global`/`variable` alias to its target cell.
    Link(VarId),
}

/// One stable variable cell. Cells are never reused for a later binding.
pub(crate) struct VarCell {
    state: VarState,
    /// The containing array cell and element key, for Tcl's alias reporting.
    /// This remains on a detached element so an `upvar` alias cannot silently
    /// retarget a later same-name element.
    parent: Option<(VarId, String)>,
    bindings: u64,
    link_refs: u64,
    pins: u64,
    operation_refs: u64,
    array_revision: u64,
    link_origin: FrameLinkOrigin,
}

impl VarCell {
    pub(crate) const fn state(&self) -> &VarState {
        &self.state
    }

    pub(crate) const fn link_origin(&self) -> FrameLinkOrigin {
        self.link_origin
    }
}

/// Monotonic arena for every variable cell in one interpreter.
#[derive(Default)]
pub(crate) struct VarArena {
    next: u64,
    cells: HashMap<VarId, VarCell>,
}

impl VarArena {
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.cells.len()
    }

    pub(crate) fn alloc(&mut self, state: VarState) -> Option<VarId> {
        self.alloc_with_parent(state, None, FrameLinkOrigin::Ordinary)
    }

    pub(crate) fn alloc_link(&mut self, target: VarId, origin: FrameLinkOrigin) -> Option<VarId> {
        self.alloc_with_parent(VarState::Link(target), None, origin)
    }

    pub(crate) fn alloc_element(
        &mut self,
        state: VarState,
        parent: VarId,
        key: String,
    ) -> Option<VarId> {
        self.alloc_with_parent(state, Some((parent, key)), FrameLinkOrigin::Ordinary)
    }

    fn alloc_with_parent(
        &mut self,
        state: VarState,
        parent: Option<(VarId, String)>,
        link_origin: FrameLinkOrigin,
    ) -> Option<VarId> {
        if let VarState::Link(target) = &state
            && self.resolve(*target).is_none()
        {
            return None;
        }
        let raw = u32::try_from(self.next).ok()?;
        self.next = self.next.checked_add(1)?;
        let id = VarId(raw);
        let link = match &state {
            VarState::Link(target) => Some(*target),
            _ => None,
        };
        self.cells.insert(
            id,
            VarCell {
                state,
                parent,
                bindings: 0,
                link_refs: 0,
                pins: 0,
                operation_refs: 0,
                array_revision: 0,
                link_origin,
            },
        );
        if let Some(target) = link
            && let Some(cell) = self.cells.get_mut(&target)
        {
            cell.link_refs = cell
                .link_refs
                .checked_add(1)
                .expect("variable link reference count overflow");
        }
        Some(id)
    }

    pub(crate) fn get(&self, id: VarId) -> Option<&VarCell> {
        self.cells.get(&id)
    }

    pub(crate) fn array_insert(&mut self, id: VarId, key: String, element: VarId) -> bool {
        let Some(cell) = self.cells.get_mut(&id) else {
            return false;
        };
        let VarState::Array(elements) = &mut cell.state else {
            return false;
        };
        if elements.contains_key(&key) {
            return false;
        }
        elements.insert(key, element);
        cell.array_revision = cell.array_revision.wrapping_add(1);
        true
    }

    pub(crate) fn array_remove(&mut self, id: VarId, key: &str) -> Option<VarId> {
        let cell = self.cells.get_mut(&id)?;
        let VarState::Array(elements) = &mut cell.state else {
            return None;
        };
        let removed = elements.remove(key)?;
        cell.array_revision = cell.array_revision.wrapping_add(1);
        Some(removed)
    }

    /// Discard an element only while its table slot still names `expected`.
    /// Garbage-collecting an undefined trace shell is not a Tcl-level array
    /// mutation and therefore does not invalidate an active search.
    pub(crate) fn array_discard_if(
        &mut self,
        id: VarId,
        key: &str,
        expected: VarId,
    ) -> Option<VarId> {
        let cell = self.cells.get_mut(&id)?;
        let VarState::Array(elements) = &mut cell.state else {
            return None;
        };
        if elements.get(key) != Some(&expected) {
            return None;
        }
        elements.remove(key)
    }

    pub(crate) fn array_revision(&self, id: VarId) -> Option<u64> {
        let cell = self.cells.get(&id)?;
        matches!(cell.state, VarState::Array(_)).then_some(cell.array_revision)
    }

    pub(crate) fn has_link_refs(&self, id: VarId) -> bool {
        self.cells.get(&id).is_some_and(|cell| cell.link_refs != 0)
    }

    pub(crate) fn element_parent(&self, id: VarId) -> Option<(VarId, &str)> {
        self.cells
            .get(&id)?
            .parent
            .as_ref()
            .map(|(parent, key)| (*parent, key.as_str()))
    }

    pub(crate) fn element_is_attached(&self, id: VarId) -> bool {
        let Some((parent, key)) = self.element_parent(id) else {
            return true;
        };
        let Some(VarState::Array(elements)) = self.get(parent).map(VarCell::state) else {
            return false;
        };
        elements
            .get(key)
            .copied()
            .and_then(|element| self.resolve(element))
            == Some(id)
    }

    pub(crate) fn replace_state(&mut self, id: VarId, state: VarState) -> bool {
        let new_link = match &state {
            VarState::Link(target) => Some(*target),
            _ => None,
        };
        if let Some(target) = new_link
            && self.resolve(target).is_none_or(|resolved| resolved == id)
        {
            return false;
        }
        let Some(cell) = self.cells.get_mut(&id) else {
            return false;
        };
        let old_was_array = matches!(cell.state, VarState::Array(_));
        let new_is_array = matches!(state, VarState::Array(_));
        let old = std::mem::replace(&mut cell.state, state);
        cell.link_origin = FrameLinkOrigin::Ordinary;
        if old_was_array || new_is_array {
            cell.array_revision = cell.array_revision.wrapping_add(1);
        }
        if let Some(target) = new_link
            && let Some(target_cell) = self.cells.get_mut(&target)
        {
            target_cell.link_refs = target_cell
                .link_refs
                .checked_add(1)
                .expect("variable link reference count overflow");
        }
        match old {
            VarState::Link(target) => {
                if let Some(target_cell) = self.cells.get_mut(&target) {
                    target_cell.link_refs = target_cell
                        .link_refs
                        .checked_sub(1)
                        .expect("variable link reference count underflow");
                }
                self.collect(target);
            }
            VarState::Array(elements) => {
                for element in elements.into_values() {
                    self.unbind(element);
                }
            }
            VarState::Undefined | VarState::Scalar(_) => {}
        }
        true
    }

    /// Apply Tcl's unset transition and invalidate an active parent-array
    /// search whenever this is an attached element. Even an already undefined
    /// trace/link shell cancels the search; a generic state replacement does
    /// not carry that operation-level meaning.
    pub(crate) fn unset_state(&mut self, id: VarId) -> Option<VarState> {
        let attached_parent = self
            .element_is_attached(id)
            .then(|| self.element_parent(id).map(|(parent, _)| parent))
            .flatten();
        let old = self.take_state(id)?;
        if let Some(parent) = attached_parent
            && let Some(parent_cell) = self.cells.get_mut(&parent)
        {
            parent_cell.array_revision = parent_cell.array_revision.wrapping_add(1);
        }
        Some(old)
    }

    /// Detach a cell's current state without releasing bindings owned by an
    /// array table inside it. Destructive lifecycle code walks those element
    /// bindings one at a time because callbacks can observe and mutate the
    /// remaining old cells.
    pub(crate) fn take_state(&mut self, id: VarId) -> Option<VarState> {
        let cell = self.cells.get_mut(&id)?;
        let old = std::mem::replace(&mut cell.state, VarState::Undefined);
        if matches!(old, VarState::Array(_)) {
            cell.array_revision = cell.array_revision.wrapping_add(1);
        }
        Some(old)
    }

    /// Follow link cells to the storage cell Tcl operations observe.
    pub(crate) fn resolve(&self, mut id: VarId) -> Option<VarId> {
        let mut visited = HashSet::new();
        while visited.insert(id) {
            match self.get(id).map(|cell| &cell.state) {
                Some(VarState::Link(target)) => id = *target,
                Some(_) => return Some(id),
                None => return None,
            }
        }
        None
    }

    pub(crate) fn bind(&mut self, id: VarId) {
        let cell = self
            .cells
            .get_mut(&id)
            .expect("binding must refer to a live variable cell");
        cell.bindings = cell
            .bindings
            .checked_add(1)
            .expect("variable binding count overflow");
    }

    pub(crate) fn unbind(&mut self, id: VarId) {
        let cell = self
            .cells
            .get_mut(&id)
            .expect("binding must refer to a live variable cell");
        cell.bindings = cell
            .bindings
            .checked_sub(1)
            .expect("variable binding count underflow");
        self.collect(id);
    }

    /// Discard a cell that was allocated but never inserted into a name table.
    /// This is deliberately distinct from [`Self::unbind`]: no binding count
    /// exists to decrement on the failed-installation path.
    pub(crate) fn discard_unbound(&mut self, id: VarId) {
        let cell = self
            .cells
            .get(&id)
            .expect("discard must refer to a live variable cell");
        assert_eq!(cell.bindings, 0, "bound variable cell cannot be discarded");
        self.collect(id);
    }

    pub(crate) fn pin(&mut self, id: VarId) {
        let cell = self
            .cells
            .get_mut(&id)
            .expect("pin must refer to a live variable cell");
        cell.pins = cell
            .pins
            .checked_add(1)
            .expect("variable pin count overflow");
    }

    pub(crate) fn unpin(&mut self, id: VarId) {
        let cell = self
            .cells
            .get_mut(&id)
            .expect("pin must refer to a live variable cell");
        cell.pins = cell
            .pins
            .checked_sub(1)
            .expect("variable pin count underflow");
        self.collect(id);
    }

    pub(crate) fn retain_operation(&mut self, id: VarId) {
        let cell = self
            .cells
            .get_mut(&id)
            .expect("operation reference must refer to a live variable cell");
        cell.operation_refs = cell
            .operation_refs
            .checked_add(1)
            .expect("variable operation reference count overflow");
    }

    pub(crate) fn release_operation(&mut self, id: VarId) {
        let cell = self
            .cells
            .get_mut(&id)
            .expect("operation reference must refer to a live variable cell");
        cell.operation_refs = cell
            .operation_refs
            .checked_sub(1)
            .expect("variable operation reference count underflow");
        self.collect(id);
    }

    pub(crate) fn has_operation_refs(&self, id: VarId) -> bool {
        self.cells
            .get(&id)
            .is_some_and(|cell| cell.operation_refs != 0)
    }

    pub(crate) fn is_final_operation_ref(&self, id: VarId) -> bool {
        self.cells
            .get(&id)
            .is_some_and(|cell| cell.operation_refs == 1)
    }

    fn collect(&mut self, id: VarId) {
        let collectable = self.cells.get(&id).is_some_and(|cell| {
            cell.bindings == 0 && cell.link_refs == 0 && cell.pins == 0 && cell.operation_refs == 0
        });
        if !collectable {
            return;
        }
        let Some(cell) = self.cells.remove(&id) else {
            return;
        };
        match cell.state {
            VarState::Link(target) => {
                if let Some(target_cell) = self.cells.get_mut(&target) {
                    target_cell.link_refs = target_cell
                        .link_refs
                        .checked_sub(1)
                        .expect("variable link reference count underflow");
                }
                self.collect(target);
            }
            VarState::Array(elements) => {
                for element in elements.into_values() {
                    self.unbind(element);
                }
            }
            VarState::Undefined | VarState::Scalar(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{VarArena, VarState, VarTable};
    use crate::value::Value;

    #[test]
    fn replacing_an_array_releases_its_element_bindings() {
        let mut arena = VarArena::default();
        let child = arena
            .alloc(VarState::Scalar(Value::string("child")))
            .expect("child cell");
        arena.bind(child);
        let mut elements = VarTable::new();
        elements.insert("key".to_owned(), child);
        let parent = arena.alloc(VarState::Array(elements)).expect("parent cell");
        arena.bind(parent);

        assert!(arena.replace_state(parent, VarState::Scalar(Value::string("scalar"))));
        assert!(arena.get(child).is_none());
    }

    #[test]
    fn a_link_keeps_its_unbound_target_alive_then_releases_it() {
        let mut arena = VarArena::default();
        let target = arena.alloc(VarState::Undefined).expect("target cell");
        arena.bind(target);
        let link = arena.alloc(VarState::Link(target)).expect("link cell");
        arena.bind(link);

        arena.unbind(target);
        assert_eq!(arena.resolve(link), Some(target));
        arena.unbind(link);
        assert!(arena.get(link).is_none());
        assert!(arena.get(target).is_none());
    }

    #[test]
    fn link_cycles_and_missing_targets_are_rejected() {
        let mut arena = VarArena::default();
        let first = arena.alloc(VarState::Undefined).expect("first cell");
        arena.bind(first);
        let second = arena.alloc(VarState::Link(first)).expect("second cell");
        arena.bind(second);

        assert!(!arena.replace_state(first, VarState::Link(second)));
        assert!(
            arena
                .alloc(VarState::Link(tcl_core_types::VarId(u32::MAX)))
                .is_none()
        );
        assert_eq!(arena.resolve(second), Some(first));
    }

    #[test]
    fn an_uninstalled_allocation_has_a_distinct_discard_path() {
        let mut arena = VarArena::default();
        let id = arena.alloc(VarState::Undefined).expect("cell");
        arena.discard_unbound(id);
        assert!(arena.get(id).is_none());
    }
}
