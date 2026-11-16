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

//! Registering and defining procedures from emitted code, with or without a compiled body.

use super::{
    detached_error_completion, input_bytes, original_frame_owner, write_detached_completion,
    AbiActivation, BorrowedArgv, CodegenOperation, TclCompletionAbi, TCL_INVOKE_ABI_HOST_REFUSED,
    TCL_INVOKE_ABI_INVALID_ARGC, TCL_INVOKE_ABI_NO_CURRENT_INTERP, TCL_INVOKE_ABI_NULL_ARGV,
    TCL_INVOKE_ABI_NULL_OUT, TCL_INVOKE_ABI_NULL_WORD, TCL_INVOKE_ABI_OK,
};
use crate::interp::NativeProcEntry;
use crate::obj;
use crate::obj::{new_string_bytes, TclObj};
use tcl_runtime_api::guard::GuardIdentity;

/// Register source metadata for a generated procedure without evaluating `proc`.
///
/// Exactly [`tcl_codegen_proc_define_native`] with no entry. It stays because
/// already-emitted legacy-tier modules import this spelling.
///
/// # Safety
/// All three pointer/length ranges must be readable shared linear memory.
#[no_mangle]
pub unsafe extern "C" fn tcl_codegen_proc_register(
    name_ptr: *const u8,
    name_len: i32,
    params_ptr: *const u8,
    params_len: i32,
    body_ptr: *const u8,
    body_len: i32,
) -> i32 {
    // SAFETY: the pointer/length ranges are forwarded under this function's
    // own contract, which is the callee's.
    unsafe {
        tcl_codegen_proc_define_native(
            name_ptr, name_len, params_ptr, params_len, body_ptr, body_len, None,
        )
    }
}

/// Define a Tcl procedure, optionally binding a compiled body to it.
///
/// `entry` is the compiled body: a wasm32 function-table index on the emitted
/// side, a nullable function pointer here — which is why the runtime types it
/// as `Option<NativeProcEntry>`, so `0` is `None` on wasm and a native test
/// can pass an ordinary Rust `extern "C"` function. A `None` entry defines an
/// ordinary source-body proc and is exactly [`tcl_codegen_proc_register`].
///
/// The `body` source is always recorded, entry or not: it is what a step trace
/// forces, what a declining entry falls back to, and what `info body` reports.
///
/// The entry binds to *this* definition. Anything that replaces the definition
/// — re-running `proc`, `rename p ""`, `namespace delete` — drops it with the
/// definition, so there is nothing to invalidate.
///
/// # Safety
/// All three pointer/length ranges must be readable shared linear memory, and
/// `entry`, when present, must satisfy [`NativeProcEntry`]'s whole contract.
#[no_mangle]
pub unsafe extern "C" fn tcl_codegen_proc_define_native(
    name_ptr: *const u8,
    name_len: i32,
    params_ptr: *const u8,
    params_len: i32,
    body_ptr: *const u8,
    body_len: i32,
    entry: Option<NativeProcEntry>,
) -> i32 {
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
    let name = unsafe { input_bytes(name_ptr, name_len) };
    let parameters = unsafe { input_bytes(params_ptr, params_len) };
    let body = unsafe { input_bytes(body_ptr, body_len) };
    let original_parameters = obj::Owned::fresh(new_string_bytes(parameters));
    let parsed = crate::cmd_proc::parse_params_object(
        &mut operation.interpreter,
        original_parameters.as_ptr(),
        name,
    );
    if operation.ensure_current().is_err() {
        drop(original_parameters);
        return operation.finish_code(crate::interp::Code::Error);
    }
    let parameters = match parsed {
        Ok(parameters) => parameters,
        Err(error) => {
            let code = operation.interpreter.report_cmd_error(error);
            drop(original_parameters);
            return operation.finish_code(code);
        }
    };
    let body = obj::Owned::fresh(new_string_bytes(body));
    let installed = operation.interpreter.install_proc_original_storage(
        name,
        parameters,
        Some(original_parameters.as_ptr()),
        body.as_ptr(),
        entry,
        None,
    );
    drop(body);
    drop(original_parameters);
    if operation.ensure_current().is_err() {
        return TCL_INVOKE_ABI_HOST_REFUSED;
    }
    if installed.is_none() {
        return 1;
    }
    operation.interpreter.set_result_bytes(b"");
    operation.finish_code(crate::interp::Code::Ok)
}

/// The number of proc bodies the current interpreter has run through a native
/// entry rather than their source body; `-1` with no current interpreter.
///
/// A diagnostic/test boundary, like [`tcl_codegen_call_frame_outstanding`].
/// The compiled and interpreted bodies of one proc produce the same Tcl result
/// by construction, so without this counter no test can tell which one ran,
/// and a binding that silently stopped taking effect would keep every
/// behavioural assertion green.
#[no_mangle]
pub extern "C" fn tcl_codegen_native_proc_dispatches() -> i32 {
    let Ok(Some(interpreter)) = original_frame_owner::interpreter() else {
        return -1;
    };
    let dispatches = interpreter.native_proc_dispatches();
    i32::try_from(dispatches).unwrap_or(i32::MAX)
}

/// Define from borrowed evaluated argv through the original procedure handler.
/// The caller retains every argument and supplies a checked compiled entry.
///
/// # Safety
/// `argv` must hold `argc` live caller-owned objects and `out` must be writable
/// completion storage. The caller must independently prove the live stock
/// procedure definer in the current lookup context; this entry bypasses head
/// dispatch. A present entry must separately satisfy `NativeProcEntry`.
#[no_mangle]
pub unsafe extern "C" fn tcl_codegen_proc_define_original_argv(
    argv: *const *mut TclObj,
    argc: i32,
    entry: Option<NativeProcEntry>,
    out: *mut TclCompletionAbi,
) -> i32 {
    if out.is_null() {
        return TCL_INVOKE_ABI_NULL_OUT;
    }
    let mut operation = match CodegenOperation::enter() {
        Ok(operation) => operation,
        Err(TCL_INVOKE_ABI_NO_CURRENT_INTERP) => {
            unsafe {
                write_detached_completion(
                    out,
                    detached_error_completion(
                        b"no current interpreter for tcl_codegen_proc_define_original_argv",
                    ),
                )
            };
            return TCL_INVOKE_ABI_NO_CURRENT_INTERP;
        }
        Err(status) => return status,
    };
    let argc = match usize::try_from(argc) {
        Ok(argc) if argc > 0 => argc,
        _ => {
            let code = operation
                .interpreter
                .set_error(b"tcl_codegen_proc_define_original_argv requires a command head");
            return unsafe { operation.settle(code, None, out, TCL_INVOKE_ABI_INVALID_ARGC) };
        }
    };
    if argv.is_null() {
        let code = operation
            .interpreter
            .set_error(b"tcl_codegen_proc_define_original_argv received a null argv");
        return unsafe { operation.settle(code, None, out, TCL_INVOKE_ABI_NULL_ARGV) };
    }
    let words = unsafe { core::slice::from_raw_parts(argv, argc) };
    if words.iter().any(|word| word.is_null()) {
        let code = operation
            .interpreter
            .set_error(b"tcl_codegen_proc_define_original_argv received a null word");
        return unsafe { operation.settle(code, None, out, TCL_INVOKE_ABI_NULL_WORD) };
    }
    // All caller-owned argv references remain borrowed through dispatch and capture.
    let _borrowed = unsafe { BorrowedArgv::retain(words) };
    let activation = unsafe { AbiActivation::enter(&mut operation.interpreter) };
    if activation.is_none() {
        return unsafe {
            operation.settle(crate::interp::Code::Error, None, out, TCL_INVOKE_ABI_OK)
        };
    }
    let code =
        crate::cmd_proc::proc_cmd_with_native_entry(&mut operation.interpreter, words, entry);
    unsafe { operation.settle(code, activation, out, TCL_INVOKE_ABI_OK) }
}

/// Dispatch borrowed original argv, admitting a compiled body only at the
/// actual selected stock definition handler. Every miss takes ordinary dispatch.
///
/// # Safety
/// `argv` holds `argc` live borrowed objects and `out` is writable. A supplied
/// entry must implement exactly the written body's formals/source contract,
/// independently of whether the runtime admits this definition candidate.
#[no_mangle]
pub unsafe extern "C" fn tcl_codegen_proc_define_guarded_argv(
    argv: *const *mut TclObj,
    argc: i32,
    entry: Option<NativeProcEntry>,
    expected_value: u64,
    out: *mut TclCompletionAbi,
) -> i32 {
    if out.is_null() {
        return TCL_INVOKE_ABI_NULL_OUT;
    }
    let mut operation = match CodegenOperation::enter() {
        Ok(operation) => operation,
        Err(TCL_INVOKE_ABI_NO_CURRENT_INTERP) => {
            unsafe {
                write_detached_completion(
                    out,
                    detached_error_completion(b"no current interpreter for guarded definition"),
                )
            };
            return TCL_INVOKE_ABI_NO_CURRENT_INTERP;
        }
        Err(status) => return status,
    };
    let Some(argc) = usize::try_from(argc).ok().filter(|argc| *argc > 0) else {
        let code = operation
            .interpreter
            .set_error(b"guarded definition requires a command head");
        return unsafe { operation.settle(code, None, out, TCL_INVOKE_ABI_INVALID_ARGC) };
    };
    if argv.is_null() {
        let code = operation
            .interpreter
            .set_error(b"guarded definition received a null argv");
        return unsafe { operation.settle(code, None, out, TCL_INVOKE_ABI_NULL_ARGV) };
    }
    // SAFETY: the caller supplies these borrowed live pointer slots.
    let words = unsafe { core::slice::from_raw_parts(argv, argc) };
    if words.iter().any(|word| word.is_null()) {
        let code = operation
            .interpreter
            .set_error(b"guarded definition received a null word");
        return unsafe { operation.settle(code, None, out, TCL_INVOKE_ABI_NULL_WORD) };
    }
    let _borrowed = unsafe { BorrowedArgv::retain(words) };
    let activation = unsafe { AbiActivation::enter(&mut operation.interpreter) };
    if activation.is_none() {
        return unsafe {
            operation.settle(crate::interp::Code::Error, None, out, TCL_INVOKE_ABI_OK)
        };
    }
    let expected = GuardIdentity::new(
        tcl_runtime_api::guard::REGISTRY_DEFINITION_IDENTITY_NAMESPACE,
        expected_value,
    );
    let code = operation
        .interpreter
        .dispatch_guarded_definition(words, entry, expected);
    unsafe { operation.settle(code, activation, out, TCL_INVOKE_ABI_OK) }
}

/// Publish the ordinary original procedure with an inert future body candidate.
/// Actual entered original compiler instructions and frame owners select any
/// checked body later; the supplied byte ranges grant no execution authority.
///
/// # Safety
/// `argv` holds `argc` live borrowed objects, `out` is writable, and each
/// nonempty byte range is valid unchanged storage for its counted length.
#[no_mangle]
pub unsafe extern "C" fn tcl_codegen_proc_define_guarded_i64_add_argv(
    argv: *const *mut TclObj,
    argc: i32,
    expected_value: u64,
    parameters: *const u8,
    parameters_len: i32,
    body: *const u8,
    body_len: i32,
    out: *mut TclCompletionAbi,
) -> i32 {
    if out.is_null() {
        return TCL_INVOKE_ABI_NULL_OUT;
    }
    let mut operation = match CodegenOperation::enter() {
        Ok(operation) => operation,
        Err(TCL_INVOKE_ABI_NO_CURRENT_INTERP) => {
            unsafe {
                write_detached_completion(
                    out,
                    detached_error_completion(b"no current interpreter for guarded definition"),
                )
            };
            return TCL_INVOKE_ABI_NO_CURRENT_INTERP;
        }
        Err(status) => return status,
    };
    let Some(argc) = usize::try_from(argc).ok().filter(|argc| *argc > 0) else {
        let code = operation
            .interpreter
            .set_error(b"guarded definition requires a command head");
        return unsafe { operation.settle(code, None, out, TCL_INVOKE_ABI_INVALID_ARGC) };
    };
    if argv.is_null() {
        let code = operation
            .interpreter
            .set_error(b"guarded definition received a null argv");
        return unsafe { operation.settle(code, None, out, TCL_INVOKE_ABI_NULL_ARGV) };
    }
    // SAFETY: the caller supplies these borrowed live pointer slots.
    let words = unsafe { core::slice::from_raw_parts(argv, argc) };
    if words.iter().any(|word| word.is_null()) {
        let code = operation
            .interpreter
            .set_error(b"guarded definition received a null word");
        return unsafe { operation.settle(code, None, out, TCL_INVOKE_ABI_NULL_WORD) };
    }
    let _borrowed = unsafe { BorrowedArgv::retain(words) };
    let activation = unsafe { AbiActivation::enter(&mut operation.interpreter) };
    if activation.is_none() {
        return unsafe {
            operation.settle(crate::interp::Code::Error, None, out, TCL_INVOKE_ABI_OK)
        };
    }
    let expected = GuardIdentity::new(
        tcl_runtime_api::guard::REGISTRY_DEFINITION_IDENTITY_NAMESPACE,
        expected_value,
    );
    let lengths = usize::try_from(parameters_len)
        .ok()
        .zip(usize::try_from(body_len).ok());
    let candidate = lengths
        .filter(|(parameters_len, body_len)| {
            *parameters_len <= 128
                && *body_len <= 4096
                && (*parameters_len == 0 || !parameters.is_null())
                && (*body_len == 0 || !body.is_null())
        })
        .map(|(parameters_len, body_len)| {
            // SAFETY: the generated module borrows these unchanged data ranges.
            let parameters = if parameters_len == 0 {
                &[][..]
            } else {
                unsafe { core::slice::from_raw_parts(parameters, parameters_len) }
            };
            let body = if body_len == 0 {
                &[][..]
            } else {
                unsafe { core::slice::from_raw_parts(body, body_len) }
            };
            crate::interp::native_i64_body::FutureNativeI64AddPlan::new(parameters, body)
        });
    let code = match candidate {
        Some(plan) => operation
            .interpreter
            .dispatch_guarded_i64_add_definition(words, plan, expected),
        None => operation.interpreter.dispatch(words),
    };
    unsafe { operation.settle(code, activation, out, TCL_INVOKE_ABI_OK) }
}
