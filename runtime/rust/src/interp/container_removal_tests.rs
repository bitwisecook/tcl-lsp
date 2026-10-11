// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original removal and mandatory retirement; no Native chronology grant.

use super::{Interp, WeakInterp};
use crate::{
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
    static READS: Cell<usize> = const { Cell::new(0) };
    static UNSETS: Cell<usize> = const { Cell::new(0) };
    static REFILL: Cell<*mut TclObj> = const { Cell::new(std::ptr::null_mut()) };
    static TRACE_OLD: Cell<*mut TclObj> = const { Cell::new(std::ptr::null_mut()) };
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
fn arm(interpreter: &Interp, mode: u8, route: u8) -> Cleanup {
    OWNER.with(|owner| *owner.borrow_mut() = Some(interpreter.downgrade()));
    MODE.with(|value| value.set(mode));
    ROUTE.with(|value| value.set(route));
    FREES.with(|value| value.set(0));
    READS.with(|value| value.set(0));
    UNSETS.with(|value| value.set(0));
    REFILL.with(|value| value.set(std::ptr::null_mut()));
    Cleanup
}
fn observed(interpreter: &Interp) -> Option<*mut TclObj> {
    let frames = interpreter.frames.borrow();
    let namespaces = interpreter.namespaces();
    match ROUTE.with(Cell::get) {
        0 | 1 => crate::vars::get_at(&frames, &namespaces, b"::removed", 0),
        2 | 3 => crate::vars::get_elem_at(&frames, &namespaces, b"::removed", b"member", 0),
        4 => crate::vars::get_at(&frames, &namespaces, b"local", frames.current_level()),
        5 => crate::vars::get_at(&frames, &namespaces, b"::retiring::value", 0),
        _ => unreachable!(),
    }
}
fn change(interpreter: &mut Interp) {
    match MODE.with(Cell::get) {
        1 => {
            interpreter.refuse_host_command("first original removal retirement");
        }
        2 => {
            let original = interpreter.runtime_context();
            let mut changed = original.clone();
            changed
                .packages
                .push(("original-removal-retirement".into(), "1.0".into()));
            interpreter.pin_context(&changed).unwrap();
            interpreter.pin_context(&original).unwrap();
        }
        3 => {
            let replacement = text(b"SAME CELL REFILL");
            match ROUTE.with(Cell::get) {
                0 | 1 => interpreter
                    .var_set(b"::removed", replacement.as_ptr())
                    .unwrap(),
                2 | 3 => interpreter
                    .var_set_elem(b"::removed", b"member", replacement.as_ptr())
                    .unwrap(),
                5 => interpreter
                    .var_set(b"::retiring::value", replacement.as_ptr())
                    .unwrap(),
                _ => unreachable!(),
            }
            REFILL.with(|pointer| pointer.set(replacement.as_ptr()));
        }
        4 => {
            let (_, retired) = interpreter
                .frames
                .borrow_mut()
                .take_frame_for_pop()
                .unwrap();
            let storage = retired.release();
            interpreter.frames.borrow_mut().recycle_jim_storage(storage);
            interpreter.frames.borrow_mut().push(GLOBAL);
            let replacement = text(b"NEW FRAME");
            interpreter.var_set(b"local", replacement.as_ptr()).unwrap();
        }
        _ => {}
    }
}
extern "C" fn retire(value: *mut TclObj) {
    let first = FREES.with(|count| {
        count.set(count.get() + 1);
        count.get() == 1
    });
    let Some(mut interpreter) =
        OWNER.with(|owner| owner.borrow().as_ref().and_then(WeakInterp::upgrade))
    else {
        return;
    };
    assert!(interpreter.frames.try_borrow_mut().is_ok());
    assert!(interpreter.namespaces.try_borrow_mut().is_ok());
    assert_eq!(
        unsafe { (*value).ref_count },
        0,
        "the removed native owner is released, without a protective +1"
    );
    // This is a storage projection, independent of guarded name admission.
    if ROUTE.with(Cell::get) != 6 {
        assert!(observed(&interpreter).is_none());
    }
    if first {
        change(&mut interpreter);
    }
}
extern "C" fn replace_frame(value: *mut TclObj) {
    READS.with(|count| count.set(count.get() + 1));
    let mut interpreter = OWNER
        .with(|owner| owner.borrow().as_ref().and_then(WeakInterp::upgrade))
        .unwrap();
    assert!(interpreter.frames.try_borrow_mut().is_ok());
    assert!(interpreter.namespaces.try_borrow_mut().is_ok());
    let (_, retired) = interpreter
        .frames
        .borrow_mut()
        .take_frame_for_pop()
        .unwrap();
    let storage = retired.release();
    interpreter.frames.borrow_mut().recycle_jim_storage(storage);
    interpreter.frames.borrow_mut().push(GLOBAL);
    install_layout(&mut interpreter);
    let replacement = text(b"NEW FRAME MEMBER");
    interpreter
        .var_set_elem(b"local", b"member", replacement.as_ptr())
        .unwrap();
    REFILL.with(|pointer| pointer.set(replacement.as_ptr()));
    unsafe { obj::set_native_updater_string_rep(value, b"member", false) };
}
static OLD: obj::TclObjType = obj::TclObjType {
    name: c"originalRemovalRetirement".as_ptr(),
    free_int_rep_proc: Some(retire),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
static ELEMENT: obj::TclObjType = obj::TclObjType {
    name: c"originalUnsetFrameGetter".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(replace_frame),
    set_from_any_proc: None,
};
fn cause(interpreter: &Interp, mode: u8) {
    let first = interpreter
        .native_execution_refusal()
        .expect("actual original refusal");
    match mode {
        1 => assert!(
            matches!(first, NativeExecutionError::HostCommandRefusal(ref first)
            if first.reason == "first original removal retirement")
        ),
        2 => assert_eq!(
            first,
            NativeExecutionError::ValueAccessRefusal(
                NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "stale entered native operation"
                )
            )
        ),
        _ => assert!(matches!(
            first,
            NativeExecutionError::ValueAccessRefusal(
                NativeValueAccessRefusal::CommandProtocolUnavailable(_)
            )
        )),
    }
}
fn setup(interpreter: &mut Interp, route: u8, value: *mut TclObj) {
    match route {
        0 | 1 => interpreter.var_set_at(b"::removed", value, 0).unwrap(),
        2 | 3 => interpreter
            .var_set_elem_at(b"::removed", b"member", value, 0)
            .unwrap(),
        4 => {
            interpreter.frames.borrow_mut().push(GLOBAL);
            interpreter.var_set(b"local", value).unwrap();
        }
        5 => {
            interpreter.ensure_global_namespace(b"retiring");
            interpreter
                .var_set_at(b"::retiring::value", value, 0)
                .unwrap();
        }
        _ => unreachable!(),
    }
}
fn remove(interpreter: &mut Interp, route: u8) -> bool {
    match route {
        0 => interpreter.var_unset(b"::removed"),
        1 => interpreter.var_unset_at(b"::removed", 0),
        2 => interpreter.var_unset_elem(b"::removed", b"member"),
        3 => interpreter.var_unset_elem_at(b"::removed", b"member", 0),
        4 => interpreter.var_unset(b"local"),
        _ => unreachable!(),
    }
}
fn install_layout(interpreter: &mut Interp) {
    use tcl_runtime_api::native_compilation::{
        NativeCompiledLocalLayout, NativeCompiledLocalLayoutKind,
    };
    let layout = NativeCompiledLocalLayout {
        owner: interpreter.native_command_interpreter,
        token: 1,
        epoch: 0,
        kind: NativeCompiledLocalLayoutKind::Procedure,
        names: vec![Some(tcl_core_types::NameBytes::from(b"local".as_slice()))],
    };
    interpreter
        .frames
        .borrow_mut()
        .install_native_compiled_local_layout(&layout)
        .unwrap();
}
struct Unset;
impl NativeVariableObserver<Interp> for Unset {
    type Error = tcl_cmd_core::CmdError;
    fn observe(
        &self,
        _: &mut Interp,
        access: NativeVariableTraceAccess<'_>,
    ) -> Result<(), Self::Error> {
        assert_eq!(access.operation, NativeVariableTraceOperation::Unset);
        UNSETS.with(|count| count.set(count.get() + 1));
        Ok(())
    }
}

#[test]
fn original_byte_unsets_retire_actual_owners_outside_all_storage_borrows() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Generic software storage safety does not attest native pointer chronology.
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        for route in 0..4 {
            for mode in 0..3 {
                let mut interpreter = native(profile);
                let old = Owned::fresh(obj::alloc_typed(&OLD, 0));
                let memory = obj::NativeObjectLifetime::retain(old.as_ptr());
                setup(&mut interpreter, route, old.as_ptr());
                drop(old);
                interpreter.set_result_bytes(b"PRIOR");
                let prior = interpreter.get_obj_result();
                let _cleanup = arm(&interpreter, mode, route);
                assert_eq!(
                    remove(&mut interpreter, route),
                    mode == 0,
                    "{profile}/{route}/{mode}"
                );
                assert_eq!(FREES.with(Cell::get), 1);
                assert!(observed(&interpreter).is_none());
                assert!(obj::check_native_liveness(memory.as_ptr()).is_err());
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
fn original_removal_allows_reached_refill_but_refuses_a_reborn_frame() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        for route in 0..4 {
            let mut interpreter = native(profile);
            let old = Owned::fresh(obj::alloc_typed(&OLD, 0));
            setup(&mut interpreter, route, old.as_ptr());
            drop(old);
            let _cleanup = arm(&interpreter, 3, route);
            assert!(remove(&mut interpreter, route));
            assert_eq!(FREES.with(Cell::get), 1);
            let reached = observed(&interpreter).unwrap();
            assert_eq!(reached, REFILL.with(Cell::get));
            assert_eq!(obj::bytes_of(reached), b"SAME CELL REFILL");
            assert_eq!(unsafe { (*reached).ref_count }, 1);
            assert!(interpreter.native_execution_refusal().is_none());
        }
        let mut interpreter = native(profile);
        let old = Owned::fresh(obj::alloc_typed(&OLD, 0));
        setup(&mut interpreter, 4, old.as_ptr());
        drop(old);
        let original = interpreter.frames.borrow().current_activation_owner();
        let _cleanup = arm(&interpreter, 4, 4);
        assert!(!remove(&mut interpreter, 4));
        assert_eq!(FREES.with(Cell::get), 1);
        assert_eq!(original.strong_count(), 0);
        assert_eq!(obj::bytes_of(observed(&interpreter).unwrap()), b"NEW FRAME");
        cause(&interpreter, 4);
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    }
}

#[test]
fn original_detached_array_finishes_member_owners_after_first_refusal_before_later_traces() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for mode in 0..3 {
            let mut interpreter = native(profile);
            let mut memory = Vec::new();
            for member in [b"one".as_slice(), b"two".as_slice()] {
                let old = Owned::fresh(obj::alloc_typed(&OLD, 0));
                memory.push(obj::NativeObjectLifetime::retain(old.as_ptr()));
                interpreter
                    .var_set_elem(b"::removed", member, old.as_ptr())
                    .unwrap();
                drop(old);
                let mut spelling = b"::removed(".to_vec();
                spelling.extend_from_slice(member);
                spelling.push(b')');
                let name = text(&spelling);
                interpreter
                    .add_native_variable_observer(
                        name.as_ptr(),
                        &[NativeVariableTraceOperation::Unset],
                        Rc::new(Unset),
                    )
                    .unwrap();
            }
            interpreter.set_result_bytes(b"PRIOR");
            let prior = interpreter.get_obj_result();
            let _cleanup = arm(&interpreter, mode, 6);
            assert_eq!(interpreter.var_unset(b"::removed"), mode == 0);
            assert_eq!(
                FREES.with(Cell::get),
                2,
                "mandatory detached generation cleanup"
            );
            assert_eq!(UNSETS.with(Cell::get), if mode == 0 { 2 } else { 0 });
            for old in memory {
                assert!(obj::check_native_liveness(old.as_ptr()).is_err());
            }
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

#[test]
fn original_namespace_variable_cleanup_retains_the_actual_table_through_refusal_and_refill() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        for mode in 0..4 {
            let mut interpreter = native(profile);
            let old = Owned::fresh(obj::alloc_typed(&OLD, 0));
            let memory = obj::NativeObjectLifetime::retain(old.as_ptr());
            setup(&mut interpreter, 5, old.as_ptr());
            drop(old);
            let namespace = interpreter
                .namespaces()
                .find_namespace(GLOBAL, b"retiring")
                .unwrap();
            let original = interpreter
                .namespaces()
                .original_variable_table_at(namespace)
                .unwrap()
                .original_table_ledger();
            interpreter.set_result_bytes(b"PRIOR");
            let prior = interpreter.get_obj_result();
            let _cleanup = arm(&interpreter, mode, 5);
            interpreter.retire_namespace_variables(namespace);
            assert_eq!(FREES.with(Cell::get), 1);
            assert!(
                observed(&interpreter).is_none(),
                "mandatory original binding retirement discards its refill"
            );
            assert!(original.ptr_eq(
                &interpreter
                    .namespaces()
                    .original_variable_table_at(namespace)
                    .unwrap()
                    .original_table_ledger()
            ));
            assert!(obj::check_native_liveness(memory.as_ptr()).is_err());
            assert_eq!(interpreter.get_obj_result(), prior);
            if mode == 1 || mode == 2 {
                cause(&interpreter, mode);
            } else {
                assert!(interpreter.native_execution_refusal().is_none());
            }
            assert_eq!(interpreter.entered_native_operation_count(), 0);
        }
    }
}

#[test]
fn original_indexed_unset_refuses_reborn_frame_before_any_slot_selection() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut interpreter = native(profile);
        interpreter.frames.borrow_mut().push(GLOBAL);
        install_layout(&mut interpreter);
        let old = text(b"OLD FRAME MEMBER");
        interpreter
            .var_set_elem(b"local", b"member", old.as_ptr())
            .unwrap();
        let original = interpreter.frames.borrow().current_activation_owner();
        let element = Owned::fresh(obj::alloc_typed(&ELEMENT, 0));
        interpreter.set_result_bytes(b"PRIOR");
        let prior = interpreter.get_obj_result();
        let _cleanup = arm(&interpreter, 0, 4);
        assert_eq!(
            interpreter.unset_original_c_indexed(0, b"local", Some(element.as_ptr()), true),
            Err(super::Code::Error)
        );
        assert_eq!(READS.with(Cell::get), 1);
        assert_eq!(original.strong_count(), 0);
        let frames = interpreter.frames.borrow();
        let namespaces = interpreter.namespaces();
        let selected = crate::vars::get_elem_at(
            &frames,
            &namespaces,
            b"local",
            b"member",
            frames.current_level(),
        )
        .unwrap();
        assert_eq!(selected, REFILL.with(Cell::get));
        assert_eq!(obj::bytes_of(selected), b"NEW FRAME MEMBER");
        assert_eq!(unsafe { (*old.as_ptr()).ref_count }, 1);
        assert_eq!(unsafe { (*element.as_ptr()).ref_count }, 1);
        drop(namespaces);
        drop(frames);
        assert_eq!(interpreter.get_obj_result(), prior);
        cause(&interpreter, 4);
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    }
}

#[test]
fn original_pending_removal_preserves_unreached_stored_owners() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let mut interpreter = native(profile);
        let old = Owned::fresh(obj::alloc_typed(&OLD, 0));
        let pointer = old.as_ptr();
        setup(&mut interpreter, 0, pointer);
        drop(old);
        let _cleanup = arm(&interpreter, 0, 0);
        interpreter.refuse_host_command("prior original removal refusal");
        let first = interpreter.native_execution_refusal();
        assert!(!interpreter.var_unset(b"::removed"));
        assert_eq!(FREES.with(Cell::get), 0);
        assert_eq!(observed(&interpreter), Some(pointer));
        assert_eq!(unsafe { (*pointer).ref_count }, 1);
        assert_eq!(interpreter.native_execution_refusal(), first);
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    }
}

struct OriginalUnset;
impl NativeVariableObserver<Interp> for OriginalUnset {
    type Error = tcl_cmd_core::CmdError;
    fn observe(
        &self,
        interpreter: &mut Interp,
        access: NativeVariableTraceAccess<'_>,
    ) -> Result<(), Self::Error> {
        assert_eq!(access.operation, NativeVariableTraceOperation::Unset);
        assert_eq!(
            FREES.with(Cell::get),
            0,
            "the actual dummy owner survives its trace"
        );
        let old = TRACE_OLD.with(Cell::get);
        assert!(obj::check_native_liveness(old).is_ok());
        assert_eq!(
            unsafe { (*old).ref_count },
            1,
            "only the transferred stored reference"
        );
        assert!(
            observed(interpreter).is_none(),
            "the selected cell is already undefined"
        );
        UNSETS.with(|count| count.set(count.get() + 1));
        match MODE.with(Cell::get) {
            1 => {
                interpreter.refuse_host_command("first original unset observer");
            }
            2 => change(interpreter),
            _ => {}
        }
        interpreter
            .check_entered_native_operation()
            .map_err(tcl_cmd_core::CmdError::from_execution_refusal)
    }
}
#[test]
fn original_c_unset_keeps_the_transferred_dummy_owner_through_trace_before_retirement() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // The actual C dummy-owner schedule is source inspected; Host/ABA is a software control.
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for route in [0, 5] {
            for mode in 0..3 {
                let mut interpreter = native(profile);
                let old = Owned::fresh(obj::alloc_typed(&OLD, 0));
                let memory = obj::NativeObjectLifetime::retain(old.as_ptr());
                setup(&mut interpreter, route, old.as_ptr());
                drop(old);
                TRACE_OLD.with(|pointer| pointer.set(memory.as_ptr()));
                let name = text(if route == 0 {
                    b"::removed"
                } else {
                    b"::retiring::value"
                });
                interpreter
                    .add_native_variable_observer(
                        name.as_ptr(),
                        &[NativeVariableTraceOperation::Unset],
                        Rc::new(OriginalUnset),
                    )
                    .unwrap();
                interpreter.set_result_bytes(b"PRIOR");
                let prior = interpreter.get_obj_result();
                let _cleanup = arm(&interpreter, mode, route);
                let completed = if route == 0 {
                    interpreter
                        .unset_original_c_variable(name.as_ptr(), true)
                        .is_ok()
                } else {
                    let namespace = interpreter
                        .namespaces()
                        .find_namespace(GLOBAL, b"retiring")
                        .unwrap();
                    interpreter.retire_namespace_variables(namespace);
                    interpreter.native_execution_refusal().is_none()
                };
                assert_eq!(completed, mode == 0);
                assert_eq!(UNSETS.with(Cell::get), 1);
                assert_eq!(
                    FREES.with(Cell::get),
                    1,
                    "mandatory original old owner retirement"
                );
                assert!(obj::check_native_liveness(memory.as_ptr()).is_err());
                assert!(observed(&interpreter).is_none());
                assert_eq!(interpreter.get_obj_result(), prior);
                if mode == 1 {
                    assert!(matches!(interpreter.native_execution_refusal(),
                    Some(NativeExecutionError::HostCommandRefusal(ref first)) if first.reason == "first original unset observer"));
                } else if mode == 2 {
                    cause(&interpreter, mode);
                } else {
                    assert!(interpreter.native_execution_refusal().is_none());
                }
                assert_eq!(interpreter.entered_native_operation_count(), 0);
                TRACE_OLD.with(|pointer| pointer.set(std::ptr::null_mut()));
            }
        }
    }
}
