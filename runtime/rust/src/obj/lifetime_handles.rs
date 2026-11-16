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

//! Handles that own or observe an object's lifetime: `Owned`, `NativeObjectLifetime`, `ProcedureObject` and the two receipts.

use super::internal_representation::{internal_rep, obj_type_ptr};
use super::{
    allocation_is_live, auxiliary, bytes_of, check_native_liveness, deallocate_object,
    decr_ref_count, has_string_rep, incr_ref_count, TclObj, TclObjType,
};
use std::rc::Rc;

/// Allocation-only receipt for an already selected original C84 primary.
/// Capturing it acquires no native reference and does not keep a backing alive.
pub struct NativePrimaryReceipt {
    pub(super) header: NativeObjectLifetime,
    pub(super) epoch: u64,
    pub(super) kind: *const TclObjType,
    representation: u64,
}

impl NativePrimaryReceipt {
    pub(crate) fn capture(value: *mut TclObj) -> Result<Self, tcl_syntax::value::ValueError> {
        check_native_liveness(value)?;
        let epoch = auxiliary(value).primary_epoch.get().ok_or(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "original primary epoch capacity",
            ),
        )?;
        Ok(Self {
            header: NativeObjectLifetime::retain(value),
            epoch,
            kind: obj_type_ptr(value),
            representation: internal_rep(value),
        })
    }
    pub(crate) fn check(&self, value: *mut TclObj) -> Result<(), tcl_syntax::value::ValueError> {
        self.check_for(value, "retired original C binary storage")
    }
    /// Same original primary observation for the actual caller's pure guard.
    /// A purpose label supplies typed refusal data, never new getter authority.
    pub(crate) fn check_for(
        &self,
        value: *mut TclObj,
        purpose: &'static str,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        check_native_liveness(value)?;
        self.header.checked_ptr()?;
        if self.header.as_ptr() != value
            || auxiliary(value).primary_epoch.get() != Some(self.epoch)
            || obj_type_ptr(value) != self.kind
            || internal_rep(value) != self.representation
        {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                purpose,
            ));
        }
        Ok(())
    }
}

/// Allocation-only observation of a genuinely released result during its free callback.
/// This cannot retain, mutate, convert or resurrect the retired object.
pub(crate) struct RetiringResultHeader {
    pub(super) original: NativeObjectLifetime,
    pub(super) kind: *const TclObjType,
}

impl RetiringResultHeader {
    pub(crate) fn capture(original: *mut TclObj) -> Result<Self, tcl_syntax::value::ValueError> {
        check_native_liveness(original)?;
        Ok(Self {
            original: NativeObjectLifetime::retain(original),
            kind: obj_type_ptr(original),
        })
    }
    pub(crate) fn resident_bytes(&self) -> Result<Rc<[u8]>, tcl_syntax::value::ValueError> {
        let value = self.original.as_ptr();
        let state = auxiliary(value);
        if state.live.get()
            || !state.retiring.get()
            || !self
                .original
                .incarnation
                .same_as(&state.incarnation.borrow())
            || obj_type_ptr(value) != self.kind
            // SAFETY: this allocation-only receipt pins the exact retiring header.
            || unsafe { (*value).ref_count } != 0
            || !has_string_rep(value)
        {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "actual retiring result resident String",
            ));
        }
        // The actual free callback has not returned, so its resident String is intact.
        Ok(Rc::from(bytes_of(value)))
    }
}

/// Memory safety lease with no native object reference. A native decr-to-zero
/// still retires the cache and its children immediately while this holds only
/// the allocation. A retired object cannot be promoted back into a native owner.
pub(crate) struct NativeObjectLifetime {
    pointer: *mut TclObj,
    pub(super) incarnation: tcl_runtime_api::native_object_incarnation::NativeObjectIncarnation,
}

impl NativeObjectLifetime {
    pub(crate) fn as_ptr(&self) -> *mut TclObj {
        self.pointer
    }
    pub(crate) fn checked_ptr(&self) -> Result<*mut TclObj, tcl_syntax::value::ValueError> {
        check_native_liveness(self.pointer)?;
        if !self
            .incarnation
            .same_as(&auxiliary(self.pointer).incarnation.borrow())
        {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "original object allocation incarnation",
            ));
        }
        Ok(self.pointer)
    }
    pub(crate) fn retain(value: *mut TclObj) -> Self {
        let state = auxiliary(value);
        assert!(state.live.get(), "lifetime lease on a retired object");
        state.lifetime_pins.set(
            state
                .lifetime_pins
                .get()
                .checked_add(1)
                .expect("object lifetime pins"),
        );
        Self {
            pointer: value,
            incarnation: state.incarnation.borrow().clone(),
        }
    }
}

impl Clone for NativeObjectLifetime {
    fn clone(&self) -> Self {
        let state = auxiliary(self.pointer);
        state.lifetime_pins.set(
            state
                .lifetime_pins
                .get()
                .checked_add(1)
                .expect("object lifetime pins"),
        );
        Self {
            pointer: self.pointer,
            incarnation: self.incarnation.clone(),
        }
    }
}

/// A borrowed object pointer provided by either a genuine reference or an
/// allocation-only procedure view. Native receivers acquire their own roles.
pub trait ObjectPointer {
    /// Borrow the selected original allocation.
    fn as_ptr(&self) -> *mut TclObj;
}

/// Memory-safe view of a procedure object without a native object reference.
/// A surviving view cannot revive resources retired by the last native role.
#[derive(Clone)]
pub struct ProcedureObject(NativeObjectLifetime);

impl ProcedureObject {
    pub(crate) fn retain_lifetime(original: &Owned) -> Self {
        Self(NativeObjectLifetime::retain(original.as_ptr()))
    }
    /// Borrow the original address; consumers must check native liveness before
    /// performing a native operation.
    #[must_use]
    pub fn as_ptr(&self) -> *mut TclObj {
        self.0.as_ptr()
    }
    /// Select a live original header without acquiring a native reference.
    pub(crate) fn checked_ptr(&self) -> Result<*mut TclObj, tcl_syntax::value::ValueError> {
        if !allocation_is_live(self.as_ptr()) {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "retired native procedure object",
            ));
        }
        self.0.checked_ptr()
    }
}

impl ObjectPointer for ProcedureObject {
    fn as_ptr(&self) -> *mut TclObj {
        self.as_ptr()
    }
}

impl Drop for NativeObjectLifetime {
    fn drop(&mut self) {
        let state = auxiliary(self.pointer);
        let remaining = state
            .lifetime_pins
            .get()
            .checked_sub(1)
            .expect("object lifetime retirement");
        state.lifetime_pins.set(remaining);
        if remaining == 0 && !state.live.get() && !state.retiring.get() {
            // SAFETY: all native resources were retired at the actual free
            // boundary, and this is the last memory safety owner.
            unsafe { deallocate_object(self.pointer) };
        }
    }
}

/// An owned object reference (`rc +1`) that releases on drop — the discipline
/// that keeps the shared recursive walk leak-/double-free-safe across early
/// returns.
pub struct Owned(*mut TclObj);

impl ObjectPointer for Owned {
    fn as_ptr(&self) -> *mut TclObj {
        self.as_ptr()
    }
}

impl Owned {
    /// Take an existing owning reference without retaining the original again.
    ///
    /// # Safety
    /// The caller must transfer one live, independently owned `+1` reference.
    pub(crate) unsafe fn from_raw(o: *mut TclObj) -> Owned {
        Owned(o)
    }

    /// Take an owning `+1` on a live object (e.g. a variable's store value).
    pub(crate) fn retain(o: *mut TclObj) -> Owned {
        // SAFETY: `o` is a live object.
        unsafe { incr_ref_count(o) };
        Owned(o)
    }

    /// Adopt a freshly-minted (`rc 0`) object, taking it to `rc 1`.
    pub(crate) fn fresh(o: *mut TclObj) -> Owned {
        // SAFETY: `o` is a fresh object from a constructor / tower op.
        unsafe { incr_ref_count(o) };
        Owned(o)
    }

    #[cfg(have_tommath)]
    #[inline]
    pub(crate) fn ptr(&self) -> *mut TclObj {
        self.0
    }

    /// The borrowed object pointer (the `+1` stays with this `Owned`). Callers
    /// that retain it (e.g. `Tcl_SetObjResult`, which takes its own `+1`) read
    /// through this and let the `Owned` drop its reference normally.
    #[inline]
    #[must_use]
    pub fn as_ptr(&self) -> *mut TclObj {
        self.0
    }

    /// Hand the `+1` to the caller without releasing it here.
    pub fn into_raw(self) -> *mut TclObj {
        let o = self.0;
        core::mem::forget(self);
        o
    }

    /// Reverse the genuine interpolation reference without freeing a native
    /// refcount-zero result header. The next real receiver must retain it.
    pub(crate) fn into_native_unowned(self) -> *mut TclObj {
        let value = self.into_raw();
        // SAFETY: this consumed Owned held exactly one genuine reference.
        unsafe {
            assert!((*value).ref_count > 0, "owned interpolation reference");
            (*value).ref_count -= 1;
        }
        value
    }
}

impl Clone for Owned {
    fn clone(&self) -> Self {
        Self::retain(self.0)
    }
}

impl Drop for Owned {
    fn drop(&mut self) {
        // SAFETY: `self.0` is the object we hold a `+1` on.
        unsafe { decr_ref_count(self.0) };
    }
}
