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

//! Namespace queries, operation failures, creation diagnostics and import control, and dictionary key diagnostics.

use super::*;

#[test]
fn ensemble_defaults_keep_actual_parent_after_85() {
    let path = ByteNamespacePath::from_segments([b"a:".as_slice(), b"q".as_slice()]);
    let context = NativeNameContext::new(&path);
    for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
        let slot = NativeNameProtocol::C(version)
            .ensemble_publication_slot(context, None)
            .unwrap();
        assert_eq!(
            slot.namespace,
            ByteNamespacePath::from_segments([b"a:".as_slice()])
        );
        assert_eq!(slot.simple.as_bytes(), b"q");
    }
    let old = NativeNameProtocol::C(TclVersion::V8_5)
        .ensemble_publication_slot(context, None)
        .unwrap();
    assert_ne!(
        old.namespace,
        ByteNamespacePath::from_segments([b"a:".as_slice()])
    );
    assert!(
        NativeNameProtocol::C(TclVersion::V8_4)
            .ensemble_publication_slot(context, None)
            .is_err()
    );
}

#[test]
fn namespace_query_reporting_and_procedure_context_keep_distinct_extents() {
    for version in TclVersion::ALL {
        let c = NativeNameProtocol::C(version);
        assert_eq!(c.namespace_variable_query_input(b"k\0z").selected(), b"k");
        if version == TclVersion::V8_4 {
            assert!(report_native_namespace_lookup_error(c, b"k\0z", b"::n").is_err());
        } else {
            let report = report_native_namespace_lookup_error(c, b"k\0z", b"::n\xff").unwrap();
            assert_eq!(report.message, b"namespace \"k\" not found in \"::n\xff\"");
            assert_eq!(report.error_code, b"TCL LOOKUP NAMESPACE k");
            let absolute = report_native_namespace_lookup_error(c, b"::k\0z", b"::n").unwrap();
            assert_eq!(absolute.message, b"namespace \"::k\" not found");
        }
        assert_eq!(
            c.namespace_variable_query_input(b"k\xc0\x80z").selected(),
            b"k\xc0\x80z"
        );
    }
    let jim = NativeNameProtocol::Jim084;
    assert_eq!(
        jim.namespace_variable_query_input(b"k\0z").selected(),
        b"k\0z"
    );
    assert_eq!(jim.namespace_qualifier_bytes(b"x\0z::p"), b"");
    assert_eq!(jim.jim_procedure_namespace(b"x\0z::p").unwrap(), b"x\0z");
    assert_eq!(
        jim.jim_namespace_parent_bytes(b"absent::child").unwrap(),
        b"::absent"
    );
    assert_eq!(jim.jim_namespace_parent_bytes(b"absent").unwrap(), b"::");
    assert!(report_native_namespace_lookup_error(jim, b"absent", b"::").is_err());
}

#[test]
fn namespace_operation_failures_keep_release_and_operation_reporting() {
    use NativeNamespaceLookupOperation as Operation;
    for version in TclVersion::ALL {
        let c = NativeNameProtocol::C(version);
        let parent =
            report_native_namespace_operation_error(c, Operation::Parent, b"n\0z", b"::ctx")
                .unwrap();
        if version == TclVersion::V8_4 {
            assert_eq!(
                parent.message,
                b"unknown namespace \"n\" in namespace parent command"
            );
            assert_eq!(parent.error_code, None);
        } else {
            assert_eq!(parent.message, b"namespace \"n\" not found in \"::ctx\"");
            assert_eq!(
                parent.error_code.as_deref(),
                Some(b"TCL LOOKUP NAMESPACE n".as_slice())
            );
        }
        let deleted =
            report_native_namespace_operation_error(c, Operation::Delete, b"n\0z", b"::ctx")
                .unwrap();
        assert_eq!(
            deleted.message,
            b"unknown namespace \"n\" in namespace delete command"
        );
    }
    assert_eq!(
        NativeNameProtocol::Jim084
            .jim_namespace_parent_bytes(b"::\0z")
            .unwrap(),
        b""
    );
}

#[test]
fn dictionary_key_diagnostics_do_not_project_counted_identity() {
    use NativeDictionaryMissingKeyOperation::{Get, UnsetIntermediate};
    for version in [
        TclVersion::V8_5,
        TclVersion::V8_6,
        TclVersion::V9_0,
        TclVersion::V9_1,
    ] {
        let recipe = NativeNameProtocol::C(version);
        for operation in [Get, UnsetIntermediate] {
            for (original, reported) in [
                (b"a\0z".as_slice(), b"a".as_slice()),
                (b"a\xc0\x80z".as_slice(), b"a\xc0\x80z".as_slice()),
                (b"a\xffz".as_slice(), b"a\xffz".as_slice()),
            ] {
                let report =
                    report_native_dictionary_missing_key(recipe, operation, original).unwrap();
                let mut expected = b"key \"".to_vec();
                expected.extend_from_slice(reported);
                expected.extend_from_slice(b"\" not known in dictionary");
                assert_eq!(report.message, expected);
                assert_eq!(
                    report.error_code.is_some(),
                    version >= TclVersion::V9_0 || operation == UnsetIntermediate
                );
                if let Some(tuple) = report.error_code {
                    let mut expected = b"TCL LOOKUP DICT ".to_vec();
                    crate::list::append_list_element(&mut expected, reported, false);
                    assert_eq!(tuple, expected);
                }
            }
        }
    }
    assert!(
        report_native_dictionary_missing_key(NativeNameProtocol::C(TclVersion::V8_4), Get, b"a")
            .is_err()
    );
    let jim =
        report_native_dictionary_missing_key(NativeNameProtocol::Jim084, Get, b"a\0z").unwrap();
    assert_eq!(jim.message, b"key \"a\" not known in dictionary");
    assert!(jim.error_code.is_none());
}

#[test]
fn empty_namespace_creation_diagnostic_requires_its_selected_address_reason() {
    for version in TclVersion::ALL {
        let report = report_native_namespace_creation_error(
            NativeNameProtocol::C(version),
            NameProjectionUnavailable::EmptyNamespaceOutsideRoot,
        )
        .unwrap();
        assert_eq!(
            report.message,
            b"can't create namespace \"\": only global namespace can have empty name"
        );
        assert_eq!(
            report.error_code.as_deref(),
            (version >= TclVersion::V8_6)
                .then_some(b"TCL OPERATION NAMESPACE CREATEGLOBAL".as_slice())
        );
        assert_eq!(
            report.string_protocol,
            crate::native_string::NativeStringProtocol::C(version)
        );
        assert!(
            report_native_namespace_creation_error(
                NativeNameProtocol::C(version),
                NameProjectionUnavailable::PurposeNotModelled
            )
            .is_err()
        );
    }
    assert!(
        report_native_namespace_creation_error(
            NativeNameProtocol::Jim084,
            NameProjectionUnavailable::EmptyNamespaceOutsideRoot
        )
        .is_err()
    );
}

#[test]
fn namespace_creation_conflict_keeps_original_spelling_and_selected_extent() {
    for version in TclVersion::ALL {
        for original in [
            b"retiring".as_slice(),
            b"::retiring",
            b"::::retiring",
            b"retiring:tail",
            b"retiring\0ignored",
        ] {
            let report =
                report_native_namespace_creation_conflict(NativeNameProtocol::C(version), original)
                    .unwrap();
            let selected = if original == b"retiring\0ignored" {
                b"retiring".as_slice()
            } else {
                original
            };
            assert_eq!(
                report.message,
                [
                    b"can't create namespace \"".as_slice(),
                    selected,
                    b"\": already exists"
                ]
                .concat(),
            );
            assert_eq!(
                report.error_code.as_deref(),
                (version >= TclVersion::V8_6)
                    .then_some(b"TCL OPERATION NAMESPACE CREATEEXISTING".as_slice()),
            );
            assert_eq!(
                report.string_protocol,
                crate::native_string::NativeStringProtocol::C(version),
            );
        }
    }
    assert!(
        report_native_namespace_creation_conflict(NativeNameProtocol::Jim084, b"retiring").is_err()
    );
}

#[test]
fn empty_relative_namespace_address_does_not_select_nonroot_caller() {
    let mut parent = ByteNamespacePath::root();
    parent.push(b"outer");
    for version in [
        TclVersion::V8_4,
        TclVersion::V8_5,
        TclVersion::V8_6,
        TclVersion::V9_0,
        TclVersion::V9_1,
    ] {
        let recipe = NativeNameProtocol::C(version);
        for written in [b"".as_slice(), b"\0suffix"] {
            assert_eq!(
                recipe.namespace_address_path(NativeNameContext::new(&parent), written),
                Err(NameProjectionUnavailable::EmptyNamespaceOutsideRoot)
            );
            assert_eq!(
                recipe
                    .namespace_address_path(NativeNameContext::root(), written)
                    .unwrap(),
                ByteNamespacePath::root()
            );
        }
        assert_eq!(
            recipe
                .namespace_address_path(NativeNameContext::new(&parent), b"::outer::")
                .unwrap(),
            parent
        );
    }
}

#[test]
fn namespace_import_force_uses_only_exact_c_extent_and_declines_jim_control() {
    for version in tcl_dialect::TclVersion::ALL {
        let protocol = NativeNameProtocol::C(version);
        for (original, expected) in [
            (b"-force".as_slice(), true),
            (b"-force\0ignored".as_slice(), true),
            (b"-force\xc0\x80tail".as_slice(), false),
            (b"-for".as_slice(), false),
            (b"--".as_slice(), false),
            (b"::source::member".as_slice(), false),
        ] {
            assert_eq!(protocol.namespace_import_forces(original), Ok(expected));
        }
    }
    assert_eq!(
        NativeNameProtocol::Jim084.namespace_import_forces(b"-force"),
        Err(NameProjectionUnavailable::PurposeNotModelled)
    );
}
