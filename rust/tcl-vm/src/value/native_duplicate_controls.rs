// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original checked Dictionary duplication observes sequential target ownership.
use super::*;
use crate::interp::native_numeric_ops::OriginalNumericOps;
use tcl_cmd_core::CmdError;
fn dictionary_member_reference_counts(source: &Value) -> Vec<(usize, usize)> {
    let primary = source.0.intrep.borrow();
    let IntRep::Dict(dict) = &*primary else {
        unreachable!()
    };
    dict.with_pairs(|pairs| {
        pairs
            .iter()
            .map(|(key, value)| {
                (
                    key.native_object_reference_count(),
                    value.native_object_reference_count(),
                )
            })
            .collect()
    })
}

#[test]
fn checked_dictionary_duplicate_holds_members_after_their_key_and_rejects_same_backing_mutation() {
    // naming.embedding.original-host-publication-and-fact-transport

    // Genuine software node-aware String callbacks inspect actual references
    // and mutate the same Dictionary backing. This is not native C/Jim callback
    // chronology or a private native reference-count observation.
    for mutate in [false, true] {
        let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("tcl9.1"));
        let protocol = NativeStringProtocol::C(tcl_dialect::TclVersion::V9_1);
        let source = Value::dict(vec![
            (
                Value::new_native_string_bytes(b"a".as_slice()),
                Value::new_native_string_bytes(b"A".as_slice()),
            ),
            (
                Value::new_native_string_bytes(b"b".as_slice()),
                Value::new_native_string_bytes(b"B".as_slice()),
            ),
        ]);
        source.seal_compound_string_protocol(protocol).unwrap();
        let original_backing = {
            let primary = source.0.intrep.borrow();
            let IntRep::Dict(dict) = &*primary else {
                unreachable!()
            };
            Rc::downgrade(dict)
        };
        let calls = Cell::new(0usize);
        let numeric = OriginalNumericOps::entered(&mut vm).unwrap();
        let reached = source.duplicate_native_object_checked(
            protocol,
            &mut || numeric.current_for_value(&source),
            &mut |key| {
                let call = calls.get();
                calls.set(call + 1);
                let counts = dictionary_member_reference_counts(&source);
                assert_eq!(
                    counts,
                    if call == 0 {
                        vec![(1, 1), (1, 1)]
                    } else {
                        vec![(2, 2), (1, 1)]
                    }
                );
                if mutate {
                    source.set_native_dictionary_member_in_place(
                        key.native_lifetime_lease().into_value(),
                        Value::new_native_string_bytes(b"changed".as_slice()),
                        protocol,
                    )?;
                }
                numeric.string(key, protocol)
            },
            &mut |first: &CmdError| numeric.retain_failure(first),
        );
        if mutate {
            let first = reached.unwrap_err();
            assert_eq!(
                first,
                CmdError::from(ValueError::CommandProtocolUnavailable(
                    "changed original duplicate Dictionary primary",
                ))
            );
            assert_eq!(calls.get(), 1);
            let primary = source.0.intrep.borrow();
            let IntRep::Dict(dict) = &*primary else {
                unreachable!()
            };
            assert!(std::rc::Weak::ptr_eq(
                &Rc::downgrade(dict),
                &original_backing
            ));
            assert_eq!(dict.epoch.get(), 2);
            dict.with_pairs(|pairs| {
                assert_eq!(
                    pairs[0].1.resident_string_bytes().unwrap().as_ref(),
                    b"changed"
                );
                assert_eq!(pairs[0].0.native_object_reference_count(), 1);
                assert_eq!(pairs[1].0.native_object_reference_count(), 1);
                assert_eq!(pairs[1].1.native_object_reference_count(), 1);
            });
            assert_eq!(numeric.current().unwrap_err(), first);
        } else {
            let duplicate = reached.unwrap();
            assert_eq!(calls.get(), 2);
            assert!(!duplicate.is_same_object(&source));
            let counts = || dictionary_member_reference_counts(&source);
            assert_eq!(counts(), vec![(2, 2), (2, 2)]);
            drop(duplicate);
            assert_eq!(counts(), vec![(1, 1), (1, 1)]);
        }
    }
}
