// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Public original argv error reporting, separate from retained source-command text.

use super::{Code, Interp};
use crate::obj::{self, Owned, TclObj};
use tcl_registry::native_object_vector::NativeObjectVectorErrorProjection;
use tcl_syntax::{native_string::NativeStringProtocol, value::ValueError};

struct JimVectorEvaluationFrame {
    interp: Interp,
    previous_len: usize,
}

impl Drop for JimVectorEvaluationFrame {
    fn drop(&mut self) {
        self.interp
            .jim_evaluation_frames
            .borrow_mut()
            .truncate(self.previous_len);
    }
}

impl Interp {
    /// Evaluate a borrowed original object vector through the public native
    /// entry. Callers keep all children live through completion; Jim additionally
    /// pins them for invocation. Source evaluators use `dispatch` instead: their
    /// error command is the retained written source, not a serialized argv.
    pub fn eval_original_object_vector(&mut self, argv: &[*mut TclObj]) -> Code {
        let Some(protocol) = self
            .native_invocation_dialect()
            .native_object_vector_protocol()
        else {
            return self.report_cmd_error(
                ValueError::CommandProtocolUnavailable("public original object-vector evaluation")
                    .into(),
            );
        };
        if argv.is_empty() && matches!(protocol.strings(), NativeStringProtocol::C(_)) {
            // TclInterpReady reaches Tcl_ResetResult before the empty-vector
            // return, with no command lookup or command counter increment.
            return match self.reset_original_c_result() {
                Ok(()) => Code::Ok,
                Err(error) => self.report_cmd_error(error.into()),
            };
        }
        let pins = protocol.pins_arguments().then(|| {
            argv.iter()
                .map(|word| Owned::retain(*word))
                .collect::<Vec<_>>()
        });
        let jim_frame = protocol.strings().is_jim084().then(|| {
            let mut frames = self.jim_evaluation_frames.borrow_mut();
            let previous_len = frames.len();
            let script = frames.last().and_then(|parent| parent.script.clone());
            frames.push(tcl_runtime_api::jim_error_stack::JimEvaluationFrame {
                procedure_level: self.jim_procedure_level.get(),
                command_name: None,
                is_procedure: false,
                script,
                invocation: Vec::new(),
            });
            JimVectorEvaluationFrame {
                interp: self.clone(),
                previous_len,
            }
        });
        let outer = self.eval_depth.get() == 0 && self.native_dispatch_depth.get() == 0;
        let previous_depth = self.eval_depth.get();
        self.eval_depth.set(previous_depth.saturating_add(1));
        let mut code = self.dispatch(argv);
        self.eval_depth.set(previous_depth);
        if !self.host_refusal_pending() && outer && !protocol.strings().is_jim084() {
            let original = code;
            code = self.settle_return(code);
            if protocol.unexpected_completion(original.as_int(), code.as_int()) {
                code = self.unexpected_object_vector_result(code);
            }
        }
        if code == Code::Error && !self.host_refusal_pending() {
            let logged = self.exc.borrow().already_logged;
            let command = match protocol.error_projection(logged) {
                NativeObjectVectorErrorProjection::CStringWords => {
                    let mut command = Vec::new();
                    for word in argv {
                        let bytes = match self.native_object_string_bytes(*word) {
                            Ok(bytes) => bytes,
                            Err(error) => return self.report_cmd_error(error.into()),
                        };
                        let end = bytes
                            .iter()
                            .position(|byte| *byte == 0)
                            .unwrap_or(bytes.len());
                        if !command.is_empty() {
                            command.push(b' ');
                        }
                        tcl_syntax::list::append_list_element(&mut command, &bytes[..end], false);
                    }
                    Some(command)
                }
                NativeObjectVectorErrorProjection::OriginalList => {
                    let list = Owned::fresh(self.new_list_object(argv));
                    match self.native_object_string_bytes(list.as_ptr()) {
                        Ok(bytes) => Some(bytes.to_vec()),
                        Err(error) => return self.report_cmd_error(error.into()),
                    }
                }
                NativeObjectVectorErrorProjection::None => None,
            };
            if let Some(command) = command {
                self.log_command_bytes(1, &command);
                if !logged {
                    self.exc.borrow_mut().already_logged = false;
                }
            }
        }
        if protocol.clears_logged_on_exit() {
            self.exc.borrow_mut().already_logged = false;
        }
        drop(jim_frame);
        drop(pins);
        code
    }

    pub(super) fn invalid_original_command(&mut self, original: *mut TclObj) -> Code {
        match self.native_object_string_bytes(original) {
            Ok(name) => self.invalid_command(&name),
            Err(error) => self.report_cmd_error(error.into()),
        }
    }

    fn unexpected_object_vector_result(&mut self, code: Code) -> Code {
        let message = match code {
            Code::Break => b"invoked \"break\" outside of a loop".to_vec(),
            Code::Continue => b"invoked \"continue\" outside of a loop".to_vec(),
            other => format!("command returned bad code: {}", other.as_int()).into_bytes(),
        };
        let error_code = self
            .native_invocation_dialect()
            .native_object_vector_protocol()
            .and_then(|protocol| protocol.unexpected_completion_error_code(code.as_int()));
        let result = match error_code {
            Some(error_code) => self.error_with_code(&message, &error_code),
            None => self.error(&message),
        };
        self.retain_object_vector_string_result(result)
    }

    fn retain_object_vector_string_result(&mut self, code: Code) -> Code {
        if !self.host_refusal_pending() {
            let Some(materialization) = self
                .native_invocation_dialect()
                .native_string_materialization(None)
            else {
                return self.report_cmd_error(
                    ValueError::CommandProtocolUnavailable(
                        "original object-vector String producer",
                    )
                    .into(),
                );
            };
            if let Err(error) =
                obj::retain_native_string_representation(self.get_obj_result(), materialization)
            {
                return self.report_cmd_error(error.into());
            }
        }
        code
    }

    pub(super) fn invalid_command_result(&mut self, name: &[u8]) -> Code {
        let protocol = self
            .native_invocation_dialect()
            .native_object_vector_protocol();
        let name = if protocol
            .is_some_and(|recipe| matches!(recipe.strings(), NativeStringProtocol::C(_)))
        {
            &name[..name
                .iter()
                .position(|byte| *byte == 0)
                .unwrap_or(name.len())]
        } else {
            name
        };
        let mut message = b"invalid command name \"".to_vec();
        message.extend_from_slice(name);
        message.push(b'"');
        let code = if protocol.is_some_and(|recipe| recipe.missing_command_has_lookup_code()) {
            let error_code = tcl_syntax::list_result::NativeListResultSerialization::Tcl85Plus
                .render(&[b"TCL".as_slice(), b"LOOKUP", b"COMMAND", name]);
            self.error_with_code(&message, &error_code)
        } else {
            self.error(&message)
        };
        if !self.host_refusal_pending()
            && protocol.is_some_and(|recipe| recipe.missing_command_has_string_primary())
        {
            return self.retain_object_vector_string_result(code);
        }
        code
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURES: [(&str, &str); 6] = [
        (
            "tcl8.4",
            include_str!("../../tests/data/native_object_vector/8.4.20.txt"),
        ),
        (
            "tcl8.5",
            include_str!("../../tests/data/native_object_vector/8.5.19.txt"),
        ),
        (
            "tcl8.6",
            include_str!("../../tests/data/native_object_vector/8.6.18.txt"),
        ),
        (
            "tcl9.0",
            include_str!("../../tests/data/native_object_vector/9.0.4.txt"),
        ),
        (
            "tcl9.1",
            include_str!("../../tests/data/native_object_vector/9.1.0.txt"),
        ),
        (
            "jim",
            include_str!("../../tests/data/native_object_vector/Jim.txt"),
        ),
    ];

    fn interpreter(environment: &str) -> Interp {
        let profile = crate::environment::profile_for_dialect(environment);
        if environment == "jim" {
            let mut interp = Interp::new();
            interp.set_dialect_profile(profile);
            interp
        } else {
            Interp::with_native_core(
                super::super::default_host(),
                profile,
                tcl_registry::special_vars::NativeBootstrapInputs {
                    package_path: Vec::new(),
                    default_library: None,
                },
            )
            .expect("authentic C constructor before bootstrap")
        }
    }

    fn fail(interp: &mut Interp, _argv: &[*mut TclObj]) -> Code {
        interp.error(b"FAILED")
    }
    fn nested(interp: &mut Interp, _argv: &[*mut TclObj]) -> Code {
        interp.eval_str(b"error CHILD")
    }
    fn pass(_interp: &mut Interp, _argv: &[*mut TclObj]) -> Code {
        Code::Ok
    }
    fn logged(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
        let code = fail(interp, argv);
        if !interp.uses_jim_error_stack() {
            interp.log_command_bytes(1, b"retained_source");
            // The native probe calls public Tcl_LogCommandInfo, which leaves
            // ERR_ALREADY_LOGGED clear. Source unwinding owns its separate flag.
            interp.exc.borrow_mut().already_logged = false;
        }
        code
    }
    fn primary(value: *mut TclObj) -> String {
        let kind = obj::obj_type_ptr(value);
        if kind.is_null() {
            "none".into()
        } else {
            // SAFETY: the interpreter result pins the descriptor-bearing header.
            unsafe { std::ffi::CStr::from_ptr((*kind).name) }
                .to_str()
                .unwrap()
                .into()
        }
    }
    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    #[test]
    fn empty_public_vectors_reset_original_result_owners_without_dispatch() {
        let fixtures = [
            ("tcl8.4", include_str!("../../../../rust/tcl-registry/tests/data/native_error_variables/empty-vector-reset-8.4.20.tsv")),
            ("tcl8.5", include_str!("../../../../rust/tcl-registry/tests/data/native_error_variables/empty-vector-reset-8.5.19.tsv")),
            ("tcl8.6", include_str!("../../../../rust/tcl-registry/tests/data/native_error_variables/empty-vector-reset-8.6.18.tsv")),
            ("tcl9.0", include_str!("../../../../rust/tcl-registry/tests/data/native_error_variables/empty-vector-reset-9.0.4.tsv")),
            ("tcl9.1", include_str!("../../../../rust/tcl-registry/tests/data/native_error_variables/empty-vector-reset-9.1.0.tsv")),
        ];
        for (environment, observations) in fixtures {
            let mut interp = interpreter(environment);
            for row in observations.lines().take(2) {
                let fields = row.split('\t').collect::<Vec<_>>();
                interp.set_result(obj::new_wide_int_obj(17));
                let original = interp.get_obj_result();
                let retained = (fields[0] == "1").then(|| Owned::retain(original));
                assert_eq!(
                    interp.eval_original_object_vector(&[]),
                    Code::from_int(fields[1].parse().unwrap()),
                    "{environment} row {row}"
                );
                let result = interp.get_obj_result();
                assert_eq!(result == original, fields[2] == "1", "{environment}");
                // The unshared header remains the result; the shared original
                // remains live through the probe's genuine external reference.
                unsafe {
                    assert_eq!((*result).ref_count, fields[3].parse().unwrap());
                    assert_eq!((*original).ref_count, fields[4].parse().unwrap());
                    assert_eq!(!(*result).bytes.is_null(), fields[6] == "1");
                    assert_eq!((*result).length, fields[7].parse().unwrap());
                }
                assert_eq!(primary(result), fields[5], "{environment}");
                assert!(!interp.host_refusal_pending(), "{environment}");
                drop(retained);
            }
        }
    }

    #[test]
    fn jim_public_vector_error_retains_original_argv_after_frame_pop() {
        let mut interp = Interp::with_native_core(
            super::super::default_host(),
            crate::environment::profile_for_dialect("jim"),
            tcl_registry::special_vars::NativeBootstrapInputs {
                package_path: Vec::new(),
                default_library: None,
            },
        )
        .unwrap();
        interp.register_builtin(b"fail_original", fail);
        interp.register_builtin(b"pass_original", pass);
        let argument = Owned::fresh(obj::new_string_bytes(b"original"));
        let pass_head = Owned::fresh(obj::new_string_bytes(b"pass_original"));
        assert_eq!(
            interp.eval_original_object_vector(&[pass_head.as_ptr(), argument.as_ptr()]),
            Code::Ok
        );
        assert!(interp.jim_evaluation_frames.borrow().is_empty());
        // SAFETY: the external original owner remains live.
        assert_eq!(unsafe { (*argument.as_ptr()).ref_count }, 1);
        let fail_head = Owned::fresh(obj::new_string_bytes(b"fail_original"));
        assert_eq!(
            interp.eval_original_object_vector(&[fail_head.as_ptr(), argument.as_ptr()]),
            Code::Error
        );
        assert!(interp.jim_evaluation_frames.borrow().is_empty());
        assert!(interp.jim_invocation_borrows.borrow().is_empty());
        // JimAddStackFrame retains the same argv in its invocation List after
        // Jim_EvalObjVector releases its temporary invocation references.
        assert_eq!(unsafe { (*argument.as_ptr()).ref_count }, 2);
        let trace = interp.jim_stacktrace_object();
        let invocation = crate::list::list_index(trace.as_ptr(), 3).unwrap().unwrap();
        assert_eq!(
            crate::list::list_index(invocation, 1).unwrap().unwrap(),
            argument.as_ptr()
        );
    }

    #[test]
    fn original_public_vectors_and_source_commands_match_all_60_native_windows() {
        let heads = [
            b"missing_original".as_slice(),
            b"fail_original",
            b"nested_original",
            b"pass_original",
            b"logged_original",
        ];
        let mut windows = 0;
        for (environment, fixture) in FIXTURES {
            for row in fixture.lines() {
                let expected: Vec<_> = row.split('\t').collect();
                let case: usize = expected[0].parse().unwrap();
                let mut interp = interpreter(environment);
                interp.register_builtin(b"fail_original", fail);
                interp.register_builtin(b"nested_original", nested);
                interp.register_builtin(b"pass_original", pass);
                interp.register_builtin(b"logged_original", logged);
                let member = Owned::fresh(obj::new_string_bytes(b"A B"));
                let arg = Owned::fresh(interp.new_list_object(&[member.as_ptr()]));
                let head = Owned::fresh(obj::new_string_bytes(heads[case % 5]));
                interp.var_set(b"arg", arg.as_ptr()).unwrap();
                assert!(!obj::has_string_rep(arg.as_ptr()));
                let code = if case < 5 {
                    interp.eval_original_object_vector(&[head.as_ptr(), arg.as_ptr()])
                } else {
                    let mut script = heads[case % 5].to_vec();
                    script.extend_from_slice(b" $arg");
                    interp.eval_str(&script)
                };
                assert!(!interp.host_refusal_pending(), "{environment}/{case}");
                // Capture physical facts before the result byte observer.
                let resident = u8::from(obj::has_string_rep(arg.as_ptr()));
                let result_primary = primary(interp.get_obj_result());
                let actual = format!(
                    "{case}\t{}\t{resident}\t{result_primary}\t{}",
                    code.as_int(),
                    hex(&interp.result_bytes())
                );
                assert_eq!(actual, row, "{environment}/{case}");
                windows += 1;
            }
        }
        assert_eq!(windows, 60);
    }

    #[test]
    fn absent_handler_produces_string_but_real_handler_preserves_original_result() {
        fn original(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
            interp.set_result(argv[2]);
            Code::Error
        }
        for (environment, _) in FIXTURES {
            if environment == "jim" {
                continue;
            }
            let mut interp = interpreter(environment);
            let head = Owned::fresh(obj::new_string_bytes(b"missing_original"));
            assert_eq!(
                interp.eval_original_object_vector(&[head.as_ptr()]),
                Code::Error
            );
            assert_eq!(primary(interp.get_obj_result()), "string", "{environment}");
            interp.register_builtin(b"unknown", original);
            let member = Owned::fresh(obj::new_string_bytes(b"SAME"));
            let arg = Owned::fresh(interp.new_list_object(&[member.as_ptr()]));
            assert_eq!(
                interp.eval_original_object_vector(&[head.as_ptr(), arg.as_ptr()]),
                Code::Error
            );
            assert_eq!(interp.get_obj_result(), arg.as_ptr(), "{environment}");
            assert_eq!(primary(arg.as_ptr()), "list", "{environment}");
        }
    }

    #[test]
    fn public_original_return_matches_all_six_native_completion_boundaries() {
        let fixtures = [
            (
                "tcl8.4",
                include_str!("../../tests/data/native_object_vector/completions/8.4.20.txt"),
            ),
            (
                "tcl8.5",
                include_str!("../../tests/data/native_object_vector/completions/8.5.19.txt"),
            ),
            (
                "tcl8.6",
                include_str!("../../tests/data/native_object_vector/completions/8.6.18.txt"),
            ),
            (
                "tcl9.0",
                include_str!("../../tests/data/native_object_vector/completions/9.0.4.txt"),
            ),
            (
                "tcl9.1",
                include_str!("../../tests/data/native_object_vector/completions/9.1.0.txt"),
            ),
            (
                "jim",
                include_str!("../../tests/data/native_object_vector/completions/Jim.txt"),
            ),
        ];
        for (environment, fixture) in fixtures {
            let mut interp = interpreter(environment);
            let member = Owned::fresh(obj::new_string_bytes(b"A B"));
            let original = Owned::fresh(interp.new_list_object(&[member.as_ptr()]));
            let head = Owned::fresh(obj::new_string_bytes(b"return"));
            let code = interp.eval_original_object_vector(&[head.as_ptr(), original.as_ptr()]);
            assert!(!interp.host_refusal_pending(), "{environment}");
            let resident = u8::from(obj::has_string_rep(original.as_ptr()));
            let result_primary = primary(interp.get_obj_result());
            let actual = format!(
                "0\t{}\t{resident}\t{result_primary}\t{}",
                code.as_int(),
                hex(&interp.result_bytes())
            );
            assert_eq!(actual, fixture.lines().next().unwrap(), "{environment}");
            if environment != "tcl8.5" {
                assert_eq!(interp.get_obj_result(), original.as_ptr(), "{environment}");
            }
        }
    }
}
