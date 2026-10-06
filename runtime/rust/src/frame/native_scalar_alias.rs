// SPDX-License-Identifier: AGPL-3.0-or-later
//! C scalar aliases retain their actual variable entry until the alias is freed.

use super::{CellContents, VarTable};
use std::{cell::RefCell, rc::Rc};
use tcl_runtime_api::VarId;

/// One new alias's genuine entry hold. Borrowed Link clones share this holder.
pub(crate) struct NativeScalarAliasEntry {
    pub(super) contents: Rc<RefCell<CellContents>>,
    pub(super) binding: VarId,
}
impl std::fmt::Debug for NativeScalarAliasEntry {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.debug_tuple("NativeScalarAliasEntry")
            .field(&self.binding)
            .finish()
    }
}
impl PartialEq for NativeScalarAliasEntry {
    fn eq(&self, other: &Self) -> bool {
        self.binding == other.binding && Rc::ptr_eq(&self.contents, &other.contents)
    }
}
impl Eq for NativeScalarAliasEntry {}
impl NativeScalarAliasEntry {
    pub(crate) fn identity(&self) -> VarId {
        self.binding
    }
    pub(crate) fn new_binding(&self) -> Rc<Self> {
        let mut cell = self.contents.borrow_mut();
        cell.native_alias_refs = cell
            .native_alias_refs
            .checked_add(1)
            .expect("native alias holds exhausted");
        drop(cell);
        Rc::new(Self {
            contents: Rc::clone(&self.contents),
            binding: self.binding,
        })
    }
    pub(crate) fn ensure_array(&self) -> Result<(), super::VarError> {
        let mut cell = self.contents.borrow_mut();
        if cell.binding_id != Some(self.binding) {
            return Err(super::VarError::DeletedNamespace);
        }
        if let Some(error) = cell.rmw_retirement {
            return Err(error);
        }
        match cell.var.as_ref() {
            Some(super::Var::Array(_)) => Ok(()),
            Some(_) => Err(super::VarError::IsScalar),
            None => {
                cell.var = Some(super::Var::Array(Default::default()));
                Ok(())
            }
        }
    }
    pub(crate) fn prepare_original_element(
        &self,
        key: &[u8],
        original: Option<*mut crate::obj::TclObj>,
    ) -> Result<(), super::VarError> {
        let mut cell = self.contents.borrow_mut();
        if cell.binding_id != Some(self.binding) {
            return Err(super::VarError::DeletedNamespace);
        }
        if let Some(error) = cell.rmw_retirement {
            return Err(error);
        }
        match cell.var.as_ref() {
            Some(super::Var::Array(_)) => {}
            Some(_) => return Err(super::VarError::IsScalar),
            None => cell.var = Some(super::Var::Array(Default::default())),
        }
        cell.insert_member_entry(key);
        let id = cell.element_ids[key];
        cell.native_member_keys.ensure(id, key, original);
        Ok(())
    }
    pub(crate) fn capture_receiver(
        &self,
        element: Option<Vec<u8>>,
        create: bool,
    ) -> Result<Option<super::VariableReceiver>, super::VarError> {
        let mut cell = self.contents.borrow_mut();
        if cell.binding_id != Some(self.binding) {
            return Err(super::VarError::DeletedNamespace);
        }
        if let Some(error) = cell.rmw_retirement {
            return Err(error);
        }
        if element.is_some() {
            match cell.var.as_ref() {
                Some(super::Var::Array(_)) => {}
                Some(super::Var::Scalar(_)) => return Err(super::VarError::IsScalar),
                Some(super::Var::Link(_)) => return Err(super::VarError::NameProtocolUnavailable),
                None if !create => return Ok(None),
                None => {
                    cell.var = Some(super::Var::Array(Default::default()));
                }
            }
        }
        drop(cell);
        Ok(Some(super::VariableReceiver::capture_contents(
            Rc::clone(&self.contents),
            element,
        )))
    }
}
impl Drop for NativeScalarAliasEntry {
    fn drop(&mut self) {
        let old = {
            let mut cell = self.contents.borrow_mut();
            cell.native_alias_refs = cell
                .native_alias_refs
                .checked_sub(1)
                .expect("actual C alias hold");
            if cell.native_alias_refs != 0
                || cell.binding_id != Some(self.binding)
                || cell.operation_refs != 0
                || cell.rmw_refs != 0
                || cell.namespace_declared
                || (!cell.undefined_shell && cell.var.is_some())
            {
                return;
            }
            let old = cell.var.take();
            cell.undefined_shell = false;
            cell.retire_rmw_shell();
            if !cell.compiled_declaration {
                cell.binding_id = None;
            }
            old
        };
        if let Some(old) = old {
            old.release();
        }
    }
}
impl VarTable {
    pub(crate) fn retain_native_scalar_alias(&mut self, name: &[u8]) -> Rc<NativeScalarAliasEntry> {
        self.prepare_native_name_cell(name, true);
        let slot = self.lookup_slot(name).expect("actual native alias target");
        self.retain_native_scalar_alias_at(slot)
    }
    pub(super) fn retain_native_scalar_alias_at(
        &mut self,
        slot: usize,
    ) -> Rc<NativeScalarAliasEntry> {
        let name = self.cells[slot].name.clone();
        let contents = Rc::clone(&self.cells[slot].contents);
        let binding = {
            let mut cell = contents.borrow_mut();
            let binding = cell.binding_id.expect("actual native target entry");
            cell.native_alias_refs = cell
                .native_alias_refs
                .checked_add(1)
                .expect("native alias holds exhausted");
            if !self.declared_slots.contains(&slot) && cell.rmw_shell_entry.is_none() {
                cell.rmw_shell_entry = Some(super::rmw::RootShellEntry::new(
                    &self.entry_order,
                    &self.native_keys,
                    &name,
                    binding,
                ));
            }
            binding
        };
        Rc::new(NativeScalarAliasEntry { contents, binding })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::obj::{self, Owned};

    #[test]
    fn alias_hold_retains_exact_key_through_unset_and_recreate() {
        let mut table = VarTable::default();
        let name = Owned::fresh(obj::new_string_bytes(b"k"));
        let value = Owned::fresh(obj::new_string_bytes(b"ONE"));
        table.store_scalar(b"k", value.as_ptr()).unwrap();
        table.retain_native_key(b"k", name.as_ptr(), false);
        let hold = table.retain_native_scalar_alias(b"k");
        let original = table.contents(b"k").unwrap().clone();
        let binding = original.borrow().binding_id;
        let borrowed = Rc::clone(&hold);
        assert_eq!(original.borrow().native_alias_refs, 1);
        assert!(table.remove(b"k"));
        assert_eq!(original.borrow().binding_id, binding);
        assert!(table.native_keys.borrow().contains_key(b"k".as_slice()));
        table.store_scalar(b"k", value.as_ptr()).unwrap();
        assert!(Rc::ptr_eq(table.contents(b"k").unwrap(), &original));
        assert_eq!(original.borrow().binding_id, binding);
        assert!(table.remove(b"k"));
        drop(hold);
        assert_eq!(original.borrow().native_alias_refs, 1);
        drop(borrowed);
        assert_eq!(original.borrow().native_alias_refs, 0);
        assert_eq!(original.borrow().binding_id, None);
        assert!(!table.native_keys.borrow().contains_key(b"k".as_slice()));
        // SAFETY: the test owns this live original header throughout.
        assert_eq!(unsafe { (*name.as_ptr()).ref_count }, 1);
    }

    #[test]
    fn namespace_table_retirement_does_not_remain_owned_by_alias() {
        let mut table = VarTable::default();
        let name = Owned::fresh(obj::new_string_bytes(b"k"));
        table.prepare_native_name_cell(b"k", true);
        table.retain_native_key(b"k", name.as_ptr(), false);
        let hold = table.retain_native_scalar_alias(b"k");
        drop(table);
        // SAFETY: table retirement releases its original key despite the cell hold.
        assert_eq!(unsafe { (*name.as_ptr()).ref_count }, 1);
        drop(hold);
    }
}
