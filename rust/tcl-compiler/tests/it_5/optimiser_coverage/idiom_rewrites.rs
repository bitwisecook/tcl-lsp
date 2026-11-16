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

//! O113 to O120: strength reduction, `incr`, nested `expr`, list and `lindex` folding, string-length checks, packing and string comparison.

use super::*;

// O113 — strength reduction

#[test]
fn o113_strength_reduction() {
    // tclsh sweep: x ** 2 == x * x.
    let p = optimised(&int_xy("if {$x ** 2} {}"), TCL);
    assert!(!p.contains("**"));
    assert!(p.contains('*'));
    assert!(opt_fires(&int_xy("if {$x ** 2} {}"), TCL, "O113"));

    // tclsh sweep (x≥0): x % 8 == x & 7, x % 16 == x & 15, x % 2 == x & 1.
    let m8 = optimised(&in_proc("if {$x % 8} {}"), TCL);
    assert!(m8.contains('&'));
    assert!(!m8.contains('%'));
    assert!(optimised(&in_proc("if {$x % 16} {}"), TCL).contains("& 15")); // tclsh: x%16 == x&15
    assert!(optimised(&in_proc("if {$x % 2} {}"), TCL).contains("& 1")); // tclsh: x%2 == x&1

    // Non-power-of-two modulus is not reduced.
    assert!(opt_absent("if {$x % 7} {}", TCL, "O113"));
    // x ** 3 is not reduced (only x ** 2).
    assert!(opt_absent("if {$x ** 3} {}", TCL, "O113"));
    // Command subst present ⇒ no reduction (side effects).
    assert!(opt_absent("if {[clock seconds] ** 2} {}", TCL, "O113"));
}

// O114 — incr idiom recognition (needs SSA-known INT type, kept live)

#[test]
fn o114_incr_idiom() {
    // the loop-counter pattern makes `x` INT-typed AND live (read via
    // `puts $x`), so the rewrite is sound. tclsh: `set x N; set x [expr {$x+1}]`
    // is the same as `incr x` for INT x.
    let add1 = "proc foo {n} {\n  for {set x 0} {$x < $n} {incr x} {\n    set x [expr {$x + 1}]\n    puts $x\n  }\n}\nfoo 3\n";
    assert!(optimised(add1, TCL).contains("incr x"));
    assert!(opt_fires(add1, TCL, "O114"));

    let add5 = "proc foo {n} {\n  for {set x 0} {$x < $n} {incr x} {\n    set x [expr {$x + 5}]\n    puts $x\n  }\n}\nfoo 3\n";
    assert!(optimised(add5, TCL).contains("incr x 5")); // tclsh: x+5 == incr x 5

    let sub3 = "proc foo {n} {\n  for {set x 0} {$x < $n} {incr x} {\n    set x [expr {$x - 3}]\n    puts $x\n  }\n}\nfoo 3\n";
    assert!(optimised(sub3, TCL).contains("incr x -3")); // tclsh: x-3 == incr x -3

    // Commutative `N + $x` also matches.
    let comm = "proc foo {n} {\n  for {set x 0} {$x < $n} {incr x} {\n    set x [expr {1 + $x}]\n    puts $x\n  }\n}\nfoo 3\n";
    assert!(optimised(comm, TCL).contains("incr x"));

    // Multiplication is not an incr idiom.
    let mul = "proc foo {n} {\n  for {set x 0} {$x < $n} {incr x} {\n    set x [expr {$x * 2}]\n    puts $x\n  }\n}\nfoo 3\n";
    assert!(opt_absent(mul, TCL, "O114"));

    // Unknown-type param must NOT be incr-rewritten (could be a float at runtime;
    // tclsh: `set x [expr {$x + 1}]` on 1.5 ⇒ 2.5, but `incr x` errors).
    assert!(opt_absent(
        "proc foo {x} {\n    set x [expr {$x + 1}]\n}\n",
        TCL,
        "O114"
    ));
    // Different target var ⇒ no incr.
    assert!(opt_absent(
        "proc foo {x} {\n    set y [expr {$x + 1}]\n}\n",
        TCL,
        "O114"
    ));
}

// O115 — redundant nested expr elimination

#[test]
fn o115_nested_expr_unwrap() {
    // `if {[expr {...}]}` — the nested expr is redundant in a condition. tclsh:
    // `if {[expr {$x + 1}]}` == `if {$x + 1}`.
    let in_if = "if {[expr {$x + 1}]} {}";
    assert!(!optimised(in_if, TCL).contains("[expr"));
    assert!(opt_fires(in_if, TCL, "O115"));

    // `[expr {[expr {...}]}]` in a return value unwraps the inner. tclsh:
    // `[expr {[expr {$x * 2}]}]` == `[expr {$x * 2}]`.
    let in_ret = "proc double_expr {x} {\n    return [expr {[expr {$x * 2}]}]\n}";
    assert!(optimised(in_ret, TCL).contains("return [expr {$x * 2}]"));
    assert!(opt_fires(in_ret, TCL, "O115"));

    // `[string length $s]` is not an expr — not unwrapped.
    assert!(opt_absent("if {[string length $s]} {}", TCL, "O115"));

    // NOTE on gaps (sound): `while {[expr {...}]}` and
    // `set y [expr {[expr {...}]}]` are not unwrapped (only if/return paths fire).
}

// O116 — constant list folding

#[test]
fn o116_list_folding() {
    // [list a] folds away the `[list` call (via O116 or O100 propagation).
    // tclsh: [list a] == "a".
    let single = "set x [list a]\nputs $x";
    assert!(!optimised(single, TCL).contains("[list"));
    assert!(opt_any_of(single, TCL, &["O116", "O100"]));

    // Empty list folds (O116, or SCCP O100/O109).
    assert!(opt_any_of(
        "set x [list]\nputs $x",
        TCL,
        &["O116", "O100", "O109"]
    ));

    // Multi-element [list a b c] must NOT fold via O116 (intrep shimmer); it may
    // be propagated as a braced word via O100, which is semantically identical
    // (tclsh: [list a b c] == {a b c}). Key invariant: no O116.
    assert!(opt_absent("set x [list a b c]\nputs $x", TCL, "O116"));
    // List with a variable element ⇒ not folded.
    assert!(opt_absent("set x [list $a b c]\nputs $x", TCL, "O116"));
    // Element with a space ⇒ not O116.
    assert!(opt_absent("set x [list \"a b\" c]\nputs $x", TCL, "O116"));
}

// O117 — string length zero-check simplification

#[test]
fn o117_strlen_zero_check() {
    // tclsh sweep: ([string length $s] == 0) <=> ($s eq ""); != 0 <=> ne "".
    assert!(opt_fires(
        &in_proc("if {[string length $s] == 0} {}"),
        TCL,
        "O117"
    ));
    assert!(opt_fires(
        &in_proc("if {[string length $s] != 0} {}"),
        TCL,
        "O117"
    ));
    // Swapped `0 == [string length $s]` also simplifies.
    assert!(opt_fires(
        &in_proc("if {0 == [string length $s]} {}"),
        TCL,
        "O117"
    ));

    // [string length $s] == 5 is NOT simplified.
    assert!(opt_absent("if {[string length $s] == 5} {}", TCL, "O117"));

    // Gaps (sound): the `> 0`, `<= 0`, and swapped `0 < [string length $s]` forms
    // are equivalent to `ne ""` / `eq ""` but only the `== 0` / `!= 0` / `0 ==`
    // forms are simplified. Confirm the conservative no-op for `> 0` (left
    // unchanged is sound).
    assert!(opt_absent("if {[string length $s] > 0} {}", TCL, "O117"));
    assert!(opt_absent("if {[string length $s] <= 0} {}", TCL, "O117"));
    assert!(opt_absent("if {0 < [string length $s]} {}", TCL, "O117"));
}

// O118 — constant lindex folding

#[test]
fn o118_lindex_folding() {
    // tclsh-verified each fold result.  The registry fold also enters the
    // SCCP lattice, so the `$x` read downstream
    // propagates too (`puts $x` → `puts a`) — semantically identical
    // (tclsh: both print `a`), one hop further folded.
    assert_eq!(
        optimised("set x [lindex {a b c} 0]\nputs $x", TCL),
        "set x a\nputs a"
    ); // tclsh: a
    assert_eq!(
        optimised("set x [lindex {a b c} 1]\nputs $x", TCL),
        "set x b\nputs b"
    ); // tclsh: b
    assert_eq!(
        optimised("set x [lindex {x y z} end]\nputs $x", TCL),
        "set x z\nputs z"
    ); // tclsh: z
    assert_eq!(
        optimised("set x [lindex {a b c d} end-1]\nputs $x", TCL),
        "set x c\nputs c"
    ); // tclsh: c

    // Out-of-range index folds (to empty); Rust emits O118 (or SCCP O100/O109).
    assert!(opt_any_of(
        "set x [lindex {a b} 5]\nputs $x",
        TCL,
        &["O118", "O100", "O109"]
    ));

    // Variable index ⇒ not folded.
    assert!(opt_absent(
        "set x [lindex {a b c} $i]\nputs $x",
        TCL,
        "O118"
    ));

    // MORE precise (tclsh-proven sound): one might expect `[lindex {{a b} c} 0]`
    // to NOT fold. tclsh disproves the caution: `[lindex {{a b} c} 0]` == "a b",
    // and it folds to `set x {a b}` — `set x {a b}; puts $x` prints "a b",
    // byte-identical to the original. Assert the sound rewrite rather than an
    // over-cautious no-op.
    assert_eq!(
        optimised("set x [lindex {{a b} c} 0]\nputs $x", TCL),
        // tclsh: [lindex {{a b} c} 0] == "a b"; the lattice fold then
        // propagates the read as the brace-quoted single word, which
        // prints identically (`puts {a b}` == `puts $x` == "a b").
        "set x {a b}\nputs {a b}"
    );
}

// O119 — multi-set packing

#[test]
fn o119_multi_set_packing() {
    // As in optimiser.rs: with an `eval {$a $b $c}` barrier the constants are
    // forwarded THROUGH the braced `eval {...}` literal (O102), and the command
    // the script runs is one the module cannot see, which may read the globals
    // `a`, `b` and `c`: the stores stay and O119 packs them.

    // Tcl 9.0: individual `set` is faster ⇒ O119 must not fire.
    assert!(opt_absent(
        "set a 1\nset b 2\nset c 3\nputs \"$a $b $c\"",
        "tcl9.0",
        "O119"
    ));
    // Too few consecutive sets ⇒ no packing.
    assert!(opt_absent("set a 1\nset b 2\neval {$a $b}", TCL, "O119"));
    // The words are forwarded and the stores are packed. tclsh: the same program.
    assert_eq!(
        optimised("set a 1\nset b 2\nset c 3\neval {$a $b $c}", TCL),
        "lassign {1 2 3} a b c\neval {1 2 3}"
    );
}

// O120 — prefer eq/ne for string comparisons

#[test]
fn o120_string_compare_eq_ne() {
    // tclsh sweep: $a == "hello" <=> $a eq "hello" ("hello" is non-numeric).
    assert!(optimised(&in_proc("if {$a == \"hello\"} {}"), TCL).contains("$a eq \"hello\""));
    assert!(opt_fires(&in_proc("if {$a == \"hello\"} {}"), TCL, "O120"));

    // != inside an expr substitution ⇒ ne. Rust routes this through O110 (expr
    // instcombine) rather than O120; the rewrite text is identical. tclsh sweep:
    // $a != "hello" <=> $a ne "hello".
    let ne = &in_proc("set ok [expr {$a != \"hello\"}]");
    assert!(optimised(ne, TCL).contains("$a ne \"hello\""));
    assert!(opt_any_of(ne, TCL, &["O120", "O110"]));

    // String literal on the left also rewrites. tclsh: "hello" == $a <=> eq.
    assert!(optimised(&in_proc("if {\"hello\" == $a} {}"), TCL).contains("eq"));
    // != with a string on the right ⇒ ne.
    assert!(optimised(&in_proc("if {$x != \"foo\"} {}"), TCL).contains("ne"));

    // Conservative non-rewrites (D5-O120: needs ≥1 provably-non-numeric operand).
    for src in [
        "set a [clock seconds]\nif {$a == \"1\"} {}", // INT-typed var, numeric literal
        "set a [string trim $raw]\nif {$a == \"1\"} {}", // STRING type ≠ non-numeric value
        "set a [string trim $raw]\nif {$a == \"true\"} {}", // boolean-like literal
        "set a [string trim $raw]\nif {$a == \"1.25\"} {}", // float-like literal
        "set a [string trim $x]\nset b [string trim $y]\nif {$a == $b} {}", // two STRING vars
        "if {$a == $b} {}",                           // both unknown
        "if {$a == \"true\"} {}",                     // boolean-like literal, unknown var
        "if {$a == \"1.25\"} {}",                     // float-like literal, unknown var
    ] {
        assert!(opt_absent(src, TCL, "O120"), "O120 must not fire: {src}");
    }
}

#[test]
fn o120_with_sccp_const_and_mixed() {
    // A var known non-numeric via SCCP CONST ("foo") proves the string path.
    // tclsh: `set a foo; expr {$a == $b}` == `expr {$a eq $b}` for every $b.
    let vc = "set a foo\nif {$a == $b} {}";
    assert!(opt_fires(vc, TCL, "O120"));
    assert!(optimised(vc, TCL).contains("eq $b"));

    // Mixed expr: only the string compare is rewritten; numeric `$n == 1` stays.
    // Rust carries this via O110 canonicalisation; the eq-half is present.
    let mixed = "set a [string trim $raw]\nif {$a == \"x\" && $n == 1} {}";
    let mx = optimised(mixed, TCL);
    assert!(mx.contains("$a eq \"x\""));
    assert!(mx.contains("$n == 1"));
    assert!(opt_any_of(mixed, TCL, &["O120", "O110"]));

    // Two SCCP-constant operands: the compare folds outright. tclsh: "foo" !=
    // "bar" ⇒ condition is constant-false ⇒ `if {0}` deleted (O112).
    let vv = "set a foo\nset b bar\nif {$a == $b} {}";
    let v = optimised(vv, TCL);
    assert!(!v.contains("=="));
    assert!(!v.contains(" eq "));
    assert!(opt_fires(vv, TCL, "O112"));
}
