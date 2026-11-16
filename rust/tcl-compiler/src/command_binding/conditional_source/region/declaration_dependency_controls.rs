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

#[test]
fn region_call_joins_only_the_actual_completed_child_declaration_owner() {
    use crate::analyser::Analyser;
    use crate::registry_invocation::source_structure::original_project_source_load_at;
    let source = "source child.tcl; helper";
    let parent = Analyser::new().analyse(source, "tcl8.6");
    let load = original_project_source_load_at(source, &parent, 0).unwrap();
    let point = load.source_point_entry(&parent).unwrap();
    let child_source = "proc helper {} {}";
    let child = Analyser::new()
        .analyse_with_source_point_entry(child_source, "tcl8.6", &point)
        .unwrap();
    let continuation = load.conditional_continuation(&parent, &child).unwrap();
    let regions = continuation.source_regions(&parent, &child).unwrap();
    assert_eq!(regions.regions().len(), 1);
    let region = &regions.regions()[0];
    let caller = Analyser::new()
        .analyse_source_region(region, &parent, &child)
        .unwrap();
    assert!(region.matches_region_analysis(&caller));
    let offset = u32::try_from(source.rfind("helper").unwrap()).unwrap();
    let calls = region.original_procedure_calls(&caller).unwrap();
    let call = calls
        .iter()
        .find(|call| call.original_words()[0].span().start() == offset)
        .unwrap();
    assert!(
        crate::registry_invocation::source_structure::source_original_procedure_call_at(
            source, &caller, offset
        )
        .is_none()
    );
    let declaration = call
        .declaration_in_region_dependency(&caller, &child)
        .unwrap();
    assert_eq!(declaration.metadata().name, "helper");
    assert!(call.declaration_in_parent(&caller, &child).is_none());
    assert!(call.declaration(&caller).is_none());

    let foreign = Analyser::new()
        .analyse_with_source_point_entry(child_source, "tcl8.6", &point)
        .unwrap();
    assert!(point.matches_child_analysis(&foreign));
    assert!(!continuation.matches_child_analysis(&foreign));
    assert!(
        call.declaration_in_region_dependency(&caller, &foreign)
            .is_none()
    );
    assert!(
        call.declaration_in_region_dependency(&caller, &parent)
            .is_none()
    );
    let mut unavailable = caller.clone();
    unavailable.analysis_context_unavailable = Some(tcl_registry::model::OverlayMiss {
        environment: "tcl8.6".to_owned(),
        overlay: 1,
    });
    assert!(
        call.declaration_in_region_dependency(&unavailable, &child)
            .is_none()
    );
    let mut wrong_config = caller.clone();
    wrong_config.body_lexer_config = None;
    assert!(
        call.declaration_in_region_dependency(&wrong_config, &child)
            .is_none()
    );
    let mut unavailable_child = child.clone();
    unavailable_child.body_lexer_config = None;
    assert!(
        call.declaration_in_region_dependency(&caller, &unavailable_child)
            .is_none()
    );
}
