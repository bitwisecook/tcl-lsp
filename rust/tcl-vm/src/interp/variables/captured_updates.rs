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

//! Read-modify-write through a captured cell: `append` and `lappend`, capturing the cell before its traces, and storing the update with its errors.

use super::super::{
    CapturedVariableUpdate, ResolvedVar, VarTraceCell, VarTraceInvocation, Vm, err,
    native_operation_currency, variable_cleanup,
};
use crate::value::Value;
use crate::vars::VarState as Local;
use std::collections::HashSet;
use tcl_runtime_api::{Completion, ROOT_NS};
use tcl_syntax::naming::NativeVariableFailureSite::ValueWrite;

impl Vm {
    pub(crate) fn append_variable_bytes(
        &mut self,
        name: &[u8],
        key: Option<&[u8]>,
        value: Value,
    ) -> Result<Value, Completion<Value>> {
        self.append_captured_bytes(name, key, &[value])
    }

    /// The compiled list-update protocol captures its cell before the read
    /// callback. An empty append validates and returns an existing value without
    /// storing it; a missing value is initialised in that same captured cell.
    pub(crate) fn lappend_list_update_bytes(
        &mut self,
        name: &[u8],
        key: Option<&[u8]>,
        additions: &[Value],
    ) -> Result<tcl_runtime_api::VariableUpdateResult<Value>, Completion<Value>> {
        self.lappend_selected_bytes(name, key, additions, None)
    }

    fn lappend_selected_bytes(
        &mut self,
        name: &[u8],
        key: Option<&[u8]>,
        additions: &[Value],
        selected: Option<ResolvedVar>,
    ) -> Result<tcl_runtime_api::VariableUpdateResult<Value>, Completion<Value>> {
        if self.dictionary_variable_containers() {
            if self.limits.confined_stores && !additions.is_empty() {
                self.confine_store_bytes(name, self.current_level(), false)?;
            }
            let current = if let Some(key) = key {
                self.get_array_elem_bytes_from(self.current_level(), name, key)
            } else {
                self.get_var_bytes(name)
            };
            let read_only = current.is_some() && additions.is_empty();
            if !read_only {
                self.confine_store_bytes(name, self.current_level(), false)?;
            }
            let value = self.lappend_list_value(current, additions)?;
            if read_only {
                return self.variable_update_result(value, &Value::empty());
            }
            let value = if let Some(key) = key {
                self.store_elem_result_bytes(name, key, value)?
            } else {
                self.store_var_result_bytes(name, value)?
            };
            return self.variable_update_result(value, &Value::empty());
        }
        let captured = if self.limits.confined_stores && additions.is_empty() {
            let resolved = selected
                .or_else(|| {
                    if key.is_some() {
                        self.resolve_var_parts_from_bytes(name, key, self.current_level())
                    } else {
                        self.resolve_var_from_bytes(name, self.current_level())
                    }
                })
                .ok_or_else(|| {
                    self.variable_access_error_bytes("set", name, "parent namespace doesn't exist")
                })?;
            if resolved
                .id
                .and_then(|id| self.var_arena.get(id))
                .is_some_and(|cell| matches!(cell.state(), Local::Scalar(_)))
            {
                self.capture_existing_readonly_update(name, key, &resolved)?
            } else {
                self.capture_selected_update(name, key, &resolved)?
            }
        } else {
            match selected {
                Some(resolved) => self.capture_selected_update(name, key, &resolved)?,
                None => self.capture_update_cell(name, key)?,
            }
        };
        self.with_variable_operation(&captured.cell, |vm| {
            vm.lappend_captured_cell(name, key, &captured, additions)
        })
    }

    pub(in super::super) fn with_variable_operation<
        R: variable_cleanup::VariableOperationOutcome,
    >(
        &mut self,
        cell: &VarTraceCell,
        operation: impl FnOnce(&mut Self) -> R,
    ) -> R {
        let original = variable_cleanup::OriginalVariableCleanup::enter(self);
        if let Err(error) = original.check(self) {
            return R::refused(self, error);
        }
        let mut retained: Vec<_> = cell.id.into_iter().collect();
        if let Some(array) = cell.array_id()
            && self.var_arena.get(array).is_some()
        {
            retained.push(array);
        }
        let bindings: Vec<_> = retained
            .iter()
            .map(|id| self.array_operation_binding(*id))
            .collect();
        for &id in &retained {
            self.var_arena.retain_operation(id);
        }
        let result = operation(self);
        let _ = original.check(self);
        for (id, binding) in retained.into_iter().zip(bindings) {
            if let Some(binding) = binding {
                self.release_array_operation_binding(&binding, id, &original);
            }
            let _ = original.retire(self, crate::vars::VariableRetirement::release_operation(id));
        }
        match original.check(self) {
            Ok(()) => result,
            Err(error) => {
                let _ = original.in_original(self, |_| drop(result));
                R::refused(self, error)
            }
        }
    }

    pub(in super::super) fn capture_update_cell(
        &mut self,
        name: &[u8],
        key: Option<&[u8]>,
    ) -> Result<CapturedVariableUpdate, Completion<Value>> {
        use tcl_syntax::naming::NativeVariableInputForm as Input;
        let input = key.map_or(Input::Combined(name), |element| Input::Separate {
            root: name,
            element: Some(element),
        });
        let level = self.current_level();
        let resolved = if self.observed_names.is_some() {
            self.confine_store_bytes(name, level, false)?;
            if key.is_some() {
                self.resolve_var_parts_from_bytes(name, key, level)
            } else {
                self.resolve_var_from_bytes(name, level)
            }
        } else {
            let selected = if key.is_some() {
                self.select_var_parts_from_bytes(name, key, level)
            } else {
                self.select_var_from_bytes(name, level)
            };
            match selected {
                Some((binding, element)) => {
                    self.confine_store_binding_bytes(name, &binding, level, false)?;
                    self.resolve_binding_var(binding, element, &mut HashSet::new())
                }
                None => None,
            }
        }
        .ok_or_else(|| {
            self.variable_access_error_input("set", input, "parent namespace doesn't exist")
        })?;
        self.capture_selected_update(name, key, &resolved)
    }

    pub(in super::super) fn capture_selected_update(
        &mut self,
        name: &[u8],
        key: Option<&[u8]>,
        resolved: &ResolvedVar,
    ) -> Result<CapturedVariableUpdate, Completion<Value>> {
        self.capture_selected_update_with_errors(name, key, resolved, true)
    }

    pub(in super::super) fn capture_selected_update_with_errors(
        &mut self,
        name: &[u8],
        key: Option<&[u8]>,
        resolved: &ResolvedVar,
        leave_error_message: bool,
    ) -> Result<CapturedVariableUpdate, Completion<Value>> {
        self.confine_store_binding_bytes(name, &resolved.binding, self.current_level(), false)?;
        self.capture_update_cell_unchecked(name, key, resolved, leave_error_message)
    }

    fn capture_existing_readonly_update(
        &mut self,
        name: &[u8],
        key: Option<&[u8]>,
        resolved: &ResolvedVar,
    ) -> Result<CapturedVariableUpdate, Completion<Value>> {
        // Read-only capture cannot create a missing cell.
        if !resolved
            .id
            .and_then(|id| self.var_arena.get(id))
            .is_some_and(|cell| matches!(cell.state(), Local::Scalar(_)))
        {
            return Err(
                self.refuse_host_command("read-only captured variable is unavailable".into())
            );
        }
        self.capture_update_cell_unchecked(name, key, resolved, true)
    }

    fn capture_update_cell_unchecked(
        &mut self,
        name: &[u8],
        key: Option<&[u8]>,
        resolved: &ResolvedVar,
        leave_error_message: bool,
    ) -> Result<CapturedVariableUpdate, Completion<Value>> {
        use tcl_syntax::naming::NativeVariableInputForm as Input;
        let input = key.map_or(Input::Combined(name), |element| Input::Separate {
            root: name,
            element: Some(element),
        });
        let id = resolved
            .id
            .or_else(|| {
                self.ensure_target_var_at_binding(&resolved.binding, resolved.elem.as_deref())
            })
            .ok_or_else(|| {
                if leave_error_message {
                    self.variable_access_error_input("set", input, "variable isn't array")
                } else {
                    err("")
                }
            })?;
        let parent = self
            .var_arena
            .element_parent(id)
            .map(|(array, key)| (array, key.to_owned()));
        let owner_id = parent.as_ref().map_or(id, |(array, _)| *array);
        let namespace = self
            .var_arena
            .get(owner_id)
            .and_then(crate::vars::VarCell::namespace_owner);
        Ok(CapturedVariableUpdate {
            binding: resolved.binding.clone(),
            explicit_array: resolved
                .elem
                .is_some()
                .then(|| parent.as_ref().map(|(array, _)| *array))
                .flatten(),
            cell: VarTraceCell {
                id: Some(id),
                array: parent.as_ref().map(|(array, _)| *array),
                elem: parent.map(|(_, key)| key),
            },
            namespace,
        })
    }

    fn lappend_list_value(
        &mut self,
        current: Option<Value>,
        additions: &[Value],
    ) -> Result<Value, Completion<Value>> {
        if additions.is_empty()
            && let Some(current) = current.as_ref()
        {
            match tcl_registry::native_compilation::NativeAppendKind::List
                .validates_empty_result(self.native_invocation_dialect())
            {
                Some(true) => {
                    current
                        .as_list()
                        .map_err(|error| crate::command::completion_from_tcl_error(self, error))?;
                }
                Some(false) => {}
                None => {
                    return Err(self.refuse_host_command(
                        "empty list-append validation policy is not selected".into(),
                    ));
                }
            }
            return Ok(current.clone());
        }
        tcl_cmd_core::var::lappend_value(self, current, additions)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))
    }

    pub(crate) fn retain_variable_read_error_code(
        &mut self,
        options: &Value,
    ) -> Result<(), Completion<Value>> {
        if let Some(code) = crate::command::opt_get_checked(self, options, b"-errorcode")
            .map_err(|error| crate::command::completion_from_tcl_error(self, error))?
        {
            let bytes = tcl_syntax::value::ValueOps::native_string_bytes(self, &code)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error.into()))?;
            self.apply_primitive_error_code(tcl_cmd_core::ResolvedCmdErrorCodeUpdate::Set(
                bytes.to_vec(),
            ));
        }
        Ok(())
    }

    pub(crate) fn variable_update_result(
        &mut self,
        value: Value,
        read_options: &Value,
    ) -> Result<tcl_runtime_api::VariableUpdateResult<Value>, Completion<Value>> {
        let mut rows = Vec::new();
        for (key, value) in crate::command::completion_option_rows_checked(self, read_options)
            .map_err(|error| crate::command::completion_from_tcl_error(self, error))?
        {
            let bytes = tcl_syntax::value::ValueOps::native_string_bytes(self, &key)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error.into()))?;
            rows.push((bytes.to_vec(), value));
        }
        let rows = tcl_runtime_api::completion_options::retained_failed_read_options(rows);
        let options = Value::list(
            rows.into_iter()
                .flat_map(|(key, value)| [Value::from_string_bytes(key), value])
                .collect(),
        );
        Ok(tcl_runtime_api::VariableUpdateResult { value, options })
    }

    pub(super) fn captured_read_missing(
        &mut self,
        name: &[u8],
        key: Option<&[u8]>,
        cell: &VarTraceCell,
    ) -> (Completion<Value>, bool) {
        use tcl_syntax::naming::{
            NativeVariableFailureSite::ValueRead, NativeVariableInputForm as Input,
        };
        let input = key.map_or(Input::Combined(name), |element| Input::Separate {
            root: name,
            element: Some(element),
        });
        let reason = if cell
            .id
            .and_then(|id| self.var_arena.get(id))
            .is_some_and(|cell| matches!(cell.state(), Local::Array(_)))
        {
            "variable is array"
        } else if key.is_some() || cell.elem.is_some() {
            "no such element in array"
        } else {
            "no such variable"
        };
        self.variable_access_diagnostic_at("read", input, reason, ValueRead)
    }

    /// Options a swallowed missing read carries onto the update's successful
    /// completion. A release whose diagnostic sets no error code leaves the
    /// interpreter's as it was (tclsh 8.5 `catch {lappend missing x} m o`
    /// reports no `-errorcode`).
    pub(in super::super) fn captured_read_missing_options(
        &mut self,
        name: &[u8],
        key: Option<&[u8]>,
        cell: &VarTraceCell,
    ) -> Value {
        let (completion, sets_code) = self.captured_read_missing(name, key, cell);
        if sets_code {
            completion.options
        } else {
            Value::empty()
        }
    }

    fn lappend_captured_cell(
        &mut self,
        name: &[u8],
        key: Option<&[u8]>,
        captured: &CapturedVariableUpdate,
        additions: &[Value],
    ) -> Result<tcl_runtime_api::VariableUpdateResult<Value>, Completion<Value>> {
        let cell = &captured.cell;
        let trace = if let Some(key) = key {
            self.variable_trace_guest_or_unwind(|vm| {
                vm.fire_elem_traces_from_cell_bytes(name, key, "read", Some(cell.clone()))
            })
        } else {
            self.variable_trace_guest_or_unwind(|vm| {
                vm.fire_var_traces_from_cell_bytes(name, "read", None, None, Some(cell.clone()))
            })
        };
        if let Some(refusal) = self.refused_completion() {
            return Err(refusal);
        }
        let id = cell.id.expect("captured update cell");
        let (current, read_options) = match trace {
            Ok(()) => {
                let current = self
                    .initial_value_at_cell(id, captured.explicit_array)
                    .cloned();
                let options = if current.is_none() {
                    self.captured_read_missing_options(name, key, cell)
                } else {
                    Value::empty()
                };
                (current, options)
            }
            Err(error) => {
                let options = self
                    .completion_options_snapshot(&error)
                    .map_err(|error| crate::command::completion_from_tcl_error(self, error))?;
                self.publish_swallowed_trace_error();
                (None, options)
            }
        };
        if let Some(refusal) = self.refused_completion() {
            return Err(refusal);
        }
        self.retain_variable_read_error_code(&read_options)?;
        let read_only = current.is_some() && additions.is_empty();
        let value = self.lappend_list_value(current, additions)?;
        if read_only {
            return self.variable_update_result(value, &read_options);
        }
        let stored = self.store_captured_update(name, key, captured, value)?;
        self.variable_update_result(stored, &read_options)
    }

    pub(in super::super) fn check_captured_update_with_errors(
        &mut self,
        name: &[u8],
        key: Option<&[u8]>,
        captured: &CapturedVariableUpdate,
        leave_error_message: bool,
    ) -> Result<(), Completion<Value>> {
        let cell = &captured.cell;
        if let Some(refused) = self.refused_completion() {
            return Err(refused);
        }
        let id = cell.id.expect("captured update cell");
        let input = key.map_or(
            tcl_syntax::naming::NativeVariableInputForm::Combined(name),
            |element| tcl_syntax::naming::NativeVariableInputForm::Separate {
                root: name,
                element: Some(element),
            },
        );
        if !self.var_arena.element_is_attached(id) {
            if !leave_error_message {
                return Err(err(""));
            }
            return Err(self.variable_access_error_at(
                "set",
                input,
                "upvar refers to element in deleted array",
                ValueWrite,
            ));
        }
        if captured
            .namespace
            .is_some_and(|namespace| namespace != ROOT_NS && !self.ns_vars.contains_key(&namespace))
        {
            if !leave_error_message {
                return Err(err(""));
            }
            return Err(self.variable_access_error_at(
                "set",
                input,
                "upvar refers to variable in deleted namespace",
                ValueWrite,
            ));
        }
        if self.const_vars.contains(&id) {
            if !leave_error_message {
                return Err(err(""));
            }
            return Err(self.variable_access_error_at(
                "set",
                input,
                "variable is a constant",
                ValueWrite,
            ));
        }
        if matches!(
            self.var_arena.get(id).map(crate::vars::VarCell::state),
            Some(Local::Array(_))
        ) {
            if !leave_error_message {
                return Err(err(""));
            }
            return Err(self.variable_access_error_at(
                "set",
                input,
                "variable is array",
                ValueWrite,
            ));
        }
        Ok(())
    }

    pub(in super::super) fn store_captured_update(
        &mut self,
        name: &[u8],
        key: Option<&[u8]>,
        captured: &CapturedVariableUpdate,
        value: Value,
    ) -> Result<Value, Completion<Value>> {
        self.store_captured_update_with_original(name, key, captured, value, None)
    }

    pub(in super::super) fn store_captured_update_with_original(
        &mut self,
        name: &[u8],
        key: Option<&[u8]>,
        captured: &CapturedVariableUpdate,
        value: Value,
        original: Option<&Value>,
    ) -> Result<Value, Completion<Value>> {
        self.store_captured_update_with_original_and_errors(
            name, key, captured, value, original, true,
        )
    }

    fn check_confined_captured_update(
        &mut self,
        name: &[u8],
        captured: &CapturedVariableUpdate,
    ) -> Result<(), Completion<Value>> {
        if !self.limits.confined_stores {
            return Ok(());
        }
        self.confine_store_binding_bytes(name, &captured.binding, self.current_level(), false)?;
        let current = self.resolve_binding_var(
            captured.binding.clone(),
            captured.cell.elem.clone(),
            &mut HashSet::new(),
        );
        if current.and_then(|resolved| resolved.id) != captured.cell.id {
            return Err(self.refuse_host_command("confined captured variable changed".into()));
        }
        Ok(())
    }

    pub(in super::super) fn store_captured_update_with_original_and_errors(
        &mut self,
        name: &[u8],
        key: Option<&[u8]>,
        captured: &CapturedVariableUpdate,
        value: Value,
        original: Option<&Value>,
        leave_error_message: bool,
    ) -> Result<Value, Completion<Value>> {
        let original_names =
            crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(self)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        let reached = (|| {
            use native_operation_currency::NativeOperationScope;
            let scope = NativeOperationScope::enter(self)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
            let checked =
                self.check_captured_update_with_errors(name, key, captured, leave_error_message);
            if leave_error_message
                && checked.is_err()
                && let Some(original) = original
            {
                let reached = original_names.read(self, original);
                scope
                    .check(self)
                    .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
                reached.map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
            }
            scope
                .check(self)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
            checked?;
            let cell = &captured.cell;
            let id = cell.id.expect("captured update cell");
            self.check_confined_captured_update(name, captured)?;
            let _ = self.var_arena.replace_state(id, Local::Scalar(value));
            scope
                .check(self)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
            if self.var_arena.get(id).is_none() {
                return Err(crate::command::completion_from_cmd_error(
                    self,
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "actual captured variable cell",
                    )
                    .into(),
                ));
            }
            if !self.var_arena.element_is_attached(id)
                || captured.namespace.is_some_and(|namespace| {
                    namespace != ROOT_NS && !self.ns_vars.contains_key(&namespace)
                })
            {
                // Recheck actual retained storage, without repeating constant/type
                // acceptance after a reached old-value retirement callback.
                self.check_captured_update_with_errors(name, key, captured, leave_error_message)?;
            }
            if let Some(original) = original
                && self.original_variable_trace_requires_name(cell, key.is_some(), "write")
            {
                let reached = original_names.read(self, original);
                scope
                    .check(self)
                    .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
                reached.map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
            }
            let input = if let Some(key) = key {
                let mut display = name.to_vec();
                display.push(b'(');
                display.extend_from_slice(key);
                display.push(b')');
                (display, (name, Some(key)), true)
            } else {
                // C parses a combined `a(k)` name into its array and element,
                // so the array's traces fire; an alias to an element does not.
                let element_syntax = cell.elem.is_some()
                    && tcl_syntax::naming::split_element_ref_bytes(name).is_some();
                (name.to_vec(), (name, None), element_syntax)
            };
            let reached = self.variable_trace_guest_or_unwind(|vm| {
                vm.fire_var_traces_from_parts(
                    VarTraceInvocation {
                        leave_error_message,
                        name: &input.0,
                        op: "write",
                        reported: input.1,
                        element_access: input.2,
                    },
                    None,
                    Some(cell.clone()),
                )
            });
            scope
                .check(self)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
            reached?;
            Ok(self.read_resolved_cell(id).unwrap_or_else(Value::empty))
        })();
        original_names
            .finish(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        reached
    }
}
