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

//! The `string` members C compiles to one instruction over evaluated operands
//! (`strcmp`, `strindex`, `strfind`, `strrfind`, `strupper`, `strlower`,
//! `strtitle`): each operand is the original word, and the result is what the
//! member's worker returns for the same words, read by the same shared core
//! the `string` command uses.
use super::*;
use tcl_registry::native_string_operand_compilation::{
    NativeStringOperandInstruction, NativeStringOperandOperation,
};

pub(super) struct StringOperandOperation {
    operands: Vec<NamespaceOperand>,
    operation: NativeStringOperandOperation,
    pub(super) prepared_words: HashMap<usize, WordInstruction>,
}

impl Builder<'_> {
    pub(super) fn string_operand_operation(
        &mut self,
        words: &NativeCompilerWords<'_>,
        recipe: NativeStringOperandInstruction,
        depth: u32,
    ) -> Result<StringOperandOperation, tcl_cmd_core::CmdError> {
        if !recipe.operation.accepts(recipe.operands.len()) {
            return Err(
                (ValueError::CommandProtocolUnavailable("native string operand geometry")).into(),
            );
        }
        let mut prepared_words = HashMap::new();
        let operands = recipe
            .operands
            .iter()
            .map(|operand| self.namespace_operand(words, operand, &mut prepared_words, depth))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(StringOperandOperation {
            operands,
            operation: recipe.operation,
            prepared_words,
        })
    }
}

impl Interp {
    pub(super) fn execute_body_string_operand(
        &mut self,
        artifact: &NativeBodyArtifact,
        command: &CommandInstruction,
        recipe: &StringOperandOperation,
        execution: &mut BodyExecution,
    ) -> Result<Code, Code> {
        let mut operands = Vec::with_capacity(recipe.operands.len());
        for operand in &recipe.operands {
            operands.push(self.body_namespace_operand(artifact, command, operand, execution)?);
            if execution.done {
                return Ok(Code::Ok);
            }
        }
        let words: Vec<*mut TclObj> = operands.iter().map(obj::Owned::as_ptr).collect();
        let numbers = artifact.stamp.physical.number_syntax();
        let result = match recipe.operation {
            NativeStringOperandOperation::Compare => tcl_cmd_core::string::compare(
                self,
                &words,
                tcl_cmd_core::string::CompareMode::Compare,
            ),
            NativeStringOperandOperation::Index => {
                tcl_cmd_core::string::index(self, &words[0], &words[1], numbers)
            }
            NativeStringOperandOperation::First => {
                tcl_cmd_core::string::first(self, &words[0], &words[1], None, numbers)
            }
            NativeStringOperandOperation::Last => {
                tcl_cmd_core::string::last(self, &words[0], &words[1], None, numbers)
            }
            NativeStringOperandOperation::ToUpper => tcl_cmd_core::string::case_convert(
                self,
                &words,
                tcl_cmd_core::string::CaseMode::Upper,
                "string toupper string ?first? ?last?",
                numbers,
            ),
            NativeStringOperandOperation::ToLower => tcl_cmd_core::string::case_convert(
                self,
                &words,
                tcl_cmd_core::string::CaseMode::Lower,
                "string tolower string ?first? ?last?",
                numbers,
            ),
            NativeStringOperandOperation::ToTitle => tcl_cmd_core::string::case_convert(
                self,
                &words,
                tcl_cmd_core::string::CaseMode::Title,
                "string totitle string ?first? ?last?",
                numbers,
            ),
            NativeStringOperandOperation::Cat => tcl_cmd_core::string::cat(self, &words),
        }
        .map_err(|error| self.report_cmd_error(error))?;
        drop(operands);
        self.set_result(result);
        Ok(Code::Ok)
    }
}
