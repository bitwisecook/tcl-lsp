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

//! Reached variable ownership releases, independently of execution authority.

use super::{NativeHashEntryKey, VarArena, VarCell, VarId, VarState, VarTable};
use crate::value::Value;
use std::rc::Rc;

/// One existing ownership transition. The driver selects physical storage;
/// these steps neither resolve a name nor acquire a new native reference.
pub(crate) struct VariableRetirement {
    pending: Vec<Step>,
    detached: Option<VarState>,
}

enum Step {
    Collect(VarId),
    CollectStatics,
    ReleaseStatic(VarId),
    Unbind(VarId),
    UnbindReady(VarId),
    Unpin(VarId),
    ReleaseOperation(VarId),
    ReleaseLink(VarId),
    RetireKey(VarId),
    Unset(VarId),
    Detach {
        id: VarId,
        ordinary: bool,
        array_cleanup: bool,
    },
    AdvanceArrayRevision(VarId),
    ClearArraySearches(VarId),
    SetOrdinaryLink(VarId),
    RetireDetached,
    State(VarState),
    Cell(VarCell),
    Release(VariableRelease),
    Elements(std::vec::IntoIter<(tcl_core_types::NameBytes, VarId)>),
}

/// A reached release leaves the arena borrow before a callback can run.
pub(crate) struct VariableRelease(Release);

enum Release {
    State(VarState),
    Cell(Box<VarCell>),
    Table(VarTable),
    Key(Rc<NativeHashEntryKey>),
    Default(Option<[Value; 2]>),
    Searches(Vec<Value>),
}

impl VariableRetirement {
    fn new(step: Step) -> Self {
        Self {
            pending: vec![step],
            detached: None,
        }
    }
    pub(crate) fn collect(id: VarId) -> Self {
        Self::new(Step::Collect(id))
    }
    pub(crate) fn collect_statics() -> Self {
        Self::new(Step::CollectStatics)
    }
    pub(crate) fn unbind(id: VarId) -> Self {
        Self::new(Step::Unbind(id))
    }
    pub(crate) fn unpin(id: VarId) -> Self {
        Self::new(Step::Unpin(id))
    }
    pub(crate) fn release_operation(id: VarId) -> Self {
        Self::new(Step::ReleaseOperation(id))
    }
    pub(crate) fn retire_key(id: VarId) -> Self {
        Self::new(Step::RetireKey(id))
    }
    pub(crate) fn state(state: VarState) -> Self {
        Self::new(Step::State(state))
    }
    pub(crate) fn unset(id: VarId) -> Self {
        Self::new(Step::Unset(id))
    }
    pub(crate) fn searches(id: VarId) -> Self {
        Self::new(Step::ClearArraySearches(id))
    }
    pub(crate) fn detach(id: VarId) -> Self {
        Self::new(Step::Detach {
            id,
            ordinary: false,
            array_cleanup: true,
        })
    }
    pub(crate) fn detach_original_c(id: VarId) -> Self {
        Self::new(Step::Detach {
            id,
            ordinary: false,
            array_cleanup: false,
        })
    }
    pub(crate) fn clear(id: VarId) -> Self {
        Self {
            pending: vec![
                Step::RetireDetached,
                Step::Detach {
                    id,
                    ordinary: true,
                    array_cleanup: true,
                },
            ],
            detached: None,
        }
    }
    pub(crate) fn take_detached(&mut self) -> Option<VarState> {
        self.detached.take()
    }

    fn release_static_reference(&mut self, arena: &mut VarArena, id: VarId) {
        let cell = arena
            .cells
            .get_mut(&id)
            .expect("retained static cell exists");
        cell.static_refs = cell
            .static_refs
            .checked_sub(1)
            .expect("static reference underflow");
        self.pending.push(Step::Collect(id));
    }

    fn release_binding_reference(&mut self, arena: &mut VarArena, id: VarId) {
        let cell = arena
            .cells
            .get_mut(&id)
            .expect("binding must refer to a live variable cell");
        cell.bindings = cell
            .bindings
            .checked_sub(1)
            .expect("variable binding count underflow");
        self.pending.push(Step::Collect(id));
    }

    fn release_pin_reference(&mut self, arena: &mut VarArena, id: VarId) {
        let cell = arena
            .cells
            .get_mut(&id)
            .expect("pin must refer to a live variable cell");
        cell.pins = cell
            .pins
            .checked_sub(1)
            .expect("variable pin count underflow");
        self.pending.push(Step::Collect(id));
    }

    fn release_operation_reference(&mut self, arena: &mut VarArena, id: VarId) {
        let cell = arena
            .cells
            .get_mut(&id)
            .expect("operation reference must refer to a live variable cell");
        cell.operation_refs = cell
            .operation_refs
            .checked_sub(1)
            .expect("variable operation reference count underflow");
        self.pending.push(Step::Collect(id));
    }

    fn detach_cell_state(
        &mut self,
        arena: &mut VarArena,
        id: VarId,
        ordinary: bool,
        array_cleanup: bool,
    ) {
        if let Some(cell) = arena.cells.get_mut(&id) {
            let old = std::mem::replace(&mut cell.state, VarState::Undefined);
            cell.scalar_retirement = None;
            if matches!(old, VarState::Array(_)) && array_cleanup {
                // These are the old metadata owners, extracted before
                // any release can install a new array on this cell.
                let default = cell.array_default.take();
                let searches = cell.array_searches.take_handles();
                self.pending
                    .push(Step::Release(VariableRelease(Release::Searches(searches))));
                self.pending.push(Step::AdvanceArrayRevision(id));
                if ordinary {
                    self.pending.push(Step::SetOrdinaryLink(id));
                }
                self.pending
                    .push(Step::Release(VariableRelease(Release::Default(default))));
            } else if ordinary {
                self.pending.push(Step::SetOrdinaryLink(id));
            }
            self.detached = Some(old);
        }
    }

    fn collect_unreferenced_cell(&mut self, arena: &mut VarArena, id: VarId) {
        let collectable = arena.cells.get(&id).is_some_and(|cell| {
            cell.bindings == 0
                && cell.link_refs == 0
                && cell.pins == 0
                && cell.operation_refs == 0
                && cell.static_refs == 0
        });
        if collectable && let Some(cell) = arena.cells.remove(&id) {
            self.pending.push(Step::Cell(cell));
        }
    }

    fn decompose_retiring_cell(&mut self, mut cell: VarCell) -> Option<VariableRelease> {
        // The old neutral worker did not move ignored scalar/name
        // payloads: whole-cell field order remains the same.
        if matches!(
            &cell.state,
            VarState::Undefined | VarState::Scalar(_) | VarState::NameLink(_)
        ) {
            return Some(VariableRelease(Release::Cell(Box::new(cell))));
        }
        let state = std::mem::replace(&mut cell.state, VarState::Undefined);
        self.pending
            .push(Step::Release(VariableRelease(Release::Cell(Box::new(
                cell,
            )))));
        self.pending.push(Step::State(state));
        None
    }

    /// Advance only pure storage/count steps, returning before an owned release.
    pub(crate) fn next(&mut self, arena: &mut VarArena) -> Option<VariableRelease> {
        while let Some(step) = self.pending.pop() {
            match step {
                Step::Collect(id) => self.collect_unreferenced_cell(arena, id),
                Step::CollectStatics => {
                    let released = std::mem::take(&mut *arena.released_statics.borrow_mut());
                    self.pending
                        .extend(released.into_iter().rev().map(Step::ReleaseStatic));
                }
                Step::ReleaseStatic(id) => self.release_static_reference(arena, id),
                Step::Unbind(id) => {
                    self.pending.push(Step::UnbindReady(id));
                    self.pending.push(Step::CollectStatics);
                }
                Step::UnbindReady(id) => self.release_binding_reference(arena, id),
                Step::Unpin(id) => self.release_pin_reference(arena, id),
                Step::ReleaseOperation(id) => self.release_operation_reference(arena, id),
                Step::ReleaseLink(id) => {
                    if let Some(cell) = arena.cells.get_mut(&id) {
                        cell.link_refs = cell
                            .link_refs
                            .checked_sub(1)
                            .expect("variable link reference count underflow");
                    }
                    arena.note_native_alias_release(id);
                    self.pending.push(Step::Collect(id));
                }
                Step::RetireKey(id) => {
                    if let Some(cell) = arena.cells.get_mut(&id)
                        && let Some(key) = cell.native_element_key.take()
                    {
                        return Some(VariableRelease(Release::Key(key)));
                    }
                }
                Step::Unset(id) => {
                    let parent = arena
                        .element_is_attached(id)
                        .then(|| arena.element_parent(id).map(|(parent, _)| parent))
                        .flatten();
                    if let Some(parent) = parent {
                        self.pending.push(Step::ClearArraySearches(parent));
                    }
                    self.pending.push(Step::Detach {
                        id,
                        ordinary: false,
                        array_cleanup: true,
                    });
                }
                Step::Detach {
                    id,
                    ordinary,
                    array_cleanup,
                } => {
                    self.detach_cell_state(arena, id, ordinary, array_cleanup);
                }
                Step::AdvanceArrayRevision(id) => {
                    if let Some(cell) = arena.cells.get_mut(&id) {
                        cell.array_revision = cell.array_revision.wrapping_add(1);
                    }
                }
                Step::ClearArraySearches(id) => {
                    if let Some(cell) = arena.cells.get_mut(&id) {
                        cell.array_revision = cell.array_revision.wrapping_add(1);
                        return Some(VariableRelease(Release::Searches(
                            cell.array_searches.take_handles(),
                        )));
                    }
                }
                Step::SetOrdinaryLink(id) => {
                    if let Some(cell) = arena.cells.get_mut(&id) {
                        cell.link_origin = tcl_runtime_api::FrameLinkOrigin::Ordinary;
                    }
                }
                Step::RetireDetached => {
                    if let Some(state) = self.detached.take() {
                        self.pending.push(Step::State(state));
                    }
                }
                Step::State(state) => match state {
                    VarState::Link(target) => self.pending.push(Step::ReleaseLink(target)),
                    VarState::Array(table) => return Some(VariableRelease(Release::Table(table))),
                    VarState::Undefined => {}
                    state => return Some(VariableRelease(Release::State(state))),
                },
                Step::Cell(cell) => {
                    if let Some(release) = self.decompose_retiring_cell(cell) {
                        return Some(release);
                    }
                }
                Step::Release(release) => return Some(release),
                Step::Elements(mut elements) => {
                    if let Some((_, id)) = elements.next() {
                        self.pending.push(Step::Elements(elements));
                        self.pending.push(Step::Unbind(id));
                        self.pending.push(Step::RetireKey(id));
                    }
                }
            }
        }
        None
    }
}

impl VariableRelease {
    /// Consume exactly the reached old roles. A driver checks currency before
    /// advancing again; expansion adds only the table's existing binding roles.
    pub(crate) fn retire(self, work: &mut VariableRetirement) {
        match self.0 {
            Release::Table(table) => work.pending.push(Step::Elements(table.into_iter())),
            Release::Key(entry) => {
                if entry.destruction_owners.get() == 0 {
                    let original = entry.original.borrow_mut().take();
                    drop(original);
                }
                drop(entry);
            }
            Release::State(state) => drop(state),
            Release::Cell(cell) => drop(cell),
            Release::Default(default) => drop(default),
            Release::Searches(searches) => drop(searches),
        }
    }
}

impl VarArena {
    /// Neutral storage callers use the identical ownership coordinator. This
    /// wrapper issues no original interpreter or entered-operation authority.
    pub(super) fn drive_retirement(&mut self, mut work: VariableRetirement) -> Option<VarState> {
        while let Some(release) = work.next(self) {
            release.retire(&mut work);
        }
        work.take_detached()
    }
}

/// The existing stored scalar +1, moved out once while its original header
/// remains visible under an allocation-only lease. No reference is acquired.
pub(crate) struct VisibleScalarRetirement {
    pub(crate) old: Value,
    marker: std::rc::Weak<()>,
}
impl VarArena {
    pub(crate) fn take_visible_scalar(&mut self, id: VarId) -> Option<VisibleScalarRetirement> {
        let cell = self.cells.get_mut(&id)?;
        if cell.scalar_retirement.is_some() {
            return None;
        }
        let VarState::Scalar(value) = &cell.state else {
            return None;
        };
        let visible = value.native_lifetime_lease().into_value();
        let VarState::Scalar(old) = std::mem::replace(&mut cell.state, VarState::Scalar(visible))
        else {
            unreachable!("selected scalar role");
        };
        let marker = Rc::new(());
        let weak = Rc::downgrade(&marker);
        cell.scalar_retirement = Some(marker);
        Some(VisibleScalarRetirement { old, marker: weak })
    }
    pub(crate) fn finish_visible_scalar(
        &mut self,
        id: VarId,
        marker: &std::rc::Weak<()>,
    ) -> Option<VarState> {
        let cell = self.cells.get_mut(&id)?;
        if !cell
            .scalar_retirement
            .as_ref()
            .is_some_and(|current| marker.ptr_eq(&Rc::downgrade(current)))
        {
            return None;
        }
        cell.scalar_retirement = None;
        Some(std::mem::replace(&mut cell.state, VarState::Undefined))
    }
}
impl VisibleScalarRetirement {
    pub(crate) fn into_parts(self) -> (Value, std::rc::Weak<()>) {
        (self.old, self.marker)
    }
}

impl VarArena {
    pub(crate) fn take_array_binding_for_cleanup(
        &mut self,
        parent: &super::WeakJimVariableCell,
        table: &std::rc::Weak<()>,
        key: &[u8],
        expected: VarId,
    ) -> Option<super::RemovedVariableBinding> {
        let parent = self.validate_jim_cell(parent)?;
        let cell = self.cells.get_mut(&parent)?;
        let VarState::Array(elements) = &mut cell.state else {
            return None;
        };
        if !elements.has_original_birth(table) || elements.get(key) != Some(&expected) {
            return None;
        }
        elements.take_binding(key)
    }
}

impl VarArena {
    pub(crate) fn original_unset_search_owner(&self, id: VarId) -> Option<VarId> {
        if self.element_is_attached(id) {
            self.element_parent(id).map(|(parent, _)| parent)
        } else {
            matches!(self.get(id).map(VarCell::state), Some(VarState::Array(_))).then_some(id)
        }
    }
    pub(crate) fn take_array_default_for_cleanup(&mut self, id: VarId) -> Option<[Value; 2]> {
        self.cells.get_mut(&id)?.array_default.take()
    }
}

#[cfg(test)]
impl VarArena {
    pub(crate) fn cleanup_pin_count(&self, id: VarId) -> Option<u64> {
        self.cells.get(&id).map(|cell| cell.pins)
    }
}
