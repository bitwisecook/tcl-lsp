// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual default +2 retirement, independent of full array Native equivalence.
use super::*;
use crate::obj::Owned;
use std::cell::{Cell, RefCell};
use tcl_runtime_api::NativeExecutionError;

thread_local! {
    static OWNER: RefCell<Option<crate::interp::WeakInterp>> = const {RefCell::new(None)};
    static MODE: Cell<u8> = const {Cell::new(0)};
    static FREES: Cell<usize> = const {Cell::new(0)};
    static OUTSIDE: Cell<bool> = const {Cell::new(false)};
    static CURRENT_DEFAULT: Cell<*mut TclObj> = const {Cell::new(core::ptr::null_mut())};
}
extern "C" fn retire_default(_: *mut TclObj) {
    FREES.with(|calls| calls.set(calls.get() + 1));
    let mut interpreter = OWNER.with(|owner| owner.borrow().as_ref().unwrap().upgrade().unwrap());
    OUTSIDE.with(|outside| outside.set(interpreter.frames.try_borrow_mut().is_ok()));
    // This is a genuine reentrant read of the original newly published default.
    // It also requires the namespace owner borrow to have been released.
    let current = crate::vars::get_elem(
        &interpreter.frames.borrow(),
        &interpreter.namespaces(),
        interpreter.current_ns(),
        b"::array",
        b"absent",
    );
    CURRENT_DEFAULT.with(|slot| slot.set(current.unwrap_or(core::ptr::null_mut())));
    match MODE.with(Cell::get) {
        1 => {
            interpreter.refuse_host_command("first original array default retirement");
        }
        2 => {
            let original = interpreter.runtime_context();
            let mut changed = original.clone();
            changed
                .packages
                .push(("array-default-retirement".into(), "1.0".into()));
            interpreter.pin_context(&changed).unwrap();
            interpreter.pin_context(&original).unwrap();
        }
        _ => {}
    }
}
static DEFAULT: obj::TclObjType = obj::TclObjType {
    name: c"originalArrayDefaultRetirement".as_ptr(),
    free_int_rep_proc: Some(retire_default),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
fn native(version: &str) -> Interp {
    Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect(version),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}
fn reset(interpreter: &Interp, mode: u8) {
    OWNER.with(|owner| *owner.borrow_mut() = Some(interpreter.downgrade()));
    MODE.with(|slot| slot.set(mode));
    FREES.with(|slot| slot.set(0));
    OUTSIDE.with(|slot| slot.set(false));
    CURRENT_DEFAULT.with(|slot| slot.set(core::ptr::null_mut()));
}
fn stored(interpreter: &Interp) -> Option<*mut TclObj> {
    crate::vars::get_elem(
        &interpreter.frames.borrow(),
        &interpreter.namespaces(),
        interpreter.current_ns(),
        b"::array",
        b"absent",
    )
}
fn install(interpreter: &mut Interp, value: *mut TclObj) {
    let frame = Frames::current(interpreter);
    assert!(matches!(
        interpreter
            .set_array_default_bytes(frame, b"::array", value)
            .unwrap(),
        Ok(())
    ));
}
fn cleanup(interpreter: &mut Interp) {
    MODE.with(|mode| mode.set(0));
    interpreter.reset_native_compilation_admission();
    assert_eq!(
        interpreter.eval_str(b"array default unset ::array"),
        crate::interp::Code::Ok
    );
    OWNER.with(|owner| *owner.borrow_mut() = None);
}

#[test]
fn original_array_default_replacement_retires_outside_borrows_after_actual_publication() {

    // Software actual callback/current/refcount control; no whole Native array grant.
    for version in ["tcl9.0", "tcl9.1"] {
        for mode in [0, 1, 2] {
            let mut interpreter = native(version);
            reset(&interpreter, mode);
            let old = Owned::fresh(obj::alloc_typed(&DEFAULT, 0));
            let pointer = old.as_ptr();
            let lifetime = obj::NativeObjectLifetime::retain(pointer);
            install(&mut interpreter, pointer);
            assert_eq!(
                unsafe { (*pointer).ref_count },
                3,
                "caller + actual table two"
            );
            drop(old);
            assert_eq!(unsafe { (*pointer).ref_count }, 2);
            let next = Owned::fresh(obj::new_string_bytes(b"NEXT\0DEFAULT"));
            interpreter.set_result_bytes(b"PRIOR");
            let prior = interpreter.get_obj_result();
            let frame = Frames::current(&interpreter);
            let outcome = interpreter.set_array_default_bytes(frame, b"::array", next.as_ptr());
            assert_eq!(outcome.is_ok(), mode == 0);
            if mode == 0 {
                assert!(matches!(outcome.unwrap(), Ok(())));
            }
            assert_eq!(FREES.with(Cell::get), 1);
            assert!(OUTSIDE.with(Cell::get));
            assert_eq!(CURRENT_DEFAULT.with(Cell::get), next.as_ptr());
            assert_eq!(stored(&interpreter), Some(next.as_ptr()));
            assert_eq!(unsafe { (*next.as_ptr()).ref_count }, 3);
            assert!(obj::check_native_liveness(lifetime.as_ptr()).is_err());
            assert_eq!(interpreter.get_obj_result(), prior);
            if mode == 1 {
                assert!(matches!(interpreter.native_execution_refusal(),
                    Some(NativeExecutionError::HostCommandRefusal(ref first)) if first.reason == "first original array default retirement"));
            }
            if mode == 2 {
                assert!(matches!(
                    interpreter.native_execution_refusal(),
                    Some(NativeExecutionError::ValueAccessRefusal(tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable("stale entered native operation")))
                ));
            }
            assert_eq!(interpreter.entered_native_operation_count(), 0);
            cleanup(&mut interpreter);
            assert_eq!(unsafe { (*next.as_ptr()).ref_count }, 1);
            assert_eq!(FREES.with(Cell::get), 1);
        }
    }
}

#[test]
fn original_array_default_unset_retires_selected_default_before_first_cause_settlement() {

    for mode in [0, 1, 2] {
        let mut interpreter = native("tcl9.1");
        reset(&interpreter, mode);
        let old = Owned::fresh(obj::alloc_typed(&DEFAULT, 0));
        install(&mut interpreter, old.as_ptr());
        drop(old);
        let code = interpreter.eval_str(b"array default unset ::array");
        assert_eq!(
            code,
            if mode == 0 {
                crate::interp::Code::Ok
            } else {
                crate::interp::Code::Error
            }
        );
        assert_eq!(FREES.with(Cell::get), 1);
        assert!(OUTSIDE.with(Cell::get));
        assert!(CURRENT_DEFAULT.with(Cell::get).is_null());
        assert_eq!(stored(&interpreter), None);
        assert_eq!(interpreter.entered_native_operation_count(), 0);
        if mode != 0 {
            assert!(interpreter.native_execution_refusal().is_some());
        }
        cleanup(&mut interpreter);
        assert_eq!(FREES.with(Cell::get), 1);
    }
}

#[test]
fn original_array_default_pending_first_host_precedes_storage_or_retirement() {

    let mut interpreter = native("tcl9.1");
    reset(&interpreter, 0);
    let old = Owned::fresh(obj::alloc_typed(&DEFAULT, 0));
    install(&mut interpreter, old.as_ptr());
    let next = Owned::fresh(obj::new_string_bytes(b"UNREACHED"));
    interpreter.refuse_host_command("prior default refusal");
    let first = interpreter.native_execution_refusal().unwrap();
    let frame = Frames::current(&interpreter);
    assert!(interpreter
        .set_array_default_bytes(frame, b"::array", next.as_ptr())
        .is_err());
    assert_eq!(stored(&interpreter), Some(old.as_ptr()));
    assert_eq!(unsafe { (*old.as_ptr()).ref_count }, 3);
    assert_eq!(unsafe { (*next.as_ptr()).ref_count }, 1);
    assert_eq!(FREES.with(Cell::get), 0);
    assert_eq!(interpreter.native_execution_refusal(), Some(first));
    assert_eq!(interpreter.entered_native_operation_count(), 0);
    interpreter.reset_native_compilation_admission();
    assert_eq!(
        interpreter.eval_str(b"array default unset ::array"),
        crate::interp::Code::Ok
    );
    assert_eq!(unsafe { (*old.as_ptr()).ref_count }, 1);
    drop(old);
    assert_eq!(FREES.with(Cell::get), 1);
    OWNER.with(|owner| *owner.borrow_mut() = None);
}

#[test]
fn original_array_default_unavailable_protocol_precedes_storage_and_retirement() {

    // ArrayDefault is independently selected only for actual C9. No C8/Jim storage grant.
    for version in ["tcl8.4", "tcl8.5", "tcl8.6", "jim"] {
        let mut interpreter = native(version);
        reset(&interpreter, 0);
        let old = Owned::fresh(obj::alloc_typed(&DEFAULT, 0));
        interpreter.set_result_bytes(b"PRIOR");
        let prior = interpreter.get_obj_result();
        let frame = Frames::current(&interpreter);
        let outcome = interpreter.set_array_default_bytes(frame, b"::array", old.as_ptr());
        assert!(matches!(
            outcome,
            Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "array default storage"
            ))
        ));
        assert_eq!(stored(&interpreter), None);
        assert_eq!(unsafe { (*old.as_ptr()).ref_count }, 1);
        assert_eq!(FREES.with(Cell::get), 0);
        assert_eq!(interpreter.get_obj_result(), prior);
        assert!(interpreter.native_execution_refusal().is_none());
        assert_eq!(interpreter.entered_native_operation_count(), 0);
        drop(old);
        assert_eq!(FREES.with(Cell::get), 1);
        assert!(CURRENT_DEFAULT.with(Cell::get).is_null());
        OWNER.with(|owner| *owner.borrow_mut() = None);
    }
}
