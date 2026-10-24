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
use tcl_bytecode::{Op, Operand};
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
            NativeScalarOperation::StringIsAscii => Op::STR_CLASS,
            NativeScalarOperation::ListLength => Op::LIST_LENGTH,
        };
        let operands = if recipe.operation == NativeScalarOperation::StringIsAscii {
            // STR_CLASS_ASCII in the C8.6/9.0/9.1 instruction enum.
            vec![Operand::Imm(2)]
        } else {
            Vec::new()
        };
        operations.push(NativeEmissionTask::SwitchOperation(op, operands, version));
        true
    }
}

impl CodegenCtx<'_> {
    pub(super) fn append_native_string_operand_tasks(
        command: &tcl_lexer::NativeScriptCommandWords,
        recipe: tcl_registry::native_string_operand_compilation::NativeStringOperandInstruction,
        version: tcl_dialect::TclVersion,
        operations: &mut Vec<NativeEmissionTask>,
    ) -> bool {
        use tcl_registry::native_string_operand_compilation::NativeStringOperandOperation as Operation;
        if !recipe.operation.accepts(recipe.operands.len())
            || recipe.literals.len() != recipe.operands.len()
        {
            return false;
        }
        if recipe.operation == Operation::Cat {
            return Self::append_native_string_cat_tasks(command, recipe, version, operations);
        }
        for operand in recipe.operands {
            let Some(task) = Self::native_namespace_word_task(&command.words, operand) else {
                return false;
            };
            operations.push(task);
        }
        let op = match recipe.operation {
            Operation::Compare => Op::STR_CMP,
            Operation::Index => Op::STR_INDEX,
            Operation::First => Op::STR_FIND,
            Operation::Last => Op::STR_RFIND,
            Operation::ToUpper => Op::STR_UPPER,
            Operation::ToLower => Op::STR_LOWER,
            Operation::ToTitle => Op::STR_TITLE,
            Operation::Cat => return false,
        };
        operations.push(NativeEmissionTask::SwitchOperation(op, Vec::new(), version));
        true
    }

    /// `TclCompileStringCatCmd`: no words push `""`; each run of compile-known
    /// words is folded into one literal; every other word is evaluated in
    /// place; `strcat` joins them, flushing at 254 operands so the folded
    /// literal still fits one instruction's count.
    fn append_native_string_cat_tasks(
        command: &tcl_lexer::NativeScriptCommandWords,
        recipe: tcl_registry::native_string_operand_compilation::NativeStringOperandInstruction,
        version: tcl_dialect::TclVersion,
        operations: &mut Vec<NativeEmissionTask>,
    ) -> bool {
        use crate::codegen::Operand;
        let concat = |count: usize| {
            NativeEmissionTask::SwitchOperation(
                Op::STR_CONCAT1,
                vec![Operand::Imm(
                    i32::try_from(count).expect("at most 254 operands"),
                )],
                version,
            )
        };
        if recipe.operands.is_empty() {
            operations.push(NativeEmissionTask::Literal(Vec::new()));
            return true;
        }
        let mut pushed = 0usize;
        let mut folded: Option<Vec<u8>> = None;
        for (operand, literal) in recipe.operands.into_iter().zip(recipe.literals) {
            if let Some(value) = literal {
                folded
                    .get_or_insert_with(Vec::new)
                    .extend_from_slice(&value);
                continue;
            }
            if let Some(value) = folded.take() {
                operations.push(NativeEmissionTask::Literal(value));
                pushed += 1;
            }
            let Some(task) = Self::native_namespace_word_task(&command.words, operand) else {
                return false;
            };
            operations.push(task);
            pushed += 1;
            if pushed >= 254 {
                operations.push(concat(pushed));
                pushed = 1;
            }
        }
        if let Some(value) = folded {
            operations.push(NativeEmissionTask::Literal(value));
            pushed += 1;
        }
        if pushed > 1 {
            operations.push(concat(pushed));
        }
        true
    }
}

impl CodegenCtx<'_> {
    pub(super) fn append_native_string_range_tasks(
        command: &tcl_lexer::NativeScriptCommandWords,
        recipe: tcl_registry::native_string_range_compilation::NativeStringRangeInstruction,
        version: tcl_dialect::TclVersion,
        operations: &mut Vec<NativeEmissionTask>,
    ) -> bool {
        use tcl_registry::native_string_range_compilation::NativeStringRangeOperation;
        let Some(task) = Self::native_namespace_word_task(&command.words, recipe.operand) else {
            return false;
        };
        operations.push(task);
        match recipe.operation {
            NativeStringRangeOperation::Empty => {
                operations.push(NativeEmissionTask::SwitchOperation(
                    Op::POP,
                    Vec::new(),
                    version,
                ));
                operations.push(NativeEmissionTask::Literal(Vec::new()));
            }
            NativeStringRangeOperation::Immediate(coordinates) => {
                operations.push(NativeEmissionTask::SwitchOperation(
                    Op::STR_RANGE_IMM,
                    vec![
                        Operand::Imm(coordinates.first.encoded()),
                        Operand::Imm(coordinates.last.encoded()),
                    ],
                    version,
                ));
            }
        }
        true
    }
}
