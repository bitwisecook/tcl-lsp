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

//! Task sequences of the variable compilers: load, store, `append` and `incr`.

use super::super::{CodegenCtx, Op, Operand};
use super::NativeEmissionTask;

impl CodegenCtx<'_> {
    pub(super) fn append_native_variable_tasks(
        &mut self,
        command: &tcl_lexer::NativeScriptCommandWords,
        instruction: tcl_registry::native_instruction_plan::NativeInstructionPlan,
        operations: &mut Vec<NativeEmissionTask>,
    ) -> bool {
        use tcl_registry::native_instruction_plan::NativeInstructionPlan;
        let (target_word, target, value_word, amount_word, immediate, increment, append) =
            match instruction {
                NativeInstructionPlan::Load {
                    target_word,
                    target,
                } => (target_word, target, None, None, None, false, None),
                NativeInstructionPlan::Store {
                    target_word,
                    target,
                    value_word,
                } => (
                    target_word,
                    target,
                    Some(value_word),
                    None,
                    None,
                    false,
                    None,
                ),
                NativeInstructionPlan::Increment {
                    target_word,
                    target,
                    amount_word,
                    immediate,
                } => (
                    target_word,
                    target,
                    None,
                    amount_word,
                    immediate,
                    true,
                    None,
                ),
                NativeInstructionPlan::Append {
                    target_word,
                    target,
                    recipe,
                } => (target_word, target, None, None, None, false, Some(recipe)),
                NativeInstructionPlan::List(_) => unreachable!("List was handled above"),
                _ => return false,
            };
        let Some(target_word) = command.words.get(target_word) else {
            return false;
        };
        // C before 8.6 reaches a literal scalar name with no local slot
        // through the scalar-specific stack forms (`TclPushVarName` isScalar).
        let literal_scalar = matches!(
            &target,
            tcl_syntax::native_variable_words::NativeVariableWordOperand::Literal {
                index: None,
                ..
            }
        ) && self
            .native_entry
            .and_then(|entry| entry.execution_point?.tcl_version())
            .is_some_and(|version| version < tcl_dialect::TclVersion::V8_6);
        let (slot, array) =
            self.prepare_native_variable_tasks(target_word, target, increment, operations);
        if let Some(recipe) = append {
            return Self::append_native_append_tasks(command, &recipe, slot, array, operations);
        }
        let start = operations.len();
        let appended = if increment {
            Self::append_native_increment_tasks(
                command,
                amount_word,
                immediate,
                slot,
                array,
                operations,
            )
        } else {
            Self::append_native_load_store_tasks(command, value_word, slot, array, operations);
            true
        };
        if literal_scalar && slot.is_none() {
            for task in &mut operations[start..] {
                if let NativeEmissionTask::Operation(op, _) = task {
                    *op = match *op {
                        Op::LOAD_STK => Op::LOAD_SCALAR_STK,
                        Op::STORE_STK => Op::STORE_SCALAR_STK,
                        Op::INCR_STK => Op::INCR_SCALAR_STK,
                        Op::INCR_STK_IMM => Op::INCR_SCALAR_STK_IMM,
                        other => other,
                    };
                }
            }
        }
        appended
    }

    pub(super) fn prepare_native_variable_tasks(
        &mut self,
        target_word: &tcl_lexer::NativeWord,
        target: tcl_syntax::native_variable_words::NativeVariableWordOperand,
        increment: bool,
        operations: &mut Vec<NativeEmissionTask>,
    ) -> (Option<usize>, bool) {
        use NativeEmissionTask as Task;
        use tcl_syntax::native_variable_words::NativeVariableWordOperand;
        match target {
            NativeVariableWordOperand::Literal { name, index, .. } => {
                let slot = self
                    .command_variable_slot(&name)
                    .filter(|slot| !increment || *slot < 256);
                if slot.is_none() {
                    operations.push(Task::Literal(name));
                }
                let array = index.is_some();
                if let Some(index) = index {
                    operations.push(Task::Literal(index));
                }
                (slot, array)
            }
            NativeVariableWordOperand::CompoundArray { name, index, .. } => {
                let slot = self
                    .command_variable_slot(&name)
                    .filter(|slot| !increment || *slot < 256);
                if slot.is_none() {
                    operations.push(Task::Literal(name));
                }
                let index = std::rc::Rc::new(index);
                let root = index.root();
                operations.push(Task::List(index, root, 0));
                (slot, true)
            }
            NativeVariableWordOperand::DynamicWord => {
                operations.push(Task::Word(target_word.clone()));
                (None, false)
            }
        }
    }

    fn append_native_append_tasks(
        command: &tcl_lexer::NativeScriptCommandWords,
        recipe: &tcl_registry::native_instruction_plan::NativeAppendInstruction,
        slot: Option<usize>,
        array: bool,
        operations: &mut Vec<NativeEmissionTask>,
    ) -> bool {
        use NativeEmissionTask as Task;
        use tcl_registry::native_compilation::NativeAppendKind;
        use tcl_registry::native_instruction_plan::{NativeAppendOperands, NativeArgumentListStep};
        let list = recipe.kind == NativeAppendKind::List;
        let list_values = matches!(recipe.operands, NativeAppendOperands::ListElements { .. });
        match &recipe.operands {
            NativeAppendOperands::StringObjects | NativeAppendOperands::ListElement => {
                operations.extend(
                    command.words[recipe.values.clone()]
                        .iter()
                        .cloned()
                        .map(Task::Word),
                );
            }
            NativeAppendOperands::ListElements {
                steps,
                strip_single_expanded,
            } => {
                for step in steps {
                    operations.push(match *step {
                        NativeArgumentListStep::Word(index) => {
                            Task::Word(command.words[index].clone())
                        }
                        NativeArgumentListStep::List(count) => Task::Operation(
                            Op::LIST,
                            vec![Operand::Imm(super::super::bytecode_imm(count))],
                        ),
                        NativeArgumentListStep::Concat => Task::Operation(Op::LIST_CONCAT, vec![]),
                    });
                }
                if *strip_single_expanded {
                    operations.push(Task::Operation(
                        Op::LIST_RANGE_IMM,
                        vec![Operand::Imm(0), Operand::Imm(tcl_bytecode::INDEX_END)],
                    ));
                }
            }
        }
        let op = match (list, list_values, array, slot) {
            (true, true, false, Some(_)) => Op::LAPPEND_LIST,
            (true, true, true, Some(_)) => Op::LAPPEND_LIST_ARRAY,
            (true, true, false, None) => Op::LAPPEND_LIST_STK,
            (true, true, true, None) => Op::LAPPEND_LIST_ARRAY_STK,
            (true, false, false, Some(slot)) if slot < 256 => Op::LAPPEND_SCALAR1,
            (true, false, false, Some(_)) => Op::LAPPEND_SCALAR4,
            (true, false, true, Some(slot)) if slot < 256 => Op::LAPPEND_ARRAY1,
            (true, false, true, Some(_)) => Op::LAPPEND_ARRAY4,
            (true, false, false, None) => Op::LAPPEND_STK,
            (true, false, true, None) => Op::LAPPEND_ARRAY_STK,
            (false, _, false, Some(slot)) if slot < 256 => Op::APPEND_SCALAR1,
            (false, _, false, Some(_)) => Op::APPEND_SCALAR4,
            (false, _, true, Some(slot)) if slot < 256 => Op::APPEND_ARRAY1,
            (false, _, true, Some(_)) => Op::APPEND_ARRAY4,
            (false, _, false, None) => Op::APPEND_STK,
            (false, _, true, None) => Op::APPEND_ARRAY_STK,
        };
        let operands =
            slot.map_or_default(|slot| vec![Operand::Imm(super::super::bytecode_imm(slot))]);
        let count = if list { 1 } else { recipe.values.len() };
        if count > 1 {
            if slot.is_none() || array {
                return false;
            }
            operations.push(Task::Operation(
                Op::REVERSE,
                vec![Operand::Imm(super::super::bytecode_imm(count))],
            ));
        }
        for index in 0..count {
            operations.push(Task::Operation(op, operands.clone()));
            if index + 1 < count {
                operations.push(Task::Operation(Op::POP, vec![]));
            }
        }
        true
    }

    fn append_native_increment_tasks(
        command: &tcl_lexer::NativeScriptCommandWords,
        amount_word: Option<usize>,
        immediate: Option<i32>,
        slot: Option<usize>,
        array: bool,
        operations: &mut Vec<NativeEmissionTask>,
    ) -> bool {
        use NativeEmissionTask as Task;
        if immediate.is_none() {
            let Some(amount) = amount_word.and_then(|index| command.words.get(index)) else {
                return false;
            };
            operations.push(Task::Word(amount.clone()));
        }
        let op = match (array, slot.is_some(), immediate.is_some()) {
            (false, true, true) => Op::INCR_SCALAR1_IMM,
            (false, true, false) => Op::INCR_SCALAR1,
            (true, true, true) => Op::INCR_ARRAY1_IMM,
            (true, true, false) => Op::INCR_ARRAY1,
            (false, false, true) => Op::INCR_STK_IMM,
            (false, false, false) => Op::INCR_STK,
            (true, false, true) => Op::INCR_ARRAY_STK_IMM,
            (true, false, false) => Op::INCR_ARRAY_STK,
        };
        let mut operands = Vec::new();
        if let Some(slot) = slot {
            operands.push(Operand::Imm(super::super::bytecode_imm(slot)));
        }
        if let Some(immediate) = immediate {
            operands.push(Operand::Imm(immediate));
        }
        operations.push(Task::Operation(op, operands));
        true
    }

    fn append_native_load_store_tasks(
        command: &tcl_lexer::NativeScriptCommandWords,
        value_word: Option<usize>,
        slot: Option<usize>,
        array: bool,
        operations: &mut Vec<NativeEmissionTask>,
    ) {
        use NativeEmissionTask as Task;
        let second = value_word.and_then(|index| command.words.get(index));
        if let Some(second) = second {
            operations.push(Task::Word(second.clone()));
        }
        let op = match (second.is_some(), array, slot) {
            (false, false, Some(slot)) if slot < 256 => Op::LOAD_SCALAR1,
            (false, false, Some(_)) => Op::LOAD_SCALAR4,
            (false, true, Some(slot)) if slot < 256 => Op::LOAD_ARRAY1,
            (false, true, Some(_)) => Op::LOAD_ARRAY4,
            (true, false, Some(slot)) if slot < 256 => Op::STORE_SCALAR1,
            (true, false, Some(_)) => Op::STORE_SCALAR4,
            (true, true, Some(slot)) if slot < 256 => Op::STORE_ARRAY1,
            (true, true, Some(_)) => Op::STORE_ARRAY4,
            (false, false, None) => Op::LOAD_STK,
            (false, true, None) => Op::LOAD_ARRAY_STK,
            (true, false, None) => Op::STORE_STK,
            (true, true, None) => Op::STORE_ARRAY_STK,
        };
        operations.push(Task::Operation(
            op,
            slot.map_or_default(|slot| vec![Operand::Imm(super::super::bytecode_imm(slot))]),
        ));
    }
}
