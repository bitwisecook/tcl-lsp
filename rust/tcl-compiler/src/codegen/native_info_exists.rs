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

//! Existence uses the shared original native variable operand preparation.

use super::{CodegenCtx, NativeEmissionTask as Task, Op, Operand};
use tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand;
use tcl_registry::native_info_exists_compilation::{
    NativeInfoExistsInstruction, NativeInfoExistsReceiver,
};

impl CodegenCtx<'_> {
    pub(super) fn append_native_info_exists_tasks(
        &mut self,
        command: &tcl_lexer::NativeScriptCommandWords,
        recipe: NativeInfoExistsInstruction,
        operations: &mut Vec<Task>,
    ) -> bool {
        let (slot, array) = match recipe.receiver {
            NativeInfoExistsReceiver::Original(target) => {
                let NativeCompilerWordOperand::Original(index) = recipe.operand else {
                    return false;
                };
                let Some(word) = command.words.get(index) else {
                    return false;
                };
                self.prepare_native_variable_tasks(word, target, false, operations)
            }
            NativeInfoExistsReceiver::ExpandedLiteral { name, index } => {
                let slot = self.command_variable_slot(&name);
                if slot.is_none() {
                    operations.push(Task::Literal(name));
                }
                let array = index.is_some();
                if let Some(index) = index {
                    operations.push(Task::Literal(index));
                }
                (slot, array)
            }
        };
        let op = match (slot.is_some(), array) {
            (true, false) => Op::EXIST_SCALAR,
            (true, true) => Op::EXIST_ARRAY,
            (false, false) => Op::EXIST_STK,
            (false, true) => Op::EXIST_ARRAY_STK,
        };
        let operands = slot
            .into_iter()
            .map(|slot| Operand::Imm(super::super::bytecode_imm(slot)))
            .collect();
        operations.push(Task::Operation(op, operands));
        true
    }
}
