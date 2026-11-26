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

//! Numbers and `expr`: operand errors, overflow promotion, `wide`, `incr`, the `::tcl::mathop` commands and numeric comparison.

use super::*;

/// A `nan` arithmetic operand is reported as a non-numeric floating-point
/// *value*, not a string (mathop operator error tests).
#[test]
fn nan_operand_error_wording() {
    let (ok, result, _) = run("catch {tcl::mathop::+ nan 0} m\nset m");
    assert!(ok, "script should complete: {result}");
    assert_eq!(
        result,
        "cannot use non-numeric floating-point value \"nan\" as left operand of \"+\""
    );
}

/// A non-numeric operand that is a well-formed multi-element **list** gets C's
/// distinct wording, `cannot use a list as <ord>operand of "<op>"` with
/// errorCode `ARITH DOMAIN list`. `IllegalExprOperandType`
/// (`tclExecute.c:9089-9107`) applies that branch to *binary* operators — where
/// `ord` is `"left "`/`"right "` — exactly as to unary ones, but only the unary
/// builder had the check.
#[test]
fn list_operand_error_wording() {
    for (e, want) in [
        ("{1 2} + 1", "cannot use a list as left operand of \"+\""),
        ("1 + {1 2}", "cannot use a list as right operand of \"+\""),
        (
            "{a 1 b 2} * 2",
            "cannot use a list as left operand of \"*\"",
        ),
        ("-{a 1 b 2}", "cannot use a list as operand of \"-\""),
    ] {
        let (ok, result, _) = run(&format!("catch {{expr {{{e}}}}} m\nset m"));
        assert!(ok, "script should complete: {result}");
        assert_eq!(result, want, "expr {{{e}}}");
    }
    // A single-element non-number is still a non-numeric *string* — the list
    // branch must not swallow it. Written through `expr` directly, with the
    // operand *quoted*: a bare `abc` never reaches the operand check in C either
    // (`ParseExpr` rejects it as `invalid bareword` first — `tclCompExpr.c:766`),
    // whereas a quoted string is a legal operand whose coercion is what fails
    // (expr-old-5.13/5.14).
    let (ok, result, _) = run("catch {expr {\"abc\" + 1}} m\nset m");
    assert!(ok, "script should complete: {result}");
    assert_eq!(
        result,
        "cannot use non-numeric string \"abc\" as left operand of \"+\""
    );
    // `mathop` reaches the same helper from its own entry point.
    let (ok, result, _) = run("catch {tcl::mathop::+ abc 1} m\nset m");
    assert!(ok, "script should complete: {result}");
    assert_eq!(
        result,
        "cannot use non-numeric string \"abc\" as left operand of \"+\""
    );
    // And `mathop` takes the list branch too (same helper, both entry points).
    let (ok, result, _) = run("catch {tcl::mathop::+ {1 2} 1} m\nset m");
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "cannot use a list as left operand of \"+\"");
}

/// Integer `expr` arithmetic promotes to i128 on i64 overflow instead of
/// wrapping (the VM's bounded stand-in for Tcl bignums), covering the common
/// large-value range that `expr`/`mathop` exercise.
#[test]
fn integer_arithmetic_promotes_on_overflow() {
    for (e, want) in [
        ("9223372036854775807 + 1", "9223372036854775808"),
        ("2 ** 70", "1180591620717411303424"),
        ("1000000000000000000000 + 5", "1000000000000000000005"),
        ("100000000000 * 100000000000", "10000000000000000000000"),
    ] {
        let (ok, result, _) = run(&format!("expr {{{e}}}"));
        assert!(ok, "`{e}` should evaluate: {result}");
        assert_eq!(result, want, "expr {{{e}}}");
    }
}

/// Unary minus of `9223372036854775808` (the magnitude 2^63, which overflows a
/// positive wide) yields the most-negative wide — C narrows `-2^63`. This
/// unblocks indexObj.test, whose `-result [expr {… ? -9223372036854775808 :
/// …}]` clauses are evaluated while the file is sourced.
#[test]
fn unary_minus_of_two_to_the_63() {
    let (ok, result, _) = run("expr {-9223372036854775808}");
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "-9223372036854775808");

    let (ok, result, _) = run("expr {-9223372036854775808 < 0}");
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "1");
}

/// `wide()` truncates an out-of-range integer literal to a 64-bit wide via
/// two's-complement wrap, matching C — `wide(0x8000000000000000)` is the most
/// negative wide, `wide(0xFFFFFFFFFFFFFFFF)` is -1. This unblocks expr.test
/// (which probes `wide(0x8000000000000000) < 0` at the top level) and brings
/// obj.test into line with C Tcl without a full bignum rep.
#[test]
fn wide_truncates_out_of_range_literals() {
    let (ok, result, _) = run("expr {wide(0x8000000000000000)}");
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "-9223372036854775808");

    let (ok, result, _) = run("expr {wide(0xFFFFFFFFFFFFFFFF)}");
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "-1");

    let (ok, result, _) = run("expr {wide(0x8000000000000000) < 0}");
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "1");
}

#[test]
fn set_expr_puts() {
    let (ok, _result, out) = run("set x 5\nputs [expr {$x * 2}]\n");
    assert!(ok);
    assert_eq!(out, "10\n");
}

#[test]
fn expr_precedence() {
    let (ok, result, _out) = run("expr {3 + 4 * 2}");
    assert!(ok);
    assert_eq!(result, "11");
}

#[test]
fn floored_integer_division() {
    let (ok, result, _out) = run("expr {-7 / 2}");
    assert!(ok);
    assert_eq!(result, "-4");
}

#[test]
fn incr_and_var_substitution() {
    let (ok, _result, out) = run("set n 0\nincr n\nincr n 5\nputs $n\n");
    assert!(ok);
    assert_eq!(out, "6\n");
}

/// `incr` routed through the shared `tcl_cmd_core::var::incr_value` core (the
/// `VarStore` read + `ValueOps::int_add` tower seam). Covers the unset → 0 start,
/// array elements (the name carries `base(key)`; the VM parses it), and the
/// canonical coercion errors. Crucially, an overflowing `incr` now **promotes**
/// through the integer tower (`i128`, then an arbitrary-precision bignum),
/// matching tclsh, rather than silently wrapping as a hand-rolled
/// `wrapping_add` would.
#[test]
fn incr_shared_core() {
    // Unset variable starts at 0 (no prior `set`).
    assert_eq!(run("incr fresh 5").1, "5");
    assert_eq!(run("incr fresh").1, "1");
    // Array element: the VM's var_get/var_set parse `a(k)` from the name.
    assert_eq!(run("set a(k) 10\nincr a(k) 5\nset a(k)").1, "15");
    // Non-integer current value / increment → the canonical coercion error.
    let (ok, msg, _) = run("set x abc\nincr x");
    assert!(!ok);
    assert_eq!(msg, "expected integer but got \"abc\"");
    let (ok, msg, _) = run("set y 1\nincr y xyz");
    assert!(!ok);
    assert_eq!(msg, "expected integer but got \"xyz\"");
    // i64::MAX + 1 promotes to the wider integer tower (as `expr` does and as
    // tclsh's arbitrary-precision integers do), rather than erroring or wrapping.
    let (ok, val, _) = run("set big 9223372036854775807\nincr big");
    assert!(ok);
    assert_eq!(val, "9223372036854775808");
    // A sum past i128 now promotes to an arbitrary-precision bignum too, matching
    // tclsh, instead of overflowing. i128::MAX + 1 == 2**127.
    let (ok, val, _) = run("set huge 170141183460469231731687303715884105727\nincr huge");
    assert!(ok);
    assert_eq!(val, "170141183460469231731687303715884105728");
}

/// `::tcl::mathop::*` — newly added to the VM, sharing `tcl_cmd_core::mathop`'s
/// fold/chain logic over the VM's `ExprOps`. Pinned against tclsh 9.0.
#[test]
fn mathop_shared() {
    assert_eq!(run("::tcl::mathop::+ 1 2 3").1, "6");
    assert_eq!(run("::tcl::mathop::+").1, "0"); // identity
    assert_eq!(run("::tcl::mathop::- 10 3 2").1, "5");
    assert_eq!(run("::tcl::mathop::- 5").1, "-5"); // 1 arg negates
    assert_eq!(run("::tcl::mathop::* 2 3 4").1, "24");
    assert_eq!(run("::tcl::mathop::/ 100 5 2").1, "10");
    assert_eq!(run("::tcl::mathop::/ 4").1, "0.25"); // reciprocal
    assert_eq!(run("::tcl::mathop::** 2 3 2").1, "512"); // right-assoc: 2**(3**2)
    assert_eq!(run("::tcl::mathop::% 17 5").1, "2");
    assert_eq!(run("::tcl::mathop::<< 1 4").1, "16");
    assert_eq!(run("::tcl::mathop::& 12 10").1, "8");
    assert_eq!(run("::tcl::mathop::~ 5").1, "-6");
    assert_eq!(run("::tcl::mathop::! 0").1, "1");
    assert_eq!(run("::tcl::mathop::< 1 2 3").1, "1"); // chained
    assert_eq!(run("::tcl::mathop::< 1 3 2").1, "0");
    assert_eq!(run("::tcl::mathop::== 5 5 5").1, "1");
    assert_eq!(run("::tcl::mathop::eq abc abc").1, "1");
    assert_eq!(run("::tcl::mathop::in b {a b c}").1, "1");
    assert_eq!(run("::tcl::mathop::ni x {a b c}").1, "1");
    let (ok, msg, _) = run("::tcl::mathop::% 1");
    assert!(!ok);
    assert_eq!(
        msg,
        "wrong # args: should be \"::tcl::mathop::% integer integer\""
    );
}

#[test]
fn string_and_numeric_compare() {
    let (ok, result, _out) = run("expr {9 < 10}");
    assert!(ok);
    assert_eq!(result, "1");
}

/// `NUMERIC_TYPE` pushes C Tcl's numeric-tower code: 0 = not a number,
/// INT = 2, BIG = 3, DOUBLE = 4, NAN = 5.
#[test]
fn opcode_numeric_type() {
    use tcl_bytecode::{Instruction, Op};
    let asm = |lit: &str| {
        run_asm(
            &[lit],
            vec![
                pushv(0),
                Instruction::new(Op::NUMERIC_TYPE, vec![]),
                Instruction::new(Op::DONE, vec![]),
            ],
        )
    };
    assert_eq!(asm("42"), "2"); // wide integer
    assert_eq!(asm("-17"), "2");
    assert_eq!(asm("3.5"), "4"); // double
    assert_eq!(asm("1e10"), "4");
    assert_eq!(asm("abc"), "0"); // not a number
    assert_eq!(asm(""), "0");
    assert_eq!(asm("99999999999999999999999999"), "3"); // bignum
    assert_eq!(asm("NaN"), "5");
}
