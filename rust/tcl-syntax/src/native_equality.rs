// SPDX-License-Identifier: AGPL-3.0-or-later
//! Full, case-sensitive native object equality with original cache transitions.
//!
//! An actual issuer selects the operation. Snapshots describe physical backing;
//! they neither grant execution permission nor manufacture object identity.

use crate::{
    naming::NativeNameProtocol,
    native_object::{NativeObjectCacheSnapshot as Cache, NativeObjectSnapshot},
    native_string::NativeStringStorageIdentity,
    raw_string::RawString,
    value::{ValueError, ValueOps},
};
use tcl_dialect::TclVersion;

fn empty(snapshot: &NativeObjectSnapshot, version: TclVersion) -> Result<Option<bool>, ValueError> {
    if version >= TclVersion::V9_0 {
        return crate::native_object::native_c9_string_emptiness(snapshot)
            .map(|shape| match shape {
                crate::native_object::NativeObjectStringEmptiness::Empty => Some(true),
                crate::native_object::NativeObjectStringEmptiness::Nonempty => Some(false),
                crate::native_object::NativeObjectStringEmptiness::Unknown => None,
            })
            .map_err(|_| ValueError::CommandProtocolUnavailable("native equality empty storage"));
    }
    if snapshot.storage == Some(NativeStringStorageIdentity::CanonicalEmpty) {
        return Ok(Some(true));
    }
    match snapshot.cache {
        Cache::List { length, .. } if snapshot.resident.is_none() => return Ok(Some(length == 0)),
        Cache::Dictionary { size, pure: true } => return Ok(Some(size == 0)),
        _ => {}
    }
    Ok(snapshot.resident.as_ref().map(|bytes| bytes.is_empty()))
}

fn backing(snapshot: &NativeObjectSnapshot, version: TclVersion) -> Option<&[u8]> {
    if let Cache::ByteArray { bytes, proper } = &snapshot.cache
        && if version >= TclVersion::V9_0 {
            *proper
        } else {
            snapshot.resident.is_none()
        }
    {
        return Some(bytes);
    }
    None
}

/// Reach native equality on the original objects, preserving each selected
/// string/count/Unicode preparation in native order.
///
/// # Errors
/// Refuses missing native authority, unknown physical identity/storage or
/// unsupported native access. Refusal stays outside guest completion.
pub fn full_native_equality<O: ValueOps>(
    ops: &mut O,
    left: &O::Value,
    right: &O::Value,
) -> Result<bool, ValueError> {
    let selected = ops
        .name_policy_protocol()
        .ok_or(ValueError::CommandProtocolUnavailable(
            "native object equality issuer",
        ))?;
    if selected.recipe().is_jim084() {
        let left_bytes = ops.native_string_bytes(left)?;
        let left_count = ops.native_char_len(left)?;
        let right_bytes = ops.native_string_bytes(right)?;
        let right_count = ops.native_char_len(right)?;
        return RawString::from_bytes(left_bytes)
            .jim084_compare(
                left_count,
                &RawString::from_bytes(right_bytes),
                right_count,
                false,
            )
            .map(std::cmp::Ordering::is_eq)
            .map_err(ValueError::from);
    }
    let NativeNameProtocol::C(version) = selected.recipe() else {
        unreachable!("C equality")
    };
    // This interface represents the TclStringCmp object equality entry.
    // C8.4/8.5 string-command entry points require their own purpose receipt.
    if version < TclVersion::V8_6 {
        return Err(ValueError::CommandProtocolUnavailable(
            "native object equality entry",
        ));
    }
    match ops.same_object(left, right) {
        Some(true) => return Ok(true),
        Some(false) => {}
        None => {
            return Err(ValueError::CommandProtocolUnavailable(
                "native object equality identity",
            ));
        }
    }
    let first = ops.native_object_snapshot(left)?;
    let second = ops.native_object_snapshot(right)?;
    if let (Some(first), Some(second)) = (backing(&first, version), backing(&second, version)) {
        return Ok(first == second);
    }
    if matches!(first.cache, Cache::String { .. }) && matches!(second.cache, Cache::String { .. }) {
        for snapshot in [&first, &second] {
            if let Cache::String { protocol, .. } = snapshot.cache
                && protocol != selected.string_protocol()
            {
                return Err(ValueError::CommandProtocolUnavailable(
                    "native equality cache origin",
                ));
            }
        }
        let first_count = ops.native_char_len(left)?;
        let second_count = ops.native_char_len(right)?;
        if first
            .resident
            .as_ref()
            .is_some_and(|bytes| bytes.len() == first_count)
            && second
                .resident
                .as_ref()
                .is_some_and(|bytes| bytes.len() == second_count)
        {
            return Ok(first.resident == second.resident);
        }
        // Unicode is prepared on both operands even if their lengths differ.
        let first_units = ops.native_unicode_units(left)?;
        let second_units = ops.native_unicode_units(right)?;
        return Ok(first_units == second_units);
    }
    match (empty(&first, version)?, empty(&second, version)?) {
        (Some(true), Some(true)) => return Ok(true),
        (Some(true), Some(false)) | (Some(false), Some(true)) => return Ok(false),
        (Some(true), None) => return Ok(ops.native_string_bytes(right)?.is_empty()),
        (None, Some(true)) => return Ok(ops.native_string_bytes(left)?.is_empty()),
        _ => {}
    }
    // Full equality's catch-all is a counted byte comparison, not decoded
    // character equality and not CString truncation.
    let first = ops.native_string_bytes(left)?;
    let second = ops.native_string_bytes(right)?;
    Ok(first == second)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;

    fn snapshot(cache: Cache, resident: Option<&[u8]>) -> NativeObjectSnapshot {
        NativeObjectSnapshot {
            resident: resident.map(Rc::from),
            storage: resident.map(|_| NativeStringStorageIdentity::Allocated),
            cache,
        }
    }

    #[test]
    fn binary_eligibility_uses_actual_release_and_backing() {
        let value = snapshot(
            Cache::ByteArray {
                bytes: Rc::from(b"x".as_slice()),
                proper: true,
            },
            Some(b"written"),
        );
        assert_eq!(backing(&value, TclVersion::V8_6), None);
        assert_eq!(backing(&value, TclVersion::V9_0), Some(b"x".as_slice()));
    }

    #[test]
    fn empty_list_resident_spelling_requires_actual_canonical_state() {
        let noncanonical = snapshot(
            Cache::List {
                length: 0,
                canonical: false,
            },
            Some(b" "),
        );
        assert_eq!(empty(&noncanonical, TclVersion::V9_0), Ok(Some(false)));
        let canonical = snapshot(
            Cache::List {
                length: 0,
                canonical: true,
            },
            Some(b" "),
        );
        assert_eq!(empty(&canonical, TclVersion::V9_0), Ok(Some(true)));
        assert_eq!(empty(&canonical, TclVersion::V8_6), Ok(Some(false)));
    }
}
