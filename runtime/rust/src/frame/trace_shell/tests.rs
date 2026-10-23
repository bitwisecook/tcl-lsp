// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual trace removal and last-owner retirement under one original operation.
use super::*;
use crate::{
    frame::Var,
    interp::{native_operation_currency::NativeOperationScope, Code, Interp, WeakInterp},
    namespace::GLOBAL,
    obj::{self, Owned, TclObj},
};
use std::cell::Cell;
use tcl_runtime_api::{
    native_variable_trace::{
        NativeVariableObserver, NativeVariableTraceAccess, NativeVariableTraceOperation,
    },
    NativeExecutionError,
};

thread_local! {
    static OWNER: RefCell<Option<WeakInterp>> = const { RefCell::new(None) };
    static MODE: Cell<u8> = const { Cell::new(0) };
    static ROOT: Cell<bool> = const { Cell::new(false) };
    static FREES: Cell<usize> = const { Cell::new(0) };
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
        crate::interp::default_host(),
        crate::environment::profile_for_dialect(profile),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}
fn text(bytes: &[u8]) -> Owned {
    Owned::fresh(obj::new_string_bytes(bytes))
}
fn arm(interpreter: &Interp, mode: u8, root: bool) -> Cleanup {
    OWNER.with(|owner| *owner.borrow_mut() = Some(interpreter.downgrade()));
    MODE.with(|value| value.set(mode));
    ROOT.with(|value| value.set(root));
    FREES.with(|value| value.set(0));
    REACHED.with(|value| value.set(std::ptr::null_mut()));
    Cleanup
}
fn observe(interpreter: &mut Interp) {
    assert!(interpreter.original_variable_storage_borrows_are_available());
    assert!(interpreter
        .frames
        .borrow()
        .table(1)
        .unwrap()
        .contents(if ROOT.with(Cell::get) { b"shell" } else { b"a" })
        .unwrap()
        .try_borrow_mut()
        .is_ok());
    if ROOT.with(Cell::get) {
        assert!(interpreter
            .frames
            .borrow()
            .table(1)
            .unwrap()
            .binding_id(b"shell")
            .is_none());
    } else {
        let frames = interpreter.frames.borrow();
        let table = frames.table(1).unwrap();
        let cell = table.contents(b"a").unwrap().borrow();
        assert!(!cell.element_ids.contains_key(b"unused".as_slice()));
        assert!(
            matches!(cell.var.as_ref(), Some(Var::Array(values)) if values.contains_key(b"k".as_slice()))
        );
    }
    match MODE.with(Cell::get) {
        3 => {
            let value = text(b"CALLBACK REPLACEMENT");
            interpreter
                .var_set(
                    if ROOT.with(Cell::get) {
                        b"shell"
                    } else {
                        b"a(unused)"
                    },
                    value.as_ptr(),
                )
                .unwrap();
        }
        4 => {
            let (_, frame) = interpreter
                .frames
                .borrow_mut()
                .take_frame_for_pop()
                .unwrap();
            let storage = frame.release();
            interpreter.frames.borrow_mut().recycle_jim_storage(storage);
            interpreter.frames.borrow_mut().push(GLOBAL);
        }
        _ => {}
    }
    interpreter.set_result_bytes(b"REACHED TRACE RETIREMENT");
    REACHED.with(|value| value.set(interpreter.get_obj_result()));
    match MODE.with(Cell::get) {
        1 => {
            interpreter.refuse_host_command("first trace-shell retirement");
        }
        2 => {
            let original = interpreter.runtime_context();
            let mut changed = original.clone();
            changed
                .packages
                .push(("trace-shell-aba".into(), "1".into()));
            interpreter.pin_context(&changed).unwrap();
            interpreter.pin_context(&original).unwrap();
        }
        _ => {}
    }
}
extern "C" fn free_key(original: *mut TclObj) {
    assert_eq!(
        unsafe { (*original).ref_count },
        0,
        "only the actual retired table-key role"
    );
    FREES.with(|value| value.set(value.get() + 1));
    let Some(mut interpreter) =
        OWNER.with(|owner| owner.borrow().as_ref().and_then(WeakInterp::upgrade))
    else {
        return;
    };
    observe(&mut interpreter);
}
static KEY: obj::TclObjType = obj::TclObjType {
    name: c"originalTraceShellKey".as_ptr(),
    free_int_rep_proc: Some(free_key),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
fn key(bytes: &[u8]) -> Owned {
    let key = text(bytes);
    obj::change_type(key.as_ptr(), &KEY, 0);
    key
}
fn assert_first(interpreter: &Interp, mode: u8) {
    let first = interpreter
        .native_execution_refusal()
        .expect("original full cause");
    if mode == 1 {
        assert!(
            matches!(first, NativeExecutionError::HostCommandRefusal(first) if first.reason == "first trace-shell retirement")
        );
    } else {
        assert!(matches!(first, NativeExecutionError::ValueAccessRefusal(_)));
    }
}

#[test]
fn original_trace_member_key_removal_retires_outside_storage_and_checks_the_same_frame_and_member()
{
    // naming.embedding.original-host-publication-and-fact-transport

    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for mode in 0..=4 {
            let mut interpreter = native(profile);
            interpreter.frames.borrow_mut().push(GLOBAL);
            let value = text(b"V");
            interpreter.var_set(b"a(k)", value.as_ptr()).unwrap();
            assert_eq!(
                interpreter.eval_str(b"trace add variable a(unused) unset {}"),
                Code::Ok
            );
            let original = key(b"unused");
            let allocation = obj::NativeObjectLifetime::retain(original.as_ptr());
            interpreter
                .frames
                .borrow_mut()
                .table_mut(1)
                .unwrap()
                .prepare_original_native_element(b"a", b"unused", Some(original.as_ptr()))
                .unwrap();
            assert_eq!(unsafe { (*original.as_ptr()).ref_count }, 2);
            let raw = original.as_ptr();
            drop(original);
            let _cleanup = arm(&interpreter, mode, false);
            let code = interpreter.eval_str(b"trace remove variable a(unused) unset {}");
            assert_eq!(FREES.with(Cell::get), 1);
            assert_eq!(unsafe { (*raw).ref_count }, 0);
            assert_eq!(code, if mode == 0 { Code::Ok } else { Code::Error });
            assert_eq!(interpreter.entered_native_operation_count(), 0);
            if mode == 0 {
                assert!(interpreter.native_execution_refusal().is_none());
            } else {
                assert_first(&interpreter, mode);
                assert_eq!(interpreter.get_obj_result(), REACHED.with(Cell::get));
            }
            if mode == 3 {
                let frames = interpreter.frames.borrow();
                let current = frames.table(1).unwrap().contents(b"a").unwrap().borrow();
                assert!(
                    matches!(current.var.as_ref(), Some(Var::Array(values)) if values.contains_key(b"unused".as_slice()))
                );
            }
            drop(allocation);
        }
    }
}

#[test]
fn original_trace_root_key_withdrawal_preserves_a_callback_reborn_cell_and_first_refusal() {
    // naming.embedding.original-host-publication-and-fact-transport

    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for mode in 0..=3 {
            let mut interpreter = native(profile);
            interpreter.frames.borrow_mut().push(GLOBAL);
            assert_eq!(
                interpreter.eval_str(b"trace add variable shell unset {}"),
                Code::Ok
            );
            let original = key(b"shell");
            let raw = original.as_ptr();
            let allocation = obj::NativeObjectLifetime::retain(raw);
            interpreter
                .frames
                .borrow_mut()
                .table_mut(1)
                .unwrap()
                .retain_native_key(b"shell", raw, false);
            assert_eq!(unsafe { (*raw).ref_count }, 2);
            drop(original);
            let _cleanup = arm(&interpreter, mode, true);
            let code = interpreter.eval_str(b"trace remove variable shell unset {}");
            assert_eq!(FREES.with(Cell::get), 1);
            assert_eq!(unsafe { (*raw).ref_count }, 0);
            assert_eq!(code, if mode == 0 { Code::Ok } else { Code::Error });
            if mode == 0 {
                assert!(interpreter.native_execution_refusal().is_none());
            } else {
                assert_first(&interpreter, mode);
                assert_eq!(interpreter.get_obj_result(), REACHED.with(Cell::get));
            }
            if mode == 3 {
                assert!(interpreter
                    .frames
                    .borrow()
                    .table(1)
                    .unwrap()
                    .binding_id(b"shell")
                    .is_some());
                assert!(matches!(
                    interpreter
                        .frames
                        .borrow()
                        .table(1)
                        .unwrap()
                        .get(b"shell")
                        .as_deref(),
                    Some(Var::Scalar(_))
                ));
            }
            drop(allocation);
        }
    }
}

struct Observer {
    owner: WeakInterp,
    mode: u8,
    drops: Rc<Cell<usize>>,
}
impl NativeVariableObserver<Interp> for Observer {
    type Error = tcl_cmd_core::CmdError;
    fn observe(&self, _: &mut Interp, _: NativeVariableTraceAccess<'_>) -> Result<(), Self::Error> {
        Ok(())
    }
}
impl Drop for Observer {
    fn drop(&mut self) {
        self.drops.set(self.drops.get() + 1);
        let mut interpreter = self.owner.upgrade().unwrap();
        assert!(interpreter.original_variable_storage_borrows_are_available());
        assert!(interpreter.traces.try_borrow_mut().is_ok());
        assert_eq!(interpreter.entered_native_operation_count(), 1);
        interpreter.set_result_bytes(b"REACHED OBSERVER DROP");
        if self.mode == 1 {
            interpreter.refuse_host_command("first observer retirement");
        }
        if self.mode == 2 {
            let original = interpreter.runtime_context();
            let mut changed = original.clone();
            changed
                .packages
                .push(("observer-drop-aba".into(), "1".into()));
            interpreter.pin_context(&changed).unwrap();
            interpreter.pin_context(&original).unwrap();
        }
    }
}

#[test]
fn original_native_observer_removal_keeps_saved_cleanup_entered_through_actual_callback_drop() {
    // naming.embedding.original-host-publication-and-fact-transport

    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for mode in 0..=3 {
            let mut interpreter = native(profile);
            let name = text(b"observer");
            let drops = Rc::new(Cell::new(0));
            let token = interpreter
                .add_native_variable_observer(
                    name.as_ptr(),
                    &[NativeVariableTraceOperation::Unset],
                    Rc::new(Observer {
                        owner: interpreter.downgrade(),
                        mode,
                        drops: drops.clone(),
                    }),
                )
                .unwrap();
            if mode == 3 {
                interpreter.refuse_host_command("prior mandatory observer cleanup");
            }
            let prior = interpreter.native_execution_refusal();
            assert!(interpreter.remove_native_variable_observer(&token));
            assert_eq!(drops.get(), 1);
            assert_eq!(unsafe { (*name.as_ptr()).ref_count }, 1);
            assert_eq!(interpreter.entered_native_operation_count(), 0);
            assert!(!interpreter.remove_native_variable_observer(&token));
            if mode == 0 {
                assert!(interpreter.native_execution_refusal().is_none());
            } else if mode == 3 {
                assert_eq!(interpreter.native_execution_refusal(), prior);
            } else {
                assert!(interpreter.native_execution_refusal().is_some());
            }
            assert_eq!(
                obj::bytes_of(interpreter.get_obj_result()),
                b"REACHED OBSERVER DROP"
            );
        }
    }
}

#[test]
fn original_trace_home_metadata_cannot_withdraw_a_reborn_binding_under_the_same_spelling() {
    // naming.embedding.original-host-publication-and-fact-transport

    let mut interpreter = native("tcl9.1");
    assert_eq!(
        interpreter.eval_str(b"trace add variable shell unset {}"),
        Code::Ok
    );
    let home = interpreter.trace_identity(b"shell");
    let operation = NativeOperationScope::enter(&interpreter).unwrap();
    let (_, retired) = interpreter
        .namespaces_mut()
        .var_table_mut(GLOBAL)
        .publish_remove_after_default(b"shell");
    retired
        .finish(|| operation.currency().ensure_current_or_refuse())
        .unwrap();
    let replacement = text(b"REPLACEMENT");
    interpreter.var_set(b"shell", replacement.as_ptr()).unwrap();
    let original_binding = interpreter.trace_identity(b"shell").binding_id;
    assert_ne!(home.binding_id, original_binding);
    interpreter
        .cleanup_trace_shell(&home, None, || {
            operation
                .currency()
                .ensure_current_or_refuse()
                .map_err(tcl_cmd_core::CmdError::from_execution_refusal)
        })
        .unwrap();
    assert_eq!(interpreter.var_get(b"shell"), Some(replacement.as_ptr()));
    assert_eq!(
        interpreter.trace_identity(b"shell").binding_id,
        original_binding
    );
    assert_eq!(unsafe { (*replacement.as_ptr()).ref_count }, 2);
}
