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

//! W210 and W213 along straight-line and branching flow: terminating arms, loops that run at least once, `try` handlers and code the module cannot see.

use super::*;

#[test]
fn w210_missing_physical_read_does_not_require_an_ssa_value_version() {
    assert!(has_code(
        "proc p {known} {set copy $known; set copy $missing}",
        "tcl8.6",
        "W210"
    ));
    assert!(!has_code(
        "proc p {known} {set copy $known}",
        "tcl8.6",
        "W210"
    ));
    assert!(
        !has_code("proc p {} {set a(k) 1; set copy $a}", "tcl8.6", "W210"),
        "an existing array's scalar-read error is not an undefined-variable read"
    );
}

#[test]
fn emit_cfg_ssa_diagnostics_w210_read_before_set() {
    // ``proc foo {} { puts $undef }`` — undef is not a
    // parameter and not in scope; W210 fires at the use.
    let mut a = Analyser::new();
    a.emit_document_cfg_ssa_diagnostics_for_test("proc foo {} { puts $undef }");
    let w210s: Vec<_> = a
        .result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W210 && d.message.contains("'undef'"))
        .collect();
    assert!(
        !w210s.is_empty(),
        "W210 expected for read of undef ``$undef``; got {:?}",
        a.result.diagnostics,
    );
    assert_eq!(w210s[0].severity, Severity::Warning);
    assert!(w210s[0].message.contains("read before it is set"));
}

#[test]
fn w210_not_fired_for_qualified_global_read() {
    // ``$::myVar`` is an explicit global read; its
    // definition may live in another proc, namespace, or file, so it must
    // never be flagged read-before-set even when this unit never sets it.
    let mut a = Analyser::new();
    a.emit_document_cfg_ssa_diagnostics_for_test("proc foo {} { puts $::myVar }");
    assert!(
        !a.result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W210),
        "W210 must not fire on a fully-qualified global read; got {:?}",
        a.result.diagnostics,
    );
}

#[test]
fn opaque_switch_recovers_exhaustive_arm_defs() {
    // A glob/regexp/fall-through switch is kept opaque, but it still
    // definitely-defines a variable assigned on *every* path. An exhaustive
    // switch (a `default` plus every arm setting `y`) defines `y`, so the
    // following `$y` read is NOT a read-before-set — no false W210.
    let mut a = Analyser::new();
    a.emit_document_cfg_ssa_diagnostics_for_test(
        "proc f {x} { switch -glob $x {a* {set y 1} default {set y 2}}\n puts $y }",
    );
    assert!(
        !a.result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W210 && d.message.contains("'y'")),
        "exhaustive opaque switch must define y (no W210); got {:?}",
        a.result.diagnostics,
    );

    // A non-exhaustive switch (no `default`) leaves `y` only maybe-defined,
    // so the read IS a read-before-set — W210 must still fire.
    let mut b = Analyser::new();
    b.emit_document_cfg_ssa_diagnostics_for_test(
        "proc g {x} { switch -glob $x {a* {set y 1} b* {set y 2}}\n puts $y }",
    );
    assert!(
        b.result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W210 && d.message.contains("'y'")),
        "non-exhaustive opaque switch leaves y maybe-undef (W210 expected); got {:?}",
        b.result.diagnostics,
    );
}

/// Helper: does the analyser emit a W210 for a read of `var` in `src`?
#[cfg(test)]
/// Under one release: the lenient span proves no read before a set.
fn w210_fires_for(src: &str, var: &str) -> bool {
    let mut a = Analyser::new();
    a.profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    a.emit_document_cfg_ssa_diagnostics_for_test(src);
    let needle = format!("'{var}'");
    a.result
        .diagnostics
        .iter()
        .any(|d| d.code == DiagCode::W210 && d.message.contains(&needle))
}

#[test]
fn fp_rbs_13_tailcall_is_a_terminator() {
    // Bug 1 / FP-RBS-13: `tailcall` replaces the current frame and never
    // returns (TclNRTailcallObjCmd always `return TCL_RETURN`), so it ends
    // straight-line flow exactly like `return`/`error`. A var set only on
    // the *other* branch of an `if {…} { tailcall … }` is therefore always
    // set at a read after the `if` — no false W210.

    // FP: `tailcall g` (with args) — only the else branch reaches `return`.
    assert!(
        !w210_fires_for(
            "proc f {cond} { if {$cond} { tailcall g } else { set result 1 }\n return $result }",
            "result",
        ),
        "tailcall g must terminate the then-branch (no W210 on result)",
    );

    // FP: bare `tailcall` is *also* a terminator (no args guard) — the C
    // impl returns TCL_RETURN regardless of arg count.
    assert!(
        !w210_fires_for(
            "proc f {cond} { if {$cond} { tailcall } else { set result 1 }\n return $result }",
            "result",
        ),
        "bare tailcall must terminate the then-branch (no W210 on result)",
    );

    // TP control: a non-terminating then-branch (`puts hi`) leaves `result`
    // maybe-unset at the read — W210 must still fire. Proves the
    // suppression is specific to the terminator, not the if/return shape.
    assert!(
        w210_fires_for(
            "proc f {cond} { if {$cond} { puts hi } else { set result 1 }\n return $result }",
            "result",
        ),
        "non-terminating then-branch must still fire W210 on result",
    );
}

#[test]
fn fp_rbs_14_opaque_switch_excludes_non_completing_arm() {
    // Bug 2 / FP-RBS-14: an opaque switch's must-define set excludes any arm
    // that cannot complete normally (it never reaches the code after the
    // switch). The default sets `y`; the `a*` arm exits, so `y` is defined
    // on every *reaching* path.

    // FP: returning arm excluded — default defines `y`.
    assert!(
        !w210_fires_for(
            "proc f {x} { switch -glob $x { a* { return 0 } default { set y 2 } }\n puts $y }",
            "y",
        ),
        "returning arm must be excluded from must-define (no W210 on y)",
    );

    // FP: erroring arm likewise cannot complete normally.
    assert!(
        !w210_fires_for(
            "proc f {x} { switch -glob $x { a* { error bad } default { set y 2 } }\n puts $y }",
            "y",
        ),
        "erroring arm must be excluded from must-define (no W210 on y)",
    );

    // TP control: a *completing* arm that omits `y` (`set z 9`) reaches the
    // code after the switch with `y` unset — W210 must fire.
    assert!(
        w210_fires_for(
            "proc f {x} { switch -glob $x { a* { set z 9 } default { set y 2 } }\n puts $y }",
            "y",
        ),
        "completing arm omitting y must still fire W210 on y",
    );

    // TP control: a `break` arm is a LOOP_JUMP, not a proc-exit —
    // it escapes the loop *without* `y`, so `y` is NOT defined on every
    // reaching path. The arm must be kept (with its empty pre-break defs) in
    // the must-define intersection, so `y` is dropped and W210 fires.
    assert!(
        w210_fires_for(
            "proc f {} { foreach x {a} { switch -glob $x { a* { break } default { set y 1 } } }\n puts $y }",
            "y",
        ),
        "break arm escapes the loop with y unset (W210 expected on y)",
    );
}

#[test]
fn fp_rbs_15_all_exiting_opaque_switch_is_a_terminator() {
    // Bug 3 / FP-RBS-15: an opaque switch with a `default` whose *every*
    // reachable arm body cannot complete normally never falls through, so
    // the code after it is dead — no W210 on a read in that dead code.

    // FP: every arm returns — the switch is a terminator, `puts $y` is dead.
    assert!(
        !w210_fires_for(
            "proc f {x} { switch -glob $x { a* { return 1 } default { return 2 } }\n puts $y }",
            "y",
        ),
        "all-returning opaque switch must terminate (puts $y is dead, no W210)",
    );

    // FP: a mix of error / tailcall arms is also all-exiting.
    assert!(
        !w210_fires_for(
            "proc f {x} { switch -glob $x { a* { error bad } default { tailcall g } }\n puts $y }",
            "y",
        ),
        "all error/tailcall opaque switch must terminate (no W210 on y)",
    );

    // TP control: drop the `default` — an unmatched subject falls through to
    // `puts $y` with `y` unset, so the read is reachable. W210 fires.
    assert!(
        w210_fires_for(
            "proc f {x} { switch -glob $x { a* { return 1 } b* { return 2 } }\n puts $y }",
            "y",
        ),
        "default-less opaque switch falls through (W210 expected on y)",
    );

    // TP control: one arm that *completes* (`set z 9`) lets the switch fall
    // through with `y` unset. W210 fires.
    assert!(
        w210_fires_for(
            "proc f {x} { switch -glob $x { a* { return 1 } default { set z 9 } }\n puts $y }",
            "y",
        ),
        "one completing arm lets the switch fall through (W210 expected on y)",
    );

    // TP control: an all-*break* switch is NOT a proc terminator —
    // it jumps to the enclosing loop's exit, so a `while 1` whose only exit
    // is the break reaches the post-loop read with the var unset. The switch
    // must wire its break edge to the loop exit (not promote to a Return),
    // so the loop exit is reachable and W210 fires.
    assert!(
        w210_fires_for(
            "proc f {x} { while 1 { switch -glob $x { a* { break } default { break } } }\n puts $y }",
            "y",
        ),
        "all-break opaque switch in while 1 reaches the loop exit (W210 expected on y)",
    );
}

#[test]
fn fp_rbs_16_dead_loop_exit_phi_operand_not_read_before_set() {
    // FP-RBS-16: a `while 1` loop-exit phi has an operand on the never-taken
    // `cond -> exit` edge carrying the var's version-0 (unset) origin. SCCP
    // marks that edge non-executable; the read-before-set phi-undef closure
    // must skip operands on dead predecessor edges, not just dead blocks.

    // FP: `while 1 { set y 1; break }` — only live exit is the break (y set).
    assert!(
        !w210_fires_for("proc f {} { while 1 { set y 1; break }\n puts $y }", "y"),
        "while 1 set+break: y set on the only live exit edge (no W210)",
    );

    // FP: the realistic `while 1 { ...; if {c} break }` early-exit idiom.
    assert!(
        !w210_fires_for(
            "proc compute {} {return 7}\nproc ok {a} {return 1}\nproc f {} { while 1 { set r [compute]; if {[ok $r]} break }\n return $r }",
            "r",
        ),
        "while 1 compute/if-break: r set before the only live exit (no W210)",
    );

    // FP-RBS-19: a non-constant condition may run the body zero times,
    // but the body unconditionally sets y, so a read after the loop is defined
    // whenever the loop ran. Matching C Tcl, we assume a may-run loop runs.
    assert!(
        !w210_fires_for("proc f {n} { while {$n>0} { set y 1 }\n puts $y }", "y"),
        "may-run while whose body defines y is silent after the loop (no W210)",
    );

    // TP control: only one of two break paths sets y, so the exit merges a
    // genuine unset version on a live break edge (not the loop-entry edge) —
    // FP-RBS-19 does not touch this; it still fires.
    assert!(
        w210_fires_for(
            "proc f {c} { while 1 { if {$c} { set y 1; break } else break }\n puts $y }",
            "y",
        ),
        "partial-def break path leaves y maybe-unset (W210 expected on y)",
    );
}

#[test]
fn fp_rbs_17_guaranteed_foreach_defines_body_vars() {
    // FP-RBS-17: a foreach over a non-empty *literal* list provably iterates
    // ≥1 time, so a body-assigned variable (or the loop variable) read after
    // the loop is defined. Analysis rotates the loop so the 0-iteration skip
    // is a dead entry-guard edge (FP-RBS-16 ignores its phi operand).

    // FP: body assigns y; the literal list guarantees ≥1 iteration.
    assert!(
        !w210_fires_for(
            "proc f {} { foreach x {1 2 3} { set y $x }\n puts $y }",
            "y"
        ),
        "foreach over a non-empty literal defines y (no W210)",
    );

    // FP: the loop variable itself is defined after a guaranteed foreach.
    assert!(
        !w210_fires_for("proc f {} { foreach x {a b c} {}\n puts $x }", "x"),
        "loop variable is defined after a guaranteed foreach (no W210)",
    );

    // TP control: an empty literal list runs zero times — y stays unset.
    assert!(
        w210_fires_for("proc f {} { foreach x {} { set y $x }\n puts $y }", "y"),
        "empty foreach list runs zero times (W210 expected on y)",
    );

    // FP-RBS-19: a dynamic (`$i`) list may be empty, but the body
    // unconditionally sets y, so a read after the loop is defined whenever the
    // loop ran. Matching C Tcl, we assume a may-run loop runs.
    assert!(
        !w210_fires_for("proc f {i} { foreach x $i { set y $x }\n puts $y }", "y"),
        "may-run dynamic foreach whose body defines y is silent after the loop (no W210)",
    );

    // TP control: a first-iteration read-before-set inside the body still
    // fires — the suppression is confined to reads *after* the loop.
    assert!(
        w210_fires_for(
            "proc f {} { foreach x {1 2 3} { puts $acc; set acc $x } }",
            "acc"
        ),
        "first-iteration read of acc before its set still fires (W210)",
    );
}

#[test]
fn fp_rbs_18_guaranteed_for_defines_body_vars() {
    // FP-RBS-18: a `for` whose condition is statically true on
    // entry (evaluated against the init clause's *constant* bindings)
    // provably iterates ≥1 time. Init is processed in order, and a
    // non-constant write invalidates a stale constant binding.

    // FP: `for {set i 0} {$i<3} …` — 0 < 3 true on entry, body sets y.
    assert!(
        !w210_fires_for(
            "proc f {} { for {set i 0} {$i<3} {incr i} { set y $i }\n puts $y }",
            "y",
        ),
        "for with statically-true entry condition defines y (no W210)",
    );

    // TP control: `for {set i 5} {$i<3} …` — 5 < 3 false, zero iterations.
    assert!(
        w210_fires_for(
            "proc f {} { for {set i 5} {$i<3} {incr i} { set y $i }\n puts $y }",
            "y",
        ),
        "for with false entry condition runs zero times (W210 expected on y)",
    );

    // FP-RBS-19: a stale-constant init (`set i $n` overwrites `set i 0`)
    // leaves the loop may-run — but its body unconditionally sets y, so a read
    // after the loop is silent (we assume a may-run loop runs). The rotation
    // decision itself (that a stale-const init is NOT claimed guaranteed) is
    // pinned directly on the CFG shape in
    // `cfg::for_rotation_requires_a_non_stale_constant_init`.
    assert!(
        !w210_fires_for(
            "proc f {n} { for {set i 0; set i $n} {$i<3} {incr i} { set y $i }\n puts $y }",
            "y",
        ),
        "may-run for whose body defines y is silent after the loop (no W210)",
    );

    // TP control: an `incr` in the init resolves the loop var to `i = 5`, so
    // SCCP folds `5 < 3` to false and the body is provably dead — a genuine
    // zero-iteration loop that leaves y unset. Still fires.
    assert!(
        w210_fires_for(
            "proc f {} { for {set i 0; incr i 5} {$i<3} {incr i} { set y $i }\n puts $y }",
            "y",
        ),
        "provably-empty for (incr init makes 5<3 false) leaves y unset (W210 on y)",
    );

    // FP-RBS-19: an init call writing the loop var through `upvar` leaves
    // the loop may-run (SCCP cannot see through the call), so the after-loop
    // read of the body-defined y is silent. The invalidation itself (the loop
    // is NOT claimed guaranteed) is pinned on the CFG shape in
    // `cfg::for_rotation_requires_a_non_stale_constant_init`.
    assert!(
        !w210_fires_for(
            "proc setter {} { upvar 1 i i; set i 5 }\nproc f {} { for {set i 0; setter} {$i < 3} {incr i} { set y $i }\n puts $y }",
            "y",
        ),
        "may-run for (upvar-writing init call) whose body defines y is silent after the loop (no W210)",
    );

    // FP guard: a *benign* call in the init (one that does not write the
    // loop var) must NOT invalidate the constant — the loop stays guaranteed
    // and `y` is defined, so no false W210 is introduced by the upvar fix.
    assert!(
        !w210_fires_for(
            "proc f {} { for {set i 0; puts hi} {$i < 3} {incr i} { set y $i }\n puts $y }",
            "y",
        ),
        "benign init call must not invalidate the constant (no W210 on y)",
    );
}

#[test]
fn fp_rbs_15_continue_in_opaque_switch_stays_silent() {
    // Companion to fp_rbs_15: a benign `continue` arm inside an
    // opaque switch in a guaranteed foreach stays silent when the variable
    // is set before the switch on every iteration.
    assert!(
        !w210_fires_for(
            "proc f {} { foreach x {1 2 3} { set y 1; switch -glob $x { a* { continue } default {} } }\n puts $y }",
            "y",
        ),
        "continue-arm switch with y set before it every iteration (no W210)",
    );
}

#[test]
fn emit_cfg_ssa_diagnostics_w210_skipped_for_lappend_autocreate() {
    // `lappend` / `append` auto-create their target, so a first use is
    // not a read-before-set (excluding RMW targets).
    for src in [
        "proc foo {} { lappend items a\nputs $items }",
        "proc foo {} { append buf x\nputs $buf }",
    ] {
        let mut a = Analyser::new();
        a.profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        a.emit_document_cfg_ssa_diagnostics_for_test(src);
        assert!(
            !a.result
                .diagnostics
                .iter()
                .any(|d| d.code == DiagCode::W210),
            "W210 must not fire for auto-creating RMW; got {:?} for {src:?}",
            a.result.diagnostics,
        );
    }
    // `unset` (without -nocomplain) is destructive, not auto-creating —
    // its missing-variable case must still raise W213.
    let mut a = Analyser::new();
    a.profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    a.emit_document_cfg_ssa_diagnostics_for_test("proc foo {} { unset gone }");
    assert!(
        a.result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W213),
        "W213 must still fire for unset of a possibly-undef var; got {:?}",
        a.result.diagnostics,
    );
}

#[test]
fn emit_cfg_ssa_diagnostics_w210_skipped_for_real_param() {
    // ``proc foo {x} { puts $x }`` — x IS a real parameter,
    // so W210 must not fire.
    let mut a = Analyser::new();
    a.emit_document_cfg_ssa_diagnostics_for_test("proc foo {x} { puts $x }");
    let w210s: Vec<_> = a
        .result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W210 && d.message.contains("'x'"))
        .collect();
    assert_eq!(
        w210s.len(),
        0,
        "W210 must not fire on real param ``x``; got {:?}",
        a.result.diagnostics,
    );
}

#[test]
fn emit_cfg_ssa_diagnostics_w213_unset_on_possibly_undef() {
    // ``proc foo {} { unset xs }`` — ``xs`` may not exist;
    // ``unset`` without ``-nocomplain`` would error at
    // runtime.  W213 fires (instead of W210) at the unset
    // statement.
    let mut a = Analyser::new();
    a.profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    a.emit_document_cfg_ssa_diagnostics_for_test("proc foo {} { unset xs }");
    let w213s: Vec<_> = a
        .result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W213)
        .collect();
    assert!(
        !w213s.is_empty(),
        "W213 expected for ``unset xs`` on possibly-undef var; got {:?}",
        a.result.diagnostics,
    );
    assert!(w213s[0].message.contains("'xs'"));
    assert!(w213s[0].message.contains("unset -nocomplain"));
    assert_eq!(w213s[0].severity, Severity::Warning);
    // The squiggle narrows to the offending variable word `xs`, not the whole
    // `unset xs` command.
    let src = "proc foo {} { unset xs }";
    let span = w213s[0].span;
    assert_eq!(&src[span.start() as usize..span.end() as usize], "xs");
    // A quick fix is attached that inserts `-nocomplain` right after `unset`.
    assert_eq!(w213s[0].fixes.len(), 1, "W213 must carry one fix");
    let fix = &w213s[0].fixes[0];
    assert_eq!(fix.new_text, " -nocomplain");
    assert_eq!(fix.span.start(), fix.span.end(), "insertion is zero-width");
    // Splicing the fix produces `unset -nocomplain xs`.
    let at = fix.span.start() as usize;
    let spliced = format!("{}{}{}", &src[..at], fix.new_text, &src[at..]);
    assert!(
        spliced.contains("unset -nocomplain xs"),
        "spliced: {spliced}"
    );
}

#[test]
fn emit_cfg_ssa_diagnostics_w210_read_after_unset() {
    // ``set a 1; unset a; puts $a`` — the `unset` kills `a`, so the
    // later `$a` read is read-before-set. W210 fires on the read line
    // (the killed real version is undef at its use, like a version-0
    // origin).
    let mut a = Analyser::new();
    a.profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    a.emit_document_cfg_ssa_diagnostics_for_test(
        "proc f {} {\n    set a 1\n    unset a\n    puts $a\n}",
    );
    assert!(
        a.result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W210 && d.message.contains("'a'")),
        "W210 expected for read after unset; got {:?}",
        a.result.diagnostics,
    );
}

#[test]
fn try_handler_edge_merges_pretry_when_body_falls_through() {
    // A body that *can* fall through normally but also contains an
    // explicit throw (`if {$c} { error e }; set y 2`) reaches the handler
    // by an abnormal completion at *any* point, so `y` is only *maybe*
    // defined — W210 must still fire on the handler's `$y` read. Sourcing
    // the on-error edge only from the explicit-throw block (after `set y 2`
    // failed to run) would wrongly suppress it.
    let mut a = Analyser::new();
    a.profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    a.emit_document_cfg_ssa_diagnostics_for_test(
            "proc f {c} {\n try {\n  if {$c} { error e }\n  set y 2\n } on error {} {\n  puts $y\n }\n}\n",
        );
    assert!(
        a.result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W210 && d.message.contains("'y'")),
        "W210 expected for handler read of maybe-undef y; got {:?}",
        a.result.diagnostics,
    );
}

#[test]
fn try_handler_edge_suppressed_when_var_set_before_sole_throw() {
    // `set x 1; error boom` — the body has no normal fall-through and `x`
    // is set before the sole throw, so the handler sees `x` defined; no
    // W210. tclsh 9.0.3 reads x == 1 here.
    let mut a = Analyser::new();
    a.emit_document_cfg_ssa_diagnostics_for_test(
        "proc f {} {\n try {\n  set x 1\n  error boom\n } on error {} {\n  puts $x\n }\n}\n",
    );
    assert!(
        !a.result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W210),
        "W210 must not fire when x is set before the sole throw; got {:?}",
        a.result.diagnostics,
    );
}

#[test]
fn emit_cfg_ssa_diagnostics_w213_skipped_with_nocomplain() {
    // ``unset -nocomplain xs`` is the safe form — W213
    // must not fire.
    let mut a = Analyser::new();
    a.emit_document_cfg_ssa_diagnostics_for_test("proc foo {} { unset -nocomplain xs }");
    assert!(
        !a.result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W213),
        "W213 must not fire when ``-nocomplain`` is present; got {:?}",
        a.result.diagnostics,
    );
}

#[test]
fn emit_cfg_ssa_diagnostics_w210_fires_at_top_level() {
    // Top-level RBS now fires when no
    // proc writes the variable.  ``puts $undef`` reads
    // ``undef`` without any preceding write.
    let mut a = Analyser::new();
    a.emit_document_cfg_ssa_diagnostics_for_test("puts $undef");
    assert!(
        a.result
            .diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W210),
        "W210 must fire at top-level when no proc writes the var; got {:?}",
        a.result.diagnostics,
    );
}

#[test]
fn issue_703_no_false_w210_on_fallthrough_group_var() {
    // When a fallthrough handler's var differs from the target's, the shared
    // body reads whichever var the reached handler bound. Real Tcl is itself
    // inconsistent here — a byte-compiled `try` binds the *matching* handler's
    // var, the interpreted form the *target*'s — and a literal `try` in a
    // procedure is the byte-compiled form.
    let reads_fallthrough_var =
        "proc p {} {\n    try {set v ok} on ok x - on error y {\n        return $x\n    }\n}\n";
    let reads_target_var =
        "proc p {} {\n    try {set v ok} on ok x - on error y {\n        return $y\n    }\n}\n";
    assert_eq!(count_code(reads_fallthrough_var, "W210"), 0);
    // Narrow correction (Sol-42, 2026-10-19): a literal `try` in a procedure
    // is byte-compiled, which binds the matching `on ok x` handler's var, and
    // `set v ok` cannot raise, so only the `ok` arm reaches the shared body.
    // tclsh 8.6.18, 9.0.4 and 9.1.0 all fail `p` with `can't read "y": no
    // such variable`; the fresh-interpreter analysis reports that read.
    assert_eq!(count_code(reads_target_var, "W210"), 1);
}

#[test]
fn issue_703_genuine_read_before_set_still_fires_in_shared_body() {
    // A read of a variable bound by no handler in the group is still a genuine
    // read-before-set.
    let src = "proc p {} {\n    try {set v 1} on ok x - on error y {\n        return $undefinedvar\n    }\n}\n";
    assert_eq!(count_code(src, "W210"), 1);
}

#[test]
fn issue_703_backslash_escaped_dash_no_false_w210() {
    // A backslash-escaped `\-` handler body evaluates to
    // `-` and is a fallthrough, so the shared target body must not be flagged
    // W210 for reading the fallthrough handler's var (same as the bare `-`).
    let src =
        "proc p {} {\n    try {set v ok} on ok x \\- on error y {\n        return $x\n    }\n}\n";
    assert_eq!(count_code(src, "W210"), 0);
}

/// FP. A lambda's own parameters are bound by `apply`, so reading one in the
/// body is never a read-before-set.  A read-scan that walked the whole lambda
/// literal as if the body ran in the enclosing frame would draw `W210` on
/// every parameter.
///
/// Parameter binding is `proc` semantics — defaults and `args` included.
/// tclsh 9.0.4 / 8.6.14, identical:
///
/// ```text
/// apply {{a b} {return $a-$b}} 1 2       → 1-2
/// apply {{a {b 99}} {return $a-$b}} 1    → 1-99
/// apply {{a args} {return $a-$args}} 1 2 3 → 1-2 3
/// apply {{a args} {return $a-[llength $args]}} 1 → 1-0
/// ```
#[test]
fn apply_lambda_parameters_are_bound_not_read_before_set() {
    for src in [
        "apply {{a} {puts $a}} 5\n",
        "apply {{a b} {puts $a$b}} 5 6\n",
        "apply {{a args} {puts $a$args}} 5 6\n",
        "apply {{a args} {puts $a[llength $args]}} 5\n",
        "apply {{a {b 1}} {puts $a$b}} 5\n",
        // The lambda's namespace element must not shift the parameter binding.
        "apply {{a} {puts $a} ::ns} 5\n",
        // Nested inside a proc body, and inside a control-flow body.
        "proc p {} { apply {{a} {puts $a}} 5 }\n",
        "if {1} { apply {{a} {puts $a}} 5 }\n",
        // The command-substitution spelling was already clean; keep it so.
        "set r [apply {{a} {puts $a}} 5]\nputs $r\n",
    ] {
        let codes = codes_for_dialect(src, "tcl9.0");
        assert!(
            !codes.contains(&"W210".to_string()),
            "a bound lambda parameter must not fire W210 for {src:?}: {codes:?}"
        );
    }
}

/// TN — the same change must not blind the lambda body.  A name the body
/// reads without binding is a genuine error (tclsh 9.0.4 / 8.6.14:
/// `apply {{a} {return $zznever}} 5` → `can't read "zznever": no such
/// variable`), and the caller's locals are *not* visible inside the lambda
/// (`set x 7; apply {{} {puts $x}}` → `can't read "x": no such variable`), so
/// its body units carry the read-before-set family in their own right.
#[test]
fn apply_lambda_body_unbound_read_still_fires_w210() {
    for src in [
        "apply {{a} {puts $zznever}} 5\n",
        "apply {{a} {set b $zznever; puts $b}} 5\n",
        "proc p {} { apply {{a} {puts $zznever}} 5 }\n",
    ] {
        let codes = codes_for_dialect(src, "tcl9.0");
        assert!(
            codes.contains(&"W210".to_string()),
            "an unbound read in a lambda body must still fire W210 for {src:?}: {codes:?}"
        );
    }
}

/// TN — a `namespace eval` body unit is deliberately *not* in the
/// read-before-set family: its variables belong to the namespace and are
/// routinely written by a different body that opens the same namespace, so
/// the closed-frame argument a lambda gives does not hold there.
#[test]
fn namespace_eval_body_is_not_read_before_set_analysed() {
    let src = "namespace eval ::demo { variable counter }\n\
               namespace eval ::demo { variable counter; incr counter }\n";
    let codes = codes_for_dialect(src, "tcl9.0");
    assert!(
        !codes.contains(&"W210".to_string()),
        "a namespace-eval body must not be read-before-set analysed: {codes:?}"
    );
}

/// FN control — a *dynamic* lambda word is a real caller-frame read, so it
/// must keep firing.  `apply $lambda` reads `lambda` in the caller's frame
/// (tclsh 9.0.4 / 8.6.14 both raise `can't read "zzlambda"` for an unset one).
#[test]
fn dynamic_apply_lambda_word_is_still_a_caller_frame_read() {
    let codes = codes_for_dialect("proc p {} { apply $zzlambda 5 }\n", "tcl9.0");
    assert!(
        codes.contains(&"W210".to_string()),
        "`apply $lambda` must still read `lambda` in the caller's frame: {codes:?}"
    );
}

/// The names the W210 diagnostics of `src` report, on the whole-file path and
/// on the per-item path.
fn w210_names_on_both_paths(src: &str) -> (Vec<String>, Vec<String>) {
    let names = |per_item: bool| -> Vec<String> {
        let mut a = Analyser::new();
        let result = if per_item {
            a.analyse_per_item(src, "tcl8.6")
        } else {
            a.analyse(src, "tcl8.6")
        };
        result
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::W210)
            .map(|d| d.message.clone())
            .collect()
    };
    (names(false), names(true))
}

/// Code the module cannot see — a sourced file, a command it does not define, a
/// computed head, the body of a `catch` that holds one — may set a global the
/// file has not set by then, so a read of the name after it is no read before
/// it is set. A read the code does not precede on every path is one (no code
/// ahead of it, code after it, code on one branch only), and so is a read in a
/// procedure after a call it is not handed by name: a callee the module cannot
/// see reaches a local through `upvar 1` under the name it is given, which the
/// per-name abstention answers, while a sourced file runs in the procedure's
/// frame and may set any of its names.
#[test]
fn a_read_after_code_the_module_cannot_see_is_no_read_before_set() {
    for src in [
        "source other.tcl\nputs $g\n",
        "foo\nputs $g\n",
        "set cmd foo\n$cmd\nputs $g\n",
        "catch {foo}\nputs $g\n",
        "set x [foo]\nputs $g\n",
        "puts [foo]\nputs $g\n",
        "if {[foo]} {puts yes}\nputs $g\n",
        "foo\nset a 1\nputs $g\n",
        "foo\nputs $g\nset g 1\n",
        "foo\nif {$argc} {puts $g}\n",
        "proc p {} {\n source other.tcl\n puts $g\n}\n",
        "proc p {} {\n foo g\n puts $g\n}\n",
    ] {
        let (whole, per_item) = w210_names_on_both_paths(src);
        assert!(whole.is_empty(), "whole file: {src}\n{whole:?}");
        assert_eq!(per_item, whole, "per item: {src}");
    }
    for src in [
        "puts $g\n",
        "set a 1\nputs $g\n",
        "puts $g\nfoo\n",
        "foo $g\n",
        "proc foo {} {}\nfoo\nputs $g\n",
        "if {$argc} {foo}\nputs $g\n",
        "if {$argc} {set g 1}\nfoo\nputs $g\n",
        "proc p {} {\n foo\n puts $g\n}\n",
    ] {
        let (whole, per_item) = w210_names_on_both_paths(src);
        assert_eq!(whole.len(), 1, "whole file: {src}\n{whole:?}");
        assert!(whole[0].contains("'g'"), "{src}: {whole:?}");
        assert_eq!(per_item, whole, "per item: {src}");
    }
}

/// #2323: a name a `catch` body inside a substitution may write is not read
/// by that statement, so it is not read before it is set there.
#[test]
fn w210_does_not_report_a_name_a_substituted_catch_body_writes() {
    let source = "proc p {f} {\n    set rc [catch {set fd [open $f]} err]\n    if {$rc} {return}\n    puts [read $fd]\n}\n";
    assert_eq!(w210_codes(source), [] as [String; 0], "{source}");
}

#[test]
fn w210_reports_required_named_read_after_unset() {
    let source = "proc p {} { set v 1; unset v; return [catch {set v} m]:$m }\nputs [p]\n";
    let read = source.rfind("v}").unwrap();
    for dialect in ["tcl8.6", "tcl9.0"] {
        let result = Analyser::new().analyse(source, dialect);
        assert!(
            result.diagnostics.iter().any(|diagnostic| {
                diagnostic.code == DiagCode::W210 && diagnostic.span.as_range() == (read..read + 1)
            }),
            "{dialect}: {:?}",
            result.diagnostics
        );

        let probe = "proc p {} { set v 1; unset v; return [info exists v] }\nputs [p]\n";
        assert!(
            w210_codes_in_dialect(probe, dialect).is_empty(),
            "{dialect}"
        );
    }
}

/// A read after a loop selects the loop's joined contents: a counter or a
/// value the loop wrote from clean data is not tainted.
#[test]
fn a_read_after_a_loop_keeps_the_taint_of_the_loop_contents() {
    let codes = |src: &str| -> Vec<String> {
        Analyser::new()
            .analyse(src, "tcl8.6")
            .diagnostics
            .iter()
            .map(|d| d.code.to_string())
            .filter(|code| code.starts_with('T'))
            .collect()
    };
    for src in [
        "foreach i {1 2 3} {}\nputs $i\n",
        "for {set i 0} {$i < 5} {incr i} {}\nputs $i\n",
        "proc p {} {set i 0; while {$i < 3} {incr i}; puts $i}\n",
        "proc p {} {set i 0; foreach x {1 2} {set i $x}; puts $i}\n",
        "set l {a b c}\nfor {set i 0} {$i < 5} {incr i} {}\nset r [lindex $l $i]\nputs $r\n",
    ] {
        assert_eq!(codes(src), [] as [String; 0], "{src}");
    }
}

/// #2531: a fused `set n [expr {…}]` reads its nested commands through the
/// `set` invocation's retained words, so a constant one leaves `n` untainted.
#[test]
fn an_expression_assignment_around_a_constant_command_is_untainted() {
    let codes = |src: &str| -> Vec<String> {
        Analyser::new()
            .analyse(src, "tcl8.6")
            .diagnostics
            .iter()
            .map(|d| d.code.to_string())
            .filter(|code| code.starts_with('T'))
            .collect()
    };
    for src in [
        "set n [expr {[string length abc] * 2}]\nif {$n == 6} {puts yes}\n",
        "set n [expr {[llength [list 1]] + 2}]\nputs [expr {$n + 1}]\n",
        "proc p {} {set n [expr {[string length abc] * 2}]; if {$n == 6} {puts yes}}\n",
    ] {
        assert_eq!(codes(src), [] as [String; 0], "{src}");
    }
}

/// #2523: a loop at the top level does not deny a procedure defined after it
/// the representation receipts its own literal stores issue, so its counter
/// stays numeric and every loop's verdict is reached.
#[test]
fn a_procedure_after_a_top_level_loop_keeps_its_loop_verdicts() {
    let w241 = |src: &str| {
        Analyser::new()
            .analyse(src, "tcl8.6")
            .diagnostics
            .iter()
            .filter(|d| d.code.to_string() == "W241")
            .count()
    };
    assert_eq!(
        w241(
            "for {set i 0} {$i < 10} {incr i -1} {puts x}\n\
             proc f {} {for {set i 0} {$i < 10} {incr i -1} {}}\n\
             proc p {} { set i 0; while {$i < 10} {incr i -1} }\n"
        ),
        3
    );
}
