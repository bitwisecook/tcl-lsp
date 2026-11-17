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

use crate::interprocedural::test_body_identity::fixture_id_if_present;

use super::*;

mod command_flow;
mod evaluation;
mod representation_plan;
mod selection;

fn conditional_representation_report(source: &str) -> Arc<DeclarationFlowReport> {
    let analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
    let input = analysis.resolved_input.as_ref().unwrap();
    let registry = input.borrowed_context_registry().commands();
    let config = input.lexer_config();
    let realm = analysis.retained_command_realm().unwrap();
    let image = tcl_lexer::SourceImage::document(source);
    let segments = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
    let mut tokens = CommandTokens::from_segmented(&image.source_map(), config, &segments[0]);
    realm
        .source_bindings_ref()
        .stamp_original_tokens(&mut tokens);
    tokens
        .source_binding
        .as_ref()
        .unwrap()
        .declaration_representation_flow_report(&tokens, input, registry, Some(realm))
        .expect("authentic root representation report")
}

fn report(source: &str) -> Arc<DeclarationFlowReport> {
    report_entered(source, false)
}

/// [`report`] for an embedding host that registers its own native commands:
/// its object types may carry conversion hooks that run anything.
fn report_in_extensible_host(source: &str) -> Arc<DeclarationFlowReport> {
    report_entered(source, true)
}

fn report_entered(source: &str, host_extensible_command_table: bool) -> Arc<DeclarationFlowReport> {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let config = tcl_lexer::LexerConfig::from_grammar(registry.profile().unwrap().grammar);
    let bindings = super::super::SourceCommandBindings::analyse_with_options(
        source,
        config,
        registry,
        super::super::SourceAnalysisOptions {
            host_extensible_command_table,
            invocation_dialect: Some(crate::environment_ingress::authoring_invocation_dialect(
                registry, None, config,
            )),
            native_compilation: crate::environment_ingress::authoring_native_compilation(),
            ..super::super::SourceAnalysisOptions::default()
        },
    );
    let original = "set anchor 1";
    let offset = u32::try_from(source.find(original).unwrap()).unwrap();
    let segment =
        crate::segmenter::segment_commands_with_offset_and_config(original, offset, config)
            .remove(0);
    let mut tokens = CommandTokens::from_segmented(
        &tcl_lexer::SourceImage::document(source).source_map(),
        config,
        &segment,
    );
    bindings.stamp_original_tokens(&mut tokens);
    tokens
        .source_binding
        .as_ref()
        .unwrap()
        .declaration_flow_report(registry)
        .expect("accepted original declaration owns a diagnostic graph")
}
