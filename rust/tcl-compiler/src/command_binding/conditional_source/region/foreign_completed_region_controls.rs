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
use crate::analyser::{Analyser, AnalysisResult};
use crate::registry_invocation::source_structure::{
    OriginalProjectSourceLoad, original_project_source_load_at,
};

fn completed_tail(
    source: &str,
    parent: &AnalysisResult,
    first: &OriginalProjectSourceLoad,
    child: &AnalysisResult,
) -> (
    OriginalSourceConditionalRegion,
    AnalysisResult,
    OriginalSourceConditionalOutcome,
) {
    let pending = first
        .conditional_continuation(parent, child)
        .unwrap()
        .source_regions(parent, child)
        .unwrap();
    assert!(!pending.complete());
    assert_eq!(pending.regions().len(), 1);
    let region = pending.regions()[0].clone();
    let view = Analyser::new()
        .analyse_source_region(&region, parent, child)
        .unwrap();
    assert!(region.matches_region_analysis(&view));
    let offset = u32::try_from(source.find("source second").unwrap()).unwrap();
    let second = original_project_source_load_at(source, &view, offset).unwrap();
    let point = second.source_point_entry(&view).unwrap();
    let second_child = Analyser::new()
        .analyse_with_source_point_entry("", "tcl8.6", &point)
        .unwrap();
    let complete = second
        .conditional_continuation(&view, &second_child)
        .unwrap()
        .source_regions(&view, &second_child)
        .unwrap();
    assert!(complete.complete());
    let outcome = complete.normal_outcome().unwrap();
    assert!(outcome.matches_prefix_analysis(&view));
    (region, view, outcome)
}

#[test]
fn return_outcome_refuses_a_byte_equal_foreign_completed_region_owner() {
    let source = "source first.tcl; source second.tcl; set after DONE";
    let parent = Analyser::new().analyse(source, "tcl8.6");
    let first = original_project_source_load_at(source, &parent, 0).unwrap();
    let point = first.source_point_entry(&parent).unwrap();
    let child = Analyser::new()
        .analyse_with_source_point_entry("", "tcl8.6", &point)
        .unwrap();
    let (region, view, completed) = completed_tail(source, &parent, &first, &child);
    let returned = region
        .return_outcome(&parent, &child, &view, &completed)
        .unwrap();
    assert!(returned.matches_prefix_analysis(&parent));

    // Keep the same parent and source point. A genuine second child analysis
    // issues an independent attachment, despite its identical source bytes.
    let other_child = Analyser::new()
        .analyse_with_source_point_entry("", "tcl8.6", &point)
        .unwrap();
    let (other_region, other_view, other_completed) =
        completed_tail(source, &parent, &first, &other_child);
    assert_ne!(region, other_region);
    assert!(!other_region.matches_region_analysis(&view));
    assert!(other_completed.matches_prefix_analysis(&other_view));
    assert!(!other_completed.matches_prefix_analysis(&view));
    assert!(
        other_region
            .return_outcome(&parent, &other_child, &other_view, &other_completed)
            .is_some()
    );
    assert!(
        region
            .return_outcome(&parent, &child, &view, &other_completed)
            .is_none()
    );
}
