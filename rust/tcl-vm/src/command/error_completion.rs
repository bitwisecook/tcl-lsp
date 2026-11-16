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

//! The `error` command and the conversion of command-layer errors into `ERROR` completions with their `-errorcode`.

use super::native_wrong_args;
use crate::error::TclError;
use crate::interp::Vm;
use crate::value::Value;
use tcl_cmd_core::CmdError;
use tcl_runtime_api::{Code, Completion};

/// Build an options dict value `-code N -level L [-errorcode ..] [-errorinfo ..]`.
pub(crate) fn options_dict(code: Code, level: i64, extra: &[(&str, Value)]) -> Value {
    let mut items = vec![
        Value::string("-code"),
        Value::int(code.as_int()),
        Value::string("-level"),
        Value::int(level),
    ];
    for (k, v) in extra {
        items.push(Value::string(*k));
        items.push(v.clone());
    }
    Value::list(items)
}

/// An `ERROR` completion carrying an `-errorcode` (e.g. `TCL BINARY DECODE
/// INVALID`), so `$errorCode` is set when the error propagates.
pub(crate) fn err_with_code(
    message: impl AsRef<[u8]>,
    code: impl AsRef<[u8]>,
) -> Completion<Value> {
    let options = options_dict(
        Code::Error,
        0,
        &[("-errorcode", Value::from_string_bytes(code.as_ref()))],
    );
    Completion::new_error_metadata(
        Code::Error,
        Value::from_string_bytes(message.as_ref()),
        options,
    )
}

impl Vm {
    /// A command core's error as this VM's completion, its error code the one
    /// the emulated release sets ([`tcl_dialect::TclVersion::release_error_code`]):
    /// a `TCL …` code the release does not set yet leaves the neutral `NONE`,
    /// as C's command left `errorCode` unset.
    pub(crate) fn cmd_error(&mut self, error: CmdError) -> Completion<Value> {
        if error.native_execution_refusal().is_some() || error.native_access_refusal().is_some() {
            return completion_from_cmd_error(self, error);
        }
        let release = self.runtime_version();
        let mut details = error.into_byte_details();
        if let tcl_cmd_core::CmdErrorCodeUpdate::Set(code) = &details.error_code
            && !release.sets_error_code(code)
        {
            details.error_code = tcl_cmd_core::CmdErrorCodeUpdate::Default;
        }
        completion_from_cmd_error(self, CmdError::from_byte_details(details))
    }
}

/// Convert a portable command-layer error without losing its Tcl identity.
pub(crate) fn completion_from_cmd_error(vm: &mut Vm, error: CmdError) -> Completion<Value> {
    if let Some(error) = error.native_execution_refusal() {
        return vm.refuse_tcl_host_failure(crate::error::TclHostFailure::Execution(error.clone()));
    }
    if let Some(error) = error.native_access_refusal() {
        return vm.refuse_tcl_host_failure(crate::error::TclHostFailure::ValueAccess(error));
    }
    let details = error.into_byte_details();
    let explicit_code_store =
        matches!(details.error_code, tcl_cmd_core::CmdErrorCodeUpdate::Set(_));
    let wrong_arguments = matches!(
        details.error_code,
        tcl_cmd_core::CmdErrorCodeUpdate::WrongArguments
    );
    let update = match details.error_code.resolve(|| {
        vm.native_invocation_dialect()
            .wrong_arguments_protocol(Some(tcl_registry::native_wrong_arguments::LogicalWrongArgumentsProvider::Tcl84CoreSimulation))
            .map(|protocol| protocol.error_code().as_bytes().to_vec())
            .ok_or(tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable("wrong arguments"))
    }) {
        Ok(update) => update,
        Err(refusal) => {
            return vm.refuse_tcl_host_failure(crate::error::TclHostFailure::ValueAccess(refusal));
        }
    };
    let string_protocol = details.string_result.or_else(|| {
        wrong_arguments
            .then(|| {
                vm.actual_native_invocation_dialect()
                    .native_wrong_arguments_protocol()
            })
            .flatten()
            .and_then(
                tcl_registry::native_wrong_arguments::NativeWrongArgumentsProtocol::string_result,
            )
    });
    let string_result = match string_protocol {
        Some(expected) => match vm
            .actual_native_invocation_dialect()
            .native_string_materialization(None)
        {
            Some(actual) if actual.protocol() == expected => Some(actual),
            _ => return vm.refuse_host_command("selected error String producer".to_owned()),
        },
        None => None,
    };
    // This adapter creates an evaluated command completion. Primitive CAPI
    // callers render through the original-value owner and do not enter here.
    let update = match details
        .primitive_getter
        .as_ref()
        .map(|getter| getter.eval_error_code_update())
    {
        Some(tcl_syntax::scalar_getter::NativeScalarGetterErrorCode::Set(code)) => {
            tcl_cmd_core::ResolvedCmdErrorCodeUpdate::Set(code.clone())
        }
        _ => update,
    };
    let code = vm.apply_cmd_error_code(update, explicit_code_store);
    let message = details
        .primitive_getter
        .as_ref()
        .map_or(details.message.as_slice(), |getter| {
            getter.eval_result_bytes(&details.message)
        });
    let result = Value::from_string_bytes(message);
    if let Some(materialization) = string_result
        && let Err(error) = result.retain_native_string_representation(materialization)
    {
        return completion_from_tcl_error(vm, error.into());
    }
    let mut extra = Vec::with_capacity(3);
    extra.push(("-errorcode", code));
    if let Some(info) = details.error_info {
        vm.add_command_error_info(message, &info);
        extra.push(("-errorinfo", Value::from_string_bytes(info)));
    }
    if let Some(line) = details.error_line {
        extra.push(("-errorline", Value::int(line)));
    }
    Completion::new_error_metadata(Code::Error, result, options_dict(Code::Error, 0, &extra))
}

/// Convert an internal expression/helper failure into its Tcl completion
/// without losing either a propagating control code or an explicit
/// `-errorcode` supplied by the registry or command implementation.
pub(crate) fn completion_from_tcl_error(vm: &mut Vm, error: TclError) -> Completion<Value> {
    match error.into_completion() {
        Ok(completion) => completion,
        Err(error) => vm.refuse_tcl_host_failure(error),
    }
}

/// `error message ?info? ?code?`.
pub(super) fn cmd_error(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some(protocol) = vm.native_invocation_dialect().error_arguments() else {
        return vm.refuse_host_command("native error argument grammar is unresolved".to_owned());
    };
    if !protocol.accepts_len(args.len()) {
        return native_wrong_args(vm, protocol.synopsis());
    }
    let msg = &args[0];
    if protocol == tcl_registry::invocation_words::NativeErrorArguments::JimStackTrace {
        let trace = args.get(1).cloned().unwrap_or_else(Value::empty);
        if args.len() == 2 {
            vm.jim_errors.stack.adopt_explicit(trace.clone());
        }
        return Completion::new_error_metadata(
            Code::Error,
            msg.clone(),
            options_dict(
                Code::Error,
                0,
                &[("-errorcode", Value::string("NONE")), ("-errorinfo", trace)],
            ),
        );
    }
    if vm
        .native_invocation_dialect()
        .native_error_variable_protocol()
        .is_some()
    {
        let mut options = vec![
            Value::string("-code"),
            Value::string("error"),
            Value::string("-level"),
            Value::string("0"),
        ];
        if let Some(info) = args.get(1) {
            options.extend([Value::string("-errorinfo"), info.clone()]);
        }
        if let Some(code) = args.get(2) {
            options.extend([Value::string("-errorcode"), code.clone()]);
        }
        options.push(msg.clone());
        return crate::return_options::apply(
            vm,
            &options,
            tcl_cmd_core::return_options::ReturnOptionsPurpose::User,
        );
    }
    // A *non-empty* `info` argument *is* the errorInfo trace: seed it directly and
    // suppress the `error` command's own `while executing` frame (C's
    // `ERR_ALREADY_LOGGED`). An empty (or absent) `info` is treated as absent —
    // the trace seeds from the message and the invoke site logs the frame as for
    // any other command error (error-4.2/4.3).
    let info = args.get(1).map(super::super::value::Value::string_bytes);
    let info_nonempty = info.as_deref().is_some_and(|s| !s.is_empty());
    if info_nonempty {
        match vm.publish_c84_error_command_info(info.as_deref().unwrap_or_default()) {
            Ok(true) => {}
            Ok(false) => vm.seed_error_info(info.clone().unwrap_or_default()),
            Err(error) => return completion_from_cmd_error(vm, error.into()),
        }
    }
    let ecode = vm.prepare_error_command_code(args.get(2));
    let einfo = if info_nonempty {
        info.unwrap_or_default()
    } else {
        msg.string_bytes()
    };
    let options = options_dict(
        Code::Error,
        0,
        &[
            ("-errorcode", ecode),
            ("-errorinfo", Value::from_string_bytes(einfo)),
        ],
    );
    Completion::new_error_metadata(Code::Error, msg.clone(), options)
}
