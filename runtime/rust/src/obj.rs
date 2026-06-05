//! The `Tcl_Obj` value model + refcount discipline (Track 1, T1.1).
//!
//! This is the **real** port, not the leaking spike (`runtime/rust-spike/`):
//! every allocation is balanced by a free driven by the refcount reaching
//! zero, and the alloc/free counters (`crate::counters`) prove it.
//!
//! ## Layout — the C-extension ABI requires `#[repr(C)]`
//!
//! `docs/design/runtime/c-extension-abi.md` §4.2 fixes the layout: extensions
//! read `objPtr->refCount` / `objPtr->bytes` directly through `tcl.h` macros,
//! so the struct must be `#[repr(C)]` with the exact field order
//! `tcl.h` declares — `{ refCount, bytes, length, typePtr, internalRep }`. On
//! `wasm32` that is `{ isize, ptr, isize, ptr, <8-byte union> }`. This is the
//! single canonical obj model for the port (the Zig runtime's 32-byte
//! handle/tagged-immediate layout is a Zig-internal codegen detail; the Rust
//! port serves the same `tcl_*`/`obj_*` codegen primitives over the
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
use std::alloc::{alloc, dealloc, Layout};

use crate::counters;

/// `Tcl_Size` — `ptrdiff_t` in `tcl.h` (Tcl 9 width-agnostic size type).
pub type TclSize = isize;
/// `Tcl_WideInt` — always 64-bit.
pub type TclWideInt = i64;

/// `Tcl_ObjType` — the registered type descriptor (`tcl.h`). For T1.1 only the
/// `name` is load-bearing (so `typePtr` discriminates int/double/string and a
/// future `Tcl_GetObjType` can find it); the four procs are filled in when
/// custom `Tcl_ObjType` registration lands (Track 2).
#[repr(C)]
pub struct TclObjType {
    pub name: *const c_char,
    pub free_int_rep_proc: *const c_void,
    pub dup_int_rep_proc: *const c_void,
    pub update_string_proc: *const c_void,
    pub set_from_any_proc: *const c_void,
}

// SAFETY: these are immortal `'static` descriptors; the raw `name` pointer is
// to a `'static` NUL-terminated byte string. They are read-only and shared.
unsafe impl Sync for TclObjType {}

const fn obj_type(name: &'static [u8]) -> TclObjType {
    TclObjType {
        name: name.as_ptr() as *const c_char,
        free_int_rep_proc: core::ptr::null(),
        dup_int_rep_proc: core::ptr::null(),
        update_string_proc: core::ptr::null(),
        set_from_any_proc: core::ptr::null(),
    }
}

/// The `int` (wide) type descriptor. `typePtr == &TCL_INT_TYPE` ⇒ the value is
/// in `internal_rep` as a `TclWideInt`; `bytes` may be null until shimmered.
pub static TCL_INT_TYPE: TclObjType = obj_type(b"int\0");
/// The `double` type descriptor.
pub static TCL_DOUBLE_TYPE: TclObjType = obj_type(b"double\0");

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
        free_string_buffer(obj);
        dealloc(obj as *mut u8, obj_layout());
    }
    counters::obj_freed();
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
        let cap = (*obj).length as usize + 1;
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
            // Generate the string rep from the internal rep.
            if (*obj).type_ptr == &TCL_INT_TYPE {
                let s = itoa((*obj).wide());
                set_owned_string(obj, s.as_ptr(), s.len());
            } else if (*obj).type_ptr == &TCL_DOUBLE_TYPE {
                // T1.1 placeholder formatting — Tcl-faithful %.17g-style double
                // string generation lands with the string-ops port (T1.5).
                let s = format!("{}", (*obj).double());
                set_owned_string(obj, s.as_ptr(), s.len());
            } else {
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
