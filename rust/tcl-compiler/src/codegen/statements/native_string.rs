// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual original pattern/subject stack construction for the C match compiler.

use super::{CodegenCtx, NativeEmissionTask};
use tcl_bytecode::{Op, Operand};
use tcl_registry::native_string_compilation::{
    NativeStringMatchInstruction, NativeStringMatchOperation,
};

impl CodegenCtx<'_> {
    pub(super) fn append_native_string_match_tasks(
        command: &tcl_lexer::NativeScriptCommandWords,
        recipe: NativeStringMatchInstruction,
        version: tcl_dialect::TclVersion,
        operations: &mut Vec<NativeEmissionTask>,
    ) -> bool {
        let Some(pattern) = Self::native_namespace_word_task(&command.words, recipe.pattern) else {
            return false;
        };
        let Some(subject) = Self::native_namespace_word_task(&command.words, recipe.subject) else {
            return false;
        };
        operations.push(pattern);
        operations.push(subject);
        let (op, operands) = match recipe.operation {
            NativeStringMatchOperation::Equal => (Op::STR_EQ, Vec::new()),
            NativeStringMatchOperation::Glob { nocase } => {
                (Op::STR_MATCH, vec![Operand::Imm(i32::from(nocase))])
            }
        };
        operations.push(NativeEmissionTask::SwitchOperation(op, operands, version));
        true
    }
}
