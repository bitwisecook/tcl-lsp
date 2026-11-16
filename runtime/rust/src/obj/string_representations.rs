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

//! The native string and Unicode representations: character counts, C string storage, Jim's string counts and in-place append.

use super::internal_representation::{
    alloc_typed, change_type, change_type_with_stage_check, internal_rep, invalidate_string,
    obj_type_ptr, set_native_updater_string_rep, set_string_rep,
};
use super::{
    allocation_is_live, bytes_of, check_native_liveness, has_canonical_empty_string,
    has_string_rep, is_shared, native_object_snapshot, revoke_script_location, set_owned_string,
    NativeStringRep, TclObj, TclObjType, TclSize, JIM_STRING_TYPE,
};
use crate::counters;
use core::ffi::c_char;
use std::alloc::{realloc, Layout};

pub(super) extern "C" fn native_string_free(value: *mut TclObj) {
    // SAFETY: this exact descriptor owns the boxed backing.
    unsafe {
        drop(Box::from_raw(
            internal_rep(value) as usize as *mut NativeStringRep
        ))
    };
}

pub(super) extern "C" fn native_string_dup(value: *mut TclObj, duplicate: *mut TclObj) {
    // SAFETY: the exact descriptor owns a live backing and duplicate is fresh.
    let backing = unsafe { &*(internal_rep(value) as usize as *const NativeStringRep) };
    if !backing
        .protocol
        .string_primary_survives_duplicate(backing.count)
    {
        return;
    }
    let copied = Box::new(backing.clone());
    change_type(
        duplicate,
        &NATIVE_STRING_TYPE,
        Box::into_raw(copied) as usize as u64,
    );
}

pub(super) extern "C" fn native_string_update(value: *mut TclObj) {
    // SAFETY: the exact descriptor owns the live backing. A stringless native
    // String is created only with validated retained Unicode units.
    let backing = unsafe { &*(internal_rep(value) as usize as *const NativeStringRep) };
    let Some(unicode) = &backing.unicode else {
        if backing.count == Some(0) {
            // SAFETY: the selected zero-count constructor owns a live empty String.
            unsafe { set_string_rep(value, b"") };
        }
        return;
    };
    let version = backing
        .protocol
        .tcl_version()
        .expect("native C String backing");
    let bytes = tcl_syntax::native_tcl_utf::NativeTclUtf::for_version(version)
        .encode_units(unicode)
        .expect("validated native Unicode unit width");
    // SAFETY: the owned encoded extent remains live through its buffer copy.
    unsafe { set_string_rep(value, &bytes) };
}

pub(super) static NATIVE_STRING_TYPE: TclObjType = TclObjType {
    name: c"string".as_ptr(),
    free_int_rep_proc: Some(native_string_free),
    dup_int_rep_proc: Some(native_string_dup),
    update_string_proc: Some(native_string_update),
    set_from_any_proc: None,
};

/// Reach the selected actual character-count protocol on the original object.
pub(crate) fn native_character_count(
    value: *mut TclObj,
    protocol: tcl_syntax::native_string::NativeStringProtocol,
    representation: tcl_registry::native_string_length::NativeStringLengthRepresentation,
) -> Result<usize, tcl_syntax::value::ValueError> {
    native_character_count_using(value, protocol, representation, &mut || Ok(()), &mut || {
        crate::dict::native_object_bytes(value, protocol).map(std::rc::Rc::from)
    })
}

/// Same reached count worker with the original operation and String producer.
/// No lifetime pin or temporary native reference protects old-primary cleanup.
pub(crate) fn native_character_count_checked(
    value: *mut TclObj,
    protocol: tcl_syntax::native_string::NativeStringProtocol,
    representation: tcl_registry::native_string_length::NativeStringLengthRepresentation,
    current: &mut impl FnMut() -> Result<(), tcl_cmd_core::CmdError>,
    original_string: &mut impl FnMut() -> Result<std::rc::Rc<[u8]>, tcl_cmd_core::CmdError>,
) -> Result<usize, tcl_cmd_core::CmdError> {
    native_character_count_using(value, protocol, representation, current, original_string)
}

fn native_character_count_using<E: From<tcl_syntax::value::ValueError>>(
    value: *mut TclObj,
    protocol: tcl_syntax::native_string::NativeStringProtocol,
    representation: tcl_registry::native_string_length::NativeStringLengthRepresentation,
    current: &mut dyn FnMut() -> Result<(), E>,
    original_string: &mut dyn FnMut() -> Result<std::rc::Rc<[u8]>, E>,
) -> Result<usize, E> {
    use tcl_syntax::value::ValueError;
    current()?;
    check_native_liveness(value)?;
    if !native_string_available(value) {
        return Err(ValueError::CommandProtocolUnavailable("native string length storage").into());
    }
    if let Some(length) = crate::bytearray::native_string_length(value, representation) {
        return Ok(length);
    }
    if representation.preserves_short_string()
        && has_string_rep(value)
        && unsafe { (*value).length } <= 1
    {
        return usize::try_from(unsafe { (*value).length }).map_err(|_| {
            ValueError::CommandProtocolUnavailable("native short String length").into()
        });
    }
    if protocol.is_jim084()
        && core::ptr::eq(obj_type_ptr(value), &JIM_STRING_TYPE)
        && internal_rep(value) != u64::MAX
    {
        return usize::try_from(internal_rep(value))
            .map_err(|_| ValueError::CommandProtocolUnavailable("Jim String count width").into());
    }
    if !protocol.is_jim084() && core::ptr::eq(obj_type_ptr(value), &NATIVE_STRING_TYPE) {
        // SAFETY: the exact live descriptor owns this resident count backing.
        let backing = unsafe { &*(internal_rep(value) as usize as *const NativeStringRep) };
        if backing.protocol != protocol {
            return Err(
                ValueError::CommandProtocolUnavailable("native string count cache origin").into(),
            );
        }
        if let Some(count) = backing.count {
            return Ok(count);
        }
    }
    let reached = original_string();
    current()?;
    check_native_liveness(value)?;
    let bytes = reached?;
    let count = if protocol.is_jim084() {
        tcl_syntax::raw_string::RawString::from_bytes(bytes.clone())
            .jim084_characters()
            .count()
    } else {
        let version = protocol
            .tcl_version()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native character units",
            ))?;
        tcl_syntax::native_tcl_utf::NativeTclUtf::for_version(version)
            .decode_units(&bytes)
            .len()
    };
    if protocol.is_jim084() {
        change_type_with_stage_check(
            value,
            &JIM_STRING_TYPE,
            u64::try_from(count).expect("native count fits u64"),
            current,
        )?;
    } else {
        let unicode = protocol
            .tcl_version()
            .filter(|version| *version <= tcl_dialect::TclVersion::V8_5)
            .filter(|_| count != bytes.len())
            .map(|version| {
                std::rc::Rc::from(
                    tcl_syntax::native_tcl_utf::NativeTclUtf::for_version(version)
                        .decode_storage_units(
                            &bytes,
                            tcl_runtime_api::native_c_string_abi::supported_backend_plain_char(),
                        ),
                )
            });
        let mut backing = Box::new(NativeStringRep {
            protocol,
            count: Some(count),
            unicode,
        });
        let representation = u64::try_from(core::ptr::from_mut(backing.as_mut()) as usize)
            .expect("native pointer fits u64");
        change_type_with_stage_check(value, &NATIVE_STRING_TYPE, representation, current)?;
        let _ = Box::into_raw(backing);
    }
    current()?;
    check_native_liveness(value)?;
    Ok(count)
}

/// Reach SetStringFromAny/count on the original header, without a public
/// character-length shortcut. Unit/range consumers select this purpose only
/// after their original index getters, independently of ByteArray count rules.
pub(crate) fn native_c_string_storage_checked<E: From<tcl_syntax::value::ValueError>>(
    value: *mut TclObj,
    protocol: tcl_syntax::native_string::NativeStringProtocol,
    current: &mut dyn FnMut() -> Result<(), E>,
    original_string: &mut dyn FnMut() -> Result<std::rc::Rc<[u8]>, E>,
) -> Result<tcl_cmd_core::string::NativeCStringStorage, E> {
    // The same count/adoption kernel supplies eager C84/85 Unicode and lazy
    // C86+ counts. String representation explicitly disables length shortcuts.
    let count = native_character_count_using(
        value,
        protocol,
        tcl_registry::native_string_length::NativeStringLengthRepresentation::String,
        current,
        original_string,
    )?;
    current()?;
    let snapshot = native_object_snapshot(value)?;
    let tcl_syntax::native_object::NativeObjectCacheSnapshot::String {
        protocol: origin,
        unicode,
        ..
    } = snapshot.cache
    else {
        return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "original C String storage primary",
        )
        .into());
    };
    if origin != protocol {
        return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "original C String storage issuer",
        )
        .into());
    }
    Ok(tcl_cmd_core::string::NativeCStringStorage {
        count,
        bytes: snapshot.resident,
        unicode,
    })
}

/// Install an actual Jim string result's native cached count without changing bytes.
pub(crate) fn retain_jim_string_count(value: *mut TclObj, count: usize) {
    change_type(
        value,
        &JIM_STRING_TYPE,
        u64::try_from(count).expect("native character count fits u64"),
    );
}

/// Install Jim's string intrep without calculating its unknown character count.
/// Native string operations preserve an existing cached count across conversion.
pub(crate) fn retain_jim_string_representation(value: *mut TclObj) {
    if !core::ptr::eq(obj_type_ptr(value), &JIM_STRING_TYPE) {
        change_type(value, &JIM_STRING_TYPE, u64::MAX);
    }
}

/// Retain the actual unknown-count String primary of an append/format producer.
/// Existing resident bytes remain owned by the same original object header.
pub(crate) fn retain_native_string_representation(
    value: *mut TclObj,
    protocol: tcl_registry::native_string_materialization::NativeStringMaterialization,
) -> Result<(), tcl_syntax::value::ValueError> {
    if !has_string_rep(value) {
        return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "native String producer resident storage",
        ));
    }
    if protocol.protocol().is_jim084() {
        retain_jim_string_representation(value);
    } else {
        let backing = Box::new(NativeStringRep {
            protocol: protocol.protocol(),
            count: None,
            unicode: None,
        });
        change_type(
            value,
            &NATIVE_STRING_TYPE,
            Box::into_raw(backing) as usize as u64,
        );
    }
    Ok(())
}

/// Apply a uniquely owned Jim suffix cut, preserving its current count receipt.
pub(crate) fn trim_jim_string_bytes(value: *mut TclObj, bytes: &[u8]) {
    debug_assert!(!is_shared(value));
    revoke_script_location(value);
    // SAFETY: the caller holds the live, unshared object and an independent
    // byte snapshot. Its existing string intrep and cached count are retained.
    unsafe { set_string_rep(value, bytes) };
}

/// Current native Jim string count; any internal-representation conversion
/// withdraws this receipt through the central type-change door.
pub(crate) fn jim_string_count(value: *mut TclObj) -> Option<usize> {
    // SAFETY: the value is live for its caller's object operation.
    unsafe {
        (std::ptr::eq((*value).type_ptr, &JIM_STRING_TYPE) && (*value).internal_rep != u64::MAX)
            .then(|| {
                usize::try_from((*value).internal_rep).expect("native cached count fits usize")
            })
    }
}

/// Jim's actual string count conversion. Reuse the object's current cache;
/// otherwise install the count of the original native byte units.
#[cfg_attr(not(have_tommath), allow(dead_code))]
pub(crate) fn jim_character_count(value: *mut TclObj) -> usize {
    if let Some(count) = jim_string_count(value) {
        return count;
    }
    let bytes = tcl_syntax::raw_string::RawString::from_bytes(bytes_of(value));
    let count = bytes.jim084_characters().count();
    retain_jim_string_count(value, count);
    count
}

/// Reach native C Unicode-unit storage on the original object.
pub(crate) fn new_native_unicode_obj(
    unicode: std::rc::Rc<[u32]>,
    dialect: tcl_registry::InvocationDialect,
) -> Result<*mut TclObj, tcl_syntax::value::ValueError> {
    use tcl_syntax::value::ValueError;
    let protocol =
        dialect
            .native_string_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native Unicode constructor",
            ))?;
    let version = protocol
        .tcl_version()
        .ok_or(ValueError::CommandProtocolUnavailable(
            "native Unicode constructor",
        ))?;
    tcl_syntax::native_tcl_utf::NativeTclUtf::for_version(version)
        .encode_units(&unicode)
        .ok_or(ValueError::CommandProtocolUnavailable(
            "native Unicode unit width",
        ))?;
    let backing = Box::new(NativeStringRep {
        protocol,
        count: Some(unicode.len()),
        unicode: protocol
            .unicode_constructor_has_unicode(unicode.len())
            .then_some(unicode),
    });
    Ok(alloc_typed(
        &NATIVE_STRING_TYPE,
        Box::into_raw(backing) as usize as u64,
    ))
}

/// Reach native C Unicode-unit storage on the original object.
pub(crate) fn native_unicode_units(
    value: *mut TclObj,
    protocol: tcl_syntax::native_string::NativeStringProtocol,
) -> Result<std::rc::Rc<[u32]>, tcl_syntax::value::ValueError> {
    native_unicode_units_using(value, protocol, &mut || Ok(()), &mut || {
        crate::dict::native_object_bytes(value, protocol).map(std::rc::Rc::from)
    })
}

pub(crate) fn native_unicode_units_checked<E: From<tcl_syntax::value::ValueError>>(
    value: *mut TclObj,
    protocol: tcl_syntax::native_string::NativeStringProtocol,
    current: &mut dyn FnMut() -> Result<(), E>,
    original_string: &mut dyn FnMut() -> Result<std::rc::Rc<[u8]>, E>,
) -> Result<std::rc::Rc<[u32]>, E> {
    native_unicode_units_using(value, protocol, current, original_string)
}

fn native_unicode_units_using<E: From<tcl_syntax::value::ValueError>>(
    value: *mut TclObj,
    protocol: tcl_syntax::native_string::NativeStringProtocol,
    current: &mut dyn FnMut() -> Result<(), E>,
    original_string: &mut dyn FnMut() -> Result<std::rc::Rc<[u8]>, E>,
) -> Result<std::rc::Rc<[u32]>, E> {
    use tcl_syntax::value::ValueError;
    current()?;
    check_native_liveness(value)?;
    let version = protocol
        .tcl_version()
        .ok_or(ValueError::CommandProtocolUnavailable(
            "native Unicode unit storage",
        ))?;
    if !native_string_available(value) {
        return Err(ValueError::CommandProtocolUnavailable("native Unicode string updater").into());
    }
    if core::ptr::eq(obj_type_ptr(value), &NATIVE_STRING_TYPE) {
        // SAFETY: exact live descriptor authenticates this original backing.
        let cached = unsafe { &*(internal_rep(value) as usize as *const NativeStringRep) };
        if cached.protocol != protocol {
            return Err(
                ValueError::CommandProtocolUnavailable("native Unicode cache origin").into(),
            );
        }
        if let Some(unicode) = &cached.unicode {
            let unicode = unicode.clone();
            current()?;
            return Ok(unicode);
        }
    }
    let reached = original_string();
    current()?;
    let bytes = reached?;
    let unicode: std::rc::Rc<[u32]> = std::rc::Rc::from(
        tcl_syntax::native_tcl_utf::NativeTclUtf::for_version(version).decode_storage_units(
            &bytes,
            tcl_runtime_api::native_c_string_abi::supported_backend_plain_char(),
        ),
    );
    let mut backing = Box::new(NativeStringRep {
        protocol,
        count: Some(unicode.len()),
        unicode: Some(unicode.clone()),
    });
    let rep = (&mut *backing as *mut NativeStringRep) as usize as u64;
    change_type_with_stage_check(value, &NATIVE_STRING_TYPE, rep, current)?;
    let _ = Box::into_raw(backing);
    current()?;
    Ok(unicode)
}

/// Whether this original cache has a resident string or a native updater.
/// Jim completion codes with absent strings cannot manufacture a spelling.
pub(crate) fn native_string_available(value: *mut TclObj) -> bool {
    if value.is_null() || !allocation_is_live(value) {
        return false;
    }
    if has_string_rep(value) {
        return true;
    }
    let kind = obj_type_ptr(value);
    if core::ptr::eq(kind, &NATIVE_STRING_TYPE) {
        // SAFETY: the exact descriptor owns this live backing.
        let backing = unsafe { &*(internal_rep(value) as usize as *const NativeStringRep) };
        return backing.unicode.is_some() || backing.count == Some(0);
    }
    // A descriptor's actual updater, rather than its name, owns materialisation.
    kind.is_null() || unsafe { (*kind).update_string_proc.is_some() }
}

/// Install the selected append String cache without invoking a previous updater.
pub(crate) fn set_native_append_string(
    value: *mut TclObj,
    protocol: tcl_syntax::native_string::NativeStringProtocol,
    resident: Option<(
        std::rc::Rc<[u8]>,
        tcl_syntax::native_string::NativeStringStorageIdentity,
    )>,
    count: Option<usize>,
    unicode: Option<std::rc::Rc<[u32]>>,
) -> Result<(), tcl_syntax::value::ValueError> {
    set_native_append_string_checked(value, protocol, resident, count, unicode, &mut || Ok(()))
}

pub(crate) fn set_native_append_string_checked<E: From<tcl_syntax::value::ValueError>>(
    value: *mut TclObj,
    protocol: tcl_syntax::native_string::NativeStringProtocol,
    resident: Option<(
        std::rc::Rc<[u8]>,
        tcl_syntax::native_string::NativeStringStorageIdentity,
    )>,
    count: Option<usize>,
    unicode: Option<std::rc::Rc<[u32]>>,
    current: &mut dyn FnMut() -> Result<(), E>,
) -> Result<(), E> {
    use tcl_syntax::native_string::NativeStringStorageIdentity as Storage;
    use tcl_syntax::value::ValueError;
    current()?;
    check_native_liveness(value)?;
    if resident.as_ref().is_some_and(|(bytes, storage)| {
        *storage == Storage::Unknown || (*storage == Storage::CanonicalEmpty && !bytes.is_empty())
    }) {
        return Err(
            ValueError::CommandProtocolUnavailable("native append resident storage").into(),
        );
    }
    if protocol.is_jim084() && (resident.is_none() || unicode.is_some()) {
        return Err(ValueError::CommandProtocolUnavailable("Jim append String backing").into());
    }
    invalidate_string(value);
    if protocol.is_jim084() {
        change_type_with_stage_check(
            value,
            &JIM_STRING_TYPE,
            count.map_or(u64::MAX, |count| count as u64),
            current,
        )?;
    } else {
        let mut backing = Box::new(NativeStringRep {
            protocol,
            count,
            unicode,
        });
        let rep = (&mut *backing as *mut NativeStringRep) as usize as u64;
        change_type_with_stage_check(value, &NATIVE_STRING_TYPE, rep, current)?;
        let _ = Box::into_raw(backing);
    }
    if let Some((bytes, storage)) = resident {
        // SAFETY: accepted adoption owns this live original String backing.
        unsafe { set_native_updater_string_rep(value, &bytes, storage == Storage::CanonicalEmpty) };
    }
    current()
}

/// Whether `obj` is a plain string (no typed internal rep) — the precondition
/// for [`string_append_inplace`].
pub(crate) fn is_plain_string(obj: *mut TclObj) -> bool {
    // SAFETY: `obj` is a live object.
    unsafe { (*obj).type_ptr.is_null() }
}

/// Append `piece` to a **plain string** object in place, growing its buffer
/// geometrically (amortised O(1)). The caller must ensure `obj` is a
/// plain string ([`is_plain_string`]) and unshared. Refreshes `bytes`/`length`
/// and the capacity in `internal_rep`. A `realloc` keeps the single live-buffer
/// count, so the leak counters stay balanced.
pub(crate) fn string_append_inplace(obj: *mut TclObj, piece: &[u8]) {
    if piece.is_empty() {
        return;
    }
    revoke_script_location(obj);
    // SAFETY: caller guarantees a live, unshared, plain-string `obj` whose
    // buffer was allocated by `set_owned_string` (capacity in `internal_rep`).
    unsafe {
        if has_canonical_empty_string(obj) {
            set_owned_string(obj, piece.as_ptr(), piece.len());
            return;
        }
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
