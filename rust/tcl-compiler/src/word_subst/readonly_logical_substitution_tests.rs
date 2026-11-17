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
fn readonly_logical_input_inventory_keeps_real_expr_and_lexical_children() {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let context = tcl_registry::model::ingress::context_for_profile(profile);
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&context),
        config,
    );
    let registry = context.commands();
    for (body, expected) in [
        ("set value [info exists target]", 1),
        ("expr {[info exists target]}", 1),
        ("set value {[info exists target]}", 0),
    ] {
        let source = format!("proc caller {{}} {{{body}}}");
        let module = crate::lowering::lower_to_ir_with(
            crate::lowering::Lowerer::with_config(registry, config)
                .with_resolved_analysis_input(input.clone()),
            &source,
        );
        let (_, procedure) = module.original_source_procedure_bodies().next().unwrap();
        let tokens = procedure
            .body
            .retained_source_tokens_for_statement(&procedure.body.statements[0])
            .unwrap();
        let calls = checked_original_lifted_calls_with_logical_source_input(
            tokens, config, registry, &input,
        )
        .expect("actual readonly Logical inventory");
        assert_eq!(calls.len(), expected, "{body}");
        let mut changed = tokens.clone();
        changed.source_binding = None;
        assert!(
            checked_original_lifted_calls_with_logical_source_input(
                &changed, config, registry, &input
            )
            .is_none()
        );
        let mut wrong_config = config;
        wrong_config.strict_quoting = !config.strict_quoting;
        assert!(
            checked_original_lifted_calls_with_logical_source_input(
                tokens,
                wrong_config,
                registry,
                &input
            )
            .is_none()
        );
        if expected != 0 {
            let mut missing = tokens.clone();
            missing.nested_bindings_mut().clear();
            assert!(
                checked_original_lifted_calls_with_logical_source_input(
                    &missing, config, registry, &input
                )
                .is_none()
            );
        }
        let wrong_profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let wrong_input = crate::analyser::ResolvedAnalysisInput::new(
            wrong_profile,
            wrong_profile,
            std::sync::Arc::clone(&context),
            config,
        );
        assert!(
            checked_original_lifted_calls_with_logical_source_input(
                tokens,
                config,
                registry,
                &wrong_input
            )
            .is_none()
        );
        let top = module
            .top_level
            .retained_source_tokens_for_statement(&module.top_level.statements[0])
            .unwrap();
        assert!(
            top.source_binding
                .as_ref()
                .unwrap()
                .has_original_source_layout_owner()
        );
        assert!(
            checked_original_lifted_calls_with_logical_source_input(top, config, registry, &input)
                .is_none()
        );
    }
}
