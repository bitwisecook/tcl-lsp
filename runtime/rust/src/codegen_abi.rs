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
//! (`rust/tcl-codegen/src/codegen/wasm/backend.rs`) emits `call`s to.
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

/// Probe generated procedure-entry eligibility on its existing entered owner.
/// Returns the unchanged Native procedure run, decline or Host-refused status.
/// It acquires no activation, object reference, getter or completion storage.
#[no_mangle]
pub extern "C" fn tcl_codegen_native_body_entry_status() -> i32 {
    original_frame_owner::native_body_entry_status()
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
        Self::enter_for(original_frame_owner::interpreter()?)
    }

    fn enter_installed() -> Result<Self, i32> {
        // SAFETY: explicit entry borrows the host's live installed interpreter.
        Self::enter_for(unsafe { current_interp().as_ref() }.cloned())
    }

    fn enter_for(interpreter: Option<Interp>) -> Result<Self, i32> {
        let Some(mut interpreter) = interpreter else {
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

impl BorrowedArgv<'_> {
    // Pure physical resident-cache classification. Retain already owns every
    // live header; no string provider, resolver or object operation is called.
    fn cached_depth_head(&self) -> &'static str {
        let Some(&head) = self.words.first() else {
            return "abi-before.noResidentBytes";
        };
        // SAFETY: this guard retains this exact live header through observation.
        let header = unsafe { &*head };
        if header.bytes.is_null() || header.length < 0 {
            return "abi-before.noResidentBytes";
        }
        let length = match header.length {
            1 => 1,
            4 => 4,
            6 => 6,
            _ => return "abi-before.other",
        };
        // SAFETY: the TclObj contract makes its non-null resident string buffer
        // readable for its cached length while this retained header is live.
        let bytes = unsafe { core::slice::from_raw_parts(header.bytes.cast::<u8>(), length) };
        match bytes {
            b"expr" => "abi-before.expr",
            b"return" => "abi-before.return",
            b"d" => "abi-before.d",
            _ => "abi-before.other",
        }
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
    original_frame_owner::explicit_bootstrap();
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
    let Ok(Some(interpreter)) = original_frame_owner::interpreter() else {
        return 0;
    };
    let bytes = interpreter.held_identity().to_bytes();
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
    #[track_caller]
    unsafe fn enter(interp: *mut Interp) -> Option<Self> {
        // SAFETY: forwarded unchanged; the default observation remains static.
        unsafe { Self::enter_labeled(interp, || "abi-before") }
    }

    #[track_caller]
    unsafe fn enter_labeled(
        interp: *mut Interp,
        label: impl FnOnce() -> &'static str,
    ) -> Option<Self> {
        // SAFETY: same retained interpreter. Label is evaluated only after the
        // existing flag/quota/filter and frame-borrow release in the observer.
        unsafe { (*interp).observe_codegen_depth_labeled(label, 0) };
        // SAFETY: forwarded per this function's contract.
        unsafe { (*interp).codegen_activation_enter() }.then(|| {
            // SAFETY: the same live interpreter just entered its activation.
            unsafe { (*interp).observe_codegen_depth("abi-entered", 0) };
            AbiActivation {
                interp,
                code: crate::interp::Code::Ok,
            }
        })
    }
}

impl Drop for AbiActivation {
    fn drop(&mut self) {
        // SAFETY: the interpreter was live when the activation was entered and
        // stays live for the ABI call that holds this guard.
        unsafe { (*self.interp).observe_codegen_depth("abi-before-leave", 0) };
        // SAFETY: this guard's original interpreter remains live through leave.
        unsafe { (*self.interp).codegen_activation_leave(self.code) };
        // SAFETY: leave does not retire this guard's retained interpreter.
        unsafe { (*self.interp).observe_codegen_depth("abi-left", 0) };
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
    original_frame_owner::activation_enter()
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
    original_frame_owner::activation_leave(code);
}

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
    // SAFETY: non-null inputs each transfer exactly one generated +1.
    let left = (!left.is_null()).then(|| unsafe { obj::Owned::from_raw(left) });
    let right = (!right.is_null()).then(|| unsafe { obj::Owned::from_raw(right) });
    let mut operation = match CodegenOperation::enter() {
        Ok(operation) => operation,
        Err(_) => {
            drop(left);
            drop(right);
            return ptr::null_mut();
        }
    };
    let result = match (left, right) {
        (Some(left), Some(right)) => crate::builtins::eval_binary_operands(
            &mut operation.interpreter,
            tcl_syntax::expr::BinOp::Add,
            left,
            right,
        ),
        (left, right) => {
            drop(left);
            drop(right);
            let _ = operation.ensure_current();
            return ptr::null_mut();
        }
    };
    let result = match result {
        Ok(result) => Some(result),
        Err(error) => {
            operation.interpreter.report_expr_error(error);
            None
        }
    };
    if operation.ensure_current().is_err() {
        drop(result);
        return ptr::null_mut();
    }
    result.map_or(ptr::null_mut(), obj::Owned::into_raw)
}

/// Report unavailable arithmetic while consuming every transferred reference.
///
/// # Safety
/// Non-null inputs transfer one live generated +1 each.
#[no_mangle]
#[cfg(not(have_tommath))]
pub unsafe extern "C" fn tcl_codegen_expr_add(
    left: *mut TclObj,
    right: *mut TclObj,
) -> *mut TclObj {
    // SAFETY: adopt both inputs before any operation/refusal/null branch.
    let left = (!left.is_null()).then(|| unsafe { obj::Owned::from_raw(left) });
    let right = (!right.is_null()).then(|| unsafe { obj::Owned::from_raw(right) });
    let operation = CodegenOperation::enter();
    drop(left);
    drop(right);
    if let Ok(mut operation) = operation {
        if operation.ensure_current().is_ok() {
            operation
                .interpreter
                .set_error(b"arithmetic support is not available");
            let _ = operation.ensure_current();
        }
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
    let Ok(Some(interpreter)) = original_frame_owner::interpreter() else {
        return -1;
    };
    let dispatches = interpreter.native_proc_dispatches();
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
    i32::from(original_frame_owner::refusal_pending())
}

mod original_frame_owner;

#[cfg(test)]
mod tests;

#[cfg(test)]
#[path = "codegen_abi/original_ingress_tests.rs"]
mod original_ingress_tests;

#[cfg(test)]
#[path = "codegen_abi/original_receiver_tests.rs"]
mod original_receiver_tests;
