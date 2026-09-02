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

//! The exported C entry points: every function `include/tclshim.h` declares.
//!
//! Each is a thin, panic-safe adapter from the C calling convention onto
//! [`Obj`] and [`InterpState`]. The Rust names are ordinary snake case; the
//! `export_name` attribute gives each its real Tcl symbol, which is what an
//! extension links against.
//!
//! **Panic safety.** An `extern "C"` function that unwinds aborts the
//! process, so every body runs under [`guarded`]: a panic is caught, its
//! message parked in a thread-local, and a benign fallback returned to C.
//! [`crate::Interp`] collects the parked message after the C procedure
//! returns and reports the invocation as [`tcl_engine_api::EngineError::Crashed`].
//! This contains Rust panics only — undefined behaviour in the C code is
//! beyond any boundary, which is the trust posture the design doc states.

use std::cell::RefCell;
use std::ffi::{CStr, c_char, c_int, c_long, c_void};
use std::panic::{AssertUnwindSafe, catch_unwind};

use tcl_cmd_core::prefix::{self, Resolution};
use tcl_syntax::list;

use crate::obj::{Obj, ObjRef, TclError};
use crate::state::{CmdDeleteProc, InterpState, ObjCmdProc};

/// `TCL_OK`.
pub const TCL_OK: c_int = 0;
/// `TCL_ERROR`.
pub const TCL_ERROR: c_int = 1;
/// `TCL_RETURN`.
pub const TCL_RETURN: c_int = 2;
/// `TCL_BREAK`.
pub const TCL_BREAK: c_int = 3;
/// `TCL_CONTINUE`.
pub const TCL_CONTINUE: c_int = 4;

const TCL_EXACT: c_int = 1;
const TCL_NULL_OK: c_int = 32;
const TCL_INDEX_TEMP_TABLE: c_int = 64;

thread_local! {
    /// The message of a panic caught at an export boundary, until the host
    /// collects it.
    static LAST_PANIC: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Take the panic message parked by the most recent caught panic, if any.
#[must_use]
pub fn take_panic() -> Option<String> {
    LAST_PANIC.with(|slot| slot.borrow_mut().take())
}

fn panic_text(payload: &(dyn std::any::Any + Send)) -> String {
    payload.downcast_ref::<&str>().map_or_else(
        || {
            payload
                .downcast_ref::<String>()
                .cloned()
                .unwrap_or_else(|| "panic with an unreadable payload".to_owned())
        },
        |message| (*message).to_owned(),
    )
}

/// Run `body` with panics converted to `fallback` and parked for the host.
fn guarded<T>(fallback: T, body: impl FnOnce() -> T) -> T {
    match catch_unwind(AssertUnwindSafe(body)) {
        Ok(value) => value,
        Err(payload) => {
            LAST_PANIC.with(|slot| *slot.borrow_mut() = Some(panic_text(payload.as_ref())));
            fallback
        }
    }
}

/// The interpreter behind a `Tcl_Interp *`, or `None` for the NULL C code
/// passes when it wants no error reporting.
///
/// # Safety
///
/// A non-null `interp` must point at an [`InterpState`] owned by a live
/// [`crate::Interp`].
unsafe fn interp<'a>(interp: *mut InterpState) -> Option<&'a InterpState> {
    // SAFETY: the caller guarantees a non-null pointer is a live state.
    unsafe { interp.as_ref() }
}

/// The object behind a `Tcl_Obj *`.
///
/// # Safety
///
/// `obj` must point at a live object from [`Obj::into_raw`].
unsafe fn obj<'a>(obj: *mut Obj) -> &'a Obj {
    // SAFETY: the caller guarantees the object is live; a NULL here is the
    // same programming error it is in C Tcl, reported as a caught panic.
    unsafe { obj.as_ref() }.expect("a NULL Tcl_Obj pointer")
}

/// Text from a C string, or `""` for NULL.
///
/// # Safety
///
/// A non-null `text` must be NUL-terminated.
unsafe fn c_text<'a>(text: *const c_char) -> std::borrow::Cow<'a, str> {
    if text.is_null() {
        return std::borrow::Cow::Borrowed("");
    }
    // SAFETY: the caller guarantees a terminated string.
    unsafe { CStr::from_ptr(text) }.to_string_lossy()
}

/// Bytes from a C pointer and a `Tcl_Size` length, `-1` meaning
/// NUL-terminated.
///
/// # Safety
///
/// `bytes` must address `length` readable bytes, or a terminated string when
/// `length` is negative.
unsafe fn c_bytes<'a>(bytes: *const c_char, length: isize) -> &'a [u8] {
    if bytes.is_null() {
        return &[];
    }
    if length < 0 {
        // SAFETY: the caller guarantees termination.
        return unsafe { CStr::from_ptr(bytes) }.to_bytes();
    }
    // SAFETY: the caller guarantees `length` readable bytes.
    unsafe { std::slice::from_raw_parts(bytes.cast::<u8>(), length.unsigned_abs()) }
}

/// Report a conversion error on `interp` (when given) and return `TCL_ERROR`.
///
/// # Safety
///
/// As [`interp`].
unsafe fn report(interp_ptr: *mut InterpState, error: &TclError) -> c_int {
    // SAFETY: as documented on the function.
    if let Some(state) = unsafe { interp(interp_ptr) } {
        state.set_error(error);
    }
    TCL_ERROR
}

/// `Tcl_CreateObjCommand`. Returns the command token (never NULL).
///
/// # Safety
///
/// `interp` must be a live shim interpreter; `cmd_name` a terminated string;
/// `proc` a valid `Tcl_ObjCmdProc`.
#[unsafe(export_name = "Tcl_CreateObjCommand")]
pub unsafe extern "C" fn tcl_create_obj_command(
    interp_ptr: *mut InterpState,
    cmd_name: *const c_char,
    proc: ObjCmdProc,
    client_data: *mut c_void,
    delete_proc: Option<CmdDeleteProc>,
) -> *mut c_void {
    guarded(std::ptr::null_mut(), || {
        // SAFETY: as documented on the function.
        let state = unsafe { interp(interp_ptr) }.expect("a NULL Tcl_Interp pointer");
        // SAFETY: as above.
        let name = unsafe { c_text(cmd_name) };
        let entry = state.create_command(&name, proc, client_data, delete_proc);
        std::rc::Rc::as_ptr(&entry).cast_mut().cast::<c_void>()
    })
}

/// `Tcl_DeleteCommand`: `0` when the command existed, `-1` otherwise.
///
/// # Safety
///
/// As [`tcl_create_obj_command`].
#[unsafe(export_name = "Tcl_DeleteCommand")]
pub unsafe extern "C" fn tcl_delete_command(
    interp_ptr: *mut InterpState,
    cmd_name: *const c_char,
) -> c_int {
    guarded(-1, || {
        // SAFETY: as documented on the function.
        let state = unsafe { interp(interp_ptr) }.expect("a NULL Tcl_Interp pointer");
        // SAFETY: as above.
        let name = unsafe { c_text(cmd_name) };
        if state.delete_command(&name) { 0 } else { -1 }
    })
}

/// `Tcl_NewStringObj`.
///
/// # Safety
///
/// `bytes` must address `length` bytes, or a terminated string for a negative
/// `length`.
#[must_use]
#[unsafe(export_name = "Tcl_NewStringObj")]
pub unsafe extern "C" fn tcl_new_string_obj(bytes: *const c_char, length: isize) -> *mut Obj {
    guarded(std::ptr::null_mut(), || {
        // SAFETY: as documented on the function.
        let bytes = unsafe { c_bytes(bytes, length) };
        Obj::from_bytes(bytes).into_raw()
    })
}

/// `Tcl_NewIntObj`.
#[must_use]
#[unsafe(export_name = "Tcl_NewIntObj")]
pub extern "C" fn tcl_new_int_obj(value: c_int) -> *mut Obj {
    guarded(std::ptr::null_mut(), || {
        Obj::int(i64::from(value)).into_raw()
    })
}

/// `Tcl_NewLongObj`.
#[must_use]
#[unsafe(export_name = "Tcl_NewLongObj")]
pub extern "C" fn tcl_new_long_obj(value: c_long) -> *mut Obj {
    // Widened through `i128` so the conversion is never same-typed on a
    // 64-bit `long` and never lossy on a 32-bit one.
    guarded(std::ptr::null_mut(), || {
        Obj::int(i64::try_from(i128::from(value)).expect("a C long is at most 64 bits")).into_raw()
    })
}

/// `Tcl_NewWideIntObj`.
#[must_use]
#[unsafe(export_name = "Tcl_NewWideIntObj")]
pub extern "C" fn tcl_new_wide_int_obj(value: i64) -> *mut Obj {
    guarded(std::ptr::null_mut(), || Obj::int(value).into_raw())
}

/// `Tcl_NewBooleanObj`: an integer `0` or `1`, as Tcl 9 defines it.
#[must_use]
#[unsafe(export_name = "Tcl_NewBooleanObj")]
pub extern "C" fn tcl_new_boolean_obj(value: c_int) -> *mut Obj {
    guarded(std::ptr::null_mut(), || {
        Obj::int(i64::from(value != 0)).into_raw()
    })
}

/// `Tcl_NewDoubleObj`.
#[must_use]
#[unsafe(export_name = "Tcl_NewDoubleObj")]
pub extern "C" fn tcl_new_double_obj(value: f64) -> *mut Obj {
    guarded(std::ptr::null_mut(), || Obj::double(value).into_raw())
}

/// `Tcl_NewListObj`: a list taking a reference to each of `objv`.
///
/// # Safety
///
/// `words` must address `word_count` live object pointers (or be NULL with
/// `word_count` zero).
#[unsafe(export_name = "Tcl_NewListObj")]
pub unsafe extern "C" fn tcl_new_list_obj(word_count: isize, words: *const *mut Obj) -> *mut Obj {
    guarded(std::ptr::null_mut(), || {
        let items = if words.is_null() || word_count <= 0 {
            Vec::new()
        } else {
            // SAFETY: as documented on the function.
            unsafe { std::slice::from_raw_parts(words, word_count.unsigned_abs()) }
                .iter()
                // SAFETY: each pointer is a live object.
                .map(|&raw| unsafe { ObjRef::adopt(raw) })
                .collect()
        };
        Obj::list(items).into_raw()
    })
}

/// `Tcl_IncrRefCount`.
///
/// # Safety
///
/// `raw` must be a live object.
#[unsafe(export_name = "Tcl_IncrRefCount")]
pub unsafe extern "C" fn tcl_incr_ref_count(raw: *mut Obj) {
    guarded((), || {
        // SAFETY: as documented on the function.
        unsafe { Obj::incr_ref_count(raw) };
    });
}

/// `Tcl_DecrRefCount`: releases a reference and frees the object at zero.
///
/// # Safety
///
/// `raw` must be a live object, and must not be used afterwards if this was
/// its last reference.
#[unsafe(export_name = "Tcl_DecrRefCount")]
pub unsafe extern "C" fn tcl_decr_ref_count(raw: *mut Obj) {
    guarded((), || {
        // SAFETY: as documented on the function.
        unsafe { Obj::decr_ref_count(raw) };
    });
}

/// `Tcl_IsShared`.
///
/// # Safety
///
/// `raw` must be a live object.
#[unsafe(export_name = "Tcl_IsShared")]
pub unsafe extern "C" fn tcl_is_shared(raw: *mut Obj) -> c_int {
    // SAFETY: as documented on the function.
    guarded(0, || c_int::from(unsafe { obj(raw) }.is_shared()))
}

/// `Tcl_DuplicateObj`: a fresh copy with a reference count of zero.
///
/// # Safety
///
/// `raw` must be a live object.
#[unsafe(export_name = "Tcl_DuplicateObj")]
pub unsafe extern "C" fn tcl_duplicate_obj(raw: *mut Obj) -> *mut Obj {
    // SAFETY: as documented on the function.
    guarded(std::ptr::null_mut(), || {
        unsafe { obj(raw) }.duplicate().into_raw()
    })
}

/// `Tcl_GetString`.
///
/// # Safety
///
/// `raw` must be a live object; the returned pointer is valid until the
/// object is mutated or freed.
#[unsafe(export_name = "Tcl_GetString")]
pub unsafe extern "C" fn tcl_get_string(raw: *mut Obj) -> *mut c_char {
    // SAFETY: as documented on the function.
    guarded(std::ptr::null_mut(), || {
        unsafe { obj(raw) }.c_string().0.cast_mut()
    })
}

/// `Tcl_GetStringFromObj`.
///
/// # Safety
///
/// As [`tcl_get_string`]; a non-null `length_ptr` must be writable.
#[unsafe(export_name = "Tcl_GetStringFromObj")]
pub unsafe extern "C" fn tcl_get_string_from_obj(
    raw: *mut Obj,
    length_ptr: *mut isize,
) -> *mut c_char {
    guarded(std::ptr::null_mut(), || {
        // SAFETY: as documented on the function.
        let (bytes, length) = unsafe { obj(raw) }.c_string();
        if !length_ptr.is_null() {
            // SAFETY: the caller guarantees a writable pointer.
            unsafe { length_ptr.write(isize::try_from(length).unwrap_or(isize::MAX)) };
        }
        bytes.cast_mut()
    })
}

/// C's `(int)` of a wide within the range Tcl 9 accepts: two's-complement
/// truncation, spelled without a lossy cast.
fn wrap_to_i32(wide: i64) -> i32 {
    let low = u32::try_from(wide.rem_euclid(1 << 32)).expect("reduced modulo 2^32");
    i32::from_ne_bytes(low.to_ne_bytes())
}

/// Whether Tcl 9's `Tcl_GetIntFromObj` accepts `wide` for a 32-bit target
/// type: anything within the *unsigned* range is truncated rather than
/// refused.
fn fits_32(wide: i64) -> bool {
    (-i64::from(u32::MAX)..=i64::from(u32::MAX)).contains(&wide)
}

/// `Tcl_GetIntFromObj`.
///
/// # Safety
///
/// `interp` NULL or live; `raw` live; `int_ptr` writable.
#[unsafe(export_name = "Tcl_GetIntFromObj")]
pub unsafe extern "C" fn tcl_get_int_from_obj(
    interp_ptr: *mut InterpState,
    raw: *mut Obj,
    int_ptr: *mut c_int,
) -> c_int {
    guarded(TCL_ERROR, || {
        // SAFETY: as documented on the function.
        let wide = match unsafe { obj(raw) }.get_wide() {
            Ok(wide) if fits_32(wide) => wide,
            Ok(_) => return unsafe { report(interp_ptr, &TclError::overflow()) },
            Err(error) => return unsafe { report(interp_ptr, &error) },
        };
        // SAFETY: the caller guarantees a writable pointer.
        unsafe { int_ptr.write(wrap_to_i32(wide)) };
        TCL_OK
    })
}

/// `Tcl_GetLongFromObj`. `long` is 32 bits on Windows and 64 elsewhere; the
/// two branches write the width the target has.
///
/// # Safety
///
/// As [`tcl_get_int_from_obj`].
#[unsafe(export_name = "Tcl_GetLongFromObj")]
pub unsafe extern "C" fn tcl_get_long_from_obj(
    interp_ptr: *mut InterpState,
    raw: *mut Obj,
    long_ptr: *mut c_long,
) -> c_int {
    guarded(TCL_ERROR, || {
        // SAFETY: as documented on the function.
        let wide = match unsafe { obj(raw) }.get_wide() {
            Ok(wide) => wide,
            Err(error) => return unsafe { report(interp_ptr, &error) },
        };
        if size_of::<c_long>() == size_of::<i64>() {
            // SAFETY: `long` is 64 bits wide here and the pointer is writable.
            unsafe { long_ptr.cast::<i64>().write(wide) };
        } else if fits_32(wide) {
            // SAFETY: `long` is 32 bits wide here and the pointer is writable.
            unsafe { long_ptr.cast::<i32>().write(wrap_to_i32(wide)) };
        } else {
            // SAFETY: as documented on the function.
            return unsafe { report(interp_ptr, &TclError::overflow()) };
        }
        TCL_OK
    })
}

/// `Tcl_GetWideIntFromObj`.
///
/// # Safety
///
/// As [`tcl_get_int_from_obj`].
#[unsafe(export_name = "Tcl_GetWideIntFromObj")]
pub unsafe extern "C" fn tcl_get_wide_int_from_obj(
    interp_ptr: *mut InterpState,
    raw: *mut Obj,
    wide_ptr: *mut i64,
) -> c_int {
    guarded(TCL_ERROR, || {
        // SAFETY: as documented on the function.
        match unsafe { obj(raw) }.get_wide() {
            Ok(wide) => {
                // SAFETY: the caller guarantees a writable pointer.
                unsafe { wide_ptr.write(wide) };
                TCL_OK
            }
            // SAFETY: as documented on the function.
            Err(error) => unsafe { report(interp_ptr, &error) },
        }
    })
}

/// `Tcl_GetBooleanFromObj`.
///
/// # Safety
///
/// As [`tcl_get_int_from_obj`].
#[unsafe(export_name = "Tcl_GetBooleanFromObj")]
pub unsafe extern "C" fn tcl_get_boolean_from_obj(
    interp_ptr: *mut InterpState,
    raw: *mut Obj,
    int_ptr: *mut c_int,
) -> c_int {
    guarded(TCL_ERROR, || {
        // SAFETY: as documented on the function.
        match unsafe { obj(raw) }.get_boolean() {
            Ok(value) => {
                // SAFETY: the caller guarantees a writable pointer.
                unsafe { int_ptr.write(c_int::from(value)) };
                TCL_OK
            }
            // SAFETY: as documented on the function.
            Err(error) => unsafe { report(interp_ptr, &error) },
        }
    })
}

/// `Tcl_GetDoubleFromObj`.
///
/// # Safety
///
/// As [`tcl_get_int_from_obj`].
#[unsafe(export_name = "Tcl_GetDoubleFromObj")]
pub unsafe extern "C" fn tcl_get_double_from_obj(
    interp_ptr: *mut InterpState,
    raw: *mut Obj,
    double_ptr: *mut f64,
) -> c_int {
    guarded(TCL_ERROR, || {
        // SAFETY: as documented on the function.
        match unsafe { obj(raw) }.get_double() {
            Ok(value) => {
                // SAFETY: the caller guarantees a writable pointer.
                unsafe { double_ptr.write(value) };
                TCL_OK
            }
            // SAFETY: as documented on the function.
            Err(error) => unsafe { report(interp_ptr, &error) },
        }
    })
}

/// Read a NULL-terminated table of C strings with `offset` bytes between
/// entries.
///
/// # Safety
///
/// `table` must be such a table.
unsafe fn read_table(table: *const c_void, offset: isize) -> Vec<String> {
    let mut entries = Vec::new();
    let mut cursor = table.cast::<u8>();
    loop {
        // SAFETY: the caller guarantees each entry slot holds a `const char *`
        // and the table ends with NULL; entries are read at `offset` strides.
        let entry = unsafe { cursor.cast::<*const c_char>().read_unaligned() };
        if entry.is_null() {
            return entries;
        }
        // SAFETY: as above, each entry is a terminated string.
        entries.push(unsafe { c_text(entry) }.into_owned());
        // SAFETY: the table extends by `offset` for each non-NULL entry.
        cursor = unsafe { cursor.offset(offset) };
    }
}

/// The `must be …` enumeration under `TCL_NULL_OK`, which C words with a
/// plain comma join and a trailing `, or ""`.
fn null_ok_message(what: &str, key: &str, ambiguous: bool, entries: &[String]) -> String {
    let kind = if ambiguous { "ambiguous" } else { "bad" };
    format!(
        "{kind} {what} \"{key}\": must be {}, or \"\"",
        entries.join(", ")
    )
}

/// Write the resolved index at the width the header encoded into `flags`.
///
/// # Safety
///
/// `index_ptr` must be writable at that width.
unsafe fn write_index(index_ptr: *mut c_void, flags: c_int, index: isize) {
    // SAFETY, for each arm: the header encoded `sizeof(*indexPtr) << 1` into
    // these bits, so the pointer has exactly the width written.
    match flags & 0b1_0110 {
        2 => unsafe {
            index_ptr
                .cast::<u8>()
                .write(u8::try_from(index).unwrap_or(u8::MAX));
        },
        4 => unsafe {
            index_ptr
                .cast::<u16>()
                .write(u16::try_from(index).unwrap_or(u16::MAX));
        },
        16 => unsafe {
            index_ptr
                .cast::<i64>()
                .write(i64::try_from(index).unwrap_or(-1));
        },
        _ => unsafe {
            index_ptr
                .cast::<i32>()
                .write(i32::try_from(index).unwrap_or(-1));
        },
    }
}

/// `Tcl_GetIndexFromObjStruct`, and through the header's macro
/// `Tcl_GetIndexFromObj`: resolve a word against an option table with C's
/// unique-prefix rule and C's messages.
///
/// # Safety
///
/// `interp` NULL or live; `raw` NULL or live; `table` a NULL-terminated table
/// with `offset` bytes between entries; `msg` terminated; `index_ptr` NULL or
/// writable at the width `flags` encodes.
#[unsafe(export_name = "Tcl_GetIndexFromObjStruct")]
pub unsafe extern "C" fn tcl_get_index_from_obj_struct(
    interp_ptr: *mut InterpState,
    raw: *mut Obj,
    table: *const c_void,
    offset: isize,
    msg: *const c_char,
    flags: c_int,
    index_ptr: *mut c_void,
) -> c_int {
    guarded(TCL_ERROR, || {
        if offset < isize::try_from(size_of::<*const c_char>()).unwrap_or(isize::MAX) {
            // SAFETY: as documented on the function.
            return unsafe {
                report(
                    interp_ptr,
                    &TclError {
                        message: format!("Invalid struct offset value {offset}."),
                        code: None,
                    },
                )
            };
        }
        // SAFETY: as documented on the function.
        let entries = unsafe { read_table(table, offset) };
        // SAFETY: as above.
        let what = unsafe { c_text(msg) }.into_owned();
        // SAFETY: a non-null `raw` is live.
        let key = if raw.is_null() {
            String::new()
        } else {
            unsafe { obj(raw) }.text()
        };
        let null_ok = flags & TCL_NULL_OK != 0;
        let exact = flags & TCL_EXACT != 0;

        let resolved: Option<isize> = if key.is_empty() && null_ok {
            Some(-1)
        } else {
            match prefix::scan(&entries, key.as_bytes(), exact) {
                Resolution::Exact(index) | Resolution::UniquePrefix(index) => {
                    isize::try_from(index).ok()
                }
                Resolution::Ambiguous | Resolution::NoMatch => None,
            }
        };
        let Some(index) = resolved else {
            let ambiguous = matches!(
                prefix::scan(&entries, key.as_bytes(), exact),
                Resolution::Ambiguous
            );
            let message = if null_ok {
                null_ok_message(&what, &key, ambiguous, &entries)
            } else {
                String::from_utf8_lossy(&prefix::bad_key_message(
                    &entries,
                    what.as_bytes(),
                    key.as_bytes(),
                    ambiguous,
                ))
                .into_owned()
            };
            // SAFETY: as documented on the function.
            return unsafe {
                report(
                    interp_ptr,
                    &TclError::with_code(
                        message,
                        list::join_list(["TCL", "LOOKUP", "INDEX", &what, &key]),
                    ),
                )
            };
        };
        if index >= 0 && !raw.is_null() && flags & TCL_INDEX_TEMP_TABLE == 0 {
            // SAFETY: `raw` is live.
            unsafe { obj(raw) }.set_index_entry(&entries[index.unsigned_abs()]);
        }
        if !index_ptr.is_null() {
            // SAFETY: as documented on the function.
            unsafe { write_index(index_ptr, flags, index) };
        }
        TCL_OK
    })
}

/// `Tcl_ListObjAppendElement`. A shared list is refused with an error where
/// C Tcl would panic the process.
///
/// # Safety
///
/// `interp` NULL or live; `list` and `element` live.
#[unsafe(export_name = "Tcl_ListObjAppendElement")]
pub unsafe extern "C" fn tcl_list_obj_append_element(
    interp_ptr: *mut InterpState,
    list_raw: *mut Obj,
    element: *mut Obj,
) -> c_int {
    guarded(TCL_ERROR, || {
        // SAFETY: as documented on the function.
        let list = unsafe { obj(list_raw) };
        if list.is_shared() {
            // SAFETY: as above.
            return unsafe {
                report(
                    interp_ptr,
                    &TclError {
                        message: "Tcl_ListObjAppendElement called with shared object".to_owned(),
                        code: None,
                    },
                )
            };
        }
        // SAFETY: `element` is live; the list takes its own reference.
        let element = unsafe { ObjRef::adopt(element) };
        match list.append_element(element) {
            Ok(()) => TCL_OK,
            // SAFETY: as documented on the function.
            Err(error) => unsafe { report(interp_ptr, &error) },
        }
    })
}

/// `Tcl_ListObjGetElements`. The returned array belongs to the list and is
/// valid until the list is mutated or freed.
///
/// # Safety
///
/// `interp` NULL or live; `list` live; `count_ptr` and `array_ptr` writable.
#[unsafe(export_name = "Tcl_ListObjGetElements")]
pub unsafe extern "C" fn tcl_list_obj_get_elements(
    interp_ptr: *mut InterpState,
    list_raw: *mut Obj,
    count_ptr: *mut isize,
    array_ptr: *mut *mut *mut Obj,
) -> c_int {
    guarded(TCL_ERROR, || {
        // SAFETY: as documented on the function.
        let list = unsafe { obj(list_raw) };
        let view = list.with_list(|items| {
            (
                isize::try_from(items.len()).unwrap_or(isize::MAX),
                // `ObjRef` is `repr(transparent)` over the object pointer, so
                // the slice *is* the `Tcl_Obj **` array.
                items.as_ptr().cast::<*mut Obj>().cast_mut(),
            )
        });
        match view {
            Ok((count, array)) => {
                // SAFETY: the caller guarantees writable out-pointers.
                unsafe {
                    count_ptr.write(count);
                    array_ptr.write(array);
                }
                TCL_OK
            }
            // SAFETY: as documented on the function.
            Err(error) => unsafe { report(interp_ptr, &error) },
        }
    })
}

/// `Tcl_ListObjLength`.
///
/// # Safety
///
/// `interp` NULL or live; `list` live; `length_ptr` writable.
#[unsafe(export_name = "Tcl_ListObjLength")]
pub unsafe extern "C" fn tcl_list_obj_length(
    interp_ptr: *mut InterpState,
    list_raw: *mut Obj,
    length_ptr: *mut isize,
) -> c_int {
    guarded(TCL_ERROR, || {
        // SAFETY: as documented on the function.
        match unsafe { obj(list_raw) }.with_list(<[ObjRef]>::len) {
            Ok(length) => {
                // SAFETY: the caller guarantees a writable pointer.
                unsafe { length_ptr.write(isize::try_from(length).unwrap_or(isize::MAX)) };
                TCL_OK
            }
            // SAFETY: as documented on the function.
            Err(error) => unsafe { report(interp_ptr, &error) },
        }
    })
}

/// `Tcl_SetObjResult`: the interpreter takes its own reference.
///
/// # Safety
///
/// `interp` live; `result` live.
#[unsafe(export_name = "Tcl_SetObjResult")]
pub unsafe extern "C" fn tcl_set_obj_result(interp_ptr: *mut InterpState, result: *mut Obj) {
    guarded((), || {
        // SAFETY: as documented on the function.
        let state = unsafe { interp(interp_ptr) }.expect("a NULL Tcl_Interp pointer");
        // SAFETY: as above.
        state.set_result(unsafe { ObjRef::adopt(result) });
    });
}

/// `Tcl_GetObjResult`: the current result, owned by the interpreter.
///
/// # Safety
///
/// `interp` live.
#[unsafe(export_name = "Tcl_GetObjResult")]
pub unsafe extern "C" fn tcl_get_obj_result(interp_ptr: *mut InterpState) -> *mut Obj {
    guarded(std::ptr::null_mut(), || {
        // SAFETY: as documented on the function.
        let state = unsafe { interp(interp_ptr) }.expect("a NULL Tcl_Interp pointer");
        state.result().as_ptr()
    })
}

/// `Tcl_ResetResult`.
///
/// # Safety
///
/// `interp` live.
#[unsafe(export_name = "Tcl_ResetResult")]
pub unsafe extern "C" fn tcl_reset_result(interp_ptr: *mut InterpState) {
    guarded((), || {
        // SAFETY: as documented on the function.
        let state = unsafe { interp(interp_ptr) }.expect("a NULL Tcl_Interp pointer");
        state.reset_result();
    });
}

/// The fixed-arity export behind the header's inline `Tcl_SetResult`.
///
/// # Safety
///
/// `interp` live; `result` NULL or terminated.
#[unsafe(export_name = "TclShim_SetResultString")]
pub unsafe extern "C" fn tclshim_set_result_string(
    interp_ptr: *mut InterpState,
    result: *const c_char,
) {
    guarded((), || {
        // SAFETY: as documented on the function.
        let state = unsafe { interp(interp_ptr) }.expect("a NULL Tcl_Interp pointer");
        // SAFETY: as above.
        state.set_result_text(&unsafe { c_text(result) });
    });
}

/// The fixed-arity export behind the header's inline `Tcl_AppendResult`.
///
/// # Safety
///
/// `interp` live; `piece` NULL or terminated.
#[unsafe(export_name = "TclShim_AppendResultString")]
pub unsafe extern "C" fn tclshim_append_result_string(
    interp_ptr: *mut InterpState,
    piece: *const c_char,
) {
    guarded((), || {
        // SAFETY: as documented on the function.
        let state = unsafe { interp(interp_ptr) }.expect("a NULL Tcl_Interp pointer");
        // SAFETY: as above.
        state.append_result(&unsafe { c_text(piece) });
    });
}

/// `Tcl_WrongNumArgs`: `wrong # args: should be "<objv…> <message>"`, with
/// each word quoted as a list element when it needs it and an abbreviated
/// option printed in full.
///
/// # Safety
///
/// `interp` live; `words` addresses `word_count` live objects; `message` NULL
/// or terminated.
#[unsafe(export_name = "Tcl_WrongNumArgs")]
pub unsafe extern "C" fn tcl_wrong_num_args(
    interp_ptr: *mut InterpState,
    word_count: isize,
    words: *const *mut Obj,
    message: *const c_char,
) {
    guarded((), || {
        // SAFETY: as documented on the function.
        let state = unsafe { interp(interp_ptr) }.expect("a NULL Tcl_Interp pointer");
        let words = if words.is_null() || word_count <= 0 {
            &[][..]
        } else {
            // SAFETY: as documented on the function.
            unsafe { std::slice::from_raw_parts(words, word_count.unsigned_abs()) }
        };
        let mut text = String::from("wrong # args: should be \"");
        for (position, &raw) in words.iter().enumerate() {
            // SAFETY: each pointer is a live object.
            let word = unsafe { obj(raw) };
            match word.index_entry() {
                Some(entry) => text.push_str(&entry),
                None => text.push_str(&list::list_element(&word.text())),
            }
            if position + 1 < words.len() || !message.is_null() {
                text.push(' ');
            }
        }
        if !message.is_null() {
            // SAFETY: as documented on the function.
            text.push_str(&unsafe { c_text(message) });
        }
        text.push('"');
        state.set_error(&TclError::with_code(text, "TCL WRONGARGS"));
    });
}

/// `Tcl_SetObjErrorCode`.
///
/// # Safety
///
/// `interp` live; `code` live.
#[unsafe(export_name = "Tcl_SetObjErrorCode")]
pub unsafe extern "C" fn tcl_set_obj_error_code(interp_ptr: *mut InterpState, code: *mut Obj) {
    guarded((), || {
        // SAFETY: as documented on the function.
        let state = unsafe { interp(interp_ptr) }.expect("a NULL Tcl_Interp pointer");
        // SAFETY: as above.
        state.set_error_code(Some(unsafe { ObjRef::adopt(code) }));
    });
}

/// `Tcl_PkgProvideEx`.
///
/// # Safety
///
/// `interp` live; `name` and `version` terminated.
#[unsafe(export_name = "Tcl_PkgProvideEx")]
pub unsafe extern "C" fn tcl_pkg_provide_ex(
    interp_ptr: *mut InterpState,
    name: *const c_char,
    version: *const c_char,
    _client_data: *const c_void,
) -> c_int {
    guarded(TCL_ERROR, || {
        // SAFETY: as documented on the function.
        let state = unsafe { interp(interp_ptr) }.expect("a NULL Tcl_Interp pointer");
        // SAFETY: as above.
        let (name, version) = unsafe { (c_text(name), c_text(version)) };
        match state.provide(&name, &version) {
            Ok(()) => TCL_OK,
            Err(error) => {
                state.set_error(&error);
                TCL_ERROR
            }
        }
    })
}

/// `Tcl_NumUtfChars`: the number of UTF-8 characters in `length` bytes (or
/// up to the terminator for a negative `length`).
///
/// # Safety
///
/// As [`tcl_new_string_obj`].
#[must_use]
#[unsafe(export_name = "Tcl_NumUtfChars")]
pub unsafe extern "C" fn tcl_num_utf_chars(src: *const c_char, length: isize) -> isize {
    guarded(0, || {
        // SAFETY: as documented on the function.
        let bytes = unsafe { c_bytes(src, length) };
        let count = bytes.iter().filter(|&&byte| byte & 0xC0 != 0x80).count();
        isize::try_from(count).unwrap_or(isize::MAX)
    })
}

/// `Tcl_UtfNcmp`: compare up to `n` characters, the terminator counting as
/// a character below every other.
///
/// # Safety
///
/// `s1` and `s2` must be terminated strings.
#[must_use]
#[unsafe(export_name = "Tcl_UtfNcmp")]
pub unsafe extern "C" fn tcl_utf_ncmp(s1: *const c_char, s2: *const c_char, n: usize) -> c_int {
    guarded(0, || {
        // SAFETY: as documented on the function.
        let (left, right) = unsafe { (c_text(s1), c_text(s2)) };
        let mut left = left.chars();
        let mut right = right.chars();
        for _ in 0..n {
            let (a, b) = (left.next(), right.next());
            match a.cmp(&b) {
                std::cmp::Ordering::Less => return -1,
                std::cmp::Ordering::Greater => return 1,
                std::cmp::Ordering::Equal if a.is_none() => return 0,
                std::cmp::Ordering::Equal => {}
            }
        }
        0
    })
}

#[cfg(test)]
mod tests {
    use super::{TCL_OK, take_panic, tcl_get_string, tcl_utf_ncmp, wrap_to_i32};

    #[test]
    fn a_panic_at_the_boundary_is_parked_not_propagated() {
        // SAFETY: a NULL object is the defect being tested; the export must
        // survive it.
        let pointer = unsafe { tcl_get_string(std::ptr::null_mut()) };
        assert!(pointer.is_null());
        assert!(
            take_panic()
                .expect("the panic was parked")
                .contains("NULL Tcl_Obj")
        );
        assert!(take_panic().is_none(), "taking clears it");
    }

    #[test]
    fn int_truncation_follows_tcl_9() {
        assert_eq!(wrap_to_i32(2_147_483_648), -2_147_483_648);
        assert_eq!(wrap_to_i32(-2_147_483_647), -2_147_483_647);
        assert_eq!(wrap_to_i32(7), 7);
    }

    #[test]
    fn utf_ncmp_compares_characters() {
        // SAFETY: both literals are terminated.
        let result = unsafe { tcl_utf_ncmp(c"héllo".as_ptr(), c"héllo".as_ptr(), 5) };
        assert_eq!(result, TCL_OK);
        // SAFETY: as above.
        let shorter = unsafe { tcl_utf_ncmp(c"ab".as_ptr(), c"abc".as_ptr(), 3) };
        assert_eq!(shorter, -1);
    }
}
