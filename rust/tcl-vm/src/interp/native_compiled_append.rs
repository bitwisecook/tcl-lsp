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

//! Selected append instructions retain their actual headers and receiver cells.

use super::{CapturedVariableUpdate, Local, Value, Vm};
use tcl_runtime_api::{Completion, VariableUpdateResult};
use tcl_syntax::native_string::NativeStringProtocol;

impl Vm {
    pub(crate) fn lappend_instruction_single_bytes(
        &mut self,
        name: &[u8],
        key: Option<&[u8]>,
        addition: &Value,
    ) -> Result<Value, Completion<Value>> {
        let captured = self.capture_update_cell(name, key)?;
        self.lappend_instruction_single_captured(name, key, &captured, addition)
    }

    pub(super) fn lappend_instruction_single_captured(
        &mut self,
        name: &[u8],
        key: Option<&[u8]>,
        captured: &CapturedVariableUpdate,
        addition: &Value,
    ) -> Result<Value, Completion<Value>> {
        let protocol = crate::cmd_dict::VmDictionaryObjects::selected(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?
            .string_protocol();
        self.with_variable_operation(&captured.cell, |vm| {
            vm.check_captured_update_with_errors(name, key, captured, true)?;
            let id = captured
                .cell
                .id
                .expect("selected single-element append cell");
            let selected = vm.initial_value_at_cell(id, captured.explicit_array);
            let value = selected.map_or_else(
                || Value::new_native_string_bytes(&b""[..]),
                |value| {
                    if value.native_object_is_shared() {
                        value.duplicate_native_object_in(protocol)
                    } else {
                        value.clone()
                    }
                },
            );
            // Single-element TclPtrSetVar installs the chosen header before
            // List conversion, including a duplicate left after parse failure.
            let _ = vm.var_arena.replace_state(id, Local::Scalar(value));
            let result = match vm.var_arena.get(id).expect("retained append cell").state() {
                Local::Scalar(value) => value
                    .native_list_append_prepared_elements(std::slice::from_ref(addition), protocol),
                _ => unreachable!("installed append header"),
            };
            result.map_err(|error| crate::command::completion_from_cmd_error(vm, error.into()))?;
            let value = vm.read_resolved_cell(id).expect("installed append value");
            vm.store_captured_update(name, key, captured, value)
        })
    }

    pub(crate) fn lappend_instruction_list_bytes(
        &mut self,
        name: &[u8],
        key: Option<&[u8]>,
        input: &Value,
    ) -> Result<VariableUpdateResult<Value>, Completion<Value>> {
        let protocol = crate::cmd_dict::VmDictionaryObjects::selected(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?
            .string_protocol();
        let members = self
            .native_object_list_elements_in(input, protocol)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error.into()))?;
        let captured = self.capture_update_cell(name, key)?;
        self.lappend_instruction_list_captured(name, key, &captured, input, &members, protocol)
    }

    pub(crate) fn lappend_instruction_compiled_list(
        &mut self,
        slot: usize,
        key: Option<&[u8]>,
        input: &Value,
    ) -> Result<VariableUpdateResult<Value>, Completion<Value>> {
        let protocol = crate::cmd_dict::VmDictionaryObjects::selected(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?
            .string_protocol();
        // GetElements belongs to the input before selected receiver lookup.
        let members = self
            .native_object_list_elements_in(input, protocol)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error.into()))?;
        let (name, captured) = self.capture_compiled_update(slot, key)?;
        self.lappend_instruction_list_captured(
            name.as_bytes(),
            key,
            &captured,
            input,
            &members,
            protocol,
        )
    }

    fn lappend_instruction_list_captured(
        &mut self,
        name: &[u8],
        key: Option<&[u8]>,
        captured: &CapturedVariableUpdate,
        input: &Value,
        members: &[Value],
        protocol: NativeStringProtocol,
    ) -> Result<VariableUpdateResult<Value>, Completion<Value>> {
        self.with_variable_operation(&captured.cell, |vm| {
            let trace = if let Some(key) = key {
                vm.variable_trace_guest_or_unwind(|vm| {
                    vm.fire_elem_traces_from_cell_bytes(
                        name,
                        key,
                        "read",
                        Some(captured.cell.clone()),
                    )
                })
            } else {
                vm.variable_trace_guest_or_unwind(|vm| {
                    vm.fire_var_traces_from_cell_bytes(
                        name,
                        "read",
                        None,
                        None,
                        Some(captured.cell.clone()),
                    )
                })
            };
            if let Some(refusal) = vm.refused_completion() {
                return Err(refusal);
            }
            let id = captured.cell.id.expect("selected append-list cell");
            let (present, options) = match trace {
                Ok(()) => {
                    let present = vm
                        .initial_value_at_cell(id, captured.explicit_array)
                        .is_some();
                    let options = if present {
                        Value::empty()
                    } else {
                        vm.captured_read_missing_options(name, key, &captured.cell)
                    };
                    (present, options)
                }
                Err(error) => {
                    let options = vm
                        .completion_options_snapshot(&error)
                        .map_err(|error| crate::command::completion_from_tcl_error(vm, error))?;
                    vm.publish_swallowed_trace_error();
                    (false, options)
                }
            };
            vm.retain_variable_read_error_code(&options)?;
            let empty =
                tcl_registry::native_instruction_plan::native_list_append_empty_publication(
                    protocol,
                )
                .ok_or_else(|| {
                    crate::command::completion_from_cmd_error(
                        vm,
                        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "native List append instruction",
                        )
                        .into(),
                    )
                })?;
            let next = if present {
                let original = vm
                    .initial_value_at_cell(id, captured.explicit_array)
                    .expect("selected receiver before further callbacks");
                if members.is_empty() && empty.skip_existing_store {
                    let value = match original.native_object_list_elements(protocol) {
                        Ok(_) => original.clone(),
                        Err(error) => {
                            return Err(crate::command::completion_from_cmd_error(
                                vm,
                                error.into(),
                            ));
                        }
                    };
                    return vm.variable_update_result(value, &options);
                }
                let next = original.native_list_append_list_elements(members, protocol);
                next.map_err(|error| crate::command::completion_from_cmd_error(vm, error.into()))?
            } else {
                // Native missing receiver adopts this same prepared input.
                if members.is_empty() && empty.fresh_missing_receiver {
                    Value::new_native_string_bytes(&b""[..])
                } else {
                    input.clone()
                }
            };
            let value = vm.store_captured_update(name, key, captured, next)?;
            vm.variable_update_result(value, &options)
        })
    }
}
