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

//! The release-unit metadata codegen stamps for a release that keeps the
//! commands running bytecode compiled (Tcl 8.4): how it binds each invoked
//! command, what each command's specialised instructions may keep across a
//! command-table change, and where each script body starts.

use tcl_bytecode::{FunctionAsm, Instruction, Op};
use tcl_codegen::compile_service::BytecodeCompileService;
use tcl_runtime_api::{CompileService, ReleaseBinding};

fn procedure(environment: &str, source: &str, name: &str) -> FunctionAsm {
    let profile = tcl_registry::model::ingress::resolve_environment(environment).analyser_profile();
    let module = BytecodeCompileService::for_profile(profile)
        .compile_for_profile(source, profile)
        .expect("compiles");
    named_procedure(&module.procedures, name).clone()
}

/// The one procedure assembly emitted under `name`; the module keys them by
/// body identity, not by name.
fn named_procedure<'a>(
    procedures: &'a std::collections::HashMap<tcl_runtime_api::ProcedureId, FunctionAsm>,
    name: &str,
) -> &'a FunctionAsm {
    let mut matching = procedures.values().filter(|asm| asm.name == name);
    let asm = matching
        .next()
        .unwrap_or_else(|| panic!("a procedure named {name:?}"));
    assert!(matching.next().is_none(), "ambiguous procedure {name:?}");
    asm
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

const NESTED: &str = "proc p {c} {if \"$c\" {foreach i {1 2} {puts [expr {1+1}]}}; \
                      puts [string length $i]; catch {puts b} m}";

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
