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
use crate::{LexerConfig, SourceImage, Span};

fn at(marked: &str, dialect: &str) -> Option<OriginalExpressionCursor> {
    let offset = u32::try_from(marked.find('|')?).ok()?;
    let source = marked.replacen('|', "", 1);
    let profile = tcl_dialect::DialectProfile::find(dialect)?;
    let grammar = ExpressionCursorGrammar {
        config: LexerConfig::for_profile(Some(profile)),
        grammar: profile.grammar,
        expr_grammar_base: profile.expr_grammar_base,
        f5_word_grammar: profile.f5_core_expr_grammar(),
    };
    let image = SourceImage::document(&source);
    original_expression_cursor_in(
        &image,
        Span::new(0, u32::try_from(source.len()).ok()?),
        offset,
        grammar,
    )
}

#[test]
fn original_expression_cursor_keeps_literal_comment_and_script_phases() {
    for source in ["si|", "{[} eq {[} ? si|", "1 # [ inert\n + si|"] {
        assert!(
            at(source, "tcl9.0").unwrap().is_expression_position(),
            "{source}"
        );
    }
    for source in [
        "[puts si|",
        "\"literal si|",
        "{literal si|",
        "1 # [ inert|\n+2",
    ] {
        assert!(
            !at(source, "tcl9.0").unwrap().is_expression_position(),
            "{source}"
        );
    }
    assert!(at("1 # inert|", "tcl8.6").is_none());
    assert!(at("${unfinished|", "tcl9.0").is_none());
}

#[test]
fn original_expression_cursor_retains_unfinished_bytes_without_a_closer() {
    let cursor = at("[puts si|", "tcl8.6").unwrap();
    assert_eq!(cursor.prefix_span(), Span::new(0, 8));
    assert_eq!(cursor.region_span(), Span::new(0, 8));
    assert_eq!(
        cursor.lexical_failures()[0].kind,
        ExprLexicalFailureKind::MissingBracket
    );
    assert_eq!(cursor.image.bytes(), b"[puts si");
    assert!(!cursor.is_expression_position());
}

#[test]
fn original_expression_cursor_refuses_foreign_image_configuration_and_geometry() {
    let cursor = at("si|", "tcl9.0").unwrap();
    assert!(cursor.matches_source(&SourceImage::document("si"), cursor.grammar));
    assert!(!cursor.matches_source(&SourceImage::document("so"), cursor.grammar));
    let mut changed = cursor.grammar;
    changed.config.strict_quoting ^= true;
    assert!(!cursor.matches_source(&SourceImage::document("si"), changed));
    assert!(
        original_expression_cursor_in(
            &SourceImage::document("si"),
            Span::new(0, 3),
            2,
            cursor.grammar
        )
        .is_none()
    );
    assert!(
        original_expression_cursor_in(
            &SourceImage::document("si"),
            Span::new(1, 2),
            0,
            cursor.grammar
        )
        .is_none()
    );
}
