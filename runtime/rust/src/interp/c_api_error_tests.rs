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

fn write_byte_error(interpreter: &mut Interp, writer: u8) {
    match writer {
        0 => assert_eq!(interpreter.error(b"FRESH RESULT"), Code::Error),
        1 => assert_eq!(
            interpreter.error_with_code(b"FRESH RESULT", b"FRESH CODE"),
            Code::Error
        ),
        2 => assert_eq!(
            interpreter.raise_with_info(b"FRESH RESULT", b"FRESH INFO", b"FRESH CODE"),
            Code::Error
        ),
        3 => assert_eq!(interpreter.set_error_state(b"FRESH CODE"), Code::Error),
        4 => interpreter.process_return_error(
            Some(b"FRESH INFO"),
            Some(b"FRESH CODE"),
            Some(b"CALL {FRESH STACK}"),
        ),
        _ => unreachable!(),
    }
}

#[test]
fn original_byte_error_authors_retire_displaced_options_outside_the_exception_borrow() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Actual software role retirement; no selected native cleanup schedule claim.
    for writer in 0..5 {
        for info in [false, true] {
            for mode in 0..3 {
                leak_free(|interpreter| {
                    interpreter.set_runtime_version(tcl_dialect::TclVersion::V8_6);
                    interpreter.set_result_bytes(b"SEEDED RESULT");
                    let prior_result = interpreter.get_obj_result();
                    interpreter.set_c_error_code(b"STAMP BEFORE");
                    let (value, calls, observed) = original(interpreter, mode);
                    let memory = obj::NativeObjectLifetime::retain(value);
                    interpreter.retain_native_error_option(info, value);
                    assert_eq!(unsafe { (*value).ref_count }, 1);
                    write_byte_error(interpreter, writer);
                    assert_eq!(calls.get(), 1, "writer={writer}, info={info}, mode={mode}");
                    assert!(!obj::allocation_is_live(memory.as_ptr()));
                    let code = if writer == 0 {
                        b"NONE".as_slice()
                    } else {
                        b"FRESH CODE"
                    };
                    assert_eq!(&*observed.borrow(), code);
                    assert_eq!(interpreter.snapshot_error().code_bytes(), code);
                    if writer <= 2 {
                        assert_eq!(obj::bytes_of(interpreter.get_obj_result()), b"FRESH RESULT");
                    } else {
                        assert_eq!(interpreter.get_obj_result(), prior_result);
                    }
                    let state = interpreter.exc.borrow();
                    assert_eq!(state.code_explicit, writer == 4);
                    assert_eq!(state.already_logged, writer == 2 || writer == 4);
                    assert_eq!(state.native.code.is_some(), mode == 0);
                    assert_eq!(
                        state.native.info.is_some(),
                        mode == 0 && (writer == 2 || writer == 4)
                    );
                    assert_eq!(
                        state.native.legacy_copy,
                        mode == 0 && (writer == 2 || writer == 4)
                    );
                    drop(state);
                    assert_refusal(interpreter, mode);
                    assert_eq!(interpreter.entered_native_operation_count(), 0);
                });
            }
        }
    }
}

#[test]
fn original_byte_error_authors_leave_all_effects_unreached_after_a_pending_host() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for writer in 0..5 {
        leak_free(|interpreter| {
            interpreter.set_runtime_version(tcl_dialect::TclVersion::V8_6);
            interpreter.set_result_bytes(b"SEEDED RESULT");
            interpreter.set_c_error_code(b"STAMP BEFORE");
            let result = interpreter.get_obj_result();
            let (value, calls, _) = original(interpreter, 0);
            interpreter.retain_native_error_option(false, value);
            interpreter.refuse_host_command("first pending refusal");
            let first = interpreter.native_execution_refusal();
            write_byte_error(interpreter, writer);
            assert_eq!(interpreter.get_obj_result(), result);
            assert_eq!(interpreter.snapshot_error().code_bytes(), b"STAMP BEFORE");
            assert_eq!(
                interpreter
                    .exc
                    .borrow()
                    .native
                    .code
                    .as_ref()
                    .unwrap()
                    .as_ptr(),
                value
            );
            assert_eq!(calls.get(), 0);
            assert_eq!(interpreter.native_execution_refusal(), first);
            assert_eq!(interpreter.entered_native_operation_count(), 0);
            let retired = interpreter.exc.take();
            drop(retired);
            assert_eq!(calls.get(), 1);
            assert_eq!(interpreter.native_execution_refusal(), first);
        });
    }
}

struct CaptureRetirement {
    interpreter: WeakInterp,
    events: Rc<RefCell<Vec<(u8, Vec<u8>)>>>,
    role: u8,
    mode: u8,
}

extern "C" fn retire_capture(value: *mut TclObj) {
    // SAFETY: this actual backing owns the single record installed below.
    let state =
        unsafe { Box::from_raw(obj::internal_rep(value) as usize as *mut CaptureRetirement) };
    let mut interpreter = state.interpreter.upgrade().unwrap();
    state.events.borrow_mut().push((
        state.role,
        interpreter.snapshot_error().code_bytes().to_vec(),
    ));
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

static CAPTURE_RETIREMENT: obj::TclObjType = obj::TclObjType {
    name: c"originalCaptureRetirement".as_ptr(),
    free_int_rep_proc: Some(retire_capture),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};

#[test]
fn original_error_capture_checks_each_retired_role_before_the_next_projection() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for fail_role in 0..2 {
        for mode in 0..3 {
            leak_free(|interpreter| {
                interpreter.set_runtime_version(tcl_dialect::TclVersion::V8_6);
                interpreter.set_c_error_code(b"STAMP BEFORE");
                let events = Rc::new(RefCell::new(Vec::new()));
                let mut originals = Vec::new();
                let mut leases = Vec::new();
                for role in 0..2 {
                    let state = Box::new(CaptureRetirement {
                        interpreter: interpreter.downgrade(),
                        events: events.clone(),
                        role,
                        mode: if role == fail_role { mode } else { 0 },
                    });
                    let value =
                        obj::alloc_typed(&CAPTURE_RETIREMENT, Box::into_raw(state) as usize as u64);
                    unsafe { obj::set_string_rep(value, b"OLD") };
                    leases.push(obj::NativeObjectLifetime::retain(value));
                    interpreter.retain_native_error_option(role == 1, value);
                    assert_eq!(unsafe { (*value).ref_count }, 1);
                    originals.push(value);
                }
                interpreter.exc.borrow_mut().code = b"FRESH CODE".to_vec();
                interpreter.exc.borrow_mut().info = Some(b"FRESH INFO".to_vec());
                let scope =
                    native_operation_currency::NativeOperationScope::enter(interpreter).unwrap();
                let captured = interpreter.capture_native_error_objects_checked(scope.currency());
                assert_eq!(captured.is_ok(), mode == 0);
                let stopped_after_code = mode != 0 && fail_role == 0;
                let expected = if stopped_after_code {
                    vec![(0, b"FRESH CODE".to_vec())]
                } else {
                    vec![(0, b"FRESH CODE".to_vec()), (1, b"FRESH CODE".to_vec())]
                };
                assert_eq!(&*events.borrow(), &expected);
                assert!(!obj::allocation_is_live(leases[0].as_ptr()));
                assert_eq!(
                    obj::allocation_is_live(leases[1].as_ptr()),
                    stopped_after_code
                );
                let state = interpreter.exc.borrow();
                assert_ne!(state.native.code.as_ref().unwrap().as_ptr(), originals[0]);
                if stopped_after_code {
                    assert_eq!(state.native.info.as_ref().unwrap().as_ptr(), originals[1]);
                    assert_eq!(state.native.info_len, 3);
                } else {
                    assert_eq!(
                        obj::bytes_of(state.native.info.as_ref().unwrap().as_ptr()),
                        b"FRESH INFO"
                    );
                    assert_eq!(state.native.info_len, b"FRESH INFO".len());
                }
                drop(state);
                assert_refusal(interpreter, mode);
                let retired = interpreter.exc.take();
                drop(retired);
                drop(scope);
                assert_eq!(events.borrow().len(), 2);
                assert_eq!(interpreter.entered_native_operation_count(), 0);
            });
        }
    }
}

#[test]
fn original_return_error_retirement_refuses_before_prepared_result_and_options_publication() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for mode in 1..3 {
        leak_free(|interpreter| {
            interpreter.set_runtime_version(tcl_dialect::TclVersion::V8_6);
            interpreter.set_result_bytes(b"SEEDED RESULT");
            let result = interpreter.get_obj_result();
            let (value, calls, observed) = original(interpreter, mode);
            interpreter.retain_native_error_option(false, value);
            let words = [
                b"-code".as_slice(),
                b"error",
                b"-errorcode",
                b"FRESH CODE",
                b"LATER RESULT",
            ];
            let owners: Vec<_> = words
                .into_iter()
                .map(|word| obj::Owned::fresh(new_string(word)))
                .collect();
            let args: Vec<_> = owners.iter().map(obj::Owned::as_ptr).collect();
            assert_eq!(
                crate::return_options::command(interpreter, &args),
                Code::Error
            );
            assert_eq!(calls.get(), 1);
            assert_eq!(&*observed.borrow(), b"FRESH CODE");
            assert_eq!(interpreter.get_obj_result(), result);
            assert!(interpreter.pending_return_option_objects().is_empty());
            assert_refusal(interpreter, mode);
        });
    }
}
