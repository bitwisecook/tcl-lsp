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

use super::{Builder, DeclarationFlowFrame, DeclarationFlowInventory, DeclarationFlowPurpose};
use crate::command_binding::{CommandAllocationSite, SourceAnalysisOptions, SourceCommandBindings};
use crate::ir::CommandTokens;
use std::collections::BTreeSet;
use std::sync::Arc;

struct Fixture {
    input: crate::analyser::ResolvedAnalysisInput,
    inventory: Arc<DeclarationFlowInventory>,
    binding: crate::command_binding::SourceInvocationBinding,
    frame: DeclarationFlowFrame,
    config: tcl_lexer::LexerConfig,
    dialect: tcl_native_model::InvocationDialect,
    tokens: CommandTokens,
    site: CommandAllocationSite,
}

impl Fixture {
    fn new(source: &str, anchor: &str, tail: &str) -> Self {
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::resolve_known_environment("tcl8.6")
                .unwrap()
                .default_context_registry(),
            config,
        );
        let registry = input.borrowed_context_registry().commands();
        let bindings = SourceCommandBindings::analyse_with_options(
            source,
            config,
            registry,
            SourceAnalysisOptions::for_logical_source(&input).unwrap(),
        );
        let image = tcl_lexer::SourceImage::document(source);
        let offset = u32::try_from(source.find(anchor).unwrap()).unwrap();
        let segment = tcl_source::segment::segment_commands_with_offset_and_config(
            &source[offset as usize..source.len() - 1],
            offset,
            config,
        )
        .remove(0);
        let mut tokens = CommandTokens::from_segmented(&image.source_map(), config, &segment);
        bindings.stamp_original_tokens(&mut tokens);
        let binding = tokens.source_binding.as_ref().unwrap();
        let site = binding.invocation_site().unwrap();
        let inventory = Arc::clone(binding.declaration_flow_inventory.as_ref().unwrap());
        let observation = crate::command_binding::declaration_layout::original_declaration_layouts(
            inventory.layouts.get(site).unwrap(),
        )
        .unwrap()
        .next()
        .unwrap();
        let frame = DeclarationFlowFrame::Existing(observation.entry.as_ref().clone());
        assert!(matches!(&frame, DeclarationFlowFrame::Existing(
                crate::command_binding::declaration_layout::OriginalDiagnosticFrameEntry::DeclaredProcedure(_))));
        let dialect = observation
            .snapshot
            .state
            .source_variables
            .invocation_dialect
            .unwrap();
        let owner_binding = binding.clone();
        let offset = u32::try_from(source.find(tail).unwrap()).unwrap();
        let site = CommandAllocationSite {
            source: Arc::clone(&site.source),
            offset,
        };
        let segment = tcl_source::segment::segment_commands_with_offset_and_config(
            &source[offset as usize..source.len() - 1],
            offset,
            config,
        )
        .remove(0);
        let tokens = CommandTokens::from_segmented(&image.source_map(), config, &segment);
        Self {
            input,
            inventory,
            binding: owner_binding,
            frame,
            config,
            dialect,
            tokens,
            site,
        }
    }

    // Real source-body visits retain layouts. This fixture exercises one
    // missing observation while preserving its actual source points.
    fn without_tail_layout(mut self) -> Self {
        let mut layouts = self.inventory.layouts.as_ref().clone();
        assert!(layouts.remove(&self.site).is_some());
        self.inventory = Arc::new(self.with_layouts_and_points(
            layouts,
            self.inventory.points.as_ref().clone(),
            self.inventory.origin_points.as_ref().clone(),
        ));
        self
    }

    fn select(
        &self,
        inventory: &DeclarationFlowInventory,
    ) -> Option<crate::registry_invocation::DeclarationInvocationFlow> {
        Builder {
            inventory,
            registry: self.input.borrowed_context_registry().commands(),
            config: self.config,
            dialect: self.dialect,
            entry: &self.frame,
            purpose: DeclarationFlowPurpose::Reads,
            representation_realm: None,
            representation_loops: Vec::new(),
            current_representation_body: None,
            nodes: Vec::new(),
            jobs: Vec::new(),
            conditional_overwrites: Vec::new(),
            read_sites: BTreeSet::new(),
        }
        .source_read_selection(&self.tokens, &self.site)
    }

    fn with_layouts_and_points(
        &self,
        layouts: crate::command_binding::declaration_layout::DeclarationLayouts,
        points: Vec<crate::command_binding::SourceBindingPoint>,
        origin_points: std::collections::BTreeMap<
            Arc<crate::command_binding::SourceOriginId>,
            Vec<crate::command_binding::SourceBindingPoint>,
        >,
    ) -> DeclarationFlowInventory {
        let mut inventory = DeclarationFlowInventory::new(
            Arc::new(layouts),
            Arc::clone(&self.inventory.accesses),
            Arc::clone(&self.inventory.origin_accesses),
            Arc::new(points),
            Arc::new(origin_points),
        );
        inventory.source_seed = self.inventory.source_seed.clone();
        inventory
    }
}

fn trace_point_facets(
    fixture: &Fixture,
    frame: &crate::var_resolve::VariableExecutionFrame,
    points: &[crate::command_binding::SourceBindingPoint],
    origin_points: &std::collections::BTreeMap<
        Arc<crate::command_binding::SourceOriginId>,
        Vec<crate::command_binding::SourceBindingPoint>,
    >,
) {
    for point in points
        .iter()
        .chain(origin_points.values().flatten())
        .take(64)
    {
        eprintln!(
            "LOGICAL_POINT_FACET offset_match={} dispatch={} frame_match={} origin_match={} words={} compiled_words={}",
            point.offset == fixture.site.offset,
            point.dispatch,
            point.state.variable_frame == *frame,
            point.state.current_source_origin.as_ref() == Some(&fixture.site.source),
            point.original_written_words.is_some(),
            point.compiled_execution.original_words.is_some(),
        );
    }
}

#[test]
fn logical_missing_layout_keeps_conditional_reads_without_faking_an_observation() {
    let fixture = Fixture::new(
        "proc p {x} {switch -glob $x {a* {return 1} default {set z 9}}; puts $missing}",
        "switch",
        "puts",
    )
    .without_tail_layout();
    assert!(!fixture.inventory.layouts.contains_key(&fixture.site));
    let selected = fixture.select(&fixture.inventory).unwrap();
    assert_eq!(
        selected.effects.schema_origin,
        crate::registry_invocation::DeclarationSchemaOrigin::OriginalSourceCatalogue
    );
    let mut layouts = fixture.inventory.layouts.as_ref().clone();
    layouts.insert(fixture.site.clone(), Vec::new());
    let corrupt = fixture.with_layouts_and_points(
        layouts,
        fixture.inventory.points.as_ref().clone(),
        fixture.inventory.origin_points.as_ref().clone(),
    );
    assert!(corrupt.layouts.contains_key(&fixture.site));
    assert!(
        fixture.select(&corrupt).is_none(),
        "present invalid layout remains terminal"
    );
}

#[test]
fn logical_missing_layout_does_not_override_corrupt_reached_point_metadata() {
    // Literal argv retains an evaluated dispatch point to corrupt.
    let fixture = Fixture::new(
        "proc p {} {set anchor 1; puts literal}",
        "set anchor",
        "puts",
    );
    let mut layouts = fixture.inventory.layouts.as_ref().clone();
    assert!(layouts.remove(&fixture.site).is_some());
    let genuine = fixture.with_layouts_and_points(
        layouts.clone(),
        fixture.inventory.points.as_ref().clone(),
        fixture.inventory.origin_points.as_ref().clone(),
    );
    assert!(fixture.select(&genuine).is_some());
    let frame = match &fixture.frame {
        DeclarationFlowFrame::Existing(entry) => entry.frame(),
        DeclarationFlowFrame::Read(_) => unreachable!(),
    };
    let mut points = fixture.inventory.points.as_ref().clone();
    let mut origin_points = fixture.inventory.origin_points.as_ref().clone();
    trace_point_facets(&fixture, frame, &points, &origin_points);
    assert!(
        points
            .iter()
            .chain(origin_points.values().flatten())
            .any(|point| point.dispatch
                && point.offset == fixture.site.offset
                && point.state.variable_frame == *frame
                && point.state.current_source_origin.as_ref() == Some(&fixture.site.source))
    );
    assert!(
        points
            .iter()
            .chain(origin_points.values().flatten())
            .any(|point| {
                point.dispatch
                    && point.offset == fixture.site.offset
                    && point.state.variable_frame == *frame
                    && point.state.current_source_origin.as_ref() == Some(&fixture.site.source)
                    && (point.original_written_words.is_some()
                        || point.compiled_execution.original_words.is_some())
            }),
        "the negative actually withdraws a retained original vector"
    );
    for point in points
        .iter_mut()
        .chain(origin_points.values_mut().flatten())
    {
        point.original_written_words = None;
        point.compiled_execution.original_words = None;
    }
    let corrupt = fixture.with_layouts_and_points(layouts, points, origin_points);
    assert!(
        fixture.select(&corrupt).is_none(),
        "reached point refusal remains terminal"
    );
}
#[test]
fn conditional_read_scope_keeps_unknown_prefix_and_wrong_point_refusals() {
    let source = "proc p {} {set anchor 1; unknown_command; puts $missing}";
    let mut fixture = Fixture::new(source, "set anchor", "puts").without_tail_layout();
    assert!(!fixture.inventory.layouts.contains_key(&fixture.site));
    let report = fixture
        .binding
        .declaration_flow_report(fixture.input.borrowed_context_registry().commands())
        .unwrap();
    assert!(report.unavailable);
    let span = report.potential_declared_reads()["missing"];
    assert!(
        !report
            .declared_absence_warning_occurrences()
            .contains_key("missing")
    );
    assert!(!report.allows_declared_absence_warning("missing", span));
    let original = fixture.site.clone();
    fixture.site.offset += 1;
    assert_ne!(fixture.site, original);
    assert!(fixture.select(&fixture.inventory).is_none());
}
