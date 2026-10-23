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

//! Actual parent Return/Syntax stages and selected error-code retirement.
use super::*;
use crate::cmd_control::original_list_conversion_tests as fixture;
use crate::obj::TclObjType;
use std::cell::Cell;
use tcl_registry::native_return_options::NativeReturnOptionsApplication as Purpose;
use tcl_registry::special_vars::NativeErrorStorageVariable as Variable;
thread_local! {
    static MODE: Cell<u8> = const { Cell::new(0) };
    static GETTERS: Cell<usize> = const { Cell::new(0) };
    static FREES: Cell<usize> = const { Cell::new(0) };
    static INPUT: Cell<*mut TclObj> = const { Cell::new(core::ptr::null_mut()) };
    static INPUT_REFS: Cell<obj::TclSize> = const { Cell::new(-1) };
    static OLD_REFS: Cell<obj::TclSize> = const { Cell::new(-1) };
    static OLD_ROLE: Cell<bool> = const { Cell::new(false) };
}
fn change() {
    let mut actual = unsafe { crate::codegen_abi::current_interp().as_ref() }
        .unwrap()
        .clone();
    match MODE.get() {
        1 => {
            actual.refuse_host_command("original parent Return callback");
        }
        2 => {
            let mut frames = actual.frames.borrow_mut();
            assert!(frames.pop().is_some());
            frames.push(crate::namespace::GLOBAL);
        }
        4 => {
            let before = actual.runtime_context();
            let mut changed = before.clone();
            changed.packages = vec![("parent-return".to_owned(), "1".to_owned())];
            actual.pin_context(&changed).unwrap();
            actual.pin_context(&before).unwrap();
        }
        _ => {}
    }
}
extern "C" fn key(original: *mut TclObj) {
    GETTERS.set(GETTERS.get() + 1);
    change();
    unsafe {
        obj::set_native_updater_string_rep(original, b"-errorinfo", false);
    }
}
extern "C" fn incoming(original: *mut TclObj) {
    GETTERS.set(GETTERS.get() + 1);
    unsafe {
        obj::set_native_updater_string_rep(original, b"UNREACHED CODE", false);
    }
}
extern "C" fn retire(original: *mut TclObj) {
    FREES.set(FREES.get() + 1);
    OLD_REFS.set(unsafe { (*original).ref_count });
    INPUT_REFS.set(unsafe { (*INPUT.get()).ref_count });
    let actual = unsafe { crate::codegen_abi::current_interp().as_ref() }.unwrap();
    OLD_ROLE.set(actual.native_error_header_for_observation(Variable::ErrorCode) == Some(original));
    change();
}
static KEY: TclObjType = TclObjType {
    name: c"originalParentReturnKey".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(key),
    set_from_any_proc: None,
};
static OLD: TclObjType = TclObjType {
    name: c"originalParentReturnRetiredCode".as_ptr(),
    free_int_rep_proc: Some(retire),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
static INCOMING: TclObjType = TclObjType {
    name: c"originalParentReturnLazyCode".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(incoming),
    set_from_any_proc: None,
};
struct Reset;
impl Drop for Reset {
    fn drop(&mut self) {
        MODE.set(0);
        INPUT.set(core::ptr::null_mut());
    }
}
fn entered(profile: &str, syntax: bool) -> (Interp, Rc<NativeBodyArtifact>) {
    let mut actual = fixture::native(profile);
    let body = fixture::string(if syntax {
        b"{"
    } else {
        b"return -level 0 -code error -errorinfo INFO MSG"
    });
    actual.define_proc(b"p", Vec::new(), body.as_ptr());
    assert_eq!(actual.eval_str(b"p"), Code::Error);
    let artifact = cache(actual.proc_def(b"p").unwrap().body.as_ptr())
        .expect("genuine cached Return/Syntax artifact");
    actual.reset_original_c_result().unwrap();
    {
        let mut frames = actual.frames.borrow_mut();
        frames.push(crate::namespace::GLOBAL);
        frames
            .install_native_compiled_local_layout(artifact.compiled_local_layout().unwrap())
            .unwrap();
    }
    (actual, artifact)
}
fn original_options(actual: &Interp, pairs: &[(*mut TclObj, *mut TclObj)]) -> obj::Owned {
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
fn first(actual: &Interp, mode: u8) {
    let error = actual
        .native_execution_refusal()
        .expect("original parent cause retained through cleanup");
    if mode == 1 {
        assert!(
            matches!(error,tcl_runtime_api::NativeExecutionError::HostCommandRefusal(reason) if reason.reason=="original parent Return callback")
        );
    } else {
        assert!(matches!(
            error,
            tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(_)
        ));
    }
}
#[test]
fn original_parent_return_and_both_syntax_ports_keep_the_entered_frame_before_keys() {
    // naming.embedding.original-host-publication-and-fact-transport

    // Actual cached opcode/literal headers and callbacks; no external Native schedule.
    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for operation in 0..3 {
            for mode in [1, 2, 4] {
                let (mut actual, artifact) = entered(profile, operation != 0);
                let _installed = fixture::install(&mut actual, 0);
                let _reset = Reset;
                let options_index = if operation == 0 {
                    match &artifact.scripts[&artifact.region].commands[0].operation {
                        Operation::SelectedReturn(_, _, Some(index)) => *index,
                        _ => panic!("real immediate Return instruction"),
                    }
                } else {
                    artifact.scripts[&artifact.region]
                        .fatal
                        .as_ref()
                        .unwrap()
                        .options
                };
                let original = artifact.literals.original(options_index).unwrap();
                let key = obj::Owned::fresh(obj::alloc_typed(&KEY, 0));
                unsafe {
                    obj::set_native_updater_string_rep(key.as_ptr(), b"-errorinfo", false);
                }
                let info = fixture::string(b"INFO");
                {
                    let input = original_options(&actual, &[(key.as_ptr(), info.as_ptr())]);
                    obj::duplicate_into(input.as_ptr(), original);
                }
                obj::invalidate_string(key.as_ptr());
                let prior = fixture::string(b"PRIOR");
                actual.set_result(prior.as_ptr());
                let scope =
                    crate::interp::native_operation_currency::NativeOperationScope::enter(&actual)
                        .unwrap();
                GETTERS.set(0);
                MODE.set(mode);
                let mut execution = BodyExecution::default();
                let code = match operation {
                    0 => actual
                        .execute_body_opcode(
                            &artifact,
                            &artifact.scripts[&artifact.region].commands[0],
                            &mut execution,
                        )
                        .unwrap_or_else(|code| code),
                    1 => {
                        let fatal = artifact.scripts[&artifact.region].fatal.as_ref().unwrap();
                        actual.execute_body_syntax_objects(&artifact, fatal.message, fatal.options)
                    }
                    _ => actual.execute_body_region(&artifact, artifact.region, &mut execution),
                };
                assert_eq!(code, Code::Error);
                first(&actual, mode);
                assert!(scope.currency().ensure_current_or_refuse().is_err());
                assert_eq!(GETTERS.get(), 1);
                assert_eq!(actual.result_obj(), prior.as_ptr());
                assert!(actual.return_options.borrow().header().is_none());
                MODE.set(0);
                actual.frames.borrow_mut().pop();
            }
        }
    }
}
#[test]
fn original_compiled_errorcode_uses_selected_old_role_before_incoming_hold_without_capi_counter() {
    // naming.error.original-explicit-error-code-header-setter

    // Native614c supplies header-observer bounds only. These actual compiled
    // frame/Host/context and incoming-reference observations are software controls.
    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for mode in [0, 1, 2, 4] {
            let (mut actual, _artifact) = entered(profile, false);
            let _installed = fixture::install(&mut actual, 0);
            let _reset = Reset;
            let scope =
                crate::interp::native_operation_currency::NativeOperationScope::enter(&actual)
                    .unwrap();
            let code = obj::Owned::fresh(obj::alloc_typed(&INCOMING, 0));
            INPUT.set(code.as_ptr());
            let key = fixture::string(b"-errorcode");
            let options = original_options(&actual, &[(key.as_ptr(), code.as_ptr())]);
            let old = obj::Owned::fresh(obj::alloc_typed(&OLD, 0));
            actual
                .replace_native_error_object_checked(
                    scope.currency(),
                    Variable::ErrorCode,
                    Some(old),
                    None,
                )
                .unwrap();
            let original = native_list_conversion::BodyObjectOperation::capture(&actual).unwrap();
            let objects = original.objects(&actual).unwrap();
            let prior = fixture::string(b"PRIOR");
            actual.set_result(prior.as_ptr());
            let before = actual.c_api_errors.get();
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
            assert_eq!(FREES.get(), 1);
            assert_eq!(OLD_REFS.get(), 0);
            assert!(OLD_ROLE.get());
            assert_eq!(INPUT_REFS.get(), 2);
            assert_eq!(GETTERS.get(), 0);
            assert_eq!(actual.c_api_errors.get(), before);
            assert_eq!(actual.result_obj(), prior.as_ptr());
            if mode == 0 {
                assert_eq!(reached.unwrap(), Code::Error);
                assert_eq!(
                    actual.native_error_header_for_observation(Variable::ErrorCode),
                    Some(code.as_ptr())
                );
                assert_eq!(unsafe { (*code.as_ptr()).ref_count }, 3);
            } else {
                let error = reached
                    .err()
                    .expect("selected retirement stops incoming publication");
                first(&actual, mode);
                assert_eq!(
                    error.native_execution_refusal(),
                    actual.native_execution_refusal().as_ref()
                );
                assert!(actual
                    .native_error_header_for_observation(Variable::ErrorCode)
                    .is_none());
                assert_eq!(unsafe { (*code.as_ptr()).ref_count }, 2);
            }
            MODE.set(0);
            actual.frames.borrow_mut().pop();
        }
    }
}
#[test]
fn original_generic_field_publication_remains_distinct_from_selected_setter_retirement() {
    // naming.error.original-explicit-error-code-header-setter

    // Generic451c byte-author publication is independently exercised, not used
    // as a selected setter or Native614c timing equivalence.
    let (mut actual, _artifact) = entered("tcl8.6", false);
    let _installed = fixture::install(&mut actual, 0);
    let _reset = Reset;
    let scope =
        crate::interp::native_operation_currency::NativeOperationScope::enter(&actual).unwrap();
    let code = obj::Owned::fresh(obj::alloc_typed(&INCOMING, 0));
    INPUT.set(code.as_ptr());
    let key = fixture::string(b"-errorcode");
    let _options = original_options(&actual, &[(key.as_ptr(), code.as_ptr())]);
    let old = obj::Owned::fresh(obj::alloc_typed(&OLD, 0));
    actual
        .replace_native_error_object_checked(scope.currency(), Variable::ErrorCode, Some(old), None)
        .unwrap();
    GETTERS.set(0);
    FREES.set(0);
    actual
        .retain_native_error_option_checked(scope.currency(), false, code.as_ptr())
        .unwrap();
    assert_eq!(FREES.get(), 1);
    assert_eq!(OLD_REFS.get(), 0);
    assert!(!OLD_ROLE.get());
    assert_eq!(INPUT_REFS.get(), 3);
    assert_eq!(GETTERS.get(), 0);
    assert_eq!(
        actual.native_error_header_for_observation(Variable::ErrorCode),
        Some(code.as_ptr())
    );
    actual.frames.borrow_mut().pop();
}

extern "C" fn retire_result(original: *mut TclObj) {
    FREES.set(FREES.get() + 1);
    OLD_REFS.set(unsafe { (*original).ref_count });
    let actual = unsafe { crate::codegen_abi::current_interp().as_ref() }.unwrap();
    assert_ne!(actual.result_obj(), original);
    // Mark the genuine later metadata stage from the reached free callback.
    actual.exc.borrow_mut().already_logged = true;
    change();
}
static OLD_RESULT: TclObjType = TclObjType {
    name: c"originalParentReturnRetiredResult".as_ptr(),
    free_int_rep_proc: Some(retire_result),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
#[test]
fn original_parent_result_retirement_checks_frame_before_later_error_metadata() {
    // naming.embedding.original-host-publication-and-fact-transport

    // The prior result is held only by the actual result slot at its free.
    // This software callback observation grants no external Native chronology.
    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for operation in 0..3 {
            for mode in [1, 2, 4] {
                let (mut actual, artifact) = entered(profile, operation != 0);
                let _installed = fixture::install(&mut actual, 0);
                let _reset = Reset;
                let prior = obj::Owned::fresh(obj::alloc_typed(&OLD_RESULT, 0));
                actual.set_result(prior.as_ptr());
                drop(prior);
                let scope =
                    crate::interp::native_operation_currency::NativeOperationScope::enter(&actual)
                        .unwrap();
                FREES.set(0);
                OLD_REFS.set(-1);
                MODE.set(mode);
                let mut execution = BodyExecution::default();
                let expected = if operation == 0 {
                    let Operation::SelectedReturn(recipe, _, _) =
                        &artifact.scripts[&artifact.region].commands[0].operation
                    else {
                        panic!("actual selected Return");
                    };
                    artifact
                        .literals
                        .original(
                            recipe
                                .value_word
                                .map(|index| {
                                    artifact.scripts[&artifact.region].commands[0].words[index]
                                        .literal
                                        .unwrap()
                                })
                                .unwrap(),
                        )
                        .unwrap()
                } else {
                    artifact
                        .literals
                        .original(
                            artifact.scripts[&artifact.region]
                                .fatal
                                .as_ref()
                                .unwrap()
                                .message,
                        )
                        .unwrap()
                };
                let code = match operation {
                    0 => actual
                        .execute_body_opcode(
                            &artifact,
                            &artifact.scripts[&artifact.region].commands[0],
                            &mut execution,
                        )
                        .unwrap_or_else(|code| code),
                    1 => {
                        let fatal = artifact.scripts[&artifact.region].fatal.as_ref().unwrap();
                        actual.execute_body_syntax_objects(&artifact, fatal.message, fatal.options)
                    }
                    _ => actual.execute_body_region(&artifact, artifact.region, &mut execution),
                };
                assert_eq!(code, Code::Error);
                first(&actual, mode);
                assert!(scope.currency().ensure_current_or_refuse().is_err());
                assert_eq!(FREES.get(), 1);
                assert_eq!(OLD_REFS.get(), 0);
                assert_eq!(actual.result_obj(), expected);
                assert!(actual.exc.borrow().already_logged);
                assert!(!execution.done);
                MODE.set(0);
                actual.frames.borrow_mut().pop();
            }
        }
    }
}
