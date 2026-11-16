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
fn original_definition_body_projection_keeps_bom_geometry_and_parent_config_separate() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    // Local syntax shared by validation/publication, never a member or Native admission receipt.
    let mut prepared = crate::analyser::Analyser::new();
    let input = prepared.prepare_analysis_input("tcl9.1").unwrap();
    let body = "\u{feff}method hidden {} {}; method shown {} {}";
    for bom in [tcl_lexer::LeadingBom::Skip, tcl_lexer::LeadingBom::Content] {
        let mut config = input.lexer_config();
        config.leading_bom = bom;
        let tokens = definition_body_command_tokens(body, config).collect::<Vec<_>>();
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].argv_texts, ["\u{feff}method", "hidden", "", ""]);
        assert_eq!(tokens[1].argv_texts, ["method", "shown", "", ""]);
        assert_eq!(tokens[0].argv[0].start(), 0);
        assert_eq!(
            body.get(tokens[0].argv[0].as_range()),
            Some("\u{feff}method")
        );
        assert_eq!(body.get(tokens[1].argv[1].as_range()), Some("shown"));
        assert!(
            tokens
                .iter()
                .all(|command| command.source_binding.is_none())
        );
        assert_eq!(config.leading_bom, bom);

        let nested = definition_body_command_tokens("method ${outer{inner}}", config)
            .next()
            .unwrap();
        assert!(matches!(
            nested.words()[1],
            crate::ir::WordExpr::Variable { .. }
        ));
        let first_close = tcl_lexer::LexerConfig {
            braced_var: tcl_lexer::BracedVarStyle::FirstClose,
            ..config
        };
        let nested = definition_body_command_tokens("method ${outer{inner}}", first_close)
            .next()
            .unwrap();
        assert!(matches!(
            nested.words()[1],
            crate::ir::WordExpr::Template { .. }
        ));
        assert!(nested.source_binding.is_none());
    }
}
