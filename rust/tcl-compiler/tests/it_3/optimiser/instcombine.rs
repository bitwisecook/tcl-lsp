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

//! Expression simplification: reassociation, identities, boolean rules, De Morgan, tautologies and the iRules word operators.

use super::*;

/// Wraps `body` in a loop where `$x` is an SCCP-typed INT loop counter (the
/// D5-O110 identity/annihilator drops need a provably-numeric operand).
fn int_x(body: &str) -> String {
    format!(
        "proc f {{n}} {{\n  for {{set x 0}} {{$x < $n}} {{incr x}} {{\n    {body}\n    puts $v\n  }}\n}}\n"
    )
}

// O110 InstCombine — algebraic / boolean / De-Morgan / ternary simplification

#[test]
fn instcombine_reassociation_and_identity_annihilator() {
    // Regrouping needs every term proven integer: `$x + 1 + 2` equals `$x +
    // 3` for every integer `$x`, but over a double the rounding is
    // order-dependent — `set x 10000000000000000.0; expr {$x + 1 + 2}` prints
    // `10000000000000002.0` and `expr {$x + 3}` `10000000000000004.0` under
    // tclsh 8.5 to 9.1 — so an unproven `$a` keeps its chain.
    let reassoc = int_x("set v [expr {$x + 1 + 2}]");
    assert!(optimised(&reassoc, TCL).contains("set v [expr {$x + 3}]"));
    assert!(opt_fires(&reassoc, TCL, "O110"));
    let unproven = "set v [expr {$a + 1 + 2}]";
    assert!(optimised(unproven, TCL).contains("set v [expr {$a + 1 + 2}]"));

    // Identity/annihilator drops need provably-INT $x — wrap in the `_int_x`
    // loop. tclsh sweep (x≥0): x**0==1, x**1==x, x<<0==x, x>>0==x, x&0==0,
    // x|0==x, x^0==x, x%1==0, ~~x==x, x^x==0.
    assert!(int_x("set v [expr {$x ** 0}]").contains("$x ** 0"));
    assert!(optimised(&int_x("set v [expr {$x ** 0}]"), TCL).contains("set v 1"));
    assert!(opt_fires(&int_x("set v [expr {$x ** 1}]"), TCL, "O110"));
    assert!(opt_fires(&int_x("set v [expr {$x << 0}]"), TCL, "O110"));
    assert!(opt_fires(&int_x("set v [expr {$x >> 0}]"), TCL, "O110"));
    assert!(optimised(&int_x("set v [expr {$x & 0}]"), TCL).contains("set v 0"));
    assert!(opt_fires(&int_x("set v [expr {$x | 0}]"), TCL, "O110"));
    assert!(opt_fires(&int_x("set v [expr {$x ^ 0}]"), TCL, "O110"));
    assert!(optimised(&int_x("set v [expr {$x % 1}]"), TCL).contains("set v 0"));
    assert!(optimised(&int_x("set v [expr {~~$x}]"), TCL).contains("set v [expr {$x}]"));
    assert!(optimised(&int_x("set v [expr {$x ^ $x}]"), TCL).contains("set v 0"));
}

#[test]
fn instcombine_boolean_simplifications() {
    // tclsh sweep: x && 0 == 0, x || 1 == 1 (annihilators).
    assert!(optimised(&int_xy("set v [expr {$x && 0}]"), TCL).contains("set v 0"));
    assert!(optimised(&int_xy("set v [expr {$x || 1}]"), TCL).contains("set v 1"));

    // Boolean canonicalisation: x && 1 → !!x, x || 0 → !!x (tclsh sweep == ).
    let and1 = optimised(&int_xy("set v [expr {$x && 1}]"), TCL);
    assert!(and1.contains("!!$x"));
    assert!(opt_fires(&int_xy("set v [expr {$x && 1}]"), TCL, "O110"));
    let or0 = optimised(&int_xy("set v [expr {$x || 0}]"), TCL);
    assert!(or0.contains("!!$x"));
    assert!(opt_fires(&int_xy("set v [expr {$x || 0}]"), TCL, "O110"));

    // Both operands non-zero constants fold to the boolean 1 (O101 const-fold,
    // not an identity rewrite). tclsh: 2&&1 == 1, 3||0 == 1.
    assert!(optimised("set v [expr {2 && 1}]", TCL).contains("set v 1"));
    assert!(optimised("set v [expr {3 || 0}]", TCL).contains("set v 1"));

    // !!(boolean) collapses: ==/!=/< are already boolean (tclsh sweep == ).
    assert!(
        optimised(&in_proc("set v [expr {!!($a == $b)}]"), TCL).contains("set v [expr {$a == $b}]")
    );
    assert!(
        optimised(&in_proc("set v [expr {!($a == $b)}]"), TCL).contains("set v [expr {$a != $b}]")
    );
    // The ordered comparisons invert only on operands proved non-NaN — an
    // untyped `$a` may hold NaN, for which `!($a < 1)` is 1 while `$a >= 1` is 0.
    // The `int_x` wrapper supplies the proof.
    assert!(!optimised(&in_proc("set v [expr {!($a < $b)}]"), TCL).contains("$a >= $b"));
    assert!(optimised(&int_x("set v [expr {!($x < 1)}]"), TCL).contains("set v [expr {$x >= 1}]"));

    // The optimiser does NOT simplify `!($a in $b)` → `$a ni $b` here (no
    // optimisation fires); a known gap.
}

#[test]
fn instcombine_de_morgan() {
    // tclsh sweep (a,b ∈ {0,1}): !($a && $b) == !$a || !$b; !($a || $b) == !$a && !$b.
    assert!(
        optimised(&int_xy("set v [expr {!($x && $y)}]"), TCL).contains("set v [expr {!$x || !$y}]")
    );
    assert!(opt_fires(
        &int_xy("set v [expr {!($x && $y)}]"),
        TCL,
        "O110"
    ));
    assert!(
        optimised(&int_xy("set v [expr {!($x || $y)}]"), TCL).contains("set v [expr {!$x && !$y}]")
    );
    assert!(opt_fires(
        &int_xy("set v [expr {!($x || $y)}]"),
        TCL,
        "O110"
    ));

    // Operands of unknown type keep the written form: the two sides raise
    // different errors for a non-boolean string (see `int_xy`).
    assert!(
        optimised(&in_proc("set v [expr {!($a && $b)}]"), TCL)
            .contains("set v [expr {!($a && $b)}]")
    );

    // De Morgan + comparison inversion via fixpoint (tclsh 4-var sweep == ).
    // `==` inverts unconditionally; the ordered `$c < $d` half keeps its `!`
    // because neither operand is proved non-NaN.
    assert!(
        optimised(&in_proc("set v [expr {!($a == $b && $c < $d)}]"), TCL)
            .contains("set v [expr {$a != $b || !($c < $d)}]")
    );
    assert!(
        optimised(&in_proc("set v [expr {!($a == $b || $c < $d)}]"), TCL)
            .contains("set v [expr {$a != $b && !($c < $d)}]")
    );

    // De Morgan inside an `if` condition.
    // The rewrite is reported under O113 (strength-reduce) rather than O110,
    // because the `if`-condition rewrite path is owned by the strength-reduction
    // pass; the replacement text is the same. tclsh proves `!($x && $y)` ==
    // `!$x || !$y`. Assert the rewrite + that either code carried it.
    let in_if = &int_xy("if {!($x && $y)} { puts yes }");
    assert!(optimised(in_if, TCL).contains("!$x || !$y"));
    assert!(opt_fires(in_if, TCL, "O110") || opt_fires(in_if, TCL, "O113"));
    let carried = opt_rewrites(in_if, TCL)
        .into_iter()
        .any(|(c, r)| (c == "O110" || c == "O113") && r.contains("!$x || !$y"));
    assert!(
        carried,
        "De Morgan rewrite must appear in O110/O113 replacement"
    );
}

#[test]
fn instcombine_self_comparison_tautologies() {
    // tclsh sweep: x == x ⇒ 1, x != x ⇒ 0 — but NOT for NaN, where tclsh gives
    // 0 and 1 respectively, so the fold needs $x proved non-NaN.
    assert!(optimised(&int_x("set v [expr {$x == $x}]"), TCL).contains("set v 1"));
    assert!(optimised(&int_x("set v [expr {$x != $x}]"), TCL).contains("set v 0"));
    // Untyped $x keeps the comparison.
    assert!(optimised(&in_proc("set v [expr {$x == $x}]"), TCL).contains("$x == $x"));
    assert!(optimised(&in_proc("set v [expr {$x != $x}]"), TCL).contains("$x != $x"));
}

#[test]
fn instcombine_ternary_and_boolean_context() {
    // tclsh: 1 ? a : b == a, 0 ? a : b == b (constant condition selects a branch).
    assert!(optimised(&in_proc("set v [expr {1 ? $a : $b}]"), TCL).contains("set v [expr {$a}]"));
    assert!(opt_fires(
        &in_proc("set v [expr {1 ? $a : $b}]"),
        TCL,
        "O110"
    ));
    assert!(optimised(&in_proc("set v [expr {0 ? $a : $b}]"), TCL).contains("set v [expr {$b}]"));
    assert!(opt_fires(
        &in_proc("set v [expr {0 ? $a : $b}]"),
        TCL,
        "O110"
    ));

    // In a boolean (`if`) context, `($a > $b) ? 1 : 0` simplifies. O110 drops the
    // redundant parens to `$a > $b ? 1 : 0` (still contains `$a > $b`). tclsh:
    // `($a>$b)?1:0` and the paren-stripped form are identical (both yield the
    // boolean of $a>$b). Assert the substring predicate, which holds.
    let ten10 = "if {($a > $b) ? 1 : 0} { puts yes }";
    assert!(opt_fires(ten10, TCL, "O110"));
    let has = opt_rewrites(ten10, TCL)
        .into_iter()
        .any(|(c, r)| c == "O110" && r.contains("$a > $b"));
    assert!(
        has,
        "boolean-context ternary O110 replacement must mention `$a > $b`"
    );

    // In an `if`, `!!$x` → `$x` (tclsh: !!x is the boolean of x; in a condition
    // identical to x). Rust carries this as O110 with replacement `{$x}`.
    let dn_if = &int_xy("if {!!$x} { puts yes }");
    assert!(optimised(dn_if, TCL).contains("if {$x}"));
    let dn = opt_rewrites(dn_if, TCL)
        .into_iter()
        .any(|(c, r)| c == "O110" && r == "{$x}");
    assert!(dn, "double-not in if must rewrite to `{{$x}}` under O110");

    // NOTE on omissions (known gaps; sound to leave un-rewritten):
    //  - `$c ? $a : $a` (identical branches) — not folded.
    //  - `!$c ? $a : $b` → `$c ? $b : $a` — not flipped.
    //  - `$x ? 0 : 1` → `!$x` — not folded.
}

// Structure elimination — O112 constant-condition compound statements

/// The iRules word operators fold through **SCCP** — not just through the
/// expression-simplification passes that already carried a dialect.
///
/// SCCP, interprocedural propagation, the static-loop simulator, and
/// codegen's expression folder must all evaluate under the document's actual
/// dialect: a dialect-blind policy leaves `FoldOps::is_irules` `false`, so
/// every word operator declines to fold. The `eq` control below folds on the
/// same input, proving any loss is dialect threading rather than the fold
/// itself.
#[test]
fn irules_word_operators_fold_through_sccp() {
    const IR: &str = "f5-irules";
    // A known-constant subject: the lattice resolves `$x` to `abcde` and the
    // word operator folds, collapsing the condition to a literal.
    let contains = "when HTTP_REQUEST {\n    set x \"abcde\"\n    if {$x contains \"cd\"} {\n        pool p1\n    }\n}";
    let contains_out = optimised(contains, IR);
    assert!(
        opt_fires(contains, IR, "O112")
            && contains_out.contains("pool p1")
            && !contains_out.contains("contains"),
        "`contains` on a known constant must fold under f5-irules; got {contains_out:?}"
    );
    // Control: `eq`, an operator plain Tcl shares, folds on the identical
    // shape — the two dialect halves now agree.
    let eq = "when HTTP_REQUEST {\n    set x \"abcde\"\n    if {$x eq \"abcde\"} {\n        pool p1\n    }\n}";
    assert!(opt_fires(eq, IR, "O112"));

    // A provably-false subject folds the other way.
    let miss = "when HTTP_REQUEST {\n    set x \"abcde\"\n    if {$x contains \"zz\"} {\n        pool p1\n    }\n}";
    let miss_out = optimised(miss, IR);
    assert!(
        opt_fires(miss, IR, "O112")
            && !miss_out.contains("pool p1")
            && !miss_out.contains("contains"),
        "a provably-false `contains` must fold to 0; got {miss_out:?}"
    );

    // TN: plain Tcl has no word operators, so the same text must not fold —
    // the dialect gate still holds after the threading.
    assert!(
        opt_codes(contains, TCL).is_empty(),
        "plain Tcl must decline the iRules word-operator fold; got {:?}",
        opt_codes(contains, TCL)
    );
}
