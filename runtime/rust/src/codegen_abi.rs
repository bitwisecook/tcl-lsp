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
#[cfg(test)]
use tcl_registry::IntrinsicId;
#[cfg(test)]
use tcl_runtime_api::guard::{GuardDomains, GuardIdentity};

use crate::interp::native_operation_currency::NativeOperationScope;
#[cfg(test)]
use crate::interp::obj_bytes;
use crate::interp::Interp;
#[cfg(test)]
use crate::interp::NativeProcEntry;
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

mod argv_invocation;
mod procedure_definition;
mod values;
mod variables;

#[cfg(test)]
use argv_invocation::resolve_intrinsic_argv;
pub use argv_invocation::{
    tcl_codegen_guard_check, tcl_codegen_guard_prepare, tcl_codegen_guard_release,
    tcl_intrinsic_invoke_argv, tcl_invoke_argv,
};
pub use procedure_definition::{
    tcl_codegen_native_proc_dispatches, tcl_codegen_proc_define_guarded_argv,
    tcl_codegen_proc_define_guarded_i64_add_argv, tcl_codegen_proc_define_native,
    tcl_codegen_proc_define_original_argv, tcl_codegen_proc_register,
};
pub use values::{
    tcl_codegen_word_concat, tcl_obj_new_string, tcl_obj_new_string_owned, tcl_obj_release,
    tcl_obj_retain, tcl_value_get_bool, tcl_value_get_bool_for_purpose, tcl_value_get_double,
    tcl_value_get_expression_bool, tcl_value_get_wide_int, tcl_value_new_bool,
    tcl_value_new_double, tcl_value_new_string, tcl_value_new_wide_int, tcl_value_release,
};
pub use variables::{
    tcl_codegen_frame_pop, tcl_codegen_frame_push, tcl_codegen_local_bind, tcl_codegen_local_get,
    tcl_codegen_local_set, tcl_codegen_slot_append, tcl_codegen_slot_bind, tcl_codegen_slot_get,
    tcl_codegen_slot_incr_i64, tcl_codegen_slot_lappend, tcl_codegen_slot_set,
    tcl_codegen_slot_traced, tcl_codegen_var_get, tcl_codegen_var_get_element, tcl_codegen_var_set,
    tcl_codegen_var_traced,
};

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
