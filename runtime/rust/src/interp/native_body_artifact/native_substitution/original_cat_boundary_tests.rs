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

//! Actual substitution flush chunks retain the original frame through unused-owner cleanup.
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
fn original_substitution_chunk_cat_checks_before_later_chunk_and_retains_first_cleanup_cause() {
    // naming.embedding.original-host-publication-and-fact-transport

    // Real 255-component flush and original owning headers, software-only.
    for profile in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        for mode in [0, 1, 2, 4] {
            let mut actual = fixture::native(profile);
            actual.frames.borrow_mut().push(crate::namespace::GLOBAL);
            let _installed = fixture::install(&mut actual, 0);
            let _reset = Reset;
            let scope =
                crate::interp::native_operation_currency::NativeOperationScope::enter(&actual)
                    .unwrap();
            let original = BodyObjectOperation::capture(&actual).unwrap();
            let mut values = vec![obj::Owned::fresh(obj::alloc_typed(&FIRST, 0))];
            values.extend((0..254).map(|_| fixture::string(b"B")));
            values.push(obj::Owned::fresh(obj::alloc_typed(&LATER_TYPE, 0)));
            let prior = fixture::string(b"PRIOR");
            actual.set_result(prior.as_ptr());
            GETTERS.set(0);
            LATER.set(0);
            FREES.set(0);
            MODE.set(mode);
            let reached = actual.flush_substitution_values(&mut values, &original);
            assert_eq!(GETTERS.get(), 1);
            assert_eq!(actual.result_obj(), prior.as_ptr());
            if mode == 0 {
                reached.unwrap();
                assert_eq!(values.len(), 1);
                let mut expected = vec![b'B'; 256];
                expected[0] = b'A';
                assert_eq!(obj::bytes_of(values[0].as_ptr()), expected);
                assert_eq!(LATER.get(), 1);
                original.check(&actual).unwrap();
                scope.currency().ensure_current_or_refuse().unwrap();
            } else {
                assert_eq!(reached, Err(Code::Error));
                assert_eq!(LATER.get(), 0);
                assert_eq!(FREES.get(), 1);
                assert!(values.is_empty());
                assert_first(&actual, mode);
                assert!(original.check(&actual).is_err());
            }
            drop(values);
            MODE.set(0);
            actual.frames.borrow_mut().pop();
        }
    }
}
