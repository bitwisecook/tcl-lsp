//! Minimal `Tcl_Interp` — just the result slot, for T1.1.
//!
//! Enough to exercise the `Tcl_SetObjResult` → `Tcl_GetObjResult` ownership
//! handshake (`c-api-ownership-contract.md`): the interp **retains** the result
//! object (+1) and releases the prior result, so a `fresh_zero` object handed
//! to `Tcl_SetObjResult` becomes interp-owned with no explicit refcount call.
//! The eval loop, frames, namespaces, and command table land in T1.3/T1.4.

use crate::obj::{self, TclObj};

/// `Tcl_Interp` (opaque to extensions). For T1.1 it holds only the current
/// result object, which it owns a `+1` reference to (never null after `new`).
#[repr(C)]
pub struct Interp {
    result: *mut TclObj,
}

impl Interp {
    /// Create an interp whose result is a fresh empty string object, owned by
    /// the interp (refCount 1).
    pub fn new() -> Box<Interp> {
        let result = obj::new_obj();
        // SAFETY: `result` is a freshly created live object; the interp takes
        // the owning reference.
        unsafe {
            obj::incr_ref_count(result);
        }
        Box::new(Interp { result })
    }

    /// `Tcl_SetObjResult`: the interp retains `obj` and releases the prior
    /// result. `obj` is `borrowed→stored` — the caller's reference is
    /// unaffected; a `fresh_zero` object thereby becomes interp-owned.
    ///
    /// # Safety
    /// `obj` must be a live `TclObj`.
    pub unsafe fn set_obj_result(&mut self, obj: *mut TclObj) {
        let old = self.result;
        // SAFETY: `obj` is live (caller); `old` is the interp's owned result.
        unsafe {
            obj::incr_ref_count(obj);
            self.result = obj;
            obj::decr_ref_count(old);
        }
    }

    /// `Tcl_GetObjResult`: the interp's current result, `borrowed` (the interp
    /// keeps its `+1`). Valid until the next result-changing call.
    pub fn get_obj_result(&self) -> *mut TclObj {
        self.result
    }
}

impl Drop for Interp {
    /// Tearing down the interp releases its hold on the result object, which
    /// frees it if the interp was the last owner. This is what makes the T1.1
    /// round-trip end at zero residual.
    fn drop(&mut self) {
        // SAFETY: `result` is the interp's owned reference, dropped exactly once.
        unsafe {
            obj::decr_ref_count(self.result);
        }
        self.result = core::ptr::null_mut();
    }
}
