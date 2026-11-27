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

//! Physical dictionary operations over borrowed original objects, and the append and increment owners a dictionary update hands on.

use super::append_objects::RuntimeAppendObjects;
use super::original_append::OriginalRuntimeAppendObjects;
use super::{RuntimeAppendValue, RuntimeDictionaryChecks, RuntimeIncrementObjects};
use crate::interp::Interp;
use crate::obj::TclObj;
use tcl_syntax::value::ValueError;

/// Physical dictionary operations over borrowed original native objects.
pub(crate) struct RuntimeDictionaryObjects {
    pub(crate) protocol: tcl_syntax::native_string::NativeStringProtocol,
    names: tcl_syntax::naming::NamePolicyProtocol,
    preparation: tcl_cmd_core::native_dictionary::NativeDictionaryPreparation,
    pub(super) original: Option<std::rc::Rc<RuntimeDictionaryChecks>>,
}

impl RuntimeDictionaryObjects {
    pub(crate) fn selected(interp: &Interp) -> Result<Self, tcl_cmd_core::CmdError> {
        let protocol = interp.native_invocation_dialect().native_string_materialization(
            Some(tcl_native_model::native_string_materialization::LogicalStringProvider::Tcl84CoreSimulation),
        ).ok_or(ValueError::CommandProtocolUnavailable("native dictionary object protocol"))?.protocol();
        let names = interp
            .name_policy_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native dictionary diagnostic protocol",
            ))?;
        Ok(Self {
            protocol,
            names,
            preparation:
                tcl_cmd_core::native_dictionary::NativeDictionaryPreparation::CopyBeforeConversion,
            original: None,
        })
    }
    /// The caller's actual entered operation/receiver and original String
    /// projection. This adds no owning Native role or conversion purpose.
    pub(crate) fn with_original_checks(
        mut self,
        current: impl Fn() -> Result<(), tcl_cmd_core::CmdError> + 'static,
        string: impl Fn(
                *mut TclObj,
                &dyn Fn() -> Result<(), tcl_cmd_core::CmdError>,
            ) -> Result<std::rc::Rc<[u8]>, tcl_cmd_core::CmdError>
            + 'static,
        refuse: impl Fn(ValueError) -> tcl_cmd_core::CmdError + 'static,
        retain: impl Fn(tcl_cmd_core::CmdError) -> tcl_cmd_core::CmdError + 'static,
    ) -> Self {
        self.original = Some(RuntimeDictionaryChecks::new(
            current, string, refuse, retain,
        ));
        self
    }
    pub(crate) fn original_check(&self) -> Result<(), tcl_cmd_core::CmdError> {
        self.original
            .as_ref()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original Dictionary/Return owner",
            ))?
            .check()
    }
    pub(crate) fn original_refuse(&self, error: ValueError) -> tcl_cmd_core::CmdError {
        match self.original.as_ref() {
            Some(owner) => (owner.refuse)(error),
            None => error.into(),
        }
    }
    pub(crate) fn original_bytes(
        &self,
        value: *mut TclObj,
    ) -> Result<std::rc::Rc<[u8]>, tcl_cmd_core::CmdError> {
        self.original
            .as_ref()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original Dictionary/Return owner",
            ))?
            .bytes(value)
    }
    /// The same original String worker, with an independently captured
    /// return-options/private-field backing check inside reached callbacks.
    pub(crate) fn original_bytes_using(
        &self,
        value: *mut TclObj,
        extra: &dyn Fn() -> Result<(), tcl_cmd_core::CmdError>,
    ) -> Result<std::rc::Rc<[u8]>, tcl_cmd_core::CmdError> {
        self.original
            .as_ref()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original Dictionary/Return owner",
            ))?
            .bytes_using(value, extra)
    }
    pub(crate) fn original_retain(&self, error: tcl_cmd_core::CmdError) -> tcl_cmd_core::CmdError {
        match self.original.as_ref() {
            Some(owner) => (owner.retain)(error),
            None => error,
        }
    }
    pub(crate) fn original_list(
        &self,
        value: *mut TclObj,
    ) -> Result<Vec<*mut TclObj>, tcl_cmd_core::CmdError> {
        crate::list::list_elements_native_using(
            value,
            self.protocol,
            &|| self.original_check(),
            &|error| self.original_refuse(error),
        )
    }
    pub(crate) fn original_pairs(
        &self,
        value: *mut TclObj,
    ) -> Result<Vec<(*mut TclObj, *mut TclObj)>, tcl_cmd_core::CmdError> {
        crate::dict::native_dict_pairs_using(
            value,
            self.protocol,
            &|| self.original_check(),
            &|error| self.original_refuse(error),
        )
    }
    /// Reuse this genuine original dictionary/frame/check owner for the same
    /// physical append/Cat backend. No scope or Native role is manufactured.
    pub(crate) fn original_append_objects(
        &self,
        base: RuntimeAppendObjects,
    ) -> Result<OriginalRuntimeAppendObjects, tcl_cmd_core::CmdError> {
        self.original_check()?;
        Ok(OriginalRuntimeAppendObjects::new(
            base,
            self.original
                .as_ref()
                .expect("checked original dictionary owner")
                .clone(),
        ))
    }
    pub(crate) fn original_increment_objects(
        &self,
        actual: &Interp,
    ) -> Result<RuntimeIncrementObjects, tcl_cmd_core::CmdError> {
        self.original_check()?;
        let mut increment = RuntimeIncrementObjects::selected(actual)?;
        increment.context.original = Some(
            self.original
                .as_ref()
                .expect("checked original dictionary owner")
                .clone(),
        );
        increment.context.check()?;
        Ok(increment)
    }
    pub(crate) fn with_preparation(
        mut self,
        preparation: tcl_cmd_core::native_dictionary::NativeDictionaryPreparation,
    ) -> Self {
        self.preparation = preparation;
        self
    }
}

impl tcl_cmd_core::native_dictionary::NativeDictionaryObjects for RuntimeDictionaryObjects {
    type Value = RuntimeAppendValue;
    type Prepared = crate::dict::PreparedNativeDictionary;
    fn prepare(
        &self,
        original: Option<&Self::Value>,
    ) -> Result<Self::Prepared, tcl_cmd_core::CmdError> {
        use tcl_cmd_core::native_dictionary::NativeDictionaryPreparation as Preparation;
        let original = original.map(RuntimeAppendValue::as_ptr);
        if let Some(checked) = &self.original {
            return Self::Prepared::prepare_selected_using(
                original,
                self.protocol,
                self.preparation,
                &|| checked.check(),
                &|error| (checked.refuse)(error),
            );
        }
        match self.preparation {
            Preparation::CopyBeforeConversion => Self::Prepared::prepare(original, self.protocol),
            Preparation::ConvertBeforeCopy => {
                Self::Prepared::prepare_after_conversion(original, self.protocol)
            }
            Preparation::IncrementCommand => {
                Self::Prepared::prepare_for_increment(original, self.protocol)
            }
        }
        .map_err(Into::into)
    }
    fn prepare_child(
        &self,
        original: Option<&Self::Value>,
    ) -> Result<Self::Prepared, tcl_cmd_core::CmdError> {
        if let Some(checked) = &self.original {
            return Self::Prepared::prepare_selected_using(
                original.map(RuntimeAppendValue::as_ptr),
                self.protocol,
                tcl_cmd_core::native_dictionary::NativeDictionaryPreparation::ConvertBeforeCopy,
                &|| checked.check(),
                &|error| (checked.refuse)(error),
            );
        }
        crate::dict::PreparedNativeDictionary::prepare_after_conversion(
            original.map(RuntimeAppendValue::as_ptr),
            self.protocol,
        )
        .map_err(Into::into)
    }
    fn with_member<R>(
        &self,
        dictionary: &Self::Prepared,
        key: &Self::Value,
        operation: impl FnOnce(Option<&Self::Value>) -> Result<R, tcl_cmd_core::CmdError>,
    ) -> Result<R, tcl_cmd_core::CmdError> {
        if let Some(checked) = &self.original {
            let observer = dictionary
                .observe_prepared()
                .map_err(|error| (checked.refuse)(error))?;
            checked.parents.borrow_mut().push(observer);
            let reached = dictionary.with_member_using(
                key.as_ptr(),
                &|| checked.check(),
                &|error| (checked.refuse)(error),
                &|value, current| checked.bytes_using(value, current),
                |member| {
                    let member = member.map(RuntimeAppendValue::borrowed);
                    operation(member.as_ref())
                },
            );
            // Extract the nonowning guard before releasing it. No callback
            // occurs under the parent-observation RefCell borrow.
            let observer = checked.parents.borrow_mut().pop();
            drop(observer);
            checked.check()?;
            return reached;
        }
        dictionary.with_member(key.as_ptr(), |member| {
            let member = member.map(RuntimeAppendValue::borrowed);
            operation(member.as_ref())
        })?
    }
    fn set_member(
        &self,
        dictionary: &mut Self::Prepared,
        key: &Self::Value,
        value: Self::Value,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        if let Some(checked) = &self.original {
            let reached = dictionary.set_member_using(
                key.as_ptr(),
                value.as_ptr(),
                &|| checked.check(),
                &|error| (checked.refuse)(error),
                &|original, current| checked.bytes_using(original, current),
            );
            drop(value);
            checked.check()?;
            return reached;
        }
        dictionary
            .set_member(key.as_ptr(), value.as_ptr())
            .map_err(Into::into)
    }
    fn remove_member(
        &self,
        dictionary: &mut Self::Prepared,
        key: &Self::Value,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        if let Some(checked) = &self.original {
            return dictionary
                .remove_member_using(
                    key.as_ptr(),
                    &|| checked.check(),
                    &|error| (checked.refuse)(error),
                    &|original, current| checked.bytes_using(original, current),
                )
                .map(|_| ());
        }
        dictionary
            .remove_member(key.as_ptr())
            .map(|_| ())
            .map_err(Into::into)
    }
    fn missing_key(&self, key: &Self::Value) -> tcl_cmd_core::CmdError {
        let bytes = if let Some(checked) = &self.original {
            checked
                .check()
                .and_then(|()| checked.bytes(key.as_ptr()))
                .map(|bytes| bytes.to_vec())
        } else {
            crate::dict::native_object_bytes(key.as_ptr(), self.protocol).map_err(Into::into)
        };
        let original = match bytes {
            Ok(bytes) => bytes,
            Err(first) => return first,
        };
        match tcl_syntax::naming::report_native_dictionary_missing_key(
            self.names.recipe(),
            tcl_syntax::naming::NativeDictionaryMissingKeyOperation::UnsetIntermediate,
            &original,
        ) {
            Ok(report) => match report.error_code {
                Some(code) => tcl_cmd_core::CmdError::with_error_code_bytes(report.message, code),
                None => tcl_cmd_core::CmdError::new_bytes(report.message),
            },
            Err(_) => {
                ValueError::CommandProtocolUnavailable("native dictionary missing-key diagnostic")
                    .into()
            }
        }
    }
    fn finish(&self, dictionary: Self::Prepared) -> Self::Value {
        let owned = dictionary.into_value();
        RuntimeAppendValue::retain(owned.as_ptr())
    }
}
