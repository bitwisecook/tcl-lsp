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

impl CodegenCtx<'_> {
    pub(super) fn append_native_string_trim_tasks(
        command: &tcl_lexer::NativeScriptCommandWords,
        recipe: tcl_registry::native_string_trim_compilation::NativeStringTrimInstruction,
        version: tcl_dialect::TclVersion,
        operations: &mut Vec<NativeEmissionTask>,
    ) -> bool {
        let Some(subject) = Self::native_namespace_word_task(&command.words, recipe.subject) else {
            return false;
        };
        let characters = if let Some(original) = recipe.characters {
            let Some(characters) = Self::native_namespace_word_task(&command.words, original)
            else {
                return false;
            };
            characters
        } else {
            NativeEmissionTask::Literal(
                tcl_registry::native_string_trim_compilation::default_trim_set(version).to_vec(),
            )
        };
        operations.push(subject);
        operations.push(characters);
        let op = match recipe.operation {
            tcl_registry::native_string_trim_compilation::NativeStringTrimOperation::Both => {
                Op::STR_TRIM
            }
            tcl_registry::native_string_trim_compilation::NativeStringTrimOperation::Left => {
                Op::STR_TRIM_LEFT
            }
            tcl_registry::native_string_trim_compilation::NativeStringTrimOperation::Right => {
                Op::STR_TRIM_RIGHT
            }
        };
        operations.push(NativeEmissionTask::SwitchOperation(op, Vec::new(), version));
        true
    }
}
