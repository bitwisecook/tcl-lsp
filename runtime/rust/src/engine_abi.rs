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

//! The C ABI through which a host that runs this runtime compiled to `wasm32`
//! drives it as an engine (`rust/tcl-engine-wasm`). It offers what
//! `crate::engine` does natively, for a host on the far side of the module
//! boundary that reaches the interpreter only by exports: the limits, the
//! confinement, the whitelist, the pinned release, a unit's procedure, a
//! provided package, and a host command's `return` and error.
//!
//! Every object argument is borrowed; an object answered is owned by the
//! caller.

use core::ffi::{c_char, c_int};

use crate::budget::LimitKind;
use crate::interp::{new_string, obj_bytes, Code, Interp};
use crate::obj::{decr_ref_count, incr_ref_count, TclObj};

const TCL_OK: c_int = 0;
const TCL_ERROR: c_int = 1;

/// The limit kinds as the ABI numbers them: 1 the command count, 2 the wall
/// clock, 3 the value size.
fn kind_number(kind: LimitKind) -> c_int {
    match kind {
        LimitKind::Commands => 1,
        LimitKind::WallClock => 2,
        LimitKind::ValueSize => 3,
    }
}

/// The limit kind `number` names, as [`kind_number`] numbers them.
fn kind_of(number: c_int) -> Option<LimitKind> {
    match number {
        1 => Some(LimitKind::Commands),
        2 => Some(LimitKind::WallClock),
        3 => Some(LimitKind::ValueSize),
        _ => None,
    }
}

/// `tcl_engine_set_limits(interp, commands, value_bytes)` — the command count
/// and value-size limits every later evaluation runs under; a negative value
/// leaves that limit off. The wall clock is the host's to keep, by the epoch.
///
/// # Safety
/// `interp` must be live.
#[no_mangle]
pub unsafe extern "C" fn tcl_engine_set_limits(
    interp: *mut Interp,
    commands: i64,
    value_bytes: i64,
) {
    // SAFETY: caller guarantees a live interpreter.
    let interp = unsafe { &*interp };
    interp.set_limits(
        u64::try_from(commands).ok(),
        None,
        u64::try_from(value_bytes).ok(),
    );
}

/// `tcl_engine_begin(interp)` — begin an evaluation under the limits.
///
/// # Safety
/// `interp` must be live.
#[no_mangle]
pub unsafe extern "C" fn tcl_engine_begin(interp: *mut Interp) {
    // SAFETY: caller guarantees a live interpreter.
    unsafe { (*interp).begin_evaluation() }
}

/// `tcl_engine_exceeded(interp) -> kind` — the limit the evaluation outran
/// (1 commands, 2 wall clock, 3 value size), or 0.
///
/// # Safety
/// `interp` must be live.
#[no_mangle]
pub unsafe extern "C" fn tcl_engine_exceeded(interp: *mut Interp) -> c_int {
    // SAFETY: caller guarantees a live interpreter.
    unsafe { (*interp).limit_exceeded() }.map_or(0, kind_number)
}

/// `tcl_engine_exceed(interp, kind) -> TCL_ERROR` — record that host work for
/// the evaluation outran the limit `kind` numbers, and raise its error. A
/// number that names no limit is the command count.
///
/// # Safety
/// `interp` must be live.
#[no_mangle]
pub unsafe extern "C" fn tcl_engine_exceed(interp: *mut Interp, kind: c_int) -> c_int {
    let kind = kind_of(kind).unwrap_or(LimitKind::Commands);
    // SAFETY: caller guarantees a live interpreter.
    let code = unsafe { (*interp).exceed_limit(kind) };
    c_int::try_from(code.as_int()).unwrap_or(TCL_ERROR)
}

/// `tcl_engine_commands_spent(interp) -> count` — the commands the evaluation
/// dispatched.
///
/// # Safety
/// `interp` must be live.
#[no_mangle]
pub unsafe extern "C" fn tcl_engine_commands_spent(interp: *mut Interp) -> i64 {
    // SAFETY: caller guarantees a live interpreter.
    i64::try_from(unsafe { (*interp).commands_since_begin() }).unwrap_or(i64::MAX)
}

/// `tcl_engine_confine_stores(interp)` — refuse every store outside the
/// running procedure's frame, and remove the globals the host seeded.
///
/// # Safety
/// `interp` must be live.
#[no_mangle]
pub unsafe extern "C" fn tcl_engine_confine_stores(interp: *mut Interp) {
    // SAFETY: caller guarantees a live interpreter.
    unsafe { (*interp).confine_stores() }
}

/// `tcl_engine_restrict(interp, allowed, kept)` — keep only the commands the
/// list `allowed` names, by the engine's whitelist rule, and those the list
/// `kept` names.
///
/// # Safety
/// `interp` must be live; `allowed` and `kept` live objects.
#[no_mangle]
pub unsafe extern "C" fn tcl_engine_restrict(
    interp: *mut Interp,
    allowed: *mut TclObj,
    kept: *mut TclObj,
) {
    let words = |list: *mut TclObj| -> Vec<String> {
        crate::parse::split_list(&obj_bytes(list))
            .unwrap_or_default()
            .into_iter()
            .map(|word| String::from_utf8_lossy(&word).into_owned())
            .collect()
    };
    let (allowed, kept) = (words(allowed), words(kept));
    let allowed: Vec<&str> = allowed.iter().map(String::as_str).collect();
    // SAFETY: caller guarantees a live interpreter.
    unsafe { (*interp).restrict_to(&allowed, &kept) }
}

/// `tcl_engine_set_release(interp, profile, length) -> status` — pin the
/// release the profile `profile[..length]` names: `TCL_OK`, or `TCL_ERROR` for
/// a name that is no release the runtime runs.
///
/// # Safety
/// `interp` must be live; `profile` must reference `length` readable bytes.
#[no_mangle]
pub unsafe extern "C" fn tcl_engine_set_release(
    interp: *mut Interp,
    profile: *const c_char,
    length: c_int,
) -> c_int {
    let length = usize::try_from(length).unwrap_or(0);
    // SAFETY: the caller guarantees `length` readable bytes.
    let bytes = unsafe { core::slice::from_raw_parts(profile.cast::<u8>(), length) };
    let Ok(profile) = core::str::from_utf8(bytes) else {
        return TCL_ERROR;
    };
    // SAFETY: caller guarantees a live interpreter.
    match unsafe { (*interp).pin_release(profile) } {
        Some(_) => TCL_OK,
        None => TCL_ERROR,
    }
}

/// `tcl_engine_define_unit(interp, name, params, body) -> status` — define the
/// procedure a compiled unit is, without the `proc` command a whitelist may
/// have removed: `TCL_ERROR` for a parameter list `proc` would refuse.
///
/// # Safety
/// `interp` must be live; `name`, `params` and `body` live objects.
#[no_mangle]
pub unsafe extern "C" fn tcl_engine_define_unit(
    interp: *mut Interp,
    name: *mut TclObj,
    params: *mut TclObj,
    body: *mut TclObj,
) -> c_int {
    // SAFETY: caller guarantees a live interpreter.
    let interp = unsafe { &mut *interp };
    if interp.host_refusal_pending() {
        return TCL_ERROR;
    }
    let name = obj_bytes(name);
    let chosen_body = match interp.choose_original_procedure_body(body) {
        Ok(body) => body,
        Err(error) => {
            return i32::try_from(interp.report_cmd_error(error.into()).as_int())
                .unwrap_or(TCL_ERROR)
        }
    };
    let parameters = match crate::cmd_proc::parse_params_object(interp, params, &name) {
        Ok(parameters) => parameters,
        Err(error) => {
            return i32::try_from(interp.report_cmd_error(error).as_int()).unwrap_or(TCL_ERROR);
        }
    };
    let generation = interp.install_proc_chosen_storage(
        &name,
        parameters,
        Some(params),
        (body, chosen_body),
        None,
        None,
    );
    if generation.is_none() || interp.host_refusal_pending() {
        TCL_ERROR
    } else {
        TCL_OK
    }
}

/// `tcl_engine_provide_package(interp, name, version) -> status` — what
/// `package provide name version` does, for a host outside any command or
/// inside one: `TCL_ERROR`, with the result and error code `package provide`
/// gives, for a version that is not one or a package provided at another.
///
/// # Safety
/// `interp` must be live; `name` and `version` live objects.
#[no_mangle]
pub unsafe extern "C" fn tcl_engine_provide_package(
    interp: *mut Interp,
    name: *mut TclObj,
    version: *mut TclObj,
) -> c_int {
    // SAFETY: caller guarantees a live interpreter.
    let interp = unsafe { &mut *interp };
    match crate::cmd_package::provide_package(interp, &obj_bytes(name), &obj_bytes(version)) {
        Code::Ok => TCL_OK,
        _ => TCL_ERROR,
    }
}

/// `tcl_engine_error_code(interp) -> object` — the error code of the error the
/// interpreter holds (`NONE` when nothing stated one).
///
/// # Safety
/// `interp` must be live.
#[no_mangle]
pub unsafe extern "C" fn tcl_engine_error_code(interp: *mut Interp) -> *mut TclObj {
    // SAFETY: caller guarantees a live interpreter.
    let code = unsafe { (*interp).error_code() };
    let object = new_string(&code);
    // SAFETY: a fresh object; the hold is the caller's.
    unsafe { incr_ref_count(object) };
    object
}

/// `tcl_engine_fail(interp, message, code) -> TCL_ERROR` — the error a host
/// command fails with, as a C command reports one: its message, and its
/// `-errorcode` when `code` is not null (`NONE` when it is).
///
/// # Safety
/// `interp` must be live; `message` a live object, and `code` one or null.
#[no_mangle]
pub unsafe extern "C" fn tcl_engine_fail(
    interp: *mut Interp,
    message: *mut TclObj,
    code: *mut TclObj,
) -> c_int {
    // SAFETY: caller guarantees a live interpreter.
    let interp = unsafe { &mut *interp };
    let code = (!code.is_null()).then(|| obj_bytes(code));
    interp.c_api_error(&obj_bytes(message), code.as_deref());
    TCL_ERROR
}

/// `tcl_engine_return(interp, options, value) -> code` — what `return -options
/// options value` does: the completion a host command answers with a
/// `TCL_RETURN` carrying options.
///
/// # Safety
/// `interp` must be live; `options` and `value` live objects.
#[no_mangle]
pub unsafe extern "C" fn tcl_engine_return(
    interp: *mut Interp,
    options: *mut TclObj,
    value: *mut TclObj,
) -> c_int {
    // SAFETY: caller guarantees a live interpreter.
    let interp = unsafe { &mut *interp };
    let words = [
        new_string(b"return"),
        new_string(b"-options"),
        options,
        value,
    ];
    for &word in &words {
        // SAFETY: each word is live; the call holds it.
        unsafe { incr_ref_count(word) };
    }
    let code: Code = crate::builtins::ret(interp, &words);
    for &word in &words {
        // SAFETY: balances the hold above.
        unsafe { decr_ref_count(word) };
    }
    c_int::try_from(code.as_int()).unwrap_or(TCL_ERROR)
}

#[cfg(test)]
mod procedure_publication_tests {
    use super::*;

    #[test]
    fn abi_definition_uses_original_storage_and_reports_failure_without_later_publication() {
        // Software contract: naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut interp = Interp::new();
            interp.pin_release(profile).unwrap();
            let name = crate::obj::Owned::fresh(new_string(b"unit"));
            let bad_parameters = crate::obj::Owned::fresh(new_string(b"{x y z}"));
            let parameters = crate::obj::Owned::fresh(new_string(b""));
            let body = crate::obj::Owned::fresh(new_string(b"return ORIGINAL"));
            // SAFETY: all borrowed originals and the interpreter remain live.
            assert_eq!(
                unsafe {
                    tcl_engine_define_unit(
                        &mut interp,
                        name.as_ptr(),
                        bad_parameters.as_ptr(),
                        body.as_ptr(),
                    )
                },
                TCL_ERROR
            );
            assert!(interp.resolve_cmd_token(b"unit").is_none());
            // SAFETY: the successful producer uses the same live originals.
            assert_eq!(
                unsafe {
                    tcl_engine_define_unit(
                        &mut interp,
                        name.as_ptr(),
                        parameters.as_ptr(),
                        body.as_ptr(),
                    )
                },
                TCL_OK
            );
            let completion = interp.eval_completion(b"unit").unwrap();
            assert_eq!(completion.code, tcl_runtime_api::Code::Ok);
            assert_eq!(completion.result, b"ORIGINAL");
            let pending_name = crate::obj::Owned::fresh(new_string(b"unentered"));
            interp.refuse_host_command("original ABI host refusal");
            let original = interp.native_execution_refusal().unwrap();
            // SAFETY: a prior host refusal stops before inspecting or publishing originals.
            assert_eq!(
                unsafe {
                    tcl_engine_define_unit(
                        &mut interp,
                        pending_name.as_ptr(),
                        parameters.as_ptr(),
                        body.as_ptr(),
                    )
                },
                TCL_ERROR
            );
            assert_eq!(interp.native_execution_refusal(), Some(original));
            assert!(interp.resolve_cmd_token(b"unentered").is_none());
        }
    }
}
