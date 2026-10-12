// SPDX-License-Identifier: AGPL-3.0-or-later
//! Genuine extension callback controls for the original checked duplicate owner.
use super::*;
use crate::interp::{native_arguments::OriginalArguments, Interp};
use std::cell::{Cell, RefCell};
use tcl_cmd_core::CmdError;
thread_local! {
    static STAGE: Cell<u8> = const { Cell::new(0) };
    static EVENTS: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
    static INPUT_REFS: Cell<TclSize> = const { Cell::new(-1) };
    static INCOMING: Cell<*mut TclObj> = const { Cell::new(core::ptr::null_mut()) };
    static RECEIVER: Cell<*mut TclObj> = const { Cell::new(core::ptr::null_mut()) };
}
fn actual() -> Interp {
    Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect("tcl9.0"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}
fn current() -> Interp {
    unsafe { crate::codegen_abi::current_interp().as_ref() }
        .unwrap()
        .clone()
}
fn reached(stage: u8) {
    let selected = STAGE.with(Cell::get);
    if selected == stage {
        current().refuse_host_command("original duplicate primary callback");
    }
    if selected == stage + 10 {
        let mut interpreter = current();
        let original = interpreter.runtime_context();
        let mut changed = original.clone();
        changed.packages = vec![("original-duplicate-context".to_owned(), "1".to_owned())];
        interpreter.pin_context(&changed).unwrap();
        interpreter.pin_context(&original).unwrap();
    }
}
extern "C" fn duplicate(source: *mut TclObj, incoming: *mut TclObj) {
    EVENTS.with(|e| e.borrow_mut().push("source-dup"));
    INPUT_REFS.with(|r| r.set(unsafe { (*source).ref_count }));
    // This genuine dup callback owns the new incoming primary, not a native hold.
    change_type(incoming, &INCOMING_TYPE, 7);
    INCOMING.with(|slot| slot.set(incoming));
    reached(1);
    if STAGE.with(Cell::get) == 21 {
        change_type(source, &REENTERED_TYPE, 91);
    }
    if STAGE.with(Cell::get) == 23 {
        let receiver = RECEIVER.with(Cell::get);
        change_type(receiver, &REENTERED_TYPE, 91);
        change_type(receiver, &RECEIVER_TYPE, 6);
    }
}
extern "C" fn receiver_free(receiver: *mut TclObj) {
    EVENTS.with(|e| e.borrow_mut().push("receiver-free"));
    reached(2);
    if STAGE.with(Cell::get) == 22 {
        // A reached extension vacates its selected slot, then publishes through
        // the genuine shared kernel. The outer epoch must preserve this cache.
        unsafe {
            (*receiver).type_ptr = core::ptr::null();
            (*receiver).internal_rep = 0;
        }
        change_type(receiver, &REENTERED_TYPE, 91);
    }
}
extern "C" fn incoming_free(_incoming: *mut TclObj) {
    EVENTS.with(|e| e.borrow_mut().push("incoming-free"));
    if STAGE.with(Cell::get) != 0 {
        current().refuse_host_command("later mandatory duplicate cleanup");
    }
}
static SOURCE_TYPE: TclObjType = TclObjType {
    name: c"originalDuplicateSourceControl".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: Some(duplicate),
    update_string_proc: None,
    set_from_any_proc: None,
};
static RECEIVER_TYPE: TclObjType = TclObjType {
    name: c"originalDuplicateReceiverControl".as_ptr(),
    free_int_rep_proc: Some(receiver_free),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
static INCOMING_TYPE: TclObjType = TclObjType {
    name: c"originalDuplicateIncomingControl".as_ptr(),
    free_int_rep_proc: Some(incoming_free),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
static REENTERED_TYPE: TclObjType = TclObjType {
    name: c"originalDuplicateReenteredControl".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
struct Current;
impl Drop for Current {
    fn drop(&mut self) {
        crate::codegen_abi::tcl_runtime_set_current_interp(core::ptr::null_mut());
    }
}
#[test]
fn checked_original_duplicate_retains_first_cause_before_cleanup_and_refuses_incoming_admission() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Actual software extension dup/free callbacks exercise this checked kernel;
    // this grants no C/Jim adversarial-callback source or native process claim.
    for stage in [1, 2, 11, 12, 21, 22] {
        let mut interpreter = actual();
        let source = Owned::fresh(alloc_typed(&SOURCE_TYPE, 5));
        let receiver = Owned::fresh(alloc_typed(&RECEIVER_TYPE, 6));
        interpreter.set_result_bytes(b"PRIOR");
        let prior = interpreter.get_obj_result();
        let arguments =
            OriginalArguments::enter(&interpreter, &[receiver.as_ptr(), source.as_ptr()]).unwrap();
        crate::codegen_abi::tcl_runtime_set_current_interp(&mut interpreter);
        let _current = Current;
        STAGE.with(|s| s.set(stage));
        EVENTS.with(|e| e.borrow_mut().clear());
        let failure = duplicate_into_with_stage_check(
            receiver.as_ptr(),
            source.as_ptr(),
            &mut || {
                arguments
                    .check_original_for(&interpreter, 0, receiver.as_ptr())
                    .and_then(|()| arguments.check_original_for(&interpreter, 1, source.as_ptr()))
            },
            &mut |error: &CmdError| {
                let _ = arguments.retain_refusal(error.clone());
            },
        )
        .unwrap_err();
        assert_eq!(
            failure.native_execution_refusal(),
            interpreter.native_execution_refusal().as_ref()
        );
        assert_eq!(interpreter.get_obj_result(), prior);
        assert_eq!(INPUT_REFS.with(Cell::get), 1);
        assert_eq!(unsafe { (*source.as_ptr()).ref_count }, 1);
        assert_eq!(unsafe { (*receiver.as_ptr()).ref_count }, 1);
        assert!(!core::ptr::eq(
            obj_type_ptr(receiver.as_ptr()),
            &INCOMING_TYPE
        ));
        if stage == 22 {
            assert!(core::ptr::eq(
                obj_type_ptr(receiver.as_ptr()),
                &REENTERED_TYPE
            ));
            assert_eq!(internal_rep(receiver.as_ptr()), 91);
        }
        EVENTS.with(|events| {
            let events = events.borrow();
            assert_eq!(events.first(), Some(&"source-dup"));
            assert_eq!(events.last(), Some(&"incoming-free"));
            assert_eq!(
                events
                    .iter()
                    .filter(|event| **event == "receiver-free")
                    .count(),
                usize::from([2, 12, 22].contains(&stage))
            );
        });
        STAGE.with(|s| s.set(0));
    }
}
#[test]
fn checked_original_duplicate_transfers_exact_incoming_primary_and_cleanup_once() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Independent direct shared-API ownership control, not a native process row.
    let mut interpreter = actual();
    let source = Owned::fresh(alloc_typed(&SOURCE_TYPE, 5));
    let receiver = Owned::fresh(alloc_typed(&RECEIVER_TYPE, 6));
    let arguments =
        OriginalArguments::enter(&interpreter, &[receiver.as_ptr(), source.as_ptr()]).unwrap();
    crate::codegen_abi::tcl_runtime_set_current_interp(&mut interpreter);
    let _current = Current;
    STAGE.with(|s| s.set(0));
    EVENTS.with(|e| e.borrow_mut().clear());
    duplicate_into_with_stage_check(
        receiver.as_ptr(),
        source.as_ptr(),
        &mut || {
            arguments
                .check_original_for(&interpreter, 0, receiver.as_ptr())
                .and_then(|()| arguments.check_original_for(&interpreter, 1, source.as_ptr()))
        },
        &mut |error: &CmdError| {
            let _ = arguments.retain_refusal(error.clone());
        },
    )
    .unwrap();
    assert!(core::ptr::eq(
        obj_type_ptr(receiver.as_ptr()),
        &INCOMING_TYPE
    ));
    assert_eq!(internal_rep(receiver.as_ptr()), 7);
    assert_eq!(INPUT_REFS.with(Cell::get), 1);
    assert_eq!(
        EVENTS.with(|e| e.borrow().clone()),
        ["source-dup", "receiver-free"]
    );
    drop(receiver);
    assert_eq!(
        EVENTS.with(|e| e.borrow().clone()),
        ["source-dup", "receiver-free", "incoming-free"]
    );
}

#[test]
fn checked_original_duplicate_detects_receiver_change_and_restore_during_source_dup() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Genuine software source-dup callback changes and restores the original
    // receiver before retirement. This is not a C/Jim callback observation.
    let mut interpreter = actual();
    let source = Owned::fresh(alloc_typed(&SOURCE_TYPE, 5));
    let receiver = Owned::fresh(alloc_typed(&RECEIVER_TYPE, 6));
    let receiver_epoch = auxiliary(receiver.as_ptr()).primary_epoch.get();
    let arguments =
        OriginalArguments::enter(&interpreter, &[receiver.as_ptr(), source.as_ptr()]).unwrap();
    crate::codegen_abi::tcl_runtime_set_current_interp(&mut interpreter);
    let _current = Current;
    STAGE.with(|s| s.set(23));
    RECEIVER.with(|slot| slot.set(receiver.as_ptr()));
    EVENTS.with(|e| e.borrow_mut().clear());
    let failure = duplicate_into_with_stage_check(
        receiver.as_ptr(),
        source.as_ptr(),
        &mut || {
            arguments
                .check_original_for(&interpreter, 0, receiver.as_ptr())
                .and_then(|()| arguments.check_original_for(&interpreter, 1, source.as_ptr()))
        },
        &mut |error: &CmdError| {
            let _ = arguments.retain_refusal(error.clone());
        },
    )
    .unwrap_err();
    let expected = CmdError::from(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
        "changed original duplicate receiver primary",
    ));
    assert_eq!(failure, expected);
    assert_eq!(
        interpreter.native_execution_refusal().as_ref(),
        failure.native_execution_refusal()
    );
    assert_ne!(
        auxiliary(receiver.as_ptr()).primary_epoch.get(),
        receiver_epoch
    );
    assert!(core::ptr::eq(
        obj_type_ptr(receiver.as_ptr()),
        &RECEIVER_TYPE
    ));
    assert_eq!(internal_rep(receiver.as_ptr()), 6);
    assert_eq!(INPUT_REFS.with(Cell::get), 1);
    assert_eq!(unsafe { (*receiver.as_ptr()).ref_count }, 1);
    assert_eq!(
        EVENTS.with(|e| e.borrow().clone()),
        ["source-dup", "receiver-free", "incoming-free"]
    );
    STAGE.with(|s| s.set(0));
    RECEIVER.with(|slot| slot.set(core::ptr::null_mut()));
}
#[test]
fn checked_original_duplicate_refuses_exhausted_source_or_receiver_epoch_before_callback() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Fixed software poison-state controls: an exhausted receipt must not
    // accept a later change-and-restore. No native C/Jim epoch is claimed.
    for source_poisoned in [false, true] {
        let mut interpreter = actual();
        let source = Owned::fresh(alloc_typed(&SOURCE_TYPE, 5));
        let receiver = Owned::fresh(alloc_typed(&RECEIVER_TYPE, 6));
        let selected = if source_poisoned {
            source.as_ptr()
        } else {
            receiver.as_ptr()
        };
        auxiliary(selected).primary_epoch.set(None);
        let arguments =
            OriginalArguments::enter(&interpreter, &[receiver.as_ptr(), source.as_ptr()]).unwrap();
        crate::codegen_abi::tcl_runtime_set_current_interp(&mut interpreter);
        let _current = Current;
        STAGE.with(|s| s.set(0));
        EVENTS.with(|e| e.borrow_mut().clear());
        let failure = duplicate_into_with_stage_check(
            receiver.as_ptr(),
            source.as_ptr(),
            &mut || {
                arguments
                    .check_original_for(&interpreter, 0, receiver.as_ptr())
                    .and_then(|()| arguments.check_original_for(&interpreter, 1, source.as_ptr()))
            },
            &mut |error: &CmdError| {
                let _ = arguments.retain_refusal(error.clone());
            },
        )
        .unwrap_err();
        let purpose = if source_poisoned {
            "original duplicate source primary epoch"
        } else {
            "original duplicate receiver primary epoch"
        };
        assert_eq!(
            failure,
            CmdError::from(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                purpose
            ))
        );
        assert!(EVENTS.with(|e| e.borrow().is_empty()));
        assert!(core::ptr::eq(
            obj_type_ptr(receiver.as_ptr()),
            &RECEIVER_TYPE
        ));
        assert_eq!(internal_rep(receiver.as_ptr()), 6);
        assert_eq!(unsafe { (*source.as_ptr()).ref_count }, 1);
        assert_eq!(unsafe { (*receiver.as_ptr()).ref_count }, 1);
    }
}
