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
fn a_retained_namespace_request_cannot_select_an_unrelated_genuine_symbol() {
    let source = "namespace exists ::A; namespace exists ::B";
    let target = "namespace eval ::B {}; namespace eval ::B::C {}";
    let analysis = Analyser::new().analyse(source, "tcl");
    let target_analysis = Analyser::new().analyse(target, "tcl");
    let a = u32::try_from(source.find("::A").unwrap()).unwrap();
    let b = u32::try_from(source.find("::B").unwrap()).unwrap();
    let std::ops::ControlFlow::Break(Some(symbol_b)) =
        crate::namespace_symbol::select_at_offset(source, &analysis, b)
    else {
        panic!("genuine other namespace operand");
    };
    let index = WorkspaceIndex::from_documents([
        ("file:///caller.tcl", &analysis),
        ("file:///target.tcl", &target_analysis),
    ]);
    let request_a = index
        .logical_namespace_location_at("file:///caller.tcl", source, &analysis, a)
        .unwrap();
    assert!(
        index
            .logical_namespace_source_locations(&request_a, &symbol_b)
            .is_empty()
    );
    assert!(
        index
            .logical_namespace_parent_locations(&request_a, &symbol_b)
            .is_empty()
    );
    let request_b = index
        .logical_namespace_location_at("file:///caller.tcl", source, &analysis, b)
        .unwrap();
    assert!(
        !index
            .logical_namespace_source_locations(&request_b, &symbol_b)
            .is_empty()
    );
    assert!(
        !index
            .logical_namespace_parent_locations(&request_b, &symbol_b)
            .is_empty()
    );
}
