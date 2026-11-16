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

//! FP-RBS-03, 11, 16, 17, 18, 19 and 21: loop bodies that run at least once, frozen loops and expanded loops.

use super::*;

// FP-RBS-03 — frozen-loop bodies (while/for with cmd-sub condition)

const FP_RBS_03_REPRO: &str = "\
proc f {fp} {
    # gets writes 'line' AND the body sets 'n' — both are body-local
    # but the frozen-loop body keeps them invisible to SSA defs.
    while {[gets $fp line] >= 0} {
        set n [string length $line]
        puts \"$line ($n chars)\"
    }
}
";

#[test]
fn fp_rbs_03_frozen_while_body_silent() {
    // FP-RBS-03: body writes (n, and line itself via cmd-sub) must not false-flag W210.
    assert!(
        !fires(FP_RBS_03_REPRO, D, "W210"),
        "FP-RBS-03: frozen-while-body reads must NOT fire W210; emitted: {:?}",
        codes(FP_RBS_03_REPRO, D)
    );
    assert!(
        !fires(FP_RBS_03_REPRO, D, "W213"),
        "FP-RBS-03: frozen-while-body reads must NOT fire W213; emitted: {:?}",
        codes(FP_RBS_03_REPRO, D)
    );
}

#[test]
fn fp_rbs_03_genuine_unset_in_body_still_fires() {
    // TP control: $missing (never written anywhere) must still fire W210.
    let src = "\
proc f {fp} {
    while {[gets $fp line] >= 0} {
        puts \"$line $missing\"
    }
}
";
    assert!(
        fires(src, D, "W210"),
        "FP-RBS-03 TP: $missing (never written anywhere) must fire W210; emitted: {:?}",
        codes(src, D)
    );
}

// FP-RBS-11 — qualified-builtin loops (::foreach / ::lmap / ::for / ::while)

const FP_RBS_11_REPRO: &str = "\
proc f {dict} {
    # html.tcl:153 pattern: ::foreach is just qualified foreach.
    # Loop vars k,v and body reads must all be silent.
    ::foreach {k v} $dict { puts \"$k=$v\" }
}
";

#[test]
fn fp_rbs_11_qualified_foreach_silent() {
    // FP-RBS-11: ::foreach is recognised by loop-form recovery, so loop vars k/v don't false-flag.
    assert!(
        !fires(FP_RBS_11_REPRO, D, "W210"),
        "FP-RBS-11: qualified ::foreach must NOT fire W210; emitted: {:?}",
        codes(FP_RBS_11_REPRO, D)
    );
    assert!(
        !fires(FP_RBS_11_REPRO, D, "W213"),
        "FP-RBS-11: qualified ::foreach must NOT fire W213; emitted: {:?}",
        codes(FP_RBS_11_REPRO, D)
    );
}

#[test]
fn fp_rbs_11_qualified_for_silent() {
    // FP control: the qualified ::for form works the same way.
    let src = "\
proc f {} {
    ::for {set i 0} {$i < 3} {incr i} { puts $i }
}
";
    assert!(
        !fires(src, D, "W210"),
        "FP-RBS-11: qualified ::for must NOT fire W210; emitted: {:?}",
        codes(src, D)
    );
    assert!(
        !fires(src, D, "W213"),
        "FP-RBS-11: qualified ::for must NOT fire W213; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_11_genuine_unset_in_body_still_fires() {
    // TP control: $missing (never bound anywhere) must still fire W210.
    let src = "\
proc f {dict} {
    ::foreach {k v} $dict { puts \"$k=$missing\" }
}
";
    assert!(
        fires(src, D, "W210"),
        "FP-RBS-11 TP: $missing (never bound) must still fire W210; emitted: {:?}",
        codes(src, D)
    );
}

// FP-RBS-16 — phi operand on a dead loop-exit edge (`while 1` + `break`)

const FP_RBS_16_REPRO: &str = "\
proc f {} {
    # while 1 runs the body >=1 time; the only exit is the break, where y is
    # already set -> $y is always defined here.
    while 1 { set y 1; break }
    puts $y
}
";

#[test]
fn fp_rbs_16_while1_break_set_silent() {
    // FP-RBS-16: `while 1` dead cond-exit edge; y is set before every break.
    assert!(
        !fires(FP_RBS_16_REPRO, D, "W210"),
        "FP-RBS-16: dead cond-exit edge operand must NOT fire W210; emitted: {:?}",
        codes(FP_RBS_16_REPRO, D)
    );
}

#[test]
fn fp_rbs_16_while1_conditional_break_silent() {
    // FP: `while 1 { set result [compute]; if {[ok $result]} break }`.
    let src = "proc f {} {\n    while 1 {\n        set result [compute]\n        if {[ok $result]} break\n    }\n    return $result\n}\n";
    assert!(
        !fires(src, D, "W210"),
        "FP-RBS-16: result is set before every conditional break; must NOT fire W210; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_16_normal_while_body_defined_silent() {
    // FP-RBS-19: a non-constant `while` may run zero times, but its body
    // unconditionally sets `y`, so a read after the loop is defined whenever the
    // loop ran. Matching C Tcl (which errors only when the condition is false on
    // entry at runtime, not merely when it could be), we assume a may-run loop
    // runs — the after-loop read is not read-before-set.
    let src = "proc f {n} {\n    while {$n > 0} { set y 1; incr n -1 }\n    puts $y\n}\n";
    assert!(
        !fires(src, D, "W210"),
        "FP-RBS-19: may-run while whose body defines 'y' must NOT fire W210 after the loop; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_16_partial_break_def_still_fires() {
    // TP control: only ONE of two break paths sets y; live exit edge carries unset origin.
    let src = "proc f {c} {\n    while 1 {\n        if {$c} { set y 1; break }\n        if {!$c} break\n    }\n    puts $y\n}\n";
    assert!(
        fires(src, D, "W210"),
        "FP-RBS-16 TP: a break path that leaves 'y' unset must fire W210; emitted: {:?}",
        codes(src, D)
    );
}

// FP-RBS-17 — foreach over a non-empty literal runs its body at least once

const FP_RBS_17_REPRO: &str = "\
proc f {} {
    # the list is a non-empty literal, so the body runs >=1 time and y is set
    # before puts reads it.
    foreach x {1 2 3} { set y $x }
    puts $y
}
";

#[test]
fn fp_rbs_17_foreach_literal_silent() {
    // FP-RBS-17: foreach over non-empty literal list always runs; y is defined after loop.
    assert!(
        !fires(FP_RBS_17_REPRO, D, "W210"),
        "FP-RBS-17: non-empty-literal foreach defines 'y'; must NOT fire W210; emitted: {:?}",
        codes(FP_RBS_17_REPRO, D)
    );
}

#[test]
fn fp_rbs_17_loop_var_after_foreach_silent() {
    // FP: loop variable is set on every iteration; defined after non-empty-literal foreach.
    let src = "proc f {} { foreach x {a b c} { } ; puts $x }\n";
    assert!(
        !fires(src, D, "W210"),
        "FP-RBS-17: loop var defined after non-empty foreach; must NOT fire W210; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_17_empty_literal_still_fires() {
    // TP control: foreach over an empty literal never runs; y is unset.
    let src = "proc f {} { foreach x {} { set y $x } ; puts $y }\n";
    assert!(
        fires(src, D, "W210"),
        "FP-RBS-17 TP: empty-list foreach leaves 'y' unset; W210 must fire; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_17_dynamic_list_requires_an_initial_value() {
    // C Tcl 8.4–9.1 and Jim all fail this read for an empty list. The body
    // write must not erase the zero-iteration alternative.
    let src = "proc f {items} { foreach x $items { set y $x } ; puts $y }\n";
    assert!(
        fires(src, D, "W210"),
        "the possibly empty list leaves 'y' undefined; emitted: {:?}",
        codes(src, D)
    );
    let initialised = "proc f {items} {set y {}; foreach x $items {set y $x}; puts $y}";
    assert!(!fires(initialised, D, "W210"));
}

#[test]
fn fp_rbs_17_continue_before_set_still_fires() {
    // TP control: continue before set can skip the def on the last iteration.
    let src = "proc f {} { foreach x {1} { if {$x==1} continue; set y $x } ; puts $y }\n";
    assert!(
        fires(src, D, "W210"),
        "FP-RBS-17 TP: continue-before-set may skip 'y'; W210 must fire; emitted: {:?}",
        codes(src, D)
    );
}

// FP-RBS-18 — for whose condition is true on entry runs its body at least once

const FP_RBS_18_REPRO: &str = "\
proc f {} {
    # 0 < 3 is true on entry, so the body runs >=1 time and y is set.
    for {set i 0} {$i < 3} {incr i} { set y $i }
    puts $y
}
";

#[test]
fn fp_rbs_18_for_true_on_entry_silent() {
    // FP-RBS-18: for whose condition is true on entry runs body at least once; y is defined.
    assert!(
        !fires(FP_RBS_18_REPRO, D, "W210"),
        "FP-RBS-18: for true-on-entry defines 'y'; must NOT fire W210; emitted: {:?}",
        codes(FP_RBS_18_REPRO, D)
    );
}

#[test]
fn fp_rbs_18_false_on_entry_still_fires() {
    // TP control: for whose condition is false on entry runs zero times; y is unset.
    let src = "proc f {} { for {set i 5} {$i < 3} {incr i} { set y $i } ; puts $y }\n";
    assert!(
        fires(src, D, "W210"),
        "FP-RBS-18 TP: false-on-entry for runs zero times; W210 must fire; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_18_unknown_bound_body_defined_silent() {
    // FP-RBS-19: a `for` with an unknown bound may run zero times, but
    // its body unconditionally sets `y`, so a read after the loop is defined
    // whenever the loop ran. Matching C Tcl, we assume a may-run loop runs.
    let src = "proc f {n} { for {set i 0} {$i < $n} {incr i} { set y $i } ; puts $y }\n";
    assert!(
        !fires(src, D, "W210"),
        "FP-RBS-19: unknown-bound for whose body defines 'y' must NOT fire W210 after the loop; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_18_break_before_set_still_fires() {
    // TP control: break before def exits with y unset.
    let src =
        "proc f {} { for {set i 0} {$i < 3} {incr i} { if {$i==0} break; set y $i } ; puts $y }\n";
    assert!(
        fires(src, D, "W210"),
        "FP-RBS-18 TP: break-before-set exits with 'y' unset; W210 must fire; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_18_stale_const_init_body_defined_silent() {
    // A stale const from the for-init (`set i $n` overwrites `set i 0`) leaves
    // the loop may-run (not guaranteed-nonempty; the rotation-soundness of that
    // classification is covered by the CFG-builder tests). Under FP-RBS-19
    // the after-loop read is still silent: the body unconditionally sets
    // `y`, so a may-run loop is assumed to run, matching C Tcl.
    let src = "proc f {n} { for {set i 0; set i $n} {$i < 3} {incr i} { set y $i }; puts $y }\n";
    assert!(
        !fires(src, D, "W210"),
        "FP-RBS-19: may-run for whose body defines 'y' must NOT fire W210 after the loop; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_18_incr_in_init_provably_empty_fires() {
    // TP: `for {set i 0; incr i 5} {$i < 3} …` resolves the loop var to `i = 5`,
    // so `5 < 3` is false — SCCP proves the body dead (a *provably* zero-trip
    // loop that always errors in tclsh), and W210 still fires. The incr also
    // invalidates the rotation constant (see the CFG-shape test
    // `cfg::for_rotation_requires_a_non_stale_constant_init`); unlike a dynamic
    // may-run bound (FP-RBS-19), this one is statically empty, not merely maybe.
    let src = "proc f {} { for {set i 0; incr i 5} {$i < 3} {incr i} { set y $i }; puts $y }\n";
    assert!(
        fires(src, D, "W210"),
        "provably-empty incr-in-init for leaves y unset; W210 must fire; emitted: {:?}",
        codes(src, D)
    );
}

// FP-RBS-19 — a may-run loop whose body defines a variable is assumed
// to run: an after-loop read is not read-before-set

// The reported pattern: a multi-group `foreach` over
// dynamic (`dict get`) lists building `lappend` accumulators, read after the
// loop. tclsh errors only when the iterator lists are actually empty at
// runtime; on real data the accumulators are defined, so the after-loop reads
// are not read-before-set.
const FP_RBS_19_REPRO: &str = "\
set data [getDataDict]
foreach time [dict get $data time] vout [dict get $data osc_out] {
    lappend timeVout [list $time $vout]
    lappend timeImeas [list $time]
}
puts $timeVout
puts $timeImeas
";

#[test]
fn fp_rbs_19_reporter_foreach_accumulator_silent() {
    assert!(
        !fires(FP_RBS_19_REPRO, D, "W210"),
        "FP-RBS-19: dynamic-foreach `lappend` accumulator read after the loop must NOT fire W210; emitted: {:?}",
        codes(FP_RBS_19_REPRO, D)
    );
}

#[test]
fn fp_rbs_19_provably_empty_literal_still_fires() {
    // TP control: a *provably* empty literal list runs zero times, so tclsh
    // always errors — the after-loop read must keep firing.
    let src = "foreach x {} { lappend acc $x }\nputs $acc\n";
    assert!(
        fires(src, D, "W210"),
        "FP-RBS-19 TP: provably-empty foreach leaves 'acc' unset; W210 must fire; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_19_conditional_def_in_body_still_fires() {
    // TP control: the body defines `y` only under an inner `if`, so even when
    // the loop runs `y` may be unset — a genuine read-before-set that fires.
    let src = "proc f {items} { foreach x $items { if {$x > 0} { set y $x } }\n puts $y }\n";
    assert!(
        fires(src, D, "W210"),
        "FP-RBS-19 TP: conditionally-defined body var may be unset after the loop; W210 must fire; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_19_first_iteration_in_body_read_still_fires() {
    // TP control: a read *inside* the loop body before the body's own set is a
    // genuine first-iteration read-before-set — the suppression is confined to
    // reads reached after the loop.
    let src = "proc f {items} { foreach x $items { puts $y; set y $x } }\n";
    assert!(
        fires(src, D, "W210"),
        "FP-RBS-19 TP: in-body read before the set must still fire; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_19_nested_loop_accumulator_silent() {
    // Nested accumulator (`foreach r $rows { foreach c $cols { lappend cells … } }`):
    // the loop-entry-only-undef analysis converges over nesting, so the inner
    // loop's defined-on-exit result makes the outer loop entry-only too — the
    // after-loop read is silent.
    let src = "proc f {rows cols} { foreach r $rows { foreach c $cols { lappend cells [list $r $c] } }\n return $cells }\n";
    assert!(
        !fires(src, D, "W210"),
        "FP-RBS-19: nested-loop accumulator read after the loops must NOT fire W210; emitted: {:?}",
        codes(src, D)
    );
    // TP control: the inner body defines `y` only conditionally, so even when
    // both loops run `y` may be unset after them — still fires.
    let tp = "proc f {rows cols} { foreach r $rows { foreach c $cols { if {$c} { set y 1 } } }\n puts $y }\n";
    assert!(
        fires(tp, D, "W210"),
        "FP-RBS-19 TP: conditionally-defined var in a nested loop may be unset; W210 must fire; emitted: {:?}",
        codes(tp, D)
    );
}

#[test]
fn fp_rbs_19_while_and_for_may_run_silent() {
    // A dynamic `while` and an unknown-bound `for` are both may-run loops whose
    // bodies define the variable — silent after the loop, like the foreach case.
    for src in [
        "proc f {n} { while {$n > 0} { set y $n; incr n -1 }\n puts $y }\n",
        "proc f {n} { for {set i 0} {$i < $n} {incr i} { set y $i }\n puts $y }\n",
    ] {
        assert!(
            !fires(src, D, "W210"),
            "FP-RBS-19: may-run loop whose body defines 'y' must be silent after the loop; emitted: {:?}",
            codes(src, D)
        );
    }
}

// FP-RBS-21 — a barriered structured loop still binds its literal loop
// variables
//
// A structured loop whose words carry a `{*}` expansion cannot be lowered to
// an `IRForeach` — the argv shape is unknowable — so it stays a
// `Statement::Barrier`.  Its body still runs in the caller's frame and is
// scanned for reads, so without the barrier contributing its
// `ArgRole::LoopVarList` names as defs, every read of a loop variable inside
// that body would read as read-before-set.  `lmap` / `dict for` / `array for`
// / `foreachLine` must not paper over this by fabricating an `IRForeach` over
// the *un-expanded* words — that is the wrong IR.
//
// Oracle (tclsh 9.0.4, identical in 8.6.16 — `{*}` exists from 8.5):
//   set spec {{a b}}
//   foreach i {*}$spec {puts $i}          → a
//                                           b
//   lmap i {*}$spec {string toupper $i}   → A B

#[test]
fn fp_rbs_21_expanded_loops_bind_their_literal_loop_variables() {
    for src in [
        "set spec {{a b}}\nforeach i {*}$spec {puts $i}\n",
        "set spec {{a b}}\nlmap i {*}$spec {puts $i}\n",
        "set spec {{a 1}}\ndict for {k v} {*}$spec {puts $k$v}\n",
    ] {
        assert!(
            !fires(src, D, "W210"),
            "FP-RBS-21: a barriered loop still binds its literal loop vars; \
             src {src:?} emitted: {:?}",
            codes(src, D)
        );
    }
}

#[test]
fn fp_rbs_21_tp_a_genuinely_unset_read_in_an_expanded_loop_body_still_fires() {
    // TP control: binding the loop variable must not silence an unrelated
    // undefined name in the same body.
    let src = "set spec {{a b}}\nlmap i {*}$spec {puts $nope}\n";
    assert!(
        fires(src, D, "W210"),
        "FP-RBS-21 TP: an unset non-loop name still fires; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_21_tp_unexpanded_loops_are_unchanged() {
    // TP control: the ordinary (lowered) forms keep both halves — the loop
    // variable is bound, an unrelated undefined name still fires.
    let quiet = "foreach i {a b} {puts $i}\n";
    assert!(
        !fires(quiet, D, "W210"),
        "FP-RBS-21 TP: a plain foreach binds its loop var; emitted: {:?}",
        codes(quiet, D)
    );
    let loud = "foreach i {a b} {puts $nope}\n";
    assert!(
        fires(loud, D, "W210"),
        "FP-RBS-21 TP: a plain foreach still reports an unset read; emitted: {:?}",
        codes(loud, D)
    );
}

// FP-RBS-21b — a *brace-quoted* var-list word binds the names it literally
// spells, `$` and `[` included.
//
// Testing the reconstructed word text for `$` / `[` conflates "the word's
// value contains a dollar" with "the word substitutes".  A braced word
// substitutes nothing, so `{$x}` is a one-element list naming the variable
// `$x` — a legal Tcl name such a byte test would drop from the def set.
//
// Oracle (tclsh 8.6.16 and 9.0.4 both print `1`):
//   set spec {{1}}
//   foreach {{$x}} {*}$spec {puts ${$x}}   → 1

#[test]
fn fp_rbs_21b_braced_var_list_binds_a_literal_dollar_name() {
    let src = "set spec {{1}}\nforeach {{$x}} {*}$spec {puts ${$x}}\n";
    assert!(
        !fires(src, D, "W210"),
        "FP-RBS-21b: a braced var-list word binds the name it literally spells; \
         emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_21b_resolved_substituted_var_list_binds_the_original_name() {
    // Frozen original values supply the var-list and expansion before the
    // native foreach binder defines x on entry to its body.
    let src = "set spec {{1}}\nset names {x}\nforeach $names {*}$spec {puts $x}\n";
    assert!(
        !fires(src, D, "W210"),
        "FP-RBS-21b: the resolved original var-list defines x; emitted: {:?}",
        codes(src, D)
    );
}
