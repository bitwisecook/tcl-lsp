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

fn misreadings(source: &str) -> Vec<PrecedenceHazard> {
    hazards_in("tcl8.6", source, HazardTier::Misreading)
}

fn hazards_in(dialect: &str, source: &str, tier: HazardTier) -> Vec<PrecedenceHazard> {
    capture(dialect, source)
        .precedence_hazards()
        .into_iter()
        .filter(|hazard| hazard.tier == tier)
        .collect()
}

#[test]
fn bitwise_over_comparison_is_a_hazard() {
    let hazards = tcl("$c & 0xff == 0x81").precedence_hazards();
    let [hazard] = hazards.as_slice() else {
        panic!("{hazards:?}");
    };
    assert_eq!(hazard.tier, HazardTier::Misreading);
    assert_eq!(hazard.outer, "&");
    assert_eq!(hazard.inner, "==");
    assert_eq!(hazard.operator, Span::new(3, 4));
    assert_eq!(hazard.as_parsed, "$c & (0xff == 0x81)");
    assert_eq!(hazard.likely_intended, "($c & 0xff) == 0x81");
    assert_eq!(hazard.reason, "'==' binds tighter than '&'");
}

/// Each row: source, tier, as parsed, the other grouping. The groupings
/// were measured on tclsh 8.4.20-9.1.0 and Jim 0.84 (see
/// `evidence/tcl/expr-precedence-groupings.md`).
#[test]
fn every_non_associative_shape_is_classified() {
    use HazardTier::{Misreading as M, NonAssociative as N};
    let rows: &[(&str, HazardTier, &str, &str)] = &[
        ("$a == 1 | $b", M, "($a == 1) | $b", "$a == (1 | $b)"),
        ("$a & $b | $c", M, "($a & $b) | $c", "$a & ($b | $c)"),
        ("$a | $b ^ $c", M, "$a | ($b ^ $c)", "($a | $b) ^ $c"),
        ("$a & $b + 1", M, "$a & ($b + 1)", "($a & $b) + 1"),
        ("$a & $b << 2", M, "$a & ($b << 2)", "($a & $b) << 2"),
        ("1 << $n - 1", M, "1 << ($n - 1)", "(1 << $n) - 1"),
        ("1 << $n * 2", M, "1 << ($n * 2)", "(1 << $n) * 2"),
        ("$a < $b < $c", M, "($a < $b) < $c", "$a < $b && $b < $c"),
        (
            "$a == $b == $c",
            M,
            "($a == $b) == $c",
            "$a == $b && $b == $c",
        ),
        ("$a == $b < $c", M, "$a == ($b < $c)", "$a == $b && $b < $c"),
        ("!$x == 5", M, "(!$x) == 5", "!($x == 5)"),
        ("!$x & 1", M, "(!$x) & 1", "!($x & 1)"),
        ("-2 ** 2", M, "(-2) ** 2", "-(2 ** 2)"),
        ("$a || $b && $c", M, "$a || ($b && $c)", "($a || $b) && $c"),
        (
            "$n + 1 ? $a : $b",
            M,
            "($n + 1) ? $a : $b",
            "$n + (1 ? $a : $b)",
        ),
        ("$a + $b * $c", N, "$a + ($b * $c)", "($a + $b) * $c"),
        ("$a - $b + $c", N, "($a - $b) + $c", "$a - ($b + $c)"),
        ("$a / $b * $c", N, "($a / $b) * $c", "$a / ($b * $c)"),
        ("2 ** 3 ** 2", N, "2 ** (3 ** 2)", "(2 ** 3) ** 2"),
        ("-7 % 3", N, "(-7) % 3", "-(7 % 3)"),
        ("$a == 1 && $b", N, "($a == 1) && $b", "$a == (1 && $b)"),
        (
            "$a < 1 ? $x : $y",
            N,
            "($a < 1) ? $x : $y",
            "$a < (1 ? $x : $y)",
        ),
        (
            "$c ? $a : $b + 1",
            N,
            "$c ? $a : ($b + 1)",
            "($c ? $a : $b) + 1",
        ),
        ("$a == $b == 1", N, "($a == $b) == 1", "$a == ($b == 1)"),
        ("~$m & $x", N, "(~$m) & $x", "~($m & $x)"),
    ];
    for &(source, tier, parsed, other) in rows {
        let hazards = tcl(source).precedence_hazards();
        let [hazard] = hazards.as_slice() else {
            panic!("{source}: {hazards:?}");
        };
        assert_eq!(
            (
                hazard.tier,
                hazard.as_parsed.as_str(),
                hazard.likely_intended.as_str()
            ),
            (tier, parsed, other),
            "{source}"
        );
    }
}

#[test]
fn associative_and_explicit_groupings_are_not_hazards() {
    for source in [
        "$a + $b + $c",
        "$a * $b * $c",
        "$a & $b & $c",
        "$a | $b | $c",
        "$a ^ $b ^ $c",
        "$a && $b && $c",
        "$a || $b || $c",
        "-$a * $b",
        "+$a - $b",
        "($c & 0xff) == 0x81",
        "$c & (0xff == 0x81)",
        "($a == 1) && ($b == 2)",
        "1 << ($n - 1)",
        "(!$x) == 1",
        "!($x == 1)",
        "max($a, 1)",
        "$c ? $a + 1 : $b",
    ] {
        let found = tcl(source).precedence_hazards();
        assert!(found.is_empty(), "{source}: {found:?}");
    }
    assert!(misreadings("!$x == !$y").is_empty());
    let inside_call = tcl("max($a + $b * $c, 1)").precedence_hazards();
    assert_eq!(inside_call[0].as_parsed, "$a + ($b * $c)");
}

#[test]
fn reasons_name_the_rule() {
    let chain = tcl("$a < $b < $c").precedence_hazards();
    assert_eq!(
        chain[0].reason,
        "comparisons do not chain: '<' compares the result of '<'"
    );
    let same = tcl("$a - $b + $c").precedence_hazards();
    assert_eq!(
        same[0].reason,
        "'-' and '+' share a precedence level and group left to right"
    );
    let pow = tcl("2 ** 3 ** 2").precedence_hazards();
    assert_eq!(
        pow[0].reason,
        "'**' and '**' share a precedence level and group right to left"
    );
    let branch = tcl("$c ? $a : $b + 1").precedence_hazards();
    assert_eq!(branch[0].operator, Span::new(3, 4));
}

#[test]
fn nested_hazards_are_each_reported() {
    let hazards = tcl("$a & $b == 1 || $c && $d").precedence_hazards();
    let shapes: Vec<_> = hazards
        .iter()
        .map(|h| (h.tier, h.outer.as_str(), h.inner.as_str()))
        .collect();
    assert_eq!(
        shapes,
        vec![
            (HazardTier::Misreading, "&", "=="),
            (HazardTier::Misreading, "||", "&&"),
            (HazardTier::NonAssociative, "||", "&"),
        ]
    );
}

#[test]
fn f5_word_operators_take_part() {
    let hazards = hazards_in(
        "f5-irules",
        "$flags & 4 contains $x",
        HazardTier::Misreading,
    );
    assert_eq!(hazards[0].inner, "contains");
    let not = hazards_in("f5-irules", "not $a equals \"x\"", HazardTier::Misreading);
    assert_eq!(not[0].likely_intended, "not ($a equals \"x\")");
    let mixed = hazards_in("f5-irules", "$a or $b and $c", HazardTier::Misreading);
    assert_eq!(mixed[0].as_parsed, "$a or ($b and $c)");
    assert!(
        capture("f5-irules", "$a and $b && $c")
            .precedence_hazards()
            .is_empty()
    );
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
    let parens = tcl("($a + $b) * (($c)) - (f($d))")
        .grouping_parens()
        .unwrap();
    let needed: Vec<_> = parens.iter().map(|p| (p.open, p.needed)).collect();
    assert_eq!(
        needed,
        vec![(0, true), (12, false), (13, false), (21, false)]
    );
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
        tcl("$a || $b && $c ? 1 : 2")
            .fully_parenthesised()
            .as_deref(),
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

#[test]
fn focused_tree_marks_the_hovered_operator_or_paren() {
    let expression = tcl("($a + 1) * -$b");
    assert_eq!(
        expression.precedence_tree_at(9).unwrap(),
        "*  (precedence 10)  ◀\n├─ +  (precedence 9)\n│  ├─ $a\n│  └─ 1\n└─ -  (unary)\n   └─ $b\n"
    );
    assert!(
        expression
            .precedence_tree_at(0)
            .unwrap()
            .contains("+  (precedence 9)  ◀")
    );
    assert!(
        expression
            .precedence_tree_at(11)
            .unwrap()
            .contains("-  (unary)  ◀")
    );
    assert_eq!(expression.precedence_tree_at(1), None);
    let ternary = tcl("$a ? 1 : 2");
    assert!(
        ternary
            .precedence_tree_at(3)
            .unwrap()
            .starts_with("?:  (ternary)  ◀")
    );
}
