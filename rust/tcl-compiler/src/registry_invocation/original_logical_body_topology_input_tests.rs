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

#[test]
fn original_logical_body_topology_requires_the_supplied_complete_input_at_same_registry() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Conditional source topology only; no Native body or compiler admission.
    let mut analyser = crate::analyser::Analyser::new();
    let input = analyser.prepare_analysis_input("tcl").unwrap();
    let registry = input.borrowed_context_registry().commands();
    let source = "if {1} {set value VALUE}";
    let image = tcl_lexer::SourceImage::document(source);
    let config = input.lexer_config();
    let bindings =
        crate::command_binding::SourceCommandBindings::analyse_image_in_frame_with_options(
            &image,
            &crate::var_resolve::VariableExecutionFrame::Unknown,
            config,
            registry,
            crate::command_binding::SourceAnalysisOptions::for_logical_source(&input).unwrap(),
        )
        .unwrap();
    let segment =
        tcl_source::segment::segment_commands_image_with_offset_and_config(&image, 0, config)
            .unwrap()
            .pop()
            .unwrap();
    let mut tokens = CommandTokens::from_segmented(&image.source_map(), config, &segment);
    bindings.stamp_original_tokens(&mut tokens);
    let metadata = InvocationMetadataContext::for_analysis_input(registry, &input).unwrap();
    assert!(
        conditional_body_topology_advice_with_metadata_context(
            registry, metadata, &tokens, &bindings,
        )
        .is_some()
    );
    assert!(
        tokens
            .source_binding
            .as_ref()
            .unwrap()
            .proved_handler_target()
            .is_none()
    );
    let mut shifted = config;
    shifted.strict_quoting = !shifted.strict_quoting;
    let changed = crate::analyser::ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        input.context_registry(),
        shifted,
    );
    let changed_metadata =
        InvocationMetadataContext::for_analysis_input(registry, &changed).unwrap();
    assert!(changed_metadata.permits_logical_source_names());
    assert!(
        conditional_body_topology_advice_with_metadata_context(
            registry,
            changed_metadata,
            &tokens,
            &bindings,
        )
        .is_none()
    );
    let older = tcl_registry::model::ingress::resolve_known_environment("tcl8.4")
        .unwrap()
        .default_context_registry();
    let older = crate::analyser::ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        std::sync::Arc::new(older.with_command_store(registry.snapshot().shared_registry())),
        config,
    );
    let older_metadata = InvocationMetadataContext::for_analysis_input(registry, &older).unwrap();
    assert!(older_metadata.permits_logical_source_names());
    assert!(older_metadata.matches_registry(registry));
    assert!(
        conditional_body_topology_advice_with_metadata_context(
            registry,
            older_metadata,
            &tokens,
            &bindings,
        )
        .is_none()
    );
    let mut missing = tokens.clone();
    missing.source_binding = None;
    assert!(
        conditional_body_topology_advice_with_metadata_context(
            registry, metadata, &missing, &bindings,
        )
        .is_none()
    );
}
