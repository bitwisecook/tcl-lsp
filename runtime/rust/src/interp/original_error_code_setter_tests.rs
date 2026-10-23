// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual explicit error-code headers, selected storage and callback retirement.

use super::*;
use crate::obj::Owned;

struct Original {
    interpreter: WeakInterp,
    updates: Rc<Cell<usize>>,
    frees: Rc<Cell<usize>>,
    action: u8,
}

extern "C" fn update(value: *mut TclObj) {
    // SAFETY: this actual extension header owns its boxed state until retirement.
    let state = unsafe { &*(obj::internal_rep(value) as usize as *const Original) };
    state.updates.set(state.updates.get() + 1);
    unsafe { obj::set_native_updater_string_rep(value, b"ORIGINAL\0\xff CODE", false) };
}

extern "C" fn retire(value: *mut TclObj) {
    // SAFETY: the descriptor retires this original boxed record exactly once.
    let state = unsafe { Box::from_raw(obj::internal_rep(value) as usize as *mut Original) };
    state.frees.set(state.frees.get() + 1);
    if let Some(mut interpreter) = state.interpreter.upgrade() {
        // This genuine reentrant read must run outside the exception/table borrow.
        drop(interpreter.snapshot_error());
        match state.action {
            1 => {
                interpreter.refuse_host_command("first original error-code retirement");
            }
            2 => {
                let original = interpreter.runtime_context();
                let mut changed = original.clone();
                changed.packages = vec![("original-error-code-retirement".into(), "1.0".into())];
                interpreter.pin_context(&changed).unwrap();
                interpreter.pin_context(&original).unwrap();
            }
            _ => {}
        }
    }
}

static ORIGINAL: obj::TclObjType = obj::TclObjType {
    name: c"originalExplicitErrorCode".as_ptr(),
    free_int_rep_proc: Some(retire),
    dup_int_rep_proc: None,
    update_string_proc: Some(update),
    set_from_any_proc: None,
};

fn native(name: &str) -> Interp {
    let environment = tcl_registry::model::ingress::resolve_known_environment(name)
        .expect("actual known environment ingress");
    Interp::with_native_core(
        default_host(),
        environment.unit_profile(),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .expect("genuine selected native bootstrap")
}

fn original(interpreter: &Interp, action: u8) -> (Owned, Rc<Cell<usize>>, Rc<Cell<usize>>) {
    let updates = Rc::new(Cell::new(0));
    let frees = Rc::new(Cell::new(0));
    let state = Box::new(Original {
        interpreter: interpreter.downgrade(),
        updates: updates.clone(),
        frees: frees.clone(),
        action,
    });
    let object = obj::alloc_typed(&ORIGINAL, Box::into_raw(state) as usize as u64);
    (Owned::fresh(object), updates, frees)
}

fn selected_header(interpreter: &Interp) -> Option<*mut TclObj> {
    if interpreter.uses_c84_global_error_info() {
        crate::vars::get_at(
            &interpreter.frames.borrow(),
            &interpreter.namespaces.borrow(),
            b"::errorCode",
            0,
        )
    } else {
        interpreter.native_error_header_for_observation(
            tcl_registry::special_vars::NativeErrorStorageVariable::ErrorCode,
        )
    }
}

#[test]
fn original_capi_error_code_setter_retains_lazy_header_without_a_string_getter() {
    // naming.error.original-explicit-error-code-header-setter

    // These software headers/getter counts test the supplied setter; provider observations remain separate.
    for name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        crate::counters::reset();
        {
            let mut interpreter = native(name);
            interpreter.set_result_bytes(b"PRESERVED RESULT");
            let prior = interpreter.get_obj_result();
            let (code, updates, frees) = original(&interpreter, 0);
            let count = interpreter.c_api_errors.get();
            unsafe { crate::capi::Tcl_SetObjErrorCode(&mut interpreter, code.as_ptr()) };
            assert_eq!(updates.get(), 0, "{name}");
            assert_eq!(frees.get(), 0);
            assert_eq!(selected_header(&interpreter), Some(code.as_ptr()));
            assert_eq!(unsafe { (*code.as_ptr()).ref_count }, 2);
            assert_eq!(interpreter.get_obj_result(), prior);
            assert_eq!(interpreter.c_api_errors.get(), count + 1);
            assert_eq!(
                interpreter.error_code_bytes_checked().unwrap(),
                b"ORIGINAL\0\xff CODE"
            );
            assert_eq!(updates.get(), 1);
            assert_eq!(selected_header(&interpreter), Some(code.as_ptr()));
            assert!(interpreter.native_execution_refusal().is_none());
        }
        assert_eq!(crate::counters::finalize(), 0);
        assert_eq!(crate::counters::double_free_count(), 0);
    }
}

#[test]
fn original_capi_error_code_setter_retirement_keeps_first_cause_and_reached_storage() {
    // naming.embedding.original-host-publication-and-fact-transport

    // Actual free hooks and selected storage, with no inferred C callback timing.
    for name in ["tcl8.4", "tcl8.6", "tcl9.1"] {
        for action in [1, 2] {
            let mut interpreter = native(name);
            let (old, updates, frees) = original(&interpreter, action);
            unsafe { crate::capi::Tcl_SetObjErrorCode(&mut interpreter, old.as_ptr()) };
            drop(old);
            let incoming = Owned::fresh(new_string(b"INCOMING CODE"));
            let count = interpreter.c_api_errors.get();
            unsafe { crate::capi::Tcl_SetObjErrorCode(&mut interpreter, incoming.as_ptr()) };
            assert_eq!(updates.get(), 0);
            assert_eq!(frees.get(), 1);
            assert_eq!(interpreter.c_api_errors.get(), count);
            if interpreter.uses_c84_global_error_info() {
                assert_eq!(selected_header(&interpreter), Some(incoming.as_ptr()));
            } else {
                assert_eq!(selected_header(&interpreter), None);
                assert_eq!(unsafe { (*incoming.as_ptr()).ref_count }, 1);
            }
            if action == 1 {
                let Some(tcl_runtime_api::NativeExecutionError::HostCommandRefusal(first)) =
                    interpreter.native_execution_refusal()
                else {
                    panic!("original Host refusal lost");
                };
                assert_eq!(first.reason, "first original error-code retirement");
            } else {
                assert_eq!(interpreter.native_execution_refusal(), Some(
                    tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                        tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable("stale entered native operation")
                    )
                ));
            }
        }
    }
}

#[test]
fn original_capi_error_code_setter_pending_host_keeps_all_later_stages_unreached() {
    // naming.embedding.original-host-publication-and-fact-transport

    for name in ["tcl8.4", "tcl8.6", "tcl9.1"] {
        let mut interpreter = native(name);
        interpreter.set_c_error_code(b"BEFORE CODE");
        let prior = selected_header(&interpreter);
        let count = interpreter.c_api_errors.get();
        let (incoming, updates, frees) = original(&interpreter, 0);
        interpreter.refuse_host_command("original pending setter refusal");
        let first = interpreter.native_execution_refusal();
        unsafe { crate::capi::Tcl_SetObjErrorCode(&mut interpreter, incoming.as_ptr()) };
        assert_eq!(updates.get(), 0);
        assert_eq!(frees.get(), 0);
        assert_eq!(unsafe { (*incoming.as_ptr()).ref_count }, 1);
        assert_eq!(selected_header(&interpreter), prior);
        assert_eq!(interpreter.c_api_errors.get(), count);
        assert_eq!(interpreter.native_execution_refusal(), first);
    }
}

unsafe extern "C" fn explicit_callback(
    _client: *mut core::ffi::c_void,
    interpreter: *mut Interp,
    _argc: core::ffi::c_int,
    _argv: *const *mut TclObj,
) -> core::ffi::c_int {
    // SAFETY: an actual installed object command supplies this live interpreter.
    let interpreter = unsafe { &mut *interpreter };
    interpreter.set_result_bytes(b"ORIGINAL CALLBACK ERROR");
    let code = Owned::fresh(new_string(b"ORIGINAL CALLBACK CODE"));
    unsafe { crate::capi::Tcl_SetObjErrorCode(interpreter, code.as_ptr()) };
    1
}

#[test]
fn original_c84_explicit_callback_code_survives_actual_command_logging() {
    // naming.error.original-explicit-error-code-header-setter

    // Software error projection is not a C8.4 Tcl_GetReturnOptions availability claim.
    let mut interpreter = native("tcl8.4");
    assert!(interpreter
        .create_obj_command(
            b"::original_explicit_error",
            ObjCommand::new(explicit_callback, core::ptr::null_mut(), None)
        )
        .is_some());
    assert_eq!(
        interpreter.eval_str(b"::original_explicit_error"),
        Code::Error
    );
    assert_eq!(
        interpreter.error_code_bytes_checked().unwrap(),
        b"ORIGINAL CALLBACK CODE"
    );
    assert!(interpreter.exc.borrow().native.global_code_set);
    assert!(interpreter.native_execution_refusal().is_none());
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct HeaderRetirementRow {
    selected: Option<*mut TclObj>,
    old_refs: obj::TclSize,
    incoming_refs: obj::TclSize,
    semantic_refused_with_original_role: bool,
}

struct HeaderRetirement {
    interpreter: WeakInterp,
    incoming: Rc<Cell<*mut TclObj>>,
    rows: Rc<RefCell<Vec<HeaderRetirementRow>>>,
    updates: Rc<Cell<usize>>,
    mode: u8,
}

extern "C" fn header_retirement_update(value: *mut TclObj) {
    // SAFETY: the real extension header owns this record until its free worker.
    let state = unsafe { &*(obj::internal_rep(value) as usize as *const HeaderRetirement) };
    state.updates.set(state.updates.get() + 1);
    unsafe { obj::set_native_updater_string_rep(value, b"OLD HEADER", false) };
}

extern "C" fn header_retirement_free(value: *mut TclObj) {
    // SAFETY: this real free worker consumes the sole boxed extension record.
    let state =
        unsafe { Box::from_raw(obj::internal_rep(value) as usize as *mut HeaderRetirement) };
    let Some(mut interpreter) = state.interpreter.upgrade() else {
        return;
    };
    let incoming = state.incoming.get();
    // These are only raw header/role observations at the actual decrement.
    // The allocation lease supplies no native retain or semantic getter grant.
    state.rows.borrow_mut().push(HeaderRetirementRow {
        selected: selected_header(&interpreter),
        old_refs: unsafe { (*value).ref_count },
        incoming_refs: if incoming.is_null() {
            -1
        } else {
            unsafe { (*incoming).ref_count }
        },
        semantic_refused_with_original_role: false,
    });
    // A readonly Rust snapshot clones the role descriptor, not the retired native owner.
    drop(interpreter.snapshot_error());
    match state.mode {
        1 => {
            interpreter.refuse_host_command("first raw header retirement");
        }
        2 => {
            let original = interpreter.runtime_context();
            let mut changed = original.clone();
            changed.packages = vec![("raw-error-header-retirement".into(), "1.0".into())];
            interpreter.pin_context(&changed).unwrap();
            interpreter.pin_context(&original).unwrap();
        }
        3 => {
            interpreter.set_c_error_code(b"REENTERED CODE");
        }
        4 => {
            let refused = interpreter.native_private_error_object(false).is_none()
                && selected_header(&interpreter) == Some(value)
                && interpreter.native_execution_refusal().is_some();
            state
                .rows
                .borrow_mut()
                .last_mut()
                .unwrap()
                .semantic_refused_with_original_role = refused;
        }
        _ => {}
    }
}

static HEADER_RETIREMENT: obj::TclObjType = obj::TclObjType {
    name: c"originalErrorHeaderRetirement".as_ptr(),
    free_int_rep_proc: Some(header_retirement_free),
    dup_int_rep_proc: None,
    update_string_proc: Some(header_retirement_update),
    set_from_any_proc: None,
};

fn retiring_header(
    interpreter: &Interp,
    mode: u8,
) -> (
    Owned,
    Rc<Cell<*mut TclObj>>,
    Rc<RefCell<Vec<HeaderRetirementRow>>>,
    Rc<Cell<usize>>,
) {
    let incoming = Rc::new(Cell::new(core::ptr::null_mut()));
    let rows = Rc::new(RefCell::new(Vec::new()));
    let updates = Rc::new(Cell::new(0));
    let state = Box::new(HeaderRetirement {
        interpreter: interpreter.downgrade(),
        incoming: incoming.clone(),
        rows: rows.clone(),
        updates: updates.clone(),
        mode,
    });
    let value = obj::alloc_typed(&HEADER_RETIREMENT, Box::into_raw(state) as usize as u64);
    (Owned::fresh(value), incoming, rows, updates)
}

fn original_header_observer_field<'a>(line: &'a str, field: &str) -> &'a str {
    line.split('|')
        .filter_map(|part| part.split_once('='))
        .find(|(key, _)| *key == field)
        .expect("actual original header observer field")
        .1
}

fn original_header_observer(name: &str) -> &'static str {
    match name {
        "tcl8.4" => include_str!(
            "../../tests/data/native_explicit_error_code_header_setter_original/614c/providers/8.4.20/execute.stdout"
        ),
        "tcl8.5" => include_str!(
            "../../tests/data/native_explicit_error_code_header_setter_original/614c/providers/8.5.19/execute.stdout"
        ),
        "tcl8.6" => include_str!(
            "../../tests/data/native_explicit_error_code_header_setter_original/614c/providers/8.6.18/execute.stdout"
        ),
        "tcl9.0" => include_str!(
            "../../tests/data/native_explicit_error_code_header_setter_original/614c/providers/9.0.4/execute.stdout"
        ),
        "tcl9.1" => include_str!(
            "../../tests/data/native_explicit_error_code_header_setter_original/614c/providers/9.1.0/execute.stdout"
        ),
        _ => panic!("fixed original five-provider observer inventory"),
    }
}

#[test]
fn original_error_code_decrement_keeps_the_selected_raw_header_role_without_a_native_hold() {
    // naming.error.original-explicit-error-code-header-setter

    // The five-provider614c observer establishes only raw header/ref/flag stages;
    // this actual software callback checks those roles, without a getter grant.
    for name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for mode in 0..3 {
            crate::counters::reset();
            {
                let mut interpreter = native(name);
                interpreter.set_result_bytes(b"PRIOR RESULT");
                let prior = interpreter.get_obj_result();
                let (old, incoming_slot, rows, updates) = retiring_header(&interpreter, mode);
                let old_pointer = old.as_ptr();
                let old_memory = obj::NativeObjectLifetime::retain(old_pointer);
                interpreter.set_original_c_error_code(old_pointer).unwrap();
                drop(old);
                assert_eq!(unsafe { (*old_pointer).ref_count }, 1);
                let incoming = Owned::fresh(new_string(b"INCOMING CODE"));
                incoming_slot.set(incoming.as_ptr());
                let count = interpreter.c_api_errors.get();
                let reached = interpreter.set_original_c_error_code(incoming.as_ptr());
                assert_eq!(reached.is_ok(), mode == 0, "{name}, mode{mode}");
                let global = interpreter.uses_c84_global_error_info();
                assert_eq!(
                    &*rows.borrow(),
                    &[HeaderRetirementRow {
                        selected: Some(if global {
                            incoming.as_ptr()
                        } else {
                            old_pointer
                        }),
                        old_refs: 0,
                        incoming_refs: if global { 2 } else { 1 },
                        semantic_refused_with_original_role: false,
                    }]
                );
                if mode == 0 {
                    let observed = original_header_observer(name);
                    assert!(observed.lines().any(|line| line == "CLOSED_CASE|1"));
                    let free = observed
                        .lines()
                        .find(|line| {
                            line.starts_with("EVENT|1|")
                                && line.contains("|phase=public_setter|kind=free_int_rep|record=A|")
                        })
                        .unwrap();
                    assert_eq!(
                        original_header_observer_field(free, "refs"),
                        rows.borrow()[0].old_refs.to_string()
                    );
                    assert_eq!(
                        original_header_observer_field(free, "selected"),
                        if global { "B" } else { "A" }
                    );
                    let before = observed
                        .lines()
                        .find(|line| {
                            line.starts_with("ROW|1|")
                                && line.contains("|stage=before_public_setter|")
                        })
                        .unwrap();
                    assert_eq!(original_header_observer_field(before, "A_refs"), "1");
                    assert_eq!(original_header_observer_field(before, "B_refs"), "1");
                    let after = observed
                        .lines()
                        .find(|line| {
                            line.starts_with("ROW|1|")
                                && line.contains("|stage=after_public_setter_before_getter|")
                        })
                        .unwrap();
                    assert_eq!(original_header_observer_field(after, "selected"), "B");
                    assert_eq!(original_header_observer_field(after, "A_free"), "1");
                    assert_eq!(
                        original_header_observer_field(after, "B_refs"),
                        unsafe { (*incoming.as_ptr()).ref_count }.to_string()
                    );
                    assert_eq!(
                        original_header_observer_field(after, "A_update"),
                        updates.get().to_string()
                    );
                    assert_eq!(original_header_observer_field(after, "B_update"), "0");
                    assert_eq!(original_header_observer_field(after, "result_same"), "1");
                }
                assert_eq!(updates.get(), 0);
                assert!(!obj::allocation_is_live(old_memory.as_ptr()));
                assert_eq!(interpreter.get_obj_result(), prior);
                assert_eq!(interpreter.c_api_errors.get(), count + u64::from(mode == 0));
                assert_eq!(
                    selected_header(&interpreter),
                    if global || mode == 0 {
                        Some(incoming.as_ptr())
                    } else {
                        None
                    }
                );
                assert_eq!(
                    unsafe { (*incoming.as_ptr()).ref_count },
                    if global || mode == 0 { 2 } else { 1 }
                );
                if mode == 1 {
                    let Some(tcl_runtime_api::NativeExecutionError::HostCommandRefusal(first)) =
                        reached.err()
                    else {
                        panic!("lost original Host cause");
                    };
                    assert_eq!(first.reason, "first raw header retirement");
                } else if mode == 2 {
                    assert_eq!(reached.err(), Some(tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                        tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable("stale entered native operation")
                    )));
                }
                assert_eq!(interpreter.entered_native_operation_count(), 0);
            }
            assert_eq!(crate::counters::finalize(), 0);
            assert_eq!(crate::counters::double_free_count(), 0);
        }
    }
}

#[test]
fn original_error_retirement_descriptor_refuses_semantic_reentry_and_callback_replacement() {
    // naming.embedding.original-host-publication-and-fact-transport

    // Pure software reentry and first-cause controls;614c grants only the raw observer above.
    for mode in [3, 4] {
        crate::counters::reset();
        {
            let mut interpreter = native("tcl8.6");
            interpreter.set_result_bytes(b"PRIOR RESULT");
            let prior = interpreter.get_obj_result();
            let (old, incoming_slot, rows, updates) = retiring_header(&interpreter, mode);
            let old_pointer = old.as_ptr();
            let old_memory = obj::NativeObjectLifetime::retain(old_pointer);
            interpreter.set_original_c_error_code(old_pointer).unwrap();
            drop(old);
            let incoming = Owned::fresh(new_string(b"UNREACHED INCOMING"));
            incoming_slot.set(incoming.as_ptr());
            let count = interpreter.c_api_errors.get();
            let reached = interpreter.set_original_c_error_code(incoming.as_ptr());
            assert_eq!(updates.get(), 0);
            assert_eq!(rows.borrow().len(), 1);
            assert_eq!(rows.borrow()[0].selected, Some(old_pointer));
            assert_eq!(rows.borrow()[0].old_refs, 0);
            assert_eq!(rows.borrow()[0].incoming_refs, 1);
            assert_eq!(
                rows.borrow()[0].semantic_refused_with_original_role,
                mode == 4
            );
            assert_eq!(unsafe { (*incoming.as_ptr()).ref_count }, 1);
            assert_eq!(interpreter.get_obj_result(), prior);
            assert!(!obj::allocation_is_live(old_memory.as_ptr()));
            assert_eq!(interpreter.c_api_errors.get(), count + u64::from(mode == 3));
            let reason = if mode == 3 {
                assert_eq!(&interpreter.exc.borrow().code, b"REENTERED CODE");
                assert!(interpreter.exc.borrow().native.code.is_some());
                "private error retirement role replaced during callback"
            } else {
                assert!(selected_header(&interpreter).is_none());
                "retiring private error header"
            };
            assert_eq!(
                reached.err(),
                Some(tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        reason
                    )
                ))
            );
            assert_eq!(
                interpreter.native_execution_refusal(),
                Some(tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        reason
                    )
                ))
            );
            assert_eq!(interpreter.entered_native_operation_count(), 0);
        }
        assert_eq!(crate::counters::finalize(), 0);
        assert_eq!(crate::counters::double_free_count(), 0);
    }
}
