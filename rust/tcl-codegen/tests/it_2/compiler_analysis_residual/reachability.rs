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

//! Dead code after `return`, `error`, `break` and `continue`, and constant guards.

use super::*;

// analyser/diagnostics/fp/rch.rs — reachability (O107) FP suppressor carve-outs
//
// The reachability suppressor decides whether code after an unconditional exit
// (return / break / continue / error), or guarded by a constant-false
// condition, is genuinely dead (O107 fires) or only apparently dead (suppressed).
// Each carve-out gets a fires/suppressed control pair, with the Tcl-observable
// reachability fact pinned to tclsh.

#[test]
fn rch_code_after_return_is_dead() {
    // tclsh (8.6, 9.0): `proc f {} { return 1; puts after }; f` → 1 (the `puts
    // after` never runs — it is unreachable).  O107 must fire on it.
    let src = "proc f {} { return 1\n puts after }";
    assert!(
        o107_fires(src, D),
        "code after an unconditional return is dead; O107 must fire; emitted {:?}",
        codes(src, D)
    );
}

#[test]
fn rch_code_after_conditional_return_is_reachable() {
    // Control: when the return is GUARDED (`if {$c} { return }`), the tail is
    // reachable — O107 must NOT fire.
    // tclsh (8.6, 9.0): `proc f {c} { if {$c} { return 1 }; puts after }; f 0`
    // → prints `after` (reachable when c is false).
    let src = "proc f {c} { if {$c} { return 1 }\n puts after }";
    assert!(
        !o107_fires(src, D),
        "tail after a conditional return is reachable; O107 must NOT fire; emitted {:?}",
        codes(src, D)
    );
}

#[test]
fn rch_code_after_error_is_dead() {
    // `error` always raises — code after it (in the same straight-line block)
    // is unreachable.
    // tclsh (8.6, 9.0): `proc f {} { error boom; puts after }; catch f` — the
    // `puts after` never runs.  O107 fires.
    let src = "proc f {} { error boom\n puts after }";
    assert!(
        o107_fires(src, D),
        "code after error is dead; O107 must fire; emitted {:?}",
        codes(src, D)
    );
}

#[test]
fn rch_code_after_break_in_loop_is_dead() {
    // An unconditional `break` makes the rest of the loop body dead.
    // tclsh (8.6, 9.0): `foreach x {1 2} { break; puts inner }` — `puts inner`
    // never runs.  O107 fires on it.
    let src = "proc f {} { foreach x {1 2} { break\n puts inner } }";
    assert!(
        o107_fires(src, D),
        "code after an unconditional break in a loop is dead; emitted {:?}",
        codes(src, D)
    );
}

#[test]
fn rch_code_after_continue_in_loop_is_dead() {
    // An unconditional `continue` makes the rest of the loop body dead.
    // tclsh (8.6, 9.0): `foreach x {1 2} { continue; puts inner }` — `puts
    // inner` never runs.  O107 fires.
    let src = "proc f {} { foreach x {1 2} { continue\n puts inner } }";
    assert!(
        o107_fires(src, D),
        "code after an unconditional continue in a loop is dead; emitted {:?}",
        codes(src, D)
    );
}

#[test]
fn rch_conditional_break_keeps_loop_tail_reachable() {
    // Control: a CONDITIONAL break leaves the loop-body tail reachable.
    // tclsh (8.4 to 9.1): `foreach x {1 2} { if {$x > 1} break; puts inner }`
    // prints `inner` once — the break runs for the second element only, so
    // the condition decides nothing and O107 must NOT fire. (Over `$x > 5`
    // every member of `{1, 2}` is false, the break never runs — tclsh prints
    // `inner` twice — and O107 removing it is right.)
    let src = "proc f {} { foreach x {1 2} { if {$x > 1} break\n puts inner } }";
    assert!(
        !o107_fires(src, D),
        "a conditional break keeps the loop tail reachable; emitted {:?}",
        codes(src, D)
    );
}

#[test]
fn rch_constant_false_guard_is_constant_branch_not_o107() {
    // A statically-false guard (`if {0} { … }`) makes its body unreachable, but
    // that is handled by SCCP's *constant-branch* path (the I230 dead-arm info),
    // NOT the post-exit reachability O107.  This is a reachability carve-out: the
    // O107 suppressor must NOT also fire on a constant-folded branch (it would be
    // a duplicate / mis-attributed diagnostic).
    // tclsh (8.6, 9.0): `if {0} { puts never }` prints nothing (the body is
    // dead) — surfaced here as I230, the constant-branch dead-arm info.
    let src = "proc f {} { if {0} { puts never } }";
    assert!(
        fires(src, D, "I230"),
        "an if {{0}} dead arm must surface as I230; emitted {:?}",
        codes(src, D)
    );
    assert!(
        !o107_fires(src, D),
        "a constant-folded if {{0}} must NOT also fire O107; emitted {:?}",
        codes(src, D)
    );
}

#[test]
fn rch_constant_true_guard_body_is_reachable() {
    // Control: an `if {1} { … }` body is always reached — its body is live, so
    // no O107 on it (the absent else-branch is the dead arm, surfaced as the
    // I230 constant-branch info, not O107).
    let src = "proc f {} { if {1} { puts always } else { puts never } }";
    assert!(
        !o107_fires(src, D),
        "an if {{1}} body is reachable; O107 must NOT fire; emitted {:?}",
        codes(src, D)
    );
    assert!(
        fires(src, D, "I230"),
        "the constant-true branch's dead else-arm surfaces as I230; emitted {:?}",
        codes(src, D)
    );
}

#[test]
fn rch_while1_no_break_post_loop_is_dead() {
    // A `while 1` body with no break/return makes the post-loop block dead.
    // tclsh: `while 1 { puts x }` never exits, so `puts after` is unreachable.
    let src = "proc f {} { while 1 { puts x }\n puts after }";
    assert!(
        o107_fires(src, D),
        "post-loop block after an infinite while-1 is dead; emitted {:?}",
        codes(src, D)
    );
}

#[test]
fn rch_while1_with_break_post_loop_is_reachable() {
    // Control: a `while 1` with a (conditional) break can exit, so the
    // post-loop block is reachable — O107 must NOT fire.
    let src = "proc f {c} { while 1 { if {$c} break }\n puts after }";
    assert!(
        !o107_fires(src, D),
        "a while-1 with a break makes the post-loop block reachable; emitted {:?}",
        codes(src, D)
    );
}
