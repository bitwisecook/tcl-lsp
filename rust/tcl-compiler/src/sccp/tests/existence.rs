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

//! The existence rung: up-frame bodies, `set`/`unset` then `info exists`, and calls to unseen code.

use super::*;

/// A static-body `uplevel 0` is an `UpFrame`, not a generic
/// barrier, and its body creates a local in the frame whose existence
/// query follows. The existence fold must never answer that the local is
/// absent. Since the binding walk follows a braced literal `uplevel`
/// script into the frame it selects, the query is decided: tclsh 8.4.20,
/// 8.6.18, 9.0.4 and 9.1.0 all print `yes` for `puts [f]`.
#[test]
fn existence_fold_abstains_for_a_live_upframe() {
    let registry = registry();
    let cu = crate::compilation_unit::CompilationUnit::build_for(
        "proc f {} { uplevel 0 { set created 1 }; if {[info exists created]} { return yes } else { return no } }",
        &registry,
        false,
    );
    let f = cu.function("::f").expect("procedure analysed");
    assert!(
        f.cfg
            .blocks
            .values()
            .flat_map(|block| block.statements.iter())
            .any(|statement| matches!(statement, Statement::UpFrame { .. })),
        "the regression requires the static uplevel lowering path"
    );
    assert!(
        f.sccp.constant_branches.iter().all(|branch| branch.value),
        "UpFrame defines `created`, so info exists must not fold to absent: {:?}",
        f.sccp.constant_branches
    );
}

/// `set x 1; unset x; info exists x` decides 0 inside the fixed point:
/// the query reads the rung, the branch is an `Applied` fact,
/// and its true arm is unreachable. tclsh 8.4 to 9.1 print `no`.
#[test]
fn set_unset_info_exists_decides_zero() {
    let registry = registry();
    let cu = crate::compilation_unit::CompilationUnit::build_for(
        "proc f {} { set x 1; unset x; if {[info exists x]} { puts yes } else { puts no } }",
        &registry,
        false,
    );
    let f = cu.function("::f").expect("procedure analysed");
    let decided = f
        .sccp
        .constant_branches
        .iter()
        .find(|branch| branch.condition == "[info exists x]")
        .unwrap_or_else(|| panic!("the query decides: {:?}", f.sccp.constant_branches));
    assert!(!decided.value);
    assert_eq!(decided.kind, BranchFactKind::Applied);
    let dead = f.cfg.block_id(&decided.not_taken_target).expect("the arm");
    assert!(!f.sccp.executable_blocks.contains(&dead));
}

#[test]
fn existence_fold_abstains_for_nested_upframe_alias_unset_mutation() {
    // `uplevel 0` evaluates in this procedure's frame. The nested body
    // aliases that frame's parameter and unsets it, so Tcl observes the else
    // branch. The no-uplevel twin proves the branch is otherwise foldable;
    // this is a mutation test of the exact fact the fold must block.
    let registry = registry();
    let stable = crate::compilation_unit::CompilationUnit::build_for(
        "proc f {local} { if {[info exists local]} { return yes } else { return no } }",
        &registry,
        false,
    );
    assert!(
        !stable
            .function("::f")
            .expect("control procedure analysed")
            .sccp
            .constant_branches
            .is_empty(),
        "the unmutated control must be foldable"
    );

    let mutated = crate::compilation_unit::CompilationUnit::build_for(
        "proc f {local} { uplevel 0 { uplevel 0 { upvar 0 local alias; unset alias } }; if {[info exists local]} { return yes } else { return no } }",
        &registry,
        false,
    );
    let f = mutated.function("::f").expect("mutated procedure analysed");
    let outer = f
        .cfg
        .blocks
        .values()
        .flat_map(|block| block.statements.iter())
        .find_map(|statement| match statement {
            Statement::UpFrame { body, .. } => Some(body),
            _ => None,
        })
        .expect("outer literal uplevel must lower to UpFrame");
    assert!(
        outer
            .statements
            .iter()
            .any(|statement| matches!(statement, Statement::UpFrame { .. })),
        "the nested literal uplevel must remain an UpFrame in its parent body"
    );
    assert!(
        f.sccp.constant_branches.is_empty(),
        "nested upframe/upvar/unset may remove local: {:?}",
        f.sccp.constant_branches
    );
}

#[test]
fn upframe_body_models_upvar_alias_unset_in_the_caller_frame() {
    // tclsh: `proc caller {} {set caller_value 1; f; info exists
    // caller_value}; proc f {} {uplevel 1 {upvar 0 caller_value alias;
    // unset alias}}; caller` returns 0. The literal UpFrame body therefore
    // carries a real caller-frame mutation, not merely a control-flow
    // wrapper, and is a separate vector from the nested local-frame test.
    let registry =
        registry().project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let cu = crate::compilation_unit::CompilationUnit::build_for(
        "proc f {} { uplevel 1 { upvar 0 caller_value alias; unset alias } }",
        &registry,
        false,
    );
    let f = cu.function("::f").expect("procedure analysed");
    let body = f
        .cfg
        .blocks
        .values()
        .flat_map(|block| block.statements.iter())
        .find_map(|statement| match statement {
            Statement::UpFrame { body, .. } => Some(body),
            _ => None,
        })
        .expect("literal caller-frame body must lower to UpFrame");
    assert!(body.statements.iter().any(|statement| {
        matches!(statement, Statement::Call { command, .. } if command == "upvar")
    }));
    assert!(body.statements.iter().any(|statement| {
        matches!(statement, Statement::Call { command, .. } if command == "unset")
    }));
}

/// A call to a command the module cannot see may create or destroy a plain
/// top-level name as it may a `::` one, and a procedure's local through
/// `upvar 1`, so `[info exists]` after it decides nothing; a call the
/// module resolves leaves the local's existence decided.
#[test]
fn an_unseen_call_leaves_no_name_with_a_settled_existence() {
    let registry =
        registry().project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    // #2427: a bare undefined command no longer widens the command table; an autoload index entry in view keeps this device unbounded.
    for (source, function) in [
        (
            "set ::auto_index(foo) {source lib.tcl}\nset x 1\nfoo\nif {[info exists x]} { puts a } else { puts b }\n",
            "::top",
        ),
        (
            "set ::auto_index(foo) {source lib.tcl}\nfoo\nif {[info exists never_set]} { puts a } else { puts b }\n",
            "::top",
        ),
        (
            "set ::auto_index(foo) {source lib.tcl}\nproc p {} { set x 1; foo; if {[info exists x]} { puts a } else { puts b } }\n",
            "::p",
        ),
    ] {
        let cu = crate::compilation_unit::CompilationUnit::build_for(source, &registry, false);
        let f = cu.function(function).expect("function analysed");
        assert!(f.sccp.constant_branches.is_empty(), "{source}");
    }
    let cu = crate::compilation_unit::CompilationUnit::build_for(
        "proc foo {} { puts hi }\nproc p {} { set x 1; foo; if {[info exists x]} { puts a } else { puts b } }\n",
        &registry,
        false,
    );
    let f = cu.function("::p").expect("procedure analysed");
    let decided = f
        .sccp
        .constant_branches
        .iter()
        .find(|branch| branch.condition == "[info exists x]")
        .expect("the local's existence is decided");
    assert!(decided.value);
}
