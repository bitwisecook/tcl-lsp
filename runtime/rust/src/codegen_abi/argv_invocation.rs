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

//! The prebuilt-argv tier: guard tokens, guarded intrinsics and the general command dispatch.

use super::{
    detached_error_completion, original_frame_owner, write_detached_completion, AbiActivation,
    BorrowedArgv, CodegenOperation, TclCompletionAbi, TCL_INTRINSIC_ABI_DECLINED,
    TCL_INVOKE_ABI_HOST_REFUSED, TCL_INVOKE_ABI_INVALID_ARGC, TCL_INVOKE_ABI_NO_CURRENT_INTERP,
    TCL_INVOKE_ABI_NULL_ARGV, TCL_INVOKE_ABI_NULL_OUT, TCL_INVOKE_ABI_NULL_WORD, TCL_INVOKE_ABI_OK,
};
use crate::obj::TclObj;
use std::sync::OnceLock;
use tcl_dialect::model::SurfaceQuery;
use tcl_registry::{CommandRegistry, IntrinsicId, SemanticOperationId};
use tcl_runtime_api::guard::{GuardDomains, GuardIdentity, GuardToken};

#[derive(Debug)]
pub(super) struct ResolvedIntrinsicArgv {
    intrinsic: IntrinsicId,
    argument_offset: usize,
    head: Vec<u8>,
}

fn intrinsic_registry() -> &'static CommandRegistry {
    static REGISTRY: OnceLock<CommandRegistry> = OnceLock::new();
    REGISTRY.get_or_init(CommandRegistry::build_default)
}

/// Resolve the evaluated command head and form through the registry.
///
/// This examines only already-boxed argv values. It neither parses source nor
/// replays substitutions, and a non-text Tcl word is conservatively declined.
pub(super) fn resolve_intrinsic_argv(
    operation: &mut CodegenOperation,
    words: &[*mut TclObj],
    dialect: Option<SurfaceQuery<'_>>,
) -> Result<Option<ResolvedIntrinsicArgv>, i32> {
    let mut spellings = Vec::with_capacity(words.len());
    for &word in words {
        let bytes = operation.original_string(word)?;
        let Ok(spelling) = String::from_utf8(bytes.to_vec()) else {
            return Ok(None);
        };
        spellings.push(spelling);
    }
    let Some((head, arguments)) = spellings.split_first() else {
        return Ok(None);
    };
    let arguments: Vec<_> = arguments.iter().map(String::as_str).collect();
    let Some(resolved) = intrinsic_registry().resolve_invocation(head, &arguments, dialect) else {
        return Ok(None);
    };
    let SemanticOperationId::Intrinsic(intrinsic) = resolved.semantics.operation else {
        return Ok(None);
    };
    Ok(Some(ResolvedIntrinsicArgv {
        intrinsic,
        argument_offset: resolved.semantics.argument_offset,
        head: head.as_bytes().to_vec(),
    }))
}

fn guarded_intrinsic_request(
    operation: &mut CodegenOperation,
    intrinsic_id: u32,
    words: &[*mut TclObj],
    expected: GuardIdentity,
    characters: tcl_dialect::StringCharacterModel,
    dialect: Option<SurfaceQuery<'_>>,
) -> Result<Option<ResolvedIntrinsicArgv>, i32> {
    let Some(intrinsic) = IntrinsicId::from_stable_id(intrinsic_id) else {
        return Ok(None);
    };
    if expected
        != GuardIdentity::registry_intrinsic_with_semantics(
            intrinsic.stable_id(),
            intrinsic.guard_semantics_key_for_characters(characters),
        )
    {
        return Ok(None);
    }
    let Some(resolved) = resolve_intrinsic_argv(operation, words, dialect)? else {
        return Ok(None);
    };
    Ok((resolved.intrinsic == intrinsic).then_some(resolved))
}

fn guard_domains_from_abi(domains: i32) -> Option<GuardDomains> {
    u16::try_from(domains)
        .ok()
        .and_then(GuardDomains::from_bits)
}

/// `tcl_codegen_guard_prepare(intrinsic, argv, argc, namespace, value, domains)`.
///
/// The full, already-evaluated argv is re-resolved through the registry before
/// the current interpreter issues a token. Zero is a safe decline: the caller
/// must take its generic argv path and must not call `check` or `release`.
///
/// # Safety
/// `argv` must be null or point to `argc` readable object pointers. Non-null
/// words must be live and caller-owned for the duration of this call.
#[no_mangle]
pub unsafe extern "C" fn tcl_codegen_guard_prepare(
    intrinsic_id: u32,
    argv: *const *mut TclObj,
    argc: i32,
    identity_namespace: u32,
    identity_value: u64,
    domains: i32,
) -> u64 {
    let Some(argc) = usize::try_from(argc).ok().filter(|argc| *argc > 0) else {
        return 0;
    };
    if argv.is_null() {
        return 0;
    }
    // SAFETY: caller guarantees these readable pointer slots.
    let words = unsafe { core::slice::from_raw_parts(argv, argc) };
    if words.iter().any(|word| word.is_null()) {
        return 0;
    }
    let Some(domains) = guard_domains_from_abi(domains) else {
        return 0;
    };
    let Ok(mut operation) = CodegenOperation::enter() else {
        return 0;
    };
    // Retain borrowed words before the first original getter.
    let _borrowed = unsafe { BorrowedArgv::retain(words) };
    let native = operation.interpreter.native_invocation_dialect();
    let (Some(characters), Some(dialect)) = (native.characters, native.authoring_query()) else {
        return 0;
    };
    let expected = GuardIdentity::new(identity_namespace, identity_value);
    let resolved = match guarded_intrinsic_request(
        &mut operation,
        intrinsic_id,
        words,
        expected,
        characters,
        Some(dialect),
    ) {
        Ok(Some(resolved)) => resolved,
        Ok(None) | Err(_) => return 0,
    };
    let token = operation
        .interpreter
        .prepare_command_guard(&resolved.head, expected, domains);
    if operation.ensure_current().is_err() {
        if let Ok(token) = token {
            let _ = operation.interpreter.release_command_guard(token);
        }
        return 0;
    }
    token.map_or(0, GuardToken::raw)
}

/// `tcl_codegen_guard_check(token, intrinsic, argv, argc) -> i32`.
///
/// Re-resolves the same evaluated argv and verifies both its requested
/// intrinsic identity and every live guard domain. One means fast-path entry
/// is still safe; zero means use the generic argv path.
///
/// # Safety
/// `argv` must be null or point to `argc` readable object pointers. Non-null
/// words must be live and caller-owned for the duration of this call.
#[no_mangle]
pub unsafe extern "C" fn tcl_codegen_guard_check(
    token: u64,
    intrinsic_id: u32,
    argv: *const *mut TclObj,
    argc: i32,
) -> i32 {
    let Some(argc) = usize::try_from(argc).ok().filter(|argc| *argc > 0) else {
        return 0;
    };
    if argv.is_null() {
        return 0;
    }
    let words = unsafe { core::slice::from_raw_parts(argv, argc) };
    if words.iter().any(|word| word.is_null()) {
        return 0;
    }
    let Some(intrinsic) = IntrinsicId::from_stable_id(intrinsic_id) else {
        return 0;
    };
    let Ok(mut operation) = CodegenOperation::enter() else {
        return 0;
    };
    let _borrowed = unsafe { BorrowedArgv::retain(words) };
    let native = operation.interpreter.native_invocation_dialect();
    let (Some(characters), Some(dialect)) = (native.characters, native.authoring_query()) else {
        return 0;
    };
    let expected = GuardIdentity::registry_intrinsic_with_semantics(
        intrinsic.stable_id(),
        intrinsic.guard_semantics_key_for_characters(characters),
    );
    let resolved = match guarded_intrinsic_request(
        &mut operation,
        intrinsic_id,
        words,
        expected,
        characters,
        Some(dialect),
    ) {
        Ok(Some(resolved)) => resolved,
        Ok(None) | Err(_) => return 0,
    };
    let valid = operation.interpreter.check_command_guard_identity(
        GuardToken::from_raw(token),
        &resolved.head,
        expected,
    );
    if operation.ensure_current().is_err() {
        return 0;
    }
    i32::from(valid)
}

/// `tcl_codegen_guard_release(token)` — release a token exactly once.
#[no_mangle]
pub extern "C" fn tcl_codegen_guard_release(token: u64) {
    if let Some(interpreter) = original_frame_owner::cleanup_interpreter() {
        let _ = interpreter.release_command_guard(GuardToken::from_raw(token));
    }
}

/// `tcl_intrinsic_invoke_argv(intrinsic, argv, argc, out) -> status`.
///
/// This is the guarded fast operation over an already-evaluated argv. It
/// verifies the registry-selected form still maps to `intrinsic`, slices off
/// the registry-declared subcommand prefix, and calls the runtime intrinsic.
/// A [`TCL_INTRINSIC_ABI_DECLINED`] result writes no completion: generated code
/// must invoke the exact generic argv slow path with the original argv.
///
/// # Safety
/// `out` must be null or writable [`TclCompletionAbi`] storage. For positive
/// `argc`, `argv` must point to readable, non-null live Tcl-object pointers
/// held by the caller for this call.
#[no_mangle]
pub unsafe extern "C" fn tcl_intrinsic_invoke_argv(
    intrinsic_id: u32,
    argv: *const *mut TclObj,
    argc: i32,
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
                        b"no current interpreter for tcl_intrinsic_invoke_argv",
                    ),
                )
            };
            return TCL_INVOKE_ABI_NO_CURRENT_INTERP;
        }
        Err(status) => return status,
    };
    let argc = match usize::try_from(argc) {
        Ok(argc) if argc > 0 => argc,
        _ => return TCL_INTRINSIC_ABI_DECLINED,
    };
    if argv.is_null() {
        return TCL_INTRINSIC_ABI_DECLINED;
    }
    let words = unsafe { core::slice::from_raw_parts(argv, argc) };
    if words.iter().any(|word| word.is_null()) {
        return TCL_INTRINSIC_ABI_DECLINED;
    }
    let Some(intrinsic) = IntrinsicId::from_stable_id(intrinsic_id) else {
        return TCL_INTRINSIC_ABI_DECLINED;
    };
    let _borrowed = unsafe { BorrowedArgv::retain(words) };
    let Some(dialect) = operation
        .interpreter
        .native_invocation_dialect()
        .authoring_query()
    else {
        return TCL_INTRINSIC_ABI_DECLINED;
    };
    let resolved = match resolve_intrinsic_argv(&mut operation, words, Some(dialect)) {
        Ok(Some(resolved)) => resolved,
        Ok(None) => return TCL_INTRINSIC_ABI_DECLINED,
        Err(status) => return status,
    };
    if resolved.intrinsic != intrinsic {
        return TCL_INTRINSIC_ABI_DECLINED;
    }
    let Some(args_start) = resolved.argument_offset.checked_add(1) else {
        return TCL_INTRINSIC_ABI_DECLINED;
    };
    let Some(args) = words.get(args_start..) else {
        return TCL_INTRINSIC_ABI_DECLINED;
    };
    let activation = unsafe { AbiActivation::enter(&mut operation.interpreter) };
    if activation.is_none() {
        return unsafe {
            operation.settle(crate::interp::Code::Error, None, out, TCL_INVOKE_ABI_OK)
        };
    }
    let code = operation.interpreter.execute_intrinsic(intrinsic, args);
    if operation.ensure_current().is_err() {
        drop(activation);
        return TCL_INVOKE_ABI_HOST_REFUSED;
    }
    let Some(code) = code else {
        drop(activation);
        return operation
            .ensure_current()
            .map_or_else(|status| status, |_| TCL_INTRINSIC_ABI_DECLINED);
    };
    unsafe { operation.settle(code, activation, out, TCL_INVOKE_ABI_OK) }
}

/// `tcl_invoke_argv(argv, argc, out) -> status` — invoke an already-evaluated
/// Tcl argv against the current interpreter.
///
/// `argv` contains the full command vector, including its command head at
/// index zero. The runtime performs its normal name resolution and dispatch —
/// namespaces, `unknown`, aliases, ensembles, and TclOO all stay on the same
/// [`Interp::dispatch`] path as interpreted Tcl — but does not parse source or
/// repeat substitutions. `out` receives the shared
/// [`tcl_runtime_api::Completion`] as [`TclCompletionAbi`].
///
/// The return value reports ABI handling only: [`TCL_INVOKE_ABI_OK`] means
/// `out` was written, even when `out.code` is Tcl `error`; a negative status
/// reports a malformed boundary request or an operational Host refusal.
/// Host refusal leaves `out` untouched. Other boundary failures with valid
/// output storage write an owned error completion for the caller's release path.
///
/// ## Ownership
///
/// `argv` is borrowed. The caller retains ownership of every word and must keep
/// each non-null object live with one owned reference for the whole call. The
/// runtime takes transient references before dispatch and releases them before
/// return; it never adopts or releases the caller's references. The written
/// `result` and `options` each transfer one owned reference to the caller;
/// release them individually through [`tcl_obj_release`] or together through
/// [`tcl_completion_release`], but never both.
///
/// # Safety
/// `out` must be null or point to writable, properly aligned
/// [`TclCompletionAbi`] storage. For a positive `argc`, `argv` must point to
/// `argc` readable pointers, each non-null and a live `TclObj` with a
/// caller-owned reference. Null, negative, and non-representable sizes are
/// rejected without reading argv memory.
#[no_mangle]
pub unsafe extern "C" fn tcl_invoke_argv(
    argv: *const *mut TclObj,
    argc: i32,
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
                    detached_error_completion(b"no current interpreter for tcl_invoke_argv"),
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
                .set_error(b"tcl_invoke_argv requires a command head");
            return unsafe { operation.settle(code, None, out, TCL_INVOKE_ABI_INVALID_ARGC) };
        }
    };
    if argv.is_null() {
        let code = operation
            .interpreter
            .set_error(b"tcl_invoke_argv received a null argv");
        return unsafe { operation.settle(code, None, out, TCL_INVOKE_ABI_NULL_ARGV) };
    }
    let words = unsafe { core::slice::from_raw_parts(argv, argc) };
    if words.iter().any(|word| word.is_null()) {
        let code = operation
            .interpreter
            .set_error(b"tcl_invoke_argv received a null word");
        return unsafe { operation.settle(code, None, out, TCL_INVOKE_ABI_NULL_WORD) };
    }
    // All caller-owned argv references remain borrowed through dispatch and capture.
    let _borrowed = unsafe { BorrowedArgv::retain(words) };
    let activation = unsafe {
        AbiActivation::enter_labeled(&mut operation.interpreter, || _borrowed.cached_depth_head())
    };
    if activation.is_none() {
        return unsafe {
            operation.settle(crate::interp::Code::Error, None, out, TCL_INVOKE_ABI_OK)
        };
    }
    let code = operation.interpreter.dispatch(words);
    unsafe { operation.settle(code, activation, out, TCL_INVOKE_ABI_OK) }
}
