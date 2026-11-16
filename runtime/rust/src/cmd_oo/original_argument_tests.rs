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
use std::cell::Cell;

thread_local! {
    static OWNER: RefCell<Option<crate::interp::WeakInterp>> = const { RefCell::new(None) };
    static MODE: Cell<u8> = const { Cell::new(0) };
    static UPDATES: Cell<usize> = const { Cell::new(0) };
    static FREES: Cell<usize> = const { Cell::new(0) };
}
#[cfg(debug_assertions)]
fn native() -> Interp {
    let mut interpreter = Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect("tcl8.6"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    assert_eq!(
        interpreter.eval_str(b"oo::class create OriginalArgumentClass"),
        Code::Ok
    );
    OWNER.with(|owner| *owner.borrow_mut() = Some(interpreter.downgrade()));
    MODE.with(|mode| mode.set(0));
    UPDATES.with(|calls| calls.set(0));
    FREES.with(|calls| calls.set(0));
    interpreter
}
#[cfg(debug_assertions)]
fn owner() -> Interp {
    OWNER.with(|owner| owner.borrow().as_ref().unwrap().upgrade().unwrap())
}
#[cfg(debug_assertions)]
fn refuse(interpreter: &mut Interp, mode: u8) {
    if mode == 1 {
        interpreter.refuse_host_command("first original OO argument callback");
    } else if mode == 2 {
        let original = interpreter.runtime_context();
        let mut changed = original.clone();
        changed
            .packages
            .push(("oo-original-argument".to_owned(), "1.0".to_owned()));
        interpreter.pin_context(&changed).unwrap();
        interpreter.pin_context(&original).unwrap();
    }
}
#[cfg(debug_assertions)]
extern "C" fn update(original: *mut TclObj) {
    UPDATES.with(|calls| calls.set(calls.get() + 1));
    let mut interpreter = owner();
    // The actual updater may inspect the reached original frame. No frame
    // RefMut may be held by the reporting driver while it runs.
    assert!(interpreter.frames.try_borrow().is_ok());
    refuse(&mut interpreter, MODE.with(Cell::get));
    unsafe { obj::set_native_updater_string_rep(original, b"OriginalArgumentClass", false) };
}
#[cfg(debug_assertions)]
extern "C" fn free_body(_original: *mut TclObj) {
    FREES.with(|calls| calls.set(calls.get() + 1));
    let mut interpreter = owner();
    assert!(interpreter.oo.try_borrow_mut().is_ok());
    refuse(&mut interpreter, MODE.with(Cell::get));
}
#[cfg(debug_assertions)]
static ARGUMENT: obj::TclObjType = obj::TclObjType {
    name: c"originalOoArgument".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(update),
    set_from_any_proc: None,
};
#[cfg(debug_assertions)]
static BODY: obj::TclObjType = obj::TclObjType {
    name: c"originalOoRetiredBody".as_ptr(),
    free_int_rep_proc: Some(free_body),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
#[cfg(debug_assertions)]
fn word(bytes: &[u8]) -> obj::Owned {
    obj::Owned::fresh(obj::new_string_bytes(bytes))
}
#[cfg(debug_assertions)]
fn argument() -> obj::Owned {
    obj::Owned::fresh(obj::alloc_typed(&ARGUMENT, 0))
}
#[cfg(debug_assertions)]
fn enter(interpreter: &mut Interp) -> (OoId, NsId) {
    let target = interpreter.oo_resolve_object(b"OriginalArgumentClass");
    let argv = [
        word(b"oo::define"),
        word(b"OriginalArgumentClass"),
        word(b""),
    ];
    let pointers: Vec<_> = argv.iter().map(obj::Owned::as_ptr).collect();
    let caller = interpreter
        .oo_enter_definition(&DefTarget::Class(target), &pointers)
        .unwrap();
    (target, caller)
}
#[cfg(debug_assertions)]
fn retained_failure(interpreter: &Interp, prior: *mut TclObj) {
    assert!(interpreter.native_execution_refusal().is_some());
    assert_eq!(interpreter.get_obj_result(), prior);
}

#[cfg(debug_assertions)]
#[test]
fn define_target_callback_stops_before_body_and_definition_effects() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Software getter/currency control; native OO operand order is separate.
    for mode in [1, 2] {
        let mut interpreter = native();
        let head = word(b"oo::define");
        let target = argument();
        let body = argument();
        let activation = interpreter.frames.borrow().current_activation();
        let namespace = interpreter.current_ns();
        interpreter.set_result_bytes(b"PRIOR");
        let prior = interpreter.get_obj_result();
        MODE.with(|selected| selected.set(mode));
        assert_eq!(
            oo_define_cmd(
                &mut interpreter,
                &[head.as_ptr(), target.as_ptr(), body.as_ptr()]
            ),
            Code::Error
        );
        retained_failure(&interpreter, prior);
        assert_eq!(UPDATES.with(Cell::get), 1);
        assert!(!obj::has_string_rep(body.as_ptr()));
        assert_eq!(obj_bytes(target.as_ptr()), b"OriginalArgumentClass");
        assert_eq!(unsafe { (*target.as_ptr()).ref_count }, 1);
        assert_eq!(unsafe { (*body.as_ptr()).ref_count }, 1);
        assert_eq!(interpreter.current_ns(), namespace);
        assert_eq!(interpreter.frames.borrow().current_activation(), activation);
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    }
}

#[cfg(debug_assertions)]
#[test]
fn method_name_callback_withdraws_later_formal_body_and_installation_stages() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for mode in [1, 2] {
        let mut interpreter = native();
        let (target, caller) = enter(&mut interpreter);
        let head = word(b"method");
        let name = argument();
        let formals = argument();
        let body = argument();
        interpreter.set_result_bytes(b"PRIOR");
        let prior = interpreter.get_obj_result();
        MODE.with(|selected| selected.set(mode));
        assert_eq!(
            def_method(
                &mut interpreter,
                &[
                    head.as_ptr(),
                    name.as_ptr(),
                    formals.as_ptr(),
                    body.as_ptr()
                ]
            ),
            Code::Error
        );
        retained_failure(&interpreter, prior);
        assert_eq!(UPDATES.with(Cell::get), 1);
        assert!(!obj::has_string_rep(formals.as_ptr()));
        assert!(!obj::has_string_rep(body.as_ptr()));
        assert!(!interpreter.oo.borrow().classes[&target]
            .methods
            .contains_key(b"OriginalArgumentClass".as_slice()));
        interpreter.oo_leave_definition(caller);
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    }
}

#[cfg(debug_assertions)]
#[test]
fn definition_frame_reporting_runs_updaters_outside_frame_borrow_and_cleans_refusal() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Definition entry retains the original tape without presentation.
    // The explicit report String stage reaches the original updater.
    for mode in [0, 1, 2] {
        let mut interpreter = native();
        let target = interpreter.oo_resolve_object(b"OriginalArgumentClass");
        let argv = [
            word(b"oo::define"),
            word(b"OriginalArgumentClass"),
            argument(),
        ];
        let pointers: Vec<_> = argv.iter().map(obj::Owned::as_ptr).collect();
        let namespace = interpreter.current_ns();
        let activation = interpreter.frames.borrow().current_activation();
        interpreter.set_result_bytes(b"PRIOR");
        let prior = interpreter.get_obj_result();
        MODE.with(|selected| selected.set(mode));
        let originals =
            crate::interp::native_arguments::OriginalArguments::enter(&interpreter, &pointers)
                .unwrap();
        let caller = interpreter
            .oo_enter_definition(&DefTarget::Class(target), &pointers)
            .unwrap();
        assert!(
            matches!(interpreter.active_def_target(), Some(DefTarget::Class(actual)) if actual == target)
        );
        assert_eq!(
            interpreter
                .frames
                .borrow()
                .original_error_stack_argv()
                .unwrap(),
            pointers,
        );
        assert_eq!(UPDATES.with(Cell::get), 0);
        assert!(!obj::has_string_rep(argv[2].as_ptr()));
        let level = interpreter.frames.borrow().current_level();
        let report = obj::Owned::fresh(
            tcl_runtime_api::Introspect::level_argv(&interpreter, level)
                .unwrap()
                .unwrap(),
        );
        let bytes = interpreter.native_object_string_bytes(report.as_ptr());
        if mode == 0 {
            assert!(bytes.is_ok());
            assert!(originals.check().is_ok());
        } else {
            assert!(bytes.is_err());
            assert!(originals.check().is_err());
            retained_failure(&interpreter, prior);
        }
        drop(report);
        interpreter.oo_leave_definition(caller);
        drop(originals);
        assert_eq!(UPDATES.with(Cell::get), 1);
        assert_eq!(unsafe { (*argv[2].as_ptr()).ref_count }, 1);
        assert_eq!(interpreter.current_ns(), namespace);
        assert_eq!(interpreter.frames.borrow().current_activation(), activation);
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    }
}

#[cfg(debug_assertions)]
#[test]
fn retired_method_callback_stops_before_later_name_and_result_publication() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for mode in [1, 2] {
        let mut interpreter = native();
        let (target, caller) = enter(&mut interpreter);
        let body = obj::Owned::fresh(obj::alloc_typed(&BODY, 0));
        unsafe { obj::set_native_updater_string_rep(body.as_ptr(), b"return", false) };
        let method = native_method_body(&interpreter, Vec::new(), body, None);
        let retired = interpreter
            .oo
            .borrow_mut()
            .classes
            .get_mut(&target)
            .unwrap()
            .methods
            .insert_generated(b"retire".to_vec(), method);
        assert!(retired.is_none());
        let head = word(b"deletemethod");
        let name = word(b"retire");
        let later = argument();
        interpreter.set_result_bytes(b"PRIOR");
        let prior = interpreter.get_obj_result();
        MODE.with(|selected| selected.set(mode));
        assert_eq!(
            def_deletemethod(
                &mut interpreter,
                &[head.as_ptr(), name.as_ptr(), later.as_ptr()]
            ),
            Code::Error
        );
        retained_failure(&interpreter, prior);
        assert_eq!(FREES.with(Cell::get), 1);
        assert_eq!(UPDATES.with(Cell::get), 0);
        assert!(!obj::has_string_rep(later.as_ptr()));
        assert!(!interpreter.oo.borrow().classes[&target]
            .methods
            .contains_key(b"retire".as_slice()));
        interpreter.oo_leave_definition(caller);
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    }
}
