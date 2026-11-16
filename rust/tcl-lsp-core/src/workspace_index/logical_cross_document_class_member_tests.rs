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
fn external_class_selector_keeps_original_header_member_and_call_allocations() {
    let source = "oo::class create C {self method ping {} {return ok}}";
    let caller = "C ping";
    let provider = Analyser::new().analyse(source, "tcl");
    let analysis = Analyser::new().analyse(caller, "tcl");
    let mut index = WorkspaceIndex::from_documents([
        ("file:///caller.tcl", &analysis),
        ("file:///provider.tcl", &provider),
    ]);
    let selected = index
        .logical_class_member_targets_at("file:///caller.tcl", caller, &analysis, 2)
        .unwrap();
    let target = selected
        .targets
        .iter()
        .find(|target| target.uri() == "file:///provider.tcl")
        .unwrap();
    let span = target.validated_span(source, &provider).unwrap();
    assert_eq!(source.get(span.as_range()), Some("ping"));
    assert!(selected.call.is_retained_in(&index));
    assert!(index.logical_member_location_is_retained(target));
    assert!(
        index
            .logical_class_member_targets_at("file:///caller.tcl", caller, &analysis, 0)
            .is_none()
    );
    assert!(
        index
            .logical_class_member_targets_at("file:///caller.tcl", "C pong", &analysis, 2)
            .is_none()
    );
    let changed = Analyser::new().analyse("oo::class create C {self method pong {} {}}", "tcl");
    index.replace_document("file:///provider.tcl", &changed);
    assert!(!index.logical_member_location_is_retained(target));
    index.remove_document("file:///caller.tcl");
    assert!(!selected.call.is_retained_in(&index));
}

#[test]
fn unexported_or_instance_members_are_not_public_class_object_cards() {
    let source =
        "oo::class create C {self method ping {} {}; self unexport ping; method pong {} {}}";
    let provider = Analyser::new().analyse(source, "tcl");
    for caller in ["C ping", "C pong"] {
        let analysis = Analyser::new().analyse(caller, "tcl");
        let index = WorkspaceIndex::from_documents([
            ("file:///caller.tcl", &analysis),
            ("file:///provider.tcl", &provider),
        ]);
        let selected = index
            .logical_class_member_targets_at("file:///caller.tcl", caller, &analysis, 2)
            .unwrap();
        assert!(selected.targets.is_empty());
    }
}
