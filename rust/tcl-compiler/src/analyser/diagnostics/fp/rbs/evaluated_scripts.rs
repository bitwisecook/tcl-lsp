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

//! FP-RBS-07 and 10 and the `uplevel` entries: reads and writes inside scripts that `eval`, `namespace eval` and `uplevel` run.

use super::*;

// FP-RBS-07 — dynamically-named namespace eval bodies are still analysed

const FP_RBS_07_REPRO: &str = "\
# logger.tcl:1007-1016 pattern: ${service} is the enclosing proc's
# parameter; the dynamic namespace name doesn't stop the body's inner
# `proc greet` from being analysed (post-fix).
proc trace_on {service} {
    namespace eval ::logger::tree::${service} {
        proc greet {who} { return \"hello $who\" }
    }
}
";

#[test]
fn fp_rbs_07_dynamic_ns_eval_inner_param_silent() {
    // FP-RBS-07: dynamic-name namespace eval body is analysed; inner param read is silent.
    assert!(
        !fires(FP_RBS_07_REPRO, D, "W210"),
        "FP-RBS-07: dynamic-name ns-eval inner proc-param read must NOT fire W210; emitted: {:?}",
        codes(FP_RBS_07_REPRO, D)
    );
    assert!(
        !fires(FP_RBS_07_REPRO, D, "W213"),
        "FP-RBS-07: dynamic-name ns-eval inner proc-param read must NOT fire W213; emitted: {:?}",
        codes(FP_RBS_07_REPRO, D)
    );
    assert!(
        !fires(FP_RBS_07_REPRO, D, "W214"),
        "FP-RBS-07: dynamic-name ns-eval inner proc-param must NOT fire W214; emitted: {:?}",
        codes(FP_RBS_07_REPRO, D)
    );
}

#[test]
fn fp_rbs_07_static_ns_eval_still_works() {
    // FP control: static-namespace form was never broken — assert it stays silent.
    let src = "\
namespace eval ::pkg {
    proc greet {who} { return \"hello $who\" }
}
";
    assert!(
        !fires(src, D, "W210"),
        "FP-RBS-07: static ns-eval inner proc-param read must NOT fire W210; emitted: {:?}",
        codes(src, D)
    );
    assert!(
        !fires(src, D, "W214"),
        "FP-RBS-07: static ns-eval inner proc-param must NOT fire W214; emitted: {:?}",
        codes(src, D)
    );
}

// FP-RBS-10 — eval / namespace eval literal-body reads are recovered

const FP_RBS_10_REPRO: &str = "\
proc f {x} {
    # eval's braced body evaluates in *this* scope: $x is a real read of
    # the parameter, so 'x' must not be reported W214 (\"unused\").
    eval { puts $x }
}
";

#[test]
fn fp_rbs_10_eval_body_param_silent() {
    // FP-RBS-10: `eval { … $x … }` evaluates in this scope; $x is a real use of param x.
    assert!(
        !fires(FP_RBS_10_REPRO, D, "W210"),
        "FP-RBS-10: eval-body $x read must NOT fire W210; emitted: {:?}",
        codes(FP_RBS_10_REPRO, D)
    );
    assert!(
        !fires(FP_RBS_10_REPRO, D, "W213"),
        "FP-RBS-10: eval-body $x read must NOT fire W213; emitted: {:?}",
        codes(FP_RBS_10_REPRO, D)
    );
    assert!(
        !fires(FP_RBS_10_REPRO, D, "W214"),
        "FP-RBS-10: eval-body $x param must NOT fire W214; emitted: {:?}",
        codes(FP_RBS_10_REPRO, D)
    );
}

#[test]
fn fp_rbs_10_genuine_unused_still_fires() {
    // TP control: a parameter that's truly never read still fires W214.
    let src = "\
proc f {x y} {
    return $x
}
";
    assert!(
        fires(src, D, "W214"),
        "FP-RBS-10 TP: truly unused param 'y' must fire W214; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_10_namespace_eval_does_not_recover_reads() {
    // TP: `namespace eval ns { ... $x ... }` runs in ns frame, NOT caller's; W214 fires.
    let src = "\
proc g {x} {
    namespace eval ::ns { puts \"hello $x\" }
}
";
    assert!(
        fires(src, D, "W214"),
        "FP-RBS-10 TP: namespace-eval body must NOT recover caller reads; W214 must fire on x; emitted: {:?}",
        codes(src, D)
    );
}

// A variable assigned indirectly in an outer frame by `uplevel`.
//
// Every claim below is pinned on tclsh 9.0.4 and 8.6.16, byte-identical.
// The three scripts print `99`, so the reads are not read-before-set:
//
//   proc setInCaller {var} { uplevel 1 [list set $var 99] }
//   proc setInCallerPlain {var} { uplevel 1 set $var 99 }
//   proc setUp2 {var} { uplevel 2 [list set $var 99] }
//
// driven respectively by `proc useIt {} {setInCaller answer; return $answer}`,
// the same with `setInCallerPlain`, and by the two-frame chain
// `proc middle {} {setUp2 answer}` / `proc outer {} {middle; return $answer}`.

/// FP — the one-hop list-built spelling, pinned here because nothing else
/// locks it in.
#[test]
fn idx24_uplevel_one_list_built_set_is_not_read_before_set() {
    let src = "\
proc setInCaller {var} {
    uplevel 1 [list set $var 99]
}
proc useIt {} {
    setInCaller answer
    return $answer
}
";
    assert!(
        !fires(src, D, "W210"),
        "idx 24: `uplevel 1 [list set $var …]` defines the caller's variable; emitted: {:?}",
        codes(src, D)
    );
}

/// FP — the one-hop word-concatenated spelling (`uplevel 1 set $var 99`),
/// which `uplevel` joins into the same script.
#[test]
fn idx24_uplevel_one_concatenated_set_is_not_read_before_set() {
    let src = "\
proc setInCaller {var} {
    uplevel 1 set $var 99
}
proc useIt {} {
    setInCaller answer
    return $answer
}
";
    assert!(
        !fires(src, D, "W210"),
        "idx 24: `uplevel 1 set $var …` defines the caller's variable; emitted: {:?}",
        codes(src, D)
    );
}

/// FP — the multi-frame shape: the writing proc is reached through an
/// *ordinary* call, and its `uplevel 2` lands in that caller's caller.
#[test]
fn idx24_uplevel_two_reaches_through_a_plain_call() {
    let src = "\
proc setUp2 {var} {
    uplevel 2 [list set $var 99]
}
proc middle {} {
    setUp2 answer
}
proc outer {} {
    middle
    return $answer
}
";
    assert!(
        !fires(src, D, "W210"),
        "idx 24: `uplevel 2` writes the plain caller's caller; emitted: {:?}",
        codes(src, D)
    );
}

/// FP — the same, with a literal braced body instead of a built list.  An
/// `UpFrame` arm that handled only level 1 would drop this.
#[test]
fn idx24_uplevel_two_literal_body_reaches_through_a_plain_call() {
    let src = "\
proc setUp2 {} {
    uplevel 2 {set answer 99}
}
proc middle {} {
    setUp2
}
proc outer {} {
    middle
    return $answer
}
";
    assert!(
        !fires(src, D, "W210"),
        "idx 24: a literal `uplevel 2` body writes the plain caller's caller; emitted: {:?}",
        codes(src, D)
    );
}

/// FP — a level word nothing static can place (`uplevel [expr {[info level] -
/// $lvl}] …`, the real corpus spelling) is the same unknown-frame case.
#[test]
fn idx24_uplevel_with_a_computed_level_is_not_read_before_set() {
    let src = "\
proc setInCaller {var lvl} {
    uplevel [expr {[info level] - $lvl}] [list set $var 99]
}
proc useIt {} {
    setInCaller answer 1
    return $answer
}
";
    assert!(
        !fires(src, D, "W210"),
        "idx 24: a computed uplevel level cannot be placed, so widen; emitted: {:?}",
        codes(src, D)
    );
}

/// TP control — with the `uplevel` gone, nothing assigns the caller's
/// variable and W210 must still fire. Without this the FP arms above would
/// pass just as well against a W210 that had been switched off.
#[test]
fn idx24_tp_without_the_uplevel_the_read_is_still_unset() {
    let src = "\
proc setInCaller {var} {
    set $var 99
}
proc useIt {} {
    setInCaller answer
    return $answer
}
";
    assert!(
        fires(src, D, "W210"),
        "idx 24 TP: a callee-local `set` never reaches the caller; emitted: {:?}",
        codes(src, D)
    );
}

/// TP control — a **level-1** effect is not transitive through a plain call.
/// tclsh 9.0.4 / 8.6.16: `real_worker`'s `upvar 1` reaches `wrapper`'s frame,
/// so the outermost caller really does get `can't read "a": no such
/// variable`. This is the guard rail that keeps the new one-hop composition
/// from degenerating into "any caller of any frame-crossing proc is silent".
#[test]
fn idx24_tp_level_one_does_not_propagate_through_a_plain_call() {
    let src = "\
proc worker {v} {
    upvar 1 $v x
    set x 1
}
proc wrapper {v} {
    worker $v
}
proc caller {} {
    wrapper a
    return $a
}
";
    assert!(
        fires(src, D, "W210"),
        "idx 24 TP: a level-1 upvar stops at the wrapper's own frame; emitted: {:?}",
        codes(src, D)
    );
}

/// TP control — `uplevel #0` is the global frame and `uplevel 0` the callee's
/// own, so neither reaches a caller and neither may silence the read.
#[test]
fn idx24_tp_global_and_own_frame_levels_do_not_silence_the_caller() {
    for level in ["#0", "0"] {
        let src = format!(
            "proc setSomewhere {{var}} {{\n    uplevel {level} [list set $var 99]\n}}\n\
             proc useIt {{}} {{\n    setSomewhere answer\n    return $answer\n}}\n"
        );
        assert!(
            fires(&src, D, "W210"),
            "idx 24 TP: `uplevel {level}` never touches the caller's frame; emitted: {:?}",
            codes(&src, D)
        );
    }
}

/// FP — W212 must not read the literal word `set` inside a `[list …]`-built
/// script as a directly-written `set $var` name/value confusion. The
/// substitution happens in the *building* frame and is the whole idiom.
#[test]
fn idx24_w212_does_not_fire_on_a_list_built_variable_name_word() {
    let src = "proc setInCaller {var} {\n    uplevel 1 [list set $var 99]\n}\n";
    assert!(
        !fires(src, D, "W212"),
        "idx 24: a list-built name word is pre-substituted, not a confusion; emitted: {:?}",
        codes(src, D)
    );
}

/// TP control — the directly-written spelling is exactly what W212 is for,
/// and still fires.
#[test]
fn idx24_tp_w212_still_fires_on_a_directly_written_name_word() {
    for src in [
        "proc f {var} { set $var 1 }\n",
        "proc f {} { set counter 1\n incr $countr }\n",
    ] {
        assert!(
            fires(src, D, "W212"),
            "idx 24 TP: a written `$` in a name position is still W212; {src} emitted: {:?}",
            codes(src, D)
        );
    }
}

/// TN guard rail (the braced-use family) — widening a caller's frame must not
/// cost a variable its liveness: a store the `uplevel`-written name later
/// feeds must not resurface as a dead store (W220) or an unused variable
/// (W211).
#[test]
fn idx24_widening_keeps_the_callers_own_stores_live() {
    let src = "\
proc setUp2 {var} {
    uplevel 2 [list set $var 99]
}
proc middle {} {
    setUp2 answer
}
proc outer {} {
    set answer 0
    middle
    return $answer
}
";
    assert!(
        !fires(src, D, "W220") && !fires(src, D, "W211"),
        "idx 24: the seeded store is read after the widened call; emitted: {:?}",
        codes(src, D)
    );
}

/// FP — the same W212 carve-out on the canonical caller-frame-injection
/// idiom the `tclopt` corpus is built from: a
/// `[list set $varName $value]` built inside a `foreach`, where **both**
/// words are substitutions.
///
/// Oracle (tclsh 9.0.4 and 8.6.16): the script below prints `1 1 1`, so all
/// three names really are materialised in `Qfrac2`'s frame.
#[test]
fn idx38_w212_does_not_fire_on_a_list_built_set_of_a_looped_name() {
    let src = "\
proc NewArrays {varNames value} {
    foreach varName $varNames {
        uplevel 1 [list set $varName $value]
    }
}
proc Qfrac2 {} {
    NewArrays {rdiagArray acnormArray waArray} 1
    return [list $rdiagArray $acnormArray $waArray]
}
";
    assert!(
        !fires(src, D, "W212"),
        "idx 38: `[list set $varName $value]` is the idiom, not a confusion; emitted: {:?}",
        codes(src, D)
    );
    assert!(
        !fires(src, D, "W210"),
        "idx 38: the three names are materialised in the caller's frame; emitted: {:?}",
        codes(src, D)
    );
}

/// TP control for the shape above — with the `NewArrays` call removed
/// nothing materialises the names and all three reads are genuinely unset
/// (tclsh: `can't read "rdiagArray": no such variable`).
#[test]
fn idx38_tp_without_the_injection_call_the_reads_are_unset() {
    let src = "\
proc Qfrac2 {} {
    return [list $rdiagArray $acnormArray $waArray]
}
";
    assert!(
        fires(src, D, "W210"),
        "idx 38 TP: nothing writes those names; emitted: {:?}",
        codes(src, D)
    );
}

/// TP control for the W212 carve-out itself, in the *same* proc shape: a
/// `set $varName $value` written directly into the loop body — no `list`
/// builder, no deferred script — is exactly the name/value confusion the
/// code exists for and must still fire.
#[test]
fn idx38_tp_w212_still_fires_on_a_directly_written_looped_name() {
    let src = "\
proc NewArrays {varNames value} {
    foreach varName $varNames {
        set $varName $value
    }
}
";
    assert!(
        fires(src, D, "W212"),
        "idx 38 TP: a written `$` in a name position is still W212; emitted: {:?}",
        codes(src, D)
    );
}
