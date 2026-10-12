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

//! Original namespace/frame operands and physical introspection operations.

use super::{CodegenCtx, NativeEmissionTask};
use tcl_bytecode::{Op, Operand};
use tcl_registry::native_introspection_compilation::{
    NativeIntrospectionInstruction, NativeIntrospectionKind as Kind,
};

#[cfg(test)]
#[path = "native_introspection_tests.rs"]
mod tests;

impl CodegenCtx<'_> {
    pub(super) fn append_native_introspection_tasks(
        &mut self,
        command: &tcl_lexer::NativeScriptCommandWords,
        recipe: NativeIntrospectionInstruction,
        version: tcl_dialect::TclVersion,
        operations: &mut Vec<NativeEmissionTask>,
    ) -> bool {
        use NativeEmissionTask as Task;
        if recipe.kind == Kind::NamespaceCode {
            operations.push(Task::Literal(b"::namespace".to_vec()));
            operations.push(Task::Literal(b"inscope".to_vec()));
            operations.push(Task::SwitchOperation(
                Op::CURRENT_NAMESPACE,
                Vec::new(),
                version,
            ));
        }
        let has_number = !recipe.operands.is_empty();
        for operand in recipe.operands {
            let Some(task) = Self::native_namespace_word_task(&command.words, operand) else {
                return false;
            };
            operations.push(task);
        }
        if recipe.kind == Kind::InfoCommands {
            let empty_result = self.fresh_label("native_info_commands_empty");
            operations.extend([
                Task::SwitchOperation(Op::RESOLVE_CMD, Vec::new(), version),
                Task::Operation(Op::DUP, Vec::new()),
                Task::SwitchOperation(Op::STR_LEN, Vec::new(), version),
                Task::Operation(Op::JUMP_FALSE1, vec![Operand::Label(empty_result.clone())]),
                Task::SwitchOperation(Op::LIST, vec![Operand::Imm(1)], version),
                Task::Label(empty_result),
            ]);
            return true;
        }
        let (op, operands) = match recipe.kind {
            Kind::NamespaceCurrent => (Op::CURRENT_NAMESPACE, Vec::new()),
            Kind::NamespaceOrigin => (Op::ORIGIN_CMD, Vec::new()),
            Kind::NamespaceCode => (Op::LIST, vec![Operand::Imm(4)]),
            Kind::InfoLevel => (
                if has_number {
                    Op::INFO_LEVEL_ARGS
                } else {
                    Op::INFO_LEVEL_NUM
                },
                Vec::new(),
            ),
            Kind::InfoCommands => unreachable!("original command result handled above"),
        };
        operations.push(Task::SwitchOperation(op, operands, version));
        true
    }
}
