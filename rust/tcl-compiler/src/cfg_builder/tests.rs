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

use crate::interprocedural::test_body_identity::{fixture_id, fixture_id_if_present};

use super::*;

mod callee_frame_effects;
mod command_classes;
mod embedded_substitutions;
mod flow_facts;
mod opaque_constructs;
mod prepared_context;
mod script_lowering;
mod unseen_code;
mod upvar_invalidation;

fn summary_id(module: &Module, label: &str) -> crate::ir::ProcedureId {
    let matching: Vec<_> = module
        .original_source_procedure_bodies()
        .filter(|(_, procedure)| {
            procedure.qualified_name == label
                || procedure.name == label
                || tcl_syntax::naming::key_tail(&procedure.name) == label
        })
        .map(|(id, _)| *id)
        .collect();
    assert_eq!(
        matching.len(),
        1,
        "authored fixture summary {label:?} must identify one body"
    );
    matching[0]
}

use crate::ir::{ForeachIterator, IfClause, Script, SwitchArm, SwitchMode};
use tcl_lexer::Span;

fn native_registry() -> CommandRegistry {
    CommandRegistry::build_default().project_for_profile(
        tcl_registry::model::ingress::resolve_environment("tcl9.1").analyser_profile(),
    )
}

fn build_test_cfg_function(name: &str, script: &Script, inline_loops: bool) -> Function {
    let registry = native_registry();
    CfgBuilder::new(inline_loops, &registry)
        .with_lexer_config(tcl_lexer::LexerConfig::for_profile(registry.profile()))
        .with_faithful_exceptions()
        .with_invocation_namespace(crate::ir::ExecutionNamespace::exact(command_namespace(
            name,
        )))
        .build_function(name, script)
}

// Uplevel-literal-set wiring tests.
//
// Each test drives the full pipeline:
// `lower_to_ir` → `build_cfg` (which calls `prepare_cfg_context`).
// The assertions inspect the resulting CFG to confirm that calls
// to upvar-using procs carry the expected caller-side defs.

fn lower_module(src: &str) -> Module {
    crate::lowering::lower_to_ir(src, &native_registry())
}

fn find_call_defs<'a>(func: &'a Function, command: &str) -> Option<&'a [String]> {
    for block in func.blocks.values() {
        for stmt in &block.statements {
            if let Statement::Call {
                command: c, defs, ..
            } = stmt
                && c == command
            {
                return Some(defs);
            }
        }
    }
    None
}

// Embedded-substitution form: `[upvar_proc arg]` in every evaluated
// statement value, argument, expression, return, or condition surface.

/// Walk every block looking for a Call whose `defs` contain
/// *def*.  Returns the command name when found.
fn find_call_with_def<'a>(func: &'a Function, def: &str) -> Option<&'a str> {
    for block in func.blocks.values() {
        for stmt in &block.statements {
            if let Statement::Call { command, defs, .. } = stmt
                && defs.iter().any(|d| d == def)
            {
                return Some(command);
            }
        }
    }
    None
}

/// The `(command, defs)` of every call in `func`'s blocks, in order.
fn calls_in_order(func: &Function) -> Vec<(&str, &[String])> {
    func.blocks
        .values()
        .flat_map(|block| &block.statements)
        .filter_map(|stmt| match stmt {
            Statement::Call { command, defs, .. } => Some((command.as_str(), defs.as_slice())),
            _ => None,
        })
        .collect()
}

fn position_of(calls: &[(&str, &[String])], command: &str) -> usize {
    calls
        .iter()
        .position(|(name, _)| *name == command)
        .unwrap_or_else(|| panic!("no {command} in {calls:?}"))
}
