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

//! What a `VarTable` stores and publishes: constants, array defaults, scalars, elements, removals and links.

use super::retired_variable::RetiredVariableParts;
use super::{
    fresh_var_id, rmw, Cell, Link, RetiredArrayDefault, RetiredVariableContents, Var, VarError,
    VarTable,
};
use crate::obj;
use crate::obj::TclObj;
use std::collections::BTreeMap;
use std::rc::Rc;
use tcl_runtime_api::FrameLinkOrigin;

impl VarTable {
    pub(crate) fn is_constant(&self, name: &[u8]) -> bool {
        self.contents(name)
            .is_some_and(|cell| cell.borrow().constant)
    }

    pub(crate) fn const_names(&self) -> Vec<&[u8]> {
        self.slots
            .iter()
            .filter(|(_, slot)| self.cells[**slot].contents.borrow().constant)
            .map(|(name, _)| name.as_slice())
            .collect()
    }

    /// Set array `name`'s default value (`array default set`), releasing any
    /// prior default. The table pins the value with **+2**: one for ownership,
    /// and one so a read of the default always sees it as *shared* — a
    /// read-modify-write (`lappend`/`append`/`dict`) then copies rather than
    /// mutating the stored default in place (TIP 508 copy-on-write).
    #[cfg(test)]
    pub(crate) fn set_array_default(&mut self, name: &[u8], obj: *mut TclObj) {
        let retired = self.replace_array_default(name, obj);
        drop(retired);
    }

    /// Same selected storage step, with retirement returned to the entered
    /// caller so no enclosing table/namespace borrow crosses a free callback.
    pub(crate) fn replace_array_default(
        &mut self,
        name: &[u8],
        original: *mut TclObj,
    ) -> Option<RetiredArrayDefault> {
        // The new table default owns +2 before the old default is retired.
        unsafe {
            obj::incr_ref_count(original);
            obj::incr_ref_count(original);
        }
        let slot = self.slot_for(name);
        let old = self.cells[slot]
            .contents
            .borrow_mut()
            .array_default
            .replace(original);
        old.map(RetiredArrayDefault::capture)
    }

    /// Array `name`'s default value, if one is set (`array default get`, and the
    /// fallback for a read of a missing element). Borrowed; the table keeps its +1.
    #[cfg(test)]
    pub(crate) fn array_default(&self, name: &[u8]) -> Option<*mut TclObj> {
        self.contents(name)?.borrow().array_default
    }

    /// Remove array `name`'s default value (`array default unset`).
    #[cfg(test)]
    pub(crate) fn unset_array_default(&mut self, name: &[u8]) {
        let retired = self.take_array_default(name);
        drop(retired);
    }

    pub(crate) fn take_array_default(&mut self, name: &[u8]) -> Option<RetiredArrayDefault> {
        let contents = self.contents(name)?;
        let old = contents.borrow_mut().array_default.take();
        old.map(RetiredArrayDefault::capture)
    }

    /// Flag the scalar `name` `const` (the `const` command, after its value is
    /// stored). A no-op if already flagged.
    pub(crate) fn mark_constant(&mut self, name: &[u8]) {
        let slot = self.slot_for(name);
        self.cells[slot].contents.borrow_mut().constant = true;
    }

    /// `set name value` into this table directly. The cell takes a **+1**.
    pub(crate) fn store_scalar(&mut self, name: &[u8], obj: *mut TclObj) -> Result<(), VarError> {
        let previous = self.publish_scalar(name, obj)?;
        drop(previous);
        Ok(())
    }

    /// The same scalar store, returning its displaced owning contents.
    pub(crate) fn publish_scalar(
        &mut self,
        name: &[u8],
        obj: *mut TclObj,
    ) -> Result<RetiredVariableContents, VarError> {
        if self.is_constant(name) {
            return Err(VarError::IsConstant);
        }
        if self.dictionary_variables() {
            return self
                .publish_dictionary_root(name, obj)
                .map(|previous| RetiredVariableContents::Dictionary(Some(previous)));
        }
        let slot = self.slot_for(name);
        let contents = self.cells[slot].contents.clone();
        let mut cell = contents.borrow_mut();
        match cell.var.as_mut() {
            Some(Var::Array(_)) => Err(VarError::IsArray),
            Some(Var::Scalar(_)) => {
                unsafe { obj::incr_ref_count(obj) };
                let previous = cell.replace_value(Var::Scalar(obj));
                drop(cell);
                Ok(RetiredVariableContents::Value(previous))
            }
            Some(Var::PendingScalar(_)) => {
                unsafe { obj::incr_ref_count(obj) };
                let previous = cell.replace_value(Var::Scalar(obj));
                drop(cell);
                Ok(RetiredVariableContents::Value(previous))
            }
            Some(Var::Link(link)) if link.name == name && link.elem.is_none() => {
                drop(cell);
                unsafe { obj::incr_ref_count(obj) };
                Ok(RetiredVariableContents::Value(
                    self.put(name, Var::Scalar(obj)),
                ))
            }
            Some(Var::Link(_)) => unreachable!("the coordinator never lands on a link"),
            None => {
                drop(cell);
                unsafe { obj::incr_ref_count(obj) };
                Ok(RetiredVariableContents::Value(
                    self.put(name, Var::Scalar(obj)),
                ))
            }
        }
    }

    /// Header-only software observation of actual published scalar storage.
    /// This cannot issue a scalar reference or allow a checked semantic read.
    #[cfg(test)]
    pub(crate) fn published_scalar_header(&self, name: &[u8]) -> Option<*mut TclObj> {
        let contents = self.contents(name)?.borrow();
        match contents.var.as_ref() {
            Some(Var::Scalar(original)) => Some(*original),
            Some(Var::PendingScalar(pending)) => Some(pending.raw_header()),
            _ => None,
        }
    }

    /// `set name` — the scalar's value (borrowed; the table keeps its +1).
    pub(crate) fn load_scalar(&self, name: &[u8]) -> Option<*mut TclObj> {
        match self.get(name).as_deref() {
            Some(Var::Scalar(p)) => Some(*p),
            _ => None,
        }
    }

    /// `set name(key) value`. Errors if `name` is a scalar.
    pub(crate) fn store_elem(
        &mut self,
        name: &[u8],
        key: &[u8],
        obj: *mut TclObj,
    ) -> Result<(), VarError> {
        let previous = self.publish_elem(name, key, obj)?;
        drop(previous);
        Ok(())
    }

    /// The same member store, returning its displaced owning contents.
    pub(crate) fn publish_elem(
        &mut self,
        name: &[u8],
        key: &[u8],
        obj: *mut TclObj,
    ) -> Result<RetiredVariableContents, VarError> {
        if self.dictionary_variables() {
            return self
                .publish_dictionary_element(name, key, obj)
                .map(|previous| RetiredVariableContents::Dictionary(Some(previous)));
        }
        if let Some(contents) = self.contents(name).cloned() {
            let mut cell = contents.borrow_mut();
            match cell.var.as_mut() {
                Some(Var::Scalar(_) | Var::PendingScalar(_)) => return Err(VarError::IsScalar),
                Some(Var::Array(map)) => {
                    unsafe { obj::incr_ref_count(obj) };
                    let fresh = !map.contains_key(key);
                    let previous = map.insert(key.to_vec(), obj);
                    cell.insert_member_entry(key);
                    cell.rmw_member_shells.remove(key);
                    if fresh && !cell.element_ids.contains_key(key) {
                        cell.element_ids.insert(key.to_vec(), fresh_var_id());
                    }
                    drop(cell);
                    return Ok(RetiredVariableContents::Value(previous.map(Var::Scalar)));
                }
                Some(Var::Link(link)) if link.name == name && link.elem.is_none() => {}
                Some(Var::Link(_)) => unreachable!("the coordinator never lands on a link"),
                None => {}
            }
        }
        let mut map = BTreeMap::new();
        unsafe { obj::incr_ref_count(obj) };
        map.insert(key.to_vec(), obj);
        Ok(RetiredVariableContents::Value(
            self.put(name, Var::Array(map)),
        ))
    }

    /// `set name(key)` — borrowed.
    pub(crate) fn load_elem(&self, name: &[u8], key: &[u8]) -> Option<*mut TclObj> {
        if self.dictionary_variables() {
            let root = self.load_scalar(name)?;
            self.prepare_dictionary_root(root).ok()?;
            return crate::dict::dict_get(root, key).ok().flatten();
        }
        match self.get(name).as_deref() {
            Some(Var::Array(map)) => map.get(key).copied(),
            _ => None,
        }
    }

    /// Remove the whole variable `name` (scalar or array); returns whether it
    /// existed. Releases every object it owned.
    #[cfg(test)]
    pub(crate) fn remove(&mut self, name: &[u8]) -> bool {
        drop(self.take_array_default(name));
        let (removed, old) = self.publish_remove_after_default(name);
        drop(old);
        removed
    }

    /// The default is a separate earlier retirement stage. The entered caller
    /// releases it and checks the original receiver before reaching this stage.
    pub(crate) fn publish_remove_after_default(
        &mut self,
        name: &[u8],
    ) -> (bool, RetiredVariableParts) {
        let mut old = RetiredVariableParts::default();
        let Some(slot) = self.lookup_slot(name) else {
            return (false, old);
        };
        {
            let mut cell = self.cells[slot].contents.borrow_mut();
            cell.array_search_epoch = cell.array_search_epoch.wrapping_add(1);
            cell.array_searches.clear();
        }
        old.hash_key(self.take_jim_key(name));
        {
            let mut cell = self.cells[slot].contents.borrow_mut();
            cell.namespace_declared = false;
            cell.undefined_shell = false;
        }
        let retire_entry = {
            let cell = self.cells[slot].contents.borrow();
            cell.operation_refs == 0 && cell.rmw_root_refs == 0 && cell.native_alias_refs == 0
        };
        if retire_entry {
            old.hash_key(self.take_native_entry(name));
        }
        if self.cells[slot].contents.borrow().rmw_refs != 0
            || self.cells[slot].contents.borrow().native_alias_refs != 0
        {
            let mut contents = self.cells[slot].contents.borrow_mut();
            let is_array = matches!(contents.var.as_ref(), Some(Var::Array(_)));
            if (contents.rmw_root_refs != 0 || contents.native_alias_refs != 0)
                && contents.rmw_shell_entry.is_none()
                && !self.declared_slots.contains(&slot)
            {
                if let Some(binding) = contents.binding_id {
                    contents.rmw_shell_entry = Some(rmw::RootShellEntry::new(
                        &self.entry_order,
                        &self.native_keys,
                        name,
                        binding,
                    ));
                }
            }
            let value = contents.take_value();
            let removed = value.is_some();
            old.elements(std::mem::take(&mut contents.native_member_keys));
            old.value(value);
            let replace = if is_array {
                contents.rmw_array_epoch = contents.rmw_array_epoch.wrapping_add(1);
                if contents.rmw_root_refs == 0 && contents.native_alias_refs == 0 {
                    contents.rmw_retirement = Some(VarError::DeletedArray);
                    true
                } else {
                    false
                }
            } else {
                false
            };
            drop(contents);
            if replace {
                old.cell(std::mem::replace(&mut self.cells[slot], Cell::empty(name)));
            }
            return (removed, old);
        }
        if self.cells[slot].contents.borrow().operation_refs != 0 {
            let mut cell = self.cells[slot].contents.borrow_mut();
            cell.element_ids.clear();
            old.elements(std::mem::take(&mut cell.native_member_keys));
            cell.element_operation_refs.clear();
            let value = cell.take_value();
            let removed = value.is_some();
            old.value(value);
            return (removed, old);
        }
        if Rc::strong_count(&self.cells[slot].contents) > 1 {
            let removed = self.cells[slot].contents.borrow().var.is_some();
            // Captured static/cell owners keep their existing value role. Moving
            // the actual table Rc does not manufacture another native reference.
            old.cell(std::mem::replace(&mut self.cells[slot], Cell::empty(name)));
            return (removed, old);
        }
        let mut cell = self.cells[slot].contents.borrow_mut();
        if !cell.compiled_declaration {
            cell.binding_id = None;
        }
        cell.element_ids.clear();
        old.elements(std::mem::take(&mut cell.native_member_keys));
        cell.rmw_member_shells.clear();
        cell.element_operation_refs.clear();
        let value = cell.take_value();
        let removed = value.is_some();
        old.value(value);
        drop(cell);
        (removed, old)
    }

    /// Ensure `name` is an (at least empty) array — `array default set` creates
    /// the array even with no elements. Returns `Err(IsScalar)` if `name` is a
    /// scalar. (`IsScalar` reuses the closest existing variant; the caller maps
    /// it to the `array default set` message.)
    pub(crate) fn ensure_array(&mut self, name: &[u8]) -> Result<(), VarError> {
        if self.dictionary_variables() {
            return self.ensure_dictionary_root(name);
        }
        let contents = self.contents(name).cloned();
        if let Some(contents) = contents {
            let cell = contents.borrow();
            if !cell.array_materialisation_needed(Some(name))? {
                return Ok(());
            }
        }
        self.put(name, Var::Array(BTreeMap::new()));
        Ok(())
    }

    /// Remove one array element `name(key)`; returns whether it existed.
    #[cfg(test)]
    pub(crate) fn remove_elem(&mut self, name: &[u8], key: &[u8]) -> bool {
        if self.dictionary_variables() {
            return self.remove_dictionary_element(name, key);
        }
        let (removed, old) = self.publish_remove_elem(name, key);
        drop(old);
        removed
    }

    pub(crate) fn publish_remove_elem(
        &mut self,
        name: &[u8],
        key: &[u8],
    ) -> (bool, RetiredVariableParts) {
        let mut old = RetiredVariableParts::default();
        let Some(contents) = self.contents(name) else {
            return (false, old);
        };
        let mut cell = contents.borrow_mut();
        if cell.member_order.contains_key(key) {
            cell.array_search_epoch = cell.array_search_epoch.wrapping_add(1);
            cell.array_searches.clear();
        }
        let value = match cell.var.as_mut() {
            Some(Var::Array(map)) => map.remove(key),
            _ => None,
        };
        if value.is_some()
            && !cell.element_operation_refs.contains_key(key)
            && !cell.native_member_keys.has_aliases(key)
        {
            cell.element_ids.remove(key);
            cell.member_order.remove(key);
            old.key(cell.native_member_keys.take_key(key));
        }
        let removed = value.is_some();
        old.value(value.map(Var::Scalar));
        drop(cell);
        (removed, old)
    }

    /// Install `link` under `name`, releasing any cell it replaces.
    pub(crate) fn insert_link(&mut self, name: &[u8], link: Link) {
        self.insert_link_with_origin(name, link, FrameLinkOrigin::Ordinary);
    }

    /// Install a link with its typed frame-binding origin.
    pub(crate) fn insert_link_with_origin(
        &mut self,
        name: &[u8],
        link: Link,
        origin: FrameLinkOrigin,
    ) {
        drop(self.publish_link_with_origin(name, link, origin));
    }

    pub(crate) fn publish_link_with_origin(
        &mut self,
        name: &[u8],
        link: Link,
        origin: FrameLinkOrigin,
    ) -> RetiredVariableParts {
        let slot = self.local_slot_for(name);
        if !self.declared_slots.contains(&slot) {
            self.entry_order.borrow_mut().insert(name);
        }
        let mut cell = self.cells[slot].contents.borrow_mut();
        cell.undefined_shell = false;
        cell.undefined_root_home = None;
        cell.link_origin = origin;
        cell.binding_id.get_or_insert_with(fresh_var_id);
        cell.element_ids.clear();
        let mut old = RetiredVariableParts::default();
        old.elements(std::mem::take(&mut cell.native_member_keys));
        cell.rmw_member_shells.clear();
        cell.element_operation_refs.clear();
        let adopted = link.clone(); // Metadata observes the transferred stored role.
        old.value_preserving_link(cell.replace_value(Var::Link(link)), &adopted);
        drop(cell);
        old
    }
}
