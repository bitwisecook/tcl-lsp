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

use crate::analyser::ResolvedAnalysisInput;
use crate::compilation_unit::{CompilationUnit, UnitBuildOptions};
use crate::registry_invocation::InvocationMetadataContext;
use std::sync::Arc;

mod footprint;
mod invocation_metadata;
mod literal_body_ownership;
mod logical_materialized_names;
mod procedure_argument_roles;

fn unit(source: &str, context: &Arc<tcl_registry::model::ContextRegistry>) -> CompilationUnit {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
    let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
    let input = ResolvedAnalysisInput::new(profile, profile, Arc::clone(context), config);
    CompilationUnit::build_with_analysis_input(
        source,
        UnitBuildOptions {
            registry: context.commands(),
            defer_top_level: false,
            config,
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        None,
        &input,
    )
}

fn logical_script_context() -> Arc<tcl_registry::model::ContextRegistry> {
    let mut registry = tcl_registry::CommandRegistry::build_default();
    registry.insert(tcl_registry::CommandSpec {
        name: "run-script",
        arity: tcl_registry::Arity::exact(1),
        arg_roles: &[(0, tcl_registry::ArgRole::Body)],
        ..tcl_registry::CommandSpec::DEFAULT
    });
    let environment = tcl_registry::model::ingress::resolve_environment("tcl");
    Arc::new(
        tcl_registry::model::context_for_profile(environment.analyser_profile())
            .with_command_store(Arc::new(registry)),
    )
}

fn logical_original_tokens(
    source: &str,
    context: &Arc<tcl_registry::model::ContextRegistry>,
) -> (crate::analyser::AnalysisResult, crate::ir::CommandTokens) {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input = ResolvedAnalysisInput::new(profile, profile, Arc::clone(context), config);
    assert!(input.has_logical_source_name_context());
    let analysis = crate::analyser::Analyser::new()
        .with_resolved_input(input)
        .analyse(source, "tcl");
    let segments = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
    let mut tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::new(source),
        config,
        segments.last().expect("actual final original command"),
    );
    analysis
        .retained_command_realm()
        .expect("actual analysis retains its original source world")
        .stamp_original_tokens(&mut tokens);
    (analysis, tokens)
}
