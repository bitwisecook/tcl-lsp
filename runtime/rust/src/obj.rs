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

//! The `Tcl_Obj` value model + refcount discipline (Track 1, T1.1).
//!
//! Every allocation is balanced by a free driven by the refcount reaching
//! zero, and the alloc/free counters (`crate::counters`) prove it.
//!
//! ## Layout — the C-extension ABI requires `#[repr(C)]`
//!
//! `docs/design/runtime/c-extension-abi.md` §4.2 fixes the layout: extensions
//! read `objPtr->refCount` / `objPtr->bytes` directly through `tcl.h` macros,
//! so the struct must be `#[repr(C)]` with the exact field order
//! `tcl.h` declares — `{ refCount, bytes, length, typePtr, internalRep }`. On
//! `wasm32` that is `{ isize, ptr, isize, ptr, <8-byte union> }`. This is the
//! single canonical obj model for the runtime (the codegen's 32-byte
//! handle/tagged-immediate layout is a separate codegen detail; the runtime
//! serves the same `tcl_*`/`obj_*` codegen primitives over the
//! ABI-faithful struct, with the immediate/inline-string optimisations layered
//! on later — see T1.5/S6).
//!
//! ## Refcount semantics — faithful to `tclObj.c`
//!
//! Constructors return an object at **refCount 0** (the `fresh_zero` C-API
//! convention, `c-api-ownership-contract.md`): the caller owns nothing until
//! it `Tcl_IncrRefCount`s or hands the object to a consumer that retains it.
//! `Tcl_DecrRefCount` frees immediately when the count reaches zero (Tcl's
//! `TclFreeObj`). The runtime-internal *deferred* free queue
//! (`tcl_obj_drain_pending`, for the eval-loop aliasing case in
//! `memory-management.md` MM-B.6) lands with the eval loop (T1.3); the C-API
//! boundary is immediate, which is what `Tcl_DecrRefCount` documents.

use core::ffi::{c_char, c_void};
use std::alloc::{alloc, dealloc, realloc, Layout};

use crate::counters;

/// `Tcl_Size` — `ptrdiff_t` in `tcl.h` (Tcl 9 width-agnostic size type).
pub type TclSize = isize;
/// `Tcl_WideInt` — always 64-bit.
pub type TclWideInt = i64;

/// `Tcl_ObjType` — the registered type descriptor (`tcl.h`). The four procs are
/// the **shimmer keystone** (value-kinds): the runtime
/// dispatches free / dup / string-generation through `typePtr`, so built-in
/// types (int, double, list, …) and extension-registered custom types share one
/// mechanism — type handling is open, never a closed enum (the §6/Track-2
/// custom-`Tcl_ObjType` requirement). Signatures match `tcl.h` so an extension's
/// `Tcl_ObjType` slots in unchanged.
pub type FreeInternalRepProc = extern "C" fn(*mut TclObj);
pub type DupInternalRepProc = extern "C" fn(*mut TclObj, *mut TclObj);
pub type UpdateStringProc = extern "C" fn(*mut TclObj);
pub type SetFromAnyProc = extern "C" fn(*mut c_void, *mut TclObj) -> core::ffi::c_int;

#[repr(C)]
pub struct TclObjType {
    pub name: *const c_char,
    pub free_int_rep_proc: Option<FreeInternalRepProc>,
    pub dup_int_rep_proc: Option<DupInternalRepProc>,
    pub update_string_proc: Option<UpdateStringProc>,
    pub set_from_any_proc: Option<SetFromAnyProc>,
}

// SAFETY: these are immortal `'static` descriptors; the raw `name` pointer is
// to a `'static` NUL-terminated byte string. They are read-only and shared.
unsafe impl Sync for TclObjType {}

/// The `int` (wide) type descriptor. `typePtr == &TCL_INT_TYPE` ⇒ the value is
/// in `internal_rep` as a `TclWideInt`; `bytes` may be null until shimmered.
pub static TCL_INT_TYPE: TclObjType = TclObjType {
    name: c"int".as_ptr(),
    free_int_rep_proc: None, // an int rep owns nothing
    dup_int_rep_proc: None,  // the i64 is copied with the header
    update_string_proc: Some(int_update_string),
    set_from_any_proc: None,
};
/// The `double` type descriptor.
pub static TCL_DOUBLE_TYPE: TclObjType = TclObjType {
    name: c"double".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(double_update_string),
    set_from_any_proc: None,
};

extern "C" fn int_update_string(obj: *mut TclObj) {
    // SAFETY: `obj` is a live int object whose string rep needs generating.
    unsafe {
        let s = itoa((*obj).wide());
        set_owned_string(obj, s.as_ptr(), s.len());
    }
}

extern "C" fn double_update_string(obj: *mut TclObj) {
    // SAFETY: `obj` is a live double object. The canonical Tcl double→string is
    // the shared `tcl_syntax::number::format_double` (also used by the compiler's
    // const-folder) — integer-valued doubles get `.0`, plus `Inf`/`NaN`.
    unsafe {
        let s = tcl_syntax::number::format_double((*obj).double());
        set_owned_string(obj, s.as_ptr(), s.len());
    }
}

/// `Tcl_Obj` — ABI-faithful to `tcl.h` (§4.2). `internal_rep` models the
/// 8-byte `Tcl_ObjInternalRep` union; core-API extensions never touch its
/// variants, so we keep the raw 8 bytes and reinterpret for `wide`/`double`.
#[repr(C)]
pub struct TclObj {
    pub ref_count: TclSize,
    pub bytes: *mut c_char,
    pub length: TclSize,
    pub type_ptr: *const TclObjType,
    pub internal_rep: u64,
}

impl TclObj {
    #[inline]
    fn wide(&self) -> TclWideInt {
        self.internal_rep as i64
    }

    #[inline]
    fn double(&self) -> f64 {
        f64::from_bits(self.internal_rep)
    }
}

// ---------------------------------------------------------------------------
// Allocation — the single allocator (§4.4). Natively this is the Rust global
// allocator; on wasm it becomes the one shared-memory allocator (T1.6). Every
// header and every owned string buffer is counted so the leak gate can prove
// balance.
// ---------------------------------------------------------------------------

fn obj_layout() -> Layout {
    Layout::new::<TclObj>()
}

/// Allocate a zeroed `TclObj` header at refCount 0 with no string rep.
fn obj_alloc() -> *mut TclObj {
    // SAFETY: `obj_layout()` is non-zero-sized and well-formed; we initialise
    // every field before returning, and the pointer is freed exactly once by
    // `obj_free`.
    unsafe {
        let p = alloc(obj_layout()) as *mut TclObj;
        if p.is_null() {
            counters::oom_set();
            return core::ptr::null_mut();
        }
        (*p).ref_count = 0;
        (*p).bytes = core::ptr::null_mut();
        (*p).length = 0;
        (*p).type_ptr = core::ptr::null();
        (*p).internal_rep = 0;
        counters::obj_alloced();
        p
    }
}

/// Free a `TclObj` and its owned string buffer (if any). `TclFreeObj`.
///
/// # Safety
/// `obj` must be a live header previously returned by `obj_alloc` and not yet
/// freed; no other reference may use it after this returns.
unsafe fn obj_free(obj: *mut TclObj) {
    if obj.is_null() {
        return;
    }
    // SAFETY: caller guarantees `obj` is a live, uniquely-owned header.
    unsafe {
        // Dispatch the type's free-internal-rep proc (releases list elements,
        // frees the list/dict backing, runs an extension's freeIntRepProc, …).
        let tp = (*obj).type_ptr;
        if !tp.is_null() {
            if let Some(free) = (*tp).free_int_rep_proc {
                free(obj);
            }
        }
        free_string_buffer(obj);
        dealloc(obj as *mut u8, obj_layout());
    }
    counters::obj_freed();
}

// ---------------------------------------------------------------------------
// Typed internal-rep helpers — the shimmer keystone's plumbing, used by the
// value-type modules (`list`, future `dict`, …). pub(crate): internal only.
// ---------------------------------------------------------------------------

/// Allocate a fresh (`rc 0`) object with a typed internal rep and no string rep.
pub(crate) fn alloc_typed(type_ptr: *const TclObjType, internal_rep: u64) -> *mut TclObj {
    let obj = obj_alloc();
    if obj.is_null() {
        return obj;
    }
    // SAFETY: `obj` is a freshly owned header.
    unsafe {
        (*obj).type_ptr = type_ptr;
        (*obj).internal_rep = internal_rep;
    }
    obj
}

/// Read the raw 8-byte internal rep (a value type stores its backing pointer here).
pub(crate) fn internal_rep(obj: *mut TclObj) -> u64 {
    // SAFETY: `obj` is a live object.
    unsafe { (*obj).internal_rep }
}

/// `obj`'s current type descriptor (null for a plain string).
pub(crate) fn obj_type_ptr(obj: *mut TclObj) -> *const TclObjType {
    // SAFETY: `obj` is a live object.
    unsafe { (*obj).type_ptr }
}

/// Shimmer `obj` to a new type: free the **old** internal rep (its proc), then
/// install `new_type` + `new_rep`. The string rep is **kept** across a
/// string→typed shimmer (Tcl's dual-rep: the original spelling survives until
/// the typed value is mutated, which invalidates it via [`invalidate_string`]).
pub(crate) fn change_type(obj: *mut TclObj, new_type: *const TclObjType, new_rep: u64) {
    // SAFETY: `obj` is live; free the prior rep before overwriting `internal_rep`.
    unsafe {
        let old = (*obj).type_ptr;
        if old.is_null() {
            // Leaving a *plain string*: its capacity lives in `internal_rep`,
            // which `new_type` is about to claim for its backing. Keep the bytes
            // as the cached (immutable) string rep, but first shrink the buffer
            // to exactly `length + 1` — once `type_ptr` is non-null,
            // `free_string_buffer` computes the dealloc size as `length + 1`, so
            // the buffer must match (any spare capacity from `append` would
            // otherwise be a layout mismatch). The next read returns these bytes
            // verbatim; an in-place mutation of the new rep drops them.
            shrink_string_to_exact(obj);
        } else if let Some(free) = (*old).free_int_rep_proc {
            free(obj);
        }
        (*obj).type_ptr = new_type;
        (*obj).internal_rep = new_rep;
    }
}

/// Shrink a plain string's buffer to exactly `length + 1` so its cached rep can
/// outlive a shimmer (after which `internal_rep` no longer tracks capacity and
/// `free_string_buffer` assumes the exact `length + 1` layout). No-op when there
/// is no buffer or it is already exact (set-only strings). On a rare shrink
/// failure, drop the rep (it regenerates lazily) rather than carry an unknown
/// capacity across the shimmer.
///
/// # Safety
/// `obj` must be live and a plain string (`type_ptr` null, capacity in
/// `internal_rep`).
unsafe fn shrink_string_to_exact(obj: *mut TclObj) {
    // SAFETY: caller guarantees a live plain-string `obj`; `bytes` (when
    // non-null) was allocated by `set_owned_string` with `Layout(internal_rep,1)`.
    unsafe {
        let bytes = (*obj).bytes;
        if bytes.is_null() {
            return;
        }
        let cur_cap = (*obj).internal_rep as usize; // allocated bytes incl. NUL
        let exact = (*obj).length as usize + 1;
        if cur_cap <= exact {
            return; // already exact — nothing to reclaim
        }
        let old_layout = Layout::from_size_align(cur_cap, 1).expect("buffer layout");
        let nb = realloc(bytes as *mut u8, old_layout, exact);
        if nb.is_null() {
            // Shrink failed; the original block is intact — free it (the rep
            // regenerates on next read) to avoid a later layout mismatch.
            free_string_buffer(obj);
            return;
        }
        (*obj).bytes = nb as *mut c_char;
        // `internal_rep` is left as-is here; `change_type` overwrites it with the
        // new typed rep immediately after this returns.
    }
}

/// Invalidate the string rep (drop the buffer) so it regenerates via the type's
/// `update_string_proc` on the next read — call after mutating a typed rep.
pub(crate) fn invalidate_string(obj: *mut TclObj) {
    // SAFETY: `obj` is live; dropping its owned buffer is sound (it will be
    // regenerated lazily).
    unsafe { free_string_buffer(obj) }
}

/// Set `obj`'s string rep to a copy of `bytes` (for `update_string_proc` impls).
///
/// # Safety
/// `obj` must be live.
pub(crate) unsafe fn set_string_rep(obj: *mut TclObj, bytes: &[u8]) {
    // SAFETY: forwarded — `obj` live, slice readable.
    unsafe { set_owned_string(obj, bytes.as_ptr(), bytes.len()) }
}

/// Read a `TCL_INT_TYPE` object's wide value from its internal rep.
pub(crate) fn wide_of(obj: *mut TclObj) -> TclWideInt {
    // SAFETY: caller has checked `obj`'s type is `TCL_INT_TYPE`.
    unsafe { (*obj).wide() }
}

/// Read a `TCL_DOUBLE_TYPE` object's value from its internal rep.
pub(crate) fn double_of(obj: *mut TclObj) -> f64 {
    // SAFETY: caller has checked `obj`'s type is `TCL_DOUBLE_TYPE`.
    unsafe { (*obj).double() }
}

/// Whether `obj` already carries a materialised string rep.
///
/// A type whose `update_string_proc` is `None` can only be attached to an
/// object that has one, since there would otherwise be no way back to a
/// spelling.
pub(crate) fn has_string_rep(obj: *mut TclObj) -> bool {
    // SAFETY: `obj` is a live object.
    !unsafe { (*obj).bytes }.is_null()
}

/// Whether a just-parsed numeric internal rep may be cached back onto `obj`.
///
/// C Tcl caches unconditionally: `TclParseNumber` (`tclStrToD.c`) writes the rep
/// it built straight onto the object it parsed, whatever the refcount, because
/// the **string** rep is kept — every other holder still reads the same
/// spelling, it just no longer pays to re-parse it. That reasoning holds here
/// for the two shapes where the string rep *is* the value: a plain string (no
/// typed rep to destroy), and any unshared object. A *shared* object already
/// carrying some other typed rep — a list, a dict — is left alone instead:
/// [`change_type`] frees that rep, and the other holders would pay to rebuild
/// what they still want.
pub(crate) fn may_cache_parsed_rep(obj: *mut TclObj) -> bool {
    obj_type_ptr(obj).is_null() || !is_shared(obj)
}

/// Cache a parsed wide-integer rep onto `obj`, keeping its string rep, so the
/// next numeric use reads the rep instead of re-parsing the spelling. A no-op
/// where [`may_cache_parsed_rep`] declines.
pub(crate) fn cache_wide_rep(obj: *mut TclObj, value: TclWideInt) {
    if may_cache_parsed_rep(obj) {
        change_type(obj, &TCL_INT_TYPE, value as u64);
    }
}

/// [`cache_wide_rep`] for a parsed double.
pub(crate) fn cache_double_rep(obj: *mut TclObj, value: f64) {
    if may_cache_parsed_rep(obj) {
        change_type(obj, &TCL_DOUBLE_TYPE, value.to_bits());
    }
}

/// Copy `obj`'s string rep (shimmering via `update_string_proc` if needed).
pub(crate) fn bytes_of(obj: *mut TclObj) -> Vec<u8> {
    // SAFETY: `obj` is a live object; `get_string` returns a borrowed pointer
    // into its (possibly just-generated) string rep, copied immediately.
    unsafe {
        let mut len: TclSize = 0;
        let p = get_string(obj, &mut len);
        if p.is_null() {
            return Vec::new();
        }
        core::slice::from_raw_parts(p as *const u8, len as usize).to_vec()
    }
}

/// Allocate an owned, NUL-terminated buffer holding `src[..len]` and attach it
/// to `obj` as its string rep. Replaces any prior owned buffer.
///
/// # Safety
/// `obj` must be live; `src` must point to at least `len` readable bytes (or be
/// null when `len == 0`).
unsafe fn set_owned_string(obj: *mut TclObj, src: *const u8, len: usize) {
    // SAFETY: see fn-doc; `obj` is live and we own its `bytes` slot.
    unsafe {
        free_string_buffer(obj);
        let cap = len + 1; // + NUL terminator (Tcl keeps string reps NUL-term)
        let layout = Layout::from_size_align(cap, 1).expect("buffer layout");
        let buf = alloc(layout);
        if buf.is_null() {
            counters::oom_set();
            (*obj).bytes = core::ptr::null_mut();
            (*obj).length = 0;
            return;
        }
        if len > 0 {
            core::ptr::copy_nonoverlapping(src, buf, len);
        }
        *buf.add(len) = 0;
        (*obj).bytes = buf as *mut c_char;
        (*obj).length = len as TclSize;
        // For a plain string (no typed rep), track the buffer's allocated
        // capacity in `internal_rep` (unused otherwise) so `string`/`append` can
        // grow it amortised and `free_string_buffer` frees the right size.
        // A typed obj's `internal_rep` is its backing pointer — never touch it.
        if (*obj).type_ptr.is_null() {
            (*obj).internal_rep = cap as u64;
        }
        counters::buf_alloced();
    }
}

/// Free `obj`'s owned string buffer if it owns one. A null `bytes` (a "pure"
/// int/double obj that has never been shimmered) owns nothing.
///
/// # Safety
/// `obj` must be live.
unsafe fn free_string_buffer(obj: *mut TclObj) {
    // SAFETY: `obj` is live per caller; `bytes`, when non-null, was allocated
    // by `set_owned_string` with `Layout(length + 1, 1)` and `length` is
    // immutable for T1.1 obj kinds (strings are not appended-to here yet).
    unsafe {
        let bytes = (*obj).bytes;
        if bytes.is_null() {
            return;
        }
        // Capacity: a plain string's allocated size lives in `internal_rep`; a
        // typed obj's cached string rep is exact (`length + 1`). Freeing with
        // the exact allocation size is required (Rust dealloc layout must match).
        let cap = if (*obj).type_ptr.is_null() {
            (*obj).internal_rep as usize
        } else {
            (*obj).length as usize + 1
        };
        let layout = Layout::from_size_align(cap, 1).expect("buffer layout");
        dealloc(bytes as *mut u8, layout);
        (*obj).bytes = core::ptr::null_mut();
        (*obj).length = 0;
        counters::buf_freed();
    }
}

// ---------------------------------------------------------------------------
// Constructors — all `fresh_zero` (refCount 0).
// ---------------------------------------------------------------------------

/// `Tcl_NewObj` — a fresh empty-string object at refCount 0.
pub fn new_obj() -> *mut TclObj {
    let obj = obj_alloc();
    if obj.is_null() {
        return obj;
    }
    // SAFETY: `obj` is a freshly allocated live header we uniquely own.
    unsafe {
        set_owned_string(obj, core::ptr::null(), 0);
    }
    obj
}

/// `Tcl_NewStringObj(bytes, length)` — copies the bytes. `length < 0` means
/// "NUL-terminated, use `strlen`". Result is `fresh_zero`.
///
/// # Safety
/// `bytes` must point to at least `length` readable bytes (or be a valid
/// NUL-terminated C string when `length < 0`).
pub unsafe fn new_string_obj(bytes: *const c_char, length: TclSize) -> *mut TclObj {
    let obj = obj_alloc();
    if obj.is_null() {
        return obj;
    }
    // SAFETY: caller guarantees `bytes`/`length`; `obj` is freshly owned.
    unsafe {
        let len = if length < 0 {
            if bytes.is_null() {
                0
            } else {
                libc_strlen(bytes)
            }
        } else {
            length as usize
        };
        set_owned_string(obj, bytes as *const u8, len);
    }
    obj
}

/// A fresh (`rc 0`) string object holding `bytes` (the internal byte-slice
/// constructor the value-type modules use).
pub(crate) fn new_string_bytes(bytes: &[u8]) -> *mut TclObj {
    // SAFETY: `bytes` is a valid readable slice.
    unsafe { new_string_obj(bytes.as_ptr() as *const c_char, bytes.len() as TclSize) }
}

/// `Tcl_IsShared` — does more than one reference hold `obj`? Mutation in place is
/// only sound on an unshared object; otherwise copy-on-write.
pub(crate) fn is_shared(obj: *mut TclObj) -> bool {
    // SAFETY: `obj` is a live object.
    unsafe { (*obj).ref_count > 1 }
}

/// `Tcl_DuplicateObj` — a fresh (`rc 0`) deep copy: the string rep (if any) plus
/// the internal rep (via the type's `dup_int_rep_proc`, or a raw copy for
/// self-contained reps like int/double).
pub(crate) fn duplicate(src: *mut TclObj) -> *mut TclObj {
    let dup = obj_alloc();
    if dup.is_null() {
        return dup;
    }
    // SAFETY: `src` is live; `dup` is freshly owned and uniquely ours.
    unsafe {
        if !(*src).bytes.is_null() {
            set_owned_string(dup, (*src).bytes as *const u8, (*src).length as usize);
        }
        let tp = (*src).type_ptr;
        if !tp.is_null() {
            match (*tp).dup_int_rep_proc {
                Some(dup_proc) => dup_proc(src, dup), // e.g. list_dup deep-copies
                None => {
                    // self-contained rep (int/double): copy type + raw 8 bytes
                    (*dup).type_ptr = tp;
                    (*dup).internal_rep = (*src).internal_rep;
                }
            }
        }
    }
    dup
}

/// Whether `obj` is a plain string (no typed internal rep) — the precondition
/// for [`string_append_inplace`].
pub(crate) fn is_plain_string(obj: *mut TclObj) -> bool {
    // SAFETY: `obj` is a live object.
    unsafe { (*obj).type_ptr.is_null() }
}

/// Append `piece` to a **plain string** object in place, growing its buffer
/// geometrically (amortised O(1), EXP-STRING). The caller must ensure `obj` is a
/// plain string ([`is_plain_string`]) and unshared. Refreshes `bytes`/`length`
/// and the capacity in `internal_rep`. A `realloc` keeps the single live-buffer
/// count, so the leak counters stay balanced.
pub(crate) fn string_append_inplace(obj: *mut TclObj, piece: &[u8]) {
    if piece.is_empty() {
        return;
    }
    // SAFETY: caller guarantees a live, unshared, plain-string `obj` whose
    // buffer was allocated by `set_owned_string` (capacity in `internal_rep`).
    unsafe {
        let cur_len = (*obj).length as usize;
        let cur_cap = (*obj).internal_rep as usize; // allocated bytes incl. NUL
        let new_len = cur_len + piece.len();
        let need = new_len + 1; // + NUL terminator
        let mut buf = (*obj).bytes as *mut u8;
        if need > cur_cap {
            let new_cap = need.max(cur_cap.saturating_mul(2));
            let old_layout = Layout::from_size_align(cur_cap, 1).expect("buffer layout");
            let nb = realloc(buf, old_layout, new_cap);
            if nb.is_null() {
                counters::oom_set();
                return;
            }
            buf = nb;
            (*obj).bytes = buf as *mut c_char;
            (*obj).internal_rep = new_cap as u64;
        }
        core::ptr::copy_nonoverlapping(piece.as_ptr(), buf.add(cur_len), piece.len());
        *buf.add(new_len) = 0;
        (*obj).length = new_len as TclSize;
    }
}

/// `Tcl_NewWideIntObj` — pure int obj (no string rep yet). `fresh_zero`.
pub fn new_wide_int_obj(value: TclWideInt) -> *mut TclObj {
    let obj = obj_alloc();
    if obj.is_null() {
        return obj;
    }
    // SAFETY: freshly owned header.
    unsafe {
        (*obj).type_ptr = &TCL_INT_TYPE;
        (*obj).internal_rep = value as u64;
        (*obj).bytes = core::ptr::null_mut(); // shimmer on demand
        (*obj).length = 0;
    }
    obj
}

/// `Tcl_NewDoubleObj` — pure double obj (no string rep yet). `fresh_zero`.
pub fn new_double_obj(value: f64) -> *mut TclObj {
    let obj = obj_alloc();
    if obj.is_null() {
        return obj;
    }
    // SAFETY: freshly owned header.
    unsafe {
        (*obj).type_ptr = &TCL_DOUBLE_TYPE;
        (*obj).internal_rep = value.to_bits();
        (*obj).bytes = core::ptr::null_mut();
        (*obj).length = 0;
    }
    obj
}

/// `Tcl_NewBooleanObj` — booleans are int objs (0/1), as in Tcl. `fresh_zero`.
pub fn new_boolean_obj(value: i32) -> *mut TclObj {
    new_wide_int_obj(if value != 0 { 1 } else { 0 })
}

// ---------------------------------------------------------------------------
// Refcount — `Tcl_IncrRefCount` / `Tcl_DecrRefCount` (faithful to the macros).
// ---------------------------------------------------------------------------

/// `Tcl_IncrRefCount`. Null-safe.
///
/// # Safety
/// `obj` must be null or a live header.
pub unsafe fn incr_ref_count(obj: *mut TclObj) {
    if obj.is_null() {
        return;
    }
    // SAFETY: caller guarantees `obj` is live.
    unsafe {
        (*obj).ref_count += 1;
    }
}

/// `Tcl_DecrRefCount`. Frees the object immediately when the count reaches
/// zero (`TclFreeObj`). Null-safe. Increments the double-free counter if called
/// on an object already at refCount 0 (a contract violation — see the MM-B
/// double-free guard).
///
/// # Safety
/// `obj` must be null or a live header to which the caller holds a reference.
pub unsafe fn decr_ref_count(obj: *mut TclObj) {
    if obj.is_null() {
        return;
    }
    // SAFETY: caller guarantees `obj` is live and holds a reference.
    unsafe {
        if (*obj).ref_count <= 0 {
            // Releasing an object with no outstanding reference: a double-free
            // / contract violation. Count it and refuse to free again.
            counters::double_free();
            return;
        }
        (*obj).ref_count -= 1;
        if (*obj).ref_count <= 0 {
            obj_free(obj);
        }
    }
}

// ---------------------------------------------------------------------------
// String rep — `Tcl_GetStringFromObj` / `Tcl_GetString` (shimmer on demand).
// ---------------------------------------------------------------------------

/// `Tcl_GetStringFromObj` — returns a borrowed pointer into the object's string
/// rep, generating it on demand for pure int objects (shimmer). The pointer is
/// valid until the object is modified or freed.
///
/// # Safety
/// `obj` must be a live header. `length_out`, if non-null, must be writable.
pub unsafe fn get_string(obj: *mut TclObj, length_out: *mut TclSize) -> *mut c_char {
    // SAFETY: caller guarantees `obj` is live; we own its `bytes` slot for the
    // shimmer write.
    unsafe {
        if (*obj).bytes.is_null() {
            // Generate the string rep via the type's update_string_proc (int,
            // double, list, an extension's custom type, …). A typed obj with no
            // proc, or an untyped obj, gets the empty string rep.
            let tp = (*obj).type_ptr;
            if !tp.is_null() {
                if let Some(update) = (*tp).update_string_proc {
                    update(obj);
                }
            }
            if (*obj).bytes.is_null() {
                set_owned_string(obj, core::ptr::null(), 0);
            }
        }
        if !length_out.is_null() {
            *length_out = (*obj).length;
        }
        (*obj).bytes
    }
}

// ---------------------------------------------------------------------------
// Small helpers (no libc dependency in the native build).
// ---------------------------------------------------------------------------

/// `strlen` over a NUL-terminated C string.
///
/// # Safety
/// `s` must point to a NUL-terminated string.
unsafe fn libc_strlen(s: *const c_char) -> usize {
    // SAFETY: caller guarantees NUL termination.
    unsafe {
        let mut n = 0usize;
        while *s.add(n) != 0 {
            n += 1;
        }
        n
    }
}

/// Decimal formatting of a signed 64-bit integer (Tcl's int string rep).
fn itoa(v: TclWideInt) -> Vec<u8> {
    if v == 0 {
        return vec![b'0'];
    }
    let neg = v < 0;
    let mut buf = Vec::new();
    // Work in u128 so i64::MIN's magnitude does not overflow.
    let mut n = (v as i128).unsigned_abs();
    while n > 0 {
        buf.push(b'0' + (n % 10) as u8);
        n /= 10;
    }
    if neg {
        buf.push(b'-');
    }
    buf.reverse();
    buf
}
