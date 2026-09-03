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

//! Tests for the `expr` precedence-climbing parser and AST. Verifies
//! `parse_expr` builds the expected `ExprNode` tree.
//!
//! C-Tcl proof: the precedence/associativity the tree encodes is Tcl's, checked
//! against tclsh8.6/9.0 by evaluating the same expressions —
//!   2 + 3 * 4 = 14 (× binds tighter ⇒ Add(2, Mul(3,4)))
//!   2 ** 3 ** 2 = 512 (right-assoc ⇒ Pow(2, Pow(3,2)))
//!   10 - 3 - 2 = 5 (left-assoc ⇒ Sub(Sub(10,3),2))
//!   1 + 2 < 4 = 1 (arith before comparison ⇒ Lt(Add(1,2),4))
//! and literal values 0xFF=255, 0o17=15, 0b101=5, .5=0.5, 5.=5.0.

use tcl_syntax::expr::ast::render_expr;
use tcl_syntax::expr::{BinOp, ExprNode, UnaryOp, parse_expr};

fn p(src: &str) -> ExprNode {
    parse_expr(src, None)
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

// -- Literals --

#[test]
fn literals() {
    assert_eq!(lit_text(&p("42")), "42");
    assert_eq!(lit_text(&p("3.14")), "3.14");
    assert_eq!(lit_text(&p("0xFF")), "0xFF");
    assert_eq!(lit_text(&p("true")), "true");
    assert_eq!(lit_text(&p("false")), "false");
    // A quoted string is a String node carrying the delimiters.
    assert!(matches!(p("\"hello\""), ExprNode::String { .. }));
}

#[test]
fn negative_integer_is_unary_neg() {
    match p("-7") {
        ExprNode::Unary { op, operand } => {
            assert_eq!(op, UnaryOp::Neg);
            assert_eq!(lit_text(&operand), "7");
        }
        other => panic!("expected Unary Neg, got {other:?}"),
    }
}

// -- Variables --

#[test]
fn variables() {
    for (src, name) in [
        ("$x", "x"),
        ("${my_var}", "my_var"),
        ("$ns::count", "ns::count"),
        ("$arr(idx)", "arr"),
    ] {
        match p(src) {
            ExprNode::Var { name: n, .. } => assert_eq!(n, name, "{src}"),
            other => panic!("{src}: expected Var, got {other:?}"),
        }
    }
}

#[test]
fn command_substitution() {
    match p("[llength $list]") {
        ExprNode::Command { text, .. } => assert_eq!(text, "[llength $list]"),
        other => panic!("expected Command, got {other:?}"),
    }
}

// -- Binary operators --

#[test]
fn binary_operator_kinds() {
    assert_eq!(binop(&p("$a + $b")), BinOp::Add);
    assert_eq!(binop(&p("$a - 1")), BinOp::Sub);
    assert_eq!(binop(&p("$x * $y")), BinOp::Mul);
    assert_eq!(binop(&p("$x / 2")), BinOp::Div);
    assert_eq!(binop(&p("$x % 2")), BinOp::Mod);
    assert_eq!(binop(&p("$a == $b")), BinOp::Eq);
    assert_eq!(binop(&p("$a != $b")), BinOp::Ne);
    assert_eq!(binop(&p("$a < $b")), BinOp::Lt);
    assert_eq!(binop(&p("$a <= $b")), BinOp::Le);
    assert_eq!(binop(&p("$a > $b")), BinOp::Gt);
    assert_eq!(binop(&p("$a >= $b")), BinOp::Ge);
    assert_eq!(binop(&p("$a & $b")), BinOp::BitAnd);
    assert_eq!(binop(&p("$a | $b")), BinOp::BitOr);
    assert_eq!(binop(&p("$a ^ $b")), BinOp::BitXor);
    assert_eq!(binop(&p("$a << 2")), BinOp::LShift);
    assert_eq!(binop(&p("$a >> 2")), BinOp::RShift);
    assert_eq!(binop(&p("$a && $b")), BinOp::And);
    assert_eq!(binop(&p("$a || $b")), BinOp::Or);
}

#[test]
fn add_operands_are_vars() {
    match p("$a + $b") {
        ExprNode::Binary { op, left, right } => {
            assert_eq!(op, BinOp::Add);
            assert!(matches!(*left, ExprNode::Var { .. }));
            assert!(matches!(*right, ExprNode::Var { .. }));
        }
        other => panic!("{other:?}"),
    }
}

// -- Unary operators --

#[test]
fn unary_operator_kinds() {
    assert!(matches!(
        p("-$x"),
        ExprNode::Unary {
            op: UnaryOp::Neg,
            ..
        }
    ));
    assert!(matches!(
        p("+$x"),
        ExprNode::Unary {
            op: UnaryOp::Pos,
            ..
        }
    ));
    assert!(matches!(
        p("!$x"),
        ExprNode::Unary {
            op: UnaryOp::Not,
            ..
        }
    ));
    assert!(matches!(
        p("~$x"),
        ExprNode::Unary {
            op: UnaryOp::BitNot,
            ..
        }
    ));
}

// -- Precedence (tclsh-proven structure) --

#[test]
fn mul_binds_tighter_than_add() {
    // 2 + 3 * 4 = 14 ⇒ Add(2, Mul(3,4)).
    match p("2 + 3 * 4") {
        ExprNode::Binary {
            op: BinOp::Add,
            right,
            ..
        } => {
            assert_eq!(binop(&right), BinOp::Mul);
        }
        other => panic!("expected Add at root, got {other:?}"),
    }
}

#[test]
fn subtraction_is_left_associative() {
    // 10 - 3 - 2 = 5 ⇒ Sub(Sub(10,3),2).
    match p("10 - 3 - 2") {
        ExprNode::Binary {
            op: BinOp::Sub,
            left,
            ..
        } => {
            assert_eq!(binop(&left), BinOp::Sub);
        }
        other => panic!("expected left-assoc Sub, got {other:?}"),
    }
}

#[test]
fn power_is_right_associative() {
    // 2 ** 3 ** 2 = 512 ⇒ Pow(2, Pow(3,2)).
    match p("2 ** 3 ** 2") {
        ExprNode::Binary {
            op: BinOp::Pow,
            right,
            ..
        } => {
            assert_eq!(binop(&right), BinOp::Pow);
        }
        other => panic!("expected right-assoc Pow, got {other:?}"),
    }
}

#[test]
fn arithmetic_binds_tighter_than_comparison() {
    // 1 + 2 < 4 = 1 ⇒ Lt(Add(1,2),4).
    match p("1 + 2 < 4") {
        ExprNode::Binary {
            op: BinOp::Lt,
            left,
            ..
        } => {
            assert_eq!(binop(&left), BinOp::Add);
        }
        other => panic!("expected Lt at root, got {other:?}"),
    }
}

#[test]
fn comparison_binds_tighter_than_logical() {
    // $a < $b && $c < $d ⇒ And(Lt(a,b), Lt(c,d)).
    match p("$a < $b && $c < $d") {
        ExprNode::Binary {
            op: BinOp::And,
            left,
            right,
        } => {
            assert_eq!(binop(&left), BinOp::Lt);
            assert_eq!(binop(&right), BinOp::Lt);
        }
        other => panic!("expected And at root, got {other:?}"),
    }
}

#[test]
fn or_binds_looser_than_and() {
    // $a && $b || $c ⇒ Or(And(a,b), c).
    match p("$a && $b || $c") {
        ExprNode::Binary {
            op: BinOp::Or,
            left,
            ..
        } => {
            assert_eq!(binop(&left), BinOp::And);
        }
        other => panic!("expected Or at root, got {other:?}"),
    }
}

#[test]
fn parentheses_override_precedence() {
    // (2 + 3) * 4 ⇒ Mul(Add(2,3), 4).
    match p("(2 + 3) * 4") {
        ExprNode::Binary {
            op: BinOp::Mul,
            left,
            ..
        } => {
            assert_eq!(binop(&left), BinOp::Add);
        }
        other => panic!("expected Mul at root, got {other:?}"),
    }
}

// -- Ternary --

#[test]
fn ternary_structure_and_right_assoc() {
    assert!(matches!(p("$x ? 1 : 0"), ExprNode::Ternary { .. }));
    // a ? b : c ? d : e ⇒ Ternary(a, b, Ternary(c, d, e)) (right-assoc).
    match p("$a ? 1 : $c ? 2 : 3") {
        ExprNode::Ternary { false_branch, .. } => {
            assert!(matches!(*false_branch, ExprNode::Ternary { .. }));
        }
        other => panic!("expected Ternary, got {other:?}"),
    }
}

// -- Function calls --

#[test]
fn function_calls() {
    match p("sqrt($x)") {
        ExprNode::Call { function, args, .. } => {
            assert_eq!(function, "sqrt");
            assert_eq!(args.len(), 1);
        }
        other => panic!("expected Call, got {other:?}"),
    }
    match p("atan2($y, $x)") {
        ExprNode::Call { function, args, .. } => {
            assert_eq!(function, "atan2");
            assert_eq!(args.len(), 2);
        }
        other => panic!("{other:?}"),
    }
    // max with several args.
    match p("max(1, 2, 3, 4)") {
        ExprNode::Call { args, .. } => assert_eq!(args.len(), 4),
        other => panic!("{other:?}"),
    }
}

// -- Fallback (Raw) --

#[test]
fn malformed_expression_is_raw() {
    // A syntax error degrades to Raw rather than panicking.
    assert!(matches!(p("@#%"), ExprNode::Raw { .. }));
}

// -- vars() extraction --

#[test]
fn vars_extraction() {
    assert_eq!(
        p("$x")
            .vars_with_grammar(tcl_dialect::LexerGrammar::default())
            .into_iter()
            .collect::<Vec<_>>(),
        ["x"]
    );
    let mut v: Vec<String> = p("$a + $b * $c")
        .vars_with_grammar(tcl_dialect::LexerGrammar::default())
        .into_iter()
        .collect();
    v.sort();
    assert_eq!(v, ["a", "b", "c"]);
    assert!(
        p("1 + 2")
            .vars_with_grammar(tcl_dialect::LexerGrammar::default())
            .is_empty()
    );
    // A repeated var appears once (set semantics).
    assert_eq!(
        p("$x + $x")
            .vars_with_grammar(tcl_dialect::LexerGrammar::default())
            .len(),
        1
    );
    // Ternary and function args contribute their vars.
    let mut tv: Vec<String> = p("$a ? $b : $c")
        .vars_with_grammar(tcl_dialect::LexerGrammar::default())
        .into_iter()
        .collect();
    tv.sort();
    assert_eq!(tv, ["a", "b", "c"]);
}

// -- render round-trip --

#[test]
fn render_round_trips() {
    // render_expr(parse_expr(s)) re-parses to an equivalent tree.
    for s in ["$a + $b", "-$x", "$x ? 1 : 0", "sqrt($y)", "2 * (3 + 4)"] {
        let once = render_expr(&p(s));
        let twice = render_expr(&parse_expr(&once, None));
        assert_eq!(once, twice, "render not stable for {s}");
    }
}

// -- Boolean literals --

#[test]
fn boolean_literals() {
    for b in ["true", "false", "yes", "no", "on", "off"] {
        assert!(matches!(p(b), ExprNode::Literal { .. }), "{b}");
    }
    // `!true` is unary-not over a boolean literal.
    assert!(matches!(
        p("!true"),
        ExprNode::Unary {
            op: UnaryOp::Not,
            ..
        }
    ));
}

// -- Numeric literals (tclsh values: 0xFF=255, 0o17=15, 0b101=5, .5=0.5, 5.=5.0) --

#[test]
fn numeric_literal_forms() {
    for s in [
        "0o17",
        "0O17",
        "0b101",
        "0B101",
        "0xff",
        "0xFF",
        "1.5e10",
        "1.5e+10",
        "1.5e-10",
        "1.5E10",
        ".5",
        "5.",
        "0",
        "123456789012345",
    ] {
        // Each parses to a single Literal node carrying the verbatim text.
        assert_eq!(lit_text(&p(s)), s, "{s}");
    }
}
