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

//! The task sequence of a registered native compiler: the dispatch over instruction plans and the dictionary, list, `uplevel`, namespace, `TclOO` and `return` families.

use super::super::{CodegenCtx, Op, Operand};
use super::{NativeEmissionTask, NativeNamespaceRollback};

impl CodegenCtx<'_> {
    // One arm per instruction plan: the dispatch is exhaustive by design.
    #[allow(clippy::too_many_lines)]
    pub(super) fn append_native_registered_tasks(
        &mut self,
        command: &tcl_lexer::NativeScriptCommandWords,
        version: tcl_dialect::TclVersion,
        instruction: tcl_registry::native_instruction_plan::NativeInstructionPlan,
        operations: &mut Vec<NativeEmissionTask>,
    ) -> bool {
        use tcl_registry::native_instruction_plan::NativeInstructionPlan;
        match instruction {
            selected @ (NativeInstructionPlan::Unset(_)
            | NativeInstructionPlan::Concat(_)
            | NativeInstructionPlan::FormatStringConcat(_)
            | NativeInstructionPlan::DictionaryScope(_)
            | NativeInstructionPlan::GenericPreparation(_)
            | NativeInstructionPlan::Error(_)
            | NativeInstructionPlan::Coroutine(_)
            | NativeInstructionPlan::Each(_)
            | NativeInstructionPlan::DictionaryEach(_)
            | NativeInstructionPlan::Expression(_)
            | NativeInstructionPlan::Control(_)
            | NativeInstructionPlan::Try(_)
            | NativeInstructionPlan::Switch(_)) => {
                self.replace_native_registered_sequence(command, selected, operations)
            }

            NativeInstructionPlan::DictionaryMutation(recipe) => {
                self.append_native_dictionary_mutation_tasks(command, recipe, version, operations)
            }

            NativeInstructionPlan::DictionaryLookup(recipe) => {
                Self::append_native_dictionary_tasks(command, recipe, operations)
            }

            NativeInstructionPlan::NamedInvocation(_) => false,
            NativeInstructionPlan::Uplevel(recipe) => {
                Self::append_native_uplevel_tasks(command, &recipe, operations)
            }

            selected @ (NativeInstructionPlan::ClockRead(_)
            | NativeInstructionPlan::Break
            | NativeInstructionPlan::Continue
            | NativeInstructionPlan::StringMap(_)) => {
                Self::append_native_leaf_tasks(command, selected, version, operations)
            }

            NativeInstructionPlan::NamespaceBindings(recipe) => {
                self.append_native_namespace_tasks(command, recipe, operations)
            }
            NativeInstructionPlan::NamespaceString(recipe) => {
                self.append_native_namespace_string_tasks(command, recipe, version, operations)
            }
            NativeInstructionPlan::StringTrim(recipe) => {
                Self::append_native_string_trim_tasks(command, recipe, version, operations)
            }
            selected @ (NativeInstructionPlan::StringMatch(_)
            | NativeInstructionPlan::Regexp(_)) => {
                Self::append_native_matcher_tasks(command, selected, version, operations)
            }
            NativeInstructionPlan::Substitution(recipe) => {
                self.append_native_substitution_tasks(command, recipe, operations)
            }
            NativeInstructionPlan::Upvar(recipe) => {
                Self::append_native_upvar_tasks(command, recipe, operations)
            }
            NativeInstructionPlan::InfoExists(recipe) => {
                self.append_native_info_exists_tasks(command, recipe, operations)
            }
            NativeInstructionPlan::ListOperations(recipe) => {
                self.append_native_list_operation_tasks(command, recipe, operations)
            }
            NativeInstructionPlan::ListIndex(recipe) => {
                Self::append_native_list_index_tasks(command, recipe, operations)
            }
            NativeInstructionPlan::Array(recipe) => {
                self.append_native_array_tasks(command, recipe, version, operations)
            }
            NativeInstructionPlan::Introspection(recipe) => {
                self.append_native_introspection_tasks(command, recipe, version, operations)
            }
            NativeInstructionPlan::MathOperator(recipe) => {
                Self::append_native_mathop_tasks(command, recipe, version, operations)
            }
            NativeInstructionPlan::Scalar(recipe) => {
                Self::append_native_scalar_tasks(command, recipe, version, operations)
            }
            NativeInstructionPlan::StringRange(recipe) => {
                Self::append_native_string_range_tasks(command, recipe, version, operations)
            }
            NativeInstructionPlan::StringReplace(recipe) => {
                Self::append_native_string_replace_tasks(command, &recipe, version, operations)
            }
            NativeInstructionPlan::StringInsert(recipe) => {
                Self::append_native_string_insert_tasks(command, &recipe, version, operations)
            }
            NativeInstructionPlan::ListReplace(recipe) => {
                Self::append_native_list_replace_tasks(command, recipe, version, operations)
            }
            NativeInstructionPlan::SyntaxError { message, options } => {
                operations.extend([
                    NativeEmissionTask::Literal(message),
                    NativeEmissionTask::Literal(options),
                    NativeEmissionTask::Operation(
                        Op::SYNTAX,
                        vec![Operand::Imm(1), Operand::Imm(0)],
                    ),
                ]);
                true
            }
            NativeInstructionPlan::StringOperand(recipe) => {
                Self::append_native_string_operand_tasks(command, recipe, version, operations)
            }
            NativeInstructionPlan::TclOoHelper(recipe) => {
                Self::append_native_tcloo_tasks(command, version, recipe, operations)
            }

            NativeInstructionPlan::ReturnOptions(recipe) => {
                Self::append_native_return_tasks(command, recipe, operations)
            }
            NativeInstructionPlan::List(recipe) => {
                self.append_native_list_tasks(command, version, recipe, operations)
            }
            selected @ (NativeInstructionPlan::Constant { .. }
            | NativeInstructionPlan::DictionaryCreate(_)
            | NativeInstructionPlan::StringIs(_)) => {
                self.append_native_construction_tasks(command, selected, version, operations)
            }
            selected @ (NativeInstructionPlan::Load { .. }
            | NativeInstructionPlan::Store { .. }
            | NativeInstructionPlan::Increment { .. }
            | NativeInstructionPlan::Append { .. }) => {
                self.append_native_variable_tasks(command, selected, operations)
            }
        }
    }

    fn append_native_leaf_tasks(
        command: &tcl_lexer::NativeScriptCommandWords,
        instruction: tcl_registry::native_instruction_plan::NativeInstructionPlan,
        version: tcl_dialect::TclVersion,
        operations: &mut Vec<NativeEmissionTask>,
    ) -> bool {
        use NativeEmissionTask as Task;
        use tcl_registry::native_instruction_plan::NativeInstructionPlan;
        match instruction {
            NativeInstructionPlan::ClockRead(which) => operations.push(Task::Operation(
                Op::CLOCK_READ,
                vec![Operand::Imm(i32::from(which))],
            )),
            NativeInstructionPlan::Break => operations.push(Task::NativeLoopExit(true, version)),
            NativeInstructionPlan::Continue => {
                operations.push(Task::NativeLoopExit(false, version));
            }
            NativeInstructionPlan::StringMap(recipe) => {
                use tcl_registry::native_string_map_compilation::NativeStringMapInstruction as Map;
                let (literals, subject) = match recipe {
                    Map::Identity(subject) => (None, subject),
                    Map::Pair {
                        key,
                        replacement,
                        subject,
                    } => (Some([key, replacement]), subject),
                };
                if let Some(literals) = &literals {
                    operations.extend(literals.iter().cloned().map(Task::Literal));
                }
                let Some(task) = Self::native_namespace_word_task(&command.words, subject) else {
                    return false;
                };
                operations.push(task);
                if literals.is_some() {
                    operations.push(Task::Operation(Op::STR_MAP, vec![]));
                }
            }
            _ => return false,
        }
        true
    }

    fn replace_native_registered_sequence(
        &mut self,
        command: &tcl_lexer::NativeScriptCommandWords,
        instruction: tcl_registry::native_instruction_plan::NativeInstructionPlan,
        operations: &mut Vec<NativeEmissionTask>,
    ) -> bool {
        let Some(tasks) = self.native_registered_task_sequence(command, instruction) else {
            return false;
        };
        *operations = tasks;
        true
    }

    fn native_registered_task_sequence(
        &mut self,
        command: &tcl_lexer::NativeScriptCommandWords,
        instruction: tcl_registry::native_instruction_plan::NativeInstructionPlan,
    ) -> Option<Vec<NativeEmissionTask>> {
        use tcl_registry::native_instruction_plan::NativeInstructionPlan;
        match instruction {
            NativeInstructionPlan::Unset(recipe) => self.native_unset_tasks(command, recipe),
            NativeInstructionPlan::Concat(recipe) => self.native_concat_tasks(command, recipe),
            NativeInstructionPlan::FormatStringConcat(recipe) => {
                Self::native_format_tasks(command, recipe)
            }
            NativeInstructionPlan::DictionaryScope(recipe) => {
                self.native_dictionary_scope_tasks(command, recipe)
            }
            NativeInstructionPlan::GenericPreparation(preparations) => {
                self.native_generic_preparation_tasks(command, &preparations)
            }
            NativeInstructionPlan::Error(recipe) => self.native_error_tasks(command, recipe),
            NativeInstructionPlan::Coroutine(recipe) => {
                self.native_coroutine_tasks(command, recipe)
            }
            NativeInstructionPlan::Each(recipe) => self.native_each_tasks(command, recipe),
            NativeInstructionPlan::DictionaryEach(recipe) => {
                self.native_dictionary_each_tasks(command, recipe)
            }
            NativeInstructionPlan::Expression(recipe) => {
                self.native_expression_tasks(command, recipe)
            }
            NativeInstructionPlan::Control(recipe) => self.native_control_tasks(command, recipe),
            NativeInstructionPlan::Try(recipe) => self.native_try_tasks(command, recipe),
            NativeInstructionPlan::Switch(recipe) => {
                Some(self.native_switch_tasks(command, &recipe))
            }
            _ => None,
        }
    }

    fn append_native_dictionary_tasks(
        command: &tcl_lexer::NativeScriptCommandWords,
        recipe: tcl_registry::native_dictionary_compilation::NativeDictionaryLookupInstruction,
        operations: &mut Vec<NativeEmissionTask>,
    ) -> bool {
        use NativeEmissionTask as Task;
        use tcl_registry::native_dictionary_compilation::NativeDictionaryLookupKind;
        for operand in recipe.operands {
            let Some(task) = Self::native_namespace_word_task(&command.words, operand) else {
                return false;
            };
            operations.push(task);
        }
        let opcode = match recipe.kind {
            NativeDictionaryLookupKind::Get => Op::DICT_GET,
            NativeDictionaryLookupKind::Exists => Op::DICT_EXISTS,
            NativeDictionaryLookupKind::GetDefault => Op::DICT_GET_DEF,
        };
        let Ok(count) = i32::try_from(recipe.key_count) else {
            return false;
        };
        operations.push(Task::Operation(opcode, vec![Operand::Imm(count)]));

        true
    }

    fn append_native_list_index_tasks(
        command: &tcl_lexer::NativeScriptCommandWords,
        recipe: tcl_registry::native_list_index_compilation::NativeListIndexInstruction,
        operations: &mut Vec<NativeEmissionTask>,
    ) -> bool {
        use NativeEmissionTask as Task;
        use tcl_registry::native_list_index_compilation::NativeListIndexOperation as Operation;
        for operand in recipe.operands {
            let Some(task) = Self::native_namespace_word_task(&command.words, operand) else {
                return false;
            };
            operations.push(task);
        }
        operations.push(match recipe.operation {
            Operation::Immediate(index) => Task::NativeListIndex(index),
            Operation::Single => Task::Operation(Op::LIST_INDEX, vec![]),
            Operation::Multi(count) => Task::Operation(
                Op::LINDEX_MULTI,
                vec![Operand::Imm(
                    i32::try_from(count).expect("native list-index operand count"),
                )],
            ),
        });
        true
    }

    fn append_native_uplevel_tasks(
        command: &tcl_lexer::NativeScriptCommandWords,
        recipe: &tcl_registry::native_instruction_plan::NativeUplevelInstruction,
        operations: &mut Vec<NativeEmissionTask>,
    ) -> bool {
        use NativeEmissionTask as Task;
        operations.push(match recipe.level_word {
            Some(index) => Task::Word(command.words[index].clone()),
            None => Task::Literal(b"1".to_vec()),
        });
        operations.extend(
            command.words[recipe.script_words.clone()]
                .iter()
                .cloned()
                .map(Task::Word),
        );
        if recipe.script_words.len() > 1 {
            operations.push(Task::Operation(
                Op::CONCAT_STK,
                vec![Operand::Imm(super::super::bytecode_imm(
                    recipe.script_words.len(),
                ))],
            ));
        }
        operations.push(Task::Operation(Op::UPLEVEL, vec![]));

        true
    }

    fn append_native_namespace_tasks(
        &mut self,
        command: &tcl_lexer::NativeScriptCommandWords,
        recipe: tcl_registry::native_namespace_binding_compilation::NativeNamespaceBindingCompilation,
        operations: &mut Vec<NativeEmissionTask>,
    ) -> bool {
        use NativeEmissionTask as Task;
        use tcl_registry::native_namespace_binding_compilation::{
            NativeNamespaceBindingKind as Kind, NativeNamespaceBindingOutcome as Outcome,
        };
        let generic = recipe.outcome == Outcome::Generic;
        if recipe.outcome == Outcome::Unknown {
            return false;
        }
        if recipe.kind == Kind::Upvar {
            let Some(tasks) = Self::native_namespace_upvar_tasks(command, &recipe) else {
                return false;
            };
            *operations = tasks;
            if generic {
                let saved = NativeNamespaceRollback {
                    // The common original compiler guard is emitted
                    // before these speculative word/declaration tasks.
                    instructions: self.instructions.len() + 1,
                    labels: self.label_positions.clone(),
                    loop_regions: self.inline_loop_regions.len(),
                    command_index: self.cmd_index,
                };
                operations.push(Task::NamespaceGenericRollback(saved, command.clone()));
            } else {
                operations.push(Task::Operation(Op::POP, vec![]));
                operations.push(Task::Literal(Vec::new()));
            }
        } else {
            for visit in &recipe.visits {
                if let tcl_registry::native_namespace_binding_compilation::NativeNamespaceBindingVisit::Literal(value) = visit {
                        operations.push(Task::Literal(value.clone()));
                    }
            }
            for binding in recipe.bindings {
                operations.push(Task::DeclareNamespaceLocal(binding.local.clone()));
                let Some(name) = Self::native_namespace_word_task(&command.words, binding.name)
                else {
                    return false;
                };
                operations.push(name);
                operations.push(Task::NamespaceLocalOperation(
                    if recipe.kind == Kind::Global {
                        Op::NSUPVAR
                    } else {
                        Op::VARIABLE
                    },
                    binding.local.clone(),
                ));
                if let Some(value) = binding.value {
                    let Some(value) = Self::native_namespace_word_task(&command.words, value)
                    else {
                        return false;
                    };
                    operations.push(value);
                    operations.push(Task::NamespaceLocalOperation(
                        Op::STORE_SCALAR1,
                        binding.local,
                    ));
                    operations.push(Task::Operation(Op::POP, vec![]));
                }
            }
            if generic {
                let saved = NativeNamespaceRollback {
                    instructions: self.instructions.len() + 1,
                    labels: self.label_positions.clone(),
                    loop_regions: self.inline_loop_regions.len(),
                    command_index: self.cmd_index,
                };
                operations.push(Task::NamespaceGenericRollback(saved, command.clone()));
            } else {
                if recipe.kind == Kind::Global {
                    operations.push(Task::Operation(Op::POP, vec![]));
                }
                operations.push(Task::Literal(Vec::new()));
            }
        }

        true
    }

    fn append_native_tcloo_tasks(
        command: &tcl_lexer::NativeScriptCommandWords,
        version: tcl_dialect::TclVersion,
        recipe: tcl_registry::native_tcloo_compilation::NativeTclOoInstruction,
        operations: &mut Vec<NativeEmissionTask>,
    ) -> bool {
        use NativeEmissionTask as Task;
        use tcl_registry::native_tcloo_compilation::NativeTclOoInstruction as Helper;
        match recipe {
            Helper::SelfObject => operations.push(Task::Operation(Op::TCLOO_SELF, vec![])),
            Helper::SelfNamespace => {
                operations.push(Task::Operation(Op::TCLOO_SELF, vec![]));
                operations.push(Task::Operation(Op::POP, vec![]));
                operations.push(Task::Operation(Op::CURRENT_NAMESPACE, vec![]));
            }
            Helper::ObjectInfo { operation, operand } => {
                use tcl_registry::native_tcloo_compilation::NativeTclOoObjectInfo as ObjectInfo;
                let Some(task) = Self::native_namespace_word_task(&command.words, operand) else {
                    return false;
                };
                operations.push(task);
                operations.push(Task::Operation(
                    match operation {
                        ObjectInfo::Class => Op::TCLOO_CLASS,
                        ObjectInfo::Namespace => Op::TCLOO_NS,
                        ObjectInfo::IsObject => Op::TCLOO_IS_OBJECT,
                        ObjectInfo::CreationId => Op::TCLOO_ID,
                    },
                    vec![],
                ));
            }
            Helper::Next { class, words, list } => {
                if let Some(steps) = list {
                    use tcl_registry::native_instruction_plan::NativeArgumentListStep;
                    for step in steps {
                        operations.push(match step {
                            NativeArgumentListStep::Word(index) => {
                                let Some(task) = Self::native_namespace_word_task(
                                    &command.words,
                                    words[index].clone(),
                                ) else {
                                    return false;
                                };
                                task
                            }
                            NativeArgumentListStep::List(count) => Task::Operation(
                                Op::LIST,
                                vec![Operand::Imm(super::super::bytecode_imm(count))],
                            ),
                            NativeArgumentListStep::Concat => {
                                Task::Operation(Op::LIST_CONCAT, vec![])
                            }
                        });
                    }
                    operations.push(Task::Operation(
                        if class {
                            Op::TCLOO_NEXT_CLASS_LIST
                        } else {
                            Op::TCLOO_NEXT_LIST
                        },
                        vec![],
                    ));
                } else {
                    let count = words.len();
                    for word in words {
                        let Some(task) = Self::native_namespace_word_task(&command.words, word)
                        else {
                            return false;
                        };
                        operations.push(task);
                    }
                    let opcode = match (class, version >= tcl_dialect::TclVersion::V9_1) {
                        (false, false) => Op::TCLOO_NEXT,
                        (true, false) => Op::TCLOO_NEXT_CLASS,
                        (false, true) => Op::TCLOO_NEXT4,
                        (true, true) => Op::TCLOO_NEXT_CLASS4,
                    };
                    operations.push(Task::Operation(
                        opcode,
                        vec![Operand::Imm(super::super::bytecode_imm(count))],
                    ));
                }
            }
        }

        true
    }

    fn append_native_return_tasks(
        command: &tcl_lexer::NativeScriptCommandWords,
        recipe: tcl_registry::native_return_compilation::NativeReturnInstruction,
        operations: &mut Vec<NativeEmissionTask>,
    ) -> bool {
        use NativeEmissionTask as Task;
        use tcl_registry::native_return_compilation::{
            NativeReturnExit as Exit, NativeReturnOptionsOperand as Options,
        };
        match &recipe.options {
            Options::StackWord(index) => {
                operations.push(Task::Word(command.words[*index].clone()));
            }
            Options::StackPairs(range) => {
                operations.extend(command.words[range.clone()].iter().cloned().map(Task::Word));
                operations.push(Task::Operation(
                    Op::LIST,
                    vec![Operand::Imm(super::super::bytecode_imm(range.len()))],
                ));
            }
            Options::Static(_) => {}
        }
        operations.push(match recipe.value_word {
            Some(index) => Task::Word(command.words[index].clone()),
            None => Task::Literal(Vec::new()),
        });
        match recipe.exit {
            Exit::Done => operations.push(Task::Operation(Op::DONE, vec![])),
            Exit::Fallthrough => {}
            Exit::Stack => operations.push(Task::Operation(Op::RETURN_STK, vec![])),
            Exit::Immediate => {
                let Options::Static(options) = recipe.options else {
                    unreachable!("immediate static options")
                };
                let (code, level) = (options.code, options.level);
                operations.push(Task::PrivateReturnOptions(options));
                operations.push(Task::Operation(
                    Op::RETURN_IMM,
                    vec![Operand::Imm(code), Operand::Imm(level)],
                ));
            }
        }

        true
    }

    fn append_native_list_tasks(
        &mut self,
        command: &tcl_lexer::NativeScriptCommandWords,
        version: tcl_dialect::TclVersion,
        recipe: tcl_registry::native_compiler_words::NativeCompiledListRecipe,
        operations: &mut Vec<NativeEmissionTask>,
    ) -> bool {
        use NativeEmissionTask as Task;
        use tcl_registry::native_compiler_words::NativeCompiledListRecipe;
        match recipe {
            NativeCompiledListRecipe::EmptyString => operations.push(Task::Literal(Vec::new())),
            NativeCompiledListRecipe::DynamicElements { from } => {
                operations.extend(command.words[from..].iter().cloned().map(Task::Word));
                operations.push(Task::Operation(
                    Op::LIST,
                    vec![Operand::Imm(super::super::bytecode_imm(
                        command.words.len() - from,
                    ))],
                ));
            }
            NativeCompiledListRecipe::PrivateConstant { members } => {
                let index = self.literals.register_private_constant_list(
                    &members,
                    tcl_syntax::native_string::NativeStringProtocol::C(version),
                );
                operations.push(Task::PoolLiteral(index));
            }
        }
        true
    }

    pub(super) fn native_namespace_word_task(
        words: &[tcl_lexer::NativeWord],
        operand: tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand,
    ) -> Option<NativeEmissionTask> {
        use tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand;
        match operand {
            NativeCompilerWordOperand::Original(index) => {
                words.get(index).cloned().map(NativeEmissionTask::Word)
            }
            NativeCompilerWordOperand::LiteralExpansion {
                original_word,
                value_span,
                value,
            } => {
                let word = words.get(original_word)?;
                word.image().bytes().get(value_span.as_range())?;
                Some(NativeEmissionTask::Literal(value))
            }
        }
    }
}
