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

//! How a failed getter or command publishes its code and result: seeded codes, the structured presenters and the arity protocol.

use super::*;

/// Seed the actual selected error-code storage used by the native probes.
/// ExceptionState bytes alone do not publish C8.4's global errorCode cell.
fn seed_original_getter_code(interp: &mut Interp, bytes: &[u8]) {
    interp.set_error_state(bytes);
    let code = crate::obj::Owned::fresh(new_string(bytes));
    // SAFETY: the actual interpreter and borrowed original code header are live.
    unsafe { crate::capi::Tcl_SetObjErrorCode(interp, code.as_ptr()) };
    assert!(interp.native_execution_refusal().is_none());
    assert_eq!(interp.error_code(), bytes);
}

#[test]
fn c84_primitive_invalid_code_is_preserved_until_actual_script_propagation() {
    // naming.numeric.original-capi-scalar-publication-width
    // docs/design/analysis/name-resolution-proofs/numeric-original-capi-scalar-publication-width.md
    // naming.numeric.seeded-wide-frontier
    // docs/design/analysis/name-resolution-proofs/numeric-seeded-wide-frontier.md
    // Software publication at the actual Runtime boundary, independently
    // of the public primitive fields and the original callback captures.
    use tcl_syntax::scalar_getter::{
        NativeScalarGetterFailure, NativeScalarGetterKind, NativeScalarGetterProtocol,
    };
    leak_free(|interp| {
        interp.set_runtime_version(tcl_dialect::TclVersion::V8_4);
        let protocol = NativeScalarGetterProtocol::for_tcl_version(tcl_dialect::TclVersion::V8_4);
        for kind in [
            NativeScalarGetterKind::Int,
            NativeScalarGetterKind::Long,
            NativeScalarGetterKind::Wide,
            NativeScalarGetterKind::Double,
            NativeScalarGetterKind::Boolean,
        ] {
            seed_original_getter_code(interp, b"SEEDED CODE");
            let record = protocol
                .failure_presentation(kind, NativeScalarGetterFailure::Invalid, b"bad")
                .unwrap();
            let message = record.message_bytes().to_vec();
            let error = tcl_syntax::value::ValueError::NativeScalarGetter(Box::new(record));
            assert_eq!(interp.report_cmd_error(error.into()), Code::Error);
            assert_eq!(interp.error_code(), b"SEEDED CODE", "{kind:?} direct");
            assert_eq!(interp.result_bytes(), message);
            interp.log_command_bytes(1, b"original_getter");
            assert_eq!(interp.error_code(), b"NONE", "{kind:?} Eval");
            assert!(interp.exc.borrow().primitive_getter.is_none());
        }
        seed_original_getter_code(interp, b"SEEDED CODE");
        let record = protocol
            .failure_presentation(
                NativeScalarGetterKind::Long,
                NativeScalarGetterFailure::IntegerOverflow,
                b"18446744073709551616",
            )
            .unwrap();
        assert_eq!(
            interp.report_cmd_error(
                tcl_syntax::value::ValueError::NativeScalarGetter(Box::new(record)).into(),
            ),
            Code::Error
        );
        let code = interp.error_code();
        assert!(code.starts_with(b"ARITH IOVERFLOW"));
        interp.log_command_bytes(1, b"original_getter");
        assert_eq!(interp.error_code(), code);
    });
}

#[test]
fn primitive_getter_preserves_seeded_code_until_script_propagation() {
    use tcl_syntax::scalar_getter::{
        NativeScalarGetterFailure, NativeScalarGetterKind, NativeScalarGetterProtocol,
    };
    leak_free(|interp| {
        interp.set_error_state(b"PROBE BEFORE");
        interp.mark_error_code_explicit();
        let protocol = NativeScalarGetterProtocol::for_tcl_version(tcl_dialect::TclVersion::V8_5);
        let record = protocol
            .failure_presentation(
                NativeScalarGetterKind::Wide,
                NativeScalarGetterFailure::CachedNonInteger,
                b"1.5\0X",
            )
            .expect("audited cached C8.5 getter failure");
        let primitive_message = record.message_bytes().to_vec();
        let propagated_message = record.eval_message_bytes().to_vec();
        assert_ne!(primitive_message, propagated_message);
        let error = tcl_syntax::value::ValueError::NativeScalarGetter(Box::new(record));
        assert_eq!(interp.report_cmd_error(error.into()), Code::Error);
        assert_eq!(interp.result_bytes(), primitive_message);
        assert_eq!(interp.error_code(), b"PROBE BEFORE");
        assert!(interp.exc.borrow().code_explicit);

        let snapshot = interp.snapshot_error();
        interp.error_with_code(b"other", b"OTHER");
        interp.set_result_bytes(&primitive_message);
        let _ = interp.restore_error(snapshot);
        interp.log_command_bytes(1, b"getter");
        assert_eq!(interp.result_bytes(), propagated_message);
        assert_eq!(interp.error_code(), b"PROBE BEFORE");
        assert!(interp.exc.borrow().primitive_getter.is_none());

        let record = protocol
            .failure_presentation(
                NativeScalarGetterKind::Wide,
                NativeScalarGetterFailure::CachedNonInteger,
                b"1.5\0X",
            )
            .expect("same primitive stage inside contextual command error");
        let error = tcl_cmd_core::CmdError::from(
            tcl_syntax::value::ValueError::NativeScalarGetter(Box::new(record)),
        );
        let mut details = error.into_byte_details();
        details.message.splice(0..0, b"context: ".iter().copied());
        interp.report_cmd_error(tcl_cmd_core::CmdError::from_byte_details(details));
        interp.log_command_bytes(1, b"getter");
        let mut wrapped = b"context: ".to_vec();
        wrapped.extend_from_slice(&propagated_message);
        assert_eq!(interp.result_bytes(), wrapped);
    });
}

#[test]
#[cfg(have_tommath)]
fn c84_direct_invalid_expression_preserves_code_until_eval() {
    // Native proof: naming.expression.original-direct-arithmetic-error-stage
    // docs/design/analysis/name-resolution-proofs/expression-original-direct-arithmetic-error-stage.md

    leak_free(|interp| {
        interp.set_runtime_version(tcl_dialect::TclVersion::V8_4);
        for source in [b"\"bad\" + 1".as_slice(), b"\"bad\" ? 1 : 0".as_slice()] {
            seed_original_getter_code(interp, b"PROBE BEFORE");
            assert_eq!(
                crate::builtins::eval_expr_obj(interp, source),
                Err(Code::Error)
            );
            assert_eq!(interp.error_code(), b"PROBE BEFORE");
            assert!(interp.exc.borrow().expression_error_stage.is_some());
            interp.log_command_bytes(1, b"expr");
            assert_eq!(interp.error_code(), b"NONE");
        }
        seed_original_getter_code(interp, b"PROBE BEFORE");
        assert_eq!(
            crate::builtins::eval_expr_obj(interp, b"NaN + 1"),
            Err(Code::Error)
        );
        assert_ne!(interp.error_code(), b"PROBE BEFORE");
        assert!(interp.exc.borrow().expression_error_stage.is_none());
        let domain = interp.error_code();
        interp.log_command_bytes(1, b"expr");
        assert_eq!(interp.error_code(), domain);
    });
}

/// What the stage above is for, as a script sees it: tclsh 8.4.20 leaves
/// `errorCode` NONE after a non-numeric or non-boolean operand, and sets the
/// arithmetic code after a domain error.
#[test]
#[cfg(have_tommath)]
fn c84_invalid_expression_operands_leave_error_code_none() {
    leak_free(|interp| {
        interp.set_runtime_version(tcl_dialect::TclVersion::V8_4);
        for (script, expected) in [
            (
                b"set errorCode PROBE; list [catch {expr {\"bad\" + 1}} m] $m $errorCode".as_slice(),
                "1 {can't use non-numeric string as operand of \"+\"} NONE",
            ),
            (
                b"set errorCode PROBE; list [catch {expr {\"bad\" ? 1 : 0}} m] $m $errorCode".as_slice(),
                "1 {expected boolean value but got \"bad\"} NONE",
            ),
            (
                b"set errorCode PROBE; list [catch {expr {1/0}} m] $m $errorCode".as_slice(),
                "1 {divide by zero} {ARITH DIVZERO {divide by zero}}",
            ),
            (
                b"set errorCode PROBE; list [catch {expr {sqrt(-1)}} m] $m $errorCode".as_slice(),
                "1 {domain error: argument not in valid range} {ARITH DOMAIN {domain error: argument not in valid range}}",
            ),
            (
                b"set errorCode PROBE; list [catch {expr {NaN + 1}} m] $m $errorCode".as_slice(),
                "1 {domain error: argument not in valid range} {ARITH DOMAIN {domain error: argument not in valid range}}",
            ),
        ] {
            assert_eq!(interp.eval_str(script), Code::Ok);
            assert_eq!(String::from_utf8_lossy(&interp.result_bytes()), expected, "{}", String::from_utf8_lossy(script));
        }
    });
}

#[test]
#[cfg(have_tommath)]
fn counted_syntax_failures_keep_direct_code_and_apply_eval_update() {
    for &(engine, case, source, message, direct, propagated) in
        tcl_test_support::expressions::NATIVE_EXPRESSION_SYNTAX_STATES
    {
        leak_free(|interp| {
            interp.set_dialect_profile(crate::environment::profile_for_dialect(engine));
            interp.set_error_state(b"PROBE BEFORE");
            assert_eq!(
                crate::builtins::eval_expr_obj(interp, source),
                Err(Code::Error),
                "{engine}/{case}"
            );
            assert_eq!(interp.result_bytes(), message, "{engine}/{case}");
            assert_eq!(interp.error_code(), direct, "{engine}/{case} Direct");
            let snapshot = interp.snapshot_error();
            interp.error_with_code(b"other failure", b"OTHER");
            interp.set_result_bytes(message);
            let _ = interp.restore_error(snapshot);
            interp.log_command_bytes(1, b"expr $probeSource");
            assert_eq!(interp.error_code(), propagated, "{engine}/{case} Eval");
            assert_eq!(interp.result_bytes(), message, "{engine}/{case}");
            assert!(interp.exc.borrow().expression_error_stage.is_none());
        });
    }
}

#[test]
fn native_wrong_arguments_string_result_requires_the_structured_presenter() {
    // naming.variable.original-upvar-and-exists-completion-and-name-windows
    // docs/design/analysis/name-resolution-proofs/variable.original-upvar-and-exists-completion-and-name-windows.md
    // naming.list.original-assign-objects-and-instructions
    // docs/design/analysis/name-resolution-proofs/list.original-assign-objects-and-instructions.md
    // The original tables independently observe C84 upvar case11 and
    // C85 lassign case28 String results. This tests the shared presenter.
    use tcl_syntax::native_object::NativeObjectCacheSnapshot as Cache;
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut interp = Interp::with_native_core(
            default_host(),
            crate::environment::profile_for_dialect(engine),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        let error = tcl_cmd_core::CmdError::wrong_args_bytes(b"upvar ?level? otherVar localVar");
        let message = error.message_bytes().to_vec();
        assert_eq!(interp.report_cmd_error(error), Code::Error);
        assert!(!interp.host_refusal_pending(), "{engine}");
        assert!(
            matches!(
                obj::native_object_snapshot(interp.result_obj())
                    .unwrap()
                    .cache,
                Cache::String { .. }
            ),
            "{engine}"
        );
        assert_eq!(interp.result_bytes(), message, "{engine}");
        assert_eq!(
            interp.report_cmd_error(tcl_cmd_core::CmdError::new_bytes(message.as_slice())),
            Code::Error
        );
        assert!(
            matches!(
                obj::native_object_snapshot(interp.result_obj())
                    .unwrap()
                    .cache,
                Cache::None
            ),
            "{engine}: unrelated message"
        );
    }
}

#[test]
fn unchanged_primitive_projection_preserves_the_original_string_result() {
    // naming.compiler.introspection-source-and-effect-frontiers
    // docs/design/analysis/name-resolution-proofs/compiler-introspection-source-and-effect-frontiers.md
    // Native case13 observes the selected integer-getter String result.
    // This separate owner control checks unchanged propagation identity.
    use tcl_syntax::scalar_getter::{
        NativeScalarGetterFailure, NativeScalarGetterKind, NativeScalarGetterProtocol,
    };
    for engine in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut interp = Interp::with_native_core(
            default_host(),
            crate::environment::profile_for_dialect(engine),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        let version = interp.native_invocation_dialect().tcl_version.unwrap();
        let record = NativeScalarGetterProtocol::for_tcl_version(version)
            .failure_presentation(
                NativeScalarGetterKind::Int,
                NativeScalarGetterFailure::Invalid,
                b"invalid",
            )
            .expect("retained native integer failure");
        let error = tcl_cmd_core::CmdError::from(
            tcl_syntax::value::ValueError::NativeScalarGetter(Box::new(record)),
        )
        .with_native_string_result(tcl_syntax::native_string::NativeStringProtocol::C(version));
        assert_eq!(interp.report_cmd_error(error), Code::Error);
        let original = obj::Owned::retain(interp.result_obj());
        let before = obj::native_object_snapshot(original.as_ptr()).unwrap();
        assert!(matches!(
            before.cache,
            tcl_syntax::native_object::NativeObjectCacheSnapshot::String { .. }
        ));
        interp.propagate_error_stage().unwrap();
        assert_eq!(interp.result_obj(), original.as_ptr(), "{engine}");
        assert_eq!(
            obj::native_object_snapshot(interp.result_obj())
                .unwrap()
                .cache,
            before.cache,
            "{engine}"
        );
        assert!(interp.exc.borrow().primitive_getter.is_none());
    }
}

#[test]
fn command_error_default_and_explicit_updates_replace_seeded_code() {
    for (environment, expected) in [
        ("tcl8.4", b"NONE".as_slice()),
        ("tcl8.5", b"NONE".as_slice()),
        ("tcl8.6", b"TCL WRONGARGS".as_slice()),
        ("tcl9.0", b"TCL WRONGARGS".as_slice()),
        ("tcl9.1", b"TCL WRONGARGS".as_slice()),
        ("jim", b"NONE".as_slice()),
    ] {
        leak_free(|interp| {
            interp.set_dialect_profile(crate::environment::profile_for_dialect(environment));
            interp.set_error_state(b"PROBE BEFORE");
            interp.mark_error_code_explicit();
            let authentic = tcl_cmd_core::CmdError::wrong_args_bytes(b"n\0\xc0\x80\xff arg");
            let original = authentic.message_bytes().to_vec();
            interp.report_cmd_error(tcl_cmd_core::CmdError::new_bytes(original.as_slice()));
            assert_eq!(interp.result_bytes(), original);
            assert_eq!(interp.error_code(), b"NONE", "{environment}");
            assert!(!interp.exc.borrow().code_explicit);
            interp.report_cmd_error(authentic);
            assert_eq!(interp.result_bytes(), original);
            assert_eq!(interp.error_code(), expected, "{environment}");
            interp.report_cmd_error(tcl_cmd_core::CmdError::with_error_code("bad", "EXACT CODE"));
            assert_eq!(interp.error_code(), b"EXACT CODE");
        });
    }
}

#[test]
fn missing_arity_protocol_refuses_before_guest_state_changes() {
    leak_free(|interp| {
        let unknown = Box::leak(Box::new(tcl_dialect::DialectProfile::projected_from_point(
            "jim",
            &[],
            "Jim",
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_79),
        )));
        interp.set_dialect_profile(unknown);
        // Bootstrap release globals use their owned slots; installing an
        // unsupported profile does not issue an unrelated variable lookup.
        assert_eq!(interp.native_access_refusal(), None);
        assert!(interp
                .native_invocation_dialect()
                .wrong_arguments_protocol(Some(tcl_native_model::native_wrong_arguments::LogicalWrongArgumentsProvider::Tcl84CoreSimulation))
                .is_none());
        interp.set_result_bytes(b"RESULT BEFORE");
        interp.set_error_state(b"CODE BEFORE");
        assert_eq!(
            interp.report_cmd_error(tcl_cmd_core::CmdError::wrong_args("native arg")),
            Code::Error
        );
        assert_eq!(interp.result_bytes(), b"RESULT BEFORE");
        assert_eq!(interp.exc.borrow().code, b"CODE BEFORE");
        assert_eq!(
            interp.error_code_bytes_checked().unwrap_err(),
            interp.native_execution_refusal().unwrap()
        );
        assert_eq!(
            interp.native_access_refusal(),
            Some(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "wrong arguments",
                ),
            )
        );
    });
}

#[test]
fn jim_diagnostic_argv_borrows_preserve_sharing_and_lazy_string_representation() {
    fn probe(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
        // SAFETY: invocation owns the argument through this callback.
        assert_eq!(unsafe { (*argv[1]).ref_count }, 1);
        assert!(unsafe { (*argv[1]).bytes.is_null() });
        // SAFETY: argv is live; set_obj_result takes its own result hold.
        unsafe { interp.set_obj_result(argv[1]) };
        Code::Ok
    }
    fn fail(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
        assert_eq!(unsafe { (*argv[1]).ref_count }, 1);
        assert!(unsafe { (*argv[1]).bytes.is_null() });
        interp.error(b"BOOM")
    }
    let mut interp = Interp::new();
    interp.set_dialect_profile(crate::environment::profile_for_dialect("jim"));
    interp
        .jim_evaluation_frames
        .borrow_mut()
        .push(OriginalJimEvaluationFrame {
            procedure_level: 0,
            command: None,
            is_procedure: false,
            script: None,
            invocation: Vec::new(),
        });
    for (callback, expected) in [
        (probe as BuiltinFn, Code::Ok),
        (fail as BuiltinFn, Code::Error),
    ] {
        let argv = [new_string(b"probe"), obj::new_wide_int_obj(42)];
        for word in argv {
            unsafe { obj::incr_ref_count(word) };
        }
        assert_eq!(
            interp.invoke_bound(Command::Builtin(callback), None, &argv),
            expected
        );
        assert!(interp.jim_invocation_borrows.borrow().is_empty());
        if expected == Code::Ok {
            assert!(unsafe { (*argv[1]).bytes.is_null() });
        } else {
            assert_eq!(interp.jim_stacktrace().unwrap(), b"{} {} 1 {probe 42}");
        }
        release_all(&argv);
        interp.set_result_bytes(b"");
    }
    interp.jim_evaluation_frames.borrow_mut().pop();
}
