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

#[test]
fn checked_original_dictionary_search_keeps_epoch_and_borrowed_pair_roles() {
    let protocol = NativeStringProtocol::C(tcl_dialect::TclVersion::V8_6);
    let key = Value::new_native_string_bytes(b"k".as_slice());
    let key_lease = key.native_lifetime_lease();
    let member = Value::new_native_string_bytes(b"v".as_slice());
    let member_lease = member.native_lifetime_lease();
    let root = Value::from_native_dictionary_cache(vec![(key, member)]);
    let mut checks = 0;
    let mut search = root
        .native_lifetime_lease()
        .into_value()
        .into_native_dictionary_search_with_strings(
            protocol,
            &mut || {
                checks += 1;
                Ok(())
            },
            &mut |_| panic!("cached Dictionary needs no String getter"),
        )
        .unwrap();
    assert_eq!(checks, 2);
    assert_eq!(key_lease.value().native_object_reference_count(), 1);
    let pair = search
        .next_original_pair_checked(&mut || Ok(()))
        .unwrap()
        .unwrap();
    assert!(pair.0.is_same_object(key_lease.value()));
    assert!(pair.1.is_same_object(member_lease.value()));
    assert_eq!(key_lease.value().native_object_reference_count(), 1);
    assert_eq!(member_lease.value().native_object_reference_count(), 1);
    drop(pair);
    let before = search.next;
    if let IntRep::Dict(backing) = &*root.0.intrep.borrow() {
        backing.epoch.set(backing.epoch.get() + 1);
    } else {
        panic!("original Dictionary backing");
    }
    let error = search
        .next_original_pair_checked(&mut || Ok(()))
        .unwrap_err();
    assert_eq!(
        error.native_access_refusal(),
        Some(
            tcl_syntax::raw_string::NativeValueAccessRefusal::FatalCondition(
                tcl_syntax::raw_string::NativeFatalCondition::DictionarySearchConcurrentMutation,
            ),
        )
    );
    assert_eq!(search.next, before);
    assert!(search.backing.is_some());
}

#[test]
fn checked_original_dictionary_search_checks_exhaustion_after_backing_before_root_release() {
    let protocol = NativeStringProtocol::C(tcl_dialect::TclVersion::V8_6);
    let key = Value::new_native_string_bytes(b"k".as_slice());
    let key_lease = key.native_lifetime_lease();
    let member = Value::new_native_string_bytes(b"v".as_slice());
    let member_lease = member.native_lifetime_lease();
    let root = Value::from_native_dictionary_cache(vec![(key, member)]);
    let mut search = root
        .native_lifetime_lease()
        .into_value()
        .into_native_dictionary_search_with_strings(protocol, &mut || Ok(()), &mut |_| {
            panic!("cached Dictionary needs no String getter")
        })
        .unwrap();
    drop(
        search
            .next_original_pair_checked(&mut || Ok(()))
            .unwrap()
            .unwrap(),
    );
    root.replace_primary(IntRep::Str);
    assert_eq!(key_lease.value().native_object_reference_count(), 1);
    let mut observed = Vec::new();
    assert!(
        search
            .next_original_pair_checked(&mut || {
                observed.push((
                    key_lease.value().native_object_reference_count(),
                    member_lease.value().native_object_reference_count(),
                    root.native_object_reference_count(),
                ));
                Ok(())
            })
            .unwrap()
            .is_none()
    );
    assert_eq!(observed, [(1, 1, 1), (0, 0, 1)]);
    assert!(search.backing.is_none());
    assert!(search.original_root().is_same_object(&root));
    assert!(root.native_object_is_live());
}

#[test]
fn native_dictionary_member_publication_owns_borrowed_live_children() {
    // A software ownership control for the member-publication boundary.
    // Whole public writeback observations are checked independently by
    // naming.dictionary.update-with-body-and-observer-frontiers:
    // docs/design/analysis/name-resolution-proofs/dictionary-update-with-body-and-observer-frontiers.md.
    for protocol in [
        NativeStringProtocol::C(tcl_dialect::TclVersion::V8_5),
        NativeStringProtocol::C(tcl_dialect::TclVersion::V8_6),
        NativeStringProtocol::C(tcl_dialect::TclVersion::V9_0),
        NativeStringProtocol::C(tcl_dialect::TclVersion::V9_1),
        NativeStringProtocol::Jim084,
    ] {
        let root = Value::dict(Vec::new());
        let mut dictionary = root.prepare_native_dictionary(protocol).unwrap();
        drop(root);
        let key = Value::new_native_string_bytes(b"key\0tail".as_slice());
        let key_lease = key.native_lifetime_lease();
        let first = Value::int(37);
        let first_lease = first.native_lifetime_lease();
        dictionary
            .set_member(
                key.native_lifetime_lease().into_value(),
                first.native_lifetime_lease().into_value(),
            )
            .unwrap();
        assert_eq!(key.native_object_reference_count(), 2);
        assert_eq!(first.native_object_reference_count(), 2);
        drop((key, first));
        assert_eq!(key_lease.value().native_object_reference_count(), 1);
        assert_eq!(first_lease.value().native_object_reference_count(), 1);
        assert!(first_lease.value().resident_string_bytes().is_none());

        let replacement_key = Value::new_native_string_bytes(b"key\0tail".as_slice());
        let replacement_key_lease = replacement_key.native_lifetime_lease();
        let replacement = Value::int(41);
        let replacement_lease = replacement.native_lifetime_lease();
        dictionary
            .set_member(
                replacement_key.native_lifetime_lease().into_value(),
                replacement.native_lifetime_lease().into_value(),
            )
            .unwrap();
        assert!(!first_lease.value().native_object_is_live());
        assert_eq!(replacement_key.native_object_reference_count(), 1);
        assert_eq!(replacement.native_object_reference_count(), 2);
        drop((replacement_key, replacement));
        assert!(!replacement_key_lease.value().native_object_is_live());
        assert_eq!(replacement_lease.value().native_object_reference_count(), 1);
        dictionary
            .original()
            .with_cached_dictionary_representation(|pairs, _| {
                assert_eq!(pairs.len(), 1);
                assert!(pairs[0].0.is_same_object(key_lease.value()));
                assert!(pairs[0].1.is_same_object(replacement_lease.value()));
            })
            .unwrap();
        assert_eq!(
            replacement_lease
                .value()
                .native_string_bytes(protocol)
                .unwrap()
                .as_ref(),
            b"41"
        );
        drop(dictionary);
        assert!(!key_lease.value().native_object_is_live());
        assert!(!replacement_lease.value().native_object_is_live());
        assert_eq!(replacement_lease.value().native_object_reference_count(), 0);
    }
}

#[test]
fn native_dictionary_member_publication_refuses_retired_lifetime_handles() {
    let protocol = NativeStringProtocol::C(tcl_dialect::TclVersion::V8_6);
    let root = Value::dict(Vec::new());
    let mut dictionary = root.prepare_native_dictionary(protocol).unwrap();
    drop(root);
    let child = Value::int(37);
    let child_lease = child.native_lifetime_lease();
    drop(child);
    assert!(!child_lease.value().native_object_is_live());
    let result = dictionary.set_member(
        Value::new_native_string_bytes(b"key".as_slice()),
        child_lease.value().native_lifetime_lease().into_value(),
    );
    assert!(matches!(
        result,
        Err(ValueError::CommandProtocolUnavailable(
            "retired native object header"
        ))
    ));
    assert_eq!(child_lease.value().native_object_reference_count(), 0);
    assert!(!child_lease.value().native_object_is_live());
    dictionary
        .original()
        .with_cached_dictionary_representation(|pairs, _| assert!(pairs.is_empty()))
        .unwrap();
}

#[test]
fn native_dictionary_getter_checks_foreign_compound_recipe_before_cached_members() {
    // naming.invocation.original-object-materialization-purpose
    // docs/design/analysis/name-resolution-proofs/invocation-original-object-materialization-purpose.md
    // Software carrier/cache contract: no Native worker or conversion admission.
    let original = NativeStringProtocol::C(tcl_dialect::TclVersion::V9_0);
    let foreign = NativeStringProtocol::C(tcl_dialect::TclVersion::V8_6);
    for dictionary in [false, true] {
        for resident in [false, true] {
            let key = Value::new_native_string_bytes(b"key".as_slice());
            let member = Value::int(17);
            let value = if dictionary {
                Value::native_dictionary_constructor(
                    vec![(key.clone(), member.clone())],
                    None,
                    original,
                )
                .unwrap()
            } else {
                Value::native_list_constructor(vec![key.clone(), member.clone()], original)
            };
            let value = if resident {
                value.with_resident_string_bytes(Rc::from(b"key 17".as_slice()))
            } else {
                value
            };
            let primary = value.native_object_type_name();
            let member_references = member.native_object_reference_count();
            assert!(matches!(
                value.native_object_dict_pairs(foreign),
                Err(ValueError::NativeStringAccess(
                    tcl_syntax::raw_string::NativeStringAccessError::Unavailable(
                        tcl_syntax::native_string::NativeStringUnavailable::ProtocolUnavailable,
                    ),
                )),
            ));
            assert_eq!(value.native_object_type_name(), primary);
            assert_eq!(value.resident_string_bytes().is_some(), resident);
            assert_eq!(member.native_object_reference_count(), member_references);
            assert!(member.resident_string_bytes().is_none());
            let pairs = value.native_object_dict_pairs(original).unwrap();
            assert!(pairs[0].0.is_same_object(&key));
            assert!(pairs[0].1.is_same_object(&member));
            assert!(member.resident_string_bytes().is_none());
        }
    }
}

#[test]
fn native_dictionary_constructor_retains_first_original_key_and_last_value() {
    let protocol = NativeStringProtocol::C(tcl_dialect::TclVersion::V9_0);
    let first_key = Value::new_native_string_bytes(b"k\0\xff".as_slice());
    let last_key = Value::new_native_string_bytes(b"k\0\xff".as_slice());
    let first_value = Value::int(1);
    let last_value = Value::int(2);
    let dictionary = Value::native_dictionary_constructor(
        vec![
            (first_key.clone(), first_value),
            (last_key.clone(), last_value.clone()),
        ],
        None,
        protocol,
    )
    .unwrap();
    dictionary
        .with_cached_dictionary_representation(|pairs, _| {
            assert_eq!(pairs.len(), 1);
            assert!(pairs[0].0.is_same_object(&first_key));
            assert!(!pairs[0].0.is_same_object(&last_key));
            assert!(pairs[0].1.is_same_object(&last_value));
            assert!(pairs[0].1.resident_string_bytes().is_none());
        })
        .unwrap();
    assert!(dictionary.resident_string_bytes().is_none());
}

#[test]
fn selected_double_updaters_match_all_native_nonfinite_payloads() {
    let fixtures = [
        include_str!("../../../tcl-syntax/testdata/native_nonfinite_double/8.4.20.tsv"),
        include_str!("../../../tcl-syntax/testdata/native_nonfinite_double/8.5.19.tsv"),
        include_str!("../../../tcl-syntax/testdata/native_nonfinite_double/8.6.18.tsv"),
        include_str!("../../../tcl-syntax/testdata/native_nonfinite_double/9.0.4.tsv"),
        include_str!("../../../tcl-syntax/testdata/native_nonfinite_double/9.1.0.tsv"),
        include_str!("../../../tcl-syntax/testdata/native_nonfinite_double/jim.tsv"),
    ];
    let protocols = [
        NativeStringProtocol::C(tcl_dialect::TclVersion::V8_4),
        NativeStringProtocol::C(tcl_dialect::TclVersion::V8_5),
        NativeStringProtocol::C(tcl_dialect::TclVersion::V8_6),
        NativeStringProtocol::C(tcl_dialect::TclVersion::V9_0),
        NativeStringProtocol::C(tcl_dialect::TclVersion::V9_1),
        NativeStringProtocol::Jim084,
    ];
    let bits = [
        0x7ff8_0000_0000_0000,
        0xfff8_0000_0000_0000,
        0x7ff0_0000_0000_0000,
        0xfff0_0000_0000_0000,
        0,
        0x8000_0000_0000_0000,
    ];
    let mut observations = 0;
    for (protocol, fixture) in protocols.into_iter().zip(fixtures) {
        for row in fixture.lines() {
            let (case, expected) = row.split_once('\t').unwrap();
            let case: usize = case.parse().unwrap();
            let value = f64::from_bits(bits[case]);
            let dialect = protocol.tcl_version().map_or_else(
                || {
                    tcl_registry::InvocationDialect::of_profile(
                        tcl_registry::model::ingress::resolve_environment("jim").unit_profile(),
                    )
                },
                tcl_registry::InvocationDialect::for_version,
            );
            let object = Value::from_native_scalar_cache(
                NativeScalarCache::Number(Number::Double(value)),
                None,
                dialect,
            )
            .unwrap();
            assert!(object.resident_string_bytes().is_none());
            let bytes = object.native_string_bytes(protocol).unwrap();
            assert!(
                matches!(object.number_representation(), Some(Number::Double(actual)) if actual.to_bits() == bits[case])
            );
            let hex: String = bytes.iter().fold(String::new(), |mut output, byte| {
                use std::fmt::Write as _;
                write!(output, "{byte:02x}").unwrap();
                output
            });
            assert_eq!(hex, expected, "{protocol:?}/{case}");
            observations += 1;
        }
    }
    assert_eq!(observations, 36);
}

#[test]
fn list_backing_canonical_flags_match_original_native_operations() {
    let fixtures = [
        include_str!("../../../tcl-syntax/testdata/native_list_canonical/8.4.20.tsv"),
        include_str!("../../../tcl-syntax/testdata/native_list_canonical/8.5.19.tsv"),
        include_str!("../../../tcl-syntax/testdata/native_list_canonical/8.6.18.tsv"),
        include_str!("../../../tcl-syntax/testdata/native_list_canonical/9.0.4.tsv"),
        include_str!("../../../tcl-syntax/testdata/native_list_canonical/9.1.0.tsv"),
    ];
    let mut operations = 0;
    for (version, fixture) in tcl_dialect::TclVersion::ALL.into_iter().zip(fixtures) {
        let protocol = NativeStringProtocol::C(version);
        let rows: Vec<_> = fixture.lines().collect();
        for pair in rows.as_chunks::<2>().0 {
            let before_fields: Vec<_> = pair[0].split('\t').collect();
            let after_fields: Vec<_> = pair[1].split('\t').collect();
            let mode: usize = before_fields[0].parse().unwrap();
            let root = Value::native_list_constructor(
                vec![
                    Value::new_native_string_bytes(b"a".as_slice()),
                    Value::new_native_string_bytes(b"b".as_slice()),
                ],
                protocol,
            );
            let root = if mode == 4 {
                let parsed = Value::new_native_string_bytes(b"a   b".as_slice());
                drop(parsed.native_object_list_elements(protocol).unwrap());
                parsed
            } else {
                root
            };
            if (2..5).contains(&mode) {
                root.native_string_bytes(protocol).unwrap();
            }
            let duplicate =
                (mode == 3 || mode == 5).then(|| root.duplicate_native_object_in(protocol));
            let selected = duplicate.as_ref().unwrap_or(&root);
            let before = selected.cached_list_representation().unwrap().1;
            assert_eq!(
                selected.resident_string_bytes().is_some(),
                before_fields[3] == "1"
            );
            if version != tcl_dialect::TclVersion::V8_4 {
                assert_eq!(before, before_fields[4] == "1");
            }
            let result = if mode == 0 || mode == 5 {
                selected.native_string_bytes(protocol).unwrap();
                selected.clone()
            } else {
                Value::native_list_append_elements(
                    Some(selected),
                    &[Value::new_native_string_bytes(b"c".as_slice())],
                    protocol,
                )
                .unwrap()
            };
            assert_eq!(
                result.resident_string_bytes().is_some(),
                after_fields[3] == "1"
            );
            if version != tcl_dialect::TclVersion::V8_4 {
                assert_eq!(
                    result.cached_list_representation().unwrap().1,
                    after_fields[4] == "1"
                );
            }
            operations += 1;
        }
    }
    assert_eq!(operations, 30);
}

#[test]
fn return_options_presence_uses_owned_shape_without_materialising_children() {
    // naming.error.original-invocation-context-capture
    // docs/design/analysis/name-resolution-proofs/error-original-invocation-context-capture.md
    // Software shape query only; no native private object or execution proof.
    assert!(!Value::empty().native_return_options_nonempty(None).unwrap());
    assert!(
        Value::string(" ")
            .native_return_options_nonempty(None)
            .unwrap()
    );
    assert!(
        Value::list(vec![Value::string("-code"), Value::int(0)])
            .native_return_options_nonempty(None)
            .unwrap()
    );
    for version in [
        tcl_dialect::TclVersion::V8_6,
        tcl_dialect::TclVersion::V9_0,
        tcl_dialect::TclVersion::V9_1,
    ] {
        let protocol = NativeStringProtocol::C(version);
        let child = Value::native_list_constructor(vec![Value::string("original")], protocol);
        let options = Value::native_list_constructor(
            vec![Value::string("-errorstack"), child.clone()],
            protocol,
        );
        let identity = child.native_object_identity();
        let references = child.native_object_reference_count();
        assert!(
            options
                .native_return_options_nonempty(Some(protocol))
                .unwrap()
        );
        assert_eq!(child.native_object_identity(), identity);
        assert_eq!(child.native_object_reference_count(), references);
        assert!(child.resident_string_bytes().is_none());
        assert!(options.resident_string_bytes().is_none());
        assert!(
            !Value::native_list_constructor(Vec::new(), protocol)
                .native_return_options_nonempty(Some(protocol))
                .unwrap()
        );
        let dictionary = Value::dict(vec![(Value::string("-errorstack"), child.clone())]);
        dictionary.seal_compound_string_protocol(protocol).unwrap();
        assert!(
            dictionary
                .native_return_options_nonempty(Some(protocol))
                .unwrap()
        );
        assert!(dictionary.resident_string_bytes().is_none());
        assert!(
            !Value::empty()
                .native_return_options_nonempty(Some(protocol))
                .unwrap()
        );
        assert!(
            Value::string(" ")
                .native_return_options_nonempty(Some(protocol))
                .unwrap()
        );
    }
}

#[test]
fn return_options_presence_preserves_retired_foreign_and_borrowed_refusals() {
    // naming.error.original-invocation-context-capture
    // docs/design/analysis/name-resolution-proofs/error-original-invocation-context-capture.md
    // An emptiness query cannot revive a header or borrow argv ownership.
    let protocol = NativeStringProtocol::C(tcl_dialect::TclVersion::V8_6);
    let original =
        Value::native_list_constructor(vec![Value::string("-code"), Value::int(0)], protocol);
    assert!(
        original
            .native_return_options_nonempty(Some(NativeStringProtocol::Jim084))
            .is_err()
    );
    let borrowed = Value::invocation_list_view(&crate::NativeListItems::new(
        vec![Value::string("-code"), Value::int(0)],
        false,
    ));
    assert!(
        borrowed
            .native_return_options_nonempty(Some(protocol))
            .is_err()
    );
    assert!(original.native_return_options_nonempty(None).is_err());
    let lifetime = original.native_lifetime_lease();
    drop(original);
    assert!(!lifetime.value().native_object_is_live());
    assert!(
        lifetime
            .value()
            .native_return_options_nonempty(Some(protocol))
            .is_err()
    );
    assert!(
        Value::int(0)
            .native_return_options_nonempty(Some(protocol))
            .is_err()
    );
}

#[test]
fn selected_empty_compound_updaters_install_native_allocation_identity() {
    for protocol in [
        NativeStringProtocol::C(tcl_dialect::TclVersion::V8_4),
        NativeStringProtocol::C(tcl_dialect::TclVersion::V8_5),
        NativeStringProtocol::C(tcl_dialect::TclVersion::V8_6),
        NativeStringProtocol::C(tcl_dialect::TclVersion::V9_0),
        NativeStringProtocol::C(tcl_dialect::TclVersion::V9_1),
        NativeStringProtocol::Jim084,
    ] {
        let list = Value::list_with_native_canonical_in(Vec::new(), false, protocol);
        assert!(list.native_string_bytes(protocol).unwrap().is_empty());
        assert_eq!(
            list.resident_string_storage_identity(),
            Some(protocol.compound_updater_storage())
        );
        assert_eq!(
            list.cached_list_representation().unwrap().1,
            protocol.updated_list_canonical(false, false)
        );
        if protocol == NativeStringProtocol::C(tcl_dialect::TclVersion::V8_4) {
            continue;
        }
        let dict = Value::dict(Vec::new())
            .with_native_compound_string_protocol(protocol)
            .unwrap();
        assert!(dict.native_string_bytes(protocol).unwrap().is_empty());
        assert_eq!(
            dict.resident_string_storage_identity(),
            Some(protocol.compound_updater_storage())
        );
    }
}

fn compound_updater_original(
    case: u8,
    protocol: NativeStringProtocol,
    source_context: Option<&Rc<NativeJimObjectContext>>,
) -> Value {
    let string = |bytes: &[u8]| Value::new_native_string_bytes(bytes);
    let list = |items| Value::native_list_constructor(items, protocol);
    let dict = || {
        Value::dict(vec![(string(b"#key"), string(b"a\"b"))])
            .with_native_compound_string_protocol(protocol)
            .unwrap()
    };
    match case {
        0 => list(vec![string(b"#first"), string(b"#later")]),
        1 => list(vec![string(b"a\"b"), string(b"]")]),
        2 => list(vec![string(b"A\0\xff"), string(b"\\\n")]),
        3 => list(vec![
            list(vec![Value::int(17), string(b"a\"b")]),
            string(b"#later"),
        ]),
        4 => dict(),
        5 => list(vec![dict(), string(b"#later")]),
        6 => {
            let value = string(b"ORIGINAL");
            if let Some(context) = source_context {
                value.bind_native_jim_context(context).unwrap();
            }
            drop(value.native_object_list_elements(protocol).unwrap());
            value
        }
        _ => unreachable!(),
    }
}

#[test]
fn compound_updater_matches_all_six_native_original_object_fixtures() {
    use super::*;
    use tcl_dialect::TclVersion;
    const FIXTURES: [&str; 6] = [
        include_str!("../../../tcl-syntax/testdata/native_compound_string_updaters/8.4.20.tsv"),
        include_str!("../../../tcl-syntax/testdata/native_compound_string_updaters/8.5.19.tsv"),
        include_str!("../../../tcl-syntax/testdata/native_compound_string_updaters/8.6.18.tsv"),
        include_str!("../../../tcl-syntax/testdata/native_compound_string_updaters/9.0.4.tsv"),
        include_str!("../../../tcl-syntax/testdata/native_compound_string_updaters/9.1.0.tsv"),
        include_str!("../../../tcl-syntax/testdata/native_compound_string_updaters/jim.tsv"),
    ];
    let protocols = [
        NativeStringProtocol::C(TclVersion::V8_4),
        NativeStringProtocol::C(TclVersion::V8_5),
        NativeStringProtocol::C(TclVersion::V8_6),
        NativeStringProtocol::C(TclVersion::V9_0),
        NativeStringProtocol::C(TclVersion::V9_1),
        NativeStringProtocol::Jim084,
    ];
    let mut rows = 0;
    let mut unavailable = 0;
    for (protocol, fixture) in protocols.into_iter().zip(FIXTURES) {
        let source_context = protocol.is_jim084().then(|| {
            NativeJimObjectContext::new(tcl_registry::InvocationDialect::of_profile(
                tcl_registry::model::ingress::resolve_environment("jim").unit_profile(),
            ))
            .unwrap()
        });
        for row in fixture.lines() {
            let fields: Vec<_> = row.split('\t').collect();
            rows += 1;
            if fields[1] == "unavailable" {
                assert_eq!(protocol, protocols[0]);
                unavailable += 1;
                continue;
            }
            let case: u8 = fields[0].parse().unwrap();
            let root = compound_updater_original(case, protocol, source_context.as_ref());
            let state = || {
                let primary = root.0.intrep.borrow();
                let describe = |kind, child: &Value| {
                    let child_kind = match &*child.0.intrep.borrow() {
                        IntRep::List { .. } => "list",
                        IntRep::Dict(_) => "dict",
                        IntRep::Str => "none",
                        IntRep::JimSource(_) => "source",
                        IntRep::JimScript(_) => "script",
                        IntRep::JimScriptLine { .. } => "scriptline",
                        _ => "other",
                    };
                    (
                        kind,
                        root.resident_string_bytes().is_some(),
                        child_kind,
                        child.resident_string_bytes().is_some(),
                        child.native_object_reference_count(),
                    )
                };
                match &*primary {
                    IntRep::List { items, .. } => describe("list", &items[0]),
                    IntRep::Dict(dict) => dict.with_pairs(|pairs| describe("dict", &pairs[0].1)),
                    _ => unreachable!(),
                }
            };
            let before = state();
            assert_eq!(before.0, fields[1]);
            assert_eq!(before.1, fields[2] == "1");
            assert_eq!(before.2, fields[3]);
            assert_eq!(before.3, fields[4] == "1");
            assert_eq!(before.4, fields[10].parse::<usize>().unwrap());
            let bytes = root.native_string_bytes(protocol).unwrap();
            let hex = bytes.iter().fold(String::new(), |mut output, byte| {
                use std::fmt::Write as _;
                write!(output, "{byte:02x}").unwrap();
                output
            });
            assert_eq!(hex, fields[5], "{protocol:?}: {row}");
            let after = state();
            assert_eq!(after.0, fields[6]);
            assert_eq!(after.1, fields[7] == "1");
            assert_eq!(after.2, fields[8]);
            assert_eq!(after.2, before.2);
            assert_eq!(after.3, fields[9] == "1");
            assert_eq!(after.4, fields[11].parse::<usize>().unwrap());
        }
    }
    assert_eq!(rows, 42);
    assert_eq!(unavailable, 2);
}

use super::*;

#[test]
fn native_dictionary_list_conversion_preserves_original_members_without_string() {
    use tcl_syntax::native_string::NativeStringProtocol;
    for protocol in [
        NativeStringProtocol::C(tcl_dialect::TclVersion::V8_5),
        NativeStringProtocol::C(tcl_dialect::TclVersion::V8_6),
        NativeStringProtocol::C(tcl_dialect::TclVersion::V9_0),
        NativeStringProtocol::C(tcl_dialect::TclVersion::V9_1),
        NativeStringProtocol::Jim084,
    ] {
        let key = Value::from_string_bytes(&b"k\0\xff"[..]);
        let child = Value::int(7);
        let parent = Value::from_native_dictionary_cache(vec![(key.clone(), child.clone())]);
        let alias = parent.clone();
        let members = parent.native_object_list_elements(protocol).unwrap();
        assert!(members[0].is_same_object(&key));
        assert!(members[1].is_same_object(&child));
        assert!(alias.resident_string_bytes().is_none());
        assert!(child.resident_string_bytes().is_none());
        assert!(!alias.cached_list_representation().unwrap().1);

        let resident = Value::from_native_dictionary_cache(vec![(key.clone(), child.clone())])
            .with_resident_string_bytes(Rc::from(&b"k FIRST k LAST"[..]));
        let source_context = protocol.is_jim084().then(|| {
            NativeJimObjectContext::new(tcl_registry::InvocationDialect::of_profile(
                tcl_registry::model::ingress::resolve_environment("jim").unit_profile(),
            ))
            .unwrap()
        });
        if let Some(context) = &source_context {
            resident.bind_native_jim_context(context).unwrap();
        }
        let members = resident.native_object_list_elements(protocol).unwrap();
        assert_eq!(members.len(), 4);
        assert_eq!(
            members[1].resident_string_bytes().unwrap().as_ref(),
            b"FIRST"
        );
        assert_eq!(
            members[3].resident_string_bytes().unwrap().as_ref(),
            b"LAST"
        );
        assert!(!members[1].is_same_object(&child));
    }
}

#[test]
fn native_name_materialisation_retains_binary_backing_and_resident_recipe() {
    use tcl_syntax::native_string::{NativeStringProtocol, NativeStringUnavailable};
    let binary = Value::byte_array(b"k\0\xff".as_slice());
    assert!(binary.resident_string_bytes().is_none());
    assert_eq!(
        binary
            .native_string_bytes(NativeStringProtocol::C(tcl_dialect::TclVersion::V8_4))
            .unwrap()
            .as_ref(),
        b"k\xc0\x80\xc3\xbf"
    );
    assert_eq!(
        binary.byte_array_representation().unwrap().as_ref(),
        b"k\0\xff"
    );
    assert_eq!(
        binary
            .native_string_bytes(NativeStringProtocol::Jim084)
            .unwrap()
            .as_ref(),
        b"k\xc0\x80\xc3\xbf"
    );
    let resident = Value::byte_array_with_resident_string(
        b"PAYLOAD".as_slice(),
        Some(Rc::from(b"k\0\xff".as_slice())),
    );
    assert_eq!(
        resident
            .native_string_bytes(NativeStringProtocol::Jim084)
            .unwrap()
            .as_ref(),
        b"k\0\xff"
    );
    let jim_binary = Value::byte_array(b"k\0\xff".as_slice());
    assert_eq!(
        jim_binary.native_string_bytes(NativeStringProtocol::Jim084),
        Err(NativeStringUnavailable::JimByteArray)
    );
    assert!(jim_binary.resident_string_bytes().is_none());
    assert_eq!(
        jim_binary.byte_array_representation().unwrap().as_ref(),
        b"k\0\xff"
    );
}

#[test]
fn jim_double_getter_retains_exact_integer_roundtrip_and_native_string_cache() {
    let jim = tcl_registry::InvocationDialect::of_profile(
        tcl_registry::model::ingress::resolve_environment("jim").unit_profile(),
    );
    let context = NativeJimObjectContext::new(jim).unwrap();
    context.select_numeric_host(Rc::new(tcl_host_native::NativeHost::new()));
    for integer in [2, i64::MAX] {
        let value = Value::int(integer);
        value.bind_native_jim_context(&context).unwrap();
        let alias = value.clone();
        assert!(
            value
                .native_scalar_getter(jim, NativeScalarGetterKind::Double)
                .is_ok()
        );
        assert!(
            matches!(*alias.0.intrep.borrow(), IntRep::CoercedDouble(retained) if retained == integer)
        );
        assert_eq!(alias.resident_string_bytes().is_some(), integer == i64::MAX);
        assert_eq!(
            alias.native_scalar_getter(jim, NativeScalarGetterKind::Wide),
            Ok(NativeScalarGetterValue::Wide(integer))
        );
        assert_eq!(alias.integer_representation(), Some(integer));
    }
    let c = Value::int(2);
    let dialect = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
    assert_eq!(
        c.native_scalar_getter(dialect, NativeScalarGetterKind::Double),
        Ok(NativeScalarGetterValue::Double(2.0))
    );
    assert_eq!(c.integer_representation(), Some(2));
    assert!(c.resident_string_bytes().is_none());
}

#[test]
fn native_numeric_prefix_conversion_preserves_the_original_object_and_bytes() {
    use number::NativeScalarNumericInputPolicy;
    let syntax = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
    let syntax = tcl_registry::InvocationDialect::of_profile(syntax).numbers;
    let value = Value::native_jim_string(&[b'1', 0, 0xff][..], 3);
    let alias = value.clone();
    assert_eq!(
        value.native_int(NativeScalarNumericInputPolicy::NulTerminatedJim084, syntax),
        Ok(1)
    );
    assert!(value.is_same_object(&alias));
    assert_eq!(alias.integer_representation(), Some(1));
    assert_eq!(alias.string_bytes().as_ref(), &[b'1', 0, 0xff]);
    let c = Value::from_string_bytes(&[b'1', 0, 0xff][..]);
    assert_eq!(
        c.native_int(NativeScalarNumericInputPolicy::LengthDelimited, syntax),
        Err(tcl_syntax::value::ValueError::NotIntegerBytes(vec![
            b'1', 0, 0xff
        ]))
    );
    assert_eq!(c.integer_representation(), None);
    let word = Value::from_string_bytes(&b"true\0suffix"[..]);
    assert_eq!(
        word.native_bool(NativeScalarNumericInputPolicy::NulTerminatedJim084, syntax),
        Ok(true)
    );
    assert_eq!(word.integer_representation(), Some(1));
    assert_eq!(word.string_bytes().as_ref(), b"true\0suffix");
}

#[test]
fn diagnostic_argv_observes_ownership_until_actual_error_capture() {
    let bytes = &[0xc3, b'A', 0xc3, 0xa9, b' ', b' '][..];
    let trim_set = RawString::from_bytes(&b" \t\n\r\0"[..]);
    let plan = RawString::from_bytes(bytes).jim084_trim_plan(&trim_set, false, true);
    let model = tcl_dialect::StringCharacterModel::Jim084Utf8;
    let argv = Rc::new(vec![Value::native_jim_string(bytes, 4)]);
    let diagnostic = Value::shared_list(Rc::clone(&argv));
    let retained_frame = diagnostic.clone();
    let result = argv[0].jim_string_trim_result(plan);
    assert!(result.is_same_object(&argv[0]));
    assert_eq!(result.string_bytes().as_ref(), &bytes[..4]);
    assert_eq!(result.native_character_count(model, None).unwrap(), 4);
    drop((result, retained_frame, diagnostic, argv));

    let argv = Rc::new(vec![Value::native_jim_string(bytes, 4)]);
    let diagnostic = Value::shared_list(Rc::clone(&argv));
    let captured = diagnostic.capture_invocation_list();
    let result = argv[0].jim_string_trim_result(plan);
    assert!(!result.is_same_object(&argv[0]));
    assert_eq!(result.string_bytes().as_ref(), &bytes[..4]);
    assert_eq!(result.native_character_count(model, None).unwrap(), 3);
    assert_eq!(argv[0].string_bytes().as_ref(), bytes);
    assert!(captured.as_list().unwrap()[0].is_same_object(&argv[0]));
}

#[test]
fn byte_array_carrier_retains_original_storage_and_string_presence() {
    let payload: Rc<[u8]> = Rc::from(&[0xff, 0][..]);
    let pure = Value::byte_array(Rc::clone(&payload));
    assert!(pure.resident_string_bytes().is_none());
    assert_eq!(pure.byte_array_representation().unwrap(), payload);
    assert!(pure.is_pure_byte_array());

    let resident: Rc<[u8]> = Rc::from(&[0xc3, 0xbf, 0, 0xff][..]);
    let cached =
        Value::byte_array_with_resident_string(Rc::clone(&payload), Some(Rc::clone(&resident)));
    assert_eq!(cached.byte_array_representation().unwrap(), payload);
    assert_eq!(cached.resident_string_bytes().unwrap(), resident);
    assert!(!cached.is_pure_byte_array());
    assert!(cached.try_to_str().is_err());
    assert_eq!(cached.string_bytes(), resident);
    assert_eq!(cached.byte_array_representation().unwrap(), payload);
}

#[test]
fn raw_string_projection_and_list_rendering_preserve_original_bytes() {
    let value = Value::from_raw_parts(
        Some(RawString::from_bytes(&[0xff, b' ', 0xed, 0xa0, 0x80][..])),
        IntRep::Str,
    );
    let alias = value.clone();
    assert!(value.try_to_str().is_err());
    assert_eq!(
        value.string_bytes().as_ref(),
        &[0xff, b' ', 0xed, 0xa0, 0x80]
    );
    let list = Value::list(vec![value.clone(), Value::string("tail")]);
    assert_eq!(
        list.string_bytes().as_ref(),
        &[
            b'{', 0xff, b' ', 0xed, 0xa0, 0x80, b'}', b' ', b't', b'a', b'i', b'l'
        ]
    );
    assert!(list.try_to_str().is_err());
    assert_eq!(
        list.as_list().unwrap()[0].string_bytes(),
        alias.string_bytes()
    );
    assert!(alias.existing_string_representation().is_none());
    assert!(format!("{value:?}").contains("bytes="));
}

#[test]
fn byte_array_payload_survives_string_generation_and_shared_decoder_selection() {
    use tcl_registry::native_binary_value::{NativeBinaryDecodeInput, NativeBinaryDecodeSource};
    let value = Value::byte_array(&[0xe9, 0][..]);
    let alias = value.clone();
    assert_eq!(
        alias
            .binary_decode_input(NativeBinaryDecodeSource::PureByteArrayOrString)
            .unwrap(),
        NativeBinaryDecodeInput::Bytes(vec![0xe9, 0])
    );
    assert_eq!(&*value.to_str(), "é\0");
    assert!(!alias.is_pure_byte_array());
    assert_eq!(
        alias
            .binary_decode_input(NativeBinaryDecodeSource::PureByteArrayOrString)
            .unwrap(),
        NativeBinaryDecodeInput::String("é\0".to_owned())
    );
    assert_eq!(
        alias
            .binary_decode_input(NativeBinaryDecodeSource::ProperByteArrayOrString)
            .unwrap(),
        NativeBinaryDecodeInput::Bytes(vec![0xe9, 0])
    );
}

#[test]
fn character_length_obeys_actual_bytearray_conversion_and_purity() {
    use tcl_dialect::TclVersion;
    use tcl_registry::InvocationDialect;
    for version in TclVersion::ALL {
        let dialect = InvocationDialect::for_version(version);
        let value = Value::byte_array(&[0xe9, b'a'][..]);
        assert_eq!(
            value.native_character_count(
                dialect.characters.unwrap(),
                dialect.string_length_representation()
            ),
            Ok(2)
        );
        assert_eq!(
            value.byte_array_representation().is_some(),
            version >= TclVersion::V8_6
        );
        assert_eq!(value.is_pure_byte_array(), version >= TclVersion::V8_6);

        let value = Value::byte_array(&[0xe9, b'a'][..]);
        assert_eq!(&*value.to_str(), "éa");
        assert_eq!(
            value.native_character_count(
                dialect.characters.unwrap(),
                dialect.string_length_representation()
            ),
            Ok(2)
        );
        assert_eq!(
            value.byte_array_representation().is_some(),
            version >= TclVersion::V8_6
        );
    }
    // Modern C's short-string bypass retains a converted object too.
    let dialect = InvocationDialect::for_version(TclVersion::V9_0);
    let value = Value::byte_array(&b"a"[..]);
    assert_eq!(&*value.to_str(), "a");
    assert_eq!(
        value.native_character_count(
            dialect.characters.unwrap(),
            dialect.string_length_representation()
        ),
        Ok(1)
    );
    assert!(value.byte_array_representation().is_some());
}

#[test]
fn byte_conversion_caches_are_selected_and_failed_conversion_preserves_source() {
    use tcl_registry::native_binary_value::NativeBinaryByteConversion;
    let cached = Value::native_jim_string(&[0xc3, b'A', 0xc3, 0xa9], 2);
    assert_eq!(
        cached
            .as_byte_array(NativeBinaryByteConversion::Utf8)
            .unwrap()
            .as_ref(),
        &[0xc3, b'A', 0xc3, 0xa9]
    );
    assert_eq!(
        cached
            .native_character_count(tcl_dialect::StringCharacterModel::Jim084Utf8, None)
            .unwrap(),
        2
    );
    assert!(cached.byte_array_representation().is_none());
    let value = Value::string("é");
    assert_eq!(
        &*value
            .as_byte_array(NativeBinaryByteConversion::Utf8)
            .unwrap(),
        &[0xc3, 0xa9]
    );
    assert_eq!(
        &*value
            .as_byte_array(NativeBinaryByteConversion::CheckedLatin1)
            .unwrap(),
        &[0xe9]
    );
    let wide = Value::list(vec![Value::string("€")]);
    assert_eq!(
        wide.as_byte_array(NativeBinaryByteConversion::CheckedLatin1)
            .unwrap_err(),
        ByteArrayAccessError::Conversion(
            tcl_registry::native_binary_value::NativeBinaryByteError {
                offset: 0,
                codepoint: 0x20ac
            }
        )
    );
    assert_eq!(&*wide.to_str(), "€");
    assert!(matches!(&*wide.0.intrep.borrow(), IntRep::List { .. }));
    assert_eq!(wide.as_list().unwrap().len(), 1);
}

#[test]
fn int_shimmers_to_string() {
    assert_eq!(&*Value::int(42).to_str(), "42");
    assert_eq!(&*Value::double(2.0).to_str(), "2.0");
    assert_eq!(&*Value::bool(true).to_str(), "1");
    assert_eq!(&*Value::empty().to_str(), "");
}

#[test]
fn string_shimmers_to_int() {
    assert_eq!(Value::string("42").as_int().unwrap(), 42);
    assert_eq!(Value::string("  -7 ").as_int().unwrap(), -7);
    assert!(Value::string("abc").as_int().is_err());
}

#[test]
fn list_roundtrip() {
    let v = Value::list(vec![Value::int(1), Value::string("a b")]);
    assert_eq!(&*v.to_str(), "1 {a b}");
    let source = Value::string("1 {a b}");
    let parsed = source.as_list().unwrap();
    assert_eq!(parsed.len(), 2);
    assert_eq!(&*parsed[1].to_str(), "a b");
}

#[test]
fn boolean_words() {
    assert!(Value::string("yes").as_bool().unwrap());
    assert!(!Value::string("off").as_bool().unwrap());
    assert!(Value::string("3").as_bool().unwrap());
}

#[test]
fn boolean_numeric_whitespace_matches_tcl() {
    assert!(Value::string("true").as_bool().unwrap());
    for value in [" true", "true ", "\ttrue\r"] {
        assert!(Value::string(value).as_bool().is_err(), "{value:?}");
    }
    assert!(Value::string("\t42\r").as_bool().unwrap());
    for value in ["\u{2003}42", "42\u{2003}"] {
        assert!(Value::string(value).as_bool().is_err(), "{value:?}");
    }
}

/// The boolean-context acceptor, oracle-pinned (tclsh 8.6/9.0): word
/// prefixes resolve, any number compares against zero, `Inf` is truthy —
/// and `NaN` is a domain error ("floating point value is Not a Number"),
/// never a truth value.
#[test]
fn boolean_context_matches_oracle() {
    assert!(Value::string("tru").as_bool().unwrap());
    assert!(!Value::string("of").as_bool().unwrap());
    assert!(Value::string("0.5").as_bool().unwrap());
    assert!(!Value::string("0x0").as_bool().unwrap());
    assert!(Value::string("Inf").as_bool().unwrap());
    assert!(Value::string("18446744073709551616").as_bool().unwrap());
    let err = Value::string("NaN").as_bool().unwrap_err();
    assert!(
        err.message_unicode()
            .expect("Unicode fixture error")
            .contains("Not a Number"),
        "NaN must be the C domain error, got: {err:?}"
    );
    assert!(Value::string("o").as_bool().is_err(), "ambiguous prefix");
}

/// `Value::to_str`'s descent into `IntRep::List` children recursing once
/// per nesting level with no depth cap — a plain `for {set i 0} {$i<N}
/// {incr i} {set v [list $v]}` loop builds the input, no `{*}` tricks
/// needed — empirically overflows the native stack (SIGABRT) between
/// depth 1200 and 1250 on a 2 MiB thread (`cargo test`'s per-test
/// default). 2000 is comfortably past both that crash range and
/// `MAX_LIST_TO_STR_DEPTH` (256); the assertion is that `to_str` returns
/// at all, not what it returns.
///
/// Deliberately NOT 50,000+: constructing (and, at the end of this
/// test, dropping) a `Value::list` chain nested that deep is its own,
/// unrelated native-stack risk — `Value` has no custom `Drop` impl, so
/// the compiler-generated recursive drop glue walks the same chain a
/// naive `to_str` would (empirically, SIGABRT between depth 3500 and 4000
/// on a 2 MiB thread for construction+drop alone, independent of
/// `to_str` or any other operation). That is a separate, genuinely
/// unbounded-depth concern in `Value`'s representation itself, and this
/// test does not cover it.
#[test]
fn deeply_nested_list_to_str_survives() {
    const DEPTH: usize = 2_000;
    let mut v = Value::string("leaf");
    for _ in 0..DEPTH {
        v = Value::list(vec![v]);
    }
    let s = v.to_str();
    assert!(!s.is_empty());
}

/// A moderately nested `[list $v]` chain (well under
/// `MAX_LIST_TO_STR_DEPTH`) is byte-for-byte unaffected by the depth
/// cap: its output matches an independent oracle (hand-driving
/// `list::join_list` the same number of times, entirely outside
/// `Value`) exactly.
#[test]
fn moderately_nested_list_to_str_matches_naive_join() {
    const DEPTH: usize = 20;
    let mut v = Value::string("a b");
    let mut expected = "a b".to_string();
    for _ in 0..DEPTH {
        v = Value::list(vec![v]);
        expected = list::join_list(std::iter::once(expected.as_str()));
    }
    assert_eq!(&*v.to_str(), expected.as_str());
}

/// The depth-cap fallback must not poison the shared string cache: a
/// value that only exceeds `MAX_LIST_TO_STR_DEPTH` because of *where*
/// it happens to be nested inside a deeper structure (reached via a
/// shared `Rc<Obj>`, e.g. the same sublist embedded in two different
/// places) must still render its true value when addressed directly
/// afterwards, starting fresh at depth 0 — not a cached placeholder
/// left over from the capped render.
#[test]
fn depth_cap_fallback_does_not_poison_the_shared_string_cache() {
    // `chain`: 10 levels of `[list $v]` around a value that needs
    // brace-quoting ("a b"), well within the cap on its own.
    let mut chain = Value::string("a b");
    let mut expected = "a b".to_string();
    for _ in 0..10 {
        chain = Value::list(vec![chain]);
        expected = list::join_list(std::iter::once(expected.as_str()));
    }
    // `far_outer`: the *same* `chain` object (`Rc` clone), wrapped 300
    // more levels deep — deep enough that reaching `chain` from here
    // exceeds `MAX_LIST_TO_STR_DEPTH` (256), even though `chain` itself
    // is shallow.
    let mut far_outer = chain.clone();
    for _ in 0..300 {
        far_outer = Value::list(vec![far_outer]);
    }
    // Force the deep, capped render first: `chain`'s own `Obj` must come
    // out of this untouched (not cached with the placeholder).
    let far_str = far_outer.to_str();
    assert!(
        far_str.contains(TOO_DEEPLY_NESTED_PLACEHOLDER),
        "expected the placeholder somewhere in: {far_str}"
    );
    // A fresh call directly on the same shared value, starting back at
    // depth 0, must render its true (well-within-cap) value — not the
    // placeholder that would have leaked in had the capped render above
    // wrongly cached it.
    assert_eq!(&*chain.to_str(), expected.as_str());
}
