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

//! The **codegen-import ABI** — the lowercase `tcl_*` host functions the WASM
//! emitter imports (module `"tcl"`) and calls from an emitted module.
//!
//! This is a *different* ABI from [`crate::capi`]'s `Tcl_*` surface. `capi`
//! exports the C **Tcl extension** API (`c-extension-abi.md` §4.3, consumed by an
//! unmodified C Tcl extension). This module exports the **compiler's** runtime
//! ABI: the small set of `tcl_*` host functions the AOT WASM backend
//! (`rust/tcl-compiler/src/codegen/wasm/backend.rs`) emits `call`s to. The
//! reference set is the retiring Python `compiler/codegen/wasm/_imports.py`.
//!
//! ## The eval-fallback tier
//!
//! The backend's current tier boxes each leaf command / condition as a Tcl
//! string in the module's data section and hands it to the runtime to interpret:
//!
//! ```text
//! command   :  code = tcl_eval_code(tcl_obj_new_string(off, len)); …dispatch on code…
//! condition :  if (tcl_expr_bool(tcl_obj_new_string(off, len)))  …
//! ```
//!
//! The emitted control flow inspects the completion `code` a leaf command
//! returns and honours it — an `error` / `return` unwinds the function, a
//! `break` / `continue` re-enters the enclosing loop's structural scopes — so
//! abrupt completion propagates like the tree-walker's command loop
//! (`RUST_ISSUE_010`). [`tcl_eval`] (returning the result object) is retained
//! for a host that wants the *value* of an evaluated script (the whole-program
//! bootstrap reads a query result through it), but the AOT command emitter uses
//! [`tcl_eval_code`].
//!
//! ## Ownership contract (leak-balanced; the alloc/free counters prove it)
//!
//! Every boxed object flows through exactly one consumer, so references balance
//! with no per-call release by the emitter:
//!
//! - [`tcl_obj_new_string`] returns a **fresh `rc 0`** object (the codebase's
//!   `Tcl_New*` convention).
//! - [`tcl_eval`], [`tcl_eval_code`], and [`tcl_expr_bool`] **adopt and free**
//!   their object argument (the boxed script / expression — the emitter never
//!   releases it separately).
//! - [`tcl_eval`] returns a **new owned (`+1`) reference** to the result; the
//!   emitter balances it with one [`tcl_obj_release`]. [`tcl_eval_code`] returns
//!   only the completion code (an `i32`), leaving the result as the interp's own
//!   (borrowed) result — nothing to release.
//!
//! ## The current interp
//!
//! The emitter's imports take no interp (a whole-program WASM artifact has one
//! interp for the program). The host / runtime bootstrap calls
//! [`tcl_runtime_set_current_interp`] once before invoking the emitted `::top`;
//! the `tcl_*` functions evaluate against that interp. WASM is single-threaded
//! in our target, so the thread-local *is* the module global.

use core::cell::Cell;
use core::ptr;

use crate::interp::{drop_fresh, obj_bytes, Interp};
use crate::obj::{self, new_string_bytes, TclObj};

// The interp emitted modules evaluate against (see the module docs). Null until
// the host calls [`tcl_runtime_set_current_interp`].
//
// Native: a `thread_local!` keeps the parallel test suite's interps isolated.
// WASM: the bare wasip1 cdylib has no `_initialize`/TLS bootstrap, so a
// `thread_local!` reads an uninitialised `__tls_base` and never observes
// `set_current_interp`. WASM is single-threaded in our target, so a plain
// `AtomicPtr` global *is* the per-module current interp — and needs no TLS init.
#[cfg(not(target_arch = "wasm32"))]
thread_local! {
    static CURRENT_INTERP: Cell<*mut Interp> = const { Cell::new(ptr::null_mut()) };
}
#[cfg(target_arch = "wasm32")]
static CURRENT_INTERP: core::sync::atomic::AtomicPtr<Interp> =
    core::sync::atomic::AtomicPtr::new(ptr::null_mut());

/// Borrow the current interp pointer (null when unset).
fn current_interp() -> *mut Interp {
    #[cfg(not(target_arch = "wasm32"))]
    {
        CURRENT_INTERP.with(Cell::get)
    }
    #[cfg(target_arch = "wasm32")]
    {
        CURRENT_INTERP.load(core::sync::atomic::Ordering::Relaxed)
    }
}

/// Set the interp the codegen ABI evaluates against. The runtime bootstrap (or a
/// test host) calls this once before running an emitted module's `::top`. Pass
/// null to clear it (e.g. before the interp is deleted).
#[no_mangle]
pub extern "C" fn tcl_runtime_set_current_interp(interp: *mut Interp) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        CURRENT_INTERP.with(|c| c.set(interp));
    }
    #[cfg(target_arch = "wasm32")]
    {
        CURRENT_INTERP.store(interp, core::sync::atomic::Ordering::Relaxed);
    }
}

/// `tcl_runtime_init_library() -> i32` — bootstrap the standard library on the
/// current interp, like C's `Tcl_Init`. [`tcl_runtime_set_current_interp`] must
/// have run first. Sources `$TCL_LIBRARY/init.tcl` from the host filesystem —
/// the embedded-stdlib VFS on the `wasm_stdlib` build — bringing up the
/// `unknown`/auto-load/`package` machinery so `package require` works. Returns
/// `0` on success, `1` on error or when no current interp is set. A standalone
/// emitted module's `_start` calls this between `set_current_interp` and `::top`
/// so the compiled script runs against a fully initialised interpreter.
#[no_mangle]
pub extern "C" fn tcl_runtime_init_library() -> i32 {
    let interp = current_interp();
    if interp.is_null() {
        return 1;
    }
    // SAFETY: `interp` is the live current interp set by the bootstrap.
    let code = unsafe { (*interp).init_library() };
    i32::from(code == crate::interp::Code::Error)
}

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

/// `tcl_eval(script) -> result` — evaluate `script` against the current interp.
/// **Adopts (frees)** the `rc 0` `script`; returns a **new owned (`+1`)**
/// reference to the result that the caller must release with
/// [`tcl_obj_release`]. (Completion codes are discarded in this tier — faithful
/// `return`/`break`/`error` propagation is an AOT-tier follow-up.)
///
/// # Safety
/// `script` must be a live `rc 0` object from [`tcl_obj_new_string`]; the current
/// interp (if set) must be live.
#[no_mangle]
pub unsafe extern "C" fn tcl_eval(script: *mut TclObj) -> *mut TclObj {
    // Copy the script text out, then free the adopted script object.
    let src = obj_bytes(script);
    drop_fresh(script);

    let interp = current_interp();
    if interp.is_null() {
        // Misuse (no current interp); stay leak-safe — return an owned empty.
        let empty = new_string_bytes(b"");
        // SAFETY: `empty` is a live fresh object.
        unsafe { obj::incr_ref_count(empty) };
        return empty;
    }
    // SAFETY: `interp` is the live current interp; `eval_str` takes `&mut`.
    let result = unsafe {
        (*interp).eval_str(&src);
        (*interp).get_obj_result()
    };
    // The result is borrowed (interp keeps its `+1`); take our own `+1` so the
    // caller's `tcl_obj_release` does not free the interp's reference.
    // SAFETY: `result` is the live interp result object.
    unsafe { obj::incr_ref_count(result) };
    result
}

/// `tcl_eval_code(script) -> i32` — evaluate `script` against the current interp
/// and return its **completion code** (`0` ok, `1` error, `2` return, `3` break,
/// `4` continue, or a `return -code N` value), leaving the result as the interp's
/// own result. This is the AOT command emitter's eval: the emitted control flow
/// branches on the returned code so an `error` / `return` inside a compiled
/// `if`/`while`/`for` body unwinds, and a `break` / `continue` re-enters the
/// enclosing loop — faithful abrupt-completion propagation (`RUST_ISSUE_010`).
///
/// **Adopts (frees)** the `rc 0` `script`. Unlike [`tcl_eval`] it returns no
/// owned reference (the result stays the interp's borrowed result), so there is
/// nothing for the emitter to release. With no current interp set, nothing runs
/// and it reports `0` (ok) — leak-safe, matching [`tcl_eval`]'s misuse path.
///
/// # Safety
/// `script` must be a live `rc 0` object from [`tcl_obj_new_string`]; the current
/// interp (if set) must be live.
#[no_mangle]
pub unsafe extern "C" fn tcl_eval_code(script: *mut TclObj) -> i32 {
    // Copy the script text out, then free the adopted script object.
    let src = obj_bytes(script);
    drop_fresh(script);

    let interp = current_interp();
    if interp.is_null() {
        return 0; // Misuse (no current interp): nothing ran — report ok.
    }
    // SAFETY: `interp` is the live current interp; `eval_str` takes `&mut`.
    let code = unsafe { (*interp).eval_str(&src) };
    // Every completion code (0..=4 or a `return -code N`) is an `i32`; the
    // `unwrap_or` is unreachable (kept so a future wider code degrades to error).
    i32::try_from(code.as_int()).unwrap_or(1)
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

/// `tcl_expr_bool(expr) -> i32` — evaluate `expr` as a Tcl boolean (`1`/`0`).
/// **Adopts (frees)** the `rc 0` `expr`. On an expression error — or in a build
/// without the numeric tower (no `expr` evaluator) — yields `0`. The wasm
/// runtime now links libtommath (`build.rs`), so `have_tommath` is set and this
/// uses the real evaluator there too — AOT-emitted `if`/`while` conditions
/// evaluate correctly.
///
/// # Safety
/// `expr` must be a live `rc 0` object from [`tcl_obj_new_string`]; the current
/// interp (if set) must be live.
#[no_mangle]
pub unsafe extern "C" fn tcl_expr_bool(expr: *mut TclObj) -> i32 {
    let interp = current_interp();
    // SAFETY: `expr` is live for the duration of the read; `interp` (if non-null)
    // is the live current interp.
    let truth = unsafe { expr_bool_impl(interp, expr) };
    drop_fresh(expr);
    truth
}

/// The expression-condition path, present only with the numeric tower.
///
/// # Safety
/// `expr` must be live; `interp` must be null or the live current interp.
#[cfg(have_tommath)]
unsafe fn expr_bool_impl(interp: *mut Interp, expr: *mut TclObj) -> i32 {
    if interp.is_null() {
        return 0;
    }
    // SAFETY: `interp` is the live current interp.
    let ok = crate::builtins::eval_bool_expr(unsafe { &mut *interp }, expr);
    i32::from(matches!(ok, Ok(true)))
}

/// Without the numeric tower there is no `expr` evaluator (the `expr` module is
/// `have_tommath`-gated), so conditions evaluate false. This branch now only
/// applies to a build that deliberately omits the tower (e.g. a wasm build where
/// `clang`/libtommath was unavailable and `build.rs` degraded the backend off).
/// The export still exists so emitted modules link.
///
/// # Safety
/// Trivially safe (dereferences nothing).
#[cfg(not(have_tommath))]
unsafe fn expr_bool_impl(_interp: *mut Interp, _expr: *mut TclObj) -> i32 {
    0
}

#[cfg(all(test, have_tommath))]
mod tests {
    use super::*;
    use crate::capi::{tcl_runtime_create_interp, tcl_runtime_delete_interp};
    use crate::counters;

    /// Run `body` under the alloc/free counters and assert zero residual — the
    /// codegen ABI's references must balance exactly like the rest of the runtime.
    fn leak_free(body: impl FnOnce()) {
        counters::reset();
        body();
        assert_eq!(
            counters::finalize(),
            0,
            "residual: {} objs, {} bufs",
            counters::live_objs(),
            counters::live_bufs()
        );
        assert_eq!(counters::double_free_count(), 0, "double frees detected");
    }

    /// Box a `&[u8]` as the emitter would (a fresh `rc 0` script/expr object).
    unsafe fn box_str(s: &[u8]) -> *mut TclObj {
        // SAFETY: `s` is a valid readable slice.
        unsafe { tcl_obj_new_string(s.as_ptr(), s.len() as i32) }
    }

    /// The eval-fallback round-trip: box → eval → release, against the current
    /// interp, leaves zero residual *and* the evaluated side effect is real
    /// (the variable it set is observable on a second eval).
    #[test]
    fn eval_box_release_round_trip() {
        leak_free(|| unsafe {
            let interp = tcl_runtime_create_interp();
            tcl_runtime_set_current_interp(interp);

            let result = tcl_eval(box_str(b"set x 42"));
            assert_eq!(obj_bytes(result), b"42");
            tcl_obj_release(result);

            // The side effect persisted: reading the var back yields 42.
            let read = tcl_eval(box_str(b"set x"));
            assert_eq!(obj_bytes(read), b"42");
            tcl_obj_release(read);

            tcl_runtime_set_current_interp(ptr::null_mut());
            tcl_runtime_delete_interp(interp);
        });
    }

    /// `tcl_expr_bool` evaluates real Tcl boolean expressions and frees its
    /// (adopted) operand object.
    #[test]
    fn expr_bool_true_and_false() {
        leak_free(|| unsafe {
            let interp = tcl_runtime_create_interp();
            tcl_runtime_set_current_interp(interp);

            assert_eq!(tcl_expr_bool(box_str(b"1 < 2")), 1);
            assert_eq!(tcl_expr_bool(box_str(b"5 == 0")), 0);
            assert_eq!(tcl_expr_bool(box_str(b"2 + 2 == 4")), 1);
            // A malformed expression is false, not a panic.
            assert_eq!(tcl_expr_bool(box_str(b"1 +")), 0);

            tcl_runtime_set_current_interp(ptr::null_mut());
            tcl_runtime_delete_interp(interp);
        });
    }

    /// `tcl_eval_code` reports the completion code of the evaluated script (so
    /// the AOT control flow can honour it) and stays leak-balanced — the result
    /// stays the interp's own, with no owned reference to release.
    #[test]
    fn eval_code_reports_completion_and_side_effects() {
        leak_free(|| unsafe {
            let interp = tcl_runtime_create_interp();
            tcl_runtime_set_current_interp(interp);

            // Ok (0): a plain command, and its side effect persisted.
            assert_eq!(tcl_eval_code(box_str(b"set x 7")), 0);
            let read = tcl_eval(box_str(b"set x"));
            assert_eq!(obj_bytes(read), b"7");
            tcl_obj_release(read);

            // Error (1), return (2), break (3), continue (4).
            assert_eq!(tcl_eval_code(box_str(b"error boom")), 1);
            assert_eq!(tcl_eval_code(box_str(b"return 9")), 2);
            assert_eq!(tcl_eval_code(box_str(b"break")), 3);
            assert_eq!(tcl_eval_code(box_str(b"continue")), 4);
            // `return -code error` (default -level 1) is a *deferred* return: it
            // completes with `return` (2) and the `-code` is applied only at a
            // proc/source boundary. `-level 0` applies the code immediately.
            assert_eq!(tcl_eval_code(box_str(b"return -code error boom")), 2);
            assert_eq!(tcl_eval_code(box_str(b"return -level 0 -code 42 x")), 42);

            tcl_runtime_set_current_interp(ptr::null_mut());
            tcl_runtime_delete_interp(interp);
        });
    }

    /// With no current interp set, the ABI stays leak-safe (an owned empty result
    /// the caller releases; conditions are false; `tcl_eval_code` reports ok).
    #[test]
    fn no_current_interp_is_leak_safe() {
        leak_free(|| unsafe {
            tcl_runtime_set_current_interp(ptr::null_mut());
            let result = tcl_eval(box_str(b"set x 42"));
            assert_eq!(obj_bytes(result), b"");
            tcl_obj_release(result);
            assert_eq!(tcl_expr_bool(box_str(b"1 < 2")), 0);
            assert_eq!(tcl_eval_code(box_str(b"error boom")), 0);
        });
    }
}
