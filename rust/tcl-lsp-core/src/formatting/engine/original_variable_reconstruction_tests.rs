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
fn variable_style_proposals_keep_original_array_indices_and_selected_name_grammar() {
    let registry = CommandRegistry::build_default();
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let config = FormatterConfig {
            enforce_braced_variables: true,
            ..FormatterConfig::for_dialect(dialect)
        };
        for (source, expected) in [
            ("puts $x\n", "puts ${x}\n"),
            ("puts \"v:$x\"\n", "puts \"v:${x}\"\n"),
            ("puts $a(k)\n", "puts $a(k)\n"),
            ("puts $a($i)\n", "puts $a($i)\n"),
            ("puts ${a($i)}\n", "puts ${a($i)}\n"),
            ("puts \"v:$a($i)\"\n", "puts \"v:$a($i)\"\n"),
            ("puts ${$n}\n", "puts ${$n}\n"),
            ("puts ${}\n", "puts ${}\n"),
            ("puts ${a{b}c}\n", "puts ${a{b}c}\n"),
        ] {
            assert_eq!(
                format_tcl(source, &config, &registry),
                expected,
                "{dialect}"
            );
        }
    }
}

#[test]
fn configured_variable_reconstruction_reborrows_whole_words_and_refuses_changed_tokens() {
    for dialect in ["tcl8.6", "tcl9.1", "jim"] {
        let config = FormatterConfig::for_dialect(dialect).lexer_config();
        let source = "puts \"v:$a($i)\"";
        let map = SourceMap::new(source);
        let tokens = Lexer::with_config(source, config).tokenise_all().unwrap();
        let (mut commands, _) = parse_commands(source, &map, &tokens, config).unwrap();
        let arg = &mut commands[0].args[1];
        assert_eq!(reconstruct_arg(&map, arg, true), "\"v:$a($i)\"");
        assert!(!same_variable_word_components(
            "\"v:$a($i)\"",
            "\"v:${a($i)}\"",
            &arg.lexical_word,
        ));
        let token = arg
            .tokens
            .iter_mut()
            .find(|token| token.kind == TokenType::Var)
            .unwrap();
        token.content_offset = token.content_offset.saturating_add(1);
        assert!(original_variable_source(&map, *token, config).is_none());
        assert_eq!(reconstruct_arg(&map, arg, true), "\"v:$a($i)\"");
        let foreign = SourceMap::new("puts other");
        assert!(parse_commands(source, &foreign, &tokens, config).is_none());
    }
}
