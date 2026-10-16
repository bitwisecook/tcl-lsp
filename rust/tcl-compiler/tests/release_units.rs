// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The release-unit metadata codegen stamps for a release that keeps the
//! commands running bytecode compiled (Tcl 8.4): how it binds each invoked
//! command, what each command's specialised instructions may keep across a
//! command-table change, and where each script body starts.

use tcl_bytecode::{FunctionAsm, Instruction, Op, ReleaseBodyEntry};
use tcl_compiler::compile_service::BytecodeCompileService;
use tcl_runtime_api::{CommandEvaluation, CompileService, ReleaseBinding, ScriptCompileTarget};

fn procedure(environment: &str, source: &str, name: &str) -> FunctionAsm {
    let profile = tcl_registry::model::ingress::resolve_environment(environment).analyser_profile();
    let module = BytecodeCompileService::for_profile(profile)
        .compile_for_profile(source, profile)
        .expect("compiles");
    module.procedures[name].clone()
}

fn invoke_of<'a>(asm: &'a FunctionAsm, text: &str, comment: &str) -> &'a Instruction {
    asm.instructions
        .iter()
        .find(|instruction| {
            matches!(instruction.op, Op::INVOKE_STK1 | Op::INVOKE_STK4)
                && instruction.source_cmd_text == text
                && instruction.comment == comment
        })
        .unwrap_or_else(|| panic!("an invoke of {comment:?} in {text:?}"))
}

fn boundary_of<'a>(asm: &'a FunctionAsm, text: &str) -> &'a Instruction {
    asm.instructions
        .iter()
        .find(|instruction| {
            instruction.source_command_boundary.is_start() && instruction.source_cmd_text == text
        })
        .unwrap_or_else(|| panic!("a boundary of {text:?}"))
}

fn body_entries(asm: &FunctionAsm) -> Vec<(u32, ReleaseBodyEntry)> {
    asm.instructions
        .iter()
        .filter_map(|instruction| {
            instruction
                .release_body_entry
                .map(|entry| (instruction.release_body, entry))
        })
        .collect()
}

const NESTED: &str = "proc p {c} {if \"$c\" {foreach i {1 2} {puts [expr {1+1}]}}; \
                      puts [string length $i]; catch {puts b} m}";

#[test]
fn eight_four_stamps_how_it_binds_each_command_and_where_bodies_start() {
    let asm = procedure("tcl8.4", NESTED, "::p");
    let text = "puts [expr {1+1}]";
    assert_eq!(
        invoke_of(&asm, text, "").release_binding,
        Some(ReleaseBinding::Compiled),
        "the substituted `expr`"
    );
    assert_eq!(
        invoke_of(&asm, text, "puts").release_binding,
        Some(ReleaseBinding::Invoked)
    );
    assert_eq!(
        boundary_of(&asm, "catch {puts b} m").release_hold,
        Some(ReleaseBinding::CompiledInProcedure),
        "a `catch` with a result variable"
    );
    assert_eq!(
        boundary_of(&asm, "puts [string length $i]").release_hold,
        Some(ReleaseBinding::Compiled),
        "no command of it is specialised"
    );

    // The body of the `if` 8.4 runs (its test is substituted) encloses the
    // `foreach` body; whether either is a unit of its own is the activation's
    // to decide.
    let entries = body_entries(&asm);
    let arm = entries
        .iter()
        .find(|(_, entry)| entry.owner == ReleaseBinding::Invoked)
        .expect("the if body starts");
    assert_eq!(arm.1.parent, 0);
    let loop_body = entries
        .iter()
        .find(|(_, entry)| entry.owner == ReleaseBinding::CompiledInProcedure)
        .expect("the foreach body starts");
    assert_eq!(loop_body.1.parent, arm.0);
    assert_eq!(boundary_of(&asm, text).release_body, loop_body.0);
}

#[test]
fn eight_four_evaluates_an_eval_body_directly() {
    let source = "proc p {} {eval {puts [expr {1+1}]}}";
    let text = "puts [expr {1+1}]";
    let v84 = procedure("tcl8.4", source, "::p");
    let invoke = invoke_of(&v84, text, "");
    assert_eq!(invoke.evaluation, CommandEvaluation::Direct);
    assert_eq!(invoke.release_binding, None, "looked up as it runs");
    let v90 = procedure("tcl9.0", source, "::p");
    assert_eq!(
        invoke_of(&v90, text, "").evaluation,
        CommandEvaluation::Compiled
    );
}

#[test]
fn a_release_that_revalidates_running_bytecode_gets_no_release_units() {
    let asm = procedure("tcl9.0", NESTED, "::p");
    assert!(asm.instructions.iter().all(|instruction| {
        instruction.release_binding.is_none()
            && instruction.release_hold.is_none()
            && instruction.release_body == 0
            && instruction.release_body_entry.is_none()
    }));
}

#[test]
fn eight_four_keeps_a_return_only_outside_a_catch_body() {
    // A `catch` body holding a `return` runs through the `catch` builtin,
    // whose activation the VM marks as a catch body.
    let asm = procedure(
        "tcl8.4",
        "proc p {} {catch {if {1} {return x}}; return y}",
        "::p",
    );
    assert_eq!(
        invoke_of(&asm, "catch {if {1} {return x}}", "catch").release_binding,
        Some(ReleaseBinding::Compiled)
    );
    assert_eq!(
        boundary_of(&asm, "return y").release_hold,
        Some(ReleaseBinding::CompiledInProcedureOutsideCatch)
    );
}

#[test]
fn a_directly_evaluated_script_marks_its_own_commands() {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let source = "set x 1\nif {$x} {puts a}\nproc p {} {puts b}\n";
    let compile = |evaluation| {
        BytecodeCompileService::for_profile(profile)
            .compile_script_for_profile(
                ScriptCompileTarget {
                    source,
                    namespace: "",
                    evaluation,
                },
                profile,
            )
            .expect("compiles")
    };
    let direct = compile(CommandEvaluation::Direct);
    let marked: Vec<&str> = direct
        .top_level
        .instructions
        .iter()
        .filter(|instruction| instruction.script_command)
        .map(|instruction| instruction.source_cmd_text.as_str())
        .collect();
    assert_eq!(
        marked,
        ["set x 1", "if {$x} {puts a}", "proc p {} {puts b}"]
    );
    assert!(
        direct.procedures["::p"]
            .instructions
            .iter()
            .all(|instruction| !instruction.script_command),
        "a procedure body is not the script's"
    );
    let compiled = compile(CommandEvaluation::Compiled);
    assert!(
        compiled
            .top_level
            .instructions
            .iter()
            .all(|instruction| !instruction.script_command)
    );
}
