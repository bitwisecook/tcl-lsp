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
        if let Some(selected) = self.capture_container_model(model, protocol) {
            selected
                .finish(|| Ok(()))
                .expect("resident array keys retain the selected protocol");
        }
    }

    pub(crate) fn capture_container_model(
        &self,
        model: VariableContainerModel,
        protocol: Option<tcl_syntax::native_string::NativeStringProtocol>,
    ) -> Option<CapturedContainerModel> {
        self.string_protocol.set(protocol);
        let changed = self.container_model.replace(model) != model;
        if changed {
            self.container_transition_pending
                .set(model == VariableContainerModel::DictionaryValue);
        }
        if !self.container_transition_pending.get() || self.container_transition_busy.replace(true)
        {
            return None;
        }
        Some(CapturedContainerModel {
            pending: self.container_transition_pending.clone(),
            busy: self.container_transition_busy.clone(),
            ledger: std::rc::Rc::downgrade(&self.entry_order),
            cells: self
                .cells
                .iter()
                .map(|cell| std::rc::Rc::downgrade(&cell.contents))
                .collect(),
            protocol,
        })
    }

    pub(crate) fn original_table_ledger(
        &self,
    ) -> std::rc::Weak<std::cell::RefCell<tcl_core_types::NativeEntryLedger>> {
        std::rc::Rc::downgrade(&self.entry_order)
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

/// A reached table transition retains its actual allocation identities, never
/// a native value hold. Native cell/member ownership transfers at the same
/// existing construction, retirement and publication stages.
pub(crate) struct CapturedContainerModel {
    pending: std::rc::Rc<std::cell::Cell<bool>>,
    busy: std::rc::Rc<std::cell::Cell<bool>>,
    ledger: std::rc::Weak<std::cell::RefCell<tcl_core_types::NativeEntryLedger>>,
    cells: Vec<std::rc::Weak<std::cell::RefCell<super::CellContents>>>,
    protocol: Option<tcl_syntax::native_string::NativeStringProtocol>,
}
impl CapturedContainerModel {
    pub(crate) fn finish(
        self,
        current: impl Fn() -> Result<(), tcl_syntax::value::ValueError>,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        use tcl_syntax::value::ValueError;
        let table_current = || {
            current()?;
            if self.ledger.strong_count() == 0 {
                return Err(ValueError::CommandProtocolUnavailable(
                    "captured variable container table",
                ));
            }
            Ok(())
        };
        table_current()?;
        let Some(protocol) = self.protocol else {
            self.pending.set(false);
            return Ok(());
        };
        for original in &self.cells {
            table_current()?;
            let Some(cell) = original.upgrade() else {
                return Err(ValueError::CommandProtocolUnavailable(
                    "captured variable container cell",
                ));
            };
            let captured = {
                let contents = cell.borrow();
                match contents.var.as_ref() {
                    Some(Var::Array(values)) => Some((
                        contents.binding_id,
                        contents.rmw_array_epoch,
                        values.clone(),
                    )),
                    _ => None,
                }
            };
            drop(cell);
            let Some((binding, epoch, values)) = captured else {
                continue;
            };
            let same_root = || {
                table_current()?;
                let valid = original.upgrade().is_some_and(|cell| {
                    let contents = cell.borrow();
                    contents.binding_id == binding && contents.rmw_array_epoch == epoch
                        && matches!(contents.var.as_ref(), Some(Var::Array(current)) if *current == values)
                });
                if valid {
                    Ok(())
                } else {
                    Err(ValueError::CommandProtocolUnavailable(
                        "captured variable container root",
                    ))
                }
            };
            let allocations: Vec<_> = values
                .values()
                .map(|value| {
                    obj::check_native_liveness(*value)?;
                    Ok(obj::NativeObjectLifetime::retain(*value))
                })
                .collect::<Result<_, ValueError>>()?;
            let pairs: Vec<_> = values
                .iter()
                .map(|(key, value)| (obj::new_string_bytes(key), *value))
                .collect();
            let reached =
                dict::new_dict_obj_native(&pairs, None, protocol).map(FreshContainerRoot::new);
            same_root()?;
            let root = reached?;
            // This is the existing transition's cell +1, transferred below.
            let root = root.adopt_cell_reference();
            let keys = {
                let cell = original
                    .upgrade()
                    .ok_or(ValueError::CommandProtocolUnavailable(
                        "captured variable container cell",
                    ))?;
                let keys = std::mem::take(&mut cell.borrow_mut().native_member_keys);
                keys
            };
            keys.retire_with_check(&same_root)?;
            same_root()?;
            let published = root.as_ptr();
            let root_allocation = obj::NativeObjectLifetime::retain(published);
            let retired = {
                let cell = original
                    .upgrade()
                    .ok_or(ValueError::CommandProtocolUnavailable(
                        "captured variable container cell",
                    ))?;
                let retired = cell.borrow_mut().var.replace(Var::Scalar(root.into_raw()));
                retired
            };
            // No cell, frame, namespace or table borrow crosses native release.
            if let Some(retired) = retired {
                retired.release();
            }
            table_current()?;
            let same_publication = original.upgrade().is_some_and(|cell| {
                let contents = cell.borrow();
                contents.binding_id == binding && contents.rmw_array_epoch == epoch
                    && matches!(contents.var.as_ref(), Some(Var::Scalar(value)) if *value == published)
            });
            if !same_publication {
                return Err(ValueError::CommandProtocolUnavailable(
                    "retired variable container publication",
                ));
            }
            obj::check_native_liveness(published)?;
            drop(root_allocation);
            drop(allocations);
            table_current()?;
        }
        table_current()?;
        self.pending.set(false);
        Ok(())
    }
}
impl Drop for CapturedContainerModel {
    fn drop(&mut self) {
        self.busy.set(false);
    }
}

/// A fresh transition result is still rc0 until the existing cell-reference
/// stage. An allocation lease permits mandatory cleanup after a refused
/// continuation without changing the sharing decision or native count.
struct FreshContainerRoot {
    original: obj::NativeObjectLifetime,
    transferred: bool,
}
impl FreshContainerRoot {
    fn new(value: *mut TclObj) -> Self {
        Self {
            original: obj::NativeObjectLifetime::retain(value),
            transferred: false,
        }
    }
    fn adopt_cell_reference(mut self) -> Owned {
        let value = self.original.as_ptr();
        unsafe { obj::incr_ref_count(value) };
        self.transferred = true;
        unsafe { Owned::from_raw(value) }
    }
}
impl Drop for FreshContainerRoot {
    fn drop(&mut self) {
        let value = self.original.as_ptr();
        if !self.transferred && obj::allocation_is_live(value) && unsafe { (*value).ref_count == 0 }
        {
            crate::interp::drop_fresh(value);
        }
    }
}
