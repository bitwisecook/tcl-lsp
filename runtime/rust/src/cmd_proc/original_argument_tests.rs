// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original procedure name/body/formal getter order at definition entry.
use super::*;
use crate::cmd_namespace::original_argument_tests::{
    compare_events_at, compare_worker, failure, instance, jim_fixture_namespace, lazy, reset, row,
    text, unhex, FixtureCleanup, JIM_ORIGINAL, ORIGINALS,
};

#[test]
fn original_procedure_definition_matches_all_c_name_body_and_formal_getter_windows() {

    // Exact C499c original String reachability/public results, independently of
    // body duplication, sharing counts, compiled code or Jim procedure semantics.
    let mut compared = 0;
    for &(engine, capture) in ORIGINALS {
        for case in [9, 10, 11] {
            reset();
            let mut interpreter = instance(engine);
            assert_eq!(
                interpreter.eval_str(b"namespace eval ::d {}; set READY READY"),
                Code::Ok
            );
            let current = interpreter.find_namespace_id(b"::d").unwrap();
            interpreter.set_current_ns(current);
            let head = text(b"proc");
            let name = lazy(
                'A',
                if case == 9 {
                    b"::missing::p"
                } else {
                    b"::d::p"
                },
            );
            let formals = lazy('B', if case == 11 { b"" } else { b"{bad" });
            let body = lazy('C', b"return OK");
            // Native499c supplies an original body already owned by its variable
            // and argument. This explicit caller owner reaches the same shared
            // body branch; the argument adapter adds no native reference.
            let body_argument = Owned::retain(body.as_ptr());
            crate::codegen_abi::tcl_runtime_set_current_interp(&mut interpreter);
            let code = proc_cmd(
                &mut interpreter,
                &[
                    head.as_ptr(),
                    name.as_ptr(),
                    formals.as_ptr(),
                    body_argument.as_ptr(),
                ],
            );
            compare_worker(&mut interpreter, code, capture, case);
            interpreter.set_current_ns(crate::namespace::GLOBAL);
            assert_eq!(
                interpreter.eval_str(
                    b"list [info commands ::d::a] [info commands ::d::b] [info commands ::d::p]"
                ),
                Code::Ok
            );
            assert_eq!(
                interpreter.result_bytes(),
                unhex(row(capture, case, "commands")[4])
            );
            if case == 11 {
                let code = interpreter.eval_str(b"::d::p");
                let original = row(capture, case, "invoke");
                assert_eq!(code.as_int(), original[3].parse::<i64>().unwrap());
                assert_eq!(interpreter.result_bytes(), unhex(original[4]));
            }
            crate::codegen_abi::tcl_runtime_set_current_interp(core::ptr::null_mut());
            drop(body_argument);
            drop(body);
            drop(formals);
            drop(name);
            reset();
            compared += 1;
        }
    }
    assert_eq!(compared, 15);
}

#[test]
fn original_procedure_definition_matches_jim_formal_name_and_deferred_body_getters() {

    // Jim499d reaches the original formals and name at definition. The body
    // updater is reached by case11's later invocation, not by definition.
    for case in [9, 10, 11] {
        reset();
        let mut interpreter = instance("jim");
        let _cleanup = FixtureCleanup(|| {
            crate::codegen_abi::tcl_runtime_set_current_interp(core::ptr::null_mut());
        });
        assert_eq!(
            interpreter.eval_str(b"namespace eval ::d {}; set READY READY"),
            Code::Ok
        );
        let namespace_owner = jim_fixture_namespace(&mut interpreter, b"::d");
        let current = namespace_owner.token;
        interpreter.set_current_ns(current);
        let head = text(b"proc");
        let name = lazy(
            'A',
            if case == 9 {
                b"::missing::p"
            } else {
                b"::d::p"
            },
        );
        let formals = lazy('B', if case == 11 { b"" } else { b"{bad" });
        let body = lazy('C', b"return OK");
        crate::codegen_abi::tcl_runtime_set_current_interp(&mut interpreter);
        let code = proc_cmd(
            &mut interpreter,
            &[
                head.as_ptr(),
                name.as_ptr(),
                formals.as_ptr(),
                body.as_ptr(),
            ],
        );
        compare_worker(&mut interpreter, code, JIM_ORIGINAL, case);
        assert!(!obj::has_string_rep(body.as_ptr()));
        interpreter.set_current_ns(crate::namespace::GLOBAL);
        assert_eq!(
            interpreter.eval_str(
                b"list [info commands ::d::a] [info commands ::d::b] [info commands ::d::p]"
            ),
            Code::Ok
        );
        assert_eq!(
            interpreter.result_bytes(),
            unhex(row(JIM_ORIGINAL, case, "commands")[4])
        );
        compare_events_at(JIM_ORIGINAL, case, "commands");
        assert!(!obj::has_string_rep(body.as_ptr()));
        if case == 11 {
            let code = interpreter.eval_str(b"::d::p");
            let original = row(JIM_ORIGINAL, case, "invoke");
            assert_eq!(code.as_int(), original[3].parse::<i64>().unwrap());
            assert_eq!(interpreter.result_bytes(), unhex(original[4]));
            compare_events_at(JIM_ORIGINAL, case, "invoke");
        }
        crate::codegen_abi::tcl_runtime_set_current_interp(core::ptr::null_mut());
        drop(body);
        drop(formals);
        drop(name);
        reset();
    }
}

#[test]
fn original_shared_body_refusal_stops_before_formals_and_procedure_publication() {

    // Software refusal/currency control: the genuine shared body String stage
    // is reached after a resident name, before the original formal getter.
    for refusal in [1, 2] {
        reset();
        let mut interpreter = instance("tcl8.6");
        let head = text(b"proc");
        let name = text(b"p");
        let formals = lazy('B', b"x");
        let body = lazy('C', b"return OK");
        let body_argument = Owned::retain(body.as_ptr());
        let refs = unsafe { (*body.as_ptr()).ref_count };
        interpreter.set_result_bytes(b"PRIOR");
        let prior = interpreter.get_obj_result();
        failure(refusal);
        crate::codegen_abi::tcl_runtime_set_current_interp(&mut interpreter);
        assert_eq!(
            proc_cmd(
                &mut interpreter,
                &[
                    head.as_ptr(),
                    name.as_ptr(),
                    formals.as_ptr(),
                    body_argument.as_ptr()
                ]
            ),
            Code::Error
        );
        assert_eq!(obj::bytes_of(body.as_ptr()), b"return OK");
        assert!(!obj::has_string_rep(formals.as_ptr()));
        assert!(!interpreter.command_exists(b"p"));
        assert!(interpreter.native_execution_refusal().is_some());
        assert_eq!(interpreter.get_obj_result(), prior);
        assert_eq!(unsafe { (*body.as_ptr()).ref_count }, refs);
        assert_eq!(interpreter.entered_native_operation_count(), 0);
        crate::codegen_abi::tcl_runtime_set_current_interp(core::ptr::null_mut());
        drop(body_argument);
        drop(body);
        drop(formals);
        reset();
    }
}
