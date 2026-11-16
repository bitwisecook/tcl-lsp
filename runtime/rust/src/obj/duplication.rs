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

//! Duplicating an object, and duplicating into an existing receiver, with the staged currency checks.

use super::internal_representation::{
    change_type_with_stage_check, internal_rep, invalidate_string, next_primary_epoch, obj_type_ptr,
};
use super::lifetime_handles::NativeObjectLifetime;
use super::{
    allocation_is_live, auxiliary, check_native_liveness, copy_scalar_object_context, free_obj,
    has_canonical_empty_string, new_string_bytes, obj_alloc, replace_scalar_object_context,
    replace_scalar_object_context_from, retain_script_location, revoke_script_location,
    script_location, set_allocated_string, set_owned_string, TclObj,
};

/// `Tcl_DuplicateObj` — a fresh (`rc 0`) deep copy: the string rep (if any) plus
/// the internal rep (via the type's `dup_int_rep_proc`, or a raw copy for
/// self-contained reps like int/double).
pub(crate) fn duplicate(src: *mut TclObj) -> *mut TclObj {
    duplicate_using::<tcl_syntax::value::ValueError>(src, None, &mut |_| {})
        .expect("neutral duplicate has no checked stage")
}

/// Duplicate the selected original and check before admitting its fresh result.
/// The original context is captured before its actual dup callback. Refusal is
/// retained before mandatory fresh cleanup, which may itself retire callbacks.
pub(crate) fn duplicate_with_stage_check<E: From<tcl_syntax::value::ValueError>>(
    source: *mut TclObj,
    check: &mut dyn FnMut() -> Result<(), E>,
    retain: &mut dyn FnMut(&E),
) -> Result<*mut TclObj, E> {
    duplicate_using(source, Some(check), retain)
}

fn duplicate_using<E: From<tcl_syntax::value::ValueError>>(
    src: *mut TclObj,
    mut check: Option<&mut dyn FnMut() -> Result<(), E>>,
    retain: &mut dyn FnMut(&E),
) -> Result<*mut TclObj, E> {
    if let Some(check) = check.as_mut() {
        check()?;
        check_native_liveness(src)?;
    }
    let _lease = check.as_ref().map(|_| NativeObjectLifetime::retain(src));
    let source_epoch = auxiliary(src).primary_epoch.get();
    if check.is_some() && source_epoch.is_none() {
        let first: E = tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "original duplicate source primary epoch",
        )
        .into();
        retain(&first);
        return Err(first);
    }
    let source_type = obj_type_ptr(src);
    let source_rep = internal_rep(src);
    let source_context = crate::native_source::capture_object_context(src);
    let source_scalar = auxiliary(src).scalar_context.borrow().clone();
    let source_location = script_location(src);
    if crate::native_source::selected_string_protocol(src).is_some_and(|protocol| {
        // SAFETY: duplicate's caller supplies this live original header.
        let resident_length =
            unsafe { (!(*src).bytes.is_null()).then_some((*src).length as usize) };
        protocol.object_header_duplicate_action(resident_length)
            == tcl_syntax::native_string::NativeObjectHeaderDuplicateAction::CanonicalEmptyString
    }) {
        let duplicate = new_string_bytes(b"");
        if let Some(check) = check.as_mut() {
            if let Err(first) = check().and_then(|()| check_native_liveness(src).map_err(E::from)) {
                retain(&first);
                retire_checked_duplicate(duplicate);
                return Err(first);
            }
        }
        crate::native_source::copy_context(src, duplicate);
        copy_scalar_object_context(src, duplicate);
        return Ok(duplicate);
    }
    let dup = obj_alloc();
    if dup.is_null() {
        return if check.is_some() {
            Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "original duplicate allocation",
            )
            .into())
        } else {
            Ok(dup)
        };
    }
    let _duplicate_lease = check.as_ref().map(|_| NativeObjectLifetime::retain(dup));
    // SAFETY: `src` is live; `dup` is freshly owned and uniquely ours.
    unsafe {
        if !(*src).bytes.is_null() {
            if has_canonical_empty_string(src) {
                set_owned_string(dup, (*src).bytes as *const u8, (*src).length as usize);
            } else {
                set_allocated_string(dup, (*src).bytes as *const u8, (*src).length as usize);
            }
        }
        let tp = (*src).type_ptr;
        if !tp.is_null() {
            match (*tp).dup_int_rep_proc {
                Some(dup_proc) => dup_proc(src, dup), // e.g. list_dup deep-copies
                None => {
                    // self-contained rep (int/double): copy type + raw 8 bytes
                    (*dup).type_ptr = tp;
                    (*dup).internal_rep = (*src).internal_rep;
                    let cache = auxiliary(src).cache.borrow().clone();
                    auxiliary(dup).cache.replace(cache);
                }
            }
        }
    }
    if let Some(check) = check.as_mut() {
        let reached = check().and_then(|()| {
            check_native_liveness(src)?;
            check_native_liveness(dup)?;
            if auxiliary(src).primary_epoch.get() != source_epoch
                || obj_type_ptr(src) != source_type
                || internal_rep(src) != source_rep
            {
                return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "changed original duplicate source primary",
                )
                .into());
            }
            Ok(())
        });
        if let Err(first) = reached {
            retain(&first);
            retire_checked_duplicate(dup);
            return Err(first);
        }
        crate::native_source::install_captured_object_context(dup, &source_context);
        auxiliary(dup).scalar_context.replace(source_scalar);
        if let Some((file, line)) = source_location {
            retain_script_location(dup, file, line);
        }
    } else {
        crate::native_source::copy_context(src, dup);
        copy_scalar_object_context(src, dup);
        if let Some((file, line)) = script_location(src) {
            retain_script_location(dup, file, line);
        }
    }
    Ok(dup)
}

/// Replace the receiver through the same checked primary retirement owner.
/// Capture the receiver primary before the source dup callback and refuse any
/// replacement, including a change followed by restoration, before retirement.
/// Incoming duplicate backing remains caller-owned until successful admission.
/// Native reference counts of source and receiver remain unchanged throughout.
pub(crate) fn duplicate_into_with_stage_check<E: From<tcl_syntax::value::ValueError>>(
    receiver: *mut TclObj,
    source: *mut TclObj,
    check: &mut dyn FnMut() -> Result<(), E>,
    retain: &mut dyn FnMut(&E),
) -> Result<(), E> {
    check()?;
    check_native_liveness(receiver)?;
    let _receiver = NativeObjectLifetime::retain(receiver);
    let receiver_epoch = auxiliary(receiver).primary_epoch.get();
    if receiver_epoch.is_none() {
        let first: E = tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "original duplicate receiver primary epoch",
        )
        .into();
        retain(&first);
        return Err(first);
    }
    let receiver_type = obj_type_ptr(receiver);
    let receiver_rep = internal_rep(receiver);
    let receiver_context = auxiliary(receiver).scalar_context.borrow().clone();
    let duplicate = duplicate_with_stage_check(source, check, retain)?;
    let _incoming = NativeObjectLifetime::retain(duplicate);
    let receiver_current = check().and_then(|()| {
        check_native_liveness(receiver)?;
        if auxiliary(receiver).primary_epoch.get() != receiver_epoch
            || obj_type_ptr(receiver) != receiver_type
            || internal_rep(receiver) != receiver_rep
        {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "changed original duplicate receiver primary",
            )
            .into());
        }
        Ok(())
    });
    if let Err(first) = receiver_current {
        retain(&first);
        retire_checked_duplicate(duplicate);
        return Err(first);
    }
    let incoming_context = auxiliary(duplicate).scalar_context.borrow().clone();
    let incoming_type = obj_type_ptr(duplicate);
    let incoming_rep = internal_rep(duplicate);
    let incoming_epoch = auxiliary(duplicate).primary_epoch.get();
    if incoming_epoch.is_none() {
        let first: E = tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "original duplicate incoming primary epoch",
        )
        .into();
        retain(&first);
        retire_checked_duplicate(duplicate);
        return Err(first);
    }
    let reached = change_type_with_stage_check(receiver, incoming_type, incoming_rep, &mut || {
        check()?;
        check_native_liveness(duplicate)?;
        if auxiliary(duplicate).primary_epoch.get() != incoming_epoch
            || obj_type_ptr(duplicate) != incoming_type
            || internal_rep(duplicate) != incoming_rep
        {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "changed original duplicate incoming backing",
            )
            .into());
        }
        Ok(())
    });
    if let Err(first) = reached {
        retain(&first);
        retire_checked_duplicate(duplicate);
        return Err(first);
    }
    // No callback/checker occurs between primary adoption and its representation
    // transport. The duplicate's incoming primary now belongs to the receiver.
    unsafe {
        (*duplicate).type_ptr = core::ptr::null();
        (*duplicate).internal_rep =
            if (*duplicate).bytes.is_null() || has_canonical_empty_string(duplicate) {
                0
            } else {
                (*duplicate).length as u64 + 1
            };
    }
    let cache = auxiliary(duplicate).cache.take();
    auxiliary(receiver).cache.replace(cache);
    replace_scalar_object_context_from(receiver, receiver_context, incoming_context);
    invalidate_string(receiver);
    unsafe {
        if !(*duplicate).bytes.is_null() {
            if has_canonical_empty_string(duplicate) {
                set_owned_string(
                    receiver,
                    (*duplicate).bytes as *const u8,
                    (*duplicate).length as usize,
                );
            } else {
                set_allocated_string(
                    receiver,
                    (*duplicate).bytes as *const u8,
                    (*duplicate).length as usize,
                );
            }
        }
    }
    let context = crate::native_source::capture_object_context(duplicate);
    crate::native_source::install_captured_object_context(receiver, &context);
    revoke_script_location(receiver);
    if let Some((file, line)) = script_location(duplicate) {
        retain_script_location(receiver, file, line);
    }
    retire_checked_duplicate(duplicate);
    check()
}

/// The exact constructor role is known; an already retired selected header
/// needs no second cleanup and cannot be promoted back into a native reference.
fn retire_checked_duplicate(duplicate: *mut TclObj) {
    if allocation_is_live(duplicate) {
        crate::interp::drop_fresh(duplicate);
    }
}

/// Replace one live object's representations with a duplicate, keeping its identity and refs.
pub(crate) fn duplicate_into(receiver: *mut TclObj, source: *mut TclObj) {
    let _ = next_primary_epoch(receiver);
    let duplicate = duplicate(source);
    replace_scalar_object_context(receiver, source);
    // SAFETY: both objects are live, and duplicate owns independent string/type storage.
    unsafe {
        let receiver_refs = (*receiver).ref_count;
        core::ptr::swap(receiver, duplicate);
        let incoming_cache = auxiliary(duplicate).cache.take();
        let retired_cache = auxiliary(receiver).cache.replace(incoming_cache);
        auxiliary(duplicate).cache.replace(retired_cache);
        (*receiver).ref_count = receiver_refs;
        (*duplicate).ref_count = 0;
        free_obj(duplicate);
    }
    revoke_script_location(receiver);
    if let Some((file, line)) = script_location(source) {
        retain_script_location(receiver, file, line);
    }
}
