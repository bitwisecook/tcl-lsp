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

//! FP-RBS-09, 13, 14 and 15: un-lowered `switch` arms, and arms and `tailcall`s that cannot complete normally.

use super::*;

// FP-RBS-09 — for-init / regexp captures in un-lowered switch arms

const FP_RBS_09_REPRO: &str = r#"
proc f {n} {
    switch -- $n {
        a {
            for {set j 0} {$j < 3} {incr j} { puts $j }
        }
        b {
            if {[regexp {(\w+)} "foo" -> v]} { puts $v }
        }
    }
}
"#;

#[test]
fn fp_rbs_09_for_init_in_switch_arm_silent() {
    // FP-RBS-09: `for {set j 0} …` inside a switch arm — for-init def is recovered.
    assert!(
        !fires(FP_RBS_09_REPRO, D, "W210"),
        "FP-RBS-09: for-init $j read inside switch arm must NOT fire W210; emitted: {:?}",
        codes(FP_RBS_09_REPRO, D)
    );
    assert!(
        !fires(FP_RBS_09_REPRO, D, "W213"),
        "FP-RBS-09: for-init $j read inside switch arm must NOT fire W213; emitted: {:?}",
        codes(FP_RBS_09_REPRO, D)
    );
}

#[test]
fn fp_rbs_09_regexp_capture_in_switch_arm_silent() {
    // FP-RBS-09: `regexp … -> v` capture inside a switch arm — capture-var def is recovered.
    // Note: tested via the same reproducer; guard on 'v' specifically.
    // Fires are already covered above (W210 must not fire for j).
    // Here we check the 'v' path (which is about regexp capture, not for-init).
    // Since both are in the same reproducer, the W210-silent check covers both.
    assert!(
        !fires(FP_RBS_09_REPRO, D, "W210"),
        "FP-RBS-09: regexp capture $v read inside switch arm must NOT fire W210; emitted: {:?}",
        codes(FP_RBS_09_REPRO, D)
    );
}

#[test]
fn fp_rbs_09_genuine_unset_in_arm_still_fires() {
    // TP control: $missing (never written anywhere) still fires W210 even inside a switch arm.
    let src = r"
proc f {n} {
    switch -- $n {
        a {
            for {set j 0} {$j < 3} {incr j} { puts $missing }
        }
    }
}
";
    assert!(
        fires(src, D, "W210"),
        "FP-RBS-09 TP: $missing (never written anywhere) must fire W210; emitted: {:?}",
        codes(src, D)
    );
}

// FP-RBS-13 — `tailcall` replaces the frame; code after it never runs

const FP_RBS_13_REPRO: &str = "\
proc f {cond} {
    # tailcall g replaces this frame: the `return $result` below is only
    # reached via the else branch, where result is always set.
    if {$cond} {
        tailcall g
    } else {
        set result 1
    }
    return $result
}
";

#[test]
fn fp_rbs_13_tailcall_with_args_silent() {
    // FP-RBS-13: `tailcall g` ends the proc's straight-line flow; return $result is safe.
    assert!(
        !fires(FP_RBS_13_REPRO, D, "W210"),
        "FP-RBS-13: tailcall-terminated branch must NOT fire W210; emitted: {:?}",
        codes(FP_RBS_13_REPRO, D)
    );
    assert!(
        !fires(FP_RBS_13_REPRO, D, "W213"),
        "FP-RBS-13: tailcall-terminated branch must NOT fire W213; emitted: {:?}",
        codes(FP_RBS_13_REPRO, D)
    );
}

#[test]
fn fp_rbs_13_bare_tailcall_silent() {
    // FP: bare `tailcall` (no args) is also a terminator.
    let src = "proc f {cond} {\n    if {$cond} { tailcall } else { set result 1 }\n    return $result\n}\n";
    assert!(
        !fires(src, D, "W210"),
        "FP-RBS-13: bare tailcall is a terminator; must NOT fire W210; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_13_non_terminating_branch_still_fires() {
    // TP control: replace tailcall with a normal command; result is maybe-unset.
    let src = "proc f {cond} {\n    if {$cond} { puts hi } else { set result 1 }\n    return $result\n}\n";
    assert!(
        fires(src, D, "W210"),
        "FP-RBS-13 TP: non-terminating then-branch leaves 'result' maybe-unset; W210 must fire; emitted: {:?}",
        codes(src, D)
    );
}

// FP-RBS-14 — opaque-switch arm that can't complete normally is excluded from must-define

const FP_RBS_14_REPRO: &str = "\
proc f {x} {
    # the a* arm returns, so it never reaches `puts $y`; the only path that
    # does (default) sets y -> y is definitely defined.
    switch -glob $x {
        a* { return 0 }
        default { set y 2 }
    }
    puts $y
}
";

#[test]
fn fp_rbs_14_returning_arm_silent() {
    // FP-RBS-14: the `a*` arm returns; every path reaching `puts $y` assigns y.
    assert!(
        !fires(FP_RBS_14_REPRO, D, "W210"),
        "FP-RBS-14: returning switch arm must NOT drop 'y' from must-define; emitted: {:?}",
        codes(FP_RBS_14_REPRO, D)
    );
}

#[test]
fn fp_rbs_14_erroring_arm_silent() {
    // FP: an arm that always errors likewise cannot complete normally.
    let src = "proc f {x} {\n    switch -glob $x { a* { error bad } default { set y 2 } }\n    puts $y\n}\n";
    assert!(
        !fires(src, D, "W210"),
        "FP-RBS-14: erroring switch arm must NOT drop 'y' from must-define; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_14_omitting_arm_still_fires() {
    // TP control: an arm that completes without assigning y leaves it maybe-unset.
    let src = "proc f {x} {\n    switch -glob $x { a* { set z 9 } default { set y 2 } }\n    puts $y\n}\n";
    assert!(
        fires(src, D, "W210"),
        "FP-RBS-14 TP: omitting arm leaves 'y' maybe-unset; W210 must fire; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_14_break_arm_escaping_loop_still_fires() {
    // TP: break in an opaque-switch arm does NOT define the var on the path escaping the loop.
    let src =
        "proc f {} { foreach x {a} { switch -glob $x {a* {break} default {set y 1}} }; puts $y }\n";
    assert!(
        fires(src, D, "W210"),
        "FP-RBS-14 TP: break-arm escaping the loop leaves 'y' unset; W210 must fire; emitted: {:?}",
        codes(src, D)
    );
}

// FP-RBS-15 — opaque switch whose every arm exits is itself a terminator

const FP_RBS_15_REPRO: &str = "\
proc f {x} {
    # every arm returns, so control never reaches `puts $y`:
    # the switch is a terminator and the read is dead code.
    switch -glob $x {
        a* { return 1 }
        default { return 2 }
    }
    puts $y
}
";

#[test]
fn fp_rbs_15_all_arms_return_dead_read_silent() {
    // FP-RBS-15: every arm returns; `puts $y` is unreachable dead code — no W210.
    assert!(
        !fires(FP_RBS_15_REPRO, D, "W210"),
        "FP-RBS-15: all-arms-return switch makes trailing read unreachable; must NOT fire W210; emitted: {:?}",
        codes(FP_RBS_15_REPRO, D)
    );
}

#[test]
fn fp_rbs_15_all_arms_error_or_tailcall_silent() {
    // FP: error and tailcall arms are non-completing too; all-exiting switch is still a terminator.
    let src = "proc f {x} {\n    switch -glob $x {\n        a* { error a }\n        b* { tailcall g }\n        default { error d }\n    }\n    puts $y\n}\n";
    assert!(
        !fires(src, D, "W210"),
        "FP-RBS-15: all-arms-exit switch makes trailing read unreachable; must NOT fire W210; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_15_no_default_falls_through_fires() {
    // TP control: without a default an unmatched subject falls through; W210 must fire.
    let src =
        "proc f {x} {\n    switch -glob $x { a* { return 1 } b* { return 2 } }\n    puts $y\n}\n";
    assert!(
        fires(src, D, "W210"),
        "FP-RBS-15 TP: no-default switch falls through; W210 must fire; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_15_one_completing_arm_fires() {
    // TP control: one arm that completes lets the switch fall through with y unset.
    let src = "proc f {x} {\n    switch -glob $x { a* { return 1 } default { set z 9 } }\n    puts $y\n}\n";
    assert!(
        fires(src, D, "W210"),
        "FP-RBS-15 TP: completing-arm switch falls through; W210 must fire; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_15_all_break_switch_not_a_proc_terminator() {
    // TP: a switch whose arms all `break` jumps to loop exit, not proc return.
    let src = "proc f {x} { foreach i {1 2 3} { switch -glob $x {a* {break} default {break}} }; puts $y }\n";
    assert!(
        fires(src, D, "W210"),
        "FP-RBS-15 TP: all-break switch is a loop jump, not a proc return; W210 must fire; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_15_while1_break_in_opaque_switch_exits_loop() {
    // TP: while 1 whose only exit is break inside an opaque switch — the post-loop read is reachable.
    let src = "proc f {x} { while 1 { switch -glob $x {a* {break} default {break}} }; puts $y }\n";
    assert!(
        fires(src, D, "W210"),
        "FP-RBS-15 TP: break-in-opaque-switch must make while-1 exit reachable; W210 must fire; emitted: {:?}",
        codes(src, D)
    );
}

#[test]
fn fp_rbs_15_continue_in_opaque_switch_loop_silent() {
    // FP control: benign opaque-switch continue arm; var set before switch every iteration.
    let src = "proc f {} { foreach x {1 2 3} { set y 1; switch -glob $x {a* {continue} default {}} }; puts $y }\n";
    assert!(
        !fires(src, D, "W210"),
        "FP-RBS-15: var set before opaque-switch continue arm stays defined; must NOT fire W210; emitted: {:?}",
        codes(src, D)
    );
}
