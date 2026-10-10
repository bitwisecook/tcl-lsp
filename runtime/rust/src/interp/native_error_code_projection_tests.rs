// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Software controls for original completion getters and first-cause transport.
//! These assertions do not establish an external C Tcl/Jim/BIG-IP observation.

use super::{Code, Interp};
use crate::obj::{self, TclObj, TclObjType};
use std::cell::{Cell, RefCell};
use tcl_dialect::TclVersion;
use tcl_runtime_api::NativeExecutionError;
use tcl_syntax::raw_string::{NativeMaterializationLimitError, NativeValueAccessRefusal};

fn native(version: TclVersion) -> Interp {
    let mut interp = Interp::new();
    interp.set_runtime_version(version);
    interp
}

fn oversized_code(interp: &mut Interp) -> obj::Owned {
    assert_eq!(interp.eval_str(b"lseq 100000001"), Code::Ok);
    let original = obj::Owned::retain(interp.get_obj_result());
    assert!(crate::native_arithseries::is_series(original.as_ptr()));
    assert!(!obj::has_string_rep(original.as_ptr()));
    assert!(!crate::native_arithseries::has_element_cache(
        original.as_ptr()
    ));
    interp.set_error_state(b"STAMP BEFORE");
    interp.retain_native_error_option(false, original.as_ptr());
    interp.set_result_bytes(b"BODY\0\xff");
    original
}

fn capacity_failure() -> NativeExecutionError {
    NativeExecutionError::ValueAccessRefusal(NativeValueAccessRefusal::Materialization(
        NativeMaterializationLimitError::new(100_000_001, 100_000_000),
    ))
}

#[test]
fn implicit_absence_explicit_empty_and_opaque_original_code_have_separate_guest_values() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut interp = native(TclVersion::V8_6);
    assert_eq!(interp.error_code_bytes_checked().unwrap(), b"NONE");
    interp.set_error_state(b"");
    interp.mark_error_code_explicit();
    let empty = obj::Owned::fresh(obj::new_string_bytes(b""));
    interp.retain_native_error_option(false, empty.as_ptr());
    assert_eq!(interp.error_code_bytes_checked().unwrap(), b"");
    let original = obj::Owned::fresh(obj::new_string_bytes(b"RAW\0\xff{}"));
    interp.retain_native_error_option(false, original.as_ptr());
    assert_eq!(interp.error_code_bytes_checked().unwrap(), b"RAW\0\xff{}");
    assert_eq!(
        interp.exc.borrow().native.code.as_ref().unwrap().as_ptr(),
        original.as_ptr()
    );
    assert!(interp.native_execution_refusal().is_none());
}

#[test]
fn original_lazy_code_getter_failure_retains_capacity_and_no_guest_fallback() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut interp = native(TclVersion::V9_0);
    let original = oversized_code(&mut interp);
    let result = interp.get_obj_result();
    let code = interp.exc.borrow().code.clone();
    let failure = interp.error_code_bytes_checked().unwrap_err();
    assert_eq!(failure, capacity_failure());
    assert_eq!(interp.native_execution_refusal(), Some(failure.clone()));
    assert_eq!(interp.error_code_bytes_checked().unwrap_err(), failure);
    assert_eq!(
        crate::cmd_error::completion_options(&mut interp, Code::Error).unwrap_err(),
        failure
    );
    assert_eq!(
        crate::completion::capture_bytes(&mut interp, Code::Error).unwrap_err(),
        failure
    );
    assert_eq!(interp.get_obj_result(), result);
    assert_eq!(interp.result_bytes(), b"BODY\0\xff");
    assert_eq!(interp.exc.borrow().code, code);
    assert_eq!(
        interp.exc.borrow().native.code.as_ref().unwrap().as_ptr(),
        original.as_ptr()
    );
    assert!(!obj::has_string_rep(original.as_ptr()));
    assert!(!crate::native_arithseries::has_element_cache(
        original.as_ptr()
    ));
    // SAFETY: the interpreter remains owned and live; Host returns no object.
    let exported = unsafe { crate::engine_abi::tcl_engine_error_code(&raw mut interp) };
    assert!(exported.is_null());
    assert_eq!(interp.native_execution_refusal(), Some(failure));
}

thread_local! {
    static UPDATER_INTERP: RefCell<Option<Interp>> = const { RefCell::new(None) };
    static UPDATER_REFERENCE_COUNT: Cell<i32> = const { Cell::new(-1) };
}
struct UpdaterContext;
impl Drop for UpdaterContext {
    fn drop(&mut self) {
        UPDATER_INTERP.with(|slot| slot.borrow_mut().take());
    }
}
extern "C" fn refusing_updater(value: *mut TclObj) {
    // SAFETY: the actual extension descriptor invokes this on its live header.
    UPDATER_REFERENCE_COUNT.with(|observed| observed.set(unsafe { (*value).ref_count }));
    let mut interp = UPDATER_INTERP.with(|slot| slot.borrow().as_ref().unwrap().clone());
    interp.refuse_host_command("first cause reached inside original updater");
    // A producer may finish its own representation after a reached refusal.
    // The consumer must still reject guest publication of these bytes.
    unsafe { obj::set_string_rep(value, b"PRODUCED AFTER REFUSAL") };
}
static EXTENSION_CODE: TclObjType = TclObjType {
    name: c"testOriginalErrorCode".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(refusing_updater),
    set_from_any_proc: None,
};

#[test]
fn reentrant_original_updater_retains_first_cause_without_adding_native_reference() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut interp = native(TclVersion::V8_6);
    interp.set_error_state(b"STAMP BEFORE");
    interp.set_result_bytes(b"BODY BEFORE");
    let original = obj::Owned::fresh(obj::alloc_typed(&EXTENSION_CODE, 0));
    let pointer = original.as_ptr();
    interp.retain_native_error_option(false, pointer);
    drop(original); // the private slot is the sole actual native reference
    UPDATER_INTERP.with(|slot| *slot.borrow_mut() = Some(interp.clone()));
    UPDATER_REFERENCE_COUNT.with(|observed| observed.set(-1));
    let _context = UpdaterContext;
    let failure = interp.error_code_bytes_checked().unwrap_err();
    let expected = NativeExecutionError::HostCommandRefusal(Box::new(
        interp.native_host_command_refusal().unwrap(),
    ));
    assert_eq!(failure, expected);
    UPDATER_REFERENCE_COUNT.with(|observed| assert_eq!(observed.get(), 1));
    assert_eq!(obj::bytes_of(pointer), b"PRODUCED AFTER REFUSAL");
    assert_eq!(interp.result_bytes(), b"BODY BEFORE");
    assert_eq!(interp.exc.borrow().code, b"STAMP BEFORE");
    interp.refuse_native_access(NativeValueAccessRefusal::ExpressionEngineUnavailable);
    assert_eq!(interp.native_execution_refusal(), Some(expected.clone()));
    assert_eq!(interp.error_code_bytes_checked().unwrap_err(), expected);
}

#[test]
fn neutral_error_carriers_and_child_transport_preserve_original_host_metadata() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut origin = native(TclVersion::V8_6);
    origin.refuse_host_command("origin's exact cause");
    let cause = origin.native_execution_refusal().unwrap();
    let carrier = tcl_cmd_core::CmdError::from_execution_refusal(cause.clone());
    assert_eq!(carrier.native_execution_refusal(), Some(&cause));
    assert_eq!(carrier.message().unwrap_err(), cause);
    assert_eq!(carrier.error_code().unwrap_err(), cause);
    assert_eq!(carrier.clone().into_details().unwrap_err(), cause);
    let mut target = native(TclVersion::V9_0);
    target.set_result_bytes(b"TARGET BEFORE");
    target.set_error_state(b"TARGET CODE");
    assert_eq!(target.report_cmd_error(carrier), Code::Error);
    assert_eq!(target.native_execution_refusal(), Some(cause.clone()));
    assert_eq!(target.result_bytes(), b"TARGET BEFORE");
    assert_eq!(target.exc.borrow().code, b"TARGET CODE");
    target.report_expr_error(crate::expr_error::ExprError::host_refusal(
        NativeValueAccessRefusal::ExpressionEngineUnavailable,
    ));
    assert_eq!(target.native_execution_refusal(), Some(cause.clone()));
    let mut expression_target = native(TclVersion::V8_5);
    expression_target.set_result_bytes(b"EXPRESSION BEFORE");
    expression_target.report_expr_error(crate::expr_error::ExprError::from_execution_refusal(
        cause.clone(),
    ));
    assert_eq!(
        expression_target.native_execution_refusal(),
        Some(cause.clone())
    );
    assert_eq!(expression_target.result_bytes(), b"EXPRESSION BEFORE");
    let mut child_target = native(TclVersion::V8_6);
    child_target.transport_host_refusal_from(&expression_target);
    assert_eq!(child_target.native_execution_refusal(), Some(cause.clone()));
    child_target.transport_host_refusal_from(&target);
    assert_eq!(child_target.native_execution_refusal(), Some(cause));
}

thread_local! {
    static ARRAY_READ_TRACES: Cell<usize> = const { Cell::new(0) };
}
fn trace_refusal(interp: &mut Interp, _argv: &[*mut TclObj]) -> Code {
    let reads = ARRAY_READ_TRACES.with(|reads| {
        reads.set(reads.get() + 1);
        reads.get()
    });
    if reads == 1 {
        Code::Ok
    } else {
        interp.refuse_host_command("reached array read trace")
    }
}

#[test]
fn array_read_trace_host_cause_cannot_become_a_skipped_guest_element() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut interp = native(TclVersion::V8_6);
    interp.register_builtin(b"host_trace", trace_refusal);
    assert_eq!(
        interp.eval_str(b"set prior BEFORE; array set a {x 1 y 2}"),
        Code::Ok
    );
    let elements = [
        interp.var_get_elem(b"a", b"x").unwrap(),
        interp.var_get_elem(b"a", b"y").unwrap(),
    ];
    // Both genuine array-cell roles remain live during this storage observation.
    let references = elements.map(|value| unsafe { (*value).ref_count });
    ARRAY_READ_TRACES.with(|reads| reads.set(0));
    let code = interp.eval_str(b"trace add variable a read host_trace; catch {array get a} captured options; set after YES");
    ARRAY_READ_TRACES.with(|reads| assert_eq!(reads.get(), 2));
    assert_eq!(
        elements.map(|value| unsafe { (*value).ref_count }),
        references,
        "prior successful element's temporary native pin was released"
    );
    assert_eq!(code, Code::Error);
    assert_eq!(
        interp.native_host_command_refusal().unwrap().reason,
        "reached array read trace"
    );
    assert_eq!(
        super::obj_bytes(interp.var_get(b"prior").unwrap()),
        b"BEFORE"
    );
    for name in [b"captured".as_slice(), b"options", b"after"] {
        assert!(!interp.var_exists(name));
    }
}

fn fail_with_lazy_code(interp: &mut Interp, _argv: &[*mut TclObj]) -> Code {
    let _original = oversized_code(interp);
    Code::Error
}

#[test]
fn try_error_code_getter_refusal_skips_handler_finally_and_later_stores() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut interp = native(TclVersion::V9_0);
    interp.register_builtin(b"lazy_code", fail_with_lazy_code);
    let code = interp.eval_str(b"set prior BEFORE; try {lazy_code} trap {ANY} {message options} {set handled YES} finally {set final YES}; set after YES");
    assert_eq!(code, Code::Error);
    assert_eq!(interp.native_execution_refusal(), Some(capacity_failure()));
    assert_eq!(
        super::obj_bytes(interp.var_get(b"prior").unwrap()),
        b"BEFORE"
    );
    for name in [
        b"message".as_slice(),
        b"options",
        b"handled",
        b"final",
        b"after",
    ] {
        assert!(!interp.var_exists(name));
    }
}

fn refuse_event(interp: &mut Interp, _argv: &[*mut TclObj]) -> Code {
    interp.refuse_host_command("first event's original host cause")
}

#[test]
fn event_host_cause_stops_later_callbacks_and_guest_stores() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut interp = native(TclVersion::V8_6);
    interp.register_builtin(b"refuse_event", refuse_event);
    let code = interp.eval_str(b"set prior BEFORE; after idle refuse_event; after idle {set later YES}; update idletasks; set after YES");
    assert_eq!(code, Code::Error);
    let cause = interp.native_execution_refusal().unwrap();
    let NativeExecutionError::HostCommandRefusal(metadata) = &cause else {
        panic!("lost reached original event host cause");
    };
    assert_eq!(metadata.reason, "first event's original host cause");
    assert_eq!(
        super::obj_bytes(interp.var_get(b"prior").unwrap()),
        b"BEFORE"
    );
    assert!(!interp.var_exists(b"later"));
    assert!(!interp.var_exists(b"after"));
    interp.process_bg_errors();
    assert_eq!(interp.native_execution_refusal(), Some(cause));
    assert!(!interp.var_exists(b"later"));
}
