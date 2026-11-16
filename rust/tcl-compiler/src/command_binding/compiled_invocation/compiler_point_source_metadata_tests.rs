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

fn fixture(source: &str) -> (crate::ir::Module, crate::analyser::ResolvedAnalysisInput) {
    let baseline = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let mut registry = baseline.commands().project_for_profile(profile);
    let mut puts = registry.get("puts").unwrap().clone();
    puts.surface = Some(tcl_dialect::model::SpecSurface::TCL86_PLUS);
    registry.insert(puts);
    let registry = registry.project_for_profile(profile);
    let context = Arc::new(baseline.with_command_store(Arc::new(registry)));
    let config = tcl_lexer::LexerConfig::for_dialect("tcl8.6");
    let input =
        crate::analyser::ResolvedAnalysisInput::new(profile, profile, Arc::clone(&context), config);
    // Unknown execution is an explicit entry; no entry means a fresh interpreter.
    let mut lowerer = crate::lowering::Lowerer::with_config(context.commands(), config)
        .with_resolved_analysis_input(input.clone())
        .with_explicit_unknown_entry();
    (lowerer.lower(source).clone(), input)
}

fn tokens(module: &crate::ir::Module) -> &crate::ir::CommandTokens {
    module
        .top_level
        .retained_source_tokens_for_statement(&module.top_level.statements[0])
        .unwrap()
}

#[test]
fn native_compiler_point_source_metadata_keeps_original_words_and_unknown_execution() {
    for (source, expected) in [("puts {$literal}", Some("$literal")), ("puts $value", None)] {
        let (module, input) = fixture(source);
        let tokens = tokens(&module);
        let binding = tokens.source_binding.as_ref().unwrap();
        let registry = input.borrowed_context_registry().commands();
        assert!(module.source_entry.native_entry.is_none());
        assert!(binding.has_nonpoint_original_source_name_role_owner());
        assert!(!binding.has_original_native_source_command());
        assert!(
            binding
                .original_compiler_point_words_for_source_metadata(tokens, &input, registry)
                .is_some()
        );
        let inventory = crate::registry_invocation::original_source_metadata_name_roles(
            registry, tokens, &input,
        )
        .expect("authentic retained root compiler words supply conditional source metadata");
        assert_eq!(inventory.facts().descriptor_key().label(), "puts");
        assert_eq!(inventory.argument_literal(0).as_deref(), expected);
        assert!(inventory.has_unknown_residual());
        assert!(inventory.direct_written_arguments());
        assert!(binding.execution_is_unknown());
        assert!(binding.execution_targets().next().is_none());
        assert!(binding.proved_handler_target().is_none());
    }
}

#[test]
fn native_compiler_point_source_metadata_refuses_missing_changed_or_foreign_carriers() {
    let (module, input) = fixture("puts VALUE");
    let tokens = tokens(&module);
    let registry = input.borrowed_context_registry().commands();
    let mut missing = tokens.clone();
    missing
        .source_binding
        .as_mut()
        .unwrap()
        .original_compiler_words = None;
    let mut changed = tokens.clone();
    Arc::make_mut(
        changed
            .source_binding
            .as_mut()
            .unwrap()
            .original_compiler_words
            .as_mut()
            .unwrap(),
    )
    .site
    .offset += 1;
    let mut argv = tokens.clone();
    argv.argv_texts[1] = "OTHER".to_owned();
    let mut unselected = tokens.clone();
    unselected
        .source_binding
        .as_mut()
        .unwrap()
        .native_compilation_admission = None;
    for negative in [&missing, &changed, &argv, &unselected] {
        assert!(
            negative
                .source_binding
                .as_ref()
                .unwrap()
                .original_compiler_point_words_for_source_metadata(negative, &input, registry)
                .is_none()
        );
    }
    let foreign = tcl_registry::model::ingress::static_context_for("tcl9.1").commands();
    assert!(
        tokens
            .source_binding
            .as_ref()
            .unwrap()
            .original_compiler_point_words_for_source_metadata(tokens, &input, foreign)
            .is_none()
    );
    assert!(
        crate::registry_invocation::original_source_metadata_name_roles(registry, &changed, &input)
            .is_none()
    );
}
