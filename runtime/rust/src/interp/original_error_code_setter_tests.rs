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
        interpreter
            .exc
            .borrow()
            .native
            .code
            .as_ref()
            .map(Owned::as_ptr)
    }
}

#[test]
fn original_capi_error_code_setter_retains_lazy_header_without_a_string_getter() {
    // naming.error.original-explicit-error-code-header-setter
    // docs/design/analysis/name-resolution-proofs/error-original-explicit-error-code-header-setter.md
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
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
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
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
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
    // docs/design/analysis/name-resolution-proofs/error-original-explicit-error-code-header-setter.md
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
