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

//! The C Tcl API exports (`#[no_mangle] extern "C"`) for the T1.1 surface.
//!
//! These are the runtime side of `c-extension-abi.md` §4.3 (direct C-ABI
//! imports): each is exported with the C ABI so an extension's WASM imports it
//! from the runtime. The ownership/error category of every function here is
//! fixed by `c-api-ownership-contract.md`, which nothing currently enforces
//! mechanically — a new export must be given its row by hand. The
//! obj-lifecycle and result/eval-core slice is exported; the rest of the
//! 81-function surface is not yet.

#![allow(non_snake_case)]

use core::ffi::{c_char, c_int};

use crate::interp::Interp;
use crate::obj::{self, TclObj, TclSize, TclWideInt};

// ---- object creation (all `fresh_zero` — refCount 0) ----

/// `Tcl_NewObj` — fresh empty-string object, refCount 0.
#[no_mangle]
pub extern "C" fn Tcl_NewObj() -> *mut TclObj {
    obj::new_obj()
}

/// `Tcl_NewStringObj` — copies `bytes[..length]` (or `strlen` when `length < 0`).
///
/// # Safety
/// `bytes` must reference `length` readable bytes, or be a valid NUL-terminated
/// C string when `length < 0` (it may be null when `length == 0`).
#[no_mangle]
pub unsafe extern "C" fn Tcl_NewStringObj(bytes: *const c_char, length: TclSize) -> *mut TclObj {
    // SAFETY: forwarded per this fn's contract.
    unsafe { obj::new_string_obj(bytes, length) }
}

/// `Tcl_NewWideIntObj` — pure int object, refCount 0.
#[no_mangle]
pub extern "C" fn Tcl_NewWideIntObj(value: TclWideInt) -> *mut TclObj {
    obj::new_wide_int_obj(value)
}

/// `Tcl_NewDoubleObj` — pure double object, refCount 0.
#[no_mangle]
pub extern "C" fn Tcl_NewDoubleObj(value: f64) -> *mut TclObj {
    obj::new_double_obj(value)
}

/// `Tcl_NewBooleanObj` — 0/1 int object, refCount 0.
#[no_mangle]
pub extern "C" fn Tcl_NewBooleanObj(value: c_int) -> *mut TclObj {
    obj::new_boolean_obj(value)
}

// ---- refcount management ----

/// `Tcl_IncrRefCount`. Null-safe.
///
/// # Safety
/// `obj` must be null or a live `TclObj`.
#[no_mangle]
pub unsafe extern "C" fn Tcl_IncrRefCount(obj: *mut TclObj) {
    // SAFETY: forwarded per contract.
    unsafe { obj::incr_ref_count(obj) }
}

/// `Tcl_DecrRefCount`. Frees at refCount 0. Null-safe.
///
/// # Safety
/// `obj` must be null or a live `TclObj` the caller holds a reference to.
#[no_mangle]
pub unsafe extern "C" fn Tcl_DecrRefCount(obj: *mut TclObj) {
    // SAFETY: forwarded per contract.
    unsafe { obj::decr_ref_count(obj) }
}

// ---- result ----

/// `Tcl_SetObjResult` — interp retains `resultObjPtr` (`borrowed→stored`).
///
/// # Safety
/// `interp` must be a live `Interp`; `resultObjPtr` must be a live `TclObj`.
#[no_mangle]
pub unsafe extern "C" fn Tcl_SetObjResult(interp: *mut Interp, resultObjPtr: *mut TclObj) {
    // SAFETY: caller guarantees both pointers are live.
    unsafe { (*interp).set_obj_result(resultObjPtr) }
}

/// `Tcl_GetObjResult` — the interp's current result (`borrowed`).
///
/// # Safety
/// `interp` must be a live `Interp`.
#[no_mangle]
pub unsafe extern "C" fn Tcl_GetObjResult(interp: *mut Interp) -> *mut TclObj {
    // SAFETY: caller guarantees `interp` is live.
    unsafe { (*interp).get_obj_result() }
}

// ---- string rep ----

/// `Tcl_GetStringFromObj` — borrowed pointer into the string rep; shimmers on
/// demand. Writes the byte length through `lengthPtr` when non-null.
///
/// # Safety
/// `objPtr` must be live; `lengthPtr` must be null or writable.
#[no_mangle]
pub unsafe extern "C" fn Tcl_GetStringFromObj(
    objPtr: *mut TclObj,
    lengthPtr: *mut TclSize,
) -> *mut c_char {
    // SAFETY: forwarded per contract.
    unsafe { obj::get_string(objPtr, lengthPtr) }
}

/// `Tcl_GetString` — borrowed pointer into the string rep; shimmers on demand.
///
/// # Safety
/// `objPtr` must be live.
#[no_mangle]
pub unsafe extern "C" fn Tcl_GetString(objPtr: *mut TclObj) -> *mut c_char {
    // SAFETY: forwarded per contract.
    unsafe { obj::get_string(objPtr, core::ptr::null_mut()) }
}

// ---- runtime interp lifecycle (host entry points, not in tcl.h) ----
//
// `Tcl_CreateInterp` / `Tcl_DeleteInterp` (the public surface) land with the
// full interp port; for T1.1 the host needs a way to mint and tear down the
// minimal result-only interp, so the runtime exposes these two entry points.

/// Create a runtime interp; returns an owning raw pointer the caller must pass
/// to [`tcl_runtime_delete_interp`].
#[no_mangle]
pub extern "C" fn tcl_runtime_create_interp() -> *mut Interp {
    // `Interp` is now a cheap `Rc` handle; box it so the C side has a stable
    // owning raw pointer to hand back to `tcl_runtime_delete_interp`.
    Box::into_raw(Box::new(Interp::new()))
}

/// Destroy a runtime interp previously returned by [`tcl_runtime_create_interp`].
/// Releases its hold on the result object. Null-safe.
///
/// # Safety
/// `interp` must be null or a pointer from `tcl_runtime_create_interp`, not yet
/// deleted.
#[no_mangle]
pub unsafe extern "C" fn tcl_runtime_delete_interp(interp: *mut Interp) {
    if interp.is_null() {
        return;
    }
    // SAFETY: caller guarantees provenance; reclaim the Box to run Drop once.
    unsafe {
        drop(Box::from_raw(interp));
    }
}

// ---- leak-check test exports (the `tcl_test_*` surface) ----

/// Reset the alloc/free/double-free counters (`tcl_test_reset_counters`).
#[no_mangle]
pub extern "C" fn tcl_test_reset_counters() {
    crate::counters::reset();
}

/// Total `TclObj` headers allocated since reset (`tcl_test_alloc_count`).
#[no_mangle]
pub extern "C" fn tcl_test_alloc_count() -> i64 {
    crate::counters::alloc_count()
}

/// Double-free / release-of-zero-refcount count (`tcl_test_double_free_count`).
#[no_mangle]
pub extern "C" fn tcl_test_double_free_count() -> i64 {
    crate::counters::double_free_count()
}

/// Residual live allocations (`tcl_test_finalize`). Zero ⇒ leak-free.
#[no_mangle]
pub extern "C" fn tcl_test_finalize() -> i64 {
    crate::counters::finalize()
}
