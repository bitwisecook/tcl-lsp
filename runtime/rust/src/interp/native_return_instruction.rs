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
        message: *mut crate::obj::TclObj,
        options: *mut crate::obj::TclObj,
    ) -> Result<(), ValueError> {
        let protocol = self
            .native_invocation_dialect()
            .native_return_options_application(NativeReturnOptionsApplication::Syntax)
            .filter(|recipe| recipe.syntax_options_share_message())
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original compiler Syntax publication issuer",
            ))?;
        let dictionary =
            crate::dict::PreparedNativeDictionary::prepare(Some(options), protocol.strings())?;
        let key = crate::obj::Owned::fresh(crate::obj::new_string_bytes(b"-errorcode"));
        let code = dictionary
            .with_member(key.as_ptr(), |member| member)?
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original compiler Syntax error code",
            ))?;
        self.set_result(message);
        self.retain_native_error_option(true, message);
        self.retain_native_error_option(false, code);
        self.exc.borrow_mut().native.legacy_copy = true;
        drop(dictionary);
        self.reset_original_c_compiler_result()
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
    ) -> Result<crate::obj::Owned, ValueError> {
        use tcl_runtime_api::completion_options::{self, ErrorOptions, OptionValue};
        let protocol = self
            .native_invocation_dialect()
            .native_return_options_application(NativeReturnOptionsApplication::Syntax)
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original syntax-error options",
            ))?;
        let strings = protocol.strings();
        if self.exc.borrow().native.info.is_none() {
            let bytes = crate::dict::native_object_bytes(self.result.get(), strings)?;
            if protocol.resets_error_line_without_info() {
                self.error_line.set(1);
            }
            let mut error = self.exc.borrow_mut();
            error.native.info = Some(crate::obj::Owned::retain(self.result.get()));
            error.native.info_len = bytes.len();
            error.info = Some(bytes);
        }
        if self.exc.borrow().native.code.is_none() {
            self.replace_native_error_code(b"NONE");
        }
        {
            let mut error = self.exc.borrow_mut();
            error.native.legacy_copy = true;
            if protocol.options_getter_copies_shared_info() {
                let original = error
                    .native
                    .info
                    .as_ref()
                    .expect("initialized native errorInfo");
                if crate::obj::is_shared(original.as_ptr()) {
                    let copied = crate::obj::Owned::fresh(crate::obj::duplicate(original.as_ptr()));
                    error.native.info = Some(copied);
                }
            }
        }
        let carried = self.pending_return_option_objects();
        let values = carried
            .iter()
            .map(|pair| (pair.key_bytes.clone(), pair.value.clone()))
            .collect::<Vec<_>>();
        let metadata = ErrorOptions {
            error_code: self.native_private_error_object(false),
            error_info: self.native_private_error_object(true),
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
        Ok(crate::obj::Owned::fresh(crate::dict::new_dict_obj_native(
            &pointers, None, strings,
        )?))
    }

    /// The compiler's reached modern Tcl_ResetResult boundary, after its
    /// original Syntax literals have acquired their own array references.
    pub(crate) fn reset_original_c_compiler_result(&mut self) -> Result<(), ValueError> {
        self.native_invocation_dialect()
            .native_return_options_application(NativeReturnOptionsApplication::Syntax)
            .filter(|protocol| protocol.omits_syntax_error_stack())
            .ok_or(ValueError::CommandProtocolUnavailable(
                "compiler result reset",
            ))?;
        let original = self.result.get();
        crate::obj::check_native_liveness(original)?;
        if crate::obj::is_shared(original) {
            let empty = crate::obj::Owned::fresh(crate::obj::new_string_bytes(b""));
            let retired = self.result.replace(empty.into_raw());
            // SAFETY: transfer the interpreter's one original result reference.
            unsafe { crate::obj::decr_ref_count(retired) };
        } else {
            crate::obj::reset_native_c_result(original);
        }
        self.publish_native_error_objects();
        self.mark_error_stack_reset();
        self.set_return_state(1, Code::Ok);
        self.clear_return_options();
        *self.exc.borrow_mut() = super::ExceptionState::default();
        self.clear_during();
        Ok(())
    }

    /// Apply an already-merged original instruction operand. This does not
    /// publish the result, log the instruction, or perform RETURN_STK's merge.
    pub(crate) fn process_original_c_return_options(
        &mut self,
        purpose: NativeReturnOptionsApplication,
        code: i32,
        level: i64,
        original_options: *mut TclObj,
    ) -> Result<Code, ValueError> {
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
        if !self.return_options.borrow().is_original(original_options) {
            let original =
                NativeReturnOptions::from_original(original_options, protocol.strings())?;
            self.restore_return_options(original);
        }
        if code == 1 {
            {
                let mut error = self.exc.borrow_mut();
                error.native.info = None;
                error.native.info_len = 0;
                error.info = None;
            }
            let pairs = crate::dict::native_dict_pairs(original_options, protocol.strings())?;
            // Dictionary extraction borrows original values. The retained
            // private header keeps them alive without temporary child owners.
            let find = |wanted: &[u8]| -> Result<Option<*mut TclObj>, ValueError> {
                for &(key, value) in &pairs {
                    if crate::dict::native_object_bytes(key, protocol.strings())? == wanted {
                        return Ok(Some(value));
                    }
                }
                Ok(None)
            };
            if let Some(info) = find(b"-errorinfo")? {
                let bytes = crate::dict::native_object_bytes(info, protocol.strings())?;
                if !bytes.is_empty() {
                    self.retain_native_error_option(true, info);
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
                self.retain_native_error_option(false, error_code);
                let mut error = self.exc.borrow_mut();
                error.code.clear();
                error.code_explicit = true;
            } else {
                self.replace_native_error_code(b"NONE");
                let mut error = self.exc.borrow_mut();
                error.code = b"NONE".to_vec();
                error.code_explicit = false;
            }
            if let Some(line) = find(b"-errorline")? {
                // NULL-interpreter GetInt still reaches cache mutations, while
                // an authentic guest conversion failure leaves errorLine alone.
                if let Ok(NativeScalarGetterValue::Wide(value)) =
                    crate::typed_value::native_scalar_probe(
                        line,
                        dialect,
                        NativeScalarGetterKind::Int,
                    )?
                {
                    self.error_line.set(value as i32 as u32);
                }
            }
        }
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
