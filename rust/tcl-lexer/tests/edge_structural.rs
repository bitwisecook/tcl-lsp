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

/// Edge-case tests for the structural index — verifies behavior at boundaries.
use tcl_lexer::{
    BraceIndex, BracketIndex, ExprParenIndex, ParenBalance, command_boundaries, reparse_window,
    script_is_complete,
};

// BracketIndex

#[test]
fn bracket_empty_source() {
    let idx = BracketIndex::build("");
    assert_eq!(idx.unterminated_count(), 0);
    assert!(idx.close_bracket_balances(0));
}

#[test]
fn bracket_lone_close_clamped() {
    let idx = BracketIndex::build("]");
    assert_eq!(idx.unterminated_count(), 0);
    assert!(idx.close_bracket_balances(0));
    assert!(idx.close_bracket_balances(1));
}

#[test]
fn bracket_simple_open_needs_close_at_end() {
    let idx = BracketIndex::build("[");
    assert_eq!(idx.unterminated_count(), 1);
    assert!(idx.close_bracket_balances(1));
    assert!(!idx.close_bracket_balances(0));
}

#[test]
fn bracket_balanced_adjacent() {
    let idx = BracketIndex::build("[]");
    assert_eq!(idx.unterminated_count(), 0);
    assert!(idx.close_bracket_balances(0));
}

#[test]
fn bracket_inside_brace_word_is_literal() {
    let idx = BracketIndex::build("{ [ }");
    assert_eq!(idx.unterminated_count(), 0);
}

#[test]
fn bracket_in_quote_is_structural() {
    let idx = BracketIndex::build("\"[a]\"");
    assert_eq!(idx.unterminated_count(), 0);
    assert!(!idx.is_inert(1)); // [ at pos 1 inside quotes — NOT inert
}

#[test]
fn bracket_escape_pair_is_inert() {
    let idx = BracketIndex::build(r"\[");
    assert_eq!(idx.unterminated_count(), 0);
    assert!(idx.is_inert(0));
    assert!(idx.is_inert(1));
}

#[test]
fn bracket_dollar_brace_is_inert() {
    let idx = BracketIndex::build("${foo]bar}");
    let pos = "${foo".len();
    assert!(idx.is_inert(u32::try_from(pos).expect("offset fits u32")));
}

#[test]
fn bracket_cmd_sub_brace_run_is_inert() {
    let idx = BracketIndex::build("[foo {bar] baz}]");
    assert_eq!(idx.unterminated_count(), 0);
    assert!(idx.is_inert(u32::try_from("[foo {bar".len()).expect("offset fits u32")));
}

#[test]
fn bracket_unterminated_cmd_sub() {
    let idx = BracketIndex::build("set x [foo bar");
    assert_eq!(idx.unterminated_count(), 1);
    assert!(idx.close_bracket_balances(13));
}

#[test]
fn bracket_nested_unterminated() {
    let idx = BracketIndex::build("[a [b");
    assert_eq!(idx.unterminated_count(), 2);
    assert!(!idx.close_bracket_balances(6));
}

#[test]
fn bracket_extra_close_clamps() {
    let idx = BracketIndex::build("a]");
    assert_eq!(idx.unterminated_count(), 0);
}

#[test]
fn bracket_unterminated_brace_swallows_tail() {
    let idx = BracketIndex::build("[foo {bar");
    assert_eq!(idx.unterminated_count(), 1);
    // EOF is inside the unterminated brace word — inert
    assert!(
        !idx.close_bracket_balances(u32::try_from("[foo {bar".len()).expect("offset fits u32"))
    );
    // But inserting right after [ should balance
    assert!(idx.close_bracket_balances(1));
}

// BraceIndex

#[test]
fn brace_empty() {
    let idx = BraceIndex::build("");
    assert_eq!(idx.unterminated_count(), 0);
}

#[test]
fn brace_mid_word_is_literal() {
    let idx = BraceIndex::build("a{");
    assert_eq!(idx.unterminated_count(), 0);
}

#[test]
fn brace_simple_open() {
    let idx = BraceIndex::build("{");
    assert_eq!(idx.unterminated_count(), 1);
    assert!(idx.close_brace_balances(1));
}

#[test]
fn brace_comment_skips() {
    let idx = BraceIndex::build("# { not a brace\n");
    assert_eq!(idx.unterminated_count(), 0);
}

#[test]
fn brace_nested_balanced() {
    let idx = BraceIndex::build("{{a}}");
    assert_eq!(idx.unterminated_count(), 0);
}

#[test]
fn brace_unterminated_nested() {
    let idx = BraceIndex::build("{a {b");
    assert_eq!(idx.unterminated_count(), 2);
    assert!(!idx.close_brace_balances(6));
}

#[test]
fn brace_backslash_does_not_close() {
    let idx = BraceIndex::build(r"{\}}");
    assert_eq!(idx.unterminated_count(), 0);
}

// ExprParenIndex

#[test]
fn expr_empty() {
    let idx = ExprParenIndex::build("");
    assert_eq!(idx.balance(), ParenBalance::Balanced);
    assert_eq!(idx.unmatched_opens(), 0);
}

#[test]
fn expr_simple_balanced() {
    let idx = ExprParenIndex::build("(1 + 2) * 3");
    assert_eq!(idx.balance(), ParenBalance::Balanced);
}

#[test]
fn expr_open_heavy() {
    let idx = ExprParenIndex::build("(1 + (2 - 3)");
    assert_eq!(idx.balance(), ParenBalance::OpenHeavy);
    assert_eq!(idx.unmatched_opens(), 1);
}

#[test]
fn expr_close_heavy() {
    let idx = ExprParenIndex::build("1 + 2) * 3");
    assert_eq!(idx.balance(), ParenBalance::CloseHeavy);
}

#[test]
fn expr_string_is_opaque() {
    let idx = ExprParenIndex::build(r#""a(b""#);
    assert_eq!(idx.balance(), ParenBalance::Balanced);
}

#[test]
fn expr_comment_is_opaque() {
    // A TIP 582 `#` comment is one whole token, so a paren inside it never
    // becomes a `ParenOpen`/`ParenClose` event and cannot skew the balance.
    let idx = ExprParenIndex::build("(1 + 2) # )");
    assert_eq!(idx.balance(), ParenBalance::Balanced);
    let idx = ExprParenIndex::build("(1 + 2) # (");
    assert_eq!(idx.balance(), ParenBalance::Balanced);
    assert_eq!(idx.unmatched_opens(), 0);
    // A genuinely unbalanced paren before the comment is still reported.
    let idx = ExprParenIndex::build("(1 + 2 # )");
    assert_eq!(idx.balance(), ParenBalance::OpenHeavy);
}

#[test]
fn expr_close_paren_balances() {
    let idx = ExprParenIndex::build("(1 + 2");
    assert_eq!(idx.balance(), ParenBalance::OpenHeavy);
    assert!(idx.close_paren_balances(6));
}

// script_is_complete

#[test]
fn complete_simple() {
    assert!(script_is_complete("puts hello"));
    assert!(script_is_complete("set x 1\nputs $x\n"));
}

#[test]
fn incomplete_open_brace() {
    assert!(!script_is_complete("set x {"));
}

#[test]
fn incomplete_open_bracket() {
    assert!(!script_is_complete("set x [expr {1 +"));
}

#[test]
fn incomplete_open_quote() {
    assert!(!script_is_complete("set x \"hello"));
}

#[test]
fn complete_with_comment() {
    assert!(script_is_complete("puts hi\n# comment\nset x 1\n"));
}

#[test]
fn incomplete_continuation() {
    assert!(!script_is_complete("puts hello\\\n"));
}

#[test]
fn complete_empty() {
    assert!(script_is_complete(""));
}

#[test]
fn complete_cmd_sub_with_brace_word() {
    // [set x {b}{] — complete because [ interior parses as recursive script
    assert!(script_is_complete("[set x {b}{]"));
}

// command_boundaries / reparse_window

#[test]
fn boundaries_simple() {
    // "set x 1\nputs hi" = 15 bytes, boundaries at end of each command.
    let b = command_boundaries("set x 1\nputs hi");
    assert_eq!(b, vec![8, 15]);
}

#[test]
fn boundaries_semicolon() {
    // "set x 1; puts hi" = 16 bytes (; then space after, unlike \n)
    let b = command_boundaries("set x 1; puts hi");
    assert_eq!(b, vec![8, 16]);
}

#[test]
fn boundaries_skip_braces() {
    let src = "set x {\n}\nputs hi";
    let b = command_boundaries(src);
    assert_eq!(
        *b.last().unwrap(),
        u32::try_from(src.len()).expect("offset fits u32")
    );
}

#[test]
fn boundaries_skip_cmd_sub() {
    let src = "if {[info exists x]} { puts hi }\nset y 1";
    let b = command_boundaries(src);
    assert_eq!(
        *b.last().unwrap(),
        u32::try_from(src.len()).expect("offset fits u32")
    );
}

#[test]
fn reparse_window_snaps_to_command() {
    let src = "set x 1\nputs hi\nset y 2\n";
    let (s, e) = reparse_window(src, 9, 9);
    assert_eq!(&src[s as usize..e as usize], "puts hi\n");
}

#[test]
fn reparse_window_multi_line_edit() {
    let src = "set x 1\nputs hi\nset y 2\n";
    let (s, e) = reparse_window(src, 9, 14);
    assert_eq!(&src[s as usize..e as usize], "puts hi\n");
}

#[test]
fn reparse_window_cross_command() {
    let src = "set x 1\nputs hi\nset y 2\n";
    let (s, e) = reparse_window(src, 6, 13);
    assert_eq!(&src[s as usize..e as usize], "set x 1\nputs hi\n");
}

#[test]
fn reparse_window_past_eof_clamps() {
    // Edit entirely past EOF: both start and end clamp to len,
    // resulting in an empty window (nothing changed in the source).
    let src = "set x 1\n";
    let (s, e) = reparse_window(src, 10, 20);
    assert_eq!(&src[s as usize..e as usize], "");
}
