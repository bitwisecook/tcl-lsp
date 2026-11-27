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

//! What the header's reference-count macros need of the runtime's `TclObj`
//! (`include/tcl.h`, `c-api-ownership-contract.md`).
//!
//! An extension compiled against the header never calls `Tcl_DecrRefCount`: the
//! macro lowers `refCount` itself and calls `TclFreeObj` when the count was one
//! or less. The runtime's side of that is the field at its declared offset
//! (`obj.rs` asserts the layout when it compiles) and an export that frees what
//! the macro hands it, whatever count the macro left. These tests do what the
//! macro does, in Rust, against the exports an extension would import; the
//! shim's tests run the macro itself, through a C compiler, against its own
//! objects.

use core::ffi::c_char;
use std::sync::atomic::{AtomicUsize, Ordering};

use tcl_runtime::capi::{
    tcl_test_double_free_count, tcl_test_finalize, tcl_test_reset_counters, TclFreeObj,
    Tcl_GetString, Tcl_NewObj, Tcl_NewStringObj,
};
use tcl_runtime::obj::{TclObj, TclObjType};

/// The free procedure counts into the `AtomicUsize` the object's internal
/// representation points at, so each test sees only its own object freed.
extern "C" fn count_free(obj: *mut TclObj) {
    // SAFETY: `counting_object` stored the address of a counter that outlives
    // the object.
    let counter = unsafe { &*((*obj).internal_rep as usize as *const AtomicUsize) };
    counter.fetch_add(1, Ordering::SeqCst);
}

/// A type whose free procedure is the only way to see an object freed.
static COUNTING: TclObjType = TclObjType {
    name: c"counting".as_ptr(),
    free_int_rep_proc: Some(count_free),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};

/// A fresh object of the counting type, at the count `Tcl_NewObj` gives it,
/// whose freeing adds one to `counter`.
fn counting_object(counter: &AtomicUsize) -> *mut TclObj {
    let obj = Tcl_NewObj();
    // SAFETY: `obj` is a live object nothing else refers to.
    unsafe {
        (*obj).type_ptr = &raw const COUNTING;
        (*obj).internal_rep = std::ptr::from_ref(counter) as usize as u64;
    }
    obj
}

/// `Tcl_IncrRefCount`, as the header defines it.
fn header_incr(obj: *mut TclObj) {
    // SAFETY: the tests pass live objects.
    unsafe { (*obj).ref_count += 1 };
}

/// `Tcl_DecrRefCount`, as the header defines it:
/// `if (obj->refCount-- <= 1) TclFreeObj(obj);`.
fn header_decr(obj: *mut TclObj) {
    // SAFETY: the tests pass live objects they hold a reference to, or a fresh
    // one, and do not use an object after it is freed.
    unsafe {
        let was = (*obj).ref_count;
        (*obj).ref_count -= 1;
        if was <= 1 {
            TclFreeObj(obj);
        }
    }
}

fn freed(counter: &AtomicUsize) -> usize {
    counter.load(Ordering::SeqCst)
}

/// What a test that frees all it makes leaves behind: no header or string buffer
/// live, and nothing released twice.
fn assert_nothing_left() {
    assert_eq!(tcl_test_finalize(), 0, "no header or string buffer is left");
    assert_eq!(tcl_test_double_free_count(), 0);
}

#[test]
fn the_last_release_through_the_macro_frees_the_object() {
    tcl_test_reset_counters();
    let counter = AtomicUsize::new(0);
    let obj = counting_object(&counter);
    header_incr(obj);
    // SAFETY: `obj` is live.
    assert_eq!(unsafe { (*obj).ref_count }, 1);
    header_decr(obj);
    assert_eq!(freed(&counter), 1, "TclFreeObj ran the type's free proc");
    assert_nothing_left();
}

#[test]
fn an_object_another_holder_keeps_is_not_freed_by_the_macro() {
    tcl_test_reset_counters();
    let counter = AtomicUsize::new(0);
    let obj = counting_object(&counter);
    header_incr(obj);
    header_incr(obj);
    header_decr(obj);
    assert_eq!(freed(&counter), 0, "one reference is still held");
    // SAFETY: `obj` is live.
    assert_eq!(unsafe { (*obj).ref_count }, 1);
    header_decr(obj);
    assert_eq!(freed(&counter), 1);
    assert_nothing_left();
}

#[test]
fn a_single_release_of_a_fresh_object_frees_it_as_the_macro_does_in_tcl() {
    tcl_test_reset_counters();
    let counter = AtomicUsize::new(0);
    let obj = counting_object(&counter);
    // SAFETY: `obj` is live.
    assert_eq!(
        unsafe { (*obj).ref_count },
        0,
        "Tcl_NewObj answers a count of 0"
    );
    header_decr(obj);
    assert_eq!(freed(&counter), 1);
    assert_nothing_left();
}

#[test]
fn tcl_free_obj_takes_the_string_rep_with_the_header_and_ignores_null() {
    tcl_test_reset_counters();
    // SAFETY: the bytes are live for the call; the object is freed once and
    // not used again.
    unsafe {
        let obj = Tcl_NewStringObj(c"text".as_ptr().cast::<c_char>(), 4);
        header_incr(obj);
        let bytes = Tcl_GetString(obj);
        assert_eq!(std::ffi::CStr::from_ptr(bytes).to_bytes(), b"text");
        assert_eq!((*obj).length, 4);
        header_decr(obj);
        TclFreeObj(core::ptr::null_mut());
    }
    assert_nothing_left();
}
