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

//! Publication of `TclOO` objects, methods and properties, aliases and child aliases, and Jim's two-word factory commands.

use super::*;
use tcl_core_types::ByteCommandSlot;

#[test]
fn native_oo_counted_method_and_cstring_object_publication_are_distinct() {
    for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
        let recipe = NativeNameProtocol::C(version);
        for original in [
            b"a\0z".as_slice(),
            b"a\xffz".as_slice(),
            b"a\xc0\x80z".as_slice(),
        ] {
            let method = recipe.oo_method_input(original).unwrap();
            assert_eq!(method.original(), original);
            assert_eq!(method.selected(), original);
            assert_eq!(method.purpose(), NativeNamePurpose::OoMethod);
            let object = recipe
                .oo_object_publication_slot(NativeNameContext::root(), original)
                .unwrap();
            assert_eq!(object.simple.as_bytes(), c_string_extent(original));
            assert_eq!(
                report_native_name_bytes(
                    recipe,
                    NativeNameReportPurpose::OoMethodEnumeration,
                    original
                )
                .unwrap(),
                c_string_extent(original)
            );
        }
        let path = ByteNamespacePath::from_segments([NameBytes::from(b"a:".as_slice())]);
        let object = recipe
            .oo_object_publication_slot(NativeNameContext::new(&path), b"p")
            .unwrap();
        assert_eq!(object.namespace, path);
    }
    for recipe in [
        NativeNameProtocol::C(TclVersion::V8_4),
        NativeNameProtocol::C(TclVersion::V8_5),
        NativeNameProtocol::Jim084,
    ] {
        assert!(recipe.oo_method_input(b"m").is_err());
        assert!(
            recipe
                .oo_object_publication_slot(NativeNameContext::root(), b"O")
                .is_err()
        );
        assert!(
            report_native_name_bytes(recipe, NativeNameReportPurpose::OoMethodEnumeration, b"m")
                .is_err()
        );
    }
}

#[test]
fn alias_publication_retains_its_distinct_native_entry_point() {
    // naming.class.jim-source-rooted-factory-initialiser-replacement
    // docs/design/analysis/name-resolution-proofs/class-jim-source-rooted-factory-initialiser-replacement.md
    // Native rooted factory observations motivate table-key comparison; extent
    // controls preserve original/reporting producer bytes independently.
    let namespace = ByteNamespacePath::from_segments(["n"]);
    let context = NativeNameContext::new(&namespace);
    for version in TclVersion::ALL {
        let c = NativeNameProtocol::C(version);
        assert_eq!(
            c.alias_publication_slot(context, b"x").unwrap(),
            ByteCommandSlot::new(ByteNamespacePath::root(), NameBytes::from("x"))
        );
        assert_eq!(
            c.alias_publication_slot(context, b"q::x").unwrap(),
            ByteCommandSlot::new(
                ByteNamespacePath::from_segments(["n", "q"]),
                NameBytes::from("x")
            )
        );
        assert_eq!(
            c.alias_publication_slot(context, b"::q::x").unwrap(),
            ByteCommandSlot::new(
                ByteNamespacePath::from_segments(["q"]),
                NameBytes::from("x")
            )
        );
        assert_eq!(
            c.alias_publication_slot(context, b"x\0z")
                .unwrap()
                .simple
                .as_bytes(),
            b"x"
        );
        assert_eq!(
            c.alias_publication_slot(context, b"x\xc0\x80z")
                .unwrap()
                .simple
                .as_bytes(),
            b"x\xc0\x80z"
        );
    }
    let jim = NativeNameProtocol::Jim084;
    for name in [b"x".as_slice(), b"q::x", b"::q::x", b"x\0z", b"x\xff"] {
        let slot = jim.alias_publication_slot(context, name).unwrap();
        assert!(slot.namespace.is_root());
        assert_eq!(slot.simple.as_bytes(), strip_jim_root(name));
        let publication = jim.alias_publication_input(context, name).unwrap();
        assert_eq!(publication.original(), name);
        assert_eq!(publication.selected(), name);
        assert_eq!(publication.jim_flat_key(), Some(strip_jim_root(name)));
    }
}

#[test]
fn rename_alias_loop_diagnostic_uses_selected_release_and_original_slot() {
    for version in TclVersion::ALL {
        let selected = NativeNameProtocol::C(version)
            .rename_alias_loop_name(b"source\0OPAQUE", b"destination\0OPAQUE");
        assert_eq!(
            selected,
            Some(if version == TclVersion::V8_4 {
                b"source".as_slice()
            } else {
                b"destination".as_slice()
            })
        );
    }
    assert_eq!(
        NativeNameProtocol::Jim084.rename_alias_loop_name(b"source", b"destination"),
        None
    );
}

#[test]
fn original_property_option_extent_is_separate_from_counted_declaration_names() {
    // Implementation contract: naming.tcloo.original-property-accessor-source-advice
    // docs/design/analysis/name-resolution-proofs/tcloo-original-property-accessor-source-advice.md
    for version in [TclVersion::V9_0, TclVersion::V9_1] {
        let recipe = NativeNameProtocol::C(version);
        for (original, expected) in [
            (b"p\0tail".as_slice(), b"-p".as_slice()),
            (b"p\xc0\x80tail", b"-p\xc0\x80tail"),
            (b"p\xed\xa0\x80", b"-p\xed\xa0\x80"),
        ] {
            let option = recipe.oo_property_option_name(original).unwrap();
            assert_eq!(option.original(), original);
            assert_eq!(option.selected(), expected);
            assert_eq!(option.purpose(), NativeNamePurpose::OoPropertyOption);
            assert_eq!(
                recipe.oo_method_input(original).unwrap().selected(),
                original
            );
        }
    }
    for recipe in [
        NativeNameProtocol::C(TclVersion::V8_4),
        NativeNameProtocol::C(TclVersion::V8_5),
        NativeNameProtocol::C(TclVersion::V8_6),
        NativeNameProtocol::Jim084,
    ] {
        assert!(recipe.oo_property_option_name(b"p").is_err());
    }
}

#[test]
fn jim_two_word_factory_publication_uses_original_units_and_existing_publication_recipe() {
    // naming.source.original-class-reference
    // docs/design/analysis/name-resolution-proofs/source-original-class-reference.md
    // Extent cases are pure owner coverage, not additional native probe claims.
    let jim = NativeNameProtocol::Jim084;
    for (class, member, expected) in [
        (b"C".as_slice(), b"get".as_slice(), b"C get".as_slice()),
        (b"::C", b"method", b"C method"),
        (b"C extra", b"get", b"C extra get"),
        (b"C\0tail", b"get", b"C\0tail get"),
    ] {
        let slot = jim
            .jim_two_word_member_publication_slot(
                NativeNameContext::root(),
                class,
                member,
                NativeNamePurpose::CommandPublication,
            )
            .unwrap();
        assert!(slot.namespace.is_root());
        assert_eq!(slot.simple.as_bytes(), expected);
    }
    assert_eq!(
        jim.jim_two_word_member_publication_slot(
            NativeNameContext::new(&ByteNamespacePath::root()),
            b"C",
            b"get",
            NativeNamePurpose::CommandPublication,
        )
        .unwrap_err(),
        NameProjectionUnavailable::MissingJimNamespaceObject
    );
    for version in [
        TclVersion::V8_4,
        TclVersion::V8_5,
        TclVersion::V8_6,
        TclVersion::V9_0,
        TclVersion::V9_1,
    ] {
        assert_eq!(
            NativeNameProtocol::C(version)
                .jim_two_word_member_publication_slot(
                    NativeNameContext::root(),
                    b"C",
                    b"get",
                    NativeNamePurpose::CommandPublication
                )
                .unwrap_err(),
            NameProjectionUnavailable::PurposeNotModelled
        );
    }
    let alias = jim
        .jim_two_word_member_publication_slot(
            NativeNameContext::root(),
            b"::C",
            b"constructor",
            NativeNamePurpose::AliasPublication,
        )
        .unwrap();
    assert_eq!(alias.simple.as_bytes(), b"C constructor");
    assert_eq!(
        jim.jim_two_word_member_publication_slot(
            NativeNameContext::root(),
            b"C",
            b"get",
            NativeNamePurpose::CommandLookup
        )
        .unwrap_err(),
        NameProjectionUnavailable::PurposeNotModelled
    );
}

#[test]
fn child_alias_publication_has_its_own_counted_input_and_c_string_registration() {
    // naming.alias.original-child-publication-and-inventory
    // docs/design/analysis/name-resolution-proofs/alias-original-child-publication-and-inventory.md
    // Native298 separately measures child invocation/inventories. This pure
    // projection grants no installed command, original header or parent realm.
    let context_path = ByteNamespacePath::from_segments([b"N".as_slice()]);
    let context = NativeNameContext::new(&context_path);
    for protocol in TclVersion::ALL
        .map(NativeNameProtocol::C)
        .into_iter()
        .chain([NativeNameProtocol::Jim084])
    {
        for input in [
            b"::::r2286".as_slice(),
            b"r2286\0_tail",
            b"r2286_tail\0",
            b"r2286_\xc3\xa9",
            b"r2286_\xff",
        ] {
            let selected = protocol
                .child_alias_publication_input(context, input)
                .unwrap();
            assert_eq!(selected.original(), input);
            assert_eq!(selected.selected(), c_string_extent(input));
            assert_eq!(selected.purpose(), NativeNamePurpose::ChildAliasPublication);
            assert_eq!(
                selected.alias_registration_report_bytes(),
                Some(if protocol.is_jim084() {
                    c_string_extent(input)
                } else {
                    input
                })
            );
            let slot = protocol
                .child_alias_publication_projection(context, input)
                .unwrap();
            assert!(slot.slot().namespace.is_root());
            if protocol.is_jim084() {
                let table =
                    super::super::NativeJimCommandTableKey::from_projection(&selected).unwrap();
                assert_eq!(table.report_bytes(), c_string_extent(input));
                assert_eq!(
                    table.comparison_bytes(),
                    strip_jim_root(c_string_extent(input))
                );
                assert_eq!(slot.slot().simple.as_bytes(), table.comparison_bytes());
                assert_eq!(
                    protocol
                        .alias_publication_input(context, input)
                        .unwrap()
                        .selected(),
                    input
                );
            }
        }
    }
}
