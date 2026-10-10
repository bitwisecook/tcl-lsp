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

//! Selected Array opcodes retain true roots, private iterator locals and original List children.

use super::*;
use tcl_registry::native_array_compilation::{NativeArrayCompilation, NativeArrayInstruction};
use tcl_registry::native_compilation::NativeArrayCommand as Command;
use tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand;
use tcl_registry::native_info_exists_compilation::NativeInfoExistsReceiver;
use tcl_syntax::value::ValueOps;

enum ArrayReceiver {
    Original {
        target: Box<Target>,
        word: usize,
    },
    Expanded {
        root: Vec<u8>,
        slot: Option<usize>,
        literal: Option<usize>,
    },
}
pub(super) struct ArrayOperation {
    recipe: NativeArrayInstruction,
    receiver: ArrayReceiver,
    alias: Option<usize>,
    temporaries: Option<[usize; 2]>,
    values: Option<NamespaceOperand>,
    parity_error: Option<[usize; 2]>,
    empty: Option<usize>,
    pub(super) prepared_words: HashMap<usize, WordInstruction>,
}
impl Builder<'_> {
    pub(super) fn array_operation(
        &mut self,
        words: &NativeCompilerWords<'_>,
        compilation: NativeArrayCompilation,
        depth: u32,
    ) -> Result<Operation, ValueError> {
        let Some(recipe) = compilation.instruction else {
            for name in compilation.declarations {
                self.local(&name, None);
            }
            return Ok(Operation::Invoke);
        };
        let mut prepared_words = HashMap::new();
        let receiver = match &recipe.receiver {
            NativeInfoExistsReceiver::Original(original) => {
                let NativeCompilerWordOperand::Original(word) = recipe.operand else {
                    return Err(unavailable("original Array name geometry"));
                };
                let target = self.target(original.clone(), depth)?;
                if matches!(original, NativeVariableWordOperand::DynamicWord) {
                    prepared_words.insert(word, self.namespace_word(words, word, false, depth)?);
                }
                ArrayReceiver::Original {
                    target: Box::new(target),
                    word,
                }
            }
            NativeInfoExistsReceiver::ExpandedLiteral { name, index } => {
                if index.is_some() {
                    return Err(unavailable("scalar Array expanded receiver"));
                }
                let slot = self.local(name, None);
                let literal = slot.is_none().then(|| self.literals.intern_bytes(name));
                ArrayReceiver::Expanded {
                    root: name.clone(),
                    slot,
                    literal,
                }
            }
        };
        let nonempty = recipe.command == Command::Set && !recipe.empty;
        let slot = match &receiver {
            ArrayReceiver::Original { target, .. } => target.slot,
            ArrayReceiver::Expanded { slot, .. } => *slot,
        };
        let alias = if nonempty && slot.is_none() {
            let (index, span) = match &recipe.operand {
                NativeCompilerWordOperand::Original(index) => {
                    (*index, words.original_words()[*index].span())
                }
                NativeCompilerWordOperand::LiteralExpansion {
                    original_word,
                    value_span,
                    ..
                } => (*original_word, *value_span),
            };
            let word = &words.original_words()[index];
            let raw = word
                .image()
                .bytes()
                .get(span.as_range())
                .ok_or_else(|| unavailable("original array alias extent"))?;
            let protocol = self
                .interp
                .native_invocation_dialect()
                .native_compiled_variable_protocol()
                .ok_or_else(|| unavailable("original array local issuer"))?;
            let slot = self.lvt.intern_native(protocol, raw);
            self.literals.intern_bytes(b"0");
            Some(slot)
        } else {
            None
        };
        let temporaries =
            nonempty.then(|| [self.lvt.intern_anonymous(), self.lvt.intern_anonymous()]);
        let values = if nonempty {
            recipe
                .values
                .as_ref()
                .map(|operand| self.namespace_operand(words, operand, &mut prepared_words, depth))
                .transpose()?
        } else {
            None
        };
        let parity_error = if nonempty && !recipe.checked_even {
            self.literals.intern_bytes(b"1");
            Some([
                self.literals
                    .intern_bytes(b"list must have an even number of elements"),
                self.literals
                    .intern_bytes(b"-errorcode {TCL ARGUMENT FORMAT}"),
            ])
        } else {
            None
        };
        let empty = (recipe.command != Command::Exists).then(|| self.literals.intern_bytes(b""));
        Ok(Operation::Array(Box::new(ArrayOperation {
            recipe,
            receiver,
            alias,
            temporaries,
            values,
            parity_error,
            empty,
            prepared_words,
        })))
    }
}
struct ArrayIteration {
    _auxiliary: obj::Owned,
    counts: obj::Owned,
}
impl ArrayIteration {
    fn new(recipe: &NativeArrayInstruction) -> Self {
        let counts = obj::Owned::fresh(obj::new_obj());
        let auxiliary = obj::Owned::fresh(obj::new_obj());
        obj::change_type(
            auxiliary.as_ptr(),
            core::ptr::null(),
            core::ptr::from_ref(recipe) as usize as u64,
        );
        Self {
            _auxiliary: auxiliary,
            counts,
        }
    }
    fn advance(&self, iteration: usize) {
        obj::change_type(self.counts.as_ptr(), core::ptr::null(), iteration as u64);
    }
}
impl Interp {
    fn array_body_target(
        &mut self,
        artifact: &NativeBodyArtifact,
        command: &CommandInstruction,
        operation: &ArrayOperation,
        execution: &mut BodyExecution,
    ) -> Result<(EvaluatedTarget, Option<usize>), Code> {
        match &operation.receiver {
            ArrayReceiver::Original { target, word } => self
                .body_target_at(artifact, command, target, *word, execution)
                .map(|target_value| (target_value, target.slot)),
            ArrayReceiver::Expanded {
                root,
                slot,
                literal,
            } => Ok((
                EvaluatedTarget {
                    root: root.clone(),
                    element: None,
                    original_name: literal.map(|index| {
                        obj::Owned::retain(
                            artifact
                                .literals
                                .original(index)
                                .expect("array scalar name PUSH"),
                        )
                    }),
                    original_index: None,
                    combined: false,
                },
                *slot,
            )),
        }
    }
    pub(super) fn execute_body_array(
        &mut self,
        artifact: &NativeBodyArtifact,
        command: &CommandInstruction,
        operation: &ArrayOperation,
        execution: &mut BodyExecution,
    ) -> Result<Code, Code> {
        let (evaluated, mut slot) =
            self.array_body_target(artifact, command, operation, execution)?;
        if execution.done {
            return Ok(Code::Ok);
        }
        let original = evaluated.original_name.as_ref().map(obj::Owned::as_ptr);
        if let Some(alias) = operation.alias {
            let level = self.frames.borrow().current_level();
            let code = self.link_original_compiled_upvar(
                original.expect("qualified Array name"),
                level,
                alias,
            );
            if code != Code::Ok {
                return Ok(code);
            }
            slot = Some(alias);
        }
        let found = self.array_exists_original_opcode(slot, &evaluated.root, original)?;
        if operation.recipe.command == Command::Exists {
            let result = self
                .array_existence_result(found)
                .map_err(|error| self.report_cmd_error(error.into()))?;
            self.set_result(result);
            return Ok(Code::Ok);
        }
        if operation.recipe.command == Command::Unset {
            if found {
                if let Some(slot) = slot {
                    self.unset_original_c_indexed(slot, &evaluated.root, None, true)?;
                } else {
                    self.unset_original_c_parts(original.expect("Array unset name"), None, true)?;
                }
            }
        } else {
            if !found {
                self.array_make_original_opcode(slot, &evaluated.root, original)?;
            }
            if let Some(values) = &operation.values {
                let mut values =
                    self.body_namespace_operand(artifact, command, values, execution)?;
                if execution.done {
                    return Ok(Code::Ok);
                }
                let protocol =
                    tcl_syntax::native_string::NativeStringProtocol::C(artifact.stamp.physical);
                let length = ValueOps::list_len(self, &values.as_ptr())
                    .map_err(|error| self.report_cmd_error(error.into()))?;
                if length % 2 != 0 {
                    let literals = operation.parity_error.ok_or_else(|| {
                        self.report_cmd_error(
                            unavailable("native known even Array list changed").into(),
                        )
                    })?;
                    let message = artifact
                        .literals
                        .original(literals[0])
                        .expect("Array parity message");
                    let options = artifact
                        .literals
                        .original(literals[1])
                        .expect("Array parity options");
                    let code=self.process_original_c_return_options(tcl_registry::native_return_options::NativeReturnOptionsApplication::Immediate,1,0,options).map_err(|error|self.report_cmd_error(error.into()))?;
                    self.set_result(message);
                    self.capture_original_return_instruction_context(tcl_registry::native_return_options::NativeReturnOptionsApplication::Immediate,message,options);
                    return Ok(code);
                }
                if obj::is_shared(values.as_ptr()) {
                    values = crate::list::native_list_copy(values.as_ptr(), protocol)
                        .map_err(|error| self.report_cmd_error(error.into()))?;
                }
                let members = crate::list::list_elements_native_checked(values.as_ptr(), protocol)
                    .map_err(|error| self.report_cmd_error(error.into()))?;
                let iteration = ArrayIteration::new(&operation.recipe);
                let temporary = operation
                    .temporaries
                    .expect("Array iterator key/value slots");
                for (index, pair) in members.chunks_exact(2).enumerate() {
                    iteration.advance(index + 1);
                    for (slot, value) in temporary.into_iter().zip(pair.iter().copied()) {
                        let result = self
                            .frames
                            .borrow_mut()
                            .store_native_compiled_temporary(slot, value);
                        result.map_err(|_| {
                            self.report_cmd_error(
                                unavailable("Array iterator original temporary").into(),
                            )
                        })?;
                    }
                    let key = obj::Owned::retain(pair[0]);
                    let value = obj::Owned::retain(pair[1]);
                    let evaluated = EvaluatedTarget {
                        root: evaluated.root.clone(),
                        element: None,
                        original_name: None,
                        original_index: Some(key),
                        combined: false,
                    };
                    // STORE_ARRAY owns the returned original only on the
                    // operand stack; its following POP never publishes it in
                    // the interpreter's result slot.
                    let stored = self.body_store_value(&evaluated, slot, &value)?;
                    drop(stored);
                }
                drop(iteration);
                drop(members);
                drop(values);
            }
        }
        self.set_result(
            artifact
                .literals
                .original(operation.empty.expect("Array empty result slot"))
                .expect("Array empty result PUSH"),
        );
        Ok(Code::Ok)
    }
}

impl ArrayOperation {
    pub(super) fn compaction_hazards(
        &self,
    ) -> Vec<tcl_registry::native_compiler_pass::NativeCompilerPassHazard> {
        self.alias
            .map(|_| tcl_registry::native_compiler_pass::NativeCompilerPassHazard::Upvar)
            .into_iter()
            .collect()
    }
}
