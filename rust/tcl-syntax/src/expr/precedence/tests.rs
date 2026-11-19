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
use crate::expr::parser::ExprParseContext;

fn capture(dialect: &str, source: &str) -> OriginalSourceExpression {
    let profile = tcl_dialect::DialectProfile::find(dialect).unwrap();
    let context = ExprParseContext::for_profile(profile);
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    OriginalSourceExpression::capture(source, &context, config).unwrap()
}

fn tcl(source: &str) -> OriginalSourceExpression {
    capture("tcl8.6", source)
}

#[test]
fn bitwise_over_comparison_is_a_hazard() {
    let hazards = tcl("$c & 0xff == 0x81").precedence_hazards();
    let [hazard] = hazards.as_slice() else {
        panic!("{hazards:?}");
    };
    assert_eq!(hazard.outer, "&");
    assert_eq!(hazard.inner, "==");
    assert_eq!(hazard.operator, Span::new(3, 4));
    assert_eq!(hazard.as_parsed, "$c & (0xff == 0x81)");
    assert_eq!(hazard.likely_intended, "($c & 0xff) == 0x81");
}

#[test]
fn comparison_on_the_left_of_bitwise_is_a_hazard() {
    let hazards = tcl("$a == 1 | $b").precedence_hazards();
    let [hazard] = hazards.as_slice() else {
        panic!("{hazards:?}");
    };
    assert_eq!(hazard.as_parsed, "($a == 1) | $b");
    assert_eq!(hazard.likely_intended, "$a == (1 | $b)");
}

#[test]
fn explicit_grouping_is_not_a_hazard() {
    for source in [
        "($c & 0xff) == 0x81",
        "$c & (0xff == 0x81)",
        "$a && $b == 1",
        "1 << ($n - 1)",
        "(!$x) == 1",
        "!$x == !$y",
        "!($x == 1)",
    ] {
        assert!(tcl(source).precedence_hazards().is_empty(), "{source}");
    }
}

#[test]
fn shift_over_additive_and_not_over_comparison_are_hazards() {
    let shift = tcl("1 << $n - 1").precedence_hazards();
    assert_eq!(shift[0].as_parsed, "1 << ($n - 1)");
    assert_eq!(shift[0].likely_intended, "(1 << $n) - 1");
    let not = tcl("!$x == 5").precedence_hazards();
    assert_eq!(not[0].as_parsed, "(!$x) == 5");
    assert_eq!(not[0].likely_intended, "!($x == 5)");
}

#[test]
fn f5_word_operators_take_part() {
    let expression = capture("f5-irules", "$flags & 4 contains $x");
    let hazards = expression.precedence_hazards();
    assert_eq!(hazards[0].inner, "contains");
    let not = capture("f5-irules", "not $a equals \"x\"").precedence_hazards();
    assert_eq!(not[0].likely_intended, "not ($a equals \"x\")");
    let grouped = capture("f5-irules", "$a starts_with \"/x\" and $b or $c");
    assert_eq!(
        grouped.fully_parenthesised().as_deref(),
        Some("(($a starts_with \"/x\") and $b) or $c")
    );
    assert_eq!(
        capture("f5-irules", "($a contains \"x\") and ($b)")
            .without_redundant_parens()
            .as_deref(),
        Some("$a contains \"x\" and $b")
    );
}

#[test]
fn grouping_parens_record_whether_the_parse_needs_them() {
    let parens = tcl("($a + $b) * (($c)) - (f($d))").grouping_parens().unwrap();
    let needed: Vec<_> = parens.iter().map(|p| (p.open, p.needed)).collect();
    assert_eq!(needed, vec![(0, true), (12, false), (13, false), (21, false)]);
}

#[test]
fn redundant_parens_are_removed_and_needed_ones_kept() {
    let cases = [
        ("($a * $b) + $c", Some("$a * $b + $c")),
        ("$a - ($b - $c)", None),
        ("(($x))", Some("$x")),
        ("($a)eq($b)", Some("$a eq $b")),
        ("-($a)", Some("-$a")),
        ("2 ** (3 ** 2)", Some("2 ** 3 ** 2")),
        ("(2 ** 3) ** 2", None),
        ("($a ? 1 : 2) ? 3 : 4", None),
        ("$a ? (1) : (2)", Some("$a ? 1 : 2")),
        ("max(($a), 2)", Some("max($a, 2)")),
    ];
    for (source, expected) in cases {
        assert_eq!(
            tcl(source).without_redundant_parens().as_deref(),
            expected,
            "{source}"
        );
    }
}

#[test]
fn fully_parenthesised_makes_every_grouping_explicit() {
    assert_eq!(
        tcl("$c & 0xff == 0x81").fully_parenthesised().as_deref(),
        Some("$c & (0xff == 0x81)")
    );
    assert_eq!(
        tcl("$a + $b * $c - -$d").fully_parenthesised().as_deref(),
        Some("($a + ($b * $c)) - -$d")
    );
    assert_eq!(
        tcl("$a || $b && $c ? 1 : 2").fully_parenthesised().as_deref(),
        Some("($a || ($b && $c)) ? 1 : 2")
    );
    assert_eq!(tcl("$a + 1").fully_parenthesised(), None);
}

#[test]
fn precedence_tree_draws_connectors() {
    let tree = tcl("$c & 0xff == 0x81").precedence_tree().unwrap();
    assert_eq!(
        tree,
        "&  (precedence 5)\n├─ $c\n└─ ==  (precedence 6)\n   ├─ 0xff\n   └─ 0x81\n"
    );
    let nested = tcl("($a + 1) * f($b, -$c)").precedence_tree().unwrap();
    assert_eq!(
        nested,
        "*  (precedence 10)\n├─ +  (precedence 9)\n│  ├─ $a\n│  └─ 1\n└─ f()\n   ├─ $b\n   └─ -  (unary)\n      └─ $c\n"
    );
}

#[test]
fn render_keeps_a_ternary_condition_grouped() {
    let tree = tcl("($a ? 1 : 0) ? 2 : 3");
    let rendered = render_expr(tree.root());
    assert_eq!(rendered, "($a ? 1 : 0) ? 2 : 3");
}
