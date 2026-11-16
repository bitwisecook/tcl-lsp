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

//! Reading and storing a variable through its original C name: scalars, array members, property and regex variables, and the C 8.4 global error objects.

use super::super::{HashSet, Value, Vm};
use crate::command::completion_from_cmd_error;
use tcl_runtime_api::Completion;
use tcl_syntax::native_variable_name::NativeVariableNameLookupPurpose;

impl Vm {
    /// Tcl 8.4's `Tcl_SetObjErrorCode` uses a bare `CString` with `GLOBAL_ONLY`,
    /// without an object-name getter or `TCL_LEAVE_ERR_MSG`.
    pub(in super::super) fn store_original_c84_global_error_code(
        &mut self,
        value: Value,
    ) -> Result<(), Completion<Value>> {
        if !self.uses_c84_global_error_info()
            || self
                .native_c_variable_name_protocol()
                .is_none_or(|protocol| protocol.version() != tcl_dialect::TclVersion::V8_4)
        {
            return Err(self.refuse_host_command(
                "actual C8.4 CString global error-code setter unavailable".into(),
            ));
        }
        let original_names =
            super::super::native_name_consumers::OriginalNameConsumerScope::enter_for_namespace(
                self,
                Some(tcl_core_types::ROOT_NS),
            )
            .map_err(|error| completion_from_cmd_error(self, error))?;
        let reached = (|| {
            original_names
                .current_for_value(self, &value)
                .map_err(|error| completion_from_cmd_error(self, error))?;
            let name = tcl_registry::special_vars::NativeBootstrapVariable::ErrorCode
                .name()
                .as_bytes();
            let binding = self
                .namespace_var_binding_from_token_bytes(tcl_core_types::ROOT_NS, name)
                .ok_or_else(|| {
                    self.refuse_host_command("actual C8.4 global error-code binding".into())
                })?;
            let resolved = self
                .resolve_binding_var(binding, None, &mut HashSet::new())
                .ok_or_else(|| {
                    self.refuse_host_command("actual C8.4 global error-code receiver".into())
                })?;
            let purpose = NativeVariableNameLookupPurpose::QuietWrite;
            let captured = self.capture_selected_update_with_errors(
                name,
                None,
                &resolved,
                purpose.leaves_error_message(),
            )?;
            self.with_variable_operation(&captured.cell, |vm| {
                vm.store_captured_update_with_original_and_errors(
                    name,
                    None,
                    &captured,
                    value,
                    None,
                    purpose.leaves_error_message(),
                )
                .map(|_| ())
            })
        })();
        original_names
            .finish(self)
            .map_err(|error| completion_from_cmd_error(self, error))?;
        reached
    }

    pub(in super::super) fn read_original_c_variable(
        &mut self,
        original: &Value,
    ) -> Result<Value, Completion<Value>> {
        let (_root, bytes, element, resolved) =
            self.prepare_native_original_variable(original, NativeVariableNameLookupPurpose::Read)?;
        self.read_selected_variable_result_bytes_with_original(
            &bytes,
            element.as_deref(),
            resolved,
            Some(original),
        )
    }

    /// Native object access separates the fresh qualified lookup object from
    /// the retained clientData object used for `PtrGetVar` diagnostics and traces.
    pub(crate) fn read_original_c_property_variable(
        &mut self,
        lookup: &Value,
        reported: &Value,
    ) -> Result<Value, Completion<Value>> {
        let original_names =
            crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(self)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        let reached = (|| {
            let (_root, _bytes, element, resolved) = self
                .prepare_native_original_variable(lookup, NativeVariableNameLookupPurpose::Link)?;
            let bytes = original_names
                .read(self, reported)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
            self.read_selected_variable_result_bytes_with_original(
                &bytes,
                element.as_deref(),
                resolved,
                Some(reported),
            )
        })();
        original_names
            .finish(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        reached
    }

    /// Native `PropertySetter` uses the selected cell without publishing its value
    /// as the interpreter result; the enclosing configure operation resets it.
    pub(crate) fn store_original_c_property_variable(
        &mut self,
        lookup: &Value,
        reported: &Value,
        value: Value,
    ) -> Result<(), Completion<Value>> {
        let original_names =
            crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(self)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        let reached = (|| {
            let (_root, _bytes, element, resolved) = self
                .prepare_native_original_variable(lookup, NativeVariableNameLookupPurpose::Write)?;
            let bytes = original_names
                .read(self, reported)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
            let captured = self.capture_selected_update_with_errors(
                &bytes,
                element.as_deref(),
                &resolved,
                true,
            )?;
            self.with_variable_operation(&captured.cell, |vm| {
                vm.store_captured_update_with_original_and_errors(
                    &bytes,
                    element.as_deref(),
                    &captured,
                    value,
                    Some(reported),
                    true,
                )
                .map(|_| ())
            })
        })();
        original_names
            .finish(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        reached
    }

    pub(crate) fn store_original_regex_variable(
        &mut self,
        original: &Value,
        value: Value,
    ) -> Result<Value, Completion<Value>> {
        let original_names =
            crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(self)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        let reached = (|| {
            let Some(protocol) = self.native_c_variable_name_protocol() else {
                return self.store_original_named_variable(original, value);
            };
            let purpose = protocol.regex_write_purpose();
            match self.store_original_c_variable_for(original, value, purpose) {
                Ok(value) => Ok(value),
                Err(failure)
                    if !purpose.leaves_error_message() && self.refused_completion().is_none() =>
                {
                    let bytes = original_names
                        .read(self, original)
                        .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
                    let current = self
                        .with_native_interp_result(|value| {
                            value.native_lifetime_lease().into_value()
                        })
                        .map_err(|error| {
                            crate::command::completion_from_cmd_error(self, error.into())
                        })?;
                    let mut message = original_names
                        .read(self, &current)
                        .map_err(|error| crate::command::completion_from_cmd_error(self, error))?
                        .to_vec();
                    message.extend_from_slice(b"couldn't set variable \"");
                    message.extend_from_slice(tcl_core_types::c_string_extent(&bytes));
                    message.push(b'"');
                    let mut failure = failure;
                    failure.result = Value::from_string_bytes(message);
                    if protocol.diagnostic_string_protocol().is_some() {
                        let recipe = self
                            .actual_native_invocation_dialect()
                            .native_string_materialization(None)
                            .ok_or_else(|| {
                                self.refuse_host_command(
                                    "actual variable-error String producer".into(),
                                )
                            })?;
                        failure
                            .result
                            .retain_native_string_representation(recipe)
                            .map_err(|error| {
                                crate::command::completion_from_cmd_error(self, error.into())
                            })?;
                    }
                    Err(failure)
                }
                Err(failure) => Err(failure),
            }
        })();
        original_names
            .finish(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        reached
    }

    pub(in super::super) fn store_original_c_variable(
        &mut self,
        original: &Value,
        value: Value,
    ) -> Result<Value, Completion<Value>> {
        self.store_original_c_variable_for(original, value, NativeVariableNameLookupPurpose::Write)
    }

    /// The array worker supplies two actual operands, independently of combined
    /// name parsing. The existing lookup and captured write own every effect.
    pub(crate) fn store_original_c_array_member(
        &mut self,
        original: &Value,
        key: &Value,
        value: &Value,
    ) -> Result<Value, Completion<Value>> {
        let original_names =
            crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(self)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        let reached = (|| {
            use super::super::native_operation_currency::NativeOperationScope;
            use crate::command::completion_from_cmd_error;
            let scope = NativeOperationScope::enter(self)
                .map_err(|error| completion_from_cmd_error(self, error))?;
            let root_lifetime = original.native_lifetime_lease();
            let key_lifetime = key.native_lifetime_lease();
            let result = (|| {
                if self.native_c_variable_name_protocol().is_none() {
                    return Err(completion_from_cmd_error(
                        self,
                        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "original C array variable name owner",
                        )
                        .into(),
                    ));
                }
                original
                    .check_native_header()
                    .map_err(|error| completion_from_cmd_error(self, error.into()))?;
                key.check_native_header()
                    .map_err(|error| completion_from_cmd_error(self, error.into()))?;
                value
                    .check_native_header()
                    .map_err(|error| completion_from_cmd_error(self, error.into()))?;
                let reached = original_names.read(self, key);
                scope
                    .check(self)
                    .map_err(|error| completion_from_cmd_error(self, error))?;
                let key_bytes = reached.map_err(|error| completion_from_cmd_error(self, error))?;
                let reached = self.prepare_native_original_variable_parts(
                    original,
                    NativeVariableNameLookupPurpose::Write,
                    None,
                    Some(&key_bytes),
                );
                scope
                    .check(self)
                    .map_err(|error| completion_from_cmd_error(self, error))?;
                let (_root, bytes, element, resolved) = reached?;
                let element_birth = resolved.elem.is_some() && resolved.id.is_none();
                let captured =
                    self.capture_selected_update(&bytes, element.as_deref(), &resolved)?;
                if element_birth
                    && self
                        .native_c_variable_name_protocol()
                        .is_some_and(tcl_syntax::native_variable_name::NativeVariableNameProtocol::element_table_retains_original)
                    && let (Some(array), Some(member)) =
                        (captured.cell.array, captured.cell.elem.as_ref())
                    && !self
                        .var_arena
                        .retain_original_array_key(array, member, key.clone())
                {
                    return Err(completion_from_cmd_error(
                        self,
                        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "original array member key publication",
                        )
                        .into(),
                    ));
                }
                scope
                    .check(self)
                    .map_err(|error| completion_from_cmd_error(self, error))?;
                value
                    .check_native_header()
                    .map_err(|error| completion_from_cmd_error(self, error.into()))?;
                self.with_variable_operation(&captured.cell, |vm| {
                    vm.store_captured_update_with_original(
                        &bytes,
                        element.as_deref(),
                        &captured,
                        value.native_lifetime_lease().into_value(),
                        Some(original),
                    )
                })
            })();
            drop(key_lifetime);
            drop(root_lifetime);
            scope
                .check(self)
                .map_err(|error| completion_from_cmd_error(self, error))?;
            result
        })();
        original_names
            .finish(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        reached
    }

    fn store_original_c_variable_for(
        &mut self,
        original: &Value,
        value: Value,
        purpose: NativeVariableNameLookupPurpose,
    ) -> Result<Value, Completion<Value>> {
        self.store_original_c_variable_in(original, value, purpose, None)
    }

    /// C8.4 `ObjSetVar2` keeps its original header and `GLOBAL_ONLY`/quiet purpose.
    pub(in super::super) fn store_original_c84_global_error_object(
        &mut self,
        original: &Value,
        value: Value,
    ) -> Result<Value, Completion<Value>> {
        if !self.uses_c84_global_error_info()
            || self
                .native_c_variable_name_protocol()
                .is_none_or(|protocol| protocol.version() != tcl_dialect::TclVersion::V8_4)
        {
            return Err(self.refuse_host_command("actual C8.4 original error-name setter".into()));
        }
        self.store_original_c_variable_in(
            original,
            value,
            NativeVariableNameLookupPurpose::QuietWrite,
            Some(tcl_core_types::ROOT_NS),
        )
    }

    /// Prepare the held object name before selecting the actual append cell.
    pub(in super::super) fn append_original_c84_global_error_object(
        &mut self,
        original: &Value,
        source: Value,
    ) -> Result<Value, Completion<Value>> {
        if !self.uses_c84_global_error_info()
            || self
                .native_c_variable_name_protocol()
                .is_none_or(|protocol| protocol.version() != tcl_dialect::TclVersion::V8_4)
        {
            return Err(self.refuse_host_command("actual C8.4 original error-name append".into()));
        }
        let (_root, bytes, element, resolved) = self.prepare_native_original_variable_in(
            original,
            NativeVariableNameLookupPurpose::QuietWrite,
            Some(tcl_core_types::ROOT_NS),
        )?;
        let reached = self.append_original_c84_selected_variable(
            original,
            &bytes,
            element.as_deref(),
            &source,
            resolved,
        );
        drop(source);
        reached
    }

    fn store_original_c_variable_in(
        &mut self,
        original: &Value,
        value: Value,
        purpose: NativeVariableNameLookupPurpose,
        namespace: Option<tcl_core_types::NsId>,
    ) -> Result<Value, Completion<Value>> {
        let (_root, bytes, element, resolved) =
            self.prepare_native_original_variable_in(original, purpose, namespace)?;
        let element_birth = resolved.elem.is_some() && resolved.id.is_none();
        let captured = self.capture_selected_update_with_errors(
            &bytes,
            element.as_deref(),
            &resolved,
            purpose.leaves_error_message(),
        )?;
        if element_birth
            && let (Some(array), Some(key)) = (captured.cell.array, captured.cell.elem.as_ref())
            && self
                .native_c_variable_name_protocol()
                .is_some_and(|protocol| protocol.version() >= tcl_dialect::TclVersion::V8_5)
        {
            let key_object = Self::original_c_parsed_element_key(original);
            if let Some(key_object) = key_object {
                self.var_arena
                    .retain_original_array_key(array, key, key_object);
            }
        }
        self.with_variable_operation(&captured.cell, |vm| {
            vm.store_captured_update_with_original_and_errors(
                &bytes,
                element.as_deref(),
                &captured,
                value,
                Some(original),
                purpose.leaves_error_message(),
            )
        })
    }
}
