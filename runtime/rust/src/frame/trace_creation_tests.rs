// SPDX-License-Identifier: AGPL-3.0-or-later
//! Real retirement/getter controls; no measured Native chronology assertion.
use super::*;
use crate::{
    interp::{native_operation_currency::NativeOperationScope, Code, Interp, WeakInterp},
    namespace::GLOBAL,
    obj::{self, Owned, TclObj},
};
use std::cell::Cell as Counter;
use tcl_runtime_api::{
    native_variable_trace::{
        NativeVariableObserver, NativeVariableTraceAccess, NativeVariableTraceOperation,
    },
    NativeExecutionError,
};
thread_local! {
    static OWNER: RefCell<Option<WeakInterp>> = const { RefCell::new(None) };
    static MODE: Counter<u8> = const { Counter::new(0) };
    static CALLS: Counter<usize> = const { Counter::new(0) };
    static DROPS: Counter<usize> = const { Counter::new(0) };
    static NAMESPACE: Counter<bool> = const { Counter::new(false) };
    static REACHED: Counter<*mut TclObj> = const { Counter::new(std::ptr::null_mut()) };
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
fn arm(interpreter: &Interp, mode: u8) -> Cleanup {
    OWNER.with(|owner| *owner.borrow_mut() = Some(interpreter.downgrade()));
    NAMESPACE.with(|value| value.set(false));
    MODE.with(|value| value.set(mode));
    CALLS.with(|value| value.set(0));
    DROPS.with(|value| value.set(0));
    REACHED.with(|value| value.set(std::ptr::null_mut()));
    Cleanup
}
fn effect(interpreter: &mut Interp) {
    assert!(interpreter.original_variable_storage_borrows_are_available());
    match MODE.with(Counter::get) {
        3 => {
            let (_, frame) = interpreter
                .frames
                .borrow_mut()
                .take_frame_for_pop()
                .unwrap();
            let storage = frame.release();
            interpreter.frames.borrow_mut().recycle_jim_storage(storage);
            interpreter.frames.borrow_mut().push(GLOBAL);
        }
        4 => {
            assert_eq!(interpreter.eval_str(b"unset a"), Code::Ok);
            let value = text(b"REBORN");
            interpreter.var_set(b"a(k)", value.as_ptr()).unwrap();
        }
        5 => {
            assert_eq!(interpreter.eval_str(b"upvar #0 ::b alias"), Code::Ok);
        }
        _ => {}
    }
    interpreter.set_result_bytes(b"REACHED TRACE CREATION");
    REACHED.with(|value| value.set(interpreter.get_obj_result()));
    match MODE.with(Counter::get) {
        1 => {
            interpreter.refuse_host_command("first trace-creation observer");
        }
        2 => {
            let original = interpreter.runtime_context();
            let mut changed = original.clone();
            changed
                .packages
                .push(("trace-create-aba".into(), "1".into()));
            interpreter.pin_context(&changed).unwrap();
            interpreter.pin_context(&original).unwrap();
        }
        _ => {}
    }
}
extern "C" fn free_search(original: *mut TclObj) {
    assert_eq!(
        unsafe { (*original).ref_count },
        0,
        "actual search-chain last stored role"
    );
    CALLS.with(|value| value.set(value.get() + 1));
    let Some(mut interpreter) =
        OWNER.with(|owner| owner.borrow().as_ref().and_then(WeakInterp::upgrade))
    else {
        return;
    };
    {
        let frames = interpreter.frames.borrow();
        let namespaces = interpreter.namespaces();
        let table = if NAMESPACE.with(Counter::get) {
            namespaces.original_variable_table_at(GLOBAL).unwrap()
        } else {
            frames.original_variable_table_at(1).unwrap()
        };
        let cell = table.contents(b"a").unwrap();
        assert!(cell.try_borrow_mut().is_ok());
        let cell = cell.borrow();
        assert!(cell.element_ids.contains_key(b"new".as_slice()));
        assert!(
            matches!(cell.var.as_ref(), Some(Var::Array(values)) if !values.contains_key(b"new".as_slice()))
        );
    }
    effect(&mut interpreter);
}
extern "C" fn update_name(original: *mut TclObj) {
    CALLS.with(|value| value.set(value.get() + 1));
    let Some(mut interpreter) =
        OWNER.with(|owner| owner.borrow().as_ref().and_then(WeakInterp::upgrade))
    else {
        return;
    };
    assert!(interpreter.check_entered_native_operation().is_ok());
    effect(&mut interpreter);
    unsafe { obj::set_native_updater_string_rep(original, b"fresh", false) };
}
static SEARCH: obj::TclObjType = obj::TclObjType {
    name: c"originalTraceCreationSearch".as_ptr(),
    free_int_rep_proc: Some(free_search),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
static NAME: obj::TclObjType = obj::TclObjType {
    name: c"originalTraceCreationName".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(update_name),
    set_from_any_proc: None,
};
fn first(interpreter: &Interp, mode: u8) {
    let cause = interpreter
        .native_execution_refusal()
        .expect("full original cause");
    if mode == 1 {
        assert!(
            matches!(cause, NativeExecutionError::HostCommandRefusal(first) if first.reason == "first trace-creation observer")
        );
    } else {
        assert!(matches!(cause, NativeExecutionError::ValueAccessRefusal(_)));
    }
}
fn search(interpreter: &Interp, namespace: bool) -> obj::NativeObjectLifetime {
    let handle = Owned::fresh(obj::alloc_typed(&SEARCH, 0));
    let allocation = obj::NativeObjectLifetime::retain(handle.as_ptr());
    if namespace {
        let mut namespaces = interpreter.namespaces_mut();
        let table = namespaces.original_variable_table_at_mut(GLOBAL).unwrap();
        table
            .contents(b"a")
            .unwrap()
            .borrow_mut()
            .array_searches
            .insert(1, Some(Owned::retain(handle.as_ptr())), vec![b"k".to_vec()]);
    } else {
        let mut frames = interpreter.frames.borrow_mut();
        let table = frames.original_variable_table_at_mut(1).unwrap();
        table
            .contents(b"a")
            .unwrap()
            .borrow_mut()
            .array_searches
            .insert(1, Some(Owned::retain(handle.as_ptr())), vec![b"k".to_vec()]);
    }
    drop(handle);
    allocation
}
#[test]
fn original_trace_member_creation_retires_searches_outside_storage_before_registration() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for mode in 0..=4 {
            let mut interpreter = native(profile);
            interpreter.frames.borrow_mut().push(GLOBAL);
            let value = text(b"V");
            interpreter.var_set(b"a(k)", value.as_ptr()).unwrap();
            let allocation = search(&interpreter, false);
            let _cleanup = arm(&interpreter, mode);
            let code = interpreter.eval_str(b"trace add variable a(new) write {}");
            assert_eq!(
                code,
                if mode == 0 { Code::Ok } else { Code::Error },
                "{profile}/{mode}"
            );
            assert_eq!(CALLS.with(Counter::get), 1);
            assert_eq!(unsafe { (*allocation.as_ptr()).ref_count }, 0);
            if mode == 0 {
                let home = interpreter.trace_identity(b"a");
                assert!(interpreter
                    .traces
                    .borrow()
                    .traces
                    .iter()
                    .any(|trace| trace.binding_id == home.binding_id
                        && trace.elem.as_deref() == Some(b"new")));
            } else {
                first(&interpreter, mode);
                assert!(interpreter
                    .traces
                    .borrow()
                    .traces
                    .iter()
                    .all(|trace| trace.elem.as_deref() != Some(b"new")));
                assert_eq!(interpreter.get_obj_result(), REACHED.with(Counter::get));
            }
            interpreter.frames.borrow_mut().pop().unwrap();
        }
    }
}
struct Observer;
impl NativeVariableObserver<Interp> for Observer {
    type Error = tcl_cmd_core::CmdError;
    fn observe(&self, _: &mut Interp, _: NativeVariableTraceAccess<'_>) -> Result<(), Self::Error> {
        Ok(())
    }
}
impl Drop for Observer {
    fn drop(&mut self) {
        DROPS.with(|value| value.set(value.get() + 1));
        let Some(interpreter) =
            OWNER.with(|owner| owner.borrow().as_ref().and_then(WeakInterp::upgrade))
        else {
            return;
        };
        assert!(interpreter.original_variable_storage_borrows_are_available());
        assert!(interpreter.check_entered_native_operation().is_err());
    }
}
#[test]
fn original_native_observer_name_getter_keeps_original_scope_through_failed_incoming_cleanup() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for mode in [1, 2] {
            let mut interpreter = native(profile);
            let original = Owned::fresh(obj::alloc_typed(&NAME, 0));
            let _cleanup = arm(&interpreter, mode);
            assert!(interpreter
                .add_native_variable_observer(
                    original.as_ptr(),
                    &[NativeVariableTraceOperation::Write],
                    Rc::new(Observer)
                )
                .is_err());
            assert_eq!(CALLS.with(Counter::get), 1);
            assert_eq!(DROPS.with(Counter::get), 1);
            assert_eq!(unsafe { (*original.as_ptr()).ref_count }, 1);
            assert!(interpreter
                .traces
                .borrow()
                .traces
                .iter()
                .all(|trace| trace.base != b"fresh"));
            first(&interpreter, mode);
            assert_eq!(interpreter.get_obj_result(), REACHED.with(Counter::get));
        }
    }
}
#[test]
fn original_trace_root_preparation_keeps_exact_addressed_home_and_undefined_alias_target() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut interpreter = native(profile);
        interpreter.frames.borrow_mut().push(GLOBAL);
        let operation = NativeOperationScope::enter(&interpreter).unwrap();
        let home = interpreter
            .prepare_trace_registration(b"fresh", None, Some(0))
            .unwrap();
        assert_eq!(home.ns, Some(GLOBAL));
        assert_eq!(home.level, None);
        assert!(home.binding_id.is_some());
        assert!(interpreter
            .namespaces()
            .original_variable_table_at(GLOBAL)
            .unwrap()
            .load_scalar(b"fresh")
            .is_none());
        assert_eq!(interpreter.eval_str(b"upvar #0 ::fresh alias"), Code::Ok);
        let alias = interpreter
            .prepare_trace_registration(b"alias", None, None)
            .unwrap();
        assert_eq!(alias.binding_id, home.binding_id);
        assert_eq!(alias.ns, home.ns);
        assert!(interpreter.var_get(b"alias").is_none());
        operation.currency().ensure_current_or_refuse().unwrap();
        drop(operation);
        interpreter.frames.borrow_mut().pop().unwrap();
    }
}

#[test]
fn original_trace_member_creation_refuses_rebound_alias_before_registering_original_or_replacement()
{
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut interpreter = native(profile);
        let value = text(b"V");
        interpreter.var_set(b"::a(k)", value.as_ptr()).unwrap();
        interpreter.var_set(b"::b(k)", value.as_ptr()).unwrap();
        interpreter.frames.borrow_mut().push(GLOBAL);
        assert_eq!(interpreter.eval_str(b"upvar #0 ::a alias"), Code::Ok);
        let allocation = search(&interpreter, true);
        let _cleanup = arm(&interpreter, 5);
        NAMESPACE.with(|value| value.set(true));
        assert_eq!(
            interpreter.eval_str(b"trace add variable alias(new) write {}"),
            Code::Error
        );
        assert_eq!(CALLS.with(Counter::get), 1);
        assert_eq!(unsafe { (*allocation.as_ptr()).ref_count }, 0);
        first(&interpreter, 5);
        assert_eq!(interpreter.get_obj_result(), REACHED.with(Counter::get));
        assert!(interpreter
            .traces
            .borrow()
            .traces
            .iter()
            .all(|trace| trace.elem.as_deref() != Some(b"new")));
        interpreter.frames.borrow_mut().pop().unwrap();
    }
}
