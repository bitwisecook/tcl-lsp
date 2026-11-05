// SPDX-License-Identifier: AGPL-3.0-or-later
//! Sequential original unset operands and their selected native receiver opcodes.

use super::{CodegenCtx, NativeEmissionTask as Task, Op, Operand};
use tcl_registry::native_unset_compilation::{NativeUnsetInstruction, NativeUnsetReceiver};
use tcl_syntax::native_variable_words::NativeVariableWordOperand;

#[cfg(test)]
#[path = "native_unset_tests.rs"]
mod tests;

impl CodegenCtx<'_> {
    pub(super) fn native_unset_tasks(
        &mut self,
        command: &tcl_lexer::NativeScriptCommandWords,
        recipe: NativeUnsetInstruction,
    ) -> Option<Vec<Task>> {
        let mut tasks = Vec::new();
        for variable in recipe.variables {
            let (slot, array) = match variable.receiver {
                NativeUnsetReceiver::Original(NativeVariableWordOperand::Literal {
                    name,
                    index,
                    ..
                })
                | NativeUnsetReceiver::ExpandedLiteral { name, index } => {
                    let slot = self.command_variable_slot(&name);
                    if slot.is_none() {
                        tasks.push(Task::Literal(name));
                    }
                    let array = index.is_some();
                    if let Some(index) = index {
                        tasks.push(Task::Literal(index));
                    }
                    (slot, array)
                }
                NativeUnsetReceiver::Original(NativeVariableWordOperand::CompoundArray {
                    name,
                    index,
                    ..
                }) => {
                    let slot = self.command_variable_slot(&name);
                    if slot.is_none() {
                        tasks.push(Task::Literal(name));
                    }
                    let index = std::rc::Rc::new(index);
                    let root = index.root();
                    tasks.push(Task::List(index, root, 0));
                    (slot, true)
                }
                NativeUnsetReceiver::Original(NativeVariableWordOperand::DynamicWord) => {
                    tasks.push(Self::native_namespace_word_task(
                        &command.words,
                        variable.operand,
                    )?);
                    (None, false)
                }
            };
            let mut operands = vec![Operand::Imm(i32::from(recipe.complain))];
            let opcode = match (slot, array) {
                (Some(slot), false) => {
                    operands.push(Operand::Imm(tcl_bytecode::bytecode_imm(slot)));
                    Op::UNSET_SCALAR
                }
                (Some(slot), true) => {
                    operands.push(Operand::Imm(tcl_bytecode::bytecode_imm(slot)));
                    Op::UNSET_ARRAY
                }
                (None, false) => Op::UNSET_STK,
                (None, true) => Op::UNSET_ARRAY_STK,
            };
            tasks.push(Task::Operation(opcode, operands));
        }
        tasks.push(Task::Literal(Vec::new()));
        Some(tasks)
    }
}
