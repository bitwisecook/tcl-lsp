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
use std::sync::Arc;
use tcl_registry::{
    ArgRole, Arity, CommandRegistry, CommandSpec, InvocationArguments, ScriptTiming,
};

fn reference_only(_arguments: InvocationArguments<'_>) -> Vec<(u8, ScriptTiming)> {
    vec![(0, ScriptTiming::ReferenceOnly)]
}

#[test]
fn original_reference_only_body_cannot_supply_local_store_suppression() {
    // naming.source.original-script-region-purpose
    // docs/design/analysis/name-resolution-proofs/original-script-region-purpose.md
    // Diagnostic suppression capability; no executed store or child entry.
    let source = "reference-script {set x 1; puts $x}\nrun-script {set x 1; puts $x}";
    let mut registry = CommandRegistry::build_default();
    for (name, timing) in [
        (
            "reference-script",
            Some(reference_only as tcl_registry::ScriptTimingResolver),
        ),
        ("run-script", None),
    ] {
        registry.insert(CommandSpec {
            name,
            arity: Arity::exact(1),
            arg_roles: &[(0, ArgRole::Body)],
            script_timing_resolver: timing,
            ..CommandSpec::DEFAULT
        });
    }
    let profile = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
    let context = Arc::new(
        tcl_registry::model::context_for_profile(profile).with_command_store(Arc::new(registry)),
    );
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input =
        crate::analyser::ResolvedAnalysisInput::new(profile, profile, Arc::clone(&context), config);
    assert!(input.has_logical_source_name_context());
    let analysis = crate::analyser::Analyser::new()
        .with_resolved_input(input)
        .analyse(source, "tcl");
    let segments = tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config);
    for (index, expected) in [(0, false), (1, true)] {
        let mut tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(source),
            config,
            &segments[index],
        );
        analysis
            .retained_command_realm()
            .unwrap()
            .stamp_original_tokens(&mut tokens);
        let words =
            crate::registry_invocation::source_structure::original_registry_words_for_tokens(
                source, &analysis, &tokens,
            )
            .expect("original source producer");
        assert_eq!(
            words.source_script_bodies(&context).len(),
            1,
            "both bodies remain original syntax"
        );
        let stmt = crate::ir::Statement::Barrier {
            span: tokens.argv[0],
            reason: "authored foreign body".into(),
            command: segments[index].texts[0].clone(),
            canonical_command: None,
            args: segments[index].texts[1..].to_vec(),
            tokens: Some(tokens.clone()),
        };
        assert_eq!(
            barrier_body_locally_sets(Some(&stmt), "x", source, &analysis, &context, Some(&tokens)),
            expected
        );
    }
}
