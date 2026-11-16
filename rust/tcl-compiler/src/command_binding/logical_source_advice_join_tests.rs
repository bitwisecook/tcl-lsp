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

fn original_child(source: &str) -> (crate::ir::Module, crate::ir::CommandTokens) {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let context = tcl_registry::model::ingress::context_for_profile(profile);
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input =
        crate::analyser::ResolvedAnalysisInput::new(profile, profile, Arc::clone(&context), config);
    let module = crate::lowering::lower_to_ir_with(
        crate::lowering::Lowerer::with_config(context.commands(), config)
            .with_resolved_analysis_input(input),
        source,
    );
    let parent = module
        .top_level
        .retained_source_tokens_for_statement(&module.top_level.statements[0])
        .unwrap();
    let mut children =
        crate::registry_invocation::source_structure::original_immediate_body_commands_for_module(
            &module,
            context.commands(),
            parent,
        )
        .unwrap();
    (module, children.remove(0))
}

#[test]
fn logical_source_advice_join_requires_the_same_complete_original_owner() {
    let (module, tokens) = original_child("catch {set ::hits VALUE}");
    let input = module.source_metadata_input.as_ref().unwrap();
    let registry = module.resolved_registry();
    let original = tokens.source_binding.as_ref().unwrap();
    assert!(original.original_logical_source_advice.is_some());
    let mut same = original.clone();
    same.join(original);
    let mut joined = tokens.clone();
    joined.source_binding = Some(same);
    assert!(
        joined
            .source_binding
            .as_ref()
            .unwrap()
            .original_logical_source_words(&joined, input, registry)
            .is_some()
    );
    let (_, other) = original_child("catch {set ::other VALUE}");
    let mut different = original.clone();
    different.join(other.source_binding.as_ref().unwrap());
    assert!(different.original_logical_source_advice.is_none());
    joined.source_binding = Some(different);
    assert!(
        joined
            .source_binding
            .as_ref()
            .unwrap()
            .original_logical_source_words(&joined, input, registry)
            .is_none()
    );
    let mut absent = original.clone();
    absent.original_logical_source_advice = None;
    let mut mixed = original.clone();
    mixed.join(&absent);
    assert!(mixed.original_logical_source_advice.is_none());
    joined.source_binding = Some(mixed);
    assert!(
        joined
            .source_binding
            .as_ref()
            .unwrap()
            .original_logical_source_words(&joined, input, registry)
            .is_none()
    );
}
