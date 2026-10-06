// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Retained array generations and staged physical member destruction.

use super::{Cell, CellContents, Var, VarError, VarTable};
use crate::obj::{self, TclObj};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    fmt,
    rc::{Rc, Weak},
};
use tcl_runtime_api::VarId;

pub(super) struct DetachedArray {
    pub(super) values: BTreeMap<Vec<u8>, *mut TclObj>,
    pub(super) retired: BTreeSet<Vec<u8>>,
    pub(super) entry_order: tcl_core_types::NativeEntryLedger,
    pub(super) native_keys: super::native_element_entry::NativeElementEntries,
}

/// One actual C array generation retained independently of its current name.
#[derive(Clone)]
pub struct RetainedArrayCell {
    pub(super) contents: Rc<RefCell<CellContents>>,
    pub(super) id: VarId,
    pub(super) epoch: u64,
}

/// Non-owning access to one original array incarnation for a native search.
/// This receipt owns no variable or object reference and never resolves a name.
#[derive(Clone)]
pub struct WeakArraySearchCell {
    contents: Weak<RefCell<CellContents>>,
    id: VarId,
}

impl WeakArraySearchCell {
    /// Original never-reused variable identity; not a new lookup authority.
    #[must_use]
    pub fn identity(&self) -> VarId {
        self.id
    }

    /// Current dedicated search epoch, or absence after actual root retirement.
    #[must_use]
    pub fn revision(&self) -> Option<u64> {
        let contents = self.contents.upgrade()?;
        let contents = contents.borrow();
        (contents.binding_id == Some(self.id)
            && matches!(contents.var.as_ref(), Some(Var::Array(_))))
        .then_some(contents.array_search_epoch)
    }

    /// Exact native bucket inventory, including attached undefined entries.
    pub fn keys(&self) -> Result<Option<Vec<Vec<u8>>>, tcl_syntax::value::ValueError> {
        let Some(contents) = self.contents.upgrade() else {
            return Ok(None);
        };
        let contents = contents.borrow();
        if contents.binding_id != Some(self.id)
            || !matches!(contents.var.as_ref(), Some(Var::Array(_)))
        {
            return Ok(None);
        }
        let keys = contents.member_order.keys().ok_or(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable("native array search table"),
        )?;
        Ok(Some(keys.into_iter().map(<[u8]>::to_vec).collect()))
    }

    /// Whether the exact current candidate has a value, without read traces.
    #[must_use]
    pub fn element_is_defined(&self, key: &[u8]) -> bool {
        let Some(contents) = self.contents.upgrade() else {
            return false;
        };
        let contents = contents.borrow();
        contents.binding_id == Some(self.id)
            && matches!(contents.var.as_ref(), Some(Var::Array(map)) if map.contains_key(key))
    }
}

impl fmt::Debug for RetainedArrayCell {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("RetainedArrayCell")
            .field(&self.id)
            .finish()
    }
}

impl PartialEq for RetainedArrayCell {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.epoch == other.epoch
            && Rc::ptr_eq(&self.contents, &other.contents)
    }
}
impl Eq for RetainedArrayCell {}

impl RetainedArrayCell {
    pub(crate) fn prepare_original_native_element(
        &self,
        element: &[u8],
        original: Option<*mut TclObj>,
    ) -> Result<(), VarError> {
        let mut contents = self.contents.borrow_mut();
        if contents.detached_arrays.contains_key(&self.id) {
            return Err(VarError::DeletedArray);
        }
        if contents.binding_id != Some(self.id)
            || contents.rmw_array_epoch != self.epoch
            || !matches!(contents.var.as_ref(), Some(Var::Array(_)))
        {
            return Err(VarError::DeletedArray);
        }
        contents.insert_member_entry(element);
        contents
            .element_ids
            .entry(element.to_vec())
            .or_insert_with(super::fresh_var_id);
        let id = contents.element_ids[element];
        contents.native_member_keys.ensure(id, element, original);
        Ok(())
    }
    /// The cursor chain belongs to the exact original array, without a root pin.
    pub(crate) fn native_array_search(
        &self,
        sub: &str,
        name: &[u8],
        handle: Option<*mut TclObj>,
        bytes: Option<&[u8]>,
        cache: Option<tcl_core_types::NativeArraySearchCache>,
        protocol: tcl_syntax::native_array_search::NativeArraySearchProtocol,
    ) -> Result<
        Result<*mut TclObj, tcl_syntax::native_array_search::NativeArraySearchFailure>,
        tcl_syntax::value::ValueError,
    > {
        use tcl_syntax::value::ValueError;
        let mut contents = self.contents.borrow_mut();
        if contents.binding_id != Some(self.id)
            || contents.rmw_array_epoch != self.epoch
            || !matches!(contents.var, Some(Var::Array(_)))
        {
            return Err(ValueError::CommandProtocolUnavailable(
                "original array search cell",
            ));
        }
        let keys = contents
            .member_order
            .keys()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native array search table",
            ))?
            .into_iter()
            .map(<[u8]>::to_vec)
            .collect();
        if sub == "startsearch" {
            let next =
                contents
                    .array_searches
                    .next_id()
                    .ok_or(ValueError::CommandProtocolUnavailable(
                        "native array search id overflow",
                    ))?;
            let value = obj::new_string_bytes(&protocol.handle(next, name));
            contents.array_searches.insert(
                next,
                (!protocol.caches_handle()).then(|| obj::Owned::retain(value)),
                keys,
            );
            return Ok(Ok(value));
        }
        let handle = handle.ok_or(ValueError::CommandProtocolUnavailable(
            "original search handle",
        ))?;
        let found = tcl_cmd_core::native_array_search::resolve(
            &contents.array_searches,
            protocol,
            name,
            &tcl_cmd_core::native_array_search::NativeArraySearchOperand {
                original: &handle,
                bytes: bytes.unwrap_or_default(),
                cache,
            },
            |held, original| held.as_ptr() == *original,
            |held| {
                crate::dict::native_object_bytes(
                    held.as_ptr(),
                    tcl_syntax::native_string::NativeStringProtocol::C(protocol.version()),
                )
            },
        )?;
        let search = match found {
            Ok(search) => search,
            Err(failure) => return Ok(Err(failure)),
        };
        if sub == "donesearch" {
            contents.array_searches.remove(search);
            return Ok(Ok(obj::new_string_bytes(b"")));
        }
        let CellContents {
            array_searches,
            var,
            ..
        } = &mut *contents;
        let next = array_searches.next_defined(
            search,
            sub == "nextelement",
            |key| matches!(var.as_ref(), Some(Var::Array(map)) if map.contains_key(key)),
        );
        Ok(Ok(if sub == "anymore" {
            obj::new_wide_int_obj(i64::from(next.is_some()))
        } else {
            obj::new_string_bytes(&next.unwrap_or_default())
        }))
    }

    /// Retain only a weak original-incarnation receipt for an array search.
    #[must_use]
    pub fn weak_search_cell(&self) -> WeakArraySearchCell {
        WeakArraySearchCell {
            contents: Rc::downgrade(&self.contents),
            id: self.id,
        }
    }

    pub(crate) fn identity(&self) -> VarId {
        self.id
    }

    fn members<'a>(
        &self,
        contents: &'a CellContents,
    ) -> Option<&'a BTreeMap<Vec<u8>, *mut TclObj>> {
        if let Some(members) = contents.detached_arrays.get(&self.id) {
            return Some(&members.values);
        }
        match contents.var.as_ref() {
            Some(Var::Array(members))
                if contents.binding_id == Some(self.id)
                    && contents.rmw_array_epoch == self.epoch =>
            {
                Some(members)
            }
            _ => None,
        }
    }

    /// Physical attached key inventory of this selected array incarnation.
    /// Undefined retained shells participate even when they have no value.
    pub(crate) fn search_keys(&self) -> Option<Vec<Vec<u8>>> {
        let contents = self.contents.borrow();
        if let Some(detached) = contents.detached_arrays.get(&self.id) {
            let keys: Vec<_> = match detached.entry_order.keys() {
                Some(keys) => keys.into_iter().map(<[u8]>::to_vec).collect(),
                None => detached
                    .entry_order
                    .physical_keys()
                    .into_iter()
                    .map(<[u8]>::to_vec)
                    .collect(),
            };
            return Some(
                keys.into_iter()
                    .filter(|key| !detached.retired.contains(key))
                    .collect(),
            );
        }
        self.members(&contents)?;
        Some(match contents.member_order.keys() {
            Some(keys) => keys.into_iter().map(<[u8]>::to_vec).collect(),
            None => contents
                .member_order
                .physical_keys()
                .into_iter()
                .map(<[u8]>::to_vec)
                .collect(),
        })
    }

    pub(crate) fn is_live(&self) -> bool {
        self.members(&self.contents.borrow()).is_some()
    }

    pub(crate) fn read(&self, element: &[u8]) -> Option<*mut TclObj> {
        self.members(&self.contents.borrow())?.get(element).copied()
    }

    pub(crate) fn default_value(&self) -> Option<*mut TclObj> {
        let contents = self.contents.borrow();
        if contents.binding_id == Some(self.id)
            && contents.rmw_array_epoch == self.epoch
            && matches!(contents.var.as_ref(), Some(Var::Array(_)))
        {
            contents.array_default
        } else {
            None
        }
    }

    pub(crate) fn store(&self, element: &[u8], value: *mut TclObj) -> Result<(), VarError> {
        let mut contents = self.contents.borrow_mut();
        if contents.constant {
            return Err(VarError::IsConstant);
        }
        let detached = contents.detached_arrays.contains_key(&self.id);
        let members = if detached {
            let selected = contents
                .detached_arrays
                .get_mut(&self.id)
                .expect("selected detached generation");
            if selected.retired.contains(element) {
                return Err(VarError::DeletedArray);
            }
            selected.entry_order.insert(element);
            &mut selected.values
        } else if contents.binding_id == Some(self.id) && contents.rmw_array_epoch == self.epoch {
            match contents.var.as_mut() {
                Some(Var::Array(members)) => members,
                _ => return Err(VarError::DeletedArray),
            }
        } else {
            return Err(VarError::DeletedArray);
        };
        // SAFETY: the selected physical element owns its new reference.
        unsafe { obj::incr_ref_count(value) };
        if let Some(old) = members.insert(element.to_vec(), value) {
            // SAFETY: balances the replaced physical element's owning hold.
            unsafe { obj::decr_ref_count(old) };
        }
        if !detached {
            contents.insert_member_entry(element);
            contents.rmw_member_shells.remove(element);
            contents
                .element_ids
                .entry(element.to_vec())
                .or_insert_with(super::fresh_var_id);
        }
        Ok(())
    }

    pub(crate) fn remove(&self, element: &[u8]) -> bool {
        let mut contents = self.contents.borrow_mut();
        if contents.rmw_array_epoch != self.epoch
            && !contents.detached_arrays.contains_key(&self.id)
        {
            return false;
        }
        if contents.binding_id == Some(self.id) && contents.member_order.contains_key(element) {
            contents.array_search_epoch = contents.array_search_epoch.wrapping_add(1);
            contents.array_searches.clear();
        }
        let old = if let Some(members) = contents.detached_arrays.get_mut(&self.id) {
            members.entry_order.remove(element);
            members.values.remove(element)
        } else if contents.binding_id == Some(self.id) && contents.rmw_array_epoch == self.epoch {
            match contents.var.as_mut() {
                Some(Var::Array(members)) => members.remove(element),
                _ => None,
            }
        } else {
            None
        };
        if !contents.detached_arrays.contains_key(&self.id)
            && !contents.element_operation_refs.contains_key(element)
            && !contents.native_member_keys.has_aliases(element)
        {
            contents.element_ids.remove(element);
            contents.member_order.remove(element);
            contents.native_member_keys.remove(element);
        }
        if let Some(old) = old {
            // SAFETY: releasing this element balances its stored reference.
            unsafe { obj::decr_ref_count(old) };
            true
        } else {
            false
        }
    }

    pub(crate) fn begin_member_retirement(
        &self,
        element: &[u8],
        preserve_definition: bool,
        traced: bool,
    ) -> Option<super::native_element_entry::NativeElementRetirementTrace> {
        let cell = self.contents.borrow();
        let selected = cell.detached_arrays.get(&self.id)?;
        selected.native_keys.retirement_trace(
            element,
            preserve_definition,
            selected.values.contains_key(element),
            traced,
        )
    }

    pub(crate) fn retire_member(&self, element: &[u8]) {
        self.remove(element);
        if let Some(selected) = self.contents.borrow_mut().detached_arrays.get_mut(&self.id) {
            selected.retired.insert(element.to_vec());
            selected.native_keys.retire(element);
        }
    }

    pub(crate) fn finish_member_retirement(&self, element: &[u8], object_table: bool) {
        if let Some(selected) = self.contents.borrow_mut().detached_arrays.get_mut(&self.id) {
            selected
                .native_keys
                .finish_member_retirement(element, object_table);
        }
    }

    pub(crate) fn elements(&self) -> Vec<Vec<u8>> {
        let contents = self.contents.borrow();
        let Some(members) = self.members(&contents) else {
            return Vec::new();
        };
        let order = contents
            .detached_arrays
            .get(&self.id)
            .map_or(&contents.member_order, |selected| &selected.entry_order);
        match order.keys() {
            Some(keys) => keys
                .into_iter()
                .filter(|key| members.contains_key(*key))
                .map(<[u8]>::to_vec)
                .collect(),
            None => members.keys().cloned().collect(),
        }
    }

    pub(crate) fn finish_destruction(&self) {
        let members = self.contents.borrow_mut().detached_arrays.remove(&self.id);
        if let Some(members) = members {
            Var::Array(members.values).release();
        }
    }
}

impl VarTable {
    pub(crate) fn capture_array_cell(&self, name: &[u8]) -> Option<RetainedArrayCell> {
        let contents = self.contents(name)?.clone();
        let borrowed = contents.borrow();
        if !matches!(borrowed.var.as_ref(), Some(Var::Array(_))) {
            return None;
        }
        let id = borrowed.binding_id?;
        drop(borrowed);
        let epoch = contents.borrow().rmw_array_epoch;
        Some(RetainedArrayCell {
            contents,
            id,
            epoch,
        })
    }

    /// Detach the root name while preserving the selected old member table.
    /// A protected root RMW shell remains refillable; its old elements use the
    /// separate retained generation until their individual retirement.
    pub(crate) fn begin_array_destruction(&mut self, name: &[u8]) -> Option<RetainedArrayCell> {
        let slot = self.lookup_slot(name)?;
        let selected = self.capture_array_cell(name)?;
        {
            let mut contents = selected.contents.borrow_mut();
            let Some(Var::Array(members)) = contents.var.take() else {
                unreachable!("selected array")
            };
            let mut entry_order = std::mem::take(&mut contents.member_order);
            entry_order.select_recipe(self.hash_recipe.get());
            // Undefined physical shells were recorded when originally allocated.
            let native_keys = std::mem::take(&mut contents.native_member_keys);
            contents.detached_arrays.insert(
                selected.id,
                DetachedArray {
                    values: members,
                    retired: BTreeSet::new(),
                    entry_order,
                    native_keys,
                },
            );
            contents.rmw_array_epoch = contents.rmw_array_epoch.wrapping_add(1);
            contents.array_search_epoch = contents.array_search_epoch.wrapping_add(1);
            contents.array_searches.clear();
            contents.element_ids.clear();
        }
        self.unset_array_default(name);
        let preserve_root = {
            let contents = selected.contents.borrow();
            contents.rmw_root_refs != 0
                || contents.operation_refs != 0
                || contents.native_alias_refs != 0
        };
        if !preserve_root {
            self.retire_native_entry(name);
            self.cells[slot] = Cell::empty(name);
        }
        Some(selected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::obj::Owned;

    #[test]
    fn weak_search_tracks_native_invalidation_without_retaining_the_array() {
        crate::counters::reset();
        {
            let mut table = VarTable::default();
            table.set_hash_recipe(Some(tcl_core_types::NativeHashRecipe::Tcl {
                promotion: tcl_core_types::NativeHashBytePromotion::Unsigned,
                width: tcl_core_types::NativeHashWordWidth::Bits64,
            }));
            let value = Owned::fresh(obj::new_wide_int_obj(1));
            table.store_elem(b"a", b"existing", value.as_ptr()).unwrap();
            table.ensure_element_shell(b"a", b"undefined").unwrap();
            let array = table.capture_array_cell(b"a").unwrap();
            let weak = array.weak_search_cell();
            let revision = weak.revision().unwrap();
            table.cleanup_trace_shell(b"a", Some(b"undefined"), Some(array.identity()));
            assert_eq!(weak.revision(), Some(revision));
            table.store_elem(b"a", b"existing", value.as_ptr()).unwrap();
            assert_eq!(weak.revision(), Some(revision));
            assert!(weak.element_is_defined(b"existing"));
            assert_eq!(weak.keys().unwrap(), Some(vec![b"existing".to_vec()]));
            table.store_elem(b"a", b"new", value.as_ptr()).unwrap();
            assert_ne!(weak.revision(), Some(revision));
            let revision = weak.revision().unwrap();
            table.remove_elem(b"a", b"new");
            assert_ne!(weak.revision(), Some(revision));
            let id = weak.identity();
            drop(array);
            table.remove(b"a");
            assert!(weak.revision().is_none());
            table
                .store_elem(b"a", b"replacement", value.as_ptr())
                .unwrap();
            let replacement = table.capture_array_cell(b"a").unwrap();
            assert_ne!(replacement.identity(), id);
            assert_eq!(weak.keys().unwrap(), None);
            drop(replacement);
            let live = table.capture_array_cell(b"a").unwrap().weak_search_cell();
            drop(table);
            assert!(live.revision().is_none());
        }
        assert_eq!(crate::counters::finalize(), 0);
    }

    #[test]
    fn search_keys_keep_undefined_members_and_selected_incarnation() {
        crate::counters::reset();
        {
            let mut table = VarTable::default();
            let old_value = Owned::fresh(obj::new_wide_int_obj(7));
            table
                .store_elem(b"array", b"old\0\xff", old_value.as_ptr())
                .unwrap();
            let undefined = table
                .capture_receiver(b"array", Some(b"undefined\xc0\x80".to_vec()))
                .unwrap();
            let selected = table.capture_array_cell(b"array").unwrap();
            assert_eq!(
                selected.search_keys().unwrap(),
                [b"old\0\xff".to_vec(), b"undefined\xc0\x80".to_vec()]
            );
            assert_eq!(selected.read(b"undefined\xc0\x80"), None);
            assert_eq!(selected.read(b"old\0\xff"), Some(old_value.as_ptr()));
            let detached = table.begin_array_destruction(b"array").unwrap();
            let replacement = Owned::fresh(obj::new_wide_int_obj(9));
            table
                .store_elem(b"array", b"replacement", replacement.as_ptr())
                .unwrap();
            assert_eq!(
                selected.search_keys().unwrap(),
                [b"old\0\xff".to_vec(), b"undefined\xc0\x80".to_vec()]
            );
            assert_eq!(selected.read(b"replacement"), None);
            detached.retire_member(b"old\0\xff");
            assert_eq!(
                selected.search_keys().unwrap(),
                [b"undefined\xc0\x80".to_vec()]
            );
            detached.finish_destruction();
            assert_eq!(selected.search_keys(), None);
            assert_eq!(
                table.array_names(b"array").unwrap(),
                [b"replacement".to_vec()]
            );
            drop(undefined);
        }
        assert_eq!(crate::counters::finalize(), 0);
    }
}
