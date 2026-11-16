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

//! Idiom rewrites O113 to O120 and O128, from strength reduction to end offsets, with their negatives.

use super::*;

// Pattern-match simplification — O110 matches_regex / matches_glob (f5-irules)
//
// OMISSION: the entire matches_regex/matches_glob → string-op simplification
// family does NOT fire in the optimiser (no O110 for any of the anchored-regex /
// wildcard-glob reprs; all sources pass through unchanged). Leaving them
// unchanged is sound (no miscompile), just not optimised, so rather than assert
// a rewrite we pin the conservative behaviour: the negative cases (which must NOT
// simplify) all hold, and the "positive" cases are a known optimiser gap.

#[test]
fn pattern_match_simplification_negatives_hold() {
    const IR: &str = "f5-irules";
    // Negative cases that must remain `matches_regex` / `matches_glob`.
    // These all hold (the construct is simply never simplified).
    let neg = [
        "when HTTP_REQUEST { if { [HTTP::uri] matches_regex {.html$} } { pool p1 } }",
        "when HTTP_REQUEST { if { [HTTP::uri] matches_regex {^/api/.*} } { pool p1 } }",
        "when HTTP_REQUEST { if { [HTTP::uri] matches_regex {^/[a-z]+$} } { pool p1 } }",
        "when HTTP_REQUEST { if { [HTTP::uri] matches_regex {} } { pool p1 } }",
        "when HTTP_REQUEST { if { [HTTP::uri] matches_regex $pattern } { pool p1 } }",
        "when HTTP_REQUEST { if { $host matches_glob {*.example.co?} } { pool p1 } }",
        "when HTTP_REQUEST { if { $host matches_glob {api.*.com} } { pool p1 } }",
        "when HTTP_REQUEST { if { $host matches_glob {*} } { pool p1 } }",
    ];
    for src in neg {
        assert!(!opt_fires(src, IR, "O110"), "must not simplify: {src}");
    }
    // The would-be-positive cases (anchored regex, leading/trailing-star glob)
    // also do not fire — confirm no spurious O110 is emitted, matching the
    // conservative path. (These could in principle simplify; a known gap.)
    let unsimplified_but_sound = [
        "when HTTP_REQUEST { if { [HTTP::uri] matches_regex {^/api$} } { pool p1 } }",
        "when HTTP_REQUEST { if { $host matches_glob {*.example.com} } { pool p1 } }",
    ];
    for src in unsimplified_but_sound {
        assert_eq!(optimised(src, IR), src);
    }
}

// Strength reduction — O113

#[test]
fn strength_reduction_o113() {
    // tclsh sweep: x ** 2 == x * x.
    let pow = &int_xy("if {$x ** 2} {}");
    let p = optimised(pow, TCL);
    assert!(!p.contains("**"));
    assert!(p.contains('*'));
    assert!(opt_fires(pow, TCL, "O113"));

    // tclsh sweep (x≥0): x % 8 == x & 7.
    let modp = "if {$x % 8} {}";
    let m = optimised(modp, TCL);
    assert!(m.contains('&'));
    assert!(!m.contains('%'));
    assert!(opt_fires(modp, TCL, "O113"));
}

// Incr idiom — O114 (needs SSA-known INT type on the loop var)

#[test]
fn incr_idiom_o114() {
    // A minimal `proc foo {} {set x 0; set x [expr {$x + N}]}` body has x unused,
    // so the whole thing is DSE'd (O108/O109) before the incr-idiom can apply. To
    // make x both INT-typed *and* live (the precondition D5-O114 documents and
    // the fp/opt.rs FP_OPT_10_TN_REPRO uses), drive it inside a `for` loop that
    // reads x via `puts $x`. tclsh: `set x N; set x [expr {$x+1}]` == `incr x`.
    let add1 = "proc foo {n} {\n  for {set x 0} {$x < $n} {incr x} {\n    set x [expr {$x + 1}]\n    puts $x\n  }\n}\nfoo 3\n";
    assert!(optimised(add1, TCL).contains("incr x"));
    assert!(opt_fires(add1, TCL, "O114"));

    let add5 = "proc foo {n} {\n  for {set x 0} {$x < $n} {incr x} {\n    set x [expr {$x + 5}]\n    puts $x\n  }\n}\nfoo 3\n";
    assert!(optimised(add5, TCL).contains("incr x 5"));
    assert!(opt_fires(add5, TCL, "O114"));

    let sub3 = "proc foo {n} {\n  for {set x 0} {$x < $n} {incr x} {\n    set x [expr {$x - 3}]\n    puts $x\n  }\n}\nfoo 3\n";
    assert!(optimised(sub3, TCL).contains("incr x -3"));
    assert!(opt_fires(sub3, TCL, "O114"));
}

// Nested expr unwrap — O115

#[test]
fn nested_expr_unwrap_o115() {
    // `if {[expr {...}]}` — the nested expr is redundant in a condition. tclsh:
    // `if {[expr {$x + 1}]}` == `if {$x + 1}`.
    let in_if = "if {[expr {$x + 1}]} {}";
    assert!(!optimised(in_if, TCL).contains("[expr"));
    assert!(opt_fires(in_if, TCL, "O115"));

    // `[expr {[expr {...}]}]` in a value position (return) unwraps the inner.
    // tclsh: `[expr {[expr {$x * 2}]}]` == `[expr {$x * 2}]`.
    let in_ret = "proc double_expr {x} {\n    return [expr {[expr {$x * 2}]}]\n}";
    assert!(optimised(in_ret, TCL).contains("return [expr {$x * 2}]"));
    assert!(opt_fires(in_ret, TCL, "O115"));

    // The optimiser does NOT unwrap a nested expr in a `set` value position such
    // as `set y [expr {[expr {$x * 2}]}]` (only the `return`/condition paths
    // fire). Sound to leave unchanged; a known gap.
}

// List folding / lindex folding — O116 / O118

#[test]
fn list_and_lindex_folding() {
    // [list a] folds (via O116 or O100 propagation); tclsh: [list a] == "a".
    let single = "set x [list a]\nputs $x";
    assert!(!optimised(single, TCL).contains("[list"));
    let codes = opt_codes(single, TCL);
    assert!(codes.iter().any(|c| c == "O116" || c == "O100"));

    // Multi-element [list a b c] must NOT fold to a braced literal under O116
    // (intrep shimmer). It is instead propagated as a braced word via O100, which
    // is semantically identical (`[list a b c]` == `{a b c}` in tclsh) — the key
    // invariant is that O116 specifically does not fire.
    let multi = "set x [list a b c]\nputs $x";
    assert!(!opt_fires(multi, TCL, "O116"));

    // lindex of a literal list folds. tclsh: [lindex {a b c} 1] == "b".
    let lf = "set x [lindex {a b c} 1]\nputs $x";
    assert!(!optimised(lf, TCL).contains("[lindex"));
    assert!(
        opt_codes(lf, TCL)
            .iter()
            .any(|c| c == "O118" || c == "O100")
    );

    // tclsh: [lindex {x y z} end] == "z".
    let le = "set x [lindex {x y z} end]\nputs $x";
    assert!(!optimised(le, TCL).contains("[lindex"));
    assert!(
        opt_codes(le, TCL)
            .iter()
            .any(|c| c == "O118" || c == "O100")
    );
}

// Strlen zero-check — O117

#[test]
fn strlen_zero_check_o117() {
    // tclsh sweep: ([string length $s] == 0) <=> ($s eq "").
    let eq0 = "if {[string length $s] == 0} {}";
    let e = optimised(eq0, TCL);
    assert!(e.contains("eq \"\"") || !e.contains("string length"));
    assert!(opt_fires(eq0, TCL, "O117"));

    // tclsh sweep: ([string length $s] != 0) <=> ($s ne "").
    let ne0 = "if {[string length $s] != 0} {}";
    let n = optimised(ne0, TCL);
    assert!(n.contains("ne \"\"") || !n.contains("string length"));
    assert!(opt_fires(ne0, TCL, "O117"));

    // The optimiser does NOT simplify the `> 0` form `[string length $s] > 0` →
    // `$s ne ""` (only `== 0` / `!= 0`). Sound to leave unchanged
    // (`[string length $s] > 0` and `$s ne ""` are equivalent, just not folded);
    // a known gap.
    assert_eq!(
        optimised(&in_proc("if {[string length $s] > 0} {}"), TCL),
        "if {[string length $s] > 0} {}"
    );
}

// String compare eq/ne — O120

#[test]
fn string_compare_eq_ne_o120() {
    // tclsh sweep: $a == "hello" <=> $a eq "hello" ("hello" is non-numeric).
    let eq = "if {$a == \"hello\"} {}";
    assert!(optimised(eq, TCL).contains("$a eq \"hello\""));
    assert!(opt_fires(eq, TCL, "O120"));

    // != inside an expr command substitution → ne. Rust routes this through
    // O110 (expr instcombine) rather than O120, but the rewrite is identical:
    // tclsh sweep $a != "hello" <=> $a ne "hello". Assert the text + that some
    // optimisation produced it.
    let ne = &in_proc("set ok [expr {$a != \"hello\"}]");
    assert!(optimised(ne, TCL).contains("$a ne \"hello\""));
    assert!(opt_fires(ne, TCL, "O120") || opt_fires(ne, TCL, "O110"));

    // A var known non-numeric via SCCP CONST ("foo") proves the string path:
    // tclsh: `set a foo; expr {$a == $b}` == `expr {$a eq $b}` for every $b.
    // Rust folds the CONST in and rewrites to eq.
    let vc = "set a foo\nif {$a == $b} {}";
    assert!(opt_fires(vc, TCL, "O120"));
    assert!(optimised(vc, TCL).contains("eq $b"));

    // Mixed expr: only the string comparison is rewritten, numeric `$n == 1`
    // left alone. tclsh: $a (string) eq "x" preserves the && result. Rust
    // carries this via O110 canonicalisation; the eq-half is present.
    let mixed = "set a [string trim $raw]\nif {$a == \"x\" && $n == 1} {}";
    let mx = optimised(mixed, TCL);
    assert!(mx.contains("$a eq \"x\""));
    assert!(mx.contains("$n == 1"));
    assert!(opt_fires(mixed, TCL, "O120") || opt_fires(mixed, TCL, "O110"));

    // Two SCCP-constant operands: the compare folds outright (better than the
    // eq rewrite). tclsh: "foo" != "bar" ⇒ condition constant-false ⇒ `if {0}`
    // deleted. Confirm neither == nor a residual `eq` survives.
    let vv = "set a foo\nset b bar\nif {$a == $b} {}";
    let v = optimised(vv, TCL);
    assert!(!v.contains("=="));
    assert!(!v.contains(" eq "));
    assert!(opt_fires(vv, TCL, "O112"));
}

#[test]
fn string_compare_o120_conservative_non_rewrites() {
    // These must NOT rewrite to eq — the D5-O120 at-least-one-non-numeric rule.
    // Each is tclsh-grounded (numeric-looking literal vs a var that could hold a
    // number ⇒ ==/eq can disagree).
    let cases = [
        "set a [clock seconds]\nif {$a == \"1\"} {}", // INT-typed var, numeric literal
        "set a [string trim $raw]\nif {$a == \"1\"} {}", // STRING type ≠ non-numeric value
        "set a [string trim $raw]\nif {$a == \"true\"} {}",
        "set a [string trim $x]\nset b [string trim $y]\nif {$a == $b} {}",
        "set a [expr {1 + 2}]\nset b [expr {3 + 4}]\nif {$a == $b} {}", // both INT
        "if {$a == $b} {}",                                             // both unknown
        "if {$a == \"true\"} {}", // boolean-like literal, unknown var
        "if {$a == \"1.25\"} {}", // float-like literal, unknown var
    ];
    for src in cases {
        assert!(!opt_fires(src, TCL, "O120"), "O120 must not fire: {src}");
    }
}

// Multi-set packing — O119

#[test]
fn multi_set_packing_o119() {
    // With an `eval {$a $b $c}` barrier the constants are forwarded *through*
    // the `eval {...}` braced literal (O102) — `eval {1 2 3}`. The command the
    // script runs, `1` here, is one the module cannot see, and at the top level
    // it may read the globals `a`, `b` and `c`, so their stores stay and O119
    // packs them. tclsh: `set a 1; set b 2; set c 3; eval {$a $b $c}`, the
    // folded `eval {1 2 3}` and the packed form are the same program.

    // Tcl 9.0: individual `set` is faster ⇒ O119 must not fire.
    let t9 = "set a 1\nset b 2\nset c 3\nputs \"$a $b $c\"";
    assert!(!opt_fires(t9, "tcl9.0", "O119"));

    // Too few consecutive sets ⇒ no packing.
    let few = "set a 1\nset b 2\nputs \"$a $b\"";
    assert!(!opt_fires(few, TCL, "O119"));

    // The words are forwarded and the stores, which the unseen command may
    // read, are packed.
    assert_eq!(
        optimised("set a 1\nset b 2\nset c 3\neval {$a $b $c}", TCL),
        "lassign {1 2 3} a b c\neval {1 2 3}"
    );
}

// End-offset index rewrite — O128

#[test]
fn end_offset_rewrites_o128() {
    // Each tclsh-verified: the length-arithmetic index equals the end-offset.
    // (Sweeps in the module doc proved lindex/lrange/lreplace/string index/range.)
    let cases: &[(&str, &str)] = &[
        (
            "set x [lindex $L [expr {[llength $L] - 1}]]",
            "set x [lindex $L end]",
        ),
        (
            "set x [lindex $L [expr {[llength $L] - 2}]]",
            "set x [lindex $L end-1]",
        ),
        (
            "set x [lrange $L 0 [expr {[llength $L] - 1}]]",
            "set x [lrange $L 0 end]",
        ),
        (
            "set x [string index $s [expr {[string length $s] - 1}]]",
            "set x [string index $s end]",
        ),
        (
            "set x [string range $s 0 [expr {[string length $s] - 1}]]",
            "set x [string range $s 0 end]",
        ),
        (
            "set x [lindex ${my::list} [expr {[llength ${my::list}] - 1}]]",
            "set x [lindex ${my::list} end]",
        ),
        (
            "puts [lindex $L [expr {[llength $L] - 3}]]",
            "puts [lindex $L end-2]",
        ),
        (
            "set x [lindex $a(1) [expr {[llength $a(1)] - 1}]]",
            "set x [lindex $a(1) end]",
        ),
        (
            "set x [lindex $L [expr {[llength $L] - 1}] 0]",
            "set x [lindex $L end 0]",
        ),
    ];
    for (src, want) in cases {
        assert_eq!(optimised(src, TCL), *want, "O128 rewrite: {src}");
        assert!(opt_fires(src, TCL, "O128"), "O128 should fire: {src}");
    }

    // lreplace rewrites BOTH indices ⇒ two O128.
    let lr = "set x [lreplace $L [expr {[llength $L] - 2}] [expr {[llength $L] - 1}] foo]";
    assert_eq!(optimised(lr, TCL), "set x [lreplace $L end-1 end foo]");
    assert_eq!(opt_count(lr, TCL, "O128"), 2);

    // Inside a proc body.
    let inproc =
        "proc last {L} {\n    set r [lindex $L [expr {[llength $L] - 1}]]\n    return $r\n}";
    assert!(optimised(inproc, TCL).contains("[lindex $L end]"));
    assert!(opt_fires(inproc, TCL, "O128"));
}

#[test]
fn end_offset_o128_must_not_fire() {
    // The robustness guard: each unsafe pattern must NOT rewrite. All confirmed
    // unchanged (no O128). Several are paired in the firing test above
    // with their safe counterpart.
    let neg = [
        "set x [lindex $L [expr {[llength $M] - 1}]]", // mismatched var
        "set x [lindex $L [expr {[llength $L] - 0}]]", // -0 is past the end
        "set x [lindex $L [expr {[llength $L]}]]",     // bare length
        "set x [lindex $L [expr {[string length $L] - 1}]]", // wrong length cmd
        "set x [string index $s [expr {[llength $s] - 1}]]", // wrong length cmd
        "set x [linsert $L [expr {[llength $L] - 1}] foo]", // linsert excluded
        "set x [linsert $L [expr {[llength $L] - 3}] foo]",
        "set x [lindex $L 0 [expr {[llength $L] - 1}]]", // later multi-index pos
        "set x [lindex $a(1) [expr {[llength $a(2)] - 1}]]", // mismatched array elem
        "set x [lindex [get_list] [expr {[llength [get_list]] - 1}]]", // cmd-sub container
        "set x [lindex {a b c d} [expr {[llength {a b c d}] - 1}]]", // literal container
        "set x [lindex $L [expr {[llength $L] - $N}]]",  // non-literal offset
        "set x [lindex $L [expr {[llength $L] + 1}]]",   // addition
        "set x [lindex $L [expr {[llength $L] * 2 - 1}]]", // multiplication
        "set x [lindex $L [expr {1 - [llength $L]}]]",   // reversed subtraction
        "set x [lindex $L [expr {[llength $L] - 1 - 1}]]", // chained subtraction
        "set x [lindex $L [expr {[llength $L] - -1}]]",  // negative constant
        "set x [lindex $L [expr [llength $L] - 1]]",     // unbraced expr
        "set x [lindex $L [expr {[llength [lsort $L]] - 1}]]", // llength of cmd result
        "set x [lindex $L:extra [expr {[llength $L:extra] - 1}]]", // adjacent text
        "lset L [expr {[llength $L] - 1}] foo",          // lset excluded
        "set x [lindex $L [expr {[llength $L] - [get_offset]}]]", // other substitution
        "set x [lindex $L [expr {{[llength $L] - 1}}]]", // nested brace grouping
    ];
    for src in neg {
        assert!(!opt_fires(src, TCL, "O128"), "O128 must not fire: {src}");
    }

    // Partial rewrites: when only one index matches, exactly one O128 fires.
    let one = "set x [lrange $L 2 [expr {[llength $L] - 1}]]";
    assert_eq!(optimised(one, TCL), "set x [lrange $L 2 end]");
    assert_eq!(opt_count(one, TCL, "O128"), 1);

    let mismatch = "set x [lreplace $L [expr {[llength $M] - 1}] [expr {[llength $L] - 1}] foo]";
    let mo = optimised(mismatch, TCL);
    assert!(mo.contains("end"));
    assert!(mo.contains("[llength $M] - 1"));
    assert_eq!(opt_count(mismatch, TCL, "O128"), 1);

    // First-position multi-index IS relative to $L ⇒ rewrites even with a
    // trailing index. tclsh: `[lindex $L [...-1] 0]` == `[lindex $L end 0]`.
    let first = "set x [lindex $L [expr {[llength $L] - 1}] 0]";
    assert_eq!(optimised(first, TCL), "set x [lindex $L end 0]");
    assert!(opt_fires(first, TCL, "O128"));

    // Matching array element rewrites.
    let arr = "set x [lindex $a(1) [expr {[llength $a(1)] - 1}]]";
    assert_eq!(optimised(arr, TCL), "set x [lindex $a(1) end]");
    assert!(opt_fires(arr, TCL, "O128"));

    // MORE precise (tclsh-proven sound): one might expect
    // `[lindex ${a(1)} [expr {[llength $a(1)] - 1}]]` to NOT rewrite, on the
    // theory that braced `${a(1)}` and bare `$a(1)` "compile to different loads".
    // tclsh disproves that: `set a(1) hello; ${a(1)}` and `$a(1)` both read array
    // element a(1) and are byte-identical, and the end-offset rewrite preserves
    // the value (sweep: `[lindex ${a(1)} [...-1]]` == `[lindex ${a(1)} end]` →
    // OK). O128 therefore (correctly) fires here; assert the sound rewrite rather
    // than an over-conservative no-op.
    let braced_arr = "set x [lindex ${a(1)} [expr {[llength $a(1)] - 1}]]";
    assert_eq!(optimised(braced_arr, TCL), "set x [lindex ${a(1)} end]");
    assert!(opt_fires(braced_arr, TCL, "O128"));
}

// Variable-shape optimisation guardrails — variable-shape forms not conflated

#[test]
fn variable_shape_guardrails() {
    for src in [
        "set x ${a(1)}\nputs $x",
        "set x $a(1)\nputs $x",
        "set x $::ns::arr(k)\nputs $x",
    ] {
        assert_eq!(optimised(src, TCL), src);
        assert!(
            opt_codes(src, TCL).is_empty(),
            "no rewrite for shape: {src}"
        );
    }
}
