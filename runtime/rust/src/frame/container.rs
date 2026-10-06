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
        let slot = self.slot_for(name);
        self.entry_order.borrow_mut().insert(name);
        let mut cell = self.cells[slot].contents.borrow_mut();
        if cell.constant {
            return Err(VarError::IsConstant);
        }
        if cell.binding_id.is_none() {
            cell.binding_id = Some(fresh_var_id());
        }
        unsafe { obj::incr_ref_count(value) };
        if let Some(old) = cell.var.replace(Var::Scalar(value)) {
            old.release();
        }
        // Logical element names will be selected from the replacement value.
        cell.element_ids.clear();
        cell.native_member_keys.clear();
        cell.element_operation_refs.clear();
        Ok(())
    }

    pub(super) fn store_dictionary_element(
        &mut self,
        name: &[u8],
        key: &[u8],
        value: *mut TclObj,
    ) -> Result<(), VarError> {
        if self.is_constant(name) {
            return Err(VarError::IsConstant);
        }
        let root = self.dictionary_copy(name)?;
        let key = Owned::fresh(unsafe {
            obj::new_string_obj(key.as_ptr().cast(), key.len() as obj::TclSize)
        });
        dict::dict_set(root.as_ptr(), key.as_ptr(), value).map_err(|_| VarError::IsScalar)?;
        self.store_dictionary_root(name, root.as_ptr())
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
