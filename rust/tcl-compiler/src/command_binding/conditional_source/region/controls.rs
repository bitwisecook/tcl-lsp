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
use crate::registry_invocation::source_structure::original_project_source_load_at;

fn source_load(
    source: &str,
    analysis: &crate::analyser::AnalysisResult,
    text: &str,
) -> crate::registry_invocation::source_structure::OriginalProjectSourceLoad {
    original_project_source_load_at(
        source,
        analysis,
        u32::try_from(source.find(text).unwrap()).unwrap(),
    )
    .unwrap()
}

#[test]
fn source_cut_body_projection_refuses_equal_but_foreign_analysis_owner() {
    let source = "source child.tcl; proc run {} {return VALUE}";
    let parent = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
    let load = source_load(source, &parent, "source child");
    let point = load.source_point_entry(&parent).unwrap();
    let child = crate::analyser::Analyser::new()
        .analyse_with_source_point_entry("proc helper {} {return VALUE}", "tcl8.6", &point)
        .unwrap();
    let regions = load
        .conditional_continuation(&parent, &child)
        .unwrap()
        .source_regions(&parent, &child)
        .unwrap();
    let suffix = crate::analyser::Analyser::new()
        .analyse_source_region(&regions.regions()[0], &parent, &child)
        .unwrap();
    let outcome = regions.normal_outcome().unwrap();
    let survey = outcome
        .single_source_cut_survey(&parent, &[&child, &suffix])
        .unwrap();
    let projection = survey.conditional_body_projection(&child).unwrap();
    let owner = child
        .retained_command_realm()
        .unwrap()
        .source_bindings_ref()
        .original_analysis_owner()
        .unwrap();
    assert!(projection.matches_analysis_owner(owner));
    let equal_foreign = Arc::new((**owner).clone());
    assert!(Arc::ptr_eq(&owner.source, &equal_foreign.source));
    assert_eq!(owner.metadata, equal_foreign.metadata);
    assert_eq!(owner.config, equal_foreign.config);
    assert!(!projection.matches_analysis_owner(&equal_foreign));
    let foreign_projection = OriginalSourceCutBodyEntryProjection {
        owner: equal_foreign,
        ..projection
    };
    assert!(!foreign_projection.matches_analysis_owner(owner));
}

#[test]
fn completed_conditional_world_keeps_exact_prefix_owner_and_opaque_refusal() {
    let source = "source child.tcl; proc run {} {return [helper 21]}";
    let parent = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
    let load = source_load(source, &parent, "source child");
    let point = load.source_point_entry(&parent).unwrap();
    let child = crate::analyser::Analyser::new()
        .analyse_with_source_point_entry("proc helper {value} {return $value}", "tcl8.6", &point)
        .unwrap();
    let regions = load
        .conditional_continuation(&parent, &child)
        .unwrap()
        .source_regions(&parent, &child)
        .unwrap();
    let outcome = regions.normal_outcome().unwrap();
    assert!(parent.original_completed_command_world().is_none());
    let world = outcome.original_completed_command_world(&parent).unwrap();
    assert_eq!(
        world.source_image(),
        &tcl_lexer::SourceImage::document(source)
    );
    assert!(world.declarations().count() >= 2);
    let foreign = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
    assert!(outcome.original_completed_command_world(&foreign).is_none());
    let mut detached = parent.clone();
    detached.resolved_input = None;
    assert!(
        outcome
            .original_completed_command_world(&detached)
            .is_none()
    );
    for suffix in [
        "puts opaque",
        "source missing.tcl",
        "proc recurse {} {recurse}; recurse",
    ] {
        let source = format!("source child.tcl; {suffix}");
        let parent = crate::analyser::Analyser::new().analyse(&source, "tcl8.6");
        let load = source_load(&source, &parent, "source child");
        let point = load.source_point_entry(&parent).unwrap();
        let child = crate::analyser::Analyser::new()
            .analyse_with_source_point_entry("", "tcl8.6", &point)
            .unwrap();
        let regions = load
            .conditional_continuation(&parent, &child)
            .unwrap()
            .source_regions(&parent, &child)
            .unwrap();
        assert!(regions.normal_outcome().is_none(), "{suffix}");
    }
}

fn assert_region_has_no_native_completion(actual: &crate::command_binding::SourceCommandBindings) {
    assert!(
        actual
            .conditional_source
            .root_outcomes
            .as_ref()
            .unwrap()
            .native_normal_completion
            .is_none()
    );
    assert!(actual.final_state.baseline.native_entry.is_none());
    assert_eq!(
        actual.final_state.baseline.native_compilation,
        tcl_registry::native_compilation::NativeCompilationContext::default()
    );
    assert!(actual.original_completed_command_world().is_none());
}

fn assert_region_view_has_no_whole_document_authority(
    view: &crate::analyser::AnalysisResult,
    source: &str,
    registry: &tcl_registry::CommandRegistry,
) {
    assert!(crate::source_graph::current_analysis(source, view).is_none());
    assert!(
        view.original_optimiser_source_unit(source, registry, None)
            .is_none()
    );
    assert!(
        view.original_source_analysis_entry(source, registry, None)
            .is_none()
    );
}

#[test]
fn suffix_view_reuses_the_same_kernel_and_refuses_whole_document_or_foreign_parent() {
    let source = "source first.tcl; set after ONE; source second.tcl";
    let parent = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
    let load = source_load(source, &parent, "source first");
    let point = load.source_point_entry(&parent).unwrap();
    let child = crate::analyser::Analyser::new()
        .analyse_with_source_point_entry("", "tcl8.6", &point)
        .unwrap();
    let continuation = load.conditional_continuation(&parent, &child).unwrap();
    let regions = continuation.source_regions(&parent, &child).unwrap();
    assert_eq!(regions.regions().len(), 1);
    assert!(!regions.complete()); // second source remains unavailable
    assert!(regions.normal_outcome().is_none());
    assert_eq!(
        continuation.source_regions(&parent, &child),
        Some(regions.clone())
    );
    let retained_region = regions.regions()[0].clone();
    drop(regions);
    let regions = continuation.source_regions(&parent, &child).unwrap();
    assert_eq!(regions.regions().first(), Some(&retained_region));
    let region = &regions.regions()[0];
    assert_eq!(
        region
            .commands()
            .iter()
            .map(|command| command.texts[0].as_str())
            .collect::<Vec<_>>(),
        ["set", "source"]
    );
    let view = crate::analyser::Analyser::new()
        .analyse_source_region(region, &parent, &child)
        .unwrap();
    assert!(region.matches_region_analysis(&view));
    assert_eq!(view.original_source_region(), Some(region));
    let registry = region.input().borrowed_context_registry().commands();
    assert_region_view_has_no_whole_document_authority(&view, source, registry);
    let actual = view.retained_command_realm().unwrap().source_bindings_ref();
    assert!(Arc::ptr_eq(
        actual.original_analysis_owner().unwrap(),
        region.bindings().original_analysis_owner().unwrap()
    ));
    assert!(actual.invocation_unpositioned("set").unknown);
    let mut actual_tokens = crate::ir::CommandTokens::from_segmented(
        &region.source_image().source_map(),
        region.input().lexer_config(),
        &region.commands()[0],
    );
    actual.stamp_original_tokens(&mut actual_tokens);
    let actual_binding = actual_tokens.source_binding.as_ref().unwrap();
    assert!(actual_binding.original_analysis_owner.is_some());
    assert!(
        actual_binding
            .original_source_argument_syntax(
                &actual_tokens,
                region.input(),
                region.input().borrowed_context_registry().commands(),
            )
            .is_none()
    );
    assert_region_has_no_native_completion(actual);
    assert!(actual.projection_at_source("set", 0).unknown);
    assert!(!region.owns_original_command_at(&view, 0));
    assert!(region.owns_original_command_at(
        &view,
        u32::try_from(source.find("set after").unwrap()).unwrap()
    ));
    assert!(
        !region
            .owns_original_command_at(&view, u32::try_from(source.find("ONE").unwrap()).unwrap())
    );
    assert!(
        source_load(source, &view, "source second")
            .source_point_entry(&view)
            .is_some()
    );
    let foreign_parent = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
    assert!(
        crate::analyser::Analyser::new()
            .analyse_source_region(region, &foreign_parent, &child)
            .is_none()
    );
    let mut unavailable = view.clone();
    unavailable.body_lexer_config = None;
    assert!(!region.matches_region_analysis(&unavailable));
    assert!(
        region
            .0
            .point
            .source_analysis_entry(registry, region.input())
            .is_none()
    );
}

#[test]
fn ordered_namespace_suffixes_restore_the_actual_inner_frame_before_outer_walk() {
    let source = "namespace eval app {source child.tcl; set inner INSIDE}; set outer OUTSIDE";
    let parent = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
    let load = source_load(source, &parent, "source child");
    let point = load.source_point_entry(&parent).unwrap();
    let child = crate::analyser::Analyser::new()
        .analyse_with_source_point_entry("", "tcl8.6", &point)
        .unwrap();
    let regions = load
        .conditional_continuation(&parent, &child)
        .unwrap()
        .source_regions(&parent, &child)
        .unwrap();
    assert_eq!(regions.regions().len(), 2);
    assert!(regions.complete());
    let inner = &regions.regions()[0];
    let outer = &regions.regions()[1];
    assert_eq!(
        inner
            .0
            .entry
            .frame()
            .context
            .namespace_key
            .display()
            .as_deref(),
        Some("::app")
    );
    assert_eq!(
        outer
            .0
            .entry
            .frame()
            .context
            .namespace_key
            .display()
            .as_deref(),
        Some("::")
    );
    assert_eq!(outer.0.entry.0.predecessor.as_ref(), Some(inner));
    assert_eq!(
        outer
            .0
            .entry
            .0
            .incoming
            .normal
            .as_ref()
            .unwrap()
            .variable_frame,
        outer.0.entry.frame().context.frame
    );
    assert!(
        regions
            .normal_outcome()
            .unwrap()
            .matches_prefix_analysis(&parent)
    );
    let inner_view = crate::analyser::Analyser::new()
        .analyse_source_region(inner, &parent, &child)
        .unwrap();
    let outer_view = crate::analyser::Analyser::new()
        .analyse_source_region(outer, &parent, &child)
        .unwrap();
    assert!(inner.matches_region_analysis(&inner_view));
    assert!(!inner.matches_region_analysis(&outer_view));
    let mut reused = crate::analyser::Analyser::new();
    let same_inner = reused
        .analyse_source_region(inner, &parent, &child)
        .unwrap();
    assert!(inner.matches_region_analysis(&same_inner));
    let fresh = reused.analyse("proc fresh {} {}", "tcl8.6");
    assert!(fresh.original_source_region().is_none());
    assert!(fresh.all_procs.contains_key("::fresh"));
    assert!(!fresh.all_procs.contains_key("::app::fresh"));
}

#[test]
fn deep_return_uses_the_completed_chain_without_renewing_the_opaque_child_prefix() {
    let source = "source child.tcl; set resumed YES";
    let child_source = "source leaf.tcl";
    let parent = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
    let load = source_load(source, &parent, "source child");
    let point = load.source_point_entry(&parent).unwrap();
    let child = crate::analyser::Analyser::new()
        .analyse_with_source_point_entry(child_source, "tcl8.6", &point)
        .unwrap();
    assert!(load.conditional_continuation(&parent, &child).is_none());
    let inner_load = source_load(child_source, &child, "source leaf");
    let leaf_point = inner_load.source_point_entry(&child).unwrap();
    let leaf = crate::analyser::Analyser::new()
        .analyse_with_source_point_entry("", "tcl8.6", &leaf_point)
        .unwrap();
    let inner = inner_load
        .conditional_continuation(&child, &leaf)
        .unwrap()
        .source_regions(&child, &leaf)
        .unwrap();
    let outcome = inner.normal_outcome().unwrap();
    assert!(outcome.matches_prefix_analysis(&child));
    let resumed = load
        .conditional_continuation_with_outcome(&parent, &child, &outcome)
        .unwrap();
    let complete = resumed.source_regions(&parent, &child).unwrap();
    assert!(complete.complete());
    assert!(
        complete
            .normal_outcome()
            .unwrap()
            .matches_prefix_analysis(&parent)
    );
    // Ordinary child root result stays unavailable, even after a complete chain.
    assert!(load.conditional_continuation(&parent, &child).is_none());
    let foreign_child = crate::analyser::Analyser::new()
        .analyse_with_source_point_entry(child_source, "tcl8.6", &point)
        .unwrap();
    assert!(!outcome.matches_prefix_analysis(&foreign_child));
    assert!(
        load.conditional_continuation_with_outcome(&parent, &foreign_child, &outcome)
            .is_none()
    );
}
