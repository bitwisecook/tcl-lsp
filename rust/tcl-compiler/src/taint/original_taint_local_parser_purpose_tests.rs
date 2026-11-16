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
fn original_taint_callback_and_bracket_shapes_keep_actual_full_config() {
    // naming.source.retained-positioned-command-realm
    // docs/design/analysis/name-resolution-proofs/retained-positioned-command-realm.md
    // These local source/synthetic replay projections grant no callback execution.
    let mut prepared = crate::analyser::Analyser::new();
    let input = prepared.prepare_analysis_input("tcl9.0").unwrap();
    for bom in [tcl_lexer::LeadingBom::Skip, tcl_lexer::LeadingBom::Content] {
        let config = tcl_lexer::LexerConfig {
            leading_bom: bom,
            strict_quoting: true,
            braced_var: tcl_lexer::BracedVarStyle::FirstClose,
            ..input.lexer_config()
        };
        let actual_input = crate::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            input.context_registry(),
            config,
        );
        let registry = actual_input.borrowed_context_registry().commands();
        let unit = crate::compilation_unit::CompilationUnit::build_with_analysis_input(
            "set value VALUE",
            crate::compilation_unit::UnitBuildOptions {
                registry,
                defer_top_level: false,
                config,
                dialect: Some(actual_input.unit_profile()),
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            &actual_input,
        );
        let source =
            TaintSourceContext::for_module_function(registry, &unit.ir_module, &unit.top_level);
        assert_eq!(source.config, config);
        assert!(source.metadata.is_some());
        let callback =
            callback_replay_script("\u{feff}eval literal", &[], "input", source.config).unwrap();
        assert!(
            callback.starts_with('\u{feff}'),
            "body mark is not the file prologue: {callback:?}"
        );
        let (head, args) =
            parse_taint_command_substitution("[\u{feff}eval literal]", source.config).unwrap();
        assert_eq!(head, "\u{feff}eval");
        assert_eq!(args, ["literal"]);
        let first_close = "eval ${a{b}; eval literal";
        assert!(callback_replay_script(first_close, &[], "input", source.config).is_some());
        let nesting = tcl_lexer::LexerConfig {
            braced_var: tcl_lexer::BracedVarStyle::Tcl9Nesting,
            ..source.config
        };
        assert!(callback_replay_script(first_close, &[], "input", nesting).is_none());
        let mut changed = unit.clone();
        changed.ir_module.lexer_config.strict_quoting = !config.strict_quoting;
        assert!(
            TaintSourceContext::for_module_function(
                registry,
                &changed.ir_module,
                &changed.top_level
            )
            .metadata
            .is_none()
        );
        changed.ir_module.source_metadata_input = None;
        assert!(
            TaintSourceContext::for_module_function(
                registry,
                &changed.ir_module,
                &changed.top_level
            )
            .metadata
            .is_none()
        );
    }
}
