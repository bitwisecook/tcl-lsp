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

use super::*;
use tcl_compiler::analyser::Analyser;

#[test]
fn namespace_locations_keep_source_issuer_and_represented_parent_geometry() {
    let caller = "namespace exists ::A";
    let target = "namespace eval ::A::B {}";
    let caller_analysis = Analyser::new().analyse(caller, "tcl");
    let target_analysis = Analyser::new().analyse(target, "tcl");
    let offset = u32::try_from(caller.find("::A").unwrap()).unwrap();
    let std::ops::ControlFlow::Break(Some(symbol)) =
        crate::namespace_symbol::select_at_offset(caller, &caller_analysis, offset)
    else {
        panic!("genuine Logical namespace operand");
    };
    let mut index = WorkspaceIndex::from_documents([
        ("file:///caller.tcl", &caller_analysis),
        ("file:///target.tcl", &target_analysis),
    ]);
    let request = index
        .logical_namespace_location_at("file:///caller.tcl", caller, &caller_analysis, offset)
        .unwrap();
    assert!(index.logical_namespace_location_is_retained(&request));
    let parents = index.logical_namespace_parent_locations(&request, &symbol);
    let parent = parents
        .iter()
        .find(|location| location.uri() == "file:///target.tcl")
        .unwrap();
    let span = parent.validated_span(target, &target_analysis).unwrap();
    assert_eq!(target.get(span.as_range()), Some("::A"));
    assert!(index.logical_namespace_location_is_retained(parent));
    assert!(
        parent
            .validated_span("namespace eval ::A::C {}", &target_analysis)
            .is_none()
    );
    let other = Analyser::new().analyse("namespace eval ::X {}", "tcl");
    index.add_document("file:///target.tcl", &other);
    assert!(index.logical_namespace_location_is_retained(parent));
    index.replace_document("file:///target.tcl", &other);
    assert!(!index.logical_namespace_location_is_retained(parent));
    index.remove_document("file:///caller.tcl");
    assert!(!index.logical_namespace_location_is_retained(&request));
}
