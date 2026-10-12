// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual anonymous-cell retirement, not compiled instruction admission.
use super::{Interp, WeakInterp};
use crate::{
    namespace::GLOBAL,
    obj::{self, Owned, TclObj},
};
use std::cell::{Cell, RefCell};
use tcl_runtime_api::NativeExecutionError;
use tcl_syntax::raw_string::NativeValueAccessRefusal;
thread_local! {
    static OWNER: RefCell<Option<WeakInterp>> = const { RefCell::new(None) };
    static MODE: Cell<u8> = const { Cell::new(0) };
    static FREES: Cell<usize> = const { Cell::new(0) };
}
struct Cleanup;
impl Drop for Cleanup {
    fn drop(&mut self) {
        OWNER.with(|owner| owner.borrow_mut().take());
    }
}
fn native(profile: &str) -> Interp {
    Interp::with_native_core(
        super::default_host(),
        crate::environment::profile_for_dialect(profile),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}
fn install(interpreter: &mut Interp, value: *mut TclObj) {
    use tcl_runtime_api::native_compilation::{
        NativeCompiledLocalLayout, NativeCompiledLocalLayoutKind,
    };
    interpreter.frames.borrow_mut().push(GLOBAL);
    let layout = NativeCompiledLocalLayout {
        owner: interpreter.native_command_interpreter,
        token: 1,
        epoch: 0,
        kind: NativeCompiledLocalLayoutKind::Procedure,
        names: vec![None],
    };
    interpreter
        .frames
        .borrow_mut()
        .install_native_compiled_local_layout(&layout)
        .unwrap();
    interpreter
        .store_native_compiled_temporary_checked(0, value)
        .unwrap();
}
fn arm(interpreter: &Interp, mode: u8) -> Cleanup {
    OWNER.with(|owner| *owner.borrow_mut() = Some(interpreter.downgrade()));
    MODE.with(|selected| selected.set(mode));
    FREES.with(|count| count.set(0));
    Cleanup
}
extern "C" fn retire(_: *mut TclObj) {
    FREES.with(|count| count.set(count.get() + 1));
    let Some(mut interpreter) =
        OWNER.with(|owner| owner.borrow().as_ref().and_then(WeakInterp::upgrade))
    else {
        return;
    };
    assert!(interpreter.frames.try_borrow_mut().is_ok());
    assert!(interpreter.namespaces.try_borrow_mut().is_ok());
    // The reached original cell is empty before its displaced owner retires.
    assert!(interpreter
        .frames
        .borrow()
        .native_compiled_temporary(0)
        .unwrap()
        .is_none());
    match MODE.with(Cell::get) {
        1 => {
            interpreter.refuse_host_command("first original anonymous retirement");
        }
        2 => {
            let original = interpreter.runtime_context();
            let mut changed = original.clone();
            changed
                .packages
                .push(("anonymous-retirement".into(), "1.0".into()));
            interpreter.pin_context(&changed).unwrap();
            interpreter.pin_context(&original).unwrap();
        }
        3 => {
            interpreter.frames.borrow_mut().pop().unwrap();
        }
        4 => {
            let replacement = Owned::fresh(obj::new_string_bytes(b"REENTRANT"));
            interpreter
                .store_native_compiled_temporary_checked(0, replacement.as_ptr())
                .unwrap();
        }
        _ => {}
    }
}
static OLD: obj::TclObjType = obj::TclObjType {
    name: c"originalAnonymousRetirement".as_ptr(),
    free_int_rep_proc: Some(retire),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
#[test]
fn original_anonymous_unset_retires_outside_borrows_before_original_context_and_frame_checks() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        for mode in [0, 1, 2, 3] {
            let mut interpreter = native(profile);
            let old = Owned::fresh(obj::alloc_typed(&OLD, 0));
            let allocation = obj::NativeObjectLifetime::retain(old.as_ptr());
            install(&mut interpreter, old.as_ptr());
            drop(old);
            interpreter.set_result_bytes(b"PRIOR");
            let prior = interpreter.get_obj_result();
            let _cleanup = arm(&interpreter, mode);
            let reached = interpreter.unset_native_compiled_temporary_checked(0);
            assert_eq!(reached.is_ok(), mode == 0, "{profile}/{mode}");
            assert_eq!(FREES.with(Cell::get), 1);
            assert!(obj::check_native_liveness(allocation.as_ptr()).is_err());
            assert_eq!(interpreter.get_obj_result(), prior);
            assert_eq!(interpreter.entered_native_operation_count(), 0);
            if mode == 3 {
                assert_eq!(interpreter.current_level(), 0);
            } else {
                assert!(interpreter
                    .frames
                    .borrow()
                    .native_compiled_temporary(0)
                    .unwrap()
                    .is_none());
            }
            match mode {
                0 => assert!(interpreter.native_execution_refusal().is_none()),
                1 => assert!(
                    matches!(interpreter.native_execution_refusal(),Some(NativeExecutionError::HostCommandRefusal(ref first)) if first.reason=="first original anonymous retirement")
                ),
                2 => assert_eq!(
                    interpreter.native_execution_refusal(),
                    Some(NativeExecutionError::ValueAccessRefusal(
                        NativeValueAccessRefusal::CommandProtocolUnavailable(
                            "stale entered native operation"
                        )
                    ))
                ),
                3 => assert_eq!(
                    interpreter.native_execution_refusal(),
                    Some(NativeExecutionError::ValueAccessRefusal(
                        NativeValueAccessRefusal::CommandProtocolUnavailable(
                            "captured native anonymous frame"
                        )
                    ))
                ),
                _ => unreachable!(),
            }
        }
    }
}
#[test]
fn original_anonymous_unset_allows_same_cell_reentry_and_empty_cleanup_without_fresh_receipt() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let mut interpreter = native(profile);
        let old = Owned::fresh(obj::alloc_typed(&OLD, 0));
        install(&mut interpreter, old.as_ptr());
        drop(old);
        let _cleanup = arm(&interpreter, 4);
        assert!(interpreter
            .unset_native_compiled_temporary_checked(0)
            .is_ok());
        assert_eq!(FREES.with(Cell::get), 1);
        let replacement = interpreter
            .frames
            .borrow()
            .native_compiled_temporary(0)
            .unwrap()
            .unwrap();
        assert_eq!(unsafe { (*replacement).ref_count }, 1);
        assert_eq!(obj::bytes_of(replacement), b"REENTRANT");
        assert!(interpreter
            .unset_native_compiled_temporary_checked(0)
            .is_ok());
        assert!(interpreter
            .unset_native_compiled_temporary_checked(0)
            .is_ok());
        assert_eq!(FREES.with(Cell::get), 1);
        assert!(interpreter.native_execution_refusal().is_none());
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    }
}
#[test]
fn original_pending_anonymous_unset_keeps_original_cell_owner_unreached() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let mut interpreter = native(profile);
        let old = Owned::fresh(obj::alloc_typed(&OLD, 0));
        let pointer = old.as_ptr();
        install(&mut interpreter, pointer);
        drop(old);
        let cleanup = arm(&interpreter, 0);
        interpreter.refuse_host_command("prior original anonymous refusal");
        let first = interpreter.native_execution_refusal();
        assert!(interpreter
            .unset_native_compiled_temporary_checked(0)
            .is_err());
        assert_eq!(FREES.with(Cell::get), 0);
        assert_eq!(
            interpreter
                .frames
                .borrow()
                .native_compiled_temporary(0)
                .unwrap(),
            Some(pointer)
        );
        assert_eq!(unsafe { (*pointer).ref_count }, 1);
        assert_eq!(interpreter.native_execution_refusal(), first);
        assert_eq!(interpreter.entered_native_operation_count(), 0);
        drop(cleanup);
    }
}
