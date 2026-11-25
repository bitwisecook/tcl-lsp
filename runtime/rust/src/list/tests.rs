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
fn selected_double_updaters_match_all_native_nonfinite_payloads() {
    let fixtures = [
        include_str!("../../../../rust/tcl-syntax/testdata/native_nonfinite_double/8.4.20.tsv"),
        include_str!("../../../../rust/tcl-syntax/testdata/native_nonfinite_double/8.5.19.tsv"),
        include_str!("../../../../rust/tcl-syntax/testdata/native_nonfinite_double/8.6.18.tsv"),
        include_str!("../../../../rust/tcl-syntax/testdata/native_nonfinite_double/9.0.4.tsv"),
        include_str!("../../../../rust/tcl-syntax/testdata/native_nonfinite_double/9.1.0.tsv"),
        include_str!("../../../../rust/tcl-syntax/testdata/native_nonfinite_double/jim.tsv"),
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
            let (case, expected) = tcl_test_support::fixture::tsv_split_once(row).unwrap();
            let case: usize = case.parse().unwrap();
            let value = f64::from_bits(bits[case]);
            let object = obj::Owned::fresh(obj::new_double_obj(value));
            assert!(!obj::has_string_rep(object.as_ptr()));
            let bytes = crate::dict::native_object_bytes(object.as_ptr(), protocol).unwrap();
            assert_eq!(obj::double_of(object.as_ptr()).to_bits(), bits[case]);
            let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
            assert_eq!(hex, expected, "{protocol:?}/{case}");
            observations += 1;
        }
    }
    assert_eq!(observations, 36);
}

#[test]
fn list_backing_canonical_flags_match_original_native_operations() {
    let fixtures = [
        include_str!("../../../../rust/tcl-syntax/testdata/native_list_canonical/8.4.20.tsv"),
        include_str!("../../../../rust/tcl-syntax/testdata/native_list_canonical/8.5.19.tsv"),
        include_str!("../../../../rust/tcl-syntax/testdata/native_list_canonical/8.6.18.tsv"),
        include_str!("../../../../rust/tcl-syntax/testdata/native_list_canonical/9.0.4.tsv"),
        include_str!("../../../../rust/tcl-syntax/testdata/native_list_canonical/9.1.0.tsv"),
    ];
    let mut operations = 0;
    for (version, fixture) in tcl_dialect::TclVersion::ALL.into_iter().zip(fixtures) {
        let protocol = NativeStringProtocol::C(version);
        let rows: Vec<_> = fixture.lines().collect();
        for pair in rows.chunks_exact(2) {
            let before_fields: Vec<_> = tcl_test_support::fixture::tsv_fields(pair[0]).collect();
            let after_fields: Vec<_> = tcl_test_support::fixture::tsv_fields(pair[1]).collect();
            let mode: usize = before_fields[0].parse().unwrap();
            leak_free(|| {
                let first = obj::Owned::fresh(obj::new_string_bytes(b"a"));
                let second = obj::Owned::fresh(obj::new_string_bytes(b"b"));
                let third = obj::Owned::fresh(obj::new_string_bytes(b"c"));
                let root = obj::Owned::fresh(super::new_list_obj_native(
                    &[first.as_ptr(), second.as_ptr()],
                    protocol,
                ));
                let root = if mode == 4 {
                    let parsed = obj::Owned::fresh(obj::new_string_bytes(b"a   b"));
                    drop(super::list_elements_native_checked(parsed.as_ptr(), protocol).unwrap());
                    parsed
                } else {
                    root
                };
                if (2..5).contains(&mode) {
                    crate::dict::native_object_bytes(root.as_ptr(), protocol).unwrap();
                }
                let duplicate = (mode == 3 || mode == 5)
                    .then(|| obj::Owned::fresh(obj::duplicate(root.as_ptr())));
                let selected = duplicate.as_ref().unwrap_or(&root);
                let before = super::native_cache_snapshot(selected.as_ptr()).unwrap().1;
                assert_eq!(
                    obj::has_string_rep(selected.as_ptr()),
                    before_fields[3] == "1"
                );
                if version != tcl_dialect::TclVersion::V8_4 {
                    assert_eq!(before, before_fields[4] == "1");
                }
                let result = if mode == 0 || mode == 5 {
                    crate::dict::native_object_bytes(selected.as_ptr(), protocol).unwrap();
                    obj::Owned::retain(selected.as_ptr())
                } else {
                    super::append_native_elements(
                        Some(selected.as_ptr()),
                        &[third.as_ptr()],
                        protocol,
                    )
                    .unwrap()
                };
                assert_eq!(obj::has_string_rep(result.as_ptr()), after_fields[3] == "1");
                if version != tcl_dialect::TclVersion::V8_4 {
                    assert_eq!(
                        super::native_cache_snapshot(result.as_ptr()).unwrap().1,
                        after_fields[4] == "1"
                    );
                }
            });
            operations += 1;
        }
    }
    assert_eq!(operations, 30);
}

#[test]
fn selected_empty_compound_updaters_install_native_allocation_identity() {
    use tcl_syntax::native_string::{NativeStringProtocol, NativeStringStorageIdentity};
    for protocol in [
        NativeStringProtocol::C(tcl_dialect::TclVersion::V8_4),
        NativeStringProtocol::C(tcl_dialect::TclVersion::V8_5),
        NativeStringProtocol::C(tcl_dialect::TclVersion::V8_6),
        NativeStringProtocol::C(tcl_dialect::TclVersion::V9_0),
        NativeStringProtocol::C(tcl_dialect::TclVersion::V9_1),
        NativeStringProtocol::Jim084,
    ] {
        let list = crate::obj::Owned::fresh(super::new_list_obj(&[]));
        super::seal_string_protocol(list.as_ptr(), protocol).unwrap();
        assert!(!crate::obj::has_string_rep(list.as_ptr()));
        assert_eq!(
            crate::dict::native_object_bytes(list.as_ptr(), protocol).unwrap(),
            b""
        );
        assert_eq!(
            crate::obj::has_canonical_empty_string(list.as_ptr()),
            protocol.compound_updater_storage() == NativeStringStorageIdentity::CanonicalEmpty
        );
        assert_eq!(
            super::native_list_backing(list.as_ptr())
                .unwrap()
                .canonical(),
            protocol.updated_list_canonical(false, false)
        );
        if protocol == NativeStringProtocol::C(tcl_dialect::TclVersion::V8_4) {
            continue;
        }
        let dict = crate::obj::Owned::fresh(crate::dict::new_dict_obj(&[]));
        crate::dict::seal_string_protocol(dict.as_ptr(), protocol).unwrap();
        assert_eq!(
            crate::dict::native_object_bytes(dict.as_ptr(), protocol).unwrap(),
            b""
        );
        assert_eq!(
            crate::obj::has_canonical_empty_string(dict.as_ptr()),
            protocol.compound_updater_storage() == NativeStringStorageIdentity::CanonicalEmpty
        );
    }
}

#[test]
fn compound_updater_matches_all_six_native_original_object_fixtures() {
    use tcl_dialect::TclVersion;
    const FIXTURES: [&str; 6] = [
        include_str!(
            "../../../../rust/tcl-syntax/testdata/native_compound_string_updaters/8.4.20.tsv"
        ),
        include_str!(
            "../../../../rust/tcl-syntax/testdata/native_compound_string_updaters/8.5.19.tsv"
        ),
        include_str!(
            "../../../../rust/tcl-syntax/testdata/native_compound_string_updaters/8.6.18.tsv"
        ),
        include_str!(
            "../../../../rust/tcl-syntax/testdata/native_compound_string_updaters/9.0.4.tsv"
        ),
        include_str!(
            "../../../../rust/tcl-syntax/testdata/native_compound_string_updaters/9.1.0.tsv"
        ),
        include_str!(
            "../../../../rust/tcl-syntax/testdata/native_compound_string_updaters/jim.tsv"
        ),
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
            crate::native_source::NativeJimObjectContext::new(
                tcl_registry::InvocationDialect::of_profile(
                    crate::environment::profile_for_dialect("jim"),
                ),
            )
            .unwrap()
        });

        for row in fixture.lines() {
            let fields: Vec<_> = tcl_test_support::fixture::tsv_fields(row).collect();
            rows += 1;
            if fields[1] == "unavailable" {
                assert_eq!(protocol, protocols[0]);
                unavailable += 1;
                continue;
            }
            let case: u8 = fields[0].parse().unwrap();
            let list = |items: &[*mut TclObj]| super::new_list_obj_native(items, protocol);
            let dict = || {
                let root = crate::dict::new_dict_obj(&[(
                    obj::new_string_bytes(b"#key"),
                    obj::new_string_bytes(b"a\"b"),
                )]);
                crate::dict::seal_string_protocol(root, protocol).unwrap();
                root
            };
            let root = obj::Owned::fresh(match case {
                0 => list(&[
                    obj::new_string_bytes(b"#first"),
                    obj::new_string_bytes(b"#later"),
                ]),
                1 => list(&[obj::new_string_bytes(b"a\"b"), obj::new_string_bytes(b"]")]),
                2 => list(&[
                    obj::new_string_bytes(b"A\0\xff"),
                    obj::new_string_bytes(b"\\\n"),
                ]),
                3 => list(&[
                    list(&[obj::new_wide_int_obj(17), obj::new_string_bytes(b"a\"b")]),
                    obj::new_string_bytes(b"#later"),
                ]),
                4 => dict(),
                5 => list(&[dict(), obj::new_string_bytes(b"#later")]),
                6 => {
                    let value = obj::new_string_bytes(b"ORIGINAL");
                    if let Some(context) = &source_context {
                        crate::native_source::bind_context(value, context).unwrap();
                    }
                    drop(super::list_elements_native_checked(value, protocol).unwrap());
                    value
                }
                _ => unreachable!(),
            });
            let type_name = |value| {
                let descriptor = obj::obj_type_ptr(value);
                if descriptor.is_null() {
                    "none"
                } else {
                    unsafe { core::ffi::CStr::from_ptr((*descriptor).name) }
                        .to_str()
                        .unwrap()
                }
            };
            let state = || {
                let child = if let Some(backing) = super::native_list_backing(root.as_ptr()) {
                    backing.elements().unwrap()[0]
                } else {
                    crate::dict::dict_pairs(root.as_ptr()).unwrap()[0].1
                };
                (
                    type_name(root.as_ptr()),
                    obj::has_string_rep(root.as_ptr()),
                    type_name(child),
                    obj::has_string_rep(child),
                    unsafe { (*child).ref_count },
                )
            };
            let before = state();
            assert_eq!(before.0, fields[1]);
            assert_eq!(before.1, fields[2] == "1");
            assert_eq!(before.2, fields[3]);
            assert_eq!(before.3, fields[4] == "1");
            assert_eq!(before.4, fields[10].parse::<isize>().unwrap());
            let bytes = crate::dict::native_object_bytes(root.as_ptr(), protocol).unwrap();
            let hex = bytes
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>();
            assert_eq!(hex, fields[5], "{protocol:?}: {row}");
            let after = state();
            assert_eq!(after.0, fields[6]);
            assert_eq!(after.1, fields[7] == "1");
            assert_eq!(after.2, fields[8]);
            assert_eq!(after.2, before.2);
            assert_eq!(after.3, fields[9] == "1");
            assert_eq!(after.4, fields[11].parse::<isize>().unwrap());
        }
    }
    assert_eq!(rows, 42);
    assert_eq!(unavailable, 2);
}

// These ABI backing tests choose the audited C8.5 string updater explicitly.
fn test_list(elems: &[*mut TclObj]) -> *mut TclObj {
    super::new_list_obj_native(
        elems,
        NativeStringProtocol::C(tcl_dialect::TclVersion::V8_5),
    )
}
use super::*;
use crate::counters;
use crate::obj::{new_string_bytes, TclObj};

fn leak_free(body: impl FnOnce()) {
    counters::reset();
    body();
    assert_eq!(
        counters::finalize(),
        0,
        "residual: {} objs, {} bufs",
        counters::live_objs(),
        counters::live_bufs()
    );
    assert_eq!(counters::double_free_count(), 0);
}

fn release(obj: *mut TclObj) {
    unsafe {
        obj::incr_ref_count(obj);
        obj::decr_ref_count(obj);
    }
}

#[test]
fn native_dictionary_list_conversion_keeps_members_and_resident_spelling() {
    use tcl_syntax::native_string::NativeStringProtocol;
    leak_free(|| {
        for protocol in [
            NativeStringProtocol::C(tcl_dialect::TclVersion::V8_5),
            NativeStringProtocol::C(tcl_dialect::TclVersion::V8_6),
            NativeStringProtocol::C(tcl_dialect::TclVersion::V9_0),
            NativeStringProtocol::C(tcl_dialect::TclVersion::V9_1),
            NativeStringProtocol::Jim084,
        ] {
            let key = obj::Owned::fresh(obj::new_string_bytes(b"k\0\xff"));
            let child = obj::Owned::fresh(obj::new_wide_int_obj(7));
            let parent =
                obj::Owned::fresh(crate::dict::new_dict_obj(&[(key.as_ptr(), child.as_ptr())]));
            let alias = parent.clone();
            let members = list_elements_native_checked(parent.as_ptr(), protocol).unwrap();
            assert_eq!(members, [key.as_ptr(), child.as_ptr()]);
            assert!(!obj::has_string_rep(alias.as_ptr()));
            assert!(!obj::has_string_rep(child.as_ptr()));
            assert!(!native_list_backing(alias.as_ptr()).unwrap().canonical());
            assert_eq!(unsafe { (*child.as_ptr()).ref_count }, 2);

            let resident =
                obj::Owned::fresh(crate::dict::new_dict_obj(&[(key.as_ptr(), child.as_ptr())]));
            unsafe { obj::set_string_rep(resident.as_ptr(), b"k FIRST k LAST") };
            let source_context = protocol.is_jim084().then(|| {
                crate::native_source::NativeJimObjectContext::new(
                    tcl_registry::InvocationDialect::of_profile(
                        crate::environment::profile_for_dialect("jim"),
                    ),
                )
                .unwrap()
            });
            if let Some(context) = &source_context {
                crate::native_source::bind_context(resident.as_ptr(), context).unwrap();
            }
            let members = list_elements_native_checked(resident.as_ptr(), protocol).unwrap();
            assert_eq!(members.len(), 4);
            assert_eq!(obj::bytes_of(members[1]), b"FIRST");
            assert_eq!(obj::bytes_of(members[3]), b"LAST");
            assert_ne!(members[1], child.as_ptr());
        }
    });
}

#[test]
fn lifetime_views_do_not_create_native_headers_or_force_member_cow() {
    leak_free(|| {
        let child = obj::Owned::fresh(obj::new_string_bytes(b"MEMBER"));
        let parent = obj::Owned::fresh(test_list(&[child.as_ptr()]));
        let view = native_list_backing(parent.as_ptr()).unwrap();
        let another_view = view.clone();
        assert_eq!(&*view.elements().unwrap(), &[child.as_ptr()]);
        assert_eq!(&*another_view.elements().unwrap(), &[child.as_ptr()]);
        assert_eq!(
            native_header_reference_count(parent.as_ptr()).unwrap(),
            Some(1)
        );
        let extra = obj::Owned::fresh(obj::new_string_bytes(b"EXTRA"));
        list_append(parent.as_ptr(), extra.as_ptr()).unwrap();
        assert_eq!(unsafe { (*child.as_ptr()).ref_count }, 2);
        // Lifetime views keep allocation safety without authorising a new
        // member-vector generation or adding native List header owners.
        let changed = tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "changed native List member vector",
        );
        assert_eq!(view.elements().unwrap_err(), changed);
        assert_eq!(another_view.elements().unwrap_err(), changed);
        let current = native_list_backing(parent.as_ptr()).unwrap();
        assert_eq!(
            &*current.elements().unwrap(),
            &[child.as_ptr(), extra.as_ptr()]
        );
        let copy = obj::Owned::fresh(obj::duplicate(parent.as_ptr()));
        assert_eq!(
            native_header_reference_count(parent.as_ptr()).unwrap(),
            Some(2)
        );
        assert_eq!(
            native_header_reference_count(copy.as_ptr()).unwrap(),
            Some(2)
        );
        list_append(copy.as_ptr(), extra.as_ptr()).unwrap();
        assert_eq!(
            native_header_reference_count(parent.as_ptr()).unwrap(),
            Some(1)
        );
        assert_eq!(
            native_header_reference_count(copy.as_ptr()).unwrap(),
            Some(1)
        );
        assert_eq!(unsafe { (*child.as_ptr()).ref_count }, 3);
        // Mutating the copied header detaches its backing; the original
        // current vector remains readable without forcing member COW.
        assert_eq!(current.elements().unwrap().len(), 2);
        assert_eq!(view.elements().unwrap_err(), changed);
    });
}

#[test]
fn original_backing_survives_shimmer_without_extra_child_owners() {
    leak_free(|| {
        let child = obj::Owned::fresh(obj::new_string_bytes(b"CHILD\xff"));
        let parent = obj::Owned::fresh(test_list(&[child.as_ptr()]));
        assert_eq!(unsafe { (*child.as_ptr()).ref_count }, 2);
        let backing = native_list_backing(parent.as_ptr()).unwrap();
        assert!(!backing.canonical());
        assert_eq!(unsafe { (*child.as_ptr()).ref_count }, 2);
        obj::change_type(parent.as_ptr(), &obj::TCL_INT_TYPE, 7);
        assert_eq!(&*backing.elements().unwrap(), &[child.as_ptr()]);
        assert_eq!(obj::bytes_of(backing.elements().unwrap()[0]), b"CHILD\xff");
        assert_eq!(unsafe { (*child.as_ptr()).ref_count }, 1);
        drop(backing);
        assert_eq!(unsafe { (*child.as_ptr()).ref_count }, 1);
    });
}

#[test]
fn native_c_list_copy_shares_backing_until_first_mutation() {
    leak_free(|| {
        let protocol = tcl_syntax::native_string::NativeStringProtocol::for_tcl_version(
            tcl_dialect::TclVersion::V9_0,
        );
        let child = obj::Owned::fresh(obj::new_wide_int_obj(7));
        let parent = obj::Owned::fresh(test_list(&[child.as_ptr()]));
        let copy = obj::Owned::fresh(obj::duplicate(parent.as_ptr()));
        duplicate_native_backing(parent.as_ptr(), copy.as_ptr(), protocol);
        assert_eq!(unsafe { (*child.as_ptr()).ref_count }, 2);
        let pinned = native_list_backing(parent.as_ptr()).unwrap();
        let extra = obj::Owned::fresh(obj::new_string_bytes(b"EXTRA"));
        list_append(copy.as_ptr(), extra.as_ptr()).unwrap();
        assert_eq!(&*pinned.elements().unwrap(), &[child.as_ptr()]);
        assert_eq!(
            list_elements(copy.as_ptr()).unwrap(),
            vec![child.as_ptr(), extra.as_ptr()]
        );
        assert_eq!(unsafe { (*child.as_ptr()).ref_count }, 3);
    });
}

#[test]
fn build_index_length() {
    leak_free(|| {
        let a = new_string_bytes(b"a");
        let b = new_string_bytes(b"b");
        let c = new_string_bytes(b"c");
        let list = test_list(&[a, b, c]);
        unsafe { obj::incr_ref_count(list) };
        assert_eq!(list_length(list).unwrap(), 3);
        assert_eq!(obj::bytes_of(list_index(list, 1).unwrap().unwrap()), b"b");
        assert!(list_index(list, 9).unwrap().is_none());
        // the bare element objs (a,b,c) are owned by the list; free our
        // construction refs (they were rc 0, list took them to rc 1)
        unsafe { obj::decr_ref_count(list) }; // frees list + its 3 elements
    });
}

#[test]
fn list_to_string_quotes() {
    leak_free(|| {
        let plain = new_string_bytes(b"a");
        let spaced = new_string_bytes(b"b c");
        let empty = new_string_bytes(b"");
        let list = test_list(&[plain, spaced, empty]);
        unsafe { obj::incr_ref_count(list) };
        // a {b c} {}
        assert_eq!(obj::bytes_of(list), b"a {b c} {}");
        unsafe { obj::decr_ref_count(list) };
    });
}

#[test]
fn string_to_list_shimmer() {
    leak_free(|| {
        let s = new_string_bytes(b"x {y z} w");
        unsafe { obj::incr_ref_count(s) };
        // reading length shimmers the string into a list
        assert_eq!(list_length(s).unwrap(), 3);
        assert_eq!(obj::bytes_of(list_index(s, 1).unwrap().unwrap()), b"y z");
        unsafe { obj::decr_ref_count(s) };
    });
}

#[test]
fn append_invalidates_string_rep() {
    leak_free(|| {
        let list = test_list(&[new_string_bytes(b"a")]);
        unsafe { obj::incr_ref_count(list) };
        assert_eq!(obj::bytes_of(list), b"a");
        list_append(list, new_string_bytes(b"b")).unwrap();
        // string rep regenerated to include the new element
        assert_eq!(obj::bytes_of(list), b"a b");
        assert_eq!(list_length(list).unwrap(), 2);
        unsafe { obj::decr_ref_count(list) };
    });
}

#[test]
fn list_element_quoting_modes() {
    let q = |e: &[u8]| {
        let mut buf = Vec::new();
        append_list_element(&mut buf, e, true);
        buf
    };
    // escape mode (unbalanced open brace) — braces escaped too.
    assert_eq!(q(b"a{b"), b"a\\{b");
    // bare — balanced braces need no quoting.
    assert_eq!(q(b"a{b}c"), b"a{b}c");
    // brace mode — `[`/`$`/`;` force quoting but braces round-trip.
    assert_eq!(q(b"[append"), b"{[append}");
    assert_eq!(q(b"a$b"), b"{a$b}");
    assert_eq!(q(b"a;b"), b"{a;b}");
    assert_eq!(q(b"a[b]c"), b"{a[b]c}");
    // mask mode — a lone `]`/`"` escapes (braces left literal).
    assert_eq!(q(b".b]"), b".b\\]");
    assert_eq!(q(b"x]y"), b"x\\]y");
    // leading `#` (first element) → brace.
    assert_eq!(q(b"#c"), b"{#c}");
    // whitespace → brace.
    assert_eq!(q(b"a b"), b"{a b}");
    let _ = release; // silence unused in this pure test
}
