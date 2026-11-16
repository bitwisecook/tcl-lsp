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

//! Error state: `errorInfo` frames, the logged flag, parse and compilation failures, Jim return state and error stack, the TIP 348 error stack, publishing `errorInfo`/`errorCode`, and host refusals.

use super::native_error_stack::NativeErrorStack;
use super::{NativeCReturnState, NativeErrorState, Vm, err};
use crate::command::err_with_code;
use crate::value::Value;
use tcl_runtime_api::error_stack::{ErrorStackFrame, ShiftedErrorStackFrame, validate_error_stack};
use tcl_runtime_api::{Code, Completion, FatalTail};

mod error_info_frames;
mod host_refusals;
mod jim_errors;

impl Default for NativeCReturnState {
    fn default() -> Self {
        Self { code: 0, level: 1 }
    }
}

/// C's `ERR_ALREADY_LOGGED` for the current error.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum ErrorLogged {
    #[default]
    No,
    /// The current command level logged its frame, so the same bytecode
    /// frame is not logged again; a frame boundary clears it.
    Yes,
    /// Logged across frame boundaries until a procedure boundary: 8.4's
    /// `Tcl_ExprObj` leaves `ERR_ALREADY_LOGGED` set after an error in a
    /// command substitution
    /// ([`tcl_dialect::TclVersion::keeps_expression_substitution_errors_logged`]),
    /// and only the procedure boundary clears it.
    ThroughFrames,
}

impl ErrorLogged {
    /// Whether the current error's command is already logged.
    pub(crate) fn is_logged(self) -> bool {
        self != Self::No
    }

    /// What a frame boundary leaves.
    pub(crate) fn past_frame(self) -> Self {
        match self {
            Self::ThroughFrames => Self::ThroughFrames,
            Self::No | Self::Yes => Self::No,
        }
    }

    /// The state once the current command level has logged.
    pub(crate) fn marked(self) -> Self {
        match self {
            Self::No => Self::Yes,
            logged => logged,
        }
    }
}

impl Default for NativeErrorState {
    fn default() -> Self {
        Self {
            error_info: None,
            native_error_info: None,
            native_error_info_len: 0,
            native_error_result: None,
            native_return_options: None,
            native_c_return_state: NativeCReturnState::default(),
            c84_execution_names: None,
            native_error_legacy_copy: false,
            c84_global_code_set: false,
            primitive_error_code: None,
            error_logged: ErrorLogged::No,
            error_stack: NativeErrorStack::default(),
            error_line: 1,
        }
    }
}

impl Vm {
    /// Clear `ERR_ALREADY_LOGGED` at a frame boundary (a nested `eval`/`[subst]`
    /// returned an error), so the enclosing command logs its own frame.
    pub(crate) fn command_error_is_logged(&self) -> bool {
        self.native_errors.error_logged.is_logged()
    }

    pub(crate) fn clear_error_logged(&mut self) {
        self.native_errors.error_logged = self.native_errors.error_logged.past_frame();
    }

    /// Mark the current error's command logged (C's `ERR_ALREADY_LOGGED`).
    pub(crate) fn mark_error_logged(&mut self) {
        self.native_errors.error_logged = self.native_errors.error_logged.marked();
    }

    /// Mark the current error logged until a procedure boundary
    /// ([`ErrorLogged::ThroughFrames`]).
    pub(crate) fn mark_error_logged_through_frames(&mut self) {
        self.native_errors.error_logged = ErrorLogged::ThroughFrames;
    }

    /// Seed the `errorInfo` trace with an explicit value and mark it logged —
    /// C's `error msg info` / `return -errorinfo`, which set the trace directly
    /// and suppress the command's own `while executing` frame.
    /// Raise a [`ScriptCommandPlan`]'s malformed tail, logging the frame C
    /// names the offending command in.
    ///
    /// C compiles a parse failure into a runtime error through
    /// `Tcl_LogCommandInfo`, so the `while executing` / `"<command>"` pair is
    /// part of `-errorinfo` exactly as it is for an ordinary runtime error in
    /// the same position (#2172). Logging goes through the same helper an
    /// ordinary command error uses, which owns the 150-byte truncation.
    ///
    /// A tail with no recorded command text falls back to seeding the bare
    /// message: an empty pair of quotes would be a visible wrong answer.
    pub(crate) fn raise_fatal_tail(&mut self, tail: FatalTail) -> Completion<Value> {
        // A script the release refused to compile whole ran nothing: its
        // `while compiling` frames replace the `while executing` pair.
        if let Some(info) = tail.compile_info.as_deref()
            && !self.uses_jim_error_stack()
        {
            let mut text = tail.message.as_bytes().to_vec();
            text.extend_from_slice(info.as_bytes());
            self.seed_error_info(text);
            self.clear_error_logged();
            if tail.line != 0 {
                self.set_error_line(tail.line);
            }
            return match tail.error_code {
                Some(code) => err_with_code(tail.message, &code),
                None => err(tail.message),
            };
        }
        if self.uses_jim_error_stack() {
            let level = self
                .jim_errors
                .frames
                .last()
                .map_or(0, |frame| frame.procedure_level);
            return self.raise_jim_script_parse_failure(tail, level, None);
        } else if tail.command_text.is_empty() {
            self.seed_error_info(tail.message.clone());
        } else {
            self.log_command_info_only(&tail.command_text, &tail.message, tail.line);
        }
        match tail.error_code {
            Some(code) => err_with_code(tail.message, &code),
            None => err(tail.message),
        }
    }

    /// Present a reached procedure compiler parse failure before formal binding.
    pub(crate) fn raise_procedure_parse_failure(
        &mut self,
        tail: FatalTail,
        invoked: &Value,
    ) -> Completion<Value> {
        let dialect = self.actual_native_invocation_dialect();
        let Some(protocol) = tcl_registry::native_procedure::procedure_activation_protocol(dialect)
        else {
            return self.refuse_host_command(
                "native procedure parse-failure protocol is unavailable".into(),
            );
        };
        let Some(string_protocol) = dialect.native_string_protocol() else {
            return self.refuse_host_command(
                "native procedure diagnostic string producer is unavailable".into(),
            );
        };
        let invoked = match invoked.native_string_bytes(string_protocol) {
            Ok(invoked) => invoked,
            Err(error) => return self.refuse_host_command(error.to_string()),
        };
        let Some(command) = tail.compilation_command_text.as_deref() else {
            return self
                .refuse_host_command("native procedure parse geometry is unavailable".into());
        };
        let Some(information) = protocol.parse_failure_information(
            tail.message.as_bytes(),
            command,
            tail.line,
            &invoked,
        ) else {
            return self.refuse_host_command(
                "native procedure parse-failure context is unavailable".into(),
            );
        };
        if self.uses_c84_global_error_info() {
            // C8.4 stores the message in the global errorInfo first, then
            // appends each compiler frame to it.
            self.seed_native_error_result_object(tail.message.as_bytes());
            self.native_errors.error_info = Some(information.clone());
            self.update_native_error_info();
        } else {
            self.seed_error_info(information.clone());
        }
        self.clear_error_logged();
        self.set_error_line(tail.line);
        let options = crate::command::options_dict(
            Code::Error,
            0,
            &[
                ("-errorcode", Value::new_native_string_bytes(&b"NONE"[..])),
                ("-errorinfo", Value::from_native_string_bytes(information)),
            ],
        );
        Completion::new_error_metadata(Code::Error, Value::string(tail.message), options)
    }

    pub(crate) fn raise_script_parse_failure(
        &mut self,
        tail: FatalTail,
        location: Option<tcl_runtime_api::script_source_location::ScriptSourceLocation>,
    ) -> Completion<Value> {
        if self.native_invocation_dialect().script_parse_timing()
            == Some(tcl_registry::invocation_words::NativeScriptParseTiming::BeforeScript)
        {
            let level = self
                .jim_errors
                .frames
                .last()
                .map_or(0, |frame| frame.procedure_level);
            self.raise_jim_script_parse_failure(tail, level, location)
        } else {
            self.raise_fatal_tail(tail)
        }
    }

    /// Present a validated native compilation failure at its actual entry.
    /// Procedure admission calls this before binding parameters; ordinary
    /// invocation boundaries add their own error frame after this completes.
    pub(crate) fn raise_native_compilation_failure(
        &mut self,
        failure: &tcl_runtime_api::NativeCompilationError,
        procedure: Option<&[u8]>,
    ) -> Completion<Value> {
        let information = failure
            .error_info_for_procedure_bytes(procedure)
            .expect("admitted native compilation failure has complete presentation");
        if self.uses_c84_global_error_info() {
            // C8.4 stores the message in the global errorInfo first, then
            // appends each compiler frame to it.
            self.seed_native_error_result_object(failure.message.as_bytes());
            self.native_errors.error_info = Some(information.clone());
            self.update_native_error_info();
        } else {
            self.seed_error_info(information.clone());
        }
        // These are compiler contexts, not the caller's command log. The
        // actual invocation boundary appends its frame once afterwards.
        self.clear_error_logged();
        let mut metadata = vec![("-errorinfo", Value::from_native_string_bytes(information))];
        if let Some(error_code) = &failure.error_code {
            metadata.insert(0, ("-errorcode", Value::string(error_code.as_str())));
        }
        let options = crate::command::options_dict(Code::Error, 0, &metadata);
        Completion::new_error_metadata(
            Code::Error,
            Value::string(failure.message.as_str()),
            options,
        )
    }

    pub(crate) fn seed_error_info(&mut self, info: impl AsRef<[u8]>) {
        if self.uses_jim_error_stack() {
            self.jim_errors
                .stack
                .adopt_explicit(Value::from_string_bytes(info.as_ref()));
        }
        self.native_errors.error_info = Some(info.as_ref().to_vec());
        self.native_errors.native_error_info_len = info.as_ref().len();
        self.native_errors.error_logged = self.native_errors.error_logged.marked();
        self.native_errors.native_error_info = self
            .native_invocation_dialect()
            .native_error_variable_protocol()
            .map(|_| Value::new_native_string_bytes(info.as_ref()));
        self.native_errors.native_error_legacy_copy =
            self.native_errors.native_error_info.is_some();
    }

    /// Seed the trace of an error a procedure boundary makes of a completion
    /// its body may not return (`invoked "break" outside of a loop`). C8.4's
    /// first `Tcl_AddObjErrorInfo` also stores the global `errorInfo` and,
    /// unless a code was set, `errorCode` `NONE`.
    pub(crate) fn seed_unexpected_completion_error(&mut self, message: &[u8]) {
        if self.uses_c84_global_error_info() {
            self.seed_native_error_result_object(message);
        }
        self.seed_error_info(message);
    }

    /// Borrow the accumulated `errorInfo` without consuming the error episode.
    pub(crate) fn error_info_value(&self) -> Option<&[u8]> {
        self.native_errors.error_info.as_deref()
    }

    /// Whether the selected Tcl release exposes TIP 348 error stacks.
    pub(crate) fn supports_error_stack(&self) -> bool {
        self.actual_native_invocation_dialect()
            .native_error_objects_protocol()
            .is_some_and(
                tcl_registry::native_error_objects::NativeErrorObjectsProtocol::has_error_stack,
            )
    }

    /// Adopt an explicit, already-validated `return -errorstack` list.
    pub(crate) fn seed_error_stack_parts(&mut self, parts: &[Value]) {
        let _ = self.native_errors.error_stack.adopt(parts);
    }

    /// Adopt an explicit validated stack through the actual current getter owner.
    /// Unsupported release fields have no stack owner; getter failures retain
    /// their original channel rather than leaving the previous stack in place.
    pub(crate) fn seed_error_stack(&mut self, stack: &Value) -> Result<(), crate::TclError> {
        if let Some(cause) = self.execution_refusal.clone() {
            return Err(crate::TclError::from_execution_failure(cause));
        }
        let recipe = self
            .actual_native_invocation_dialect()
            .native_error_objects_protocol()
            .ok_or_else(|| {
                crate::command::completion_option_failure(
                    self,
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "native error-stack owner",
                    ),
                )
            })?;
        if !recipe.has_error_stack() {
            return Ok(());
        }
        let parts = tcl_syntax::value::ValueOps::list_elements(self, stack)
            .map_err(|error| crate::command::completion_option_failure(self, error))?;
        let parts =
            validate_error_stack(Ok::<_, std::convert::Infallible>(parts)).map_err(|_| {
                crate::command::completion_option_failure(
                    self,
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "validated native completion error-stack pairs",
                    ),
                )
            })?;
        self.seed_error_stack_parts(&parts);
        Ok(())
    }

    /// Add the innermost command context for a new Tcl 8.6+ error episode.
    #[cfg(test)]
    pub(crate) fn begin_error_stack_context(&mut self, context: Value) {
        if self.supports_error_stack()
            && !self.native_errors.error_logged.is_logged()
            && let Err(error) = self
                .native_errors
                .error_stack
                .begin_inner(Value::string("INNER"), context)
        {
            let _ = crate::command::completion_from_tcl_error(self, error.into());
        }
    }

    /// Retain original return-instruction operands in the interpreter's inner context.
    pub(crate) fn capture_original_return_instruction_context(
        &mut self,
        purpose: tcl_registry::native_return_options::NativeReturnOptionsApplication,
        name: tcl_syntax::native_instruction_name::NativeInstructionName,
        operands: &[Value],
    ) {
        let recipe = self
            .actual_native_invocation_dialect()
            .native_return_options_application(purpose);
        if recipe.and_then(tcl_registry::native_return_options::NativeReturnOptionsApplicationProtocol::inner_context_name) != Some(name) {
            return;
        }
        if recipe.is_some_and(tcl_registry::native_return_options::NativeReturnOptionsApplicationProtocol::clears_logged_after_result) {
            self.native_errors.error_logged = ErrorLogged::No;
        }
        if !self.native_errors.error_logged.is_logged()
            && let Err(error) = self
                .native_errors
                .error_stack
                .begin_instruction(name, operands)
        {
            let _ = crate::command::completion_from_tcl_error(self, error.into());
        }
    }

    /// Record the frame owned by an actual command-log operation.
    pub(crate) fn error_stack_log_value(&mut self, context: Value) {
        if !self.supports_error_stack() {
            return;
        }
        if let Err(error) = self
            .native_errors
            .error_stack
            .begin_inner(Value::string("INNER"), context)
        {
            let _ = crate::command::completion_from_tcl_error(self, error.into());
            return;
        }
        let frame = if let Some(original) = self
            .native_errors
            .error_stack
            .shifted_context_frame(self.frames.len())
        {
            match original {
                ShiftedErrorStackFrame::Unreported => ErrorStackFrame::Unreported,
                ShiftedErrorStackFrame::Redirect(delta) => {
                    ErrorStackFrame::Redirect(Value::int(i64::try_from(delta).unwrap_or(i64::MAX)))
                }
            }
        } else if let Some(frame) = self
            .frames
            .last()
            .filter(|frame| !frame.call_argv.is_empty())
        {
            ErrorStackFrame::Call(Value::native_list_constructor(
                frame.call_argv.iter().map(Value::clone).collect(),
                self.actual_native_string_protocol()
                    .expect("selected C error stack"),
            ))
        } else {
            ErrorStackFrame::Unreported
        };
        let _ = self
            .native_errors
            .error_stack
            .log_frame(frame, |tag| Value::string(tag));
    }

    /// Replace the current episode when a non-error trace completion is
    /// transformed into a new error by the variable subsystem.
    pub(crate) fn error_stack_restart_with_inner(&mut self, context: Value) {
        if !self.supports_error_stack() {
            return;
        }
        if let Err(error) = self
            .native_errors
            .error_stack
            .restart_inner(Value::string("INNER"), context)
        {
            let _ = crate::command::completion_from_tcl_error(self, error.into());
        }
    }

    /// Return the last TIP 348 stack as a Tcl list value.
    pub(crate) fn error_stack_value(&self) -> Value {
        self.native_errors.error_stack.value()
    }

    /// Native error return-options overlays always retain the live private List,
    /// including while its next error episode is waiting for lazy reset.
    pub(crate) fn error_stack_for_completion(&self, _carried: Option<Value>) -> Value {
        self.native_errors.error_stack.value()
    }

    /// Reset transient metadata for a distinct nested evaluation. The error
    /// stack itself resets lazily so `info errorstack` retains the last value.
    pub(super) fn reset_error_state_for_eval(&mut self) {
        self.jim_errors.stack.mark_reset();
        self.native_errors.error_stack.mark_reset();
        self.native_errors.error_info = None;
        self.native_errors.native_error_info = None;
        self.native_errors.native_error_info_len = 0;
        self.native_errors.native_error_result = None;
        self.native_errors.native_return_options = None;
        self.native_errors.native_error_legacy_copy = false;
        self.native_errors.error_logged = ErrorLogged::No;
        self.native_errors.error_line = 1;
    }

    /// Take the accumulated `errorInfo` trace (if any) and reset it for the next
    /// error — used when `catch` reports an error.
    /// Forget `ERR_ALREADY_LOGGED` while keeping the accumulated `errorInfo`,
    /// as an error leaving a script level does.
    pub(crate) fn forget_error_logged(&mut self) {
        self.native_errors.error_logged = ErrorLogged::No;
    }

    pub(crate) fn take_error_info(&mut self) -> Option<Vec<u8>> {
        self.native_errors.error_logged = ErrorLogged::No;
        self.native_errors.error_stack.mark_reset();
        self.native_errors.error_info.take()
    }

    /// Apply a primitive getter's authored action to interpreter error state.
    /// Unchanged borrows the retained value and performs no store or guest trace.
    pub(crate) fn apply_primitive_error_code(
        &mut self,
        update: tcl_cmd_core::ResolvedCmdErrorCodeUpdate,
    ) -> Value {
        match update {
            tcl_cmd_core::ResolvedCmdErrorCodeUpdate::Set(bytes) => {
                let value = self.native_default_error_code(&bytes);
                self.native_errors.primitive_error_code = Some(value.clone());
                value
            }
            tcl_cmd_core::ResolvedCmdErrorCodeUpdate::Unchanged => self
                .native_errors
                .primitive_error_code
                .clone()
                .unwrap_or_else(|| self.native_default_error_code(b"NONE")),
        }
    }

    /// Retain the original return-error code object, without guest publication.
    /// C resets a missing code to NONE; Jim keeps its current code object.
    pub(crate) fn retain_return_error_code(
        &mut self,
        original: Option<&Value>,
        reset_to_none: bool,
    ) -> Value {
        if let Some(original) = original {
            self.native_errors.primitive_error_code = Some(original.clone());
            return original.clone();
        }
        if reset_to_none {
            return self.apply_primitive_error_code(tcl_cmd_core::ResolvedCmdErrorCodeUpdate::Set(
                b"NONE".to_vec(),
            ));
        }
        self.apply_primitive_error_code(tcl_cmd_core::ResolvedCmdErrorCodeUpdate::Unchanged)
    }

    /// Publish `errorInfo` / `errorCode` into the global frame.
    pub(crate) fn publish_error(&mut self, info: impl AsRef<[u8]>, code: &Value) {
        self.publish_error_bytes(info.as_ref(), code);
    }

    pub(super) fn publish_error_bytes(&mut self, info: &[u8], code: &Value) {
        if self.limits.confined_stores
            || self.uses_jim_error_stack()
            || self.execution_refusal.is_some()
        {
            return;
        }
        if self.uses_c84_global_error_info() {
            // C8.4 publishes through Tcl_AddObjErrorInfo at the reached
            // logging frontier. Catch digestion must not store it again.
            self.seed_native_error_result_object(info);
            return;
        }
        if self
            .native_invocation_dialect()
            .native_error_variable_protocol()
            .is_some()
        {
            self.publish_native_error_objects(info, code);
            return;
        }
        self.write_scalar_from(0, "::errorInfo", Value::from_string_bytes(info));
        self.write_scalar_from(0, "::errorCode", code.clone());
    }

    /// Publish the `errorInfo` global alone, leaving `errorCode` as it is.
    pub(crate) fn publish_error_info(&mut self, info: impl AsRef<[u8]>) {
        if self.limits.confined_stores || self.uses_jim_error_stack() {
            return;
        }
        self.write_scalar_from(0, "::errorInfo", Value::from_string_bytes(info.as_ref()));
    }
}
