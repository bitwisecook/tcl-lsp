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

//! Original dynamic expression Cat uses its actual compiled frame before operands.
use super::*;
use crate::cmd_control::original_list_conversion_tests as fixture;
use crate::obj::TclObjType;
use std::cell::Cell;
thread_local! {
    static MODE: Cell<u8> = const { Cell::new(0) };
    static GETTERS: Cell<usize> = const { Cell::new(0) };
    static LATER: Cell<usize> = const { Cell::new(0) };
    static FREES: Cell<usize> = const { Cell::new(0) };
    static REFS: Cell<obj::TclSize> = const { Cell::new(-1) };
}
fn change() {
    let mut actual = unsafe { crate::codegen_abi::current_interp().as_ref() }
        .unwrap()
        .clone();
    match MODE.get() {
        1 => {
            actual.refuse_host_command("original production Cat first callback");
        }
        2 => {
            let mut frames = actual.frames.borrow_mut();
            assert!(frames.pop().is_some());
            frames.push(crate::namespace::GLOBAL);
        }
        4 => {
            let before = actual.runtime_context();
            let mut changed = before.clone();
            changed.packages = vec![("original-production-cat".to_owned(), "1".to_owned())];
            actual.pin_context(&changed).unwrap();
            actual.pin_context(&before).unwrap();
        }
        _ => {}
    }
}
extern "C" fn first(original: *mut TclObj) {
    GETTERS.set(GETTERS.get() + 1);
    REFS.set(unsafe { (*original).ref_count });
    change();
    unsafe {
        obj::set_native_updater_string_rep(original, b"A", false);
    }
}
extern "C" fn later(original: *mut TclObj) {
    LATER.set(LATER.get() + 1);
    unsafe {
        obj::set_native_updater_string_rep(original, b"B", false);
    }
}
extern "C" fn retire(_original: *mut TclObj) {
    FREES.set(FREES.get() + 1);
    if MODE.get() != 0 {
        let mut actual = unsafe { crate::codegen_abi::current_interp().as_ref() }
            .unwrap()
            .clone();
        actual.refuse_host_command("original production Cat unused owner cleanup");
    }
}
static FIRST: TclObjType = TclObjType {
    name: c"originalProductionCatFirst".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(first),
    set_from_any_proc: None,
};
static LATER_TYPE: TclObjType = TclObjType {
    name: c"originalProductionCatLater".as_ptr(),
    free_int_rep_proc: Some(retire),
    dup_int_rep_proc: None,
    update_string_proc: Some(later),
    set_from_any_proc: None,
};
struct Reset;
impl Drop for Reset {
    fn drop(&mut self) {
        MODE.set(0);
    }
}
fn assert_first(actual: &Interp, mode: u8) {
    let first = actual
        .native_execution_refusal()
        .expect("actual first production Cat cause");
    if mode == 1 {
        assert!(
            matches!(first, tcl_runtime_api::NativeExecutionError::HostCommandRefusal(ref reason)
            if reason.reason == "original production Cat first callback")
        );
    } else {
        assert!(matches!(
            first,
            tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(_)
        ));
    }
}

#[test]
fn original_dynamic_expression_cat_refuses_before_later_getter_and_backend() {
    // naming.embedding.original-host-publication-and-fact-transport

    // Software callbacks on genuine compiled cells, including no-TomMath builds.
    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for mode in [1, 2, 4] {
            let mut actual = fixture::native(profile);
            assert_eq!(actual.eval_str(b"proc p {} {expr $x $y}"), Code::Ok);
            assert_eq!(actual.eval_str(b"p"), Code::Error);
            let artifact = super::super::cache(actual.proc_def(b"p").unwrap().body.as_ptr())
                .expect("actual retained expression artifact");
            let command = &artifact.scripts[&artifact.region].commands[0];
            let Operation::Expression(expression) = &command.operation else {
                panic!("genuine retained expression instruction");
            };
            assert!(expression.static_program.is_none());
            {
                let mut frames = actual.frames.borrow_mut();
                frames.push(crate::namespace::GLOBAL);
                frames
                    .install_native_compiled_local_layout(artifact.compiled_local_layout().unwrap())
                    .unwrap();
            }
            let _installed = fixture::install(&mut actual, 0);
            let _reset = Reset;
            let x = obj::Owned::fresh(obj::alloc_typed(&FIRST, 0));
            let y = obj::Owned::fresh(obj::alloc_typed(&LATER_TYPE, 0));
            for (root, value) in [(b"x".as_slice(), &x), (b"y".as_slice(), &y)] {
                let slot = command
                    .words
                    .iter()
                    .find_map(|word| {
                        word.arena.locals.iter().find_map(|(span, slot)| {
                            (word.arena.original.bytes(*span) == Some(root)).then_some(*slot)
                        })
                    })
                    .expect("actual compiled expression local slot");
                let target = EvaluatedTarget {
                    root: root.to_vec(),
                    element: None,
                    original_name: None,
                    original_index: None,
                    combined: false,
                };
                actual.body_store_value(&target, Some(slot), value).unwrap();
            }
            let prior = fixture::string(b"PRIOR");
            actual.set_result(prior.as_ptr());
            let scope =
                crate::interp::native_operation_currency::NativeOperationScope::enter(&actual)
                    .unwrap();
            GETTERS.set(0);
            LATER.set(0);
            REFS.set(-1);
            FREES.set(0);
            MODE.set(mode);
            let reached = actual.execute_body_expression_instruction(
                &artifact,
                command,
                expression,
                &mut BodyExecution::default(),
            );
            assert_eq!(reached, Err(Code::Error));
            assert_eq!(GETTERS.get(), 1);
            assert_eq!(REFS.get(), 3);
            assert_eq!(LATER.get(), 0);
            assert_eq!(actual.result_obj(), prior.as_ptr());
            assert_first(&actual, mode);
            assert!(scope.currency().ensure_current_or_refuse().is_err());
            MODE.set(0);
            actual.frames.borrow_mut().pop();
        }
    }
}
