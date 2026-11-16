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
fn original_self_selector_keeps_member_side_and_saved_source_owner() {
    let caller = "oo::class create Child {superclass Base; method invoke {} {my hit}}";
    let provider = "oo::class create Base {method hit {} {return instance}; self method hit {} {return class}}";
    let analysis = Analyser::new().analyse(caller, "tcl");
    let target_analysis = Analyser::new().analyse(provider, "tcl");
    let mut index = WorkspaceIndex::from_documents([
        ("file:///caller.tcl", &analysis),
        ("file:///provider.tcl", &target_analysis),
    ]);
    let cursor = u32::try_from(caller.rfind("hit").unwrap()).unwrap();
    let selected = index
        .logical_self_member_targets_at("file:///caller.tcl", caller, &analysis, cursor)
        .unwrap();
    let target = selected
        .targets
        .iter()
        .find(|target| target.uri() == "file:///provider.tcl")
        .unwrap();
    assert_eq!(
        target.source_member().side(),
        tcl_compiler::analyser::types::MemberSide::Instance
    );
    assert_eq!(
        target
            .validated_span(provider, &target_analysis)
            .unwrap()
            .start() as usize,
        provider.find("hit").unwrap()
    );
    assert!(index.logical_member_location_is_retained(&selected.caller));
    assert!(index.logical_member_location_is_retained(target));
    index.replace_document(
        "file:///provider.tcl",
        &Analyser::new().analyse(provider, "tcl"),
    );
    assert!(!index.logical_member_location_is_retained(target));
    assert!(
        index
            .logical_self_member_targets_at("file:///caller.tcl", caller, &analysis, cursor - 3)
            .is_none()
    );
    assert!(index.logical_member_location_is_retained(&selected.caller));
    index.replace_document(
        "file:///caller.tcl",
        &Analyser::new().analyse(caller, "tcl"),
    );
    assert!(!index.logical_member_location_is_retained(&selected.caller));
}
