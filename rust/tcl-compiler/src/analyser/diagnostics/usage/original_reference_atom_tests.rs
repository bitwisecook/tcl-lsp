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
fn expression_atoms_use_the_original_complete_reference_grammar() {
    for (dialect, unicode, nested) in [
        ("tcl8.6", false, false),
        ("tcl9.1", false, true),
        ("jim", true, false),
    ] {
        let config = tcl_lexer::LexerConfig::for_profile(Some(
            tcl_registry::model::ingress::resolve_known_environment(dialect)
                .expect("selected original reference grammar environment")
                .unit_profile(),
        ));
        for reference in [
            "${ with space }",
            "${$literal}",
            "${[literal]}",
            "${}",
            "$::N::value",
        ] {
            assert!(is_expr_atom(reference, config), "{dialect} {reference}");
        }
        assert_eq!(is_expr_atom("$café", config), unicode, "{dialect}");
        assert_eq!(is_expr_atom("${a{b}c}", config), nested, "{dialect}");
        for reference in [
            "$array(key)",
            "${array(key)}",
            "${value}tail",
            "$value+1",
            "$",
            "${missing",
            "${value}}",
        ] {
            assert!(!is_expr_atom(reference, config), "{dialect} {reference}");
        }
        for literal in ["42", "3.14", "0x1f", "true"] {
            assert!(is_expr_atom(literal, config));
        }
    }
}

#[test]
fn nested_expression_fix_retains_current_whole_word_geometry_and_configuration() {
    let source = "set { with space } 7; expr {[expr {${ with space }}] + 1}";
    let mut analyser = Analyser::new();
    let result = analyser.analyse(source, "tcl8.6");
    let diagnostic = result
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == DiagCode::W114)
        .expect("actual original expression role and nested expression source");
    assert_eq!(diagnostic.fixes.len(), 1);
    assert_eq!(diagnostic.fixes[0].new_text, "${ with space }");
    assert_eq!(
        &source[diagnostic.fixes[0].span.as_range()],
        "[expr {${ with space }}]"
    );
    assert_eq!(
        diagnostic.fixes[0].safety,
        crate::irules_checks::FixSafety::RequiresReview
    );
    let count = analyser.result.diagnostics.len();
    let config = analyser.file_lexer_config();
    for (text, config) in [
        (format!("{source} # changed"), config),
        (
            source.to_owned(),
            tcl_lexer::LexerConfig {
                strict_quoting: !config.strict_quoting,
                ..config
            },
        ),
    ] {
        let words = tcl_lexer::native_script_words_in(
            tcl_lexer::SourceImage::document(&text),
            tcl_lexer::Span::new(0, u32::try_from(text.len()).unwrap()),
            config,
        )
        .unwrap();
        let word = &words.commands[1].words[1];
        analyser.emit_w114_redundant_nested_expr(word);
        assert_eq!(analyser.result.diagnostics.len(), count);
    }
}
