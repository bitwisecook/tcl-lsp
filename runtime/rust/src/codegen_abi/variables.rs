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

//! The compiled frame, its indexed local slots and their read-modify-write commands, and variables addressed by name.

use super::values::unexpected_scalar_output;
use super::{
    input_bytes, original_frame_owner, BorrowedArgv, CodegenOperation, TCL_INVOKE_ABI_HOST_REFUSED,
    TCL_VALUE_GET_ERROR, TCL_VALUE_GET_OK,
};
use crate::interp::Interp;
use crate::obj;
use crate::obj::{new_string_bytes, TclObj};
use core::ptr;

/// Attempt a name-addressable Tcl frame on the explicitly installed interpreter.
/// Every attempt, including refusal, requires exactly one matching pop.
#[no_mangle]
pub extern "C" fn tcl_codegen_frame_push() {
    original_frame_owner::frame_push();
}

/// Retire the matching original frame attempt, independent of mutable TLS.
#[no_mangle]
pub extern "C" fn tcl_codegen_frame_pop() {
    original_frame_owner::frame_pop();
}

/// Bind a compiled slot index to the Tcl-visible variable cell and store value.
///
/// # Safety
/// The name range must be readable, and `value` must be a live owned reference.
#[no_mangle]
pub unsafe extern "C" fn tcl_codegen_local_bind(
    slot: i32,
    name_ptr: *const u8,
    name_len: i32,
    value: *mut TclObj,
) -> i32 {
    // SAFETY: a non-null input transfers exactly one generated +1 reference.
    let value = (!value.is_null()).then(|| unsafe { obj::Owned::from_raw(value) });
    let mut operation = match CodegenOperation::enter() {
        Ok(operation) => operation,
        Err(status) => {
            drop(value);
            return if status == TCL_INVOKE_ABI_HOST_REFUSED {
                status
            } else {
                1
            };
        }
    };
    let Some(value) = value else {
        return 1;
    };
    if slot < 0 {
        drop(value);
        return operation.finish_code(crate::interp::Code::Error);
    }
    let name = unsafe { input_bytes(name_ptr, name_len) };
    let bound = operation
        .interpreter
        .codegen_bind_slot(usize::try_from(slot).unwrap(), name);
    let code = if operation.ensure_current().is_err() {
        crate::interp::Code::Error
    } else {
        match bound {
            Err(error) => operation.interpreter.report_cmd_error(error),
            Ok(()) => {
                let stored = operation.interpreter.var_set(name, value.as_ptr());
                if operation.ensure_current().is_err() {
                    crate::interp::Code::Error
                } else {
                    match stored {
                        Ok(()) => crate::interp::Code::Ok,
                        Err(error) => {
                            crate::builtins::var_error(&mut operation.interpreter, name, error)
                        }
                    }
                }
            }
        }
    };
    drop(value);
    operation.finish_code(code)
}

/// Store through an indexed compiled-local port.
///
/// # Safety
/// `value` must be a live owned reference transferred by generated code.
#[no_mangle]
pub unsafe extern "C" fn tcl_codegen_local_set(slot: i32, value: *mut TclObj) -> i32 {
    // SAFETY: a non-null input transfers exactly one generated +1 reference.
    let value = (!value.is_null()).then(|| unsafe { obj::Owned::from_raw(value) });
    let mut operation = match CodegenOperation::enter() {
        Ok(operation) => operation,
        Err(status) => {
            drop(value);
            return if status == TCL_INVOKE_ABI_HOST_REFUSED {
                status
            } else {
                1
            };
        }
    };
    let Some(value) = value else {
        return 1;
    };
    let Some(name) = slot_name(&operation.interpreter, slot) else {
        drop(value);
        return operation.finish_code(crate::interp::Code::Error);
    };
    let stored = operation.interpreter.var_set(&name, value.as_ptr());
    let code = if operation.ensure_current().is_err() {
        crate::interp::Code::Error
    } else {
        match stored {
            Ok(()) => crate::interp::Code::Ok,
            Err(error) => crate::builtins::var_error(&mut operation.interpreter, &name, error),
        }
    };
    drop(value);
    operation.finish_code(code)
}

/// Load through an indexed port, returning an owned operand-stack value.
///
/// # Safety
/// The current interpreter and generated frame must remain live for the call.
#[no_mangle]
pub unsafe extern "C" fn tcl_codegen_local_get(slot: i32) -> *mut TclObj {
    let Ok(mut operation) = CodegenOperation::enter() else {
        return ptr::null_mut();
    };
    let Ok(slot) = usize::try_from(slot) else {
        return ptr::null_mut();
    };
    if let Some(value) = operation.interpreter.codegen_slot_scalar(slot) {
        if operation.ensure_current().is_err() {
            return ptr::null_mut();
        }
        // SAFETY: the original cell owns this live value.
        unsafe { obj::incr_ref_count(value) };
        return value;
    }
    let Some(name) = operation.interpreter.codegen_slot_name(slot) else {
        return ptr::null_mut();
    };
    let traced = operation.interpreter.fire_read_trace(&name, None);
    if operation.ensure_current().is_err() || traced.is_some() {
        return ptr::null_mut();
    }
    let value = operation.interpreter.var_get(&name);
    if operation.ensure_current().is_err() {
        return ptr::null_mut();
    }
    let Some(value) = value else {
        let message = operation.interpreter.read_miss_msg(&name, None);
        operation.interpreter.set_error(&message);
        let _ = operation.ensure_current();
        return ptr::null_mut();
    };
    unsafe { obj::incr_ref_count(value) };
    value
}

/// `tcl_codegen_var_traced(name_ptr, name_len) -> i32` — whether any variable
/// trace can observe accesses to `name`.
///
/// This is the runtime half of a guarded `TraceBarrier`: generated code that has
/// proved everything *except* the absence of a trace asks this once and takes
/// its native path when the answer is `0`. The name resolves to the same
/// identity trace firing uses, so an `upvar`/`global`/`variable` link reports
/// its **target**'s traces and an array reports its elements'. The answer is
/// conservative in one direction only: `1` may be broader than the exact
/// access, `0` is a promise that nothing can observe the cell.
///
/// Returns `1` when traced or the original operation is unavailable/refused,
/// and `0` only when the current cell owner reports an untraced access.
///
/// # Safety
/// The name range must be readable shared linear memory.
#[no_mangle]
pub unsafe extern "C" fn tcl_codegen_var_traced(name_ptr: *const u8, name_len: i32) -> i32 {
    let Ok(operation) = CodegenOperation::enter() else {
        return 1;
    };
    let name = unsafe { input_bytes(name_ptr, name_len) };
    let traced = operation.interpreter.var_is_traced(name);
    if operation.ensure_current().is_err() {
        1
    } else {
        i32::from(traced)
    }
}

/// `tcl_codegen_slot_traced(slot) -> i32` — [`tcl_codegen_var_traced`] for the
/// variable a compiled slot addresses.
///
/// Answered from the cell's own trace bit, which is validated against the
/// interpreter's variable-trace epoch — so a repeated guard costs one array
/// index, and a trace added or removed anywhere (including a proc frame's
/// teardown dropping its locals' traces) forces the answer to be recomputed
/// rather than trusted.
#[no_mangle]
pub extern "C" fn tcl_codegen_slot_traced(slot: i32) -> i32 {
    let Ok(operation) = CodegenOperation::enter() else {
        return 1;
    };
    let Ok(slot) = usize::try_from(slot) else {
        return 1;
    };
    let traced = operation.interpreter.codegen_slot_is_traced(slot);
    if operation.ensure_current().is_err() {
        1
    } else {
        i32::from(traced)
    }
}

/// Resolve a compiled slot's Tcl-visible name, or `None` when it is unbound.
fn slot_name(interp: &Interp, slot: i32) -> Option<Vec<u8>> {
    interp.codegen_slot_name(usize::try_from(slot).ok()?)
}

/// Run one of the runtime's own read-modify-write commands over a compiled
/// slot's variable.
///
/// The slot contributes the **addressing** — an O(1) index to the name its cell
/// is bound to — and the command contributes the **semantics**. There is no
/// second implementation of `incr`/`append`/`lappend` here: the runtime's
/// command runs over a prebuilt argv, so the numeric tower's bignum promotion,
/// the copy-on-write in-place growth, `const`, write traces, and every error
/// message are exactly what interpreted Tcl produces.
///
/// Returns the Tcl completion code.
///
/// **Ownership.** The borrow guard's `+1` is released after the call, so a
/// word's fate is decided by the reference it arrived with: the two words
/// this function creates are `rc 0`, so that release is what frees them,
/// while a caller-owned `value` (`rc >= 1`) survives it. A caller handing in
/// a *fresh* `rc 0` value must therefore not release it again — that second
/// release is a double free (`tcl_codegen_slot_incr_i64` pins its operand
/// for exactly this reason). This is the same rule
/// [`crate::codegen_native::run_named_cell_command`] states for the named
/// spelling of the same call.
///
/// # Safety
/// `interp` must be the live current interpreter and `value` a live object.
unsafe fn slot_modify(
    interp: &mut Interp,
    slot: i32,
    head: &[u8],
    value: Option<*mut TclObj>,
    command: fn(&mut Interp, &[*mut TclObj]) -> crate::interp::Code,
) -> crate::interp::Code {
    let Some(name) = slot_name(interp, slot) else {
        let mut message = head.to_vec();
        message.extend_from_slice(b" on an unbound compiled slot");
        return interp.set_error(&message);
    };
    let head_obj = new_string_bytes(head);
    let name_obj = new_string_bytes(&name);
    let mut argv = vec![head_obj, name_obj];
    argv.extend(value);
    // The command borrows its argv; pin every word for the call exactly as the
    // dispatcher does. The borrow's `+1` is the only reference `head_obj` and
    // `name_obj` have, so releasing it also frees them; a `drop_fresh` here
    // would be a second release of each.
    // SAFETY: every word is live for the whole call.
    let borrowed = unsafe { BorrowedArgv::retain(&argv) };
    let code = command(interp, &argv);
    drop(borrowed);
    code
}

/// `tcl_codegen_slot_incr_i64(slot, delta, out) -> status` — Tcl `incr` on the
/// variable a compiled slot addresses, writing the new value through `out`.
///
/// Full `incr` semantics: the sum promotes to a bignum on overflow (Tcl
/// integers never wrap), an existing bignum cell increments correctly, a
/// non-integer cell raises C's `expected integer but got …`, and `const` and
/// write traces apply. [`TCL_VALUE_GET_OK`] means `*out` was written;
/// [`TCL_VALUE_GET_ERROR`] means the interpreter carries the Tcl error and
/// `*out` is untouched — including the case where the new value is a bignum
/// past the wide range, which is `integer value too large to represent` just as
/// reading it as an `i64` anywhere else would be.
///
/// # Safety
/// `out` must be null or writable, properly aligned `i64` storage.
#[no_mangle]
pub unsafe extern "C" fn tcl_codegen_slot_incr_i64(slot: i32, delta: i64, out: *mut i64) -> i32 {
    let Ok(mut operation) = CodegenOperation::enter() else {
        return TCL_VALUE_GET_ERROR;
    };
    let delta = obj::Owned::fresh(obj::new_wide_int_obj(delta));
    let code = unsafe {
        slot_modify(
            &mut operation.interpreter,
            slot,
            b"incr",
            Some(delta.as_ptr()),
            crate::cmd_var::installed_incr(),
        )
    };
    drop(delta);
    if operation.ensure_current().is_err() || code != crate::interp::Code::Ok {
        return TCL_VALUE_GET_ERROR;
    }
    if out.is_null() {
        return TCL_VALUE_GET_OK;
    }
    let value = operation.interpreter.result_obj();
    match operation.scalar(
        value,
        tcl_syntax::scalar_getter::NativeScalarGetterKind::Wide,
    ) {
        Ok(tcl_syntax::scalar_getter::NativeScalarGetterValue::Wide(value)) => {
            unsafe { out.write(value) };
            TCL_VALUE_GET_OK
        }
        Ok(_) => unexpected_scalar_output(&mut operation.interpreter),
        Err(_) => TCL_VALUE_GET_ERROR,
    }
}

/// `tcl_codegen_slot_append(slot, value) -> code` — Tcl `append` onto the
/// variable a compiled slot addresses.
///
/// Copy-on-write is the runtime's own: an unshared plain string grows in place
/// (amortised O(1)), anything else copies. `value` is borrowed. Returns the Tcl
/// completion code (`0` ok, `1` error with the interpreter's message set).
///
/// # Safety
/// `value` must be a live object with a caller-owned reference.
#[no_mangle]
pub unsafe extern "C" fn tcl_codegen_slot_append(slot: i32, value: *mut TclObj) -> i32 {
    let mut operation = match CodegenOperation::enter() {
        Ok(operation) => operation,
        Err(status) => {
            return if status == TCL_INVOKE_ABI_HOST_REFUSED {
                status
            } else {
                1
            };
        }
    };
    if value.is_null() {
        return 1;
    }
    let code = unsafe {
        slot_modify(
            &mut operation.interpreter,
            slot,
            b"append",
            Some(value),
            crate::cmd_string::append,
        )
    };
    operation.finish_code(code)
}

/// `tcl_codegen_slot_lappend(slot, value) -> code` — Tcl `lappend` onto the
/// variable a compiled slot addresses, with the runtime's copy-on-write list
/// growth (an uniquely owned backing vector is extended in place).
///
/// `value` is borrowed. Returns the Tcl completion code.
///
/// # Safety
/// `value` must be a live object with a caller-owned reference.
#[no_mangle]
pub unsafe extern "C" fn tcl_codegen_slot_lappend(slot: i32, value: *mut TclObj) -> i32 {
    let mut operation = match CodegenOperation::enter() {
        Ok(operation) => operation,
        Err(status) => {
            return if status == TCL_INVOKE_ABI_HOST_REFUSED {
                status
            } else {
                1
            };
        }
    };
    if value.is_null() {
        return 1;
    }
    let code = unsafe {
        slot_modify(
            &mut operation.interpreter,
            slot,
            b"lappend",
            Some(value),
            crate::cmd_list::lappend,
        )
    };
    operation.finish_code(code)
}

/// `tcl_codegen_slot_get(slot) -> obj` — the ABI v2 spelling of
/// [`tcl_codegen_local_get`]. Both names address the same indexed cell; the
/// `local_*` spellings stay for already-emitted modules.
///
/// # Safety
/// As [`tcl_codegen_local_get`].
#[no_mangle]
pub unsafe extern "C" fn tcl_codegen_slot_get(slot: i32) -> *mut TclObj {
    // SAFETY: forwarded per this function's contract.
    unsafe { tcl_codegen_local_get(slot) }
}

/// `tcl_codegen_slot_set(slot, value) -> status` — the ABI v2 spelling of
/// [`tcl_codegen_local_set`].
///
/// # Safety
/// As [`tcl_codegen_local_set`].
#[no_mangle]
pub unsafe extern "C" fn tcl_codegen_slot_set(slot: i32, value: *mut TclObj) -> i32 {
    // SAFETY: forwarded per this function's contract.
    unsafe { tcl_codegen_local_set(slot, value) }
}

/// `tcl_codegen_slot_bind(slot, name_ptr, name_len, value) -> status` — the ABI
/// v2 spelling of [`tcl_codegen_local_bind`].
///
/// # Safety
/// As [`tcl_codegen_local_bind`].
#[no_mangle]
pub unsafe extern "C" fn tcl_codegen_slot_bind(
    slot: i32,
    name_ptr: *const u8,
    name_len: i32,
    value: *mut TclObj,
) -> i32 {
    // SAFETY: forwarded per this function's contract.
    unsafe { tcl_codegen_local_bind(slot, name_ptr, name_len, value) }
}

/// Store a top-level or namespace variable by name.
///
/// # Safety
/// The name range must be readable, and `value` must be a live owned reference.
#[no_mangle]
pub unsafe extern "C" fn tcl_codegen_var_set(
    name_ptr: *const u8,
    name_len: i32,
    value: *mut TclObj,
) -> i32 {
    // SAFETY: a non-null input transfers exactly one generated +1 reference.
    let value = (!value.is_null()).then(|| unsafe { obj::Owned::from_raw(value) });
    let mut operation = match CodegenOperation::enter() {
        Ok(operation) => operation,
        Err(status) => {
            drop(value);
            return if status == TCL_INVOKE_ABI_HOST_REFUSED {
                status
            } else {
                1
            };
        }
    };
    let Some(value) = value else {
        return 1;
    };
    let name = unsafe { input_bytes(name_ptr, name_len) };
    let stored = operation.interpreter.var_set_named(name, value.as_ptr());
    let code = if operation.ensure_current().is_err() {
        crate::interp::Code::Error
    } else {
        match stored {
            Ok(()) => crate::interp::Code::Ok,
            Err(error) => crate::builtins::var_error(&mut operation.interpreter, name, error),
        }
    };
    drop(value);
    operation.finish_code(code)
}

/// Load a top-level or namespace variable by name as an owned stack value.
///
/// # Safety
/// The name range must be readable shared linear memory.
#[no_mangle]
pub unsafe extern "C" fn tcl_codegen_var_get(name_ptr: *const u8, name_len: i32) -> *mut TclObj {
    let Ok(mut operation) = CodegenOperation::enter() else {
        return ptr::null_mut();
    };
    let name = unsafe { input_bytes(name_ptr, name_len) };
    let read = operation.interpreter.read_named_variable(name);
    if operation.ensure_current().is_err() {
        return ptr::null_mut();
    }
    let Ok(value) = read else {
        return ptr::null_mut();
    };
    unsafe { obj::incr_ref_count(value) };
    value
}

/// Read one Tcl array element as an owned generated-word value.
///
/// This is the array-element half of the compiled word-evaluation surface:
/// [`tcl_codegen_var_get`] resolves a complete variable name, including a
/// literal element key in `${a(b)}`. This entry receives an already separated
/// root and evaluated key, as produced by `$name(key)`. It does not substitute
/// either supplied byte range again.
///
/// Fires `name`'s read traces first, exactly as the interpreted `$name(key)`
/// substitution does. A read-trace error, a missing array, or a missing
/// element sets the interpreter error and returns null; generated code treats
/// null as an aborting Tcl error for the enclosing command.
///
/// # Safety
/// Both ranges must be readable shared linear memory.
#[no_mangle]
pub unsafe extern "C" fn tcl_codegen_var_get_element(
    name_ptr: *const u8,
    name_len: i32,
    key_ptr: *const u8,
    key_len: i32,
) -> *mut TclObj {
    let Ok(mut operation) = CodegenOperation::enter() else {
        return ptr::null_mut();
    };
    let name = unsafe { input_bytes(name_ptr, name_len) };
    let key = unsafe { input_bytes(key_ptr, key_len) };
    let traced = operation.interpreter.fire_read_trace(name, Some(key));
    if operation.ensure_current().is_err() || traced.is_some() {
        return ptr::null_mut();
    }
    let value = operation.interpreter.var_get_elem(name, key);
    if operation.ensure_current().is_err() {
        return ptr::null_mut();
    }
    let Some(value) = value else {
        let message = operation.interpreter.read_miss_msg(name, Some(key));
        operation.interpreter.set_error(&message);
        let _ = operation.ensure_current();
        return ptr::null_mut();
    };
    unsafe { obj::incr_ref_count(value) };
    value
}
