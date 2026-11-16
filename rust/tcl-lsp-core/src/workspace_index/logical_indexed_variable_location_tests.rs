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
fn logical_namespace_variable_locations_retain_the_actual_recipe() {
    let source = "set ::shared 1";
    let caller = "puts $::shared";
    let declaration_analysis = Analyser::new().analyse(source, "tcl");
    let caller_analysis = Analyser::new().analyse(caller, "tcl");
    let image = tcl_lexer::SourceImage::document(caller);
    let occurrence = caller_analysis
        .original_variable_symbol_in_source(
            &image,
            caller_analysis.body_lexer_config.unwrap(),
            u32::try_from(caller.find("::shared").unwrap()).unwrap(),
        )
        .expect("original Logical namespace variable root");
    assert!(
        occurrence
            .matches_logical_source_view(&image, caller_analysis.resolved_input.as_ref().unwrap())
    );
    let mut index = WorkspaceIndex::from_documents([
        ("file:///caller.tcl", &caller_analysis),
        ("file:///variable.tcl", &declaration_analysis),
    ]);
    let request = index
        .logical_variable_location_for_occurrence(
            "file:///caller.tcl",
            caller,
            &caller_analysis,
            occurrence,
        )
        .unwrap();
    let targets = index.logical_variable_source_locations(&request);
    let target = targets
        .iter()
        .find(|target| target.uri() == "file:///variable.tcl" && target.is_declaration())
        .unwrap();
    let span = target
        .validated_span(source, &declaration_analysis)
        .unwrap();
    assert_eq!(source.get(span.as_range()), Some("::shared"));
    assert!(index.logical_variable_location_is_retained(&request));
    assert!(index.logical_variable_location_is_retained(target));
    assert!(
        target
            .validated_span("set ::shared 2", &declaration_analysis)
            .is_none()
    );
    let other = Analyser::new().analyse("set ::other 2", "tcl");
    index.add_document("file:///variable.tcl", &other);
    assert!(index.logical_variable_location_is_retained(target));
    index.replace_document(
        "file:///variable.tcl",
        &Analyser::new().analyse(source, "tcl"),
    );
    assert!(!index.logical_variable_location_is_retained(target));
    index.remove_document("file:///caller.tcl");
    assert!(!index.logical_variable_location_is_retained(&request));
}
