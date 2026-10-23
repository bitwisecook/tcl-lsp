// SPDX-License-Identifier: AGPL-3.0-or-later
//! Reached container conversions use the original operation and storage identities.
use super::{native_operation_currency::NativeOperationScope, Interp, WeakInterp};
use crate::{
    frame::Var,
    namespace::GLOBAL,
    obj::{self, Owned, TclObj},
};
use std::cell::{Cell, RefCell};
use tcl_runtime_api::NativeExecutionError;
use tcl_syntax::{
    native_string::NativeStringProtocol, raw_string::NativeValueAccessRefusal, value::ValueOps,
};

thread_local! {
    static OWNER: RefCell<Option<WeakInterp>> = const { RefCell::new(None) };
    static MODE: Cell<u8> = const { Cell::new(0) };
    static READS: Cell<usize> = const { Cell::new(0) };
    static FREES: Cell<usize> = const { Cell::new(0) };
    static REFERENCES: Cell<obj::TclSize> = const { Cell::new(0) };
    static ROOT: Cell<*mut TclObj> = const { Cell::new(std::ptr::null_mut()) };
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
fn arm(interpreter: &Interp, mode: u8) -> Cleanup {
    OWNER.with(|owner| *owner.borrow_mut() = Some(interpreter.downgrade()));
    MODE.with(|value| value.set(mode));
    READS.with(|value| value.set(0));
    FREES.with(|value| value.set(0));
    REFERENCES.with(|value| value.set(0));
    Cleanup
}
fn effect(original: *mut TclObj) {
    let Some(mut interpreter) =
        OWNER.with(|owner| owner.borrow().as_ref().and_then(WeakInterp::upgrade))
    else {
        return;
    };
    assert!(interpreter.frames.try_borrow_mut().is_ok());
    assert!(interpreter.namespaces.try_borrow_mut().is_ok());
    REFERENCES.with(|value| value.set(unsafe { (*original).ref_count }));
    match MODE.with(Cell::get) {
        1 => {
            interpreter.refuse_host_command("first original container stage");
        }
        2 => {
            let original = interpreter.runtime_context();
            let mut changed = original.clone();
            changed
                .packages
                .push(("container-stage".into(), "1.0".into()));
            interpreter.pin_context(&changed).unwrap();
            interpreter.pin_context(&original).unwrap();
        }
        4 => {
            interpreter.frames.borrow_mut().pop().unwrap();
            interpreter.frames.borrow_mut().push(GLOBAL);
            let replacement = text(b"NEW FRAME");
            interpreter.var_set(b"a", replacement.as_ptr()).unwrap();
        }
        5 | 6 => {
            let root = ROOT.with(Cell::get);
            assert!(interpreter.list_elements(&root).is_ok());
            if MODE.with(Cell::get) == 6 {
                crate::dict::ensure_dict_native(root, NativeStringProtocol::Jim084).unwrap();
            }
        }
        _ => {
            // The existing defined-cell predicate needs no dictionary
            // conversion, including during a partial model transition.
            assert!(interpreter.var_exists(b"a"));
        }
    }
}
extern "C" fn update(original: *mut TclObj) {
    READS.with(|reads| reads.set(reads.get() + 1));
    effect(original);
    unsafe { obj::set_native_updater_string_rep(original, b"k V", false) };
}
extern "C" fn key_update(original: *mut TclObj) {
    READS.with(|reads| reads.set(reads.get() + 1));
    effect(original);
    unsafe { obj::set_native_updater_string_rep(original, b"k", false) };
}
extern "C" fn free(original: *mut TclObj) {
    FREES.with(|frees| frees.set(frees.get() + 1));
    effect(original);
}
static LAZY: obj::TclObjType = obj::TclObjType {
    name: c"originalContainerGetter".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(update),
    set_from_any_proc: None,
};
static PRIMARY: obj::TclObjType = obj::TclObjType {
    name: c"originalContainerPrimary".as_ptr(),
    free_int_rep_proc: Some(free),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
static KEY: obj::TclObjType = obj::TclObjType {
    name: c"originalContainerKey".as_ptr(),
    free_int_rep_proc: Some(free),
    dup_int_rep_proc: None,
    update_string_proc: Some(key_update),
    set_from_any_proc: None,
};
fn cause(interpreter: &Interp, mode: u8) {
    let actual = interpreter
        .native_execution_refusal()
        .expect("original typed refusal");
    match mode {
        1 => assert!(
            matches!(actual, NativeExecutionError::HostCommandRefusal(ref first) if first.reason == "first original container stage")
        ),
        2 => assert_eq!(
            actual,
            NativeExecutionError::ValueAccessRefusal(
                NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "stale entered native operation"
                )
            )
        ),
        _ => assert!(matches!(
            actual,
            NativeExecutionError::ValueAccessRefusal(
                NativeValueAccessRefusal::CommandProtocolUnavailable(_)
            )
        )),
    }
}
fn query(interpreter: &Interp, route: u8) -> bool {
    match route {
        0 => interpreter.var_get_elem(b"a", b"k").is_some(),
        1 => interpreter.var_is_array(b"a"),
        2 => interpreter.array_names(b"a").is_some(),
        3 => interpreter.var_exists(b"a(k)"),
        _ => unreachable!(),
    }
}
#[test]
fn original_container_queries_check_the_reached_getter_before_dictionary_adoption() {
    // naming.embedding.original-host-publication-and-fact-transport

    // Byte-container compatibility is independent of original Jim command COW.
    for route in 0..4 {
        for mode in 0..3 {
            let mut interpreter = native("jim");
            let value = Owned::fresh(obj::alloc_typed(&LAZY, 0));
            interpreter.var_set(b"a", value.as_ptr()).unwrap();
            let memory = obj::NativeObjectLifetime::retain(value.as_ptr());
            drop(value);
            interpreter.set_result_bytes(b"PRIOR");
            let prior = interpreter.get_obj_result();
            let _cleanup = arm(&interpreter, mode);
            let result = query(&interpreter, route);
            assert_eq!(result, mode == 0);
            assert_eq!(READS.with(Cell::get), 1);
            assert_eq!(
                REFERENCES.with(Cell::get),
                1,
                "only the actual cell owns the root"
            );
            assert_eq!(unsafe { (*memory.as_ptr()).ref_count }, 1);
            assert_eq!(obj::bytes_of(memory.as_ptr()), b"k V");
            assert_eq!(interpreter.get_obj_result(), prior);
            assert_eq!(
                std::ptr::eq(
                    obj::obj_type_ptr(memory.as_ptr()),
                    &crate::dict::TCL_DICT_TYPE
                ),
                mode == 0
            );
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
fn original_container_conversion_retires_the_primary_outside_all_storage_borrows() {
    // naming.embedding.original-host-publication-and-fact-transport

    for mode in 0..3 {
        let mut interpreter = native("jim");
        let value = Owned::fresh(obj::alloc_typed(&PRIMARY, 0));
        unsafe { obj::set_string_rep(value.as_ptr(), b"k V") };
        interpreter.var_set(b"a", value.as_ptr()).unwrap();
        let memory = obj::NativeObjectLifetime::retain(value.as_ptr());
        drop(value);
        interpreter.set_result_bytes(b"PRIOR");
        let prior = interpreter.get_obj_result();
        let _cleanup = arm(&interpreter, mode);
        assert_eq!(query(&interpreter, 0), mode == 0);
        assert_eq!(FREES.with(Cell::get), 1);
        assert_eq!(READS.with(Cell::get), 0);
        assert_eq!(REFERENCES.with(Cell::get), 1);
        assert_eq!(unsafe { (*memory.as_ptr()).ref_count }, 1);
        assert_eq!(interpreter.get_obj_result(), prior);
        assert_eq!(
            std::ptr::eq(
                obj::obj_type_ptr(memory.as_ptr()),
                &crate::dict::TCL_DICT_TYPE
            ),
            mode == 0
        );
        if mode == 0 {
            assert!(interpreter.native_execution_refusal().is_none());
        } else {
            cause(&interpreter, mode);
        }
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    }
}
#[test]
fn original_container_frame_observation_cannot_adopt_a_new_frame_at_the_same_level() {
    // naming.embedding.original-host-publication-and-fact-transport

    let mut interpreter = native("jim");
    interpreter.frames.borrow_mut().push(GLOBAL);
    let value = Owned::fresh(obj::alloc_typed(&LAZY, 0));
    interpreter.var_set(b"a", value.as_ptr()).unwrap();
    let original = interpreter.frames.borrow().current_activation_owner();
    let _cleanup = arm(&interpreter, 4);
    assert!(interpreter.var_get_elem(b"a", b"k").is_none());
    assert_eq!(READS.with(Cell::get), 1);
    assert_eq!(REFERENCES.with(Cell::get), 2);
    assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 1);
    assert_eq!(original.strong_count(), 0);
    assert!(!std::rc::Weak::ptr_eq(
        &original,
        &interpreter.frames.borrow().current_activation_owner()
    ));
    let frames = interpreter.frames.borrow();
    let observed = frames
        .original_variable_table_at(frames.current_level())
        .unwrap()
        .load_scalar(b"a")
        .unwrap();
    assert_eq!(obj::bytes_of(observed), b"NEW FRAME");
    drop(frames);
    assert!(!std::ptr::eq(
        obj::obj_type_ptr(value.as_ptr()),
        &crate::dict::TCL_DICT_TYPE
    ));
    cause(&interpreter, 4);
    assert_eq!(interpreter.entered_native_operation_count(), 0);
}
#[test]
fn original_container_queries_keep_existing_reference_roles_across_all_native_profiles() {
    // naming.embedding.original-host-publication-and-fact-transport

    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let mut interpreter = native(profile);
        let value = text(b"V\0TAIL");
        interpreter.var_set(b"scalar", value.as_ptr()).unwrap();
        assert_eq!(interpreter.var_get(b"scalar"), Some(value.as_ptr()));
        assert!(interpreter.var_exists(b"scalar"));
        assert!(!interpreter.var_is_array(b"scalar"));
        assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 2);
        interpreter
            .var_set_elem(b"a", b"k", value.as_ptr())
            .unwrap();
        assert_eq!(interpreter.var_get_elem(b"a", b"k"), Some(value.as_ptr()));
        assert!(interpreter.var_exists(b"a(k)"));
        assert!(interpreter.var_is_array(b"a"));
        assert_eq!(interpreter.array_names(b"a"), Some(vec![b"k".to_vec()]));
        assert_eq!(
            unsafe { (*value.as_ptr()).ref_count },
            3,
            "caller, scalar cell, actual array member"
        );
        let scope = NativeOperationScope::enter(&interpreter).unwrap();
        interpreter
            .clone()
            .refuse_host_command("already entered container cause");
        assert!(interpreter.var_get_elem(b"a", b"k").is_none());
        assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 3);
        assert!(
            matches!(interpreter.native_execution_refusal(), Some(NativeExecutionError::HostCommandRefusal(ref first)) if first.reason == "already entered container cause")
        );
        drop(scope);
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    }
}
#[test]
fn original_container_transition_retires_real_member_keys_before_root_publication() {
    // naming.embedding.original-host-publication-and-fact-transport

    // An explicit physical model transition is software state, not a Native C/Jim observation.
    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for mode in 0..3 {
            let mut interpreter = native(profile);
            let value = text(b"V");
            interpreter
                .var_set_elem(b"a", b"k", value.as_ptr())
                .unwrap();
            let key = Owned::fresh(obj::alloc_typed(&PRIMARY, 0));
            unsafe { obj::set_string_rep(key.as_ptr(), b"k") };
            crate::vars::prepare_original_c_element_cell(
                &mut interpreter.frames.borrow_mut(),
                &mut interpreter.namespaces_mut(),
                crate::vars::NativeOriginalNameScope {
                    current: GLOBAL,
                    namespace: Some(GLOBAL),
                },
                b"a",
                b"k",
                Some(key.as_ptr()),
                true,
            )
            .unwrap();
            let allocation = obj::NativeObjectLifetime::retain(key.as_ptr());
            drop(key);
            interpreter.namespaces_mut().variable_container_model =
                tcl_dialect::VariableContainerModel::DictionaryValue;
            let _cleanup = arm(&interpreter, mode);
            assert_eq!(
                interpreter.var_get_elem(b"a", b"k"),
                (mode == 0).then_some(value.as_ptr())
            );
            assert_eq!(FREES.with(Cell::get), 1);
            assert!(!obj::allocation_is_live(allocation.as_ptr()));
            let namespaces = interpreter.namespaces();
            let root = namespaces
                .original_variable_table_at(GLOBAL)
                .unwrap()
                .cell(b"a")
                .unwrap();
            assert_eq!(matches!(*root, Var::Scalar(_)), mode == 0);
            assert_eq!(matches!(*root, Var::Array(_)), mode != 0);
            drop(root);
            drop(namespaces);
            assert_eq!(
                unsafe { (*value.as_ptr()).ref_count },
                2,
                "caller plus current container member"
            );
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
fn original_container_member_getters_refuse_changed_and_restored_dictionary_backings() {
    // naming.embedding.original-host-publication-and-fact-transport

    for mode in [0, 5, 6] {
        let mut interpreter = native("jim");
        let key = Owned::fresh(obj::alloc_typed(&KEY, 0));
        unsafe { obj::set_string_rep(key.as_ptr(), b"k") };
        let value = text(b"V");
        let root = Owned::fresh(
            crate::dict::new_dict_obj_native(
                &[(key.as_ptr(), value.as_ptr())],
                None,
                NativeStringProtocol::Jim084,
            )
            .unwrap(),
        );
        interpreter.var_set(b"a", root.as_ptr()).unwrap();
        drop(obj::bytes_of(root.as_ptr()));
        obj::invalidate_string(key.as_ptr());
        ROOT.with(|original| original.set(root.as_ptr()));
        let _cleanup = arm(&interpreter, mode);
        let result = interpreter.array_names(b"a");
        assert_eq!(result, (mode == 0).then(|| vec![b"k".to_vec()]));
        assert_eq!(READS.with(Cell::get), 1);
        assert_eq!(REFERENCES.with(Cell::get), 2);
        assert_eq!(
            unsafe { (*root.as_ptr()).ref_count },
            2,
            "caller and actual cell; observer is allocation-only"
        );
        if mode == 0 {
            assert!(interpreter.native_execution_refusal().is_none());
        } else {
            cause(&interpreter, mode);
        }
        assert_eq!(
            std::ptr::eq(
                obj::obj_type_ptr(root.as_ptr()),
                &crate::dict::TCL_DICT_TYPE
            ),
            mode != 5
        );
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    }
}
