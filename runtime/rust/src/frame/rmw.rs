// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Physical variable receivers retained across native read/write callbacks.

use super::{CellContents, Var, VarError, VarTable};
use crate::obj::{self, TclObj};
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};
use tcl_runtime_api::VarId;

/// An entry created by an operation in its actual original table. The weak
/// ledger cannot retain a dropped table or re-resolve a replacement by name.
pub(super) struct RootShellEntry {
    ledger: Weak<RefCell<tcl_core_types::NativeEntryLedger>>,
    keys: Weak<
        RefCell<std::collections::BTreeMap<Vec<u8>, super::native_variable_names::NativeHashKey>>,
    >,
    name: Vec<u8>,
    binding: VarId,
}

impl RootShellEntry {
    pub(super) fn new(
        ledger: &Rc<RefCell<tcl_core_types::NativeEntryLedger>>,
        keys: &super::native_variable_names::NativeHashKeys,
        name: &[u8],
        binding: VarId,
    ) -> Self {
        Self {
            ledger: Rc::downgrade(ledger),
            keys: Rc::downgrade(keys),
            name: name.to_vec(),
            binding,
        }
    }

    pub(super) fn retire(self, binding: Option<VarId>) {
        if binding == Some(self.binding) {
            if let Some(ledger) = self.ledger.upgrade() {
                ledger.borrow_mut().remove(&self.name);
            }
            if let Some(keys) = self.keys.upgrade() {
                let retired = keys.borrow_mut().remove(&self.name);
                drop(retired);
            }
        }
    }
}

/// One physical receiver. Its cell borrow never survives a guest callback.
pub(crate) struct VariableReceiver {
    contents: Rc<RefCell<CellContents>>,
    element: Option<Vec<u8>>,
    array_epoch: u64,
    retained_array: Option<super::RetainedArrayCell>,
}

impl VariableReceiver {
    pub(crate) fn binding_id(&self) -> Option<VarId> {
        self.contents.borrow().binding_id
    }
    pub(crate) fn is_constant(&self) -> bool {
        self.contents.borrow().constant
    }
    pub(crate) fn is_array(&self) -> bool {
        matches!(self.contents.borrow().var.as_ref(), Some(Var::Array(_)))
    }
    pub(crate) fn selected_array(&self) -> Option<super::RetainedArrayCell> {
        if let Some(array) = &self.retained_array {
            return Some(array.clone());
        }
        let cell = self.contents.borrow();
        if !matches!(cell.var.as_ref(), Some(Var::Array(_))) {
            return None;
        }
        Some(super::RetainedArrayCell {
            contents: Rc::clone(&self.contents),
            id: cell.binding_id?,
            epoch: cell.rmw_array_epoch,
        })
    }
    pub(crate) fn begin_array_destruction(&self) -> Option<super::RetainedArrayCell> {
        if self.element.is_some() {
            return None;
        }
        let mut contents = self.contents.borrow_mut();
        let id = contents.binding_id?;
        let epoch = contents.rmw_array_epoch;
        if !matches!(contents.var.as_ref(), Some(Var::Array(_))) {
            return None;
        }
        let Some(Var::Array(values)) = contents.var.take() else {
            unreachable!()
        };
        let entry_order = std::mem::take(&mut contents.member_order);
        contents.member_order.select_recipe(entry_order.recipe());
        let native_keys = std::mem::take(&mut contents.native_member_keys);
        contents.detached_arrays.insert(
            id,
            super::destruction::DetachedArray {
                values,
                retired: Default::default(),
                entry_order,
                native_keys,
            },
        );
        contents.rmw_array_epoch = contents.rmw_array_epoch.wrapping_add(1);
        contents.array_search_epoch = contents.array_search_epoch.wrapping_add(1);
        contents.array_searches.clear();
        contents.element_ids.clear();
        let default = contents.array_default.take();
        drop(contents);
        if let Some(default) = default {
            // SAFETY: the selected array owns the native default's two holds.
            unsafe {
                obj::decr_ref_count(default);
                obj::decr_ref_count(default);
            }
        }
        Some(super::RetainedArrayCell {
            contents: Rc::clone(&self.contents),
            id,
            epoch,
        })
    }
    pub(crate) fn retain_original_element_key(&self, original: Option<*mut TclObj>) {
        if let Some(key) = &self.element {
            let mut contents = self.contents.borrow_mut();
            contents.insert_member_entry(key);
            let id = contents.element_ids[key.as_slice()];
            contents.native_member_keys.ensure(id, key, original);
        }
    }
    /// Remove only this selected cell/member; callbacks belong to the caller.
    pub(crate) fn unset(&self) -> Result<bool, VarError> {
        if let Some(error) = self.contents.borrow().rmw_retirement {
            return Err(error);
        }
        if let (Some(array), Some(key)) = (&self.retained_array, &self.element) {
            return Ok(array.remove(key));
        }
        let mut default = None;
        let old = {
            let mut contents = self.contents.borrow_mut();
            if contents.constant {
                return Err(VarError::IsConstant);
            }
            contents.array_search_epoch = contents.array_search_epoch.wrapping_add(1);
            contents.array_searches.clear();
            if let Some(key) = &self.element {
                match contents.var.as_mut() {
                    Some(Var::Array(values)) => {
                        let old = values.remove(key).map(Var::Scalar);
                        if old.is_some() {
                            contents.rmw_member_shells.insert(key.clone());
                        }
                        old
                    }
                    Some(Var::Scalar(_)) => return Err(VarError::IsScalar),
                    _ => None,
                }
            } else {
                contents.namespace_declared = false;
                contents.undefined_shell = false;
                contents.rmw_array_epoch = contents.rmw_array_epoch.wrapping_add(1);
                default = contents.array_default.take();
                contents.native_member_keys.clear();
                contents.element_ids.clear();
                let recipe = contents.member_order.recipe();
                contents.member_order = Default::default();
                contents.member_order.select_recipe(recipe);
                contents.rmw_member_shells.clear();
                contents.var.take()
            }
        };
        if let Some(default) = default {
            // SAFETY: root destruction releases the array's two real default holds.
            unsafe {
                obj::decr_ref_count(default);
                obj::decr_ref_count(default);
            }
        }
        let existed = matches!(old.as_ref(), Some(Var::Scalar(_) | Var::Array(_)));
        if let Some(old) = old {
            old.release();
        }
        Ok(existed)
    }
    pub(super) fn capture_contents(
        contents: Rc<RefCell<CellContents>>,
        element: Option<Vec<u8>>,
    ) -> Self {
        let array_epoch = {
            let mut cell = contents.borrow_mut();
            cell.rmw_refs += 1;
            cell.binding_id.get_or_insert_with(super::fresh_var_id);
            if let Some(key) = &element {
                if !cell.element_ids.contains_key(key) {
                    cell.rmw_member_shells.insert(key.clone());
                }
                cell.insert_member_entry(key);
                cell.element_ids
                    .entry(key.clone())
                    .or_insert_with(super::fresh_var_id);
                *cell.element_operation_refs.entry(key.clone()).or_default() += 1;
            } else {
                cell.rmw_root_refs += 1;
            }
            cell.rmw_array_epoch
        };
        VariableReceiver {
            contents,
            element,
            array_epoch,
            retained_array: None,
        }
    }
    pub(crate) fn read(&self) -> Result<Option<*mut TclObj>, VarError> {
        if let Some(error) = self.contents.borrow().rmw_retirement {
            return Err(error);
        }
        if let Some(array) = &self.retained_array {
            return Ok(array.read(self.element.as_deref().expect("array element receiver")));
        }
        let contents = self.contents.borrow();
        if let Some(error) = contents.rmw_retirement {
            return Err(error);
        }
        if self.element.is_some() && contents.rmw_array_epoch != self.array_epoch {
            return Err(VarError::DeletedArray);
        }
        match (&self.element, contents.var.as_ref()) {
            (None, Some(Var::Scalar(value))) => Ok(Some(*value)),
            (None, Some(Var::Array(_))) => Err(VarError::IsArray),
            (Some(key), Some(Var::Array(values))) => Ok(values.get(key).copied()),
            (Some(_), Some(Var::Scalar(_))) => Err(VarError::IsScalar),
            (_, None) => Ok(None),
            (_, Some(Var::Link(_))) => Ok(None),
        }
    }

    /// Read native initial contents, including the selected array's current default.
    /// Publication result reads use `read` and never substitute a default.
    pub(crate) fn read_initial(&self) -> Result<Option<*mut TclObj>, VarError> {
        let value = self.read()?;
        if value.is_some() || self.element.is_none() {
            return Ok(value);
        }
        if let Some(array) = &self.retained_array {
            return Ok(array.default_value());
        }
        let contents = self.contents.borrow();
        Ok(matches!(contents.var.as_ref(), Some(Var::Array(_)))
            .then_some(contents.array_default)
            .flatten())
    }

    pub(crate) fn is_element(&self) -> bool {
        self.element.is_some()
    }

    pub(crate) fn store(&self, value: *mut TclObj) -> Result<(), VarError> {
        if let Some(error) = self.contents.borrow().rmw_retirement {
            return Err(error);
        }
        if let Some(array) = &self.retained_array {
            return array.store(
                self.element.as_deref().expect("array element receiver"),
                value,
            );
        }
        let mut contents = self.contents.borrow_mut();
        if let Some(error) = contents.rmw_retirement {
            return Err(error);
        }
        if self.element.is_some() && contents.rmw_array_epoch != self.array_epoch {
            return Err(VarError::DeletedArray);
        }
        if contents.constant {
            return Err(VarError::IsConstant);
        }
        if let Some(key) = &self.element {
            match contents.var.as_mut() {
                Some(Var::Array(values)) => {
                    // SAFETY: the cell takes one owning hold on the live new value.
                    unsafe { obj::incr_ref_count(value) };
                    if let Some(old) = values.insert(key.clone(), value) {
                        // SAFETY: balances the old element's owning hold.
                        unsafe { obj::decr_ref_count(old) };
                    }
                }
                Some(Var::Scalar(_)) => return Err(VarError::IsScalar),
                None => return Err(VarError::DeletedArray),
                Some(Var::Link(_)) => unreachable!("receiver was resolved before capture"),
            }
        } else {
            if matches!(contents.var.as_ref(), Some(Var::Array(_))) {
                return Err(VarError::IsArray);
            }
            // SAFETY: retain new contents before releasing old contents.
            unsafe { obj::incr_ref_count(value) };
            if let Some(old) = contents.var.replace(Var::Scalar(value)) {
                old.release();
            }
            contents.undefined_shell = false;
            contents.rmw_shell_entry = None;
        }
        if let Some(key) = &self.element {
            contents.rmw_member_shells.remove(key);
        }
        Ok(())
    }
}

impl Drop for VariableReceiver {
    fn drop(&mut self) {
        let mut contents = self.contents.borrow_mut();
        contents.rmw_refs -= 1;
        if self.element.is_none() {
            contents.rmw_root_refs -= 1;
        }
        if let Some(key) = &self.element {
            if let Some(count) = contents.element_operation_refs.get_mut(key) {
                *count -= 1;
                if *count == 0 {
                    contents.element_operation_refs.remove(key);
                }
            }
        }
        if let Some(key) = &self.element {
            if contents.rmw_array_epoch == self.array_epoch
                && !contents.element_operation_refs.contains_key(key)
                && !contents.native_member_keys.has_aliases(key)
                && contents.rmw_member_shells.remove(key)
                && matches!(contents.var.as_ref(), Some(Var::Array(map)) if !map.contains_key(key))
            {
                contents.member_order.remove(key);
                contents.native_member_keys.remove(key);
                contents.element_ids.remove(key);
            }
        }
        if contents.rmw_refs == 0
            && contents.operation_refs == 0
            && contents.native_alias_refs == 0
            && contents.var.is_none()
        {
            contents.retire_rmw_shell();
            if !contents.compiled_declaration {
                contents.binding_id = None;
            }
        }
    }
}

impl VarTable {
    /// Native object-variable GET retains an existing root and may create an
    /// element shell only inside an existing array.
    pub(crate) fn capture_get_receiver(
        &mut self,
        name: &[u8],
        element: Option<Vec<u8>>,
    ) -> Result<Option<VariableReceiver>, VarError> {
        let Some(contents) = self.contents(name) else {
            return Ok(None);
        };
        let cell = contents.borrow();
        if cell.binding_id.is_none() {
            return Ok(None);
        }
        if element.is_some() && !matches!(cell.var.as_ref(), Some(Var::Array(_))) {
            return Ok(None);
        }
        drop(cell);
        self.capture_receiver(name, element).map(Some)
    }

    pub(crate) fn capture_receiver(
        &mut self,
        name: &[u8],
        element: Option<Vec<u8>>,
    ) -> Result<VariableReceiver, VarError> {
        if element.is_some() {
            self.ensure_array(name)?;
        }
        let slot = self.slot_for(name);
        self.capture_receiver_at(slot, element, true)?
            .ok_or(VarError::NameProtocolUnavailable)
    }

    /// Capture the actual indexed cell; its name is reporting data only.
    pub(super) fn capture_receiver_at(
        &mut self,
        slot: usize,
        element: Option<Vec<u8>>,
        create: bool,
    ) -> Result<Option<VariableReceiver>, VarError> {
        let name = self
            .cells
            .get(slot)
            .ok_or(VarError::NameProtocolUnavailable)?
            .name
            .clone();
        if element.is_some() {
            let missing = {
                let contents = self.cells[slot].contents.borrow();
                match contents.var.as_ref() {
                    Some(Var::Array(_)) => false,
                    Some(Var::Scalar(_)) => return Err(VarError::IsScalar),
                    Some(Var::Link(_)) => return Err(VarError::NameProtocolUnavailable),
                    None => true,
                }
            };
            if missing {
                if !create {
                    return Ok(None);
                }
                if let Some(old) = self.put_at(slot, Var::Array(Default::default())) {
                    old.release();
                }
            }
        }
        if !self.declared_slots.contains(&slot) {
            self.entry_order.borrow_mut().insert(&name);
        }
        let contents = self.cells[slot].contents.clone();
        {
            let mut cell = contents.borrow_mut();
            let binding = *cell.binding_id.get_or_insert_with(super::fresh_var_id);
            if element.is_none()
                && cell.var.is_none()
                && !self.declared_slots.contains(&slot)
                && cell.rmw_shell_entry.is_none()
            {
                cell.rmw_shell_entry = Some(RootShellEntry::new(
                    &self.entry_order,
                    &self.native_keys,
                    &name,
                    binding,
                ));
            }
        }
        Ok(Some(VariableReceiver::capture_contents(contents, element)))
    }
}

impl super::RetainedArrayCell {
    pub(crate) fn capture_receiver(&self, element: Vec<u8>) -> VariableReceiver {
        let mut cell = self.contents.borrow_mut();
        cell.rmw_refs += 1;
        *cell
            .element_operation_refs
            .entry(element.clone())
            .or_default() += 1;
        VariableReceiver {
            contents: self.contents.clone(),
            element: Some(element),
            array_epoch: cell.rmw_array_epoch,
            retained_array: Some(self.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::obj::Owned;

    #[test]
    fn failed_receivers_retire_only_their_original_unused_entries() {
        let mut table = VarTable::default();
        let first = table.capture_receiver(b"missing", None).unwrap();
        let second = table.capture_receiver(b"missing", None).unwrap();
        drop(first);
        assert_eq!(
            table.entry_order.borrow().physical_keys(),
            vec![b"missing".as_slice()]
        );
        drop(second);
        assert!(table.entry_order.borrow().physical_keys().is_empty());

        table.ensure_array(b"a").unwrap();
        let first = table
            .capture_receiver(b"a", Some(b"unused".to_vec()))
            .unwrap();
        let second = table
            .capture_receiver(b"a", Some(b"unused".to_vec()))
            .unwrap();
        drop(first);
        let array = table.capture_array_cell(b"a").unwrap();
        assert_eq!(array.search_keys(), Some(vec![b"unused".to_vec()]));
        drop(second);
        assert_eq!(array.search_keys(), Some(vec![]));

        let receiver = table
            .capture_receiver(b"a", Some(b"traced".to_vec()))
            .unwrap();
        table.ensure_element_shell(b"a", b"traced").unwrap();
        drop(receiver);
        assert_eq!(array.search_keys(), Some(vec![b"traced".to_vec()]));
    }

    #[test]
    fn trace_shell_definition_survives_trace_removal_without_leaking() {
        crate::counters::reset();
        {
            let mut table = VarTable::default();
            let link = super::super::Link {
                original_jim_target: None,
                native_scalar_entry: None,
                native_element_entry: None,
                array_identity: None,
                array_cell: None,
                home: super::super::VarHome::Frame(0),
                name: b"x".to_vec(),
                elem: None,
            };
            table.insert_link_with_origin(b"x", link, tcl_runtime_api::FrameLinkOrigin::Ordinary);
            table.mark_trace_shell(b"x");
            let id = table.binding_id(b"x");
            let receiver = table.capture_receiver(b"x", None).unwrap();
            let value = Owned::fresh(obj::new_wide_int_obj(1));
            receiver.store(value.as_ptr()).unwrap();
            drop(receiver);
            table.cleanup_trace_shell(b"x", None, id);
            assert_eq!(table.load_scalar(b"x"), Some(value.as_ptr()));
        }
        assert_eq!(crate::counters::finalize(), 0);
    }

    #[test]
    fn native_get_creates_elements_only_with_an_existing_array_root() {
        let mut table = VarTable::default();
        assert!(table
            .capture_get_receiver(b"absent", None)
            .unwrap()
            .is_none());
        assert!(table.binding_id(b"absent").is_none());
        table.slot_for(b"reserved");
        assert!(table
            .capture_get_receiver(b"reserved", None)
            .unwrap()
            .is_none());
        table.ensure_array(b"a").unwrap();
        let receiver = table
            .capture_get_receiver(b"a", Some(b"missing".to_vec()))
            .unwrap()
            .unwrap();
        assert!(receiver.read().unwrap().is_none());
        let array = table.capture_array_cell(b"a").unwrap();
        assert_eq!(array.search_keys(), Some(vec![b"missing".to_vec()]));
    }

    #[test]
    fn initial_default_is_dynamic_and_does_not_replace_publication_result() {
        let mut table = VarTable::default();
        table.ensure_array(b"a").unwrap();
        let first = Owned::fresh(obj::new_string_bytes(b"PRE"));
        let second = Owned::fresh(obj::new_string_bytes(b"NEXT"));
        table.set_array_default(b"a", first.as_ptr());
        let receiver = table
            .capture_get_receiver(b"a", Some(b"x".to_vec()))
            .unwrap()
            .unwrap();
        assert_eq!(receiver.read_initial().unwrap(), Some(first.as_ptr()));
        assert!(receiver.read().unwrap().is_none());
        assert!(obj::is_shared(first.as_ptr()));
        table.set_array_default(b"a", second.as_ptr());
        assert_eq!(receiver.read_initial().unwrap(), Some(second.as_ptr()));
        assert!(!obj::is_shared(first.as_ptr()));
        table.unset_array_default(b"a");
        assert!(receiver.read_initial().unwrap().is_none());
        assert!(!obj::is_shared(second.as_ptr()));
    }

    #[test]
    fn retired_array_default_cannot_follow_same_name_recreation() {
        let mut table = VarTable::default();
        table.ensure_array(b"a").unwrap();
        let old = table.capture_array_cell(b"a").unwrap();
        let first = Owned::fresh(obj::new_string_bytes(b"OLD"));
        let second = Owned::fresh(obj::new_string_bytes(b"NEW"));
        table.set_array_default(b"a", first.as_ptr());
        assert_eq!(old.default_value(), Some(first.as_ptr()));
        let detached = table.begin_array_destruction(b"a").unwrap();
        table.ensure_array(b"a").unwrap();
        table.set_array_default(b"a", second.as_ptr());
        assert!(old.default_value().is_none());
        assert_eq!(table.array_default(b"a"), Some(second.as_ptr()));
        detached.finish_destruction();
    }
}
