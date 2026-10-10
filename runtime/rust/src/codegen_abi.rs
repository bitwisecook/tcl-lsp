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
//! source-evaluation tier, which boxes a Tcl string from the module's data
//! section and hands it to the runtime. Conditions use the completion-bearing
//! `tcl_codegen_expr_bool` to retain Guest and Host outcomes independently:
//!
//! ```text
//! command   :  code = tcl_eval_code(tcl_obj_new_string(off, len)); …dispatch on code…
//! condition :  status = tcl_codegen_expr_bool(owned_expr, completion, truth); …
//! ```
//!
//! The emitted control flow inspects the completion `code` a leaf command
//! returns and honours it — an `error` / `return` unwinds the function, and a
//! `break` / `continue` follows the appropriate enclosing loop boundary. A
//! condition's Guest completion remains independent of its success-only truth
//! output. Host refusal writes neither completion nor truth.
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

use crate::interp::native_operation_currency::NativeOperationScope;
#[cfg(test)]
use crate::interp::obj_bytes;
use crate::interp::{Interp, NativeProcEntry};
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
/// A reached host refusal writes no Tcl completion output.
pub const TCL_INVOKE_ABI_HOST_REFUSED: i32 = -6;
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
unsafe fn write_detached_completion(out: *mut TclCompletionAbi, completion: TclCompletionAbi) {
    // SAFETY: detached transport has no interpreter or entered Guest producer.
    unsafe { out.write(completion) };
}

/// Transport adapter over the shared entered-operation and completion owners.
/// The retained original interpreter, rather than mutable TLS after a callback,
/// supplies every getter, reached effect and completion in this operation.
pub(crate) struct CodegenOperation {
    pub(crate) interpreter: Interp,
    scope: NativeOperationScope,
}

impl CodegenOperation {
    pub(crate) fn enter() -> Result<Self, i32> {
        // SAFETY: the host retains its installed interpreter through ABI entry.
        let Some(mut interpreter) = (unsafe { current_interp().as_ref() }).cloned() else {
            return Err(TCL_INVOKE_ABI_NO_CURRENT_INTERP);
        };
        let scope = NativeOperationScope::enter(&interpreter).map_err(|first| {
            interpreter.refuse_native_execution(first);
            TCL_INVOKE_ABI_HOST_REFUSED
        })?;
        Ok(Self { interpreter, scope })
    }

    pub(crate) fn ensure_current(&self) -> Result<(), i32> {
        self.scope
            .currency()
            .ensure_current_or_refuse()
            .map_err(|_| TCL_INVOKE_ABI_HOST_REFUSED)
    }

    fn scalar(
        &mut self,
        value: *mut TclObj,
        kind: tcl_syntax::scalar_getter::NativeScalarGetterKind,
    ) -> Result<tcl_syntax::scalar_getter::NativeScalarGetterValue, i32> {
        self.ensure_current()?;
        let result = crate::capi::read_scalar_for_interpreter(&self.interpreter, value, kind);
        self.ensure_current()?;
        result.map_err(|error| {
            error.publish(&mut self.interpreter);
            self.ensure_current()
                .map_or_else(|status| status, |_| TCL_VALUE_GET_ERROR)
        })
    }

    fn original_string(&mut self, value: *mut TclObj) -> Result<std::rc::Rc<[u8]>, i32> {
        self.ensure_current()?;
        let result = self.interpreter.native_object_string_bytes(value);
        self.ensure_current()?;
        result.map_err(|error| {
            self.interpreter.report_cmd_error(error.into());
            self.ensure_current().map_or_else(|status| status, |_| 1)
        })
    }

    pub(crate) fn finish_code(&self, code: crate::interp::Code) -> i32 {
        self.ensure_current().map_or_else(
            |status| status,
            |_| i32::try_from(code.as_int()).unwrap_or(1),
        )
    }

    /// Capture while the activation still owns its Guest error state, then
    /// unwind it before final output publication. All cleanup remains reached.
    unsafe fn settle(
        &mut self,
        code: crate::interp::Code,
        mut activation: Option<AbiActivation>,
        out: *mut TclCompletionAbi,
        status: i32,
    ) -> i32 {
        let captured = crate::state_traits::capture_completion_checked(
            &mut self.interpreter,
            self.scope.currency(),
            code,
        );
        if let Some(active) = activation.as_mut() {
            active.code = code;
        }
        drop(activation);
        let completion = match captured {
            Ok(completion) => completion_abi(completion),
            Err(_) => return TCL_INVOKE_ABI_HOST_REFUSED,
        };
        if self.ensure_current().is_err() {
            // SAFETY: capture owns each non-null handle until publication.
            unsafe {
                obj::decr_ref_count(completion.result);
                obj::decr_ref_count(completion.options);
            }
            return TCL_INVOKE_ABI_HOST_REFUSED;
        }
        // SAFETY: the ABI caller provided writable output storage.
        unsafe { out.write(completion) };
        status
    }
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
    let Ok(mut operation) = CodegenOperation::enter() else {
        return 1;
    };
    let code = operation.interpreter.init_library();
    if operation.ensure_current().is_err() {
        return TCL_INVOKE_ABI_HOST_REFUSED;
    }
    i32::from(code == crate::interp::Code::Error)
}

/// `tcl_runtime_identity(out, capacity) -> i32` — what this runtime states of
/// itself, in the shape a compiled artefact states its own: the current
/// interp's pinned environment, release, build and package floors, this
/// build's ABI version, intrinsic-table hash and embedded library revision,
/// and no pack facts, encoded as `ArtefactIdentityManifest::to_bytes` does. A
/// host that holds a module's own manifest
/// (`ArtefactIdentityManifest::from_wasm`) compares the two before it links
/// them, and refuses one built for another ABI or intrinsic table.
///
/// Returns the encoding's byte length, and writes it to `out` only when
/// `capacity` holds it, so a host asks once with a null buffer for the size.
/// Returns `0` when no interp is current.
///
/// # Safety
/// `out` must be null or reference `capacity` writable bytes; `capacity` must
/// be non-negative.
#[no_mangle]
pub unsafe extern "C" fn tcl_runtime_identity(out: *mut u8, capacity: i32) -> i32 {
    let interp = current_interp();
    if interp.is_null() {
        return 0;
    }
    // SAFETY: `interp` is the live current interp set by the bootstrap.
    let bytes = unsafe { (*interp).held_identity() }.to_bytes();
    let Ok(len) = i32::try_from(bytes.len()) else {
        return 0;
    };
    if !out.is_null() && capacity >= len {
        // SAFETY: `out` references `capacity >= len` writable bytes.
        unsafe { ptr::copy_nonoverlapping(bytes.as_ptr(), out, bytes.len()) };
    }
    len
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

fn unexpected_scalar_output(interpreter: &mut Interp) -> i32 {
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
        tcl_registry::native_boolean_truth::NativeBooleanTruthPurpose::from_abi(purpose_tag)
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
        tcl_registry::native_boolean_truth::NativeBooleanExpressionResultProduction::from_abi(
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
    let Ok(mut operation) = CodegenOperation::enter() else {
        return 1;
    };
    let entered = operation.interpreter.codegen_activation_enter();
    if operation.ensure_current().is_err() {
        if entered {
            operation
                .interpreter
                .codegen_activation_leave(crate::interp::Code::Error);
        }
        return TCL_INVOKE_ABI_HOST_REFUSED;
    }
    i32::from(!entered)
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
    let Ok(mut operation) = CodegenOperation::enter() else {
        return;
    };
    operation.interpreter.codegen_frame_push();
    let _ = operation.ensure_current();
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
            Err(error) => operation.interpreter.report_cmd_error(error.into()),
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
            }
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
            }
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
                unsafe { (*interp).report_expr_error(err) };
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
    let command = obj::Owned::fresh(new_string_bytes(b"puts"));
    let code = crate::cmd_chan::puts_cmd(
        &mut operation.interpreter,
        &[command.as_ptr(), value.as_ptr()],
    );
    drop(command);
    drop(value);
    operation.finish_code(code)
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
    let mut operation = match CodegenOperation::enter() {
        Ok(operation) => operation,
        Err(status) => {
            return if status == TCL_INVOKE_ABI_HOST_REFUSED {
                status
            } else {
                1
            }
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
    let Ok(mut operation) = CodegenOperation::enter() else {
        return;
    };
    let source = unsafe { input_bytes(src_ptr, src_len) };
    let line = u32::try_from(line).unwrap_or(1).max(1);
    operation
        .interpreter
        .log_evaluated_command_bytes(line, source);
    let _ = operation.ensure_current();
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
    let Ok(mut operation) = CodegenOperation::enter() else {
        return;
    };
    let level = usize::try_from(level).unwrap_or(0);
    operation
        .interpreter
        .set_return_state(level, crate::interp::Code::from_int(code));
    let _ = operation.ensure_current();
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
/// [`tcl_obj_release`]. (Completion codes are discarded in this tier: faithful
/// `return`/`break`/`error` propagation is not implemented in this ABI tier.)
///
/// # Safety
/// `script` must be a live `rc 0` object from [`tcl_obj_new_string`]; the current
/// interp (if set) must be live.
#[no_mangle]
pub unsafe extern "C" fn tcl_eval(script: *mut TclObj) -> *mut TclObj {
    // This legacy input transfers a fresh rc-0 object, including refusal paths.
    let script = obj::Owned::fresh(script);
    let mut operation = match CodegenOperation::enter() {
        Ok(operation) => operation,
        Err(TCL_INVOKE_ABI_NO_CURRENT_INTERP) => {
            return obj::Owned::fresh(new_string_bytes(b"")).into_raw();
        }
        Err(_) => return ptr::null_mut(),
    };
    let source = match operation.original_string(script.as_ptr()) {
        Ok(source) => source,
        Err(_) => return ptr::null_mut(),
    };
    operation.interpreter.eval_str(&source);
    let result = obj::Owned::retain(operation.interpreter.get_obj_result());
    drop(script);
    if operation.ensure_current().is_err() {
        return ptr::null_mut();
    }
    result.into_raw()
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
    let script = obj::Owned::fresh(script);
    let mut operation = match CodegenOperation::enter() {
        Ok(operation) => operation,
        // Explicit detached compatibility: no interpreter exists to execute.
        Err(TCL_INVOKE_ABI_NO_CURRENT_INTERP) => return 0,
        Err(status) => return status,
    };
    let source = match operation.original_string(script.as_ptr()) {
        Ok(source) => source,
        Err(status) => return status,
    };
    let code = operation.interpreter.eval_str(&source);
    drop(script);
    operation.finish_code(code)
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
    let activation = unsafe { AbiActivation::enter(&mut operation.interpreter) };
    if activation.is_none() {
        return unsafe {
            operation.settle(crate::interp::Code::Error, None, out, TCL_INVOKE_ABI_OK)
        };
    }
    let code = operation.interpreter.dispatch(words);
    unsafe { operation.settle(code, activation, out, TCL_INVOKE_ABI_OK) }
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
/// **Adopts (frees)** the `rc 0` `expr`. On a genuine Guest expression error — or in a build
/// without the numeric tower (no `expr` evaluator) — yields `0`. The wasm
/// runtime now links libtommath (`build.rs`), so `have_tommath` is set and this
/// uses the real evaluator there too. This compatibility export projects
/// genuine Guest failure to false; full compiler conditions use the completion-
/// bearing `tcl_codegen_expr_bool`. Host refusal returns `-6` and stays in the
/// independent interpreter channel.
///
/// # Safety
/// `expr` must be a live `rc 0` object from [`tcl_obj_new_string`]; the current
/// interp (if set) must be live.
#[no_mangle]
pub unsafe extern "C" fn tcl_expr_bool(expr: *mut TclObj) -> i32 {
    let expr = obj::Owned::fresh(expr);
    let mut operation = match CodegenOperation::enter() {
        Ok(operation) => operation,
        Err(TCL_INVOKE_ABI_NO_CURRENT_INTERP) => return 0,
        Err(status) => return status,
    };
    let truth = unsafe { expr_bool_impl(&mut operation.interpreter, expr.as_ptr()) };
    drop(expr);
    operation
        .ensure_current()
        .map_or_else(|status| status, |_| truth)
}

/// The expression-condition path, present only with the numeric tower.
///
/// # Safety
/// `expr` must be live; `interp` is the retained original operation target.
#[cfg(have_tommath)]
unsafe fn expr_bool_impl(interp: &mut Interp, expr: *mut TclObj) -> i32 {
    let ok = crate::builtins::eval_bool_expr(interp, expr);
    i32::from(matches!(ok, Ok(true)))
}

/// Without the numeric tower there is no `expr` evaluator (the `expr` module is
/// `have_tommath`-gated), so conditions evaluate false. This fallback only
/// applies to a build that omits the tower (e.g. a wasm build where
/// `clang`/libtommath was unavailable and `build.rs` degraded the backend off).
/// The export still exists so emitted modules link.
///
/// # Safety
/// Trivially safe (dereferences nothing).
#[cfg(not(have_tommath))]
unsafe fn expr_bool_impl(_interp: &mut Interp, _expr: *mut TclObj) -> i32 {
    0
}

/// Query retained operational failures before any guest catch/finally transition.
/// This does not reset the failure or publish a catchable Tcl completion.
#[unsafe(no_mangle)]
pub extern "C" fn tcl_codegen_host_refusal_pending() -> i32 {
    // SAFETY: the registered current interpreter is live until its host clears it.
    unsafe { current_interp().as_ref() }
        .map_or(0, |interp| i32::from(interp.host_refusal_pending()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capi::{tcl_runtime_create_interp, tcl_runtime_delete_interp};
    use crate::counters;
    use crate::interp::Code;
    use std::cell::RefCell;
    use std::rc::Rc;
    use tcl_dialect::TclVersion;
    use tcl_host_native::NativeHost;
    use tcl_platform::{Capabilities, Clock, Env, Filesystem, Host, Process, StdIo};
    use tcl_runtime_api::codegen_abi::{NATIVE_PROC_STATUS_DECLINED, NATIVE_PROC_STATUS_RAN};
    use tcl_runtime_api::guard::GuardDomain;

    #[test]
    fn reached_unicode_refusal_keeps_argv_completion_output_untouched() {
        leak_free(|| unsafe {
            let interp = tcl_runtime_create_interp();
            tcl_runtime_set_current_interp(interp);
            let words = [b"string".as_slice(), b"toupper", b"A\xff"].map(|bytes| {
                tcl_obj_new_string_owned(bytes.as_ptr(), i32::try_from(bytes.len()).unwrap())
            });
            let mut out = TclCompletionAbi {
                code: 77,
                result: core::ptr::null_mut(),
                options: core::ptr::null_mut(),
            };
            assert_eq!(
                tcl_invoke_argv(words.as_ptr(), 3, &mut out),
                TCL_INVOKE_ABI_HOST_REFUSED
            );
            assert_eq!(tcl_codegen_host_refusal_pending(), 1);
            assert_eq!(out.code, 77);
            assert!(out.result.is_null());
            assert!(out.options.is_null());
            assert_eq!((*interp).unicode_access_refusal().unwrap().valid_up_to, 1);
            for word in words {
                tcl_obj_release(word);
            }
            tcl_runtime_set_current_interp(core::ptr::null_mut());
            tcl_runtime_delete_interp(interp);
        });
    }

    #[test]
    fn reached_refusal_bypasses_capture_finally_and_rmw_read_error_recovery() {
        for script in [
            b"incr prior; catch {string toupper $raw} captured options; set after YES".as_slice(),
            b"incr prior; try {string toupper $raw} finally {set final YES}; set after YES",
            b"incr prior; set x 10; proc cb {args} {string toupper $::raw}; trace add variable x read cb; incr x; set after YES",
            b"incr prior; array set a {k 1}; proc cb {args} {string toupper $::raw}; trace add variable a(k) read cb; catch {array get a} captured options; set after YES",
        ] {
            leak_free(|| {
                let mut interp = Interp::new();
                let raw = new_string_bytes(b"A\xff");
                interp.var_set(b"raw", raw).unwrap();
                interp.var_set(b"prior", crate::obj::new_wide_int_obj(0)).unwrap();
                assert_eq!(interp.eval_str(script), Code::Error);
                assert!(interp.host_refusal_pending());
                assert_eq!(obj_bytes(interp.var_get(b"prior").unwrap()), b"1");
                for name in [b"captured".as_slice(), b"options", b"final", b"after"] {
                    assert!(!interp.var_exists(name));
                }
                if let Some(x) = interp.var_get(b"x") { assert_eq!(obj_bytes(x), b"10"); }
            });
        }
    }

    #[test]
    fn byte_guest_error_metadata_is_captured_without_host_refusal() {
        fn fail(interp: &mut Interp, _argv: &[*mut TclObj]) -> Code {
            interp.report_cmd_error(tcl_cmd_core::CmdError::with_byte_error_details(
                b"BOOM\xff".to_vec(),
                b"RAW \xfe".to_vec(),
                Some(b"TRACE\xfd".to_vec()),
                Some(7),
            ))
        }
        leak_free(|| {
            let mut interp = Interp::new();
            interp.register_builtin(b"rawfail", fail);
            assert_eq!(interp.eval_str(b"catch {rawfail} r opts"), Code::Ok);
            assert_eq!(interp.result_bytes(), b"1");
            assert!(!interp.host_refusal_pending());
            assert_eq!(obj_bytes(interp.var_get(b"r").unwrap()), b"BOOM\xff");
            assert_eq!(interp.eval_str(b"dict get $opts -errorcode"), Code::Ok);
            assert_eq!(interp.result_bytes(), b"RAW \xfe");
            assert_eq!(interp.eval_str(b"dict get $opts -errorinfo"), Code::Ok);
            assert!(interp.result_bytes().starts_with(b"TRACE\xfd"));
        });
    }

    struct CaptureHost {
        native: NativeHost,
        stdout: Rc<RefCell<Vec<u8>>>,
    }

    impl StdIo for CaptureHost {
        fn write_stdout(&self, bytes: &[u8]) {
            self.stdout.borrow_mut().extend_from_slice(bytes);
        }

        fn write_stderr(&self, _bytes: &[u8]) {}
    }

    impl Host for CaptureHost {
        fn capabilities(&self) -> Capabilities {
            self.native.capabilities()
        }

        fn clock(&self) -> &dyn Clock {
            self.native.clock()
        }

        fn stdio(&self) -> &dyn StdIo {
            self
        }

        fn env(&self) -> &dyn Env {
            self.native.env()
        }

        fn filesystem(&self) -> Option<&dyn Filesystem> {
            self.native.filesystem()
        }

        fn process(&self) -> Option<&dyn Process> {
            self.native.process()
        }
    }

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
        let characters = unsafe { (*interp).native_invocation_dialect() }
            .characters
            .expect("test runtime has a selected character protocol");
        let identity = GuardIdentity::registry_intrinsic_with_semantics(
            intrinsic.stable_id(),
            intrinsic.guard_semantics_key_for_characters(characters),
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
            let native = (*interp).native_invocation_dialect();
            assert!(
                resolve_intrinsic_argv(
                    &mut CodegenOperation::enter().unwrap(),
                    &words,
                    native.authoring_query()
                )
                .unwrap()
                .is_some(),
                "actual Jim StringLength form is retained"
            );
            let identity = GuardIdentity::registry_intrinsic_with_semantics(
                IntrinsicId::StringLength.stable_id(),
                IntrinsicId::StringLength
                    .guard_semantics_key_for_characters(native.characters.unwrap()),
            );
            let direct = (*interp).prepare_command_guard(
                b"string",
                identity,
                GuardDomains::one(GuardDomain::Interpreter),
            );
            assert!(direct.is_ok(), "native stock string identity: {direct:?}");
            tcl_codegen_guard_release(direct.unwrap().raw());
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
            assert_eq!(status, TCL_INVOKE_ABI_HOST_REFUSED);
            assert_eq!(
                (*interp).native_access_refusal(),
                Some(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "native string count cache origin",
                    ),
                )
            );
            tcl_completion_release(&mut completion);
            assert_eq!((*interp).eval_str(b""), Code::Ok);
            let c86_words = [
                owned_word(b"string"),
                owned_word(b"length"),
                owned_word("é🙂".as_bytes()),
            ];
            let (status, mut completion) = invoke_intrinsic(IntrinsicId::StringLength, &c86_words);
            assert_eq!(status, TCL_INVOKE_ABI_OK);
            assert_eq!(obj_bytes(completion.result), b"3");
            tcl_completion_release(&mut completion);
            release_words(&words);
            release_words(&c86_words);
            tcl_runtime_set_current_interp(ptr::null_mut());
            tcl_runtime_delete_interp(interp);
        });
    }

    #[test]
    fn guarded_intrinsic_jim_character_identity_ignores_the_c_assistance_release() {
        leak_free(|| unsafe {
            let interp = tcl_runtime_create_interp();
            (*interp).set_dialect_profile(crate::environment::profile_for_dialect("jim"));
            tcl_runtime_set_current_interp(interp);
            let words = [
                owned_word(b"string"),
                owned_word(b"length"),
                owned_word("é🙂".as_bytes()),
            ];
            let domains = i32::from(GuardDomains::one(GuardDomain::Interpreter).bits());
            let c_identity = GuardIdentity::registry_intrinsic_with_semantics(
                IntrinsicId::StringLength.stable_id(),
                IntrinsicId::StringLength.guard_semantics_key((*interp).runtime_version()),
            );
            assert_eq!(
                tcl_codegen_guard_prepare(
                    IntrinsicId::StringLength.stable_id(),
                    words.as_ptr(),
                    3,
                    c_identity.namespace(),
                    c_identity.value(),
                    domains,
                ),
                0
            );
            let native = (*interp).native_invocation_dialect();
            assert!(
                resolve_intrinsic_argv(
                    &mut CodegenOperation::enter().unwrap(),
                    &words,
                    native.authoring_query()
                )
                .unwrap()
                .is_some(),
                "actual Jim StringLength form is retained"
            );
            let identity = GuardIdentity::registry_intrinsic_with_semantics(
                IntrinsicId::StringLength.stable_id(),
                IntrinsicId::StringLength
                    .guard_semantics_key_for_characters(native.characters.unwrap()),
            );
            let direct = (*interp).prepare_command_guard(
                b"string",
                identity,
                GuardDomains::one(GuardDomain::Interpreter),
            );
            assert!(direct.is_ok(), "native stock string identity: {direct:?}");
            tcl_codegen_guard_release(direct.unwrap().raw());
            let token = prepare_intrinsic_guard(IntrinsicId::StringLength, &words, domains);
            assert_ne!(token, 0);
            assert_eq!(
                tcl_codegen_guard_check(
                    token,
                    IntrinsicId::StringLength.stable_id(),
                    words.as_ptr(),
                    3
                ),
                1
            );
            let (status, mut completion) = invoke_intrinsic(IntrinsicId::StringLength, &words);
            assert_eq!(status, TCL_INVOKE_ABI_OK);
            assert_eq!(obj_bytes(completion.result), b"2");
            tcl_completion_release(&mut completion);
            tcl_codegen_guard_release(token);
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

    /// A guard is bound to `string`'s command token: defining, renaming or
    /// aliasing another command leaves the exact check an emitted module makes
    /// answering one, and rebinding `string` itself answers zero.
    #[test]
    fn guarded_intrinsic_guards_survive_unrelated_command_mutation() {
        leak_free(|| unsafe {
            let interp = tcl_runtime_create_interp();
            tcl_runtime_set_current_interp(interp);
            let words = [
                owned_word(b"string"),
                owned_word(b"length"),
                owned_word(b"abc"),
            ];
            let base_domains = i32::from(
                GuardDomains::one(GuardDomain::CommandEnvironment)
                    .with(GuardDomain::Namespace)
                    .with(GuardDomain::CommandTrace)
                    .with(GuardDomain::Interpreter)
                    .bits(),
            );
            let token = prepare_intrinsic_guard(IntrinsicId::StringLength, &words, base_domains);
            assert_ne!(token, 0);
            let check = || {
                tcl_codegen_guard_check(
                    token,
                    IntrinsicId::StringLength.stable_id(),
                    words.as_ptr(),
                    i32::try_from(words.len()).unwrap(),
                )
            };
            assert_eq!(check(), 1);
            for script in [
                &b"proc unrelated {} {return 1}"[..],
                b"rename unrelated other",
                b"interp alias {} alias_of_other {} other",
            ] {
                assert_eq!(tcl_eval_code(box_str(script)), 0);
                assert_eq!(check(), 1, "{}", String::from_utf8_lossy(script));
            }
            assert_eq!(
                tcl_eval_code(box_str(b"proc string args {return shadow}")),
                0
            );
            assert_eq!(check(), 0);
            tcl_codegen_guard_release(token);
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
    fn compiled_puts_uses_channel_conversion_and_balances_its_owned_value() {
        leak_free(|| unsafe {
            let stdout = Rc::new(RefCell::new(Vec::new()));
            let interp = tcl_runtime_create_interp();
            (*interp).set_runtime_version(TclVersion::V9_0);
            (*interp).set_host(Rc::new(CaptureHost {
                native: NativeHost::new(),
                stdout: Rc::clone(&stdout),
            }));
            tcl_runtime_set_current_interp(interp);

            assert_eq!(
                tcl_eval_code(box_str(b"fconfigure stdout -translation binary")),
                0
            );
            assert_eq!(tcl_codegen_puts(owned_word(&[0xff, b'A'])), 0);
            assert_eq!(&*stdout.borrow(), &[0xff, b'A', b'\n']);

            assert_eq!(
                tcl_eval_code(box_str(
                    b"fconfigure stdout -translation lf -encoding iso8859-1 -profile strict"
                )),
                0
            );
            assert_eq!(tcl_codegen_puts(owned_word("A\u{0178}B".as_bytes())), 1);
            assert_eq!(&*stdout.borrow(), &[0xff, b'A', b'\n', b'A']);
            assert_eq!(
                (*interp).result_bytes(),
                b"error writing \"stdout\": invalid or incomplete multibyte or wide character"
            );
            assert_eq!(
                (*interp).error_code(),
                b"POSIX EILSEQ {invalid or incomplete multibyte or wide character}"
            );

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
        // naming.runtime.original-procedure-error-context
        // docs/design/analysis/name-resolution-proofs/runtime-original-procedure-error-context.md
        // The default ABI interpreter selects C9.0. Native295's ordinary-miss
        // public errorCode differs from the namespace-origin error producer.
        leak_free(|| unsafe {
            let interp = tcl_runtime_create_interp();
            assert_eq!(
                (*interp).native_invocation_dialect().tcl_version,
                Some(tcl_dialect::TclVersion::V9_0)
            );
            tcl_runtime_set_current_interp(interp);
            let words = [owned_word(b"definitely_missing_command")];

            let mut completion = invoke(&words);
            assert_eq!(completion.code, 1);
            assert!(obj_bytes(completion.result).starts_with(b"invalid command name"));
            assert_eq!(option(&completion, b"-code"), b"1");
            assert_eq!(option(&completion, b"-level"), b"0");
            assert_eq!(
                option(&completion, b"-errorcode"),
                b"TCL LOOKUP COMMAND definitely_missing_command"
            );
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

    fn scalar_interpreter(environment: &str) -> Interp {
        Interp::with_native_core(
            crate::interp::default_host(),
            tcl_registry::model::resolve_environment(environment).unit_profile(),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap()
    }

    #[test]
    fn boxed_primitive_reads_share_actual_environment_and_raw_boolean_projection() {
        // Software adapters: naming.numeric.original-capi-scalar-publication-width
        // docs/design/analysis/name-resolution-proofs/numeric-original-capi-scalar-publication-width.md
        leak_free(|| unsafe {
            for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
                let mut interp = scalar_interpreter(environment);
                tcl_runtime_set_current_interp(&mut interp);
                let wide = owned_word(b"0x10");
                let double = owned_word(b"1.5");
                assert_eq!(get_wide(wide), (TCL_VALUE_GET_OK, 16), "{environment}");
                assert_eq!(get_double(double), (TCL_VALUE_GET_OK, 1.5), "{environment}");
                assert_eq!(obj_bytes(wide), b"0x10");
                assert_eq!(obj_bytes(double), b"1.5");
                release_words(&[wide, double]);
                for number in [17, 4_294_967_296] {
                    let primitive = obj::Owned::fresh(obj::new_wide_int_obj(number));
                    let boxed = obj::Owned::fresh(obj::new_wide_int_obj(number));
                    let mut raw = 777;
                    assert_eq!(
                        crate::capi::Tcl_GetBooleanFromObj(
                            &mut interp,
                            primitive.as_ptr(),
                            &mut raw,
                        ),
                        0
                    );
                    let expected = if environment == "jim" {
                        if number == 17 {
                            17
                        } else {
                            0
                        }
                    } else {
                        1
                    };
                    assert_eq!(raw, expected, "{environment}, {number}");
                    assert_eq!(
                        get_bool(boxed.as_ptr()),
                        (TCL_VALUE_GET_OK, i32::from(raw != 0))
                    );
                    assert!(!interp.host_refusal_pending());
                }
                tcl_runtime_set_current_interp(ptr::null_mut());
            }
        });
    }

    struct ScalarNumericHost {
        actual: Rc<dyn Host>,
        interpreter: RefCell<Option<Interp>>,
        double_calls: std::cell::Cell<usize>,
        abi_calls: std::cell::Cell<usize>,
        change_abi_context: std::cell::Cell<bool>,
        abi_query_fails: std::cell::Cell<bool>,
        abi_available: bool,
        change_context: bool,
    }

    impl Host for ScalarNumericHost {
        fn capabilities(&self) -> Capabilities {
            self.actual.capabilities()
        }
        fn clock(&self) -> &dyn Clock {
            self.actual.clock()
        }
        fn stdio(&self) -> &dyn StdIo {
            self.actual.stdio()
        }
        fn env(&self) -> &dyn Env {
            self.actual.env()
        }
        fn numeric_environment(&self) -> Option<&dyn tcl_platform::NumericEnvironment> {
            self.abi_available.then_some(self)
        }
    }

    impl ScalarNumericHost {
        fn actual(&self) -> &dyn tcl_platform::NumericEnvironment {
            self.actual
                .numeric_environment()
                .expect("actual native numeric environment")
        }
        fn change_and_restore_context(&self) {
            let mut interp = self.interpreter.borrow().as_ref().unwrap().clone();
            let original = interp.runtime_context();
            let mut changed = original.clone();
            changed.packages = vec![("boxed-scalar-currency".to_owned(), "1.0".to_owned())];
            interp.pin_context(&changed).unwrap();
            interp.pin_context(&original).unwrap();
        }
    }

    impl tcl_platform::NumericEnvironment for ScalarNumericHost {
        fn c_integer_abi(
            &self,
        ) -> Result<tcl_platform::NativeCIntegerAbi, tcl_platform::NumericEnvironmentUnavailable>
        {
            let result = self.actual().c_integer_abi();
            self.abi_calls.set(self.abi_calls.get() + 1);
            if self.change_abi_context.get() {
                self.change_and_restore_context();
            }
            if self.abi_query_fails.get() {
                Err(tcl_platform::NumericEnvironmentUnavailable::Target)
            } else {
                result
            }
        }
        fn state(
            &self,
        ) -> Result<tcl_platform::NumericErrorState, tcl_platform::NumericEnvironmentUnavailable>
        {
            self.actual().state()
        }
        fn reset(&self) -> Result<(), tcl_platform::NumericEnvironmentUnavailable> {
            self.actual().reset()
        }
        fn unsigned_c84(
            &self,
            input: &[u8],
            offset: usize,
            long: bool,
        ) -> Result<
            tcl_platform::UnsignedNumericConversion,
            tcl_platform::NumericEnvironmentUnavailable,
        > {
            self.actual().unsigned_c84(input, offset, long)
        }
        fn unsigned(
            &self,
            input: &[u8],
            offset: usize,
            base: u32,
        ) -> Result<
            tcl_platform::UnsignedNumericConversion,
            tcl_platform::NumericEnvironmentUnavailable,
        > {
            self.actual().unsigned(input, offset, base)
        }
        fn signed_long(
            &self,
            input: &[u8],
            base: u32,
        ) -> Result<
            tcl_platform::SignedNumericConversion,
            tcl_platform::NumericEnvironmentUnavailable,
        > {
            self.actual().signed_long(input, base)
        }
        fn double(
            &self,
            input: &[u8],
            reset: bool,
        ) -> Result<
            tcl_platform::DoubleNumericConversion,
            tcl_platform::NumericEnvironmentUnavailable,
        > {
            let result = self.actual().double(input, reset);
            self.double_calls.set(self.double_calls.get() + 1);
            if self.change_context {
                self.change_and_restore_context();
            }
            result
        }
    }

    fn scalar_host(
        interp: &Interp,
        abi_available: bool,
        change_context: bool,
    ) -> Rc<ScalarNumericHost> {
        let host = Rc::new(ScalarNumericHost {
            actual: interp.host(),
            interpreter: RefCell::new(None),
            double_calls: std::cell::Cell::new(0),
            abi_calls: std::cell::Cell::new(0),
            change_abi_context: std::cell::Cell::new(false),
            abi_query_fails: std::cell::Cell::new(false),
            abi_available,
            change_context,
        });
        interp.set_host(host.clone());
        host
    }

    #[test]
    fn boxed_primitive_reads_keep_missing_abi_and_first_host_terminal() {
        // Software owner: naming.numeric.original-capi-scalar-publication-width
        // docs/design/analysis/name-resolution-proofs/numeric-original-capi-scalar-publication-width.md
        leak_free(|| unsafe {
            for first_host in [false, true] {
                let mut interp = scalar_interpreter("tcl8.4");
                let host = scalar_host(&interp, false, false);
                interp.set_result_bytes(b"PRIOR\0\xff");
                if first_host {
                    interp.refuse_host_command("original first refusal");
                }
                let first = interp.native_execution_refusal();
                let value = owned_word(b"17");
                let before = obj::native_object_snapshot(value).unwrap();
                tcl_runtime_set_current_interp(&mut interp);
                assert_eq!(get_wide(value), (TCL_VALUE_GET_ERROR, i64::MIN));
                let double = get_double(value);
                assert_eq!(double.0, TCL_VALUE_GET_ERROR);
                assert!(double.1.is_nan());
                assert_eq!(get_bool(value), (TCL_VALUE_GET_ERROR, -1));
                assert_eq!(host.double_calls.get(), 0);
                assert_eq!(obj::native_object_snapshot(value).unwrap(), before);
                assert_eq!(interp.result_bytes(), b"PRIOR\0\xff");
                assert!(interp.host_refusal_pending());
                if first_host {
                    assert_eq!(interp.native_execution_refusal(), first);
                }
                release_words(&[value]);
                tcl_runtime_set_current_interp(ptr::null_mut());
            }
        });
    }

    #[test]
    fn boxed_primitive_reads_refuse_foreign_and_restored_engine_issuers() {
        // Software owner: naming.numeric.original-capi-scalar-publication-width
        // docs/design/analysis/name-resolution-proofs/numeric-original-capi-scalar-publication-width.md
        leak_free(|| unsafe {
            for foreign in [false, true] {
                let mut owner = scalar_interpreter("tcl8.6");
                let mut other = scalar_interpreter("tcl8.6");
                let value = owned_word(b"17");
                crate::capi::bind_scalar_getter_context(&owner, value).unwrap();
                let before = obj::native_object_snapshot(value).unwrap();
                let original = owner.runtime_context();
                if !foreign {
                    let mut changed = original.clone();
                    changed.packages = vec![("boxed-scalar-owner".to_owned(), "1.0".to_owned())];
                    owner.pin_context(&changed).unwrap();
                    owner.pin_context(&original).unwrap();
                }
                let selected = if foreign { &mut other } else { &mut owner };
                selected.set_result_bytes(b"UNCHANGED");
                tcl_runtime_set_current_interp(selected);
                assert_eq!(get_wide(value), (TCL_VALUE_GET_ERROR, i64::MIN));
                assert_eq!(get_bool(value), (TCL_VALUE_GET_ERROR, -1));
                assert!(selected.host_refusal_pending());
                assert_eq!(selected.result_bytes(), b"UNCHANGED");
                assert_eq!(obj::native_object_snapshot(value).unwrap(), before);
                release_words(&[value]);
                tcl_runtime_set_current_interp(ptr::null_mut());
            }
        });
    }

    #[test]
    fn boxed_primitive_host_change_and_restore_stops_before_cache_and_output() {
        // Software currency: naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        leak_free(|| unsafe {
            for environment in ["tcl8.4", "jim"] {
                let mut interp = scalar_interpreter(environment);
                let host = scalar_host(&interp, true, true);
                *host.interpreter.borrow_mut() = Some(interp.clone());
                let context = interp.runtime_context();
                let profile = interp.dialect_profile();
                interp.set_result_bytes(b"PRIOR\0\xff");
                let prior = interp.get_obj_result();
                let value = owned_word(b"1.5");
                let before = obj::native_object_snapshot(value).unwrap();
                tcl_runtime_set_current_interp(&mut interp);
                let outcome = get_double(value);
                assert_eq!(outcome.0, TCL_VALUE_GET_ERROR);
                assert!(outcome.1.is_nan());
                assert_eq!(host.double_calls.get(), 1, "{environment}");
                assert_eq!(interp.runtime_context(), context);
                assert!(std::ptr::eq(interp.dialect_profile(), profile));
                let first = interp.native_execution_refusal().unwrap();
                assert_eq!(first, tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "stale entered native operation",
                    ),
                ));
                assert_eq!(obj::native_object_snapshot(value).unwrap(), before);
                assert_eq!(interp.get_obj_result(), prior);
                assert_eq!(interp.result_bytes(), b"PRIOR\0\xff");
                assert_eq!(get_bool(value), (TCL_VALUE_GET_ERROR, -1));
                assert_eq!(host.double_calls.get(), 1);
                assert_eq!(interp.native_execution_refusal(), Some(first));
                tcl_runtime_set_current_interp(ptr::null_mut());
                release_words(&[value]);
                host.interpreter.borrow_mut().take();
            }
        });
    }

    #[test]
    fn scalar_abi_query_change_and_restore_cannot_issue_or_refresh_a_receipt() {
        // Software currency: naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        leak_free(|| unsafe {
            for retained in [false, true] {
                for query_fails in [false, true] {
                    let mut interp = scalar_interpreter("tcl8.6");
                    let host = scalar_host(&interp, true, false);
                    *host.interpreter.borrow_mut() = Some(interp.clone());
                    let value = owned_word(b"1.5");
                    if retained {
                        crate::capi::bind_scalar_getter_context(&interp, value).unwrap();
                    }
                    let before = obj::native_object_snapshot(value).unwrap();
                    let context = interp.runtime_context();
                    let calls = host.abi_calls.get();
                    host.change_abi_context.set(true);
                    host.abi_query_fails.set(query_fails);
                    interp.set_result_bytes(b"PRIOR");
                    let error = crate::capi::probe_scalar_getter(
                        if retained { None } else { Some(&interp) },
                        value,
                        tcl_syntax::scalar_getter::NativeScalarGetterKind::Double,
                    )
                    .unwrap_err();
                    let first = interp.native_execution_refusal().unwrap();
                    assert!(
                        matches!(error, crate::capi::NativeScalarObjectAccessError::Execution(ref cause) if *cause == first)
                    );
                    assert_eq!(first, tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                        tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                            "stale entered native operation",
                        ),
                    ));
                    assert_eq!(host.abi_calls.get(), calls + 1);
                    assert_eq!(host.double_calls.get(), 0);
                    assert_eq!(interp.runtime_context(), context);
                    assert_eq!(obj::native_object_snapshot(value).unwrap(), before);
                    assert_eq!(interp.result_bytes(), b"PRIOR");
                    if !retained {
                        assert!(obj::scalar_object_context(value).unwrap().is_none());
                    }
                    tcl_runtime_set_current_interp(&mut interp);
                    let outcome = get_double(value);
                    assert_eq!(outcome.0, TCL_VALUE_GET_ERROR);
                    assert!(outcome.1.is_nan());
                    assert_eq!(host.abi_calls.get(), calls + 1);
                    assert_eq!(interp.native_execution_refusal(), Some(first));
                    tcl_runtime_set_current_interp(ptr::null_mut());
                    release_words(&[value]);
                    host.interpreter.borrow_mut().take();
                }
            }
        });
    }

    thread_local! {
        static SCALAR_UPDATER_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    }

    extern "C" fn scalar_context_changing_updater(value: *mut TclObj) {
        SCALAR_UPDATER_CALLS.with(|calls| calls.set(calls.get() + 1));
        // SAFETY: the actual caller retains this original interpreter for the
        // updater. The clone shares its real state without borrowing it.
        let mut interp = unsafe { current_interp().as_ref() }.unwrap().clone();
        let original = interp.runtime_context();
        let mut changed = original.clone();
        changed.packages = vec![("boxed-scalar-updater".to_owned(), "1.0".to_owned())];
        interp.pin_context(&changed).unwrap();
        interp.pin_context(&original).unwrap();
        // SAFETY: this actual descriptor owns the live original header.
        unsafe { obj::set_native_updater_string_rep(value, b"17", false) };
    }

    static SCALAR_CONTEXT_TYPE: obj::TclObjType = obj::TclObjType {
        name: c"originalScalarCurrency".as_ptr(),
        free_int_rep_proc: None,
        dup_int_rep_proc: None,
        update_string_proc: Some(scalar_context_changing_updater),
        set_from_any_proc: None,
    };

    #[test]
    fn boxed_primitive_updater_change_and_restore_keeps_only_reached_string_effects() {
        // Software stages: naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        leak_free(|| unsafe {
            let mut interp = scalar_interpreter("tcl8.6");
            let original = obj::Owned::fresh(obj::alloc_typed(&SCALAR_CONTEXT_TYPE, 0));
            let value = original.as_ptr();
            let context = interp.runtime_context();
            let references = (*value).ref_count;
            interp.set_result_bytes(b"PRIOR");
            let result = interp.get_obj_result();
            SCALAR_UPDATER_CALLS.with(|calls| calls.set(0));
            tcl_runtime_set_current_interp(&mut interp);
            assert_eq!(get_wide(value), (TCL_VALUE_GET_ERROR, i64::MIN));
            let first = interp.native_execution_refusal().unwrap();
            assert_eq!(
                first,
                tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "stale entered native operation",
                    ),
                )
            );
            SCALAR_UPDATER_CALLS.with(|calls| assert_eq!(calls.get(), 1));
            assert_eq!(interp.runtime_context(), context);
            assert_eq!(obj::bytes_of(value), b"17");
            assert_eq!(obj::obj_type_ptr(value), &SCALAR_CONTEXT_TYPE as *const _);
            assert!(obj::native_scalar_cache(value).unwrap().is_none());
            assert_eq!((*value).ref_count, references);
            assert_eq!(interp.get_obj_result(), result);
            assert_eq!(interp.result_bytes(), b"PRIOR");
            assert_eq!(get_bool(value), (TCL_VALUE_GET_ERROR, -1));
            SCALAR_UPDATER_CALLS.with(|calls| assert_eq!(calls.get(), 1));
            assert_eq!(interp.native_execution_refusal(), Some(first));
            tcl_runtime_set_current_interp(ptr::null_mut());
        });
    }

    #[test]
    fn entered_expression_number_updater_stops_before_cache_and_truth_output() {
        // Software stages: naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        leak_free(|| unsafe {
            for production in [0, 1] {
                let mut interp = scalar_interpreter("tcl8.6");
                let original = obj::Owned::fresh(obj::alloc_typed(&SCALAR_CONTEXT_TYPE, 0));
                let value = original.as_ptr();
                let context = interp.runtime_context();
                let references = (*value).ref_count;
                interp.set_result_bytes(b"PRIOR");
                let result = interp.get_obj_result();
                SCALAR_UPDATER_CALLS.with(|calls| calls.set(0));
                tcl_runtime_set_current_interp(&mut interp);
                let mut out = -17;
                assert_eq!(
                    tcl_value_get_expression_bool(value, production, &mut out),
                    TCL_VALUE_GET_ERROR
                );
                assert_eq!(out, -17);
                let first = interp.native_execution_refusal().unwrap();
                assert_eq!(first, tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "stale entered native operation",
                    ),
                ));
                SCALAR_UPDATER_CALLS.with(|calls| assert_eq!(calls.get(), 1));
                assert_eq!(interp.runtime_context(), context);
                assert_eq!(obj::bytes_of(value), b"17");
                assert_eq!(obj::obj_type_ptr(value), &SCALAR_CONTEXT_TYPE as *const _);
                assert!(obj::native_scalar_cache(value).unwrap().is_none());
                assert_eq!((*value).ref_count, references);
                assert_eq!(interp.get_obj_result(), result);
                assert_eq!(interp.result_bytes(), b"PRIOR");
                assert_eq!(
                    tcl_value_get_expression_bool(value, production, &mut out),
                    TCL_VALUE_GET_ERROR
                );
                assert_eq!(out, -17);
                SCALAR_UPDATER_CALLS.with(|calls| assert_eq!(calls.get(), 1));
                assert_eq!(interp.native_execution_refusal(), Some(first));
                tcl_runtime_set_current_interp(ptr::null_mut());
            }
        });
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
            assert_eq!(
                (*interp).error_code(),
                b"ARITH IOVERFLOW {integer value too large to represent}"
            );
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
                TCL_VALUE_GET_ERROR,
                "the actual C9 primitive double read rejects cached NaN"
            );

            release_words(&[ambiguous, nan]);
            tcl_runtime_set_current_interp(ptr::null_mut());
            tcl_runtime_delete_interp(interp);
        });
    }

    #[test]
    fn actual_cached_getter_preserves_private_c85_code_and_exact_result_extent() {
        leak_free(|| unsafe {
            let interp = tcl_runtime_create_interp();
            tcl_runtime_set_current_interp(interp);
            (*interp).set_dialect_profile(crate::environment::profile_for_dialect("tcl8.5"));
            (*interp).set_error_state(b"KEEP CODE");
            let value = owned_word(b"1.5\0tail");
            assert_eq!(get_double(value), (TCL_VALUE_GET_OK, 1.5));
            assert_eq!(get_wide(value), (TCL_VALUE_GET_ERROR, i64::MIN));
            assert_eq!(
                (*interp).result_bytes(),
                b"expected integer but got \"1.5\0tail\""
            );
            assert_eq!((*interp).error_code(), b"KEEP CODE");
            assert!(core::ptr::eq(
                obj::obj_type_ptr(value),
                &obj::TCL_DOUBLE_TYPE
            ));
            assert!(!(*interp).host_refusal_pending());
            release_words(&[value]);
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

    /// A compiled activation's matching leave retains the native error
    /// objects. Public original-name reads reach their hidden core read hooks
    /// without starting an eval loop or resetting the error episode.
    #[test]
    fn compiled_activation_leave_retains_native_errors_for_public_read() {
        // naming.runtime.compiled-procedure-fallback-and-source-log
        // docs/design/analysis/name-resolution-proofs/compiled-procedure-fallback-and-source-log.md
        leak_free(|| unsafe {
            let native = Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect("tcl9.0"),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            let interp = Box::into_raw(Box::new(native));
            tcl_runtime_set_current_interp(interp);

            assert_eq!(tcl_codegen_activation_enter(), 0);
            let words = [owned_word(b"error"), owned_word(b"boom")];
            let mut completion = invoke(&words);
            assert_eq!(completion.code, 1);

            // The ordinary cell has no eager write. This is not a claim that
            // a public Read cannot reach an active native private exception.
            // An intervening eval would reset the error episode under test.
            assert!(
                (*interp).var_get(b"::errorInfo").is_none(),
                "no eager ordinary error-variable cell write"
            );

            tcl_codegen_activation_leave(1);
            let info_name = obj::Owned::fresh(obj::new_string_bytes(b"::errorInfo"));
            let published = (*interp)
                .read_original_named_variable(info_name.as_ptr())
                .expect("the public Read reaches the published native trace");
            assert!(obj_bytes(published).starts_with(b"boom"));
            let code_name = obj::Owned::fresh(obj::new_string_bytes(b"::errorCode"));
            let published = (*interp)
                .read_original_named_variable(code_name.as_ptr())
                .expect("the public Read reaches the published native error code");
            assert_eq!(obj_bytes(published), b"NONE");

            tcl_completion_release(&mut completion);
            release_words(&words);
            tcl_runtime_set_current_interp(ptr::null_mut());
            tcl_runtime_delete_interp(interp);
        });
    }

    fn hold_actual_activation_bound() -> usize {
        // Reach the real entry bound without modifying interpreter depth fields.
        for entered in 0..1024 {
            if tcl_codegen_activation_enter() != 0 {
                assert!(entered > 0, "the genuine interpreter enters an activation");
                return entered;
            }
        }
        panic!("the bounded software fixture did not reach the actual activation refusal");
    }

    fn selected_c86_activation_interpreter() -> Interp {
        Interp::with_native_core(
            crate::interp::default_host(),
            tcl_registry::model::ingress::resolve_environment("tcl8.6").unit_profile(),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap()
    }

    #[test]
    fn activation_refusal_preserves_first_host_and_all_caller_owned_outputs() {
        // Software contract: naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // This reaches the actual software activation bound; it does not assert
        // a C Tcl nesting limit or an original provider callback chronology.
        leak_free(|| unsafe {
            let mut interp = selected_c86_activation_interpreter();
            tcl_runtime_set_current_interp(&raw mut interp);
            let entered = hold_actual_activation_bound();
            interp.set_result_bytes(b"PRIOR\0\xff");
            let prior = interp.result_obj();
            let first =
                tcl_syntax::raw_string::NativeValueAccessRefusal::ExpressionEngineUnavailable;
            interp.refuse_native_access(first.clone());
            let words = [
                owned_word(b"string"),
                owned_word(b"length"),
                owned_word(b"abc"),
            ];
            let counts = words.map(|word| (*word).ref_count);
            let result = owned_word(b"CALLER RESULT");
            let options = owned_word(b"-caller YES");
            let output_counts = ((*result).ref_count, (*options).ref_count);
            let mut out = TclCompletionAbi {
                code: 77,
                result,
                options,
            };
            for intrinsic in [false, true] {
                let status = if intrinsic {
                    tcl_intrinsic_invoke_argv(
                        IntrinsicId::StringLength.stable_id(),
                        words.as_ptr(),
                        3,
                        &raw mut out,
                    )
                } else {
                    tcl_invoke_argv(words.as_ptr(), 3, &raw mut out)
                };
                assert_eq!(status, TCL_INVOKE_ABI_HOST_REFUSED);
                assert_eq!(
                    interp.native_execution_refusal(),
                    Some(tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                        first.clone()
                    ),)
                );
                assert_eq!(out.code, 77);
                assert_eq!(out.result, result);
                assert_eq!(out.options, options);
                assert_eq!(((*result).ref_count, (*options).ref_count), output_counts);
                assert_eq!(words.map(|word| (*word).ref_count), counts);
                assert_eq!(interp.result_obj(), prior);
                assert_eq!(interp.result_bytes(), b"PRIOR\0\xff");
            }
            for _ in 0..entered {
                tcl_codegen_activation_leave(0);
            }
            tcl_completion_release(&raw mut out);
            release_words(&words);
            tcl_runtime_set_current_interp(ptr::null_mut());
        });
    }

    #[test]
    fn activation_guest_refusal_keeps_its_real_owned_completion() {
        // Software contract: naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Both public routes reach the genuine software entry bound; this only
        // checks its Guest/Host separation and ownership, not a native depth claim.
        leak_free(|| unsafe {
            let mut interp = selected_c86_activation_interpreter();
            tcl_runtime_set_current_interp(&raw mut interp);
            let entered = hold_actual_activation_bound();
            let words = [
                owned_word(b"string"),
                owned_word(b"length"),
                owned_word(b"abc"),
            ];
            let counts = words.map(|word| (*word).ref_count);
            for intrinsic in [false, true] {
                let mut out = TclCompletionAbi {
                    code: 77,
                    result: ptr::null_mut(),
                    options: ptr::null_mut(),
                };
                let status = if intrinsic {
                    tcl_intrinsic_invoke_argv(
                        IntrinsicId::StringLength.stable_id(),
                        words.as_ptr(),
                        3,
                        &raw mut out,
                    )
                } else {
                    tcl_invoke_argv(words.as_ptr(), 3, &raw mut out)
                };
                assert_eq!(status, TCL_INVOKE_ABI_OK);
                assert_eq!(out.code, 1);
                assert!(!out.result.is_null());
                assert!(!out.options.is_null());
                assert_eq!(
                    obj_bytes(out.result),
                    b"too many nested evaluations (infinite loop?)"
                );
                assert_eq!(option(&out, b"-code"), b"1");
                assert!(interp.native_execution_refusal().is_none());
                assert_eq!(words.map(|word| (*word).ref_count), counts);
                tcl_completion_release(&raw mut out);
            }
            for _ in 0..entered {
                tcl_codegen_activation_leave(0);
            }
            release_words(&words);
            tcl_runtime_set_current_interp(ptr::null_mut());
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

    #[test]
    fn var_get_named_preserves_a_literal_element_key() {
        leak_free(|| unsafe {
            let interp = tcl_runtime_create_interp();
            tcl_runtime_set_current_interp(interp);
            assert_eq!(
                tcl_eval_code(box_str(
                    b"set a(k) dynamic; set {a($key)} literal; set key k"
                )),
                0
            );
            let value = tcl_codegen_var_get(b"a($key)".as_ptr(), 7);
            assert!(!value.is_null());
            assert_eq!(obj_bytes(value), b"literal");
            tcl_obj_release(value);
            let value = tcl_codegen_var_get_element(b"a".as_ptr(), 1, b"k".as_ptr(), 1);
            assert!(!value.is_null());
            assert_eq!(obj_bytes(value), b"dynamic");
            tcl_obj_release(value);

            let value = owned_word(b"updated");
            assert_eq!(tcl_codegen_var_set(b"a($key)".as_ptr(), 7, value), 0);
            assert_eq!(tcl_eval_code(box_str(b"list ${a($key)} $a($key)")), 0);
            assert_eq!((*interp).result_bytes(), b"updated dynamic");

            assert!(tcl_codegen_var_get(b"a(missing)".as_ptr(), 10).is_null());
            assert_eq!(
                (*interp).result_bytes(),
                b"can't read \"a(missing)\": no such element in array"
            );
            tcl_runtime_set_current_interp(ptr::null_mut());
            tcl_runtime_delete_interp(interp);
        });
    }

    #[test]
    fn named_element_store_runs_its_observer_before_reporting_the_error() {
        for engine in tcl_test_support::available_tclshs() {
            let trace = if engine.version == TclVersion::V8_4 {
                "trace variable {arr($i)} rw observer"
            } else {
                "trace add variable {arr($i)} {read write} observer"
            };
            let setup = format!(
                "set {{arr($i)}} OLD; set i k; set arr(k) DYNAMIC; set events {{}}; \
                 proc observer {{name key op}} {{lappend ::events [list $name $key $op]; \
                 if {{$op eq \"w\" || $op eq \"write\"}} {{error REJECT}}}}; {trace}"
            );
            let expected = tcl_test_support::run_script(
                &engine.path,
                format!(
                    "{setup}; set before ${{arr($i)}}; \
                     set code [catch {{set {{arr($i)}} NEW}} result]; \
                     puts [list $before $code $result ${{arr($i)}} $arr($i) $events]\n"
                )
                .as_bytes(),
            )
            .expect("native named-element observer execution");
            assert!(
                expected.success() && expected.stderr.is_empty(),
                "{expected:?}"
            );
            leak_free(|| unsafe {
                let interp = tcl_runtime_create_interp();
                (*interp).set_runtime_version(engine.version);
                tcl_runtime_set_current_interp(interp);
                assert_eq!(tcl_eval_code(box_str(setup.as_bytes())), 0);
                let before = tcl_codegen_var_get(b"arr($i)".as_ptr(), 7);
                assert!(!before.is_null());
                let before_bytes = obj_bytes(before);
                tcl_obj_release(before);
                let code = tcl_codegen_var_set(b"arr($i)".as_ptr(), 7, owned_word(b"NEW"));
                let message = (*interp).result_bytes();
                let after = tcl_codegen_var_get(b"arr($i)".as_ptr(), 7);
                assert!(!after.is_null());
                let after_bytes = obj_bytes(after);
                tcl_obj_release(after);
                let sibling = tcl_codegen_var_get_element(b"arr".as_ptr(), 3, b"k".as_ptr(), 1);
                assert!(!sibling.is_null());
                let sibling_bytes = obj_bytes(sibling);
                tcl_obj_release(sibling);
                assert_eq!(tcl_eval_code(box_str(b"set events")), 0);
                let values = [
                    before_bytes,
                    code.to_string().into_bytes(),
                    message,
                    after_bytes,
                    sibling_bytes,
                    (*interp).result_bytes(),
                ];
                let mut observed = Vec::new();
                for (index, value) in values.iter().enumerate() {
                    if index > 0 {
                        observed.push(b' ');
                    }
                    crate::list::append_list_element(&mut observed, value, index == 0);
                }
                assert_eq!(
                    observed,
                    expected
                        .stdout
                        .strip_suffix(b"\n")
                        .unwrap_or(&expected.stdout),
                    "{:?}",
                    engine.path
                );
                assert!(!(*interp).host_refusal_pending());
                tcl_runtime_set_current_interp(ptr::null_mut());
                tcl_runtime_delete_interp(interp);
            });
        }
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

    // native proc entries

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
        // naming.runtime.compiled-procedure-fallback-and-source-log
        // docs/design/analysis/name-resolution-proofs/compiled-procedure-fallback-and-source-log.md
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
    /// leaves, both pinned here as a gate that fails in either direction.
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
                // A compiled statement calls no `log_command_info`, and that
                // one absence has two visible consequences here, both closed
                // by `tcl_codegen_log_command` once the emitter calls it on a
                // statement's error edge:
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

    /// Source logging reproduces the public source trace. A generic argv
    /// invocation retains its own inner error-stack context, independently from
    /// the original body's selected native instruction.
    #[test]
    fn a_logged_statement_site_preserves_the_source_trace_without_instruction_identity() {
        // naming.runtime.compiled-procedure-fallback-and-source-log
        // docs/design/analysis/name-resolution-proofs/compiled-procedure-fallback-and-source-log.md
        // This ABI stub exercises source logging, not native returnImm admission.
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
                assert_eq!(
                    source_stack, b"INNER {returnImm {bad Q} {}} CALL {boom Q}",
                    "the source body owns its selected native instruction context"
                );
                assert_eq!(
                    eval(interp, "dict get $o -errorstack").1,
                    b"INNER {error \"bad $a\"} CALL {boom Q}",
                    "logging source text does not grant an instruction context"
                );
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
                define_native(
                    b"bad",
                    b"",
                    b"return source",
                    Some(stub_return_without_state),
                );

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

    #[test]
    fn the_runtime_states_its_identity_to_a_host() {
        use tcl_runtime_api::ArtefactIdentityManifest;
        use tcl_runtime_api::codegen_abi::CODEGEN_ABI_VERSION;
        use tcl_runtime_api::manifest::EMBEDDED_STDLIB_REVISION;

        // SAFETY: every buffer is as long as the capacity it is passed with.
        unsafe {
            tcl_runtime_set_current_interp(ptr::null_mut());
            assert_eq!(
                tcl_runtime_identity(ptr::null_mut(), 0),
                0,
                "no interp is current, so there is nothing to state"
            );

            let interp = tcl_runtime_create_interp();
            tcl_runtime_set_current_interp(interp);
            (*interp).set_runtime_version(TclVersion::V8_6);
            let needed = tcl_runtime_identity(ptr::null_mut(), 0);
            assert!(needed > 0);

            let mut short = vec![0xAA_u8; usize::try_from(needed).unwrap() - 1];
            assert_eq!(tcl_runtime_identity(short.as_mut_ptr(), needed - 1), needed);
            assert!(
                short.iter().all(|byte| *byte == 0xAA),
                "a short buffer is not written"
            );

            let mut buffer = vec![0_u8; usize::try_from(needed).unwrap()];
            assert_eq!(tcl_runtime_identity(buffer.as_mut_ptr(), needed), needed);
            let manifest = ArtefactIdentityManifest::from_bytes(&buffer).expect("it decodes");
            assert_eq!(manifest, (*interp).held_identity());
            assert_eq!(manifest.environment, "tcl8.6");
            assert_eq!(manifest.release, "8.6");
            assert_eq!(manifest.abi_version, CODEGEN_ABI_VERSION);
            assert_eq!(
                manifest.intrinsic_table_hash,
                tcl_registry::intrinsic_table_hash()
            );
            assert_eq!(manifest.embedded_stdlib_revision, EMBEDDED_STDLIB_REVISION);
            assert!(manifest.packs.is_empty());

            tcl_runtime_set_current_interp(ptr::null_mut());
            tcl_runtime_delete_interp(interp);
        }
    }
}

#[cfg(test)]
#[path = "codegen_abi/original_ingress_tests.rs"]
mod original_ingress_tests;

#[cfg(test)]
#[path = "codegen_abi/original_receiver_tests.rs"]
mod original_receiver_tests;
