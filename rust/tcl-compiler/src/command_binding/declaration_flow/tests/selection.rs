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

//! Source scopes and declaration selections: original parents, absent dispatch and reached owners.

use super::*;

#[test]
#[allow(clippy::too_many_lines)] // one end-to-end scenario
fn conditional_root_source_scope_uses_its_original_parent_and_input() {
    let source = "set ys {}; foreach pt {{1 2} {3 4}} {lappend ys $pt}";
    let analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
    let input = analysis.resolved_input.as_ref().unwrap();
    let registry = input.borrowed_context_registry().commands();
    let config = input.lexer_config();
    let realm = analysis.retained_command_realm().unwrap();
    let bindings = realm.source_bindings_ref();
    let image = tcl_lexer::SourceImage::document(source);
    let segments = tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config);
    let mut anchor = CommandTokens::from_segmented(&image.source_map(), config, &segments[0]);
    bindings.stamp_original_tokens(&mut anchor);
    let binding = anchor.source_binding.as_ref().unwrap();
    let inventory = binding.declaration_flow_inventory.as_ref().unwrap();
    let observation = super::super::super::declaration_layout::original_declaration_layouts(
        inventory
            .layouts
            .get(binding.invocation_site().unwrap())
            .unwrap(),
    )
    .unwrap()
    .next()
    .unwrap();
    let frame = DeclarationFlowFrame::Existing(observation.entry.as_ref().clone());
    assert!(matches!(
        frame,
        DeclarationFlowFrame::Existing(
            super::super::super::declaration_layout::OriginalDiagnosticFrameEntry::RootScript { .. }
        )
    ));
    let dialect = observation
        .snapshot
        .state
        .source_variables
        .invocation_dialect
        .unwrap();
    let offset = u32::try_from(source.find("lappend").unwrap()).unwrap();
    let segment = tcl_source::segment::segment_commands_with_offset_and_config(
        "lappend ys $pt",
        offset,
        config,
    )
    .remove(0);
    let child = CommandTokens::from_segmented(&image.source_map(), config, &segment);
    let site = CommandAllocationSite {
        source: Arc::clone(frame.origin()),
        offset,
    };
    macro_rules! builder {
        ($purpose:expr, $inventory:expr) => {
            Builder {
                inventory: $inventory,
                registry,
                config,
                dialect,
                entry: &frame,
                purpose: $purpose,
                representation_realm: None,
                representation_loops: Vec::new(),
                current_representation_body: None,
                nodes: Vec::new(),
                jobs: Vec::new(),
                conditional_overwrites: Vec::new(),
                read_sites: BTreeSet::new(),
            }
        };
    }
    assert!(
        builder!(DeclarationFlowPurpose::Reads, inventory)
            .source_read_selection(&child, &site)
            .is_none()
    );
    let root_builder = builder!(DeclarationFlowPurpose::Representation, inventory);
    let related = root_builder
        .source_read_point_scope(&child, &site)
        .expect("actual nested activation is related to the root");
    assert!(
        matches!(related, DeclarationReadPointScope::ConditionalRootRelated { ref bodies, .. }
            if !bodies.is_empty() && bodies.iter().all(|span| *span == tcl_lexer::Span::new(offset, offset + 14)))
    );
    let selected = builder!(DeclarationFlowPurpose::Representation, inventory)
        .source_read_selection(&child, &site)
        .expect("conditional original body schema");
    assert!(selected.accumulator_writes.iter().any(|name| name == "ys"));
    assert!(
        root_builder
            .representation_root_naming_scope(&child, &site, Some(&selected))
            .is_some()
    );
    assert!(
        root_builder
            .representation_root_naming_scope(&child, &site, None)
            .is_none()
    );
    let mut changed_child = child.clone();
    changed_child.argv_texts[1] = "foreign".to_owned();
    assert!(
        root_builder
            .representation_root_naming_scope(&changed_child, &site, Some(&selected))
            .is_none()
    );
    let mut changed_flow = selected.clone();
    changed_flow.effects.unknown_writes = true;
    assert!(
        root_builder
            .representation_root_naming_scope(&child, &site, Some(&changed_flow))
            .is_none()
    );
    let foreign_analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
    let foreign_realm = foreign_analysis.retained_command_realm().unwrap();
    assert!(
        binding
            .declaration_representation_flow_report(&anchor, input, registry, Some(foreign_realm))
            .is_none(),
        "equal text from a different source owner cannot warm the report cache"
    );
    let report = binding
        .declaration_representation_flow_report(&anchor, input, registry, Some(realm))
        .expect("same original analysis owns the conditional report");
    assert!(
        report.conditional_handler_sites.contains(&offset),
        "the body handler must be modelled"
    );
    assert!(
        binding
            .declaration_representation_flow_report(&anchor, input, registry, Some(realm))
            .is_some(),
        "the original source owner remains usable after a foreign request"
    );
    assert!(
        binding
            .declaration_representation_flow_report(&anchor, input, registry, Some(foreign_realm))
            .is_none(),
        "a cached original report cannot be borrowed by a foreign owner"
    );

    let mut synthetic = child.clone();
    synthetic.synthetic = Some(crate::ir::SyntheticMarker::Condition);
    assert!(
        builder!(DeclarationFlowPurpose::Representation, inventory)
            .source_read_selection(&synthetic, &site)
            .is_none()
    );
    let mut changed = child.clone();
    changed.argv_texts[1] = "foreign".to_owned();
    assert!(
        builder!(DeclarationFlowPurpose::Representation, inventory)
            .source_read_selection(&changed, &site)
            .is_none()
    );

    let mut missing = DeclarationFlowInventory::new(
        Arc::new(BTreeMap::new()),
        Arc::clone(&inventory.accesses),
        Arc::clone(&inventory.origin_accesses),
        Arc::clone(&inventory.points),
        Arc::clone(&inventory.origin_points),
    );
    missing.source_seed = inventory.source_seed.clone();
    assert!(
        builder!(DeclarationFlowPurpose::Representation, &missing)
            .source_read_selection(&child, &site)
            .is_none()
    );
    let mut corrupt = inventory.layouts.as_ref().clone();
    corrupt.get_mut(&site).unwrap()[0].words = Arc::from([]);
    let mut corrupt = DeclarationFlowInventory::new(
        Arc::new(corrupt),
        Arc::clone(&inventory.accesses),
        Arc::clone(&inventory.origin_accesses),
        Arc::clone(&inventory.points),
        Arc::clone(&inventory.origin_points),
    );
    corrupt.source_seed = inventory.source_seed.clone();
    assert!(
        builder!(DeclarationFlowPurpose::Representation, &corrupt)
            .source_read_selection(&child, &site)
            .is_none()
    );
    for axis in 0..3 {
        let mut changed = inventory.layouts.as_ref().clone();
        for row in changed.get_mut(&site).unwrap() {
            let mut state = row.snapshot.state.clone();
            let baseline = Arc::make_mut(&mut state.baseline);
            match axis {
                0 => baseline.unknown_entry = true,
                1 => baseline.registry_snapshot = None,
                _ => {
                    baseline.metadata_context =
                        crate::registry_invocation::OwnedInvocationMetadataContext::Standalone;
                }
            }
            row.snapshot = Arc::new(super::super::super::SourceLookupSnapshot::in_realm(
                state,
                row.snapshot.realm,
            ));
        }
        let mut changed = DeclarationFlowInventory::new(
            Arc::new(changed),
            Arc::clone(&inventory.accesses),
            Arc::clone(&inventory.origin_accesses),
            Arc::clone(&inventory.points),
            Arc::clone(&inventory.origin_points),
        );
        changed.source_seed = inventory.source_seed.clone();
        assert!(
            builder!(DeclarationFlowPurpose::Representation, &changed)
                .source_read_selection(&child, &site)
                .is_none(),
            "changed child baseline axis {axis}"
        );
    }
    let mut no_parent = inventory.layouts.as_ref().clone();
    let parent_site = CommandAllocationSite {
        source: Arc::clone(&site.source),
        offset: u32::try_from(source.find("foreach").unwrap()).unwrap(),
    };
    assert!(no_parent.remove(&parent_site).is_some());
    let mut no_parent = DeclarationFlowInventory::new(
        Arc::new(no_parent),
        Arc::clone(&inventory.accesses),
        Arc::clone(&inventory.origin_accesses),
        Arc::clone(&inventory.points),
        Arc::clone(&inventory.origin_points),
    );
    no_parent.source_seed = inventory.source_seed.clone();
    assert!(
        builder!(DeclarationFlowPurpose::Representation, &no_parent)
            .source_read_selection(&child, &site)
            .is_none()
    );

    for axis in 0..4 {
        let mut changed = inventory.layouts.as_ref().clone();
        for row in changed.get_mut(&site).unwrap() {
            let mut entry = row.entry.as_ref().clone();
            let super::super::super::declaration_layout::OriginalDiagnosticFrameEntry::ScriptActivation {
                source: activation,
                frame,
                namespace,
                config,
            } = &mut entry
            else {
                panic!("genuine script activation expected");
            };
            match axis {
                0 => {
                    *activation = ExecutedScriptSource::contiguous_image(
                        Arc::clone(&site.source),
                        site.source.source_image().clone(),
                        0,
                    )
                    .unwrap();
                }
                1 => {
                    *frame = crate::var_resolve::VariableExecutionFrame::Namespace(
                        "::foreign".to_owned(),
                    );
                }
                2 => *namespace = super::super::super::SourceNamespaceKey::authored("::foreign"),
                _ => config.expand_syntax = !config.expand_syntax,
            }
            row.entry = Arc::new(entry);
        }
        let mut changed = DeclarationFlowInventory::new(
            Arc::new(changed),
            Arc::clone(&inventory.accesses),
            Arc::clone(&inventory.origin_accesses),
            Arc::clone(&inventory.points),
            Arc::clone(&inventory.origin_points),
        );
        changed.source_seed = inventory.source_seed.clone();
        if axis == 0 {
            assert!(
                builder!(DeclarationFlowPurpose::Representation, &changed)
                    .source_read_point_scope(&child, &site)
                    .is_some(),
                "wider source reaches the ancestry gate"
            );
        }
        assert!(
            builder!(DeclarationFlowPurpose::Representation, &changed)
                .source_read_selection(&child, &site)
                .is_none(),
            "changed activation axis {axis}"
        );
    }
    let mut foreign = site.clone();
    foreign.source = Arc::new(super::super::super::SourceOriginId::authored_image(
        image.clone(),
    ));
    assert!(
        builder!(DeclarationFlowPurpose::Representation, inventory)
            .source_read_selection(&child, &foreign)
            .is_none()
    );
}

#[test]
fn declaration_selection_keeps_absent_dispatch_separate_from_invalid_reached_owner() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
    let source = "set anchor 1; puts $absent";
    let bindings = super::super::super::SourceCommandBindings::analyse(source, config, registry);
    let segment =
        tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config).remove(0);
    let mut tokens = CommandTokens::from_segmented(
        &tcl_lexer::SourceImage::document(source).source_map(),
        config,
        &segment,
    );
    bindings.stamp_original_tokens(&mut tokens);
    let binding = tokens.source_binding.as_ref().unwrap();
    let site = binding.invocation_site().unwrap();
    let inventory = binding.declaration_flow_inventory.as_ref().unwrap();
    let observation = super::super::super::declaration_layout::original_declaration_layouts(
        inventory.layouts.get(site).unwrap(),
    )
    .unwrap()
    .next()
    .unwrap();
    let dialect = observation
        .snapshot
        .state
        .source_variables
        .invocation_dialect
        .unwrap();
    let frame = DeclarationFlowFrame::Existing(observation.entry.as_ref().clone());
    let selection = |inventory| {
        let builder = Builder {
            inventory,
            registry,
            config,
            dialect,
            entry: &frame,
            purpose: DeclarationFlowPurpose::Reads,
            representation_realm: None,
            representation_loops: Vec::new(),
            current_representation_body: None,
            nodes: Vec::new(),
            jobs: Vec::new(),
            conditional_overwrites: Vec::new(),
            read_sites: BTreeSet::new(),
        };
        let mut raw = tokens.clone();
        raw.source_binding = None;
        builder.selection(&raw, site)
    };
    assert!(
        selection(inventory).is_some(),
        "genuine original reached owner"
    );
    let absent = DeclarationFlowInventory::new(
        Arc::clone(&inventory.layouts),
        Arc::clone(&inventory.accesses),
        Arc::clone(&inventory.origin_accesses),
        Arc::new(Vec::new()),
        Arc::new(BTreeMap::new()),
    );
    assert!(
        selection(&absent).is_some(),
        "sealed raw declaration without dispatch"
    );
    let mut invalid = inventory.points.as_ref().clone();
    assert!(
        invalid
            .iter()
            .any(|point| point.dispatch && point.offset == site.offset)
    );
    for point in &mut invalid {
        point.original_written_words = None;
        point.compiled_execution.original_words = None;
    }
    let invalid = DeclarationFlowInventory::new(
        Arc::clone(&inventory.layouts),
        Arc::clone(&inventory.accesses),
        Arc::clone(&inventory.origin_accesses),
        Arc::new(invalid),
        Arc::new(BTreeMap::new()),
    );
    assert!(
        selection(&invalid).is_none(),
        "invalid reached owner remains terminal"
    );
}

#[test]
fn source_read_scope_preserves_reached_owner_refusal_after_loop() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
    let source = "proc p {n} {set anchor 1; while {$n > 0} {set x 1; incr n -1}; puts $x}";
    let bindings = super::super::super::SourceCommandBindings::analyse(source, config, registry);
    let offset = u32::try_from(source.find("puts $x").unwrap()).unwrap();
    let segment =
        tcl_source::segment::segment_commands_with_offset_and_config("puts $x", offset, config)
            .remove(0);
    let mut tokens = CommandTokens::from_segmented(
        &tcl_lexer::SourceImage::document(source).source_map(),
        config,
        &segment,
    );
    bindings.stamp_original_tokens(&mut tokens);
    let binding = tokens.source_binding.as_ref().unwrap();
    let site = binding.invocation_site().unwrap();
    let inventory = binding.declaration_flow_inventory.as_ref().unwrap();
    let observation = super::super::super::declaration_layout::original_declaration_layouts(
        inventory.layouts.get(site).unwrap(),
    )
    .unwrap()
    .next()
    .unwrap();
    let dialect = observation
        .snapshot
        .state
        .source_variables
        .invocation_dialect
        .unwrap();
    let frame = DeclarationFlowFrame::Existing(observation.entry.as_ref().clone());
    let selected = |inventory| {
        let builder = Builder {
            inventory,
            registry,
            config,
            dialect,
            entry: &frame,
            purpose: DeclarationFlowPurpose::Reads,
            representation_realm: None,
            representation_loops: Vec::new(),
            current_representation_body: None,
            nodes: Vec::new(),
            jobs: Vec::new(),
            conditional_overwrites: Vec::new(),
            read_sites: BTreeSet::new(),
        };
        let mut raw = tokens.clone();
        raw.source_binding = None;
        builder.source_read_selection(&raw, site)
    };
    assert!(selected(inventory).is_some());
    let mut points = inventory.points.as_ref().clone();
    assert!(
        points
            .iter()
            .any(|point| point.dispatch && point.offset == site.offset)
    );
    for point in &mut points {
        point.original_written_words = None;
        point.compiled_execution.original_words = None;
    }
    let mut invalid = DeclarationFlowInventory::new(
        Arc::clone(&inventory.layouts),
        Arc::clone(&inventory.accesses),
        Arc::clone(&inventory.origin_accesses),
        Arc::new(points),
        Arc::new(BTreeMap::new()),
    );
    invalid.source_seed = inventory.source_seed.clone();
    assert!(
        selected(&invalid).is_none(),
        "malformed reached metadata stays terminal"
    );
}
