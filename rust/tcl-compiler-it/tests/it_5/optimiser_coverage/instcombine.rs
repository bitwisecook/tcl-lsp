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

//! O110: reassociation, identities, tautologies, inversions and De Morgan.

use super::*;

// O110 — InstCombine: reassociation / identity / annihilator

#[test]
fn o110_reassociation() {
    crate::install_surfaces();
    // Regrouping needs every term proven integer — wrap in `int_x`. tclsh
    // sweep over integers: $x + 1 + 2 == $x + 3, $x * 2 * 3 == $x * 6,
    // $x + 3 - 1 == $x + 2.
    assert!(optimised(&int_x("set v [expr {$x + 1 + 2}]"), TCL).contains("$x + 3"));
    assert!(optimised(&int_x("set v [expr {$x * 2 * 3}]"), TCL).contains("$x * 6"));
    assert!(optimised(&int_x("set v [expr {$x + 3 - 1}]"), TCL).contains("$x + 2"));
    // Real fold through reassoc: ($x + 1) + 2 flattens to $x + 3.
    assert!(optimised(&int_x("set v [expr {($x + 1) + 2}]"), TCL).contains("$x + 3"));
    assert!(opt_fires(
        &int_x("set v [expr {($x + 1) + 2}]"),
        TCL,
        "O110"
    ));
    // An unproven term keeps its chain: over a double the rounding is
    // order-dependent (`set x 10000000000000000.0; expr {$x + 1 + 2}` prints
    // `10000000000000002.0` and `expr {$x + 3}` `10000000000000004.0` under
    // tclsh 8.5 to 9.1).
    assert!(opt_absent("set v [expr {$a + 1 + 2}]", TCL, "O110"));
    assert!(opt_absent("set v [expr {$a * 2 * 3}]", TCL, "O110"));
    // #1962: a `return [expr {...}]` value position agrees with the `set`
    // body above — the same reassoc folds there too.
    let returned =
        "proc f {n} {\n  for {set x 0} {$x < $n} {incr x} {}\n  return [expr {($x + 1) + 2}]\n}\n";
    assert!(optimised(returned, TCL).contains("return [expr {$x + 3}]"));
    assert!(opt_fires(returned, TCL, "O110"));
}

#[test]
fn o110_identity_and_annihilator() {
    crate::install_surfaces();
    // Identity/annihilator drops need a provably-INT operand (D5-O110) — wrap in
    // `int_x`. tclsh sweep (x≥0): x**0==1, x**1==x, x<<0==x, x>>0==x, x&0==0,
    // x|0==x, x^0==x, x%1==0, x*0==0, x*1==x, x+0==x, x-0==x, x/1==x.
    assert!(optimised(&int_x("set v [expr {$x ** 0}]"), TCL).contains("set v 1")); // tclsh: 1
    assert!(opt_fires(&int_x("set v [expr {$x ** 1}]"), TCL, "O110"));
    assert!(opt_fires(&int_x("set v [expr {$x << 0}]"), TCL, "O110"));
    assert!(opt_fires(&int_x("set v [expr {$x >> 0}]"), TCL, "O110"));
    assert!(optimised(&int_x("set v [expr {$x & 0}]"), TCL).contains("set v 0")); // tclsh: 0
    assert!(opt_fires(&int_x("set v [expr {$x | 0}]"), TCL, "O110"));
    assert!(opt_fires(&int_x("set v [expr {$x ^ 0}]"), TCL, "O110"));
    assert!(optimised(&int_x("set v [expr {$x % 1}]"), TCL).contains("set v 0")); // tclsh: 0
    assert!(optimised(&int_x("set v [expr {$x * 0}]"), TCL).contains("set v 0")); // tclsh: 0
    assert!(opt_fires(&int_x("set v [expr {$x * 1}]"), TCL, "O110"));
    assert!(opt_fires(&int_x("set v [expr {$x + 0}]"), TCL, "O110"));
    assert!(opt_fires(&int_x("set v [expr {$x - 0}]"), TCL, "O110"));
    assert!(opt_fires(&int_x("set v [expr {$x / 1}]"), TCL, "O110"));
    assert!(optimised(&int_x("set v [expr {$x % 1}]"), TCL).contains("set v 0"));

    // Left-side identity (commutativity). tclsh sweep: 0|x==x, 0^x==x.
    assert!(opt_fires(&int_x("set v [expr {0 | $x}]"), TCL, "O110"));
    assert!(opt_fires(&int_x("set v [expr {0 ^ $x}]"), TCL, "O110"));
}

#[test]
fn o110_logical_identities() {
    crate::install_surfaces();
    // Logical annihilators/identities do not require an INT proof (boolean
    // context). tclsh sweep: x&&0==0, x||1==1, 0&&x==0, 1||x==1.
    assert!(optimised(&int_xy("set v [expr {$x && 0}]"), TCL).contains("set v 0")); // tclsh: 0
    assert!(optimised(&int_xy("set v [expr {$x || 1}]"), TCL).contains("set v 1")); // tclsh: 1
    assert!(optimised(&in_proc("set v [expr {0 && $x}]"), TCL).contains("set v 0")); // tclsh: 0
    assert!(optimised(&in_proc("set v [expr {1 || $x}]"), TCL).contains("set v 1")); // tclsh: 1
    // x&&1, x||0, 1&&x, 0||x simplify (O110) without becoming a constant.
    assert!(opt_fires(&int_xy("set v [expr {$x && 1}]"), TCL, "O110"));
    assert!(opt_fires(&int_xy("set v [expr {$x || 0}]"), TCL, "O110"));
    assert!(opt_fires(&in_proc("set v [expr {1 && $x}]"), TCL, "O110"));
    assert!(opt_fires(&in_proc("set v [expr {0 || $x}]"), TCL, "O110"));
}

#[test]
fn o110_whitespace_noise_suppressed() {
    crate::install_surfaces();
    // Whitespace-only differences must NOT fire O110 (the user's spacing is not a
    // defect). All confirmed no-O110.
    for src in [
        "set v [expr {$x-1}]",
        "set v [expr {$x*$y}]",
        "set v [expr {$a*$b/$c}]",
        "set v [expr {$secure ? 6697: 6667}]",
        "set v [expr {$tagnumber >>7}]",
        "proc f {x} { if {$x<0} { return -1 }; return 1 }",
        "proc f {x} { while {$x>0} { incr x -1 } }",
        // Pure commutative reorder — no win.
        "proc f {x} { return [expr {1 + $x}] }",
        "proc f {x} { return [expr {2 * $x}] }",
        // Mixed bitwise/shift parens are kept for readability (CERT EXP00-C).
        "set v [expr {($x >> 16) & 0xFF}]",
        // Gaps (sound): `$x - -1` ⇒ `$x + 1` and the same-op bitwise chain
        // `(a & b) & c` could fire O110 but NEITHER does (no rewrite) — leaving
        // both unchanged is sound.
        "proc f {x} { return [expr {$x - -1}] }",
        "set v [expr {($a & $b) & $c}]",
    ] {
        assert!(opt_absent(src, TCL, "O110"), "spurious O110: {src}");
    }

    // Genuine structural changes that DO still fire O110.
    // Additive identity `$x + 0` ⇒ `$x` (INT-proven).
    assert!(opt_fires(&int_x("set v [expr {$x + 0}]"), TCL, "O110"));
    // Boolean simplify `($action && 1) ? ...` ⇒ `$action ? ...`. `$action` is
    // bound first: tclsh errors on reading an unset variable.
    assert!(opt_fires(
        "set action [gets stdin]\nset v [expr {($action && 1) ? \"on\" : \"off\"}]",
        TCL,
        "O110"
    ));
}

#[test]
fn o110_self_comparison_tautologies() {
    crate::install_surfaces();
    // tclsh sweep (any defined $x, string or numeric — `<`/`>` use the string
    // fallback): x<x ⇒ 0, x>x ⇒ 0 — false for every value, NaN included, so
    // these fold with no type proof at all.
    assert!(optimised(&in_proc("set v [expr {$x < $x}]"), TCL).contains("set v 0"));
    assert!(optimised(&in_proc("set v [expr {$x > $x}]"), TCL).contains("set v 0"));

    // The *reflexive* rows are wrong for a NaN operand — tclsh proves
    // `set x NaN; expr {$x == $x}` ⇒ 0 and `expr {$x != $x}` ⇒ 1, the
    // opposite of the fold. So they must not fire on an untyped $x…
    for src in [
        "set v [expr {$x == $x}]",
        "set v [expr {$x != $x}]",
        "set v [expr {$x <= $x}]",
        "set v [expr {$x >= $x}]",
    ] {
        let out = optimised(src, TCL);
        assert!(
            out.contains("$x"),
            "untyped self-comparison must not fold (NaN): {src} ⇒ {out}"
        );
    }
    // …but an INT-proven $x cannot be NaN, so there the folds stand. tclsh
    // sweep (x≥0): x==x ⇒ 1, x!=x ⇒ 0, x<=x ⇒ 1, x>=x ⇒ 1.
    assert!(optimised(&int_x("set v [expr {$x == $x}]"), TCL).contains("set v 1"));
    assert!(optimised(&int_x("set v [expr {$x != $x}]"), TCL).contains("set v 0"));
    assert!(optimised(&int_x("set v [expr {$x <= $x}]"), TCL).contains("set v 1"));
    assert!(optimised(&int_x("set v [expr {$x >= $x}]"), TCL).contains("set v 1"));
    // String self-comparison. tclsh: x eq x ⇒ 1, x ne x ⇒ 0 (any $x).
    assert!(optimised(&in_proc("set v [expr {$x eq $x}]"), TCL).contains("set v 1"));
    assert!(optimised(&in_proc("set v [expr {$x ne $x}]"), TCL).contains("set v 0"));
    // x ^ x ⇒ 0 — needs an INT proof (xor errors on non-numeric strings); the
    // `int_x` wrapper supplies it. tclsh sweep (x≥0): x^x == 0.
    assert!(optimised(&int_x("set v [expr {$x ^ $x}]"), TCL).contains("set v 0"));

    // FIXED (was a miscompile): bare `set v [expr {$x - $x}]` on an UNTYPED $x
    // must NOT fold to `set v 0`. tclsh proves the original RAISES an error when
    // $x is a non-numeric string:
    //   set x hello; expr {$x - $x}  ⇒  "can't use non-numeric string as operand
    //   of \"-\"" (8.6) / "cannot use non-numeric string ... as left operand"
    //   (9.0)  — catch ⇒ 1
    // so folding to 0 would silently swallow that error. Subtraction is
    // arithmetic-only, unlike `==`/`<` (string fallback), so this is not folded.
    assert!(
        !optimised(&in_proc("set v [expr {$x - $x}]"), TCL).contains("set v 0"),
        "untyped $x - $x must not fold to 0 (arithmetic op errors on non-numeric)"
    );
    // …but with a provably-numeric operand the fold IS sound. tclsh sweep (x≥0):
    // x-x == 0. The `int_x` wrapper makes $x provably numeric.
    assert!(optimised(&int_x("set v [expr {$x - $x}]"), TCL).contains("set v 0"));
}

#[test]
fn o110_unary_and_not_inversions() {
    crate::install_surfaces();
    // tclsh: !!(bool) collapses (==/!=/< are already boolean).
    assert!(optimised(&in_proc("set v [expr {!!($a == $b)}]"), TCL).contains("$a == $b"));
    // tclsh sweep: !(a==b) ⇒ a!=b, !(a!=b) ⇒ a==b. `!=` is the exact complement
    // of `==` for every value, NaN included, so these need no type proof.
    assert!(optimised(&in_proc("set v [expr {!($a == $b)}]"), TCL).contains("$a != $b"));
    assert!(optimised(&in_proc("set v [expr {!($a != $b)}]"), TCL).contains("$a == $b"));

    // The ordered four are NOT their own negations on a NaN operand —
    // tclsh proves `set a NaN; expr {!($a < 1)}` ⇒ 1 while `expr {$a >= 1}`
    // ⇒ 0. Untyped operands must be left as written…
    for src in [
        "set v [expr {!($a < $b)}]",
        "set v [expr {!($a >= $b)}]",
        "set v [expr {!($a > $b)}]",
        "set v [expr {!($a <= $b)}]",
    ] {
        let out = optimised(src, TCL);
        assert!(out.contains('!'), "untyped {src} must keep its `!`: {out}");
    }
    // …and fire again once the operands are INT-proven (an integer is never
    // NaN). tclsh sweep (x ≥ 0): !(x<1) ⇒ x>=1, !(x>=1) ⇒ x<1, !(x>1) ⇒ x<=1,
    // !(x<=1) ⇒ x>1.
    assert!(optimised(&int_x("set v [expr {!($x < 1)}]"), TCL).contains("$x >= 1"));
    assert!(optimised(&int_x("set v [expr {!($x >= 1)}]"), TCL).contains("$x < 1"));
    assert!(optimised(&int_x("set v [expr {!($x > 1)}]"), TCL).contains("$x <= 1"));
    assert!(optimised(&int_x("set v [expr {!($x <= 1)}]"), TCL).contains("$x > 1"));
    // String comparison inversions. tclsh: !(a eq b) ⇒ a ne b, !(a ne b) ⇒ a eq b.
    assert!(optimised(&in_proc("set v [expr {!($a eq $b)}]"), TCL).contains("$a ne $b"));
    assert!(optimised(&in_proc("set v [expr {!($a ne $b)}]"), TCL).contains("$a eq $b"));

    // Double-not in an `if` condition simplifies (O110). tclsh: !!x ⇒ x (bool ctx).
    assert!(opt_fires(&int_xy("if {!!$x} { puts yes }"), TCL, "O110"));
    assert!(optimised(&int_xy("if {!!$x} { puts yes }"), TCL).contains("if {$x}"));

    // INT-proven unary folds: ~~x ⇒ x, -(-x) ⇒ x, +x ⇒ x. tclsh sweep (x≥0).
    assert!(optimised(&int_x("set v [expr {~~$x}]"), TCL).contains("set v [expr {$x}]"));
    assert!(optimised(&int_x("set v [expr {-(-$x)}]"), TCL).contains("$x"));
    assert!(opt_fires(&int_x("set v [expr {+$x}]"), TCL, "O110"));

    // NOTE on gaps (sound to leave unchanged):
    //  - `!($a in $b)` ⇒ `$a ni $b` and `!($a ni $b)` ⇒ `$a in $b` — in/ni are
    //    not inverted.
    //  - `-(5)` ⇒ `-5` literal negation — not folded (stays `-(5)`).
}

#[test]
fn o110_de_morgan() {
    crate::install_surfaces();
    // tclsh sweep (a,b ∈ {0,1,5}): !($a && $b) == !$a || !$b; !($a || $b) ==
    // !$a && !$b; the 4-var chain matches its inversion (tclsh-proven).
    assert!(optimised(&int_xy("set v [expr {!($x && $y)}]"), TCL).contains("!$x || !$y"));
    assert!(optimised(&int_xy("set v [expr {!($x || $y)}]"), TCL).contains("!$x && !$y"));
    // Operands of unknown type keep the written form: the two sides raise
    // different errors for a non-boolean string (see `int_xy`).
    assert!(optimised(&in_proc("set v [expr {!($a && $b)}]"), TCL).contains("!($a && $b)"));
    // De Morgan itself still distributes over the chain; the `$a == $b` half
    // inverts, while the ordered `$c < $d` half keeps its `!` because neither
    // operand is proved non-NaN.
    assert!(
        optimised(&in_proc("set v [expr {!($a == $b && $c < $d)}]"), TCL)
            .contains("$a != $b || !($c < $d)")
    );

    // De Morgan inside an `if` condition is carried under O110 or O113 (the
    // if-condition rewrite path is owned by the strength-reduce pass). The
    // rewrite text matches tclsh either way.
    let in_if = &int_xy("if {!($x && $y)} { puts yes }");
    assert!(optimised(in_if, TCL).contains("!$x || !$y"));
    assert!(opt_any_of(in_if, TCL, &["O110", "O113"]));
}

#[test]
fn o110_ternary_and_double_not() {
    crate::install_surfaces();
    // Constant-condition ternary simplifies (O110). tclsh: 1?a:b==a, 0?a:b==b.
    assert!(opt_fires(
        &in_proc("set v [expr {1 ? $a : $b}]"),
        TCL,
        "O110"
    ));
    assert!(opt_fires(
        &in_proc("set v [expr {0 ? $a : $b}]"),
        TCL,
        "O110"
    ));
    // Double-not in an `if`. (Structural: O110 fires.)
    assert!(opt_fires(&int_xy("if {!!$x} { puts yes }"), TCL, "O110"));
    // De Morgan inside an `if` is reported under O110/O113.
    assert!(opt_any_of(
        &int_xy("if {!($x && $y)} { puts yes }"),
        TCL,
        &["O110", "O113"]
    ));
    // Boolean-context ternary `($a > $b) ? 1 : 0` simplifies (O110).
    assert!(opt_fires(
        &in_proc("if {($a > $b) ? 1 : 0} { puts yes }"),
        TCL,
        "O110"
    ));

    // NOTE on gaps (sound): `$x ? 0 : 1` ⇒ `!$x`, `$c ? $a : $a` (identical
    // branches), and `!$c ? $a : $b` ⇒ `$c ? $b : $a` (cond flip) are not folded.
    assert!(opt_absent("set v [expr {$x ? 0 : 1}]", TCL, "O110"));
}

#[test]
fn o110_no_instcombine_with_command_subst() {
    crate::install_surfaces();
    // x+0=x is unsafe when x has side effects ([clock seconds]). No O110.
    assert!(opt_absent(
        "set v [expr {[clock seconds] + 0}]",
        TCL,
        "O110"
    ));
}

// De Morgan / inversion — exercised via O110 rewrites

#[test]
fn demorgan_logic_via_o110() {
    crate::install_surfaces();
    // The De-Morgan logic is the O110 expr rewrite; assert the transforms on
    // `[expr {...}]`. All tclsh-swept above.
    assert!(optimised(&int_xy("set v [expr {!($x && $y)}]"), TCL).contains("!$x || !$y"));
    assert!(optimised(&int_xy("set v [expr {!($x || $y)}]"), TCL).contains("!$x && !$y"));
    // Plain `&&` / `==` / single `!` are NOT De-Morgan candidates (no rewrite to
    // the inverse form fires).
    assert!(optimised(&in_proc("set v [expr {$a && $b}]"), TCL).contains("$a && $b"));
    assert!(!optimised(&in_proc("set v [expr {$a == $b}]"), TCL).contains("||"));
}

#[test]
fn invert_logic_via_o110() {
    crate::install_surfaces();
    // The inversion logic surfaces as the `!(...)` O110 rewrite. tclsh-swept.
    assert!(optimised(&in_proc("set v [expr {!($a == $b)}]"), TCL).contains("$a != $b"));
    // The ordered rows need both operands proved non-NaN.
    assert!(optimised(&int_x("set v [expr {!($x < 1)}]"), TCL).contains("$x >= 1"));
    assert!(optimised(&int_x("set v [expr {!($x >= 1)}]"), TCL).contains("$x < 1"));
    assert!(optimised(&int_xy("set v [expr {!($x && $y)}]"), TCL).contains("!$x || !$y"));
    assert!(optimised(&int_xy("set v [expr {!($x || $y)}]"), TCL).contains("!$x && !$y"));
    // Complex chain: De Morgan distributes and `==` inverts; the ordered half
    // keeps its `!` on untyped operands.
    assert!(
        optimised(&in_proc("set v [expr {!($a == $b && $c < $d)}]"), TCL)
            .contains("$a != $b || !($c < $d)")
    );
    // Double-not removal in a boolean context. tclsh: !!x in `if` == x.
    assert!(optimised(&int_xy("if {!!$x} { puts yes }"), TCL).contains("if {$x}"));
}

/// The `!(x <cmp> y)` inversion is derived from `BinOp::inverse()`
/// (`tcl_syntax::expr::operators`), which covers all fourteen comparison
/// operators: `==`/`!=`/`<`/`>=`/`>`/`<=`/`eq`/`ne`, the TIP 461
/// string-ordering four (`lt`/`le`/`gt`/`ge`), and list membership
/// (`in`/`ni`). The negation identity holds across all of them alike (a
/// total order for `lt`/etc., a direct definitional negation for `in`/`ni`).
#[test]
fn invert_logic_covers_tip461_and_membership_operators() {
    crate::install_surfaces();
    const TCL9: &str = "tcl9.0";
    assert!(optimised(&in_proc("set v [expr {!($a lt $b)}]"), TCL9).contains("$a ge $b"));
    assert!(optimised(&in_proc("set v [expr {!($a le $b)}]"), TCL9).contains("$a gt $b"));
    assert!(optimised(&in_proc("set v [expr {!($a gt $b)}]"), TCL9).contains("$a le $b"));
    assert!(optimised(&in_proc("set v [expr {!($a ge $b)}]"), TCL9).contains("$a lt $b"));
    assert!(optimised(&in_proc("set v [expr {!($a eq $b)}]"), TCL9).contains("$a ne $b"));
    assert!(optimised(&in_proc("set v [expr {!($a in $b)}]"), TCL9).contains("$a ni $b"));
    assert!(optimised(&in_proc("set v [expr {!($a ni $b)}]"), TCL9).contains("$a in $b"));
}
