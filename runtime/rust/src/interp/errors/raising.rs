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

//! Raising an error: the generic and coded forms, command-layer errors, C API error codes, invalid and unknown commands, and the arity messages.

use super::super::{
    native_argument_usage, native_arguments, native_error_variables, native_operation_currency,
    native_variable_names, new_string, Code, ExceptionState, Interp,
};
use crate::obj;
use crate::obj::TclObj;
use tcl_syntax::value::ValueError;

impl Interp {
    /// An error a C API call reports: `message` as the result and, when given,
    /// `code` as its `-errorcode` (`NONE` otherwise), starting a new error.
    /// `code` as the release sets it
    /// ([`tcl_dialect::TclVersion::release_error_code`]).
    fn release_error_code<'c>(&self, code: &'c [u8]) -> &'c [u8] {
        self.runtime_version().release_error_code(code)
    }

    pub(crate) fn c_api_error(&mut self, message: &[u8], code: Option<&[u8]>) {
        let Ok(scope) = native_operation_currency::NativeOperationScope::enter(self) else {
            return;
        };
        self.set_result_bytes(message);
        if scope.currency().ensure_current_or_refuse().is_err() {
            return;
        }
        let code = code.map(|code| self.release_error_code(code));
        let retired = self.exc.replace(ExceptionState {
            info: None,
            code: code.map(<[u8]>::to_vec).unwrap_or_default(),
            code_explicit: code.is_some(),
            already_logged: false,
            ..ExceptionState::default()
        });
        // A retained native error object can retire extension backing. Release
        // it outside the exception borrow and keep the already reached result.
        drop(retired);
        if scope.currency().ensure_current_or_refuse().is_err() {
            return;
        }
        self.capture_native_error_objects();
        if scope.currency().ensure_current_or_refuse().is_err() {
            return;
        }
        self.c_api_errors.set(self.c_api_errors.get() + 1);
    }

    /// `Tcl_SetObjErrorCode`: `code` is the `-errorcode` of the error the running
    /// C command is about to return, whatever its result.
    pub(crate) fn set_c_error_code(&mut self, code: &[u8]) {
        let Ok(scope) = native_operation_currency::NativeOperationScope::enter(self) else {
            return;
        };
        let retired = self.exc.replace(ExceptionState {
            info: None,
            code: code.to_vec(),
            code_explicit: true,
            already_logged: false,
            ..ExceptionState::default()
        });
        drop(retired);
        if scope.currency().ensure_current_or_refuse().is_err() {
            return;
        }
        let original = obj::Owned::fresh(new_string(code));
        if let Err(first) = self.set_original_c_error_code(original.as_ptr()) {
            self.refuse_native_execution(first);
        }
    }

    /// Set the actual C error-code header without reaching its String getter.
    /// Global C8.4 publication and modern private storage retain their own
    /// selected protocols; a byte author is a separate original object producer.
    pub(crate) fn set_original_c_error_code(
        &mut self,
        original: *mut TclObj,
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        let scope = native_operation_currency::NativeOperationScope::enter(self)?;
        let actual = self.clone();
        self.set_original_c_error_code_using(
            scope.currency(),
            original,
            &|| Ok(()),
            &|error| actual.clone().refuse_completion_value_access(error),
            &|error| {
                actual.clone().refuse_native_execution(error.clone());
                error
            },
        )?;
        scope.currency().ensure_current_or_refuse()?;
        self.c_api_errors.set(self.c_api_errors.get() + 1);
        Ok(())
    }

    /// The same selected old-code retirement and incoming native role stages.
    /// A compiled caller supplies its original frame/pair receipt; only the
    /// public C API wrapper increments its software error-call counter.
    pub(in super::super::super) fn set_original_c_error_code_using<E>(
        &mut self,
        currency: &native_operation_currency::NativeOperationCurrency,
        original: *mut TclObj,
        extra: &dyn Fn() -> Result<(), E>,
        refuse: &dyn Fn(ValueError) -> E,
        retain: &dyn Fn(E) -> E,
    ) -> Result<(), E>
    where
        E: From<tcl_runtime_api::NativeExecutionError>,
    {
        currency.ensure_current_or_refuse()?;
        extra()?;
        obj::check_native_liveness(original).map_err(refuse)?;
        let lifetime = obj::NativeObjectLifetime::retain(original);
        let reached = (|| -> Result<(), E> {
            currency.ensure_current_or_refuse()?;
            extra()?;
            if self.uses_c84_global_error_info() {
                // The reached C84 global write settles ERROR_CODE_SET even
                // when a trace refuses; classify only after mandatory cleanup.
                self.publish_original_c84_error_code(original);
            } else if self
                .native_invocation_dialect()
                .native_error_variable_protocol()
                .is_some()
            {
                self.retire_native_error_object_checked(
                    currency,
                    tcl_registry::special_vars::NativeErrorStorageVariable::ErrorCode,
                    native_error_variables::NativeErrorObjectRetirementPurpose::OriginalSetter,
                )?;
                currency.ensure_current_or_refuse()?;
                extra()?;
                obj::check_native_liveness(original).map_err(refuse)?;
                let reentered = self
                    .exc
                    .borrow_mut()
                    .native
                    .code
                    .replace(obj::Owned::retain(original));
                drop(reentered);
            } else {
                return Err(refuse(ValueError::CommandProtocolUnavailable(
                    "original C error-code setter",
                )));
            }
            currency.ensure_current_or_refuse()?;
            extra()?;
            obj::check_native_liveness(lifetime.as_ptr()).map_err(refuse)?;
            Ok(())
        })();
        let reached = reached.map_err(retain);
        drop(lifetime);
        currency.ensure_current_or_refuse()?;
        extra()?;
        reached
    }

    /// Note that a C API call left an error the interpreter already holds (a
    /// command's own, such as `package provide`'s), as [`Self::c_api_error`]
    /// would have.
    pub(crate) fn note_c_api_error(&self) {
        self.c_api_errors.set(self.c_api_errors.get() + 1);
    }

    /// Raise an error with result `msg` and return [`Code::Error`] — the generic
    /// throw. Resets the [`ExceptionState`] to a fresh error: the source trace
    /// (`::errorInfo`) is then built up **as the error unwinds**
    /// ([`log_command_info`](Self::log_command_info) /
    /// [`make_proc_error`](Self::make_proc_error)) and published to the globals at
    /// the catch / outermost-eval boundary — not stamped here.
    ///
    /// Arbitrary result bytes carry the neutral `NONE` error-code identity.
    pub(crate) fn error(&mut self, msg: &[u8]) -> Code {
        self.error_using(msg, None)
    }

    pub(crate) fn error_using(
        &mut self,
        msg: &[u8],
        stage: native_variable_names::OriginalCNameStage<'_>,
    ) -> Code {
        if let Err(first) = self.check_original_c_name_stage(stage) {
            return self.report_cmd_error(first);
        }

        let Ok(scope) = native_operation_currency::NativeOperationScope::enter(self) else {
            return Code::Error;
        };
        self.set_result_bytes(msg);
        if self.check_original_c_name_stage(stage).is_err()
            || scope.currency().ensure_current_or_refuse().is_err()
        {
            return Code::Error;
        }
        // `Tcl_WrongNumArgs` and the list parser tag their errors only from
        // 8.6; before it `$errorCode` is `NONE`.
        let tags = self.runtime_version().tags_usage_error_codes();
        let code: &[u8] = if !tags {
            b"NONE"
        } else if msg.starts_with(b"wrong # args:") {
            b"TCL WRONGARGS"
        } else {
            std::str::from_utf8(msg)
                .ok()
                .and_then(tcl_syntax::list::ListError::from_message)
                .map_or(b"NONE", |error| error.error_code().as_bytes())
        };
        let state = ExceptionState {
            native: Default::default(),
            info: None,
            code: code.to_vec(),
            code_explicit: false,
            already_logged: false,
            primitive_getter: None,
            expression_error_stage: None,
        };
        if self
            .replace_exception_state_using(scope.currency(), state, stage)
            .and_then(|()| self.capture_native_error_objects_using(scope.currency(), stage))
            .is_err()
        {
            return Code::Error;
        }
        Code::Error
    }

    /// Publish a failed typed read with the message and `-errorcode` C sets.
    pub(crate) fn report_typed_error(&mut self, error: &crate::typed_value::TypedError) -> Code {
        self.error_with_code(&error.message, error.code)
    }

    /// Start the error trace at the result message, where none is started, so
    /// the command holding the failure logs `invoked from within` rather than
    /// `while executing` — C's trace for an error a compile-time fold raised.
    /// Only `expr`, which needs the numeric tower, raises one.
    #[cfg(have_tommath)]
    pub(crate) fn start_error_info_at_message(&mut self) {
        let message = self.result_bytes();
        let mut exc = self.exc.borrow_mut();
        if exc.info.is_none() {
            exc.info = Some(message);
        }
    }

    /// Like [`error`](Self::error) but with an explicit `-errorcode` (the trace
    /// still builds up as the error unwinds). For commands that mirror C's
    /// richer error codes (`TCL LOOKUP INDEX …`, `TCL OO …`).
    pub(crate) fn error_with_code(&mut self, msg: &[u8], code: &[u8]) -> Code {
        self.error_with_code_using(msg, code, None)
    }

    pub(crate) fn error_with_code_using(
        &mut self,
        msg: &[u8],
        code: &[u8],
        stage: native_variable_names::OriginalCNameStage<'_>,
    ) -> Code {
        if let Err(first) = self.check_original_c_name_stage(stage) {
            return self.report_cmd_error(first);
        }

        let Ok(scope) = native_operation_currency::NativeOperationScope::enter(self) else {
            return Code::Error;
        };
        self.set_result_bytes(msg);
        if self.check_original_c_name_stage(stage).is_err()
            || scope.currency().ensure_current_or_refuse().is_err()
        {
            return Code::Error;
        }
        let code = self.release_error_code(code);
        let state = ExceptionState {
            native: Default::default(),
            info: None,
            code: code.to_vec(),
            code_explicit: false,
            already_logged: false,
            primitive_getter: None,
            expression_error_stage: None,
        };
        if self
            .replace_exception_state_using(scope.currency(), state, stage)
            .and_then(|()| self.capture_native_error_objects_using(scope.currency(), stage))
            .is_err()
        {
            return Code::Error;
        }
        // C8.4's Tcl_SetErrorCode writes the global at once and marks it set,
        // so the later errorInfo logging keeps it rather than stamping NONE.
        if self.uses_c84_global_error_info() {
            let original = obj::Owned::fresh(new_string(code));
            let actual = self.clone();
            if self
                .set_original_c_error_code_using(
                    scope.currency(),
                    original.as_ptr(),
                    &|| Ok(()),
                    &|error| actual.clone().refuse_completion_value_access(error),
                    &|error| {
                        actual.clone().refuse_native_execution(error.clone());
                        error
                    },
                )
                .is_err()
            {
                return Code::Error;
            }
        }
        Code::Error
    }

    /// Select result representation from retained command-error metadata.
    /// A lookalike message cannot select the native arity append producer.
    fn command_error_string_result(
        &self,
        expected: Option<tcl_syntax::native_string::NativeStringProtocol>,
        wrong_arguments: bool,
    ) -> Result<
        Option<tcl_registry::native_string_materialization::NativeStringMaterialization>,
        tcl_syntax::value::ValueError,
    > {
        let dialect = self.native_invocation_dialect();
        let expected = expected.or_else(|| {
            wrong_arguments.then(|| dialect.native_wrong_arguments_protocol())
                .flatten()
                .and_then(tcl_registry::native_wrong_arguments::NativeWrongArgumentsProtocol::string_result)
        });
        let Some(expected) = expected else {
            return Ok(None);
        };
        dialect
            .native_string_materialization(None)
            .filter(|actual| actual.protocol() == expected)
            .map(Some)
            .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "selected error String producer",
            ))
    }

    /// Publish a portable command-layer error through this runtime's result
    /// and exception-state ABI without losing a structured error code.
    pub(crate) fn report_cmd_error(&mut self, error: tcl_cmd_core::CmdError) -> Code {
        self.report_cmd_error_using(error, None)
    }

    /// An owned object-command shim states its shared error through the C ABI.
    pub(crate) fn report_obj_cmd_error(&mut self, error: tcl_cmd_core::CmdError) -> Code {
        self.c_api_errors.set(self.c_api_errors.get() + 1);
        self.report_cmd_error(error)
    }

    pub(crate) fn report_cmd_error_using(
        &mut self,
        error: tcl_cmd_core::CmdError,
        stage: native_variable_names::OriginalCNameStage<'_>,
    ) -> Code {
        if self.host_refusal_pending() {
            return Code::Error;
        }
        if let Some(error) = error.native_execution_refusal() {
            return self.refuse_native_execution(error.clone());
        }
        if let Err(first) = self.check_original_c_name_stage(stage) {
            return self.report_cmd_error(first);
        }

        let Ok(scope) = native_operation_currency::NativeOperationScope::enter(self) else {
            return Code::Error;
        };
        let details = error.into_byte_details();
        let explicit_code_store =
            matches!(details.error_code, tcl_cmd_core::CmdErrorCodeUpdate::Set(_));
        let wrong_arguments = matches!(
            details.error_code,
            tcl_cmd_core::CmdErrorCodeUpdate::WrongArguments
        );
        let update = match details.error_code.resolve(|| {
            self.native_invocation_dialect()
                .wrong_arguments_protocol(Some(tcl_registry::native_wrong_arguments::LogicalWrongArgumentsProvider::Tcl84CoreSimulation))
                .map(|protocol| protocol.error_code().as_bytes().to_vec())
                .ok_or(tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable("wrong arguments"))
        }) {
            Ok(update) => update,
            Err(refusal) => return self.refuse_native_access(refusal),
        };
        let string_result =
            match self.command_error_string_result(details.string_result, wrong_arguments) {
                Ok(result) => result,
                Err(error) => return self.report_cmd_error_using(error.into(), stage),
            };
        self.set_result_bytes(&details.message);
        if self.check_original_c_name_stage(stage).is_err()
            || scope.currency().ensure_current_or_refuse().is_err()
        {
            return Code::Error;
        }
        if let Some(materialization) = string_result {
            if let Err(error) =
                obj::retain_native_string_representation(self.get_obj_result(), materialization)
            {
                return self.report_cmd_error_using(error.into(), stage);
            }
        }
        if self.check_original_c_name_stage(stage).is_err() {
            return Code::Error;
        }
        if let tcl_cmd_core::ResolvedCmdErrorCodeUpdate::Set(code) = &update {
            if !self.uses_c84_global_error_info() || explicit_code_store {
                if self
                    .replace_native_error_code_using(scope.currency(), code, stage)
                    .is_err()
                {
                    return Code::Error;
                }
            } else {
                self.exc.borrow_mut().native.global_code_set = false;
            }
        }
        if self.check_original_c_name_stage(stage).is_err() {
            return Code::Error;
        }
        let original_info = self
            .native_invocation_dialect()
            .native_error_variable_protocol()
            .is_some()
            .then(|| {
                details
                    .error_info
                    .as_deref()
                    .map(|bytes| obj::Owned::fresh(new_string(bytes)))
            });
        let info_len = details.error_info.as_ref().map_or(0, Vec::len);
        let mut exc = self.exc.borrow_mut();
        match update {
            tcl_cmd_core::ResolvedCmdErrorCodeUpdate::Set(code) => {
                exc.code = code;
                exc.code_explicit = false;
            }
            tcl_cmd_core::ResolvedCmdErrorCodeUpdate::Unchanged => {}
        }
        exc.info = details.error_info;
        exc.already_logged = false;
        exc.primitive_getter = details.primitive_getter;
        exc.expression_error_stage = None;
        drop(exc);
        if let Some(original_info) = original_info {
            if self
                .replace_native_error_object_using(
                    scope.currency(),
                    tcl_registry::special_vars::NativeErrorStorageVariable::ErrorInfo,
                    original_info,
                    Some(info_len),
                    stage,
                )
                .is_err()
            {
                return Code::Error;
            }
        }
        if self.check_original_c_name_stage(stage).is_err()
            || scope.currency().ensure_current_or_refuse().is_err()
        {
            return Code::Error;
        }
        if let Some(line) = details
            .error_line
            .and_then(|value| u32::try_from(value).ok())
        {
            self.error_line.set(line);
        }
        Code::Error
    }

    /// The `invalid command name "X"` error (the resolver miss; `unknown` later).
    pub(crate) fn invalid_command(&mut self, name: &[u8]) -> Code {
        self.invalid_command_result(name)
    }

    /// C's `Tcl_FindCommand` + `TCL_LEAVE_ERR_MSG` miss (`unknown command "X"`,
    /// `tclNamesp.c`) — distinct from `invalid_command`'s `invalid command
    /// name`. Used by `trace add|remove|info command|execution`.
    pub(crate) fn unknown_command(&mut self, name: &[u8]) -> Code {
        let mut msg = b"unknown command \"".to_vec();
        msg.extend_from_slice(name);
        msg.push(b'"');
        self.error(&msg)
    }

    /// Set an error result and return [`Code::Error`] — for builtins.
    pub(crate) fn set_error(&mut self, msg: &[u8]) -> Code {
        self.error(msg)
    }

    /// Render the arity error with the actual invoked command word.
    pub(crate) fn wrong_args_for_invocation(
        &mut self,
        arguments: &[*mut TclObj],
        suffix: &[u8],
    ) -> Code {
        self.wrong_args_for_prefix(arguments, 1, suffix)
    }

    /// Render actual leading argument values through the retained dispatch
    /// rewrite and the selected native usage quoting policy, preserving bytes.
    pub(crate) fn wrong_args_for_prefix(
        &mut self,
        arguments: &[*mut TclObj],
        prefix_words: usize,
        suffix: &[u8],
    ) -> Code {
        let originals = match native_arguments::OriginalArguments::enter(self, arguments) {
            Ok(originals) => originals,
            Err(error) => return self.report_cmd_error(error),
        };
        let code = (|| {
            let mut usage = match self.argument_usage_prefix(arguments, prefix_words) {
                Ok(usage) => usage,
                Err(code) => return code,
            };
            if let Err(error) = originals.check() {
                return self.report_cmd_error(error);
            }
            if !suffix.is_empty() {
                if !usage.is_empty() {
                    usage.push(b' ');
                }
                usage.extend_from_slice(suffix);
            }
            self.wrong_args(&usage)
        })();
        originals.finish(code)
    }

    /// Render only the retained actual handler header, without touching operands.
    pub(crate) fn argument_usage_prefix(
        &mut self,
        arguments: &[*mut TclObj],
        prefix_words: usize,
    ) -> Result<Vec<u8>, Code> {
        let Some(protocol) = self.native_invocation_dialect().usage_protocol(Some(
            tcl_registry::native_usage::LogicalUsageProvider::Tcl84CoreSimulation,
        )) else {
            return Err(self.refuse_native_access(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "argument usage",
                ),
            ));
        };
        let original = native_argument_usage::OriginalArgumentUsage::capture(
            self,
            arguments,
            prefix_words,
            true,
        )
        .map_err(|error| self.report_cmd_error(error))?;
        let outcome = original.render_header(self, protocol);
        original
            .check()
            .map_err(|error| self.report_cmd_error(error))?;
        outcome.map_err(|error| self.report_cmd_error(error))
    }

    pub(crate) fn wrong_args(&mut self, usage: &[u8]) -> Code {
        self.report_cmd_error(tcl_cmd_core::CmdError::wrong_args_bytes(usage))
    }

    pub(crate) fn wrong_arguments_message(&mut self, message: &[u8]) -> Code {
        self.report_cmd_error(tcl_cmd_core::CmdError::wrong_arguments_message_bytes(
            message,
        ))
    }
}
