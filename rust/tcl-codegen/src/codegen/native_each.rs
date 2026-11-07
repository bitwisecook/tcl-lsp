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

//! Original foreach auxiliary slots and shared native iterator body emission.

use super::{CodegenCtx, NativeEmissionTask as Task, NativeNamespaceRollback, Op, Operand};
use tcl_registry::native_compilation::NativeCompiledBodyContext;
use tcl_registry::native_control_compilation::{
    NativeControlCompilation, NativeControlOutcome, NativeControlPreparationStep,
};
use tcl_registry::native_control_instructions::NativeControlBody;
use tcl_registry::native_each_compilation::{NativeEachCollection, NativeEachInstruction};

type Temporary = std::rc::Rc<std::cell::Cell<Option<usize>>>;

struct NativeEachBodyLabels {
    body: String,
    body_end: String,
    step: String,
    done: String,
}

#[cfg(test)]
#[path = "native_each_tests.rs"]
mod tests;

impl CodegenCtx<'_> {
    fn native_each_declined_tasks(
        &self,
        command: &tcl_lexer::NativeScriptCommandWords,
        preparations: Vec<NativeControlPreparationStep>,
    ) -> Option<Vec<Task>> {
        let mut tasks = Vec::new();
        for visit in preparations {
            match visit {
                NativeControlPreparationStep::DeclareLocal(name) => {
                    tasks.push(Task::DeclareNamespaceLocal(name));
                }
                NativeControlPreparationStep::DeclareAnonymousLocal => {
                    tasks.push(Task::DeclareNativeTemporary(std::rc::Rc::new(
                        std::cell::Cell::new(None),
                    )));
                }
                _ => return None,
            }
        }
        let saved = NativeNamespaceRollback {
            instructions: self.instructions.len(),
            labels: self.label_positions.clone(),
            loop_regions: self.inline_loop_regions.len(),
            command_index: self.cmd_index,
        };
        tasks.push(Task::NamespaceGenericRollback(saved, command.clone()));
        Some(tasks)
    }

    fn native_each_value_tasks(
        command: &tcl_lexer::NativeScriptCommandWords,
        instruction: &NativeEachInstruction,
        operand: tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand,
        group_index: usize,
        temporaries: &[Temporary],
    ) -> Option<Vec<Task>> {
        let group = instruction.groups.get(group_index)?;
        if operand != group.values {
            return None;
        }
        let mut tasks = Vec::new();
        if group_index == 0 && instruction.collection == NativeEachCollection::Lmap {
            tasks.push(Task::Operation(Op::LIST, vec![Operand::Imm(0)]));
        }
        tasks.push(Self::native_namespace_word_task(&command.words, operand)?);
        if instruction.local_temporaries {
            tasks.push(Task::NativeTemporaryOperation(
                Op::STORE_SCALAR4,
                std::rc::Rc::clone(temporaries.get(group_index)?),
                vec![],
            ));
            tasks.push(Task::Operation(Op::POP, vec![]));
        }
        Some(tasks)
    }

    fn native_each_body_tasks(
        command: &tcl_lexer::NativeScriptCommandWords,
        instruction: &NativeEachInstruction,
        body: NativeControlBody,
        context: NativeCompiledBodyContext,
        temporaries: &mut Vec<Temporary>,
        labels: &NativeEachBodyLabels,
    ) -> Option<Vec<Task>> {
        if instruction.local_temporaries && temporaries.len() != instruction.groups.len() + 1 {
            return None;
        }
        let collect = instruction.collection == NativeEachCollection::Lmap;
        let original = command.words.first()?;
        Some(vec![
            Task::NativeEachStart(
                instruction.version,
                instruction
                    .groups
                    .iter()
                    .map(|group| group.variables.clone())
                    .collect(),
                std::mem::take(temporaries),
                collect,
            ),
            Task::Label(labels.body.clone()),
            Task::ControlScript(original.image().clone(), body, context, original.config()),
            Task::Operation(if collect { Op::LMAP_COLLECT } else { Op::POP }, vec![]),
            Task::Label(labels.body_end.clone()),
            Task::Label(labels.step.clone()),
            Task::Operation(Op::FOREACH_STEP, vec![]),
            Task::Label(labels.done.clone()),
            Task::Operation(Op::FOREACH_END, vec![]),
        ])
    }

    pub(super) fn native_each_tasks(
        &mut self,
        command: &tcl_lexer::NativeScriptCommandWords,
        recipe: NativeControlCompilation<NativeEachInstruction>,
    ) -> Option<Vec<Task>> {
        let NativeControlOutcome::Inline(instruction) = recipe.outcome else {
            if matches!(recipe.outcome, NativeControlOutcome::Rejected(_)) {
                return None;
            }
            return self.native_each_declined_tasks(command, recipe.preparations);
        };
        let mut tasks = Vec::new();
        let mut temporaries = Vec::new();
        let collect = instruction.collection == NativeEachCollection::Lmap;
        let labels = NativeEachBodyLabels {
            body: self.fresh_label("native_each_body"),
            body_end: self.fresh_label("native_each_body_end"),
            step: self.fresh_label("native_each_step"),
            done: self.fresh_label("native_each_end"),
        };
        let mut values = 0;
        let mut body_visited = false;
        for visit in recipe.preparations {
            match visit {
                NativeControlPreparationStep::DeclareLocal(name)
                    if values == 0 && !body_visited =>
                {
                    tasks.push(Task::DeclareNamespaceLocal(name));
                }
                NativeControlPreparationStep::DeclareAnonymousLocal
                    if values == 0 && !body_visited =>
                {
                    let slot = std::rc::Rc::new(std::cell::Cell::new(None));
                    tasks.push(Task::DeclareNativeTemporary(std::rc::Rc::clone(&slot)));
                    temporaries.push(slot);
                }
                NativeControlPreparationStep::Word(operand) if !body_visited => {
                    tasks.extend(Self::native_each_value_tasks(
                        command,
                        &instruction,
                        operand,
                        values,
                        &temporaries,
                    )?);
                    values += 1;
                }
                NativeControlPreparationStep::Script {
                    operand,
                    span,
                    context,
                } if !body_visited
                    && values == instruction.groups.len()
                    && operand == instruction.body
                    && span == instruction.body_span
                    && context == NativeCompiledBodyContext::Loop =>
                {
                    tasks.extend(Self::native_each_body_tasks(
                        command,
                        &instruction,
                        NativeControlBody {
                            operand,
                            script: Some(span),
                        },
                        context,
                        &mut temporaries,
                        &labels,
                    )?);
                    body_visited = true;
                }
                NativeControlPreparationStep::Literal(value) if body_visited && !collect => {
                    tasks.push(Task::Literal(value));
                }
                _ => return None,
            }
        }
        if !body_visited {
            return None;
        }
        self.inline_loop_regions
            .push(super::super::InlineLoopRegion {
                start: labels.body,
                end: labels.body_end,
                continue_target: Some(labels.step),
                break_target: labels.done,
            });
        Some(tasks)
    }

    /// `CompileDictEachCmd`: iterate under a catch range that releases the
    /// iterator, with the body in its own loop range.
    pub(super) fn native_dictionary_each_tasks(
        &mut self,
        command: &tcl_lexer::NativeScriptCommandWords,
        recipe: NativeControlCompilation<
            tcl_registry::native_dictionary_each_compilation::NativeDictionaryEachInstruction,
        >,
    ) -> Option<Vec<Task>> {
        let NativeControlOutcome::Inline(instruction) = recipe.outcome else {
            return None;
        };
        let (mut tasks, temporaries) = Self::native_declaration_tasks(recipe.preparations);
        let info = temporaries.last()?.clone();
        let collect = if instruction.collect {
            Some(temporaries.first()?.clone())
        } else {
            None
        };
        let temporary = |op, slot: &Temporary, operands: Vec<Operand>| {
            Task::NativeTemporaryOperation(op, std::rc::Rc::clone(slot), operands)
        };
        let local = |op, name: &[u8]| Task::NamespaceLocalOperation(op, name.to_vec());
        let operation = |op| Task::Operation(op, vec![]);
        if instruction.version == tcl_dialect::TclVersion::V8_5 {
            return self.native_dictionary_for_c85_tasks(command, &instruction, &info, tasks);
        }
        let iteration = self.fresh_label("native_dict_each");
        let body_start = self.fresh_label("native_dict_each_body");
        let body_end = self.fresh_label("native_dict_each_body_end");
        let step = self.fresh_label("native_dict_each_step");
        let handler = self.fresh_label("native_dict_each_handler");
        let end = self.fresh_label("native_dict_each_end");
        let finish = self.fresh_label("native_dict_each_break");
        if let Some(collect) = &collect {
            tasks.extend([
                Task::Literal(Vec::new()),
                temporary(Op::STORE_SCALAR1, collect, vec![]),
                operation(Op::POP),
            ]);
        }
        tasks.push(Self::native_namespace_word_task(
            &command.words,
            instruction.dictionary.clone(),
        )?);
        let original = command.words.first()?;
        tasks.extend([
            Task::BeginNativeCatch(handler.clone()),
            temporary(Op::DICT_FIRST, &info, vec![]),
            Task::Operation(Op::JUMP_TRUE4, vec![Operand::Label(end.clone())]),
            Task::Label(iteration.clone()),
            local(Op::STORE_SCALAR1, &instruction.key),
            operation(Op::POP),
            local(Op::STORE_SCALAR1, &instruction.value),
            operation(Op::POP),
            Task::Label(body_start.clone()),
            Task::ControlScript(
                original.image().clone(),
                NativeControlBody {
                    operand: instruction.body.clone(),
                    script: Some(instruction.body_span),
                },
                NativeCompiledBodyContext::ProtectedLoop,
                original.config(),
            ),
        ]);
        if let Some(collect) = &collect {
            tasks.push(local(Op::LOAD_SCALAR1, &instruction.key));
            if instruction.version >= tcl_dialect::TclVersion::V9_1 {
                tasks.push(operation(Op::SWAP));
                tasks.push(temporary(Op::DICT_SET, collect, vec![Operand::Imm(1)]));
            } else {
                tasks.push(Task::Operation(Op::OVER, vec![Operand::Imm(1)]));
                tasks.push(temporary(Op::DICT_SET, collect, vec![Operand::Imm(1)]));
                tasks.push(operation(Op::POP));
            }
        }
        tasks.extend([
            operation(Op::POP),
            Task::Label(body_end.clone()),
            Task::Label(step.clone()),
            temporary(Op::DICT_NEXT, &info, vec![]),
            Task::Operation(Op::JUMP_FALSE4, vec![Operand::Label(iteration)]),
            Task::Operation(Op::JUMP4, vec![Operand::Label(end.clone())]),
        ]);
        tasks.extend(Self::native_dictionary_each_exit_tasks(
            &info,
            collect.as_ref(),
            handler,
            end,
            finish.clone(),
        ));
        self.inline_loop_regions
            .push(super::super::InlineLoopRegion {
                start: body_start,
                end: body_end,
                continue_target: Some(step),
                break_target: finish,
            });
        Some(tasks)
    }

    /// C8.5's `TclCompileDictForCmd`: `DICT_FIRST` precedes the catch range,
    /// and each exit releases the iterator before leaving it.
    fn native_dictionary_for_c85_tasks(
        &mut self,
        command: &tcl_lexer::NativeScriptCommandWords,
        instruction: &tcl_registry::native_dictionary_each_compilation::NativeDictionaryEachInstruction,
        info: &Temporary,
        mut tasks: Vec<Task>,
    ) -> Option<Vec<Task>> {
        let release = || {
            Task::NativeTemporaryOperation(
                Op::UNSET_SCALAR,
                std::rc::Rc::clone(info),
                vec![Operand::Imm(0)],
            )
        };
        let operation = |op| Task::Operation(op, vec![]);
        let local = |op, name: &[u8]| Task::NamespaceLocalOperation(op, name.to_vec());
        let iteration = self.fresh_label("native_dict_for");
        let body_start = self.fresh_label("native_dict_for_body");
        let body_end = self.fresh_label("native_dict_for_body_end");
        let step = self.fresh_label("native_dict_for_step");
        let finish = self.fresh_label("native_dict_for_break");
        let handler = self.fresh_label("native_dict_for_handler");
        let empty = self.fresh_label("native_dict_for_empty");
        let end = self.fresh_label("native_dict_for_end");
        let original = command.words.first()?;
        tasks.extend([
            Self::native_namespace_word_task(&command.words, instruction.dictionary.clone())?,
            Task::NativeTemporaryOperation(Op::DICT_FIRST, std::rc::Rc::clone(info), vec![]),
            Task::Operation(Op::JUMP_TRUE4, vec![Operand::Label(empty.clone())]),
            Task::BeginNativeCatch(handler.clone()),
            Task::Label(iteration.clone()),
            local(Op::STORE_SCALAR1, &instruction.key),
            operation(Op::POP),
            local(Op::STORE_SCALAR1, &instruction.value),
            operation(Op::POP),
            Task::Label(body_start.clone()),
            Task::ControlScript(
                original.image().clone(),
                NativeControlBody {
                    operand: instruction.body.clone(),
                    script: Some(instruction.body_span),
                },
                NativeCompiledBodyContext::ProtectedLoop,
                original.config(),
            ),
            operation(Op::POP),
            Task::Label(body_end.clone()),
            Task::Label(step.clone()),
            Task::NativeTemporaryOperation(Op::DICT_NEXT, std::rc::Rc::clone(info), vec![]),
            Task::Operation(Op::JUMP_FALSE4, vec![Operand::Label(iteration)]),
            operation(Op::POP),
            operation(Op::POP),
            Task::Label(finish.clone()),
            release(),
            Task::EndNativeCatchBranch,
            Task::Operation(Op::JUMP4, vec![Operand::Label(end.clone())]),
            Task::Label(handler),
            operation(Op::PUSH_RETURN_OPTS),
            operation(Op::PUSH_RESULT),
            release(),
            Task::EndNativeCatch,
            operation(Op::RETURN_STK),
            Task::Label(empty),
            operation(Op::POP),
            operation(Op::POP),
            release(),
            Task::Label(end),
            Task::Literal(Vec::new()),
        ]);
        self.inline_loop_regions
            .push(super::super::InlineLoopRegion {
                start: body_start,
                end: body_end,
                continue_target: Some(step),
                break_target: finish,
            });
        Some(tasks)
    }

    /// The catch handler that releases the iterator and re-raises, then the
    /// normal and `break` exits that release it and push the result.
    fn native_dictionary_each_exit_tasks(
        info: &Temporary,
        collect: Option<&Temporary>,
        handler: String,
        end: String,
        finish: String,
    ) -> Vec<Task> {
        let temporary = |op, slot: &Temporary, operands: Vec<Operand>| {
            Task::NativeTemporaryOperation(op, std::rc::Rc::clone(slot), operands)
        };
        let operation = |op| Task::Operation(op, vec![]);
        let mut tasks = vec![
            Task::Label(handler),
            operation(Op::PUSH_RETURN_OPTS),
            operation(Op::PUSH_RESULT),
            Task::EndNativeCatchBranch,
            temporary(Op::UNSET_SCALAR, info, vec![Operand::Imm(0)]),
        ];
        if let Some(collect) = collect {
            tasks.push(temporary(Op::UNSET_SCALAR, collect, vec![Operand::Imm(0)]));
        }
        tasks.extend([
            operation(Op::RETURN_STK),
            Task::Label(end),
            operation(Op::POP),
            operation(Op::POP),
            Task::Label(finish),
            Task::EndNativeCatch,
            temporary(Op::UNSET_SCALAR, info, vec![Operand::Imm(0)]),
        ]);
        match collect {
            Some(collect) => tasks.extend([
                temporary(Op::LOAD_SCALAR1, collect, vec![]),
                temporary(Op::UNSET_SCALAR, collect, vec![Operand::Imm(0)]),
            ]),
            None => tasks.push(Task::Literal(Vec::new())),
        }
        tasks
    }

    /// The leading local declarations of a control recipe, with the
    /// anonymous temporaries in declaration order.
    fn native_declaration_tasks(
        preparations: Vec<NativeControlPreparationStep>,
    ) -> (Vec<Task>, Vec<Temporary>) {
        let mut tasks = Vec::new();
        let mut temporaries = Vec::new();
        for step in preparations {
            match step {
                NativeControlPreparationStep::DeclareLocal(name) => {
                    tasks.push(Task::DeclareNamespaceLocal(name));
                }
                NativeControlPreparationStep::DeclareAnonymousLocal => {
                    let slot = std::rc::Rc::new(std::cell::Cell::new(None));
                    tasks.push(Task::DeclareNativeTemporary(std::rc::Rc::clone(&slot)));
                    temporaries.push(slot);
                }
                _ => break,
            }
        }
        (tasks, temporaries)
    }
}
