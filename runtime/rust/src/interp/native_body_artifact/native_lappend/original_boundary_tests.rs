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

//! Real compiled receiver conversion and checked argument-List controls.
use super::*;
use crate::cmd_control::original_list_conversion_tests as fixture;
use crate::obj::TclObjType;
use std::cell::RefCell;
thread_local! {
    static MODE: Cell<u8> = const { Cell::new(0) };
    static FREES: Cell<usize> = const { Cell::new(0) };
    static REFS: Cell<obj::TclSize> = const { Cell::new(-1) };
    static REPLACEMENT: Cell<*mut TclObj> = const { Cell::new(core::ptr::null_mut()) };
    static RECEIVER: RefCell<Option<Rc<OriginalCVariableCapture>>> = const { RefCell::new(None) };
}
extern "C" fn duplicate(_source: *mut TclObj, incoming: *mut TclObj) {
    obj::change_type(incoming, &ORIGINAL, 0);
}
extern "C" fn retire(pointer: *mut TclObj) {
    FREES.set(FREES.get() + 1);
    REFS.set(unsafe { (*pointer).ref_count });
    let mut actual = unsafe { crate::codegen_abi::current_interp().as_ref() }
        .unwrap()
        .clone();
    match MODE.get() {
        1 => {
            actual.refuse_host_command("original compiled Lappend retirement");
        }
        2 => {
            let before = actual.runtime_context();
            let mut changed = before.clone();
            changed.packages = vec![("original-lappend".to_owned(), "1".to_owned())];
            actual.pin_context(&changed).unwrap();
            actual.pin_context(&before).unwrap();
        }
        3 => {
            let mut frames = actual.frames.borrow_mut();
            assert!(frames.pop().is_some());
            frames.push(crate::namespace::GLOBAL);
        }
        4 => {
            RECEIVER.with(|receiver| {
                receiver
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
static ORIGINAL: TclObjType = TclObjType {
    name: c"originalCompiledLappendRetirement".as_ptr(),
    free_int_rep_proc: Some(retire),
    dup_int_rep_proc: Some(duplicate),
    update_string_proc: None,
    set_from_any_proc: None,
};
struct Reset;
impl Drop for Reset {
    fn drop(&mut self) {
        MODE.set(0);
        RECEIVER.with(|receiver| receiver.borrow_mut().take());
        REPLACEMENT.set(core::ptr::null_mut());
    }
}
fn exercise(profile: &str, mode: u8) {
    let mut actual = fixture::native(profile);
    assert_eq!(actual.eval_str(b"proc p {} {lappend x B}"), Code::Ok);
    assert_eq!(actual.eval_str(b"p"), Code::Ok);
    let artifact = cache(actual.proc_def(b"p").unwrap().body.as_ptr())
        .expect("actual compiled append artifact");
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
        NativeAppendOperands::ListElement | NativeAppendOperands::ListElements { .. }
    ));
    {
        let mut frames = actual.frames.borrow_mut();
        frames.push(crate::namespace::GLOBAL);
        frames
            .install_native_compiled_local_layout(artifact.compiled_local_layout().unwrap())
            .unwrap();
    }
    let _current = fixture::install(&mut actual, 0);
    let _reset = Reset;
    let evaluated = EvaluatedTarget {
        root: b"x".to_vec(),
        element: None,
        original_name: None,
        original_index: None,
        combined: false,
    };
    let input = obj::Owned::fresh(obj::alloc_typed(&ORIGINAL, 0));
    unsafe { obj::set_native_updater_string_rep(input.as_ptr(), b"A", false) };
    actual
        .body_store_value(&evaluated, target.slot, &input)
        .unwrap();
    let replacement = fixture::string(b"REPLACED");
    REPLACEMENT.set(replacement.as_ptr());
    RECEIVER.with(|receiver| {
        *receiver.borrow_mut() = Some(Rc::new(
            actual
                .body_capture_target(&evaluated, target.slot, true)
                .unwrap(),
        ))
    });
    actual.set_result_bytes(b"PRIOR");
    let prior = actual.get_obj_result();
    let scope =
        crate::interp::native_operation_currency::NativeOperationScope::enter(&actual).unwrap();
    FREES.set(0);
    REFS.set(-1);
    MODE.set(mode);
    let reached = actual.execute_body_append(
        &artifact,
        command,
        target,
        recipe,
        &mut BodyExecution::default(),
    );
    assert_eq!(FREES.get(), 1);
    assert_eq!(REFS.get(), 2);
    if mode == 0 {
        assert_eq!(reached, Ok(Code::Ok));
        assert_eq!(obj::bytes_of(actual.get_obj_result()).as_slice(), b"A B");
        scope.currency().ensure_current_or_refuse().unwrap();
    } else {
        assert_eq!(reached, Err(Code::Error));
        assert_eq!(actual.get_obj_result(), prior);
        assert!(scope.currency().ensure_current_or_refuse().is_err());
        let first = actual.native_execution_refusal().unwrap();
        if mode == 1 {
            assert!(
                matches!(first,tcl_runtime_api::NativeExecutionError::HostCommandRefusal(ref refusal) if refusal.reason=="original compiled Lappend retirement")
            );
        } else {
            assert!(matches!(
                first,
                tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(_)
            ));
        }
        if mode == 4 {
            RECEIVER.with(|receiver| {
                assert_eq!(
                    receiver.borrow().as_ref().unwrap().receiver.read_initial(),
                    Ok(Some(replacement.as_ptr()))
                )
            });
        }
    }
    MODE.set(0);
}
#[test]
fn original_compiled_lappend_checks_same_receiver_and_frame_before_list_adoption() {

    // Software extension callbacks; no external Native refcount chronology grant.
    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for mode in [1, 2, 3, 4] {
            exercise(profile, mode);
        }
    }
}
#[test]
fn original_compiled_lappend_keeps_current_conversion_and_real_native_roles() {

    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        exercise(profile, 0);
    }
}
#[test]
fn original_argument_list_concat_checks_source_free_before_target_mutation_and_full_range() {

    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for mode in [0, 1, 2, 3] {
            let mut actual = fixture::native(profile);
            actual.frames.borrow_mut().push(crate::namespace::GLOBAL);
            let _current = fixture::install(&mut actual, 0);
            let _reset = Reset;
            let target = fixture::string(b"A");
            let source = obj::Owned::fresh(obj::alloc_typed(&ORIGINAL, 0));
            unsafe { obj::set_native_updater_string_rep(source.as_ptr(), b"B", false) };
            let original = BodyObjectOperation::capture(&actual).unwrap();
            let headers =
                OriginalArguments::enter(&actual, &[target.as_ptr(), source.as_ptr()]).unwrap();
            FREES.set(0);
            REFS.set(-1);
            MODE.set(mode);
            let reached = crate::list::concatenate_native_lists_using(
                target.as_ptr(),
                source.as_ptr(),
                NativeStringProtocol::C(actual.native_invocation_dialect().tcl_version.unwrap()),
                &|| {
                    original.check(&actual)?;
                    headers.check_for(&actual)
                },
                &|error| original.refuse(error),
            );
            assert_eq!(FREES.get(), 1);
            assert_eq!(REFS.get(), 1);
            if mode == 0 {
                let result = reached.unwrap();
                assert_eq!(obj::bytes_of(result.as_ptr()).as_slice(), b"A B");
            } else {
                assert!(reached.is_err());
                assert!(original.check(&actual).is_err());
                assert_eq!(obj::bytes_of(target.as_ptr()).as_slice(), b"A");
            }
            MODE.set(0);
        }
    }
}

#[test]
fn original_neutral_list_helpers_keep_actual_alias_copy_and_reference_roles() {

    // Neutral alias/copy roles are software controls, without Native authority.
    let protocol = NativeStringProtocol::C(tcl_dialect::TclVersion::V9_1);
    let empty = fixture::string(b"");
    let range = crate::list::native_full_list_range(empty.as_ptr(), protocol).unwrap();
    assert_eq!(range.as_ptr(), empty.as_ptr());
    assert_eq!(unsafe { (*empty.as_ptr()).ref_count }, 2);
    drop(range);
    assert_eq!(unsafe { (*empty.as_ptr()).ref_count }, 1);
    let a = fixture::string(b"A");
    let b = fixture::string(b"B");
    let original = obj::Owned::fresh(crate::list::new_list_obj_native(&[a.as_ptr()], protocol));
    let appended =
        crate::list::append_native_list_elements(original.as_ptr(), &[b.as_ptr()], protocol)
            .unwrap();
    assert_eq!(appended.as_ptr(), original.as_ptr());
    assert_eq!(unsafe { (*original.as_ptr()).ref_count }, 2);
    assert_eq!(
        crate::list::list_elements_native_checked(appended.as_ptr(), protocol).unwrap(),
        [a.as_ptr(), b.as_ptr()]
    );
    assert_eq!(unsafe { (*a.as_ptr()).ref_count }, 2);
    assert_eq!(unsafe { (*b.as_ptr()).ref_count }, 2);
    let copied = crate::list::native_full_list_range(original.as_ptr(), protocol).unwrap();
    assert_ne!(copied.as_ptr(), original.as_ptr());
    assert_eq!(unsafe { (*copied.as_ptr()).ref_count }, 1);
    assert_eq!(unsafe { (*original.as_ptr()).ref_count }, 2);
    assert_eq!(
        crate::list::list_elements_native_checked(copied.as_ptr(), protocol).unwrap(),
        [a.as_ptr(), b.as_ptr()]
    );
}
