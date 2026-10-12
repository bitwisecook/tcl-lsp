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
            assert_eq!(interpreter.c_api_errors.get(), count + u64::from(mode == 0));
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
                assert_eq!(interpreter.c_api_errors.get(), count + u64::from(mode == 0));
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
                let scope =
                    native_operation_currency::NativeOperationScope::enter(interpreter).unwrap();
                let (value, calls, observed) = original(interpreter, mode);
                let memory = obj::NativeObjectLifetime::retain(value);
                interpreter.retain_native_error_option(info, value);
                assert_eq!(unsafe { (*value).ref_count }, 1);
                interpreter.clear_error_logged();
                assert!(!interpreter.exc.borrow().already_logged);
                let _ = interpreter.restore_error(snapshot);
                assert_eq!(calls.get(), 1);
                assert!(!obj::allocation_is_live(memory.as_ptr()));
                assert_eq!(&*observed.borrow(), b"SNAPSHOT CODE");
                assert_eq!(interpreter.snapshot_error().code_bytes(), b"SNAPSHOT CODE");
                assert!(interpreter.exc.borrow().already_logged);
                assert_eq!(interpreter.get_obj_result(), result);
                assert_eq!(
                    scope.currency().ensure_current_or_refuse().is_ok(),
                    mode == 0
                );
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

#[test]
fn original_private_error_field_retirement_keeps_the_published_operand_and_stops_its_getter() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Genuine private +1 roles and software callbacks, not Native cleanup chronology.
    for info in [false, true] {
        for mode in 0..3 {
            leak_free(|interpreter| {
                interpreter.set_runtime_version(tcl_dialect::TclVersion::V8_6);
                interpreter.set_c_error_code(b"STAMP BEFORE");
                let (old, calls, observed) = original(interpreter, mode);
                interpreter.retain_native_error_option(info, old);
                let fresh = obj::Owned::fresh(new_string(b"FRESH"));
                let scope =
                    native_operation_currency::NativeOperationScope::enter(interpreter).unwrap();
                let outcome = interpreter.retain_native_error_option_checked(
                    scope.currency(),
                    info,
                    fresh.as_ptr(),
                );
                assert_eq!(outcome.is_ok(), mode == 0);
                assert_eq!(calls.get(), 1);
                assert_eq!(&*observed.borrow(), b"STAMP BEFORE");
                let state = interpreter.exc.borrow();
                let stored = if info {
                    &state.native.info
                } else {
                    &state.native.code
                };
                assert_eq!(stored.as_ref().unwrap().as_ptr(), fresh.as_ptr());
                assert_eq!(unsafe { (*fresh.as_ptr()).ref_count }, 2);
                if info {
                    assert_eq!(state.native.info_len, if mode == 0 { 5 } else { 3 });
                }
                drop(state);
                assert_refusal(interpreter, mode);
                drop(scope);
            });
        }
    }
}

struct ErrorInfoGetter {
    interpreter: WeakInterp,
    updates: Rc<Cell<usize>>,
    frees: Rc<Cell<usize>>,
    references: Rc<Cell<obj::TclSize>>,
    current_role: Rc<Cell<bool>>,
    mode: u8,
}

extern "C" fn update_error_info(value: *mut TclObj) {
    // SAFETY: the callback borrows the backing installed on this live header.
    let state = unsafe { &*(obj::internal_rep(value) as usize as *const ErrorInfoGetter) };
    state.updates.set(state.updates.get() + 1);
    state.references.set(unsafe { (*value).ref_count });
    let mut interpreter = state.interpreter.upgrade().unwrap();
    state.current_role.set(
        interpreter
            .exc
            .borrow()
            .native
            .info
            .as_ref()
            .map(obj::Owned::as_ptr)
            == Some(value),
    );
    assert_eq!(interpreter.snapshot_error().code_bytes(), b"STAMP BEFORE");
    unsafe { obj::set_string_rep(value, b"BASE") };
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
        3 => interpreter.set_result_bytes(b"REACHED REPLACEMENT"),
        _ => {}
    }
}

extern "C" fn free_error_info(value: *mut TclObj) {
    // SAFETY: the actual primary backing owns this single record.
    let state = unsafe { Box::from_raw(obj::internal_rep(value) as usize as *mut ErrorInfoGetter) };
    state.frees.set(state.frees.get() + 1);
    let interpreter = state.interpreter.upgrade().unwrap();
    // A conversion/free hook may reenter the genuine error-state reader.
    drop(interpreter.snapshot_error());
}

static ERROR_INFO_GETTER: obj::TclObjType = obj::TclObjType {
    name: c"originalErrorInfoGetter".as_ptr(),
    free_int_rep_proc: Some(free_error_info),
    dup_int_rep_proc: None,
    update_string_proc: Some(update_error_info),
    set_from_any_proc: None,
};

fn lazy_error_info(
    interpreter: &Interp,
    mode: u8,
) -> (
    *mut TclObj,
    Rc<Cell<usize>>,
    Rc<Cell<usize>>,
    Rc<Cell<obj::TclSize>>,
    Rc<Cell<bool>>,
) {
    let updates = Rc::new(Cell::new(0));
    let frees = Rc::new(Cell::new(0));
    let references = Rc::new(Cell::new(0));
    let current_role = Rc::new(Cell::new(false));
    let state = Box::new(ErrorInfoGetter {
        interpreter: interpreter.downgrade(),
        updates: updates.clone(),
        frees: frees.clone(),
        references: references.clone(),
        current_role: current_role.clone(),
        mode,
    });
    let value = obj::alloc_typed(&ERROR_INFO_GETTER, Box::into_raw(state) as usize as u64);
    (value, updates, frees, references, current_role)
}

#[test]
fn original_private_error_info_getter_keeps_its_real_role_without_an_exception_borrow() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for mode in 0..3 {
        leak_free(|interpreter| {
            interpreter.set_runtime_version(tcl_dialect::TclVersion::V8_6);
            interpreter.set_c_error_code(b"STAMP BEFORE");
            interpreter.exc.borrow_mut().native.info_len = 17;
            let (value, updates, frees, references, current_role) =
                lazy_error_info(interpreter, mode);
            let memory = obj::NativeObjectLifetime::retain(value);
            let scope =
                native_operation_currency::NativeOperationScope::enter(interpreter).unwrap();
            let outcome =
                interpreter.retain_native_error_option_checked(scope.currency(), true, value);
            assert_eq!(outcome.is_ok(), mode == 0);
            assert_eq!(updates.get(), 1);
            assert_eq!(references.get(), 1);
            assert!(current_role.get());
            assert_eq!(frees.get(), 0);
            assert_eq!(
                interpreter
                    .exc
                    .borrow()
                    .native
                    .info
                    .as_ref()
                    .unwrap()
                    .as_ptr(),
                value
            );
            assert_eq!(
                interpreter.exc.borrow().native.info_len,
                if mode == 0 { 4 } else { 17 }
            );
            assert_eq!(obj::bytes_of(value), b"BASE");
            assert_refusal(interpreter, mode);
            let retired = interpreter.exc.borrow_mut().native.info.take();
            drop(retired);
            assert_eq!(frees.get(), 1);
            assert!(!obj::allocation_is_live(memory.as_ptr()));
            drop(scope);
        });
    }
}

#[test]
fn original_error_info_append_checks_the_reached_getter_before_later_cache_publication() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Actual original storage and shared physical kernel; no external opcode equivalence.
    for mode in 0..3 {
        leak_free(|interpreter| {
            interpreter.set_runtime_version(tcl_dialect::TclVersion::V8_6);
            interpreter.set_c_error_code(b"STAMP BEFORE");
            let (value, updates, frees, references, current_role) =
                lazy_error_info(interpreter, mode);
            let memory = obj::NativeObjectLifetime::retain(value);
            {
                let mut state = interpreter.exc.borrow_mut();
                state.native.info = Some(obj::Owned::fresh(value));
                state.native.info_len = 4;
                state.info = Some(b"BASE+".to_vec());
            }
            assert_eq!(unsafe { (*value).ref_count }, 1);
            let scope =
                native_operation_currency::NativeOperationScope::enter(interpreter).unwrap();
            let outcome = interpreter.update_native_error_info_checked(scope.currency());
            assert_eq!(outcome.is_ok(), mode == 0);
            assert_eq!(updates.get(), 1);
            assert_eq!(references.get(), 2);
            assert!(current_role.get());
            assert_eq!(
                interpreter
                    .exc
                    .borrow()
                    .native
                    .info
                    .as_ref()
                    .unwrap()
                    .as_ptr(),
                value
            );
            assert_eq!(unsafe { (*value).ref_count }, 1);
            assert_eq!(
                obj::bytes_of(value),
                if mode == 0 {
                    b"BASE+".as_slice()
                } else {
                    b"BASE"
                }
            );
            assert_eq!(frees.get(), usize::from(mode == 0));
            assert_eq!(
                interpreter.exc.borrow().native.info_len,
                if mode == 0 { 5 } else { 4 }
            );
            assert_eq!(interpreter.exc.borrow().native.legacy_copy, mode == 0);
            assert_refusal(interpreter, mode);
            let retired = interpreter.exc.borrow_mut().native.info.take();
            drop(retired);
            assert_eq!(frees.get(), 1);
            assert!(!obj::allocation_is_live(memory.as_ptr()));
            drop(scope);
        });
    }
}

#[test]
fn original_portable_error_reporting_stops_after_private_info_retirement() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for mode in 0..3 {
        leak_free(|interpreter| {
            interpreter.set_runtime_version(tcl_dialect::TclVersion::V8_6);
            interpreter.set_c_error_code(b"STAMP BEFORE");
            interpreter.error_line.set(11);
            let (value, calls, observed) = original(interpreter, mode);
            interpreter.retain_native_error_option(true, value);
            let mut details =
                tcl_cmd_core::CmdError::new_bytes(b"FRESH RESULT".to_vec()).into_byte_details();
            details.error_code = tcl_cmd_core::CmdErrorCodeUpdate::Set(b"FRESH CODE".to_vec());
            details.error_info = Some(b"FRESH INFO".to_vec());
            details.error_line = Some(23);
            assert_eq!(
                interpreter.report_cmd_error(tcl_cmd_core::CmdError::from_byte_details(details)),
                Code::Error
            );
            assert_eq!(calls.get(), 1);
            assert_eq!(&*observed.borrow(), b"FRESH CODE");
            assert_eq!(obj::bytes_of(interpreter.get_obj_result()), b"FRESH RESULT");
            assert_eq!(
                interpreter.exc.borrow().info.as_deref(),
                Some(b"FRESH INFO".as_slice())
            );
            assert_eq!(
                obj::bytes_of(
                    interpreter
                        .exc
                        .borrow()
                        .native
                        .info
                        .as_ref()
                        .unwrap()
                        .as_ptr()
                ),
                b"FRESH INFO"
            );
            assert_eq!(
                interpreter.error_line.get(),
                if mode == 0 { 23 } else { 11 }
            );
            assert_refusal(interpreter, mode);
        });
    }
}

#[test]
fn original_error_info_fallback_keeps_the_exact_result_across_its_getter() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // The actual callback changes the result without changing context or Host.
    leak_free(|interpreter| {
        interpreter.set_runtime_version(tcl_dialect::TclVersion::V8_6);
        interpreter.set_c_error_code(b"STAMP BEFORE");
        let (pointer, updates, frees, references, current_role) = lazy_error_info(interpreter, 3);
        let value = obj::Owned::fresh(pointer);
        interpreter.set_result(value.as_ptr());
        assert_eq!(unsafe { (*pointer).ref_count }, 2);
        assert!(interpreter.exc.borrow().info.is_none());
        assert!(interpreter.exc.borrow().native.info.is_none());
        let scope = native_operation_currency::NativeOperationScope::enter(interpreter).unwrap();
        let reached = interpreter.update_native_error_info_checked(scope.currency());
        let expected = tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
            tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                "original error-info result replaced during getter",
            ),
        );
        assert_eq!(reached, Err(expected.clone()));
        assert_eq!(interpreter.native_execution_refusal(), Some(expected));
        assert_eq!(updates.get(), 1);
        assert_eq!(references.get(), 2);
        assert!(!current_role.get());
        assert_eq!(frees.get(), 0);
        assert_eq!(unsafe { (*pointer).ref_count }, 1);
        assert_eq!(obj::bytes_of(pointer), b"BASE");
        assert_eq!(
            obj::bytes_of(interpreter.get_obj_result()),
            b"REACHED REPLACEMENT"
        );
        assert!(interpreter.exc.borrow().info.is_none());
        assert!(interpreter.exc.borrow().native.info.is_none());
        assert!(!interpreter.exc.borrow().native.legacy_copy);
        drop(value);
        assert_eq!(frees.get(), 1);
        drop(scope);
    });
}
type SavedCleanupOutcome = Rc<RefCell<Option<Result<(), tcl_runtime_api::NativeExecutionError>>>>;

#[derive(Clone)]
struct SavedCleanupObservation {
    calls: Rc<Cell<usize>>,
    entered: Rc<Cell<usize>>,
    observed: Rc<RefCell<Vec<u8>>>,
    nested: SavedCleanupOutcome,
    engine: SavedCleanupOutcome,
}

struct SavedCleanupRetirement {
    interpreter: WeakInterp,
    observation: SavedCleanupObservation,
    mode: u8,
}

extern "C" fn retire_saved_cleanup(value: *mut TclObj) {
    // SAFETY: the original extension owns exactly this boxed observer.
    let state =
        unsafe { Box::from_raw(obj::internal_rep(value) as usize as *mut SavedCleanupRetirement) };
    state
        .observation
        .calls
        .set(state.observation.calls.get() + 1);
    let mut interpreter = state.interpreter.upgrade().unwrap();
    state
        .observation
        .entered
        .set(interpreter.entered_native_operation_count());
    *state.observation.observed.borrow_mut() = interpreter.snapshot_error().code_bytes().to_vec();
    match state.mode {
        1 => {
            interpreter.refuse_host_command("saved cleanup original first cause");
        }
        2 => {
            let original = interpreter.runtime_context();
            let mut changed = original.clone();
            changed.packages = vec![("saved-cleanup".into(), "1.0".into())];
            interpreter.pin_context(&changed).unwrap();
            interpreter.pin_context(&original).unwrap();
        }
        _ => {}
    }
    // These real nested ingress attempts happen after any reached retirement
    // effect. Neither cleanup nor an engine callback may reset its first cause.
    *state.observation.nested.borrow_mut() = Some(
        native_operation_currency::NativeOperationScope::enter(&interpreter)
            .and_then(|scope| scope.currency().ensure_current_or_refuse()),
    );
    *state.observation.engine.borrow_mut() = Some(
        native_operation_currency::NativeOperationScope::enter_engine_evaluation(&interpreter)
            .and_then(|scope| scope.currency().ensure_current_or_refuse()),
    );
}

static SAVED_CLEANUP_RETIREMENT: obj::TclObjType = obj::TclObjType {
    name: c"originalSavedCleanupRetirement".as_ptr(),
    free_int_rep_proc: Some(retire_saved_cleanup),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};

fn install_saved_cleanup_retirement(
    interpreter: &Interp,
    mode: u8,
    checked: bool,
) -> SavedCleanupObservation {
    let observation = SavedCleanupObservation {
        calls: Rc::new(Cell::new(0)),
        entered: Rc::new(Cell::new(0)),
        observed: Rc::new(RefCell::new(Vec::new())),
        nested: Rc::new(RefCell::new(None)),
        engine: Rc::new(RefCell::new(None)),
    };
    let state = Box::new(SavedCleanupRetirement {
        interpreter: interpreter.downgrade(),
        observation: observation.clone(),
        mode,
    });
    let value = obj::alloc_typed(
        &SAVED_CLEANUP_RETIREMENT,
        Box::into_raw(state) as usize as u64,
    );
    // SAFETY: live fresh original extension; this storage role owns its sole +1.
    unsafe { obj::set_string_rep(value, b"DISPLACED") };
    if checked {
        interpreter.retain_native_error_option(false, value);
    } else {
        // Model already-reached storage awaiting mandatory settlement. This
        // fixture bypasses ingress without claiming a new checked operation.
        let retired = interpreter
            .exc
            .borrow_mut()
            .native
            .code
            .replace(obj::Owned::retain(value));
        drop(retired);
    }
    assert_eq!(unsafe { (*value).ref_count }, 1);
    observation
}

enum SavedErrorBoundary {
    Error(ErrorSnapshot),
    Constructor(ErrorStateSnapshot),
    Trace(native_error_variables::NativeErrorTraceState),
    Result(native_trace_result::NativeTraceResultState),
}

impl SavedErrorBoundary {
    fn capture(interpreter: &mut Interp, boundary: u8) -> Self {
        match boundary {
            0 => Self::Error(interpreter.snapshot_error()),
            1 => Self::Constructor(interpreter.error_snapshot()),
            2 => Self::Trace(interpreter.save_native_error_trace_state().unwrap()),
            _ => Self::Result(
                interpreter
                    .save_native_variable_trace_result(false)
                    .unwrap()
                    .unwrap(),
            ),
        }
    }

    fn restore(
        self,
        interpreter: &mut Interp,
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        match self {
            Self::Error(saved) => interpreter.restore_error(saved),
            Self::Constructor(saved) => interpreter.error_restore(saved),
            Self::Trace(saved) => interpreter.restore_native_error_trace_state(saved),
            Self::Result(saved) => interpreter.restore_native_variable_trace_result(saved),
        }
    }
}

#[test]
fn original_saved_error_cleanup_attaches_the_expired_entry_before_real_retirement() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Real software saved-state/retirement transport, not a native callback schedule.
    for boundary in 0..4 {
        for mode in 0..3 {
            leak_free(|interpreter| {
                interpreter.set_runtime_version(tcl_dialect::TclVersion::V8_6);
                interpreter.set_result_bytes(b"SAVED RESULT");
                interpreter.set_c_error_code(b"SAVED CODE");
                let entry =
                    native_operation_currency::NativeOperationScope::enter(interpreter).unwrap();
                let saved = SavedErrorBoundary::capture(interpreter, boundary);
                drop(entry);
                assert_eq!(interpreter.entered_native_operation_count(), 0);
                let SavedCleanupObservation {
                    calls,
                    entered,
                    observed,
                    nested,
                    engine,
                } = install_saved_cleanup_retirement(interpreter, mode, true);
                interpreter.exc.borrow_mut().code = b"DISPLACED CODE".to_vec();
                let restored = saved.restore(interpreter);
                assert_eq!(calls.get(), 1);
                assert!(entered.get() >= 1);
                assert_eq!(&*observed.borrow(), b"SAVED CODE");
                assert_eq!(interpreter.entered_native_operation_count(), 0);
                assert_eq!(interpreter.snapshot_error().code_bytes(), b"SAVED CODE");
                let first = interpreter.native_execution_refusal();
                assert_eq!(restored, first.clone().map_or(Ok(()), Err));
                assert_eq!(*nested.borrow(), Some(first.clone().map_or(Ok(()), Err)));
                assert_eq!(*engine.borrow(), Some(first.clone().map_or(Ok(()), Err)));
                if mode == 0 {
                    assert!(first.is_none());
                }
                if mode == 1 {
                    let Some(tcl_runtime_api::NativeExecutionError::HostCommandRefusal(first)) =
                        first
                    else {
                        panic!("original saved cleanup Host cause missing");
                    };
                    assert_eq!(first.reason, "saved cleanup original first cause");
                }
                if mode == 2 {
                    assert_refusal(interpreter, 2);
                }
            });
        }
    }
}

#[test]
fn original_saved_error_cleanup_preserves_withdrawn_same_target_and_failed_capture() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for failed_capture in [false, true] {
        leak_free(|interpreter| {
            interpreter.set_runtime_version(tcl_dialect::TclVersion::V8_6);
            interpreter.set_c_error_code(b"SAVED CODE");
            let entry =
                native_operation_currency::NativeOperationScope::enter(interpreter).unwrap();
            if failed_capture {
                interpreter.refuse_host_command("saved cleanup original first cause");
            }
            let saved = interpreter.snapshot_error();
            drop(entry);
            if !failed_capture {
                let original = interpreter.runtime_context();
                let mut changed = original.clone();
                changed.packages = vec![("saved-cleanup".into(), "1.0".into())];
                interpreter.pin_context(&changed).unwrap();
                interpreter.pin_context(&original).unwrap();
            }
            let SavedCleanupObservation {
                calls,
                entered,
                observed,
                nested,
                engine,
            } = install_saved_cleanup_retirement(interpreter, 0, false);
            interpreter.exc.borrow_mut().code = b"DISPLACED CODE".to_vec();
            let reached = interpreter.restore_error(saved);
            let first = interpreter.native_execution_refusal().unwrap();
            assert_eq!(reached, Err(first.clone()));
            assert!(entered.get() >= 1);
            assert_eq!(*nested.borrow(), Some(Err(first.clone())));
            assert_eq!(*engine.borrow(), Some(Err(first.clone())));
            assert_eq!(calls.get(), 1);
            assert_eq!(&*observed.borrow(), b"SAVED CODE");
            assert_eq!(interpreter.snapshot_error().code_bytes(), b"SAVED CODE");
            assert_eq!(interpreter.entered_native_operation_count(), 0);
            if failed_capture {
                let tcl_runtime_api::NativeExecutionError::HostCommandRefusal(first) = first else {
                    panic!("failed saved capture lost first cause");
                };
                assert_eq!(first.reason, "saved cleanup original first cause");
            } else {
                assert_refusal(interpreter, 2);
            }
        });
    }
}

#[test]
fn original_saved_error_cleanup_refuses_foreign_target_before_any_state_install() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    crate::counters::reset();
    {
        let mut source = Interp::new();
        let mut foreign = Interp::new();
        source.set_runtime_version(tcl_dialect::TclVersion::V8_6);
        foreign.set_runtime_version(tcl_dialect::TclVersion::V8_6);
        source.set_c_error_code(b"ORIGINAL CODE");
        foreign.set_c_error_code(b"FOREIGN CODE");
        foreign.set_result_bytes(b"FOREIGN RESULT");
        let (foreign_value, foreign_calls, foreign_observed) = original(&foreign, 0);
        foreign.retain_native_error_option(false, foreign_value);
        assert_eq!(unsafe { (*foreign_value).ref_count }, 1);
        for boundary in 0..4 {
            let saved = SavedErrorBoundary::capture(&mut source, boundary);
            let reached = saved.restore(&mut foreign);
            let expected = tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "saved native operation belongs to another interpreter",
                ),
            );
            assert_eq!(reached, Err(expected.clone()));
            assert_eq!(foreign.native_execution_refusal(), Some(expected));
            assert_eq!(foreign_calls.get(), 0);
            assert_eq!(
                foreign.exc.borrow().native.code.as_ref().unwrap().as_ptr(),
                foreign_value
            );
            assert_eq!(unsafe { (*foreign_value).ref_count }, 1);
            assert_eq!(foreign.snapshot_error().code_bytes(), b"FOREIGN CODE");
            assert_eq!(obj::bytes_of(foreign.result_obj()), b"FOREIGN RESULT");
            assert_eq!(source.snapshot_error().code_bytes(), b"ORIGINAL CODE");
            assert_eq!(foreign.entered_native_operation_count(), 0);
            assert_eq!(source.entered_native_operation_count(), 0);
        }
        let retired = foreign.exc.borrow_mut().native.code.take();
        drop(retired);
        assert_eq!(foreign_calls.get(), 1);
        assert_eq!(&*foreign_observed.borrow(), b"FOREIGN CODE");
    }
    assert_eq!(crate::counters::finalize(), 0);
    assert_eq!(crate::counters::double_free_count(), 0);
}

#[test]
fn original_jim_trace_replacement_retires_outside_borrow_before_checked_readers() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Actual software callbacks/roles, without a Jim callback chronology claim.
    for mode in 0..3 {
        leak_free(|interpreter| {
            interpreter.set_dialect_profile(
                tcl_registry::model::ingress::resolve_environment("jim").unit_profile(),
            );
            interpreter.set_result_bytes(b"PRIOR RESULT");
            interpreter.exc.borrow_mut().code = b"STAMP BEFORE".to_vec();
            let (old, calls, observed) = original(interpreter, mode);
            interpreter
                .adopt_jim_stacktrace(obj::Owned::fresh(old))
                .unwrap();
            assert_eq!(unsafe { (*old).ref_count }, 1);
            let fresh = obj::Owned::fresh(new_string(b"REPLACEMENT TRACE"));
            let pointer = fresh.as_ptr();
            let prior = interpreter.result_obj();
            let scope =
                native_operation_currency::NativeOperationScope::enter(interpreter).unwrap();
            let reached = interpreter.adopt_jim_stacktrace(fresh);
            assert_eq!(calls.get(), 1);
            assert_eq!(&*observed.borrow(), b"STAMP BEFORE");
            assert_eq!(unsafe { (*pointer).ref_count }, 1);
            assert_eq!(interpreter.result_obj(), prior);
            let stored = {
                let stack = interpreter.jim_error_stack.borrow();
                let Some(JimErrorTrace::Explicit(value)) = stack.trace() else {
                    panic!("actual published Jim trace missing");
                };
                value.as_ptr()
            };
            assert_eq!(stored, pointer);
            if mode == 0 {
                assert_eq!(reached, Ok(()));
                let queried = interpreter.jim_stacktrace_object().unwrap();
                assert_eq!(queried.as_ptr(), pointer);
                assert_eq!(unsafe { (*pointer).ref_count }, 2);
                drop(queried);
                assert_eq!(interpreter.jim_stacktrace().unwrap(), b"REPLACEMENT TRACE");
            } else {
                let first = interpreter.native_execution_refusal().unwrap();
                assert_eq!(reached, Err(first.clone()));
                assert!(
                    matches!(interpreter.jim_stacktrace_object(), Err(error) if error == first)
                );
                assert_eq!(interpreter.jim_stacktrace(), Err(first));
                assert_eq!(unsafe { (*pointer).ref_count }, 1);
                assert_refusal(interpreter, mode);
            }
            drop(scope);
            assert_eq!(interpreter.entered_native_operation_count(), 0);
        });
    }
}

struct JimCaptureMutation {
    interpreter: WeakInterp,
    calls: Rc<Cell<usize>>,
    mode: u8,
}

extern "C" fn mutate_jim_capture_episode(value: *mut TclObj) {
    // SAFETY: one actual extension allocation owns this observer.
    let state =
        unsafe { Box::from_raw(obj::internal_rep(value) as usize as *mut JimCaptureMutation) };
    let interpreter = state.interpreter.upgrade().unwrap();
    state.calls.set(state.calls.get() + 1);
    match state.mode {
        0 => interpreter.jim_error_stack.borrow_mut().mark_reset(),
        1 => {
            let value = obj::Owned::fresh(new_string(b"REENTRANT ADOPT"));
            let retired = interpreter
                .jim_error_stack
                .borrow_mut()
                .replace_explicit(value);
            drop(retired);
        }
        _ => {
            let ticket = interpreter
                .jim_error_stack
                .borrow()
                .capture_ticket()
                .unwrap();
            let value = obj::Owned::fresh(new_string(b"REENTRANT CAPTURE"));
            let retired = interpreter
                .jim_error_stack
                .borrow_mut()
                .publish_capture(&ticket, JimErrorTrace::Explicit(value));
            drop(retired);
        }
    }
}

static JIM_CAPTURE_MUTATION: obj::TclObjType = obj::TclObjType {
    name: c"originalJimCaptureMutation".as_ptr(),
    free_int_rep_proc: Some(mutate_jim_capture_episode),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};

#[test]
fn original_jim_capture_episode_refuses_reentrant_reset_adopt_and_capture() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // A reached original destructor changes this real cache during preparation.
    for mode in 0..3 {
        leak_free(|interpreter| {
            interpreter.set_dialect_profile(
                tcl_registry::model::ingress::resolve_environment("jim").unit_profile(),
            );
            interpreter
                .adopt_jim_stacktrace(obj::Owned::fresh(new_string(b"BEFORE")))
                .unwrap();
            interpreter.reset_jim_error_capture();
            let ticket = interpreter
                .jim_error_stack
                .borrow()
                .capture_ticket()
                .unwrap();
            let scope =
                native_operation_currency::NativeOperationScope::enter(interpreter).unwrap();
            let calls = Rc::new(Cell::new(0));
            let state = Box::new(JimCaptureMutation {
                interpreter: interpreter.downgrade(),
                calls: calls.clone(),
                mode,
            });
            let temporary = obj::Owned::fresh(obj::alloc_typed(
                &JIM_CAPTURE_MUTATION,
                Box::into_raw(state) as usize as u64,
            ));
            let prepared = obj::Owned::fresh(new_string(b"STALE PREPARED"));
            let prepared_pointer = prepared.as_ptr();
            drop(temporary);
            assert_eq!(calls.get(), 1);
            scope.currency().ensure_current_or_refuse().unwrap();
            let declined = interpreter
                .jim_error_stack
                .borrow_mut()
                .publish_capture(&ticket, JimErrorTrace::Explicit(prepared));
            let Err(JimErrorTrace::Explicit(original)) = declined else {
                panic!("old capture episode overwrote reached mutation");
            };
            assert_eq!(original.as_ptr(), prepared_pointer);
            assert_eq!(unsafe { (*prepared_pointer).ref_count }, 1);
            drop(original);
            let expected = [
                b"BEFORE".as_slice(),
                b"REENTRANT ADOPT",
                b"REENTRANT CAPTURE",
            ][usize::from(mode)];
            assert_eq!(interpreter.jim_stacktrace().unwrap(), expected);
            if mode == 0 {
                let current = interpreter
                    .jim_error_stack
                    .borrow()
                    .capture_ticket()
                    .unwrap();
                let fresh = obj::Owned::fresh(new_string(b"CURRENT CAPTURE"));
                let retired = interpreter
                    .jim_error_stack
                    .borrow_mut()
                    .publish_capture(&current, JimErrorTrace::Explicit(fresh));
                assert!(retired.is_ok());
                drop(retired);
                assert_eq!(interpreter.jim_stacktrace().unwrap(), b"CURRENT CAPTURE");
            } else {
                assert!(interpreter
                    .jim_error_stack
                    .borrow()
                    .capture_ticket()
                    .is_none());
            }
            assert!(interpreter.native_execution_refusal().is_none());
            drop(scope);
        });
    }
}

#[test]
fn original_jim_consumed_trace_cleanup_keeps_the_failed_entry_during_destructor_reentry() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Actual software consumed-owner cleanup, without a Jim callback-order claim.
    leak_free(|interpreter| {
        interpreter.set_dialect_profile(
            tcl_registry::model::ingress::resolve_environment("jim").unit_profile(),
        );
        interpreter.set_result_bytes(b"PRIOR RESULT");
        interpreter.exc.borrow_mut().code = b"STAMP BEFORE".to_vec();
        interpreter
            .adopt_jim_stacktrace(obj::Owned::fresh(new_string(b"EXISTING TRACE")))
            .unwrap();
        let existing = {
            let stack = interpreter.jim_error_stack.borrow();
            let Some(JimErrorTrace::Explicit(value)) = stack.trace() else {
                panic!("existing original trace missing");
            };
            value.as_ptr()
        };
        let observation = SavedCleanupObservation {
            calls: Rc::new(Cell::new(0)),
            entered: Rc::new(Cell::new(0)),
            observed: Rc::new(RefCell::new(Vec::new())),
            nested: Rc::new(RefCell::new(None)),
            engine: Rc::new(RefCell::new(None)),
        };
        let state = Box::new(SavedCleanupRetirement {
            interpreter: interpreter.downgrade(),
            observation: observation.clone(),
            mode: 0,
        });
        let value = obj::Owned::fresh(obj::alloc_typed(
            &SAVED_CLEANUP_RETIREMENT,
            Box::into_raw(state) as usize as u64,
        ));
        let memory = obj::NativeObjectLifetime::retain(value.as_ptr());
        assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 1);
        interpreter.refuse_host_command("original consumed trace refusal");
        let first = interpreter.native_execution_refusal().unwrap();
        assert_eq!(interpreter.adopt_jim_stacktrace(value), Err(first.clone()));
        assert_eq!(observation.calls.get(), 1);
        assert!(observation.entered.get() >= 1);
        assert_eq!(&*observation.observed.borrow(), b"STAMP BEFORE");
        assert_eq!(*observation.nested.borrow(), Some(Err(first.clone())));
        assert_eq!(*observation.engine.borrow(), Some(Err(first.clone())));
        assert!(!obj::allocation_is_live(memory.as_ptr()));
        let stack = interpreter.jim_error_stack.borrow();
        let Some(JimErrorTrace::Explicit(value)) = stack.trace() else {
            panic!("refusal changed stored trace");
        };
        assert_eq!(value.as_ptr(), existing);
        assert_eq!(unsafe { (*existing).ref_count }, 1);
        drop(stack);
        assert_eq!(obj::bytes_of(interpreter.result_obj()), b"PRIOR RESULT");
        assert_eq!(interpreter.native_execution_refusal(), Some(first));
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    });
}

#[test]
fn original_result_reset_retires_complete_exception_before_later_return_state() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Real software reset/destructor stages, without provider chronology inference.
    for mode in 0..3 {
        leak_free(|interpreter| {
            interpreter.set_runtime_version(tcl_dialect::TclVersion::V8_6);
            interpreter.set_result_bytes(b"PRIOR RESULT");
            interpreter.set_return_state(7, Code::Break);
            let (value, calls, observed) = original(interpreter, mode);
            let memory = obj::NativeObjectLifetime::retain(value);
            interpreter.retain_native_error_option(false, value);
            assert_eq!(unsafe { (*value).ref_count }, 1);
            interpreter.reset_result();
            assert_eq!(calls.get(), 1);
            assert_eq!(&*observed.borrow(), b"");
            assert!(!obj::allocation_is_live(memory.as_ptr()));
            assert_eq!(obj::bytes_of(interpreter.result_obj()), b"");
            assert!(interpreter.exc.borrow().native.code.is_none());
            assert!(interpreter.exc.borrow().native.info.is_none());
            assert_eq!(
                interpreter.return_level.get(),
                if mode == 0 { 1 } else { 7 }
            );
            assert_eq!(
                interpreter.return_code.get(),
                if mode == 0 { Code::Ok } else { Code::Break }
            );
            assert_refusal(interpreter, mode);
            assert_eq!(interpreter.entered_native_operation_count(), 0);
        });
    }
}

#[test]
fn original_private_reset_checks_each_real_retirement_before_the_next_field() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Private roles are released in the selected reset order even without a setter copy.
    for fail_role in 0..2 {
        for mode in 0..3 {
            leak_free(|interpreter| {
                interpreter.set_runtime_version(tcl_dialect::TclVersion::V8_6);
                interpreter.set_result_bytes(b"PRIOR RESULT");
                let mut records = Vec::new();
                for role in 0..2 {
                    let (value, calls, observed) =
                        original(interpreter, if role == fail_role { mode } else { 0 });
                    let memory = obj::NativeObjectLifetime::retain(value);
                    interpreter.retain_native_error_option(role == 1, value);
                    assert_eq!(unsafe { (*value).ref_count }, 1);
                    records.push((memory, calls, observed));
                }
                assert!(!interpreter.exc.borrow().native.legacy_copy);
                let scope =
                    native_operation_currency::NativeOperationScope::enter(interpreter).unwrap();
                let reached = interpreter.publish_native_error_objects_checked(scope.currency());
                assert_eq!(reached.is_ok(), mode == 0);
                let stopped_after_code = mode != 0 && fail_role == 0;
                assert_eq!(records[0].1.get(), 1);
                assert!(!obj::allocation_is_live(records[0].0.as_ptr()));
                assert_eq!(records[1].1.get(), usize::from(!stopped_after_code));
                assert_eq!(
                    obj::allocation_is_live(records[1].0.as_ptr()),
                    stopped_after_code
                );
                assert!(interpreter.exc.borrow().native.code.is_none());
                assert_eq!(
                    interpreter.exc.borrow().native.info.is_some(),
                    stopped_after_code
                );
                assert_eq!(obj::bytes_of(interpreter.result_obj()), b"PRIOR RESULT");
                assert_refusal(interpreter, mode);
                // Remaining actual owned state still receives mandatory teardown,
                // independently of whether another reset stage was licensed.
                let retired = interpreter.exc.take();
                drop(retired);
                assert_eq!(records[1].1.get(), 1);
                drop(scope);
                assert_eq!(interpreter.entered_native_operation_count(), 0);
            });
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct OriginalResetRow {
    variable: tcl_registry::special_vars::NativeErrorStorageVariable,
    code: Option<*mut TclObj>,
    info: Option<*mut TclObj>,
    refs: obj::TclSize,
}

struct OriginalResetHeader {
    interpreter: WeakInterp,
    variable: tcl_registry::special_vars::NativeErrorStorageVariable,
    rows: Rc<RefCell<Vec<OriginalResetRow>>>,
    mode: u8,
}

extern "C" fn original_reset_header_free(value: *mut TclObj) {
    // SAFETY: this actual free worker consumes its boxed original record.
    let state =
        unsafe { Box::from_raw(obj::internal_rep(value) as usize as *mut OriginalResetHeader) };
    let Some(mut interpreter) = state.interpreter.upgrade() else {
        return;
    };
    use tcl_registry::special_vars::NativeErrorStorageVariable::{ErrorCode, ErrorInfo};
    state.rows.borrow_mut().push(OriginalResetRow {
        variable: state.variable,
        code: interpreter.native_error_header_for_observation(ErrorCode),
        info: interpreter.native_error_header_for_observation(ErrorInfo),
        refs: unsafe { (*value).ref_count },
    });
    // This Rust metadata projection adds no native hold on the retiring field.
    drop(interpreter.snapshot_error());
    match state.mode {
        1 => {
            interpreter.refuse_host_command("first selected reset retirement");
        }
        2 => {
            let original = interpreter.runtime_context();
            let mut changed = original.clone();
            changed.packages = vec![("original-reset-retirement".into(), "1.0".into())];
            interpreter.pin_context(&changed).unwrap();
            interpreter.pin_context(&original).unwrap();
        }
        _ => {}
    }
}

static ORIGINAL_RESET_HEADER: obj::TclObjType = obj::TclObjType {
    name: c"originalSelectedResetHeader".as_ptr(),
    free_int_rep_proc: Some(original_reset_header_free),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};

#[test]
fn original_private_reset_decrement_keeps_each_raw_role_without_a_native_reference() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Reset order is source-derived here:614c executed only the distinct setter.
    // These real software callbacks check raw roles and first-cause cleanup.
    use tcl_registry::special_vars::NativeErrorStorageVariable::{ErrorCode, ErrorInfo};
    for environment in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for fail_role in [ErrorCode, ErrorInfo] {
            for mode in 0..3 {
                crate::counters::reset();
                {
                    let selected =
                        tcl_registry::model::ingress::resolve_known_environment(environment)
                            .unwrap();
                    let mut interpreter = Interp::with_native_core(
                        default_host(),
                        selected.unit_profile(),
                        tcl_registry::special_vars::NativeBootstrapInputs::default(),
                    )
                    .unwrap();
                    interpreter.set_result_bytes(b"PRIOR RESULT");
                    let result = interpreter.get_obj_result();
                    let rows = Rc::new(RefCell::new(Vec::new()));
                    let mut originals = Vec::new();
                    for variable in [ErrorCode, ErrorInfo] {
                        let state = Box::new(OriginalResetHeader {
                            interpreter: interpreter.downgrade(),
                            variable,
                            rows: rows.clone(),
                            mode: if variable == fail_role { mode } else { 0 },
                        });
                        let pointer = obj::alloc_typed(
                            &ORIGINAL_RESET_HEADER,
                            Box::into_raw(state) as usize as u64,
                        );
                        // A resident original spelling needs no updater at installation.
                        unsafe {
                            obj::set_native_updater_string_rep(
                                pointer,
                                b"ORIGINAL RESET FIELD",
                                false,
                            )
                        };
                        let memory = obj::NativeObjectLifetime::retain(pointer);
                        interpreter.retain_native_error_option(variable == ErrorInfo, pointer);
                        assert_eq!(unsafe { (*pointer).ref_count }, 1);
                        originals.push(memory);
                    }
                    let code = originals[0].as_ptr();
                    let info = originals[1].as_ptr();
                    assert!(!interpreter.exc.borrow().native.legacy_copy);
                    let scope =
                        native_operation_currency::NativeOperationScope::enter(&interpreter)
                            .unwrap();
                    let reached =
                        interpreter.publish_native_error_objects_checked(scope.currency());
                    assert_eq!(
                        reached.is_ok(),
                        mode == 0,
                        "{environment}, {fail_role:?}, mode{mode}"
                    );
                    let stopped_after_code = mode != 0 && fail_role == ErrorCode;
                    let mut expected = vec![OriginalResetRow {
                        variable: ErrorCode,
                        code: Some(code),
                        info: Some(info),
                        refs: 0,
                    }];
                    if !stopped_after_code {
                        expected.push(OriginalResetRow {
                            variable: ErrorInfo,
                            code: None,
                            info: Some(info),
                            refs: 0,
                        });
                    }
                    assert_eq!(&*rows.borrow(), &expected);
                    assert!(!obj::allocation_is_live(code));
                    assert_eq!(obj::allocation_is_live(info), stopped_after_code);
                    assert_eq!(
                        interpreter.native_error_header_for_observation(ErrorCode),
                        None
                    );
                    assert_eq!(
                        interpreter.native_error_header_for_observation(ErrorInfo),
                        if stopped_after_code { Some(info) } else { None }
                    );
                    assert_eq!(interpreter.get_obj_result(), result);
                    if mode == 1 {
                        let Some(tcl_runtime_api::NativeExecutionError::HostCommandRefusal(first)) =
                            reached.err()
                        else {
                            panic!("lost reset Host cause");
                        };
                        assert_eq!(first.reason, "first selected reset retirement");
                    } else if mode == 2 {
                        assert_eq!(reached.err(), Some(tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                            tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable("stale entered native operation")
                        )));
                    }
                    // Teardown of an unreached next role remains mandatory and
                    // follows whole-state publication, not this selected reset.
                    let retired = interpreter.exc.take();
                    drop(retired);
                    assert_eq!(rows.borrow().len(), 2);
                    drop(scope);
                    assert_eq!(interpreter.entered_native_operation_count(), 0);
                }
                assert_eq!(crate::counters::finalize(), 0);
                assert_eq!(crate::counters::double_free_count(), 0);
            }
        }
    }
}
