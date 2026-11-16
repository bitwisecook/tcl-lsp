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

//! Reads and existence checks that fire read traces first, and pinning an array cell as the target of an array operation.

use super::super::{
    ArrayOperationBinding, ResolvedVar, VarBinding, VarTableOwner, VarTraceCell, Vm,
    native_name_consumers, native_operation_currency, variable_cleanup,
};
use crate::value::Value;
use crate::vars::VarState as Local;
use std::rc::Rc;
use tcl_runtime_api::{ArrayTarget, Completion, FrameId, VarId, VarStore};

impl Vm {
    pub(crate) fn var_trace_info_bytes(&self, name: &[u8]) -> Vec<(Vec<String>, Value)> {
        let Some(protocol) = self
            .actual_native_invocation_dialect()
            .native_variable_trace_protocol()
        else {
            return Vec::new();
        };
        let Some(key) = self
            .registered_trace_cell_bytes(name)
            .and_then(|cell| cell.id)
        else {
            return Vec::new();
        };
        self.variable_observers
            .script_traces
            .get(&key)
            .map_or_else(Vec::new, |list| {
                list.iter()
                    .rev()
                    .filter_map(|t| {
                        t.command.as_ref().map(|command| {
                            (
                                t.ops.clone(),
                                Value::new_native_string_bytes(
                                    protocol.variable_prefix_report(&command.string_bytes()),
                                ),
                            )
                        })
                    })
                    .collect()
            })
    }

    /// Read through one stable variable identity across a callback. If no cell
    /// existed before the read trace, the callback may create it and the name
    /// is resolved once afterwards; otherwise deletion/recreation of the same
    /// spelling must not steer the pending read to the replacement cell.
    pub(crate) fn read_var_traced(
        &mut self,
        name: &str,
    ) -> Result<Option<Value>, Completion<Value>> {
        self.read_var_traced_bytes(name.as_bytes())
    }

    pub(crate) fn read_var_traced_bytes(
        &mut self,
        name: &[u8],
    ) -> Result<Option<Value>, Completion<Value>> {
        if self.observed_names.is_some() {
            return self
                .observed_variable_get(name, self.current_level())
                .map_err(|_| {
                    self.refuse_host_command("observed variable storage is unavailable".into())
                });
        }
        if self.name_policy_protocol().is_none() {
            return Err(
                self.refuse_host_command("native variable name policy is unavailable".into())
            );
        }
        if self.dictionary_variable_containers() {
            return Ok(self.get_var_bytes(name));
        }
        let resolved = self.resolve_var_from_bytes(name, self.current_level());
        let explicit_array = resolved
            .as_ref()
            .filter(|resolved| resolved.elem.is_some())
            .and_then(|resolved| resolved.base_id);
        let Some(cell) = resolved
            .as_ref()
            .and_then(|resolved| self.trace_cell_from_resolved(resolved))
            .filter(|cell| cell.id.is_some())
        else {
            self.variable_trace_guest_or_unwind(|vm| {
                vm.fire_var_traces_from_cell_bytes(name, "read", None, None, None)
            })?;
            return Ok(self.get_var_bytes(name));
        };
        self.with_variable_operation(&cell, |vm| {
            vm.variable_trace_guest_or_unwind(|vm| {
                vm.fire_var_traces_from_cell_bytes(name, "read", None, None, Some(cell.clone()))
            })?;
            Ok(vm
                .initial_value_at_cell(cell.id.expect("selected native read"), explicit_array)
                .cloned())
        })
    }

    /// Produce the value or native missing completion while the originally
    /// selected receiver remains retained. Reporting never re-resolves an
    /// alias after a callback has replaced its binding.
    pub(crate) fn read_variable_result_bytes(
        &mut self,
        root: &[u8],
        element: Option<&[u8]>,
    ) -> Result<Value, Completion<Value>> {
        use tcl_syntax::naming::{
            NativeVariableFailureSite as Site, NativeVariableInputForm as Input,
        };

        if self.observed_name_policy_selected() {
            if element.is_some() {
                return Err(self.refuse_host_command(
                    "observed separate variable input is unavailable".into(),
                ));
            }
            return match self.observed_variable_get(root, self.current_level()) {
                Ok(Some(value)) => Ok(value),
                Ok(None) => Err(self.refuse_host_command(
                    "observed missing variable completion is unavailable".into(),
                )),
                Err(error) => Err(self.refuse_host_command(format!(
                    "observed variable storage is unavailable: {error}"
                ))),
            };
        }

        let input = element.map_or(Input::Combined(root), |element| Input::Separate {
            root,
            element: Some(element),
        });
        if self.name_policy_protocol().is_none() {
            return Err(
                self.refuse_host_command("native variable name policy is unavailable".into())
            );
        }
        let resolved = if element.is_some() {
            self.resolve_var_parts_from_bytes(root, element, self.current_level())
        } else {
            self.resolve_var_from_bytes(root, self.current_level())
        }
        .ok_or_else(|| {
            // A read creates nothing, so C's lookup reports the variable
            // missing whether or not its namespace exists.
            self.variable_access_error_at("read", input, "no such variable", Site::NameLookup)
        })?;
        self.read_selected_variable_result_bytes(root, element, resolved)
    }

    /// Read as C's `Tcl_ObjGetVar2` without `TCL_LEAVE_ERR_MSG`, the probe
    /// a no-value `lappend` makes. A failure returns the completion options it
    /// leaves on a successful update: a lookup miss reaches no trace and leaves
    /// none, while a reached read that fails leaves the release's value-read
    /// error code (`TclPtrGetVarIdx` sets it whatever the flags).
    pub(crate) fn read_variable_unreported_bytes(
        &mut self,
        name: &[u8],
    ) -> Result<Result<Value, Value>, Completion<Value>> {
        use tcl_syntax::naming::{
            NativeVariableDiagnosticOperation as Operation,
            NativeVariableDiagnosticReason as Reason, NativeVariableFailureSite as Site,
            NativeVariableInputForm as Input,
        };
        let reported =
            self.dictionary_variable_containers() || self.observed_name_policy_selected();
        if !reported
            && !self
                .resolve_var_from_bytes(name, self.current_level())
                .is_some_and(|resolved| self.lookup_reaches_variable_cell(&resolved))
        {
            return Ok(Err(Value::empty()));
        }
        let failure = match self.read_variable_result_bytes(name, None) {
            Ok(value) => return Ok(Ok(value)),
            Err(failure) => failure,
        };
        if self.refused_completion().is_some() {
            return Err(failure);
        }
        self.publish_swallowed_trace_error();
        if reported {
            return Ok(Err(failure.options));
        }
        let Some(policy) = self.name_policy_protocol() else {
            return Err(
                self.refuse_host_command("native variable diagnostic policy is unavailable".into())
            );
        };
        let code = tcl_syntax::naming::report_native_variable_diagnostic_at(
            policy.recipe(),
            Operation::Read,
            Reason::NoSuchVariable,
            Site::ValueRead,
            Input::Combined(name),
        )
        .map_err(|error| {
            self.refuse_host_command(format!(
                "native variable diagnostic is unavailable: {error:?}"
            ))
        })?
        .error_code;
        Ok(Err(code.map_or_else(Value::empty, |words| {
            Value::list(vec![
                Value::string("-errorcode"),
                Value::list(words.into_iter().map(Value::from_string_bytes).collect()),
            ])
        })))
    }

    /// Whether a variable lookup that creates nothing reaches a cell: an
    /// existing variable or link target, or an element of an existing array.
    fn lookup_reaches_variable_cell(&self, resolved: &ResolvedVar) -> bool {
        let base = resolved.base_id.and_then(|id| self.var_arena.get(id));
        if resolved.elem.is_some() {
            resolved.id.is_some()
                || base.is_some_and(|cell| matches!(cell.state(), Local::Array(_)))
        } else {
            resolved.id.is_some()
        }
    }

    pub(in super::super) fn read_selected_variable_result_bytes(
        &mut self,
        root: &[u8],
        element: Option<&[u8]>,
        resolved: ResolvedVar,
    ) -> Result<Value, Completion<Value>> {
        self.read_selected_variable_result_bytes_with_original(root, element, resolved, None)
    }

    fn read_selected_variable_after_traces(
        &mut self,
        resolved: &ResolvedVar,
        input: tcl_syntax::naming::NativeVariableInputForm<'_>,
        original: Option<&Value>,
        original_names: &native_name_consumers::OriginalNameConsumerScope,
    ) -> Result<Value, Completion<Value>> {
        use tcl_syntax::naming::NativeVariableFailureSite as Site;
        let current = if self.dictionary_variable_containers() {
            self.read_variable_contents(resolved)
        } else if let Some(id) = resolved.id {
            self.initial_value_at_cell(id, resolved.elem.as_ref().and(resolved.base_id))
                .cloned()
        } else if let Some((base, element)) = resolved.base_id.zip(resolved.elem.as_ref()) {
            self.read_resolved_elem(base, element)
        } else {
            None
        };
        if let Some(value) = current {
            return Ok(value);
        }
        let reason = if resolved.elem.is_some() {
            if resolved
                .base_id
                .and_then(|id| self.var_arena.get(id))
                .is_some_and(|cell| matches!(cell.state(), Local::Array(_)))
            {
                "no such element in array"
            } else {
                "no such variable"
            }
        } else if resolved
            .id
            .and_then(|id| self.var_arena.get(id))
            .is_some_and(|cell| matches!(cell.state(), Local::Array(_)))
        {
            "variable is array"
        } else {
            "no such variable"
        };
        let site = if resolved.id.is_some() || resolved.base_id.is_some() {
            Site::ValueRead
        } else {
            Site::NameLookup
        };
        if let Some(original) = original {
            original_names
                .read(self, original)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        }
        Err(self.variable_access_error_at("read", input, reason, site))
    }

    pub(in super::super) fn read_selected_variable_result_bytes_with_original(
        &mut self,
        root: &[u8],
        element: Option<&[u8]>,
        mut resolved: ResolvedVar,
        original: Option<&Value>,
    ) -> Result<Value, Completion<Value>> {
        let original_names =
            crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(self)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        let reached = (|| {
            use tcl_syntax::naming::{
                NativeVariableFailureSite as Site, NativeVariableInputForm as Input,
            };
            let input = element.map_or(Input::Combined(root), |element| Input::Separate {
                root,
                element: Some(element),
            });
            if resolved.elem.is_some()
                && !self.dictionary_variable_containers()
                && resolved
                    .base_id
                    .and_then(|id| self.var_arena.get(id))
                    .is_some_and(|cell| matches!(cell.state(), Local::Scalar(_)))
            {
                if let Some(original) = original {
                    original_names
                        .read(self, original)
                        .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
                }
                return Err(self.variable_access_error_at(
                    "read",
                    input,
                    "variable isn't array",
                    Site::NameLookup,
                ));
            }
            self.prepare_existing_array_read(&mut resolved)?;
            let cell = self.trace_cell_from_resolved(&resolved);
            let read = |vm: &mut Self| -> Result<Value, Completion<Value>> {
                if let Some(original) = original
                    && cell.as_ref().is_some_and(|cell| {
                        vm.original_variable_trace_requires_name(cell, element.is_some(), "read")
                    })
                {
                    original_names
                        .read(vm, original)
                        .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?;
                }
                if let Some(element) = element {
                    vm.variable_trace_guest_or_unwind(|vm| {
                        vm.fire_elem_traces_from_cell_bytes(root, element, "read", cell.clone())
                    })?;
                } else {
                    vm.variable_trace_guest_or_unwind(|vm| {
                        vm.fire_var_traces_from_cell_bytes(root, "read", None, None, cell.clone())
                    })?;
                }
                vm.read_selected_variable_after_traces(&resolved, input, original, &original_names)
            };
            if let Some(cell) = cell.as_ref() {
                self.with_variable_operation(cell, read)
            } else {
                read(self)
            }
        })();
        original_names
            .finish(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        reached
    }

    pub(in super::super) fn prepare_existing_array_read(
        &mut self,
        resolved: &mut ResolvedVar,
    ) -> Result<(), Completion<Value>> {
        if resolved.id.is_some() || self.dictionary_variable_containers() {
            return Ok(());
        }
        let Some((array, key)) = resolved.base_id.zip(resolved.elem.as_ref()) else {
            return Ok(());
        };
        if !self
            .var_arena
            .get(array)
            .is_some_and(|cell| matches!(cell.state(), Local::Array(_)))
        {
            return Ok(());
        }
        let id = self
            .var_arena
            .alloc_element(Local::Undefined, array, key.clone())
            .ok_or_else(|| {
                self.refuse_host_command("native array element allocation is unavailable".into())
            })?;
        if !self.var_arena.array_insert(array, key.clone(), id) {
            self.var_arena.discard_unbound(id);
            return Err(self.refuse_host_command("native array read receiver expired".into()));
        }
        self.var_arena.bind(id);
        resolved.id = Some(id);
        Ok(())
    }

    pub(crate) fn read_elem_traced_bytes(
        &mut self,
        name: &[u8],
        key: &[u8],
    ) -> Result<Option<Value>, Completion<Value>> {
        self.read_elem_traced_from_bytes(name, key, self.current_level())
            .1
    }

    pub(in super::super) fn read_elem_traced_from_bytes(
        &mut self,
        name: &[u8],
        key: &[u8],
        start: usize,
    ) -> (bool, Result<Option<Value>, Completion<Value>>) {
        let resolved = self.resolve_var_parts_from_bytes(name, Some(key), start);
        let live_was_array = resolved
            .as_ref()
            .and_then(|resolved| resolved.base_id)
            .and_then(|id| self.var_arena.get(id))
            .is_some_and(|cell| matches!(cell.state(), Local::Array(_)));
        let cell = resolved
            .as_ref()
            .and_then(|resolved| self.trace_cell_from_resolved(resolved));
        let selected = cell.as_ref().and_then(|cell| cell.id);
        if let Some(id) = selected {
            self.var_arena.retain_operation(id);
        }
        let trace = self.variable_trace_guest_or_unwind(|vm| {
            vm.fire_elem_traces_from_cell_bytes(name, key, "read", cell)
        });
        let value = if trace.is_ok() {
            selected.map_or_else(
                || self.get_array_elem_bytes_from(start, name, key),
                |id| self.read_resolved_cell(id),
            )
        } else {
            None
        };
        if let Some(id) = selected {
            self.var_arena.release_operation(id);
        }
        (live_was_array, trace.map(|()| value))
    }

    pub(crate) fn exists_var_traced_bytes(
        &mut self,
        name: &[u8],
    ) -> Result<bool, tcl_runtime_api::NativeExecutionError> {
        let scope = native_operation_currency::NativeOperationScope::enter_native(self)?;
        if self.observed_name_policy_selected() {
            let reached = self.observed_variable_exists(name, self.current_level());
            scope.check_native(self)?;
            return reached.map_err(|error| {
                self.retain_host_command_refusal(format!(
                    "observed variable storage is unavailable: {error}"
                ))
            });
        }
        let cell = self.trace_cell_bytes(name).and_then(|cell| cell.id);
        self.fire_original_existence_read(name, None, None)?;
        scope.check_native(self)?;
        let found = cell.map_or_else(
            || self.exists_var_bytes_from(name, self.current_level()),
            |id| {
                self.var_arena
                    .get(id)
                    .is_some_and(|cell| matches!(cell.state(), Local::Scalar(_) | Local::Array(_)))
            },
        );
        scope.check_native(self)?;
        Ok(found)
    }

    pub(in super::super) fn exists_var_bytes_from(&self, name: &[u8], start: usize) -> bool {
        let Some(resolved) = self.resolve_var_from_bytes(name, start) else {
            return false;
        };
        if self.dictionary_variable_containers() {
            return self.read_variable_contents(&resolved).is_some();
        }
        resolved
            .id
            .and_then(|id| self.var_arena.get(id))
            .is_some_and(|cell| matches!(cell.state(), Local::Scalar(_) | Local::Array(_)))
    }

    pub(crate) fn exists_elem_traced_bytes(
        &mut self,
        root: &[u8],
        element: &[u8],
    ) -> Result<bool, tcl_runtime_api::NativeExecutionError> {
        let scope = native_operation_currency::NativeOperationScope::enter_native(self)?;
        let resolved = self.resolve_var_parts_from_bytes(root, Some(element), self.current_level());
        let cell = resolved
            .as_ref()
            .and_then(|resolved| self.trace_cell_from_resolved(resolved));
        let selected = cell.as_ref().and_then(|cell| cell.id);
        self.fire_original_existence_read(root, Some(element), cell)?;
        scope.check_native(self)?;
        let found = selected.map_or_else(
            || {
                self.get_array_elem_bytes_from(self.current_level(), root, element)
                    .is_some()
            },
            |id| self.read_resolved_cell(id).is_some(),
        );
        scope.check_native(self)?;
        Ok(found)
    }

    pub(crate) fn unset_array_elem_checked_bytes(
        &mut self,
        root: &[u8],
        element: &[u8],
        complain: bool,
    ) -> Result<(), Completion<Value>> {
        self.confine_store_parts_bytes(root, Some(element), self.current_level(), true)?;
        let resolved = self.resolve_var_parts_from_bytes(root, Some(element), self.current_level());
        let reason = resolved.as_ref().map_or("no such variable", |resolved| {
            match resolved
                .base_id
                .and_then(|id| self.var_arena.get(id))
                .map(crate::vars::VarCell::state)
            {
                Some(Local::Array(_)) => "no such element in array",
                Some(Local::Scalar(_)) => "variable isn't array",
                _ => "no such variable",
            }
        });
        let existed = resolved
            .as_ref()
            .and_then(|resolved| resolved.base_id.map(|base| (resolved, base)))
            .is_some_and(|(resolved, base)| {
                self.array_unset_elem_reporting_bytes_at(
                    base,
                    root,
                    resolved.elem.as_deref().unwrap_or(element),
                    None,
                )
            });
        if !existed && complain {
            return Err(self.variable_access_error_input(
                "unset",
                tcl_syntax::naming::NativeVariableInputForm::Separate {
                    root,
                    element: Some(element),
                },
                reason,
            ));
        }
        Ok(())
    }

    fn fire_array_trace_at(
        &mut self,
        name: &[u8],
        id: Option<VarId>,
    ) -> Result<(), Completion<Value>> {
        if !self.variable_observers_active() {
            return Ok(());
        }
        let Some(id) = id else {
            return Ok(());
        };
        let parent = self
            .var_arena
            .element_parent(id)
            .map(|(array, key)| (array, key.to_owned()));
        let cell = VarTraceCell {
            id: Some(id),
            array: parent.as_ref().map(|(array, _)| *array),
            elem: parent.map(|(_, key)| key),
        };
        // `env`'s host hook is an array trace C installs (`TclSetupEnv`).
        let has_array_trace = self.variable_observers.env_cell == Some(id)
            || self
                .variable_observers
                .script_traces
                .get(&id)
                .is_some_and(|traces| {
                    traces
                        .iter()
                        .any(|trace| trace.ops.iter().any(|op| op == "array"))
                });
        let traceable_state = self
            .var_arena
            .get(id)
            .is_some_and(|cell| matches!(cell.state(), Local::Undefined | Local::Array(_)));
        if !has_array_trace || !traceable_state {
            return Ok(());
        }
        self.variable_trace_guest_or_unwind(|vm| {
            vm.fire_var_traces_from_cell_bytes(name, "array", None, None, Some(cell))
        })
    }

    /// Locate and pin one array cell across its operation trace and command.
    /// This is the VM's `LocateArray` boundary: callbacks may remove the last
    /// binding or retarget the source alias, but the shared array core can
    /// still enumerate the cell Tcl originally reached.
    pub(crate) fn with_array_trace_target_bytes(
        &mut self,
        name: &[u8],
        operation: impl FnOnce(&mut Self, &ArrayTarget) -> Completion<Value>,
    ) -> Completion<Value> {
        use native_operation_currency::NativeOperationScope;
        let scope = match NativeOperationScope::enter(self) {
            Ok(scope) => scope,
            Err(error) => return crate::command::completion_from_cmd_error(self, error),
        };
        let original = variable_cleanup::OriginalVariableCleanup::enter(self);
        if let Err(error) = original.check(self) {
            return crate::command::completion_from_cmd_error(self, error);
        }
        let here = FrameId(self.current_level());
        let target = match VarStore::array_target_bytes(self, here, name) {
            Ok(target) => target,
            Err(error) => return crate::command::completion_from_cmd_error(self, error.into()),
        };
        if self.observed_name_policy_selected() {
            let result = operation(self, &target);
            return match original.check(self).and_then(|()| scope.check(self)) {
                Ok(()) => result,
                Err(error) => {
                    let _ = original.in_original(self, |_| drop(result));
                    crate::command::completion_from_cmd_error(self, error)
                }
            };
        }
        let id = target.cell_id();
        let protected_binding = id.and_then(|id| self.array_operation_binding(id));
        if let Some(id) = id {
            self.var_arena.retain_operation(id);
        }
        let reached = self.fire_array_trace_at(name, id);
        let result = match scope.check(self) {
            Err(error) => {
                drop(reached);
                crate::command::completion_from_cmd_error(self, error)
            }
            Ok(()) => match reached {
                Ok(()) => operation(self, &target),
                Err(completion) => completion,
            },
        };
        if let Some(id) = id {
            if let Some(binding) = protected_binding {
                self.release_array_operation_binding(&binding, id, &original);
            }
            let _ = original.retire(self, crate::vars::VariableRetirement::release_operation(id));
        }
        match original.check(self).and_then(|()| scope.check(self)) {
            Ok(()) => result,
            Err(error) => {
                let _ = original.in_original(self, |_| drop(result));
                crate::command::completion_from_cmd_error(self, error)
            }
        }
    }

    /// Drop the undefined name-table shell retained for a direct array
    /// operation once its final operation reference leaves. Keeping that shell
    /// during the callback lets an unset-and-recreate refill the same Tcl
    /// variable cell; a namespace token that is deleted and recreated owns a
    /// different table and therefore remains a distinct identity.
    pub(super) fn array_operation_binding(&self, id: VarId) -> Option<ArrayOperationBinding> {
        if let Some((parent, key)) = self.var_arena.element_parent(id)
            && matches!(
                self.var_arena.get(parent).map(crate::vars::VarCell::state),
                Some(Local::Array(elements)) if elements.get(key) == Some(&id)
            )
        {
            return Some(ArrayOperationBinding::Element);
        }
        for (level, frame) in self.frames.iter().enumerate() {
            if let Some((name, _)) = frame.local_bindings().find(|(_, raw)| **raw == id) {
                return Some(ArrayOperationBinding::Variable {
                    binding: VarBinding {
                        owner: VarTableOwner::Frame(level),
                        name: name.clone(),
                    },
                    table: frame.locals.weak_birth(),
                    frame: Some(Rc::downgrade(&frame.activation)),
                });
            }
        }
        for (&ns, table) in &self.ns_vars {
            if let Some((name, _)) = table.iter().find(|(_, raw)| **raw == id) {
                return Some(ArrayOperationBinding::Variable {
                    binding: VarBinding {
                        owner: VarTableOwner::Namespace(ns),
                        name: name.clone(),
                    },
                    table: table.weak_birth(),
                    frame: None,
                });
            }
        }
        None
    }

    pub(super) fn release_array_operation_binding(
        &mut self,
        binding: &ArrayOperationBinding,
        id: VarId,
        original: &variable_cleanup::OriginalVariableCleanup,
    ) {
        original.release_binding(self, binding, id);
    }

    /// Remove one unreferenced undefined element shell when its final trace is
    /// removed outside an operation that retains that element. An operation-
    /// retained shell deliberately survives its eventual release.
    pub(in super::super) fn discard_undefined_array_shell(
        &mut self,
        parent: VarId,
        key: impl AsRef<[u8]>,
        id: VarId,
    ) {
        let original = variable_cleanup::OriginalVariableCleanup::enter(self);
        original.discard_shell(self, parent, key.as_ref(), id);
    }
}
