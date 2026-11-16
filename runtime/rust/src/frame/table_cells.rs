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

//! Cell and slot lookup in a `VarTable`: name-to-slot resolution, compiled cells, cached trace flags and binding identities.

use super::materialisation::PreparedAliasReplacement;
#[cfg(test)]
use super::VarError;
use super::{
    fresh_var_id, Cell, CellContents, Link, NativeCompiledAliasError, RetainedVariableCell,
    StaticVariables, Var, VarHome, VarTable,
};
use crate::obj::TclObj;
use std::cell::{Ref, RefCell};
use std::rc::Rc;
use tcl_runtime_api::{FrameLinkOrigin, VarId};

impl VarTable {
    pub(crate) fn cached_increment_scalar_identity(
        &self,
        name: &[u8],
        expected: *mut TclObj,
    ) -> Option<VarId> {
        let slot = self.slots.get(name)?;
        let cell = self.cells.get(*slot)?;
        let contents = cell.contents.try_borrow().ok()?;
        if contents.check_owned_value_role().is_err()
            || contents.operation_refs != 0
            || contents.native_alias_refs != 0
            || contents.rmw_refs != 0
            || contents.constant
            || !matches!(contents.var.as_ref(), Some(Var::Scalar(pointer)) if *pointer == expected)
        {
            return None;
        }
        contents.binding_id
    }

    /// Passive WORK diagnostic: inspect only an existing physical scalar slot.
    /// No semantic lookup, links, conversion, trace, provider or ownership change.
    pub(crate) fn inspect_existing_scalar<T>(
        &self,
        name: &[u8],
        inspect: impl FnOnce(*mut TclObj) -> T,
    ) -> Option<T> {
        let slot = *self.slots.get(name)?;
        let contents = self.cells.get(slot)?.contents.try_borrow().ok()?;
        contents.check_owned_value_role().ok()?;
        let Var::Scalar(pointer) = contents.var.as_ref()? else {
            return None;
        };
        Some(inspect(*pointer))
    }

    pub(crate) fn prepare_original_alias(
        &mut self,
        name: &[u8],
        link: &Link,
        traced: bool,
    ) -> Result<PreparedAliasReplacement, NativeCompiledAliasError> {
        let contents = self
            .contents(name)
            .ok_or(NativeCompiledAliasError::Unavailable)?;
        PreparedAliasReplacement::capture(contents, link, traced, FrameLinkOrigin::Ordinary)
    }

    pub(crate) fn set_hash_recipe(&self, recipe: Option<tcl_core_types::NativeHashRecipe>) {
        if self.hash_recipe.replace(recipe) == recipe {
            return;
        }
        self.entry_order.borrow_mut().select_recipe(recipe);
        for cell in &self.cells {
            cell.contents
                .borrow_mut()
                .member_order
                .select_recipe(recipe);
        }
    }

    pub(super) fn ordered_names(&self) -> Vec<&[u8]> {
        match self.entry_order.borrow().keys() {
            Some(keys) => keys
                .into_iter()
                .filter_map(|key| self.slots.get_key_value(key).map(|(key, _)| key.as_slice()))
                .collect(),
            None => self
                .entry_order
                .borrow()
                .physical_keys()
                .into_iter()
                .filter_map(|key| self.slots.get_key_value(key).map(|(key, _)| key.as_slice()))
                .collect(),
        }
    }

    pub(super) fn lookup_slot(&self, name: &[u8]) -> Option<usize> {
        if let Some(protocol) = self.compiled_protocol {
            if let Some(slot) = self
                .declared_slots
                .iter()
                .find(|&&slot| protocol.dynamic_local_names_equal(&self.cells[slot].name, name))
            {
                return Some(*slot);
            }
        }
        self.slots
            .get(name)
            .or_else(|| self.static_slots.get(name))
            .copied()
    }

    pub(crate) fn declare_compiled_cell(&mut self, name: &[u8]) -> usize {
        let slot = self.cells.len();
        let cell = Cell::empty(name);
        cell.contents.borrow_mut().compiled_declaration = true;
        cell.contents
            .borrow_mut()
            .member_order
            .select_recipe(self.hash_recipe.get());
        self.cells.push(cell);
        self.declared_slots.push(slot);
        slot
    }

    /// Allocate the actual value cell for one unnamed native compiler slot.
    /// It has no table key, name-lookup entry, or introspection declaration.
    pub(crate) fn declare_compiled_temporary(&mut self) -> usize {
        let slot = self.cells.len();
        let mut cell = Cell::empty(b"");
        cell.anonymous = true;
        cell.contents.borrow_mut().compiled_declaration = true;
        self.cells.push(cell);
        slot
    }

    pub(crate) fn mark_namespace_declared(&mut self, name: &[u8]) {
        if let Some(contents) = self.contents(name) {
            contents.borrow_mut().namespace_declared = true;
        }
    }

    /// The resolved producer retains its actual home, independently of name text.
    pub(crate) fn mark_undefined_root(&mut self, name: &[u8], home: VarHome) -> bool {
        let Some(contents) = self.contents(name) else {
            return false;
        };
        let mut cell = contents.borrow_mut();
        if matches!(cell.var.as_ref(), Some(Var::Link(link))
            if link.home == home && link.name == name && link.elem.is_none())
        {
            cell.undefined_root_home = Some(home);
            return true;
        }
        false
    }

    pub(crate) fn mark_trace_shell(&mut self, name: &[u8], home: VarHome) {
        if self.mark_undefined_root(name, home) {
            self.contents(name).unwrap().borrow_mut().undefined_shell = true;
        }
    }

    #[cfg(test)]
    pub(crate) fn ensure_element_shell(&mut self, name: &[u8], key: &[u8]) -> Result<(), VarError> {
        self.ensure_array(name)?;
        let Some(contents) = self.contents(name) else {
            return Err(VarError::IsScalar);
        };
        let mut cell = contents.borrow_mut();
        cell.insert_member_entry(key);
        cell.rmw_member_shells.remove(key);
        cell.element_ids
            .entry(key.to_vec())
            .or_insert_with(fresh_var_id);
        Ok(())
    }

    pub(super) fn visible_cell(&self, slot: usize, include_links: bool) -> bool {
        let cell = self.cells[slot].contents.borrow();
        match cell.var.as_ref() {
            Some(Var::Scalar(_) | Var::PendingScalar(_) | Var::Array(_)) => true,
            Some(Var::Link(_)) => {
                include_links && (!cell.undefined_shell || cell.namespace_declared)
            }
            None => include_links && cell.namespace_declared,
        }
    }

    /// The slot `name` occupies, reserving an empty one if it has none yet.
    ///
    /// This is the compiled-local binding operation: after it, the slot and the
    /// name are two ways of addressing one cell, for the table's lifetime.
    pub(crate) fn slot_for(&mut self, name: &[u8]) -> usize {
        if let Some(slot) = self.lookup_slot(name) {
            return slot;
        }
        if let Some(contents) = self
            .statics
            .as_ref()
            .and_then(|s| s.cells.get(name))
            .cloned()
        {
            let slot = self.cells.len();
            self.cells.push(Cell {
                name: name.to_vec(),
                anonymous: false,
                contents,
            });
            self.static_slots.insert(name.to_vec(), slot);
            return slot;
        }
        self.local_slot_for(name)
    }

    pub(super) fn local_slot_for(&mut self, name: &[u8]) -> usize {
        if let Some(slot) = self.lookup_slot(name) {
            return slot;
        }
        self.counted_dynamic_slot(name)
    }

    pub(super) fn counted_dynamic_slot(&mut self, name: &[u8]) -> usize {
        if let Some(slot) = self.slots.get(name) {
            return *slot;
        }
        let slot = self.cells.len();
        let cell = Cell::empty(name);
        cell.contents
            .borrow_mut()
            .member_order
            .select_recipe(self.hash_recipe.get());
        self.cells.push(cell);
        self.slots.insert(name.to_vec(), slot);
        slot
    }

    pub(super) fn contents(&self, name: &[u8]) -> Option<&Rc<RefCell<CellContents>>> {
        if let Some(slot) = self.lookup_slot(name) {
            return Some(&self.cells[slot].contents);
        }
        self.statics.as_ref()?.cells.get(name)
    }

    pub(crate) fn capture_cell(&self, name: &[u8]) -> Option<RetainedVariableCell> {
        let contents = self.contents(name)?;
        {
            let cell = contents.borrow();
            cell.check_owned_value_role().ok()?;
            cell.var.as_ref()?;
        }
        Some(RetainedVariableCell(contents.clone()))
    }

    pub(crate) fn install_statics(&mut self, statics: Rc<StaticVariables>) {
        self.statics = Some(statics);
    }

    /// Borrow one cell for a synchronous read. Callers release the guard before
    /// dispatching callbacks, so native reentry never retains a cell borrow.
    pub(crate) fn cell_at(&self, slot: usize) -> Option<Ref<'_, Var>> {
        Ref::filter_map(self.cells.get(slot)?.contents.borrow(), |c| {
            c.check_owned_value_role().ok()?;
            c.var.as_ref()
        })
        .ok()
    }

    pub(crate) fn slot_name(&self, slot: usize) -> Option<&[u8]> {
        self.cells
            .get(slot)
            .filter(|cell| !cell.anonymous)
            .map(|cell| cell.name.as_slice())
    }

    pub(crate) fn cached_trace_flag(&self, slot: usize, epoch: u64) -> Option<bool> {
        let (cached_epoch, traced) = self.cells.get(slot)?.contents.borrow().traced.get();
        (cached_epoch == epoch).then_some(traced)
    }

    pub(crate) fn set_cached_trace_flag(&self, slot: usize, epoch: u64, traced: bool) {
        if let Some(cell) = self.cells.get(slot) {
            cell.contents.borrow().traced.set((epoch, traced));
        }
    }

    pub(super) fn get(&self, name: &[u8]) -> Option<Ref<'_, Var>> {
        Ref::filter_map(self.contents(name)?.borrow(), |c| {
            c.jim_scalar_retirement.is_none().then_some(())?;
            c.var.as_ref()
        })
        .ok()
    }

    pub(super) fn put(&mut self, name: &[u8], var: Var) -> Option<Var> {
        let slot = self.slot_for(name);
        if !self.declared_slots.contains(&slot) {
            self.entry_order.borrow_mut().insert(name);
        }
        self.put_at(slot, var)
    }

    pub(super) fn put_at(&mut self, slot: usize, var: Var) -> Option<Var> {
        let mut cell = self.cells[slot].contents.borrow_mut();
        cell.undefined_shell = false;
        cell.undefined_root_home = None;
        cell.rmw_shell_entry = None;
        cell.link_origin = FrameLinkOrigin::Ordinary;
        if cell.binding_id.is_none() {
            cell.binding_id = Some(fresh_var_id());
        }
        cell.element_ids.clear();
        cell.native_member_keys.clear();
        cell.rmw_member_shells.clear();
        cell.element_operation_refs.clear();
        cell.array_search_epoch = cell.array_search_epoch.wrapping_add(1);
        cell.array_searches.clear();
        cell.member_order = tcl_core_types::NativeEntryLedger::default();
        cell.member_order.select_recipe(self.hash_recipe.get());
        if let Var::Array(elements) = &var {
            for key in elements.keys() {
                cell.insert_member_entry(key);
            }
            cell.element_ids
                .extend(elements.keys().cloned().map(|key| (key, fresh_var_id())));
        }
        cell.replace_value(var)
    }

    /// Identity of the variable currently bound at `name`.
    pub(crate) fn binding_id(&self, name: &[u8]) -> Option<VarId> {
        let cell = self.contents(name)?.borrow();
        cell.var.as_ref()?;
        cell.binding_id
    }

    /// Actual allocated binding for trace routing, including undefined roots.
    /// Definition and value-existence queries keep their separate predicates.
    pub(crate) fn trace_binding_identity(&self, name: &[u8]) -> Option<VarId> {
        self.contents(name)?.borrow().binding_id
    }

    /// Existing member allocation of the original root, including undefined shells.
    /// The root's retained identity selects the cell, rather than its display name.
    pub(crate) fn native_trace_element_identity(&self, root: VarId, key: &[u8]) -> Option<VarId> {
        self.cells.iter().find_map(|cell| {
            let contents = cell.contents.borrow();
            (contents.binding_id == Some(root))
                .then(|| contents.element_ids.get(key).copied())
                .flatten()
        })
    }

    /// Identity of a defined element of the current array binding.
    #[cfg(test)]
    pub(crate) fn element_id(&self, name: &[u8], key: &[u8]) -> Option<VarId> {
        let cell = self.contents(name)?.borrow();
        matches!(cell.var.as_ref(), Some(Var::Array(map)) if map.contains_key(key))
            .then(|| cell.element_ids.get(key).copied())
            .flatten()
    }

    /// A real namespace hash cell, including a retained undefined shell.
    pub(crate) fn has_native_namespace_cell(&self, name: &[u8]) -> bool {
        self.lookup_slot(name)
            .is_some_and(|slot| self.cells[slot].contents.borrow().binding_id.is_some())
    }

    pub(crate) fn cell(&self, name: &[u8]) -> Option<Ref<'_, Var>> {
        self.get(name)
    }
}
