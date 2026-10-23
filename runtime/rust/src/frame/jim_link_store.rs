// SPDX-License-Identifier: AGPL-3.0-or-later
//! Exact weak compatibility link targets, published outside their owner borrows.
//!
//! Link(None) contains no original Jim recursive-name object. This adapter keeps
//! its existing physical storage policy; it does not issue Jim COW authority.

use super::{CellContents, Var, VarError, VarTable};
use crate::{
    dict::EnteredNativeDictionary,
    interp::native_operation_currency::NativeOperationCurrency,
    obj::{self, NativeObjectLifetime, Owned, TclObj},
};
use std::{cell::RefCell, rc::Weak};
use tcl_cmd_core::CmdError;
use tcl_runtime_api::VarId;
use tcl_syntax::{native_string::NativeStringProtocol, value::ValueError};

pub(crate) enum JimLinkStoreError {
    Variable(VarError),
    Command(CmdError),
}
impl From<VarError> for JimLinkStoreError {
    fn from(error: VarError) -> Self {
        Self::Variable(error)
    }
}
impl From<ValueError> for JimLinkStoreError {
    fn from(error: ValueError) -> Self {
        if error.native_access_refusal().is_some() {
            Self::Command(error.into())
        } else {
            // The existing physical dictionary store reports a scalar mismatch.
            Self::Variable(VarError::IsScalar)
        }
    }
}

#[derive(Clone)]
pub(crate) struct CapturedJimLinkRoot {
    cell: Weak<RefCell<CellContents>>,
    binding: Option<VarId>,
    epoch: u64,
    name: Vec<u8>,
    element: Option<Vec<u8>>,
    dictionary: bool,
    protocol: Option<NativeStringProtocol>,
    ledger: Weak<RefCell<tcl_core_types::NativeEntryLedger>>,
}
impl VarTable {
    pub(crate) fn capture_byte_store_root(
        &mut self,
        name: &[u8],
        element: Option<Vec<u8>>,
    ) -> Result<CapturedJimLinkRoot, VarError> {
        self.slot_for(name);
        self.capture_jim_link_root(name, element)
    }
    pub(crate) fn capture_jim_link_root(
        &self,
        name: &[u8],
        element: Option<Vec<u8>>,
    ) -> Result<CapturedJimLinkRoot, VarError> {
        let cell = self
            .contents(name)
            .ok_or(VarError::NameProtocolUnavailable)?;
        let contents = cell.borrow();
        if contents.rmw_retirement.is_some() {
            return Err(VarError::NameProtocolUnavailable);
        }
        Ok(CapturedJimLinkRoot {
            cell: std::rc::Rc::downgrade(cell),
            binding: contents.binding_id,
            epoch: contents.rmw_array_epoch,
            name: name.to_vec(),
            element,
            dictionary: self.dictionary_variables(),
            protocol: self.string_protocol.get(),
            ledger: std::rc::Rc::downgrade(&self.entry_order),
        })
    }
}
impl CapturedJimLinkRoot {
    pub(crate) fn check(
        &self,
        currency: &NativeOperationCurrency,
    ) -> Result<(), JimLinkStoreError> {
        currency
            .ensure_current_or_refuse()
            .map_err(|error| JimLinkStoreError::Command(CmdError::from_execution_refusal(error)))?;
        self.check_storage()
            .map_err(|error| JimLinkStoreError::Command(error.into()))
    }
    pub(crate) fn matches_table(&self, table: &VarTable) -> bool {
        Weak::ptr_eq(&self.ledger, &std::rc::Rc::downgrade(&table.entry_order))
            && self.ledger.strong_count() != 0
    }
    fn check_storage(&self) -> Result<(), ValueError> {
        let current = self.cell.upgrade().is_some_and(|cell| {
            let contents = cell.borrow();
            contents.binding_id == self.binding
                && contents.rmw_array_epoch == self.epoch
                && contents.rmw_retirement.is_none()
        });
        if !current {
            return Err(ValueError::CommandProtocolUnavailable(
                "captured compatibility linked-variable root",
            ));
        }
        Ok(())
    }

    pub(crate) fn check_link(
        &self,
        link: &super::Link,
        currency: &NativeOperationCurrency,
    ) -> Result<(), JimLinkStoreError> {
        self.check(currency)?;
        let valid = self.cell.upgrade().is_some_and(|cell| {
            matches!(cell.borrow().var.as_ref(), Some(Var::Link(current)) if current == link)
        });
        if valid {
            Ok(())
        } else {
            Err(JimLinkStoreError::Command(
                ValueError::CommandProtocolUnavailable("captured byte variable link").into(),
            ))
        }
    }
    fn check_owned_value_role(
        &self,
        currency: &NativeOperationCurrency,
    ) -> Result<(), JimLinkStoreError> {
        self.check(currency)?;
        let cell = self
            .cell
            .upgrade()
            .ok_or(VarError::NameProtocolUnavailable)?;
        cell.borrow().check_owned_value_role()?;
        Ok(())
    }
    fn original_scalar(
        &self,
        currency: &NativeOperationCurrency,
    ) -> Result<Option<NativeObjectLifetime>, JimLinkStoreError> {
        self.check(currency)?;
        let cell = self
            .cell
            .upgrade()
            .ok_or(VarError::NameProtocolUnavailable)?;
        let contents = cell.borrow();
        contents.check_owned_value_role()?;
        match contents.var.as_ref() {
            Some(Var::Scalar(value)) => {
                obj::check_native_liveness(*value)?;
                Ok(Some(NativeObjectLifetime::retain(*value)))
            }
            Some(Var::PendingScalar(_)) => Err(ValueError::CommandProtocolUnavailable(
                "unsettled scalar native reference role",
            )
            .into()),
            None | Some(Var::Link(_)) => Ok(None),
            Some(Var::Array(_)) => Err(VarError::IsArray.into()),
        }
    }
    fn scalar_stage_check(&self, original: *mut TclObj) -> crate::dict::NativeDictionaryStageCheck {
        let captured = self.clone();
        std::rc::Rc::new(move || {
            captured.check_storage()?;
            let valid = captured.cell.upgrade().is_some_and(|cell| {
                {
                    let contents = cell.borrow();
                    contents.check_owned_value_role().is_ok()
                        && matches!(contents.var.as_ref(), Some(Var::Scalar(value)) if *value == original)
                }
            });
            if valid {
                Ok(())
            } else {
                Err(ValueError::CommandProtocolUnavailable(
                    "captured byte dictionary header",
                ))
            }
        })
    }
    fn converted_root(
        &self,
        currency: &NativeOperationCurrency,
    ) -> Result<Option<EnteredNativeDictionary>, JimLinkStoreError> {
        let Some(original) = self.original_scalar(currency)? else {
            return Ok(None);
        };
        let protocol = self.protocol.ok_or(VarError::NameProtocolUnavailable)?;
        let reached = EnteredNativeDictionary::borrowed_converted(
            original.as_ptr(),
            protocol,
            currency,
            self.scalar_stage_check(original.as_ptr()),
        );
        self.check(currency)?;
        reached.map(Some).map_err(Into::into)
    }
    pub(crate) fn read(
        &self,
        currency: &NativeOperationCurrency,
        default: bool,
    ) -> Result<Option<*mut TclObj>, JimLinkStoreError> {
        self.check_owned_value_role(currency)?;
        let Some(key) = self.element.as_ref() else {
            return self
                .original_scalar(currency)
                .map(|value| value.map(|value| value.as_ptr()));
        };
        if self.dictionary {
            let Some(root) = self.converted_root(currency)? else {
                return Ok(None);
            };
            let reached = root.get_member_bytes(key);
            self.check(currency)?;
            let result = reached?;
            drop(root);
            self.check(currency)?;
            return Ok(result);
        }
        let cell = self
            .cell
            .upgrade()
            .ok_or(VarError::NameProtocolUnavailable)?;
        let contents = cell.borrow();
        let result = match contents.var.as_ref() {
            Some(Var::Array(values)) => values
                .get(key)
                .copied()
                .or_else(|| default.then_some(contents.array_default).flatten()),
            _ => None,
        };
        drop(contents);
        drop(cell);
        self.check(currency)?;
        Ok(result)
    }
    pub(crate) fn exists(
        &self,
        currency: &NativeOperationCurrency,
    ) -> Result<bool, JimLinkStoreError> {
        if self.element.is_some() {
            return self.read(currency, false).map(|value| value.is_some());
        }
        self.check_owned_value_role(currency)?;
        let cell = self
            .cell
            .upgrade()
            .ok_or(VarError::NameProtocolUnavailable)?;
        let result = matches!(
            cell.borrow().var.as_ref(),
            Some(Var::Scalar(_) | Var::Array(_))
        );
        drop(cell);
        self.check(currency)?;
        Ok(result)
    }
    pub(crate) fn is_array(
        &self,
        currency: &NativeOperationCurrency,
    ) -> Result<bool, JimLinkStoreError> {
        self.check_owned_value_role(currency)?;
        if self.element.is_some() {
            return Ok(false);
        }
        if self.dictionary {
            let reached = self.converted_root(currency);
            self.check(currency)?;
            let original = reached?;
            let present = original.is_some();
            drop(original);
            self.check(currency)?;
            return Ok(present);
        }
        let cell = self
            .cell
            .upgrade()
            .ok_or(VarError::NameProtocolUnavailable)?;
        let result = matches!(cell.borrow().var.as_ref(), Some(Var::Array(_)));
        drop(cell);
        self.check(currency)?;
        Ok(result)
    }
    pub(crate) fn array_names(
        &self,
        currency: &NativeOperationCurrency,
    ) -> Result<Option<Vec<Vec<u8>>>, JimLinkStoreError> {
        self.check_owned_value_role(currency)?;
        if self.element.is_some() {
            return Ok(None);
        }
        if self.dictionary {
            let Some(root) = self.converted_root(currency)? else {
                return Ok(None);
            };
            let reached = root.key_bytes();
            self.check(currency)?;
            let result = reached?;
            drop(root);
            self.check(currency)?;
            return Ok(Some(result));
        }
        let cell = self
            .cell
            .upgrade()
            .ok_or(VarError::NameProtocolUnavailable)?;
        let contents = cell.borrow();
        let result = match contents.var.as_ref() {
            Some(Var::Array(values)) => Some(match contents.member_order.keys() {
                Some(keys) => keys
                    .into_iter()
                    .filter(|key| values.contains_key(*key))
                    .map(<[u8]>::to_vec)
                    .collect(),
                None => values.keys().cloned().collect(),
            }),
            _ => None,
        };
        drop(contents);
        drop(cell);
        self.check(currency)?;
        Ok(result)
    }
    pub(crate) fn ensure_array(
        &mut self,
        currency: &NativeOperationCurrency,
    ) -> Result<(), JimLinkStoreError> {
        self.check_owned_value_role(currency)?;
        if self.element.is_some() {
            return Err(VarError::IsScalar.into());
        }
        if self.dictionary {
            if self.converted_root(currency)?.is_some() {
                return Ok(());
            }
            let protocol = self.protocol.ok_or(VarError::NameProtocolUnavailable)?;
            let root = Owned::fresh(crate::dict::new_dict_obj_native(&[], None, protocol)?);
            let reached = self.publish(root.as_ptr(), currency, true);
            drop(root);
            self.check(currency)?;
            return reached;
        }
        let cell = self
            .cell
            .upgrade()
            .ok_or(VarError::NameProtocolUnavailable)?;
        let mut contents = cell.borrow_mut();
        let retired = if contents.array_materialisation_needed(Some(&self.name))? {
            let retired = contents.replace_value(Var::Array(Default::default()));
            contents.undefined_shell = false;
            contents.undefined_root_home = None;
            contents.rmw_shell_entry = None;
            contents.binding_id.get_or_insert_with(super::fresh_var_id);
            self.binding = contents.binding_id;
            retired
        } else {
            None
        };
        drop(contents);
        drop(cell);
        let ledger = self
            .ledger
            .upgrade()
            .ok_or(VarError::NameProtocolUnavailable)?;
        ledger.borrow_mut().insert(&self.name);
        drop(ledger);
        if let Some(retired) = retired {
            retired.release();
        }
        self.check(currency)
    }

    pub(crate) fn remove_dictionary_element(
        &mut self,
        currency: &NativeOperationCurrency,
    ) -> Result<bool, JimLinkStoreError> {
        self.check(currency)?;
        let cell = self
            .cell
            .upgrade()
            .ok_or(VarError::NameProtocolUnavailable)?;
        let constant = cell.borrow().constant;
        drop(cell);
        if constant {
            return Err(VarError::IsConstant.into());
        }
        let Some(original) = self.original_scalar(currency)? else {
            return Ok(false);
        };
        let protocol = self.protocol.ok_or(VarError::NameProtocolUnavailable)?;
        let reached = EnteredNativeDictionary::compatibility_copy_after_conversion(
            original.as_ptr(),
            protocol,
            currency,
            self.scalar_stage_check(original.as_ptr()),
        );
        self.check(currency)?;
        let mut working = reached?;
        let reached =
            working.remove_member_bytes(self.element.as_deref().ok_or(VarError::IsScalar)?);
        self.check(currency)?;
        let removed = reached?;
        let reached = if removed {
            self.publish(working.as_ptr(), currency, true)
        } else {
            Ok(())
        };
        drop(working);
        drop(original);
        self.check(currency)?;
        reached?;
        Ok(removed)
    }
    pub(crate) fn is_dictionary_element(&self) -> bool {
        self.dictionary && self.element.is_some()
    }
    pub(crate) fn store(
        &mut self,
        value: *mut TclObj,
        currency: &NativeOperationCurrency,
    ) -> Result<(), JimLinkStoreError> {
        self.check(currency)?;
        obj::check_native_liveness(value)?;
        let reached = if self.dictionary && self.element.is_some() {
            self.store_dictionary_element(value, currency)
        } else {
            self.publish(value, currency, self.dictionary)
        };
        // All working dictionary/header and retired-value owners have left the
        // inner worker before the final original receipt is checked.
        self.check(currency)?;
        reached
    }

    fn store_dictionary_element(
        &mut self,
        value: *mut TclObj,
        currency: &NativeOperationCurrency,
    ) -> Result<(), JimLinkStoreError> {
        let protocol = self.protocol.ok_or_else(|| {
            JimLinkStoreError::Command(
                ValueError::CommandProtocolUnavailable("compatibility dictionary root String")
                    .into(),
            )
        })?;
        let original = {
            let cell = self
                .cell
                .upgrade()
                .ok_or(VarError::NameProtocolUnavailable)?;
            let contents = cell.borrow();
            contents.check_owned_value_role()?;
            if contents.constant {
                return Err(VarError::IsConstant.into());
            }
            match contents.var.as_ref() {
                Some(Var::Scalar(root)) => {
                    obj::check_native_liveness(*root)?;
                    Some(NativeObjectLifetime::retain(*root))
                }
                None => None,
                _ => return Err(VarError::IsScalar.into()),
            }
        };
        // The legacy Link(None) worker always copied after conversion. This is
        // compatibility behaviour, distinct from Jim_SetDictKeysVector's COW.
        let captured = self.clone();
        let stage_check: crate::dict::NativeDictionaryStageCheck =
            std::rc::Rc::new(move || captured.check_storage());
        let reached = match original.as_ref() {
            Some(root) => EnteredNativeDictionary::compatibility_copy_after_conversion(
                root.as_ptr(),
                protocol,
                currency,
                stage_check,
            ),
            None => EnteredNativeDictionary::empty(protocol, currency, stage_check),
        };
        self.check(currency)?;
        let mut working = reached?;
        let key = Owned::fresh(obj::new_string_bytes(self.element.as_deref().unwrap()));
        let reached = working.set_member(key.as_ptr(), value);
        self.check(currency)?;
        reached?;
        self.publish(working.as_ptr(), currency, true)
    }

    fn publish(
        &mut self,
        value: *mut TclObj,
        currency: &NativeOperationCurrency,
        dictionary: bool,
    ) -> Result<(), JimLinkStoreError> {
        self.check(currency)?;
        obj::check_native_liveness(value)?;
        let ledger = self.ledger.upgrade().ok_or_else(|| {
            JimLinkStoreError::Command(
                ValueError::CommandProtocolUnavailable("captured compatibility variable table")
                    .into(),
            )
        })?;
        let cell = self
            .cell
            .upgrade()
            .ok_or(VarError::NameProtocolUnavailable)?;
        let mut contents = cell.borrow_mut();
        if contents.constant {
            return Err(VarError::IsConstant.into());
        }
        if dictionary {
            let retired = contents.publish_dictionary_root(value);
            self.binding = contents.binding_id;
            drop(contents);
            drop(cell);
            ledger.borrow_mut().insert(&self.name);
            drop(ledger);
            drop(retired);
        } else if let Some(key) = self.element.as_ref() {
            match contents.var.as_ref() {
                Some(Var::Scalar(_) | Var::PendingScalar(_)) => {
                    return Err(VarError::IsScalar.into())
                }
                Some(Var::Link(link)) if link.name != self.name || link.elem.is_some() => {
                    return Err(VarError::NameProtocolUnavailable.into());
                }
                _ => {}
            }
            let retired_root = if matches!(contents.var.as_ref(), Some(Var::Array(_))) {
                None
            } else {
                contents.replace_value(Var::Array(Default::default()))
            };
            // SAFETY: this exact published member takes its owning reference.
            unsafe { obj::incr_ref_count(value) };
            let Some(Var::Array(map)) = contents.var.as_mut() else {
                unreachable!()
            };
            let retired_member = map.insert(key.clone(), value);
            contents.binding_id.get_or_insert_with(super::fresh_var_id);
            contents.undefined_shell = false;
            contents.undefined_root_home = None;
            contents.rmw_shell_entry = None;
            contents.insert_member_entry(key);
            contents.rmw_member_shells.remove(key);
            self.binding = contents.binding_id;
            drop(contents);
            drop(cell);
            ledger.borrow_mut().insert(&self.name);
            drop(ledger);
            if let Some(retired) = retired_root {
                retired.release();
            }
            if let Some(retired) = retired_member {
                unsafe { obj::decr_ref_count(retired) };
            }
        } else {
            match contents.var.as_ref() {
                Some(Var::Array(_)) => return Err(VarError::IsArray.into()),
                Some(Var::Link(link)) if link.name != self.name || link.elem.is_some() => {
                    return Err(VarError::NameProtocolUnavailable.into());
                }
                _ => {}
            }
            // SAFETY: install the new cell hold before retiring the old one.
            unsafe { obj::incr_ref_count(value) };
            let retired = contents.replace_value(Var::Scalar(value));
            contents.binding_id.get_or_insert_with(super::fresh_var_id);
            contents.undefined_shell = false;
            contents.undefined_root_home = None;
            contents.rmw_shell_entry = None;
            self.binding = contents.binding_id;
            drop(contents);
            drop(cell);
            ledger.borrow_mut().insert(&self.name);
            drop(ledger);
            if let Some(retired) = retired {
                retired.release();
            }
        }
        self.check(currency)
    }
}

#[cfg(test)]
mod tests;
