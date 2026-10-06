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

//! Existence uses the actual indexed cell or the original stack operand headers.

use super::*;
use tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand;
use tcl_registry::native_info_exists_compilation::{
    NativeInfoExistsInstruction, NativeInfoExistsReceiver,
};

enum ExistsTarget {
    Original(Box<Target>),
    Expanded {
        root: Vec<u8>,
        slot: Option<usize>,
        root_literal: Option<usize>,
        index_literal: Option<usize>,
    },
}

pub(super) struct InfoExistsOperation {
    target: ExistsTarget,
    word: usize,
    pub(super) prepared_words: HashMap<usize, WordInstruction>,
}

impl Builder<'_> {
    pub(super) fn info_exists_operation(
        &mut self,
        words: &NativeCompilerWords<'_>,
        recipe: NativeInfoExistsInstruction,
        depth: u32,
    ) -> Result<InfoExistsOperation, ValueError> {
        let mut prepared_words = HashMap::new();
        let (target, word) = match recipe.receiver {
            NativeInfoExistsReceiver::Original(original) => {
                let NativeCompilerWordOperand::Original(word) = recipe.operand else {
                    return Err(unavailable("original existence operand geometry"));
                };
                if matches!(original, NativeVariableWordOperand::DynamicWord) {
                    prepared_words.insert(word, self.namespace_word(words, word, false, depth)?);
                }
                (
                    ExistsTarget::Original(Box::new(self.target(original, depth)?)),
                    word,
                )
            }
            NativeInfoExistsReceiver::ExpandedLiteral { name, index } => {
                let NativeCompilerWordOperand::LiteralExpansion { original_word, .. } =
                    recipe.operand
                else {
                    return Err(unavailable("expanded existence operand geometry"));
                };
                let slot = self.local(&name, None);
                let root_literal = slot.is_none().then(|| self.literals.intern_bytes(&name));
                let index_literal = index.map(|index| self.literals.intern_bytes(&index));
                (
                    ExistsTarget::Expanded {
                        root: name,
                        slot,
                        root_literal,
                        index_literal,
                    },
                    original_word,
                )
            }
        };
        Ok(InfoExistsOperation {
            target,
            word,
            prepared_words,
        })
    }
}

impl Interp {
    fn body_exists_target(
        &mut self,
        artifact: &NativeBodyArtifact,
        command: &CommandInstruction,
        exists: &InfoExistsOperation,
        execution: &mut BodyExecution,
    ) -> Result<(EvaluatedTarget, Option<usize>), Code> {
        match &exists.target {
            ExistsTarget::Original(target) => self
                .body_target_at(artifact, command, target, exists.word, execution)
                .map(|evaluated| (evaluated, target.slot)),
            ExistsTarget::Expanded {
                root,
                slot,
                root_literal,
                index_literal,
            } => {
                let retained = |index| {
                    obj::Owned::retain(
                        artifact
                            .literals
                            .original(index)
                            .expect("expanded existence PUSH"),
                    )
                };
                Ok((
                    EvaluatedTarget {
                        root: root.clone(),
                        element: None,
                        original_name: root_literal.map(retained),
                        original_index: index_literal.map(retained),
                        combined: false,
                    },
                    *slot,
                ))
            }
        }
    }

    pub(super) fn execute_body_info_exists(
        &mut self,
        artifact: &NativeBodyArtifact,
        command: &CommandInstruction,
        exists: &InfoExistsOperation,
        execution: &mut BodyExecution,
    ) -> Result<Code, Code> {
        let (evaluated, slot) = self.body_exists_target(artifact, command, exists, execution)?;
        if execution.done {
            return Ok(Code::Ok);
        }
        let element = evaluated.original_index.as_ref().map(obj::Owned::as_ptr);
        let found = match slot {
            Some(slot) => self.exists_original_c_indexed(slot, &evaluated.root, element)?,
            None => self.exists_original_c_parts(
                evaluated
                    .original_name
                    .as_ref()
                    .expect("existence stack name")
                    .as_ptr(),
                element,
            )?,
        };
        let result = self
            .native_execution_boolean_constant(found)
            .map_err(|error| self.report_cmd_error(error.into()))?;
        self.set_result(result);
        Ok(Code::Ok)
    }
}
