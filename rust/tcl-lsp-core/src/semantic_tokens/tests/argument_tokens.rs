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

//! Argument words: expression operands, quoted strings, barewords, variables and numbers.

use super::*;

#[test]
fn expression_commands_keep_nested_substitution_closers() {
    let source = "if {[llength [list $value]]} {}\n";
    for label in ["tcl8.6", "tcl9.1"] {
        let context = tcl_registry::model::ingress::static_context_for(label);
        let profile = context
            .commands()
            .profile()
            .expect("explicit release fixture");
        let tokens = decode_full(source, profile, context.commands());
        let list = u32::try_from(source.find("list").unwrap()).unwrap();
        assert!(
            tokens
                .iter()
                .any(|&(line, column, length, kind, modifiers)| {
                    line == 0
                        && column == list
                        && length == 4
                        && kind == TokenKind::Function as u32
                        && modifiers & MOD_DEFAULT_LIBRARY != 0
                }),
            "{label}: {tokens:?}"
        );
        let value = u32::try_from(source.find("$value").unwrap()).unwrap();
        assert!(
            tokens.iter().any(|&(line, column, length, kind, _)| {
                line == 0
                    && column <= value + 1
                    && column + length >= value + 6
                    && kind == TokenKind::Variable as u32
            }),
            "{label}: {tokens:?}"
        );
    }
}

#[test]
fn nested_command_head_substitution_in_an_expression_keeps_word_extents_2265() {
    let source = "if {[[list string] length abcdef] > 1} {}\n";
    let tokens = decode_full(source, tcl(), &reg());
    for word in ["length", "abcdef"] {
        let start = u32::try_from(source.find(word).unwrap()).unwrap();
        let length = u32::try_from(word.len()).unwrap();
        assert!(
            tokens
                .iter()
                .any(|&(line, column, len, _, _)| line == 0 && column == start && len == length),
            "{word}: {tokens:?}"
        );
    }
    assert_non_overlapping(source, &reg());
}

#[test]
fn quoted_var_at_string_start_no_overlap() {
    // The lexer's empty-content clamp spans the opening `"` fragment over
    // `"$`, overlapping the `$x` variable token; the opening fragment must
    // shrink to just the `"`.
    let r = reg();
    assert_non_overlapping("puts \"$x y\"\n", &r);
    assert_non_overlapping("set x 1\nputs \"$x — résumé — 日本語\"\n", &r);
    // Adjacent substitutions: the empty ESC between `$a` and `$b`
    // carries no delimiter, so it must vanish entirely (no zero-area
    // overlap at the `$b`).
    assert_non_overlapping("puts \"$a$b\"\n", &r);
    // Command substitution introducer `[` at string start.
    assert_non_overlapping("puts \"[expr {1+2}] z\"\n", &r);
    // Dense line with several adjacent substitutions/strings.
    assert_non_overlapping("set a 1;set b 2;puts \"$a [expr {$a+$b}] $b\";# tail\n", &r);
}

#[test]
fn quoted_string_opening_fragment_is_single_quote() {
    // `puts "$x y"` — the opening string fragment is exactly the `"`
    // (col 5, len 1), not `"$` (len 2).
    let toks = decode("puts \"$x y\"\n", tcl(), &reg());
    // The opening `"` lands at byte/col 5 on line 0 with length 1.
    assert!(
        toks.contains(&(0, 5, 1)),
        "expected a length-1 string token at col 5, got {toks:?}",
    );
}

#[test]
fn bareword_argument_classified_as_string() {
    // `puts hello` → function head + a `string` token for the bareword
    // arg, not a dropped arg.
    let ks = kinds("puts hello\n", tcl(), &reg());
    assert_eq!(ks.len(), 2, "expected head + arg token; got {ks:?}");
    assert!(
        ks.contains(&(TokenKind::String as u32)),
        "bareword arg not classified as string; got {ks:?}"
    );
}

#[test]
fn builtin_name_as_bareword_arg_is_string() {
    // A builtin name used as a plain dict value stays a string, not a
    // keyword — the KEYWORD role is position-aware (if/try only).
    let src = "dict set frame proc \"asasdas asd\"";
    let proc = decode_words(src, &reg())
        .into_iter()
        .find(|(_, _, _, _, word)| word == "proc")
        .expect("a `proc` token");
    assert_eq!(proc.3, TokenKind::String as u32, "{proc:?}");
}

#[test]
fn variables_classified_as_variable() {
    let s = full("set $x 1\n", tcl(), &reg());
    // The `$x` token kind should be Variable.
    let kinds: Vec<u32> = s.data.chunks(5).map(|c| c[3]).collect();
    assert!(
        kinds.contains(&(TokenKind::Variable as u32)),
        "expected Variable in kinds; got {kinds:?}",
    );
}

/// End-to-end: an `0o17` argument paints as a `Number` under an 8.6
/// document and not under an 8.4 one, where `0o` is not a prefix at all and
/// the word is an ordinary bareword.
#[test]
fn number_tokens_follow_the_document_dialect() {
    let registry = reg();
    let modern = kinds(
        "puts 0o17\n",
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        &registry,
    );
    let old = kinds(
        "puts 0o17\n",
        tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile(),
        &registry,
    );
    assert!(
        modern.contains(&(TokenKind::Number as u32)),
        "0o17 is a number from 8.5: {modern:?}"
    );
    assert!(
        !old.contains(&(TokenKind::Number as u32)),
        "0o17 is not a number under 8.4: {old:?}"
    );
}
