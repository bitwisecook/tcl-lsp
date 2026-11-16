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

//! Jim's error stack: capturing frames at the error, the stacktrace object, and the return receipt with its captured error code.

use super::super::native_error_variables::OriginalJimErrorCodeCapture;
use super::super::{
    native_operation_currency, new_string, Code, Interp, OriginalJimCapturedOptionPairs,
};
use crate::obj;
use tcl_runtime_api::jim_error_stack::{
    select_jim_error_frame_sources, JimErrorFrame, JimErrorTrace, JimEvaluationFrame,
    JimScriptLocation, NativeErrorStackProtocol,
};

impl Interp {
    pub(crate) fn uses_jim_error_stack(&self) -> bool {
        self.native_invocation_dialect().error_stack_protocol()
            == Some(NativeErrorStackProtocol::Jim084)
    }

    /// Retain only selected original objects at actual error capture. Merely
    /// observing a frame must not change argv sharing or read an unused key.
    fn materialized_jim_error_frames(
        &self,
        script_failure: Option<&JimScriptLocation<obj::Owned>>,
    ) -> Result<Option<Vec<JimErrorFrame<obj::Owned>>>, tcl_runtime_api::NativeExecutionError> {
        let empty = self
            .native_jim_object_context()
            .map_err(|error| self.clone().refuse_completion_value_access(error))?
            .empty_object()
            .clone();
        let level = self.jim_procedure_level.get();
        if level == 0 {
            if let Some(script) = script_failure {
                return Ok(Some(vec![JimErrorFrame {
                    procedure: empty.clone(),
                    file: script.file.clone(),
                    line: script.line,
                    invocation: empty,
                }]));
            }
        }
        let frames = self.jim_evaluation_frames.borrow();
        let selections = {
            let world = self.namespaces.borrow();
            let metadata: Vec<_> = frames
                .iter()
                .rev()
                .map(|frame| JimEvaluationFrame {
                    procedure_level: frame.procedure_level,
                    command_name: frame.command.and_then(|command| {
                        world
                            .original_jim_command_name_observation(command)
                            .map(|_| ())
                    }),
                    is_procedure: frame.is_procedure,
                    script: None,
                    invocation: (),
                })
                .collect();
            select_jim_error_frame_sources(level, &metadata)
        };
        let Some(selections) = selections else {
            return Ok(None);
        };
        let borrows = self.jim_invocation_borrows.borrow();
        let mut result = Vec::new();
        for selection in selections {
            let index = frames.len() - 1 - selection.frame;
            let frame = &frames[index];
            let procedure = if let Some(selected) = selection.procedure {
                let command = frames[frames.len() - 1 - selected]
                    .command
                    .expect("selected observed command");
                let world = self.namespaces.borrow();
                let observation = world
                    .original_jim_command_name_observation(command)
                    .expect("selected original command-name observation");
                let original = observation
                    .current_ptr()
                    .map_err(|error| self.clone().refuse_completion_value_access(error))?;
                obj::Owned::retain(original)
            } else {
                empty.clone()
            };
            let invocation = borrows
                .iter()
                .rev()
                .find(|view| view.frame_index == index && !view.argv.is_empty())
                .map_or_else(
                    || obj::Owned::fresh(new_string(&frame.invocation)),
                    |view| obj::Owned::fresh(self.new_list_object(&view.argv)),
                );
            let file = frame.script.as_ref().map_or_else(
                || empty.clone(),
                |script| {
                    borrows
                        .iter()
                        .rev()
                        .find_map(|view| {
                            (view.frame_index == index)
                                .then_some(view.script_filename)
                                .flatten()
                        })
                        .map_or_else(
                            || obj::Owned::fresh(new_string(&script.file)),
                            obj::Owned::retain,
                        )
                },
            );
            result.push(JimErrorFrame {
                procedure,
                file,
                line: frame.script.as_ref().map_or(1, |script| script.line),
                invocation,
            });
        }
        Ok(Some(result))
    }

    fn jim_trace_object(
        &self,
        frames: Vec<tcl_runtime_api::jim_error_stack::JimErrorFrame<obj::Owned>>,
    ) -> obj::Owned {
        let values: Vec<_> = frames
            .into_iter()
            .flat_map(|frame| {
                [
                    frame.procedure,
                    frame.file,
                    obj::Owned::fresh(obj::new_wide_int_obj(i64::from(frame.line))),
                    frame.invocation,
                ]
            })
            .collect();
        let pointers: Vec<_> = values.iter().map(obj::Owned::as_ptr).collect();
        obj::Owned::fresh(self.new_list_object(&pointers))
    }

    pub(in super::super) fn capture_jim_error_stack(
        &self,
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        self.capture_jim_error_stack_at(None)
    }

    pub(in super::super) fn capture_jim_error_stack_at(
        &self,
        script: Option<&JimScriptLocation<obj::Owned>>,
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        let scope = native_operation_currency::NativeOperationScope::enter(self)?;
        let ticket = self.jim_error_stack.borrow().capture_ticket();
        let Some(ticket) = ticket else {
            return Ok(());
        };
        let captured = self
            .materialized_jim_error_frames(script)?
            .map(|frames| self.jim_trace_object(frames));
        if let Err(first) = scope.currency().ensure_current_or_refuse() {
            drop(captured);
            let _ = scope.currency().ensure_current_or_refuse();
            return Err(self.native_execution_refusal().unwrap_or(first));
        }
        if let Some(captured) = captured {
            let retired = self
                .jim_error_stack
                .borrow_mut()
                .publish_capture(&ticket, JimErrorTrace::Explicit(captured));
            // Both old and declined original traces retire outside the borrow.
            drop(retired);
        }
        scope.currency().ensure_current_or_refuse()
    }

    pub(in super::super) fn capture_jim_script_parse_failure(
        &self,
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        let scope = native_operation_currency::NativeOperationScope::enter(self)?;
        let script = self
            .cmd_frames
            .borrow()
            .last()
            .map(|frame| JimScriptLocation {
                file: obj::Owned::fresh(new_string(frame.file.as_deref().unwrap_or(b""))),
                line: frame.line_base + 1,
            })
            .unwrap_or_else(|| JimScriptLocation {
                file: obj::Owned::fresh(new_string(b"")),
                line: 1,
            });
        let reached = self.capture_jim_error_stack_at(Some(&script));
        drop(script);
        scope.currency().ensure_current_or_refuse()?;
        reached
    }

    pub(crate) fn jim_stacktrace_object(
        &self,
    ) -> Result<obj::Owned, tcl_runtime_api::NativeExecutionError> {
        let scope = native_operation_currency::NativeOperationScope::enter(self)?;
        let trace = self.jim_error_stack.borrow().trace().cloned();
        let value = match trace {
            Some(JimErrorTrace::Explicit(value)) => value,
            Some(JimErrorTrace::Automatic(frames)) => self.jim_trace_object(frames),
            None => obj::Owned::fresh(self.new_list_object(&[])),
        };
        if let Err(first) = scope.currency().ensure_current_or_refuse() {
            drop(value);
            let _ = scope.currency().ensure_current_or_refuse();
            return Err(self.native_execution_refusal().unwrap_or(first));
        }
        Ok(value)
    }

    pub(crate) fn jim_stacktrace(&self) -> Result<Vec<u8>, tcl_runtime_api::NativeExecutionError> {
        let scope = native_operation_currency::NativeOperationScope::enter(self)?;
        let original = self.jim_stacktrace_object()?;
        let reached = self.native_object_string_bytes(original.as_ptr());
        let checked = scope.currency().ensure_current_or_refuse();
        drop(original);
        scope.currency().ensure_current_or_refuse()?;
        checked?;
        reached
            .map(|bytes| bytes.to_vec())
            .map_err(|error| self.clone().refuse_completion_value_access(error))
    }

    pub(crate) fn reset_jim_error_capture(&self) {
        self.jim_error_stack.borrow_mut().mark_reset();
    }

    pub(crate) fn adopt_jim_stacktrace(
        &self,
        original: obj::Owned,
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        let saved = native_operation_currency::SavedNativeOperationCurrency::capture(self);
        let cleanup =
            native_operation_currency::NativeOperationScope::enter_saved_cleanup(&saved, self)?;
        if let Err(first) = cleanup.check() {
            // The consumed original argument still has mandatory retirement.
            // Keep its actual failed entry attached through that callback.
            drop(original);
            let _ = cleanup.check();
            return Err(self.native_execution_refusal().unwrap_or(first));
        }
        let retired = self.jim_error_stack.borrow_mut().replace_explicit(original);
        drop(retired);
        cleanup.check()
    }

    pub(crate) fn jim_return_receipt(
        &self,
    ) -> Result<
        tcl_runtime_api::jim_return_state::JimReturnReceipt<obj::Owned>,
        tcl_runtime_api::NativeExecutionError,
    > {
        let captured = self
            .clone()
            .capture_original_jim_error_code()
            .map_err(|error| {
                error
                    .native_execution_refusal()
                    .cloned()
                    .unwrap_or_else(|| {
                        self.clone().refuse_completion_value_access(
                            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                                "original Jim error-code receipt capture",
                            ),
                        )
                    })
            })?;
        self.jim_return_receipt_with_captured_error_code(&captured)
    }

    pub(crate) fn jim_return_receipt_with_captured_error_code(
        &self,
        captured: &OriginalJimErrorCodeCapture,
    ) -> Result<
        tcl_runtime_api::jim_return_state::JimReturnReceipt<obj::Owned>,
        tcl_runtime_api::NativeExecutionError,
    > {
        let scope = native_operation_currency::NativeOperationScope::enter(self)?;
        let error_code = captured.retain_for_receipt(self).map_err(|error| {
            error
                .native_execution_refusal()
                .cloned()
                .unwrap_or_else(|| {
                    self.clone().refuse_completion_value_access(
                        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "original Jim captured error-code receipt",
                        ),
                    )
                })
        })?;
        if let Err(first) = scope.currency().ensure_current_or_refuse() {
            drop(error_code);
            let _ = scope.currency().ensure_current_or_refuse();
            return Err(self.native_execution_refusal().unwrap_or(first));
        }
        let stack_trace = match self.jim_stacktrace_object() {
            Ok(value) => value,
            Err(first) => {
                drop(error_code);
                let _ = scope.currency().ensure_current_or_refuse();
                return Err(self.native_execution_refusal().unwrap_or(first));
            }
        };
        if let Err(error) = captured.original(self) {
            drop(error_code);
            drop(stack_trace);
            return Err(error
                .native_execution_refusal()
                .cloned()
                .unwrap_or_else(|| {
                    self.clone().refuse_completion_value_access(
                        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "original Jim captured error-code publication",
                        ),
                    )
                }));
        }
        Ok(tcl_runtime_api::jim_return_state::JimReturnReceipt {
            pending: tcl_runtime_api::jim_return_state::JimReturnState {
                code: self.return_code.get().as_int() as i32,
                level: i64::try_from(self.return_level.get()).unwrap_or(i64::MAX),
            },
            error_code,
            stack_trace,
        })
    }

    pub(crate) fn jim_option_pairs_with_captured_error_code(
        &self,
        captured: &OriginalJimErrorCodeCapture,
        exit: Code,
    ) -> Result<OriginalJimCapturedOptionPairs, tcl_runtime_api::NativeExecutionError> {
        let scope = native_operation_currency::NativeOperationScope::enter(self)?;
        let current = || {
            captured.check_context(self).map_err(|error| {
                error
                    .native_execution_refusal()
                    .cloned()
                    .unwrap_or_else(|| {
                        self.clone().refuse_completion_value_access(
                            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                                "original Jim captured options context",
                            ),
                        )
                    })
            })
        };
        current()?;
        let exit = tcl_runtime_api::Code::from_int(exit.as_int() as i32);
        let pairs =
            if tcl_runtime_api::jim_return_state::JimReturnReceipt::<()>::uses_error_metadata(exit)
            {
                self.jim_return_receipt_with_captured_error_code(captured)?
                    .option_pairs(exit)
            } else {
                tcl_runtime_api::jim_return_state::JimReturnState {
                    code: self.return_code.get().as_int() as i32,
                    level: i64::try_from(self.return_level.get()).unwrap_or(i64::MAX),
                }
                .option_pairs(exit)
            };
        scope.currency().ensure_current_or_refuse()?;
        current()?;
        Ok(pairs)
    }
}
