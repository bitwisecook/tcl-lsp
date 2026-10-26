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

//! Real ReturnError workers retain compiled frame, options and private-field roles.
use super::*;
use crate::cmd_control::original_list_conversion_tests as fixture;
use crate::obj::TclObjType;
use std::cell::{Cell, RefCell};
use tcl_registry::native_return_options::NativeReturnOptionsApplication as Purpose;
thread_local! {
    static ORIGINAL_CURRENCY: RefCell<Option<crate::interp::native_operation_currency::NativeOperationCurrency>> = const { RefCell::new(None) };
    static MODE: Cell<u8> = const { Cell::new(0) };
    static GETTERS: Cell<usize> = const { Cell::new(0) };
    static LATER: Cell<usize> = const { Cell::new(0) };
    static FREES: Cell<usize> = const { Cell::new(0) };
    static REFS: Cell<obj::TclSize> = const { Cell::new(-1) };
}
fn change() {
    let mut actual = unsafe { crate::codegen_abi::current_interp().as_ref() }
        .expect("actual installed compiled interpreter")
        .clone();
    match MODE.get() {
        1 => {
            actual.refuse_host_command("original ReturnError reached getter");
        }
        2 => {
            let mut frames = actual.frames.borrow_mut();
            assert!(frames.pop().is_some());
            frames.push(crate::namespace::GLOBAL);
        }
        3 => {
            actual.clear_return_options_checked().unwrap();
        }
        4 => {
            let before = actual.runtime_context();
            let mut next = before.clone();
            next.packages = vec![("original-return-frame".to_owned(), "1".to_owned())];
            actual.pin_context(&next).unwrap();
            actual.pin_context(&before).unwrap();
        }
        _ => {}
    }
}
fn observed(original: *mut TclObj) {
    GETTERS.set(GETTERS.get() + 1);
    REFS.set(unsafe { (*original).ref_count });
    change();
}
extern "C" fn key(original: *mut TclObj) {
    observed(original);
    unsafe {
        obj::set_native_updater_string_rep(original, b"-errorinfo", false);
    }
}
extern "C" fn info(original: *mut TclObj) {
    observed(original);
    if MODE.get() == 5 {
        let actual = unsafe { crate::codegen_abi::current_interp().as_ref() }
            .unwrap()
            .clone();
        let retired = obj::Owned::fresh(obj::alloc_typed(&RETIRED_FIELD, 0));
        ORIGINAL_CURRENCY
            .with(|saved| {
                actual.replace_native_error_object_checked(
                    saved.borrow().as_ref().expect("actual original scope"),
                    tcl_registry::special_vars::NativeErrorStorageVariable::ErrorInfo,
                    Some(retired),
                    None,
                )
            })
            .unwrap();
    }
    unsafe {
        obj::set_native_updater_string_rep(original, b"INFO", false);
    }
}
extern "C" fn stack(original: *mut TclObj) {
    observed(original);
    unsafe {
        obj::set_native_updater_string_rep(original, b"INNER COMMAND", false);
    }
}
extern "C" fn later(original: *mut TclObj) {
    LATER.set(LATER.get() + 1);
    unsafe {
        obj::set_native_updater_string_rep(original, b"-errorcode", false);
    }
}
extern "C" fn free(_original: *mut TclObj) {
    FREES.set(FREES.get() + 1);
}
extern "C" fn retire_field(_original: *mut TclObj) {
    FREES.set(FREES.get() + 1);
    let actual = unsafe { crate::codegen_abi::current_interp().as_ref() }.unwrap();
    let mut frames = actual.frames.borrow_mut();
    assert!(frames.pop().is_some());
    frames.push(crate::namespace::GLOBAL);
}
static RETIRED_FIELD: TclObjType = TclObjType {
    name: c"originalReturnDisplacedInfo".as_ptr(),
    free_int_rep_proc: Some(retire_field),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
static KEY: TclObjType = TclObjType {
    name: c"originalReturnKeyFrame".as_ptr(),
    free_int_rep_proc: Some(free),
    dup_int_rep_proc: None,
    update_string_proc: Some(key),
    set_from_any_proc: None,
};
static INFO: TclObjType = TclObjType {
    name: c"originalReturnInfoFrame".as_ptr(),
    free_int_rep_proc: Some(free),
    dup_int_rep_proc: None,
    update_string_proc: Some(info),
    set_from_any_proc: None,
};
static STACK: TclObjType = TclObjType {
    name: c"originalReturnStackFrame".as_ptr(),
    free_int_rep_proc: Some(free),
    dup_int_rep_proc: None,
    update_string_proc: Some(stack),
    set_from_any_proc: None,
};
static LATER_KEY: TclObjType = TclObjType {
    name: c"originalReturnLaterKey".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(later),
    set_from_any_proc: None,
};
struct Reset;
impl Drop for Reset {
    fn drop(&mut self) {
        MODE.set(0);
        ORIGINAL_CURRENCY.with(|saved| saved.borrow_mut().take());
    }
}
fn entered(profile: &str) -> Interp {
    let mut actual = fixture::native(profile);
    assert_eq!(actual.eval_str(b"proc p {} {error E I C}"), Code::Ok);
    assert_eq!(actual.eval_str(b"p"), Code::Error);
    let procedure = actual.proc_def(b"p").unwrap();
    let artifact = cache(procedure.body.as_ptr()).expect("genuine compiled Error artifact");
    if actual.runtime_version() == tcl_dialect::TclVersion::V8_5 {
        let script = artifact.scripts.get(&artifact.region).unwrap();
        assert_eq!(script.commands.len(), 1);
        assert!(matches!(&script.commands[0].operation, Operation::Invoke));
        assert!(!artifact
            .scripts
            .values()
            .flat_map(|script| &script.commands)
            .any(|command| matches!(&command.operation, Operation::Error(_))));
    } else {
        assert!(artifact
            .scripts
            .values()
            .flat_map(|script| &script.commands)
            .any(|command| matches!(&command.operation, Operation::Error(_))));
    }
    actual.reset_original_c_result().unwrap();
    {
        let mut frames = actual.frames.borrow_mut();
        frames.push(crate::namespace::GLOBAL);
        frames
            .install_native_compiled_local_layout(artifact.compiled_local_layout().unwrap())
            .unwrap();
    }
    actual
}
fn options(actual: &Interp, pairs: &[(*mut TclObj, *mut TclObj)]) -> obj::Owned {
    obj::Owned::fresh(
        crate::dict::new_dict_obj_native(
            pairs,
            None,
            actual
                .native_invocation_dialect()
                .native_string_protocol()
                .unwrap(),
        )
        .unwrap(),
    )
}
fn first(actual: &Interp, error: &tcl_cmd_core::CmdError, mode: u8) {
    let cause = actual
        .native_execution_refusal()
        .expect("retained original full cause");
    assert_eq!(error.native_execution_refusal(), Some(&cause));
    if mode == 1 {
        assert!(
            matches!(cause, tcl_runtime_api::NativeExecutionError::HostCommandRefusal(reason)
            if reason.reason == "original ReturnError reached getter")
        );
    } else {
        assert!(matches!(
            cause,
            tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(_)
        ));
    }
}
#[test]
fn original_return_options_key_stops_on_frame_host_context_and_carried_record_change() {

    // Software frame/header/callback observations; no external Native schedule.
    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for mode in [1, 2, 3, 4] {
            let mut actual = entered(profile);
            let _installed = fixture::install(&mut actual, 0);
            let _reset = Reset;
            MODE.set(0);
            let key = obj::Owned::fresh(obj::alloc_typed(&KEY, 0));
            let later = obj::Owned::fresh(obj::alloc_typed(&LATER_KEY, 0));
            unsafe {
                obj::set_native_updater_string_rep(key.as_ptr(), b"-errorinfo", false);
                obj::set_native_updater_string_rep(later.as_ptr(), b"-errorcode", false);
            }
            let value = fixture::string(b"INFO");
            let code = fixture::string(b"CODE");
            let options = options(
                &actual,
                &[
                    (key.as_ptr(), value.as_ptr()),
                    (later.as_ptr(), code.as_ptr()),
                ],
            );
            obj::invalidate_string(key.as_ptr());
            obj::invalidate_string(later.as_ptr());
            GETTERS.set(0);
            LATER.set(0);
            FREES.set(0);
            let prior = fixture::string(b"PRIOR");
            actual.set_result(prior.as_ptr());
            let original = BodyObjectOperation::capture(&actual).unwrap();
            let objects = original.objects(&actual).unwrap();
            MODE.set(mode);
            let error = actual
                .process_original_c_return_options_with_objects(
                    Purpose::Immediate,
                    1,
                    0,
                    options.as_ptr(),
                    &objects,
                )
                .err()
                .unwrap_or_else(|| {
                    panic!("original callback stops before stale options publication")
                });
            first(&actual, &error, mode);
            assert_eq!(GETTERS.get(), 1);
            assert_eq!(LATER.get(), 0);
            assert_eq!(REFS.get(), 2);
            assert!(actual.return_options.borrow().header().is_none());
            assert_eq!(actual.result_obj(), prior.as_ptr());
            assert_eq!(FREES.get(), 0);
            MODE.set(0);
            actual.frames.borrow_mut().pop();
        }
    }
}
#[test]
fn original_return_info_string_keeps_frame_and_private_role_before_second_getter() {

    // Genuine borrowed value String; no extra protective Native reference.
    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for mode in [0, 1, 2, 4] {
            let mut actual = entered(profile);
            let _installed = fixture::install(&mut actual, 0);
            let _reset = Reset;
            let key = fixture::string(b"-errorinfo");
            let value = obj::Owned::fresh(obj::alloc_typed(&INFO, 0));
            let options = options(&actual, &[(key.as_ptr(), value.as_ptr())]);
            let original = BodyObjectOperation::capture(&actual).unwrap();
            let objects = original.objects(&actual).unwrap();
            let prior = fixture::string(b"PRIOR");
            actual.set_result(prior.as_ptr());
            GETTERS.set(0);
            FREES.set(0);
            MODE.set(mode);
            let reached = actual.process_original_c_return_options_with_objects(
                Purpose::Immediate,
                1,
                0,
                options.as_ptr(),
                &objects,
            );
            assert_eq!(GETTERS.get(), 1);
            assert_eq!(REFS.get(), 2);
            assert_eq!(actual.result_obj(), prior.as_ptr());
            if mode == 0 {
                assert_eq!(reached.unwrap(), Code::Error);
                assert_eq!(
                    actual
                        .exc
                        .borrow()
                        .native
                        .info
                        .as_ref()
                        .map(obj::Owned::as_ptr),
                    Some(value.as_ptr())
                );
                assert_eq!(actual.exc.borrow().native.info_len, 4);
                assert!(actual.native_execution_refusal().is_none());
            } else {
                let error = reached
                    .err()
                    .unwrap_or_else(|| panic!("frame/Host callback blocks private info adoption"));
                first(&actual, &error, mode);
                assert!(actual.exc.borrow().native.info.is_none());
            }
            assert_eq!(FREES.get(), 0);
            MODE.set(0);
            actual.frames.borrow_mut().pop();
        }
    }
}
#[test]
fn original_return_stack_string_refuses_before_list_backing_and_private_stack_adoption() {

    // Genuine modern private List stack and callback; C84/C85 have no such owner.
    for profile in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        for mode in [1, 2, 4] {
            let mut actual = entered(profile);
            let _installed = fixture::install(&mut actual, 0);
            let _reset = Reset;
            let key = fixture::string(b"-errorstack");
            let value = obj::Owned::fresh(obj::alloc_typed(&STACK, 0));
            let options = options(&actual, &[(key.as_ptr(), value.as_ptr())]);
            let original = BodyObjectOperation::capture(&actual).unwrap();
            let objects = original.objects(&actual).unwrap();
            let before = actual.error_stack.borrow().original_header();
            let prior = fixture::string(b"PRIOR");
            actual.set_result(prior.as_ptr());
            GETTERS.set(0);
            FREES.set(0);
            MODE.set(mode);
            let error = actual
                .process_original_c_return_options_with_objects(
                    Purpose::Immediate,
                    1,
                    0,
                    options.as_ptr(),
                    &objects,
                )
                .err()
                .unwrap_or_else(|| {
                    panic!("original stack conversion callback blocks backing adoption")
                });
            first(&actual, &error, mode);
            assert_eq!(GETTERS.get(), 1);
            assert_eq!(REFS.get(), 2);
            assert_eq!(FREES.get(), 0);
            assert!(core::ptr::eq(obj::obj_type_ptr(value.as_ptr()), &STACK));
            assert_eq!(actual.error_stack.borrow().original_header(), before);
            assert_eq!(actual.result_obj(), prior.as_ptr());
            MODE.set(0);
            actual.frames.borrow_mut().pop();
        }
    }
}

#[test]
fn original_return_info_retirement_keeps_reached_field_and_blocks_later_getter() {

    // Actual private-field publication/retirement and same-depth frame withdrawal;
    // this software control grants no selected Native setter chronology.
    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut actual = entered(profile);
        let _installed = fixture::install(&mut actual, 0);
        let _reset = Reset;
        let scope =
            crate::interp::native_operation_currency::NativeOperationScope::enter(&actual).unwrap();
        ORIGINAL_CURRENCY.with(|saved| *saved.borrow_mut() = Some(scope.currency().clone()));
        let key = fixture::string(b"-errorinfo");
        let value = obj::Owned::fresh(obj::alloc_typed(&INFO, 0));
        let options = options(&actual, &[(key.as_ptr(), value.as_ptr())]);
        let original = BodyObjectOperation::capture(&actual).unwrap();
        let objects = original.objects(&actual).unwrap();
        let prior = fixture::string(b"PRIOR");
        actual.set_result(prior.as_ptr());
        GETTERS.set(0);
        FREES.set(0);
        MODE.set(5);
        let error = actual
            .process_original_c_return_options_with_objects(
                Purpose::Immediate,
                1,
                0,
                options.as_ptr(),
                &objects,
            )
            .err()
            .unwrap_or_else(|| {
                panic!("private owner retirement blocks continuation under original frame")
            });
        first(&actual, &error, 2);
        assert_eq!(GETTERS.get(), 1);
        assert_eq!(REFS.get(), 2);
        assert_eq!(FREES.get(), 1);
        assert_eq!(
            actual
                .exc
                .borrow()
                .native
                .info
                .as_ref()
                .map(obj::Owned::as_ptr),
            Some(value.as_ptr())
        );
        assert_eq!(actual.exc.borrow().native.info_len, 0);
        assert_eq!(actual.result_obj(), prior.as_ptr());
        MODE.set(0);
        actual.frames.borrow_mut().pop();
    }
}
