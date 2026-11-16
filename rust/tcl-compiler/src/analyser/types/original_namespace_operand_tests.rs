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

use crate::analyser::Analyser;

#[test]
fn original_namespace_names_keep_surrogate_and_nul_units_in_their_receipts() {
    let source =
        r"namespace eval n\uD800 {}; namespace eval n\uD801 {}; namespace eval n\u0000tail {}";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let declarations = analysis
        .namespace_refs
        .iter()
        .filter(|reference| reference.declares)
        .collect::<Vec<_>>();
    assert_eq!(declarations.len(), 3);
    let names = declarations
        .iter()
        .map(|reference| {
            reference
                .original_name_input
                .as_ref()
                .unwrap()
                .bytes()
                .to_vec()
        })
        .collect::<Vec<_>>();
    assert_eq!(names[0], b"n\xed\xa0\x80");
    assert_eq!(names[1], b"n\xed\xa0\x81");
    assert_eq!(names[2], b"n\xc0\x80tail");
    assert_ne!(
        declarations[0].source_namespace,
        declarations[1].source_namespace
    );
    assert!(
        declarations
            .iter()
            .all(|reference| reference.source_namespace.is_some())
    );
}

#[test]
fn computed_namespace_values_have_identity_without_word_edit_geometry() {
    let source = r"set target ::n; namespace eval $target {}";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let offset = u32::try_from(source.find("$target").unwrap()).unwrap();
    let references = analysis
        .namespace_refs
        .iter()
        .filter(|reference| reference.span.start() == offset)
        .collect::<Vec<_>>();
    assert_eq!(references.len(), 1, "one authentic namespace occurrence");
    let reference = references[0];
    let input = reference
        .original_name_input
        .as_ref()
        .expect("retained evaluated bytes");
    assert_eq!(input.bytes(), b"::n");
    assert!(input.original_word_key().is_none());
    assert!(reference.source_namespace.is_some());
    assert!(
        reference
            .original_extent_span(source, reference.source_namespace.as_ref().unwrap(), false)
            .is_none()
    );
    let child = analysis
        .global_scope
        .children
        .iter()
        .find(|scope| scope.name_span == Some(reference.span))
        .unwrap();
    assert_eq!(child.naming_scope, reference.source_namespace);
}

#[test]
fn namespace_path_values_retain_parent_list_lineage_without_word_geometry() {
    let source = r"namespace eval n {}; namespace path {::n ::n\u0000tail}";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let values = analysis
        .namespace_refs
        .iter()
        .filter(|reference| !reference.declares)
        .collect::<Vec<_>>();
    assert_eq!(values.len(), 2);
    let second = values[1].original_name_input.as_ref().unwrap();
    assert_eq!(second.bytes(), b"::n\xc0\x80tail");
    assert!(second.original_word_key().is_none());
    assert!(
        values
            .iter()
            .all(|reference| reference.source_namespace.is_some())
    );
}
