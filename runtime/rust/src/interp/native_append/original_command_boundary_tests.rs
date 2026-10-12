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

//! Actual ordinary append Name/frame and selected source stages.
use super::*;
use crate::cmd_control::original_list_conversion_tests as fixture;
use crate::obj;
use std::cell::Cell;
thread_local! {
    static MODE: Cell<u8> = const { Cell::new(0) };
    static NAME_GETTERS: Cell<usize> = const { Cell::new(0) };
    static SOURCE_GETTERS: Cell<usize> = const { Cell::new(0) };
    static LATER_GETTERS: Cell<usize> = const { Cell::new(0) };
    static REFS: Cell<obj::TclSize> = const { Cell::new(-1) };
    static REPLACEMENT: Cell<*mut TclObj> = const { Cell::new(core::ptr::null_mut()) };
}
fn change() {
    let mut actual = unsafe { crate::codegen_abi::current_interp().as_ref() }
        .unwrap()
        .clone();
    match MODE.get() {
        1 => {
            actual.refuse_host_command("original command append getter");
        }
        2 => {
            let before = actual.runtime_context();
            let mut changed = before.clone();
            changed.packages = vec![("original-command-append".into(), "1".into())];
            actual.pin_context(&changed).unwrap();
            actual.pin_context(&before).unwrap();
        }
        3 => {
            let mut frames = actual.frames.borrow_mut();
            assert!(frames.pop().is_some());
            frames.push(crate::namespace::GLOBAL);
        }
        4 => {
            actual.var_set(b"x", REPLACEMENT.get()).unwrap();
        }
        _ => {}
    }
}
extern "C" fn name(original: *mut TclObj) {
    NAME_GETTERS.set(NAME_GETTERS.get() + 1);
    REFS.set(unsafe { (*original).ref_count });
    unsafe {
        obj::set_native_updater_string_rep(original, b"x", false);
    }
    change();
}
extern "C" fn source(original: *mut TclObj) {
    SOURCE_GETTERS.set(SOURCE_GETTERS.get() + 1);
    REFS.set(unsafe { (*original).ref_count });
    unsafe {
        obj::set_native_updater_string_rep(original, b"B", false);
    }
    change();
}
extern "C" fn later(original: *mut TclObj) {
    LATER_GETTERS.set(LATER_GETTERS.get() + 1);
    unsafe {
        obj::set_native_updater_string_rep(original, b"C", false);
    }
}
static NAME: obj::TclObjType = obj::TclObjType {
    name: c"originalCommandAppendName".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(name),
    set_from_any_proc: None,
};
static SOURCE: obj::TclObjType = obj::TclObjType {
    name: c"originalCommandAppendSource".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(source),
    set_from_any_proc: None,
};
static LATER: obj::TclObjType = obj::TclObjType {
    name: c"originalCommandAppendLater".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(later),
    set_from_any_proc: None,
};
struct Reset;
impl Drop for Reset {
    fn drop(&mut self) {
        MODE.set(0);
        REPLACEMENT.set(core::ptr::null_mut());
    }
}
fn assert_first(actual: &Interp, mode: u8) {
    let first = actual
        .native_execution_refusal()
        .expect("actual original append first cause");
    if mode == 1 {
        assert!(
            matches!(first,tcl_runtime_api::NativeExecutionError::HostCommandRefusal(ref cause) if cause.reason=="original command append getter")
        );
    } else {
        assert!(matches!(
            first,
            tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(_)
        ));
    }
}
fn reset() {
    MODE.set(0);
    NAME_GETTERS.set(0);
    SOURCE_GETTERS.set(0);
    LATER_GETTERS.set(0);
    REFS.set(-1);
}
#[test]
fn original_command_append_captures_frame_before_name_and_blocks_later_sources() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Software original frame/header checks; no Native private callback chronology.
    for profile in ["tcl8.6", "tcl9.1"] {
        for mode in [0, 1, 2, 3] {
            let mut actual = fixture::native(profile);
            actual.frames.borrow_mut().push(crate::namespace::GLOBAL);
            let _installed = fixture::install(&mut actual, 0);
            let _reset = Reset;
            reset();
            let initial = fixture::string(b"A");
            actual.var_set(b"x", initial.as_ptr()).unwrap();
            let command = fixture::string(b"append");
            let name = obj::Owned::fresh(obj::alloc_typed(&NAME, 0));
            let source = obj::Owned::fresh(obj::alloc_typed(&SOURCE, 0));
            let prior = fixture::string(b"PRIOR");
            actual.set_result(prior.as_ptr());
            let scope =
                super::super::native_operation_currency::NativeOperationScope::enter(&actual)
                    .unwrap();
            MODE.set(mode);
            let reached = crate::cmd_string::append(
                &mut actual,
                &[command.as_ptr(), name.as_ptr(), source.as_ptr()],
            );
            assert_eq!(NAME_GETTERS.get(), 1);
            assert_eq!(REFS.get(), 1);
            assert_eq!(SOURCE_GETTERS.get(), usize::from(mode == 0));
            if mode == 0 {
                assert_eq!(reached, Code::Ok);
                assert_eq!(obj::bytes_of(actual.result_obj()).as_slice(), b"AB");
                scope.currency().ensure_current_or_refuse().unwrap();
            } else {
                assert_eq!(reached, Code::Error);
                assert_first(&actual, mode);
                assert!(scope.currency().ensure_current_or_refuse().is_err());
                assert_eq!(actual.result_obj(), prior.as_ptr());
                assert_eq!(obj::bytes_of(initial.as_ptr()).as_slice(), b"A");
            }
            MODE.set(0);
            actual.frames.borrow_mut().pop();
        }
    }
}
#[test]
fn original_command_append_checks_selected_cell_inside_source_getter_before_later_operand() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for profile in ["tcl8.6", "tcl9.1"] {
        for mode in [0, 1, 2, 3, 4] {
            let mut actual = fixture::native(profile);
            actual.frames.borrow_mut().push(crate::namespace::GLOBAL);
            let _installed = fixture::install(&mut actual, 0);
            let _reset = Reset;
            reset();
            let initial = fixture::string(b"A");
            actual.var_set(b"x", initial.as_ptr()).unwrap();
            let replacement = fixture::string(b"REACHED REPLACEMENT");
            REPLACEMENT.set(replacement.as_ptr());
            let command = fixture::string(b"append");
            let name = fixture::string(b"x");
            let source = obj::Owned::fresh(obj::alloc_typed(&SOURCE, 0));
            let later = obj::Owned::fresh(obj::alloc_typed(&LATER, 0));
            let prior = fixture::string(b"PRIOR");
            actual.set_result(prior.as_ptr());
            let scope =
                super::super::native_operation_currency::NativeOperationScope::enter(&actual)
                    .unwrap();
            MODE.set(mode);
            let reached = crate::cmd_string::append(
                &mut actual,
                &[
                    command.as_ptr(),
                    name.as_ptr(),
                    source.as_ptr(),
                    later.as_ptr(),
                ],
            );
            assert_eq!(SOURCE_GETTERS.get(), 1);
            assert_eq!(REFS.get(), 1);
            assert_eq!(LATER_GETTERS.get(), usize::from(mode == 0));
            if mode == 0 {
                assert_eq!(reached, Code::Ok);
                assert_eq!(obj::bytes_of(actual.result_obj()).as_slice(), b"ABC");
                scope.currency().ensure_current_or_refuse().unwrap();
            } else {
                assert_eq!(reached, Code::Error);
                assert_first(&actual, mode);
                assert!(scope.currency().ensure_current_or_refuse().is_err());
                assert_eq!(actual.result_obj(), prior.as_ptr());
                assert_eq!(obj::bytes_of(initial.as_ptr()).as_slice(), b"A");
                if mode == 4 {
                    assert_eq!(actual.var_get(b"x"), Some(replacement.as_ptr()));
                }
            }
            MODE.set(0);
            actual.frames.borrow_mut().pop();
        }
    }
}
#[test]
fn original_command_append_keeps_explicit_logical_simulation_and_read_only_purpose() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut actual = Interp::new();
    let command = fixture::string(b"append");
    let name = fixture::string(b"x");
    let value = fixture::string(b"A");
    let issued = actual
        .native_invocation_dialect()
        .native_object_append_protocol(Some(
            tcl_registry::native_object_append::LogicalAppendProvider::Tcl84CoreSimulation,
        ))
        .unwrap();
    assert!(issued.logical_provider().is_some());
    assert_eq!(actual.entered_native_operation_count(), 0);
    assert_eq!(
        crate::cmd_string::append(
            &mut actual,
            &[command.as_ptr(), name.as_ptr(), value.as_ptr()]
        ),
        Code::Ok
    );
    assert_eq!(obj::bytes_of(actual.result_obj()).as_slice(), b"A");
    assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 1);
    assert_eq!(actual.entered_native_operation_count(), 0);
    let before = actual.var_get(b"x").unwrap();
    assert_eq!(
        crate::cmd_string::append(&mut actual, &[command.as_ptr(), name.as_ptr()]),
        Code::Ok
    );
    assert_eq!(actual.result_obj(), before);
    assert_eq!(actual.entered_native_operation_count(), 0);
}

#[test]
fn original_command_append_native_issuer_cannot_use_the_explicit_logical_byte_port() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut actual = fixture::native("tcl9.1");
    let issued = actual
        .native_invocation_dialect()
        .native_object_append_protocol(Some(
            tcl_registry::native_object_append::LogicalAppendProvider::Tcl84CoreSimulation,
        ))
        .unwrap();
    assert!(issued.logical_provider().is_none());
    let source = obj::Owned::fresh(obj::alloc_typed(&SOURCE, 0));
    let prior = fixture::string(b"PRIOR");
    actual.set_result(prior.as_ptr());
    let _installed = fixture::install(&mut actual, 0);
    let _reset = Reset;
    reset();
    let scope =
        super::super::native_operation_currency::NativeOperationScope::enter(&actual).unwrap();
    assert_eq!(
        actual.append_native_operands(b"x", b"x", None, &[source.as_ptr()]),
        Code::Error
    );
    assert_eq!(SOURCE_GETTERS.get(), 0);
    assert_eq!(actual.result_obj(), prior.as_ptr());
    assert_eq!(unsafe { (*source.as_ptr()).ref_count }, 1);
    assert!(matches!(
        actual.native_execution_refusal(),
        Some(tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(_))
    ));
    assert!(scope.currency().ensure_current_or_refuse().is_err());
}
