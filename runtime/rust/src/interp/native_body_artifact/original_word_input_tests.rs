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

//! Original compiled operand Cat stages and genuine component-owner cleanup.
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
            actual.refuse_host_command("original compiled operand Cat callback");
        }
        2 => {
            let mut frames = actual.frames.borrow_mut();
            assert!(frames.pop().is_some());
            frames.push(crate::namespace::GLOBAL);
        }
        4 => {
            let before = actual.runtime_context();
            let mut changed = before.clone();
            changed.packages = vec![("original-word".to_owned(), "1".to_owned())];
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
        obj::set_native_updater_string_rep(original, b"X", false);
    }
}
extern "C" fn later(original: *mut TclObj) {
    LATER.set(LATER.get() + 1);
    unsafe {
        obj::set_native_updater_string_rep(original, b"Y", false);
    }
}
extern "C" fn retire(original: *mut TclObj) {
    FREES.set(FREES.get() + 1);
    REFS.set(unsafe { (*original).ref_count });
    change();
}
static FIRST: TclObjType = TclObjType {
    name: c"originalBodyWordFirst".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(first),
    set_from_any_proc: None,
};
static LATER_TYPE: TclObjType = TclObjType {
    name: c"originalBodyWordLater".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(later),
    set_from_any_proc: None,
};
static RETIRE: TclObjType = TclObjType {
    name: c"originalBodyWordRetiredComponent".as_ptr(),
    free_int_rep_proc: Some(retire),
    dup_int_rep_proc: None,
    update_string_proc: None,
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
        .expect("actual first Cat cause retained");
    if mode == 1 {
        assert!(
            matches!(first,tcl_runtime_api::NativeExecutionError::HostCommandRefusal(ref refusal) if refusal.reason=="original compiled operand Cat callback")
        );
    } else {
        assert!(matches!(
            first,
            tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(_)
        ));
    }
}
#[test]
fn original_compiled_word_cat_keeps_entered_frame_inside_first_string_before_later_component() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Genuine cached arena/local cells and their original getters. Reference
    // observations are software roles, not external Native callback chronology.
    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for mode in [0, 1, 2, 4] {
            let mut actual = fixture::native(profile);
            assert_eq!(actual.eval_str(b"proc p {} {return \"A$x$y\"}"), Code::Ok);
            assert_eq!(actual.eval_str(b"p"), Code::Error);
            let artifact = cache(actual.proc_def(b"p").unwrap().body.as_ptr())
                .expect("actual original operand artifact");
            let command = &artifact.scripts[&artifact.region].commands[0];
            let word = &command.words[1];
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
                let slot = word
                    .arena
                    .locals
                    .iter()
                    .find_map(|(span, slot)| {
                        (word.arena.original.bytes(*span) == Some(root)).then_some(*slot)
                    })
                    .expect("genuine retained compiled local slot");
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
            MODE.set(mode);
            let reached = actual.body_word(&artifact, word, &mut BodyExecution::default());
            assert_eq!(GETTERS.get(), 1);
            assert_eq!(REFS.get(), 3);
            assert_eq!(actual.result_obj(), prior.as_ptr());
            if mode == 0 {
                let result = reached.unwrap();
                assert_eq!(obj::bytes_of(result.as_ptr()), b"AXY");
                assert_eq!(LATER.get(), 1);
                scope.currency().ensure_current_or_refuse().unwrap();
            } else {
                assert_eq!(reached.err(), Some(Code::Error));
                assert_first(&actual, mode);
                assert!(scope.currency().ensure_current_or_refuse().is_err());
                assert_eq!(LATER.get(), 0);
            }
            MODE.set(0);
            actual.frames.borrow_mut().pop();
        }
    }
}
#[test]
fn original_compiled_word_cat_checks_after_genuine_component_retirement_before_returning_result() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // The real second component is the sole native owner, so actual checked
    // Cat completion retires it without a protective native hold.
    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for mode in [0, 1, 2, 4] {
            let mut actual = fixture::native(profile);
            actual.frames.borrow_mut().push(crate::namespace::GLOBAL);
            let _installed = fixture::install(&mut actual, 0);
            let _reset = Reset;
            let original = native_list_conversion::BodyObjectOperation::capture(&actual).unwrap();
            let first = fixture::string(b"A");
            let second = obj::Owned::fresh(obj::alloc_typed(&RETIRE, 0));
            unsafe {
                obj::set_native_updater_string_rep(second.as_ptr(), b"B", false);
            }
            let prior = fixture::string(b"PRIOR");
            actual.set_result(prior.as_ptr());
            let scope =
                crate::interp::native_operation_currency::NativeOperationScope::enter(&actual)
                    .unwrap();
            FREES.set(0);
            REFS.set(-1);
            MODE.set(mode);
            let reached =
                actual.concatenate_body_values_using(vec![first, second], Some(&original));
            assert_eq!(FREES.get(), 1);
            assert_eq!(REFS.get(), 0);
            assert_eq!(actual.result_obj(), prior.as_ptr());
            if mode == 0 {
                let result = reached.unwrap();
                assert_eq!(obj::bytes_of(result.as_ptr()), b"AB");
                original.check(&actual).unwrap();
            } else {
                let error = reached
                    .err()
                    .expect("original component retirement stops result handoff");
                assert_first(&actual, mode);
                assert_eq!(
                    error.native_execution_refusal(),
                    actual.native_execution_refusal().as_ref()
                );
                assert!(original.check(&actual).is_err());
            }
            MODE.set(0);
            actual.frames.borrow_mut().pop();
        }
    }
}

extern "C" fn retire_command(original: *mut TclObj) {
    FREES.set(FREES.get() + 1);
    REFS.set(unsafe { (*original).ref_count });
    let actual = unsafe { crate::codegen_abi::current_interp().as_ref() }.unwrap();
    // The authentic displaced command header must retire outside this borrow.
    drop(actual.cmd_frames.borrow_mut());
}
static COMMAND_ROLE: TclObjType = TclObjType {
    name: c"originalBodyWordCommandRole".as_ptr(),
    free_int_rep_proc: Some(retire_command),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
fn command_component(actual: &mut Interp, _argv: &[*mut TclObj]) -> Code {
    let incoming = obj::Owned::fresh(obj::alloc_typed(&COMMAND_ROLE, 0));
    let old = {
        let mut frames = actual.cmd_frames.borrow_mut();
        let frame = frames.last_mut().expect("actual original command frame");
        frame.line = 99;
        frame.cmd = b"REACHED CHILD".to_vec();
        frame.original_command.replace(incoming)
    };
    drop(old);
    actual.set_result_bytes(b"C");
    if MODE.get() == 5 {
        let removed = actual.cmd_frames.borrow_mut().pop();
        drop(removed);
        actual.refuse_host_command("original compiled operand Cat callback");
    } else {
        change();
    }
    if actual.host_refusal_pending() { Code::Error } else { Code::Ok }
}
#[test]
fn original_compiled_command_component_restores_saved_metadata_before_refusal_settlement() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Actual command component and its displaced owning header, software-only.
    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for mode in [0, 1, 2, 4, 5] {
            let mut actual = fixture::native(profile);
            let _installed = fixture::install(&mut actual, 0);
            let _reset = Reset;
            actual.register_builtin(b"originalWordCommand", command_component);
            assert_eq!(actual.eval_str(b"proc p {} {return \"A[originalWordCommand]B\"}"), Code::Ok);
            assert_eq!(actual.eval_str(b"p"), Code::Ok);
            let artifact = cache(actual.proc_def(b"p").unwrap().body.as_ptr())
                .expect("actual original Command component artifact");
            let word = &artifact.scripts[&artifact.region].commands[0].words[1];
            actual.frames.borrow_mut().push(crate::namespace::GLOBAL);
            let parent = if mode == 5 {
                obj::Owned::fresh(obj::alloc_typed(&COMMAND_ROLE, 0))
            } else {
                fixture::string(b"ORIGINAL PARENT")
            };
            let parent_pointer = parent.as_ptr();
            let parent_lifetime = obj::NativeObjectLifetime::retain(parent_pointer);
            let mut frame = CmdFrame::root();
            frame.line = 77;
            frame.cmd = b"PARENT COMMAND".to_vec();
            frame.original_command = Some(parent);
            actual.cmd_frames.borrow_mut().push(frame);
            let scope = crate::interp::native_operation_currency::NativeOperationScope::enter(&actual).unwrap();
            FREES.set(0);
            REFS.set(-1);
            MODE.set(mode);
            let reached = actual.body_word(&artifact, word, &mut BodyExecution::default());
            if mode != 5 {
                let frames = actual.cmd_frames.borrow();
                let frame = frames.last().unwrap();
                assert_eq!(frame.line, 77);
                assert_eq!(frame.cmd, b"PARENT COMMAND");
                assert_eq!(frame.original_command.as_ref().unwrap().as_ptr(), parent_pointer);
            }
            assert_eq!(unsafe { (*parent_pointer).ref_count }, if mode == 5 { 0 } else { 1 });
            if mode == 5 {
                assert!(actual.cmd_frames.borrow().is_empty());
                assert!(!obj::allocation_is_live(parent_lifetime.as_ptr()));
            }
            assert_eq!(FREES.get(), if mode == 5 { 2 } else { 1 });
            assert_eq!(REFS.get(), 0);
            assert_eq!(obj::native_object_snapshot(actual.result_obj()).unwrap().resident.unwrap().as_ref(), b"C");
            if mode == 0 {
                assert_eq!(obj::bytes_of(reached.unwrap().as_ptr()), b"ACB");
                scope.currency().ensure_current_or_refuse().unwrap();
            } else {
                assert_eq!(reached.err(), Some(Code::Error));
                assert_first(&actual, if mode == 5 { 1 } else { mode });
                assert!(scope.currency().ensure_current_or_refuse().is_err());
            }
            MODE.set(0);
            let retired = actual.cmd_frames.borrow_mut().pop();
            drop(retired);
            actual.frames.borrow_mut().pop();
        }
    }
}
