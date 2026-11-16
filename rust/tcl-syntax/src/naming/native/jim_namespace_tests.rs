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

//! Jim namespace canonicalisation, construction, command-query candidates and parent and qualifier text.

use super::*;

#[test]
fn jim_helper_canonicalisation_keeps_relative_object_extent() {
    let path = ByteNamespacePath::root();
    let context = NativeNameContext::with_jim_namespace(&path, b"n\0z");
    let canonical = NativeNameProtocol::Jim084
        .jim_namespace_canonical_input(context, b"p\0q")
        .unwrap();
    assert_eq!(canonical.selected(), b"n\0z::p\0q");
    let absolute = NativeNameProtocol::Jim084
        .jim_namespace_canonical_input(context, b":::p\0q")
        .unwrap();
    assert_eq!(absolute.selected(), b"p");
    let published = NativeNameProtocol::Jim084
        .command_publication_input(context, b"p\0q")
        .unwrap();
    assert_ne!(canonical.selected(), published.selected());
    assert!(
        NativeNameProtocol::C(TclVersion::V9_0)
            .jim_namespace_canonical_input(context, b"p")
            .is_err()
    );
}

#[test]
fn jim_canonical_variable_reports_retain_selected_extent_and_purpose() {
    let jim = NativeNameProtocol::Jim084;
    let path = ByteNamespacePath::root();
    for (namespace, original, expected) in [
        (
            b"n\0z".as_slice(),
            b"p\0q".as_slice(),
            b"::n\0z::p\0q".as_slice(),
        ),
        (b"n".as_slice(), b"p::q".as_slice(), b"::n::p::q".as_slice()),
        (b"n".as_slice(), b":p".as_slice(), b"::n:::p".as_slice()),
        (b"n".as_slice(), b"".as_slice(), b"::n::".as_slice()),
        (b"".as_slice(), b"p\0q".as_slice(), b"::p\0q".as_slice()),
        (b"n\0z".as_slice(), b":::p\0q".as_slice(), b"::p".as_slice()),
        (b"n".as_slice(), b"::::".as_slice(), b"::".as_slice()),
        (
            b"n".as_slice(),
            b"::p\xff".as_slice(),
            b"::p\xff".as_slice(),
        ),
        (
            b"n\xff".as_slice(),
            b"a(k\0z)".as_slice(),
            b"::n\xff::a(k\0z)".as_slice(),
        ),
    ] {
        let selected = jim
            .jim_namespace_canonical_input(
                NativeNameContext::with_jim_namespace(&path, namespace),
                original,
            )
            .unwrap();
        assert_eq!(
            jim.jim_namespace_canonical_report_bytes(&selected).unwrap(),
            expected
        );
    }
    let absolute = jim
        .jim_namespace_absolute_canonical_input(b"::p\0q")
        .unwrap()
        .unwrap();
    assert!(absolute.context().is_none());
    assert_eq!(
        jim.jim_namespace_canonical_report_bytes(&absolute).unwrap(),
        b"::p"
    );
    let wrong_purpose = jim.package_key(b"p");
    assert_eq!(wrong_purpose.selected(), absolute.selected());
    assert!(
        jim.jim_namespace_canonical_report_bytes(&wrong_purpose)
            .is_err()
    );
    for version in TclVersion::ALL {
        let c = NativeNameProtocol::C(version);
        let c_input = c.namespace_variable_query_input(b"::p");
        assert!(jim.jim_namespace_canonical_report_bytes(&c_input).is_err());
        assert!(c.jim_namespace_canonical_report_bytes(&absolute).is_err());
    }
}

#[test]
fn jim_absolute_canonical_projection_needs_no_namespace_bytes() {
    let jim = NativeNameProtocol::Jim084;
    for (original, expected) in [
        (b":::p\0q".as_slice(), b"p".as_slice()),
        (b"::::".as_slice(), b"".as_slice()),
        (b"::p\xff".as_slice(), b"p\xff".as_slice()),
    ] {
        let canonical = jim
            .jim_namespace_absolute_canonical_input(original)
            .unwrap()
            .unwrap();
        assert_eq!(canonical.original(), original);
        assert_eq!(canonical.selected(), expected);
        assert_eq!(
            canonical.purpose(),
            NativeNamePurpose::JimNamespaceCanonical
        );
        assert!(canonical.context().is_none());
        let candidates = jim
            .jim_namespace_command_query_candidates_from_canonical(&canonical)
            .unwrap();
        let mut expected_rooted = b"::".to_vec();
        expected_rooted.extend_from_slice(expected);
        assert_eq!(candidates[0].lookup.as_bytes(), expected_rooted);
        assert_eq!(candidates[0].reported, candidates[0].lookup);
        let mut original_rooted = b"::".to_vec();
        original_rooted.extend_from_slice(original);
        assert_eq!(candidates[1].lookup.as_bytes(), original_rooted);
        assert_eq!(candidates[1].reported, candidates[1].lookup);
        // The existing contextual entry still requires an actual namespace.
        assert!(
            jim.jim_namespace_canonical_input(
                NativeNameContext::new(&ByteNamespacePath::root()),
                original,
            )
            .is_err()
        );
    }
    for relative in [b"p".as_slice(), b":p", b"", b"p\0q"] {
        assert!(
            jim.jim_namespace_absolute_canonical_input(relative)
                .unwrap()
                .is_none()
        );
    }
    let wrong_purpose = jim.package_key(b"::p");
    assert!(
        jim.jim_namespace_command_query_candidates_from_canonical(&wrong_purpose)
            .is_err()
    );
    for version in TclVersion::ALL {
        let c = NativeNameProtocol::C(version);
        assert!(c.jim_namespace_absolute_canonical_input(b"::p").is_err());
        let canonical = jim
            .jim_namespace_absolute_canonical_input(b"::p")
            .unwrap()
            .unwrap();
        assert!(
            c.jim_namespace_command_query_candidates_from_canonical(&canonical)
                .is_err()
        );
    }
}

#[test]
fn jim_namespace_construction_retains_original_object_only_for_root_relative_names() {
    use NativeJimNamespaceConstruction::{
        DuplicateNamespaceAndAppend, FreshString, RetainOriginal,
    };
    let jim = NativeNameProtocol::Jim084;
    for name in [&b"a\0z"[..], &b"a\xffz"[..], &b""[..]] {
        assert_eq!(
            jim.jim_namespace_construction(NativeNameContext::root(), name)
                .unwrap(),
            RetainOriginal
        );
    }
    assert_eq!(
        jim.jim_namespace_construction(NativeNameContext::root(), b"::a\0z")
            .unwrap(),
        FreshString
    );
    let path = ByteNamespacePath::root();
    let context = NativeNameContext::with_jim_namespace(&path, b"n\0z");
    assert_eq!(
        jim.jim_namespace_construction(context, b"a").unwrap(),
        DuplicateNamespaceAndAppend
    );
    assert_eq!(
        jim.jim_namespace_construction(context, b"::a").unwrap(),
        FreshString
    );
}

#[cfg(test)]
#[test]
fn jim_namespace_command_query_uses_two_fresh_report_candidates() {
    let jim = NativeNameProtocol::Jim084;
    let path = ByteNamespacePath::root().with_child(b"__nh_context");
    let context = NativeNameContext::with_jim_namespace(&path, b"__nh_context");
    for (original, first, second) in [
        (
            b"__nh_shared".as_slice(),
            b"::__nh_context::__nh_shared".as_slice(),
            b"::__nh_shared".as_slice(),
        ),
        (
            b"::__nh_shared".as_slice(),
            b"::__nh_shared".as_slice(),
            b"::::__nh_shared".as_slice(),
        ),
        (
            b":__nh_literal".as_slice(),
            b"::__nh_context:::__nh_literal".as_slice(),
            b":::__nh_literal".as_slice(),
        ),
    ] {
        let candidates = jim
            .jim_namespace_command_query_candidates(context, original)
            .unwrap();
        assert_eq!(candidates[0].lookup.as_bytes(), first);
        assert_eq!(candidates[1].lookup.as_bytes(), second);
        for candidate in candidates {
            assert_eq!(candidate.lookup, candidate.reported);
        }
    }
    let same = jim
        .jim_namespace_command_query_candidates(NativeNameContext::root(), b"__nh_shared")
        .unwrap();
    assert_eq!(same[0], same[1]);
    assert_eq!(
        jim.jim_namespace_command_query_candidates(NativeNameContext::new(&path), b"name"),
        Err(NameProjectionUnavailable::MissingJimNamespaceObject)
    );
    assert_eq!(
        NativeNameProtocol::C(TclVersion::V9_0)
            .jim_namespace_command_query_candidates(context, b"name"),
        Err(NameProjectionUnavailable::PurposeNotModelled)
    );
}

#[test]
fn original_jim_parent_producer_distinguishes_unchanged_counted_and_rooted_branches() {
    use NativeJimNamespaceParentResult::{
        CountedQualifier, RootedQualifier, RootedSharedEmpty, Unchanged,
    };
    let jim = NativeNameProtocol::Jim084;
    for input in [b"".as_slice(), b"::", b"::\0tail"] {
        assert_eq!(jim.jim_namespace_parent_result(input).unwrap(), Unchanged);
    }
    assert_eq!(
        jim.jim_namespace_parent_result(b"::absent::child").unwrap(),
        CountedQualifier(b"::absent")
    );
    assert_eq!(
        jim.jim_namespace_parent_result(b"absent::child").unwrap(),
        RootedQualifier(b"absent")
    );
    assert_eq!(
        jim.jim_namespace_parent_result(b"84").unwrap(),
        RootedSharedEmpty
    );
    assert_eq!(
        jim.jim_namespace_parent_result(b"::child").unwrap(),
        RootedQualifier(b"")
    );
    assert_eq!(
        jim.jim_namespace_parent_result(b"unpaired:").unwrap(),
        RootedSharedEmpty
    );
    for version in TclVersion::ALL {
        assert!(
            NativeNameProtocol::C(version)
                .jim_namespace_parent_result(b"absent")
                .is_err()
        );
    }
}

#[test]
fn jim_qualifiers_shared_empty_is_distinct_from_a_paired_root_counted_empty() {
    let jim = NativeNameProtocol::Jim084;
    for original in [b"".as_slice(), b"plain", b":", b"plain\0::tail"] {
        assert_eq!(
            jim.namespace_text_result(original, false),
            NativeNamespaceTextResult::SharedEmpty
        );
        assert_eq!(
            jim.namespace_text_result(original, true),
            NativeNamespaceTextResult::Original
        );
    }
    for original in [b"::".as_slice(), b"::\0tail"] {
        assert_eq!(
            jim.namespace_text_result(original, false),
            NativeNamespaceTextResult::Counted(b"")
        );
        assert_eq!(
            jim.namespace_text_result(original, true),
            NativeNamespaceTextResult::Counted(b"")
        );
    }
    for version in TclVersion::ALL {
        assert_ne!(
            NativeNameProtocol::C(version).namespace_text_result(b"plain", false),
            NativeNamespaceTextResult::SharedEmpty
        );
    }
}
