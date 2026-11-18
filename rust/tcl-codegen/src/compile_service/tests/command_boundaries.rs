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
