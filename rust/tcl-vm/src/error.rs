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

//! The VM's internal error type.

use tcl_runtime_api::{Code, Completion, NativeExecutionError};
use tcl_syntax::raw_string::{NativeValueAccessRefusal, RawString, UnicodeAccessError};

use crate::value::Value;

/// An operational failure that guest Tcl completion cannot represent.
#[derive(Debug, Clone)]
pub enum TclHostFailure {
    /// A concrete value operation is unavailable to this backend.
    ValueAccess(NativeValueAccessRefusal),
    /// A compilation or reached execution provider obligation.
    Execution(NativeExecutionError),
    /// A context-free value API has no interpreter state for an Unchanged
    /// primitive error-code obligation. VM-aware getters resolve this directly.
    PrimitiveErrorStateRequired(Box<tcl_syntax::scalar_getter::NativeScalarGetterError>),
}

impl std::fmt::Display for TclHostFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ValueAccess(error) => error.fmt(formatter),
            Self::Execution(error) => error.fmt(formatter),
            Self::PrimitiveErrorStateRequired(_) => formatter
                .write_str("primitive getter error requires retained interpreter error-code state"),
        }
    }
}

/// Internal evaluation failure preserves either the complete original guest
/// completion or an operational host refusal. Guest results and options retain
/// their actual objects until an explicit consumer requests a byte/text view.
#[derive(Debug, Clone)]
pub enum TclError {
    /// A guest completion propagated through an internal evaluation helper.
    Guest(Completion<Value>),
    /// A host refusal, outside guest catch/try and completion settlement.
    Host(TclHostFailure),
}

impl TclError {
    /// Build an ordinary guest error with its explicit default error code.
    pub fn new(message: impl AsRef<[u8]>) -> Self {
        Self::Guest(crate::command::err_with_code(message, b"NONE"))
    }

    /// Build a fresh control completion. Propagating an existing completion
    /// must instead use [`Self::from_completion`] to retain all its options.
    pub fn with_code(message: impl AsRef<[u8]>, code: Code) -> Self {
        let (option_code, level) = if code == Code::Return {
            (Code::Ok, 1)
        } else {
            (code, 0)
        };
        Self::Guest(Completion::new(
            code,
            Value::from_string_bytes(message.as_ref()),
            crate::command::options_dict(option_code, level, &[]),
        ))
    }

    /// Build an ordinary guest error with an authored structured error code.
    pub fn with_error_code(message: impl AsRef<[u8]>, error_code: impl AsRef<[u8]>) -> Self {
        Self::Guest(crate::command::err_with_code(message, error_code))
    }

    /// Retain an existing completion without rendering or rebuilding objects.
    #[must_use]
    pub const fn from_completion(completion: Completion<Value>) -> Self {
        Self::Guest(completion)
    }

    /// Retain the complete operational execution failure.
    #[must_use]
    pub const fn from_execution_failure(error: NativeExecutionError) -> Self {
        Self::Host(TclHostFailure::Execution(error))
    }

    /// The retained guest completion, independently of its control code.
    #[must_use]
    pub const fn guest_completion(&self) -> Option<&Completion<Value>> {
        match self {
            Self::Guest(completion) => Some(completion),
            Self::Host(_) => None,
        }
    }

    /// Whether this failure belongs to the operational host channel.
    #[must_use]
    pub const fn is_host(&self) -> bool {
        matches!(self, Self::Host(_))
    }

    /// Whether the original guest completion is an ordinary Tcl error.
    #[must_use]
    pub fn is_guest_error(&self) -> bool {
        self.guest_completion()
            .is_some_and(|completion| completion.code == Code::Error)
    }

    /// Compatibility projection of a concrete value-access refusal.
    #[must_use]
    pub const fn native_access_refusal(&self) -> Option<NativeValueAccessRefusal> {
        match self {
            Self::Host(TclHostFailure::ValueAccess(refusal)) => Some(*refusal),
            _ => None,
        }
    }

    /// Exact guest result bytes at an explicitly byte-valued boundary.
    ///
    /// # Errors
    /// Host failures cannot be rendered as a guest message.
    pub fn message_bytes(&self) -> Result<std::rc::Rc<[u8]>, TclHostFailure> {
        match self {
            Self::Guest(completion) => Ok(completion.result.string_bytes()),
            Self::Host(error) => Err(error.clone()),
        }
    }

    /// Project a guest message as Unicode only where the consumer requires it.
    ///
    /// # Errors
    /// Preserves host failures and refuses non-Unicode guest message bytes.
    pub fn message_unicode(&self) -> Result<std::rc::Rc<str>, TclHostFailure> {
        RawString::from_bytes(self.message_bytes()?.as_ref())
            .unicode()
            .map_err(|error| TclHostFailure::ValueAccess(error.into()))
    }

    /// The authored error code of an ordinary guest error.
    #[must_use]
    pub fn error_code_bytes(&self) -> Option<std::rc::Rc<[u8]>> {
        let completion = self.guest_completion()?;
        (completion.code == Code::Error)
            .then(|| crate::command::opt_get(&completion.options, "-errorcode"))
            .flatten()
            .map(|value| value.string_bytes())
    }

    /// Replace one selected error code without discarding other guest options.
    pub(crate) fn set_error_code(&mut self, code: impl AsRef<[u8]>) {
        let Self::Guest(completion) = self else {
            return;
        };
        if completion.code != Code::Error {
            return;
        }
        let Ok(items) = completion.options.as_list() else {
            return;
        };
        let mut replacement = Vec::with_capacity(items.len() + 2);
        let mut replaced = false;
        for pair in items.as_chunks::<2>().0 {
            replacement.push(pair[0].clone());
            if pair[0].string_bytes().as_ref() == b"-errorcode" {
                replacement.push(Value::from_string_bytes(code.as_ref()));
                replaced = true;
            } else {
                replacement.push(pair[1].clone());
            }
        }
        if !replaced {
            replacement.push(Value::string("-errorcode"));
            replacement.push(Value::from_string_bytes(code.as_ref()));
        }
        completion.options = Value::list(replacement);
    }

    /// Move the original completion into a guest dispatch boundary.
    ///
    /// # Errors
    /// Operational host failures remain outside guest Tcl completion.
    pub fn into_completion(self) -> Result<Completion<Value>, TclHostFailure> {
        match self {
            Self::Guest(completion) => Ok(completion),
            Self::Host(error) => Err(error),
        }
    }

    /// Move the original guest result object into a value consumer.
    ///
    /// # Errors
    /// Operational host failures cannot become guest values.
    pub fn into_value(self) -> Result<Value, TclHostFailure> {
        self.into_completion().map(|completion| completion.result)
    }
}

impl From<TclHostFailure> for TclError {
    fn from(error: TclHostFailure) -> Self {
        Self::Host(error)
    }
}

impl From<NativeValueAccessRefusal> for TclError {
    fn from(refusal: NativeValueAccessRefusal) -> Self {
        Self::Host(TclHostFailure::ValueAccess(refusal))
    }
}

impl From<UnicodeAccessError> for TclError {
    fn from(refusal: UnicodeAccessError) -> Self {
        NativeValueAccessRefusal::from(refusal).into()
    }
}

impl From<tcl_syntax::value::ValueError> for TclError {
    fn from(error: tcl_syntax::value::ValueError) -> Self {
        if let Some(refusal) = error.native_access_refusal() {
            return refusal.into();
        }
        let details = tcl_cmd_core::CmdError::from(error).into_byte_details();
        let code = match details.error_code {
            tcl_cmd_core::CmdErrorCodeUpdate::WrongArguments => {
                return tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable("wrong arguments").into();
            }
            tcl_cmd_core::CmdErrorCodeUpdate::Default => Value::string("NONE"),
            tcl_cmd_core::CmdErrorCodeUpdate::Set(bytes) => Value::from_string_bytes(bytes),
            tcl_cmd_core::CmdErrorCodeUpdate::Unchanged => {
                return Self::Host(TclHostFailure::PrimitiveErrorStateRequired(
                    details
                        .primitive_getter
                        .expect("Unchanged originates in retained primitive getter"),
                ));
            }
        };
        let message = details
            .primitive_getter
            .as_ref()
            .map_or(details.message.as_slice(), |getter| {
                getter.eval_result_bytes(&details.message)
            });
        let mut extra = vec![("-errorcode", code)];
        if let Some(info) = details.error_info {
            extra.push(("-errorinfo", Value::from_string_bytes(info)));
        }
        if let Some(line) = details.error_line {
            extra.push(("-errorline", Value::int(line)));
        }
        Self::Guest(Completion::new_error_metadata(
            Code::Error,
            Value::from_string_bytes(message),
            crate::command::options_dict(Code::Error, 0, &extra),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Vm,
        command::{Command, completion_from_tcl_error, opt_get},
    };

    #[test]
    fn byte_guest_error_and_code_survive_the_completion_boundary() {
        let mut vm = Vm::new();
        let completion = completion_from_tcl_error(
            &mut vm,
            TclError::with_error_code(b"message \xff\0tail", b"RAW \xfe\0code"),
        );
        assert_eq!(completion.code, Code::Error);
        assert_eq!(
            completion.result.string_bytes().as_ref(),
            b"message \xff\0tail"
        );
        assert_eq!(
            opt_get(&completion.options, "-errorcode")
                .expect("carried code")
                .string_bytes()
                .as_ref(),
            b"RAW \xfe\0code"
        );
        assert!(vm.refused_completion().is_none());

        let propagated =
            completion_from_tcl_error(&mut vm, TclError::with_code(b"return \xff", Code::Return));
        assert_eq!(propagated.code, Code::Return);
        assert_eq!(propagated.result.string_bytes().as_ref(), b"return \xff");
    }

    #[test]
    fn propagated_guest_completion_retains_original_objects_and_options() {
        let mut vm = Vm::new();
        for code in [
            Code::Return,
            Code::Break,
            Code::Continue,
            Code::Other(7),
            Code::Error,
        ] {
            let result = Value::from_string_bytes(b"original \xff\0result".as_slice());
            let options = crate::command::options_dict(
                Code::Error,
                2,
                &[
                    (
                        "-errorcode",
                        Value::from_string_bytes(b"RAW \xfe\0code".as_slice()),
                    ),
                    (
                        "-errorinfo",
                        Value::from_string_bytes(b"original \xff\0info".as_slice()),
                    ),
                    ("-custom", Value::list(vec![Value::int(19)])),
                ],
            );
            let completion = Completion::new(code, result.clone(), options.clone());
            let propagated =
                completion_from_tcl_error(&mut vm, TclError::from_completion(completion));
            assert_eq!(propagated.code, code);
            assert!(propagated.result.is_same_object(&result));
            assert!(propagated.options.is_same_object(&options));
            assert!(vm.refused_completion().is_none());
        }
    }

    #[test]
    fn byte_guest_error_survives_command_logging_and_catch_publication() {
        let mut vm = Vm::new();
        let completion = crate::command::err_with_code(b"guest \xff\0tail", b"RAW \xfe\0code");
        let original_result = completion.result.native_lifetime_lease();
        let original_options = completion.options.native_lifetime_lease();
        let original_code = opt_get(&completion.options, "-errorcode").unwrap();
        let completion = vm.publish_native_interp_completion(completion).unwrap();
        assert!(completion.result.is_same_object(original_result.value()));
        assert!(completion.options.is_same_object(original_options.value()));
        assert!(
            vm.native_return_error_code()
                .unwrap()
                .is_same_object(&original_code)
        );
        vm.log_command_info("raw_failure", completion.result.string_bytes(), 1);
        let snapshot = vm.completion_options_snapshot(&completion);
        assert!(
            opt_get(&snapshot, "-errorinfo")
                .expect("logged trace")
                .string_bytes()
                .starts_with(b"guest \xff\0tail")
        );
        let result = vm.finish_catch(completion, None, None, 0);
        assert_eq!(result.code, Code::Ok);
        assert_eq!(
            vm.get_var("::errorCode")
                .expect("published code")
                .string_bytes()
                .as_ref(),
            b"RAW \xfe\0code"
        );
        assert!(
            vm.get_var("::errorInfo")
                .expect("published trace")
                .string_bytes()
                .starts_with(b"guest \xff\0tail")
        );
        assert!(vm.refused_completion().is_none());
    }

    #[test]
    fn guest_error_receiver_preserves_the_original_primitive_code_owner() {
        let mut vm = Vm::new();
        let original = vm.apply_primitive_error_code(
            tcl_cmd_core::ResolvedCmdErrorCodeUpdate::Set(b"TCL VALUE INTEGER".to_vec()),
        );
        let references = original.native_object_reference_count();
        let completion = crate::command::err_with_code(b"integer required", b"TCL VALUE INTEGER");
        let rebuilt = opt_get(&completion.options, "-errorcode").unwrap();
        assert!(!rebuilt.is_same_object(&original));
        let completion = vm.publish_native_interp_completion(completion).unwrap();
        assert!(
            vm.native_return_error_code()
                .unwrap()
                .is_same_object(&original)
        );
        assert_eq!(original.native_object_reference_count(), references);
        assert_eq!(completion.code, Code::Error);
        assert!(vm.refused_completion().is_none());
    }

    #[test]
    fn guest_error_receiver_does_not_invent_missing_code_metadata() {
        let mut vm = Vm::new();
        let completion = Completion::new_error_metadata(
            Code::Error,
            Value::from_string_bytes(b"guest error".as_slice()),
            crate::command::options_dict(Code::Error, 0, &[]),
        );
        let options = completion.options.native_lifetime_lease();
        let completion = vm.publish_native_interp_completion(completion).unwrap();
        assert!(completion.options.is_same_object(options.value()));
        assert!(vm.native_return_error_code().is_none());
        assert!(opt_get(&completion.options, "-errorcode").is_none());
        assert!(vm.refused_completion().is_none());
    }

    #[test]
    fn frozen_guest_completion_restores_the_same_error_code_object() {
        let mut vm = Vm::new();
        let completion = crate::command::err_with_code(b"original message", b"RAW \xfe\0code");
        let original = opt_get(&completion.options, "-errorcode").unwrap();
        vm.apply_primitive_error_code(tcl_cmd_core::ResolvedCmdErrorCodeUpdate::Set(
            b"FINALLY OTHER".to_vec(),
        ));
        vm.restore_completion_error_state(&completion);
        assert!(
            vm.native_return_error_code()
                .unwrap()
                .is_same_object(&original)
        );
        assert_eq!(
            vm.native_return_error_code()
                .unwrap()
                .string_bytes()
                .as_ref(),
            b"RAW \xfe\0code"
        );
        assert!(vm.refused_completion().is_none());
    }

    #[test]
    fn frozen_guest_code_restoration_uses_original_storage_in_six_engines() {
        for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
            let mut vm = Vm::with_native_core(
                Box::new(std::io::sink()),
                std::rc::Rc::new(crate::host_native::NativeHost::new()),
                profile,
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            let completion = crate::command::err_with_code(b"original message", b"RAW \xfe\0code");
            let original = opt_get(&completion.options, "-errorcode").unwrap();
            vm.restore_completion_error_state(&completion);
            assert!(
                vm.native_return_error_code()
                    .unwrap()
                    .is_same_object(&original),
                "{engine}"
            );
            if engine == "tcl8.4" {
                assert!(
                    vm.get_var("::errorCode").unwrap().is_same_object(&original),
                    "{engine}"
                );
            }
            assert_eq!(
                vm.native_return_error_code()
                    .unwrap()
                    .string_bytes()
                    .as_ref(),
                b"RAW \xfe\0code",
                "{engine}"
            );
            assert!(vm.refused_completion().is_none(), "{engine}");
        }
    }

    #[test]
    fn expression_math_function_preserves_complete_abrupt_guest_objects() {
        struct Abrupt(Completion<Value>);
        impl crate::command::NativeCommand for Abrupt {
            fn invoke(&self, _: &mut Vm, _: &[Value]) -> Completion<Value> {
                self.0.clone()
            }
        }
        for code in [
            Code::Return,
            Code::Break,
            Code::Continue,
            Code::Other(7),
            Code::Error,
        ] {
            let mut vm = Vm::new();
            vm.set_runtime_version(tcl_dialect::TclVersion::V9_0);
            let result = Value::from_string_bytes(b"math \xff\0result".as_slice());
            let options = crate::command::options_dict(
                Code::Error,
                2,
                &[
                    (
                        "-errorcode",
                        Value::from_string_bytes(b"MATH \xfe\0code".as_slice()),
                    ),
                    (
                        "-errorinfo",
                        Value::from_string_bytes(b"retained \xff\0info".as_slice()),
                    ),
                    (
                        "-custom",
                        Value::from_string_bytes(b"CUSTOM \xfe".as_slice()),
                    ),
                ],
            );
            vm.register_native_command(
                "::tcl::mathfunc::abrupt",
                std::rc::Rc::new(Abrupt(Completion::new(
                    code,
                    result.clone(),
                    options.clone(),
                ))),
            );
            let completion = vm.try_eval_expr("abrupt()").expect("guest completion");
            assert_eq!(completion.code, code);
            assert!(completion.result.is_same_object(&result));
            assert!(completion.options.is_same_object(&options));
        }
    }

    #[test]
    fn expression_quotes_follow_selected_native_control_and_level_protocol() {
        for engine in ["tcl9.0", "jim"] {
            let mut vm = Vm::new();
            let profile =
                tcl_registry::model::ingress::resolve_environment(engine).analyser_profile();
            vm.set_dialect_profile(profile);
            vm.set_compiler(Box::new(
                tcl_compiler::compile_service::BytecodeCompileService::for_profile(profile),
            ));
            let source = "\"prefix [return -level 2 -code error -errorcode {X Y} -errorinfo ORIGINAL MESSAGE]\"";
            let completion = vm.try_eval_expr(source).expect("guest quote completion");
            if engine == "jim" {
                assert_eq!(completion.code, Code::Ok);
                assert_eq!(completion.result.string_bytes().as_ref(), b"prefix MESSAGE");
                assert_eq!(
                    opt_get(&completion.options, "-level")
                        .expect("retained level")
                        .as_int()
                        .unwrap(),
                    2
                );
                let snapshot = vm.completion_options_snapshot(&completion);
                assert_eq!(opt_get(&snapshot, "-code").unwrap().as_int().unwrap(), 0);
                assert_eq!(opt_get(&snapshot, "-level").unwrap().as_int().unwrap(), 2);
            } else {
                assert_eq!(completion.code, Code::Return);
                assert_eq!(completion.result.string_bytes().as_ref(), b"MESSAGE");
                assert_eq!(
                    opt_get(&completion.options, "-errorcode")
                        .unwrap()
                        .string_bytes()
                        .as_ref(),
                    b"X Y"
                );
                assert_eq!(
                    opt_get(&completion.options, "-errorinfo")
                        .unwrap()
                        .string_bytes()
                        .as_ref(),
                    b"ORIGINAL"
                );
                assert_eq!(
                    opt_get(&completion.options, "-level")
                        .unwrap()
                        .as_int()
                        .unwrap(),
                    2
                );
            }
        }
    }

    #[test]
    fn typed_access_refusal_bypasses_catch_and_finally() {
        fn refuse(vm: &mut Vm, _: &[Value]) -> tcl_runtime_api::Completion<Value> {
            vm.set_var("reached", Value::int(1))
                .expect("unobserved handler entry marker");
            completion_from_tcl_error(
                vm,
                UnicodeAccessError {
                    valid_up_to: 0,
                    error_len: Some(1),
                }
                .into(),
            )
        }
        for source in [
            "catch {byte_refusal} caught; set after 1",
            "try {byte_refusal} on error {m o} {set handled 1} finally {set final 1}; set after 1",
            "after 0 byte_refusal; after 0 {set later 1}; update; set after 1",
        ] {
            let mut vm = Vm::new();
            vm.set_dialect_profile(
                tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            );
            vm.set_compiler(Box::new(
                tcl_compiler::compile_service::BytecodeCompileService::default(),
            ));
            vm.register_command("byte_refusal", Command::Builtin(refuse));
            let refusal = vm.try_eval_source(source).expect_err("host access refusal");
            assert_eq!(
                refusal,
                tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                    UnicodeAccessError {
                        valid_up_to: 0,
                        error_len: Some(1),
                    }
                    .into()
                ),
                "{source}"
            );
            assert_eq!(
                vm.get_var("reached")
                    .expect("actual refusing handler reached")
                    .string_bytes()
                    .as_ref(),
                b"1"
            );
            for name in ["caught", "handled", "final", "later", "after"] {
                assert!(vm.get_var(name).is_none(), "{source}: {name}");
            }
        }
    }
}
