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

//! Genuine compiled List/Assign/Index stages; software callbacks only.
use super::super::native_list_conversion::BodyObjectOperation;
use super::*;
use crate::cmd_control::original_list_conversion_tests as fixture;
use std::cell::Cell;
use tcl_runtime_api::NativeExecutionError;
thread_local! {
    static MODE: Cell<u8> = const { Cell::new(0) };
    static FREES: Cell<usize> = const { Cell::new(0) };
    static REFS: Cell<obj::TclSize> = const { Cell::new(-1) };
    static GETTERS: Cell<usize> = const { Cell::new(0) };
}
fn actual() -> Interp {
    unsafe { crate::codegen_abi::current_interp().as_ref() }
        .unwrap()
        .clone()
}
extern "C" fn free(_value: *mut TclObj) {
    FREES.set(FREES.get() + 1);
    REFS.set(unsafe { (*_value).ref_count });
    match MODE.get() {
        1 => {
            actual().refuse_host_command("original compiled List frontier");
        }
        2 => {
            let interpreter = actual();
            let mut frames = interpreter.frames.borrow_mut();
            assert!(frames.pop().is_some());
            frames.push(crate::namespace::GLOBAL);
        }
        _ => {}
    }
}
extern "C" fn later(value: *mut TclObj) {
    GETTERS.set(GETTERS.get() + 1);
    unsafe {
        obj::set_native_updater_string_rep(value, b"{A B} C", false);
    }
}
static ORIGINAL: obj::TclObjType = obj::TclObjType {
    name: c"originalCompiledListFrontier".as_ptr(),
    free_int_rep_proc: Some(free),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
static LATER: obj::TclObjType = obj::TclObjType {
    name: c"originalCompiledListLaterGetter".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(later),
    set_from_any_proc: None,
};
fn source(bytes: &[u8]) -> obj::Owned {
    let owner = obj::Owned::fresh(obj::alloc_typed(&ORIGINAL, 0));
    unsafe {
        obj::set_native_updater_string_rep(owner.as_ptr(), bytes, false);
    }
    owner
}
fn reset(mode: u8) {
    MODE.set(mode);
    FREES.set(0);
    REFS.set(-1);
    GETTERS.set(0);
}
fn first(interpreter: &Interp, mode: u8) {
    let reached = interpreter.native_execution_refusal().unwrap();
    if mode == 1 {
        assert!(
            matches!(reached, NativeExecutionError::HostCommandRefusal(ref cause) if cause.reason=="original compiled List frontier")
        );
    } else {
        assert!(matches!(
            reached,
            NativeExecutionError::ValueAccessRefusal(_)
        ));
    }
}
fn artifact(interpreter: &mut Interp, body: &[u8]) -> Rc<NativeBodyArtifact> {
    let mut definition = b"proc p {} {".to_vec();
    definition.extend_from_slice(body);
    definition.push(b'}');
    assert_eq!(interpreter.eval_str(&definition), Code::Ok);
    assert_eq!(interpreter.eval_str(b"p"), Code::Error); // actual missing local x, cache nevertheless authentic
    cache(interpreter.proc_def(b"p").unwrap().body.as_ptr())
        .expect("actual selected compiled List command")
}
fn install_frame(interpreter: &Interp, artifact: &NativeBodyArtifact) {
    let mut frames = interpreter.frames.borrow_mut();
    frames.push(crate::namespace::GLOBAL);
    frames
        .install_native_compiled_local_layout(artifact.compiled_local_layout().unwrap())
        .unwrap();
}
fn store(interpreter: &mut Interp, artifact: &NativeBodyArtifact, name: &[u8], value: &obj::Owned) {
    let slot = artifact
        .compiled_local_layout()
        .unwrap()
        .names
        .iter()
        .position(|word| word.as_ref().is_some_and(|word| word.as_bytes() == name))
        .expect("actual compiled local");
    let target = EvaluatedTarget {
        root: name.to_vec(),
        element: None,
        original_name: None,
        original_index: None,
        combined: false,
    };
    interpreter
        .body_store_value(&target, Some(slot), value)
        .unwrap();
}
#[test]
fn original_compiled_range_and_assign_check_converter_retirement_before_result_and_target_stores() {
    // Native source/result comparators remain in the unchanged older controls;
    // these actual software callbacks do not claim external C callback timing.
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for assign in [false, true] {
            if assign && profile == "tcl8.4" {
                continue;
            } // no selected C84 lassign instruction
            for mode in [0, 1, 2] {
                let mut interpreter = fixture::native(profile);
                let artifact = artifact(
                    &mut interpreter,
                    if assign {
                        b"lassign $x a b"
                    } else {
                        b"lrange $x 0 1"
                    },
                );
                let command = &artifact.scripts[&artifact.region].commands[0];
                let Operation::ListOperations(recipe) = &command.operation else {
                    panic!("actual Range/Assign instruction")
                };
                install_frame(&interpreter, &artifact);
                let _installed = fixture::install(&mut interpreter, 0);
                reset(0);
                let value = source(b"A B C");
                store(&mut interpreter, &artifact, b"x", &value);
                interpreter.set_result_bytes(b"PRIOR");
                let prior = interpreter.result_obj();
                let original = BodyObjectOperation::capture(&interpreter).unwrap();
                reset(mode);
                let reached = interpreter.execute_body_list_operation(
                    &artifact,
                    command,
                    recipe,
                    &mut BodyExecution::default(),
                );
                assert_eq!(FREES.get(), 1);
                assert_eq!(REFS.get(), if assign { 4 } else { 3 }); // actual source+cell+PUSH and Assign DUP
                if mode == 0 {
                    assert_eq!(reached, Ok(Code::Ok));
                    original.check(&interpreter).unwrap();
                    assert_eq!(
                        interpreter.result_bytes(),
                        if assign { b"C".as_slice() } else { b"A B" }
                    );
                } else {
                    assert_eq!(reached, Err(Code::Error));
                    first(&interpreter, mode);
                    assert_eq!(interpreter.result_obj(), prior);
                    assert!(!core::ptr::eq(
                        obj::obj_type_ptr(value.as_ptr()),
                        &crate::list::TCL_LIST_TYPE
                    ));
                    assert!(original.check(&interpreter).is_err());
                }
                MODE.set(0);
                interpreter.frames.borrow_mut().pop();
            }
        }
    }
}
#[test]
fn original_list_index_using_checks_scalar_and_list_cache_before_later_original_getters() {
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for mode in [0, 1, 2] {
            let mut interpreter = fixture::native(profile);
            interpreter
                .frames
                .borrow_mut()
                .push(crate::namespace::GLOBAL);
            let _installed = fixture::install(&mut interpreter, 0);
            reset(0);
            interpreter.set_result_bytes(b"PRIOR");
            let prior = interpreter.result_obj();
            let original = BodyObjectOperation::capture(&interpreter).unwrap();
            obj::register_legacy_extension_type(&LATER);
            let list = obj::Owned::fresh(obj::alloc_typed(&LATER, 0));
            let index = source(b"0");
            reset(mode);
            let reached = tcl_cmd_core::native_list_index::single_using(
                &mut interpreter,
                &list.as_ptr(),
                &index.as_ptr(),
                Some(&|actual| original.check(actual)),
            );
            assert_eq!(FREES.get(), 1);
            assert_eq!(REFS.get(), 1);
            if mode == 0 {
                let result = reached.unwrap();
                assert_eq!(obj_bytes(result.as_ptr()), b"A B");
                assert_eq!(GETTERS.get(), 1);
                original.check(&interpreter).unwrap();
            } else {
                assert!(reached.is_err());
                first(&interpreter, mode);
                assert_eq!(GETTERS.get(), 0);
                assert!(core::ptr::eq(
                    obj::obj_type_ptr(index.as_ptr()),
                    core::ptr::null()
                ));
                assert_eq!(interpreter.result_obj(), prior);
            }
            MODE.set(0);
            interpreter.frames.borrow_mut().pop();
        }
    }
}
#[test]
fn original_compiled_list_index_keeps_reached_result_and_input_roles_through_prior_result_retirement(
) {
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for mode in [0, 1, 2] {
            let mut interpreter = fixture::native(profile);
            // Missing x warms the genuine immediate instruction, not a made-up recipe.
            let artifact = artifact(&mut interpreter, b"lindex $x 0");
            let command = &artifact.scripts[&artifact.region].commands[0];
            let Operation::ListIndex(recipe) = &command.operation else {
                panic!("actual immediate ListIndex")
            };
            install_frame(&interpreter, &artifact);
            let _installed = fixture::install(&mut interpreter, 0);
            reset(0);
            let member = fixture::string(b"A");
            let list = obj::Owned::fresh(crate::list::new_list_obj_native(
                &[member.as_ptr()],
                artifact.stamp.source_protocol,
            ));
            store(&mut interpreter, &artifact, b"x", &list);
            let old = source(b"PRIOR");
            interpreter.set_result(old.as_ptr());
            drop(old);
            let original = BodyObjectOperation::capture(&interpreter).unwrap();
            reset(mode);
            let reached = interpreter.execute_body_list_index(
                &artifact,
                command,
                recipe,
                &mut BodyExecution::default(),
            );
            assert_eq!(FREES.get(), 1);
            assert_eq!(REFS.get(), 0); // actual last result role retired
            assert_eq!(interpreter.result_obj(), member.as_ptr());
            assert_eq!(unsafe { (*member.as_ptr()).ref_count }, 3); // caller+List member+reached result; transport retired
            assert_eq!(
                unsafe { (*list.as_ptr()).ref_count },
                if mode == 2 { 1 } else { 2 }
            ); // source+actual cell; PUSH retired
            if mode == 0 {
                assert_eq!(reached, Ok(Code::Ok));
                original.check(&interpreter).unwrap();
            } else {
                assert_eq!(reached, Err(Code::Error));
                first(&interpreter, mode);
                assert!(original.check(&interpreter).is_err());
            }
            MODE.set(0);
            interpreter.frames.borrow_mut().pop();
        }
    }
}
#[test]
fn neutral_immediate_none_preserves_frame_callback_conversion_while_original_using_refuses_it() {
    // Explicit None keeps the existing operation-only worker and reference schedule.
    for checked in [false, true] {
        let mut interpreter = fixture::native("tcl9.1");
        interpreter
            .frames
            .borrow_mut()
            .push(crate::namespace::GLOBAL);
        let _installed = fixture::install(&mut interpreter, 0);
        reset(0);
        let original = BodyObjectOperation::capture(&interpreter).unwrap();
        let value = source(b"A B");
        reset(2);
        let coordinate = Index::from_encoded(0);
        let reached = if checked {
            tcl_cmd_core::native_list_index::immediate_using(
                &mut interpreter,
                &value.as_ptr(),
                coordinate,
                Some(&|actual| original.check(actual)),
            )
        } else {
            tcl_cmd_core::native_list_index::immediate(
                &mut interpreter,
                &value.as_ptr(),
                coordinate,
            )
        };
        assert_eq!(FREES.get(), 1);
        assert_eq!(REFS.get(), 1);
        if checked {
            assert!(reached.is_err());
            first(&interpreter, 2);
            assert!(!core::ptr::eq(
                obj::obj_type_ptr(value.as_ptr()),
                &crate::list::TCL_LIST_TYPE
            ));
        } else {
            let selected = reached.unwrap();
            assert_eq!(obj_bytes(selected.as_ptr()), b"A");
            assert!(core::ptr::eq(
                obj::obj_type_ptr(value.as_ptr()),
                &crate::list::TCL_LIST_TYPE
            ));
        }
        MODE.set(0);
        interpreter.frames.borrow_mut().pop();
    }
}
