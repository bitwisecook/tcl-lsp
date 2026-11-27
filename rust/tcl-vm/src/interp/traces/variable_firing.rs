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

//! Firing variable traces: selecting the array and cell groups, invoking script and native callbacks, and presenting a trace failure.

use super::super::errors::ErrorLogged;
use super::super::variable_trace_outcomes::VariableTraceOutcome;
use super::super::{
    Step, TakenVarTraces, TraceGroup, VarTrace, VarTraceCell, VarTraceInvocation, Vm, err,
    native_name_consumers, native_numeric_ops, native_operation_currency, ok, variable_cleanup,
};
use crate::value::Value;
use std::rc::Rc;
use tcl_runtime_api::{Code, Completion};

impl Vm {
    pub(in super::super) fn fire_var_traces_from_cell_bytes(
        &mut self,
        name: &[u8],
        op: &str,
        reported_name1: Option<&[u8]>,
        taken: Option<TakenVarTraces>,
        located: Option<VarTraceCell>,
    ) -> VariableTraceOutcome {
        let Some(policy) = self.name_policy_protocol() else {
            return Err(self.retain_host_command_refusal(
                "native variable trace reporting policy is unavailable".into(),
            ));
        };
        let input = policy.recipe().combined_variable_input(name);
        let reported_element = reported_name1.and_then(|_| {
            located
                .as_ref()
                .or_else(|| taken.as_ref().map(|taken| &taken.cell))
                .and_then(|cell| cell.elem.clone())
        });
        self.fire_var_traces_from_parts(
            VarTraceInvocation {
                leave_error_message: true,
                name,
                op,
                reported: (reported_name1.unwrap_or(name), reported_element.as_deref()),
                element_access: input.element().is_some(),
            },
            taken,
            located,
        )
    }

    pub(in super::super) fn fire_elem_traces_from_cell_bytes(
        &mut self,
        base: &[u8],
        key: &[u8],
        op: &str,
        located: Option<VarTraceCell>,
    ) -> VariableTraceOutcome {
        let mut display = base.to_vec();
        display.push(b'(');
        display.extend_from_slice(key);
        display.push(b')');
        self.fire_var_traces_from_parts(
            VarTraceInvocation {
                leave_error_message: true,
                name: &display,
                op,
                reported: (base, Some(key)),
                element_access: true,
            },
            None,
            located,
        )
    }

    #[allow(clippy::too_many_lines)] // one trace dispatch, step by step
    pub(in super::super) fn fire_var_traces_from_parts(
        &mut self,
        invocation: VarTraceInvocation<'_>,
        taken: Option<TakenVarTraces>,
        located: Option<VarTraceCell>,
    ) -> VariableTraceOutcome {
        let original = variable_cleanup::OriginalVariableCleanup::enter(self);
        original.check_native(self)?;
        // With no script or native observer, no taken trace and no static
        // publication, every step below is a balanced no-op.
        if taken.is_none()
            && !self.variable_observers_active()
            && !(invocation.op == "write" && self.authored_tmm_static.policy.is_some())
        {
            return Ok(Ok(()));
        }
        let Some(cell) = located.or_else(|| {
            taken.as_ref().map_or_else(
                || self.trace_cell_bytes(invocation.name),
                |taken| Some(taken.cell.clone()),
            )
        }) else {
            original.check_native(self)?;
            return Ok(Ok(()));
        };
        let publication_hold =
            invocation.op == "write" && self.authored_tmm_static.policy.is_some();
        if taken.is_none()
            && !publication_hold
            && cell.id.is_none_or(|id| !self.variable_cell_observed(id))
            && cell
                .array_id()
                .is_none_or(|array| !self.variable_cell_observed(array))
        {
            original.check_native(self)?;
            return Ok(Ok(()));
        }
        if publication_hold && let Some(id) = cell.id {
            self.var_arena.retain_operation(id);
            let reached = self.publish_authored_static_write(id);
            let current = original.check_native(self);
            if current.is_err() || reached.is_err() {
                let _ =
                    original.retire(self, crate::vars::VariableRetirement::release_operation(id));
                current?;
                original.check_native(self)?;
                return Ok(reached);
            }
        }
        let taken_unset = taken.is_some()
            && tcl_runtime_api::native_variable_trace::NativeVariableTraceOperation::from_name(invocation.op)
                .is_some_and(tcl_runtime_api::native_variable_trace::NativeVariableTraceOperation::uses_destroyed_cell_callbacks);
        if cell.id.is_some_and(|id| self.active_traces.contains(&id)) && !taken_unset {
            if publication_hold && let Some(id) = cell.id {
                let _ =
                    original.retire(self, crate::vars::VariableRetirement::release_operation(id));
            }
            original.check_native(self)?;
            return Ok(Ok(()));
        }
        let array_active = cell
            .array_id()
            .is_some_and(|array| self.active_traces.contains(&array));
        let active_position = self.active_traces.len();
        let selected = cell.id.and_then(|id| original.cell(self, id));
        if let Some(id) = cell.id {
            if !publication_hold {
                self.var_arena.retain_operation(id);
            }
            self.active_traces.push(id);
        }
        let global_error_flags = self.capture_native_global_error_flags();
        let reached =
            self.fire_var_traces_inner(invocation, &cell, array_active, taken, original.scope());
        if let Err(first) = &reached {
            self.retain_tcl_host_failure(crate::error::TclHostFailure::Execution(first.clone()));
        }
        let after_callback = original.check_native(self);
        if after_callback.is_ok() && matches!(reached, Ok(Ok(()))) {
            let _ = original.in_original(self, |vm| {
                vm.restore_native_global_error_flags(global_error_flags);
            });
        }
        if let Some(id) = cell.id {
            let removed = original.state_mut(self).is_ok_and(|state| {
                if state.active_traces.get(active_position) == Some(&id) {
                    state.active_traces.remove(active_position);
                    true
                } else {
                    false
                }
            });
            if !removed {
                let error: tcl_cmd_core::CmdError =
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "original variable active-trace slot",
                    )
                    .into();
                variable_cleanup::retain(self, &error);
            }
            if selected
                .as_ref()
                .is_some_and(|selected| original.has_cell(self, selected))
            {
                let _ =
                    original.retire(self, crate::vars::VariableRetirement::release_operation(id));
            }
        }
        let current = original.check_native(self);
        match reached {
            Err(first) => Err(first),
            Ok(guest) => {
                current?;
                Ok(guest)
            }
        }
    }

    /// Select the parent-array and reached-cell trace groups in Tcl order.
    /// Tcl 9 also recovers the parent and element name through a scalar-looking
    /// alias to an array element; the release policy owns that distinction.
    pub(in super::super) fn variable_trace_groups(
        &self,
        element_access: bool,
        cell: &VarTraceCell,
        array_active: bool,
        taken: Option<TakenVarTraces>,
        op: &str,
    ) -> Vec<TraceGroup> {
        let mut groups = Vec::with_capacity(2);
        let include_array = taken.as_ref().is_none_or(|taken| taken.include_array);
        if element_access {
            if include_array
                && !array_active
                && let Some(array) = cell.array_id()
            {
                groups.push(TraceGroup::Live(array));
            }
        } else if self.runtime_version.traces_recover_linked_array_element()
            && cell.elem.is_some()
            && op != "array"
            && include_array
            && !array_active
            && let Some(array) = cell.array_id()
        {
            groups.push(TraceGroup::Live(array));
        }
        match taken {
            Some(taken) => groups.push(TraceGroup::Taken(taken.entries)),
            None => {
                if let Some(id) = cell.id {
                    groups.push(TraceGroup::Live(id));
                }
            }
        }
        groups
    }

    /// Inner firing loop for [`Self::fire_var_traces_from_parts`]: run the whole-array
    /// traces, then the element-specific ones, with the variable already
    /// marked active by the caller.
    fn invoke_variable_trace_callback(
        &mut self,
        trace: VarTrace,
        invocation: VarTraceInvocation<'_>,
        cell: &VarTraceCell,
        policy: tcl_syntax::naming::NamePolicyProtocol,
        scope: &native_operation_currency::NativeOperationScope,
    ) -> Result<(Completion<Value>, Option<Value>), tcl_runtime_api::NativeExecutionError> {
        scope.check_native(self)?;
        if self.observed_names.is_some() {
            return Err(self.retain_host_command_refusal(
                "observed variable trace operation is unavailable".into(),
            ));
        }
        let VarTraceInvocation {
            op,
            reported: (name1, elem),
            ..
        } = invocation;
        let command = trace.command;
        let old_style = trace.old_style;
        let explicit_unset_element = (op == "unset"
            && policy
                .recipe()
                .tcl_version()
                .is_some_and(|version| version >= tcl_dialect::TclVersion::V9_0))
        .then_some(cell.elem.as_deref())
        .flatten();
        let reported = tcl_syntax::naming::report_native_variable_access_trace_names(
            policy.recipe(),
            tcl_syntax::naming::NativeVariableTraceReportingInput {
                original_part1: name1,
                original_part2: elem.or(explicit_unset_element),
                actual_element_key: cell.elem.as_deref(),
            },
        )
        .map_err(|error| {
            self.retain_host_command_refusal(format!(
                "variable trace reporting is unavailable: {error:?}"
            ))
        })?;
        let completion = if let Some((_, observer)) = trace.native {
            use tcl_runtime_api::native_variable_trace::{
                NativeVariableTraceAccess, NativeVariableTraceOperation,
            };
            let operation =
                NativeVariableTraceOperation::from_name(op).expect("selected trace operation");
            let observed = observer.observe(
                self,
                NativeVariableTraceAccess {
                    operation,
                    name1: reported.name1,
                    name2: reported.name2,
                    destroyed: operation == NativeVariableTraceOperation::Unset,
                },
            );
            if let Err(error) = &observed {
                if let Some(first) = error.native_execution_refusal() {
                    self.retain_tcl_host_failure(crate::error::TclHostFailure::Execution(
                        first.clone(),
                    ));
                } else if let Some(first) = error.native_access_refusal() {
                    self.retain_tcl_host_failure(crate::error::TclHostFailure::ValueAccess(first));
                }
            }
            scope.check_native(self)?;
            match observed {
                Ok(()) => ok(Value::empty()),
                Err(error) => self.variable_trace_command_error(error)?,
            }
        } else {
            self.invoke_script_variable_trace_callback(
                command.as_ref().expect("script trace owns its command"),
                op,
                old_style,
                (reported.name1, reported.name2),
                scope,
            )?
        };
        scope.check_native(self)?;
        Ok((completion, command))
    }

    fn invoke_script_variable_trace_callback(
        &mut self,
        command: &Value,
        op: &str,
        old_style: bool,
        reported: (&[u8], &[u8]),
        scope: &native_operation_currency::NativeOperationScope,
    ) -> Result<Completion<Value>, tcl_runtime_api::NativeExecutionError> {
        let original_names = match native_name_consumers::OriginalNameConsumerScope::enter(self) {
            Ok(scope) => scope,
            Err(error) => return self.variable_trace_command_error(error),
        };
        let reached = (|| {
            scope.check_native(self)?;
            let original = original_names.read(self, command);
            scope.check_native(self)?;
            let mut script = match original {
                Ok(bytes) => bytes.to_vec(),
                Err(error) => return self.variable_trace_command_error(error),
            };
            for argument in [
                reported.0,
                reported.1,
                tcl_cmd_core::trace::callback_op_word(op, old_style).as_bytes(),
            ] {
                script.push(b' ');
                tcl_syntax::list::append_list_element(&mut script, argument, false);
            }
            let protocol = self
                .actual_native_invocation_dialect()
                .native_variable_trace_protocol()
                .ok_or_else(|| {
                    self.retain_host_command_refusal(
                        "variable trace callback source protocol is unavailable".into(),
                    )
                })?;
            let source = Value::new_native_string_bytes(protocol.variable_callback_source(&script));
            let saved = self.save_native_script_trace_result()?;
            let reached = (|| {
                scope.check_native(self)?;
                self.reset_native_error_objects_before_trace_script();
                scope.check_native(self)?;
                let evaluated = self.eval_original_script_value(
                    &source,
                    tcl_native_model::native_eval_object::EvalObjectPurpose::TraceCallback,
                    None,
                );
                let mut completion = match evaluated {
                    Ok(completion) | Err(crate::error::TclError::Guest(completion)) => completion,
                    Err(crate::error::TclError::Host(first)) => {
                        return Err(self.retain_tcl_host_failure(first));
                    }
                };
                scope.check_native(self)?;
                if !completion.code.is_ok() {
                    completion.result = completion.result.into_native_reference();
                    scope.check_native(self)?;
                }
                Ok(completion)
            })();
            let cleanup = if let Some(saved) = saved {
                self.restore_native_interp_trace_result(saved)
            } else {
                Ok(())
            };
            let current = scope.check_native(self);
            match reached {
                Err(first) => Err(first),
                Ok(completion) => {
                    cleanup?;
                    current?;
                    Ok(completion)
                }
            }
        })();
        if let Err(error) = original_names.finish(self) {
            return self.variable_trace_command_error(error);
        }
        reached
    }

    fn original_trace_failure_message(
        &mut self,
        original: &Value,
        native_result: bool,
        scope: &native_operation_currency::NativeOperationScope,
    ) -> Result<Rc<[u8]>, tcl_cmd_core::CmdError> {
        scope.check(self)?;
        let reached = if native_result {
            let protocol = self.actual_native_string_protocol().ok_or(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "original variable trace result String protocol",
                ),
            )?;
            let objects = native_numeric_ops::OriginalNumericOps::in_original_scope(self, scope)?;
            objects.string(original, protocol)
        } else {
            // Authored presentation retains its independently selected value semantics.
            Ok(original.string_bytes())
        };
        scope.check(self)?;
        reached
    }

    fn publish_variable_trace_failure_code(
        &mut self,
        mut failure: Completion<Value>,
        op: &str,
        name: &[u8],
        scope: &native_operation_currency::NativeOperationScope,
    ) -> Result<
        std::ops::ControlFlow<Completion<Value>, Completion<Value>>,
        tcl_runtime_api::NativeExecutionError,
    > {
        // From 8.6 the failed access sets the bare `TCL READ|WRITE VARNAME`
        // code (`tclVar.c` 9.0.4:1472, :2073); 8.4 and 8.5 keep whatever code
        // the callback left.
        if op != "array"
            && (self.native_c_variable_name_protocol().is_none()
                || self.runtime_version >= tcl_dialect::TclVersion::V8_6)
        {
            let kind = if op == "write" { "WRITE" } else { "READ" };
            let mut words = vec![
                Value::string("TCL"),
                Value::string(kind),
                Value::string("VARNAME"),
            ];
            if self.native_c_variable_name_protocol().is_none() {
                words.push(Value::from_string_bytes(name));
            }
            let code = Value::list(words);
            scope.check_native(self)?;
            // `Tcl_SetErrorCode`: the interpreter's code is the access's, not
            // the one the callback's `return -errorcode` left.
            if self.native_c_variable_name_protocol().is_some() {
                self.restore_guest_error_code(code.clone());
                scope.check_native(self)?;
            }
            let options = match crate::command::completion_options(self, &failure) {
                Ok(options) => options,
                Err(crate::error::TclError::Host(first)) => {
                    return Err(self.retain_tcl_host_failure(first));
                }
                Err(crate::error::TclError::Guest(completion)) => {
                    return Ok(std::ops::ControlFlow::Break(completion));
                }
            };
            scope.check_native(self)?;
            let selected = crate::command::with_return_option(self, &options, "-errorcode", code);
            scope.check_native(self)?;
            failure.options = match selected {
                Ok(options) => options,
                Err(crate::error::TclError::Host(first)) => {
                    return Err(self.retain_tcl_host_failure(first));
                }
                Err(crate::error::TclError::Guest(completion)) => {
                    return Ok(std::ops::ControlFlow::Break(completion));
                }
            };
        }
        Ok(std::ops::ControlFlow::Continue(failure))
    }

    pub(super) fn present_variable_trace_failure(
        &mut self,
        mut failure: Completion<Value>,
        command: Option<&Value>,
        op: &str,
        name: &[u8],
        scope: &native_operation_currency::NativeOperationScope,
    ) -> Result<Completion<Value>, tcl_runtime_api::NativeExecutionError> {
        scope.check_native(self)?;
        let native_result = self
            .native_c_variable_name_protocol()
            .is_some_and(|protocol| protocol.version() >= tcl_dialect::TclVersion::V8_5);
        if native_result {
            // TclCallVarTraces first installs the actual returned trace result.
            let reached = self.adopt_native_interp_result(failure.result.clone());
            scope.check_native(self)?;
            let original =
                reached.map_err(|error| self.retain_variable_trace_value_error(&error))?;
            self.observe_native_error_result(original.value());
            drop(original);
            scope.check_native(self)?;
        }
        if native_result
            && self
                .native_errors
                .error_info
                .as_ref()
                .is_none_or(Vec::is_empty)
        {
            let reached =
                self.original_trace_failure_message(&failure.result, native_result, scope);
            let original_message = match reached {
                Ok(bytes) => bytes,
                Err(error) => {
                    return match self.variable_trace_command_error(error) {
                        Ok(guest) => Ok(guest),
                        Err(first) => Err(first),
                    };
                }
            };
            // Tcl_AddErrorInfo("") initializes from the already published result.
            self.seed_error_info_frame(&original_message, b"");
        } else if native_result {
            // The actual script callback's existing episode owns its metadata.
            self.seed_error_info_frame(b"", b"");
        }
        scope.check_native(self)?;
        let was_error = failure.code == Code::Error;
        if !was_error && let Some(command) = command {
            self.error_stack_restart_with_inner(command.clone());
        }
        // C's `TclCallVarTraces` logs a `(write|read trace
        // on "name")` frame, then clears ERR_ALREADY_LOGGED
        // so the command that triggered the trace logs its
        // own `invoked from within` frame as the error
        // unwinds (set-2.4 / set-4.4); 8.4 drops the callback's errorInfo
        // instead ([`tcl_dialect::TclVersion::has_var_trace_error_frames`]).
        if self.uses_jim_error_stack() || self.runtime_version.has_var_trace_error_frames() {
            self.append_var_trace_frame_bytes(op, name);
        } else {
            self.native_errors.error_info = None;
            self.native_errors.error_logged = ErrorLogged::No;
        }
        scope.check_native(self)?;
        let verb = match op {
            "write" => "set",
            "array" => "trace array",
            _ => "read",
        };
        let mut message = format!("can't {verb} \"").into_bytes();
        message.extend_from_slice(name);
        message.extend_from_slice(b"\": ");
        let original_message =
            match self.original_trace_failure_message(&failure.result, native_result, scope) {
                Ok(bytes) => bytes,
                Err(error) => {
                    return match self.variable_trace_command_error(error) {
                        Ok(guest) => Ok(guest),
                        Err(first) => Err(first),
                    };
                }
            };
        message.extend_from_slice(&original_message);
        settle_trace_failure_code(&mut failure, was_error);
        failure.result = Value::from_string_bytes(message);
        if self
            .native_c_variable_name_protocol()
            .and_then(tcl_syntax::native_variable_name::NativeVariableNameProtocol::diagnostic_string_protocol)
            .is_some()
        {
            let recipe = self
                .actual_native_invocation_dialect()
                .native_string_materialization(None)
                .ok_or_else(|| {
                    self.retain_host_command_refusal(
                        "actual variable-trace String producer".into(),
                    )
                })?;
            failure
                .result
                .retain_native_string_representation(recipe)
                .map_err(|error| self.retain_variable_trace_value_error(&error))?;
            scope.check_native(self)?;
        }

        failure = match self.publish_variable_trace_failure_code(failure, op, name, scope)? {
            std::ops::ControlFlow::Break(completion) => return Ok(completion),
            std::ops::ControlFlow::Continue(failure) => failure,
        };
        if native_result {
            let reached = self.adopt_native_interp_result(failure.result.clone());
            scope.check_native(self)?;
            let original =
                reached.map_err(|error| self.retain_variable_trace_value_error(&error))?;
            drop(original);
        }
        scope.check_native(self)?;
        Ok(failure)
    }

    fn fire_variable_trace_groups(
        &mut self,
        groups: Vec<TraceGroup>,
        invocation: VarTraceInvocation<'_>,
        cell: &VarTraceCell,
        policy: tcl_syntax::naming::NamePolicyProtocol,
        scope: &native_operation_currency::NativeOperationScope,
    ) -> VariableTraceOutcome {
        let VarTraceInvocation {
            leave_error_message,
            name,
            op,
            ..
        } = invocation;
        for group in groups {
            scope.check_native(self)?;
            let steps: Vec<Step> = match &group {
                TraceGroup::Live(key) => self
                    .variable_observers
                    .script_traces
                    .get(key)
                    .map_or_else(Vec::new, |list| {
                        list.iter().rev().map(|t| Step::Live(t.id)).collect()
                    }),
                TraceGroup::Taken(list) => list
                    .iter()
                    .rev()
                    .filter(|t| t.ops.iter().any(|o| o == op))
                    .map(|t| Step::Taken(t.clone()))
                    .collect(),
            };
            let live_key = match &group {
                TraceGroup::Live(key) => Some(*key),
                TraceGroup::Taken(_) => None,
            };
            for step in steps {
                scope.check_native(self)?;
                let entry = match step {
                    Step::Live(id) => self
                        .variable_observers
                        .script_traces
                        .get(&live_key.expect("live trace group has a key"))
                        .and_then(|list| list.iter().find(|t| t.id == id))
                        .filter(|t| t.ops.iter().any(|o| o == op))
                        .cloned(),
                    Step::Taken(trace) => Some(trace),
                };
                let Some(trace) = entry else {
                    continue;
                };
                let (completion, command) =
                    self.invoke_variable_trace_callback(trace, invocation, cell, policy, scope)?;
                scope.check_native(self)?;
                if !completion.code.is_ok() {
                    if !leave_error_message {
                        return Ok(Err(err("")));
                    }
                    if matches!(op, "write" | "read" | "array") {
                        let failure = self.present_variable_trace_failure(
                            completion,
                            command.as_ref(),
                            op,
                            name,
                            scope,
                        )?;
                        return Ok(Err(failure));
                    }
                    // Genuine unset Guest errors are ignored; dropping their original results remains a reached stage.
                    drop(completion);
                    scope.check_native(self)?;
                }
            }
            if let Some(id) = live_key {
                self.fire_native_error_variable_trace(id, op);
                scope.check_native(self)?;
            }
            if live_key.is_some() && live_key == self.variable_observers.precision_cell {
                let precision = self.fire_native_precision_trace_bytes(name, op, cell, scope)?;
                scope.check_native(self)?;
                if let Err(guest) = precision {
                    return Ok(Err(guest));
                }
            }
            // The host-environment hook is the `env` array's oldest
            // trace: it runs after the script traces of the array's group
            // and before the element's own (tclsh 8.6 and 9.0).
            if live_key.is_some() && live_key == self.variable_observers.env_cell {
                let env = self.fire_native_env_trace(name, op, cell, scope)?;
                scope.check_native(self)?;
                if let Err(guest) = env {
                    return Ok(Err(guest));
                }
            }
        }
        Ok(Ok(()))
    }

    fn fire_var_traces_inner(
        &mut self,
        invocation: VarTraceInvocation<'_>,
        cell: &VarTraceCell,
        array_active: bool,
        taken: Option<TakenVarTraces>,
        scope: &native_operation_currency::NativeOperationScope,
    ) -> VariableTraceOutcome {
        scope.check_native(self)?;
        let VarTraceInvocation {
            leave_error_message,
            element_access,
            op,
            ..
        } = invocation;
        let groups = self.variable_trace_groups(element_access, cell, array_active, taken, op);
        let policy = self.name_policy_protocol().ok_or_else(|| {
            self.retain_host_command_refusal(
                "native variable trace reporting policy is unavailable".into(),
            )
        })?;
        let observers = &self.variable_observers;
        let callbacks_possible = groups.iter().any(|group| match group {
            TraceGroup::Live(key) => {
                observers
                    .script_traces
                    .get(key)
                    .is_some_and(|list| !list.is_empty())
                    || observers.native_error_cells.contains_key(key)
                    || observers.precision_cell == Some(*key)
                    || observers.env_cell == Some(*key)
            }
            TraceGroup::Taken(list) => !list.is_empty(),
        });
        let saved = if callbacks_possible {
            self.save_native_interp_trace_result()?
        } else {
            None
        };
        let reached = self.fire_variable_trace_groups(groups, invocation, cell, policy, scope);
        let restore = !matches!(reached, Ok(Err(_))) || !leave_error_message;
        let cleanup = if let Some(saved) = saved {
            if restore {
                self.restore_native_interp_trace_result(saved)
            } else {
                drop(saved);
                Ok(())
            }
        } else {
            Ok(())
        };
        let current = scope.check_native(self);
        match reached {
            Err(first) => Err(first),
            Ok(guest) => {
                cleanup?;
                current?;
                Ok(guest)
            }
        }
    }
}

/// A variable trace failure is the access's own level-0 error: a callback's
/// merged control code does not survive into the options it reports.
fn settle_trace_failure_code(failure: &mut Completion<Value>, was_error: bool) {
    failure.code = Code::Error;
    if !was_error
        && let tcl_core_types::CompletionOptionOrigin::MergedReturnOptions { .. } =
            failure.option_origin
    {
        failure.option_origin =
            tcl_core_types::CompletionOptionOrigin::MergedReturnOptions { code: 1, level: 0 };
    }
}
