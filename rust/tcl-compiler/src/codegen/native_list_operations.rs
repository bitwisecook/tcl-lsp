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

//! Original list operation stack order and physical compiler coordinates.

use super::{CodegenCtx, NativeEmissionTask as Task, Op, Operand};
use tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand;
use tcl_registry::native_list_operations_compilation::{
    NativeListOperationInstruction as Plan, NativeListVariableOperand,
};
use tcl_syntax::native_compiled_index::{
    NativeCompiledListIndex as Index, NativeCompiledListRange,
};

fn operation(op: Op, values: &[i32]) -> Task {
    Task::Operation(op, values.iter().copied().map(Operand::Imm).collect())
}
fn range(first: i32, last: i32) -> Task {
    Task::NativeListRange(NativeCompiledListRange {
        first: Index::from_encoded(first),
        last: Index::from_encoded(last),
    })
}
fn variable_access(slot: Option<usize>, array: bool, store: bool) -> Task {
    let op = match (slot, array, store) {
        (Some(slot), false, false) if slot < 256 => Op::LOAD_SCALAR1,
        (Some(_), false, false) => Op::LOAD_SCALAR4,
        (Some(slot), true, false) if slot < 256 => Op::LOAD_ARRAY1,
        (Some(_), true, false) => Op::LOAD_ARRAY4,
        (Some(slot), false, true) if slot < 256 => Op::STORE_SCALAR1,
        (Some(_), false, true) => Op::STORE_SCALAR4,
        (Some(slot), true, true) if slot < 256 => Op::STORE_ARRAY1,
        (Some(_), true, true) => Op::STORE_ARRAY4,
        (None, false, false) => Op::LOAD_STK,
        (None, true, false) => Op::LOAD_ARRAY_STK,
        (None, false, true) => Op::STORE_STK,
        (None, true, true) => Op::STORE_ARRAY_STK,
    };
    operation(
        op,
        &slot
            .into_iter()
            .map(super::super::bytecode_imm)
            .collect::<Vec<_>>(),
    )
}

impl CodegenCtx<'_> {
    pub(super) fn append_native_list_operation_tasks(
        &mut self,
        command: &tcl_lexer::NativeScriptCommandWords,
        recipe: Plan,
        operations: &mut Vec<Task>,
    ) -> bool {
        match recipe {
            Plan::Range { list, first, last } => {
                if !Self::list_operand(command, list, operations) {
                    return false;
                }
                operations.push(Task::NativeListRange(NativeCompiledListRange {
                    first,
                    last,
                }));
            }
            Plan::Assign { list, targets } => {
                if !Self::list_operand(command, list, operations) {
                    return false;
                }
                for (index, target) in targets.iter().cloned().enumerate() {
                    let Some((slot, array)) = self.list_target(command, target, operations) else {
                        return false;
                    };
                    let depth = usize::from(slot.is_none()) + usize::from(array);
                    operations.push(if depth == 0 {
                        operation(Op::DUP, &[])
                    } else {
                        operation(Op::OVER, &[super::super::bytecode_imm(depth)])
                    });
                    operations.push(Task::NativeListIndex(Index::from_encoded(
                        super::super::bytecode_imm(index),
                    )));
                    operations.push(variable_access(slot, array, true));
                    operations.push(operation(Op::POP, &[]));
                }
                operations.push(range(super::super::bytecode_imm(targets.len()), -2));
            }
            Plan::Insert { .. } | Plan::Set { .. } => return false,
        }
        true
    }

    fn list_operand(
        command: &tcl_lexer::NativeScriptCommandWords,
        operand: NativeCompilerWordOperand,
        operations: &mut Vec<Task>,
    ) -> bool {
        let Some(task) = Self::native_namespace_word_task(&command.words, operand) else {
            return false;
        };
        operations.push(task);
        true
    }
    fn list_target(
        &mut self,
        command: &tcl_lexer::NativeScriptCommandWords,
        target: NativeListVariableOperand,
        operations: &mut Vec<Task>,
    ) -> Option<(Option<usize>, bool)> {
        Some(match target {
            NativeListVariableOperand::Original {
                operand: NativeCompilerWordOperand::Original(index),
                variable,
            } => self.prepare_native_variable_tasks(
                command.words.get(index)?,
                variable,
                false,
                operations,
            ),
            NativeListVariableOperand::ExpandedLiteral { name, index, .. } => {
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
            NativeListVariableOperand::Original { .. } => return None,
        })
    }
}
