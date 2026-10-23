// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual stored-owner retirement and entered currency; no Native chronology grant.

use super::{native_operation_currency::NativeOperationScope, Interp, WeakInterp};
use crate::{
    frame::VarError,
    namespace::GLOBAL,
    obj::{self, Owned, TclObj},
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use tcl_runtime_api::{
    native_variable_trace::{
        NativeVariableObserver, NativeVariableTraceAccess, NativeVariableTraceOperation,
    },
    NativeExecutionError,
};
use tcl_syntax::raw_string::NativeValueAccessRefusal;

thread_local! {
    static OWNER: RefCell<Option<WeakInterp>> = const { RefCell::new(None) };
    static MODE: Cell<u8> = const { Cell::new(0) };
    static ROUTE: Cell<u8> = const { Cell::new(0) };
    static FREES: Cell<usize> = const { Cell::new(0) };
    static WRITES: Cell<usize> = const { Cell::new(0) };
    static REACHED: Cell<*mut TclObj> = const { Cell::new(std::ptr::null_mut()) };
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
fn text(bytes: &[u8]) -> Owned {
    Owned::fresh(obj::new_string_bytes(bytes))
}
fn observed(interpreter: &Interp) -> Option<*mut TclObj> {
    let frames = interpreter.frames.borrow();
    let namespaces = interpreter.namespaces();
    match ROUTE.with(Cell::get) {
        0 | 1 => crate::vars::get_at(&frames, &namespaces, b"::stored", 0),
        2 | 3 => crate::vars::get_elem_at(&frames, &namespaces, b"::stored", b"member", 0),
        4 => frames.native_compiled_temporary(0).unwrap(),
        5 => crate::vars::get_at(&frames, &namespaces, b"::tcl_version", 0),
        _ => unreachable!(),
    }
}
extern "C" fn retire(_: *mut TclObj) {
    FREES.with(|frees| frees.set(frees.get() + 1));
    let Some(mut interpreter) =
        OWNER.with(|owner| owner.borrow().as_ref().and_then(WeakInterp::upgrade))
    else {
        return;
    };
    // The real enclosing frame, namespace and cell owners are all available.
    assert!(interpreter.frames.try_borrow_mut().is_ok());
    assert!(interpreter.namespaces.try_borrow_mut().is_ok());
    REACHED.with(|value| value.set(observed(&interpreter).unwrap()));
    match MODE.with(Cell::get) {
        1 => {
            interpreter.refuse_host_command("first original byte-store retirement");
        }
        2 => {
            let original = interpreter.runtime_context();
            let mut changed = original.clone();
            changed
                .packages
                .push(("byte-store-retirement".into(), "1.0".into()));
            interpreter.pin_context(&changed).unwrap();
            interpreter.pin_context(&original).unwrap();
        }
        3 => {
            assert!(interpreter.var_unset(b"::stored"));
            let replacement = text(b"RECREATED");
            interpreter
                .var_set(b"::stored", replacement.as_ptr())
                .unwrap();
        }
        4 => {
            // The new value is a pure owned String. Popping this actual frame
            // withdraws its incarnation while the captured receiver remains.
            interpreter.frames.borrow_mut().pop().unwrap();
        }
        _ => {}
    }
}
static OLD: obj::TclObjType = obj::TclObjType {
    name: c"originalStoredRetirement".as_ptr(),
    free_int_rep_proc: Some(retire),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
struct Write;
impl NativeVariableObserver<Interp> for Write {
    type Error = tcl_cmd_core::CmdError;
    fn observe(
        &self,
        _: &mut Interp,
        access: NativeVariableTraceAccess<'_>,
    ) -> Result<(), Self::Error> {
        assert_eq!(access.operation, NativeVariableTraceOperation::Write);
        WRITES.with(|writes| writes.set(writes.get() + 1));
        Ok(())
    }
}
fn setup(interpreter: &mut Interp, route: u8, value: *mut TclObj) {
    ROUTE.with(|selected| selected.set(route));
    match route {
        0 | 1 => interpreter.var_set_at(b"::stored", value, 0).unwrap(),
        2 | 3 => interpreter
            .var_set_elem_at(b"::stored", b"member", value, 0)
            .unwrap(),
        4 => {
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
        5 => interpreter.var_set_at(b"::tcl_version", value, 0).unwrap(),
        _ => unreachable!(),
    }
}
fn store(interpreter: &mut Interp, route: u8, value: *mut TclObj) -> Result<(), VarError> {
    match route {
        0 => interpreter.var_set(b"::stored", value),
        1 => interpreter.var_set_at(b"::stored", value, 0),
        2 => interpreter.var_set_elem(b"::stored", b"member", value),
        3 => interpreter.var_set_elem_at(b"::stored", b"member", value, 0),
        4 => interpreter.store_native_compiled_temporary_checked(0, value),
        5 => {
            interpreter.write_release_globals();
            interpreter
                .check_entered_native_operation()
                .map_err(|_| VarError::NameProtocolUnavailable)
        }
        _ => unreachable!(),
    }
}
fn arm(interpreter: &Interp, mode: u8) -> Cleanup {
    OWNER.with(|owner| *owner.borrow_mut() = Some(interpreter.downgrade()));
    MODE.with(|selected| selected.set(mode));
    FREES.with(|count| count.set(0));
    WRITES.with(|count| count.set(0));
    REACHED.with(|value| value.set(std::ptr::null_mut()));
    Cleanup
}
fn cause(interpreter: &Interp, mode: u8) {
    let first = interpreter
        .native_execution_refusal()
        .expect("actual original refusal");
    match mode {
        1 => assert!(
            matches!(first, NativeExecutionError::HostCommandRefusal(ref first) if first.reason=="first original byte-store retirement")
        ),
        2 => assert_eq!(
            first,
            NativeExecutionError::ValueAccessRefusal(
                NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "stale entered native operation"
                )
            )
        ),
        3 => assert!(matches!(
            first,
            NativeExecutionError::ValueAccessRefusal(
                NativeValueAccessRefusal::CommandProtocolUnavailable(_)
            )
        )),
        _ => unreachable!(),
    }
}
#[test]
fn original_byte_stores_retire_actual_old_owner_outside_all_storage_borrows() {
    // naming.embedding.original-host-publication-and-fact-transport

    // Logical storage transport, distinct from original C/Jim pointer chronology.
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        for route in 0..5 {
            for mode in [0, 1, 2] {
                let mut interpreter = native(profile);
                let old = Owned::fresh(obj::alloc_typed(&OLD, 0));
                let allocation = obj::NativeObjectLifetime::retain(old.as_ptr());
                setup(&mut interpreter, route, old.as_ptr());
                drop(old);
                let next = text(b"NEXT\0VALUE");
                interpreter.set_result_bytes(b"PRIOR");
                let prior = interpreter.get_obj_result();
                let _cleanup = arm(&interpreter, mode);
                let result = store(&mut interpreter, route, next.as_ptr());
                assert_eq!(result.is_ok(), mode == 0, "{profile}/{route}/{mode}");
                assert_eq!(FREES.with(Cell::get), 1);
                assert_eq!(REACHED.with(Cell::get), next.as_ptr());
                assert_eq!(observed(&interpreter), Some(next.as_ptr()));
                assert_eq!(
                    unsafe { (*next.as_ptr()).ref_count },
                    2,
                    "caller plus actual cell"
                );
                assert!(!obj::allocation_is_live(allocation.as_ptr()));
                assert_eq!(interpreter.get_obj_result(), prior);
                if mode == 0 {
                    assert!(interpreter.native_execution_refusal().is_none());
                } else {
                    cause(&interpreter, mode);
                }
                assert_eq!(interpreter.entered_native_operation_count(), 0);
            }
        }
    }
}
#[test]
fn original_scalar_store_stops_before_write_trace_after_retirement_refusal() {
    // naming.embedding.original-host-publication-and-fact-transport

    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for mode in [0, 1, 2] {
            let mut interpreter = native(profile);
            let old = Owned::fresh(obj::alloc_typed(&OLD, 0));
            setup(&mut interpreter, 0, old.as_ptr());
            drop(old);
            let name = text(b"::stored");
            interpreter
                .add_native_variable_observer(
                    name.as_ptr(),
                    &[NativeVariableTraceOperation::Write],
                    Rc::new(Write),
                )
                .unwrap();
            let next = text(b"NEXT");
            let _cleanup = arm(&interpreter, mode);
            let result = store(&mut interpreter, 0, next.as_ptr());
            assert_eq!(result.is_ok(), mode == 0);
            assert_eq!(WRITES.with(Cell::get), usize::from(mode == 0));
            assert_eq!(observed(&interpreter), Some(next.as_ptr()));
            if mode != 0 {
                cause(&interpreter, mode);
            }
        }
    }
}
#[test]
fn original_byte_store_cannot_adopt_a_recreated_binding_after_old_release() {
    // naming.embedding.original-host-publication-and-fact-transport

    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let mut interpreter = native(profile);
        let old = Owned::fresh(obj::alloc_typed(&OLD, 0));
        setup(&mut interpreter, 0, old.as_ptr());
        drop(old);
        let next = text(b"REACHED");
        let _cleanup = arm(&interpreter, 3);
        let result = store(&mut interpreter, 0, next.as_ptr());
        assert!(result.is_err());
        assert_eq!(FREES.with(Cell::get), 1);
        assert_eq!(REACHED.with(Cell::get), next.as_ptr());
        let current = observed(&interpreter).unwrap();
        assert_ne!(current, next.as_ptr());
        assert_eq!(obj::bytes_of(current), b"RECREATED");
        assert_eq!(unsafe { (*next.as_ptr()).ref_count }, 1);
        cause(&interpreter, 3);
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    }
}
#[test]
fn original_release_global_storage_retires_before_next_owned_bootstrap_slot() {
    // naming.embedding.original-host-publication-and-fact-transport

    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        for mode in [0, 1, 2] {
            let mut interpreter = native(profile);
            let old = Owned::fresh(obj::alloc_typed(&OLD, 0));
            setup(&mut interpreter, 5, old.as_ptr());
            drop(old);
            let patch = text(b"PATCH-SENTINEL");
            interpreter
                .var_set_at(b"::tcl_patchLevel", patch.as_ptr(), 0)
                .unwrap();
            let _cleanup = arm(&interpreter, mode);
            let entered = NativeOperationScope::enter(&interpreter).unwrap();
            interpreter.write_release_globals();
            assert_eq!(FREES.with(Cell::get), 1);
            assert_eq!(REACHED.with(Cell::get), observed(&interpreter).unwrap());
            assert_ne!(
                obj::bytes_of(observed(&interpreter).unwrap()),
                b"PATCH-SENTINEL"
            );
            let after = crate::vars::get_at(
                &interpreter.frames.borrow(),
                &interpreter.namespaces(),
                b"::tcl_patchLevel",
                0,
            )
            .unwrap();
            if mode == 0 {
                assert_ne!(after, patch.as_ptr());
                assert!(entered.currency().ensure_current().is_ok());
            } else {
                assert_eq!(after, patch.as_ptr());
                cause(&interpreter, mode);
            }
            drop(entered);
            assert_eq!(interpreter.entered_native_operation_count(), 0);
        }
    }
}

#[test]
fn original_pending_byte_store_refusal_keeps_old_owner_and_incoming_reference_unreached() {
    // naming.embedding.original-host-publication-and-fact-transport

    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let mut interpreter = native(profile);
        let old = Owned::fresh(obj::alloc_typed(&OLD, 0));
        let allocation = obj::NativeObjectLifetime::retain(old.as_ptr());
        setup(&mut interpreter, 0, old.as_ptr());
        drop(old);
        let next = text(b"UNREACHED");
        let _cleanup = arm(&interpreter, 0);
        interpreter.refuse_host_command("already pending byte-store refusal");
        let first = interpreter.native_execution_refusal();
        assert!(store(&mut interpreter, 0, next.as_ptr()).is_err());
        assert_eq!(FREES.with(Cell::get), 0);
        assert_eq!(observed(&interpreter), Some(allocation.as_ptr()));
        assert_eq!(unsafe { (*next.as_ptr()).ref_count }, 1);
        assert_eq!(interpreter.native_execution_refusal(), first);
        assert_eq!(interpreter.entered_native_operation_count(), 0);
        // Mandatory cell retirement remains possible after the refused store.
        // Disarm the observer rather than starting another guarded operation.
        OWNER.with(|owner| owner.borrow_mut().take());
    }
}

#[test]
fn original_anonymous_store_keeps_its_frame_incarnation_through_actual_retirement() {
    // naming.embedding.original-host-publication-and-fact-transport

    // Actual installed cell/frame transport, not a compiler instruction grant.
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let mut interpreter = native(profile);
        let old = Owned::fresh(obj::alloc_typed(&OLD, 0));
        setup(&mut interpreter, 4, old.as_ptr());
        drop(old);
        let next = text(b"REACHED");
        let _cleanup = arm(&interpreter, 4);
        let result = store(&mut interpreter, 4, next.as_ptr());
        assert!(result.is_err());
        assert_eq!(FREES.with(Cell::get), 1);
        assert_eq!(REACHED.with(Cell::get), next.as_ptr());
        assert_eq!(unsafe { (*next.as_ptr()).ref_count }, 1);
        assert_eq!(interpreter.current_level(), 0);
        assert!(matches!(
            interpreter.native_execution_refusal(),
            Some(NativeExecutionError::ValueAccessRefusal(
                NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "captured native anonymous frame"
                )
            ))
        ));
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    }
}
