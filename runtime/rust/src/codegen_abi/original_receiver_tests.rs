// SPDX-License-Identifier: AGPL-3.0-or-later
//! Software controls for generated receiver currency and transferred ownership.

use super::*;
use crate::interp::Code;
use std::cell::Cell;
use tcl_runtime_api::NativeExecutionError;
use tcl_syntax::raw_string::NativeValueAccessRefusal;

thread_local! {
    static FREES: Cell<usize> = const { Cell::new(0) };
    static CALLS: Cell<usize> = const { Cell::new(0) };
    static UPDATES: Cell<usize> = const { Cell::new(0) };
}
fn original() -> Interp {
    Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect("tcl8.6"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}
fn changed(interpreter: &mut Interp) {
    let original = interpreter.runtime_context();
    let mut other = original.clone();
    other.packages = vec![("original-receiver".to_owned(), "1.0".to_owned())];
    interpreter.pin_context(&other).unwrap();
    interpreter.pin_context(&original).unwrap();
}
fn stale() -> NativeExecutionError {
    NativeExecutionError::ValueAccessRefusal(NativeValueAccessRefusal::CommandProtocolUnavailable(
        "stale entered native operation",
    ))
}
fn trace(interpreter: &mut Interp, _: &[*mut TclObj]) -> Code {
    CALLS.with(|calls| calls.set(calls.get() + 1));
    changed(interpreter);
    Code::Ok
}
extern "C" fn free_original(_: *mut TclObj) {
    FREES.with(|calls| calls.set(calls.get() + 1));
}
extern "C" fn updater(value: *mut TclObj) {
    UPDATES.with(|calls| calls.set(calls.get() + 1));
    let mut interpreter = unsafe { current_interp().as_ref() }.unwrap().clone();
    changed(&mut interpreter);
    unsafe { obj::set_native_updater_string_rep(value, b"17", false) };
}
static ORIGINAL: obj::TclObjType = obj::TclObjType {
    name: c"originalCodegenReceiver".as_ptr(),
    free_int_rep_proc: Some(free_original),
    dup_int_rep_proc: None,
    update_string_proc: Some(updater),
    set_from_any_proc: None,
};
unsafe fn consumed(route: usize, value: *mut TclObj) -> i32 {
    match route {
        0 => unsafe { tcl_codegen_local_bind(-1, b"x".as_ptr(), 1, value) },
        1 => unsafe { tcl_codegen_local_set(-1, value) },
        2 => unsafe { tcl_codegen_slot_bind(-1, b"x".as_ptr(), 1, value) },
        3 => unsafe { tcl_codegen_slot_set(-1, value) },
        4 => unsafe { tcl_codegen_var_set(b"x".as_ptr(), 1, value) },
        5 => unsafe {
            crate::codegen_native::tcl_codegen_var_set_element(
                b"a".as_ptr(),
                1,
                b"k".as_ptr(),
                1,
                value,
            )
        },
        6 => unsafe { tcl_codegen_puts(value) },
        _ => unreachable!(),
    }
}
#[test]
fn consumed_receiver_inputs_retire_once_before_every_early_refusal() {

    // Real extension backing retirement and generated +1 ownership, not an external observation.
    for with_interpreter in [false, true] {
        for route in 0..7 {
            let mut interpreter = original();
            interpreter.set_result_bytes(b"SEEDED RESULT");
            let prior = interpreter.get_obj_result();
            if with_interpreter {
                tcl_runtime_set_current_interp(&mut interpreter);
                interpreter.refuse_host_command("first receiver refusal");
            } else {
                tcl_runtime_set_current_interp(ptr::null_mut());
            }
            let first = interpreter.native_execution_refusal();
            FREES.with(|calls| calls.set(0));
            UPDATES.with(|calls| calls.set(0));
            let value = obj::Owned::fresh(obj::alloc_typed(&ORIGINAL, 0)).into_raw();
            assert_eq!(unsafe { (*value).ref_count }, 1);
            assert_eq!(
                unsafe { consumed(route, value) },
                if with_interpreter {
                    TCL_INVOKE_ABI_HOST_REFUSED
                } else {
                    1
                }
            );
            assert_eq!(FREES.with(Cell::get), 1, "route {route}");
            assert_eq!(UPDATES.with(Cell::get), 0);
            assert_eq!(interpreter.get_obj_result(), prior);
            assert_eq!(interpreter.native_execution_refusal(), first);
            tcl_runtime_set_current_interp(ptr::null_mut());
        }
    }
}
#[test]
fn receiver_write_trace_retains_reached_store_and_stops_guest_publication() {

    // Each route reaches the real trace through the original cell owner.
    for route in 0..4 {
        let mut interpreter = original();
        interpreter.register_builtin(b"change_context", trace);
        tcl_runtime_set_current_interp(&mut interpreter);
        tcl_codegen_frame_push();
        let old = obj::Owned::fresh(new_string_bytes(b"old"));
        assert_eq!(
            unsafe { tcl_codegen_slot_bind(0, b"x".as_ptr(), 1, old.into_raw()) },
            0
        );
        assert_eq!(
            interpreter.eval_str(b"trace add variable x write change_context"),
            Code::Ok
        );
        interpreter.set_result_bytes(b"SEEDED RESULT");
        let prior = interpreter.get_obj_result();
        CALLS.with(|calls| calls.set(0));
        let value = obj::Owned::fresh(new_string_bytes(b"reached\0\xffvalue"));
        let incoming = value.clone().into_raw();
        let status = match route {
            0 => unsafe { tcl_codegen_local_set(0, incoming) },
            1 => unsafe { tcl_codegen_slot_set(0, incoming) },
            2 => unsafe { tcl_codegen_var_set(b"x".as_ptr(), 1, incoming) },
            _ => unsafe { tcl_codegen_local_bind(0, b"x".as_ptr(), 1, incoming) },
        };
        assert_eq!(status, TCL_INVOKE_ABI_HOST_REFUSED);
        assert_eq!(CALLS.with(Cell::get), 1);
        assert_eq!(interpreter.native_execution_refusal(), Some(stale()));
        assert_eq!(interpreter.get_obj_result(), prior);
        assert_eq!(
            unsafe { (*value.as_ptr()).ref_count },
            2,
            "reached cell plus inspection hold"
        );
        assert!(unsafe { tcl_codegen_local_get(0) }.is_null());
        assert_eq!(unsafe { tcl_codegen_var_traced(b"x".as_ptr(), 1) }, 1);
        // Mandatory cleanup releases the reached cell under the first Host.
        tcl_codegen_frame_pop();
        assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 1);
        assert_eq!(interpreter.native_execution_refusal(), Some(stale()));
        tcl_runtime_set_current_interp(ptr::null_mut());
    }
}
#[test]
fn receiver_read_trace_refuses_before_owned_result_or_missing_read_error() {

    for route in 0..3 {
        let mut interpreter = original();
        interpreter.register_builtin(b"change_context", trace);
        tcl_runtime_set_current_interp(&mut interpreter);
        tcl_codegen_frame_push();
        let value = obj::Owned::fresh(new_string_bytes(b"old\0value"));
        assert_eq!(
            unsafe { tcl_codegen_slot_bind(0, b"x".as_ptr(), 1, value.clone().into_raw()) },
            0
        );
        assert_eq!(
            interpreter.eval_str(b"trace add variable x read change_context"),
            Code::Ok
        );
        interpreter.set_result_bytes(b"SEEDED RESULT");
        let prior = interpreter.get_obj_result();
        let references = unsafe { (*value.as_ptr()).ref_count };
        CALLS.with(|calls| calls.set(0));
        let result = match route {
            0 => unsafe { tcl_codegen_local_get(0) },
            1 => unsafe { tcl_codegen_slot_get(0) },
            _ => unsafe { tcl_codegen_var_get(b"x".as_ptr(), 1) },
        };
        assert!(result.is_null());
        assert_eq!(CALLS.with(Cell::get), 1);
        assert_eq!(interpreter.native_execution_refusal(), Some(stale()));
        assert_eq!(interpreter.get_obj_result(), prior);
        assert_eq!(unsafe { (*value.as_ptr()).ref_count }, references);
        tcl_codegen_frame_pop();
        assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 1);
        tcl_runtime_set_current_interp(ptr::null_mut());
    }
}
#[test]
fn neutral_try_getter_retains_original_access_refusal_before_fast_path_output() {

    // Real updater/current issuer before cache adoption; no external primitive claim.
    for wide in [false, true] {
        let mut interpreter = original();
        tcl_runtime_set_current_interp(&mut interpreter);
        interpreter.set_result_bytes(b"SEEDED RESULT");
        let prior = interpreter.get_obj_result();
        let value = obj::Owned::fresh(obj::alloc_typed(&ORIGINAL, 0));
        let references = unsafe { (*value.as_ptr()).ref_count };
        UPDATES.with(|calls| calls.set(0));
        let mut integer = 73;
        let mut double = 73.0;
        let status = if wide {
            unsafe {
                crate::codegen_native::tcl_codegen_value_try_wide_int(value.as_ptr(), &mut integer)
            }
        } else {
            unsafe {
                crate::codegen_native::tcl_codegen_value_try_double(value.as_ptr(), &mut double)
            }
        };
        assert_eq!(status, crate::codegen_native::TCL_VALUE_TRY_NOT_NATIVE);
        assert_eq!(UPDATES.with(Cell::get), 1);
        assert_eq!(integer, 73);
        assert_eq!(double, 73.0);
        assert_eq!(interpreter.get_obj_result(), prior);
        assert_eq!(interpreter.native_execution_refusal(), Some(stale()));
        assert_eq!(obj::bytes_of(value.as_ptr()), b"17");
        assert_eq!(obj::obj_type_ptr(value.as_ptr()), &ORIGINAL as *const _);
        assert_eq!(unsafe { (*value.as_ptr()).ref_count }, references);
        tcl_runtime_set_current_interp(ptr::null_mut());
    }
    let mut interpreter = original();
    tcl_runtime_set_current_interp(&mut interpreter);
    interpreter.set_result_bytes(b"SEEDED RESULT");
    let prior = interpreter.get_obj_result();
    let value = obj::Owned::fresh(new_string_bytes(b"17"));
    let mut integer = 73;
    assert_eq!(
        unsafe {
            crate::codegen_native::tcl_codegen_value_try_wide_int(value.as_ptr(), &mut integer)
        },
        1
    );
    assert_eq!(integer, 17);
    assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 1);
    let invalid = obj::Owned::fresh(new_string_bytes(b"ordinary nonnumeric"));
    assert_eq!(
        unsafe {
            crate::codegen_native::tcl_codegen_value_try_wide_int(invalid.as_ptr(), &mut integer)
        },
        0
    );
    assert_eq!(integer, 17);
    assert_eq!(interpreter.get_obj_result(), prior);
    assert!(interpreter.native_execution_refusal().is_none());
    tcl_runtime_set_current_interp(ptr::null_mut());
}

#[test]
fn unavailable_trace_admission_and_current_invalid_slots_keep_ownership_separate() {

    // An unavailable operation cannot promise an untraced fast path. Actual
    // current invalid-slot refusal still consumes its transferred reference.
    tcl_runtime_set_current_interp(ptr::null_mut());
    assert_eq!(unsafe { tcl_codegen_var_traced(b"x".as_ptr(), 1) }, 1);
    assert_eq!(tcl_codegen_slot_traced(-1), 1);
    for route in 0..4 {
        let mut interpreter = original();
        tcl_runtime_set_current_interp(&mut interpreter);
        assert_eq!(
            unsafe { tcl_codegen_var_traced(b"untraced".as_ptr(), 8) },
            0
        );
        FREES.with(|calls| calls.set(0));
        UPDATES.with(|calls| calls.set(0));
        let value = obj::Owned::fresh(obj::alloc_typed(&ORIGINAL, 0)).into_raw();
        assert_eq!(unsafe { consumed(route, value) }, 1);
        assert_eq!(FREES.with(Cell::get), 1);
        assert_eq!(UPDATES.with(Cell::get), 0);
        assert!(interpreter.native_execution_refusal().is_none());
        tcl_runtime_set_current_interp(ptr::null_mut());
    }
}

#[test]
fn guarded_definition_keeps_the_original_head_getter_and_first_context_refusal() {
    for guarded in [false, true] {
        let mut interpreter = original();
        tcl_runtime_set_current_interp(&mut interpreter);
        UPDATES.with(|calls| calls.set(0));
        let head = obj::Owned::fresh(obj::alloc_typed(&ORIGINAL, 0));
        let name = obj::Owned::fresh(new_string_bytes(b"p"));
        let parameters = obj::Owned::fresh(new_string_bytes(b""));
        let body = obj::Owned::fresh(new_string_bytes(b"return ORIGINAL"));
        let words = [
            head.as_ptr(),
            name.as_ptr(),
            parameters.as_ptr(),
            body.as_ptr(),
        ];
        let expected = tcl_registry::NativeLowering::Definition
            .definition_guard_identity(interpreter.native_invocation_dialect())
            .unwrap();
        let mut out = TclCompletionAbi {
            code: 777,
            result: ptr::null_mut(),
            options: ptr::null_mut(),
        };
        let status = unsafe {
            if guarded {
                tcl_codegen_proc_define_guarded_argv(
                    words.as_ptr(),
                    4,
                    Some(super::tests::stub_declines),
                    expected.value(),
                    &mut out,
                )
            } else {
                tcl_invoke_argv(words.as_ptr(), 4, &mut out)
            }
        };
        assert_eq!(status, TCL_INVOKE_ABI_HOST_REFUSED);
        assert_eq!(UPDATES.with(Cell::get), 1);
        assert_eq!(interpreter.native_execution_refusal(), Some(stale()));
        assert_eq!(out.code, 777);
        assert!(out.result.is_null() && out.options.is_null());
        tcl_runtime_set_current_interp(ptr::null_mut());
    }
}

#[test]
fn definition_candidate_frame_observation_follows_actual_coroutine_handoff() {
    let interpreter = original();
    let frame = crate::interp::native_arguments::OriginalArgumentFrame::capture(&interpreter);
    assert!(frame.check_frame(&interpreter).is_ok());
    let mut coroutine = crate::interp::CoroContext::fresh(crate::namespace::GLOBAL);
    interpreter.swap_coro_ctx(&mut coroutine);
    assert!(frame.check_frame(&interpreter).is_err());
    interpreter.swap_coro_ctx(&mut coroutine);
    assert!(frame.check_frame(&interpreter).is_ok());
}
