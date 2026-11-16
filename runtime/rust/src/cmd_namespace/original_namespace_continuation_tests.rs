// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;
use crate::obj::Owned;
use std::cell::{Cell, RefCell};

thread_local! {
    static ACTUAL: RefCell<Option<Interp>> = const { RefCell::new(None) };
    static PAYLOADS: RefCell<Vec<Vec<u8>>> = const { RefCell::new(Vec::new()) };
    static EVENTS: RefCell<Vec<usize>> = const { RefCell::new(Vec::new()) };
    static ACTION: Cell<u8> = const { Cell::new(0) };
    static AT: Cell<usize> = const { Cell::new(0) };
    static TARGET: Cell<NsId> = const { Cell::new(GLOBAL) };
    static PARENT: Cell<*mut TclObj> = const { Cell::new(std::ptr::null_mut()) };
    static FIRST: RefCell<Option<tcl_runtime_api::NativeExecutionError>> = const { RefCell::new(None) };
}

fn callback() {
    let mut actual = ACTUAL.with(|slot| slot.borrow().as_ref().unwrap().clone());
    match ACTION.with(Cell::get) {
        0 => {}
        1 => {
            actual.refuse_host_command("original namespace continuation first cause");
            FIRST.with(|slot| *slot.borrow_mut() = actual.native_execution_refusal());
        }
        2 => {
            let original = actual.runtime_context();
            let mut changed = original.clone();
            changed.packages = vec![("namespace-continuation-control".into(), "1.0".into())];
            actual.pin_context(&changed).unwrap();
            actual.pin_context(&original).unwrap();
        }
        3 => actual.set_current_ns(TARGET.with(Cell::get)),
        4 => {
            let old = TARGET.with(Cell::get);
            actual.delete_namespace_by_id(old);
            let replacement = actual.ensure_global_namespace(b"::tree");
            assert_ne!(replacement, old);
        }
        5 => obj::change_type(PARENT.with(Cell::get), std::ptr::null(), 0),
        6 => {
            let namespace = actual.current_ns();
            actual.frames.borrow_mut().push(namespace);
        }
        _ => unreachable!(),
    }
}

extern "C" fn update(original: *mut TclObj) {
    let index = usize::try_from(obj::internal_rep(original)).unwrap();
    EVENTS.with(|events| events.borrow_mut().push(index));
    let bytes = PAYLOADS.with(|payloads| payloads.borrow()[index].clone());
    unsafe { obj::set_native_updater_string_rep(original, &bytes, false) };
    if AT.with(Cell::get) == index {
        callback();
    }
}
#[cfg(debug_assertions)]
extern "C" fn free(_: *mut TclObj) {
    EVENTS.with(|events| events.borrow_mut().push(99));
    callback();
}
static LAZY: obj::TclObjType = obj::TclObjType {
    name: c"originalNamespaceContinuation".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(update),
    set_from_any_proc: None,
};
#[cfg(debug_assertions)]
static RETIRED: obj::TclObjType = obj::TclObjType {
    name: c"originalNamespaceHandlerRetirement".as_ptr(),
    free_int_rep_proc: Some(free),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};

struct Cleanup;
impl Drop for Cleanup {
    fn drop(&mut self) {
        ACTION.with(|action| action.set(0));
        ACTUAL.with(|slot| slot.borrow_mut().take());
        PARENT.with(|parent| parent.set(std::ptr::null_mut()));
    }
}
fn actual(engine: &str) -> (Interp, Cleanup, NsId, NsId) {
    ACTUAL.with(|slot| slot.borrow_mut().take());
    PAYLOADS.with(|values| values.borrow_mut().clear());
    EVENTS.with(|values| values.borrow_mut().clear());
    ACTION.with(|value| value.set(0));
    AT.with(|value| value.set(0));
    FIRST.with(|value| value.borrow_mut().take());
    let mut interp = original_argument_tests::instance(engine);
    assert_eq!(interp.eval_str(b"namespace eval tree {namespace eval child {}; variable value VALUE}; namespace eval dest {}"), Code::Ok);
    let tree = interp.ensure_global_namespace(b"::tree");
    let dest = interp.ensure_global_namespace(b"::dest");
    interp.set_current_ns(dest);
    ACTUAL.with(|slot| *slot.borrow_mut() = Some(interp.clone()));
    (interp, Cleanup, tree, dest)
}
fn lazy(bytes: &[u8]) -> Owned {
    obj::register_legacy_extension_type(&LAZY);
    let index = PAYLOADS.with(|payloads| {
        let mut payloads = payloads.borrow_mut();
        let index = payloads.len();
        payloads.push(bytes.to_vec());
        index
    });
    Owned::fresh(obj::alloc_typed(&LAZY, u64::try_from(index).unwrap()))
}
fn text(bytes: &[u8]) -> Owned {
    Owned::fresh(obj::new_string_bytes(bytes))
}
fn call(actual: &mut Interp, subcommand: &[u8], operands: &[&Owned]) -> Code {
    let head = text(b"namespace");
    let member = text(subcommand);
    let mut argv = vec![head.as_ptr(), member.as_ptr()];
    argv.extend(operands.iter().map(|word| word.as_ptr()));
    let code = namespace_cmd(actual, &argv);
    assert_eq!(actual.entered_native_operation_count(), 0);
    code
}
fn events() -> Vec<usize> {
    EVENTS.with(|events| events.borrow().clone())
}
fn action(mode: u8, at: usize, target: NsId) {
    ACTION.with(|action| action.set(mode));
    AT.with(|selected| selected.set(at));
    TARGET.with(|slot| slot.set(target));
}

#[test]
fn original_namespace_children_path_unknown_upvar_keep_successful_getter_order_and_objects() {
    // Software extension controls preserve the existing native success paths.
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let (mut actual, _cleanup, tree, dest) = actual(engine);
        let target = lazy(b"::tree");
        let pattern = lazy(b"*");
        assert_eq!(
            call(&mut actual, b"children", &[&target, &pattern]),
            Code::Ok,
            "{engine}"
        );
        assert_eq!(actual.result_bytes(), b"::tree::child", "{engine}");
        assert_eq!(events(), vec![0, 1]);

        if engine == "tcl8.4" {
            continue;
        }
        let first = lazy(b"::tree");
        let second = lazy(b"::dest");
        let path = Owned::fresh(actual.new_list_object(&[first.as_ptr(), second.as_ptr()]));
        assert_eq!(call(&mut actual, b"path", &[&path]), Code::Ok, "{engine}");
        assert_eq!(actual.namespaces().path(dest), &[tree, dest]);
        assert_eq!(events(), vec![0, 1, 2, 3]);
        assert_eq!(call(&mut actual, b"path", &[]), Code::Ok);
        assert_eq!(actual.result_bytes(), b"::tree ::dest");

        let handler = lazy(b"::handler retained\0counted");
        assert_eq!(call(&mut actual, b"unknown", &[&handler]), Code::Ok);
        assert_eq!(actual.get_obj_result(), handler.as_ptr());
        assert_eq!(call(&mut actual, b"unknown", &[]), Code::Ok);
        assert_eq!(actual.get_obj_result(), handler.as_ptr());
        assert_eq!(
            obj::bytes_of(handler.as_ptr()),
            b"::handler retained\0counted"
        );

        let namespace = lazy(b"::tree");
        let variable = lazy(b"value");
        let local = lazy(b"linked\0discarded");
        let start = events().len();
        let first_index = PAYLOADS.with(|payloads| payloads.borrow().len()) - 3;
        assert_eq!(
            call(&mut actual, b"upvar", &[&namespace, &variable, &local]),
            Code::Ok,
            "{engine}"
        );
        assert_eq!(
            &events()[start..],
            &[first_index, first_index + 2, first_index + 1]
        );
        assert_eq!(
            actual.eval_str(b"set linked CHANGED; set ::tree::value"),
            Code::Ok
        );
        assert_eq!(actual.result_bytes(), b"CHANGED");
        assert_eq!(obj::bytes_of(local.as_ptr()), b"linked\0discarded");
        assert!(actual.native_execution_refusal().is_none());
    }
}

#[test]
fn original_namespace_children_keeps_selected_target_birth_across_pattern_callback() {
    let (mut actual, _cleanup, tree, _) = actual("tcl8.6");
    let target = text(b"::tree");
    let pattern = lazy(b"*");
    actual.set_result_bytes(b"PRIOR");
    let prior = actual.get_obj_result();
    action(4, 0, tree);
    assert_eq!(
        call(&mut actual, b"children", &[&target, &pattern]),
        Code::Error
    );
    assert_eq!(events(), vec![0]);
    assert_eq!(actual.get_obj_result(), prior);
    assert!(actual.native_execution_refusal().is_some());
    assert_ne!(
        actual.namespaces().find_namespace(GLOBAL, b"::tree"),
        Some(tree)
    );
}

#[cfg(debug_assertions)]
#[test]
fn original_namespace_path_checks_prior_targets_and_original_member_backing_before_commit() {
    for mode in [1, 2, 3, 4, 5] {
        let (mut actual, _cleanup, tree, dest) = actual("tcl9.1");
        actual.namespaces_mut().set_path(dest, vec![dest]);
        let first = lazy(b"::tree");
        let second = lazy(b"::dest");
        let later = lazy(b"::tree");
        let path = Owned::fresh(actual.new_list_object(&[
            first.as_ptr(),
            second.as_ptr(),
            later.as_ptr(),
        ]));
        PARENT.with(|parent| parent.set(path.as_ptr()));
        actual.set_result_bytes(b"PRIOR");
        let prior = actual.get_obj_result();
        action(mode, 1, tree);
        assert_eq!(call(&mut actual, b"path", &[&path]), Code::Error, "{mode}");
        assert_eq!(events(), vec![0, 1], "{mode}");
        assert!(!obj::has_string_rep(later.as_ptr()));
        assert_eq!(actual.namespaces().path(dest), &[dest]);
        assert_eq!(actual.get_obj_result(), prior);
        let refusal = actual.native_execution_refusal().unwrap();
        if mode == 1 {
            assert_eq!(FIRST.with(|first| first.borrow().clone()), Some(refusal));
        }
    }
}

#[cfg(debug_assertions)]
#[test]
fn original_namespace_unknown_validates_original_list_and_frame_before_replacing_handler() {
    for mode in [1, 2, 3, 6] {
        let (mut actual, _cleanup, tree, dest) = actual("tcl8.6");
        let old = text(b"::old handler");
        assert_eq!(call(&mut actual, b"unknown", &[&old]), Code::Ok);
        let replacement = lazy(b"::new handler");
        actual.set_result_bytes(b"PRIOR");
        let prior = actual.get_obj_result();
        action(mode, 0, tree);
        assert_eq!(call(&mut actual, b"unknown", &[&replacement]), Code::Error);
        assert_eq!(events(), vec![0]);
        assert_eq!(
            actual.namespace_unknown_root(dest, false).unwrap().as_ptr(),
            old.as_ptr()
        );
        assert_eq!(actual.get_obj_result(), prior);
        assert!(actual.native_execution_refusal().is_some());
        if mode == 6 {
            actual.frames.borrow_mut().pop();
        }
    }
}

#[cfg(debug_assertions)]
#[test]
fn original_namespace_unknown_checks_retired_handler_callback_before_new_publication() {
    for mode in [1, 2, 3, 6] {
        let (mut actual, _cleanup, tree, dest) = actual("tcl8.6");
        let retired = Owned::fresh(obj::alloc_typed(&RETIRED, 0));
        let old = Owned::fresh(actual.new_list_object(&[retired.as_ptr()]));
        drop(retired);
        assert!(actual
            .namespaces_mut()
            .set_unknown_handler(dest, Some(old))
            .is_none());
        let replacement = text(b"::new handler");
        actual.set_result_bytes(b"PRIOR");
        let prior = actual.get_obj_result();
        action(mode, 99, tree);
        assert_eq!(call(&mut actual, b"unknown", &[&replacement]), Code::Error);
        assert_eq!(events(), vec![99]);
        assert!(actual.namespace_unknown_root(dest, false).is_none());
        assert_eq!(actual.get_obj_result(), prior);
        assert!(actual.native_execution_refusal().is_some());
        if mode == 1 {
            assert_eq!(
                FIRST.with(|first| first.borrow().clone()),
                actual.native_execution_refusal()
            );
        }
        if mode == 6 {
            actual.frames.borrow_mut().pop();
        }
    }
}

#[cfg(debug_assertions)]
#[test]
fn original_namespace_upvar_checks_namespace_before_target_getter_and_alias_publication() {
    for mode in [1, 2, 3, 4, 6] {
        let (mut actual, _cleanup, tree, dest) = actual("tcl8.6");
        let namespace = text(b"::tree");
        let target = lazy(b"value");
        let local = lazy(b"linked");
        actual.set_result_bytes(b"PRIOR");
        let prior = actual.get_obj_result();
        action(mode, 1, tree);
        assert_eq!(
            call(&mut actual, b"upvar", &[&namespace, &target, &local]),
            Code::Error,
            "{mode}"
        );
        assert_eq!(events(), vec![1], "{mode}");
        assert!(!obj::has_string_rep(target.as_ptr()));
        assert_eq!(actual.get_obj_result(), prior);
        assert!(actual
            .namespaces()
            .var_table(dest)
            .cell(b"linked")
            .is_none());
        assert!(actual.native_execution_refusal().is_some());
        if mode == 6 {
            actual.frames.borrow_mut().pop();
        }
    }
}

#[test]
fn original_namespace_upvar_transports_outer_namespace_check_through_original_target_conversion() {
    let (mut actual, _cleanup, tree, dest) = actual("tcl8.6");
    let namespace = text(b"::tree");
    let target = lazy(b"value");
    let local = text(b"linked");
    actual.set_result_bytes(b"PRIOR");
    let prior = actual.get_obj_result();
    action(3, 0, tree);
    assert_eq!(
        call(&mut actual, b"upvar", &[&namespace, &target, &local]),
        Code::Error
    );
    assert_eq!(events(), vec![0]);
    assert_eq!(actual.get_obj_result(), prior);
    assert!(actual
        .namespaces()
        .var_table(dest)
        .cell(b"linked")
        .is_none());
    assert!(actual.native_execution_refusal().is_some());
}
