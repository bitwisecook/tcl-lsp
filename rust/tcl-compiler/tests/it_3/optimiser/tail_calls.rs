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

//! Tail calls O121 to O123: detection, loop conversion, the accumulator hint and self calls that block the conversion.

use super::*;

// Tail-call optimisation — O121 (tailcall) / O122 (loop) / O123 (accumulator)
//
// The optimise_with_dialect set is post-overlap; selection makes O122 subsume
// per-site O121. For a tail call we therefore assert the disjunction
// "O121 or O122" on the applied set.

#[test]
fn tail_call_detection_o121_o122() {
    // Tail-position self-call ⇒ O121 (tailcall) or O122 (loop). tclsh: the
    // tailcall/loop forms compute the same result as the original recursion
    // (factorial 5 1 == 120, gcd 48 36 == 12 — proven in the module doc).
    let pos = [
        "proc factorial {n acc} {\n    if {$n <= 1} {\n        return $acc\n    }\n    return [factorial [expr {$n - 1}] [expr {$n * $acc}]]\n}\n",
        "proc loop {items} {\n    if {[llength $items] == 0} {\n        return\n    }\n    puts [lindex $items 0]\n    loop [lrange $items 1 end]\n}\n",
        "proc gcd {a b} {\n    if {$b == 0} {\n        return $a\n    } else {\n        return [gcd $b [expr {$a % $b}]]\n    }\n}\n",
        "proc walk {tree} {\n    switch [lindex $tree 0] {\n        leaf {\n            return [lindex $tree 1]\n        }\n        node {\n            walk [lindex $tree 2]\n        }\n    }\n}\n",
        "namespace eval ns {\n    proc f {n} {\n        return [::ns::f [expr {$n - 1}]]\n    }\n}\n",
    ];
    for src in pos {
        assert!(
            opt_fires(src, TCL, "O121") || opt_fires(src, TCL, "O122"),
            "tail-call should be detected: {src}"
        );
    }

    // Non-tail / mutual / mixed self-calls ⇒ neither O121 nor O122.
    let neg = [
        "proc fib {n} {\n    if {$n <= 1} {\n        return $n\n    }\n    expr {[fib [expr {$n - 1}]] + [fib [expr {$n - 2}]]}\n}\n",
        "proc even {n} {\n    if {$n == 0} { return 1 }\n    return [odd [expr {$n - 1}]]\n}\nproc odd {n} {\n    if {$n == 0} { return 0 }\n    return [even [expr {$n - 1}]]\n}\n",
    ];
    for src in neg {
        assert!(!opt_fires(src, TCL, "O121"), "no O121: {src}");
        assert!(!opt_fires(src, TCL, "O122"), "no O122: {src}");
    }
}

#[test]
fn tail_call_loop_conversion_o122() {
    // O122 rewrites tail recursion to a `while {1}` loop. tclsh proved factorial
    // and gcd loop-forms equal the recursive originals.
    let fac = "proc factorial {n acc} {\n    if {$n <= 1} {\n        return $acc\n    }\n    return [factorial [expr {$n - 1}] [expr {$n * $acc}]]\n}\n";
    // Both arguments are bracketed words, so the call passes one argument per
    // parameter and O122 takes the whole proc; overlap selection prefers it
    // over the per-site O121 `tailcall` covering the same range. tclsh proved
    // the loop form equals the recursive original for n in 0..20.
    let fo = optimised(fac, TCL);
    assert!(
        fo.contains("while {1}"),
        "expected the loop conversion: {fo}"
    );
    assert!(
        fo.contains("lassign [list [expr {$n - 1}] [expr {$n * $acc}]] n acc"),
        "each bracketed argument must stay one word: {fo}",
    );
    assert!(opt_fires(fac, TCL, "O122"));

    // The bare self-call `loop` body DOES take the O122 loop conversion.
    let bare = "proc loop {items} {\n    if {[llength $items] == 0} {\n        return\n    }\n    puts [lindex $items 0]\n    loop [lrange $items 1 end]\n}\n";
    let bo = optimised(bare, TCL);
    assert!(bo.contains("while {1}"));
    assert!(bo.contains("set items"));
    assert!(opt_fires(bare, TCL, "O122"));

    // O122 must NOT fire for non-tail / mixed recursion.
    let nontail = "proc fib {n} {\n    if {$n <= 1} {\n        return $n\n    }\n    expr {[fib [expr {$n - 1}]] + [fib [expr {$n - 2}]]}\n}\n";
    assert!(!opt_fires(nontail, TCL, "O122"));
    let mixed = "proc bad {n acc} {\n    if {$n <= 1} {\n        return $acc\n    }\n    set partial [bad [expr {$n - 2}] $acc]\n    return [bad [expr {$n - 1}] $partial]\n}\n";
    assert!(!opt_fires(mixed, TCL, "O122"));

    // O122 must NOT fire when a self-call sits in a condition / switch subject.
    for src in [
        "proc f {n} {\n    if {[f $n]} {\n        return [f [expr {$n - 1}]]\n    } else {\n        return $n\n    }\n}\n",
        "proc f {n} {\n    switch [f [expr {$n - 1}]] {\n        0 { return 0 }\n        default { return [f [expr {$n - 2}]] }\n    }\n}\n",
        "proc f {n} {\n    while {[f $n]} {\n        return [f [expr {$n - 1}]]\n    }\n    return $n\n}\n",
        "proc f {n} {\n    for {set i 0} {[f $n]} {incr i} {\n        return [f [expr {$n - 1}]]\n    }\n    return $n\n}\n",
    ] {
        assert!(
            !opt_fires(src, TCL, "O122"),
            "no O122 (self-call in control): {src}"
        );
    }

    // Arity mismatch: O121 may fire but O122 (full loop conversion) must not.
    let arity = "proc f {a b} {\n    return [f $a]\n}\n";
    assert!(opt_fires(arity, TCL, "O121"));
    assert!(!opt_fires(arity, TCL, "O122"));

    // Braced literal `[self ...]` is not an executable call ⇒ no O121/O122.
    let braced = "proc f {n} {\n    return {[f $n]}\n}\n";
    assert!(!opt_fires(braced, TCL, "O121"));
    assert!(!opt_fires(braced, TCL, "O122"));

    // Literal bracket text in a braced `set` value must not count as recursion;
    // the real tail call still converts.
    let braced_set = "proc fact {n acc} {\n    set marker {[fact $n]}\n    if {$n <= 1} {\n        return $acc\n    }\n    return [fact [expr {$n - 1}] [expr {$n * $acc}]]\n}\n";
    assert!(opt_fires(braced_set, TCL, "O121") || opt_fires(braced_set, TCL, "O122"));

    // Selection subsumes per-site O121 under a chosen O122 — or, in the
    // selection for this body, keeps O121 and drops O122. Either way exactly one
    // of the two is present (not both) for the simple factorial body.
    let suppress = "proc f {n acc} {\n    if {$n <= 1} { return $acc }\n    return [f [expr {$n - 1}] [expr {$n * $acc}]]\n}\n";
    let has121 = opt_fires(suppress, TCL, "O121");
    let has122 = opt_fires(suppress, TCL, "O122");
    assert!(
        has121 ^ has122,
        "exactly one of O121/O122 should survive selection"
    );
}

#[test]
fn accumulator_hint_o123() {
    // O123 is a hint (hint_only) for accumulator-eligible non-tail recursion.
    let fac = "proc factorial {n} {\n    if {$n <= 1} {\n        return 1\n    }\n    return [expr {$n * [factorial [expr {$n - 1}]]}]\n}\n";
    assert!(opt_fires(fac, TCL, "O123"));

    // O123 must NOT fire for already-tail-recursive / non-recursive / doubly-
    // recursive / non-accumulator patterns.
    let neg = [
        "proc factorial {n acc} {\n    if {$n <= 1} {\n        return $acc\n    }\n    return [factorial [expr {$n - 1}] [expr {$n * $acc}]]\n}\n",
        "proc add {a b} {\n    return [expr {$a + $b}]\n}\n",
        "proc fib {n} {\n    if {$n <= 1} { return $n }\n    return [expr {[fib [expr {$n-1}]] + [fib [expr {$n-2}]]}]\n}\n",
        "proc transform {x} {\n    if {$x eq \"\"} { return \"\" }\n    return [format \"%s\" [transform [string range $x 1 end]]]\n}\n",
    ];
    for src in neg {
        assert!(!opt_fires(src, TCL, "O123"), "no O123: {src}");
    }

    // The O123 finding is hint-only (informational, not an applied rewrite).
    let registry = static_context_for(TCL).commands();
    let o123: Vec<_> = optimise_with_dialect(
        fac,
        registry,
        Some(tcl_registry::model::ingress::resolve_environment(TCL).analyser_profile()),
    )
    .into_iter()
    .filter(|o| o.code.as_str() == "O123")
    .collect();
    assert_eq!(o123.len(), 1);
    assert!(o123[0].hint_only);

    // Mixed tail + non-tail still surfaces the O123 hint for the non-tail site.
    let mixed = "proc calc {n} {\n    if {$n <= 0} { return 0 }\n    if {$n == 1} {\n        return [expr {$n * [calc [expr {$n - 1}]]}]\n    }\n    return [calc [expr {$n - 2}]]\n}\n";
    assert!(opt_fires(mixed, TCL, "O123"));

    // Braced expr literal must not produce an O123 hint.
    let braced = "proc f {n} {\n    return {[expr {$n * [f [expr {$n - 1}]]}]}\n}\n";
    assert!(!opt_fires(braced, TCL, "O123"));

    // O123 coexists with independent passes (O102 constant fold of `$a + 0`).
    // tclsh: the n==1 base case returns 1; the recursive return is untouched.
    let coexist = "proc f {n} {\n    set a 1\n    if {$n <= 1} { return [expr {$a + 0}] }\n    return [expr {$n * [f [expr {$n - 1}]]}]\n}\n";
    assert!(opt_fires(coexist, TCL, "O123"));
}

/// O122's gate counts every self-call and converts only when all of them are
/// in tail position. A statement that keeps no argument words counted zero.
///
/// `count_self_calls_in_stmt` enumerated the variants that retain argument
/// text and closed with a wildcard, so a fused `AssignExpr`, a bare
/// `ExprEval` and an `Incr` each reported no self-call at all. Under-counting
/// is the unsound direction — it makes the tail-site count match the total —
/// and the same proc converted or not depending only on how its non-tail call
/// was spelled (#2118).
#[test]
fn a_non_tail_self_call_blocks_o122_however_it_is_spelled() {
    for (why, nontail) in [
        (
            "a plain command word",
            "set acc [combine $acc [walk [left $node] 0]]",
        ),
        (
            "a fused expression assignment",
            "set acc [expr {$acc + [walk [left $node] 0]}]",
        ),
        (
            "a bare expression statement",
            "expr {[walk [left $node] 0]}",
        ),
        ("an incr amount", "incr acc [walk [left $node] 0]"),
    ] {
        let src = format!(
            "proc walk {{node acc}} {{\n    if {{$node eq \"\"}} {{\n        return $acc\n    }}\n    {nontail}\n    return [walk [right $node] $acc]\n}}\n"
        );
        assert!(
            !opt_fires(&src, TCL, "O122"),
            "{why}: the loop body would still recurse: {:?}",
            opt_codes(&src, TCL)
        );
    }
}

/// Precision: the gate counts self-calls, not brackets. A nested call to
/// something else is not recursion, so the tail call still converts.
#[test]
fn o122_still_converts_past_a_nested_call_to_another_proc() {
    for (why, nested) in [
        (
            "a fused expression assignment",
            "set acc [expr {$acc + [weight $n]}]",
        ),
        ("an incr amount", "incr acc [weight $n]"),
    ] {
        let src = format!(
            "proc walk {{n acc}} {{\n    if {{$n <= 0}} {{\n        return $acc\n    }}\n    {nested}\n    return [walk [expr {{$n - 1}}] $acc]\n}}\n"
        );
        assert!(
            opt_fires(&src, TCL, "O122"),
            "{why}: `weight` is not a self-call: {:?}",
            opt_codes(&src, TCL)
        );
    }
}
