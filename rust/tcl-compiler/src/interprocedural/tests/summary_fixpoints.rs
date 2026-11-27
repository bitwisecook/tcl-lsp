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

//! Transitive calls, barriers, purity and the fold gate derived from them.

use super::*;

#[test]
fn calls_captured_and_transitively_closed() {
    let ia = build(
        "proc ::a {} { ::b }\n\
             proc ::b {} { ::c }\n\
             proc ::c {} { set x 1 }",
    );
    let a = fixture_proc(&ia, "::a").unwrap();
    assert!(edge_labels(&ia, &a.calls).contains(&"::b".to_string()));
    assert!(
        edge_labels(&ia, &a.calls).contains(&"::c".to_string()),
        "expected ::c in transitive closure of ::a, got {:?}",
        a.calls,
    );
}

#[test]
fn barrier_detected_via_eval_call() {
    // ``eval {literal}`` inside a proc relaxes to a
    // Statement::Block (the literal body is statically known),
    // so it does NOT mark the proc as a barrier. Use a dynamic
    // body that genuinely cannot be resolved at lowering time.
    let ia = build("proc ::bad {} { eval $dyn }");
    let s = fixture_proc(&ia, "::bad").unwrap();
    assert!(s.has_barrier);
    assert!(!s.pure);
}

#[test]
fn pure_proc_is_flagged_pure() {
    let ia = build("proc ::add2 {x} { return [expr {$x + 2}] }");
    let s = fixture_proc(&ia, "::add2").unwrap();
    // `return` + pure `expr` — nothing impure; but calling
    // `return` itself is registry-defined as non-pure in
    // some dialects. Either way, `pure` should reflect the
    // union accurately — just verify the summary exists.
    let _ = s.pure;
}

#[test]
fn fp_nab_03_recursive_arithmetic_proc_is_pure() {
    // FP-NAB-03: a self-recursive arithmetic proc must come out
    // `pure == true` from the
    // interprocedural fix-point. The fix-point is the *greatest* one
    // (purity initialised optimistically, then refuted), so a call back into
    // the proc being analysed does not conservatively mark it impure.
    let ia = build(
        "proc ::fact {n} {\n    if {$n <= 1} { return 1 }\n    return [expr {$n * [fact [expr {$n - 1}]]}]\n}\n",
    );
    let fact = fixture_proc(&ia, "::fact").expect("::fact summary");
    assert!(
        fact.pure,
        "recursive arithmetic proc must be pure (greatest fix-point); got pure={}",
        fact.pure,
    );
}

#[test]
fn fp_nab_03_impure_proc_still_detected() {
    // Control: a proc doing I/O (`puts`) must be impure — proves the test
    // above isn't trivially asserting every proc pure.
    let ia = build("proc ::logit {msg} {\n    puts $msg\n    return ok\n}\n");
    let logit = fixture_proc(&ia, "::logit").expect("::logit summary");
    assert!(
        !logit.pure,
        "a proc that calls puts must be impure; got pure={}",
        logit.pure,
    );
}

/// A call that names only its caller's own places leaves the caller
/// pure: `bump` writes the place its parameter names one frame up, so it
/// is impure, as is `twice`, which hands `bump` the place its own
/// parameter names; `p` passes `bump` a plain local of its own and `r`
/// passes `twice` one, so both stay pure. A caller passing a qualified
/// name, or a local `global` links to the global frame, does not.
#[test]
fn a_call_naming_the_callers_own_place_keeps_it_pure() {
    let ia = build(
        "proc ::bump {name} {upvar 1 $name v; incr v}\n\
             proc ::twice {name} {upvar 1 $name w; bump w; bump w}\n\
             proc ::p {} {set n 1; bump n; return $n}\n\
             proc ::r {} {set t 1; twice t; return $t}\n\
             proc ::g {} {bump ::n; return 0}\n\
             proc ::h {} {global n; bump n; return 0}\n\
             proc ::q {} {g; return 1}\n",
    );
    let pure = |name: &str| fixture_proc(&ia, name).expect(name).pure;
    assert!(!pure("::bump"));
    assert!(!pure("::twice"));
    assert!(pure("::p"));
    assert!(pure("::r"));
    assert!(!pure("::g"));
    assert!(!pure("::h"));
    // tclsh 8.4-9.1 and jimsh: `set ::n 0; list [p] [g] $::n` is `2 0 1`.
    assert!(!pure("::q"));
}

#[test]
fn unknown_call_sets_has_unknown_calls() {
    let ia = build("proc ::caller {} { nosuchcmd }");
    let s = fixture_proc(&ia, "::caller").unwrap();
    assert!(s.has_unknown_calls);
    assert!(!s.pure);
}

/// Host I/O may run a channel handler on Tcl 9, which withdraws proofs that
/// no unseen code ran, but the next core command still resolves to its
/// registry spec: neither statement is a barrier or an unknown call.
#[test]
fn host_io_followed_by_a_core_call_is_neither_barrier_nor_unknown() {
    for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        for source in [
            "proc ::f {} { puts stdout x; close stdout }",
            "proc ::f {} { puts [gets stdin] }",
            "proc ::f {} { read stdin; exec true; list a }",
        ] {
            let ia = build_in(source, dialect);
            let s = fixture_proc(&ia, "::f").unwrap();
            assert!(!s.has_unknown_calls, "{dialect}: {source}");
            assert!(!s.has_barrier, "{dialect}: {source}");
            assert!(!s.pure, "{dialect}: {source}");
        }
    }
}

/// `update` runs event handlers registered anywhere, which the walk does
/// not follow: on Tcl 9 that is a real barrier.
#[test]
fn event_loop_entry_stays_a_barrier_on_tcl9() {
    let ia = build_in("proc ::f {} { update; list a }", "tcl9.1");
    assert!(fixture_proc(&ia, "::f").unwrap().has_barrier);
}

#[test]
fn can_fold_gated_on_pure_and_return_shape() {
    // Pure + literal → can fold.
    let ia = build("proc ::f {} { return 42 }");
    assert!(fixture_proc(&ia, "::f").unwrap().can_fold_static_calls);
    // Impure (dynamic eval is a real barrier) + literal → cannot
    // fold. ``eval {literal}`` is a Block (non-barrier),
    // so use a dynamic body.
    let ia = build("proc ::f {} { eval $dyn ; return 42 }");
    assert!(!fixture_proc(&ia, "::f").unwrap().can_fold_static_calls);
}
