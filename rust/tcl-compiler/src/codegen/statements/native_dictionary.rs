// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original dictionary mutation instructions over actual compiled local slots.

use super::{CodegenCtx, NativeEmissionTask as Task};
use tcl_bytecode::{Op, Operand};
use tcl_registry::native_dictionary_compilation::{
    NativeDictionaryMutationInstruction, NativeDictionaryMutationKind as Kind,
};

impl CodegenCtx<'_> {
    pub(super) fn append_native_dictionary_mutation_tasks(
        &mut self,
        command: &tcl_lexer::NativeScriptCommandWords,
        recipe: NativeDictionaryMutationInstruction,
        _version: tcl_dialect::TclVersion,
        operations: &mut Vec<Task>,
    ) -> bool {
        let Some(slot) = self.command_variable_slot(&recipe.receiver) else {
            return false;
        };
        let Ok(slot) = i32::try_from(slot) else {
            return false;
        };
        let operand_count = recipe.operands.len();
        for operand in recipe.operands {
            let Some(task) = Self::native_namespace_word_task(&command.words, operand) else {
                return false;
            };
            operations.push(task);
        }
        let Ok(keys) = i32::try_from(recipe.key_count) else {
            return false;
        };
        let (opcode, immediates) = match recipe.kind {
            Kind::Set => (Op::DICT_SET, vec![Operand::Imm(keys), Operand::Imm(slot)]),
            Kind::Unset => (Op::DICT_UNSET, vec![Operand::Imm(keys), Operand::Imm(slot)]),
            Kind::Incr(amount) => (
                Op::DICT_INCR_IMM,
                vec![Operand::Imm(amount), Operand::Imm(slot)],
            ),
            Kind::Append => {
                if operand_count > 2 {
                    let Ok(values) = i32::try_from(operand_count - 1) else {
                        return false;
                    };
                    operations.push(Task::Operation(Op::STR_CONCAT1, vec![Operand::Imm(values)]));
                }
                (Op::DICT_APPEND, vec![Operand::Imm(slot)])
            }
            Kind::Lappend => (Op::DICT_LAPPEND, vec![Operand::Imm(slot)]),
        };
        operations.push(Task::Operation(opcode, immediates));
        true
    }
}
