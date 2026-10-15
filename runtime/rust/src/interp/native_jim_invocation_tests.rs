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

//! Original Jim command selection across previous-result cleanup.

use super::{Code, Interp, ObjCommand, WeakInterp};
use crate::obj::{self, Owned, TclObj};
use std::{
    cell::{Cell, RefCell},
    ffi::{c_int, c_void},
};

thread_local! {
    static ACTUAL: RefCell<Option<WeakInterp>> = const { RefCell::new(None) };
    static GENERATION: Cell<u64> = const { Cell::new(0) };
    static REBIND_NAME: Cell<&'static [u8]> = const { Cell::new(b"target") };
    static EXPECT_UNKNOWN_HEADER: Cell<bool> = const { Cell::new(false) };
    static EVENTS: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
    static SELECTED_BEFORE_RESET: Cell<bool> = const { Cell::new(false) };
}
unsafe extern "C" fn old_worker(
    _: *mut c_void,
    actual: *mut Interp,
    _: c_int,
    _: *const *mut TclObj,
) -> c_int {
    EVENTS.with(|events| events.borrow_mut().push("OLD_INVOKE"));
    // SAFETY: this native callback receives its actual live interpreter.
    unsafe { (*actual).set_result_bytes(b"OLD") };
    0
}
unsafe extern "C" fn new_worker(
    _: *mut c_void,
    actual: *mut Interp,
    argc: c_int,
    argv: *const *mut TclObj,
) -> c_int {
    EVENTS.with(|events| events.borrow_mut().push("NEW_INVOKE"));
    if EXPECT_UNKNOWN_HEADER.with(Cell::get) {
        assert_eq!(argc, 2);
        // SAFETY: the actual callback receives its live original argv and interpreter.
        let (actual, words) = unsafe { (&mut *actual, std::slice::from_raw_parts(argv, 2)) };
        let context = actual.native_jim_object_context().unwrap();
        assert_eq!(
            words[0],
            crate::native_source::NativeJimObjectContext::unknown_object(&context)
                .unwrap()
                .as_ptr()
        );
    }
    // SAFETY: this native callback receives its actual live interpreter.
    unsafe { (*actual).set_result_bytes(b"NEW") };
    0
}
unsafe extern "C" fn delete_old(_: *mut c_void) {
    EVENTS.with(|events| events.borrow_mut().push("OLD_DELETE"));
}
extern "C" fn previous_free(_: *mut TclObj) {
    let mut actual = ACTUAL
        .with(|actual| actual.borrow().as_ref().and_then(WeakInterp::upgrade))
        .expect("actual native previous-result cleanup target");
    EVENTS.with(|events| events.borrow_mut().push("PREVIOUS_FREE"));
    SELECTED_BEFORE_RESET.with(|observed| {
        observed.set(GENERATION.with(|generation| {
            actual
                .retained_jim_command_reference(generation.get())
                .is_some()
        }))
    });
    actual
        .create_obj_command(
            REBIND_NAME.with(Cell::get),
            ObjCommand::new(new_worker, std::ptr::null_mut(), None),
        )
        .unwrap();
    EVENTS.with(|events| events.borrow_mut().push("REBIND_COMPLETE"));
}
static PREVIOUS_RESULT: obj::TclObjType = obj::TclObjType {
    name: c"previous-result-rebind".as_ptr(),
    free_int_rep_proc: Some(previous_free),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
fn word(actual: &mut Interp, bytes: &[u8]) -> Owned {
    let original = Owned::fresh(obj::new_string_bytes(bytes));
    crate::native_source::bind_context(
        original.as_ptr(),
        &actual.native_jim_object_context().unwrap(),
    )
    .unwrap();
    original
}

#[test]
fn original_jim_vector_and_pure_list_select_before_reset_while_source_resets_before_lookup() {
    let evidence = include_str!("../../tests/data/native_jim_invocation/result-reset-20261015.tsv");
    for (surface, labels) in [
        (0, ["VECTOR_COLD", "VECTOR_HOT"]),
        (1, ["PURE_LIST_COLD", "PURE_LIST_HOT"]),
        (2, ["SOURCE_STRING_COLD", "SOURCE_STRING_HOT"]),
    ] {
        for (hot, label) in labels.into_iter().enumerate() {
            let mut actual = Interp::with_native_core(
                super::default_host(),
                crate::environment::profile_for_dialect("jimtcl"),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            let generation = actual
                .create_obj_command(
                    b"target",
                    ObjCommand::new(old_worker, std::ptr::null_mut(), Some(delete_old)),
                )
                .unwrap();
            GENERATION.with(|slot| slot.set(generation));
            REBIND_NAME.with(|name| name.set(b"target"));
            EXPECT_UNKNOWN_HEADER.with(|expected| expected.set(false));
            let head = word(&mut actual, b"target");
            let script = if surface == 2 {
                word(&mut actual, b"target")
            } else {
                Owned::fresh(actual.new_list_object(&[head.as_ptr()]))
            };
            if hot != 0 {
                if surface == 0 {
                    assert!(actual
                        .resolve_original_command(head.as_ptr())
                        .unwrap()
                        .is_some());
                } else {
                    assert_eq!(actual.eval_body_obj(script.as_ptr()), Code::Ok);
                }
            }
            EVENTS.with(|events| events.borrow_mut().clear());
            SELECTED_BEFORE_RESET.with(|observed| observed.set(false));
            ACTUAL.with(|slot| *slot.borrow_mut() = Some(actual.downgrade()));
            let previous = Owned::fresh(obj::alloc_typed(&PREVIOUS_RESULT, 0));
            // SAFETY: this actual fresh typed header owns its String slot.
            unsafe { obj::set_string_rep(previous.as_ptr(), b"PRIOR") };
            crate::native_source::bind_context(
                previous.as_ptr(),
                &actual.native_jim_object_context().unwrap(),
            )
            .unwrap();
            actual.set_result(previous.as_ptr());
            drop(previous);
            let expected_events = evidence
                .lines()
                .filter_map(|line| {
                    let fields = line.split('|').collect::<Vec<_>>();
                    (fields.get(1) == Some(&label)
                        && matches!(
                            fields[0],
                            "PREVIOUS_FREE"
                                | "REBIND_COMPLETE"
                                | "OLD_INVOKE"
                                | "NEW_INVOKE"
                                | "OLD_DELETE"
                        ))
                    .then_some(fields[0])
                })
                .collect::<Vec<_>>();
            let code = if surface == 0 {
                actual.eval_original_object_vector(&[head.as_ptr()])
            } else {
                actual.eval_body_obj(script.as_ptr())
            };
            assert_eq!(
                EVENTS.with(|events| events.borrow().clone()),
                expected_events,
                "{label}"
            );
            // This observes the actual software role, not Jim's numeric inUse count.
            assert_eq!(
                SELECTED_BEFORE_RESET.with(Cell::get),
                surface != 2,
                "{label}"
            );
            let result = evidence
                .lines()
                .find(|line| line.starts_with(&format!("RESULT|{label}|")))
                .unwrap()
                .split('|')
                .collect::<Vec<_>>();
            assert_eq!(code.as_int().to_string(), result[2], "{label}");
            let bytes = result[3]
                .as_bytes()
                .chunks_exact(2)
                .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect::<Vec<_>>();
            assert_eq!(actual.result_bytes(), bytes, "{label}");
            assert!(actual.native_execution_refusal().is_none());
            assert!(actual.jim_active_command_workers.borrow().is_empty());
            ACTUAL.with(|slot| *slot.borrow_mut() = None);
        }
    }
}

#[test]
fn original_jim_missing_lookup_releases_previous_result_before_selecting_original_unknown_header() {
    let evidence =
        include_str!("../../tests/data/native_jim_invocation/missing-result-20261015.tsv");
    for (surface, label) in [
        (0, "VECTOR_MISSING"),
        (1, "PURE_LIST_MISSING"),
        (2, "SOURCE_STRING_MISSING"),
    ] {
        let mut actual = Interp::with_native_core(
            super::default_host(),
            crate::environment::profile_for_dialect("jimtcl"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        let generation = actual
            .create_obj_command(
                b"unknown",
                ObjCommand::new(old_worker, std::ptr::null_mut(), Some(delete_old)),
            )
            .unwrap();
        GENERATION.with(|slot| slot.set(generation));
        REBIND_NAME.with(|name| name.set(b"unknown"));
        EXPECT_UNKNOWN_HEADER.with(|expected| expected.set(true));
        let head = word(&mut actual, b"absent");
        let script = if surface == 2 {
            word(&mut actual, b"absent")
        } else {
            Owned::fresh(actual.new_list_object(&[head.as_ptr()]))
        };
        EVENTS.with(|events| events.borrow_mut().clear());
        SELECTED_BEFORE_RESET.with(|observed| observed.set(false));
        ACTUAL.with(|slot| *slot.borrow_mut() = Some(actual.downgrade()));
        let previous = Owned::fresh(obj::alloc_typed(&PREVIOUS_RESULT, 0));
        // SAFETY: this actual fresh typed header owns its String slot.
        unsafe { obj::set_string_rep(previous.as_ptr(), b"PRIOR") };
        crate::native_source::bind_context(
            previous.as_ptr(),
            &actual.native_jim_object_context().unwrap(),
        )
        .unwrap();
        actual.set_result(previous.as_ptr());
        drop(previous);
        let code = if surface == 0 {
            actual.eval_original_object_vector(&[head.as_ptr()])
        } else {
            actual.eval_body_obj(script.as_ptr())
        };
        let expected = evidence
            .lines()
            .filter_map(|line| {
                let fields = line.split('|').collect::<Vec<_>>();
                (fields.get(1) == Some(&label)
                    && matches!(
                        fields[0],
                        "PREVIOUS_FREE"
                            | "REBIND_COMPLETE"
                            | "OLD_INVOKE"
                            | "NEW_INVOKE"
                            | "OLD_DELETE"
                    ))
                .then_some(fields[0])
            })
            .collect::<Vec<_>>();
        assert_eq!(
            EVENTS.with(|events| events.borrow().clone()),
            expected,
            "{label}"
        );
        assert!(
            !SELECTED_BEFORE_RESET.with(Cell::get),
            "unknown selected after previous result cleanup"
        );
        assert_eq!(code, Code::Ok);
        assert_eq!(actual.result_bytes(), b"NEW");
        assert!(actual.native_execution_refusal().is_none());
        assert!(actual.jim_active_command_workers.borrow().is_empty());
        ACTUAL.with(|slot| *slot.borrow_mut() = None);
    }
}

#[test]
fn missing_jim_lookup_cannot_replace_a_withdrawn_original_unknown_header() {
    let mut actual = Interp::with_native_core(
        super::default_host(),
        crate::environment::profile_for_dialect("jimtcl"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    let context = actual.native_jim_object_context().unwrap();
    let retired =
        context.release_object(tcl_runtime_api::jim_interpreter::JimInterpreterObjectRole::Unknown);
    drop(retired);
    assert!(crate::native_source::NativeJimObjectContext::unknown_object(&context).is_err());
    let head = word(&mut actual, b"absent");
    assert_eq!(
        actual.eval_original_object_vector(&[head.as_ptr()]),
        Code::Error
    );
    assert!(actual.native_execution_refusal().is_some());
}
