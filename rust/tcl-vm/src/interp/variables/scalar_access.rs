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

//! Reading and writing a scalar by name: the access diagnostics, traced writes, raw writes and constants.

use super::super::{ResolvedVar, Vm, err};
use crate::command::err_with_code;
use crate::value::Value;
use crate::vars::VarState as Local;
use tcl_runtime_api::Completion;

impl Vm {
    /// Validate the namespace portion of a qualified variable write.  Tcl's
    /// `TclGetNamespaceForQualName` rejects a missing parent namespace rather
    /// than creating it implicitly.
    pub(super) fn validate_var_parent(&mut self, name: &str) -> Result<(), Completion<Value>> {
        if self.name_policy_protocol().is_none() {
            return Err(
                self.refuse_host_command("native variable name policy is unavailable".into())
            );
        }
        if self.var_parent_exists_bytes(name.as_bytes()) {
            return Ok(());
        }
        Err(self.variable_access_error_bytes(
            "set",
            name.as_bytes(),
            "parent namespace doesn't exist",
        ))
    }

    pub(crate) fn var_parent_exists_bytes(&self, name: &[u8]) -> bool {
        self.var_parent_exists_bytes_from(self.current_level(), name)
    }

    fn var_parent_exists_bytes_from(&self, level: usize, name: &[u8]) -> bool {
        self.resolve_var_from_bytes(name, level).is_some()
    }

    /// Read a scalar (following links). A link may resolve to an array element
    /// name (`upvar 0 arr(key) alias`), in which case the element is read.
    #[must_use]
    pub fn get_var(&self, name: &str) -> Option<Value> {
        self.get_var_bytes(name.as_bytes())
    }

    /// Read a native byte name from the active physical variable frame.
    #[must_use]
    pub fn get_var_bytes(&self, name: &[u8]) -> Option<Value> {
        if self.observed_name_policy_selected() {
            return self
                .observed_variable_get(name, self.current_level())
                .ok()
                .flatten();
        }
        let resolved = self.resolve_var_from_bytes(name, self.current_level())?;
        self.read_variable_contents(&resolved)
    }

    pub(in super::super) fn variable_access_error_bytes(
        &mut self,
        verb: &str,
        name: &[u8],
        reason: &str,
    ) -> Completion<Value> {
        self.variable_access_error_input(
            verb,
            tcl_syntax::naming::NativeVariableInputForm::Combined(name),
            reason,
        )
    }

    pub(in super::super) fn variable_access_error_input(
        &mut self,
        verb: &str,
        input: tcl_syntax::naming::NativeVariableInputForm<'_>,
        reason: &str,
    ) -> Completion<Value> {
        self.variable_access_error_at(
            verb,
            input,
            reason,
            tcl_syntax::naming::NativeVariableFailureSite::NameLookup,
        )
    }

    pub(in super::super) fn variable_access_error_at(
        &mut self,
        verb: &str,
        input: tcl_syntax::naming::NativeVariableInputForm<'_>,
        reason: &str,
        site: tcl_syntax::naming::NativeVariableFailureSite,
    ) -> Completion<Value> {
        self.variable_access_diagnostic_at(verb, input, reason, site)
            .0
    }

    /// The diagnostic completion, and whether the release's diagnostic sets
    /// an error code of its own (C 8.6 and later).
    pub(super) fn variable_access_diagnostic_at(
        &mut self,
        verb: &str,
        input: tcl_syntax::naming::NativeVariableInputForm<'_>,
        reason: &str,
        site: tcl_syntax::naming::NativeVariableFailureSite,
    ) -> (Completion<Value>, bool) {
        use tcl_syntax::naming::{
            NativeVariableDiagnosticOperation as Operation,
            NativeVariableDiagnosticReason as Reason, report_native_variable_diagnostic_at,
        };
        let Some(policy) = self.name_policy_protocol() else {
            return (
                self.refuse_host_command("native variable diagnostic policy is unavailable".into()),
                false,
            );
        };
        let operation = match verb {
            "read" => Operation::Read,
            "unset" => Operation::Unset,
            "incr" => Operation::Increment,
            "define" => Operation::Define,
            "make constant" | "const" => Operation::MakeConstant,
            _ => Operation::Write,
        };
        let selected_reason = match reason {
            "no such variable" => Reason::NoSuchVariable,
            "no such element in array" => Reason::NoSuchElement,
            "variable isn't array" => Reason::NotArray,
            "variable is array" => Reason::IsArray,
            "parent namespace doesn't exist" => Reason::MissingParentNamespace,
            "variable is a constant" => Reason::Constant,
            "upvar refers to element in deleted array" => Reason::DetachedElement,
            "upvar refers to variable in deleted namespace" => Reason::RetiredNamespace,
            "variable already exists" => Reason::AlreadyExists,
            "name refers to an element in an array" => Reason::ArrayElement,
            _ => {
                return (
                    self.refuse_host_command(
                        "native variable diagnostic reason is unavailable".into(),
                    ),
                    false,
                );
            }
        };
        let projection = match report_native_variable_diagnostic_at(
            policy.recipe(),
            operation,
            selected_reason,
            site,
            input,
        ) {
            Ok(projection) => projection,
            Err(error) => {
                return (
                    self.refuse_host_command(format!(
                        "native variable diagnostic is unavailable: {error:?}"
                    )),
                    false,
                );
            }
        };
        let sets_code = projection.error_code.is_some();
        let mut message = format!("can't {verb} \"").into_bytes();
        message.extend_from_slice(&projection.name);
        message.extend_from_slice(b"\": ");
        message.extend_from_slice(projection.reason.message().as_bytes());
        if let Some(protocol) = self.native_c_variable_name_protocol() {
            if protocol.diagnostic_resets_interpreter_result() {
                self.reset_native_global_error_episode();
            }
            let completion = crate::command::completion_from_cmd_error(
                self,
                tcl_cmd_core::CmdError::from_byte_details(tcl_cmd_core::CmdErrorDetails {
                    message,
                    string_result: protocol.diagnostic_string_protocol(),
                    error_code: projection.error_code.map_or(
                        tcl_cmd_core::CmdErrorCodeUpdate::Default,
                        |words| {
                            let code = Value::list(
                                words.into_iter().map(Value::from_string_bytes).collect(),
                            );
                            tcl_cmd_core::CmdErrorCodeUpdate::Set(code.string_bytes().to_vec())
                        },
                    ),
                    error_info: None,
                    error_line: None,
                    primitive_getter: None,
                }),
            );
            return (completion, sets_code);
        }
        let completion = if let Some(words) = projection.error_code {
            let code = Value::list(words.into_iter().map(Value::from_string_bytes).collect());
            err_with_code(message, code.string_bytes())
        } else {
            err(message)
        };
        (completion, sets_code)
    }

    /// Write a native combined-name operand while retaining its original
    /// spelling independently of the followed binding and element cells.
    pub fn set_var_bytes(&mut self, name: &[u8], value: Value) -> Result<(), Completion<Value>> {
        self.set_var_bytes_reporting(name, value, None)
    }

    fn set_var_bytes_reporting(
        &mut self,
        name: &[u8],
        value: Value,
        reported_name: Option<&[u8]>,
    ) -> Result<(), Completion<Value>> {
        self.confine_store_bytes(name, self.current_level(), false)?;
        if let Some(refused) = self.refused_completion() {
            return Err(refused);
        }
        if self.observed_names.is_some() {
            return self
                .observed_variable_set(name, self.current_level(), value)
                .map_err(|_| {
                    self.refuse_host_command("observed variable storage is unavailable".into())
                });
        }
        if self.name_policy_protocol().is_none() {
            return Err(
                self.refuse_host_command("native variable name policy is unavailable".into())
            );
        }
        let Some(resolved) = self.resolve_var_from_bytes(name, self.current_level()) else {
            return Err(self.variable_access_error_bytes(
                "set",
                name,
                "parent namespace doesn't exist",
            ));
        };
        if resolved.id.is_some_and(|id| self.const_vars.contains(&id)) {
            return Err(self.variable_access_error_at(
                "set",
                tcl_syntax::naming::NativeVariableInputForm::Combined(name),
                "variable is a constant",
                tcl_syntax::naming::NativeVariableFailureSite::ValueWrite,
            ));
        }
        if resolved.elem.is_none()
            && resolved.id.is_some_and(|id| {
                !self.var_arena.element_is_attached(id)
                    && self
                        .var_arena
                        .get(id)
                        .is_none_or(|cell| matches!(cell.state(), Local::Undefined))
            })
        {
            return Err(self.variable_access_error_at(
                "set",
                tcl_syntax::naming::NativeVariableInputForm::Combined(name),
                "upvar refers to element in deleted array",
                tcl_syntax::naming::NativeVariableFailureSite::ValueWrite,
            ));
        }
        if self.dictionary_variable_containers()
            && let Some(key) = &resolved.elem
        {
            self.write_dictionary_element(&resolved.binding, key, value)?;
        } else {
            if resolved.elem.is_none()
                && resolved.id.is_some_and(|id| {
                    matches!(
                        self.var_arena.get(id).map(crate::vars::VarCell::state),
                        Some(Local::Array(_))
                    )
                })
            {
                return Err(self.variable_access_error_at(
                    "set",
                    tcl_syntax::naming::NativeVariableInputForm::Combined(name),
                    "variable is array",
                    tcl_syntax::naming::NativeVariableFailureSite::ValueWrite,
                ));
            }
            let id = self
                .ensure_target_var_at_binding(&resolved.binding, resolved.elem.as_deref())
                .ok_or_else(|| {
                    self.variable_access_error_bytes("set", name, "variable isn't array")
                })?;
            self.drop_array_descendant_traces(id);
            let _ = self.var_arena.replace_state(id, Local::Scalar(value));
        }
        if self.variable_observers_active() || self.authored_tmm_static.policy.is_some() {
            self.variable_trace_guest_or_unwind(|vm| {
                vm.fire_var_traces_from_cell_bytes(name, "write", reported_name, None, None)
            })?;
        }
        Ok(())
    }

    /// Write a scalar with no trace firing (frame argument binding, rollback).
    /// A link resolving to an array element name writes that element.
    pub(in super::super) fn write_scalar_raw(&mut self, name: &str, value: Value) {
        let Some(resolved) = self.resolve_var_from(name, self.current_level()) else {
            return;
        };
        self.write_scalar_resolved_raw(resolved, value);
    }

    pub(in super::super) fn write_scalar_resolved_raw(
        &mut self,
        resolved: ResolvedVar,
        value: Value,
    ) {
        if self.dictionary_variable_containers()
            && let Some(key) = &resolved.elem
        {
            let _ = self.write_dictionary_element(&resolved.binding, key, value);
            return;
        }
        if let Some(id) = resolved.id {
            self.drop_array_descendant_traces(id);
            let _ = self.var_arena.replace_state(id, Local::Scalar(value));
        } else if let Some(key) = resolved.elem {
            if let Some(id) =
                self.ensure_target_var_at_binding(&resolved.binding, Some(key.as_bytes()))
            {
                let _ = self.var_arena.replace_state(id, Local::Scalar(value));
            }
        } else {
            let _ = self.bind_new_var(&resolved.binding, Local::Scalar(value));
        }
    }

    /// Mark `name`'s scalar cell as a `const` (immutable).
    #[cfg(test)]
    pub(crate) fn mark_constant(&mut self, name: &str) {
        if let Some(id) = self
            .resolve_var_from(name, self.current_level())
            .and_then(|resolved| resolved.id)
        {
            self.const_vars.insert(id);
        }
    }

    /// Whether `name` resolves to a `const` cell.
    pub(crate) fn is_constant(&self, name: &[u8]) -> bool {
        self.is_constant_from_bytes(self.current_level(), name)
    }

    /// Whether `name`, resolved as if `start` were active, is a `const` cell.
    pub(in super::super) fn is_constant_from(&self, start: usize, name: &str) -> bool {
        self.is_constant_from_bytes(start, name.as_bytes())
    }

    fn is_constant_from_bytes(&self, start: usize, name: &[u8]) -> bool {
        self.resolve_var_from_bytes(name, start)
            .and_then(|resolved| resolved.id)
            .is_some_and(|id| self.const_vars.contains(&id))
    }

    /// Write a scalar, firing `write` traces afterwards. A callback error
    /// fails the *command* but leaves the new value stored: C swaps the value
    /// into the cell before calling the traces and its error path jumps
    /// straight to `cleanup`, which never restores the old one
    /// (`TclPtrSetVarIdx`, `tclVar.c` 9.0.4:1913 — value in at :2023-2032,
    /// traces at :2040-2046, `cleanup:` at :2070; same shape in 8.6.16). A
    /// cell the write created therefore survives too.
    ///
    /// A name that spells an array element writes the element, creating the
    /// array, as `Tcl_ObjSetVar2` reads a name with no element word: every
    /// command that stores into a variable it is named (`catch`, `foreach`,
    /// `lassign`, `scan`, …) takes `a(k)` so.
    pub fn set_var(&mut self, name: &str, value: Value) -> Result<(), Completion<Value>> {
        self.set_var_bytes(name.as_bytes(), value)
    }
}
