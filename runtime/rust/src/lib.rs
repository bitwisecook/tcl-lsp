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

//! Rust port of the Tcl WASM runtime — support library + interpreter fallback.
//!
//! Track 1 of the runtime port. The `Tcl_Obj` model is
//! ABI-faithful (`c-extension-abi.md` §4.2), refcounts are balanced, and the
//! alloc/free counters prove leak-freedom.
//!
//! ## Scope so far
//!
//! T1.1 — value model:
//! - [`obj`] — the `#[repr(C)]` `TclObj` value model with `fresh_zero`
//!   constructors, immediate refcount-driven free, and on-demand string shimmer.
//! - [`interp`] — a minimal result-only `Interp` exercising the
//!   `Tcl_SetObjResult`/`Tcl_GetObjResult` ownership handshake.
//! - [`counters`] — the leak-check instrumentation (`tcl_test_*`).
//! - [`capi`] — the `#[no_mangle] extern "C"` C-API exports for the above.
//!
//! T1.2 — parse/subst (a re-derived borrow-based enum model, all `unsafe`-free):
//! - [`parse`] — the script/word parser: a `Command`/`Word`/`WordBody` enum
//!   tree with a literal fast path and a shared component scanner. Backslash
//!   decoding is the shared `tcl_syntax::backslash` (reference `TclParseBackslash`
//!   via `tcl_lexer::backslash_subst`) — the runtime's old `bs.rs` is retired.
//! - [`subst`] — the substitution engine over the shared scanner; variable and
//!   command resolution are supplied as closures (wired to the eval loop in
//!   T1.3/T1.4).
//!
//! The eval loop, frames, namespaces, the command table, and the builtins land
//! in later Track-1 chunks (T1.3–T1.6); the `tcl_*`/`obj_*` codegen-import
//! re-exports and the wasm `memory`/table exports land in T1.6.

// On `wasm32-unknown-unknown` the native-only features — coroutines (native
// stack swap + threads) and the libtommath bignum tower — are cfg-disabled (see
// the `BrowserHost` and `have_tommath` gates below), so the machinery that
// drives them (coroutine context fields and swap methods, the OO exec frame,
// the rand seed, the conditional line-base bookkeeping) is unreachable on that
// target. (The ARE regex engine is the pure-Rust `tcl-regex` crate now, not a
// C FFI, so it builds on wasm32 too — no gate.) Native is the primary build where
// `clippy -D warnings` runs and nothing is dead; only the wasm32 build sees this
// dead code, so the allow is scoped to that target alone.
#![cfg_attr(target_arch = "wasm32", allow(dead_code, unused_imports))]

// The bignum rung of the numeric tower (libtommath `mp_int` FFI + the
// `TCL_BIGNUM_TYPE` obj rep). Compiled only when `build.rs` links libtommath
// (`have_tommath`); the rest of the runtime builds without it.
#[cfg(have_tommath)]
pub mod bignum;
// The `expr` evaluator (value-ops impl of the shared `tcl_syntax::expr` walk);
// needs the bignum tower, so it tracks the same `have_tommath` cfg.
pub mod builtins;
pub mod bytearray;
#[cfg(all(target_arch = "wasm32", target_os = "unknown", have_tommath))]
mod c_alloc;
pub mod capi;
pub mod cmd_alias;
pub mod cmd_array;
pub mod cmd_binary;
pub mod cmd_chan;
pub mod cmd_clock;
pub mod cmd_control;
pub mod cmd_coro;
pub mod cmd_dict;
pub mod cmd_error;
pub mod cmd_eval;
pub mod cmd_event;
pub mod cmd_format;
pub mod cmd_fs;
pub mod cmd_info;
pub mod cmd_list;
// `lseq` arithmetic-series generator; needs the numeric tower (doubles / expr).
#[cfg(have_tommath)]
pub mod cmd_lseq;
// `::tcl::mathfunc::*` / `::tcl::mathop::*` commands; need the numeric tower.
#[cfg(have_tommath)]
pub mod cmd_mathfunc;
#[cfg(have_tommath)]
pub mod cmd_mathop;
pub mod cmd_misc;
pub mod cmd_namespace;
pub mod cmd_oo;
pub mod cmd_package;
pub mod cmd_proc;
pub mod cmd_scan;
// `regexp`/`regsub`, driven by the pure-Rust `tcl-regex` ARE engine. The engine
// is wasm32-clean (no FFI), so regular expressions work on every target.
pub mod cmd_regex;
pub mod cmd_string;
pub mod cmd_switch;
pub mod cmd_trace;
pub mod cmd_var;
pub mod cmd_zlib;
// The codegen-import ABI: the lowercase `tcl_*` host functions the WASM emitter
// imports (`rust/tcl-compiler` codegen), distinct from `capi`'s `Tcl_*` surface.
pub mod codegen_abi;
pub mod counters;
pub mod dict;
// The Tcl 9 stdlib embedded in the binary, seeded into the WASM VFS so the
// filesystem-backed startup path (`init.tcl`, `package require`) works with no
// host filesystem. Gated so non-WASM consumers don't carry the embedded bytes.
#[cfg(feature = "wasm_stdlib")]
pub mod embedded_stdlib;
pub mod ensemble;
#[cfg(have_tommath)]
pub mod expr;
pub mod frame;
// The in-memory filesystem the WASM hosts mount. Compiled on the WASM-stdlib
// build and under `cfg(test)` (its own unit tests run natively).
#[cfg(any(feature = "wasm_stdlib", test))]
pub mod mem_fs;
// The placeholder `wasm32-unknown-unknown` capability host (native builds use
// `tcl-host-native::NativeHost`).
#[cfg(target_arch = "wasm32")]
pub mod host_wasm;
pub mod interp;
pub mod list;
pub mod namespace;
pub mod obj;
pub mod parse;
pub mod state_traits;
// The C-ABI shim that re-exports the pure-Rust ARE engine (`tcl-regex`) under
// the Tcl regex engine's C symbols, so C Tcl code/extensions link against it.
pub mod regex_capi;
pub mod subst;
/// `ValueOps` impl binding `tcl-cmd-core`'s portable command logic to `*mut
/// TclObj` (the value seam).
pub mod value_ops;
pub mod vars;
mod version;

#[cfg(test)]
mod tests {
    use crate::capi::*;
    use crate::counters;
    use core::ffi::c_char;

    /// Reset, run `body`, then assert the run left zero residual and no
    /// double-frees — the T1.1 acceptance shape.
    fn assert_leak_free(body: impl FnOnce()) {
        tcl_test_reset_counters();
        body();
        assert_eq!(
            counters::finalize(),
            0,
            "residual live allocations: {} objs, {} bufs",
            counters::live_objs(),
            counters::live_bufs(),
        );
        assert_eq!(counters::double_free_count(), 0, "double frees detected");
        assert!(!counters::oom(), "allocator hit OOM");
    }

    /// **The T1.1 gate.** The canonical C-API round-trip from
    /// `c-api-ownership-contract.md`'s acceptance:
    /// `Tcl_NewObj` → `Tcl_IncrRefCount` → `Tcl_SetObjResult` →
    /// `Tcl_DecrRefCount`, then interp teardown — zero residual.
    #[test]
    fn round_trip_zero_residual() {
        assert_leak_free(|| unsafe {
            let interp = tcl_runtime_create_interp();
            let obj = Tcl_NewObj(); // fresh_zero (rc 0)
            Tcl_IncrRefCount(obj); // rc 1 — caller owns
            Tcl_SetObjResult(interp, obj); // interp retains → rc 2, old result freed
            Tcl_DecrRefCount(obj); // caller releases → rc 1 (interp holds)
            tcl_runtime_delete_interp(interp); // releases interp's hold → rc 0, freed
        });
    }

    /// `fresh_zero` really is rc 0: creating and then immediately
    /// `Tcl_IncrRefCount`+`Tcl_DecrRefCount` frees the object (no interp).
    #[test]
    fn fresh_zero_incr_decr_frees() {
        assert_leak_free(|| unsafe {
            let obj = Tcl_NewWideIntObj(42);
            Tcl_IncrRefCount(obj);
            Tcl_DecrRefCount(obj); // rc 0 → freed
        });
    }

    /// A string object copies its bytes and frees both header and buffer.
    #[test]
    fn string_obj_owns_and_frees_buffer() {
        assert_leak_free(|| unsafe {
            let s = b"hello\0";
            let obj = Tcl_NewStringObj(s.as_ptr() as *const c_char, 5);
            Tcl_IncrRefCount(obj);
            // Read it back through the borrowed string rep.
            let mut len = 0isize;
            let p = Tcl_GetStringFromObj(obj, &mut len);
            assert_eq!(len, 5);
            let got = core::slice::from_raw_parts(p as *const u8, len as usize);
            assert_eq!(got, b"hello");
            Tcl_DecrRefCount(obj);
        });
    }

    /// Int → string shimmer allocates the rep buffer on demand and frees it.
    #[test]
    fn int_shimmers_to_string() {
        assert_leak_free(|| unsafe {
            let obj = Tcl_NewWideIntObj(-12345);
            Tcl_IncrRefCount(obj);
            let mut len = 0isize;
            let p = Tcl_GetStringFromObj(obj, &mut len);
            let got = core::slice::from_raw_parts(p as *const u8, len as usize);
            assert_eq!(got, b"-12345");
            Tcl_DecrRefCount(obj);
        });
    }

    /// Releasing an object with no outstanding reference is counted as a
    /// double-free, not a crash, and does not free twice.
    #[test]
    fn double_free_is_detected() {
        tcl_test_reset_counters();
        unsafe {
            let obj = Tcl_NewObj();
            Tcl_IncrRefCount(obj); // rc 1
            Tcl_DecrRefCount(obj); // rc 0 → freed
                                   // obj is now dangling; calling decr again must be caught BEFORE we
                                   // would dereference freed memory. We exercise the guard with a
                                   // fresh object instead to avoid UB in the test:
            let obj2 = Tcl_NewObj(); // rc 0 (never incremented)
            Tcl_DecrRefCount(obj2); // release at rc 0 → double-free counted
        }
        assert_eq!(counters::double_free_count(), 1);
        // obj2 was never freed (guard refused), so it leaks by design here;
        // this test asserts the *counter*, not leak-freedom.
    }

    /// i64::MIN formats without overflow (itoa edge case).
    #[test]
    fn int_min_shimmers() {
        assert_leak_free(|| unsafe {
            let obj = Tcl_NewWideIntObj(i64::MIN);
            Tcl_IncrRefCount(obj);
            let mut len = 0isize;
            let p = Tcl_GetStringFromObj(obj, &mut len);
            let got = core::slice::from_raw_parts(p as *const u8, len as usize);
            assert_eq!(got, b"-9223372036854775808");
            Tcl_DecrRefCount(obj);
        });
    }
}
