// SPDX-License-Identifier: AGPL-3.0-or-later
//! Array operations select the original cell, independently of its name.

use super::{CellContents, NativeScalarAliasEntry, RetainedArrayCell, Var, VarTable};
use crate::obj::{self, TclObj};
use std::{cell::RefCell, rc::Rc};
use tcl_runtime_api::{ArrayDefaultState, VarId};

#[derive(Clone)]
pub(crate) struct NativeArrayTargetCell {
    contents: Rc<RefCell<CellContents>>,
    id: VarId,
}
impl std::fmt::Debug for NativeArrayTargetCell {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.debug_tuple("NativeArrayTargetCell")
            .field(&self.id)
            .finish()
    }
}
impl NativeArrayTargetCell {
    pub(crate) fn identity(&self) -> VarId {
        self.id
    }
    pub(crate) fn array(&self) -> Option<RetainedArrayCell> {
        let cell = self.contents.borrow();
        (cell.binding_id == Some(self.id)
            && cell.rmw_retirement.is_none()
            && matches!(cell.var.as_ref(), Some(Var::Array(_))))
        .then(|| RetainedArrayCell {
            contents: Rc::clone(&self.contents),
            id: self.id,
            epoch: cell.rmw_array_epoch,
        })
    }
    pub(crate) fn is_set(&self) -> bool {
        let cell = self.contents.borrow();
        cell.binding_id == Some(self.id)
            && cell.rmw_retirement.is_none()
            && matches!(cell.var.as_ref(), Some(Var::Scalar(_) | Var::Array(_)))
    }
    pub(crate) fn default_state(&self) -> ArrayDefaultState<*mut TclObj> {
        let cell = self.contents.borrow();
        if cell.binding_id != Some(self.id) || cell.rmw_retirement.is_some() {
            return ArrayDefaultState::Undefined;
        }
        match cell.var.as_ref() {
            Some(Var::Array(_)) => ArrayDefaultState::Array {
                default: cell.array_default,
            },
            Some(Var::Scalar(_)) => ArrayDefaultState::NonArray,
            _ => ArrayDefaultState::Undefined,
        }
    }
    pub(crate) fn unset_default(&self) {
        let old = {
            let mut cell = self.contents.borrow_mut();
            if cell.binding_id != Some(self.id) || !matches!(cell.var.as_ref(), Some(Var::Array(_)))
            {
                return;
            }
            cell.array_default.take()
        };
        if let Some(old) = old {
            // SAFETY: the same actual array owns these two references.
            unsafe {
                obj::decr_ref_count(old);
                obj::decr_ref_count(old);
            }
        }
    }
    pub(crate) fn retain_operation(&self) -> bool {
        let mut cell = self.contents.borrow_mut();
        if cell.binding_id != Some(self.id) || cell.rmw_retirement.is_some() {
            return false;
        }
        cell.operation_refs = cell
            .operation_refs
            .checked_add(1)
            .expect("array operation overflow");
        true
    }
    pub(crate) fn release_operation(&self) {
        let mut cell = self.contents.borrow_mut();
        if cell.binding_id != Some(self.id) || cell.operation_refs == 0 {
            return;
        }
        cell.operation_refs -= 1;
        if cell.operation_refs == 0
            && cell.rmw_refs == 0
            && cell.native_alias_refs == 0
            && cell.var.is_none()
        {
            cell.retire_rmw_shell();
            if !cell.compiled_declaration {
                cell.binding_id = None;
            }
        }
    }
    pub(crate) fn element_id(&self, key: &[u8]) -> Option<VarId> {
        let cell = self.contents.borrow();
        (cell.binding_id == Some(self.id)
            && matches!(cell.var.as_ref(), Some(Var::Array(values)) if values.contains_key(key)))
        .then(|| cell.element_ids.get(key).copied())
        .flatten()
    }
    pub(crate) fn retain_element(&self, key: &[u8], element: VarId) -> bool {
        let mut cell = self.contents.borrow_mut();
        if cell.binding_id != Some(self.id) || cell.element_ids.get(key) != Some(&element) {
            return false;
        }
        *cell.element_operation_refs.entry(key.to_vec()).or_default() += 1;
        true
    }
    pub(crate) fn release_element(&self, key: &[u8], element: VarId) {
        let mut cell = self.contents.borrow_mut();
        if cell.binding_id != Some(self.id) || cell.element_ids.get(key) != Some(&element) {
            return;
        }
        let Some(count) = cell.element_operation_refs.get_mut(key) else {
            return;
        };
        *count = count.checked_sub(1).expect("element operation underflow");
        if *count == 0 {
            cell.element_operation_refs.remove(key);
            let defined =
                matches!(cell.var.as_ref(), Some(Var::Array(values)) if values.contains_key(key));
            if !defined && !cell.native_member_keys.has_aliases(key) {
                cell.element_ids.remove(key);
                cell.member_order.remove(key);
                cell.native_member_keys.remove(key);
            }
        }
    }
    pub(crate) fn read_element(&self, key: &[u8], element: VarId) -> Option<*mut TclObj> {
        let cell = self.contents.borrow();
        if cell.binding_id != Some(self.id) || cell.element_ids.get(key) != Some(&element) {
            return None;
        }
        match cell.var.as_ref()? {
            Var::Array(values) => values.get(key).copied(),
            _ => None,
        }
    }
}
impl NativeScalarAliasEntry {
    pub(crate) fn array_operation_cell(&self) -> Option<NativeArrayTargetCell> {
        let cell = self.contents.borrow();
        (cell.binding_id == Some(self.binding) && cell.rmw_retirement.is_none()).then(|| {
            NativeArrayTargetCell {
                contents: Rc::clone(&self.contents),
                id: self.binding,
            }
        })
    }
}
impl VarTable {
    pub(crate) fn array_operation_cell(&self, name: &[u8]) -> Option<NativeArrayTargetCell> {
        let contents = self.contents(name)?;
        let mut cell = contents.borrow_mut();
        let id = cell.binding_id?;
        if cell.rmw_retirement.is_some() {
            return None;
        }
        if !cell.compiled_declaration && cell.rmw_shell_entry.is_none() {
            cell.rmw_shell_entry = Some(super::rmw::RootShellEntry::new(
                &self.entry_order,
                &self.native_keys,
                name,
                id,
            ));
        }
        Some(NativeArrayTargetCell {
            contents: Rc::clone(contents),
            id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::obj::Owned;

    #[test]
    fn scalar_alias_array_operations_keep_the_selected_duplicate_declaration() {
        let mut table = VarTable::default();
        let first = table.declare_compiled_cell(b"same");
        let second = table.declare_compiled_cell(b"same");
        drop(
            table
                .capture_receiver_at(first, None, true)
                .unwrap()
                .unwrap(),
        );
        drop(
            table
                .capture_receiver_at(second, None, true)
                .unwrap()
                .unwrap(),
        );
        let first_alias = table.retain_native_scalar_alias_at(first);
        let second_alias = table.retain_native_scalar_alias_at(second);
        let value = Owned::fresh(obj::new_string_bytes(b"SECOND"));
        second_alias.prepare_original_element(b"k", None).unwrap();
        second_alias
            .capture_receiver(Some(b"k".to_vec()), true)
            .unwrap()
            .unwrap()
            .store(value.as_ptr())
            .unwrap();
        let selected = second_alias.array_operation_cell().unwrap();
        let first_selected = first_alias.array_operation_cell().unwrap();
        assert_ne!(selected.identity(), first_selected.identity());
        assert!(first_selected.array().is_none());
        let element = selected.element_id(b"k").unwrap();
        assert_eq!(selected.array().unwrap().elements(), [b"k".to_vec()]);
        assert_eq!(selected.read_element(b"k", element), Some(value.as_ptr()));
        assert!(selected.retain_operation());
        assert!(selected.retain_element(b"k", element));
        assert!(selected.array().unwrap().remove(b"k"));
        assert_eq!(selected.read_element(b"k", element), None);
        selected.release_element(b"k", element);
        selected.release_operation();
        assert!(first_selected.array().is_none());
    }

    #[test]
    fn current_alias_receiver_and_captured_search_keep_distinct_array_generations() {
        let mut table = VarTable::default();
        let alias = table.retain_native_scalar_alias(b"array");
        let value = Owned::fresh(obj::new_string_bytes(b"OLD"));
        alias.prepare_original_element(b"old", None).unwrap();
        alias
            .capture_receiver(Some(b"old".to_vec()), true)
            .unwrap()
            .unwrap()
            .store(value.as_ptr())
            .unwrap();
        let target = alias.array_operation_cell().unwrap();
        let search_generation = target.array().unwrap();
        assert!(target.retain_operation());
        assert!(alias
            .capture_receiver(None, false)
            .unwrap()
            .unwrap()
            .unset()
            .unwrap());
        alias.prepare_original_element(b"new", None).unwrap();
        alias
            .capture_receiver(Some(b"new".to_vec()), true)
            .unwrap()
            .unwrap()
            .store(value.as_ptr())
            .unwrap();
        assert_eq!(target.array().unwrap().elements(), [b"new".to_vec()]);
        assert!(!search_generation.is_live());
        assert!(search_generation.search_keys().is_none());
        target.release_operation();
    }
}
