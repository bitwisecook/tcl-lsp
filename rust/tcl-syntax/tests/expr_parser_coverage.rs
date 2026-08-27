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

//! Exhaustive coverage for the `tcl-syntax` `expr` precedence-climbing
//! parser (`tcl_syntax::expr::parse_expr`).
//!
//! Companion to `expr_parser.rs` (which covers the basics). This file
//! drives the REMAINING branches of [`parser.rs`] / [`ast.rs`] toward 100%
//! line coverage: the full operator set and their precedence/associativity,
//! unary chains, nested ternary, every math function arity, all literal forms,
//! string / quote / brace operands, `$var` / `${var}` / array / namespace refs,
//! `[cmd]` substitutions, and especially the ERROR / RECOVERY paths
//! (`ExprNode::Raw` fallbacks for malformed / unbalanced / dangling input).
//!
//! C-Tcl proof. Operator semantics, precedence, and associativity are
//! Tcl-defined; the parse tree's *grouping* is internal, so we assert it
//! structurally and pin the shape to the VALUE real `tclsh` computes for the
//! same expression (checked with `scripts/dev/tclsh_check.sh`, 8.6 + 9.0).
//! Each non-trivial case cites the value with `// tclsh:`. Representative
//! ground truth (8.6 unless noted):
//!   expr {2-3-4}        = -5    (left-assoc  ⇒ Sub(Sub(2,3),4))
//!   expr {2**3**2}      = 512   (right-assoc ⇒ Pow(2,Pow(3,2)))
//!   expr {1+2*3}        = 7     (× tighter   ⇒ Add(1,Mul(2,3)))
//!   expr {7 % -3}       = -2    (Tcl floor mod)
//!   expr {-2**2}        = 4     (** tighter than unary - ⇒ Neg(Pow(2,2)))
//!   expr {1<<2+1}       = 8     (+ tighter than <<     ⇒ LShift(1,Add(2,1)))
//!   expr {1|2&3}        = 3     (& tighter than |      ⇒ BitOr(1,BitAnd(2,3)))
//!   expr {5 > 3 == 1}   = 1     (> tighter than ==     ⇒ Eq(Gt(5,3),1))
//!   expr {1 || 0 && 0}  = 1     (&& tighter than ||    ⇒ Or(1,And(0,0)))
//!   expr {1 + 1 eq 2}   = 1     (+ tighter than eq     ⇒ StrEq(Add(1,1),2))
//! The `lt le gt ge` string operators are 9.0-ONLY (8.6 errors with "invalid
//! bareword"); the Rust parser models the latest dialect, so we parse them and
//! pin values to 9.0 only, noting the 8.6 divergence at each site.

use std::collections::HashSet;

use tcl_syntax::expr::ast::render_expr;
use tcl_syntax::expr::{BinOp, ExprNode, UnaryOp, parse_expr};

// -- Harness ----------------------------------------------------------------

/// Parse with the default dialect.
fn p(src: &str) -> ExprNode {
    parse_expr(src, None)
}

/// Parse with the f5-irules dialect (word/string operators).
fn pi(src: &str) -> ExprNode {
    parse_expr(src, Some("f5-irules"))
}

fn lit_text(n: &ExprNode) -> &str {
    match n {
        ExprNode::Literal { text, .. } => text,
        other => panic!("expected Literal, got {other:?}"),
    }
}

fn binop(n: &ExprNode) -> BinOp {
    match n {
        ExprNode::Binary { op, .. } => *op,
        other => panic!("expected Binary, got {other:?}"),
    }
}

fn unop(n: &ExprNode) -> UnaryOp {
    match n {
        ExprNode::Unary { op, .. } => *op,
        other => panic!("expected Unary, got {other:?}"),
    }
}

/// Left child of a Binary node.
fn left(n: &ExprNode) -> &ExprNode {
    match n {
        ExprNode::Binary { left, .. } => left,
        other => panic!("expected Binary, got {other:?}"),
    }
}

/// Right child of a Binary node.
fn right(n: &ExprNode) -> &ExprNode {
    match n {
        ExprNode::Binary { right, .. } => right,
        other => panic!("expected Binary, got {other:?}"),
    }
}

/// Operand of a Unary node.
fn operand(n: &ExprNode) -> &ExprNode {
    match n {
        ExprNode::Unary { operand, .. } => operand,
        other => panic!("expected Unary, got {other:?}"),
    }
}

fn is_raw(n: &ExprNode) -> bool {
    matches!(n, ExprNode::Raw { .. })
}

// ===========================================================================
// 1. Every binary operator maps to its BinOp variant.
// ===========================================================================

#[test]
fn all_arithmetic_operators() {
    assert_eq!(binop(&p("$a + $b")), BinOp::Add);
    assert_eq!(binop(&p("$a - $b")), BinOp::Sub);
    assert_eq!(binop(&p("$a * $b")), BinOp::Mul);
    assert_eq!(binop(&p("$a / $b")), BinOp::Div);
    assert_eq!(binop(&p("$a % $b")), BinOp::Mod);
    assert_eq!(binop(&p("$a ** $b")), BinOp::Pow);
}

#[test]
fn all_shift_and_bitwise_operators() {
    assert_eq!(binop(&p("$a << $b")), BinOp::LShift);
    assert_eq!(binop(&p("$a >> $b")), BinOp::RShift);
    assert_eq!(binop(&p("$a & $b")), BinOp::BitAnd);
    assert_eq!(binop(&p("$a | $b")), BinOp::BitOr);
    assert_eq!(binop(&p("$a ^ $b")), BinOp::BitXor);
}

#[test]
fn all_logical_operators() {
    assert_eq!(binop(&p("$a && $b")), BinOp::And);
    assert_eq!(binop(&p("$a || $b")), BinOp::Or);
}

#[test]
fn all_numeric_comparison_operators() {
    assert_eq!(binop(&p("$a == $b")), BinOp::Eq);
    assert_eq!(binop(&p("$a != $b")), BinOp::Ne);
    assert_eq!(binop(&p("$a < $b")), BinOp::Lt);
    assert_eq!(binop(&p("$a <= $b")), BinOp::Le);
    assert_eq!(binop(&p("$a > $b")), BinOp::Gt);
    assert_eq!(binop(&p("$a >= $b")), BinOp::Ge);
}

#[test]
fn all_string_comparison_operators() {
    // `eq ne` are valid in both 8.6 and 9.0.
    assert_eq!(binop(&p("$a eq $b")), BinOp::StrEq);
    assert_eq!(binop(&p("$a ne $b")), BinOp::StrNe);
    // `lt le gt ge` are 9.0-ONLY string operators (8.6 raises "invalid
    // bareword"); the parser models the latest dialect and accepts them.
    assert_eq!(binop(&p("$a lt $b")), BinOp::StrLt);
    assert_eq!(binop(&p("$a le $b")), BinOp::StrLe);
    assert_eq!(binop(&p("$a gt $b")), BinOp::StrGt);
    assert_eq!(binop(&p("$a ge $b")), BinOp::StrGe);
}

#[test]
fn list_membership_operators() {
    assert_eq!(binop(&p("$x in $list")), BinOp::In);
    assert_eq!(binop(&p("$x ni $list")), BinOp::Ni);
}

#[test]
fn irules_word_and_string_operators() {
    // The word/iRules operators only tokenise as operators under the
    // f5-irules dialect.
    assert_eq!(binop(&pi("$a and $b")), BinOp::WordAnd);
    assert_eq!(binop(&pi("$a or $b")), BinOp::WordOr);
    assert_eq!(binop(&pi(r#"$u contains "x""#)), BinOp::Contains);
    assert_eq!(binop(&pi(r#"$u starts_with "x""#)), BinOp::StartsWith);
    assert_eq!(binop(&pi(r#"$u ends_with "x""#)), BinOp::EndsWith);
    assert_eq!(binop(&pi(r#"$u equals "x""#)), BinOp::StrEquals);
    assert_eq!(binop(&pi(r#"$u matches "x""#)), BinOp::Matches);
    assert_eq!(binop(&pi(r#"$u matches_glob "x*""#)), BinOp::MatchesGlob);
    assert_eq!(binop(&pi(r#"$u matches_regex "x.""#)), BinOp::MatchesRegex);
}

#[test]
fn irules_word_not_is_unary() {
    // `not $x` under f5-irules is the word-based unary NOT.
    assert_eq!(unop(&pi("not $x")), UnaryOp::WordNot);
}

// ===========================================================================
// 2. Every unary operator + unary chains.
// ===========================================================================

#[test]
fn all_unary_operators() {
    assert_eq!(unop(&p("-$x")), UnaryOp::Neg);
    assert_eq!(unop(&p("+$x")), UnaryOp::Pos);
    assert_eq!(unop(&p("!$x")), UnaryOp::Not);
    assert_eq!(unop(&p("~$x")), UnaryOp::BitNot);
}

#[test]
fn unary_not_chain() {
    // !!5 = 1 ⇒ Not(Not(5)) — unary operators are prefix and stack.
    // tclsh: expr {!!5} == 1
    let n = p("!!5");
    assert_eq!(unop(&n), UnaryOp::Not);
    assert_eq!(unop(operand(&n)), UnaryOp::Not);
    assert_eq!(lit_text(operand(operand(&n))), "5");
}

#[test]
fn unary_bitnot_chain() {
    // ~~5 = 5 ⇒ BitNot(BitNot(5)).
    // tclsh: expr {~~5} == 5
    let n = p("~~5");
    assert_eq!(unop(&n), UnaryOp::BitNot);
    assert_eq!(unop(operand(&n)), UnaryOp::BitNot);
}

#[test]
fn unary_neg_chain() {
    // --5 = 5 ⇒ Neg(Neg(5)) (Tcl has no decrement operator; two prefix
    // minuses). tclsh: expr {--5} == 5
    let n = p("--5");
    assert_eq!(unop(&n), UnaryOp::Neg);
    assert_eq!(unop(operand(&n)), UnaryOp::Neg);
}

#[test]
fn unary_pos_chain() {
    // ++5 ⇒ Pos(Pos(5)).
    let n = p("++5");
    assert_eq!(unop(&n), UnaryOp::Pos);
    assert_eq!(unop(operand(&n)), UnaryOp::Pos);
}

#[test]
fn mixed_unary_chain() {
    // -~!$x ⇒ Neg(BitNot(Not($x))) — distinct prefix operators stack in
    // source order.
    let n = p("-~!$x");
    assert_eq!(unop(&n), UnaryOp::Neg);
    assert_eq!(unop(operand(&n)), UnaryOp::BitNot);
    assert_eq!(unop(operand(operand(&n))), UnaryOp::Not);
}

#[test]
fn negative_literal_is_unary_neg_over_literal() {
    // -7 is Unary(Neg, Literal "7"), not a signed literal.
    let n = p("-7");
    assert_eq!(unop(&n), UnaryOp::Neg);
    assert_eq!(lit_text(operand(&n)), "7");
}

// ===========================================================================
// 3. Precedence + associativity (each pinned to a tclsh value).
// ===========================================================================

#[test]
fn mul_binds_tighter_than_add() {
    // 1 + 2 * 3 = 7 ⇒ Add(1, Mul(2,3)). tclsh: expr {1+2*3} == 7
    let n = p("1 + 2 * 3");
    assert_eq!(binop(&n), BinOp::Add);
    assert_eq!(binop(right(&n)), BinOp::Mul);
    assert_eq!(lit_text(left(&n)), "1");
}

#[test]
fn add_binds_tighter_than_shift() {
    // 1 << 2 + 1 = 8 ⇒ LShift(1, Add(2,1)) (+ tighter than <<).
    // tclsh: expr {1<<2+1} == 8  (== 1<<3). If << bound tighter it would be
    // (1<<2)+1 == 5.
    let n = p("1 << 2 + 1");
    assert_eq!(binop(&n), BinOp::LShift);
    assert_eq!(binop(right(&n)), BinOp::Add);
}

#[test]
fn bitand_binds_tighter_than_bitor() {
    // 1 | 2 & 3 = 3 ⇒ BitOr(1, BitAnd(2,3)). tclsh: expr {1|2&3} == 3
    let n = p("1 | 2 & 3");
    assert_eq!(binop(&n), BinOp::BitOr);
    assert_eq!(binop(right(&n)), BinOp::BitAnd);
}

#[test]
fn bitxor_between_bitor_and_bitand() {
    // 1 ^ 2 | 4 = 7 ⇒ BitOr(BitXor(1,2), 4) (^ tighter than |).
    // tclsh: expr {1^2|4} == 7
    let n = p("1 ^ 2 | 4");
    assert_eq!(binop(&n), BinOp::BitOr);
    assert_eq!(binop(left(&n)), BinOp::BitXor);
    // 5 & 3 | 8 = 9 ⇒ BitOr(BitAnd(5,3), 8). tclsh: expr {5 & 3 | 8} == 9
    let m = p("5 & 3 | 8");
    assert_eq!(binop(&m), BinOp::BitOr);
    assert_eq!(binop(left(&m)), BinOp::BitAnd);
}

#[test]
fn comparison_binds_tighter_than_equality() {
    // 5 > 3 == 1 = 1 ⇒ Eq(Gt(5,3), 1). tclsh: expr {5 > 3 == 1} == 1
    let n = p("5 > 3 == 1");
    assert_eq!(binop(&n), BinOp::Eq);
    assert_eq!(binop(left(&n)), BinOp::Gt);
}

#[test]
fn arithmetic_binds_tighter_than_equality() {
    // 1 + 2 == 3 = 1 ⇒ Eq(Add(1,2), 3). tclsh: expr {1 + 2 == 3} == 1
    let n = p("1 + 2 == 3");
    assert_eq!(binop(&n), BinOp::Eq);
    assert_eq!(binop(left(&n)), BinOp::Add);
}

#[test]
fn arithmetic_binds_tighter_than_string_eq() {
    // 1 + 1 eq 2 = 1 ⇒ StrEq(Add(1,1), 2) (+ tighter than eq).
    // tclsh: expr {1 + 1 eq 2} == 1
    let n = p("1 + 1 eq 2");
    assert_eq!(binop(&n), BinOp::StrEq);
    assert_eq!(binop(left(&n)), BinOp::Add);
}

#[test]
fn and_binds_tighter_than_or() {
    // 1 || 0 && 0 = 1 ⇒ Or(1, And(0,0)). tclsh: expr {1 || 0 && 0} == 1
    // (if || bound tighter it would be And(Or(1,0),0) == 0).
    let n = p("1 || 0 && 0");
    assert_eq!(binop(&n), BinOp::Or);
    assert_eq!(binop(right(&n)), BinOp::And);
}

#[test]
fn equality_binds_tighter_than_logical_and() {
    // 3 == 3 && 4 == 4 = 1 ⇒ And(Eq(3,3), Eq(4,4)).
    // tclsh: expr {3 == 3 && 4 == 4} == 1
    let n = p("3 == 3 && 4 == 4");
    assert_eq!(binop(&n), BinOp::And);
    assert_eq!(binop(left(&n)), BinOp::Eq);
    assert_eq!(binop(right(&n)), BinOp::Eq);
}

#[test]
fn subtraction_is_left_associative() {
    // 2 - 3 - 4 = -5 ⇒ Sub(Sub(2,3),4). tclsh: expr {2-3-4} == -5
    // (right-assoc would be Sub(2,Sub(3,4)) == 3).
    let n = p("2 - 3 - 4");
    assert_eq!(binop(&n), BinOp::Sub);
    assert_eq!(binop(left(&n)), BinOp::Sub);
    assert_eq!(lit_text(right(&n)), "4");
}

#[test]
fn division_is_left_associative() {
    // 8 / 4 / 2 = 1 ⇒ Div(Div(8,4),2). tclsh: expr {8 / 4 / 2} == 1
    // (right-assoc would be Div(8,Div(4,2)) == 4).
    let n = p("8 / 4 / 2");
    assert_eq!(binop(&n), BinOp::Div);
    assert_eq!(binop(left(&n)), BinOp::Div);
}

#[test]
fn mul_and_mod_same_precedence_left_assoc() {
    // 2 * 3 % 4 = 2 ⇒ Mod(Mul(2,3),4). tclsh: expr {2 * 3 % 4} == 2
    let n = p("2 * 3 % 4");
    assert_eq!(binop(&n), BinOp::Mod);
    assert_eq!(binop(left(&n)), BinOp::Mul);
}

#[test]
fn power_is_right_associative() {
    // 2 ** 3 ** 2 = 512 ⇒ Pow(2, Pow(3,2)). tclsh: expr {2**3**2} == 512
    // (left-assoc would be Pow(Pow(2,3),2) == 64).
    let n = p("2 ** 3 ** 2");
    assert_eq!(binop(&n), BinOp::Pow);
    assert_eq!(binop(right(&n)), BinOp::Pow);
    assert_eq!(lit_text(left(&n)), "2");
}

#[test]
fn unary_minus_binds_tighter_than_power() {
    // Despite the Tcl man page listing `**` "below unary", the real Tcl
    // bytecode compiler binds unary minus TIGHTER than `**`: it reads
    // `-N ** M` as `(-N) ** M`, not `-(N ** M)`. Proof from tclsh (8.6+9.0):
    //   expr {-3**2}  == 9   (== (-3)**2 == 9, NOT -(3**2) == -9)
    //   expr {-2**2}  == 4   (== (-2)**2)
    //   expr {-2**3}  == -8  (both readings coincide here)
    // The Rust parser matches: prefix() parses the unary operand at UNARY_BP
    // (24), and `**` has left_bp 23 < 24, so `**` does NOT swallow the unary —
    // giving Pow(Neg(2), 2). This is the structurally correct tree.
    let n = p("-2 ** 2");
    assert_eq!(binop(&n), BinOp::Pow);
    assert_eq!(unop(left(&n)), UnaryOp::Neg);
    assert_eq!(lit_text(operand(left(&n))), "2");
    assert_eq!(lit_text(right(&n)), "2");
}

#[test]
fn unary_minus_distributes_over_add() {
    // -$a + $b ⇒ Add(Neg(a), b): the unary binds only its immediate operand,
    // looser-binding `+` wraps it.
    let n = p("-$a + $b");
    assert_eq!(binop(&n), BinOp::Add);
    assert_eq!(unop(left(&n)), UnaryOp::Neg);
}

#[test]
fn parentheses_override_precedence() {
    // (2 + 3) * 4 = 20 ⇒ Mul(Add(2,3), 4). tclsh: expr {(2+3)*4} == 20
    let n = p("(2 + 3) * 4");
    assert_eq!(binop(&n), BinOp::Mul);
    assert_eq!(binop(left(&n)), BinOp::Add);
}

#[test]
fn nested_parentheses_collapse_to_inner() {
    // (((1 + 2))) ⇒ Add(1,2): redundant parens add no nodes.
    let n = p("(((1 + 2)))");
    assert_eq!(binop(&n), BinOp::Add);
}

#[test]
fn parens_then_unary_then_pow() {
    // ~(1 | 2) = -4 ⇒ BitNot(BitOr(1,2)). tclsh: expr {~(1|2)} == -4
    let n = p("~(1 | 2)");
    assert_eq!(unop(&n), UnaryOp::BitNot);
    assert_eq!(binop(operand(&n)), BinOp::BitOr);
}

#[test]
fn full_precedence_ladder_root_is_ternary() {
    // A maximal mixed expression: the ternary is the loosest operator, so it
    // must sit at the root with everything else below it.
    let n = p("$a || $b && $c | $d ^ $e & $f == $g < $h << $i + $j * $k ** $l ? 1 : 0");
    assert!(matches!(n, ExprNode::Ternary { .. }));
    if let ExprNode::Ternary { condition, .. } = &n {
        // Loosest binary under the ternary condition is ||.
        assert_eq!(binop(condition), BinOp::Or);
    }
}

// ===========================================================================
// 4. Ternary (structure, nesting, right-associativity).
// ===========================================================================

#[test]
fn ternary_basic_structure() {
    // 1 ? 2 : 3 = 2 ⇒ Ternary(1, 2, 3). tclsh: expr {1 ? 2 : 3} == 2
    match p("1 ? 2 : 3") {
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            assert_eq!(lit_text(&condition), "1");
            assert_eq!(lit_text(&true_branch), "2");
            assert_eq!(lit_text(&false_branch), "3");
        }
        other => panic!("expected Ternary, got {other:?}"),
    }
}

#[test]
fn ternary_is_right_associative() {
    // 1 ? 2 : 0 ? 3 : 4 = 2 ⇒ Ternary(1, 2, Ternary(0, 3, 4)).
    // tclsh: expr {1 ? 2 : 0 ? 3 : 4} == 2
    match p("1 ? 2 : 0 ? 3 : 4") {
        ExprNode::Ternary { false_branch, .. } => {
            assert!(matches!(*false_branch, ExprNode::Ternary { .. }));
        }
        other => panic!("expected Ternary, got {other:?}"),
    }
}

#[test]
fn ternary_condition_can_be_complex() {
    // ($a < $b) ? $a : $b — the condition is itself a comparison.
    match p("$a < $b ? $a : $b") {
        ExprNode::Ternary { condition, .. } => assert_eq!(binop(&condition), BinOp::Lt),
        other => panic!("expected Ternary, got {other:?}"),
    }
}

#[test]
fn ternary_branches_can_be_expressions() {
    // $x ? $a + 1 : $b * 2 — both arms parse as their own subtrees.
    match p("$x ? $a + 1 : $b * 2") {
        ExprNode::Ternary {
            true_branch,
            false_branch,
            ..
        } => {
            assert_eq!(binop(&true_branch), BinOp::Add);
            assert_eq!(binop(&false_branch), BinOp::Mul);
        }
        other => panic!("expected Ternary, got {other:?}"),
    }
}

// ===========================================================================
// 5. Function calls: every documented arity, nesting, expr args.
// ===========================================================================

#[test]
fn zero_arg_functions() {
    // rand() has no arguments. tclsh: expr {rand()} is a float in [0,1).
    match p("rand()") {
        ExprNode::Call { function, args, .. } => {
            assert_eq!(function, "rand");
            assert!(args.is_empty());
        }
        other => panic!("expected Call, got {other:?}"),
    }
}

#[test]
fn one_arg_functions() {
    for (src, name) in [
        ("abs($x)", "abs"),
        ("sqrt($x)", "sqrt"),
        ("int($x)", "int"),
        ("double($x)", "double"),
        ("ceil($x)", "ceil"),
        ("floor($x)", "floor"),
        ("round($x)", "round"),
        ("sin($x)", "sin"),
        ("cos($x)", "cos"),
        ("tan($x)", "tan"),
        ("exp($x)", "exp"),
        ("log($x)", "log"),
        ("log10($x)", "log10"),
        ("bool($x)", "bool"),
        ("wide($x)", "wide"),
        ("entier($x)", "entier"),
        ("isqrt($x)", "isqrt"),
        ("srand($x)", "srand"),
        ("isinf($x)", "isinf"),
        ("isnan($x)", "isnan"),
    ] {
        match p(src) {
            ExprNode::Call { function, args, .. } => {
                assert_eq!(function, name, "{src}");
                assert_eq!(args.len(), 1, "{src}");
            }
            other => panic!("{src}: expected Call, got {other:?}"),
        }
    }
}

#[test]
fn two_arg_functions() {
    for (src, name) in [
        ("pow($a, $b)", "pow"),
        ("atan2($y, $x)", "atan2"),
        ("hypot($a, $b)", "hypot"),
        ("fmod($a, $b)", "fmod"),
    ] {
        match p(src) {
            ExprNode::Call { function, args, .. } => {
                assert_eq!(function, name, "{src}");
                assert_eq!(args.len(), 2, "{src}");
            }
            other => panic!("{src}: expected Call, got {other:?}"),
        }
    }
}

#[test]
fn variadic_min_max() {
    // min/max take any positive arity. tclsh: expr {min(3,1,2)} == 1,
    // expr {max(3,1,2)} == 3.
    match p("min(3, 1, 2)") {
        ExprNode::Call { function, args, .. } => {
            assert_eq!(function, "min");
            assert_eq!(args.len(), 3);
        }
        other => panic!("{other:?}"),
    }
    match p("max(1, 2, 3, 4, 5)") {
        ExprNode::Call { args, .. } => assert_eq!(args.len(), 5),
        other => panic!("{other:?}"),
    }
}

#[test]
fn function_argument_is_full_expression() {
    // abs($a - $b) ⇒ Call("abs", [Sub(a,b)]).
    match p("abs($a - $b)") {
        ExprNode::Call { args, .. } => {
            assert_eq!(args.len(), 1);
            assert_eq!(binop(&args[0]), BinOp::Sub);
        }
        other => panic!("expected Call, got {other:?}"),
    }
}

#[test]
fn nested_function_calls() {
    // min(5, abs(-2)) = 2 ⇒ second arg is itself a Call.
    // tclsh: expr {min(5, abs(-2))} == 2
    match p("min(5, abs(-2))") {
        ExprNode::Call { args, .. } => {
            assert_eq!(args.len(), 2);
            assert!(matches!(args[1], ExprNode::Call { .. }));
        }
        other => panic!("expected Call, got {other:?}"),
    }
}

#[test]
fn function_call_inside_arithmetic() {
    // abs(-5) + 1 = 6 ⇒ Add(Call("abs",[Neg(5)]), 1).
    // tclsh: expr {abs(-5) + 1} == 6
    let n = p("abs(-5) + 1");
    assert_eq!(binop(&n), BinOp::Add);
    assert!(matches!(left(&n), ExprNode::Call { .. }));
}

#[test]
fn function_call_as_operand_keeps_precedence() {
    // max(1,2) * 3 = 6 ⇒ Mul(Call, 3). tclsh: expr {max(1,2) * 3} == 6
    let n = p("max(1, 2) * 3");
    assert_eq!(binop(&n), BinOp::Mul);
    assert!(matches!(left(&n), ExprNode::Call { .. }));
}

// ===========================================================================
// 6. Literals — every numeric form, booleans, IEEE specials.
// ===========================================================================

#[test]
fn integer_literal_forms() {
    for s in ["0", "42", "123456789012345", "9223372036854775807"] {
        // Each parses to a single Literal node carrying the verbatim text.
        // (Tcl 9.0 also accepts digit-grouped `1_000`, but the expr lexer's
        // plain-decimal scan stops at `_`, so that form is not a single
        // Number token here — exercised in the Raw-recovery tests instead.)
        assert_eq!(lit_text(&p(s)), s, "{s}");
    }
}

#[test]
fn radix_literal_forms() {
    // tclsh: 0xFF==255, 0o17==15, 0b101==5. Octal `017` is 15 on 8.6 but 17
    // on 9.0 (legacy-octal removed) — the parser is dialect-agnostic here and
    // simply preserves the digits in a single Literal node.
    for s in [
        "0xFF", "0xff", "0Xff", "0o17", "0O17", "0b101", "0B101", "017",
    ] {
        assert_eq!(lit_text(&p(s)), s, "{s}");
    }
}

#[test]
fn float_literal_forms() {
    for s in [
        "3.14", ".5", "5.", "1.5e10", "1.5e+10", "1.5e-10", "1.5E10", "0.0",
    ] {
        // tclsh: .5==0.5, 5.==5.0, 1.5e10==15000000000.0
        assert_eq!(lit_text(&p(s)), s, "{s}");
    }
}

#[test]
fn ieee_special_float_literals() {
    // Tcl 9.0 recognises these as numeric literals (NOT function calls), so
    // they must parse to a Literal node, never a Call.
    for s in ["Inf", "inf", "Infinity", "infinity", "NaN", "nan"] {
        assert_eq!(lit_text(&p(s)), s, "{s}");
    }
}

#[test]
fn boolean_literals_case_insensitive() {
    // `Tcl_GetBoolean` accepts these case-insensitively; each is one Literal.
    for s in [
        "true", "false", "yes", "no", "on", "off", "True", "FALSE", "Yes", "NO", "On", "Off",
    ] {
        assert_eq!(lit_text(&p(s)), s, "{s}");
    }
}

#[test]
fn boolean_under_unary_not() {
    // !true ⇒ Not(Literal "true").
    let n = p("!true");
    assert_eq!(unop(&n), UnaryOp::Not);
    assert_eq!(lit_text(operand(&n)), "true");
}

// ===========================================================================
// 7. String / brace operands.
// ===========================================================================

#[test]
fn quoted_string_is_string_node() {
    match p(r#""hello world""#) {
        ExprNode::String { text, .. } => assert_eq!(text, r#""hello world""#),
        other => panic!("expected String, got {other:?}"),
    }
}

#[test]
fn empty_quoted_string() {
    // "" is a valid (empty) string operand carrying just the delimiters.
    match p(r#""""#) {
        ExprNode::String { text, .. } => assert_eq!(text, r#""""#),
        other => panic!("expected String, got {other:?}"),
    }
}

#[test]
fn quoted_string_with_escape() {
    // The lexer skips the byte after a backslash, so the closing quote is
    // found correctly even with an embedded escaped quote.
    match p(r#""a\"b""#) {
        ExprNode::String { text, .. } => assert_eq!(text, r#""a\"b""#),
        other => panic!("expected String, got {other:?}"),
    }
}

#[test]
fn braced_operand_is_string_node() {
    // A balanced `{...}` operand becomes a String node carrying the braces.
    match p("{abc}") {
        ExprNode::String { text, .. } => assert_eq!(text, "{abc}"),
        other => panic!("expected String, got {other:?}"),
    }
}

#[test]
fn quoted_strings_around_eq() {
    // "a" eq "a" = 1 ⇒ StrEq(String, String). tclsh: expr {"a" eq "a"} == 1
    let n = p(r#""a" eq "a""#);
    assert_eq!(binop(&n), BinOp::StrEq);
    assert!(matches!(left(&n), ExprNode::String { .. }));
    assert!(matches!(right(&n), ExprNode::String { .. }));
}

// ===========================================================================
// 8. Variable forms + command substitution nodes.
// ===========================================================================

#[test]
fn variable_forms_normalise_base_name() {
    for (src, name) in [
        ("$x", "x"),
        ("${my var}", "my var"),
        ("$ns::count", "ns::count"),
        ("$a::b::c", "a::b::c"),
        ("$arr(idx)", "arr"),
        ("$arr($i)", "arr"),
    ] {
        match p(src) {
            ExprNode::Var { name: n, .. } => assert_eq!(n, name, "{src}"),
            other => panic!("{src}: expected Var, got {other:?}"),
        }
    }
}

#[test]
fn command_substitution_node() {
    match p("[llength $list]") {
        ExprNode::Command { text, .. } => assert_eq!(text, "[llength $list]"),
        other => panic!("expected Command, got {other:?}"),
    }
}

#[test]
fn nested_command_substitution_node() {
    // Bracket nesting is tracked by the lexer; the whole thing is one
    // opaque Command node.
    match p("[expr {[llength $x] + 1}]") {
        ExprNode::Command { text, .. } => assert_eq!(text, "[expr {[llength $x] + 1}]"),
        other => panic!("expected Command, got {other:?}"),
    }
}

#[test]
fn command_substitution_as_operand() {
    // [llength $list] + 1 ⇒ Add(Command, Literal).
    let n = p("[llength $list] + 1");
    assert_eq!(binop(&n), BinOp::Add);
    assert!(matches!(left(&n), ExprNode::Command { .. }));
    assert!(matches!(right(&n), ExprNode::Literal { .. }));
}

#[test]
fn command_and_var_mixed() {
    // $base + [string length $s] * 2 — Command appears mid-tree.
    let n = p("$base + [string length $s] * 2");
    assert_eq!(binop(&n), BinOp::Add);
    assert_eq!(binop(right(&n)), BinOp::Mul);
    assert!(matches!(left(right(&n)), ExprNode::Command { .. }));
}

// ===========================================================================
// 9. vars() and command_texts() walkers over every node kind.
// ===========================================================================

fn sorted_vars(n: &ExprNode) -> Vec<String> {
    let mut v: Vec<String> = n.vars().into_iter().collect();
    v.sort();
    v
}

#[test]
fn vars_collected_through_all_node_kinds() {
    // Binary, Unary, Ternary, and Call children all contribute their vars.
    assert_eq!(sorted_vars(&p("$a + $b")), ["a", "b"]);
    assert_eq!(sorted_vars(&p("-$x")), ["x"]);
    assert_eq!(sorted_vars(&p("$a ? $b : $c")), ["a", "b", "c"]);
    assert_eq!(sorted_vars(&p("max($a, $b)")), ["a", "b"]);
    // Array base name is what is collected.
    assert_eq!(sorted_vars(&p("$arr(idx)")), ["arr"]);
}

#[test]
fn vars_are_a_set_no_duplicates() {
    // $x + $x * $x ⇒ {"x"} (set semantics).
    let s: HashSet<String> = p("$x + $x * $x").vars();
    assert_eq!(s.len(), 1);
    assert!(s.contains("x"));
}

#[test]
fn vars_empty_for_pure_arithmetic() {
    assert!(p("1 + 2 * 3").vars().is_empty());
}

#[test]
fn vars_stop_at_command_boundary() {
    // A command substitution is opaque to the pure-AST var scan.
    assert!(p("[set y $x]").vars().is_empty());
}

#[test]
fn vars_recovered_from_raw_fallback() {
    // Malformed input degrades to Raw, whose text is re-lexed so top-level
    // `$var` reads are still recovered (for liveness analysis).
    let n = p("$col @@@ $other");
    assert!(is_raw(&n));
    assert_eq!(sorted_vars(&n), ["col", "other"]);
}

#[test]
fn command_texts_collected_from_tree() {
    // command_texts() walks Binary/Unary/Ternary/Call and gathers each
    // `[cmd]` verbatim.
    let n = p("[a] + [b] * [c]");
    let mut texts = n.command_texts();
    texts.sort();
    assert_eq!(texts, ["[a]", "[b]", "[c]"]);
}

#[test]
fn command_texts_empty_when_no_commands() {
    assert!(p("$a + 1").command_texts().is_empty());
}

// ===========================================================================
// 10. render_expr round-trips (precedence-preserving).
// ===========================================================================

#[test]
fn render_inserts_parens_for_precedence() {
    // The renderer must re-add parens that precedence would otherwise drop.
    assert_eq!(render_expr(&p("(2 + 3) * 4")), "(2 + 3) * 4");
    assert_eq!(render_expr(&p("(1 | 2) & 3")), "(1 | 2) & 3");
}

#[test]
fn render_keeps_natural_associativity_without_parens() {
    // Left-assoc subtraction needs no parens; the inner Sub renders bare.
    assert_eq!(render_expr(&p("2 - 3 - 4")), "2 - 3 - 4");
    // Right-assoc power needs no parens on the right.
    assert_eq!(render_expr(&p("2 ** 3 ** 4")), "2 ** 3 ** 4");
}

#[test]
fn render_unary_over_binary_parenthesises() {
    // -(1 + 2): the unary operand is a Binary, so it must be wrapped.
    assert_eq!(render_expr(&p("-(1 + 2)")), "-(1 + 2)");
}

#[test]
fn render_round_trips_for_many_shapes() {
    for s in [
        "$a + $b",
        "-$x",
        "$x ? 1 : 0",
        "sqrt($y)",
        "2 * (3 + 4)",
        "max($a, $b, $c)",
        "$a && $b || $c",
        "~$x + 1",
        r#""s" eq "t""#,
    ] {
        let once = render_expr(&p(s));
        let twice = render_expr(&parse_expr(&once, None));
        assert_eq!(once, twice, "render not stable for {s}");
    }
}

#[test]
fn render_word_not_keeps_space() {
    // `not` is alphabetic, so the renderer keeps a space before the operand.
    assert_eq!(render_expr(&pi("not $x")), "not $x");
}

// ===========================================================================
// 11. ERROR / RECOVERY paths — the usually-uncovered lines.
//
// `parse_expr` must NEVER panic; every malformed input degrades to
// `ExprNode::Raw` (empty token stream, unknown chars, parse failure, or
// unconsumed trailing tokens).
// ===========================================================================

#[test]
fn empty_and_whitespace_are_raw() {
    // Empty token stream → Raw.
    assert!(is_raw(&p("")));
    assert!(is_raw(&p("   ")));
    assert!(is_raw(&p("\t\n ")));
}

#[test]
fn unknown_characters_are_raw() {
    // The lexer flags `@`, `` ` ``, `\` (lone), etc. as unknown → Raw.
    // `@#%` still qualifies on the strength of its leading `@`, even though the
    // `#%` that follows now lexes as a comment.
    for s in ["@#%", "`", "1 @ 2", "1 \\ 2", "@"] {
        assert!(is_raw(&p(s)), "{s} should be Raw");
    }
}

/// `1 # 2` used to be listed above as an unknown-character case. That encoded a
/// gap, not C's behaviour: `#` begins a comment lasting to the end of the line
/// or the end of the expression (`tclCompExpr.c:1931`, TIP 582), so this parses
/// as the bare literal `1` — C's `expr {1 # 2}` is `1`, not an error.
#[test]
fn a_hash_begins_a_comment_not_an_unknown_character() {
    assert!(!is_raw(&p("1 # 2")), "1 # 2 should parse (comment)");
    assert_eq!(p("1 # 2"), p("1      "));
    // Still unknown — hence still Raw — under a pre-9.0 dialect, where `#`
    // genuinely begins no lexeme.
    assert!(is_raw(&tcl_syntax::expr::parse_expr(
        "1 # 2",
        Some("tcl8.6")
    )));
}

#[test]
fn dangling_binary_operator_is_raw() {
    // A binary op with no right operand fails in `prefix()` (EOF) → Raw.
    for s in ["1 +", "$a *", "2 -", "$x ==", "1 <<", "$a &&", "1 |"] {
        assert!(is_raw(&p(s)), "{s} should be Raw");
    }
}

#[test]
fn leading_binary_operator_is_raw() {
    // A binary-only operator in prefix position (no unary meaning) → Raw.
    // `*`, `/`, `%`, `**` are not unary operators, so `* 2` has no prefix.
    for s in ["* 2", "/ 2", "% 2", "** 2", "== 2", "< 2", "&& 1"] {
        assert!(is_raw(&p(s)), "{s} should be Raw");
    }
}

#[test]
fn dangling_unary_operator_is_raw() {
    // A unary op with nothing to apply to (EOF after it) → Raw.
    for s in ["-", "~", "!", "+"] {
        assert!(is_raw(&p(s)), "{s} should be Raw");
    }
}

#[test]
fn unbalanced_open_paren_is_raw() {
    // `(2 + 3` never sees its `)` → expect(ParenClose) fails → Raw.
    for s in ["(2 + 3", "(", "((1)", "(1 + (2)"] {
        assert!(is_raw(&p(s)), "{s} should be Raw");
    }
}

#[test]
fn unbalanced_close_paren_is_raw() {
    // A stray `)` is either an unexpected prefix or an unconsumed trailing
    // token → Raw.
    for s in [")", "2 + 3)", "1)"] {
        assert!(is_raw(&p(s)), "{s} should be Raw");
    }
}

#[test]
fn empty_parens_are_raw() {
    // `()` — after `(`, `prefix()` sees `)` which is not a valid atom → Raw.
    assert!(is_raw(&p("()")));
    assert!(is_raw(&p("1 + ()")));
}

#[test]
fn unconsumed_trailing_tokens_are_raw() {
    // After a complete expression, a leftover token the parser can't attach
    // (a bareword/function token with no `(`) leaves pos < len → Raw.
    for s in ["1 + 2 garbage", "$a $b", "1 2", "5 foo"] {
        assert!(is_raw(&p(s)), "{s} should be Raw");
    }
}

#[test]
fn function_missing_parens_is_raw() {
    // `sin` (Function token) with no `(` → expect(ParenOpen) fails → Raw.
    assert!(is_raw(&p("sin")));
    assert!(is_raw(&p("sin $x")));
}

#[test]
fn function_unclosed_arglist_is_raw() {
    // `sin($x` never closes → the arg loop hits EOF (peek().ok_or) → Raw.
    for s in ["sin($x", "max(1, 2", "pow(1,", "abs("] {
        assert!(is_raw(&p(s)), "{s} should be Raw");
    }
}

#[test]
fn function_trailing_comma_is_raw() {
    // `max(1, )` — after the comma, `prefix()` sees `)` (not an atom) → Raw.
    // `max(, 1)` — leading comma has no first argument expression → Raw.
    for s in ["max(1, )", "max(, 1)", "pow(1,,2)"] {
        assert!(is_raw(&p(s)), "{s} should be Raw");
    }
}

#[test]
fn function_bad_separator_is_raw() {
    // `max(1 2)` — after the first arg, a non-comma non-`)` token in the arg
    // loop is an error → Raw.
    assert!(is_raw(&p("max(1 2)")));
}

#[test]
fn ternary_missing_colon_is_raw() {
    // `$x ? 1` and `$x ? 1 2` never reach the `:`; expect(TernaryC) fails →
    // Raw.
    for s in ["$x ? 1", "$x ? 1 2", "1 ? 2"] {
        assert!(is_raw(&p(s)), "{s} should be Raw");
    }
}

#[test]
fn ternary_missing_branches_is_raw() {
    // Dangling `?` / `:` with no operands → Raw.
    for s in ["1 ?", "1 ? :", "? 1 : 2", "1 : 2"] {
        assert!(is_raw(&p(s)), "{s} should be Raw");
    }
}

#[test]
fn double_binary_operator_is_raw() {
    // `1 + + 2` parses as Add(1, Pos(2)) (the second `+` is unary), so it is
    // NOT Raw — but `1 * * 2` IS Raw because `*` has no unary meaning.
    assert!(!is_raw(&p("1 + + 2")));
    assert_eq!(binop(&p("1 + + 2")), BinOp::Add);
    assert_eq!(unop(right(&p("1 + + 2"))), UnaryOp::Pos);
    for s in ["1 * * 2", "1 / / 2", "1 % % 2", "1 == == 2"] {
        assert!(is_raw(&p(s)), "{s} should be Raw");
    }
}

#[test]
fn deeply_nested_parens_degrade_to_raw_not_panic() {
    // Past MAX_EXPR_DEPTH (256) the depth guard returns ParseError → Raw,
    // rather than overflowing the Rust stack. ~9000 levels.
    let depth = 9000;
    let src = format!("{}1{}", "(".repeat(depth), ")".repeat(depth));
    assert!(is_raw(&p(&src)));
}

#[test]
fn deeply_chained_unary_degrades_to_raw_not_panic() {
    // 9000 prefix minuses also blow the depth guard → Raw, no SIGABRT.
    let src = format!("{}1", "-".repeat(9000));
    assert!(is_raw(&p(&src)));
}

#[test]
fn modestly_nested_parens_still_parse() {
    // Well under the cap, real code is unaffected: 32 levels still build the
    // Add tree.
    let src = format!("{}1 + 2{}", "(".repeat(32), ")".repeat(32));
    assert_eq!(binop(&p(&src)), BinOp::Add);
}

#[test]
fn unclosed_brace_operand_is_raw() {
    // `{abc` never balances: the brace lexer rewinds and emits a lone `{`
    // String token, then the unconsumed remainder leaves pos < len → Raw.
    assert!(is_raw(&p("{abc")));
    assert!(is_raw(&p("{1 + 2")));
}

#[test]
fn raw_node_preserves_original_text() {
    // The Raw fallback carries the verbatim source so consumers can fall
    // back to string analysis.
    match p("@@@ garbage @@@") {
        ExprNode::Raw { text } => assert_eq!(text, "@@@ garbage @@@"),
        other => panic!("expected Raw, got {other:?}"),
    }
}

#[test]
fn parser_never_panics_on_fuzzy_inputs() {
    // A broad smoke screen: none of these may panic; each is either a valid
    // tree or Raw. This exercises a wide spread of error branches at once.
    for s in [
        "",
        " ",
        "(",
        ")",
        "()",
        "((",
        "))",
        "?",
        ":",
        ",",
        "+",
        "-",
        "*",
        "/",
        "**",
        "==",
        "!",
        "~",
        "&&",
        "||",
        "1 +",
        "+ 1",
        "1 ? 2",
        "1 : 2",
        "sin",
        "sin(",
        "sin()",
        "max(,)",
        "$",
        "${",
        "[",
        "]",
        "[]",
        "1 2 3",
        "@#$%^&",
        "\\",
        "1 \\ 2",
        "{",
        "}",
        "{}",
        "1.2.3",
        "0x",
        "0xG",
        "1e",
        "1e+",
        ".",
        "..",
        "...",
        "$a $b $c",
        "1 ? 2 : 3 : 4",
        "(((",
        ")))",
        "a(b(c(",
        "* * *",
        "~ ~ ~ ~",
    ] {
        let n = parse_expr(s, None);
        // Any node kind is acceptable; the contract is "did not panic".
        let _ = render_expr(&n);
        let _ = n.vars();
    }
}
