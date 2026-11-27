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

//! Boxing strings and scalars, reading typed scalars back, joining word parts, and the retain/release pair.

use super::{input_bytes, BorrowedArgv, CodegenOperation, TCL_VALUE_GET_ERROR, TCL_VALUE_GET_OK};
use crate::interp::Interp;
use crate::obj;
use crate::obj::{new_string_bytes, TclObj};
use core::ptr;

/// `tcl_obj_new_string(ptr, len) -> obj` — box `len` bytes of (shared linear)
/// memory as a fresh `TclObj` (`rc 0`). The consumer ([`tcl_eval`] /
/// [`tcl_expr_bool`]) adopts and frees it.
///
/// # Safety
/// `ptr` must reference `len` readable bytes (it may be null only when
/// `len == 0`); `len` must be non-negative.
#[no_mangle]
pub unsafe extern "C" fn tcl_obj_new_string(ptr: *const u8, len: i32) -> *mut TclObj {
    if ptr.is_null() || len <= 0 {
        return new_string_bytes(b"");
    }
    // SAFETY: forwarded per this fn's contract.
    let slice = unsafe { core::slice::from_raw_parts(ptr, len as usize) };
    new_string_bytes(slice)
}

/// Construct an owned Tcl value for the generated operand stack.
///
/// # Safety
/// `ptr..ptr+len` must be readable shared linear memory when `len` is positive.
#[no_mangle]
pub unsafe extern "C" fn tcl_value_new_string(ptr: *const u8, len: i32) -> *mut TclObj {
    let value = new_string_bytes(unsafe { input_bytes(ptr, len) });
    // SAFETY: a generated operand-stack value owns one reference.
    unsafe { obj::incr_ref_count(value) };
    value
}

/// `tcl_value_new_wide_int(value) -> obj` — materialise a native i64 at a Tcl
/// value boundary with one generated-code-owned (`+1`) reference.
///
/// This deliberately reuses [`crate::capi::Tcl_NewWideIntObj`] for the Tcl
/// integer's lazy dual representation (wide internal form, string form on
/// demand), then adds the generated operand stack's owning reference. It is
/// not a command operation: callers may use the result wherever an owned Tcl
/// object is required, and must balance it with [`tcl_value_release`] or
/// [`tcl_obj_release`].
#[no_mangle]
pub extern "C" fn tcl_value_new_wide_int(value: i64) -> *mut TclObj {
    let object = crate::capi::Tcl_NewWideIntObj(value);
    // SAFETY: the C ABI constructor returned a fresh live object (or null on
    // allocation failure, which `incr_ref_count` accepts defensively).
    unsafe { obj::incr_ref_count(object) };
    object
}

/// `tcl_value_new_double(value) -> obj` — materialise a native `f64` at a Tcl
/// value boundary with one generated-code-owned (`+1`) reference.
///
/// The double half of [`tcl_value_new_wide_int`]: the object begins with the
/// `TCL_DOUBLE_TYPE` internal rep and no string rep, so a value that never
/// crosses a string boundary never pays for one.
#[no_mangle]
pub extern "C" fn tcl_value_new_double(value: f64) -> *mut TclObj {
    let object = obj::new_double_obj(value);
    // SAFETY: a freshly constructed live object (or null on allocation
    // failure, which `incr_ref_count` accepts defensively).
    unsafe { obj::incr_ref_count(object) };
    object
}

/// `tcl_value_new_bool(value) -> obj` — materialise a native boolean as the
/// Tcl integer `0`/`1`, with one generated-code-owned (`+1`) reference.
///
/// Tcl booleans *are* integer objects (C's `Tcl_NewBooleanObj`), so this is
/// [`tcl_value_new_wide_int`] with C's normalisation of any non-zero input to
/// `1` — the spelling `expr` and `string is boolean` both produce.
#[no_mangle]
pub extern "C" fn tcl_value_new_bool(value: i32) -> *mut TclObj {
    let object = obj::new_boolean_obj(value);
    // SAFETY: a freshly constructed live object.
    unsafe { obj::incr_ref_count(object) };
    object
}

/// `tcl_value_get_wide_int(value, out) -> status` — read a boxed Tcl value as a
/// native `i64` (`Tcl_GetWideIntFromObj`).
///
/// [`TCL_VALUE_GET_OK`] means `*out` was written. [`TCL_VALUE_GET_ERROR`] means
/// the current interpreter carries a Tcl error — C's exact message and
/// `-errorcode` (`expected integer but got …` / `TCL VALUE NUMBER`, a
/// double-typed object's `TCL VALUE INTEGER`, or `integer value too large to
/// represent` / `ARITH IOVERFLOW`) — and `*out` is untouched, so generated code
/// treats it as an aborting error for the enclosing command.
///
/// A successful read **caches the parsed rep onto `value`**, so reading the same
/// object in a loop parses its spelling once. The string rep is kept: `"0x10"`
/// reads as 16 and still prints `0x10`.
///
/// `value` is borrowed; the caller keeps its reference.
///
/// # Safety
/// `value` must be a live object with a caller-owned reference, and `out` must
/// be writable, properly aligned `i64` storage.
#[no_mangle]
pub unsafe extern "C" fn tcl_value_get_wide_int(value: *mut TclObj, out: *mut i64) -> i32 {
    let Ok(mut operation) = CodegenOperation::enter() else {
        return TCL_VALUE_GET_ERROR;
    };
    if value.is_null() || out.is_null() {
        return TCL_VALUE_GET_ERROR;
    }
    match operation.scalar(
        value,
        tcl_syntax::scalar_getter::NativeScalarGetterKind::Wide,
    ) {
        Ok(tcl_syntax::scalar_getter::NativeScalarGetterValue::Wide(parsed)) => {
            // SAFETY: the caller provides aligned writable output storage.
            unsafe { out.write(parsed) };
            TCL_VALUE_GET_OK
        }
        Ok(_) => unexpected_scalar_output(&mut operation.interpreter),
        Err(_) => TCL_VALUE_GET_ERROR,
    }
}

/// `tcl_value_get_double(value, out) -> status` — read a boxed Tcl value as a
/// native `f64` (`Tcl_GetDoubleFromObj`), with the same status contract and the
/// same original-object conversion contract as [`tcl_value_get_wide_int`].
/// Acceptance and cache changes follow the selected actual engine, including
/// its release-specific NaN and overflow behaviour.
///
/// # Safety
/// `value` must be a live object with a caller-owned reference, and `out` must
/// be writable, properly aligned `f64` storage.
#[no_mangle]
pub unsafe extern "C" fn tcl_value_get_double(value: *mut TclObj, out: *mut f64) -> i32 {
    let Ok(mut operation) = CodegenOperation::enter() else {
        return TCL_VALUE_GET_ERROR;
    };
    if value.is_null() || out.is_null() {
        return TCL_VALUE_GET_ERROR;
    }
    match operation.scalar(
        value,
        tcl_syntax::scalar_getter::NativeScalarGetterKind::Double,
    ) {
        Ok(tcl_syntax::scalar_getter::NativeScalarGetterValue::Double(parsed)) => {
            // SAFETY: the caller provides aligned writable output storage.
            unsafe { out.write(parsed) };
            TCL_VALUE_GET_OK
        }
        Ok(_) => unexpected_scalar_output(&mut operation.interpreter),
        Err(_) => TCL_VALUE_GET_ERROR,
    }
}

/// Read the selected public primitive Boolean getter's truth projection.
/// Expression instructions use `tcl_value_get_bool_for_purpose`; evaluated
/// expression results use `tcl_value_get_expression_bool`, which performs the
/// genuine result producer before conversion. Jim's primitive integer output
/// has an independent public C API and cannot stand in for expression truth.
///
/// # Safety
/// `value` must be a live object with a caller-owned reference, and `out` must
/// be writable, properly aligned `i32` storage.
#[no_mangle]
pub unsafe extern "C" fn tcl_value_get_bool(value: *mut TclObj, out: *mut i32) -> i32 {
    let Ok(mut operation) = CodegenOperation::enter() else {
        return TCL_VALUE_GET_ERROR;
    };
    if value.is_null() || out.is_null() {
        return TCL_VALUE_GET_ERROR;
    }
    match operation.scalar(
        value,
        tcl_syntax::scalar_getter::NativeScalarGetterKind::Boolean,
    ) {
        Ok(tcl_syntax::scalar_getter::NativeScalarGetterValue::Boolean(parsed)) => {
            // SAFETY: the caller provides aligned writable output storage.
            unsafe { out.write(i32::from(parsed.is_true())) };
            TCL_VALUE_GET_OK
        }
        Ok(_) => unexpected_scalar_output(&mut operation.interpreter),
        Err(_) => TCL_VALUE_GET_ERROR,
    }
}

pub(super) fn unexpected_scalar_output(interpreter: &mut Interp) -> i32 {
    interpreter.refuse_native_access(
        tcl_syntax::raw_string::NativeValueAccessRefusal::ScalarNumericInputUnavailable,
    );
    TCL_VALUE_GET_ERROR
}

/// Convert the original value at a reached expression operand instruction.
/// Tags4/5 require a genuine expression result producer and are refused here.
/// Unknown purpose tags refuse before getters and never select a default.
/// # Safety
/// `value` carries a live caller-owned reference; `out` is writable aligned i32.
#[no_mangle]
pub unsafe extern "C" fn tcl_value_get_bool_for_purpose(
    value: *mut TclObj,
    purpose_tag: i32,
    out: *mut i32,
) -> i32 {
    let Ok(mut operation) = CodegenOperation::enter() else {
        return TCL_VALUE_GET_ERROR;
    };
    if value.is_null() || out.is_null() {
        return TCL_VALUE_GET_ERROR;
    }
    let result =
        tcl_native_model::native_boolean_truth::NativeBooleanTruthPurpose::from_abi(purpose_tag)
            .ok_or_else(|| {
                tcl_cmd_core::CmdError::from(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "Boolean operand purpose tag",
                    ),
                )
            })
            .and_then(|purpose| {
                crate::typed_value::native_boolean_for_interp(
                    &mut operation.interpreter,
                    value,
                    purpose,
                )
            });
    if operation.ensure_current().is_err() {
        return TCL_VALUE_GET_ERROR;
    }
    let status = finish_original_boolean(&mut operation.interpreter, result, out);
    if operation.ensure_current().is_err() {
        TCL_VALUE_GET_ERROR
    } else {
        status
    }
}

/// Perform the selected outer Boolean expression result producer, then convert
/// its actual owned result. Tags0/1 select InlineExpression/PublicExpressionApi;
/// a public tag never attests that another caller already normalised an object.
/// # Safety
/// `value` carries a live caller-owned reference; `out` is writable aligned i32.
#[no_mangle]
pub unsafe extern "C" fn tcl_value_get_expression_bool(
    value: *mut TclObj,
    production_tag: i32,
    out: *mut i32,
) -> i32 {
    let Ok(mut operation) = CodegenOperation::enter() else {
        return TCL_VALUE_GET_ERROR;
    };
    if value.is_null() || out.is_null() {
        return TCL_VALUE_GET_ERROR;
    }
    let result =
        tcl_native_model::native_boolean_truth::NativeBooleanExpressionResultProduction::from_abi(
            production_tag,
        )
        .ok_or_else(|| {
            tcl_cmd_core::CmdError::from(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "Boolean expression producer tag",
            ))
        })
        .and_then(|purpose| {
            crate::typed_value::expression_boolean_for_interp(
                &mut operation.interpreter,
                value,
                purpose,
            )
        });
    if operation.ensure_current().is_err() {
        return TCL_VALUE_GET_ERROR;
    }
    let status = finish_original_boolean(&mut operation.interpreter, result, out);
    if operation.ensure_current().is_err() {
        TCL_VALUE_GET_ERROR
    } else {
        status
    }
}

fn finish_original_boolean(
    interp: &mut Interp,
    result: Result<bool, tcl_cmd_core::CmdError>,
    out: *mut i32,
) -> i32 {
    if interp.host_refusal_pending() {
        return TCL_VALUE_GET_ERROR;
    }
    match result {
        Ok(truth) => {
            // SAFETY: each ABI caller has already validated writable aligned out.
            unsafe { out.write(i32::from(truth)) };
            TCL_VALUE_GET_OK
        }
        Err(error) => {
            interp.report_cmd_error(error);
            TCL_VALUE_GET_ERROR
        }
    }
}

/// Release one generated operand-stack value.
///
/// # Safety
/// `value` must carry one live reference owned by generated code.
#[no_mangle]
pub unsafe extern "C" fn tcl_value_release(value: *mut TclObj) {
    // SAFETY: generated code transfers its owned stack reference here.
    unsafe { obj::decr_ref_count(value) };
}

/// Join `count` evaluated word parts into one owned Tcl value.
///
/// This is the runtime half of compiled compound-word evaluation: a quoted or
/// concatenated Tcl word evaluates each part and joins their string
/// representations, so the emitter hands over the already-evaluated parts
/// rather than performing string work itself.
///
/// `parts` is **borrowed**: the caller keeps its owned reference to each part
/// and releases them on its own cleanup path. The returned value carries one
/// caller-owned reference. A null part yields null so generated code can treat
/// it as an aborting error; a zero `count` yields an owned empty value.
///
/// # Safety
/// For a positive `count`, `parts` must point to `count` readable pointers,
/// each null or a live `TclObj` the caller keeps alive for the call.
#[no_mangle]
pub unsafe extern "C" fn tcl_codegen_word_concat(
    parts: *const *mut TclObj,
    count: i32,
) -> *mut TclObj {
    let Ok(count) = usize::try_from(count) else {
        return ptr::null_mut();
    };
    if count > 0 && parts.is_null() {
        return ptr::null_mut();
    }
    let words = if count == 0 {
        &[][..]
    } else {
        // SAFETY: the caller retains count readable pointer slots.
        unsafe { core::slice::from_raw_parts(parts, count) }
    };
    if words.iter().any(|word| word.is_null()) {
        return ptr::null_mut();
    }
    let Ok(mut operation) = CodegenOperation::enter() else {
        return ptr::null_mut();
    };
    // SAFETY: each word is live and caller-owned; this temporary borrow is balanced.
    let borrowed = unsafe { BorrowedArgv::retain(words) };
    let mut joined = Vec::new();
    for &word in words {
        let bytes = match operation.original_string(word) {
            Ok(bytes) => bytes,
            Err(_) => return ptr::null_mut(),
        };
        joined.extend_from_slice(&bytes);
    }
    drop(borrowed);
    if operation.ensure_current().is_err() {
        return ptr::null_mut();
    }
    obj::Owned::fresh(new_string_bytes(&joined)).into_raw()
}

/// `tcl_obj_new_string_owned(ptr, len) -> obj` — copy a string from shared
/// linear memory and return one caller-owned (`+1`) reference.
///
/// This is the argv constructor for generated generic invocation code. The
/// caller may free or reuse its source call frame immediately after dispatch
/// only after it releases this returned reference. Unlike [`tcl_obj_new_string`]
/// it is not adopted by [`tcl_invoke_argv`]: that ABI borrows argv words.
///
/// # Safety
/// `ptr` must reference `len` readable bytes (it may be null only when
/// `len == 0`); `len` must be non-negative.
#[no_mangle]
pub unsafe extern "C" fn tcl_obj_new_string_owned(ptr: *const u8, len: i32) -> *mut TclObj {
    // SAFETY: forwarded per this function's contract.
    let object = unsafe { tcl_obj_new_string(ptr, len) };
    // SAFETY: the constructor returned a live fresh object; this creates the
    // caller-owned reference the borrowed argv ABI requires.
    unsafe { obj::incr_ref_count(object) };
    object
}

/// `tcl_obj_release(obj)` — release one owned reference (the result of
/// [`tcl_eval`]). Frees at `rc 0`. Null-safe.
///
/// # Safety
/// `obj` must be null or an object the caller holds an owned reference to.
#[no_mangle]
pub unsafe extern "C" fn tcl_obj_release(obj: *mut TclObj) {
    // SAFETY: forwarded per contract.
    unsafe { obj::decr_ref_count(obj) };
}

/// `tcl_obj_retain(obj) -> obj` — duplicate one owned reference.
///
/// Generated functions use this to forward a completion result or options
/// handle while still releasing their private [`TclCompletionAbi`] storage.
/// The returned handle is the same object and carries one additional `+1`
/// reference which the receiving generated caller (or host) must release.
/// Null is accepted and returned unchanged for defensive ABI composition.
///
/// # Safety
/// `obj` must be null or a live object.
#[no_mangle]
pub unsafe extern "C" fn tcl_obj_retain(obj: *mut TclObj) -> *mut TclObj {
    // SAFETY: forwarded per this function's contract.
    unsafe { obj::incr_ref_count(obj) };
    obj
}
