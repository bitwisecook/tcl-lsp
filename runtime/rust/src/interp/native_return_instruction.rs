// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original merged Dictionary operands at TclProcessReturn instruction boundaries.

use super::{Code, Interp, NativeReturnOptions};
use crate::obj::TclObj;
use tcl_registry::native_return_options::NativeReturnOptionsApplication;
use tcl_syntax::{
    scalar_getter::{NativeScalarGetterKind, NativeScalarGetterValue},
    value::ValueError,
};

impl Interp {
    /// Reset a reached C9.1 compile error after both original array operands
    /// exist, publishing the actual private fields through native variable setters.
    pub(super) fn publish_original_compiler_syntax(
        &mut self,
        message: *mut TclObj,
        options: *mut TclObj,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        let scope = super::native_operation_currency::NativeOperationScope::enter(self)?;
        let reached = (|| -> Result<(), tcl_cmd_core::CmdError> {
            let protocol = self
                .native_invocation_dialect()
                .native_return_options_application(NativeReturnOptionsApplication::Syntax)
                .filter(|recipe| recipe.syntax_options_share_message())
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "original compiler Syntax publication issuer",
                ))?;
            crate::obj::check_native_liveness(message)?;
            crate::obj::check_native_liveness(options)?;
            let message_lifetime = crate::obj::NativeObjectLifetime::retain(message);
            let options_lifetime = crate::obj::NativeObjectLifetime::retain(options);
            let dictionary = crate::dict::PreparedNativeDictionary::prepare_using(
                Some(options),
                protocol.strings(),
                &|| {
                    scope.currency().ensure_current_or_refuse()?;
                    crate::obj::check_native_liveness(message_lifetime.as_ptr())
                        .and_then(|()| crate::obj::check_native_liveness(options_lifetime.as_ptr()))
                        .map_err(|error| self.clone().refuse_completion_value_access(error))
                },
                &|error| self.clone().refuse_completion_value_access(error),
            );
            scope.currency().ensure_current_or_refuse()?;
            let dictionary = dictionary?;
            let key = crate::obj::Owned::fresh(crate::obj::new_string_bytes(b"-errorcode"));
            let code = dictionary.with_member(key.as_ptr(), |member| member);
            scope.currency().ensure_current_or_refuse()?;
            let code = code?.ok_or(ValueError::CommandProtocolUnavailable(
                "original compiler Syntax error code",
            ))?;
            self.set_result(message);
            scope.currency().ensure_current_or_refuse()?;
            crate::obj::check_native_liveness(message_lifetime.as_ptr())?;
            self.retain_native_error_option_checked(scope.currency(), true, message)?;
            self.retain_native_error_option_checked(scope.currency(), false, code)?;
            self.exc.borrow_mut().native.legacy_copy = true;
            drop(key);
            drop(dictionary);
            drop(options_lifetime);
            drop(message_lifetime);
            scope.currency().ensure_current_or_refuse()?;
            self.reset_original_c_compiler_result()?;
            Ok(())
        })();
        scope.currency().ensure_current_or_refuse()?;
        reached
    }

    /// Capture a reached return/Syntax instruction's original operands before cleanup.
    pub(crate) fn capture_original_return_instruction_context(
        &self,
        purpose: NativeReturnOptionsApplication,
        result: *mut TclObj,
        options: *mut TclObj,
    ) {
        if let Some(name) = self
            .native_invocation_dialect()
            .native_return_options_application(purpose)
            .and_then(|recipe| recipe.inner_context_name())
        {
            if !self.exc.borrow().already_logged {
                self.error_stack
                    .borrow_mut()
                    .begin_instruction(name, &[result, options]);
            }
        }
    }
    /// Capture the native compile-failure options after the parser publishes
    /// its actual result/private error code, before evaluation resets them.
    pub(crate) fn capture_original_c_syntax_options(
        &mut self,
    ) -> Result<crate::obj::Owned, tcl_cmd_core::CmdError> {
        use tcl_registry::special_vars::NativeErrorStorageVariable as Variable;
        use tcl_runtime_api::completion_options::{self, ErrorOptions, OptionValue};
        let scope = super::native_operation_currency::NativeOperationScope::enter(self)?;
        let reached = (|| -> Result<crate::obj::Owned, tcl_cmd_core::CmdError> {
            let protocol = self
                .native_invocation_dialect()
                .native_return_options_application(NativeReturnOptionsApplication::Syntax)
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "original syntax-error options",
                ))?;
            let strings = protocol.strings();
            if self.exc.borrow().native.info.is_none() {
                let original = self.result.get();
                crate::obj::check_native_liveness(original)?;
                let lifetime = crate::obj::NativeObjectLifetime::retain(original);
                let bytes = self.completion_result_bytes_checked();
                scope.currency().ensure_current_or_refuse()?;
                crate::obj::check_native_liveness(lifetime.as_ptr())?;
                if self.result.get() != original {
                    return Err(self
                        .refuse_completion_value_access(ValueError::CommandProtocolUnavailable(
                            "original Syntax result replaced during getter",
                        ))
                        .into());
                }
                let bytes = bytes?;
                if protocol.resets_error_line_without_info() {
                    self.error_line.set(1);
                }
                self.replace_native_error_object_checked(
                    scope.currency(),
                    Variable::ErrorInfo,
                    Some(crate::obj::Owned::retain(original)),
                    Some(bytes.len()),
                )?;
                self.exc.borrow_mut().info = Some(bytes);
                drop(lifetime);
                scope.currency().ensure_current_or_refuse()?;
            }
            if self.exc.borrow().native.code.is_none() {
                self.replace_native_error_code_checked(scope.currency(), b"NONE")?;
            }
            self.exc.borrow_mut().native.legacy_copy = true;
            if protocol.options_getter_copies_shared_info() {
                let original = self
                    .exc
                    .borrow()
                    .native
                    .info
                    .as_ref()
                    .map(crate::obj::Owned::as_ptr)
                    .ok_or(ValueError::CommandProtocolUnavailable(
                        "initialized native errorInfo",
                    ))?;
                crate::obj::check_native_liveness(original)?;
                let original_lifetime = crate::obj::NativeObjectLifetime::retain(original);
                self.check_native_error_object(
                    scope.currency(),
                    Variable::ErrorInfo,
                    &original_lifetime,
                )?;
                if crate::obj::is_shared(original) {
                    let copied = crate::obj::Owned::fresh(crate::obj::duplicate(original));
                    self.check_native_error_object(
                        scope.currency(),
                        Variable::ErrorInfo,
                        &original_lifetime,
                    )?;
                    self.replace_native_error_object_checked(
                        scope.currency(),
                        Variable::ErrorInfo,
                        Some(copied),
                        None,
                    )?;
                }
                drop(original_lifetime);
                scope.currency().ensure_current_or_refuse()?;
            }
            let carried = self.pending_return_option_objects_checked(scope.currency())?;
            let values = carried
                .iter()
                .map(|pair| (pair.key_bytes.clone(), pair.value.clone()))
                .collect::<Vec<_>>();
            let metadata = ErrorOptions {
                error_code: self.native_private_error_object_checked(scope.currency(), false)?,
                error_info: self.native_private_error_object_checked(scope.currency(), true)?,
                error_line: Some(i64::from(self.error_line.get() as i32)),
                ..ErrorOptions::default()
            };
            let mut planned = completion_options::plan(
                self.runtime_version(),
                tcl_runtime_api::Code::Error,
                0,
                &values,
                Some(&metadata),
            );
            if !protocol.syntax_retains_error_stack() {
                planned.retain(|(key, _)| key.as_slice() != b"-errorstack");
            }
            let members = planned
                .into_iter()
                .map(|(key, value)| {
                    let original_key = carried
                        .iter()
                        .find(|pair| pair.key_bytes == key)
                        .map_or_else(
                            || crate::obj::Owned::fresh(crate::obj::new_string_bytes(&key)),
                            |pair| pair.key.clone(),
                        );
                    let original_value = match value {
                        OptionValue::Integer(value) => {
                            crate::obj::Owned::fresh(crate::obj::new_wide_int_obj(value))
                        }
                        OptionValue::Value(value) => value,
                    };
                    (original_key, original_value)
                })
                .collect::<Vec<_>>();
            let pointers = members
                .iter()
                .map(|(key, value)| (key.as_ptr(), value.as_ptr()))
                .collect::<Vec<_>>();
            let options = crate::dict::new_dict_obj_native_with_stage_check(
                &pointers,
                None,
                strings,
                &|| scope.currency().ensure_current_or_refuse(),
            )?;
            let options = crate::obj::Owned::fresh(options);
            drop(members);
            drop(metadata);
            drop(values);
            drop(carried);
            scope.currency().ensure_current_or_refuse()?;
            Ok(options)
        })();
        scope.currency().ensure_current_or_refuse()?;
        reached
    }

    /// The compiler's reached modern Tcl_ResetResult boundary, after its
    /// original Syntax literals have acquired their own array references.
    pub(crate) fn reset_original_c_compiler_result(
        &mut self,
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        let scope = super::native_operation_currency::NativeOperationScope::enter(self)?;
        self.native_invocation_dialect()
            .native_return_options_application(NativeReturnOptionsApplication::Syntax)
            .filter(|protocol| protocol.omits_syntax_error_stack())
            .ok_or_else(|| {
                self.refuse_completion_value_access(ValueError::CommandProtocolUnavailable(
                    "compiler result reset",
                ))
            })?;
        let reached = self.reset_original_c_result();
        scope.currency().ensure_current_or_refuse()?;
        reached
    }

    /// Reset the actual C result, then publish and release its error fields in
    /// native order. C8.4 clears its episode flags and leaves the global cells
    /// and their original object references intact.
    pub(super) fn reset_original_c_result(
        &mut self,
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        let scope = super::native_operation_currency::NativeOperationScope::enter(self)?;
        self.native_invocation_dialect()
            .native_error_log_protocol()
            .ok_or_else(|| {
                self.refuse_completion_value_access(ValueError::CommandProtocolUnavailable(
                    "original C result reset",
                ))
            })?;
        let original = self.result.get();
        crate::obj::check_native_liveness(original)
            .map_err(|error| self.refuse_completion_value_access(error))?;
        let lifetime = crate::obj::NativeObjectLifetime::retain(original);
        let published = if crate::obj::is_shared(original) {
            let empty = crate::obj::Owned::fresh(crate::obj::new_string_bytes(b""));
            let published = crate::obj::NativeObjectLifetime::retain(empty.as_ptr());
            let retired = self.result.replace(empty.into_raw());
            // SAFETY: transfer the interpreter's one actual result reference.
            unsafe { crate::obj::decr_ref_count(retired) };
            published
        } else {
            crate::obj::reset_native_c_result(original);
            lifetime.clone()
        };
        scope.currency().ensure_current_or_refuse()?;
        crate::obj::check_native_liveness(published.as_ptr())
            .map_err(|error| self.refuse_completion_value_access(error))?;
        if self.result.get() != published.as_ptr() {
            return Err(self.refuse_completion_value_access(
                ValueError::CommandProtocolUnavailable("original C result replaced during reset"),
            ));
        }
        drop(published);
        drop(lifetime);
        scope.currency().ensure_current_or_refuse()?;
        if self.uses_c84_global_error_info() {
            self.reset_native_global_error_episode();
            return scope.currency().ensure_current_or_refuse();
        }
        self.publish_native_error_objects_checked(scope.currency())?;
        self.mark_error_stack_reset();
        self.set_return_state(1, Code::Ok);
        self.clear_return_options();
        scope.currency().ensure_current_or_refuse()?;
        self.replace_exception_state_checked(scope.currency(), super::ExceptionState::default())?;
        self.clear_during();
        scope.currency().ensure_current_or_refuse()
    }

    /// Apply an already-merged original instruction operand. This does not
    /// publish the result, log the instruction, or perform RETURN_STK's merge.
    pub(crate) fn process_original_c_return_options(
        &mut self,
        purpose: NativeReturnOptionsApplication,
        code: i32,
        level: i64,
        original_options: *mut TclObj,
    ) -> Result<Code, tcl_cmd_core::CmdError> {
        let scope = super::native_operation_currency::NativeOperationScope::enter(self)?;
        let reached = (|| -> Result<Code, tcl_cmd_core::CmdError> {
            let dialect = self.native_invocation_dialect();
            let protocol = dialect
                .native_return_options_application(purpose)
                .filter(|protocol| {
                    protocol.retains_merged_header() && protocol.accepts_control(code, level)
                })
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "original merged return-instruction options",
                ))?;
            crate::obj::check_native_liveness(original_options)?;
            let options_lifetime = crate::obj::NativeObjectLifetime::retain(original_options);
            if !self.return_options.borrow().is_original(original_options) {
                let original = NativeReturnOptions::from_original_checked(
                    original_options,
                    protocol.strings(),
                    &|| {
                        scope.currency().ensure_current_or_refuse()?;
                        crate::obj::check_native_liveness(options_lifetime.as_ptr())
                            .map_err(|error| self.clone().refuse_completion_value_access(error))
                    },
                )?;
                self.restore_return_options(original);
                scope.currency().ensure_current_or_refuse()?;
            }
            if code == 1 {
                self.replace_native_error_object_checked(
                    scope.currency(),
                    tcl_registry::special_vars::NativeErrorStorageVariable::ErrorInfo,
                    None,
                    Some(0),
                )?;
                self.exc.borrow_mut().info = None;
                let pairs = crate::dict::native_dict_pairs_using(
                    original_options,
                    protocol.strings(),
                    &|| {
                        scope.currency().ensure_current_or_refuse()?;
                        crate::obj::check_native_liveness(options_lifetime.as_ptr())
                            .map_err(|error| self.clone().refuse_completion_value_access(error))
                    },
                    &|error| self.clone().refuse_completion_value_access(error),
                );
                scope.currency().ensure_current_or_refuse()?;
                let pairs = pairs?;
                let backing = crate::dict::original_dictionary_backing(original_options)?;
                let members = pairs
                    .iter()
                    .map(|&(key, value)| {
                        crate::obj::check_native_liveness(key)?;
                        crate::obj::check_native_liveness(value)?;
                        Ok((
                            crate::obj::NativeObjectLifetime::retain(key),
                            crate::obj::NativeObjectLifetime::retain(value),
                        ))
                    })
                    .collect::<Result<Vec<_>, ValueError>>()?;
                // Dictionary extraction borrows original values. The retained
                // private header keeps them alive without temporary child owners.
                let find = |wanted: &[u8]| -> Result<Option<*mut TclObj>, tcl_cmd_core::CmdError> {
                    for &(key, value) in &pairs {
                        let reached = crate::dict::native_object_bytes(key, protocol.strings());
                        scope.currency().ensure_current_or_refuse()?;
                        crate::dict::original_dictionary_pairs_current(
                            original_options,
                            &backing,
                            &pairs,
                        )?;
                        crate::obj::check_native_liveness(key)?;
                        crate::obj::check_native_liveness(value)?;
                        if reached? == wanted {
                            return Ok(Some(value));
                        }
                    }
                    Ok(None)
                };
                if let Some(info) = find(b"-errorinfo")? {
                    let bytes = crate::dict::native_object_bytes(info, protocol.strings());
                    scope.currency().ensure_current_or_refuse()?;
                    crate::dict::original_dictionary_pairs_current(
                        original_options,
                        &backing,
                        &pairs,
                    )?;
                    crate::obj::check_native_liveness(info)?;
                    let bytes = bytes?;
                    if !bytes.is_empty() {
                        self.retain_native_error_option_checked(scope.currency(), true, info)?;
                        let mut error = self.exc.borrow_mut();
                        error.info = Some(bytes);
                        error.already_logged = true;
                    }
                }
                if dialect
                    .native_error_objects_protocol()
                    .is_some_and(|objects| objects.has_error_stack())
                {
                    if let Some(stack) = find(b"-errorstack")? {
                        self.seed_original_error_stack(stack)?;
                    }
                }
                if let Some(error_code) = find(b"-errorcode")? {
                    // Tcl_SetObjErrorCode retains the original without GetString.
                    self.retain_native_error_option_checked(scope.currency(), false, error_code)?;
                    let mut error = self.exc.borrow_mut();
                    error.code.clear();
                    error.code_explicit = true;
                } else {
                    self.replace_native_error_code_checked(scope.currency(), b"NONE")?;
                    let mut error = self.exc.borrow_mut();
                    error.code = b"NONE".to_vec();
                    error.code_explicit = false;
                }
                if let Some(line) = find(b"-errorline")? {
                    // NULL-interpreter GetInt still reaches cache mutations, while
                    // an authentic guest conversion failure leaves errorLine alone.
                    let number = crate::typed_value::native_scalar_probe(
                        line,
                        dialect,
                        NativeScalarGetterKind::Int,
                    );
                    scope.currency().ensure_current_or_refuse()?;
                    crate::dict::original_dictionary_pairs_current(
                        original_options,
                        &backing,
                        &pairs,
                    )?;
                    crate::obj::check_native_liveness(line)?;
                    if let Ok(NativeScalarGetterValue::Wide(value)) = number? {
                        self.error_line.set(value as i32 as u32);
                    }
                }
                drop(members);
                scope.currency().ensure_current_or_refuse()?;
            }
            drop(options_lifetime);
            scope.currency().ensure_current_or_refuse()?;
            let code = Code::from_int(code);
            if level != 0 {
                self.set_return_state(level as usize, code);
                Ok(Code::Return)
            } else {
                if code == Code::Error {
                    self.exc.borrow_mut().native.legacy_copy = true;
                }
                Ok(code)
            }
        })();
        scope.currency().ensure_current_or_refuse()?;
        reached
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::obj;
    use tcl_dialect::TclVersion;
    use tcl_syntax::native_string::NativeStringProtocol;

    #[test]
    fn direct_instruction_retains_original_headers_and_unmaterialized_code() {
        for version in [
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let mut interp = Interp::with_native_core(
                super::super::default_host(),
                crate::environment::profile_for_dialect(version.dialect_profile_name()),
                tcl_registry::special_vars::NativeBootstrapInputs {
                    package_path: Vec::new(),
                    default_library: None,
                },
            )
            .unwrap();
            let strings = NativeStringProtocol::C(version);
            let member = obj::Owned::fresh(obj::new_string_bytes(b"ORIGINAL"));
            let code = obj::Owned::fresh(crate::list::new_list_obj_native(
                &[member.as_ptr()],
                strings,
            ));
            let key = obj::Owned::fresh(obj::new_string_bytes(b"-errorcode"));
            let options = obj::Owned::fresh(
                crate::dict::new_dict_obj_native(&[(key.as_ptr(), code.as_ptr())], None, strings)
                    .unwrap(),
            );
            assert!(!obj::has_string_rep(code.as_ptr()));
            let result_before = interp.result_obj();
            assert_eq!(
                interp
                    .process_original_c_return_options(
                        NativeReturnOptionsApplication::Immediate,
                        1,
                        2,
                        options.as_ptr(),
                    )
                    .unwrap(),
                Code::Return
            );
            assert_eq!(
                interp.return_options.borrow().header(),
                Some(options.as_ptr())
            );
            assert_eq!(
                interp.native_private_error_object(false).unwrap().as_ptr(),
                code.as_ptr()
            );
            assert!(!obj::has_string_rep(code.as_ptr()));
            assert_eq!(interp.result_obj(), result_before);
            let refs = unsafe { (*options.as_ptr()).ref_count };
            assert_eq!(
                interp
                    .process_original_c_return_options(
                        NativeReturnOptionsApplication::Syntax,
                        1,
                        0,
                        options.as_ptr(),
                    )
                    .unwrap(),
                Code::Error
            );
            assert_eq!(unsafe { (*options.as_ptr()).ref_count }, refs);
            assert_eq!(
                interp.return_options.borrow().header(),
                Some(options.as_ptr())
            );
            assert!(!obj::has_string_rep(code.as_ptr()));
            assert!(interp
                .process_original_c_return_options(
                    NativeReturnOptionsApplication::Stack,
                    1,
                    0,
                    options.as_ptr(),
                )
                .is_err());
            assert_eq!(
                interp.return_options.borrow().header(),
                Some(options.as_ptr())
            );
            let line_key = obj::Owned::fresh(obj::new_string_bytes(b"-errorline"));
            let line = obj::Owned::fresh(obj::new_string_bytes(b"7"));
            let line_options = obj::Owned::fresh(
                crate::dict::new_dict_obj_native(
                    &[
                        (key.as_ptr(), code.as_ptr()),
                        (line_key.as_ptr(), line.as_ptr()),
                    ],
                    None,
                    strings,
                )
                .unwrap(),
            );
            assert_eq!(
                interp
                    .process_original_c_return_options(
                        NativeReturnOptionsApplication::Immediate,
                        1,
                        0,
                        line_options.as_ptr()
                    )
                    .unwrap(),
                Code::Error
            );
            assert_eq!(interp.error_line(), 7);
            assert_eq!(
                obj::obj_type_ptr(line.as_ptr()),
                &obj::TCL_INT_TYPE as *const _
            );
            assert!(obj::has_string_rep(line.as_ptr()));
            let bad = obj::Owned::fresh(obj::new_string_bytes(b"bad"));
            let failed_options = obj::Owned::fresh(
                crate::dict::new_dict_obj_native(
                    &[(line_key.as_ptr(), bad.as_ptr())],
                    None,
                    strings,
                )
                .unwrap(),
            );
            assert_eq!(
                interp
                    .process_original_c_return_options(
                        NativeReturnOptionsApplication::Immediate,
                        1,
                        0,
                        failed_options.as_ptr()
                    )
                    .unwrap(),
                Code::Error
            );
            assert_eq!(interp.error_line(), 7);
            assert!(obj::obj_type_ptr(bad.as_ptr()).is_null());
            assert_eq!(interp.result_obj(), result_before);
            if version.has_error_stack() {
                let old = interp.original_error_stack_value();
                let stack_key = obj::Owned::fresh(obj::new_string_bytes(b"-errorstack"));
                let stack_options = obj::Owned::fresh(
                    crate::dict::new_dict_obj_native(
                        &[(stack_key.as_ptr(), old.as_ptr())],
                        None,
                        strings,
                    )
                    .unwrap(),
                );
                assert_eq!(
                    interp
                        .process_original_c_return_options(
                            NativeReturnOptionsApplication::Immediate,
                            1,
                            0,
                            stack_options.as_ptr()
                        )
                        .unwrap(),
                    Code::Error
                );
                assert_ne!(interp.original_error_stack_value().as_ptr(), old.as_ptr());
                assert_eq!(unsafe { (*old.as_ptr()).ref_count }, 2);
                assert!(!interp.error_stack.borrow().is_reset());
            }
        }
    }

    #[test]
    fn syntax_options_capture_uses_native_info_header_copy_and_numerical_controls() {
        let observations =
            include_str!("../../tests/data/native_return_instructions/observations.tsv");
        let mut matched = 0;
        for (version, native) in [
            (TclVersion::V8_5, "8.5.19"),
            (TclVersion::V8_6, "8.6.18"),
            (TclVersion::V9_0, "9.0.4"),
            (TclVersion::V9_1, "9.1.0"),
        ] {
            let mut interp = Interp::with_native_core(
                super::super::default_host(),
                crate::environment::profile_for_dialect(version.dialect_profile_name()),
                tcl_registry::special_vars::NativeBootstrapInputs {
                    package_path: Vec::new(),
                    default_library: None,
                },
            )
            .unwrap();
            assert_eq!(interp.error(b"SYNTAX"), Code::Error);
            let message = obj::Owned::retain(interp.result_obj());
            let options = interp.capture_original_c_syntax_options().unwrap();
            let strings = NativeStringProtocol::C(version);
            let pairs = crate::dict::native_dict_pairs(options.as_ptr(), strings).unwrap();
            let value = |wanted: &[u8]| {
                pairs
                    .iter()
                    .find(|(key, _)| obj::bytes_of(*key) == wanted)
                    .map(|(_, value)| *value)
            };
            let info = value(b"-errorinfo").unwrap();
            let code = value(b"-errorcode").unwrap();
            let control = value(b"-code").unwrap();
            let level = value(b"-level").unwrap();
            let prefix = format!("{native}\tcapture\t");
            let expected = observations
                .lines()
                .find(|row| row.starts_with(&prefix))
                .unwrap()
                .split('\t')
                .collect::<Vec<_>>();
            assert_eq!(
                info == message.as_ptr(),
                expected[2] == "1",
                "{native} original Info"
            );
            assert_eq!(
                code,
                interp.native_private_error_object(false).unwrap().as_ptr()
            );
            assert_eq!(obj::obj_type_ptr(control), &obj::TCL_INT_TYPE as *const _);
            assert_eq!(obj::obj_type_ptr(level), &obj::TCL_INT_TYPE as *const _);
            assert!(!obj::has_string_rep(control));
            assert!(!obj::has_string_rep(level));
            assert!(value(b"-errorstack").is_none());
            assert!(!obj::has_string_rep(code));
            assert_eq!(expected[3..], ["1", "int", "1", "int", "1", "1", "1"]);
            if version >= TclVersion::V8_6 {
                interp.reset_original_c_compiler_result().unwrap();
                assert!(interp.native_private_error_object(true).is_none());
                assert!(interp.native_private_error_object(false).is_none());
                let prefix = format!("{native}\tcapture-reset\t");
                let expected = observations
                    .lines()
                    .find(|row| row.starts_with(&prefix))
                    .unwrap()
                    .split('\t')
                    .collect::<Vec<_>>();
                assert_eq!(info == message.as_ptr(), expected[4] == "1");
                assert_eq!(
                    unsafe { (*message.as_ptr()).ref_count },
                    expected[5].parse::<isize>().unwrap()
                );
                assert_eq!(interp.result_bytes(), b"");
            }
            matched += 1;
        }
        assert_eq!(matched, 4);
    }
}

#[cfg(test)]
mod original_syntax_transport_tests {
    use super::super::{default_host, ExceptionState, NsId, WeakInterp};
    use super::*;
    use crate::obj::{self, Owned};
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };

    #[derive(Clone)]
    struct Original {
        counted: bool,
        interpreter: WeakInterp,
        mode: u8,
        effect_on_free: bool,
        arm: Rc<Cell<bool>>,
        updates: Rc<Cell<usize>>,
        frees: Rc<Cell<usize>>,
        references: Rc<RefCell<Vec<obj::TclSize>>>,
        parent: Rc<Cell<*mut TclObj>>,
    }
    fn reached_effect(state: &Original) {
        if !state.arm.get() {
            return;
        }
        if let Some(mut interpreter) = state.interpreter.upgrade() {
            // Actual reentry must not collide with exception/stack RefCell borrows.
            drop(interpreter.exc.borrow());
            drop(interpreter.error_stack.borrow());
            match state.mode {
                1 | 5 => {
                    interpreter.refuse_host_command("original Syntax callback first cause");
                }
                2 | 6 => {
                    let original = interpreter.runtime_context();
                    let mut changed = original.clone();
                    changed.packages = vec![("original-syntax-callback".into(), "1.0".into())];
                    interpreter.pin_context(&changed).unwrap();
                    interpreter.pin_context(&original).unwrap();
                }
                7 | 8 | 9 => {
                    // The new carried-key observer reaches the actual private
                    // slot. Its getter must run after the owner releases its borrow.
                    drop(interpreter.return_options.borrow_mut());
                    if state.mode == 8 {
                        interpreter
                            .refuse_host_command("original carried-key callback first cause");
                    } else if state.mode == 9 {
                        let original = interpreter.runtime_context();
                        let mut changed = original.clone();
                        changed.packages =
                            vec![("original-carried-key-callback".into(), "1.0".into())];
                        interpreter.pin_context(&changed).unwrap();
                        interpreter.pin_context(&original).unwrap();
                    }
                }
                3 if state.effect_on_free => {
                    let saved = interpreter
                        .save_native_variable_trace_result(true)
                        .unwrap()
                        .unwrap();
                    interpreter.mark_error_stack_reset();
                    interpreter
                        .restore_native_variable_trace_result(saved)
                        .unwrap();
                }
                3 => {
                    interpreter.set_result_bytes(b"REACHED REPLACEMENT");
                }
                4 => {
                    let parent = state.parent.get();
                    obj::change_type(parent, core::ptr::null(), 0);
                    unsafe { obj::set_string_rep(parent, b"RESTORED MEMBERS") };
                    let protocol = interpreter
                        .native_invocation_dialect()
                        .native_string_protocol()
                        .unwrap();
                    crate::list::list_elements_native_checked(parent, protocol).unwrap();
                }
                _ => {}
            }
        }
    }
    extern "C" fn update(value: *mut TclObj) {
        let state = unsafe { &*(obj::internal_rep(value) as usize as *const Original) };
        state.updates.set(state.updates.get() + 1);
        state
            .references
            .borrow_mut()
            .push(unsafe { (*value).ref_count });
        unsafe { obj::set_native_updater_string_rep(value, b"ORIGINAL SYNTAX VALUE", false) };
        if !state.effect_on_free {
            reached_effect(state);
        }
    }
    extern "C" fn retire(value: *mut TclObj) {
        let state = unsafe { Box::from_raw(obj::internal_rep(value) as usize as *mut Original) };
        if state.counted {
            state.frees.set(state.frees.get() + 1);
        }
        if state.counted && state.effect_on_free {
            state
                .references
                .borrow_mut()
                .push(unsafe { (*value).ref_count });
            reached_effect(&state);
        }
    }
    extern "C" fn duplicate(source: *mut TclObj, destination: *mut TclObj) {
        let state = unsafe { &*(obj::internal_rep(source) as usize as *const Original) };
        let mut copied = state.clone();
        copied.counted = false;
        obj::change_type(
            destination,
            &ORIGINAL,
            Box::into_raw(Box::new(copied)) as usize as u64,
        );
        if state.mode >= 5 {
            state
                .references
                .borrow_mut()
                .push(unsafe { (*source).ref_count });
            reached_effect(state);
        }
    }
    static ORIGINAL: obj::TclObjType = obj::TclObjType {
        name: c"originalSyntaxTransport".as_ptr(),
        free_int_rep_proc: Some(retire),
        dup_int_rep_proc: Some(duplicate),
        update_string_proc: Some(update),
        set_from_any_proc: None,
    };
    fn native(name: &str) -> Interp {
        let environment = tcl_registry::model::ingress::resolve_known_environment(name).unwrap();
        Interp::with_native_core(
            default_host(),
            environment.unit_profile(),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap()
    }
    fn original(
        interpreter: &Interp,
        mode: u8,
        effect_on_free: bool,
    ) -> (
        Owned,
        Rc<Cell<bool>>,
        Rc<Cell<usize>>,
        Rc<Cell<usize>>,
        Rc<RefCell<Vec<obj::TclSize>>>,
        Rc<Cell<*mut TclObj>>,
    ) {
        let arm = Rc::new(Cell::new(true));
        let updates = Rc::new(Cell::new(0));
        let frees = Rc::new(Cell::new(0));
        let references = Rc::new(RefCell::new(Vec::new()));
        let parent = Rc::new(Cell::new(core::ptr::null_mut()));
        let state = Box::new(Original {
            counted: true,
            interpreter: interpreter.downgrade(),
            mode,
            effect_on_free,
            arm: arm.clone(),
            updates: updates.clone(),
            frees: frees.clone(),
            references: references.clone(),
            parent: parent.clone(),
        });
        let value = obj::alloc_typed(&ORIGINAL, Box::into_raw(state) as usize as u64);
        (Owned::fresh(value), arm, updates, frees, references, parent)
    }
    fn retire_test_state(interpreter: &Interp) {
        let result = interpreter
            .result
            .replace(Owned::fresh(obj::new_string_bytes(b"")).into_raw());
        unsafe { obj::decr_ref_count(result) };
        let state = interpreter.exc.replace(ExceptionState::default());
        drop(state);
        let options = interpreter.return_options.replace(Default::default());
        drop(options);
    }

    #[test]
    fn original_syntax_result_getter_keeps_borrowed_header_and_full_first_cause() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Genuine software extension callbacks/roles; no Native getter chronology is inferred.
        for name in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            for mode in 0..4 {
                crate::counters::reset();
                {
                    let mut interpreter = native(name);
                    let state = interpreter.exc.replace(ExceptionState::default());
                    drop(state);
                    interpreter.error_line.set(37);
                    let (value, arm, updates, frees, references, _) =
                        original(&interpreter, mode, false);
                    interpreter.set_result(value.as_ptr());
                    let reached = interpreter.capture_original_c_syntax_options();
                    assert_eq!(updates.get(), 1, "{name} mode{mode}");
                    assert_eq!(&*references.borrow(), &[2]);
                    assert_eq!(reached.is_ok(), mode == 0);
                    if mode == 0 {
                        assert!(interpreter.native_execution_refusal().is_none());
                        assert_eq!(interpreter.get_obj_result(), value.as_ptr());
                        drop(reached.unwrap());
                    } else {
                        let error = reached.unwrap_err();
                        let first = interpreter.native_execution_refusal().unwrap();
                        assert_eq!(error.native_execution_refusal(), Some(&first));
                        assert!(interpreter.exc.borrow().native.info.is_none());
                        assert!(interpreter.exc.borrow().native.code.is_none());
                        assert_eq!(interpreter.error_line.get(), 37);
                        assert_eq!(interpreter.get_obj_result() == value.as_ptr(), mode != 3);
                        assert_eq!(
                            obj::native_object_snapshot(interpreter.get_obj_result())
                                .unwrap()
                                .resident
                                .unwrap()
                                .as_ref(),
                            if mode == 3 {
                                b"REACHED REPLACEMENT".as_slice()
                            } else {
                                b"ORIGINAL SYNTAX VALUE".as_slice()
                            }
                        );
                    }
                    arm.set(false);
                    retire_test_state(&interpreter);
                    drop(value);
                    assert_eq!(frees.get(), 1);
                }
                assert_eq!(crate::counters::finalize(), 0);
                assert_eq!(crate::counters::double_free_count(), 0);
            }
        }
    }

    #[test]
    fn original_syntax_shared_info_copy_checks_real_duplicate_before_adoption() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Actual extension duplication/borrowed roles, independent of Native reset/setter observations.
        for name in ["tcl8.5", "tcl8.6"] {
            for mode in [5, 6] {
                crate::counters::reset();
                {
                    let mut interpreter = native(name);
                    interpreter.set_result_bytes(b"PRIOR RESULT");
                    let prior = interpreter.get_obj_result();
                    let (value, arm, updates, frees, references, _) =
                        original(&interpreter, mode, true);
                    let scope =
                        super::super::native_operation_currency::NativeOperationScope::enter(
                            &interpreter,
                        )
                        .unwrap();
                    interpreter
                        .retain_native_error_option_checked(scope.currency(), true, value.as_ptr())
                        .unwrap();
                    drop(scope);
                    let error = interpreter.capture_original_c_syntax_options().unwrap_err();
                    let first = interpreter.native_execution_refusal().unwrap();
                    assert_eq!(error.native_execution_refusal(), Some(&first));
                    assert_eq!(updates.get(), 0);
                    assert_eq!(&*references.borrow(), &[2]);
                    assert_eq!(
                        interpreter
                            .exc
                            .borrow()
                            .native
                            .info
                            .as_ref()
                            .unwrap()
                            .as_ptr(),
                        value.as_ptr()
                    );
                    assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 2);
                    assert_eq!(interpreter.get_obj_result(), prior);
                    arm.set(false);
                    retire_test_state(&interpreter);
                    drop(value);
                    assert_eq!(frees.get(), 1);
                }
                assert_eq!(crate::counters::finalize(), 0);
                assert_eq!(crate::counters::double_free_count(), 0);
            }
        }
    }

    #[test]
    fn original_syntax_literal_action_transports_reached_header_retirement_refusal() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        use super::super::native_literal_pool::{
            NativeRuntimeLiteral as Literal, NativeRuntimeLiteralAction as Action,
        };
        for mode in [1, 2] {
            crate::counters::reset();
            {
                let mut interpreter = native("tcl9.1");
                interpreter.set_result_bytes(b"PRIOR");
                let prior = interpreter.get_obj_result();
                let (old, arm, updates, frees, references, _) = original(&interpreter, mode, true);
                interpreter.set_original_c_error_code(old.as_ptr()).unwrap();
                drop(old);
                let strings = interpreter
                    .native_invocation_dialect()
                    .native_string_protocol()
                    .unwrap();
                let key = Owned::fresh(obj::new_string_bytes(b"-errorcode"));
                let code = Owned::fresh(obj::new_string_bytes(b"SOURCE SYNTAX CODE"));
                let options = Owned::fresh(
                    crate::dict::new_dict_obj_native(
                        &[(key.as_ptr(), code.as_ptr())],
                        None,
                        strings,
                    )
                    .unwrap(),
                );
                let source = Owned::fresh(obj::new_string_bytes(b"ORIGINAL BODY"));
                let declarations = [
                    Literal::UnsharedBytes(b"REACHED SYNTAX MESSAGE".to_vec()),
                    Literal::UnsharedOriginal(options),
                ];
                let reached = interpreter.create_native_literal_array_with_actions(
                    source.as_ptr(),
                    &declarations,
                    &[
                        Action::Register(0),
                        Action::Register(1),
                        Action::RetainSyntaxErrorInfo {
                            options: 1,
                            message: 0,
                        },
                    ],
                );
                let error = reached
                    .err()
                    .expect("actual literal action must retain retirement refusal");
                assert_eq!(
                    error.native_execution_refusal(),
                    interpreter.native_execution_refusal().as_ref()
                );
                assert_eq!(frees.get(), 1);
                assert_eq!(updates.get(), 0);
                assert_eq!(&*references.borrow(), &[0]);
                assert_ne!(interpreter.get_obj_result(), prior);
                assert_eq!(
                    obj::native_object_snapshot(interpreter.get_obj_result())
                        .unwrap()
                        .resident
                        .unwrap()
                        .as_ref(),
                    b"REACHED SYNTAX MESSAGE"
                );
                arm.set(false);
                retire_test_state(&interpreter);
                drop(declarations);
            }
            assert_eq!(crate::counters::finalize(), 0);
            assert_eq!(crate::counters::double_free_count(), 0);
        }
    }

    #[test]
    fn original_builder_syntax_retains_callback_cause_before_formatter_or_cache() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        for name in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            for mode in [1, 2] {
                crate::counters::reset();
                {
                    let mut interpreter = native(name);
                    let (old, arm, updates, frees, references, _) =
                        original(&interpreter, mode, true);
                    interpreter.set_original_c_error_code(old.as_ptr()).unwrap();
                    drop(old);
                    let source = Owned::fresh(obj::new_string_bytes(b"{"));
                    let reached =
                        interpreter.prepare_original_c_body(source.as_ptr(), NsId::GLOBAL, None);
                    assert!(matches!(reached, Err(Code::Error)), "{name} mode{mode}");
                    let first = interpreter
                        .native_execution_refusal()
                        .expect("actual syntax worker first cause");
                    assert_eq!(frees.get(), 1);
                    assert_eq!(updates.get(), 0);
                    assert_eq!(&*references.borrow(), &[0]);
                    assert!(obj::obj_type_ptr(source.as_ptr()).is_null());
                    let result = interpreter.get_obj_result();
                    assert!(!obj::native_object_snapshot(result)
                        .unwrap()
                        .resident
                        .unwrap()
                        .is_empty());
                    let later = interpreter.capture_original_c_syntax_options().unwrap_err();
                    assert_eq!(later.native_execution_refusal(), Some(&first));
                    assert_eq!(interpreter.get_obj_result(), result);
                    arm.set(false);
                    retire_test_state(&interpreter);
                }
                assert_eq!(crate::counters::finalize(), 0);
                assert_eq!(crate::counters::double_free_count(), 0);
            }
        }
    }

    #[test]
    fn original_error_stack_replacement_checks_real_retirement_and_owner_aba() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // List-member retirement and mutation-ticket/backing callbacks are software controls, not Native error-stack timing.
        for name in ["tcl8.6", "tcl9.1"] {
            for mode in 0..5 {
                crate::counters::reset();
                {
                    let interpreter = native(name);
                    let strings = interpreter
                        .native_invocation_dialect()
                        .native_string_protocol()
                        .unwrap();
                    let (old, arm, updates, frees, references, parent) =
                        original(&interpreter, mode, true);
                    let tag = Owned::fresh(obj::new_string_bytes(b"INNER"));
                    let header = Owned::fresh(crate::list::new_list_obj_native(
                        &[tag.as_ptr(), old.as_ptr()],
                        strings,
                    ));
                    parent.set(header.as_ptr());
                    arm.set(false);
                    interpreter
                        .seed_original_error_stack(header.as_ptr())
                        .unwrap();
                    let captured = interpreter.original_error_stack_value();
                    parent.set(captured.as_ptr());
                    drop(captured);
                    drop(header);
                    drop(old);
                    arm.set(true);
                    let value = Owned::fresh(obj::new_string_bytes(b"REACHED VALUE"));
                    let supplied = Owned::fresh(crate::list::new_list_obj_native(
                        &[tag.as_ptr(), value.as_ptr()],
                        strings,
                    ));
                    let reached = interpreter.seed_original_error_stack(supplied.as_ptr());
                    assert_eq!(reached.is_ok(), mode == 0, "{name} mode{mode}");
                    assert_eq!(frees.get(), 1);
                    assert_eq!(updates.get(), 0);
                    assert_eq!(&*references.borrow(), &[0]);
                    if mode == 0 {
                        assert_eq!(
                            crate::list::list_elements_native_checked(parent.get(), strings)
                                .unwrap(),
                            [tag.as_ptr(), value.as_ptr()]
                        );
                        assert!(interpreter.native_execution_refusal().is_none());
                    } else {
                        let first = interpreter.native_execution_refusal().unwrap();
                        assert_eq!(
                            reached.unwrap_err().native_execution_refusal(),
                            Some(&first)
                        );
                        assert!(!interpreter.error_stack.borrow().is_reset());
                        assert_eq!(
                            interpreter.error_stack.borrow().original_header().unwrap(),
                            parent.get()
                        );
                    }
                    arm.set(false);
                }
                assert_eq!(crate::counters::finalize(), 0);
                assert_eq!(crate::counters::double_free_count(), 0);
            }
        }
    }

    #[test]
    fn original_error_converters_check_real_primary_retirement_before_backing_adoption() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Real software freeIntRep callbacks test transport and adoption boundaries,
        // without attesting external Tcl converter or error-stack chronology.
        for name in ["tcl8.6", "tcl9.1"] {
            for operation in 0..3 {
                if operation == 2 && name != "tcl9.1" {
                    continue;
                }
                for mode in 0..4 {
                    crate::counters::reset();
                    {
                        let mut interpreter = native(name);
                        let strings = interpreter
                            .native_invocation_dialect()
                            .native_string_protocol()
                            .unwrap();
                        let (value, arm, updates, frees, references, _) =
                            original(&interpreter, mode, true);
                        let spelling: &[u8] = if operation == 0 {
                            b"INNER VALUE"
                        } else {
                            b"-errorcode CODE"
                        };
                        unsafe { obj::set_string_rep(value.as_ptr(), spelling) };
                        let message = Owned::fresh(obj::new_string_bytes(b"COMPILE MESSAGE"));
                        let before = interpreter.error_stack.borrow().original_header();
                        let scope =
                            super::super::native_operation_currency::NativeOperationScope::enter(
                                &interpreter,
                            )
                            .unwrap();
                        let reached = match operation {
                            0 => interpreter.seed_original_error_stack(value.as_ptr()),
                            1 => NativeReturnOptions::from_original_checked(
                                value.as_ptr(),
                                strings,
                                &|| scope.currency().ensure_current_or_refuse(),
                            )
                            .map(drop),
                            _ => interpreter
                                .publish_original_compiler_syntax(message.as_ptr(), value.as_ptr()),
                        };
                        let accepted = mode == 0 || (mode == 3 && operation != 0);
                        assert_eq!(
                            reached.is_ok(),
                            accepted,
                            "{name} operation{operation} mode{mode}"
                        );
                        assert_eq!(frees.get(), 1);
                        assert_eq!(updates.get(), 0);
                        assert_eq!(
                            references.borrow().as_slice(),
                            &[if operation == 1 { 2 } else { 1 }]
                        );
                        if accepted {
                            let expected = if operation == 0 {
                                &crate::list::TCL_LIST_TYPE
                            } else {
                                &crate::dict::TCL_DICT_TYPE
                            };
                            assert!(core::ptr::eq(obj::obj_type_ptr(value.as_ptr()), expected));
                            assert!(interpreter.native_execution_refusal().is_none());
                        } else {
                            let first = interpreter.native_execution_refusal().unwrap();
                            assert_eq!(
                                reached.unwrap_err().native_execution_refusal(),
                                Some(&first)
                            );
                            // The real old primary was retired, but no later backing was installed.
                            assert!(obj::obj_type_ptr(value.as_ptr()).is_null());
                            assert_eq!(
                                obj::native_object_snapshot(value.as_ptr())
                                    .unwrap()
                                    .resident
                                    .unwrap()
                                    .as_ref(),
                                spelling
                            );
                            assert_eq!(interpreter.error_stack.borrow().original_header(), before);
                        }
                        arm.set(false);
                        retire_test_state(&interpreter);
                        drop(message);
                        drop(value);
                        drop(scope);
                    }
                    assert_eq!(crate::counters::finalize(), 0);
                    assert_eq!(crate::counters::double_free_count(), 0);
                }
            }
        }
    }

    #[test]
    fn original_completion_reads_keep_original_borrowed_roles_and_full_callback_cause() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Real software getter callbacks measure roles and typed transport, not external chronology.
        for name in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            for operation in 0..2 {
                for mode in 0..3 {
                    crate::counters::reset();
                    {
                        let mut interpreter = native(name);
                        interpreter.set_result_bytes(b"PRIOR COMPLETION");
                        let prior = interpreter.get_obj_result();
                        interpreter.error_line.set(37);
                        let (value, arm, updates, frees, references, _) =
                            original(&interpreter, mode, false);
                        if operation == 0 {
                            interpreter.set_result(value.as_ptr());
                        } else {
                            interpreter
                                .set_original_c_error_code(value.as_ptr())
                                .unwrap();
                        }
                        let scope =
                            super::super::native_operation_currency::NativeOperationScope::enter(
                                &interpreter,
                            )
                            .unwrap();
                        let reached = if operation == 0 {
                            interpreter.completion_result_bytes_checked()
                        } else {
                            interpreter.error_code_bytes_checked()
                        };
                        assert_eq!(updates.get(), 1, "{name} operation{operation} mode{mode}");
                        assert_eq!(references.borrow().as_slice(), &[2]);
                        assert_eq!(frees.get(), 0);
                        assert_eq!(interpreter.error_line.get(), 37);
                        assert_eq!(
                            interpreter.get_obj_result(),
                            if operation == 0 {
                                value.as_ptr()
                            } else {
                                prior
                            },
                        );
                        assert_eq!(
                            obj::native_object_snapshot(value.as_ptr())
                                .unwrap()
                                .resident
                                .unwrap()
                                .as_ref(),
                            b"ORIGINAL SYNTAX VALUE",
                        );
                        if mode == 0 {
                            assert_eq!(reached.unwrap(), b"ORIGINAL SYNTAX VALUE");
                            scope.currency().ensure_current_or_refuse().unwrap();
                            assert!(interpreter.native_execution_refusal().is_none());
                        } else {
                            let first = interpreter.native_execution_refusal().unwrap();
                            assert_eq!(reached.unwrap_err(), first);
                            let later = if operation == 0 {
                                interpreter.completion_result_bytes_checked()
                            } else {
                                interpreter.error_code_bytes_checked()
                            };
                            assert_eq!(later.unwrap_err(), first);
                            assert_eq!(updates.get(), 1);
                        }
                        arm.set(false);
                        retire_test_state(&interpreter);
                        drop(value);
                        drop(scope);
                        assert_eq!(frees.get(), 1);
                    }
                    assert_eq!(crate::counters::finalize(), 0);
                    assert_eq!(crate::counters::double_free_count(), 0);
                }
            }
        }
    }

    #[test]
    fn original_completion_options_read_real_carried_key_before_duplicate_or_overlay() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Actual stored Dictionary members and software updater effects, independent of Native options order.
        for name in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            for mode in 7..10 {
                crate::counters::reset();
                {
                    let mut interpreter = native(name);
                    interpreter.set_result_bytes(b"PRIOR COMPLETION");
                    let prior = interpreter.get_obj_result();
                    let (key, arm, updates, frees, references, _) =
                        original(&interpreter, mode, false);
                    arm.set(false);
                    unsafe { obj::set_string_rep(key.as_ptr(), b"ORIGINAL SYNTAX VALUE") };
                    let value = Owned::fresh(obj::new_string_bytes(b"CARRIED ORIGINAL VALUE"));
                    interpreter.set_return_option_objects(vec![
                        tcl_cmd_core::return_options::ReturnOptionPair {
                            key: key.clone(),
                            value: value.clone(),
                            key_bytes: b"ORIGINAL SYNTAX VALUE".to_vec(),
                        },
                    ]);
                    assert!(interpreter.native_execution_refusal().is_none());
                    let header = interpreter.return_options.borrow().header().unwrap();
                    let backing = crate::dict::original_dictionary_backing(header).unwrap();
                    assert_eq!(unsafe { (*key.as_ptr()).ref_count }, 2);
                    assert_eq!(updates.get(), 0);
                    obj::invalidate_string(key.as_ptr());
                    arm.set(true);
                    let scope =
                        super::super::native_operation_currency::NativeOperationScope::enter(
                            &interpreter,
                        )
                        .unwrap();
                    let reached = crate::cmd_error::completion_options(&mut interpreter, Code::Ok);
                    assert_eq!(updates.get(), 1, "{name} mode{mode}");
                    assert_eq!(references.borrow().as_slice(), &[2]);
                    assert_eq!(frees.get(), 0);
                    assert_eq!(interpreter.return_options.borrow().header(), Some(header));
                    crate::dict::original_dictionary_backing_current(header, &backing).unwrap();
                    assert_eq!(interpreter.get_obj_result(), prior);
                    if mode == 7 {
                        let options = Owned::fresh(reached.unwrap());
                        let strings = interpreter
                            .native_invocation_dialect()
                            .native_string_protocol()
                            .unwrap();
                        let pairs =
                            crate::dict::native_dict_pairs(options.as_ptr(), strings).unwrap();
                        assert!(pairs.iter().any(|&(original_key, original_value)| {
                            original_key == key.as_ptr() && original_value == value.as_ptr()
                        }));
                        for (name, expected) in [
                            (b"-code".as_slice(), b"0".as_slice()),
                            (b"-level".as_slice(), b"0".as_slice()),
                        ] {
                            let (_, original) = pairs
                                .iter()
                                .find(|&&(original, _)| obj::bytes_of(original) == name)
                                .unwrap();
                            assert_eq!(obj::bytes_of(*original), expected);
                        }
                        scope.currency().ensure_current_or_refuse().unwrap();
                        drop(options);
                    } else {
                        let first = interpreter.native_execution_refusal().unwrap();
                        assert_eq!(reached.unwrap_err(), first);
                        assert_eq!(unsafe { (*key.as_ptr()).ref_count }, 2);
                        assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 2);
                        assert!(interpreter.exc.borrow().native.info.is_none());
                        let later =
                            crate::cmd_error::completion_options(&mut interpreter, Code::Ok);
                        assert_eq!(later.unwrap_err(), first);
                        assert_eq!(updates.get(), 1);
                    }
                    arm.set(false);
                    retire_test_state(&interpreter);
                    drop(value);
                    drop(key);
                    drop(scope);
                    assert_eq!(frees.get(), 1);
                }
                assert_eq!(crate::counters::finalize(), 0);
                assert_eq!(crate::counters::double_free_count(), 0);
            }
        }
    }
}
