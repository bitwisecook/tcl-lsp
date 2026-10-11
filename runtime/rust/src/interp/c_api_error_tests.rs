// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual extension retirement at C API error publication boundaries.

use super::*;

fn leak_free(body: impl FnOnce(&mut Interp)) {
    crate::counters::reset();
    {
        let mut interpreter = Interp::new();
        body(&mut interpreter);
    }
    assert_eq!(crate::counters::finalize(), 0);
    assert_eq!(crate::counters::double_free_count(), 0);
}

struct Retirement {
    interpreter: WeakInterp,
    calls: Rc<Cell<usize>>,
    observed: Rc<RefCell<Vec<u8>>>,
    mode: u8,
}

extern "C" fn retire(value: *mut TclObj) {
    // SAFETY: this actual extension backing owns exactly one boxed record.
    let state = unsafe { Box::from_raw(obj::internal_rep(value) as usize as *mut Retirement) };
    state.calls.set(state.calls.get() + 1);
    let mut interpreter = state.interpreter.upgrade().unwrap();
    *state.observed.borrow_mut() = interpreter.snapshot_error().code_bytes().to_vec();
    match state.mode {
        1 => {
            interpreter.refuse_host_command("original error retirement refusal");
        }
        2 => {
            let original = interpreter.runtime_context();
            let mut changed = original.clone();
            changed.packages = vec![("error-retirement".into(), "1.0".into())];
            interpreter.pin_context(&changed).unwrap();
            interpreter.pin_context(&original).unwrap();
        }
        _ => {}
    }
}

static RETIREMENT: obj::TclObjType = obj::TclObjType {
    name: c"originalErrorRetirement".as_ptr(),
    free_int_rep_proc: Some(retire),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};

fn original(
    interpreter: &Interp,
    mode: u8,
) -> (*mut TclObj, Rc<Cell<usize>>, Rc<RefCell<Vec<u8>>>) {
    let calls = Rc::new(Cell::new(0));
    let observed = Rc::new(RefCell::new(Vec::new()));
    let state = Box::new(Retirement {
        interpreter: interpreter.downgrade(),
        calls: calls.clone(),
        observed: observed.clone(),
        mode,
    });
    let value = obj::alloc_typed(&RETIREMENT, Box::into_raw(state) as usize as u64);
    // SAFETY: actual fresh extension object; resident bytes are independently owned.
    unsafe { obj::set_string_rep(value, b"OLD") };
    (value, calls, observed)
}

fn assert_refusal(interpreter: &Interp, mode: u8) {
    match mode {
        0 => assert!(interpreter.native_execution_refusal().is_none()),
        1 => {
            let Some(tcl_runtime_api::NativeExecutionError::HostCommandRefusal(first)) =
                interpreter.native_execution_refusal()
            else {
                panic!("original Host cause was not retained");
            };
            assert_eq!(first.reason, "original error retirement refusal");
        }
        _ => assert_eq!(
            interpreter.native_execution_refusal(),
            Some(tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "stale entered native operation"
                )
            ))
        ),
    }
}

#[test]
fn original_result_retirement_stops_later_exception_and_counter_publication() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Actual software free hooks and ownership, independently of native callback timing.
    for mode in 0..3 {
        leak_free(|interpreter| {
            interpreter.set_runtime_version(tcl_dialect::TclVersion::V8_6);
            interpreter.set_c_error_code(b"STAMP BEFORE");
            let count = interpreter.c_api_errors.get();
            let context = interpreter.runtime_context();
            let (value, calls, observed) = original(interpreter, mode);
            // SAFETY: the result slot retains the original fresh rc0 object.
            unsafe { interpreter.set_obj_result(value) };
            interpreter.c_api_error(b"FRESH RESULT", Some(b"FRESH CODE"));
            assert_eq!(calls.get(), 1);
            assert_eq!(&*observed.borrow(), b"STAMP BEFORE");
            assert_eq!(obj::bytes_of(interpreter.get_obj_result()), b"FRESH RESULT");
            assert_eq!(interpreter.runtime_context(), context);
            let expected = if mode == 0 {
                b"FRESH CODE".as_slice()
            } else {
                b"STAMP BEFORE"
            };
            assert_eq!(interpreter.snapshot_error().code_bytes(), expected);
            assert_eq!(
                interpreter.c_api_errors.get(),
                count + u64::from(mode == 0)
            );
            assert_refusal(interpreter, mode);
        });
    }
}

#[test]
fn original_exception_retirement_observes_reached_state_without_a_live_borrow() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Reentrant snapshot and genuine original native-option ownership; no provider assertion.
    for with_result in [false, true] {
        for mode in 0..3 {
            leak_free(|interpreter| {
                interpreter.set_runtime_version(tcl_dialect::TclVersion::V8_6);
                interpreter.set_result_bytes(b"SEEDED RESULT");
                let prior_result = interpreter.get_obj_result();
                interpreter.set_c_error_code(b"STAMP BEFORE");
                let count = interpreter.c_api_errors.get();
                let (value, calls, observed) = original(interpreter, mode);
                interpreter.retain_native_error_option(false, value);
                assert_eq!(unsafe { (*value).ref_count }, 1);
                if with_result {
                    interpreter.c_api_error(b"FRESH RESULT", Some(b"FRESH CODE"));
                } else {
                    interpreter.set_c_error_code(b"FRESH CODE");
                }
                assert_eq!(calls.get(), 1);
                assert_eq!(&*observed.borrow(), b"FRESH CODE");
                assert_eq!(interpreter.snapshot_error().code_bytes(), b"FRESH CODE");
                assert_eq!(
                    interpreter.c_api_errors.get(),
                    count + u64::from(mode == 0)
                );
                if with_result {
                    assert_eq!(obj::bytes_of(interpreter.get_obj_result()), b"FRESH RESULT");
                } else {
                    assert_eq!(interpreter.get_obj_result(), prior_result);
                }
                assert_refusal(interpreter, mode);
            });
        }
    }
}

#[test]
fn original_pending_host_keeps_result_exception_and_retirement_unreached() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    leak_free(|interpreter| {
        interpreter.set_c_error_code(b"STAMP BEFORE");
        let count = interpreter.c_api_errors.get();
        let (value, calls, _) = original(interpreter, 0);
        unsafe { interpreter.set_obj_result(value) };
        interpreter.refuse_host_command("first pending refusal");
        let first = interpreter.native_execution_refusal();
        interpreter.c_api_error(b"LATER RESULT", Some(b"LATER CODE"));
        interpreter.set_c_error_code(b"LATER CODE");
        assert_eq!(calls.get(), 0);
        assert_eq!(interpreter.get_obj_result(), value);
        assert_eq!(interpreter.snapshot_error().code_bytes(), b"STAMP BEFORE");
        assert_eq!(interpreter.c_api_errors.get(), count);
        assert_eq!(interpreter.native_execution_refusal(), first);
        // Mandatory ownership cleanup remains reached after the refused calls.
        interpreter.set_result_bytes(b"");
        assert_eq!(calls.get(), 1);
    });
}

#[test]
fn original_error_snapshot_restoration_retires_native_options_after_full_state_publication() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Software snapshot ownership and callback transport; no Native cleanup chronology claim.
    for info in [false, true] {
        for mode in 0..3 {
            leak_free(|interpreter| {
                interpreter.set_runtime_version(tcl_dialect::TclVersion::V8_6);
                assert_eq!(
                    interpreter.eval_str(b"error SNAPSHOT_ERROR {} {SNAPSHOT CODE}"),
                    Code::Error
                );
                assert!(interpreter.exc.borrow().already_logged);
                let snapshot = interpreter.snapshot_error();
                let result = interpreter.get_obj_result();
                let scope = native_operation_currency::NativeOperationScope::enter(interpreter)
                    .unwrap();
                let (value, calls, observed) = original(interpreter, mode);
                let memory = obj::NativeObjectLifetime::retain(value);
                interpreter.retain_native_error_option(info, value);
                assert_eq!(unsafe { (*value).ref_count }, 1);
                interpreter.clear_error_logged();
                assert!(!interpreter.exc.borrow().already_logged);
                interpreter.restore_error(snapshot);
                assert_eq!(calls.get(), 1);
                assert!(!obj::allocation_is_live(memory.as_ptr()));
                assert_eq!(&*observed.borrow(), b"SNAPSHOT CODE");
                assert_eq!(interpreter.snapshot_error().code_bytes(), b"SNAPSHOT CODE");
                assert!(interpreter.exc.borrow().already_logged);
                assert_eq!(interpreter.get_obj_result(), result);
                assert_eq!(scope.currency().ensure_current_or_refuse().is_ok(), mode == 0);
                assert_refusal(interpreter, mode);
                drop(scope);
                assert_eq!(interpreter.entered_native_operation_count(), 0);
            });
        }
    }
}
