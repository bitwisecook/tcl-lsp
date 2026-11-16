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

//! Command boundaries of structured commands: start-command owners, constant-`if` boundaries and bodies left for runtime compilation.

use super::*;

#[test]
fn structured_commands_after_a_generic_invoke_own_their_start_command() {
    let source = "set n 0; foo\n\
                      if {$n} {puts a}\n\
                      switch $n {1 {puts b}}\n\
                      for {set i 0} {$i < 1} {incr i} {puts c}\n\
                      for {} {$n} {} {puts d}\n\
                      while {$n} {if {$n} {puts e} else {puts f}}";
    let module = BytecodeCompileService::default().compile(source).unwrap();
    let markers: Vec<(&str, i32)> = module
        .top_level
        .instructions
        .iter()
        .filter(|instruction| instruction.op == tcl_bytecode::Op::START_CMD)
        .map(|instruction| {
            let count = match instruction.operands.get(1) {
                Some(tcl_bytecode::Operand::Imm(count)) => *count,
                _ => 0,
            };
            (
                instruction
                    .source_cmd_text
                    .try_text()
                    .expect("utf-8 command"),
                count,
            )
        })
        .collect();
    // Tcl 9.0.4's startCommands for the same script, in order.
    assert_eq!(
        markers,
        [
            ("if {$n} {puts a}", 1),
            ("switch $n {1 {puts b}}", 1),
            ("for {set i 0} {$i < 1} {incr i} {puts c}", 2),
            ("incr i", 1),
            ("for {} {$n} {} {puts d}", 1),
            ("while {$n} {if {$n} {puts e} else {puts f}}", 1),
            ("if {$n} {puts e} else {puts f}", 1),
        ],
    );
}

#[test]
fn cfg_edges_keep_ranges_but_are_not_runtime_recompile_sites() {
    let source = "proc continue {} {return -code break}\n\
                      if {[incr i] > 3} {set seen 1}\ncontinue";
    let module = BytecodeCompileService::default().compile(source).unwrap();
    let edge = module
        .top_level
        .instructions
        .iter()
        .find(|instruction| {
            matches!(
                instruction.op,
                tcl_bytecode::Op::JUMP1 | tcl_bytecode::Op::JUMP4
            ) && instruction.source_span.is_some()
        })
        .expect("the if body has a source-mapped CFG edge");
    assert_eq!(edge.source_cmd_text, "");
    assert!(module.top_level.instructions.iter().any(|instruction| {
        instruction.source_cmd_text == "continue"
            && matches!(
                instruction.op,
                tcl_bytecode::Op::INVOKE_STK1 | tcl_bytecode::Op::INVOKE_STK4
            )
    }));
}

#[test]
fn generic_eval_keeps_its_body_for_runtime_compilation() {
    let module = BytecodeCompileService::default()
        .compile("eval {while {0} {}}")
        .unwrap();
    assert_eq!(
        module.top_level.command_bindings,
        [] as [tcl_runtime_api::CommandBindingIdentity; 0]
    );
    assert!(module.top_level.instructions.iter().any(|instruction| {
        matches!(
            instruction.op,
            tcl_bytecode::Op::INVOKE_STK1 | tcl_bytecode::Op::INVOKE_STK4
        ) && instruction.source_cmd_text == "eval {while {0} {}}"
    }));
}

#[test]
fn synthetic_structured_boundaries_have_the_exact_owning_command() {
    let source = "set marker 1; if {1} {set marker 2}; proc p {} {set marker 1; foreach x {a} {if {1} {append marker $x}}; while {0} {}}";
    let module = BytecodeCompileService::default().compile(source).unwrap();
    let boundaries: Vec<&str> = module
        .top_level
        .instructions
        .iter()
        .chain(
            module
                .procedures
                .get(&fixture_id(&module.procedures, "::p"))
                .expect("authored fixture body")
                .instructions
                .iter(),
        )
        .filter(|instruction| instruction.op == tcl_bytecode::Op::START_CMD)
        .map(|instruction| {
            instruction
                .source_cmd_text
                .try_text()
                .expect("Unicode fixture source")
        })
        .collect();

    for expected in [
        "if {1} {set marker 2}",
        "foreach x {a} {if {1} {append marker $x}}",
        "if {1} {append marker $x}",
        "while {0} {}",
    ] {
        assert!(
            boundaries.contains(&expected),
            "missing boundary for {expected:?}: {boundaries:?}"
        );
    }
}

#[test]
fn final_constant_if_boundary_keeps_its_replay_continuation() {
    let module = BytecodeCompileService::default()
        .compile("pid; if {1} {set ::body_ran 1}")
        .unwrap();
    let boundary = module
        .top_level
        .instructions
        .iter()
        .find(|instruction| {
            instruction.op == tcl_bytecode::Op::START_CMD
                && instruction.source_cmd_text == "if {1} {set ::body_ran 1}"
        })
        .expect("constant if has a runtime boundary");
    let tcl_bytecode::Operand::Label(label) = &boundary.operands[0] else {
        panic!("START_CMD continuation is a label: {boundary:?}");
    };
    assert!(
        module.top_level.labels.contains_key(label),
        "missing {label:?}: {:?}",
        module.top_level.labels
    );
}

#[test]
fn proc_constant_if_boundary_has_one_tcl_shaped_owner_in_both_compile_paths() {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let service = BytecodeCompileService::for_profile(profile);
    let body = "pid; if {1} {set ::body_ran 1}";
    let if_source = "if {1} {set ::body_ran 1}";
    let static_module = service
        .compile_for_profile(&format!("proc p {{}} {{{body}}}"), profile)
        .unwrap();
    let parameters = Vec::new();
    let procedure_module = service
        .compile_procedure_for_profile(
            ProcedureCompileTarget {
                source: body,
                parameters: &parameters,
                namespace: "",
            },
            profile,
            ProcedureDispatch::Optimised,
        )
        .unwrap();

    let static_proc = static_module
        .procedures
        .get(&fixture_id(&static_module.procedures, "::p"))
        .expect("authored fixture body");
    let direct_proc = &procedure_module.top_level;
    for (path, procedure) in [("static", static_proc), ("direct", direct_proc)] {
        let owners: Vec<_> = procedure
            .instructions
            .iter()
            .filter(|instruction| {
                instruction.op == tcl_bytecode::Op::START_CMD
                    && instruction.source_cmd_text == if_source
            })
            .collect();
        assert_eq!(owners.len(), 1, "{path} proc owners: {owners:?}");
        let [
            tcl_bytecode::Operand::Label(end),
            tcl_bytecode::Operand::Imm(count),
        ] = owners[0].operands.as_slice()
        else {
            panic!("{path} proc owner has wrong operands: {:?}", owners[0]);
        };
        assert_eq!(*count, 2, "{path} proc owner must count if + body");
        assert!(
            procedure.labels.contains_key(end),
            "{path} proc owner has no replay continuation {end:?}"
        );
        assert!(
            procedure.instructions.iter().all(|instruction| {
                instruction.op != tcl_bytecode::Op::START_CMD
                    || instruction.source_cmd_text != "set ::body_ran 1"
            }),
            "{path} proc kept a nested marker under the owning if"
        );
    }

    let boundary_shape = |procedure: &tcl_bytecode::FunctionAsm| {
        procedure
            .instructions
            .iter()
            .filter(|instruction| instruction.op == tcl_bytecode::Op::START_CMD)
            .map(|instruction| {
                (
                    instruction.source_cmd_text.clone(),
                    instruction.operands.get(1).cloned(),
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(boundary_shape(static_proc), boundary_shape(direct_proc));
}

#[test]
fn proc_constant_if_owner_is_absent_without_an_earlier_generic_invoke() {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let service = BytecodeCompileService::for_profile(profile);
    let cases = [
        "if {1} {set ::body_ran 1}; mutate",
        "set local 0; if {1} {set ::body_ran 1}",
    ];

    for body in cases {
        let static_module = service
            .compile_for_profile(&format!("proc p {{}} {{{body}}}"), profile)
            .unwrap();
        let parameters = Vec::new();
        let direct_module = service
            .compile_procedure_for_profile(
                ProcedureCompileTarget {
                    source: body,
                    parameters: &parameters,
                    namespace: "",
                },
                profile,
                ProcedureDispatch::Optimised,
            )
            .unwrap();
        for procedure in [
            static_module
                .procedures
                .get(&fixture_id(&static_module.procedures, "::p"))
                .expect("authored fixture body"),
            &direct_module.top_level,
        ] {
            assert!(
                procedure.instructions.iter().all(|instruction| {
                    instruction.op != tcl_bytecode::Op::START_CMD
                        || instruction.source_cmd_text != "if {1} {set ::body_ran 1}"
                }),
                "unexpected owning if marker for {body:?}: {:?}",
                procedure.instructions
            );
        }
    }
}
