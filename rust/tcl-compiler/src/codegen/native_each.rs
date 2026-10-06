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
}
