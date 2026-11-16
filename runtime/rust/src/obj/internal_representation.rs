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

//! Typed internal-rep plumbing: installing, changing and discarding an object's internal representation, and invalidating or setting its string rep.

use super::lifetime_handles::NativeObjectLifetime;
use super::{
    allocation_is_live, auxiliary, check_native_liveness, free_string_buffer,
    has_canonical_empty_string, has_string_rep, is_shared, obj_alloc, revoke_script_location,
    set_allocated_string, set_owned_string, TclObj, TclObjType,
};
use core::ffi::c_char;
use std::alloc::{realloc, Layout};

// Typed internal-rep helpers — the shimmer keystone's plumbing, used by the
// value-type modules (`list`, future `dict`, …). pub(crate): internal only.

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
    let _ = change_type_using::<tcl_syntax::value::ValueError>(obj, new_type, new_rep, None);
}

/// Retire the selected old primary before transferring the incoming backing.
/// Every Err leaves `new_rep` unadopted and owned by the caller. On Ok the
/// object owns it; no callback or checker runs after the raw field adoption.
/// A lifetime lease supplies allocation safety, never an extra native reference.
pub(crate) fn change_type_with_stage_check<E: From<tcl_syntax::value::ValueError>>(
    value: *mut TclObj,
    new_type: *const TclObjType,
    new_rep: u64,
    check: &mut dyn FnMut() -> Result<(), E>,
) -> Result<(), E> {
    change_type_using(value, new_type, new_rep, Some(check))
}

pub(super) fn next_primary_epoch(value: *mut TclObj) -> Option<u64> {
    let state = auxiliary(value);
    let next = state
        .primary_epoch
        .get()
        .and_then(|epoch| epoch.checked_add(1));
    state.primary_epoch.set(next);
    next
}

pub(super) fn change_type_using<E: From<tcl_syntax::value::ValueError>>(
    obj: *mut TclObj,
    new_type: *const TclObjType,
    new_rep: u64,
    mut check: Option<&mut dyn FnMut() -> Result<(), E>>,
) -> Result<(), E> {
    use tcl_syntax::value::ValueError;
    if let Some(check) = check.as_mut() {
        check()?;
        check_native_liveness(obj)?;
    }
    let _lease = check.as_ref().map(|_| NativeObjectLifetime::retain(obj));
    let epoch = next_primary_epoch(obj);
    if check.is_some() && epoch.is_none() {
        return Err(
            ValueError::CommandProtocolUnavailable("native primary publication epoch").into(),
        );
    }
    let old = obj_type_ptr(obj);
    let mut old_rep = internal_rep(obj);
    let retired_cache = auxiliary(obj).cache.take();
    // SAFETY: the original live header and selected old primary are retained.
    unsafe {
        if old.is_null() {
            shrink_string_to_exact(obj);
            old_rep = internal_rep(obj);
        } else if let Some(free) = (*old).free_int_rep_proc {
            free(obj);
        }
    }
    if let Some(check) = check.as_mut() {
        // Check first cause now, but settle the reached old primary even when
        // that check refuses. A reentrant publication must never be cleared.
        let after_free = check();
        let same_slot = allocation_is_live(obj)
            && auxiliary(obj).primary_epoch.get() == epoch
            && obj_type_ptr(obj) == old
            && internal_rep(obj) == old_rep;
        if same_slot {
            // SAFETY: only this original operation owns the vacated live slot.
            unsafe {
                (*obj).type_ptr = core::ptr::null();
                (*obj).internal_rep = if (*obj).bytes.is_null() || has_canonical_empty_string(obj) {
                    0
                } else {
                    (*obj).length as u64 + 1
                };
            }
        }
        drop(retired_cache);
        let after_retirement = check();
        after_free?;
        after_retirement?;
        if !same_slot {
            return Err(ValueError::CommandProtocolUnavailable(
                "changed native primary publication slot",
            )
            .into());
        }
        if auxiliary(obj).primary_epoch.get() != epoch || !obj_type_ptr(obj).is_null() {
            return Err(ValueError::CommandProtocolUnavailable(
                "changed native primary publication slot",
            )
            .into());
        }
        check()?;
    } else {
        // Neutral utilities retain their original free -> adoption -> auxiliary
        // retirement order and perform no entered-operation checks.
        unsafe {
            (*obj).type_ptr = new_type;
            (*obj).internal_rep = new_rep;
        }
        drop(retired_cache);
        return Ok(());
    }
    // SAFETY: final original check precedes this callback-free ownership transfer.
    unsafe {
        (*obj).type_ptr = new_type;
        (*obj).internal_rep = new_rep;
    }
    Ok(())
}

/// Drop a reached cache while retaining exact resident storage and capacity.
pub(crate) fn discard_native_internal_representation(
    value: *mut TclObj,
) -> Result<(), tcl_syntax::value::ValueError> {
    discard_native_internal_representation_using(value, None)
}

/// The same resident-cache retirement, checked before an entered consumer may
/// look up or adopt its actual next role. No incoming backing is constructed.
pub(crate) fn discard_native_internal_representation_with_stage_check<
    E: From<tcl_syntax::value::ValueError>,
>(
    value: *mut TclObj,
    check: &mut dyn FnMut() -> Result<(), E>,
) -> Result<(), E> {
    discard_native_internal_representation_using(value, Some(check))
}

fn discard_native_internal_representation_using<E: From<tcl_syntax::value::ValueError>>(
    value: *mut TclObj,
    mut check: Option<&mut dyn FnMut() -> Result<(), E>>,
) -> Result<(), E> {
    if let Some(check) = check.as_mut() {
        check()?;
    }
    check_native_liveness(value)?;
    if !has_string_rep(value) {
        return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "native cache discard resident string",
        )
        .into());
    }
    if obj_type_ptr(value).is_null() {
        return Ok(());
    }
    let capacity = if has_canonical_empty_string(value) {
        0
    } else {
        // SAFETY: a typed object's resident allocation has exactly length+1 bytes.
        unsafe { (*value).length as usize + 1 }
    };
    change_type_using(value, core::ptr::null(), capacity as u64, check)
}

/// Reset the same unshared C result header to canonical empty storage.
pub(crate) fn reset_native_c_result(value: *mut TclObj) {
    debug_assert!(!is_shared(value));
    // SAFETY: the interpreter owns this live unshared result. Publish its
    // canonical empty bytes before releasing the previous internal backing.
    unsafe { set_string_rep(value, b"") };
    change_type(value, core::ptr::null(), 0);
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
    revoke_script_location(obj);
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

/// Install the selected native updater storage without deriving canonical
/// empty identity from byte contents alone.
pub(crate) unsafe fn set_native_updater_string_rep(
    obj: *mut TclObj,
    bytes: &[u8],
    canonical_empty: bool,
) {
    unsafe {
        if bytes.is_empty() && !canonical_empty {
            set_allocated_string(obj, bytes.as_ptr(), bytes.len());
        } else {
            set_owned_string(obj, bytes.as_ptr(), bytes.len());
        }
    }
}
