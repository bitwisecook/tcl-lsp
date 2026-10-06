// SPDX-License-Identifier: AGPL-3.0-or-later
//! `TclOO` variable declaration validation and slot selection.
//!
//! Accepted declarations retain their full counted names and original values.
//! `CString` validation and failure reporting do not alter declaration identity.
//! Property membership and runtime variable lookup have separate owners.

use super::{NameProjectionUnavailable, NativeNameProtocol};
use crate::native_glob::{NativeNameGlobPurpose, match_native_name_pattern};
use tcl_core_types::c_string_extent;
use tcl_dialect::TclVersion;

/// The public operation selected on the native variable declaration slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeOoVariableSlotOperation {
    Append,
    AppendIfNew,
    Clear,
    Prepend,
    Remove,
    Set,
}

/// Slot selection before declaration validation or mutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeOoVariableSlotSelection {
    /// Actual native slot operation.
    pub operation: NativeOoVariableSlotOperation,
    /// Whether the first operand is the selected slot method.
    pub consumes_first: bool,
}

/// Counted native failure bytes and explicit error-code words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeOoVariableError {
    /// Native diagnostic bytes, separate from the retained declaration key.
    pub message: Vec<u8>,
    /// Explicit error-code words selected by this failure.
    pub error_code: Vec<Vec<u8>>,
}

/// Apply a native slot list operation while retaining each original record.
/// Append/prepend preserve duplicates; remove and append-if-new use counted
/// `ObjHash` membership. Field setters separately validate and unique their list.
#[must_use]
pub fn apply_native_oo_slot_records<T>(
    existing: Vec<T>,
    incoming: Vec<T>,
    operation: NativeOoVariableSlotOperation,
    key: impl Fn(&T) -> &[u8],
) -> Vec<T> {
    use NativeOoVariableSlotOperation as Operation;
    match operation {
        Operation::Clear => Vec::new(),
        Operation::Set => incoming,
        Operation::Append => existing.into_iter().chain(incoming).collect(),
        Operation::Prepend => incoming.into_iter().chain(existing).collect(),
        Operation::Remove => existing
            .into_iter()
            .filter(|old| !incoming.iter().any(|new| key(new) == key(old)))
            .collect(),
        Operation::AppendIfNew => {
            let mut seen: std::collections::BTreeSet<Vec<u8>> =
                existing.iter().map(|entry| key(entry).to_vec()).collect();
            existing
                .into_iter()
                .chain(
                    incoming
                        .into_iter()
                        .filter(|entry| seen.insert(key(entry).to_vec())),
                )
                .collect()
        }
    }
}

/// Apply a declaration slot operation with native counted `ObjHash` membership.
/// The key must be the materialised full counted name of each retained object.
/// Validation is applied to the returned list, not removal operands. Selected
/// records retain their original object identity and the first equal key.
#[must_use]
pub fn apply_native_oo_variable_slot<T>(
    existing: Vec<T>,
    incoming: Vec<T>,
    operation: NativeOoVariableSlotOperation,
    key: impl Fn(&T) -> &[u8],
) -> Vec<T> {
    let candidates = apply_native_oo_slot_records(existing, incoming, operation, &key);
    let mut seen = std::collections::BTreeSet::new();
    candidates
        .into_iter()
        .filter(|entry| seen.insert(key(entry).to_vec()))
        .collect()
}

fn version(protocol: NativeNameProtocol) -> Result<TclVersion, NameProjectionUnavailable> {
    match protocol {
        NativeNameProtocol::C(version) if version >= TclVersion::V8_6 => Ok(version),
        _ => Err(NameProjectionUnavailable::PurposeNotModelled),
    }
}

/// Select the exact native method or the default append operation.
/// A name starting with `-` is a method lookup, without prefix abbreviation.
///
/// # Errors
/// Refuses engines without an audited `TclOO` variable slot.
pub fn native_oo_variable_slot(
    protocol: NativeNameProtocol,
    first: Option<&[u8]>,
) -> Result<Result<NativeOoVariableSlotSelection, NativeOoVariableError>, NameProjectionUnavailable>
{
    use NativeOoVariableSlotOperation as Operation;
    let version = version(protocol)?;
    let Some(first) = first.filter(|first| first.first() == Some(&b'-')) else {
        return Ok(Ok(NativeOoVariableSlotSelection {
            operation: Operation::Append,
            consumes_first: false,
        }));
    };
    let operation = match first {
        b"-append" => Some(Operation::Append),
        b"-clear" => Some(Operation::Clear),
        b"-set" => Some(Operation::Set),
        b"-appendifnew" if version >= TclVersion::V9_0 => Some(Operation::AppendIfNew),
        b"-prepend" if version >= TclVersion::V9_0 => Some(Operation::Prepend),
        b"-remove" if version >= TclVersion::V9_0 => Some(Operation::Remove),
        _ => None,
    };
    if let Some(operation) = operation {
        return Ok(Ok(NativeOoVariableSlotSelection {
            operation,
            consumes_first: true,
        }));
    }
    let reported = c_string_extent(first);
    let mut message = b"unknown method \"".to_vec();
    message.extend_from_slice(reported);
    message.extend_from_slice(if version >= TclVersion::V9_0 {
        b"\": must be -append, -appendifnew, -clear, -prepend, -remove or -set"
    } else {
        b"\": must be -append, -clear or -set"
    });
    Ok(Err(NativeOoVariableError {
        message,
        error_code: vec![
            b"TCL".to_vec(),
            b"LOOKUP".to_vec(),
            b"METHOD".to_vec(),
            reported.to_vec(),
        ],
    }))
}

/// Validate a declaration's native `CString` prefix while retaining full identity.
/// `None` means the full counted original is accepted unchanged.
///
/// # Errors
/// Refuses unaudited engines or unavailable native matcher recipes.
pub fn validate_native_oo_variable(
    protocol: NativeNameProtocol,
    original: &[u8],
) -> Result<Option<NativeOoVariableError>, NameProjectionUnavailable> {
    version(protocol)?;
    let reported = c_string_extent(original);
    let reason: &[u8] = if reported.windows(2).any(|bytes| bytes == b"::") {
        b"contain namespace separators"
    } else if match_native_name_pattern(
        protocol,
        NativeNameGlobPurpose::OoDeclaredVariableValidation,
        b"*(*)",
        original,
    )
    .map_err(|_| NameProjectionUnavailable::PurposeNotModelled)?
    {
        b"refer to an array element"
    } else {
        return Ok(None);
    };
    let mut message = b"invalid declared variable name \"".to_vec();
    message.extend_from_slice(reported);
    message.extend_from_slice(b"\": must not ");
    message.extend_from_slice(reason);
    Ok(Some(NativeOoVariableError {
        message,
        error_code: vec![b"TCL".to_vec(), b"OO".to_vec(), b"BAD_DECLVAR".to_vec()],
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_and_counted_declaration_identity_remain_independent() {
        for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
            let protocol = NativeNameProtocol::C(version);
            for accepted in [
                b"a\0z".as_slice(),
                b"a\0z::q",
                b"a\0z(k)",
                b"a(k\0z)",
                b"a\xff",
                b"a\xc0\x80z",
            ] {
                assert_eq!(
                    validate_native_oo_variable(protocol, accepted).unwrap(),
                    None
                );
            }
            for rejected in [b"a::q".as_slice(), b"a(k)", b"a(k\xc0\x80z)"] {
                let error = validate_native_oo_variable(protocol, rejected)
                    .unwrap()
                    .unwrap();
                assert_eq!(
                    error.error_code,
                    vec![b"TCL".to_vec(), b"OO".to_vec(), b"BAD_DECLVAR".to_vec()]
                );
            }
        }
        assert!(validate_native_oo_variable(NativeNameProtocol::Jim084, b"a").is_err());
    }

    #[test]
    fn slot_reduction_preserves_full_keys_and_original_selected_records() {
        use NativeOoVariableSlotOperation as Operation;
        let existing = vec![(b"a\0old".to_vec(), 1), (b"a".to_vec(), 2)];
        let incoming = vec![(b"a\0new".to_vec(), 3), (b"a".to_vec(), 4)];
        fn key(entry: &(Vec<u8>, u8)) -> &[u8] {
            &entry.0
        }
        assert_eq!(
            apply_native_oo_variable_slot(
                existing.clone(),
                incoming.clone(),
                Operation::Append,
                key
            ),
            vec![
                (b"a\0old".to_vec(), 1),
                (b"a".to_vec(), 2),
                (b"a\0new".to_vec(), 3)
            ]
        );
        assert_eq!(
            apply_native_oo_variable_slot(
                existing.clone(),
                incoming.clone(),
                Operation::Prepend,
                key
            ),
            vec![
                (b"a\0new".to_vec(), 3),
                (b"a".to_vec(), 4),
                (b"a\0old".to_vec(), 1)
            ]
        );
        assert_eq!(
            apply_native_oo_variable_slot(existing, incoming, Operation::Remove, key),
            vec![(b"a\0old".to_vec(), 1)]
        );
    }

    #[test]
    fn generic_slot_keeps_original_duplicate_members_until_field_setter() {
        use NativeOoVariableSlotOperation as Operation;
        let old = vec![(b"a\0z".to_vec(), 1), (b"a\0z".to_vec(), 2)];
        let new = vec![(b"a\0z".to_vec(), 3), (b"a".to_vec(), 4)];
        let appended =
            apply_native_oo_slot_records(old.clone(), new.clone(), Operation::Append, |_| {
                panic!("append does not read member keys")
            });
        assert_eq!(appended.len(), 4);
        assert_eq!(
            appended.iter().map(|record| record.1).collect::<Vec<_>>(),
            vec![1, 2, 3, 4]
        );
        let unique =
            apply_native_oo_variable_slot(old.clone(), new.clone(), Operation::Append, |record| {
                record.0.as_slice()
            });
        assert_eq!(
            unique.iter().map(|record| record.1).collect::<Vec<_>>(),
            vec![1, 4]
        );
        let added = apply_native_oo_slot_records(old, new, Operation::AppendIfNew, |record| {
            record.0.as_slice()
        });
        assert_eq!(
            added.iter().map(|record| record.1).collect::<Vec<_>>(),
            vec![1, 2, 4]
        );
    }

    #[test]
    fn slot_method_membership_is_release_specific_and_exact() {
        use NativeOoVariableSlotOperation as Operation;
        let old = NativeNameProtocol::C(TclVersion::V8_6);
        let new = NativeNameProtocol::C(TclVersion::V9_0);
        assert_eq!(
            native_oo_variable_slot(old, None)
                .unwrap()
                .unwrap()
                .operation,
            Operation::Append
        );
        assert!(
            native_oo_variable_slot(old, Some(b"-remove"))
                .unwrap()
                .is_err()
        );
        for (name, expected) in [
            (b"-remove".as_slice(), Operation::Remove),
            (b"-prepend", Operation::Prepend),
            (b"-appendifnew", Operation::AppendIfNew),
        ] {
            assert_eq!(
                native_oo_variable_slot(new, Some(name))
                    .unwrap()
                    .unwrap()
                    .operation,
                expected
            );
        }
        assert!(native_oo_variable_slot(new, Some(b"-se")).unwrap().is_err());
        assert!(
            native_oo_variable_slot(new, Some(b"-set\0z"))
                .unwrap()
                .is_err()
        );
        assert!(native_oo_variable_slot(NativeNameProtocol::C(TclVersion::V8_4), None).is_err());
    }
}
