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

//! A Logical command layout keeps unknown physical frames separate.

use super::*;

#[test]
fn logical_command_layout_keeps_unknown_physical_frames_separate() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input =
        crate::analyser::ResolvedAnalysisInput::new(profile, profile, Arc::clone(&context), config);
    let image = tcl_lexer::SourceImage::document("proc subject {} {return VALUE}");
    let bindings = SourceCommandBindings::analyse_image_in_frame_with_options(
        &image,
        &crate::var_resolve::VariableExecutionFrame::Unknown,
        config,
        context.commands(),
        SourceAnalysisOptions::for_logical_source(&input).unwrap(),
    )
    .unwrap();
    let point = bindings.points.first().expect("original source command");
    assert_eq!(point.state.logical_source_name_advice_input(), Some(&input));
    assert_eq!(
        point.state.variable_frame,
        crate::var_resolve::VariableExecutionFrame::Unknown
    );
    let segment =
        tcl_source::segment::segment_commands_image_with_offset_and_config(&image, 0, config)
            .unwrap()
            .remove(0);
    let tokens = crate::ir::CommandTokens::from_segmented(&image.source_map(), config, &segment);
    let site = CommandAllocationSite {
        source: Arc::clone(point.state.current_source_origin.as_ref().unwrap()),
        offset: segment.span.start(),
    };
    let snapshot = Arc::new(SourceLookupSnapshot::new(point.state.clone()));
    let selected = original_site_operand_layout_advice(
        &site,
        &tokens,
        &snapshot,
        &point.namespace_key,
        config,
    )
    .expect("conditional original Logical layout");
    assert_eq!(
        selected.frame(),
        &crate::var_resolve::VariableExecutionFrame::Unknown
    );
    assert!(selected.targets.iter().all(|target| target.registry_backed));
    assert!(snapshot.state.baseline.native_entry.is_none());
    assert!(snapshot.state.baseline.execution_name_policy.is_none());

    let mut stale = config;
    stale.strict_quoting = !stale.strict_quoting;
    assert!(
            original_site_operand_layout_advice(
                &site,
                &tokens,
                &snapshot,
                &point.namespace_key,
                stale,
            )
            .is_none()
        );
    let mut unowned = snapshot.state.clone();
    Arc::make_mut(&mut unowned.baseline).logical_source_input = None;
    let unowned = Arc::new(SourceLookupSnapshot::new(unowned));
    assert!(
            original_site_operand_layout_advice(
                &site,
                &tokens,
                &unowned,
                &point.namespace_key,
                config,
            )
            .is_none()
        );
    let mut foreign_source = snapshot.state.clone();
    foreign_source.current_source_origin = None;
    let foreign_source = Arc::new(SourceLookupSnapshot::new(foreign_source));
    assert!(
        original_site_operand_layout_advice(
            &site,
            &tokens,
            &foreign_source,
            &point.namespace_key,
            config,
        )
        .is_none()
    );
}
