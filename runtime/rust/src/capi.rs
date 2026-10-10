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

//! The C Tcl API exports (`#[no_mangle] extern "C"`).
//!
//! These are the runtime side of `c-extension-abi.md` §4.3 (direct C-ABI
//! imports): each is exported with the C ABI so an extension's WASM imports it
//! from the runtime, and each is a function the WASM leg of
//! `include/tcl.h` declares (`make check-c-extension-wasm` holds the two to
//! each other). The ownership/error category of every function here is fixed
//! by `c-api-ownership-contract.md`, and `make check-c-api-ownership` requires
//! its row. Exported: command registration, object construction, the typed
//! reads and the option-table lookup, lists, the interpreter's result and error
//! state, `Tcl_PkgProvideEx` and two UTF-8 helpers; the rest of the 81-function
//! surface is not.

#![allow(non_snake_case)]

use core::ffi::{CStr, c_char, c_int, c_long, c_void};

use tcl_cmd_core::prefix::{self, Resolution};

use crate::cmd_package::provide_package;
use crate::interp::{Code, Interp, ObjCommand, TclCmdDeleteProc, TclObjCmdProc, error_code_list};
use crate::list::{self, append_list_element};
use crate::namespace::RenameOutcome;
use crate::obj::{self, TclObj, TclObjType, TclSize, TclWideInt};
use crate::parse::{self, ListError};
mod scalar;
pub(crate) use scalar::publish_access_error as scalar_publish_access_error;
pub(crate) use scalar::read_scalar_for_interpreter;
pub use scalar::{bind_scalar_getter_context, probe_scalar_getter, NativeScalarObjectAccessError};
use tcl_syntax::scalar_getter::{
    NativeScalarGetterKind as ScalarKind, NativeScalarGetterValue as ScalarValue,
};

const TCL_OK: c_int = 0;
const TCL_ERROR: c_int = 1;
const TCL_EXACT: c_int = 1;
const TCL_NULL_OK: c_int = 32;
const TCL_INDEX_TEMP_TABLE: c_int = 64;

// command registration

/// `Tcl_CreateObjCommand` — bind `cmdName` to the extension's `proc`, which a
/// dispatch of that command calls with `clientData` and the call's words;
/// `deleteProc`, when given, runs with `clientData` when the command goes.
/// Answers an opaque token for the new command (never null on success), or null
/// when there is nothing to bind (a null interpreter, name or procedure).
///
/// `proc` is a function pointer: on `wasm32`, an index into the shared
/// `__indirect_function_table` the extension's module installed its procedure
/// in (`c-extension-abi.md` §4.5).
///
/// # Safety
/// `interp` must be a live `Interp`; `cmdName` a NUL-terminated string;
/// `proc` a valid `Tcl_ObjCmdProc` that stays callable for as long as the
/// command lives.
#[no_mangle]
pub unsafe extern "C" fn Tcl_CreateObjCommand(
    interp: *mut Interp,
    cmdName: *const c_char,
    proc_: Option<TclObjCmdProc>,
    clientData: *mut c_void,
    deleteProc: Option<TclCmdDeleteProc>,
) -> *mut c_void {
    let (Some(proc_), false, false) = (proc_, interp.is_null(), cmdName.is_null()) else {
        return core::ptr::null_mut();
    };
    // SAFETY: caller guarantees a terminated name and a live interpreter.
    let (name, interp) = unsafe { (CStr::from_ptr(cmdName).to_bytes(), &mut *interp) };
    let _scope = match crate::interp::native_operation_currency::NativeOperationScope::enter(interp)
    {
        Ok(scope) => scope,
        Err(cause) => {
            interp.refuse_native_execution(cause);
            return core::ptr::null_mut();
        }
    };
    let created = interp.create_obj_command(name, ObjCommand::new(proc_, clientData, deleteProc));
    if _scope.currency().ensure_current_or_refuse().is_err() {
        return core::ptr::null_mut();
    }
    created
        .and_then(|generation| generation.checked_add(1))
        .and_then(|token| usize::try_from(token).ok())
        .map_or(core::ptr::null_mut(), |token| token as *mut c_void)
}

/// `Tcl_DeleteCommand` — delete the command `cmdName` names, as `rename
/// cmdName {}` does. `0` when there was one, `-1` when there was none.
///
/// # Safety
/// `interp` must be a live `Interp`; `cmdName` a NUL-terminated string.
#[no_mangle]
pub unsafe extern "C" fn Tcl_DeleteCommand(interp: *mut Interp, cmdName: *const c_char) -> c_int {
    if interp.is_null() || cmdName.is_null() {
        return -1;
    }
    // SAFETY: caller guarantees a terminated name and a live interpreter.
    let (name, interp) = unsafe { (CStr::from_ptr(cmdName).to_bytes(), &mut *interp) };
    let _scope = match crate::interp::native_operation_currency::NativeOperationScope::enter(interp)
    {
        Ok(scope) => scope,
        Err(cause) => {
            interp.refuse_native_execution(cause);
            return -1;
        }
    };
    let outcome = interp.rename_command(name, b"");
    if _scope.currency().ensure_current_or_refuse().is_err() {
        return -1;
    }
    match outcome {
        RenameOutcome::Deleted => 0,
        _ => -1,
    }
}

// object creation (all `fresh_zero` — refCount 0)

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

// refcount management

/// `TclFreeObj` — free an object whose count the header's `Tcl_DecrRefCount`
/// macro has lowered to zero or below (the macro frees through this when the
/// count was one or less, a fresh object included). `Tcl_IncrRefCount`,
/// `Tcl_DecrRefCount` and `Tcl_IsShared` are macros over `refCount` in the
/// header, so an extension reaches the allocator's free path only here.
///
/// # Safety
/// `obj` must be null or a live `TclObj` that no reference holds; it is dangling
/// afterwards.
#[no_mangle]
pub unsafe extern "C" fn TclFreeObj(obj: *mut TclObj) {
    // SAFETY: forwarded per contract.
    unsafe { obj::free_obj(obj) }
}

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

// result

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

// string rep

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

// objects: the integer, list and copying constructors (all `fresh_zero`)

/// `Tcl_NewIntObj` — an integer object, refCount 0.
#[no_mangle]
pub extern "C" fn Tcl_NewIntObj(intValue: c_int) -> *mut TclObj {
    obj::new_wide_int_obj(TclWideInt::from(intValue))
}

/// `Tcl_NewLongObj` — an integer object, refCount 0. `long` is the target's:
/// 32 bits on `wasm32`, 64 on an LP64 host.
#[no_mangle]
pub extern "C" fn Tcl_NewLongObj(longValue: c_long) -> *mut TclObj {
    obj::new_wide_int_obj(wide_of_long(longValue))
}

/// `Tcl_NewListObj` — a list of `objv[..objc]`, each retained, refCount 0.
///
/// # Safety
/// `objv` must reference `objc` live objects, or be null when `objc` is zero.
#[no_mangle]
pub unsafe extern "C" fn Tcl_NewListObj(objc: TclSize, objv: *const *mut TclObj) -> *mut TclObj {
    // SAFETY: forwarded per this fn's contract.
    list::new_list_obj(unsafe { words(objc, objv) })
}

/// `Tcl_DuplicateObj` — a copy of the value and its internal rep, refCount 0.
///
/// # Safety
/// `objPtr` must be live.
#[no_mangle]
pub unsafe extern "C" fn Tcl_DuplicateObj(objPtr: *mut TclObj) -> *mut TclObj {
    obj::duplicate(objPtr)
}

// objects: the typed reads, each leaving C Tcl's error in `interp` when it is
// given one

/// `Tcl_GetIntFromObj` — the selected original C primitive's returned `int`.
/// Missing engine, ABI or object ownership is a typed host refusal.
///
/// # Safety
/// `interp` must be null or live; `objPtr` live; `intPtr` writable.
#[no_mangle]
pub unsafe extern "C" fn Tcl_GetIntFromObj(
    interp: *mut Interp,
    objPtr: *mut TclObj,
    intPtr: *mut c_int,
) -> c_int {
    // SAFETY: forwarded from this entry's original-object contract.
    match unsafe { scalar::read(interp, objPtr, ScalarKind::Int) } {
        Ok(ScalarValue::Wide(value)) => match scalar::output_integer::<c_int>(value) {
            Some(value) => {
                // SAFETY: the caller supplies the writable success output.
                unsafe { intPtr.write(value) };
                TCL_OK
            }
            // SAFETY: the original interpreter is null or live.
            None => unsafe { scalar::unexpected_output(interp) },
        },
        // SAFETY: as above; an unexpected output is not a guest overflow.
        Ok(_) => unsafe { scalar::unexpected_output(interp) },
        Err(code) => code,
    }
}

/// `Tcl_GetLongFromObj` — the selected native-long getter and its actual ABI.
/// A Wide getter or this Rust target's size cannot choose the Long recipe.
///
/// # Safety
/// `interp` must be null or live; `objPtr` live; `longPtr` writable.
#[no_mangle]
pub unsafe extern "C" fn Tcl_GetLongFromObj(
    interp: *mut Interp,
    objPtr: *mut TclObj,
    longPtr: *mut c_long,
) -> c_int {
    // SAFETY: forwarded from this entry's original-object contract.
    match unsafe { scalar::read(interp, objPtr, ScalarKind::Long) } {
        Ok(ScalarValue::Wide(value)) => match scalar::output_integer::<c_long>(value) {
            Some(value) => {
                // SAFETY: the caller supplies the writable success output.
                unsafe { longPtr.write(value) };
                TCL_OK
            }
            // SAFETY: the original interpreter is null or live.
            None => unsafe { scalar::unexpected_output(interp) },
        },
        // SAFETY: as above; this cannot invent a guest Long failure.
        Ok(_) => unsafe { scalar::unexpected_output(interp) },
        Err(code) => code,
    }
}

/// `Tcl_GetWideIntFromObj` — the selected original wide-integer primitive.
///
/// # Safety
/// `interp` must be null or live; `objPtr` live; `widePtr` writable.
#[no_mangle]
pub unsafe extern "C" fn Tcl_GetWideIntFromObj(
    interp: *mut Interp,
    objPtr: *mut TclObj,
    widePtr: *mut TclWideInt,
) -> c_int {
    // SAFETY: forwarded from this entry's original-object contract.
    match unsafe { scalar::read(interp, objPtr, ScalarKind::Wide) } {
        Ok(ScalarValue::Wide(value)) => {
            // SAFETY: the caller supplies the writable success output.
            unsafe { widePtr.write(value) };
            TCL_OK
        }
        // SAFETY: an unexpected owner output remains a host refusal.
        Ok(_) => unsafe { scalar::unexpected_output(interp) },
        Err(code) => code,
    }
}

/// `Tcl_GetBooleanFromObj` — the public primitive's returned C integer.
/// Jim can return an existing cached integer's C cast rather than `0` or `1`.
/// Expression truth conversion is an independently selected purpose.
///
/// # Safety
/// `interp` must be null or live; `objPtr` live; `intPtr` writable.
#[no_mangle]
pub unsafe extern "C" fn Tcl_GetBooleanFromObj(
    interp: *mut Interp,
    objPtr: *mut TclObj,
    intPtr: *mut c_int,
) -> c_int {
    // SAFETY: forwarded from this entry's original-object contract.
    match unsafe { scalar::read(interp, objPtr, ScalarKind::Boolean) } {
        Ok(ScalarValue::Boolean(value)) => {
            // SAFETY: the caller supplies the writable success output.
            unsafe { intPtr.write(value.returned_integer()) };
            TCL_OK
        }
        // SAFETY: an unexpected owner output remains a host refusal.
        Ok(_) => unsafe { scalar::unexpected_output(interp) },
        Err(code) => code,
    }
}

/// `Tcl_GetDoubleFromObj` — the original double primitive, including its
/// selected NaN failure, cache changes and private error-code update.
///
/// # Safety
/// `interp` must be null or live; `objPtr` live; `doublePtr` writable.
#[no_mangle]
pub unsafe extern "C" fn Tcl_GetDoubleFromObj(
    interp: *mut Interp,
    objPtr: *mut TclObj,
    doublePtr: *mut f64,
) -> c_int {
    // SAFETY: forwarded from this entry's original-object contract.
    match unsafe { scalar::read(interp, objPtr, ScalarKind::Double) } {
        Ok(ScalarValue::Double(value)) => {
            // SAFETY: the caller supplies the writable success output.
            unsafe { doublePtr.write(value) };
            TCL_OK
        }
        // SAFETY: an unexpected owner output remains a host refusal.
        Ok(_) => unsafe { scalar::unexpected_output(interp) },
        Err(code) => code,
    }
}

/// The index an option table lookup resolved, kept on the word it resolved: the
/// internal rep is the address of the table's entry, so [`Tcl_WrongNumArgs`]
/// can spell an abbreviated word in full, as C Tcl's `tclIndexType` lets it.
/// The string rep is the word's own and is never regenerated.
static TCL_INDEX_TYPE: TclObjType = TclObjType {
    name: c"index".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};

/// The table entry a word resolved to through [`Tcl_GetIndexFromObjStruct`],
/// if it did.
fn index_entry(word: *mut TclObj) -> Option<Vec<u8>> {
    if obj::obj_type_ptr(word) != &TCL_INDEX_TYPE {
        return None;
    }
    let entry = obj::internal_rep(word) as usize as *const c_char;
    // SAFETY: the rep was set from a live table's terminated entry, which an
    // extension's static table keeps for as long as it is loaded.
    Some(unsafe { CStr::from_ptr(entry) }.to_bytes().to_vec())
}

/// `Tcl_GetIndexFromObjStruct` (and, through the header's macro,
/// `Tcl_GetIndexFromObj`): resolve the word against a NULL-terminated table of
/// strings `offset` bytes apart, with C Tcl's unique-prefix rule
/// (`TCL_EXACT` asks for an exact match) and its messages, and write the index
/// at the width the header encoded into `flags`. `TCL_NULL_OK` takes the empty
/// word as `-1`; `TCL_INDEX_TEMP_TABLE` keeps nothing on the word.
///
/// # Safety
/// `interp` must be null or live; `objPtr` null or live; `tablePtr` such a
/// table; `msg` a terminated string; `indexPtr` null or writable at the width
/// `flags` encodes.
#[no_mangle]
pub unsafe extern "C" fn Tcl_GetIndexFromObjStruct(
    interp: *mut Interp,
    objPtr: *mut TclObj,
    tablePtr: *const c_void,
    offset: TclSize,
    msg: *const c_char,
    flags: c_int,
    indexPtr: *mut c_void,
) -> c_int {
    if offset < core::mem::size_of::<*const c_char>() as TclSize {
        let message = format!("Invalid struct offset value {offset}.");
        // SAFETY: forwarded per this fn's contract.
        return unsafe { report(interp, message.as_bytes(), None) };
    }
    // SAFETY: forwarded per this fn's contract.
    let entries = unsafe { read_table(tablePtr, offset) };
    let names: Vec<&[u8]> = entries.iter().map(|(name, _)| name.as_slice()).collect();
    // SAFETY: as above.
    let what = unsafe { c_bytes(msg) };
    let key = if objPtr.is_null() {
        Vec::new()
    } else {
        obj::bytes_of(objPtr)
    };
    let null_ok = flags & TCL_NULL_OK != 0;
    let resolution = prefix::scan(&names, &key, flags & TCL_EXACT != 0);
    let index = match resolution {
        _ if key.is_empty() && null_ok => -1,
        Resolution::Exact(index) | Resolution::UniquePrefix(index) => index as TclSize,
        Resolution::Ambiguous | Resolution::NoMatch => {
            let ambiguous = matches!(resolution, Resolution::Ambiguous);
            let message = if null_ok {
                let mut message = if ambiguous {
                    b"ambiguous ".to_vec()
                } else {
                    b"bad ".to_vec()
                };
                message.extend_from_slice(&what);
                message.extend_from_slice(b" \"");
                message.extend_from_slice(&key);
                message.extend_from_slice(b"\": must be ");
                for (position, name) in names.iter().enumerate() {
                    if position > 0 {
                        message.extend_from_slice(b", ");
                    }
                    message.extend_from_slice(name);
                }
                message.extend_from_slice(b", or \"\"");
                message
            } else {
                prefix::bad_key_message(&names, &what, &key, ambiguous)
            };
            let code = error_code_list(&[b"TCL", b"LOOKUP", b"INDEX", &what, &key]);
            // SAFETY: forwarded per this fn's contract.
            return unsafe { report(interp, &message, Some(code.as_slice())) };
        }
    };
    if index >= 0 && !objPtr.is_null() && flags & TCL_INDEX_TEMP_TABLE == 0 {
        let entry = entries[index as usize].1;
        obj::change_type(objPtr, &TCL_INDEX_TYPE, entry as usize as u64);
    }
    if !indexPtr.is_null() {
        // SAFETY: the header encoded `sizeof(*indexPtr) << 1` into these bits,
        // so the pointer is writable at exactly the width each arm writes.
        unsafe {
            match flags & 0b1_0110 {
                2 => indexPtr.cast::<u8>().write(index as u8),
                4 => indexPtr.cast::<u16>().write(index as u16),
                16 => indexPtr.cast::<i64>().write(index as i64),
                _ => indexPtr.cast::<i32>().write(index as i32),
            }
        }
    }
    TCL_OK
}

/// The entries of a NULL-terminated table with `offset` bytes between them, each
/// with the address of its string.
///
/// # Safety
/// `table` must be such a table.
unsafe fn read_table(table: *const c_void, offset: TclSize) -> Vec<(Vec<u8>, *const c_char)> {
    let mut entries = Vec::new();
    let mut cursor = table.cast::<u8>();
    loop {
        // SAFETY: each slot holds a `const char *`, and the table ends with NULL.
        let entry = unsafe { cursor.cast::<*const c_char>().read_unaligned() };
        if entry.is_null() {
            return entries;
        }
        // SAFETY: each entry is a terminated string.
        entries.push((unsafe { CStr::from_ptr(entry) }.to_bytes().to_vec(), entry));
        // SAFETY: the table extends by `offset` for each entry before its end.
        cursor = unsafe { cursor.offset(offset) };
    }
}

// lists

/// `Tcl_ListObjAppendElement` — append `objPtr`, retained, to an unshared list
/// in place. A shared list is refused with an error where C Tcl panics.
///
/// # Safety
/// `interp` must be null or live; `listPtr` and `objPtr` live.
#[no_mangle]
pub unsafe extern "C" fn Tcl_ListObjAppendElement(
    interp: *mut Interp,
    listPtr: *mut TclObj,
    objPtr: *mut TclObj,
) -> c_int {
    if obj::is_shared(listPtr) {
        // SAFETY: forwarded per this fn's contract.
        return unsafe {
            report(
                interp,
                b"Tcl_ListObjAppendElement called with shared object",
                None,
            )
        };
    }
    match list::list_append(listPtr, objPtr) {
        Ok(()) => TCL_OK,
        // SAFETY: forwarded per this fn's contract.
        Err(error) => unsafe { list_error(interp, listPtr, error) },
    }
}

/// `Tcl_ListObjGetElements` — the list's length and its own element array, good
/// until the list changes or is freed.
///
/// # Safety
/// `interp` must be null or live; `listPtr` live; `objcPtr` and `objvPtr`
/// writable.
#[no_mangle]
pub unsafe extern "C" fn Tcl_ListObjGetElements(
    interp: *mut Interp,
    listPtr: *mut TclObj,
    objcPtr: *mut TclSize,
    objvPtr: *mut *mut *mut TclObj,
) -> c_int {
    match list::elements_raw(listPtr) {
        Ok((elements, count)) => {
            // SAFETY: the caller guarantees writable out-pointers.
            unsafe {
                objcPtr.write(count as TclSize);
                objvPtr.write(elements);
            }
            TCL_OK
        }
        // SAFETY: forwarded per this fn's contract.
        Err(error) => unsafe { list_error(interp, listPtr, error) },
    }
}

/// `Tcl_ListObjLength` — the number of elements, shimmering a string to a list.
///
/// # Safety
/// `interp` must be null or live; `listPtr` live; `lengthPtr` writable.
#[no_mangle]
pub unsafe extern "C" fn Tcl_ListObjLength(
    interp: *mut Interp,
    listPtr: *mut TclObj,
    lengthPtr: *mut TclSize,
) -> c_int {
    match list::list_length(listPtr) {
        Ok(length) => {
            // SAFETY: the caller guarantees a writable size.
            unsafe { lengthPtr.write(length as TclSize) };
            TCL_OK
        }
        // SAFETY: forwarded per this fn's contract.
        Err(error) => unsafe { list_error(interp, listPtr, error) },
    }
}

// the interpreter's error state

/// `Tcl_ResetResult` — an empty result, and no error or `return` in flight.
///
/// # Safety
/// `interp` must be live.
#[no_mangle]
pub unsafe extern "C" fn Tcl_ResetResult(interp: *mut Interp) {
    // SAFETY: caller guarantees `interp` is live.
    unsafe { (*interp).reset_result() }
}

/// `Tcl_WrongNumArgs` — the error `wrong # args: should be "…"` with the first
/// `objc` words, each quoted as a list element and a word an option lookup
/// resolved spelt as its table entry, then `message` when there is one;
/// `-errorcode TCL WRONGARGS`.
///
/// # Safety
/// `interp` must be live; `objv` must reference `objc` live objects; `message`
/// null or a terminated string.
#[no_mangle]
pub unsafe extern "C" fn Tcl_WrongNumArgs(
    interp: *mut Interp,
    objc: TclSize,
    objv: *const *mut TclObj,
    message: *const c_char,
) {
    // SAFETY: forwarded per this fn's contract.
    let words = unsafe { words(objc, objv) };
    let mut text = b"wrong # args: should be \"".to_vec();
    for (position, &word) in words.iter().enumerate() {
        let spelling = index_entry(word).unwrap_or_else(|| obj::bytes_of(word));
        append_list_element(&mut text, &spelling, true);
        if position + 1 < words.len() || !message.is_null() {
            text.push(b' ');
        }
    }
    if !message.is_null() {
        // SAFETY: as above.
        text.extend_from_slice(&unsafe { c_bytes(message) });
    }
    text.push(b'"');
    // SAFETY: as above.
    unsafe { report(interp, &text, Some(b"TCL WRONGARGS".as_slice())) };
}

/// `Tcl_SetObjErrorCode` — the `-errorcode` of the error the running command
/// returns. The interpreter keeps the code's text, so a fresh object is freed.
///
/// # Safety
/// `interp` must be live; `errorObjPtr` live.
#[no_mangle]
pub unsafe extern "C" fn Tcl_SetObjErrorCode(interp: *mut Interp, errorObjPtr: *mut TclObj) {
    let code = obj::bytes_of(errorObjPtr);
    // SAFETY: caller guarantees both pointers are live; the hold and release
    // free an object no one else holds, as storing it and dropping it would.
    unsafe {
        (*interp).set_c_error_code(&code);
        obj::incr_ref_count(errorObjPtr);
        obj::decr_ref_count(errorObjPtr);
    }
}

/// The fixed-arity export behind the header's inline `Tcl_SetResult`: the
/// result is a copy of `result` (empty for null).
///
/// # Safety
/// `interp` must be live; `result` null or a terminated string.
#[no_mangle]
pub unsafe extern "C" fn TclHost_SetResultString(interp: *mut Interp, result: *const c_char) {
    // SAFETY: caller guarantees a live interpreter and a terminated string.
    unsafe { (*interp).set_result_bytes(&c_bytes(result)) }
}

/// The fixed-arity export behind the header's inline `Tcl_AppendResult`: the
/// result grows by `piece` (nothing for null).
///
/// # Safety
/// `interp` must be live; `piece` null or a terminated string.
#[no_mangle]
pub unsafe extern "C" fn TclHost_AppendResultString(interp: *mut Interp, piece: *const c_char) {
    // SAFETY: caller guarantees a live interpreter and a terminated string.
    unsafe { (*interp).append_result_bytes(&c_bytes(piece)) }
}

// packages

/// `Tcl_PkgProvideEx` (and the header's `Tcl_PkgProvide`) — `package provide
/// name version`: a version already provided differently is Tcl's error, left
/// in the result, and a package provided leaves the caller's result as it was.
/// `clientData` is not kept.
///
/// # Safety
/// `interp` must be live; `name` and `version` terminated strings.
#[no_mangle]
pub unsafe extern "C" fn Tcl_PkgProvideEx(
    interp: *mut Interp,
    name: *const c_char,
    version: *const c_char,
    clientData: *const c_void,
) -> c_int {
    let _ = clientData;
    // SAFETY: caller guarantees a live interpreter and terminated strings.
    let (interp, name, version) = unsafe { (&mut *interp, c_bytes(name), c_bytes(version)) };
    match provide_package(interp, &name, &version) {
        Code::Ok => TCL_OK,
        _ => {
            if !interp.host_refusal_pending() {
                interp.note_c_api_error();
            }
            TCL_ERROR
        }
    }
}

// UTF-8

/// `Tcl_NumUtfChars` — the characters in `src[..length]`, or up to the
/// terminator for a negative `length`, each read as [`decode`] reads it. Near
/// the end of `src[..length]`, a byte that starts a sequence longer than the
/// bytes left is a character of its own, as C Tcl's `Tcl_UtfCharComplete`
/// makes it.
///
/// # Safety
/// `src` must reference `length` readable bytes, or be a terminated string when
/// `length` is negative (it may be null when `length` is zero).
#[no_mangle]
pub unsafe extern "C" fn Tcl_NumUtfChars(src: *const c_char, length: TclSize) -> TclSize {
    if src.is_null() {
        return 0;
    }
    let src = src.cast::<u8>();
    let mut count: TclSize = 0;
    if length < 0 {
        let mut at = src;
        // SAFETY: the caller guarantees a terminated string, and the NUL that
        // ends it ends every sequence `next_char` reads.
        while unsafe { *at } != 0 {
            // SAFETY: as above.
            unsafe { next_char(&mut at) };
            count += 1;
        }
        return count;
    }
    let length = length as usize;
    let mut offset = 0;
    while offset < length {
        // SAFETY: `offset` is within the `length` bytes the caller guarantees.
        let at = unsafe { src.add(offset) };
        // SAFETY: as above.
        let lead = unsafe { *at };
        offset += if length - offset >= complete_width(lead) {
            // SAFETY: the bytes the lead byte asks for are within `length`.
            unsafe { decode(at) }.1
        } else {
            1
        };
        count += 1;
    }
    count
}

/// `Tcl_UtfNcmp` — compare the first `numChars` characters of `cs` and `ct`,
/// answering the difference of the first pair that differs. As C Tcl's, it reads
/// `numChars` characters from each whatever bytes they hold, so a NUL inside a
/// value is a character like any other.
///
/// # Safety
/// `cs` and `ct` must each hold at least `numChars` characters.
#[no_mangle]
pub unsafe extern "C" fn Tcl_UtfNcmp(
    cs: *const c_char,
    ct: *const c_char,
    numChars: usize,
) -> c_int {
    let (mut left, mut right) = (cs.cast::<u8>(), ct.cast::<u8>());
    for _ in 0..numChars {
        // SAFETY: the caller guarantees `numChars` characters in each.
        let (one, two) = unsafe { (next_char(&mut left), next_char(&mut right)) };
        if one != two {
            return one as c_int - two as c_int;
        }
    }
    0
}

/// The character at `*at`, advancing past it.
///
/// # Safety
/// As [`decode`].
unsafe fn next_char(at: &mut *const u8) -> u32 {
    // SAFETY: forwarded per this fn's contract.
    let (character, width) = unsafe { decode(*at) };
    // SAFETY: the character was `width` readable bytes.
    *at = unsafe { at.add(width) };
    character
}

/// The characters C Tcl reads the naked trail bytes `0x80` to `0x9F` as: the
/// ones cp1252 gives those bytes.
const CP1252: [u16; 32] = [
    0x20AC, 0x0081, 0x201A, 0x0192, 0x201E, 0x2026, 0x2020, 0x2021, 0x02C6, 0x2030, 0x0160, 0x2039,
    0x0152, 0x008D, 0x017D, 0x008F, 0x0090, 0x2018, 0x2019, 0x201C, 0x201D, 0x2022, 0x2013, 0x2014,
    0x02DC, 0x2122, 0x0161, 0x203A, 0x0153, 0x009D, 0x017E, 0x0178,
];

/// The character at `at` and the bytes it takes, as C Tcl's `Tcl_UtfToUniChar`
/// reads them. A lead byte with its trail bytes is the code point they encode
/// when that is its shortest form, and `C0 80`, C Tcl's NUL inside a value, is
/// a NUL; a naked trail byte from `0x80` to `0x9F` is the character cp1252 gives
/// it; and any other byte, a lead byte whose sequence is cut short or overlong
/// among them, is a character of its own value.
///
/// # Safety
/// `at` must be readable, and so must each byte after it up to the first that
/// is not a trail byte or the last of the sequence its lead byte starts.
unsafe fn decode(at: *const u8) -> (u32, usize) {
    // SAFETY: forwarded per this fn's contract.
    let lead = unsafe { *at };
    // The low six bits of the byte `step` on when it is a trail byte. Each is
    // read only once every byte before it was one.
    let trail = |step: usize| {
        // SAFETY: forwarded per this fn's contract.
        let byte = unsafe { *at.add(step) };
        (byte & 0xC0 == 0x80).then_some(u32::from(byte & 0x3F))
    };
    let high = |mask: u8, shift: u32| u32::from(lead & mask) << shift;
    let decoded = match lead {
        0x80..=0x9F => Some((u32::from(CP1252[usize::from(lead - 0x80)]), 1)),
        // `C1` starts no sequence: its two-byte forms are all overlong, and
        // `Tcl_UtfCharComplete` asks for one byte for it, so the byte after it
        // is not read.
        0xC0 | 0xC2..=0xDF => trail(1)
            .map(|one| (high(0x1F, 6) | one, 2))
            .filter(|&(character, _)| character == 0 || character >= 0x80),
        0xE0..=0xEF => trail(1)
            .and_then(|one| trail(2).map(|two| (high(0x0F, 12) | (one << 6) | two, 3)))
            .filter(|&(character, _)| character > 0x7FF),
        0xF0..=0xF4 => trail(1)
            .and_then(|one| {
                trail(2).and_then(|two| {
                    trail(3).map(|three| (high(0x07, 18) | (one << 12) | (two << 6) | three, 4))
                })
            })
            .filter(|&(character, _)| (0x1_0000..=0x10_FFFF).contains(&character)),
        _ => None,
    };
    decoded.unwrap_or((u32::from(lead), 1))
}

/// The bytes C Tcl's `Tcl_UtfCharComplete` asks for before a character is read
/// from `lead`: the sequence a lead byte starts, three for a trail byte (which
/// may be the second of four), one for any other byte.
fn complete_width(lead: u8) -> usize {
    match lead {
        0x80..=0xBF | 0xE0..=0xEF => 3,
        0xC0 | 0xC2..=0xDF => 2,
        0xF0..=0xF4 => 4,
        _ => 1,
    }
}

// helpers shared by the exports above

/// `objv[..objc]` as a slice; empty for a null array or a non-positive count.
///
/// # Safety
/// `objv` must reference `objc` live objects when non-null and `objc` is
/// positive.
unsafe fn words<'a>(objc: TclSize, objv: *const *mut TclObj) -> &'a [*mut TclObj] {
    if objv.is_null() || objc <= 0 {
        return &[];
    }
    // SAFETY: forwarded per this fn's contract.
    unsafe { core::slice::from_raw_parts(objv, objc as usize) }
}

/// A terminated C string's bytes, or none for null.
///
/// # Safety
/// `text` must be null or a terminated string.
unsafe fn c_bytes(text: *const c_char) -> Vec<u8> {
    if text.is_null() {
        return Vec::new();
    }
    // SAFETY: forwarded per this fn's contract.
    unsafe { CStr::from_ptr(text) }.to_bytes().to_vec()
}

/// Leave an error in `interp` when the call was given one, answering
/// `TCL_ERROR`: a C call with a null interpreter reports nothing.
///
/// # Safety
/// `interp` must be null or live.
unsafe fn report(interp: *mut Interp, message: &[u8], code: Option<&[u8]>) -> c_int {
    // SAFETY: forwarded per this fn's contract.
    if let Some(interp) = unsafe { interp.as_mut() } {
        interp.c_api_error(message, code);
    }
    TCL_ERROR
}

/// A list that would not parse, reported with the shared list owner's message
/// and code.
///
/// # Safety
/// `interp` must be null or live; `listPtr` live.
unsafe fn list_error(interp: *mut Interp, listPtr: *mut TclObj, error: ListError) -> c_int {
    let source = obj::bytes_of(listPtr);
    let message = parse::list_error_message(&source, error);
    // SAFETY: forwarded per this fn's contract.
    unsafe { report(interp, &message, Some(error.error_code())) }
}

/// A C `long` as a `Tcl_WideInt`, whichever width `long` has.
fn wide_of_long(value: c_long) -> TclWideInt {
    TclWideInt::try_from(i128::from(value)).unwrap_or(TclWideInt::MAX)
}

// runtime interp lifecycle (host entry points, not in tcl.h)
//
// These runtime-specific entry points create and destroy an interpreter for
// the host bridge. They are separate from the extension header's Tcl API.

/// Create a runtime interp; returns an owning raw pointer the caller must pass
/// to [`tcl_runtime_delete_interp`].
#[no_mangle]
pub extern "C" fn tcl_runtime_create_interp() -> *mut Interp {
    // `Interp` is a cheap `Rc` handle; box it so the C side has a stable
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

// leak-check test exports (the `tcl_test_*` surface)

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
