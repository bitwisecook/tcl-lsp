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

//! Expression compilation: operators, constant folding, math functions and the iRules operators.

use super::*;

// Expression compilation

#[test]
fn expr_binary_add() {
    assert!(ops_of("set x 1; set r [expr {$x + 2}]").contains(&Op::ADD));
}

#[test]
fn expr_binary_eq() {
    assert!(ops_of("set x 1; set r [expr {$x == 2}]").contains(&Op::EQ));
}

#[test]
fn expr_binary_lt() {
    assert!(ops_of("set x 1; set r [expr {$x < 2}]").contains(&Op::LT));
}

#[test]
fn expr_constant_fold() {
    // All-constant expressions are folded at compile time.
    // tclsh: expr {1 + 2}  →  3   (8.6 + 9.0)
    let fa = top_asm("set r [expr {1 + 2}]");
    let ops = opcodes(&fa);
    assert!(!ops.contains(&Op::ADD), "folded away");
    assert!(folds_to_integer(&fa, 3));
}

#[test]
fn expr_unary_neg() {
    // tclsh: expr {-1}  →  -1   (8.6 + 9.0)
    let fa = top_asm("set r [expr {-1}]");
    let ops = opcodes(&fa);
    assert!(!ops.contains(&Op::UMINUS), "constant folded");
    assert!(folds_to_integer(&fa, -1));
}

#[test]
fn expr_var_load() {
    let ops = ops_of("set x 1; set r [expr {$x + 1}]");
    // Top-level uses stack-based load.
    assert!(ops.contains(&Op::LOAD_STK));
    assert!(ops.contains(&Op::ADD));
}

#[test]
fn expr_math_func_inline() {
    // Math function calls compile inline as invokeStk + tryCvtToNumeric.
    let ops = ops_of("set x 1; set r [expr {sin($x)}]");
    assert!(ops.contains(&Op::INVOKE_STK1));
    assert!(ops.contains(&Op::TRY_CVT_TO_NUMERIC));
}

#[test]
fn standalone_expr_inline() {
    // Standalone `expr` is compiled inline, not as invokeStk.
    let ops = ops_of("set x 1; expr {$x + 2}");
    assert!(ops.contains(&Op::ADD));
    assert!(ops.contains(&Op::LOAD_STK));
}

#[test]
fn standalone_expr_constant_fold() {
    // tclsh: expr {2 + 3}  →  5   (8.6 + 9.0)
    let fa = top_asm("expr {2 + 3}");
    assert!(!opcodes(&fa).contains(&Op::ADD));
    assert!(folds_to_integer(&fa, 5));
}

// iRules expression operators
//
// Uses the `emit_binop`/`emit_unaryop` helpers: a hand-built CFG with an
// `AssignExpr` carrying the binary/unary node.

#[test]
fn irule_contains() {
    assert!(emit_binop(BinOp::Contains).contains(&Op::IRULE_CONTAINS));
}

#[test]
fn irule_starts_with() {
    assert!(emit_binop(BinOp::StartsWith).contains(&Op::IRULE_STARTS_WITH));
}

#[test]
fn irule_ends_with() {
    assert!(emit_binop(BinOp::EndsWith).contains(&Op::IRULE_ENDS_WITH));
}

#[test]
fn irule_equals() {
    assert!(emit_binop(BinOp::StrEquals).contains(&Op::IRULE_EQUALS));
}

#[test]
fn irule_matches() {
    assert!(emit_binop(BinOp::Matches).contains(&Op::IRULE_MATCHES));
}

#[test]
fn irule_matches_glob() {
    assert!(emit_binop(BinOp::MatchesGlob).contains(&Op::IRULE_MATCHES_GLOB));
}

#[test]
fn irule_matches_regex() {
    assert!(emit_binop(BinOp::MatchesRegex).contains(&Op::IRULE_MATCHES_REGEX));
}

#[test]
fn irule_word_and() {
    assert!(emit_binop(BinOp::WordAnd).contains(&Op::IRULE_WORD_AND));
}

#[test]
fn irule_word_or() {
    assert!(emit_binop(BinOp::WordOr).contains(&Op::IRULE_WORD_OR));
}

#[test]
fn irule_word_not() {
    assert!(emit_unaryop(UnaryOp::WordNot).contains(&Op::IRULE_WORD_NOT));
}

// Complex expressions

#[test]
fn ternary_in_expr() {
    let src =
        "proc clamp {x lo hi} { set r [expr {$x < $lo ? $lo : ($x > $hi ? $hi : $x)}]; return $r }";
    let ops = opcodes(&proc_asm(src, "::clamp"));
    assert!(ops.contains(&Op::LT));
    assert!(ops.contains(&Op::GT));
    assert!(has_cond_jump(&ops), "ternary needs conditional jumps");
}

#[test]
fn chained_logical_and() {
    let src = "proc all_positive {a b c} { set r [expr {$a > 0 && $b > 0 && $c > 0}]; return $r }";
    // tclsh: with all > 0  →  1   (9.0)
    let ops = opcodes(&proc_asm(src, "::all_positive"));
    assert!(count(&ops, Op::GT) >= 3);
}

#[test]
fn chained_logical_or() {
    let src = "proc any_zero {a b c} { set r [expr {$a == 0 || $b == 0 || $c == 0}]; return $r }";
    let ops = opcodes(&proc_asm(src, "::any_zero"));
    assert!(count(&ops, Op::EQ) >= 3);
}

#[test]
fn mixed_arithmetic_and_comparison() {
    let src = "proc check {x y} { set r [expr {($x + $y) * 2 > ($x - $y) ** 2}]; return $r }";
    let ops = opcodes(&proc_asm(src, "::check"));
    assert!(ops.contains(&Op::ADD));
    assert!(ops.contains(&Op::SUB));
    assert!(ops.contains(&Op::MULT));
    assert!(ops.contains(&Op::EXPON));
    assert!(ops.contains(&Op::GT));
}

#[test]
fn bitwise_operations() {
    // tclsh: expr {7 & 3}=3, {1 << 4}=16, {2 ** 10}=1024   (8.6 + 9.0)
    let src = "proc bits {x y} { set a [expr {$x & $y}]; set b [expr {$x | $y}]; set c [expr {$x ^ $y}]; set d [expr {$x << 2}]; set e [expr {$x >> 1}]; set f [expr {~$x}]; return [expr {$a + $b + $c + $d + $e + $f}] }";
    let ops = opcodes(&proc_asm(src, "::bits"));
    assert!(ops.contains(&Op::BITAND));
    assert!(ops.contains(&Op::BITOR));
    assert!(ops.contains(&Op::BITXOR));
    assert!(ops.contains(&Op::LSHIFT));
    assert!(ops.contains(&Op::RSHIFT));
    assert!(ops.contains(&Op::BITNOT));
}

#[test]
fn string_comparison_operators_compile_in_tcl_9_0() {
    let src = "proc str_cmp {a b} { set r1 [expr {$a eq $b}]; set r2 [expr {$a ne $b}]; set r3 [expr {$a lt $b}]; set r4 [expr {$a gt $b}]; return [expr {$r1 + $r2 + $r3 + $r4}] }";
    // `lt` and `gt` are Tcl 9 operators: tclsh8.6 compiles each to a
    // `syntax` error ("invalid bareword"), tclsh9.0 to `strlt` and `strgt`.
    let ops = opcodes(&crate::vm_compiled::procedure("tcl9.0", src, "::str_cmp"));
    assert!(ops.contains(&Op::STR_EQ));
    assert!(ops.contains(&Op::STR_NEQ));
    assert!(ops.contains(&Op::STR_LT));
    assert!(ops.contains(&Op::STR_GT));
}

#[test]
fn list_membership_in_ni() {
    let src = "proc membership {x lst} { set a [expr {$x in $lst}]; set b [expr {$x ni $lst}]; return [expr {$a + $b}] }";
    let ops = opcodes(&proc_asm(src, "::membership"));
    assert!(ops.contains(&Op::LIST_IN));
    assert!(ops.contains(&Op::LIST_NOT_IN));
}

#[test]
fn nested_math_functions() {
    let src = "proc compute {x} { set r [expr {abs(int(sin($x) * 100))}]; return $r }";
    let ops = opcodes(&proc_asm(src, "::compute"));
    assert!(ops.contains(&Op::INVOKE_STK1));
    assert!(ops.contains(&Op::TRY_CVT_TO_NUMERIC));
}

#[test]
fn constant_fold_complex() {
    // tclsh: expr {(2 + 3) * 4 - 1}  →  19   (8.6 + 9.0)
    let fa = top_asm("set r [expr {(2 + 3) * 4 - 1}]");
    let ops = opcodes(&fa);
    assert!(!ops.contains(&Op::ADD));
    assert!(!ops.contains(&Op::MULT));
    assert!(!ops.contains(&Op::SUB));
    assert!(folds_to_integer(&fa, 19));
}

#[test]
fn constant_fold_boolean() {
    // tclsh: expr {1 && 0}  →  0   (8.6 + 9.0)
    let fa = top_asm("set r [expr {1 && 0}]");
    assert!(!opcodes(&fa).contains(&Op::LAND));
    assert!(fa.literals.entries().iter().any(|l| l == "0"));
}

#[test]
fn partial_fold_mixed_const_var() {
    // 2*3=6 folds, but $x + 6 still needs ADD.
    let ops = ops_of("set x 1; set r [expr {$x + 2 * 3}]");
    assert!(ops.contains(&Op::ADD));
    assert!(!ops.contains(&Op::MULT));
}
