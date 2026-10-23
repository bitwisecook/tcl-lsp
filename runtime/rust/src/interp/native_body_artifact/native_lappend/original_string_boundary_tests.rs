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

//! Genuine original compiled String append stages and the separate neutral worker.
use super::*;
use crate::cmd_control::original_list_conversion_tests as fixture;
use crate::obj::TclObjType;
use std::cell::RefCell;
thread_local! {
    static MODE: Cell<u8> = const { Cell::new(0) };
    static FREES: Cell<usize> = const { Cell::new(0) };
    static GETTERS: Cell<usize> = const { Cell::new(0) };
    static REFS: Cell<obj::TclSize> = const { Cell::new(-1) };
    static RECEIVER: RefCell<Option<Rc<OriginalCVariableCapture>>> = const { RefCell::new(None) };
    static REPLACEMENT: Cell<*mut TclObj> = const { Cell::new(core::ptr::null_mut()) };
}
fn change() {
    let mut actual = unsafe { crate::codegen_abi::current_interp().as_ref() }
        .unwrap()
        .clone();
    match MODE.get() {
        1 => {
            actual.refuse_host_command("original compiled String append callback");
        }
        2 => {
            let before = actual.runtime_context();
            let mut changed = before.clone();
            changed.packages = vec![("original-string-append".to_owned(), "1".to_owned())];
            actual.pin_context(&changed).unwrap();
            actual.pin_context(&before).unwrap();
        }
        3 => {
            let mut frames = actual.frames.borrow_mut();
            assert!(frames.pop().is_some());
            frames.push(crate::namespace::GLOBAL);
        }
        4 => {
            RECEIVER.with(|owner| {
                owner
                    .borrow()
                    .as_ref()
                    .unwrap()
                    .receiver
                    .store(REPLACEMENT.get())
                    .unwrap()
            });
        }
        _ => {}
    }
}
extern "C" fn duplicate(_original: *mut TclObj, incoming: *mut TclObj) {
    obj::change_type(incoming, &ORIGINAL, 0);
}
extern "C" fn retire(original: *mut TclObj) {
    FREES.set(FREES.get() + 1);
    REFS.set(unsafe { (*original).ref_count });
    change();
}
extern "C" fn source(original: *mut TclObj) {
    GETTERS.set(GETTERS.get() + 1);
    REFS.set(unsafe { (*original).ref_count });
    change();
    unsafe {
        obj::set_native_updater_string_rep(original, b"B", false);
    }
}
static ORIGINAL: TclObjType = TclObjType {
    name: c"originalCompiledStringAppendRetirement".as_ptr(),
    free_int_rep_proc: Some(retire),
    dup_int_rep_proc: Some(duplicate),
    update_string_proc: None,
    set_from_any_proc: None,
};
static SOURCE: TclObjType = TclObjType {
    name: c"originalCompiledStringAppendSource".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(source),
    set_from_any_proc: None,
};
struct Reset;
impl Drop for Reset {
    fn drop(&mut self) {
        MODE.set(0);
        RECEIVER.with(|owner| owner.borrow_mut().take());
        REPLACEMENT.set(core::ptr::null_mut());
    }
}
fn expected_first(actual: &Interp, mode: u8) {
    let first = actual
        .native_execution_refusal()
        .expect("actual original String append first cause");
    if mode == 1 {
        assert!(
            matches!(first,tcl_runtime_api::NativeExecutionError::HostCommandRefusal(ref reason)
            if reason.reason == "original compiled String append callback")
        );
    } else {
        assert!(matches!(
            first,
            tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(_)
        ));
    }
}
fn exercise(profile: &str, mode: u8, source_callback: bool) {
    let mut actual = fixture::native(profile);
    let text: &[u8] = if source_callback {
        b"proc p {} {append x $y}"
    } else {
        b"proc p {} {append x B}"
    };
    assert_eq!(actual.eval_str(text), Code::Ok);
    assert_eq!(
        actual.eval_str(b"p"),
        if source_callback {
            Code::Error
        } else {
            Code::Ok
        }
    );
    let artifact =
        cache(actual.proc_def(b"p").unwrap().body.as_ptr()).expect("actual String append artifact");
    let command = artifact.scripts[&artifact.region]
        .commands
        .iter()
        .find(|command| matches!(command.operation, Operation::Append(..)))
        .unwrap();
    let Operation::Append(target, recipe) = &command.operation else {
        unreachable!()
    };
    assert!(matches!(
        recipe.operands,
        NativeAppendOperands::StringObjects
    ));
    {
        let mut frames = actual.frames.borrow_mut();
        frames.push(crate::namespace::GLOBAL);
        frames
            .install_native_compiled_local_layout(artifact.compiled_local_layout().unwrap())
            .unwrap();
    }
    let _installed = fixture::install(&mut actual, 0);
    let _reset = Reset;
    let evaluated = EvaluatedTarget {
        root: b"x".to_vec(),
        element: None,
        original_name: None,
        original_index: None,
        combined: false,
    };
    let input = if source_callback {
        fixture::string(b"A")
    } else {
        let input = obj::Owned::fresh(obj::alloc_typed(&ORIGINAL, 0));
        unsafe {
            obj::set_native_updater_string_rep(input.as_ptr(), b"A", false);
        }
        input
    };
    actual
        .body_store_value(&evaluated, target.slot, &input)
        .unwrap();
    let y = obj::Owned::fresh(obj::alloc_typed(&SOURCE, 0));
    if source_callback {
        let slot = command.words[recipe.values.start]
            .arena
            .locals
            .iter()
            .find_map(|(span, slot)| {
                (command.words[recipe.values.start]
                    .arena
                    .original
                    .bytes(*span)
                    == Some(b"y".as_slice()))
                .then_some(*slot)
            })
            .expect("actual source compiled local");
        let target = EvaluatedTarget {
            root: b"y".to_vec(),
            element: None,
            original_name: None,
            original_index: None,
            combined: false,
        };
        actual.body_store_value(&target, Some(slot), &y).unwrap();
    }
    RECEIVER.with(|owner| {
        *owner.borrow_mut() = Some(Rc::new(
            actual
                .body_capture_target(&evaluated, target.slot, true)
                .unwrap(),
        ))
    });
    let replacement = fixture::string(b"REACHED REPLACEMENT");
    REPLACEMENT.set(replacement.as_ptr());
    let prior = fixture::string(b"PRIOR");
    actual.set_result(prior.as_ptr());
    let scope =
        crate::interp::native_operation_currency::NativeOperationScope::enter(&actual).unwrap();
    FREES.set(0);
    GETTERS.set(0);
    REFS.set(-1);
    MODE.set(mode);
    let reached = actual.execute_body_append(
        &artifact,
        command,
        target,
        recipe,
        &mut BodyExecution::default(),
    );
    if source_callback {
        assert_eq!(GETTERS.get(), 1);
        assert_eq!(REFS.get(), 3);
    } else {
        assert_eq!(FREES.get(), 1);
        assert_eq!(REFS.get(), 1);
    }
    if mode == 0 {
        assert_eq!(reached, Ok(Code::Ok));
        assert_eq!(obj::bytes_of(actual.result_obj()), b"AB");
        assert_eq!(obj::bytes_of(input.as_ptr()), b"A");
        scope.currency().ensure_current_or_refuse().unwrap();
    } else {
        assert_eq!(reached, Err(Code::Error));
        assert_eq!(actual.result_obj(), prior.as_ptr());
        expected_first(&actual, mode);
        assert!(scope.currency().ensure_current_or_refuse().is_err());
        if mode == 4 {
            RECEIVER.with(|owner| {
                assert_eq!(
                    owner.borrow().as_ref().unwrap().receiver.read_initial(),
                    Ok(Some(replacement.as_ptr()))
                )
            });
        }
    }
    MODE.set(0);
    RECEIVER.with(|owner| owner.borrow_mut().take());
    actual.frames.borrow_mut().pop();
}
#[test]
fn original_compiled_string_append_checks_actual_cow_retirement_before_cache_adoption() {
    // naming.embedding.original-host-publication-and-fact-transport

    // Actual duplicate working +1 and original cell/caller roles; software-only.
    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for mode in [0, 1, 2, 3, 4] {
            exercise(profile, mode, false);
        }
    }
}
#[test]
fn original_compiled_string_append_checks_source_getter_before_mutation_and_publication() {
    // naming.embedding.original-host-publication-and-fact-transport

    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for mode in [0, 1, 2, 3, 4] {
            exercise(profile, mode, true);
        }
    }
}
#[test]
fn original_neutral_append_keeps_authored_worker_without_native_entry_or_extra_hold() {
    // naming.embedding.original-host-publication-and-fact-transport

    // Authored neutral purpose remains independent of compiled Native admission.
    let mut actual = Interp::new();
    let first = fixture::string(b"A");
    let second = fixture::string(b"B");
    assert_eq!(actual.entered_native_operation_count(), 0);
    assert_eq!(
        actual.append_native_operands(b"x", b"x", None, &[first.as_ptr(), second.as_ptr()]),
        Code::Ok
    );
    assert_eq!(obj::bytes_of(actual.result_obj()), b"AB");
    assert_eq!(actual.entered_native_operation_count(), 0);
    assert_eq!(unsafe { (*first.as_ptr()).ref_count }, 1);
    assert_eq!(unsafe { (*second.as_ptr()).ref_count }, 1);
}
