// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Selected root storage shared by every variable operation, including statics.

use super::{fresh_var_id, Var, VarError, VarTable};
use crate::{
    dict,
    obj::Owned,
    obj::{self, TclObj},
};
use tcl_dialect::VariableContainerModel;

impl VarTable {
    pub(crate) fn set_container_model(
        &self,
        model: VariableContainerModel,
        protocol: Option<tcl_syntax::native_string::NativeStringProtocol>,
    ) {
        self.string_protocol.set(protocol);
        if self.container_model.replace(model) == model || !self.dictionary_variables() {
            return;
        }
        // An interpreter may change profile with live cells. Convert the root
        // representation while retaining the raw cell and its captured aliases.
        for cell in &self.cells {
            let mut contents = cell.contents.borrow_mut();
            let Some(Var::Array(map)) = contents.var.as_ref() else {
                continue;
            };
            let Some(protocol) = protocol else {
                continue;
            };
            let pairs: Vec<_> = map
                .iter()
                .map(|(key, value)| {
                    let key = unsafe {
                        obj::new_string_obj(key.as_ptr().cast(), key.len() as obj::TclSize)
                    };
                    (key, *value)
                })
                .collect();
            let root = dict::new_dict_obj_native(&pairs, None, protocol)
                .expect("resident array keys retain the selected protocol");
            unsafe { obj::incr_ref_count(root) };
            contents.native_member_keys.clear();
            if let Some(old) = contents.var.replace(Var::Scalar(root)) {
                old.release();
            }
        }
    }

    pub(super) fn dictionary_variables(&self) -> bool {
        self.container_model.get() == VariableContainerModel::DictionaryValue
    }

    pub(super) fn prepare_dictionary_root(&self, value: *mut TclObj) -> Result<(), VarError> {
        let protocol = self
            .string_protocol
            .get()
            .ok_or(VarError::NameProtocolUnavailable)?;
        dict::ensure_dict_native(value, protocol).map_err(|error| {
            if error.native_access_refusal().is_some() {
                VarError::NameProtocolUnavailable
            } else {
                VarError::IsScalar
            }
        })
    }

    fn new_dictionary_root(&self) -> Result<Owned, VarError> {
        let protocol = self
            .string_protocol
            .get()
            .ok_or(VarError::NameProtocolUnavailable)?;
        dict::new_dict_obj_native(&[], None, protocol)
            .map(Owned::fresh)
            .map_err(|_| VarError::NameProtocolUnavailable)
    }

    pub(super) fn store_dictionary_root(
        &mut self,
        name: &[u8],
        value: *mut TclObj,
    ) -> Result<(), VarError> {
        let previous = self.publish_dictionary_root(name, value)?;
        drop(previous);
        Ok(())
    }

    pub(super) fn publish_dictionary_root(
        &mut self,
        name: &[u8],
        value: *mut TclObj,
    ) -> Result<RetiredRootContents, VarError> {
        let slot = self.slot_for(name);
        self.entry_order.borrow_mut().insert(name);
        let mut cell = self.cells[slot].contents.borrow_mut();
        if cell.constant {
            return Err(VarError::IsConstant);
        }
        let previous = cell.publish_dictionary_root(value);
        drop(cell);
        Ok(previous)
    }

    pub(super) fn publish_dictionary_element(
        &mut self,
        name: &[u8],
        key: &[u8],
        value: *mut TclObj,
    ) -> Result<RetiredRootContents, VarError> {
        if self.is_constant(name) {
            return Err(VarError::IsConstant);
        }
        let root = self.dictionary_copy(name)?;
        let key = Owned::fresh(unsafe {
            obj::new_string_obj(key.as_ptr().cast(), key.len() as obj::TclSize)
        });
        dict::dict_set(root.as_ptr(), key.as_ptr(), value).map_err(|_| VarError::IsScalar)?;
        self.publish_dictionary_root(name, root.as_ptr())
    }

    fn dictionary_copy(&self, name: &[u8]) -> Result<Owned, VarError> {
        let Some(root) = self.load_scalar(name) else {
            return self.new_dictionary_root();
        };
        self.prepare_dictionary_root(root)?;
        Ok(Owned::fresh(obj::duplicate(root)))
    }

    pub(super) fn ensure_dictionary_root(&mut self, name: &[u8]) -> Result<(), VarError> {
        if let Some(root) = self.load_scalar(name) {
            return self.prepare_dictionary_root(root);
        }
        let root = self.new_dictionary_root()?;
        self.store_dictionary_root(name, root.as_ptr())
    }

    pub(super) fn remove_dictionary_element(&mut self, name: &[u8], key: &[u8]) -> bool {
        let Ok(root) = self.dictionary_copy(name) else {
            return false;
        };
        if !dict::dict_unset(root.as_ptr(), key).unwrap_or(false) {
            return false;
        }
        self.store_dictionary_root(name, root.as_ptr()).is_ok()
    }
}

/// Ownership removed from a published root. Retirement happens after the caller
/// releases every cell, frame and namespace borrow.
pub(crate) struct RetiredRootContents {
    previous: Option<Var>,
    _member_keys: super::native_element_entry::NativeElementEntries,
}
impl Drop for RetiredRootContents {
    fn drop(&mut self) {
        if let Some(previous) = self.previous.take() {
            previous.release();
        }
    }
}
impl super::CellContents {
    pub(super) fn publish_dictionary_root(&mut self, value: *mut TclObj) -> RetiredRootContents {
        // SAFETY: publication takes the actual owning cell's reference first.
        unsafe { obj::incr_ref_count(value) };
        self.binding_id.get_or_insert_with(fresh_var_id);
        let previous = self.var.replace(Var::Scalar(value));
        self.element_ids.clear();
        self.element_operation_refs.clear();
        RetiredRootContents {
            previous,
            _member_keys: std::mem::take(&mut self.native_member_keys),
        }
    }
}
