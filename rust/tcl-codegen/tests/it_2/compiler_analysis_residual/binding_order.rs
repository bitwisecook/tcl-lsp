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

//! Binding states follow Tcl's evaluation order: sibling and nested substitutions, short circuits and loop back edges.

use super::*;

#[test]
fn sibling_if_conditions_join_source_order_binding_states() {
    let cu = CompilationUnit::build_for(
        "proc safe args {return 0}; set x 5; if {0} {} elseif {[rename safe {}; rename eval safe] eq \"never\"} {} elseif {[safe {set x 6; expr 0}]} {}; if {$x == 5} {puts stale} else {puts changed}",
        &reg(),
        false,
    );
    assert!(
        !cu.top_level
            .sccp
            .constant_branches
            .iter()
            .any(|branch| branch.condition == "$x == 5"),
        "a later elseif condition must observe preceding condition transitions",
    );
}

#[test]
fn for_init_transition_reaches_first_condition_projection() {
    let cu = CompilationUnit::build_for(
        "proc safe args {return 0}; set x 5; for {rename safe {}; rename eval safe} {[safe {set x 6; expr 0}]} {} {}; if {$x == 5} {puts stale} else {puts changed}",
        &reg(),
        false,
    );
    assert!(
        !cu.top_level
            .sccp
            .constant_branches
            .iter()
            .any(|branch| branch.condition == "$x == 5"),
        "a for init transition must widen the first condition projection",
    );
}

#[test]
fn foreach_backedge_transition_widens_next_iteration() {
    let cu = CompilationUnit::build_for(
        "proc safe args {return 0}; set x 5; foreach i {1 2} {safe {set x 6}; if {$i == 1} {rename safe {}; rename eval safe}}; if {$x == 5} {puts stale} else {puts changed}",
        &reg(),
        false,
    );
    assert!(
        !cu.top_level
            .sccp
            .constant_branches
            .iter()
            .any(|branch| branch.condition == "$x == 5"),
        "a foreach backedge transition must widen later list/body evaluation",
    );
}

#[test]
fn sibling_substitutions_follow_tcl_argument_order() {
    let cu = CompilationUnit::build_for(
        "proc aaa args {return 0}; set x 5; list [rename aaa {}; rename eval aaa] [aaa {set x 6}]; if {$x == 5} {puts stale} else {puts changed}",
        &reg(),
        false,
    );
    assert!(
        cu.top_level.sccp.constant_branches.is_empty(),
        "later substitution arguments must see earlier transitions",
    );
}

#[test]
fn nested_substitution_runs_before_outer_invocation() {
    let cu = CompilationUnit::build_for(
        "proc aaa args {return 0}; set x 5; set ignored [aaa [rename aaa {}; rename eval aaa; set _ {set x 6}]]; if {$x == 5} {puts stale} else {puts changed}",
        &reg(),
        false,
    );
    assert!(
        cu.top_level.sccp.constant_branches.is_empty(),
        "nested substitutions must transition bindings before their outer command",
    );
}

#[test]
fn nested_alias_substitution_runs_before_outer_invocation() {
    let cu = CompilationUnit::build_for(
        "interp alias {} safe {} string length; set x 5; set ignored [safe [rename safe {}; rename eval safe; set _ {set x 6}]]; if {$x == 5} {puts stale} else {puts changed}",
        &reg(),
        false,
    );
    assert!(
        cu.top_level.sccp.constant_branches.is_empty(),
        "a nested rename must reach the outer registry alias before it is resolved",
    );
}

#[test]
fn bracket_script_segments_run_in_order_around_nested_substitution() {
    let cu = CompilationUnit::build_for(
        "proc aaa args {return 0}; set x 5; set ignored [rename eval zzz; aaa [rename aaa {}; rename zzz aaa; set _ {set x 6}]]; if {$x == 5} {puts stale} else {puts changed}",
        &reg(),
        false,
    );
    assert!(
        cu.top_level.sccp.constant_branches.is_empty(),
        "a nested substitution in command two must not run before command one",
    );
}

#[test]
fn short_circuit_condition_keeps_skipped_binding_path() {
    let cu = CompilationUnit::build_for(
        "proc unknown args {return 0}; rename eval safe; set x 5; if {[pid] < 0 && [rename safe {}]} {}; safe {set x 6}; if {$x == 5} {puts stale} else {puts changed}",
        &reg(),
        false,
    );
    assert!(
        cu.top_level.sccp.constant_branches.is_empty(),
        "a skipped short-circuit RHS must not erase the reachable eval alias",
    );
}

#[test]
fn short_circuit_expression_keeps_skipped_binding_path() {
    let cu = CompilationUnit::build_for(
        "proc unknown args {return 0}; rename eval safe; set x 5; expr {[pid] < 0 && [rename safe {}]}; safe {set x 6}; if {$x == 5} {puts stale} else {puts changed}",
        &reg(),
        false,
    );
    assert!(
        cu.top_level.sccp.constant_branches.is_empty(),
        "conditional expression substitutions outside if must retain their skipped path",
    );
}

#[test]
fn later_substitution_keeps_short_circuit_binding_uncertainty() {
    let cu = CompilationUnit::build_for(
        "proc unknown args {return 0}; rename eval safe; set x 5; expr {([pid] < 0 && [rename safe {}]) + [safe {set x 6; expr 0}]}; if {$x == 5} {puts stale} else {puts changed}",
        &reg(),
        false,
    );
    assert!(
        cu.top_level.sccp.constant_branches.is_empty(),
        "a later substitution must retain the skipped binding path in the same expression",
    );
}

#[test]
fn foreach_binding_projection_closes_more_than_one_backedge() {
    let cu = CompilationUnit::build_for(
        "proc b args {return 0}; proc c args {return 0}; rename eval a; set x 5; foreach i {1 2 3} {c {set x 6}; if {$i < 3} {rename c {}; rename b c; if {$i == 1} {rename a b}}}; if {$x == 5} {puts stale} else {puts changed}",
        &reg(),
        false,
    );
    assert!(
        !cu.top_level
            .sccp
            .constant_branches
            .iter()
            .any(|branch| branch.condition == "$x == 5"),
        "a loop binding projection must not mistake its first post-state for closure",
    );
}

#[test]
fn braced_nested_substitution_text_does_not_execute() {
    let cu = CompilationUnit::build_for(
        "interp alias {} safe {} string length; set x 5; set ignored [list {[rename safe {}; rename eval safe]}]; safe {set x 6}; if {$x == 5} {puts kept} else {puts changed}",
        &reg(),
        false,
    );
    assert!(
        !cu.top_level.sccp.constant_branches.is_empty(),
        "braced text in a recovered command word must remain inert",
    );
}
