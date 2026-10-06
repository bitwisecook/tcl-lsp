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

//! Original Array instructions and private key/value foreach storage.

use super::{CodegenCtx, NativeEmissionTask as Task};
use tcl_bytecode::{Op, Operand};
use tcl_registry::native_array_compilation::NativeArrayCompilation;
use tcl_registry::native_compilation::NativeArrayCommand as Command;
use tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand;
use tcl_registry::native_info_exists_compilation::NativeInfoExistsReceiver;

impl CodegenCtx<'_> {
    pub(super) fn append_native_array_tasks(
        &mut self,
        command: &tcl_lexer::NativeScriptCommandWords,
        recipe: NativeArrayCompilation,
        version: tcl_dialect::TclVersion,
        operations: &mut Vec<Task>,
    ) -> bool {
        let Some(recipe) = recipe.instruction else {
            for name in recipe.declarations {
                operations.push(Task::DeclareNamespaceLocal(name));
            }
            let saved = super::NativeNamespaceRollback {
                instructions: self.instructions.len() + 1,
                labels: self.label_positions.clone(),
                loop_regions: self.inline_loop_regions.len(),
                command_index: self.cmd_index,
            };
            operations.push(Task::NamespaceGenericRollback(saved, command.clone()));
            return true;
        };
        let mut slot = match recipe.receiver {
            NativeInfoExistsReceiver::Original(target) => {
                let NativeCompilerWordOperand::Original(index) = &recipe.operand else {
                    return false;
                };
                let Some(word) = command.words.get(*index) else {
                    return false;
                };
                let (slot, array) =
                    self.prepare_native_variable_tasks(word, target, false, operations);
                if array {
                    return false;
                };
                slot
            }
            NativeInfoExistsReceiver::ExpandedLiteral { name, index } => {
                if index.is_some() {
                    return false;
                }
                let slot = self.command_variable_slot(&name);
                if slot.is_none() {
                    operations.push(Task::Literal(name));
                }
                slot
            }
        };
        let exists = if slot.is_some() {
            Op::ARRAY_EXISTS_IMM
        } else {
            Op::ARRAY_EXISTS_STK
        };
        let immediate = slot
            .map(|index| vec![Operand::Imm(super::super::bytecode_imm(index))])
            .unwrap_or_default();
        if recipe.command == Command::Exists {
            operations.push(Task::SwitchOperation(exists, immediate, version));
            return true;
        }
        if recipe.command == Command::Set && !recipe.empty && slot.is_none() {
            let Some(protocol) = self.compiled_variable_protocol else {
                return false;
            };
            let raw = match &recipe.operand {
                NativeCompilerWordOperand::Original(index) => {
                    let Some(word) = command.words.get(*index) else {
                        return false;
                    };
                    let Some(raw) = word.image().bytes().get(word.span().as_range()) else {
                        return false;
                    };
                    raw
                }
                NativeCompilerWordOperand::LiteralExpansion {
                    original_word,
                    value_span,
                    ..
                } => {
                    let Some(word) = command.words.get(*original_word) else {
                        return false;
                    };
                    let Some(raw) = word.image().bytes().get(value_span.as_range()) else {
                        return false;
                    };
                    raw
                }
            };
            slot = Some(self.lvt.intern_native(protocol, raw));
            operations.push(Task::Literal(b"0".to_vec()));
            operations.push(Task::Operation(Op::REVERSE, vec![Operand::Imm(2)]));
            operations.push(Task::Operation(
                Op::UPVAR,
                vec![Operand::Imm(super::super::bytecode_imm(slot.unwrap()))],
            ));
            operations.push(Task::Operation(Op::POP, Vec::new()));
        }
        let nonempty = recipe.command == Command::Set && !recipe.empty;
        let temporaries: [std::rc::Rc<std::cell::Cell<Option<usize>>>; 2] =
            std::array::from_fn(|_| std::rc::Rc::new(std::cell::Cell::new(None)));
        if nonempty {
            for temporary in &temporaries {
                operations.push(Task::DeclareNativeTemporary(temporary.clone()));
            }
        }
        let exists = if slot.is_some() {
            Op::ARRAY_EXISTS_IMM
        } else {
            Op::ARRAY_EXISTS_STK
        };
        let immediate = slot
            .map(|index| vec![Operand::Imm(super::super::bytecode_imm(index))])
            .unwrap_or_default();
        let no_operation = self.fresh_label("native_array_no_operation");
        let ready = self.fresh_label("native_array_ready");
        if slot.is_none() {
            operations.push(Task::Operation(Op::DUP, Vec::new()));
        }
        operations.push(Task::SwitchOperation(exists, immediate.clone(), version));
        operations.push(Task::Operation(
            if recipe.command == Command::Unset {
                Op::JUMP_FALSE1
            } else {
                Op::JUMP_TRUE1
            },
            vec![Operand::Label(if slot.is_some() {
                ready.clone()
            } else {
                no_operation.clone()
            })],
        ));
        let (op, operands) = if recipe.command == Command::Unset {
            let mut operands = vec![Operand::Imm(1)];
            operands.extend(immediate);
            (
                if slot.is_some() {
                    Op::UNSET_SCALAR
                } else {
                    Op::UNSET_STK
                },
                operands,
            )
        } else {
            (
                if slot.is_some() {
                    Op::ARRAY_MAKE_IMM
                } else {
                    Op::ARRAY_MAKE_STK
                },
                immediate,
            )
        };
        operations.push(Task::SwitchOperation(op, operands, version));
        if slot.is_none() {
            operations.push(Task::Operation(
                Op::JUMP1,
                vec![Operand::Label(ready.clone())],
            ));
            operations.push(Task::Label(no_operation));
            operations.push(Task::Operation(Op::POP, Vec::new()));
        }
        operations.push(Task::Label(ready));
        if nonempty {
            let Some(values) = recipe.values else {
                return false;
            };
            let Some(task) = Self::native_namespace_word_task(&command.words, values) else {
                return false;
            };
            operations.push(task);
            if !recipe.checked_even {
                let even = self.fresh_label("native_array_even");
                operations.push(Task::Operation(Op::DUP, Vec::new()));
                operations.push(Task::SwitchOperation(Op::LIST_LENGTH, Vec::new(), version));
                operations.push(Task::Literal(b"1".to_vec()));
                operations.push(Task::Operation(Op::BITAND, Vec::new()));
                operations.push(Task::Operation(
                    Op::JUMP_FALSE1,
                    vec![Operand::Label(even.clone())],
                ));
                operations.push(Task::Literal(
                    b"list must have an even number of elements".to_vec(),
                ));
                operations.push(Task::Literal(b"-errorcode {TCL ARGUMENT FORMAT}".to_vec()));
                operations.push(Task::SwitchOperation(
                    Op::RETURN_IMM,
                    vec![Operand::Imm(1), Operand::Imm(0)],
                    version,
                ));
                operations.push(Task::Label(even));
            }
            operations.push(Task::NativeArrayEachStart(version, temporaries.clone()));
            operations.push(Task::NativeTemporaryOperation(
                Op::LOAD_SCALAR4,
                temporaries[0].clone(),
                Vec::new(),
            ));
            operations.push(Task::NativeTemporaryOperation(
                Op::LOAD_SCALAR4,
                temporaries[1].clone(),
                Vec::new(),
            ));
            operations.push(Task::SwitchOperation(
                if slot.unwrap() < 256 {
                    Op::STORE_ARRAY1
                } else {
                    Op::STORE_ARRAY4
                },
                vec![Operand::Imm(super::super::bytecode_imm(slot.unwrap()))],
                version,
            ));
            operations.push(Task::Operation(Op::POP, Vec::new()));
            operations.push(Task::Operation(Op::FOREACH_STEP, Vec::new()));
            operations.push(Task::Operation(Op::FOREACH_END, Vec::new()));
        }
        operations.push(Task::Literal(Vec::new()));
        true
    }
}
