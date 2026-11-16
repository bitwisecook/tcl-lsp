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
fn logical_target_cards_retain_actual_call_and_declaration_allocations() {
    let caller = "helper";
    let source = "proc helper {} {}";
    let call_analysis = Analyser::new().analyse(caller, "tcl");
    let declaration_analysis = Analyser::new().analyse(source, "tcl");
    let caller_uri = "file:///caller.tcl";
    let target_uri = "file:///target.tcl";
    let mut index = WorkspaceIndex::from_documents([
        (caller_uri, &call_analysis),
        (target_uri, &declaration_analysis),
    ]);
    let call = index
        .logical_command_call_at(caller_uri, caller, &call_analysis, 0)
        .unwrap();
    let targets = index.logical_declaration_targets_for_call(&call).unwrap();
    let target = targets
        .iter()
        .find(|target| target.uri() == target_uri)
        .unwrap();
    assert!(index.logical_declaration_location_is_retained(target));
    let span = target
        .validated_span(source, &declaration_analysis)
        .unwrap();
    assert_eq!(source.get(span.as_range()), Some("helper"));
    assert!(
        target
            .validated_span("proc changed {} {}", &declaration_analysis)
            .is_none()
    );
    let other = Analyser::new().analyse("proc other {} {}", "tcl");
    index.add_document(target_uri, &other);
    assert!(index.logical_declaration_location_is_retained(target));
    index.replace_document(target_uri, &declaration_analysis);
    // Reusing the same analysis preserves its actual header allocation.
    assert!(index.logical_declaration_location_is_retained(target));
    let reanalysed = Analyser::new().analyse(source, "tcl");
    index.replace_document(target_uri, &reanalysed);
    assert!(!index.logical_declaration_location_is_retained(target));
    index.replace_document(caller_uri, &Analyser::new().analyse(caller, "tcl"));
    assert!(!call.is_retained_in(&index));
    assert!(index.logical_declaration_targets_for_call(&call).is_none());
}

#[test]
fn logical_class_cards_are_source_factories_not_equal_report_names() {
    let caller = "C";
    let source = "oo::class create C {}";
    let call_analysis = Analyser::new().analyse(caller, "tcl");
    let declaration_analysis = Analyser::new().analyse(source, "tcl");
    let mut index = WorkspaceIndex::from_documents([
        ("file:///caller.tcl", &call_analysis),
        ("file:///class.tcl", &declaration_analysis),
    ]);
    let call = index
        .logical_command_call_at("file:///caller.tcl", caller, &call_analysis, 0)
        .unwrap();
    let targets = index.logical_declaration_targets_for_call(&call).unwrap();
    let target = targets
        .iter()
        .find(|target| target.uri() == "file:///class.tcl")
        .unwrap();
    let span = target
        .validated_span(source, &declaration_analysis)
        .unwrap();
    assert_eq!(source.get(span.as_range()), Some("C"));
    assert!(index.logical_declaration_location_is_retained(target));
    index.remove_document("file:///class.tcl");
    assert!(!index.logical_declaration_location_is_retained(target));
}
