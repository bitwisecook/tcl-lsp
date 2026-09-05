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
//! (`rust/tcl-compiler/src/codegen/wasm/backend.rs`) emits `call`s to.
//!
//! ## The prebuilt-argv tier
//!
//! The general backend's **normal** path for a leaf command evaluates the words
//! itself and hands this ABI a complete argv, so the runtime resolves and
//! dispatches the command without re-lexing, re-parsing, or re-substituting the
//! source:
//!
//! ```text
//! frame = tcl_codegen_call_frame_alloc(bytes, 4);   ;; argv + completion storage
//!         tcl_obj_new_string_owned(off, len)        ;; a literal word
//!         tcl_codegen_var_get(off, len)             ;; a $scalar word
//!         tcl_codegen_var_get_element(…)            ;; a $arr(key) word
//!         tcl_codegen_word_concat(parts, count)     ;; a compound word
//!         tcl_invoke_argv(argv, argc, completion);  ;; …dispatch on the code…
//!         tcl_obj_release(word) per slot; tcl_codegen_call_frame_free(frame)
//! ```
//!
//! `docs/design/compiler/wasm-codegen.md` describes the frame layout and the
//! single cleanup path the emitter uses so an abrupt completion cannot leak.
//!
//! ## The eval-fallback tier
//!
//! A word shape the emitter cannot prove — `{*}` expansion, backslash
//! substitution, a computed variable name — keeps its whole statement on the
//! older tier, which boxes the command / condition as a Tcl string in the
//! module's data section and hands it to the runtime to interpret. Conditions
//! always take this path:
//!
//! ```text
//! command   :  code = tcl_eval_code(tcl_obj_new_string(off, len)); …dispatch on code…
//! condition :  if (tcl_expr_bool(tcl_obj_new_string(off, len)))  …
//! ```
//!
//! The emitted control flow inspects the completion `code` a leaf command
//! returns and honours it — an `error` / `return` unwinds the function, a
//! `break` / `continue` re-enters the enclosing loop's structural scopes — so
//! abrupt completion propagates like the tree-walker's command loop.
//! [`tcl_eval`] (returning the result object) is retained
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
#[cfg(target_arch = "wasm32")]
use core::sync::atomic::{AtomicUsize, Ordering};
use std::alloc::{alloc, dealloc, handle_alloc_error, Layout};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use tcl_dialect::model::SurfaceQuery;
use tcl_registry::{CommandRegistry, IntrinsicId, SemanticOperationId};
use tcl_runtime_api::guard::{GuardDomains, GuardIdentity, GuardToken};

use crate::interp::{drop_fresh, obj_bytes, Interp, NativeProcEntry};
use crate::obj::{self, new_string_bytes, TclObj};
#[cfg(target_arch = "wasm32")]
use tcl_runtime_api::codegen_abi::{
    WASM32_COMPLETION_ALIGN, WASM32_COMPLETION_CODE_OFFSET, WASM32_COMPLETION_OPTIONS_OFFSET,
    WASM32_COMPLETION_RESULT_OFFSET, WASM32_COMPLETION_SIZE,
};

/// A command completion crossing the compiler/runtime ABI.
///
/// This is the C-compatible storage layout used as the explicit output of
/// [`tcl_invoke_argv`]. Consumers must provide storage with
/// `size_of::<TclCompletionAbi>()` bytes and `align_of::<TclCompletionAbi>()`
/// alignment (or the corresponding C `sizeof`/`_Alignof`). The `code` field is
/// a Tcl completion code, including every arbitrary `i32` produced by
/// `return -code N`. `result` and `options` are separate **owned** `Tcl_Obj`
/// references; after a successful write, release both exactly once with
/// [`tcl_obj_release`], or release the pair once with
/// [`tcl_completion_release`].
///
/// The struct is deliberately output storage, rather than a C aggregate return:
/// a by-value aggregate return has target-specific hidden-sret lowering and
/// therefore is not a stable WASM import signature.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TclCompletionAbi {
    /// Tcl completion code: `0` through `4`, or any other `i32`.
    pub code: i32,
    /// Owned completion result object (never null after a successful write).
    pub result: *mut TclObj,
    /// Owned return-options dictionary (never null after a successful write).
    pub options: *mut TclObj,
}

/// The invocation ran and wrote a [`TclCompletionAbi`]. A Tcl error is reported
/// in `out.code == 1`, not through this status.
pub const TCL_INVOKE_ABI_OK: i32 = 0;
/// `out` was null, so no completion could be written.
pub const TCL_INVOKE_ABI_NULL_OUT: i32 = -1;
/// No current interpreter was installed for the codegen ABI.
pub const TCL_INVOKE_ABI_NO_CURRENT_INTERP: i32 = -2;
/// `argc` was zero, negative, or did not fit the target address space.
pub const TCL_INVOKE_ABI_INVALID_ARGC: i32 = -3;
/// `argv` was null for a non-empty argv.
pub const TCL_INVOKE_ABI_NULL_ARGV: i32 = -4;
/// At least one argv entry was null.
pub const TCL_INVOKE_ABI_NULL_WORD: i32 = -5;
/// The requested guarded intrinsic cannot run directly; use generic argv invoke.
///
/// No completion is written for this status, so the caller must not release
/// its output storage before taking the exact generic slow path.
pub const TCL_INTRINSIC_ABI_DECLINED: i32 = 1;

/// A `tcl_value_get_*` read succeeded and wrote its `out` storage.
pub const TCL_VALUE_GET_OK: i32 = 0;
/// A `tcl_value_get_*` read failed: the current interpreter carries the Tcl
/// error (C's message and `-errorcode`) and `out` is untouched.
pub const TCL_VALUE_GET_ERROR: i32 = 1;

#[cfg(target_arch = "wasm32")]
const _: () = {
    assert!(core::mem::size_of::<TclCompletionAbi>() == WASM32_COMPLETION_SIZE as usize);
    assert!(core::mem::align_of::<TclCompletionAbi>() == WASM32_COMPLETION_ALIGN as usize);
    assert!(
        core::mem::offset_of!(TclCompletionAbi, code) == WASM32_COMPLETION_CODE_OFFSET as usize
    );
    assert!(
        core::mem::offset_of!(TclCompletionAbi, result) == WASM32_COMPLETION_RESULT_OFFSET as usize
    );
    assert!(
        core::mem::offset_of!(TclCompletionAbi, options)
            == WASM32_COMPLETION_OPTIONS_OFFSET as usize
    );
};

// Number of live frames allocated through the compiler transport boundary.
//
// This intentionally tracks only raw transient frames, not Tcl objects; test
// code uses it to prove that every generated cleanup path balances allocation.
//
// Per-thread on native, for the same reason `crate::counters` is: the parallel
// `cargo test` build must not let one test's frames show up in another test's
// ledger. WASM keeps a plain global — it is single-threaded, and the bare
// wasip1 cdylib has no TLS bootstrap (see `CURRENT_INTERP` below).
#[cfg(not(target_arch = "wasm32"))]
thread_local! {
    static CODEGEN_CALL_FRAMES_OUTSTANDING: Cell<usize> = const { Cell::new(0) };
}
#[cfg(target_arch = "wasm32")]
static CODEGEN_CALL_FRAMES_OUTSTANDING: AtomicUsize = AtomicUsize::new(0);

fn call_frame_allocated() {
    #[cfg(not(target_arch = "wasm32"))]
    CODEGEN_CALL_FRAMES_OUTSTANDING.with(|frames| frames.set(frames.get().saturating_add(1)));
    #[cfg(target_arch = "wasm32")]
    CODEGEN_CALL_FRAMES_OUTSTANDING.fetch_add(1, Ordering::SeqCst);
}

fn call_frame_released() {
    #[cfg(not(target_arch = "wasm32"))]
    CODEGEN_CALL_FRAMES_OUTSTANDING.with(|frames| frames.set(frames.get().saturating_sub(1)));
    #[cfg(target_arch = "wasm32")]
    CODEGEN_CALL_FRAMES_OUTSTANDING.fetch_sub(1, Ordering::SeqCst);
}

fn call_frames_outstanding() -> usize {
    #[cfg(not(target_arch = "wasm32"))]
    {
        CODEGEN_CALL_FRAMES_OUTSTANDING.with(Cell::get)
    }
    #[cfg(target_arch = "wasm32")]
    {
        CODEGEN_CALL_FRAMES_OUTSTANDING.load(Ordering::SeqCst)
    }
}

/// Live frame layouts, keyed by their exact returned shared-memory address.
///
/// The registry makes the public free import robust against a forged address,
/// mismatched layout, or a second free: only an allocation created by this ABI
/// can yield a layout to `dealloc`.
static CODEGEN_CALL_FRAME_LAYOUTS: OnceLock<Mutex<HashMap<usize, Layout>>> = OnceLock::new();

fn codegen_call_frame_layouts() -> &'static Mutex<HashMap<usize, Layout>> {
    CODEGEN_CALL_FRAME_LAYOUTS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn lock_call_frame_layouts() -> std::sync::MutexGuard<'static, HashMap<usize, Layout>> {
    codegen_call_frame_layouts()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Return a distinct, dynamically allocated shared-memory call frame.
///
/// A generated module must use this for transient argv and completion storage;
/// it must not place either in a data segment or at a fixed memory offset.
/// Each successful call has one matching [`tcl_codegen_call_frame_free`]. The
/// allocation is independent of the Tcl interpreter, so a command may re-enter
/// generated code (or allocate another frame) during [`tcl_invoke_argv`]
/// without overwriting its caller's argv or completion triple.
///
/// `bytes` must be positive and `align` must be a non-zero power of two. An
/// invalid layout returns null. A valid allocation either returns a suitably
/// aligned frame or terminates through Rust's allocation-error path; it never
/// returns a null frame that could be mistaken for usable shared storage.
#[no_mangle]
pub extern "C" fn tcl_codegen_call_frame_alloc(bytes: i32, align: i32) -> *mut u8 {
    let Ok(bytes) = usize::try_from(bytes) else {
        return ptr::null_mut();
    };
    let Ok(align) = usize::try_from(align) else {
        return ptr::null_mut();
    };
    let Ok(layout) = Layout::from_size_align(bytes, align) else {
        return ptr::null_mut();
    };
    if layout.size() == 0 {
        return ptr::null_mut();
    }
    // SAFETY: `layout` was validated above. The allocation is returned to the
    // caller as an opaque frame and is released only by the matching free ABI.
    let frame = unsafe { alloc(layout) };
    if frame.is_null() {
        handle_alloc_error(layout);
    }
    lock_call_frame_layouts().insert(frame.addr(), layout);
    call_frame_allocated();
    frame
}

/// Release exactly one call frame returned by [`tcl_codegen_call_frame_alloc`].
///
/// The runtime records the exact allocation layout at allocation time, so the
/// caller supplies only `frame` and cannot forge a deallocation layout. Invoke
/// this once after releasing all objects and completion references stored in
/// the frame. Null, unknown, and repeated frees return `-1`; a successful
/// deallocation returns `0`.
///
/// # Safety
/// `frame` may be any address. Unknown addresses are rejected before
/// deallocation; a valid frame must not be accessed concurrently by another
/// caller while it is being freed.
#[no_mangle]
pub unsafe extern "C" fn tcl_codegen_call_frame_free(frame: *mut u8) -> i32 {
    if frame.is_null() {
        return -1;
    }
    let Some(layout) = lock_call_frame_layouts().remove(&frame.addr()) else {
        return -1;
    };
    // SAFETY: upheld by this function's contract.
    unsafe { dealloc(frame, layout) };
    call_frame_released();
    0
}

/// Return the number of outstanding compiler transport call frames on this
/// thread.
///
/// This is a diagnostic/test boundary rather than an allocation mechanism. A
/// generated module is single-threaded, so "this thread" is the whole program
/// there; the native test build keeps a per-thread ledger so parallel tests do
/// not observe each other's frames.
#[no_mangle]
pub extern "C" fn tcl_codegen_call_frame_outstanding() -> i32 {
    i32::try_from(call_frames_outstanding()).unwrap_or(i32::MAX)
}

/// Convert the shared semantic completion to its object-handle ABI form.
fn completion_abi(completion: tcl_runtime_api::Completion<*mut TclObj>) -> TclCompletionAbi {
    let code = match completion.code {
        tcl_runtime_api::Code::Ok => 0,
        tcl_runtime_api::Code::Error => 1,
        tcl_runtime_api::Code::Return => 2,
        tcl_runtime_api::Code::Break => 3,
        tcl_runtime_api::Code::Continue => 4,
        tcl_runtime_api::Code::Other(code) => code,
    };
    TclCompletionAbi {
        code,
        result: completion.result,
        options: completion.options,
    }
}

/// Make an owned host-boundary error completion when there is no interpreter
/// state available to create a normal Tcl error.
fn detached_error_completion(message: &[u8]) -> TclCompletionAbi {
    let result = new_string_bytes(message);
    let options = crate::dict::new_dict_obj(&[
        (new_string_bytes(b"-code"), new_string_bytes(b"1")),
        (new_string_bytes(b"-level"), new_string_bytes(b"0")),
        (new_string_bytes(b"-errorcode"), new_string_bytes(b"NONE")),
        (new_string_bytes(b"-errorinfo"), new_string_bytes(message)),
    ]);
    // SAFETY: both objects are fresh and live. The ABI transfers one owned
    // reference of each to its caller.
    unsafe {
        obj::incr_ref_count(result);
        obj::incr_ref_count(options);
    }
    TclCompletionAbi {
        code: 1,
        result,
        options,
    }
}

/// Write one completion to ABI-provided output storage.
///
/// # Safety
/// `out` must be non-null, aligned, and writable for one [`TclCompletionAbi`].
unsafe fn write_completion(out: *mut TclCompletionAbi, completion: TclCompletionAbi) {
    // SAFETY: guaranteed by the caller of this helper.
    unsafe { out.write(completion) };
}

/// Keep caller-owned argv references alive for exactly one dispatch.
///
/// The ABI accepts borrowed words, so it first takes a temporary reference to
/// every word and returns to the caller's counts in `Drop`, including every
/// normal error/completion path through the dispatcher.
struct BorrowedArgv<'a> {
    words: &'a [*mut TclObj],
}

impl<'a> BorrowedArgv<'a> {
    /// # Safety
    /// Every word must be a live object with a caller-owned reference.
    unsafe fn retain(words: &'a [*mut TclObj]) -> Self {
        for &word in words {
            // SAFETY: upheld by this method's contract.
            unsafe { obj::incr_ref_count(word) };
        }
        Self { words }
    }
}

impl Drop for BorrowedArgv<'_> {
    fn drop(&mut self) {
        for &word in self.words {
            // SAFETY: `retain` took exactly one reference on every live word.
            unsafe { obj::decr_ref_count(word) };
        }
    }
}

unsafe fn input_bytes<'a>(ptr: *const u8, len: i32) -> &'a [u8] {
    if ptr.is_null() || len <= 0 {
        return b"";
    }
    // SAFETY: codegen passes a data-segment address and its exact byte length.
    unsafe { core::slice::from_raw_parts(ptr, len as usize) }
}

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
pub(crate) fn current_interp() -> *mut Interp {
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

/// Report a failed typed read on the current interpreter, exactly as the
/// corresponding `Tcl_Get*FromObj` does: C's message as the interpreter result,
/// C's `-errorcode`. Returns the ABI's error status.
///
/// # Safety
/// `interp` must be the live current interpreter.
unsafe fn typed_read_error(interp: *mut Interp, error: &crate::typed_value::TypedError) -> i32 {
    // SAFETY: forwarded per this function's contract.
    unsafe { (*interp).error_with_code(&error.message, error.code) };
    TCL_VALUE_GET_ERROR
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
    let interp = current_interp();
    if value.is_null() || out.is_null() || interp.is_null() {
        return TCL_VALUE_GET_ERROR;
    }
    match crate::typed_value::wide_int(value) {
        Ok(parsed) => {
            // SAFETY: `out` is writable aligned storage per the contract.
            unsafe { out.write(parsed) };
            TCL_VALUE_GET_OK
        }
        // SAFETY: `interp` is the live current interpreter.
        Err(error) => unsafe { typed_read_error(interp, &error) },
    }
}

/// `tcl_value_get_double(value, out) -> status` — read a boxed Tcl value as a
/// native `f64` (`Tcl_GetDoubleFromObj`), with the same status contract and the
/// same write-back as [`tcl_value_get_wide_int`]. An integer or bignum widens;
/// `NaN` is a value here, not an error (only the boolean context refuses it).
///
/// # Safety
/// `value` must be a live object with a caller-owned reference, and `out` must
/// be writable, properly aligned `f64` storage.
#[no_mangle]
pub unsafe extern "C" fn tcl_value_get_double(value: *mut TclObj, out: *mut f64) -> i32 {
    let interp = current_interp();
    if value.is_null() || out.is_null() || interp.is_null() {
        return TCL_VALUE_GET_ERROR;
    }
    match crate::typed_value::double(value) {
        Ok(parsed) => {
            // SAFETY: `out` is writable aligned storage per the contract.
            unsafe { out.write(parsed) };
            TCL_VALUE_GET_OK
        }
        // SAFETY: `interp` is the live current interpreter.
        Err(error) => unsafe { typed_read_error(interp, &error) },
    }
}

/// `tcl_value_get_bool(value, out) -> status` — read a boxed Tcl value in
/// **boolean context** (`Tcl_GetBooleanFromObj`), writing `0`/`1`.
///
/// This is the acceptor behind `if`, `while`, and `expr`'s `?:`/`&&`/`||`/`!`:
/// a boolean word by unique case-insensitive prefix (`tru`, `ye`, `of`; the
/// ambiguous `o` is refused), else any number compared against zero. The words
/// come from [`tcl_syntax::boolean`], the shared owner every boolean acceptor in
/// this tree uses, so compiled code cannot drift from interpreted code. `NaN` is
/// C's `floating point value is Not a Number` domain error.
///
/// # Safety
/// `value` must be a live object with a caller-owned reference, and `out` must
/// be writable, properly aligned `i32` storage.
#[no_mangle]
pub unsafe extern "C" fn tcl_value_get_bool(value: *mut TclObj, out: *mut i32) -> i32 {
    let interp = current_interp();
    if value.is_null() || out.is_null() || interp.is_null() {
        return TCL_VALUE_GET_ERROR;
    }
    match crate::typed_value::boolean(value) {
        Ok(parsed) => {
            // SAFETY: `out` is writable aligned storage per the contract.
            unsafe { out.write(i32::from(parsed)) };
            TCL_VALUE_GET_OK
        }
        // SAFETY: `interp` is the live current interpreter.
        Err(error) => unsafe { typed_read_error(interp, &error) },
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

/// One eval-loop activation held for the span of an ABI dispatch.
///
/// [`tcl_invoke_argv`] and [`tcl_intrinsic_invoke_argv`] *are* the compiled
/// statement as far as the interpreter can see, and today's generated code does
/// not yet bracket its statements with
/// [`tcl_codegen_activation_enter`]/[`tcl_codegen_activation_leave`], so they
/// hold the activation themselves. Without it a dispatched `catch` runs its
/// body at depth 0, the eval loop's outermost rule publishes and resets the
/// exception state inside the body, and `catch {…} r opts` reports
/// `-errorcode NONE` where interpreted Tcl reports the raised code.
///
/// Leaving on `Drop` also applies the outermost rule for a genuinely top-level
/// compiled statement — the "publish an uncaught error even though no eval loop
/// ran" behaviour these entry points already had, now expressed once as the
/// activation's tail instead of a separate publish call.
struct AbiActivation {
    interp: *mut Interp,
    code: crate::interp::Code,
}

impl AbiActivation {
    /// Enter an activation on the live current interpreter, or `None` when the
    /// native nesting bound refuses it (the interpreter then carries the
    /// catchable "too many nested evaluations" error and holds no activation).
    ///
    /// # Safety
    /// `interp` must be the live current interpreter for the whole guard's life.
    unsafe fn enter(interp: *mut Interp) -> Option<Self> {
        // SAFETY: forwarded per this function's contract.
        unsafe { (*interp).codegen_activation_enter() }.then_some(AbiActivation {
            interp,
            code: crate::interp::Code::Ok,
        })
    }

    /// Record the completion this activation ends with, before it is left.
    fn complete(&mut self, code: tcl_runtime_api::Code) {
        self.code = crate::interp::Code::from_int(i32::try_from(code.as_int()).unwrap_or(1));
    }
}

impl Drop for AbiActivation {
    fn drop(&mut self) {
        // SAFETY: the interpreter was live when the activation was entered and
        // stays live for the ABI call that holds this guard.
        unsafe { (*self.interp).codegen_activation_leave(self.code) };
    }
}

/// `tcl_codegen_activation_enter() -> i32` — make the compiled work that
/// follows count as one eval-loop activation.
///
/// The runtime's outermost-eval rule (`interp.rs`'s eval loop, at depth 0) is
/// what publishes an uncaught error's trace to `::errorInfo`/`::errorCode` and
/// drains the background-error queue. Generated code that dispatches commands
/// without entering that loop runs at depth 0, so a dispatched `catch` would
/// see the rule fire *inside* its own body — resetting the exception state
/// before `catch` reads `-errorcode`/`-errorinfo`. An activation restores what
/// interpreted Tcl always has: an enclosing activation at depth ≥ 1.
///
/// Returns `0` when the activation was entered; the caller then owes exactly
/// one [`tcl_codegen_activation_leave`] with the activation's completion code.
/// Returns non-zero when there is no current interpreter, or when the
/// activation would exceed the runtime's native nesting bound (in which case
/// the interpreter carries the same catchable "too many nested evaluations"
/// error the eval loop raises); **no** activation is held, and the caller must
/// not leave one.
#[no_mangle]
pub extern "C" fn tcl_codegen_activation_enter() -> i32 {
    let interp = current_interp();
    if interp.is_null() {
        return 1;
    }
    // SAFETY: the bootstrap installed a live current interpreter.
    i32::from(!unsafe { (*interp).codegen_activation_enter() })
}

/// `tcl_codegen_activation_leave(code)` — leave the activation entered by the
/// matching [`tcl_codegen_activation_enter`].
///
/// `code` is the activation's Tcl completion code (`0` ok, `1` error, `2`
/// return, `3` break, `4` continue, or any `return -code N` integer). Leaving
/// the outermost activation applies exactly the eval loop's tail: an error
/// publishes its accumulated trace to `::errorInfo`/`::errorCode`, and any
/// queued background errors are drained with the current handler.
#[no_mangle]
pub extern "C" fn tcl_codegen_activation_leave(code: i32) {
    let interp = current_interp();
    if interp.is_null() {
        return;
    }
    // SAFETY: the bootstrap installed a live current interpreter.
    unsafe { (*interp).codegen_activation_leave(crate::interp::Code::from_int(code)) };
}

/// Push a name-addressable Tcl frame for a generated procedure.
#[no_mangle]
pub extern "C" fn tcl_codegen_frame_push() {
    let interp = current_interp();
    if !interp.is_null() {
        // SAFETY: the bootstrap installed a live current interpreter.
        unsafe { (*interp).codegen_frame_push() };
    }
}

/// Pop the current generated procedure frame.
#[no_mangle]
pub extern "C" fn tcl_codegen_frame_pop() {
    let interp = current_interp();
    if !interp.is_null() {
        // SAFETY: the bootstrap installed a live current interpreter.
        unsafe { (*interp).codegen_frame_pop() };
    }
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
    let interp = current_interp();
    if interp.is_null() || slot < 0 || value.is_null() {
        return 1;
    }
    let name = unsafe { input_bytes(name_ptr, name_len) };
    // SAFETY: the bootstrap installed a live current interpreter.
    let interp = unsafe { &mut *interp };
    interp.codegen_bind_slot(usize::try_from(slot).unwrap_or(0), name);
    let code = match interp.var_set(name, value) {
        Ok(()) => 0,
        Err(e) => i32::try_from(crate::builtins::var_error(interp, name, e).as_int()).unwrap_or(1),
    };
    // SAFETY: generated assignment transfers its operand-stack reference.
    unsafe { obj::decr_ref_count(value) };
    code
}

/// Store through an indexed compiled-local port.
///
/// # Safety
/// `value` must be a live owned reference transferred by generated code.
#[no_mangle]
pub unsafe extern "C" fn tcl_codegen_local_set(slot: i32, value: *mut TclObj) -> i32 {
    let interp = current_interp();
    if interp.is_null() || slot < 0 || value.is_null() {
        return 1;
    }
    // SAFETY: the bootstrap installed a live current interpreter.
    let interp = unsafe { &mut *interp };
    let Some(name) = interp.codegen_slot_name(usize::try_from(slot).unwrap_or(0)) else {
        // SAFETY: generated assignment transfers its operand-stack reference.
        unsafe { obj::decr_ref_count(value) };
        return 1;
    };
    let code = match interp.var_set(&name, value) {
        Ok(()) => 0,
        Err(e) => i32::try_from(crate::builtins::var_error(interp, &name, e).as_int()).unwrap_or(1),
    };
    // SAFETY: generated assignment transfers its operand-stack reference.
    unsafe { obj::decr_ref_count(value) };
    code
}

/// Load through an indexed port, returning an owned operand-stack value.
///
/// # Safety
/// The current interpreter and generated frame must remain live for the call.
#[no_mangle]
pub unsafe extern "C" fn tcl_codegen_local_get(slot: i32) -> *mut TclObj {
    let interp = current_interp();
    if interp.is_null() || slot < 0 {
        return ptr::null_mut();
    }
    // SAFETY: the bootstrap installed a live current interpreter.
    let interp = unsafe { &mut *interp };
    let slot = usize::try_from(slot).unwrap_or(0);
    // The indexed path: one array index to the frame's cell, taken whenever
    // nothing can observe the read differently (an untraced plain scalar).
    if let Some(value) = interp.codegen_slot_scalar(slot) {
        // SAFETY: the cell owns the existing reference; the stack claims another.
        unsafe { obj::incr_ref_count(value) };
        return value;
    }
    let Some(name) = interp.codegen_slot_name(slot) else {
        return ptr::null_mut();
    };
    if interp.fire_read_trace(&name, None).is_some() {
        return ptr::null_mut();
    }
    let Some(value) = interp.var_get(&name) else {
        let msg = interp.read_miss_msg(&name, None);
        interp.set_error(&msg);
        return ptr::null_mut();
    };
    // SAFETY: the frame owns the existing reference; the stack claims another.
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
/// Returns `1` when traced, `0` when not (including a name that does not
/// resolve, and when there is no current interpreter).
///
/// # Safety
/// The name range must be readable shared linear memory.
#[no_mangle]
pub unsafe extern "C" fn tcl_codegen_var_traced(name_ptr: *const u8, name_len: i32) -> i32 {
    let interp = current_interp();
    if interp.is_null() {
        return 0;
    }
    let name = unsafe { input_bytes(name_ptr, name_len) };
    // SAFETY: the bootstrap installed a live current interpreter.
    i32::from(unsafe { (*interp).var_is_traced(name) })
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
    let interp = current_interp();
    let Ok(slot) = usize::try_from(slot) else {
        return 0;
    };
    if interp.is_null() {
        return 0;
    }
    // SAFETY: the bootstrap installed a live current interpreter.
    i32::from(unsafe { (*interp).codegen_slot_is_traced(slot) })
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
    interp: *mut Interp,
    slot: i32,
    head: &[u8],
    value: Option<*mut TclObj>,
    command: fn(&mut Interp, &[*mut TclObj]) -> crate::interp::Code,
) -> crate::interp::Code {
    // SAFETY: the caller passes the live current interpreter.
    let interp = unsafe { &mut *interp };
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
    let interp = current_interp();
    if interp.is_null() {
        return TCL_VALUE_GET_ERROR;
    }
    let delta_obj = obj::new_wide_int_obj(delta);
    // Give the fresh operand a reference of its own before handing it over:
    // `slot_modify` borrows its words and releases that borrow after the
    // call, which would take a bare `rc 0` operand back to zero and free it
    // here — leaving the release below a double free.
    // SAFETY: `delta_obj` is a fresh live object.
    unsafe { obj::incr_ref_count(delta_obj) };
    // SAFETY: `interp` is live; `delta_obj` is a fresh live object.
    let code = unsafe {
        slot_modify(
            interp,
            slot,
            b"incr",
            Some(delta_obj),
            // The trace-safe `incr` — see `cmd_var::installed_incr`.
            crate::cmd_var::installed_incr(),
        )
    };
    // SAFETY: releases exactly the reference taken above.
    unsafe { obj::decr_ref_count(delta_obj) };
    if code != crate::interp::Code::Ok {
        return TCL_VALUE_GET_ERROR;
    }
    if out.is_null() {
        return TCL_VALUE_GET_OK;
    }
    // SAFETY: `interp` is live; the result is `incr`'s new value.
    let result = unsafe { (*interp).result_obj() };
    match crate::typed_value::wide_int(result) {
        Ok(value) => {
            // SAFETY: `out` is writable aligned storage per the contract.
            unsafe { out.write(value) };
            TCL_VALUE_GET_OK
        }
        // SAFETY: `interp` is the live current interpreter.
        Err(error) => unsafe { typed_read_error(interp, &error) },
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
    let interp = current_interp();
    if interp.is_null() || value.is_null() {
        return 1;
    }
    // SAFETY: `interp` is live and `value` is caller-owned for the call.
    let code = unsafe {
        slot_modify(
            interp,
            slot,
            b"append",
            Some(value),
            crate::cmd_string::append,
        )
    };
    i32::try_from(code.as_int()).unwrap_or(1)
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
    let interp = current_interp();
    if interp.is_null() || value.is_null() {
        return 1;
    }
    // SAFETY: `interp` is live and `value` is caller-owned for the call.
    let code = unsafe {
        slot_modify(
            interp,
            slot,
            b"lappend",
            Some(value),
            crate::cmd_list::lappend,
        )
    };
    i32::try_from(code.as_int()).unwrap_or(1)
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
    let interp = current_interp();
    if interp.is_null() || value.is_null() {
        return 1;
    }
    let name = unsafe { input_bytes(name_ptr, name_len) };
    // SAFETY: the bootstrap installed a live current interpreter.
    let interp = unsafe { &mut *interp };
    let code = match interp.var_set(name, value) {
        Ok(()) => 0,
        Err(e) => i32::try_from(crate::builtins::var_error(interp, name, e).as_int()).unwrap_or(1),
    };
    // SAFETY: generated assignment transfers its operand-stack reference.
    unsafe { obj::decr_ref_count(value) };
    code
}

/// Load a top-level or namespace variable by name as an owned stack value.
///
/// # Safety
/// The name range must be readable shared linear memory.
#[no_mangle]
pub unsafe extern "C" fn tcl_codegen_var_get(name_ptr: *const u8, name_len: i32) -> *mut TclObj {
    let interp = current_interp();
    if interp.is_null() {
        return ptr::null_mut();
    }
    let name = unsafe { input_bytes(name_ptr, name_len) };
    // SAFETY: the bootstrap installed a live current interpreter.
    let interp = unsafe { &mut *interp };
    if interp.fire_read_trace(name, None).is_some() {
        return ptr::null_mut();
    }
    let Some(value) = interp.var_get(name) else {
        let msg = interp.read_miss_msg(name, None);
        interp.set_error(&msg);
        return ptr::null_mut();
    };
    // SAFETY: variable storage owns the existing reference; the stack claims one.
    unsafe { obj::incr_ref_count(value) };
    value
}

/// Read one Tcl array element as an owned generated-word value.
///
/// This is the array-element half of the compiled word-evaluation surface:
/// [`tcl_codegen_var_get`] reads a variable under its exact name (the scalar a
/// `${a(b)}` spelling names), while this reads `name(key)` as the element
/// access a `$name(key)` spelling means. The compiler already split the two,
/// so the runtime never re-parses a variable reference here.
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
    let interp = current_interp();
    if interp.is_null() {
        return ptr::null_mut();
    }
    let name = unsafe { input_bytes(name_ptr, name_len) };
    let key = unsafe { input_bytes(key_ptr, key_len) };
    // SAFETY: the bootstrap installed a live current interpreter.
    let interp = unsafe { &mut *interp };
    if interp.fire_read_trace(name, Some(key)).is_some() {
        return ptr::null_mut();
    }
    let Some(value) = interp.var_get_elem(name, key) else {
        let msg = interp.read_miss_msg(name, Some(key));
        interp.set_error(&msg);
        return ptr::null_mut();
    };
    // SAFETY: the array cell owns the existing reference; the stack claims one.
    unsafe { obj::incr_ref_count(value) };
    value
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
    let joined = if count == 0 {
        Vec::new()
    } else {
        if parts.is_null() {
            return ptr::null_mut();
        }
        // SAFETY: the caller guarantees `count` readable pointers.
        let words = unsafe { core::slice::from_raw_parts(parts, count) };
        if words.iter().any(|word| word.is_null()) {
            return ptr::null_mut();
        }
        let mut joined = Vec::new();
        for &word in words {
            joined.extend_from_slice(&obj_bytes(word));
        }
        joined
    };
    let value = new_string_bytes(&joined);
    // SAFETY: a generated operand-stack value owns one reference.
    unsafe { obj::incr_ref_count(value) };
    value
}

/// Add two owned stack values through Tcl's numeric tower.
///
/// # Safety
/// Both operands must be live references owned by generated code.
#[no_mangle]
#[cfg(have_tommath)]
pub unsafe extern "C" fn tcl_codegen_expr_add(
    left: *mut TclObj,
    right: *mut TclObj,
) -> *mut TclObj {
    if left.is_null() || right.is_null() {
        unsafe {
            obj::decr_ref_count(left);
            obj::decr_ref_count(right);
        }
        return ptr::null_mut();
    }
    let interp = current_interp();
    let result = crate::bignum::add(left, right);
    // SAFETY: the operation consumes both generated operand-stack references.
    unsafe {
        obj::decr_ref_count(left);
        obj::decr_ref_count(right);
    }
    match result {
        Ok(value) => {
            // SAFETY: transfer a single owned result to generated code.
            unsafe { obj::incr_ref_count(value) };
            value
        }
        Err(e) => {
            if !interp.is_null() {
                let err = crate::expr::arith_err(e);
                // SAFETY: the bootstrap installed a live current interpreter.
                unsafe { (*interp).set_error(&err.msg) };
            }
            ptr::null_mut()
        }
    }
}

/// Report that arithmetic is unavailable in a deliberately reduced runtime.
///
/// # Safety
/// Both operands must be live references owned by generated code.
#[no_mangle]
#[cfg(not(have_tommath))]
pub unsafe extern "C" fn tcl_codegen_expr_add(
    left: *mut TclObj,
    right: *mut TclObj,
) -> *mut TclObj {
    if left.is_null() || right.is_null() {
        unsafe {
            obj::decr_ref_count(left);
            obj::decr_ref_count(right);
        }
        return ptr::null_mut();
    }
    // SAFETY: the operation consumes both generated operand-stack references.
    unsafe {
        obj::decr_ref_count(left);
        obj::decr_ref_count(right);
    }
    let interp = current_interp();
    if !interp.is_null() {
        // SAFETY: the bootstrap installed a live current interpreter.
        unsafe { (*interp).set_error(b"arithmetic support is not available") };
    }
    ptr::null_mut()
}

/// Write one owned value to stdout using the runtime's `puts` implementation.
///
/// # Safety
/// `value` must be a live reference owned by generated code.
#[no_mangle]
pub unsafe extern "C" fn tcl_codegen_puts(value: *mut TclObj) -> i32 {
    let interp = current_interp();
    if interp.is_null() || value.is_null() {
        return 1;
    }
    let command = new_string_bytes(b"puts");
    // SAFETY: the bootstrap installed a live current interpreter.
    let code = unsafe { crate::cmd_chan::puts_cmd(&mut *interp, &[command, value]) };
    drop_fresh(command);
    // SAFETY: the command consumes the generated stack reference after dispatch.
    unsafe { obj::decr_ref_count(value) };
    i32::try_from(code.as_int()).unwrap_or(1)
}

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
    let interp = current_interp();
    if interp.is_null() {
        return 1;
    }
    let name = unsafe { input_bytes(name_ptr, name_len) };
    let params = unsafe { input_bytes(params_ptr, params_len) };
    let body = unsafe { input_bytes(body_ptr, body_len) };
    // SAFETY: the bootstrap installed a live current interpreter.
    let interp = unsafe { &mut *interp };
    let params = match crate::cmd_proc::parse_params(params) {
        Ok(params) => params,
        Err(message) => return i32::try_from(interp.set_error(&message).as_int()).unwrap_or(1),
    };
    let body_obj = new_string_bytes(body);
    interp.define_proc_native(name, params, body_obj, entry);
    drop_fresh(body_obj);
    interp.set_result_bytes(b"");
    0
}

/// Log one `while executing` / `invoked from within` `errorInfo` frame for a
/// compiled statement that completed with an error.
///
/// Generated code has no eval loop, so nothing calls `log_command_info` for
/// it: a compiled statement that fails leaves no `while executing "<source>"`
/// frame and does not advance `errorLine`, which surfaces as a stale line in
/// the enclosing `(procedure "p" line N)` and as a missing TIP 348 `CALL`
/// entry. This is the emitter's way to close both, on a statement's error
/// edge.
///
/// `line` is the statement's 1-based line **within the body it was compiled
/// from** — the same raw line the eval loop reads off the script text — so the
/// enclosing frame's `line_base`/`proc_line_base` turn it into `errorLine`
/// exactly as they do for an interpreted command. `src` is the statement's
/// exact source text; the runtime applies C's 150-byte truncation to it.
///
/// The runtime owns the `already_logged` protocol, so a statement whose error
/// was already logged deeper in the same body is a no-op here and the emitter
/// has no decision to make. Hence no result.
///
/// # Safety
/// `src_ptr`/`src_len` must be a readable range of shared linear memory.
#[no_mangle]
pub unsafe extern "C" fn tcl_codegen_log_command(line: i32, src_ptr: *const u8, src_len: i32) {
    let interp = current_interp();
    if interp.is_null() {
        return;
    }
    let src = unsafe { input_bytes(src_ptr, src_len) };
    // Lines are 1-based; a caller that has no line still gets a logged frame
    // rather than a silently dropped one.
    let line = u32::try_from(line).unwrap_or(1).max(1);
    // SAFETY: the bootstrap installed a live current interpreter.
    unsafe { (*interp).log_command_bytes(line, src) };
}

/// Record the pending `return -level`/`-code` state, exactly as the `return`
/// command records it.
///
/// A compiled `return` completes with code `2` without dispatching the `return`
/// command, so without this nothing writes the state and the procedure's return
/// boundary ([`Interp::settle_return`]) consumes whatever an earlier
/// `return -level N` left behind — a caught `return -level 2` anywhere ahead of
/// the call would make the procedure propagate code `2`, or a stale requested
/// code, instead of returning its value. The emitter therefore writes
/// `(level = 1, code = Ok)` at every compiled plain `return`, which is exactly
/// what `return`'s own implementation writes for that form.
///
/// An out-of-range `level` is clamped at zero and an unknown `code` becomes an
/// error code, the same way the `return` command's own parsing bounds them.
#[no_mangle]
pub extern "C" fn tcl_codegen_return_state(level: i32, code: i32) {
    let interp = current_interp();
    if interp.is_null() {
        return;
    }
    let level = usize::try_from(level).unwrap_or(0);
    // SAFETY: the bootstrap installed a live current interpreter.
    unsafe { (*interp).set_return_state(level, crate::interp::Code::from_int(code)) };
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
    let interp = current_interp();
    if interp.is_null() {
        return -1;
    }
    // SAFETY: the bootstrap installed a live current interpreter.
    let dispatches = unsafe { (*interp).native_proc_dispatches() };
    i32::try_from(dispatches).unwrap_or(i32::MAX)
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
/// enclosing loop — faithful abrupt-completion propagation.
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

#[derive(Debug)]
struct ResolvedIntrinsicArgv {
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
fn resolve_intrinsic_argv(
    words: &[*mut TclObj],
    dialect: Option<SurfaceQuery<'_>>,
) -> Option<ResolvedIntrinsicArgv> {
    let spellings: Vec<String> = words
        .iter()
        .map(|word| String::from_utf8(obj_bytes(*word)).ok())
        .collect::<Option<_>>()?;
    let (head, arguments) = spellings.split_first()?;
    let arguments: Vec<_> = arguments.iter().map(String::as_str).collect();
    let resolved = intrinsic_registry().resolve_invocation(head, &arguments, dialect)?;
    let SemanticOperationId::Intrinsic(intrinsic) = resolved.semantics.operation else {
        return None;
    };
    Some(ResolvedIntrinsicArgv {
        intrinsic,
        argument_offset: resolved.semantics.argument_offset,
        head: head.as_bytes().to_vec(),
    })
}

fn guarded_intrinsic_request(
    intrinsic_id: u32,
    words: &[*mut TclObj],
    expected: GuardIdentity,
    runtime_version: tcl_dialect::TclVersion,
    dialect: Option<SurfaceQuery<'_>>,
) -> Option<ResolvedIntrinsicArgv> {
    let intrinsic = IntrinsicId::from_stable_id(intrinsic_id)?;
    if expected
        != GuardIdentity::registry_intrinsic_with_semantics(
            intrinsic.stable_id(),
            intrinsic.guard_semantics_key(runtime_version),
        )
    {
        return None;
    }
    let resolved = resolve_intrinsic_argv(words, dialect)?;
    (resolved.intrinsic == intrinsic).then_some(resolved)
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
    // SAFETY: the caller contract guarantees `argc` readable entries.
    let words = unsafe { core::slice::from_raw_parts(argv, argc) };
    if words.iter().any(|word| word.is_null()) {
        return 0;
    }
    let Some(domains) = guard_domains_from_abi(domains) else {
        return 0;
    };
    let expected = GuardIdentity::new(identity_namespace, identity_value);
    let interp = current_interp();
    if interp.is_null() {
        return 0;
    }
    // Runtime version is interpreter policy, not a compile-time constant. The
    // Interpreter guard domain makes a later policy change stale this token,
    // while resolving against the live environment prevents a version-gated
    // form from entering the fast path in the first place. The release name
    // resolves through the one ingress seam (ledger row C2 — this file held
    // the backends' last raw `DialectProfile::find` ingresses), fail-closed: an
    // undeclared name declines rather than entering the guarded path under
    // the lenient environment's permissive mask.
    let runtime_version = unsafe { (*interp).runtime_version() };
    let Some(dialect) =
        crate::environment::known_surface_point_for_dialect(runtime_version.dialect_profile_name())
    else {
        return 0;
    };
    let Some(resolved) = guarded_intrinsic_request(
        intrinsic_id,
        words,
        expected,
        runtime_version,
        Some(dialect.query()),
    ) else {
        return 0;
    };
    // SAFETY: every word was validated non-null and is caller-owned.
    let _borrowed = unsafe { BorrowedArgv::retain(words) };
    // SAFETY: `interp` is the live current interpreter.
    unsafe {
        (*interp)
            .prepare_command_guard(&resolved.head, expected, domains)
            .map_or(0, GuardToken::raw)
    }
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
    // SAFETY: the caller contract guarantees `argc` readable entries.
    let words = unsafe { core::slice::from_raw_parts(argv, argc) };
    if words.iter().any(|word| word.is_null()) {
        return 0;
    }
    let Some(intrinsic) = IntrinsicId::from_stable_id(intrinsic_id) else {
        return 0;
    };
    let interp = current_interp();
    if interp.is_null() {
        return 0;
    }
    let runtime_version = unsafe { (*interp).runtime_version() };
    let expected = GuardIdentity::registry_intrinsic_with_semantics(
        intrinsic.stable_id(),
        intrinsic.guard_semantics_key(runtime_version),
    );
    let Some(dialect) =
        crate::environment::known_surface_point_for_dialect(runtime_version.dialect_profile_name())
    else {
        return 0;
    };
    let Some(resolved) = guarded_intrinsic_request(
        intrinsic_id,
        words,
        expected,
        runtime_version,
        Some(dialect.query()),
    ) else {
        return 0;
    };
    // SAFETY: every word was validated non-null and is caller-owned.
    let _borrowed = unsafe { BorrowedArgv::retain(words) };
    // SAFETY: `interp` is the live current interpreter.
    i32::from(unsafe {
        (*interp).check_command_guard_identity(
            GuardToken::from_raw(token),
            &resolved.head,
            expected,
        )
    })
}

/// `tcl_codegen_guard_release(token)` — release a token exactly once.
#[no_mangle]
pub extern "C" fn tcl_codegen_guard_release(token: u64) {
    let interp = current_interp();
    if !interp.is_null() {
        // SAFETY: `interp` is the live current interpreter.
        unsafe {
            let _ = (*interp).release_command_guard(GuardToken::from_raw(token));
        }
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
    let interp = current_interp();
    if interp.is_null() {
        // SAFETY: `out` was checked above and meets this function's contract.
        unsafe {
            write_completion(
                out,
                detached_error_completion(b"no current interpreter for tcl_intrinsic_invoke_argv"),
            )
        };
        return TCL_INVOKE_ABI_NO_CURRENT_INTERP;
    }
    let argc = match usize::try_from(argc) {
        Ok(argc) if argc > 0 => argc,
        _ => return TCL_INTRINSIC_ABI_DECLINED,
    };
    if argv.is_null() {
        return TCL_INTRINSIC_ABI_DECLINED;
    }
    // SAFETY: the caller contract guarantees `argc` readable entries.
    let words = unsafe { core::slice::from_raw_parts(argv, argc) };
    if words.iter().any(|word| word.is_null()) {
        return TCL_INTRINSIC_ABI_DECLINED;
    }
    let Some(intrinsic) = IntrinsicId::from_stable_id(intrinsic_id) else {
        return TCL_INTRINSIC_ABI_DECLINED;
    };
    let Some(dialect) = crate::environment::known_surface_point_for_dialect(unsafe {
        (*interp).runtime_version().dialect_profile_name()
    }) else {
        return TCL_INTRINSIC_ABI_DECLINED;
    };
    let Some(resolved) = resolve_intrinsic_argv(words, Some(dialect.query())) else {
        return TCL_INTRINSIC_ABI_DECLINED;
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
    // SAFETY: every word was validated non-null and is caller-owned.
    let _borrowed = unsafe { BorrowedArgv::retain(words) };
    // An intrinsic reached from generated code is an activation for the same
    // reason a dispatched command is (see [`AbiActivation`]): the implementation
    // may evaluate a body, and this call is the only enclosing activation there
    // is.
    // SAFETY: the current interpreter is live for this ABI call.
    let Some(mut activation) = (unsafe { AbiActivation::enter(interp) }) else {
        // SAFETY: current interp and `out` are live per the function contract.
        unsafe {
            write_completion(
                out,
                completion_abi(crate::state_traits::capture_completion(
                    &mut *interp,
                    crate::interp::Code::Error,
                )),
            );
        }
        return TCL_INVOKE_ABI_OK;
    };
    // SAFETY: `interp` is live; direct execution only borrows `args`.
    let Some(code) = (unsafe { (*interp).execute_intrinsic(intrinsic, args) }) else {
        return TCL_INTRINSIC_ABI_DECLINED;
    };
    // SAFETY: `interp` is live and `out` is caller-provided writable storage.
    let completion = unsafe { crate::state_traits::capture_completion(&mut *interp, code) };
    activation.complete(completion.code);
    drop(activation);
    // SAFETY: `out` is caller-provided writable storage.
    unsafe { write_completion(out, completion_abi(completion)) };
    TCL_INVOKE_ABI_OK
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
/// reports a malformed boundary request. When `out` is valid, every status
/// except [`TCL_INVOKE_ABI_NULL_OUT`] still writes an owned error completion so
/// the caller has a uniform release path.
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

    let interp = current_interp();
    if interp.is_null() {
        // SAFETY: `out` was checked above and meets this function's contract.
        unsafe {
            write_completion(
                out,
                detached_error_completion(b"no current interpreter for tcl_invoke_argv"),
            )
        };
        return TCL_INVOKE_ABI_NO_CURRENT_INTERP;
    }

    let argc = match usize::try_from(argc) {
        Ok(argc) if argc > 0 => argc,
        _ => {
            // SAFETY: current interp and `out` are live per the function contract.
            unsafe {
                let code = (*interp).set_error(b"tcl_invoke_argv requires a command head");
                write_completion(
                    out,
                    completion_abi(crate::state_traits::capture_completion(&mut *interp, code)),
                );
            }
            return TCL_INVOKE_ABI_INVALID_ARGC;
        }
    };
    if argv.is_null() {
        // SAFETY: current interp and `out` are live per the function contract.
        unsafe {
            let code = (*interp).set_error(b"tcl_invoke_argv received a null argv");
            write_completion(
                out,
                completion_abi(crate::state_traits::capture_completion(&mut *interp, code)),
            );
        }
        return TCL_INVOKE_ABI_NULL_ARGV;
    }

    // SAFETY: caller guarantees `argv` references `argc` readable pointers.
    let words = unsafe { core::slice::from_raw_parts(argv, argc) };
    if words.iter().any(|word| word.is_null()) {
        // SAFETY: current interp and `out` are live per the function contract.
        unsafe {
            let code = (*interp).set_error(b"tcl_invoke_argv received a null word");
            write_completion(
                out,
                completion_abi(crate::state_traits::capture_completion(&mut *interp, code)),
            );
        }
        return TCL_INVOKE_ABI_NULL_WORD;
    }

    // Keep the caller's borrowed words alive through dispatch. `Drop` restores
    // their exact pre-call reference counts after the completion has captured
    // independent result/options references.
    // SAFETY: null words were rejected; the caller guarantees every word is
    // live and holds an owned reference for this call.
    let _borrowed = unsafe { BorrowedArgv::retain(words) };
    // Dispatching directly skips the eval loop, so this call *is* the enclosing
    // activation: it must count as one while the dispatched command runs (a
    // `catch` body would otherwise trip the outermost-eval rule and lose its
    // `-errorcode`), and it applies that rule itself on the way out — which is
    // what publishes an uncaught error's trace to `::errorInfo`/`::errorCode`.
    // SAFETY: the current interpreter is live for this ABI call.
    let Some(mut activation) = (unsafe { AbiActivation::enter(interp) }) else {
        // SAFETY: current interp and `out` are live per the function contract.
        unsafe {
            write_completion(
                out,
                completion_abi(crate::state_traits::capture_completion(
                    &mut *interp,
                    crate::interp::Code::Error,
                )),
            );
        }
        return TCL_INVOKE_ABI_OK;
    };
    // SAFETY: the current interpreter is live for this ABI call.
    let completion = unsafe { crate::state_traits::dispatch_prebuilt_argv(&mut *interp, words) };
    activation.complete(completion.code);
    drop(activation);
    // SAFETY: `out` is live and properly aligned per this function's contract.
    unsafe { write_completion(out, completion_abi(completion)) };
    TCL_INVOKE_ABI_OK
}

/// Release both owned object references in a [`TclCompletionAbi`] and reset its
/// fields to null/zero. This is an alternative to releasing `result` and
/// `options` separately with [`tcl_obj_release`]; do not use both forms.
///
/// Resetting the fields deliberately makes a repeated call on the *same output
/// storage* idempotent. It does not make mixed release styles safe: after a
/// caller releases either non-null handle separately, it must not call this
/// function unless it first clears that field.
///
/// # Safety
/// `completion` must be null or writable completion storage whose two object
/// handles are still owned by the caller and have not already been released.
#[no_mangle]
pub unsafe extern "C" fn tcl_completion_release(completion: *mut TclCompletionAbi) {
    if completion.is_null() {
        return;
    }
    // SAFETY: caller guarantees writable, valid completion storage.
    let completion = unsafe { &mut *completion };
    // SAFETY: the two handles are caller-owned by this function's contract.
    unsafe {
        obj::decr_ref_count(completion.result);
        obj::decr_ref_count(completion.options);
    }
    completion.code = 0;
    completion.result = ptr::null_mut();
    completion.options = ptr::null_mut();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capi::{tcl_runtime_create_interp, tcl_runtime_delete_interp};
    use crate::counters;
    use crate::interp::Code;
    use std::cell::RefCell;
    use tcl_runtime_api::codegen_abi::{NATIVE_PROC_STATUS_DECLINED, NATIVE_PROC_STATUS_RAN};
    use tcl_runtime_api::guard::GuardDomain;

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

    #[cfg(have_tommath)]
    unsafe fn owned_str(s: &[u8]) -> *mut TclObj {
        // SAFETY: `s` is a valid readable slice.
        unsafe { tcl_value_new_string(s.as_ptr(), s.len() as i32) }
    }

    /// Box one borrowed argv word and take the caller's owning reference.
    unsafe fn owned_word(s: &[u8]) -> *mut TclObj {
        // SAFETY: `box_str` copies valid bytes into a fresh object.
        let word = unsafe { box_str(s) };
        // SAFETY: the test owns the fresh object and keeps this +1 until it
        // releases the argv after invoking.
        unsafe { obj::incr_ref_count(word) };
        word
    }

    fn empty_completion() -> TclCompletionAbi {
        TclCompletionAbi {
            code: 0,
            result: ptr::null_mut(),
            options: ptr::null_mut(),
        }
    }

    unsafe fn invoke(words: &[*mut TclObj]) -> TclCompletionAbi {
        let mut completion = empty_completion();
        // SAFETY: `words` is a Rust slice of live caller-owned object handles;
        // completion storage is local and correctly aligned.
        assert_eq!(
            unsafe {
                tcl_invoke_argv(
                    words.as_ptr(),
                    i32::try_from(words.len()).expect("test argv fits i32"),
                    &mut completion,
                )
            },
            TCL_INVOKE_ABI_OK
        );
        completion
    }

    unsafe fn invoke_intrinsic(
        intrinsic: IntrinsicId,
        words: &[*mut TclObj],
    ) -> (i32, TclCompletionAbi) {
        let mut completion = empty_completion();
        // SAFETY: `words` is a Rust slice of live caller-owned object handles;
        // completion storage is local and correctly aligned.
        let status = unsafe {
            tcl_intrinsic_invoke_argv(
                intrinsic.stable_id(),
                words.as_ptr(),
                i32::try_from(words.len()).expect("test argv fits i32"),
                &mut completion,
            )
        };
        (status, completion)
    }

    unsafe fn prepare_intrinsic_guard(
        intrinsic: IntrinsicId,
        words: &[*mut TclObj],
        domains: i32,
    ) -> u64 {
        let interp = current_interp();
        assert!(
            !interp.is_null(),
            "test helper requires a current interpreter"
        );
        // SAFETY: tests install a live current interpreter before using this helper.
        let runtime_version = unsafe { (*interp).runtime_version() };
        let identity = GuardIdentity::registry_intrinsic_with_semantics(
            intrinsic.stable_id(),
            intrinsic.guard_semantics_key(runtime_version),
        );
        // SAFETY: `words` is a Rust slice of live caller-owned object handles.
        unsafe {
            tcl_codegen_guard_prepare(
                intrinsic.stable_id(),
                words.as_ptr(),
                i32::try_from(words.len()).expect("test argv fits i32"),
                identity.namespace(),
                identity.value(),
                domains,
            )
        }
    }

    unsafe fn release_words(words: &[*mut TclObj]) {
        for &word in words {
            // SAFETY: balances `owned_word`'s caller-owned reference.
            unsafe { obj::decr_ref_count(word) };
        }
    }

    fn option(completion: &TclCompletionAbi, key: &[u8]) -> Vec<u8> {
        let value = crate::dict::dict_get(completion.options, key)
            .expect("completion options are a dict")
            .expect("expected completion option");
        obj_bytes(value)
    }

    #[test]
    fn native_wide_int_value_is_owned_and_keeps_its_internal_rep_after_shimmering() {
        leak_free(|| unsafe {
            let value = tcl_value_new_wide_int(i64::MIN);
            assert!(!value.is_null());
            assert_eq!((*value).ref_count, 1, "generated code owns the result");
            assert!(std::ptr::eq((*value).type_ptr, &obj::TCL_INT_TYPE));
            assert!((*value).bytes.is_null(), "wide integers begin without text");
            assert_eq!(obj_bytes(value), i64::MIN.to_string().as_bytes());
            assert!(
                std::ptr::eq((*value).type_ptr, &obj::TCL_INT_TYPE),
                "string materialisation must retain the wide internal representation"
            );
            assert_eq!((*value).internal_rep as i64, i64::MIN);
            tcl_obj_release(value);
        });
    }

    #[test]
    fn guarded_intrinsic_string_length_hits_and_releases_exact_completion_and_token() {
        leak_free(|| unsafe {
            let interp = tcl_runtime_create_interp();
            tcl_runtime_set_current_interp(interp);
            let words = [
                owned_word(b"string"),
                owned_word(b"length"),
                owned_word(b"abc"),
            ];
            let counts = words.map(|word| (*word).ref_count);
            let domains = i32::from(GuardDomains::one(GuardDomain::CommandEnvironment).bits());
            let token = prepare_intrinsic_guard(IntrinsicId::StringLength, &words, domains);
            assert_ne!(token, 0);
            assert_eq!(
                tcl_codegen_guard_check(
                    token,
                    IntrinsicId::StringLength.stable_id(),
                    words.as_ptr(),
                    i32::try_from(words.len()).unwrap(),
                ),
                1
            );

            let (status, mut completion) = invoke_intrinsic(IntrinsicId::StringLength, &words);
            assert_eq!(status, TCL_INVOKE_ABI_OK);
            assert_eq!(completion.code, 0);
            assert_eq!(obj_bytes(completion.result), b"3");
            assert_eq!(option(&completion, b"-code"), b"0");
            assert_eq!(words.map(|word| (*word).ref_count), counts);

            tcl_completion_release(&mut completion);
            assert!(completion.result.is_null());
            assert!(completion.options.is_null());
            tcl_codegen_guard_release(token);
            assert_eq!(
                tcl_codegen_guard_check(
                    token,
                    IntrinsicId::StringLength.stable_id(),
                    words.as_ptr(),
                    i32::try_from(words.len()).unwrap(),
                ),
                0
            );
            release_words(&words);
            tcl_runtime_set_current_interp(ptr::null_mut());
            tcl_runtime_delete_interp(interp);
        });
    }

    #[test]
    fn guarded_intrinsic_string_length_identity_and_count_follow_runtime_version() {
        leak_free(|| unsafe {
            let interp = tcl_runtime_create_interp();
            tcl_runtime_set_current_interp(interp);
            let words = [
                owned_word(b"string"),
                owned_word(b"length"),
                owned_word("é🙂".as_bytes()),
            ];

            let (status, mut completion) = invoke_intrinsic(IntrinsicId::StringLength, &words);
            assert_eq!(status, TCL_INVOKE_ABI_OK);
            assert_eq!(completion.code, 0);
            assert_eq!(obj_bytes(completion.result), b"2");
            tcl_completion_release(&mut completion);

            let tcl9_identity = GuardIdentity::registry_intrinsic_with_semantics(
                IntrinsicId::StringLength.stable_id(),
                IntrinsicId::StringLength.guard_semantics_key(tcl_dialect::TclVersion::V9_0),
            );
            (*interp).set_runtime_version(tcl_dialect::TclVersion::V8_6);
            let domains = i32::from(
                GuardDomains::one(GuardDomain::CommandEnvironment)
                    .with(GuardDomain::Namespace)
                    .with(GuardDomain::CommandTrace)
                    .with(GuardDomain::Interpreter)
                    .bits(),
            );
            assert_eq!(
                tcl_codegen_guard_prepare(
                    IntrinsicId::StringLength.stable_id(),
                    words.as_ptr(),
                    i32::try_from(words.len()).unwrap(),
                    tcl9_identity.namespace(),
                    tcl9_identity.value(),
                    domains,
                ),
                0,
                "a Tcl 9 scalar-count identity must not enter a Tcl 8 runtime",
            );
            let (status, mut completion) = invoke_intrinsic(IntrinsicId::StringLength, &words);
            assert_eq!(status, TCL_INVOKE_ABI_OK);
            assert_eq!(obj_bytes(completion.result), b"3");
            tcl_completion_release(&mut completion);
            release_words(&words);
            tcl_runtime_set_current_interp(ptr::null_mut());
            tcl_runtime_delete_interp(interp);
        });
    }

    #[test]
    fn guarded_intrinsic_unsupported_form_declines_without_completion_or_source_replay() {
        leak_free(|| unsafe {
            let interp = tcl_runtime_create_interp();
            tcl_runtime_set_current_interp(interp);
            let words = [owned_word(b"llength"), owned_word(b"a b c")];
            let (status, completion) = invoke_intrinsic(IntrinsicId::ListLength, &words);

            assert_eq!(status, TCL_INTRINSIC_ABI_DECLINED);
            assert_eq!(completion.code, 0);
            assert!(completion.result.is_null());
            assert!(completion.options.is_null());
            release_words(&words);
            tcl_runtime_set_current_interp(ptr::null_mut());
            tcl_runtime_delete_interp(interp);
        });
    }

    #[test]
    fn guarded_intrinsic_guards_fail_after_rename_or_trace_registration() {
        leak_free(|| unsafe {
            let interp = tcl_runtime_create_interp();
            tcl_runtime_set_current_interp(interp);
            let words = [
                owned_word(b"string"),
                owned_word(b"length"),
                owned_word(b"abc"),
            ];
            let command_domains =
                i32::from(GuardDomains::one(GuardDomain::CommandEnvironment).bits());
            let token = prepare_intrinsic_guard(IntrinsicId::StringLength, &words, command_domains);
            assert_ne!(token, 0);
            assert_eq!(tcl_eval_code(box_str(b"rename string string2")), 0);
            assert_eq!(
                tcl_codegen_guard_check(
                    token,
                    IntrinsicId::StringLength.stable_id(),
                    words.as_ptr(),
                    i32::try_from(words.len()).unwrap(),
                ),
                0
            );
            tcl_codegen_guard_release(token);
            release_words(&words);
            tcl_runtime_set_current_interp(ptr::null_mut());
            tcl_runtime_delete_interp(interp);
        });

        leak_free(|| unsafe {
            let interp = tcl_runtime_create_interp();
            tcl_runtime_set_current_interp(interp);
            assert_eq!(
                tcl_eval_code(box_str(b"trace add command string rename callback")),
                0
            );
            let words = [
                owned_word(b"string"),
                owned_word(b"length"),
                owned_word(b"abc"),
            ];
            let trace_domains = i32::from(GuardDomains::one(GuardDomain::CommandTrace).bits());
            assert_eq!(
                prepare_intrinsic_guard(IntrinsicId::StringLength, &words, trace_domains),
                0
            );
            release_words(&words);
            tcl_runtime_set_current_interp(ptr::null_mut());
            tcl_runtime_delete_interp(interp);
        });
    }

    #[test]
    #[cfg(have_tommath)]
    fn compiled_slots_and_named_access_share_one_cell() {
        leak_free(|| unsafe {
            let interp = tcl_runtime_create_interp();
            tcl_runtime_set_current_interp(interp);
            tcl_codegen_frame_push();

            assert_eq!(
                tcl_codegen_local_bind(0, b"b".as_ptr(), 1, owned_str(b"2")),
                0
            );
            assert_eq!(
                tcl_codegen_local_bind(1, b"c".as_ptr(), 1, owned_str(b"4")),
                0
            );
            assert_eq!(tcl_eval_code(box_str(b"set b 5")), 0);

            let sum = tcl_codegen_expr_add(tcl_codegen_local_get(0), tcl_codegen_local_get(1));
            assert_eq!(obj_bytes(sum), b"9");
            tcl_value_release(sum);

            tcl_codegen_frame_pop();
            tcl_runtime_set_current_interp(ptr::null_mut());
            tcl_runtime_delete_interp(interp);
        });
    }

    /// The read-modify-write slot ABI is the runtime's own `incr` / `append` /
    /// `lappend` addressed by index: full Tcl semantics, including the bignum
    /// promotion `incr` gets from the numeric tower, and a by-name view of the
    /// very same cell.
    #[cfg(have_tommath)]
    #[test]
    fn slot_read_modify_write_runs_the_real_commands_on_the_same_cell() {
        leak_free(|| unsafe {
            let interp = tcl_runtime_create_interp();
            tcl_runtime_set_current_interp(interp);
            tcl_codegen_frame_push();

            assert_eq!(
                tcl_codegen_slot_bind(0, b"n".as_ptr(), 1, owned_str(b"1")),
                0
            );
            let mut value: i64 = 0;
            assert_eq!(
                tcl_codegen_slot_incr_i64(0, 41, &mut value),
                TCL_VALUE_GET_OK
            );
            assert_eq!(value, 42);
            // The named view sees the indexed write, and vice versa.
            assert_eq!(tcl_eval_code(box_str(b"set n")), 0);
            assert_eq!((*interp).result_bytes(), b"42");
            assert_eq!(tcl_eval_code(box_str(b"incr n")), 0);
            let indexed = tcl_codegen_slot_get(0);
            assert_eq!(obj_bytes(indexed), b"43");
            tcl_value_release(indexed);

            // `incr` promotes past the wide range instead of wrapping, and the
            // i64 read of that new value reports C's overflow error.
            assert_eq!(tcl_eval_code(box_str(b"set n 9223372036854775807")), 0);
            assert_eq!(
                tcl_codegen_slot_incr_i64(0, 1, &mut value),
                TCL_VALUE_GET_ERROR
            );
            assert_eq!(
                (*interp).result_bytes(),
                b"integer value too large to represent"
            );
            assert_eq!(tcl_eval_code(box_str(b"set n")), 0);
            assert_eq!((*interp).result_bytes(), b"9223372036854775808");

            // `incr` on a non-integer cell keeps C's message.
            assert_eq!(tcl_eval_code(box_str(b"set n abc")), 0);
            assert_eq!(
                tcl_codegen_slot_incr_i64(0, 1, &mut value),
                TCL_VALUE_GET_ERROR
            );
            assert_eq!(
                (*interp).result_bytes(),
                b"expected integer but got \"abc\""
            );

            // `append` / `lappend` through the slot, seen by name.
            assert_eq!(
                tcl_codegen_slot_bind(1, b"s".as_ptr(), 1, owned_str(b"a")),
                0
            );
            let piece = owned_str(b"bc");
            assert_eq!(tcl_codegen_slot_append(1, piece), 0);
            tcl_value_release(piece);
            assert_eq!(tcl_eval_code(box_str(b"set s")), 0);
            assert_eq!((*interp).result_bytes(), b"abc");

            assert_eq!(
                tcl_codegen_slot_bind(2, b"l".as_ptr(), 1, owned_str(b"x")),
                0
            );
            let item = owned_str(b"y z");
            assert_eq!(tcl_codegen_slot_lappend(2, item), 0);
            tcl_value_release(item);
            assert_eq!(tcl_eval_code(box_str(b"llength $l")), 0);
            assert_eq!((*interp).result_bytes(), b"2");
            assert_eq!(tcl_eval_code(box_str(b"lindex $l 1")), 0);
            assert_eq!((*interp).result_bytes(), b"y z");

            tcl_codegen_frame_pop();
            tcl_runtime_set_current_interp(ptr::null_mut());
            tcl_runtime_delete_interp(interp);
        });
    }

    /// An indexed slot keeps addressing one cell across `unset` and re-creation,
    /// and a variable trace, an `upvar` link, and `info locals` all reach that
    /// same cell — the indexed path must never become a second variable.
    #[cfg(have_tommath)]
    #[test]
    fn an_indexed_slot_survives_unset_and_stays_the_named_cell() {
        leak_free(|| unsafe {
            let interp = tcl_runtime_create_interp();
            tcl_runtime_set_current_interp(interp);
            tcl_codegen_frame_push();

            assert_eq!(
                tcl_codegen_slot_bind(0, b"v".as_ptr(), 1, owned_str(b"first")),
                0
            );
            assert_eq!(tcl_eval_code(box_str(b"unset v")), 0);
            assert!(
                tcl_codegen_slot_get(0).is_null(),
                "an unset cell reads as a missing variable"
            );
            assert_eq!(tcl_eval_code(box_str(b"set v second")), 0);
            let value = tcl_codegen_slot_get(0);
            assert_eq!(
                obj_bytes(value),
                b"second",
                "the re-created variable refills the same cell"
            );
            tcl_value_release(value);

            // A read trace observes the indexed read: the fast path declines
            // whenever anything can see the difference.
            assert_eq!(
                tcl_eval_code(box_str(
                    b"set ::hits 0; trace add variable v read {apply {{a b op} {incr ::hits}}}"
                )),
                0
            );
            let traced = tcl_codegen_slot_get(0);
            assert!(!traced.is_null());
            tcl_value_release(traced);
            assert_eq!(tcl_eval_code(box_str(b"set ::hits")), 0);
            assert_eq!((*interp).result_bytes(), b"1");

            tcl_codegen_frame_pop();
            tcl_runtime_set_current_interp(ptr::null_mut());
            tcl_runtime_delete_interp(interp);
        });
    }

    /// Ask the trace barrier about a name. Only the `have_tommath` tests
    /// below reach it.
    #[cfg(have_tommath)]
    unsafe fn var_traced(name: &[u8]) -> i32 {
        // SAFETY: `name` is a valid readable slice.
        unsafe { tcl_codegen_var_traced(name.as_ptr(), name.len() as i32) }
    }

    /// The per-cell trace bit answers for a name and for a slot, follows an
    /// `upvar` link to its target's traces, covers an array's elements, and —
    /// the case a hand-maintained bit would get wrong — goes back to `0` when a
    /// proc frame's teardown drops its locals' traces.
    #[cfg(have_tommath)]
    #[test]
    fn the_trace_barrier_tracks_adds_removes_links_and_frame_teardown() {
        leak_free(|| unsafe {
            let interp = tcl_runtime_create_interp();
            tcl_runtime_set_current_interp(interp);

            assert_eq!(tcl_eval_code(box_str(b"set g 1; set ::hits 0")), 0);
            assert_eq!(var_traced(b"g"), 0, "an untraced global");

            assert_eq!(
                tcl_eval_code(box_str(
                    b"trace add variable g write {apply {{a b op} {incr ::hits}}}"
                )),
                0
            );
            assert_eq!(var_traced(b"g"), 1);
            assert_eq!(var_traced(b"other"), 0, "the bit is per cell, not global");

            // An array reports traced when any element is: the guard is
            // conservative in the safe direction.
            assert_eq!(tcl_eval_code(box_str(b"set arr(k) 1")), 0);
            assert_eq!(var_traced(b"arr"), 0);
            assert_eq!(
                tcl_eval_code(box_str(
                    b"trace add variable arr(k) write {apply {{a b op} {incr ::hits}}}"
                )),
                0
            );
            assert_eq!(var_traced(b"arr"), 1);
            assert_eq!(var_traced(b"arr(k)"), 1);

            assert_eq!(
                tcl_eval_code(box_str(
                    b"trace remove variable g write {apply {{a b op} {incr ::hits}}}"
                )),
                0
            );
            assert_eq!(var_traced(b"g"), 0, "removing the last trace clears it");

            // A trace added inside a proc frame is visible through the local
            // name and through an `upvar` alias to it, and both answers go back
            // to untraced when the frame is popped.
            assert_eq!(
                tcl_eval_code(box_str(
                    b"proc probe {} {\n\
                        set loc 1\n\
                        upvar 0 loc alias\n\
                        set ::before [tcltrace loc],[tcltrace alias]\n\
                        trace add variable loc write {apply {{a b op} {incr ::hits}}}\n\
                        set ::during [tcltrace loc],[tcltrace alias]\n\
                      }"
                )),
                0
            );
            // A tiny Tcl-visible probe over the same ABI entry point.
            assert_eq!(
                tcl_eval_code(box_str(
                    b"proc tcltrace {n} { uplevel 1 [list ::tcl::probe_traced $n] }"
                )),
                0
            );
            (*interp).register_builtin(b"::tcl::probe_traced", |interp, argv| {
                let name = obj_bytes(argv[1]);
                let traced = interp.var_is_traced(&name);
                interp.set_result_bytes(if traced { b"1" } else { b"0" });
                crate::interp::Code::Ok
            });

            assert_eq!(tcl_eval_code(box_str(b"probe")), 0);
            assert_eq!(tcl_eval_code(box_str(b"set ::before")), 0);
            assert_eq!((*interp).result_bytes(), b"0,0");
            assert_eq!(tcl_eval_code(box_str(b"set ::during")), 0);
            assert_eq!(
                (*interp).result_bytes(),
                b"1,1",
                "an upvar alias reports its target's traces"
            );
            // The frame is gone, so its local's trace is too.
            assert_eq!(var_traced(b"loc"), 0);

            tcl_runtime_set_current_interp(ptr::null_mut());
            tcl_runtime_delete_interp(interp);
        });
    }

    /// The slot spelling answers from the cell's own bit, and that bit is
    /// re-derived — never trusted — after the trace set changes.
    #[cfg(have_tommath)]
    #[test]
    fn the_slot_trace_bit_is_revalidated_when_the_trace_set_changes() {
        leak_free(|| unsafe {
            let interp = tcl_runtime_create_interp();
            tcl_runtime_set_current_interp(interp);
            tcl_codegen_frame_push();

            assert_eq!(
                tcl_codegen_slot_bind(0, b"v".as_ptr(), 1, owned_str(b"1")),
                0
            );
            assert_eq!(tcl_codegen_slot_traced(0), 0);
            // Asked twice: the second answer comes from the cached bit.
            assert_eq!(tcl_codegen_slot_traced(0), 0);

            assert_eq!(tcl_eval_code(box_str(b"set ::hits 0")), 0);
            assert_eq!(
                tcl_eval_code(box_str(
                    b"trace add variable v write {apply {{a b op} {incr ::hits}}}"
                )),
                0
            );
            assert_eq!(
                tcl_codegen_slot_traced(0),
                1,
                "the epoch moved, so the cached 0 was recomputed"
            );
            assert_eq!(tcl_codegen_slot_traced(0), 1);

            assert_eq!(
                tcl_eval_code(box_str(
                    b"trace remove variable v write {apply {{a b op} {incr ::hits}}}"
                )),
                0
            );
            assert_eq!(tcl_codegen_slot_traced(0), 0);
            assert_eq!(tcl_codegen_slot_traced(9), 0, "an unbound slot is untraced");

            tcl_codegen_frame_pop();
            tcl_runtime_set_current_interp(ptr::null_mut());
            tcl_runtime_delete_interp(interp);
        });
    }

    /// The prebuilt argv boundary reaches the normal builtin dispatcher and
    /// leaves each caller-owned argv reference untouched.
    #[test]
    fn invoke_argv_dispatches_builtin_without_adopting_words() {
        leak_free(|| unsafe {
            let interp = tcl_runtime_create_interp();
            tcl_runtime_set_current_interp(interp);
            let words = [
                owned_word(b"string"),
                owned_word(b"length"),
                owned_word(b"abc"),
            ];
            let counts = words.map(|word| (*word).ref_count);

            let mut completion = invoke(&words);
            assert_eq!(completion.code, 0);
            assert_eq!(obj_bytes(completion.result), b"3");
            assert_eq!(option(&completion, b"-code"), b"0");
            assert_eq!(option(&completion, b"-level"), b"0");
            assert_eq!(words.map(|word| (*word).ref_count), counts);

            tcl_completion_release(&mut completion);
            assert!(completion.result.is_null());
            assert!(completion.options.is_null());
            // Reset output storage makes this release form deliberately
            // idempotent (unlike mixing it with individual handle releases).
            tcl_completion_release(&mut completion);
            release_words(&words);
            tcl_runtime_set_current_interp(ptr::null_mut());
            tcl_runtime_delete_interp(interp);
        });
    }

    #[test]
    #[cfg(have_tommath)]
    fn nested_add_propagates_the_inner_arithmetic_error() {
        leak_free(|| unsafe {
            let interp = tcl_runtime_create_interp();
            tcl_runtime_set_current_interp(interp);

            let inner = tcl_codegen_expr_add(owned_str(b"not-a-number"), owned_str(b"1"));
            assert!(inner.is_null());
            let error = (*interp).result_bytes();
            assert!(!error.is_empty());

            let outer = tcl_codegen_expr_add(inner, owned_str(b"2"));
            assert!(outer.is_null());
            assert_eq!((*interp).result_bytes(), error);

            tcl_runtime_set_current_interp(ptr::null_mut());
            tcl_runtime_delete_interp(interp);
        });
    }

    /// Command lookup happens after argv construction, so a renamed command is
    /// still resolved through the existing namespace table rather than a
    /// compiler-side command-name table.
    #[test]
    fn invoke_argv_follows_renamed_command() {
        leak_free(|| unsafe {
            let interp = tcl_runtime_create_interp();
            tcl_runtime_set_current_interp(interp);
            assert_eq!(
                tcl_eval_code(box_str(
                    b"proc original {x} {list got:$x}; rename original renamed"
                )),
                0
            );
            let words = [owned_word(b"renamed"), owned_word(b"value")];

            let mut completion = invoke(&words);
            assert_eq!(completion.code, 0);
            assert_eq!(obj_bytes(completion.result), b"got:value");

            tcl_completion_release(&mut completion);
            release_words(&words);
            tcl_runtime_set_current_interp(ptr::null_mut());
            tcl_runtime_delete_interp(interp);
        });
    }

    /// Alias dispatch is resolved by the interpreter after argv construction;
    /// the prebuilt path neither knows nor cares which command it redirects to.
    #[test]
    fn invoke_argv_follows_interp_alias() {
        leak_free(|| unsafe {
            let interp = tcl_runtime_create_interp();
            tcl_runtime_set_current_interp(interp);
            assert_eq!(
                tcl_eval_code(box_str(b"interp alias {} prefixed {} list prefix")),
                0
            );
            let words = [owned_word(b"prefixed"), owned_word(b"value")];

            let mut completion = invoke(&words);
            assert_eq!(completion.code, 0);
            assert_eq!(obj_bytes(completion.result), b"prefix value");

            tcl_completion_release(&mut completion);
            release_words(&words);
            tcl_runtime_set_current_interp(ptr::null_mut());
            tcl_runtime_delete_interp(interp);
        });
    }

    /// A resolver miss takes the usual `unknown`/error path and returns the
    /// real error options instead of an empty placeholder.
    #[test]
    fn invoke_argv_reports_unknown_error_with_options() {
        leak_free(|| unsafe {
            let interp = tcl_runtime_create_interp();
            tcl_runtime_set_current_interp(interp);
            let words = [owned_word(b"definitely_missing_command")];

            let mut completion = invoke(&words);
            assert_eq!(completion.code, 1);
            assert!(obj_bytes(completion.result).starts_with(b"invalid command name"));
            assert_eq!(option(&completion, b"-code"), b"1");
            assert_eq!(option(&completion, b"-level"), b"0");
            assert_eq!(option(&completion, b"-errorcode"), b"NONE");
            assert!(option(&completion, b"-errorinfo").starts_with(b"invalid command name"));

            tcl_completion_release(&mut completion);
            release_words(&words);
            tcl_runtime_set_current_interp(ptr::null_mut());
            tcl_runtime_delete_interp(interp);
        });
    }

    /// An uncaught error taken through the argv path leaves the same
    /// interpreter error state as the source-eval path: `::errorInfo` and
    /// `::errorCode` exist as globals.
    #[test]
    fn invoke_argv_publishes_uncaught_error_globals() {
        leak_free(|| unsafe {
            let interp = tcl_runtime_create_interp();
            tcl_runtime_set_current_interp(interp);
            let words = [owned_word(b"error"), owned_word(b"boom")];

            let mut completion = invoke(&words);
            assert_eq!(completion.code, 1);

            // The source-eval path publishes these at the outermost eval; the
            // argv path must not diverge.
            assert_eq!(tcl_eval_code(box_str(b"set ::errorInfo")), 0);
            assert!((*interp).result_bytes().starts_with(b"boom"));

            tcl_completion_release(&mut completion);
            release_words(&words);
            tcl_runtime_set_current_interp(ptr::null_mut());
            tcl_runtime_delete_interp(interp);
        });
    }

    /// Read one typed scalar through the C ABI, returning the status and the
    /// value written (untouched storage on a refusal).
    unsafe fn get_wide(value: *mut TclObj) -> (i32, i64) {
        let mut out: i64 = i64::MIN;
        // SAFETY: `value` is live and `out` is local aligned storage.
        let status = unsafe { tcl_value_get_wide_int(value, &mut out) };
        (status, out)
    }

    unsafe fn get_double(value: *mut TclObj) -> (i32, f64) {
        let mut out: f64 = f64::NAN;
        // SAFETY: `value` is live and `out` is local aligned storage.
        let status = unsafe { tcl_value_get_double(value, &mut out) };
        (status, out)
    }

    unsafe fn get_bool(value: *mut TclObj) -> (i32, i32) {
        let mut out: i32 = -1;
        // SAFETY: `value` is live and `out` is local aligned storage.
        let status = unsafe { tcl_value_get_bool(value, &mut out) };
        (status, out)
    }

    /// A successful typed read caches the parsed rep onto the object — C's
    /// `TclParseNumber` write-back — and keeps the spelling, so a hot loop
    /// parses once and `puts` still prints what the script wrote.
    #[test]
    fn typed_value_reads_cache_the_parsed_rep_and_keep_the_spelling() {
        leak_free(|| unsafe {
            let interp = tcl_runtime_create_interp();
            tcl_runtime_set_current_interp(interp);

            let decimal = owned_word(b"12");
            assert!((*decimal).type_ptr.is_null(), "starts as a plain string");
            assert_eq!(get_wide(decimal), (TCL_VALUE_GET_OK, 12));
            assert!(
                std::ptr::eq((*decimal).type_ptr, &obj::TCL_INT_TYPE),
                "the parsed integer rep is cached back onto the object"
            );
            assert_eq!(obj_bytes(decimal), b"12", "the spelling survives");

            // A radix spelling keeps its own text, exactly as in C Tcl.
            let hex = owned_word(b"0x10");
            assert_eq!(get_wide(hex), (TCL_VALUE_GET_OK, 16));
            assert!(std::ptr::eq((*hex).type_ptr, &obj::TCL_INT_TYPE));
            assert_eq!(obj_bytes(hex), b"0x10");

            let scientific = owned_word(b"1e3");
            assert_eq!(get_double(scientific), (TCL_VALUE_GET_OK, 1000.0));
            assert!(std::ptr::eq((*scientific).type_ptr, &obj::TCL_DOUBLE_TYPE));
            assert_eq!(obj_bytes(scientific), b"1e3");

            // Now that it carries a double rep, the integer read reports C's
            // double-branch wording and code (`tclObj.c`).
            assert_eq!(get_wide(scientific).0, TCL_VALUE_GET_ERROR);
            assert_eq!(
                (*interp).result_bytes(),
                b"expected integer but got \"1e3\""
            );
            assert_eq!((*interp).error_code(), b"TCL VALUE INTEGER");

            release_words(&[decimal, hex, scientific]);
            tcl_runtime_set_current_interp(ptr::null_mut());
            tcl_runtime_delete_interp(interp);
        });
    }

    /// Beyond-wide integers and non-numbers report C's exact message and
    /// `-errorcode` through the interpreter, leaving the out storage untouched.
    #[test]
    fn typed_value_reads_report_overflow_and_non_numbers_like_c() {
        leak_free(|| unsafe {
            let interp = tcl_runtime_create_interp();
            tcl_runtime_set_current_interp(interp);

            // `tclsh9.0`: `string repeat a [expr {2**200}]` →
            // "integer value too large to represent" / ARITH IOVERFLOW.
            let huge = owned_word(b"99999999999999999999999");
            let (status, untouched) = get_wide(huge);
            assert_eq!(status, TCL_VALUE_GET_ERROR);
            assert_eq!(untouched, i64::MIN, "the out storage is left alone");
            assert_eq!(
                (*interp).result_bytes(),
                b"integer value too large to represent"
            );
            assert_eq!((*interp).error_code(), b"ARITH IOVERFLOW");
            // The same value still widens to a double.
            assert_eq!(get_double(huge), (TCL_VALUE_GET_OK, 1e23));

            let word = owned_word(b"abc");
            assert_eq!(get_wide(word).0, TCL_VALUE_GET_ERROR);
            assert_eq!(
                (*interp).result_bytes(),
                b"expected integer but got \"abc\""
            );
            assert_eq!((*interp).error_code(), b"TCL VALUE NUMBER");
            assert_eq!(get_double(word).0, TCL_VALUE_GET_ERROR);
            assert_eq!(
                (*interp).result_bytes(),
                b"expected floating-point number but got \"abc\""
            );
            assert!(
                (*word).type_ptr.is_null(),
                "a refused read caches nothing onto the object"
            );

            // C renders a well-formed multi-token value as `a list`.
            let listy = owned_word(b"a b");
            assert_eq!(get_bool(listy).0, TCL_VALUE_GET_ERROR);
            assert_eq!(
                (*interp).result_bytes(),
                b"expected boolean value but got a list"
            );

            release_words(&[huge, word, listy]);
            tcl_runtime_set_current_interp(ptr::null_mut());
            tcl_runtime_delete_interp(interp);
        });
    }

    /// The boolean read is the boolean-*context* acceptor over
    /// `tcl_syntax::boolean`'s one word table: unique prefixes resolve, the
    /// ambiguous `o` does not, and any number compares against zero.
    #[test]
    fn typed_value_boolean_read_uses_the_shared_word_table() {
        leak_free(|| unsafe {
            let interp = tcl_runtime_create_interp();
            tcl_runtime_set_current_interp(interp);

            for (spelling, expected) in [
                (&b"tru"[..], 1),
                (b"t", 1),
                (b"ye", 1),
                (b"YES", 1),
                (b"of", 0),
                (b"n", 0),
                (b"2", 1),
                (b"-1", 1),
                (b"0.0", 0),
                (b"0x0", 0),
            ] {
                let value = owned_word(spelling);
                assert_eq!(
                    get_bool(value),
                    (TCL_VALUE_GET_OK, expected),
                    "{:?}",
                    String::from_utf8_lossy(spelling)
                );
                release_words(&[value]);
            }

            // `on`/`off` share the prefix `o`, so tclsh rejects it.
            let ambiguous = owned_word(b"o");
            assert_eq!(get_bool(ambiguous).0, TCL_VALUE_GET_ERROR);
            assert_eq!(
                (*interp).result_bytes(),
                b"expected boolean value but got \"o\""
            );

            // `tclsh9.0`: `expr {NaN ? 1 : 0}` is a domain error, not truthy.
            let nan = owned_word(b"NaN");
            assert_eq!(get_bool(nan).0, TCL_VALUE_GET_ERROR);
            assert_eq!(
                (*interp).result_bytes(),
                b"floating point value is Not a Number"
            );
            assert_eq!(
                get_double(nan).0,
                TCL_VALUE_GET_OK,
                "a double read accepts NaN"
            );

            release_words(&[ambiguous, nan]);
            tcl_runtime_set_current_interp(ptr::null_mut());
            tcl_runtime_delete_interp(interp);
        });
    }

    /// The native constructors produce owned, typed objects whose string rep is
    /// materialised only on demand.
    #[test]
    fn native_double_and_boolean_values_are_owned_and_typed() {
        leak_free(|| unsafe {
            let double = tcl_value_new_double(1.5);
            assert_eq!((*double).ref_count, 1, "generated code owns the result");
            assert!(std::ptr::eq((*double).type_ptr, &obj::TCL_DOUBLE_TYPE));
            assert!((*double).bytes.is_null(), "doubles begin without text");
            assert_eq!(obj_bytes(double), b"1.5");
            assert!(
                std::ptr::eq((*double).type_ptr, &obj::TCL_DOUBLE_TYPE),
                "materialising the text keeps the double internal rep"
            );
            tcl_obj_release(double);

            // C's `Tcl_NewBooleanObj` normalises any non-zero input to 1.
            let truthy = tcl_value_new_bool(7);
            assert_eq!((*truthy).ref_count, 1);
            assert_eq!(obj_bytes(truthy), b"1");
            tcl_obj_release(truthy);

            let falsy = tcl_value_new_bool(0);
            assert_eq!(obj_bytes(falsy), b"0");
            tcl_obj_release(falsy);
        });
    }

    /// A `catch` dispatched from generated code must read its **own** error
    /// state. Before compiled activations existed, `tcl_invoke_argv` dispatched
    /// at `eval_depth == 0`, so `catch`'s `eval_control_body` returned to depth
    /// 0 and the eval loop's outermost rule published-and-reset the exception
    /// before `catch` read `error_code()` — `-errorcode` came back `NONE`.
    ///
    /// Oracle (`tclsh9.0`): `catch {error m NEGATIVE {MYERR NEG}} r opts` sets
    /// `-errorcode` to `MYERR NEG` and `-errorinfo` to `NEGATIVE`.
    #[test]
    fn invoke_argv_catch_reads_the_raised_errorcode_at_compiled_top_level() {
        leak_free(|| unsafe {
            let interp = tcl_runtime_create_interp();
            tcl_runtime_set_current_interp(interp);
            let words = [
                owned_word(b"catch"),
                owned_word(b"error m NEGATIVE {MYERR NEG}"),
                owned_word(b"r"),
                owned_word(b"opts"),
            ];

            let mut completion = invoke(&words);
            assert_eq!(completion.code, 0);
            assert_eq!(obj_bytes(completion.result), b"1", "catch reports an error");

            assert_eq!(tcl_eval_code(box_str(b"dict get $opts -errorcode")), 0);
            assert_eq!((*interp).result_bytes(), b"MYERR NEG");
            assert_eq!(tcl_eval_code(box_str(b"dict get $opts -errorinfo")), 0);
            assert_eq!((*interp).result_bytes(), b"NEGATIVE");
            assert_eq!(tcl_eval_code(box_str(b"set r")), 0);
            assert_eq!((*interp).result_bytes(), b"m");

            tcl_completion_release(&mut completion);
            release_words(&words);
            tcl_runtime_set_current_interp(ptr::null_mut());
            tcl_runtime_delete_interp(interp);
        });
    }

    /// An explicit compiled activation is an eval-loop activation: an uncaught
    /// error inside it is *not* published while it is held (the enclosing
    /// activation, not the dispatch, is the outermost one), and the matching
    /// leave publishes `::errorInfo`/`::errorCode` exactly as the eval loop's
    /// tail does.
    #[test]
    fn compiled_activation_defers_error_publication_to_its_own_leave() {
        leak_free(|| unsafe {
            let interp = tcl_runtime_create_interp();
            tcl_runtime_set_current_interp(interp);

            assert_eq!(tcl_codegen_activation_enter(), 0);
            let words = [owned_word(b"error"), owned_word(b"boom")];
            let mut completion = invoke(&words);
            assert_eq!(completion.code, 1);

            // Still inside the activation, so the globals are untouched — the
            // eval loop would not have published at depth 1 either. Read them
            // straight out of the interpreter: an intervening `eval` would
            // reset the very error episode under test.
            assert!(
                (*interp).var_get(b"::errorInfo").is_none(),
                "a held activation is not the outermost one"
            );

            tcl_codegen_activation_leave(1);
            let published = (*interp)
                .var_get(b"::errorInfo")
                .expect("leaving the outermost activation publishes the trace");
            assert!(obj_bytes(published).starts_with(b"boom"));
            assert_eq!(
                obj_bytes((*interp).var_get(b"::errorCode").expect("::errorCode")),
                b"NONE"
            );

            tcl_completion_release(&mut completion);
            release_words(&words);
            tcl_runtime_set_current_interp(ptr::null_mut());
            tcl_runtime_delete_interp(interp);
        });
    }

    /// Without a current interpreter the activation entry declines rather than
    /// silently pretending to hold one, and the paired leave is a no-op.
    #[test]
    fn compiled_activation_declines_without_a_current_interpreter() {
        leak_free(|| {
            tcl_runtime_set_current_interp(ptr::null_mut());
            assert_ne!(tcl_codegen_activation_enter(), 0);
            tcl_codegen_activation_leave(0);
        });
    }

    /// Non-standard completion codes remain raw `i32`s at the ABI boundary,
    /// with a matching return-options `-code` value.
    #[test]
    fn invoke_argv_preserves_custom_completion_code() {
        leak_free(|| unsafe {
            let interp = tcl_runtime_create_interp();
            tcl_runtime_set_current_interp(interp);
            let words = [
                owned_word(b"return"),
                owned_word(b"-level"),
                owned_word(b"0"),
                owned_word(b"-code"),
                owned_word(b"73"),
                owned_word(b"custom result"),
            ];

            let mut completion = invoke(&words);
            assert_eq!(completion.code, 73);
            assert_eq!(obj_bytes(completion.result), b"custom result");
            assert_eq!(option(&completion, b"-code"), b"73");
            assert_eq!(option(&completion, b"-level"), b"0");

            tcl_completion_release(&mut completion);
            release_words(&words);
            tcl_runtime_set_current_interp(ptr::null_mut());
            tcl_runtime_delete_interp(interp);
        });
    }

    /// Malformed ABI inputs do not read a null argv and still provide an owned
    /// error completion whenever output storage is available.
    #[test]
    fn invoke_argv_validates_boundary_inputs_without_leaking() {
        leak_free(|| unsafe {
            let interp = tcl_runtime_create_interp();
            tcl_runtime_set_current_interp(interp);
            let mut completion = empty_completion();

            assert_eq!(
                tcl_invoke_argv(ptr::null(), -1, &mut completion),
                TCL_INVOKE_ABI_INVALID_ARGC
            );
            assert_eq!(completion.code, 1);
            tcl_completion_release(&mut completion);

            assert_eq!(
                tcl_invoke_argv(ptr::null(), 1, &mut completion),
                TCL_INVOKE_ABI_NULL_ARGV
            );
            assert_eq!(completion.code, 1);
            tcl_completion_release(&mut completion);

            assert_eq!(
                tcl_invoke_argv(ptr::null(), 1, ptr::null_mut()),
                TCL_INVOKE_ABI_NULL_OUT
            );
            tcl_runtime_set_current_interp(ptr::null_mut());
            tcl_runtime_delete_interp(interp);
        });
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
    #[cfg(have_tommath)]
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

    /// Call-frame storage is dynamically distinct, so a nested generated call
    /// cannot overwrite the argv or completion storage held by its caller.
    #[test]
    fn call_frames_are_reentrant_and_layout_checked() {
        unsafe {
            let before = tcl_codegen_call_frame_outstanding();
            let outer = tcl_codegen_call_frame_alloc(32, 8);
            assert!(!outer.is_null());
            assert_eq!(tcl_codegen_call_frame_outstanding(), before + 1);
            outer.write_bytes(0xA5, 32);

            let inner = tcl_codegen_call_frame_alloc(48, 16);
            assert!(!inner.is_null());
            assert_ne!(outer, inner);
            assert_eq!(tcl_codegen_call_frame_outstanding(), before + 2);
            inner.write_bytes(0x5A, 48);
            assert_eq!(outer.read(), 0xA5);

            assert_eq!(tcl_codegen_call_frame_free(inner), 0);
            assert_eq!(tcl_codegen_call_frame_free(outer.wrapping_add(1)), -1);
            assert_eq!(tcl_codegen_call_frame_free(outer), 0);
            assert_eq!(tcl_codegen_call_frame_outstanding(), before);
            assert_eq!(tcl_codegen_call_frame_free(outer), -1);
            assert!(tcl_codegen_call_frame_alloc(0, 4).is_null());
            assert!(tcl_codegen_call_frame_alloc(8, 3).is_null());
        }
    }

    /// Model the frame an emitted leaf statement allocates: `slots` owned
    /// object slots followed by `completions` completion records.
    ///
    /// This mirrors `codegen::wasm::leaf_invoke`'s layout, so the round-trip
    /// tests below exercise the same allocation, adoption, and release order
    /// the emitted module performs. The emitter's byte offsets are the wasm32
    /// ones fixed in `tcl-runtime-api`; a native test frame uses the host's own
    /// pointer width for the same shape.
    struct CompiledFrame {
        base: *mut u8,
        slots: usize,
    }

    impl CompiledFrame {
        const STRIDE: usize = core::mem::size_of::<*mut TclObj>();
        const ALIGN: usize =
            if core::mem::align_of::<TclCompletionAbi>() > core::mem::align_of::<*mut TclObj>() {
                core::mem::align_of::<TclCompletionAbi>()
            } else {
                core::mem::align_of::<*mut TclObj>()
            };

        fn new(slots: usize, completions: usize) -> Self {
            let objects = slots * Self::STRIDE;
            let completion_base = objects.next_multiple_of(Self::ALIGN);
            let bytes = completion_base + completions * core::mem::size_of::<TclCompletionAbi>();
            let base = tcl_codegen_call_frame_alloc(
                i32::try_from(bytes).expect("test frame fits i32"),
                i32::try_from(Self::ALIGN).expect("test frame alignment fits i32"),
            );
            assert!(!base.is_null());
            let frame = Self { base, slots };
            // The emitted prologue nulls every object slot so the single
            // cleanup path can release them null-safely.
            for slot in 0..slots {
                frame.store(slot, ptr::null_mut());
            }
            frame
        }

        fn slot_ptr(&self, slot: usize) -> *mut *mut TclObj {
            assert!(slot < self.slots);
            // SAFETY: `slot` is inside the allocation this frame owns.
            unsafe { self.base.add(slot * Self::STRIDE).cast::<*mut TclObj>() }
        }

        fn store(&self, slot: usize, value: *mut TclObj) {
            // SAFETY: the slot is inside this frame's allocation.
            unsafe { self.slot_ptr(slot).write(value) };
        }

        fn load(&self, slot: usize) -> *mut TclObj {
            // SAFETY: every slot was written by `new` before any read.
            unsafe { self.slot_ptr(slot).read() }
        }

        fn completion(&self, index: usize) -> *mut TclCompletionAbi {
            let base = (self.slots * Self::STRIDE).next_multiple_of(Self::ALIGN);
            // SAFETY: completion storage follows the object slots.
            unsafe {
                self.base
                    .add(base + index * core::mem::size_of::<TclCompletionAbi>())
                    .cast::<TclCompletionAbi>()
            }
        }

        /// The emitted epilogue: release every object slot, then free the frame.
        fn release(self) {
            for slot in 0..self.slots {
                // SAFETY: each slot is null or one owned reference.
                unsafe { tcl_obj_release(self.load(slot)) };
            }
            // SAFETY: the frame came from the matching allocator.
            assert_eq!(unsafe { tcl_codegen_call_frame_free(self.base) }, 0);
        }
    }

    /// Run one compiled invocation exactly as the emitter lays it out: dispatch
    /// the argv run starting at `argv_slot`, stash the code, and adopt the
    /// completion's owned result and options into their frame slots.
    unsafe fn compiled_invoke(
        frame: &CompiledFrame,
        argv_slot: usize,
        argc: usize,
        completion: usize,
        result_slot: usize,
        options_slot: usize,
    ) -> i32 {
        let out = frame.completion(completion);
        // SAFETY: the argv run and completion record are inside the frame.
        unsafe {
            tcl_invoke_argv(
                frame.slot_ptr(argv_slot).cast_const(),
                i32::try_from(argc).expect("test argc fits i32"),
                out,
            );
            frame.store(result_slot, (*out).result);
            frame.store(options_slot, (*out).options);
            (*out).code
        }
    }

    /// `$a(b)` reads the array element, fires its read traces, and hands
    /// generated code exactly one releasable reference.
    #[test]
    fn var_get_element_reads_an_array_element() {
        leak_free(|| unsafe {
            let interp = tcl_runtime_create_interp();
            tcl_runtime_set_current_interp(interp);
            assert_eq!(tcl_eval_code(box_str(b"set a(b) element")), 0);

            let value = tcl_codegen_var_get_element(b"a".as_ptr(), 1, b"b".as_ptr(), 1);
            assert!(!value.is_null());
            assert_eq!(obj_bytes(value), b"element");
            tcl_obj_release(value);

            // A missing element reports the C-faithful read error, not a scalar
            // miss on a name that happens to contain parentheses.
            assert!(tcl_codegen_var_get_element(b"a".as_ptr(), 1, b"z".as_ptr(), 1).is_null());
            assert_eq!(
                (*interp).result_bytes(),
                b"can't read \"a(z)\": no such element in array"
            );

            tcl_runtime_set_current_interp(ptr::null_mut());
            tcl_runtime_delete_interp(interp);
        });
    }

    /// `tcl_codegen_word_concat` borrows its parts and returns one owned join.
    #[test]
    fn word_concat_borrows_its_parts() {
        leak_free(|| unsafe {
            let interp = tcl_runtime_create_interp();
            tcl_runtime_set_current_interp(interp);
            let parts = [owned_word(b"hi "), owned_word(b"world")];
            let counts = parts.map(|part| (*part).ref_count);

            let joined = tcl_codegen_word_concat(parts.as_ptr(), 2);
            assert_eq!(obj_bytes(joined), b"hi world");
            assert_eq!((*joined).ref_count, 1);
            assert_eq!(parts.map(|part| (*part).ref_count), counts);
            tcl_obj_release(joined);

            let empty = tcl_codegen_word_concat(ptr::null(), 0);
            assert_eq!(obj_bytes(empty), b"");
            tcl_obj_release(empty);
            assert!(tcl_codegen_word_concat(ptr::null(), 1).is_null());

            release_words(&parts);
            tcl_runtime_set_current_interp(ptr::null_mut());
            tcl_runtime_delete_interp(interp);
        });
    }

    /// The whole compiled leaf-statement shape — allocate one frame, evaluate
    /// every word into it, dispatch, adopt the completion, release everything —
    /// balances its allocations and leaves no outstanding call frame.
    #[test]
    fn compiled_leaf_statement_round_trip_is_leak_free() {
        let frames_before = tcl_codegen_call_frame_outstanding();
        leak_free(|| unsafe {
            let interp = tcl_runtime_create_interp();
            tcl_runtime_set_current_interp(interp);
            assert_eq!(tcl_eval_code(box_str(b"set value abc")), 0);

            // `string length $value`: three argv slots, then the root result
            // and options, and one completion record.
            let frame = CompiledFrame::new(5, 1);
            frame.store(0, tcl_obj_new_string_owned(b"string".as_ptr(), 6));
            frame.store(1, tcl_obj_new_string_owned(b"length".as_ptr(), 6));
            frame.store(2, tcl_codegen_var_get(b"value".as_ptr(), 5));
            assert!(!frame.load(2).is_null());

            let code = compiled_invoke(&frame, 0, 3, 0, 3, 4);
            assert_eq!(code, 0);
            assert_eq!(obj_bytes(frame.load(3)), b"3");
            frame.release();

            tcl_runtime_set_current_interp(ptr::null_mut());
            tcl_runtime_delete_interp(interp);
        });
        assert_eq!(tcl_codegen_call_frame_outstanding(), frames_before);
    }

    /// A nested `[…]` word adopts the inner completion's owned result as the
    /// outer word's value; both invocations share one frame and one cleanup.
    #[test]
    fn compiled_nested_word_round_trip_is_leak_free() {
        let frames_before = tcl_codegen_call_frame_outstanding();
        leak_free(|| unsafe {
            let interp = tcl_runtime_create_interp();
            tcl_runtime_set_current_interp(interp);

            // `string length [string tolower ABC]`:
            //   slots 0..2  outer argv (slot 2 receives the nested result)
            //   slot  3     outer options, slot 4 outer result
            //   slots 5..7  nested argv, slot 8 nested options
            let frame = CompiledFrame::new(9, 2);
            frame.store(0, tcl_obj_new_string_owned(b"string".as_ptr(), 6));
            frame.store(1, tcl_obj_new_string_owned(b"length".as_ptr(), 6));
            frame.store(5, tcl_obj_new_string_owned(b"string".as_ptr(), 6));
            frame.store(6, tcl_obj_new_string_owned(b"tolower".as_ptr(), 7));
            frame.store(7, tcl_obj_new_string_owned(b"ABC".as_ptr(), 3));

            assert_eq!(compiled_invoke(&frame, 5, 3, 1, 2, 8), 0);
            assert_eq!(obj_bytes(frame.load(2)), b"abc");
            assert_eq!(compiled_invoke(&frame, 0, 3, 0, 4, 3), 0);
            assert_eq!(obj_bytes(frame.load(4)), b"3");
            frame.release();

            tcl_runtime_set_current_interp(ptr::null_mut());
            tcl_runtime_delete_interp(interp);
        });
        assert_eq!(tcl_codegen_call_frame_outstanding(), frames_before);
    }

    /// The abrupt-completion paths are the ones that can leak: the statement
    /// leaves part-way through word evaluation, or the invocation itself
    /// completes `error` / `break` / `return`. Every one still runs the single
    /// cleanup path, so the counters and the frame ledger both balance.
    #[test]
    fn compiled_abrupt_completions_release_every_partial_word() {
        let frames_before = tcl_codegen_call_frame_outstanding();
        leak_free(|| unsafe {
            let interp = tcl_runtime_create_interp();
            tcl_runtime_set_current_interp(interp);

            // A missing variable aborts mid-argv: two words are already owned
            // by the frame and the third read failed.
            let frame = CompiledFrame::new(5, 1);
            frame.store(0, tcl_obj_new_string_owned(b"string".as_ptr(), 6));
            frame.store(1, tcl_obj_new_string_owned(b"length".as_ptr(), 6));
            frame.store(2, tcl_codegen_var_get(b"missing".as_ptr(), 7));
            assert!(frame.load(2).is_null(), "the read must report an error");
            frame.release();

            // `error`, `break`, and `return` all complete through the same
            // adopt-then-release path before the emitted dispatch branches.
            for (command, expected) in [(&b"error"[..], 1), (&b"break"[..], 3), (&b"return"[..], 2)]
            {
                let frame = CompiledFrame::new(3, 1);
                frame.store(
                    0,
                    tcl_obj_new_string_owned(
                        command.as_ptr(),
                        i32::try_from(command.len()).expect("test word fits i32"),
                    ),
                );
                assert_eq!(compiled_invoke(&frame, 0, 1, 0, 1, 2), expected);
                frame.release();
            }

            tcl_runtime_set_current_interp(ptr::null_mut());
            tcl_runtime_delete_interp(interp);
        });
        assert_eq!(tcl_codegen_call_frame_outstanding(), frames_before);
    }

    /// The argv constructor transfers exactly one owned reference, which is
    /// the reference the generated cleanup path releases after dispatch.
    #[test]
    fn owned_string_constructor_has_one_releasable_reference() {
        leak_free(|| unsafe {
            let word = tcl_obj_new_string_owned(b"word".as_ptr(), 4);
            assert_eq!((*word).ref_count, 1);
            assert_eq!(obj_bytes(word), b"word");
            tcl_obj_release(word);
        });
    }

    // ---- native proc entries (issue #1774) ---------------------------------

    thread_local! {
        /// The argv [`stub_body`] dispatches, and how many times a stub ran.
        static STUB_ARGV: RefCell<Vec<Vec<u8>>> = const { RefCell::new(Vec::new()) };
        static STUB_CALLS: Cell<u32> = const { Cell::new(0) };
        /// `(line, source text)` [`stub_body`] logs on an error completion, as
        /// the emitter will on a statement's error edge. `None` logs nothing.
        static STUB_LOG: RefCell<Option<(i32, Vec<u8>)>> = const { RefCell::new(None) };
    }

    /// Arm the stub with the one command its body runs, and reset the counter.
    fn stub_argv(words: &[&[u8]]) {
        STUB_ARGV.with(|argv| {
            *argv.borrow_mut() = words.iter().map(|word| word.to_vec()).collect();
        });
        STUB_LOG.with(|log| *log.borrow_mut() = None);
        STUB_CALLS.with(|calls| calls.set(0));
    }

    /// Give the armed stub the statement site it logs when its command fails.
    fn stub_logs(line: i32, source: &[u8]) {
        STUB_LOG.with(|log| *log.borrow_mut() = Some((line, source.to_vec())));
    }

    fn stub_calls() -> u32 {
        STUB_CALLS.with(Cell::get)
    }

    /// A stub compiled proc body.
    ///
    /// It dispatches one prebuilt argv through the same [`tcl_invoke_argv`] an
    /// emitted body uses for a statement it could not lower, and forwards that
    /// completion as its own. Nothing here is test-shaped: the completion
    /// protocol, the borrowed argv, and the absent frame/activation prologue
    /// are exactly what `native_emit` must emit for a proc entry.
    unsafe extern "C" fn stub_body(
        _argv: *const *mut TclObj,
        _argc: i32,
        out: *mut TclCompletionAbi,
    ) -> i32 {
        STUB_CALLS.with(|calls| calls.set(calls.get() + 1));
        let words = STUB_ARGV.with(|argv| argv.borrow().clone());
        // SAFETY: each word is a valid readable slice; `owned_word` boxes it
        // and takes the caller-owned reference the borrowed-argv ABI needs.
        let boxed: Vec<*mut TclObj> = words
            .iter()
            .map(|word| unsafe { owned_word(word) })
            .collect();
        // SAFETY: `boxed` are live caller-owned words and `out` is the
        // runtime's own zeroed completion storage.
        let status = unsafe {
            tcl_invoke_argv(
                boxed.as_ptr(),
                i32::try_from(boxed.len()).expect("stub argv fits i32"),
                out,
            )
        };
        assert_eq!(status, TCL_INVOKE_ABI_OK);
        // SAFETY: balances `owned_word`'s reference on each word.
        unsafe { release_words(&boxed) };
        // The statement's error edge: an emitted body logs its own source site
        // here, because nothing else will.
        // SAFETY: `out` was written by the invoke above.
        if unsafe { (*out).code } == 1 {
            if let Some((line, source)) = STUB_LOG.with(|log| log.borrow().clone()) {
                // SAFETY: `source` is a live readable slice.
                unsafe {
                    tcl_codegen_log_command(
                        line,
                        source.as_ptr(),
                        i32::try_from(source.len()).expect("source fits i32"),
                    );
                }
            }
        }
        NATIVE_PROC_STATUS_RAN
    }

    /// A stub in the shape the emitter produces for a body whose last command
    /// leaves no result of its own — a bare `return`, or a `puts`.
    ///
    /// It reports success with `out` untouched, which the ABI defines as "the
    /// runtime's own result is the body's answer".
    unsafe extern "C" fn stub_no_result(
        _argv: *const *mut TclObj,
        _argc: i32,
        _out: *mut TclCompletionAbi,
    ) -> i32 {
        STUB_CALLS.with(|calls| calls.set(calls.get() + 1));
        NATIVE_PROC_STATUS_RAN
    }

    /// A stub in the shape the emitter produces for a plain `return v`: it
    /// records the pending return state the `return` command records, then
    /// completes with the raw `Code::Return` the interpreted body reports.
    unsafe extern "C" fn stub_plain_return(
        _argv: *const *mut TclObj,
        _argc: i32,
        out: *mut TclCompletionAbi,
    ) -> i32 {
        STUB_CALLS.with(|calls| calls.set(calls.get() + 1));
        tcl_codegen_return_state(1, Code::Ok.as_int() as i32);
        // SAFETY: `out` is the runtime's own zeroed completion storage, and
        // `owned_word` hands over one owned reference on the result.
        unsafe {
            (*out).code = Code::Return.as_int() as i32;
            (*out).result = owned_word(b"v");
        }
        NATIVE_PROC_STATUS_RAN
    }

    /// The same stub without the state write — what an emitter that treated
    /// `Code::Return` as nothing but a completion code would produce.
    unsafe extern "C" fn stub_return_without_state(
        _argv: *const *mut TclObj,
        _argc: i32,
        out: *mut TclCompletionAbi,
    ) -> i32 {
        STUB_CALLS.with(|calls| calls.set(calls.get() + 1));
        // SAFETY: as above.
        unsafe {
            (*out).code = Code::Return.as_int() as i32;
            (*out).result = owned_word(b"v");
        }
        NATIVE_PROC_STATUS_RAN
    }

    /// A stub that declines before doing anything at all — the only shape a
    /// decline may take, since the runtime then runs the source body in the
    /// frame it already prepared.
    unsafe extern "C" fn stub_declines(
        _argv: *const *mut TclObj,
        _argc: i32,
        _out: *mut TclCompletionAbi,
    ) -> i32 {
        STUB_CALLS.with(|calls| calls.set(calls.get() + 1));
        NATIVE_PROC_STATUS_DECLINED
    }

    /// The call an emitted module makes for a `proc` statement.
    unsafe fn define_native(
        name: &[u8],
        params: &[u8],
        body: &[u8],
        entry: Option<NativeProcEntry>,
    ) {
        // SAFETY: all three ranges are live readable slices.
        let code = unsafe {
            tcl_codegen_proc_define_native(
                name.as_ptr(),
                i32::try_from(name.len()).expect("name fits i32"),
                params.as_ptr(),
                i32::try_from(params.len()).expect("params fit i32"),
                body.as_ptr(),
                i32::try_from(body.len()).expect("body fits i32"),
                entry,
            )
        };
        assert_eq!(code, 0, "defining {}", String::from_utf8_lossy(name));
    }

    /// Evaluate `script` and return its completion code and result text.
    unsafe fn eval(interp: *mut Interp, script: &str) -> (Code, Vec<u8>) {
        // SAFETY: the caller holds a live interpreter.
        unsafe {
            (
                (*interp).eval_str(script.as_bytes()),
                (*interp).result_bytes(),
            )
        }
    }

    /// A fresh interpreter, installed as the current one for the codegen ABI.
    unsafe fn with_current_interp(body: impl FnOnce(*mut Interp)) {
        let interp = tcl_runtime_create_interp();
        // SAFETY: a live interpreter this helper owns for the call's duration.
        unsafe {
            tcl_runtime_set_current_interp(interp);
            body(interp);
            tcl_runtime_set_current_interp(ptr::null_mut());
            tcl_runtime_delete_interp(interp);
        }
    }

    #[test]
    fn a_native_entry_runs_instead_of_the_source_body_and_is_counted() {
        leak_free(|| unsafe {
            with_current_interp(|interp| {
                assert_eq!(tcl_codegen_native_proc_dispatches(), 0);
                stub_argv(&[b"return", b"native"]);
                define_native(b"p", b"a", b"return source", Some(stub_body));

                assert_eq!(eval(interp, "p x"), (Code::Ok, b"native".to_vec()));
                assert_eq!(stub_calls(), 1);
                assert_eq!(tcl_codegen_native_proc_dispatches(), 1);

                // The source body is still the proc's body: it is what a step
                // trace forces, what a decline falls back to, and what
                // introspection reports.
                assert_eq!(eval(interp, "info body p").1, b"return source");
                assert_eq!(eval(interp, "info args p").1, b"a");
            });
        });
    }

    #[test]
    fn proc_register_is_proc_define_native_with_no_entry() {
        leak_free(|| unsafe {
            with_current_interp(|interp| {
                let (name, params, body) = (b"only", b"a", b"return source");
                assert_eq!(
                    tcl_codegen_proc_register(
                        name.as_ptr(),
                        i32::try_from(name.len()).unwrap(),
                        params.as_ptr(),
                        i32::try_from(params.len()).unwrap(),
                        body.as_ptr(),
                        i32::try_from(body.len()).unwrap(),
                    ),
                    0
                );
                assert_eq!(eval(interp, "only x"), (Code::Ok, b"source".to_vec()));
                assert_eq!(tcl_codegen_native_proc_dispatches(), 0);
            });
        });
    }

    #[test]
    fn the_native_entry_runs_in_the_call_frame_run_proc_prepared() {
        leak_free(|| unsafe {
            with_current_interp(|interp| {
                // `info level 0` is the double-frame detector: it reads the
                // *current* frame's recorded invocation words, and only the
                // frame `run_proc` pushed has any. An entry that pushed its
                // own would report the wrong level and have no words at all.
                stub_argv(&[b"info", b"level", b"0"]);
                define_native(b"lvl", b"a b", b"return source", Some(stub_body));
                assert_eq!(eval(interp, "lvl 7 8").1, b"lvl 7 8");

                stub_argv(&[b"info", b"level"]);
                define_native(b"depth", b"", b"return source", Some(stub_body));
                assert_eq!(eval(interp, "depth").1, b"1");

                // Formals are bound by name before the entry runs, so the body
                // reads them as ordinary cells.
                stub_argv(&[b"set", b"b"]);
                define_native(b"second", b"a b", b"return source", Some(stub_body));
                assert_eq!(eval(interp, "second 7 8").1, b"8");

                // The body runs in the proc's defining namespace.
                assert_eq!(eval(interp, "namespace eval ns {}").0, Code::Ok);
                stub_argv(&[b"namespace", b"current"]);
                define_native(b"ns::inner", b"", b"return source", Some(stub_body));
                assert_eq!(eval(interp, "ns::inner").1, b"::ns");
            });
        });
    }

    #[test]
    fn wrong_number_of_arguments_never_reaches_the_native_entry() {
        // The message is `run_proc`'s, produced before any body runs, so the
        // native and source paths cannot differ. tclsh 9.0.4 and 8.6.16 both
        // print `wrong # args: should be "greet name"`.
        leak_free(|| unsafe {
            with_current_interp(|interp| {
                stub_argv(&[b"return", b"native"]);
                define_native(b"greet", b"name", b"return source", Some(stub_body));

                assert_eq!(
                    eval(interp, "greet"),
                    (
                        Code::Error,
                        b"wrong # args: should be \"greet name\"".to_vec()
                    )
                );
                assert_eq!(
                    eval(interp, "greet a b"),
                    (
                        Code::Error,
                        b"wrong # args: should be \"greet name\"".to_vec()
                    )
                );
                assert_eq!(stub_calls(), 0, "the entry must never see a bad arity");
                assert_eq!(tcl_codegen_native_proc_dispatches(), 0);
            });
        });
    }

    /// A multi-line body whose error carries a `(procedure ... line N)` frame.
    const BOOM_BODY: &[u8] = b"\n    set b 1\n    error \"bad $a\"\n";

    #[test]
    fn a_declining_entry_falls_back_to_the_source_body_observably_unchanged() {
        leak_free(|| unsafe {
            with_current_interp(|interp| {
                // The same proc, same name, same body — once with a
                // declining entry and once with none. Comparing under one name
                // is what makes the comparison total: the proc name appears in
                // the `(procedure ...)` frame and in the caller's own.
                stub_argv(&[]);
                define_native(b"boom", b"a", BOOM_BODY, Some(stub_declines));
                assert_eq!(eval(interp, "catch {boom Q} m o").1, b"1");
                let declined_msg = eval(interp, "set m").1;
                let declined_info = eval(interp, "dict get $o -errorinfo").1;
                let declined_stack = eval(interp, "dict get $o -errorstack").1;
                let declined_code = eval(interp, "dict get $o -errorcode").1;
                assert_eq!(stub_calls(), 1, "the entry was asked, and declined");
                assert_eq!(
                    tcl_codegen_native_proc_dispatches(),
                    0,
                    "a decline is not a native run"
                );

                define_native(b"boom", b"a", BOOM_BODY, None);
                assert_eq!(eval(interp, "catch {boom Q} m o").1, b"1");
                assert_eq!(eval(interp, "set m").1, declined_msg);
                assert_eq!(eval(interp, "dict get $o -errorinfo").1, declined_info);
                assert_eq!(eval(interp, "dict get $o -errorstack").1, declined_stack);
                assert_eq!(eval(interp, "dict get $o -errorcode").1, declined_code);

                // And that shared answer is the interpreted one: tclsh 9.0.4
                // and 8.6.16 print exactly these five lines.
                assert_eq!(declined_msg, b"bad Q");
                assert_eq!(
                    String::from_utf8_lossy(&declined_info),
                    "bad Q\n    while executing\n\"error \"bad $a\"\"\n    \
                     (procedure \"boom\" line 3)\n    invoked from within\n\"boom Q\""
                );
            });
        });
    }

    #[test]
    fn redefining_the_proc_drops_the_native_entry() {
        leak_free(|| unsafe {
            with_current_interp(|interp| {
                stub_argv(&[b"return", b"native"]);
                define_native(b"twice", b"x", b"return source", Some(stub_body));
                assert_eq!(eval(interp, "twice 4").1, b"native");
                assert_eq!(tcl_codegen_native_proc_dispatches(), 1);

                // The `proc` command builds a definition with no entry, so the
                // new source body runs interpreted. Nothing invalidates
                // anything: the entry lived on the definition that just went.
                assert_eq!(
                    eval(interp, "proc twice {x} {return redefined}").0,
                    Code::Ok
                );
                assert_eq!(eval(interp, "twice 4").1, b"redefined");
                assert_eq!(tcl_codegen_native_proc_dispatches(), 1);
                assert_eq!(stub_calls(), 1);
            });
        });
    }

    #[test]
    fn rename_carries_the_native_entry_and_deleting_the_proc_drops_it() {
        leak_free(|| unsafe {
            with_current_interp(|interp| {
                stub_argv(&[b"return", b"native"]);
                define_native(b"twice", b"x", b"return source", Some(stub_body));

                assert_eq!(eval(interp, "rename twice thrice").0, Code::Ok);
                assert_eq!(eval(interp, "thrice 4").1, b"native");
                assert_eq!(tcl_codegen_native_proc_dispatches(), 1);
                assert_eq!(eval(interp, "catch {twice 4} m"), (Code::Ok, b"1".to_vec()));
                assert_eq!(eval(interp, "set m").1, b"invalid command name \"twice\"");

                assert_eq!(eval(interp, "rename thrice {}").0, Code::Ok);
                assert_eq!(eval(interp, "catch {thrice 4} m").1, b"1");
                assert_eq!(eval(interp, "set m").1, b"invalid command name \"thrice\"");

                // A whole namespace going away takes its definitions, and the
                // entries on them, with it.
                assert_eq!(eval(interp, "namespace eval ns {}").0, Code::Ok);
                define_native(b"ns::inner", b"", b"return source", Some(stub_body));
                assert_eq!(eval(interp, "ns::inner").1, b"native");
                assert_eq!(eval(interp, "namespace delete ns").0, Code::Ok);
                assert_eq!(eval(interp, "catch {ns::inner} m").1, b"1");
                assert_eq!(
                    eval(interp, "set m").1,
                    b"invalid command name \"ns::inner\""
                );
                assert_eq!(tcl_codegen_native_proc_dispatches(), 2);
            });
        });
    }

    #[test]
    fn a_live_step_trace_forces_the_source_body() {
        // A natively lowered `set`/`list`/`return` never reaches `dispatch`,
        // so it would fire no `enterstep` and the transcript would silently
        // lose lines. `dispatch_traced` installs the proc's own step traces
        // before dispatching, so one read of `step_active` in `run_proc`
        // covers both this proc's traces and an outer one's.
        //
        // Transcript pinned against tclsh 9.0.4 and 8.6.16: step traces fire
        // on the *substituted* command, after its `[...]` words have run.
        leak_free(|| unsafe {
            with_current_interp(|interp| {
                stub_argv(&[b"return", b"NATIVE"]);
                define_native(
                    b"stepped",
                    b"x",
                    b"set y $x; return [list $y $y]",
                    Some(stub_body),
                );
                assert_eq!(eval(interp, "stepped ab").1, b"NATIVE");
                assert_eq!(tcl_codegen_native_proc_dispatches(), 1);

                assert_eq!(
                    eval(
                        interp,
                        "trace add execution stepped enterstep \
                         {apply {{cmd op} {lappend ::steps $cmd}}}"
                    )
                    .0,
                    Code::Ok
                );
                assert_eq!(
                    eval(interp, "stepped ab").1,
                    b"ab ab",
                    "the traced call must run the source body"
                );
                assert_eq!(
                    eval(interp, "set ::steps").1,
                    b"{set y ab} {list ab ab} {return {ab ab}}"
                );
                assert_eq!(
                    tcl_codegen_native_proc_dispatches(),
                    1,
                    "the traced call must have run the source body"
                );
                assert_eq!(stub_calls(), 1);

                // Removing the trace restores the compiled body.
                assert_eq!(
                    eval(
                        interp,
                        "trace remove execution stepped enterstep \
                         {apply {{cmd op} {lappend ::steps $cmd}}}"
                    )
                    .0,
                    Code::Ok
                );
                assert_eq!(eval(interp, "stepped ab").1, b"NATIVE");
                assert_eq!(tcl_codegen_native_proc_dispatches(), 2);
            });
        });
    }

    /// An error out of a compiled body takes `run_proc`'s ordinary error tail
    /// — with the two gaps a compiled statement's missing `log_command_info`
    /// leaves, both pinned here so PR-B's fix has a gate that fails in both
    /// directions.
    #[test]
    fn an_error_from_the_native_body_unwinds_through_the_procedure_frame() {
        leak_free(|| unsafe {
            with_current_interp(|interp| {
                stub_argv(&[b"error", b"bad Q"]);
                define_native(b"boom", b"a", BOOM_BODY, Some(stub_body));

                assert_eq!(eval(interp, "catch {boom Q} m o").1, b"1");
                assert_eq!(eval(interp, "set m").1, b"bad Q");
                // `run_proc`'s error tail is shared by both bodies: the
                // `(procedure ...)` frame and the TIP 348 `CALL` entry are
                // appended exactly as they are for the source body.
                let info = eval(interp, "dict get $o -errorinfo").1;
                assert!(
                    info.starts_with(b"bad Q"),
                    "errorInfo seeds from the message: {}",
                    String::from_utf8_lossy(&info)
                );
                assert!(
                    info.windows(26)
                        .any(|w| w == b"(procedure \"boom\" line 1)\n"),
                    "errorInfo must carry the procedure frame: {}",
                    String::from_utf8_lossy(&info)
                );
                // LEDGERED (#1774 step 6, PR-B). A compiled statement calls
                // no `log_command_info`, and that one absence has two visible
                // consequences here, both closed by `tcl_codegen_log_command`
                // once the emitter calls it on a statement's error edge:
                //
                //  - no `while executing "<source text>"` frame, and
                //    `error_line` never advances, so the procedure frame reads
                //    `line 1` where the interpreted body reads `line 3`;
                //  - no TIP 348 `CALL` entry either. `error_stack_push_call`
                //    still runs, but it deliberately refuses to chain a `CALL`
                //    onto an error stack with no inner context yet
                //    (`reset_error_stack`), so the errorstack holds only the
                //    caller's own `INNER`.
                assert!(
                    !info.windows(16).any(|w| w == b"while executing "),
                    "unexpected `while executing` frame: {}",
                    String::from_utf8_lossy(&info)
                );
                assert_eq!(eval(interp, "dict get $o -errorstack").1, b"INNER {boom Q}");
            });
        });
    }

    /// With the statement's site logged, a compiled body's error is
    /// indistinguishable from the interpreted one — the whole point of
    /// `tcl_codegen_log_command`, proved here with a stub standing in for the
    /// emitter so the emitter has a target rather than a guess.
    #[test]
    fn a_logged_statement_site_makes_the_compiled_error_identical_to_the_source_one() {
        leak_free(|| unsafe {
            with_current_interp(|interp| {
                define_native(b"boom", b"a", BOOM_BODY, None);
                assert_eq!(eval(interp, "catch {boom Q} m o").1, b"1");
                let source_info = eval(interp, "dict get $o -errorinfo").1;
                let source_stack = eval(interp, "dict get $o -errorstack").1;

                // `error "bad $a"` is the third line of the body and the exact
                // text of the failing statement — what the emitter carries.
                stub_argv(&[b"error", b"bad Q"]);
                stub_logs(3, b"error \"bad $a\"");
                define_native(b"boom", b"a", BOOM_BODY, Some(stub_body));
                assert_eq!(eval(interp, "catch {boom Q} m o").1, b"1");
                assert_eq!(tcl_codegen_native_proc_dispatches(), 1);

                assert_eq!(eval(interp, "dict get $o -errorinfo").1, source_info);
                assert_eq!(eval(interp, "dict get $o -errorstack").1, source_stack);
                assert_eq!(
                    String::from_utf8_lossy(&source_info),
                    "bad Q\n    while executing\n\"error \"bad $a\"\"\n    \
                     (procedure \"boom\" line 3)\n    invoked from within\n\"boom Q\"",
                    "and that shared answer is tclsh 9.0.4's and 8.6.16's"
                );
            });
        });
    }

    #[test]
    fn a_logged_command_is_truncated_and_deduplicated_like_an_interpreted_one() {
        leak_free(|| unsafe {
            with_current_interp(|interp| {
                // One `log_command_bytes` protocol for both callers: the first
                // frame seeds errorInfo from the message and truncates the
                // command at 150 bytes with `...`; a second call for the same
                // error is a no-op (`already_logged`).
                let long = format!("error nope ;# {}", "x".repeat(200));
                stub_argv(&[b"error", b"nope"]);
                stub_logs(1, long.as_bytes());
                define_native(b"long", b"", b"error nope", Some(stub_body));
                assert_eq!(eval(interp, "catch {long} m o").1, b"1");
                let info = eval(interp, "dict get $o -errorinfo").1;
                let expected = format!(
                    "nope\n    while executing\n\"{}...\"\n    (procedure \"long\" line 1)",
                    &long[..150]
                );
                assert!(
                    info.starts_with(expected.as_bytes()),
                    "{}",
                    String::from_utf8_lossy(&info)
                );
            });
        });
    }

    #[test]
    fn the_native_path_holds_one_activation_per_level_and_refuses_at_the_bound() {
        // `run_native_body` holds the activation the eval loop would hold, and
        // the body's own `tcl_invoke_argv` holds one more: two eval-depth
        // units per Tcl level, the same as the interpreted plain-recursion
        // shape. A level that cost more would fail sooner and one that cost
        // less would fail later, so the reached depth pins the accounting.
        leak_free(|| unsafe {
            with_current_interp(|interp| {
                stub_argv(&[b"recur"]);
                define_native(b"recur", b"", b"return source", Some(stub_body));

                assert_eq!(eval(interp, "catch {recur} m").1, b"1");
                assert_eq!(
                    eval(interp, "set m").1,
                    b"too many nested evaluations (infinite loop?)",
                    "the refusal is the eval loop's own message"
                );
                assert_eq!(
                    tcl_codegen_native_proc_dispatches(),
                    63,
                    "the outermost script and the `catch` body cost one \
                     eval-depth unit each, leaving 126 of the native bound's \
                     128 for 63 levels at two apiece"
                );
            });
        });
    }

    /// A completion that comes back with no result answers with the empty
    /// string, never the caller's.
    ///
    /// `Tcl_EvalEx` resets the result at entry and the eval loop can skip that
    /// because its first command always sets one. A compiled body cannot make
    /// that promise — `tcl_codegen_var_set` and `tcl_codegen_puts` set no
    /// interpreter result — so `run_native_body` resets it instead. Without
    /// that a `puts`-ending body would answer with whatever the caller's last
    /// command left, which is the kind of wrong answer no result assertion on
    /// the body itself would catch.
    #[test]
    fn a_native_entry_that_writes_no_result_answers_with_the_empty_string() {
        leak_free(|| unsafe {
            with_current_interp(|interp| {
                define_native(b"quiet", b"", b"return source", Some(stub_no_result));
                assert_eq!(eval(interp, "set marker abc"), (Code::Ok, b"abc".to_vec()));
                assert_eq!(eval(interp, "quiet"), (Code::Ok, Vec::new()));
                assert_eq!(stub_calls(), 1);
                // The caller's own live result is the interesting case: the
                // list's first element sets it, and the second must not report
                // it back.
                assert_eq!(
                    eval(interp, "list [set marker] [quiet]").1,
                    b"abc {}".to_vec()
                );
            });
        });
    }

    /// A compiled `return` has to record the pending return state, because
    /// the return boundary consumes it whether or not anything set it.
    ///
    /// `catch {return -level 2 …}` leaves `(level 2, code error)` behind:
    /// `catch` reports the code without crossing a boundary, so nothing
    /// decrements it. An entry that then completes with `Code::Return` and no
    /// state write has its level counted down from 2 instead of 1, and the
    /// call propagates `Code::Return` to its caller instead of returning `v`.
    /// tclsh 9.0.4 and 8.6.16 both answer `0 v` here.
    #[test]
    fn a_compiled_return_records_the_state_the_return_command_records() {
        leak_free(|| unsafe {
            with_current_interp(|interp| {
                define_native(b"p", b"", b"return source", Some(stub_plain_return));
                define_native(b"bad", b"", b"return source", Some(stub_return_without_state));

                // Poison the pending state exactly as a caught deferred
                // return does, then ask each entry for its answer.
                let poison = "catch {return -level 2 -code error boom}";
                assert_eq!(eval(interp, poison).1, b"2");
                assert_eq!(eval(interp, "list [catch {p} m] $m").1, b"0 v");

                assert_eq!(eval(interp, poison).1, b"2");
                assert_eq!(
                    eval(interp, "list [catch {bad} m] $m").1,
                    b"2 v",
                    "without the state write the boundary consumes the stale \
                     level and the call propagates a return"
                );
            });
        });
    }

    #[test]
    fn the_dispatch_counter_reports_a_missing_interpreter_distinctly() {
        tcl_runtime_set_current_interp(ptr::null_mut());
        assert_eq!(tcl_codegen_native_proc_dispatches(), -1);
    }
}
