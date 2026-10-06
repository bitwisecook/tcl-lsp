// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original native unset validation emits sequential receiver operations.

use super::*;
use tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand;
use tcl_registry::native_unset_compilation::{NativeUnsetInstruction, NativeUnsetReceiver};

pub(super) struct UnsetOperation {
    complain: bool,
    variables: Vec<UnsetVariable>,
    pub(super) prepared_words: HashMap<usize, WordInstruction>,
    empty: usize,
}

enum UnsetVariable {
    Original {
        target: Target,
        word: usize,
    },
    Expanded {
        slot: Option<usize>,
        root: Vec<u8>,
        name_literal: Option<usize>,
        index_literal: Option<usize>,
    },
}

impl Builder<'_> {
    pub(super) fn unset_operation(
        &mut self,
        captured: &NativeCompilerWords<'_>,
        recipe: NativeUnsetInstruction,
        depth: u32,
    ) -> Result<UnsetOperation, ValueError> {
        let mut variables = Vec::with_capacity(recipe.variables.len());
        let mut prepared_words = HashMap::new();
        for variable in recipe.variables {
            let target = match variable.receiver {
                NativeUnsetReceiver::Original(original) => {
                    let NativeCompilerWordOperand::Original(word) = variable.operand else {
                        return Err(unavailable("original native unset word geometry"));
                    };
                    let target = self.target(original, depth)?;
                    if matches!(target.original, NativeVariableWordOperand::DynamicWord) {
                        prepared_words
                            .insert(word, self.namespace_word(captured, word, false, depth)?);
                    }
                    UnsetVariable::Original { target, word }
                }
                NativeUnsetReceiver::ExpandedLiteral { name, index } => {
                    let slot = self.local(&name, None);
                    let name_literal = slot.is_none().then(|| self.literals.intern_bytes(&name));
                    let index_literal = index
                        .as_ref()
                        .map(|index| self.literals.intern_bytes(index));
                    UnsetVariable::Expanded {
                        slot,
                        root: name,
                        name_literal,
                        index_literal,
                    }
                }
            };
            variables.push(target);
        }
        Ok(UnsetOperation {
            complain: recipe.complain,
            variables,
            prepared_words,
            empty: self.literals.intern_bytes(b""),
        })
    }
}

impl Interp {
    pub(super) fn execute_body_unset(
        &mut self,
        artifact: &NativeBodyArtifact,
        command: &CommandInstruction,
        unset: &UnsetOperation,
        execution: &mut BodyExecution,
    ) -> Result<Code, Code> {
        for variable in &unset.variables {
            let (evaluated, slot) = match variable {
                UnsetVariable::Original { target, word } => (
                    self.body_target_at(artifact, command, target, *word, execution)?,
                    target.slot,
                ),
                UnsetVariable::Expanded {
                    slot,
                    root,
                    name_literal,
                    index_literal,
                } => (
                    EvaluatedTarget {
                        root: root.clone(),
                        element: None,
                        original_name: name_literal.map(|index| {
                            obj::Owned::retain(
                                artifact
                                    .literals
                                    .original(index)
                                    .expect("projected unset root PUSH"),
                            )
                        }),
                        original_index: index_literal.map(|index| {
                            obj::Owned::retain(
                                artifact
                                    .literals
                                    .original(index)
                                    .expect("projected unset index PUSH"),
                            )
                        }),
                        combined: false,
                    },
                    *slot,
                ),
            };
            if execution.done {
                return Ok(Code::Ok);
            }
            let element = evaluated.original_index.as_ref().map(obj::Owned::as_ptr);
            if let Some(slot) = slot {
                self.unset_original_c_indexed(slot, &evaluated.root, element, unset.complain)?;
            } else {
                self.unset_original_c_parts(
                    evaluated
                        .original_name
                        .as_ref()
                        .expect("unset original stack name")
                        .as_ptr(),
                    element,
                    unset.complain,
                )?;
            }
        }
        self.set_result(
            artifact
                .literals
                .original(unset.empty)
                .expect("unset empty result PUSH"),
        );
        Ok(Code::Ok)
    }
}
