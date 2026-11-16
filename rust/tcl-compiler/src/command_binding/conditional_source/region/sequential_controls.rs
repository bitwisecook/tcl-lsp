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

use crate::registry_invocation::source_structure::original_project_source_load_at;

#[test]
fn sequential_tail_returns_to_the_same_original_prefix_without_reanalysis() {
    let source = "source first.tcl; source second.tcl; set after DONE";
    let parent = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
    let first = original_project_source_load_at(source, &parent, 0).unwrap();
    let point = first.source_point_entry(&parent).unwrap();
    let child = crate::analyser::Analyser::new()
        .analyse_with_source_point_entry("", "tcl8.6", &point)
        .unwrap();
    let pending = first
        .conditional_continuation(&parent, &child)
        .unwrap()
        .source_regions(&parent, &child)
        .unwrap();
    assert!(!pending.complete());
    let region = &pending.regions()[0];
    let region_view = crate::analyser::Analyser::new()
        .analyse_source_region(region, &parent, &child)
        .unwrap();
    let offset = u32::try_from(source.find("source second").unwrap()).unwrap();
    let second = original_project_source_load_at(source, &region_view, offset).unwrap();
    let second_point = second.source_point_entry(&region_view).unwrap();
    let second_child = crate::analyser::Analyser::new()
        .analyse_with_source_point_entry("", "tcl8.6", &second_point)
        .unwrap();
    let resumed = second
        .conditional_continuation(&region_view, &second_child)
        .unwrap()
        .source_regions(&region_view, &second_child)
        .unwrap();
    assert!(resumed.complete());
    let completed = resumed.normal_outcome().unwrap();
    assert!(completed.matches_prefix_analysis(&region_view));
    assert!(!completed.matches_prefix_analysis(&parent));
    let returned = region
        .return_outcome(&parent, &child, &region_view, &completed)
        .unwrap();
    assert!(returned.matches_prefix_analysis(&parent));
    assert_eq!(
        region.return_outcome(&parent, &child, &region_view, &completed),
        Some(returned)
    );
    let foreign = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
    assert!(
        region
            .return_outcome(&foreign, &child, &region_view, &completed)
            .is_none()
    );
    assert!(
        first
            .conditional_continuation(&parent, &child)
            .unwrap()
            .source_regions(&parent, &child)
            .unwrap()
            .normal_outcome()
            .is_none()
    );
}

#[test]
fn twelve_literal_source_hops_return_only_through_the_original_live_chain() {
    let mut prefixes = Vec::new();
    let mut source = "source hop0.tcl".to_owned();
    let mut analysis = crate::analyser::Analyser::new().analyse(&source, "tcl8.6");
    for index in 0..12 {
        let load = original_project_source_load_at(&source, &analysis, 0).unwrap();
        let point = load.source_point_entry(&analysis).unwrap();
        let next = if index == 11 {
            String::new()
        } else {
            format!("source hop{}.tcl", index + 1)
        };
        let mut analyser = crate::analyser::Analyser::new();
        assert!(point.configure_analyser(&analysis, &mut analyser));
        let child = analyser
            .analyse_with_source_point_entry(&next, "tcl8.6", &point)
            .unwrap();
        prefixes.push((source, analysis, load));
        source = next;
        analysis = child;
    }
    let mut completed = None;
    for (_, prefix, load) in prefixes.iter().rev() {
        let continuation = match completed.as_ref() {
            Some(outcome) => load.conditional_continuation_with_outcome(prefix, &analysis, outcome),
            None => load.conditional_continuation(prefix, &analysis),
        }
        .unwrap();
        let regions = continuation.source_regions(prefix, &analysis).unwrap();
        assert!(regions.complete());
        completed = regions.normal_outcome();
        assert!(completed.as_ref().unwrap().matches_prefix_analysis(prefix));
        analysis = prefix.clone();
    }
    let (_, root, _) = &prefixes[0];
    assert!(completed.unwrap().matches_prefix_analysis(root));
    assert!(root.original_source_region().is_none());
}
