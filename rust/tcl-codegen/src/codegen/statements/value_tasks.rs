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

//! Task sequences of compilers that build a value: `string is`, `dict create`, `const`, `format` and concatenation.

use super::super::{CodegenCtx, Op, Operand};
use super::{NativeEmissionTask, is_jump, is_op};

impl CodegenCtx<'_> {
    /// `TclCompileStringIsCmd` (C8.6 to C9.1): the subject, then the class
    /// branch's own test sequence. C9.1 tests emptiness with `isEmpty` and
    /// drops the first of two objects with `swap`.
    fn append_native_string_is_tasks(
        &mut self,
        command: &tcl_lexer::NativeScriptCommandWords,
        recipe: tcl_registry::native_string_is_compilation::NativeStringIsInstruction,
        version: tcl_dialect::TclVersion,
        operations: &mut Vec<NativeEmissionTask>,
    ) -> bool {
        use tcl_registry::native_string_is_compilation::NativeStringIsClass as Class;
        let Some(subject) = Self::native_namespace_word_task(&command.words, recipe.subject) else {
            return false;
        };
        operations.push(subject);
        let strict = recipe.strict;
        match recipe.class {
            Class::Character(class) => {
                self.append_string_is_character_tasks(class, strict, version, operations);
            }
            Class::Boolean | Class::True | Class::False => {
                self.append_string_is_boolean_tasks(recipe.class, strict, version, operations);
            }
            Class::Double => self.append_string_is_double_tasks(strict, version, operations),
            Class::Integer | Class::Wide | Class::Entier => {
                self.append_string_is_integer_tasks(recipe.class, strict, version, operations);
            }
            Class::Dict | Class::List => {
                let handler = self.fresh_label("is_catch");
                operations.extend([
                    NativeEmissionTask::BeginNativeCatch(handler.clone()),
                    is_op(Op::DUP),
                ]);
                if recipe.class == Class::Dict {
                    operations.push(is_op(Op::VERIFY_DICT));
                } else {
                    operations.extend([is_op(Op::LIST_LENGTH), is_op(Op::POP)]);
                }
                operations.extend([
                    NativeEmissionTask::Label(handler),
                    is_op(Op::POP),
                    is_op(Op::PUSH_RETURN_CODE),
                    NativeEmissionTask::EndNativeCatch,
                    is_op(Op::LNOT),
                ]);
            }
        }
        true
    }

    fn append_string_is_character_tasks(
        &mut self,
        class: u8,
        strict: bool,
        version: tcl_dialect::TclVersion,
        operations: &mut Vec<NativeEmissionTask>,
    ) {
        use NativeEmissionTask as Task;
        let test = if class == 2 {
            Task::SwitchOperation(Op::STR_CLASS, vec![Operand::Imm(2)], version)
        } else {
            Task::Operation(Op::STR_CLASS, vec![Operand::Imm(i32::from(class))])
        };
        if !strict {
            operations.push(test);
            return;
        }
        let (over, over2) = (self.fresh_label("is_over"), self.fresh_label("is_over2"));
        operations.extend([
            is_op(Op::DUP),
            test,
            is_jump(Op::JUMP_TRUE1, &over),
            is_op(Op::POP),
            Task::Literal(b"0".to_vec()),
            is_jump(Op::JUMP1, &over2),
            Task::Label(over),
        ]);
        operations.extend(Self::string_is_empty_tasks(version, true));
        operations.push(Task::Label(over2));
    }

    /// C8.6/C9.0 compare against "" where C9.1 emits `isEmpty`.
    fn string_is_empty_tasks(
        version: tcl_dialect::TclVersion,
        negate: bool,
    ) -> Vec<NativeEmissionTask> {
        if version >= tcl_dialect::TclVersion::V9_1 {
            let mut tasks = vec![is_op(Op::IS_EMPTY)];
            if negate {
                tasks.push(is_op(Op::LNOT));
            }
            tasks
        } else {
            vec![
                NativeEmissionTask::Literal(Vec::new()),
                is_op(if negate { Op::STR_NEQ } else { Op::STR_EQ }),
            ]
        }
    }

    /// C8.6/C9.0 `reverse 2` where C9.1 emits `swap`.
    fn string_is_swap_task(version: tcl_dialect::TclVersion) -> NativeEmissionTask {
        if version >= tcl_dialect::TclVersion::V9_1 {
            is_op(Op::SWAP)
        } else {
            NativeEmissionTask::Operation(Op::REVERSE, vec![Operand::Imm(2)])
        }
    }

    fn append_string_is_boolean_tasks(
        &mut self,
        class: tcl_registry::native_string_is_compilation::NativeStringIsClass,
        strict: bool,
        version: tcl_dialect::TclVersion,
        operations: &mut Vec<NativeEmissionTask>,
    ) {
        use NativeEmissionTask as Task;
        use tcl_registry::native_string_is_compilation::NativeStringIsClass as Class;
        operations.push(is_op(Op::TRY_CVT_TO_BOOLEAN));
        if class == Class::Boolean {
            if strict {
                operations.extend([Self::string_is_swap_task(version), is_op(Op::POP)]);
            } else {
                let (over, over2) = (self.fresh_label("is_over"), self.fresh_label("is_over2"));
                operations.push(is_jump(Op::JUMP_TRUE1, &over));
                operations.extend(Self::string_is_empty_tasks(version, false));
                operations.extend([
                    is_jump(Op::JUMP1, &over2),
                    Task::Label(over),
                    is_op(Op::POP),
                    Task::Literal(b"1".to_vec()),
                    Task::Label(over2),
                ]);
            }
            return;
        }
        let over = self.fresh_label("is_over");
        let truth = class == Class::True;
        operations.push(is_jump(Op::JUMP_TRUE1, &over));
        if version >= tcl_dialect::TclVersion::V9_1 {
            let over2 = self.fresh_label("is_over2");
            if strict {
                operations.extend([is_op(Op::POP), Task::Literal(b"0".to_vec())]);
            } else {
                operations.push(is_op(Op::IS_EMPTY));
            }
            operations.extend([
                is_jump(Op::JUMP1, &over2),
                Task::Label(over),
                is_op(Op::LNOT),
            ]);
            if truth {
                operations.push(is_op(Op::LNOT));
            }
            operations.push(Task::Label(over2));
            return;
        }
        if strict {
            operations.extend([
                is_op(Op::POP),
                Task::Literal(if truth { b"0" } else { b"1" }.to_vec()),
            ]);
        } else {
            operations.extend([
                Task::Literal(Vec::new()),
                is_op(if truth { Op::STR_EQ } else { Op::STR_NEQ }),
            ]);
        }
        operations.extend([Task::Label(over), is_op(Op::LNOT)]);
        if truth {
            operations.push(is_op(Op::LNOT));
        }
    }

    fn append_string_is_double_tasks(
        &mut self,
        strict: bool,
        version: tcl_dialect::TclVersion,
        operations: &mut Vec<NativeEmissionTask>,
    ) {
        use NativeEmissionTask as Task;
        let (satisfied, end) = (self.fresh_label("is_satisfied"), self.fresh_label("is_end"));
        if strict {
            operations.extend([
                is_op(Op::NUMERIC_TYPE),
                is_jump(Op::JUMP_TRUE1, &satisfied),
                Task::Literal(b"0".to_vec()),
                is_jump(Op::JUMP1, &end),
                Task::Label(satisfied),
            ]);
        } else {
            let empty = self.fresh_label("is_empty");
            operations.push(is_op(Op::DUP));
            operations.extend(Self::string_is_empty_tasks(version, false));
            operations.extend([
                is_jump(Op::JUMP_TRUE1, &empty),
                is_op(Op::NUMERIC_TYPE),
                is_jump(Op::JUMP_TRUE1, &satisfied),
                Task::Literal(b"0".to_vec()),
                is_jump(Op::JUMP1, &end),
                Task::Label(empty),
                is_op(Op::POP),
                Task::Label(satisfied),
            ]);
        }
        operations.extend([Task::Literal(b"1".to_vec()), Task::Label(end)]);
    }

    fn append_string_is_integer_tasks(
        &mut self,
        class: tcl_registry::native_string_is_compilation::NativeStringIsClass,
        strict: bool,
        version: tcl_dialect::TclVersion,
        operations: &mut Vec<NativeEmissionTask>,
    ) {
        use NativeEmissionTask as Task;
        use tcl_registry::native_string_is_compilation::NativeStringIsClass as Class;
        let end = self.fresh_label("is_end");
        if strict {
            operations.extend([
                is_op(Op::NUMERIC_TYPE),
                is_op(Op::DUP),
                is_jump(Op::JUMP_FALSE1, &end),
            ]);
        } else {
            let test = self.fresh_label("is_numeric");
            operations.extend([
                is_op(Op::DUP),
                is_op(Op::NUMERIC_TYPE),
                is_op(Op::DUP),
                is_jump(Op::JUMP_TRUE1, &test),
                is_op(Op::POP),
            ]);
            operations.extend(Self::string_is_empty_tasks(version, false));
            operations.extend([
                is_jump(Op::JUMP1, &end),
                Task::Label(test),
                Self::string_is_swap_task(version),
                is_op(Op::POP),
            ]);
        }
        // C8.6 `integer` is TCL_NUMBER_LONG exactly; C9 shares entier's bound.
        let (bound, compare) = match class {
            Class::Wide => (b"2", Op::LE),
            Class::Integer if version < tcl_dialect::TclVersion::V9_0 => (b"1", Op::EQ),
            _ => (b"3", Op::LE),
        };
        operations.extend([
            Task::Literal(bound.to_vec()),
            is_op(compare),
            Task::Label(end),
        ]);
    }

    pub(super) fn append_native_construction_tasks(
        &mut self,
        command: &tcl_lexer::NativeScriptCommandWords,
        instruction: tcl_registry::native_instruction_plan::NativeInstructionPlan,
        version: tcl_dialect::TclVersion,
        operations: &mut Vec<NativeEmissionTask>,
    ) -> bool {
        use tcl_registry::native_instruction_plan::NativeInstructionPlan;
        match instruction {
            NativeInstructionPlan::StringIs(recipe) => {
                self.append_native_string_is_tasks(command, recipe, version, operations)
            }
            NativeInstructionPlan::Constant {
                target_word,
                target,
                value_word,
            } => self.append_native_constant_tasks(
                command,
                target_word,
                target,
                value_word,
                operations,
            ),
            NativeInstructionPlan::DictionaryCreate(recipe) => {
                Self::append_native_dictionary_create_tasks(command, recipe, operations)
            }
            _ => false,
        }
    }

    /// `TclCompileDictCreateCmd`: a constant literal checked by `dictVerify`,
    /// C9.1 `dictPut` construction, or C8.6/C9.0 anonymous-local `dictSet`.
    fn append_native_dictionary_create_tasks(
        command: &tcl_lexer::NativeScriptCommandWords,
        recipe: tcl_registry::native_dictionary_compilation::NativeDictionaryCreateInstruction,
        operations: &mut Vec<NativeEmissionTask>,
    ) -> bool {
        use NativeEmissionTask as Task;
        use tcl_registry::native_dictionary_compilation::NativeDictionaryCreateInstruction as Create;
        match recipe {
            Create::Constant(literal) => {
                operations.extend([
                    Task::Literal(literal),
                    Task::Operation(Op::DUP, vec![]),
                    Task::Operation(Op::VERIFY_DICT, vec![]),
                ]);
            }
            Create::Put(operands) => {
                operations.push(Task::Literal(Vec::new()));
                for pair in operands.as_chunks::<2>().0 {
                    for operand in pair {
                        let Some(task) =
                            Self::native_namespace_word_task(&command.words, operand.clone())
                        else {
                            return false;
                        };
                        operations.push(task);
                    }
                    operations.push(Task::Operation(Op::DICT_PUT, vec![]));
                }
            }
            Create::Worker(operands) => {
                let worker = std::rc::Rc::new(std::cell::Cell::new(None));
                operations.extend([
                    Task::DeclareNativeTemporary(std::rc::Rc::clone(&worker)),
                    Task::Literal(Vec::new()),
                    Task::NativeTemporaryOperation(
                        Op::STORE_SCALAR1,
                        std::rc::Rc::clone(&worker),
                        vec![],
                    ),
                    Task::Operation(Op::POP, vec![]),
                ]);
                for pair in operands.as_chunks::<2>().0 {
                    for operand in pair {
                        let Some(task) =
                            Self::native_namespace_word_task(&command.words, operand.clone())
                        else {
                            return false;
                        };
                        operations.push(task);
                    }
                    operations.push(Task::NativeTemporaryOperation(
                        Op::DICT_SET,
                        std::rc::Rc::clone(&worker),
                        vec![Operand::Imm(1)],
                    ));
                    operations.push(Task::Operation(Op::POP, vec![]));
                }
                operations.extend([
                    Task::NativeTemporaryOperation(
                        Op::LOAD_SCALAR1,
                        std::rc::Rc::clone(&worker),
                        vec![],
                    ),
                    Task::NativeTemporaryOperation(Op::UNSET_SCALAR, worker, vec![Operand::Imm(0)]),
                ]);
            }
        }
        true
    }

    /// `TclCompileConstCmd`: name (unless local), value, constImm/constStk,
    /// then the empty result.
    fn append_native_constant_tasks(
        &mut self,
        command: &tcl_lexer::NativeScriptCommandWords,
        target_word: usize,
        target: tcl_syntax::native_variable_words::NativeVariableWordOperand,
        value_word: usize,
        operations: &mut Vec<NativeEmissionTask>,
    ) -> bool {
        use NativeEmissionTask as Task;
        let (Some(target_word), Some(value)) = (
            command.words.get(target_word),
            command.words.get(value_word),
        ) else {
            return false;
        };
        let (slot, array) =
            self.prepare_native_variable_tasks(target_word, target, false, operations);
        if array {
            return false;
        }
        operations.push(Task::Word(value.clone()));
        operations.push(match slot {
            Some(slot) => Task::Operation(
                Op::CONST_IMM,
                vec![Operand::Imm(super::super::bytecode_imm(slot))],
            ),
            None => Task::Operation(Op::CONST_STK, vec![]),
        });
        operations.push(Task::Literal(Vec::new()));
        true
    }

    pub(super) fn native_format_tasks(
        command: &tcl_lexer::NativeScriptCommandWords,
        recipe: tcl_registry::native_instruction_plan::NativeFormatStringInstruction,
    ) -> Option<Vec<NativeEmissionTask>> {
        use tcl_registry::native_instruction_plan::{
            NativeFormatLiteralAllocation as Allocation, NativeFormatStringPiece as Piece,
        };
        let count = recipe.pieces.len();
        let mut tasks = Vec::new();
        for piece in recipe.pieces {
            tasks.push(match piece {
                Piece::Literal(bytes) => match recipe.literal_allocation {
                    Allocation::Registered => NativeEmissionTask::Literal(bytes),
                    Allocation::PrivateString => {
                        NativeEmissionTask::NativePrivateFormatLiteral(bytes)
                    }
                },
                Piece::Value(word) => NativeEmissionTask::Word(command.words.get(word)?.clone()),
            });
        }
        if count > 1 {
            tasks.push(NativeEmissionTask::Operation(
                Op::STR_CONCAT1,
                vec![Operand::Imm(super::super::bytecode_imm(count))],
            ));
        }
        Some(tasks)
    }

    pub(super) fn native_concat_tasks(
        &mut self,
        command: &tcl_lexer::NativeScriptCommandWords,
        recipe: tcl_registry::native_instruction_plan::NativeConcatInstruction,
    ) -> Option<Vec<NativeEmissionTask>> {
        use tcl_registry::native_instruction_plan::{
            NativeConcatInstruction, NativeConcatLiteralAllocation,
        };
        match recipe {
            NativeConcatInstruction::Literal { bytes, allocation } => {
                let index = match allocation {
                    NativeConcatLiteralAllocation::Registered => self.literals.intern_bytes(&bytes),
                    NativeConcatLiteralAllocation::PrivateEmpty => {
                        self.literals.register_unshared(&bytes)
                    }
                    NativeConcatLiteralAllocation::PrivateString => {
                        self.literals.register_private_concat_string(&bytes)
                    }
                };
                Some(vec![NativeEmissionTask::PoolLiteral(index)])
            }
            NativeConcatInstruction::Operands(operands) => {
                let count = operands.len();
                let mut tasks = operands
                    .into_iter()
                    .map(|operand| Self::native_namespace_word_task(&command.words, operand))
                    .collect::<Option<Vec<_>>>()?;
                tasks.push(NativeEmissionTask::Operation(
                    Op::CONCAT_STK,
                    vec![Operand::Imm(super::super::bytecode_imm(count))],
                ));
                Some(tasks)
            }
        }
    }
}
