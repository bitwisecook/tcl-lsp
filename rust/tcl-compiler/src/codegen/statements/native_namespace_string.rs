// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Selected namespace string compiler programs on the native work stack.

use super::{CodegenCtx, NativeEmissionTask as Task, Op, Operand};
use tcl_registry::native_namespace_string_compilation::{
    NativeNamespaceStringInstruction, NativeNamespaceStringOperation,
};

#[cfg(test)]
#[path = "native_namespace_string_tests.rs"]
mod tests;

impl CodegenCtx<'_> {
    pub(super) fn append_native_namespace_string_tasks(
        &mut self,
        command: &tcl_lexer::NativeScriptCommandWords,
        recipe: NativeNamespaceStringInstruction,
        version: tcl_dialect::TclVersion,
        operations: &mut Vec<Task>,
    ) -> bool {
        let Some(word) = Self::native_namespace_word_task(&command.words, recipe.operand) else {
            return false;
        };
        match recipe.operation {
            NativeNamespaceStringOperation::Qualifiers => {
                let previous_colon = self.fresh_label("native_namespace_qualifier_colon");
                *operations = vec![
                    word,
                    Task::Literal(b"0".to_vec()),
                    Task::Literal(b"::".to_vec()),
                    Task::Operation(Op::OVER, vec![Operand::Imm(2)]),
                    Task::SwitchOperation(Op::STR_RFIND, vec![], version),
                    Task::Label(previous_colon.clone()),
                    Task::Literal(b"1".to_vec()),
                    Task::Operation(Op::SUB, vec![]),
                    Task::Operation(Op::OVER, vec![Operand::Imm(2)]),
                    Task::Operation(Op::OVER, vec![Operand::Imm(1)]),
                    Task::SwitchOperation(Op::STR_INDEX, vec![], version),
                    Task::Literal(b":".to_vec()),
                    Task::SwitchOperation(Op::STR_EQ, vec![], version),
                    Task::Operation(Op::JUMP_TRUE4, vec![Operand::Label(previous_colon)]),
                    Task::SwitchOperation(Op::STR_RANGE, vec![], version),
                ];
            }
            NativeNamespaceStringOperation::Tail => {
                let keep_index = self.fresh_label("native_namespace_tail_index");
                *operations = vec![
                    word,
                    Task::Literal(b"::".to_vec()),
                    Task::Operation(Op::OVER, vec![Operand::Imm(1)]),
                    Task::SwitchOperation(Op::STR_RFIND, vec![], version),
                    Task::Operation(Op::DUP, vec![]),
                    Task::Literal(b"0".to_vec()),
                    Task::Operation(Op::GE, vec![]),
                    Task::Operation(Op::JUMP_FALSE4, vec![Operand::Label(keep_index.clone())]),
                    Task::Literal(b"2".to_vec()),
                    Task::Operation(Op::ADD, vec![]),
                    Task::Label(keep_index),
                    Task::Literal(b"end".to_vec()),
                    Task::SwitchOperation(Op::STR_RANGE, vec![], version),
                ];
            }
        }
        true
    }
}
