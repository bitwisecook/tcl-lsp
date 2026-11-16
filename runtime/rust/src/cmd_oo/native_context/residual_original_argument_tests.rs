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
#[cfg(debug_assertions)]
use crate::cmd_oo::configurable::prop_define;
use crate::cmd_oo::def_constructor;
#[cfg(debug_assertions)]
use crate::cmd_oo::slots::slot_inst_set;
use std::cell::Cell;

thread_local! {
    static OWNER: RefCell<Option<crate::interp::WeakInterp>> = const { RefCell::new(None) };
    static MODE: Cell<u8> = const { Cell::new(0) };
    static UPDATES: Cell<usize> = const { Cell::new(0) };
    static PAYLOAD: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}
fn native(version: &str) -> Interp {
    let mut interpreter = Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect(version),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    assert_eq!(
        interpreter.eval_str(b"oo::class create OriginalResidualClass"),
        Code::Ok
    );
    OWNER.with(|owner| *owner.borrow_mut() = Some(interpreter.downgrade()));
    MODE.with(|mode| mode.set(0));
    UPDATES.with(|calls| calls.set(0));
    interpreter
}
fn word(bytes: &[u8]) -> obj::Owned {
    obj::Owned::fresh(obj::new_string_bytes(bytes))
}
extern "C" fn update(original: *mut TclObj) {
    UPDATES.with(|calls| calls.set(calls.get() + 1));
    let mut interpreter = OWNER.with(|owner| owner.borrow().as_ref().unwrap().upgrade().unwrap());
    match MODE.with(Cell::get) {
        1 => {
            interpreter.refuse_host_command("first residual OO original getter");
        }
        2 => {
            let original = interpreter.runtime_context();
            let mut changed = original.clone();
            changed
                .packages
                .push(("residual-oo-stage".to_owned(), "1".to_owned()));
            interpreter.pin_context(&changed).unwrap();
            interpreter.pin_context(&original).unwrap();
        }
        3 => {
            assert_eq!(
                interpreter.eval_str(b"OriginalResidualClass destroy"),
                Code::Ok
            );
        }
        _ => {}
    }
    let bytes = PAYLOAD.with(|payload| payload.borrow().clone());
    unsafe { obj::set_native_updater_string_rep(original, &bytes, false) };
}
static ORIGINAL: obj::TclObjType = obj::TclObjType {
    name: c"residualOriginalOoArgument".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(update),
    set_from_any_proc: None,
};
fn original(payload: &[u8], mode: u8) -> obj::Owned {
    MODE.with(|selected| selected.set(mode));
    PAYLOAD.with(|selected| *selected.borrow_mut() = payload.to_vec());
    obj::Owned::fresh(obj::alloc_typed(&ORIGINAL, 0))
}
fn definition(interpreter: &mut Interp) -> (OoId, NsId) {
    let target = interpreter.oo_resolve_object(b"OriginalResidualClass");
    let words = [
        word(b"oo::define"),
        word(b"OriginalResidualClass"),
        word(b""),
    ];
    let pointers: Vec<_> = words.iter().map(obj::Owned::as_ptr).collect();
    let caller = interpreter
        .oo_enter_definition(&DefTarget::Class(target), &pointers)
        .unwrap();
    (target, caller)
}

#[cfg(debug_assertions)]
#[test]
fn class_factory_original_name_refusal_stops_namespace_and_later_operand_effects() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Software original getter/currency control; no stock factory-order observation.
    for mode in [1, 2] {
        let mut interpreter = native("tcl8.6");
        let class = interpreter.oo_resolve_object(b"OriginalResidualClass");
        let name = original(b"CreatedResidual", mode);
        let later = obj::Owned::fresh(obj::alloc_typed(&ORIGINAL, 0));
        let head = word(b"OriginalResidualClass");
        let selector = word(b"create");
        let tape = [
            head.as_ptr(),
            selector.as_ptr(),
            name.as_ptr(),
            later.as_ptr(),
        ];
        interpreter.set_result_bytes(b"PRIOR");
        let prior = interpreter.get_obj_result();
        assert_eq!(
            interpreter.oo_class_factory(
                class,
                b"OriginalResidualClass",
                b"create",
                &tape[2..],
                false,
                Some(&tape)
            ),
            Some(Code::Error)
        );
        assert_eq!(UPDATES.with(Cell::get), 1);
        assert!(!obj::has_string_rep(later.as_ptr()));
        assert_eq!(interpreter.get_obj_result(), prior);
        assert!(interpreter.native_execution_refusal().is_some());
        assert!(!interpreter
            .oo
            .borrow()
            .names
            .values()
            .any(|name| name == b"::CreatedResidual"));
        assert_eq!(unsafe { (*name.as_ptr()).ref_count }, 1);
        assert_eq!(unsafe { (*later.as_ptr()).ref_count }, 1);
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    }
}

#[cfg(debug_assertions)]
#[test]
fn original_selector_refusal_stops_cache_dispatch_and_keeps_later_header_unread() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for mode in [0, 1, 2] {
        let mut interpreter = native("tcl8.6");
        assert_eq!(interpreter.eval_str(b"oo::define OriginalResidualClass method m {arg} {return CALLED}; OriginalResidualClass create residualObject"), Code::Ok);
        let target = interpreter.oo_resolve_object(b"residualObject");
        let selector = original(b"m", mode);
        let later = obj::Owned::fresh(obj::alloc_typed(&ORIGINAL, 0));
        let head = word(b"residualObject");
        let tape = [head.as_ptr(), selector.as_ptr(), later.as_ptr()];
        interpreter.set_result_bytes(b"PRIOR");
        let prior = interpreter.get_obj_result();
        let code = interpreter.oo_invoke_with_original(
            target,
            b"m",
            &tape[2..],
            true,
            Some(b"residualObject"),
            Some(&tape),
        );
        assert_eq!(UPDATES.with(Cell::get), 1);
        assert!(!obj::has_string_rep(later.as_ptr()));
        if mode == 0 {
            assert_eq!(code, Code::Ok);
            assert_eq!(interpreter.result_bytes(), b"CALLED");
        } else {
            assert_eq!(code, Code::Error);
            assert_eq!(interpreter.get_obj_result(), prior);
            assert!(interpreter.native_execution_refusal().is_some());
            assert!(core::ptr::eq(
                obj::obj_type_ptr(selector.as_ptr()),
                &ORIGINAL
            ));
            assert!(obj::native_method_name::chain(selector.as_ptr()).is_none());
        }
        assert_eq!(unsafe { (*later.as_ptr()).ref_count }, 1);
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    }
}

#[cfg(debug_assertions)]
#[test]
fn slot_selected_list_preserves_parse_refusal_and_stops_before_cache_after_callback() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for mode in [0, 1, 2] {
        let mut interpreter = native("tcl8.6");
        let (target, caller) = definition(&mut interpreter);
        let slot = interpreter.oo_resolve_object(b"::oo::define::filter");
        interpreter
            .oo
            .borrow_mut()
            .classes
            .get_mut(&target)
            .unwrap()
            .filters = vec![b"SENTINEL".to_vec()];
        let value = if mode == 0 {
            word(b"{")
        } else {
            original(b"first second", mode)
        };
        interpreter.set_result_bytes(b"PRIOR");
        let prior = interpreter.get_obj_result();
        assert_eq!(
            slot_inst_set(&mut interpreter, slot, &[value.as_ptr()]),
            Code::Error
        );
        assert_eq!(
            interpreter.oo.borrow().classes[&target].filters,
            [b"SENTINEL".to_vec()]
        );
        if mode == 0 {
            assert!(interpreter.native_execution_refusal().is_none());
            assert!(interpreter
                .result_bytes()
                .windows(b"unmatched".len())
                .any(|part| part == b"unmatched"));
        } else {
            assert_eq!(UPDATES.with(Cell::get), 1);
            assert!(interpreter.native_execution_refusal().is_some());
            assert_eq!(interpreter.get_obj_result(), prior);
            assert!(core::ptr::eq(obj::obj_type_ptr(value.as_ptr()), &ORIGINAL));
            assert!(obj::has_string_rep(value.as_ptr()));
        }
        assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 1);
        interpreter.oo_leave_definition(caller);
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    }
    let mut interpreter = native("tcl8.6");
    let (target, caller) = definition(&mut interpreter);
    let slot = interpreter.oo_resolve_object(b"::oo::define::filter");
    let first = word(b"a\0b");
    let value = obj::Owned::fresh(interpreter.new_list_object(&[first.as_ptr()]));
    assert_eq!(
        slot_inst_set(&mut interpreter, slot, &[value.as_ptr()]),
        Code::Ok
    );
    assert_eq!(
        interpreter.oo.borrow().classes[&target].filters,
        [b"a\0b".to_vec()]
    );
    assert!(!obj::has_string_rep(value.as_ptr()));
    assert_eq!(unsafe { (*first.as_ptr()).ref_count }, 2);
    interpreter.oo_leave_definition(caller);
}

#[cfg(debug_assertions)]
#[test]
fn property_script_refusal_stops_later_name_and_accessor_publication() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for mode in [1, 2] {
        let mut interpreter = native("tcl9.1");
        let (target, caller) = definition(&mut interpreter);
        let head = word(b"property");
        let name = word(b"p");
        let option = word(b"-get");
        let body = original(b"return REACHED", mode);
        let later = obj::Owned::fresh(obj::alloc_typed(&ORIGINAL, 0));
        interpreter.set_result_bytes(b"PRIOR");
        let prior = interpreter.get_obj_result();
        assert_eq!(
            prop_define(
                &mut interpreter,
                &[
                    head.as_ptr(),
                    name.as_ptr(),
                    option.as_ptr(),
                    body.as_ptr(),
                    later.as_ptr()
                ],
                false
            ),
            Code::Error
        );
        assert_eq!(UPDATES.with(Cell::get), 1);
        assert!(!obj::has_string_rep(later.as_ptr()));
        assert!(!interpreter.oo.borrow().classes[&target]
            .methods
            .contains_key(b"<ReadProp-p>".as_slice()));
        assert_eq!(interpreter.get_obj_result(), prior);
        assert!(interpreter.native_execution_refusal().is_some());
        assert_eq!(unsafe { (*body.as_ptr()).ref_count }, 1);
        interpreter.oo_leave_definition(caller);
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    }
}

#[test]
fn deleted_definition_target_cannot_supply_constructor_continuation() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // A real nested destroy changes target lifecycle while operation context remains current.
    let mut interpreter = native("tcl8.6");
    let (target, caller) = definition(&mut interpreter);
    let head = word(b"constructor");
    let formals = obj::Owned::fresh(obj::alloc_typed(&ORIGINAL, 0));
    let body = original(b"return REACHED", 3);
    assert_eq!(
        def_constructor(
            &mut interpreter,
            &[head.as_ptr(), formals.as_ptr(), body.as_ptr()]
        ),
        Code::Error
    );
    assert_eq!(UPDATES.with(Cell::get), 1);
    assert!(!obj::has_string_rep(formals.as_ptr()));
    assert!(!interpreter.oo.borrow().objects.contains_key(&target));
    assert!(!interpreter.oo.borrow().classes.contains_key(&target));
    assert!(interpreter.native_execution_refusal().is_none());
    assert_eq!(interpreter.error_code(), b"TCL OO MONKEY_BUSINESS");
    assert_eq!(unsafe { (*formals.as_ptr()).ref_count }, 1);
    interpreter.oo_leave_definition(caller);
    assert_eq!(interpreter.entered_native_operation_count(), 0);
}
