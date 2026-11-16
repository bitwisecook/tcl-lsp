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

//! What one command lowers to: a smoke test per statement kind, and the runtime barriers of the default call.

use super::*;

#[test]
fn empty_source() {
    let m = lower_to_ir("", &reg());
    assert_eq!(m.top_level.statements, [] as [crate::ir::Statement; 0]);
}

#[test]
fn simple_set() {
    let m = lower_to_ir("set x 1", &reg());
    assert_eq!(m.top_level.statements.len(), 1);
    assert!(matches!(
        &m.top_level.statements[0],
        Statement::AssignConst { name, value, .. } if name == "x" && value == "1"
    ));
}

#[test]
fn set_with_variable_value() {
    let m = lower_to_ir("set x [gets stdin]\nset y $x", &reg());
    assert_eq!(m.top_level.statements.len(), 2);
    assert!(matches!(
        &m.top_level.statements[1],
        Statement::AssignValue { name, .. } if name == "y"
    ));
}

#[test]
fn incr_command() {
    let m = lower_to_ir("incr i", &reg());
    assert_eq!(m.top_level.statements.len(), 1);
    assert!(matches!(
        &m.top_level.statements[0],
        Statement::Incr { name, .. } if name == "i"
    ));
}

#[test]
fn proc_definition() {
    let m = lower_to_ir("proc greet {name} {puts $name}", &reg());
    // proc emits a Statement::Call + registers a procedure.
    assert!(fixture_id_if_present(&m.procedures, "::greet").is_some());
    let p = m
        .procedures
        .get(&fixture_id(&m.procedures, "::greet"))
        .expect("authored fixture body");
    assert_eq!(p.params, vec!["name"]);
}

#[test]
fn if_statement() {
    let m = lower_to_ir("if {1} {set x 1}", &reg());
    assert_eq!(m.top_level.statements.len(), 1);
    assert!(matches!(&m.top_level.statements[0], Statement::If { .. }));
}

#[test]
fn for_loop() {
    let m = lower_to_ir("for {set i 0} {$i < 10} {incr i} {puts $i}", &reg());
    assert_eq!(m.top_level.statements.len(), 1);
    assert!(matches!(&m.top_level.statements[0], Statement::For { .. }));
}

#[test]
fn while_loop() {
    let m = lower_to_ir("while {1} {puts loop}", &reg());
    assert_eq!(m.top_level.statements.len(), 1);
    assert!(matches!(
        &m.top_level.statements[0],
        Statement::While { .. }
    ));
}

#[test]
fn foreach_loop() {
    let m = lower_to_ir("foreach x {a b c} {puts $x}", &reg());
    assert_eq!(m.top_level.statements.len(), 1);
    assert!(matches!(
        &m.top_level.statements[0],
        Statement::Foreach { .. }
    ));
}

#[test]
fn catch_statement() {
    let m = lower_to_ir("catch {error oops} result", &reg());
    assert_eq!(m.top_level.statements.len(), 1);
    assert!(matches!(
        &m.top_level.statements[0],
        Statement::Catch { .. }
    ));
}

#[test]
fn generic_command() {
    let m = lower_to_ir("puts hello", &reg());
    assert_eq!(m.top_level.statements.len(), 1);
    assert!(matches!(
        &m.top_level.statements[0],
        Statement::Call { command, .. } if command == "puts"
    ));
}

#[test]
fn multiple_commands() {
    let m = lower_to_ir("set x 1\nset y 2\nputs $x", &reg());
    assert_eq!(m.top_level.statements.len(), 3);
}

#[test]
fn return_statement() {
    let m = lower_to_ir("return 42", &reg());
    assert_eq!(m.top_level.statements.len(), 1);
    assert!(matches!(
        &m.top_level.statements[0],
        Statement::Return { value: Some(v), .. } if v == "42"
    ));
}

#[test]
fn deferred_tk_callback_does_not_hide_constructor_variable_defs() {
    let m = lower_to_ir(
        "entry .e -textvariable input -validatecommand {return 1}",
        &reg(),
    );
    let stmt = m.top_level.statements.first().expect("entry call");
    match stmt {
        Statement::Call { command, defs, .. } => {
            assert_eq!(command, "entry");
            assert_eq!(defs, &["::input"]);
        }
        other => panic!("deferred callback must not make the constructor opaque: {other:?}"),
    }
}

#[test]
fn immediate_tcltest_body_remains_a_runtime_barrier() {
    let m = lower_to_ir(
        "tcltest::test sample description -body {error failure}",
        &reg(),
    );
    let stmt = m.top_level.statements.first().expect("test call");
    assert_runtime_opaque_call(stmt, &reg(), "tcltest::test");
}

#[test]
fn same_invocation_command_prefix_is_a_runtime_barrier() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let m = lower_to_ir("lsort -command compare {b a}", &reg());
    let stmt = m.top_level.statements.first().expect("lsort call");
    assert_runtime_opaque_call(stmt, &reg(), "lsort");
}

#[test]
fn deferred_command_prefix_does_not_create_a_runtime_barrier() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    for (source, expected_command) in [
        ("scrollbar .s -command moved", "scrollbar"),
        ("trace add variable v write changed", "trace"),
    ] {
        let m = lower_to_ir(source, &reg());
        let stmt = m.top_level.statements.first().expect("callback host call");
        assert!(
            matches!(stmt, Statement::Call { command, .. } if command == expected_command),
            "a stored command prefix cannot affect its registration invocation: {stmt:?}",
        );
    }
}

#[test]
fn reference_only_command_prefix_does_not_create_a_runtime_barrier() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let m = lower_to_ir("trace remove variable v write changed", &reg());
    let stmt = m.top_level.statements.first().expect("trace remove call");
    assert!(
        matches!(stmt, Statement::Call { command, .. } if command == "trace"),
        "matching a prior registration does not execute the prefix: {stmt:?}",
    );
}
