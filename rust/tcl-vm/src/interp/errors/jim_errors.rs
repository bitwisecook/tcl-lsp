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

//! Jim's error state: the private return state and its receipts, return options, the error stack and `stacktrace`, and script parse failures.

use super::super::{
    OriginalJimEvaluationFrame, Vm, err, native_jim_lookup, native_numeric_ops,
    native_operation_currency, oo_cleanup,
};
use crate::value::Value;
use tcl_runtime_api::jim_error_stack::{
    JimErrorFrame, JimErrorTrace, JimEvaluationFrame, NativeErrorStackProtocol,
    select_jim_error_frame_sources,
};
use tcl_runtime_api::{Code, Completion, FatalTail};

impl Vm {
    /// Write the private counters only after a reached Jim return command.
    pub(crate) fn set_jim_return_state(&mut self, code: i32, level: i64) {
        self.jim_errors.pending_return =
            tcl_runtime_api::jim_return_state::JimReturnState { code, level };
    }

    /// Capture live private state independently of the latest command's options.
    pub(crate) fn jim_return_receipt(
        &mut self,
    ) -> Result<tcl_runtime_api::jim_return_state::JimReturnReceipt<Value>, tcl_cmd_core::CmdError>
    {
        let operation = native_operation_currency::NativeOperationScope::enter(self)?;
        let error_code = self.read_original_jim_global_error_code().map_err(|_| {
            self.execution_refusal.clone().map_or_else(
                || {
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "original Jim global error-code receipt",
                    )
                    .into()
                },
                tcl_cmd_core::CmdError::from_execution_refusal,
            )
        })?;
        operation.check(self)?;
        if let Some(value) = &error_code {
            native_numeric_ops::OriginalNumericOps::in_original_scope(self, &operation)?
                .current_for_value(value)?;
        }
        let error_code = error_code.map(Value::into_native_reference);
        operation.check(self)?;
        if let Some(value) = &error_code {
            native_numeric_ops::OriginalNumericOps::in_original_scope(self, &operation)?
                .current_for_value(value)?;
        }
        operation.check(self)?;
        self.jim_return_receipt_with_captured_error_code(error_code, &operation)
    }

    /// An earlier reached catch/try getter already selected this exact value.
    /// Building its options must not perform another global-name lookup.
    fn jim_return_receipt_with_captured_error_code(
        &mut self,
        error_code: Option<Value>,
        operation: &native_operation_currency::NativeOperationScope,
    ) -> Result<tcl_runtime_api::jim_return_state::JimReturnReceipt<Value>, tcl_cmd_core::CmdError>
    {
        operation.check(self)?;
        let stack_trace = self.jim_stacktrace();
        {
            let objects =
                native_numeric_ops::OriginalNumericOps::in_original_scope(self, operation)?;
            if let Some(value) = &error_code {
                objects.current_for_value(value)?;
            }
            objects.current_for_value(&stack_trace)?;
        }
        operation.check(self)?;
        Ok(tcl_runtime_api::jim_return_state::JimReturnReceipt {
            pending: self.jim_errors.pending_return,
            error_code,
            stack_trace,
        })
    }

    /// Jim catch/try options observe live private state for this raw exit code.
    pub(crate) fn current_jim_options_for_exit_code(
        &mut self,
        exit: Code,
    ) -> Result<Value, tcl_cmd_core::CmdError> {
        if !self.uses_jim_error_stack() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "Jim return state",
            )
            .into());
        }
        self.jim_return_options(exit)
    }

    pub(crate) fn current_jim_options_for_exit_code_with_error_code(
        &mut self,
        exit: Code,
        captured: &native_jim_lookup::OriginalJimErrorCodeCapture,
    ) -> Result<Value, tcl_cmd_core::CmdError> {
        if !self.uses_jim_error_stack() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "Jim return state",
            )
            .into());
        }
        let operation = native_operation_currency::NativeOperationScope::enter(self)?;
        captured.check(self)?;
        if !tcl_runtime_api::jim_return_state::JimReturnReceipt::<Value>::uses_error_metadata(exit)
        {
            let pairs = self.jim_errors.pending_return.option_pairs::<Value>(exit);
            operation.check(self)?;
            captured.check(self)?;
            return Ok(Self::jim_option_pairs(pairs));
        }
        let original = captured.original(self)?;
        let receipt = self.jim_return_receipt_with_captured_error_code(original, &operation)?;
        captured.check(self)?;
        Ok(Self::jim_receipt_options(&receipt, exit))
    }

    fn jim_receipt_options(
        receipt: &tcl_runtime_api::jim_return_state::JimReturnReceipt<Value>,
        exit: Code,
    ) -> Value {
        Self::jim_option_pairs(receipt.option_pairs(exit))
    }

    fn jim_option_pairs(
        pairs: Vec<(
            &'static [u8],
            tcl_runtime_api::completion_options::OptionValue<Value>,
        )>,
    ) -> Value {
        use tcl_runtime_api::completion_options::OptionValue;
        Value::list(
            pairs
                .into_iter()
                .flat_map(|(key, value)| {
                    [
                        Value::from_string_bytes(key),
                        match value {
                            OptionValue::Integer(integer) => Value::int(integer),
                            OptionValue::Value(original) => original,
                        },
                    ]
                })
                .collect(),
        )
    }

    pub(crate) fn jim_return_options(
        &mut self,
        exit: Code,
    ) -> Result<Value, tcl_cmd_core::CmdError> {
        Ok(Self::jim_receipt_options(&self.jim_return_receipt()?, exit))
    }

    pub(crate) fn uses_jim_error_stack(&self) -> bool {
        self.actual_native_protocols().error_stack == Some(NativeErrorStackProtocol::Jim084)
    }

    pub(crate) fn borrow_jim_evaluation_frame(
        frame: &OriginalJimEvaluationFrame,
    ) -> OriginalJimEvaluationFrame {
        OriginalJimEvaluationFrame {
            evaluation_frame_entered: frame.evaluation_frame_entered,
            procedure_level: frame.procedure_level,
            command: frame.command.clone(),
            is_procedure: frame.is_procedure,
            script: frame.script.as_ref().map(|location| {
                tcl_runtime_api::jim_error_stack::JimScriptLocation {
                    file: location.file.native_lifetime_lease().into_value(),
                    line: location.line,
                }
            }),
            invocation: frame.invocation.native_lifetime_lease().into_value(),
        }
    }

    pub(in super::super) fn materialize_original_jim_error_frames(
        &self,
        level: u32,
        script_failure: Option<&tcl_runtime_api::jim_error_stack::JimScriptLocation<Value>>,
    ) -> Result<Option<Vec<JimErrorFrame<Value>>>, tcl_cmd_core::CmdError> {
        let empty = self
            .native_jim_object_context()?
            .empty_object()
            .native_lifetime_lease()
            .into_value();
        if let (0, Some(script)) = (level, script_failure) {
            return Ok(Some(vec![JimErrorFrame {
                procedure: empty.clone(),
                file: script.file.clone(),
                line: script.line,
                invocation: empty,
            }]));
        }
        let frames: Vec<_> = self
            .jim_errors
            .frames
            .iter()
            .rev()
            .filter(|frame| frame.evaluation_frame_entered)
            .collect();
        let metadata: Vec<_> = frames
            .iter()
            .map(|frame| JimEvaluationFrame {
                procedure_level: frame.procedure_level,
                command_name: frame
                    .command
                    .as_ref()
                    .filter(|name| name.is_observed())
                    .map(|_| ()),
                is_procedure: frame.is_procedure,
                script: None,
                invocation: (),
            })
            .collect();
        let Some(selected) = select_jim_error_frame_sources(level, &metadata) else {
            return Ok(None);
        };
        let mut captured = Vec::new();
        for selection in selected {
            let frame = frames[selection.frame];
            let procedure = match selection.procedure {
                Some(index) => frames[index]
                    .command
                    .as_ref()
                    .expect("selected actual command-name observation")
                    .original()?
                    .expect("selected original name"),
                None => empty.clone(),
            };
            captured.push(JimErrorFrame {
                procedure,
                file: frame
                    .script
                    .as_ref()
                    .map_or_else(|| empty.clone(), |location| location.file.clone()),
                line: frame.script.as_ref().map_or(1, |location| location.line),
                invocation: frame.invocation.capture_invocation_list(),
            });
        }
        Ok(Some(captured))
    }

    pub(crate) fn capture_jim_error_stack(&mut self) {
        self.capture_original_jim_error_stack_at(None);
    }

    fn capture_original_jim_error_stack_at(
        &mut self,
        script: Option<(
            u32,
            &tcl_runtime_api::jim_error_stack::JimScriptLocation<Value>,
        )>,
    ) {
        let Some(ticket) = self.jim_errors.stack.capture_ticket() else {
            return;
        };
        let scope = match native_operation_currency::NativeOperationScope::enter(self) {
            Ok(scope) => scope,
            Err(error) => {
                oo_cleanup::retain_error(self, &error);
                return;
            }
        };
        let level = script.map_or_else(
            || {
                self.jim_errors
                    .frames
                    .last()
                    .map_or(0, |frame| frame.procedure_level)
            },
            |(level, _)| level,
        );
        let captured =
            self.materialize_original_jim_error_frames(level, script.map(|(_, script)| script));
        let captured = match captured {
            Ok(frames) => frames.map(Self::jim_trace_object),
            Err(error) => {
                oo_cleanup::retain_error(self, &error);
                return;
            }
        };
        if let Err(error) = scope.check(self) {
            drop(captured);
            oo_cleanup::retain_error(self, &error);
            return;
        }
        if let Some(captured) = captured {
            let retired = self
                .jim_errors
                .stack
                .publish_capture(&ticket, JimErrorTrace::Explicit(captured));
            drop(retired);
        }
        if let Err(error) = scope.check(self) {
            oo_cleanup::retain_error(self, &error);
        }
    }

    pub(crate) fn capture_original_jim_script_parse_failure(
        &mut self,
        level: u32,
        location: &tcl_runtime_api::jim_error_stack::JimScriptLocation<Value>,
    ) {
        self.capture_original_jim_error_stack_at(Some((level, location)));
    }

    pub(crate) fn raise_jim_script_parse_failure(
        &mut self,
        tail: FatalTail,
        level: u32,
        location: Option<tcl_runtime_api::script_source_location::ScriptSourceLocation>,
    ) -> Completion<Value> {
        let script = location.map_or_else(
            || tcl_runtime_api::jim_error_stack::JimScriptLocation {
                file: Value::empty(),
                line: 1,
            },
            |location| tcl_runtime_api::jim_error_stack::JimScriptLocation {
                file: Value::string(location.file),
                line: location.line,
            },
        );
        self.capture_original_jim_error_stack_at(Some((level, &script)));
        err(tail.message)
    }

    fn jim_trace_object(
        frames: Vec<tcl_runtime_api::jim_error_stack::JimErrorFrame<Value>>,
    ) -> Value {
        Value::list(
            frames
                .into_iter()
                .flat_map(|frame| {
                    [
                        frame.procedure,
                        frame.file,
                        Value::int(i64::from(frame.line)),
                        frame.invocation,
                    ]
                })
                .collect(),
        )
    }

    pub(crate) fn jim_stacktrace(&self) -> Value {
        match self.jim_errors.stack.trace() {
            Some(JimErrorTrace::Explicit(value)) => value.clone(),
            Some(JimErrorTrace::Automatic(frames)) => Value::list(
                frames
                    .iter()
                    .flat_map(|frame| {
                        [
                            frame.procedure.clone(),
                            frame.file.clone(),
                            Value::int(i64::from(frame.line)),
                            frame.invocation.clone(),
                        ]
                    })
                    .collect(),
            ),
            None => Value::empty(),
        }
    }
}
