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

use super::LexerConfig;
use tcl_dialect::model::{Family, Release, grammar};

#[test]
fn from_grammar_carries_every_lexer_axis() {
    let g = grammar(Family::Jim, Release::JIM_0_84);
    let c = LexerConfig::from_grammar(g);
    assert_eq!(c.expand_syntax, g.expand_syntax);
    assert_eq!(c.irules_brace_separator, g.irules_brace_separator);
    assert_eq!(c.brace_line_continuation, g.brace_line_continuation);
    assert_eq!(c.braced_var, g.braced_var);
    assert_eq!(c.array_index, g.array_index);
    assert_eq!(c.escapes, g.escapes);
    assert_eq!(c.word_separators, g.word_separators);
    assert_eq!(c.quote_termination, g.quote_termination);
    assert_eq!(c.var_syntax, g.var_syntax);
    assert_eq!(c.brace_backslash_newline, g.brace_backslash_newline);
    assert_eq!(c.list_parse, g.list_parse);
    assert_eq!(c.source_line_endings, g.source_line_endings);
}

#[test]
fn native_grammar_selection_preserves_source_coordinates_and_parser_modes() {
    let original = LexerConfig {
        base_offset: 123,
        base_line: 7,
        base_col: 11,
        strict_quoting: false,
        leading_bom: super::LeadingBom::Skip,
        ..LexerConfig::default()
    };
    let selected = original.with_grammar(grammar(Family::Jim, Release::JIM_0_84));
    assert_eq!(
        selected.normalized(),
        LexerConfig {
            strict_quoting: original.strict_quoting,
            leading_bom: original.leading_bom,
            ..LexerConfig::from_grammar(grammar(Family::Jim, Release::JIM_0_84))
        }
    );
    assert_eq!(selected.base_offset, original.base_offset);
    assert_eq!(selected.base_line, original.base_line);
    assert_eq!(selected.base_col, original.base_col);
}

#[test]
fn lexical_overlay_preserves_independently_selected_semantic_axes() {
    let jim = grammar(Family::Jim, Release::JIM_0_84);
    let c = LexerConfig::from_grammar(jim);
    let base = tcl_dialect::LexerGrammar::default();
    let overlaid = c.grammar_over(base);
    assert_eq!(LexerConfig::from_grammar(overlaid), c);
    assert_eq!(overlaid.numbers, base.numbers);
    assert_eq!(overlaid.expr_comments, base.expr_comments);
    assert_eq!(
        overlaid.script_skips_leading_bom,
        base.script_skips_leading_bom
    );
}

/// The four hand-written copies of the 9.x grammar — `LexerGrammar::
/// default()`, the catalogue's, the family ladder's, and the config's
/// default — are one value.
#[test]
fn the_default_grammar_is_tcl_9_0_everywhere() {
    let ladder = grammar(Family::Tcl, Release::TCL_9_0);
    assert_eq!(tcl_dialect::LexerGrammar::default(), ladder);
    let from_default = LexerConfig::from_grammar(tcl_dialect::LexerGrammar::default());
    let plain = LexerConfig::default();
    assert_eq!(from_default.escapes, plain.escapes);
    assert_eq!(from_default.braced_var, plain.braced_var);
    assert_eq!(from_default.word_separators, plain.word_separators);
    assert_eq!(from_default.list_parse, plain.list_parse);
    assert_eq!(
        from_default.brace_backslash_newline,
        plain.brace_backslash_newline
    );
    assert_eq!(from_default.source_line_endings, plain.source_line_endings);
}
