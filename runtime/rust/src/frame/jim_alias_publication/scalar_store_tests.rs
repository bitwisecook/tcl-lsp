// SPDX-License-Identifier: AGPL-3.0-or-later
//! Software callbacks on genuine cached Jim scalar and header primaries.
use super::*;
use crate::{
    interp::{Code, Interp, WeakInterp},
    namespace::GLOBAL,
    obj,
};
use std::cell::Cell as Count;
use tcl_runtime_api::NativeExecutionError;
use tcl_syntax::raw_string::NativeValueAccessRefusal;
thread_local! {
    static OWNER: RefCell<Option<WeakInterp>> = const { RefCell::new(None) };
    static MODE: Count<u8> = const { Count::new(0) };
    static FREES: Count<usize> = const { Count::new(0) };
    static OLD_PTR: Count<*mut TclObj> = const { Count::new(std::ptr::null_mut()) };
    static INCOMING: Count<*mut TclObj> = const { Count::new(std::ptr::null_mut()) };
    static REACHED: Count<*mut TclObj> = const { Count::new(std::ptr::null_mut()) };
}
struct Cleanup;
impl Drop for Cleanup {
    fn drop(&mut self) {
        OWNER.with(|owner| owner.borrow_mut().take());
    }
}
fn native() -> Interp {
    Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect("jim"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}
fn text(bytes: &[u8]) -> Owned {
    Owned::fresh(obj::new_string_bytes(bytes))
}
fn arm(interpreter: &Interp, mode: u8, old: *mut TclObj, incoming: *mut TclObj) -> Cleanup {
    OWNER.with(|owner| *owner.borrow_mut() = Some(interpreter.downgrade()));
    MODE.with(|value| value.set(mode));
    FREES.with(|value| value.set(0));
    OLD_PTR.with(|value| value.set(old));
    INCOMING.with(|value| value.set(incoming));
    REACHED.with(|value| value.set(std::ptr::null_mut()));
    Cleanup
}
fn finish_observer(interpreter: &mut Interp) {
    interpreter.set_result_bytes(b"REACHED JIM SCALAR/CACHE RETIREMENT");
    REACHED.with(|value| value.set(interpreter.get_obj_result()));
    match MODE.with(Count::get) {
        1 => {
            interpreter.refuse_host_command("first Jim scalar/cache retirement");
        }
        2 => {
            let original = interpreter.runtime_context();
            let mut other = original.clone();
            other
                .packages
                .push(("jim-scalar-cache-aba".into(), "1".into()));
            interpreter.pin_context(&other).unwrap();
            interpreter.pin_context(&original).unwrap();
        }
        _ => {}
    }
}
fn assert_first(interpreter: &Interp, mode: u8) {
    let first = interpreter.native_execution_refusal().unwrap();
    match mode {
        1 => assert!(
            matches!(first, NativeExecutionError::HostCommandRefusal(first) if first.reason == "first Jim scalar/cache retirement")
        ),
        2 => assert_eq!(
            first,
            NativeExecutionError::ValueAccessRefusal(
                NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "entered native operation configuration changed"
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
extern "C" fn scalar_free(original: *mut TclObj) {
    let Some(mut interpreter) =
        OWNER.with(|owner| owner.borrow().as_ref().and_then(WeakInterp::upgrade))
    else {
        return;
    };
    FREES.with(|value| value.set(value.get() + 1));
    assert_eq!(original, OLD_PTR.with(Count::get));
    assert_eq!(unsafe { (*original).ref_count }, 0);
    assert_eq!(
        unsafe { (*INCOMING.with(Count::get)).ref_count },
        2,
        "caller and actual incoming cell reference only"
    );
    assert!(interpreter.original_variable_storage_borrows_are_available());
    {
        let frames = interpreter.frames.borrow();
        let table = frames.table(1).unwrap();
        assert_eq!(table.published_scalar_header(b"scalar"), Some(original));
        assert!(table.cell(b"scalar").is_none());
        assert!(matches!(
            table.weak_jim_cell(b"scalar").unwrap().read_checked(),
            Err(ValueError::CommandProtocolUnavailable(
                "retiring Jim scalar stored role"
            ))
        ));
    }
    match MODE.with(Count::get) {
        3 => {
            let replacement = text(b"REPLACEMENT");
            let retired = interpreter
                .frames
                .borrow_mut()
                .table_mut(1)
                .unwrap()
                .publish_scalar(b"scalar", replacement.as_ptr())
                .unwrap();
            drop(retired);
        }
        4 => {
            let (_, owners) = interpreter
                .frames
                .borrow_mut()
                .take_frame_for_pop()
                .unwrap();
            let storage = owners.release();
            interpreter.frames.borrow_mut().recycle_jim_storage(storage);
            interpreter.frames.borrow_mut().push(GLOBAL);
        }
        _ => {}
    }
    finish_observer(&mut interpreter);
}
static OLD_SCALAR: obj::TclObjType = obj::TclObjType {
    name: c"originalJimCachedScalar".as_ptr(),
    free_int_rep_proc: Some(scalar_free),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
extern "C" fn name_free(original: *mut TclObj) {
    let Some(mut interpreter) =
        OWNER.with(|owner| owner.borrow().as_ref().and_then(WeakInterp::upgrade))
    else {
        return;
    };
    FREES.with(|value| value.set(value.get() + 1));
    assert_eq!(original, OLD_PTR.with(Count::get));
    assert_eq!(
        unsafe { (*original).ref_count },
        1,
        "allocation observation adds no native reference"
    );
    assert_eq!(
        unsafe { (*INCOMING.with(Count::get)).ref_count },
        1,
        "before genuine scalar cell acquisition"
    );
    assert!(interpreter.original_variable_storage_borrows_are_available());
    if MODE.with(Count::get) == 3 {
        // The extension settles its freed primary, then publishes through
        // the existing canonical type worker, including its real epoch.
        unsafe {
            (*original).type_ptr = std::ptr::null();
            (*original).internal_rep = (*original).length as u64 + 1;
        }
        obj::change_type(original, &PUBLISHED_NAME, 77);
    }
    finish_observer(&mut interpreter);
}
static OLD_NAME: obj::TclObjType = obj::TclObjType {
    name: c"originalJimVariableNamePrimary".as_ptr(),
    free_int_rep_proc: Some(name_free),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
static PUBLISHED_NAME: obj::TclObjType = obj::TclObjType {
    name: c"callbackPublishedJimVariableName".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};

#[test]
fn original_jim_cached_scalar_store_retires_visible_old_role_before_assignment_and_checks_original_cell(
) {
    // naming.embedding.original-host-publication-and-fact-transport

    for mode in [0, 1, 2, 3, 4] {
        let mut interpreter = native();
        interpreter.frames.borrow_mut().push(GLOBAL);
        let old = Owned::fresh(obj::alloc_typed(&OLD_SCALAR, 0));
        unsafe { obj::set_native_updater_string_rep(old.as_ptr(), b"old", false) };
        interpreter.var_set(b"scalar", old.as_ptr()).unwrap();
        let name = text(b"scalar");
        assert_eq!(
            interpreter
                .read_original_named_variable(name.as_ptr())
                .unwrap(),
            old.as_ptr()
        );
        let original_cell =
            obj::with_jim_variable_cache(name.as_ptr(), |cache| cache.cell.clone()).unwrap();
        let old_allocation = obj::NativeObjectLifetime::retain(old.as_ptr());
        drop(old);
        let incoming = text(b"incoming");
        let _cleanup = arm(
            &interpreter,
            mode,
            old_allocation.as_ptr(),
            incoming.as_ptr(),
        );
        let stored = interpreter.assign_original_named_variable(name.as_ptr(), incoming.as_ptr());
        assert_eq!(stored.is_ok(), mode == 0);
        assert_eq!(FREES.with(Count::get), 1);
        assert!(obj::check_native_liveness(old_allocation.as_ptr()).is_err());
        assert_eq!(
            unsafe { (*incoming.as_ptr()).ref_count },
            if mode == 0 { 2 } else { 1 }
        );
        assert_eq!(interpreter.get_obj_result(), REACHED.with(Count::get));
        assert_eq!(interpreter.entered_native_operation_count(), 0);
        if mode == 0 {
            assert!(
                matches!(original_cell.read_checked().unwrap(), Some(super::super::JimVariableRead::Scalar(value)) if value == incoming.as_ptr())
            );
            assert!(interpreter.native_execution_refusal().is_none());
        } else {
            assert_first(&interpreter, mode);
            let frames = interpreter.frames.borrow();
            let current = frames.table(1).unwrap().get(b"scalar");
            if mode == 3 {
                assert!(
                    matches!(current.as_deref(), Some(Var::Scalar(value)) if obj::bytes_of(*value) == b"REPLACEMENT")
                );
            } else {
                assert!(current.is_none());
            }
        }
    }
}
#[test]
fn original_jim_variable_cache_adoption_preserves_callback_primary_and_stops_before_scalar_store() {
    // naming.embedding.original-host-publication-and-fact-transport

    for mode in [0, 1, 2, 3] {
        let mut interpreter = native();
        interpreter.frames.borrow_mut().push(GLOBAL);
        let initial = text(b"initial");
        interpreter.var_set(b"scalar", initial.as_ptr()).unwrap();
        let name = Owned::fresh(obj::alloc_typed(&OLD_NAME, 0));
        unsafe { obj::set_native_updater_string_rep(name.as_ptr(), b"scalar", false) };
        let incoming = text(b"incoming");
        let _cleanup = arm(&interpreter, mode, name.as_ptr(), incoming.as_ptr());
        let stored = interpreter.assign_original_named_variable(name.as_ptr(), incoming.as_ptr());
        assert_eq!(stored.is_ok(), mode == 0);
        assert_eq!(FREES.with(Count::get), 1);
        assert_eq!(interpreter.get_obj_result(), REACHED.with(Count::get));
        assert_eq!(interpreter.entered_native_operation_count(), 0);
        assert_eq!(unsafe { (*name.as_ptr()).ref_count }, 1);
        assert_eq!(
            unsafe { (*incoming.as_ptr()).ref_count },
            if mode == 0 { 2 } else { 1 }
        );
        let frames = interpreter.frames.borrow();
        let current = frames.table(1).unwrap().get(b"scalar").unwrap();
        assert!(
            matches!(&*current, Var::Scalar(value) if *value == if mode == 0 { incoming.as_ptr() } else { initial.as_ptr() })
        );
        if mode == 0 {
            assert!(obj::with_jim_variable_cache(name.as_ptr(), |_| ()).is_some());
            assert!(interpreter.native_execution_refusal().is_none());
        } else {
            assert!(obj::with_jim_variable_cache(name.as_ptr(), |_| ()).is_none());
            assert_first(&interpreter, mode);
            if mode == 3 {
                assert!(std::ptr::eq(
                    obj::obj_type_ptr(name.as_ptr()),
                    &PUBLISHED_NAME
                ));
                assert_eq!(obj::internal_rep(name.as_ptr()), 77);
                assert_eq!(
                    interpreter.native_execution_refusal().unwrap(),
                    NativeExecutionError::ValueAccessRefusal(
                        NativeValueAccessRefusal::CommandProtocolUnavailable(
                            "changed native primary publication slot"
                        )
                    )
                );
            } else {
                assert!(obj::obj_type_ptr(name.as_ptr()).is_null());
            }
        }
    }
}
#[test]
fn original_jim_cached_static_scalar_store_and_link_recursion_keep_original_shared_cell_roles() {
    // naming.embedding.original-host-publication-and-fact-transport

    let mut interpreter = native();
    interpreter.frames.borrow_mut().push(GLOBAL);
    let initial = text(b"initial");
    interpreter.var_set(b"scalar", initial.as_ptr()).unwrap();
    let retained = interpreter
        .frames
        .borrow()
        .table(1)
        .unwrap()
        .capture_cell(b"scalar")
        .unwrap();
    let original = Rc::downgrade(&retained.0);
    let mut statics = super::super::StaticVariables::default();
    statics.insert_capture(b"scalar", retained);
    interpreter.frames.borrow_mut().push(GLOBAL);
    interpreter
        .frames
        .borrow_mut()
        .table_mut(2)
        .unwrap()
        .install_statics(Rc::new(statics));
    let name = text(b"scalar");
    let incoming = text(b"incoming");
    assert_eq!(
        interpreter
            .read_original_named_variable(name.as_ptr())
            .unwrap(),
        initial.as_ptr()
    );
    assert!(interpreter
        .assign_original_named_variable(name.as_ptr(), incoming.as_ptr())
        .is_ok());
    {
        let frames = interpreter.frames.borrow();
        let table = frames.table(2).unwrap();
        assert!(!table.has_jim_primary(b"scalar"));
        assert!(table.entry_order.borrow().physical_keys().is_empty());
        assert!(Weak::ptr_eq(
            &original,
            &Rc::downgrade(table.contents(b"scalar").unwrap())
        ));
        assert!(
            matches!(frames.table(1).unwrap().get(b"scalar").as_deref(), Some(Var::Scalar(value)) if *value == incoming.as_ptr())
        );
    }
    assert_eq!(unsafe { (*initial.as_ptr()).ref_count }, 1);
    assert_eq!(unsafe { (*incoming.as_ptr()).ref_count }, 2);
    let alias = text(b"alias");
    assert_eq!(
        interpreter.link_original_jim_upvar_objects(name.as_ptr(), 1, alias.as_ptr()),
        Code::Ok
    );
    assert!(interpreter
        .assign_original_named_variable(alias.as_ptr(), incoming.as_ptr())
        .is_ok());
    assert_eq!(
        unsafe { (*incoming.as_ptr()).ref_count },
        2,
        "same-value recursion preserves the single cell role"
    );
    assert!(interpreter.native_execution_refusal().is_none());
}
