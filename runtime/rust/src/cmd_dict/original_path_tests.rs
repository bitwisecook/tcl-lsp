// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual original-argv dictionary consumers; source and software scope only.
use super::*;
use crate::obj::Owned;
use std::cell::{Cell, RefCell};
thread_local! {
    static OWNER: RefCell<Option<Interp>> = const { RefCell::new(None) };
    static MODE: Cell<bool> = const { Cell::new(false) };
    static CALLS: Cell<usize> = const { Cell::new(0) };
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
fn arm(interpreter: &Interp, change: bool) -> Cleanup {
    OWNER.with(|owner| *owner.borrow_mut() = Some(interpreter.clone()));
    MODE.with(|mode| mode.set(change));
    CALLS.with(|calls| calls.set(0));
    Cleanup
}
extern "C" fn updater(value: *mut TclObj) {
    CALLS.with(|calls| calls.set(calls.get() + 1));
    let mut interpreter = OWNER.with(|owner| owner.borrow().as_ref().unwrap().clone());
    if MODE.with(Cell::get) {
        let original = interpreter.runtime_context();
        let mut changed = original.clone();
        changed.packages = vec![("original-dict-command".into(), "1.0".into())];
        interpreter.pin_context(&changed).unwrap();
        interpreter.pin_context(&original).unwrap();
    } else {
        interpreter.refuse_host_command("first original dict command getter");
    }
    unsafe { obj::set_native_updater_string_rep(value, b"set", false) };
}
static LAZY: obj::TclObjType = obj::TclObjType {
    name: c"originalDictionaryCommandGetter".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(updater),
    set_from_any_proc: None,
};
fn current(interpreter: &Interp, name: &[u8]) -> Option<*mut TclObj> {
    crate::vars::get(
        &interpreter.frames.borrow(),
        &interpreter.namespaces(),
        interpreter.current_ns(),
        name,
    )
}
fn assert_original_cause(interpreter: &Interp, change: bool) {
    let cause = interpreter
        .native_execution_refusal()
        .expect("original command cause");
    if change {
        assert_eq!(
            cause,
            tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "stale entered native operation"
                ),
            )
        );
    } else {
        let tcl_runtime_api::NativeExecutionError::HostCommandRefusal(first) = cause else {
            panic!("original Host cause lost");
        };
        assert_eq!(first.reason, "first original dict command getter");
    }
}
#[test]
fn original_dictionary_subcommand_getter_refuses_before_name_and_worker_effects() {
    // naming.embedding.original-host-publication-and-fact-transport

    for change in [false, true] {
        let mut interpreter = native();
        let _cleanup = arm(&interpreter, change);
        let command = text(b"dict");
        let sub = Owned::fresh(obj::alloc_typed(&LAZY, 0));
        let name = Owned::fresh(obj::alloc_typed(&LAZY, 0));
        let key = text(b"k");
        let value = text(b"V");
        interpreter.set_result_bytes(b"SUBCOMMAND PRIOR");
        let prior = interpreter.get_obj_result();
        assert_eq!(
            dict_cmd(
                &mut interpreter,
                &[
                    command.as_ptr(),
                    sub.as_ptr(),
                    name.as_ptr(),
                    key.as_ptr(),
                    value.as_ptr()
                ]
            ),
            Code::Error
        );
        assert_eq!(CALLS.with(Cell::get), 1);
        assert!(obj::has_string_rep(sub.as_ptr()));
        assert!(!obj::has_string_rep(name.as_ptr()));
        assert_eq!(current(&interpreter, b"set"), None);
        assert_eq!(interpreter.get_obj_result(), prior);
        assert_original_cause(&interpreter, change);
        assert_eq!(unsafe { (*name.as_ptr()).ref_count }, 1);
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    }
}
#[test]
fn original_dictionary_key_refusal_retains_early_empty_publication_and_stops_later_keys() {
    // naming.embedding.original-host-publication-and-fact-transport

    for change in [false, true] {
        let mut interpreter = native();
        let _cleanup = arm(&interpreter, change);
        let command = text(b"dict");
        let sub = text(b"set");
        let name = text(b"a");
        let first = Owned::fresh(obj::alloc_typed(&LAZY, 0));
        let later = Owned::fresh(obj::alloc_typed(&LAZY, 0));
        let value = text(b"V");
        interpreter.set_result_bytes(b"PATH PRIOR");
        let prior = interpreter.get_obj_result();
        assert_eq!(
            dict_cmd(
                &mut interpreter,
                &[
                    command.as_ptr(),
                    sub.as_ptr(),
                    name.as_ptr(),
                    first.as_ptr(),
                    later.as_ptr(),
                    value.as_ptr()
                ]
            ),
            Code::Error
        );
        assert_eq!(CALLS.with(Cell::get), 1);
        assert!(obj::has_string_rep(first.as_ptr()));
        assert!(!obj::has_string_rep(later.as_ptr()));
        let root = current(&interpreter, b"a")
            .expect("Jim published the missing root before the key getter");
        assert_eq!(
            obj::obj_type_ptr(root),
            &dict::TCL_DICT_TYPE as *const obj::TclObjType
        );
        assert_eq!(dict::original_cached_member(root, b"set").unwrap(), None);
        assert_eq!(unsafe { (*root).ref_count }, 1);
        assert_eq!(interpreter.get_obj_result(), prior);
        assert_original_cause(&interpreter, change);
        let first = interpreter.native_execution_refusal().unwrap();
        assert_eq!(interpreter.var_get(b"a"), None);
        assert_eq!(interpreter.native_execution_refusal(), Some(first));
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    }
}
#[test]
fn original_dictionary_stock_set_and_unset_use_retained_alias_and_member_roots() {
    // naming.embedding.original-host-publication-and-fact-transport

    // Genuine actual worker integration; no argument-reference parity inferred.
    let mut interpreter = native();
    assert_eq!(interpreter.eval_str(b"set a [dict create slot [dict create old 1]]; proc update {} {upvar #0 a alias; dict set alias(slot) new 2; dict unset alias(slot) old}; update"),Code::Ok);
    let parent = current(&interpreter, b"a").unwrap();
    let root = dict::original_cached_member(parent, b"slot")
        .unwrap()
        .unwrap();
    assert_eq!(interpreter.get_obj_result(), root);
    assert_eq!(unsafe { (*root).ref_count }, 2);
    assert_eq!(dict::original_cached_member(root, b"old").unwrap(), None);
    assert_eq!(
        obj::bytes_of(dict::original_cached_member(root, b"new").unwrap().unwrap()),
        b"2"
    );
    assert!(interpreter.native_execution_refusal().is_none());
    assert_eq!(interpreter.current_level(), 0);
    assert_eq!(interpreter.entered_native_operation_count(), 0);
}
#[test]
fn original_dictionary_generic_unset_missing_root_and_sugar_required_unset_remain_distinct() {
    // naming.dictionary.original-path-missing-variable-publication

    let mut interpreter = native();
    let command = text(b"dict");
    let sub = text(b"unset");
    let name = text(b"generic");
    let key = text(b"k");
    assert_eq!(
        dict_cmd(
            &mut interpreter,
            &[command.as_ptr(), sub.as_ptr(), name.as_ptr(), key.as_ptr()]
        ),
        Code::Ok
    );
    let root = current(&interpreter, b"generic").unwrap();
    assert_eq!(interpreter.get_obj_result(), root);
    assert_eq!(unsafe { (*root).ref_count }, 2);
    let sugar = text(b"required(k)");
    assert_eq!(
        interpreter.unset_original_jim_variable(sugar.as_ptr()),
        Err(Code::Error)
    );
    assert_eq!(current(&interpreter, b"required"), None);
    assert!(interpreter.native_execution_refusal().is_none());
    assert_eq!(interpreter.entered_native_operation_count(), 0);
}
