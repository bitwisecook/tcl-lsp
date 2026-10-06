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

//! Original scalar operand preparation followed by genuine stack instructions.

use super::{CodegenCtx, NativeEmissionTask};
use tcl_bytecode::Op;
use tcl_registry::native_scalar_compilation::{NativeScalarInstruction, NativeScalarOperation};

impl CodegenCtx<'_> {
    pub(super) fn append_native_scalar_tasks(
        command: &tcl_lexer::NativeScriptCommandWords,
        recipe: NativeScalarInstruction,
        version: tcl_dialect::TclVersion,
        operations: &mut Vec<NativeEmissionTask>,
    ) -> bool {
        if recipe.operands.len() != recipe.operation.arity() {
            return false;
        }
        for operand in recipe.operands {
            let Some(task) = Self::native_namespace_word_task(&command.words, operand) else {
                return false;
            };
            operations.push(task);
        }
        let op = match recipe.operation {
            NativeScalarOperation::StringEqual => Op::STR_EQ,
            NativeScalarOperation::StringLength => Op::STR_LEN,
            NativeScalarOperation::ListLength => Op::LIST_LENGTH,
        };
        operations.push(NativeEmissionTask::SwitchOperation(op, Vec::new(), version));
        true
    }
}
