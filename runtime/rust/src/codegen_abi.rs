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
use core::sync::atomic::{AtomicUsize, Ordering};
use std::alloc::{alloc, dealloc, handle_alloc_error, Layout};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crate::interp::{drop_fresh, obj_bytes, Interp};
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

/// Number of live frames allocated through the compiler transport boundary.
///
/// This intentionally tracks only raw transient frames, not Tcl objects; test
/// code uses it to prove that every generated cleanup path balances allocation.
static CODEGEN_CALL_FRAMES_OUTSTANDING: AtomicUsize = AtomicUsize::new(0);

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
    CODEGEN_CALL_FRAMES_OUTSTANDING.fetch_add(1, Ordering::SeqCst);
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
    CODEGEN_CALL_FRAMES_OUTSTANDING.fetch_sub(1, Ordering::SeqCst);
    0
}

/// Return the number of outstanding compiler transport call frames.
///
/// This is a diagnostic/test boundary rather than an allocation mechanism.
#[no_mangle]
pub extern "C" fn tcl_codegen_call_frame_outstanding() -> i32 {
    i32::try_from(CODEGEN_CALL_FRAMES_OUTSTANDING.load(Ordering::SeqCst)).unwrap_or(i32::MAX)
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

/// Release one generated operand-stack value.
///
/// # Safety
/// `value` must carry one live reference owned by generated code.
#[no_mangle]
pub unsafe extern "C" fn tcl_value_release(value: *mut TclObj) {
    // SAFETY: generated code transfers its owned stack reference here.
    unsafe { obj::decr_ref_count(value) };
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
    let Some(name) = interp.codegen_slot_name(usize::try_from(slot).unwrap_or(0)) else {
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
    interp.define_proc(name, params, body_obj);
    drop_fresh(body_obj);
    interp.set_result_bytes(b"");
    0
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
    // SAFETY: the current interpreter is live for this ABI call.
    let completion = unsafe { crate::state_traits::dispatch_prebuilt_argv(&mut *interp, words) };
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
}
