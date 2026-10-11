// SPDX-License-Identifier: AGPL-3.0-or-later
//! Software callbacks on the genuine Jim stored pointer and incoming roles.
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
    static GETTERS: Count<usize> = const { Count::new(0) };
    static OLD_PTR: Count<*mut TclObj> = const { Count::new(std::ptr::null_mut()) };
    static INCOMING: Count<*mut TclObj> = const { Count::new(std::ptr::null_mut()) };
    static REACHED: Count<*mut TclObj> = const { Count::new(std::ptr::null_mut()) };
    static OLD_ROLE: RefCell<Option<Weak<OriginalJimLinkTarget>>> = const { RefCell::new(None) };
}
struct Cleanup;
impl Drop for Cleanup {
    fn drop(&mut self) {
        OWNER.with(|owner| owner.borrow_mut().take());
        OLD_ROLE.with(|role| role.borrow_mut().take());
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
fn arm(
    interpreter: &Interp,
    mode: u8,
    old: *mut TclObj,
    incoming: *mut TclObj,
    role: &Rc<OriginalJimLinkTarget>,
) -> Cleanup {
    OWNER.with(|owner| *owner.borrow_mut() = Some(interpreter.downgrade()));
    MODE.with(|selected| selected.set(mode));
    FREES.with(|count| count.set(0));
    GETTERS.with(|count| count.set(0));
    OLD_PTR.with(|selected| selected.set(old));
    INCOMING.with(|selected| selected.set(incoming));
    REACHED.with(|selected| selected.set(std::ptr::null_mut()));
    OLD_ROLE.with(|selected| *selected.borrow_mut() = Some(Rc::downgrade(role)));
    Cleanup
}
extern "C" fn target_string(original: *mut TclObj) {
    GETTERS.with(|count| count.set(count.get() + 1));
    if let Some(interpreter) =
        OWNER.with(|owner| owner.borrow().as_ref().and_then(WeakInterp::upgrade))
    {
        assert!(interpreter.original_variable_storage_borrows_are_available());
        let frames = interpreter.frames.borrow();
        let table = frames.table(1).unwrap();
        let cell = table
            .get(b"alias")
            .expect("detached genuine scalar remains visible");
        assert!(matches!(&*cell, Var::Scalar(value) if *value == OLD_PTR.with(Count::get)));
        assert_eq!(unsafe { (*OLD_PTR.with(Count::get)).ref_count }, 1);
        assert_eq!(
            unsafe { (*original).ref_count },
            1,
            "before target temporary/incoming roles"
        );
        assert_eq!(FREES.with(Count::get), 0);
        assert!(OLD_ROLE.with(|role| role
            .borrow()
            .as_ref()
            .unwrap()
            .upgrade()
            .unwrap()
            .original_name()
            .is_err()));
    }
    unsafe { obj::set_native_updater_string_rep(original, b"next", false) };
}
extern "C" fn retire_old(original: *mut TclObj) {
    let Some(mut interpreter) =
        OWNER.with(|owner| owner.borrow().as_ref().and_then(WeakInterp::upgrade))
    else {
        return;
    };
    FREES.with(|count| count.set(count.get() + 1));
    assert_eq!(original, OLD_PTR.with(Count::get));
    assert_eq!(unsafe { (*original).ref_count }, 0);
    assert_eq!(
        unsafe { (*INCOMING.with(Count::get)).ref_count },
        3,
        "caller, target temporary and genuine incoming cell role"
    );
    assert!(interpreter.original_variable_storage_borrows_are_available());
    {
        let frames = interpreter.frames.borrow();
        let table = frames.table(1).unwrap();
        assert_eq!(table.published_scalar_header(b"alias"), Some(original));
        assert!(
            table.cell(b"alias").is_none(),
            "retiring pointer supplies no semantic read role"
        );
        let weak = table.weak_jim_cell(b"alias").unwrap();
        assert!(matches!(
            weak.read_checked(),
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
                .publish_scalar(b"alias", replacement.as_ptr())
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
    interpreter.set_result_bytes(b"REACHED JIM OLD RETIREMENT");
    REACHED.with(|value| value.set(interpreter.get_obj_result()));
    match MODE.with(Count::get) {
        1 => {
            interpreter.refuse_host_command("first Jim old scalar retirement");
        }
        2 => {
            let original_context = interpreter.runtime_context();
            let mut changed = original_context.clone();
            changed
                .packages
                .push(("jim-link-retirement-aba".into(), "1".into()));
            interpreter.pin_context(&changed).unwrap();
            interpreter.pin_context(&original_context).unwrap();
        }
        _ => {}
    }
}
static OLD: obj::TclObjType = obj::TclObjType {
    name: c"originalJimLinkOldScalar".as_ptr(),
    free_int_rep_proc: Some(retire_old),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
static NEXT: obj::TclObjType = obj::TclObjType {
    name: c"originalJimLinkNextName".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(target_string),
    set_from_any_proc: None,
};
fn role(interpreter: &Interp, level: usize, name: &[u8]) -> Rc<OriginalJimLinkTarget> {
    let frames = interpreter.frames.borrow();
    let cell = frames.table(level).unwrap().get(name).unwrap();
    let Var::Link(link) = &*cell else {
        panic!("actual Jim link")
    };
    Rc::clone(link.original_jim_target.as_ref().unwrap())
}
#[test]
fn original_jim_link_detaches_before_getter_and_retires_visible_old_pointer_before_assignment() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Genuine stored role callback; pinned Jim source order and software refusal are separate claims.
    for mode in [0, 1, 2, 3, 4] {
        let mut interpreter = native();
        interpreter.frames.borrow_mut().push(GLOBAL);
        let local = text(b"alias");
        let old = Owned::fresh(obj::alloc_typed(&OLD, 0));
        unsafe { obj::set_native_updater_string_rep(old.as_ptr(), b"old", false) };
        assert_eq!(
            interpreter.link_original_jim_upvar_objects(old.as_ptr(), 0, local.as_ptr()),
            Code::Ok
        );
        let retained = role(&interpreter, 1, b"alias");
        assert_eq!(
            unsafe { (*old.as_ptr()).ref_count },
            2,
            "caller and one stored target name; Rc metadata adds no +1"
        );
        let old_allocation = obj::NativeObjectLifetime::retain(old.as_ptr());
        drop(old);
        let next = Owned::fresh(obj::alloc_typed(&NEXT, 0));
        let _cleanup = arm(
            &interpreter,
            mode,
            old_allocation.as_ptr(),
            next.as_ptr(),
            &retained,
        );
        assert_eq!(
            interpreter.link_original_jim_upvar_objects(next.as_ptr(), 0, local.as_ptr())
                == Code::Ok,
            mode == 0
        );
        assert_eq!(GETTERS.with(Count::get), 1);
        assert_eq!(FREES.with(Count::get), 1);
        assert!(obj::check_native_liveness(old_allocation.as_ptr()).is_err());
        assert!(retained.original_name().is_err());
        assert_eq!(interpreter.get_obj_result(), REACHED.with(Count::get));
        assert_eq!(interpreter.entered_native_operation_count(), 0);
        assert_eq!(
            unsafe { (*next.as_ptr()).ref_count },
            if mode == 0 { 2 } else { 1 }
        );
        let frames = interpreter.frames.borrow();
        let current = frames.table(1).unwrap().get(b"alias");
        if mode == 0 {
            assert!(
                matches!(current.as_deref(), Some(Var::Link(link)) if link.original_jim_target.as_ref().unwrap().original_name().unwrap() == next.as_ptr())
            );
            assert!(interpreter.native_execution_refusal().is_none());
        } else {
            let first = interpreter.native_execution_refusal().unwrap();
            match mode {
                1 => assert!(
                    matches!(first, NativeExecutionError::HostCommandRefusal(first) if first.reason == "first Jim old scalar retirement")
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
fn original_jim_same_target_rebinding_uses_only_existing_temporary_and_cell_references() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut interpreter = native();
    interpreter.frames.borrow_mut().push(GLOBAL);
    let local = text(b"alias");
    let target = text(b"target");
    assert_eq!(
        interpreter.link_original_jim_upvar_objects(target.as_ptr(), 0, local.as_ptr()),
        Code::Ok
    );
    let previous = role(&interpreter, 1, b"alias");
    assert_eq!(unsafe { (*target.as_ptr()).ref_count }, 2);
    assert_eq!(
        interpreter.link_original_jim_upvar_objects(target.as_ptr(), 0, local.as_ptr()),
        Code::Ok
    );
    assert!(previous.original_name().is_err());
    assert_eq!(
        role(&interpreter, 1, b"alias").original_name().unwrap(),
        target.as_ptr()
    );
    assert_eq!(unsafe { (*target.as_ptr()).ref_count }, 2);
    assert_eq!(
        unsafe { (*local.as_ptr()).ref_count },
        2,
        "caller plus original hash key, no key temporary"
    );
    assert!(interpreter
        .frames
        .borrow()
        .table(1)
        .unwrap()
        .original_native_key_is(b"alias", local.as_ptr()));
    assert!(interpreter.native_execution_refusal().is_none());
}
#[test]
fn original_jim_qualified_missing_link_keeps_stripped_fresh_name_role_and_same_local_cell() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut interpreter = native();
    interpreter.frames.borrow_mut().push(GLOBAL);
    let local = text(b"::alias");
    let target = text(b"::next");
    assert_eq!(
        interpreter.link_original_jim_upvar_objects(target.as_ptr(), 0, local.as_ptr()),
        Code::Ok
    );
    let actual = {
        let namespaces = interpreter.namespaces();
        let cell = namespaces.var_table(GLOBAL).get(b"alias").unwrap();
        let Var::Link(link) = &*cell else {
            panic!("actual global Jim alias")
        };
        Rc::clone(link.original_jim_target.as_ref().unwrap())
    };
    let stored = actual.original_name().unwrap();
    assert_ne!(stored, target.as_ptr());
    assert_eq!(obj::bytes_of(stored), b"next");
    assert_eq!(
        unsafe { (*stored).ref_count },
        1,
        "only genuine stored role after temporary retirement"
    );
    assert_eq!(unsafe { (*target.as_ptr()).ref_count }, 1);
    assert_eq!(unsafe { (*local.as_ptr()).ref_count }, 1);
    assert!(!interpreter
        .namespaces()
        .var_table(GLOBAL)
        .original_native_key_is(b"alias", local.as_ptr()));
    assert!(obj::with_jim_variable_cache(local.as_ptr(), |cache| cache.global).unwrap());
    assert!(interpreter.native_execution_refusal().is_none());
}

#[test]
fn original_jim_static_link_rebinding_keeps_shared_cell_without_new_local_table_entry() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut interpreter = native();
    interpreter.frames.borrow_mut().push(GLOBAL);
    let first_local = text(b"alias");
    let first_target = text(b"first");
    assert_eq!(
        interpreter.link_original_jim_upvar_objects(first_target.as_ptr(), 0, first_local.as_ptr()),
        Code::Ok
    );
    let previous = role(&interpreter, 1, b"alias");
    let retained_cell = interpreter
        .frames
        .borrow()
        .table(1)
        .unwrap()
        .capture_cell(b"alias")
        .unwrap();
    let original_cell = Rc::downgrade(&retained_cell.0);
    let mut statics = super::super::StaticVariables::default();
    statics.insert_capture(b"alias", retained_cell);
    interpreter.frames.borrow_mut().push(GLOBAL);
    interpreter
        .frames
        .borrow_mut()
        .table_mut(2)
        .unwrap()
        .install_statics(Rc::new(statics));
    let local = text(b"alias");
    let target = text(b"next");
    {
        let frames = interpreter.frames.borrow();
        let table = frames.table(2).unwrap();
        assert!(!table.has_jim_primary(b"alias"));
        assert!(table.entry_order.borrow().physical_keys().is_empty());
        assert!(Weak::ptr_eq(
            &original_cell,
            &Rc::downgrade(table.contents(b"alias").unwrap())
        ));
    }
    assert_eq!(
        interpreter.link_original_jim_upvar_objects(target.as_ptr(), 0, local.as_ptr()),
        Code::Ok
    );
    assert!(previous.original_name().is_err());
    assert_eq!(
        role(&interpreter, 2, b"alias").original_name().unwrap(),
        target.as_ptr()
    );
    let frames = interpreter.frames.borrow();
    let table = frames.table(2).unwrap();
    assert!(!table.has_jim_primary(b"alias"));
    assert!(table.entry_order.borrow().physical_keys().is_empty());
    assert!(Weak::ptr_eq(
        &original_cell,
        &Rc::downgrade(table.contents(b"alias").unwrap())
    ));
    assert_eq!(
        role(&interpreter, 1, b"alias").original_name().unwrap(),
        target.as_ptr()
    );
    assert_eq!(unsafe { (*first_target.as_ptr()).ref_count }, 1);
    assert_eq!(unsafe { (*target.as_ptr()).ref_count }, 2);
    assert_eq!(
        unsafe { (*local.as_ptr()).ref_count },
        1,
        "existing static binding acquires no new hash key role"
    );
    assert!(interpreter.native_execution_refusal().is_none());
}
