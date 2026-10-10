// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Software controls for original ABI getters and first-cause settlement.
//! These do not establish an external C Tcl/Jim/BIG-IP observation.

use super::*;
use crate::obj::{self, Owned, TclObjType};
use std::cell::{Cell, RefCell};
use tcl_dialect::TclVersion;
use tcl_runtime_api::NativeExecutionError;
use tcl_syntax::raw_string::{NativeMaterializationLimitError, NativeValueAccessRefusal};

#[derive(Clone, Copy)]
enum Input {
    DefinitionName,
    PackageName,
    PackageVersion,
    ErrorMessage,
    ErrorCode,
}

fn native(version: TclVersion) -> Interp {
    let mut interp = Interp::new();
    interp.set_runtime_version(version);
    interp
}

fn call_with_original(interp: &mut Interp, input: Input, original: *mut TclObj) -> c_int {
    let name = Owned::fresh(new_string(b"blocked"));
    let parameters = Owned::fresh(new_string(b""));
    let body = Owned::fresh(new_string(b"return INSTALLED"));
    let version = Owned::fresh(new_string(b"1.0"));
    let message = Owned::fresh(new_string(b"LATER MESSAGE"));
    let code = Owned::fresh(new_string(b"LATER CODE"));
    // SAFETY: this interpreter and every selected original remain live.
    unsafe {
        match input {
            Input::DefinitionName => {
                tcl_engine_define_unit(interp, original, parameters.as_ptr(), body.as_ptr())
            }
            Input::PackageName => tcl_engine_provide_package(interp, original, version.as_ptr()),
            Input::PackageVersion => tcl_engine_provide_package(interp, name.as_ptr(), original),
            Input::ErrorMessage => tcl_engine_fail(interp, original, code.as_ptr()),
            Input::ErrorCode => tcl_engine_fail(interp, message.as_ptr(), original),
        }
    }
}

fn seed_guest_state(interp: &mut Interp) -> *mut TclObj {
    assert_eq!(interp.eval_str(b"set prior BEFORE"), Code::Ok);
    interp.set_error_state(b"STAMP BEFORE");
    interp.set_result_bytes(b"BODY\0\xff");
    interp.set_return_state(2, Code::Other(7));
    interp.set_return_options(vec![(b"-custom".to_vec(), b"ORIGINAL".to_vec())]);
    interp.get_obj_result()
}

fn assert_guest_state(interp: &Interp, original_result: *mut TclObj) {
    assert_eq!(interp.get_obj_result(), original_result);
    assert!(obj::allocation_is_live(original_result));
    assert_eq!(obj::bytes_of(original_result), b"BODY\0\xff");
    assert_eq!(interp.snapshot_error().code_bytes(), b"STAMP BEFORE");
    assert_eq!(interp.pending_return_level(), 2);
    assert_eq!(interp.pending_return_code(), Code::Other(7));
    assert_eq!(interp.pending_return_option_objects().len(), 1);
    assert_eq!(obj::bytes_of(interp.var_get(b"prior").unwrap()), b"BEFORE");
}

thread_local! {
    static UPDATER_INTERP: RefCell<Option<Interp>> = const { RefCell::new(None) };
    static UPDATER_CALLS: Cell<usize> = const { Cell::new(0) };
    static UPDATER_REFERENCE_COUNT: Cell<isize> = const { Cell::new(-1) };
    static LATER_GETTERS: Cell<usize> = const { Cell::new(0) };
}

struct UpdaterContext;
impl Drop for UpdaterContext {
    fn drop(&mut self) {
        UPDATER_INTERP.with(|slot| slot.borrow_mut().take());
    }
}

extern "C" fn refusing_updater(original: *mut TclObj) {
    UPDATER_CALLS.with(|calls| calls.set(calls.get() + 1));
    // SAFETY: this extension's actual updater receives its live header.
    UPDATER_REFERENCE_COUNT.with(|count| count.set(unsafe { (*original).ref_count }));
    let mut interp = UPDATER_INTERP.with(|slot| slot.borrow().as_ref().unwrap().clone());
    interp.refuse_host_command("first cause reached inside ABI original updater");
    // The producer can finish its own bytes; they cannot authorise publication.
    unsafe { obj::set_string_rep(original, b"PRODUCED AFTER REFUSAL") };
}

extern "C" fn later_updater(original: *mut TclObj) {
    LATER_GETTERS.with(|calls| calls.set(calls.get() + 1));
    // SAFETY: this extension's actual updater receives its live header.
    unsafe { obj::set_string_rep(original, b"LATER") };
}

static REFUSING_TYPE: TclObjType = TclObjType {
    name: c"testAbiOriginalRefusal".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(refusing_updater),
    set_from_any_proc: None,
};
static LATER_TYPE: TclObjType = TclObjType {
    name: c"testAbiLaterGetter".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(later_updater),
    set_from_any_proc: None,
};

#[test]
fn original_abi_lazy_input_refusals_preserve_guest_state_and_stop_publication() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for input in [
        Input::DefinitionName,
        Input::PackageName,
        Input::PackageVersion,
        Input::ErrorMessage,
        Input::ErrorCode,
    ] {
        let mut interp = native(TclVersion::V9_0);
        assert_eq!(interp.eval_str(b"lseq 100000001"), Code::Ok);
        let original = Owned::retain(interp.get_obj_result());
        assert!(crate::native_arithseries::is_series(original.as_ptr()));
        assert!(!obj::has_string_rep(original.as_ptr()));
        assert!(!crate::native_arithseries::has_element_cache(
            original.as_ptr()
        ));
        let result = seed_guest_state(&mut interp);
        let result_lifetime = obj::NativeObjectLifetime::retain(result);
        let commands = interp.namespaces().native_command_generations();
        assert_eq!(
            call_with_original(&mut interp, input, original.as_ptr()),
            TCL_ERROR
        );
        let cause =
            NativeExecutionError::ValueAccessRefusal(NativeValueAccessRefusal::Materialization(
                NativeMaterializationLimitError::new(100_000_001, 100_000_000),
            ));
        assert_eq!(interp.native_execution_refusal(), Some(cause.clone()));
        assert_guest_state(&interp, result_lifetime.as_ptr());
        assert_eq!(interp.namespaces().native_command_generations(), commands);
        assert!(!obj::has_string_rep(original.as_ptr()));
        assert!(!crate::native_arithseries::has_element_cache(
            original.as_ptr()
        ));
        let later = Owned::fresh(obj::alloc_typed(&LATER_TYPE, 0));
        LATER_GETTERS.with(|calls| calls.set(0));
        for later_input in [
            Input::DefinitionName,
            Input::PackageName,
            Input::ErrorMessage,
        ] {
            assert_eq!(
                call_with_original(&mut interp, later_input, later.as_ptr()),
                TCL_ERROR
            );
        }
        LATER_GETTERS.with(|calls| assert_eq!(calls.get(), 0));
        assert!(!obj::has_string_rep(later.as_ptr()));
        assert_eq!(interp.native_execution_refusal(), Some(cause.clone()));
        assert_guest_state(&interp, result_lifetime.as_ptr());
        assert_eq!(
            crate::completion::capture_bytes(&mut interp, Code::Error).unwrap_err(),
            cause
        );
        // A later, independent query checks that no package was provided.
        interp.reset_native_compilation_admission();
        interp.reset_result();
        assert_eq!(interp.eval_str(b"package provide blocked"), Code::Ok);
        assert_eq!(interp.result_bytes(), b"");
    }
}

#[test]
fn original_abi_reentrant_updaters_keep_first_cause_and_skip_later_getters() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for input in [Input::DefinitionName, Input::PackageName, Input::ErrorCode] {
        let mut interp = native(TclVersion::V8_6);
        let result = seed_guest_state(&mut interp);
        let result_lifetime = obj::NativeObjectLifetime::retain(result);
        let original = Owned::fresh(obj::alloc_typed(&REFUSING_TYPE, 0));
        let later = Owned::fresh(obj::alloc_typed(&LATER_TYPE, 0));
        UPDATER_INTERP.with(|slot| *slot.borrow_mut() = Some(interp.clone()));
        let _updater_context = UpdaterContext;
        UPDATER_CALLS.with(|calls| calls.set(0));
        LATER_GETTERS.with(|calls| calls.set(0));
        // SAFETY: all originals are live, and the updater uses this interpreter's shared owner.
        let status = unsafe {
            match input {
                Input::DefinitionName => tcl_engine_define_unit(
                    &mut interp,
                    original.as_ptr(),
                    later.as_ptr(),
                    later.as_ptr(),
                ),
                Input::PackageName => {
                    tcl_engine_provide_package(&mut interp, original.as_ptr(), later.as_ptr())
                }
                Input::ErrorCode => tcl_engine_fail(&mut interp, later.as_ptr(), original.as_ptr()),
                _ => unreachable!(),
            }
        };
        assert_eq!(status, TCL_ERROR);
        UPDATER_CALLS.with(|calls| assert_eq!(calls.get(), 1));
        UPDATER_REFERENCE_COUNT.with(|count| assert_eq!(count.get(), 1));
        LATER_GETTERS.with(|calls| assert_eq!(calls.get(), 0));
        assert!(!obj::has_string_rep(later.as_ptr()));
        assert_eq!(obj::bytes_of(original.as_ptr()), b"PRODUCED AFTER REFUSAL");
        let cause = interp.native_execution_refusal().unwrap();
        let NativeExecutionError::HostCommandRefusal(metadata) = &cause else {
            panic!("lost original updater Host cause");
        };
        assert_eq!(
            metadata.reason,
            "first cause reached inside ABI original updater"
        );
        assert_guest_state(&interp, result_lifetime.as_ptr());
        interp.refuse_native_access(NativeValueAccessRefusal::ExpressionEngineUnavailable);
        assert_eq!(interp.native_execution_refusal(), Some(cause.clone()));
        assert_eq!(
            call_with_original(&mut interp, Input::ErrorMessage, later.as_ptr()),
            TCL_ERROR
        );
        LATER_GETTERS.with(|calls| assert_eq!(calls.get(), 0));
        assert_guest_state(&interp, result_lifetime.as_ptr());
        assert_eq!(
            crate::completion::capture_bytes(&mut interp, Code::Error).unwrap_err(),
            cause
        );
    }
}

#[test]
fn original_abi_counted_package_and_error_bytes_keep_nuls_and_opaque_suffixes() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for version in [
        TclVersion::V8_4,
        TclVersion::V8_5,
        TclVersion::V8_6,
        TclVersion::V9_0,
        TclVersion::V9_1,
    ] {
        let mut interp = native(version);
        let original_result = seed_guest_state(&mut interp);
        let procedure_name = Owned::fresh(obj::new_wide_int_obj(123));
        let parameters = Owned::fresh(new_string(b""));
        let body = Owned::fresh(new_string(b"return ORIGINAL"));
        assert!(!obj::has_string_rep(procedure_name.as_ptr()));
        // SAFETY: the selected updater and installer borrow these live originals.
        assert_eq!(
            unsafe {
                tcl_engine_define_unit(
                    &mut interp,
                    procedure_name.as_ptr(),
                    parameters.as_ptr(),
                    body.as_ptr(),
                )
            },
            TCL_OK
        );
        assert_eq!(obj::bytes_of(procedure_name.as_ptr()), b"123");
        assert!(interp.resolve_cmd_token(b"123").is_some());
        assert_guest_state(&interp, original_result);
        let name = Owned::fresh(new_string(b"package\0\xff"));
        let prefix = Owned::fresh(new_string(b"package"));
        let one = Owned::fresh(new_string(b"1.0"));
        let two = Owned::fresh(new_string(b"2.0"));
        // SAFETY: actual original name/version objects remain live through both calls.
        assert_eq!(
            unsafe { tcl_engine_provide_package(&mut interp, name.as_ptr(), one.as_ptr()) },
            TCL_OK
        );
        assert_eq!(
            unsafe { tcl_engine_provide_package(&mut interp, prefix.as_ptr(), two.as_ptr()) },
            TCL_OK
        );
        assert_guest_state(&interp, original_result);
        assert_eq!(
            unsafe { tcl_engine_provide_package(&mut interp, name.as_ptr(), two.as_ptr()) },
            TCL_ERROR
        );
        assert_eq!(
            interp.result_bytes(),
            b"conflicting versions provided for package \"package\0\xff\": 1.0, then 2.0"
        );
        assert_eq!(
            interp.error_code_bytes_checked().unwrap(),
            b"TCL PACKAGE VERSIONCONFLICT"
        );
        let invalid = Owned::fresh(new_string(b"1.0\0\xff"));
        assert_eq!(
            unsafe { tcl_engine_provide_package(&mut interp, prefix.as_ptr(), invalid.as_ptr()) },
            TCL_ERROR
        );
        assert_eq!(
            interp.result_bytes(),
            b"expected version number but got \"1.0\0\xff\""
        );
        assert_eq!(
            interp.error_code_bytes_checked().unwrap(),
            if matches!(version, TclVersion::V8_4 | TclVersion::V8_5) {
                b"NONE".as_slice()
            } else {
                b"TCL VALUE VERSION".as_slice()
            }
        );
        let message = Owned::fresh(new_string(b"MESSAGE\0\xff"));
        let code = Owned::fresh(new_string(b"CODE\0\xff{}"));
        // SAFETY: each status call borrows the live original message/code objects.
        assert_eq!(
            unsafe { tcl_engine_fail(&mut interp, message.as_ptr(), code.as_ptr()) },
            TCL_ERROR
        );
        assert_eq!(interp.result_bytes(), b"MESSAGE\0\xff");
        assert_eq!(interp.error_code_bytes_checked().unwrap(), b"CODE\0\xff{}");
        let empty_code = Owned::fresh(new_string(b""));
        assert_eq!(
            unsafe { tcl_engine_fail(&mut interp, message.as_ptr(), empty_code.as_ptr()) },
            TCL_ERROR
        );
        assert_eq!(interp.error_code_bytes_checked().unwrap(), b"");
        assert_eq!(
            unsafe { tcl_engine_fail(&mut interp, message.as_ptr(), core::ptr::null_mut()) },
            TCL_ERROR
        );
        assert_eq!(interp.error_code_bytes_checked().unwrap(), b"NONE");
        assert!(!interp.host_refusal_pending());
    }
}
