// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original dictionary scope auxiliary slots and protected body emission.

use super::{CodegenCtx, NativeEmissionTask as Task, NativeNamespaceRollback, Op, Operand};
use tcl_registry::native_compilation::NativeCompiledBodyContext;
use tcl_registry::native_control_compilation::{
    NativeControlCompilation, NativeControlOutcome, NativeControlPreparationStep as Visit,
};
use tcl_registry::native_control_instructions::NativeControlBody;
use tcl_registry::native_dictionary_scope_compilation::{
    NativeDictionaryScopeInstruction, NativeDictionaryScopeKind as Kind,
    NativeDictionaryScopeReceiver as Receiver,
};

type Temporary = std::rc::Rc<std::cell::Cell<Option<usize>>>;

fn operation(op: Op, immediates: &[i32]) -> Task {
    Task::Operation(op, immediates.iter().copied().map(Operand::Imm).collect())
}
fn temporary(op: Op, slot: &Temporary) -> Task {
    Task::NativeTemporaryOperation(op, std::rc::Rc::clone(slot), Vec::new())
}
fn path_tasks(tasks: &mut Vec<Task>, path: Option<&Temporary>) {
    if let Some(slot) = path {
        tasks.push(temporary(Op::LOAD_SCALAR1, slot));
    } else {
        tasks.push(Task::Literal(Vec::new()));
    }
}
fn recombine(
    tasks: &mut Vec<Task>,
    receiver: &Receiver,
    name: Option<&Temporary>,
    path: Option<&Temporary>,
    keys: &Temporary,
) {
    if let Some(name) = name {
        tasks.push(temporary(Op::LOAD_SCALAR1, name));
    }
    path_tasks(tasks, path);
    tasks.push(temporary(Op::LOAD_SCALAR1, keys));
    tasks.push(match receiver {
        Receiver::Local(name) => {
            Task::NamespaceLocalOperation(Op::DICT_RECOMBINE_IMM, name.clone())
        }
        Receiver::Stack(_) => operation(Op::DICT_RECOMBINE_STK, &[]),
    });
}

struct NativeScopeBodyTasks {
    body: Task,
    handler: String,
    end: String,
}

impl CodegenCtx<'_> {
    pub(super) fn native_preparation_receipt_tasks(visits: &[Visit]) -> Option<Vec<Task>> {
        visits
            .iter()
            .map(|visit| match visit {
                Visit::DeclareLocal(name) => Some(Task::DeclareNamespaceLocal(name.clone())),
                Visit::DeclareAnonymousLocal => Some(Task::DeclareNativeTemporary(
                    std::rc::Rc::new(std::cell::Cell::new(None)),
                )),
                Visit::Literal(bytes) => Some(Task::NativePreparationLiteral(bytes.clone())),
                _ => None,
            })
            .collect()
    }

    pub(super) fn native_generic_preparation_tasks(
        &self,
        command: &tcl_lexer::NativeScriptCommandWords,
        visits: &[Visit],
    ) -> Option<Vec<Task>> {
        let mut tasks = Self::native_preparation_receipt_tasks(visits)?;
        let saved = NativeNamespaceRollback {
            instructions: self.instructions.len(),
            labels: self.label_positions.clone(),
            loop_regions: self.inline_loop_regions.len(),
            command_index: self.cmd_index,
        };
        tasks.push(Task::NamespaceGenericRollback(saved, command.clone()));
        Some(tasks)
    }

    pub(super) fn native_dictionary_scope_tasks(
        &mut self,
        command: &tcl_lexer::NativeScriptCommandWords,
        selected: NativeControlCompilation<NativeDictionaryScopeInstruction>,
    ) -> Option<Vec<Task>> {
        let mut tasks = Vec::new();
        let mut temporaries = Vec::new();
        for visit in &selected.preparations {
            match visit {
                Visit::DeclareLocal(name) => tasks.push(Task::DeclareNamespaceLocal(name.clone())),
                Visit::DeclareAnonymousLocal => {
                    let slot = std::rc::Rc::new(std::cell::Cell::new(None));
                    tasks.push(Task::DeclareNativeTemporary(std::rc::Rc::clone(&slot)));
                    temporaries.push(slot);
                }
                _ => break,
            }
        }
        let recipe = match selected.outcome {
            NativeControlOutcome::Inline(recipe) => recipe,
            NativeControlOutcome::Generic => {
                let saved = NativeNamespaceRollback {
                    instructions: self.instructions.len(),
                    labels: self.label_positions.clone(),
                    loop_regions: self.inline_loop_regions.len(),
                    command_index: self.cmd_index,
                };
                tasks.push(Task::NamespaceGenericRollback(saved, command.clone()));
                return Some(tasks);
            }
            NativeControlOutcome::Rejected(_) => return None,
        };
        let original = command.words.first()?;
        let body = Task::ControlScript(
            original.image().clone(),
            NativeControlBody {
                operand: recipe.body.clone(),
                script: Some(recipe.body_span),
            },
            NativeCompiledBodyContext::ExceptionRange,
            original.config(),
        );
        let handler = self.fresh_label("native_dictionary_cleanup");
        let end = self.fresh_label("native_dictionary_end");
        let finish = NativeScopeBodyTasks { body, handler, end };
        match &recipe.kind {
            Kind::Update { .. } => {
                Self::native_dictionary_update_tasks(command, &recipe, &mut tasks, finish)?;
            }
            Kind::With {
                empty_body: true, ..
            } => Self::native_dictionary_empty_with_tasks(command, &recipe, &mut tasks)?,
            Kind::With { .. } => Self::native_dictionary_with_tasks(
                command,
                &recipe,
                &mut tasks,
                &temporaries,
                finish,
            )?,
        }
        Some(tasks)
    }

    fn native_dictionary_update_tasks(
        command: &tcl_lexer::NativeScriptCommandWords,
        recipe: &NativeDictionaryScopeInstruction,
        tasks: &mut Vec<Task>,
        finish: NativeScopeBodyTasks,
    ) -> Option<()> {
        let Kind::Update { keys, targets } = &recipe.kind else {
            return None;
        };
        let NativeScopeBodyTasks { body, handler, end } = finish;
        let Receiver::Local(root) = &recipe.receiver else {
            return None;
        };
        for key in keys {
            tasks.push(Self::native_namespace_word_task(
                &command.words,
                key.clone(),
            )?);
        }
        tasks.push(operation(
            Op::LIST,
            &[super::super::bytecode_imm(keys.len())],
        ));
        tasks.push(Task::NativeDictionaryUpdateOperation(
            Op::DICT_UPDATE_START,
            root.clone(),
            targets.clone(),
        ));
        tasks.push(Task::BeginNativeCatch(handler.clone()));
        tasks.push(body);
        tasks.push(Task::EndNativeCatch);
        tasks.push(operation(Op::REVERSE, &[2]));
        tasks.push(Task::NativeDictionaryUpdateOperation(
            Op::DICT_UPDATE_END,
            root.clone(),
            targets.clone(),
        ));
        tasks.push(Task::Operation(
            Op::JUMP4,
            vec![Operand::Label(end.clone())],
        ));
        tasks.push(Task::Label(handler));
        tasks.push(operation(Op::PUSH_RESULT, &[]));
        tasks.push(operation(Op::PUSH_RETURN_OPTS, &[]));
        tasks.push(Task::EndNativeCatchBranch);
        tasks.push(operation(Op::REVERSE, &[3]));
        tasks.push(Task::NativeDictionaryUpdateOperation(
            Op::DICT_UPDATE_END,
            root.clone(),
            targets.clone(),
        ));
        tasks.push(operation(Op::RETURN_STK, &[]));
        tasks.push(Task::Label(end));
        Some(())
    }

    fn native_dictionary_empty_with_tasks(
        command: &tcl_lexer::NativeScriptCommandWords,
        recipe: &NativeDictionaryScopeInstruction,
        tasks: &mut Vec<Task>,
    ) -> Option<()> {
        let Kind::With { path, .. } = &recipe.kind else {
            return None;
        };
        if let Receiver::Stack(receiver) = &recipe.receiver {
            tasks.push(Self::native_namespace_word_task(
                &command.words,
                receiver.clone(),
            )?);
        }
        for operand in path {
            tasks.push(Self::native_namespace_word_task(
                &command.words,
                operand.clone(),
            )?);
        }
        if path.is_empty() {
            match &recipe.receiver {
                Receiver::Local(name) => {
                    tasks.push(Task::Literal(Vec::new()));
                    tasks.push(Task::NamespaceLocalOperation(
                        Op::LOAD_SCALAR1,
                        name.clone(),
                    ));
                }
                Receiver::Stack(_) => {
                    tasks.push(operation(Op::DUP, &[]));
                    tasks.push(operation(Op::LOAD_STK, &[]));
                }
            }
            tasks.push(Task::Literal(Vec::new()));
            tasks.push(operation(Op::DICT_EXPAND, &[]));
            if matches!(recipe.receiver, Receiver::Stack(_)) {
                tasks.push(Task::Literal(Vec::new()));
                tasks.push(operation(Op::REVERSE, &[2]));
            }
        } else {
            tasks.push(operation(
                Op::LIST,
                &[super::super::bytecode_imm(path.len())],
            ));
            match &recipe.receiver {
                Receiver::Local(name) => tasks.push(Task::NamespaceLocalOperation(
                    Op::LOAD_SCALAR1,
                    name.clone(),
                )),
                Receiver::Stack(_) => {
                    tasks.push(operation(Op::OVER, &[1]));
                    tasks.push(operation(Op::LOAD_STK, &[]));
                }
            }
            tasks.push(operation(Op::OVER, &[1]));
            tasks.push(operation(Op::DICT_EXPAND, &[]));
        }
        tasks.push(match &recipe.receiver {
            Receiver::Local(name) => {
                Task::NamespaceLocalOperation(Op::DICT_RECOMBINE_IMM, name.clone())
            }
            Receiver::Stack(_) => operation(Op::DICT_RECOMBINE_STK, &[]),
        });
        tasks.push(Task::Literal(Vec::new()));
        Some(())
    }

    fn native_dictionary_with_tasks(
        command: &tcl_lexer::NativeScriptCommandWords,
        recipe: &NativeDictionaryScopeInstruction,
        tasks: &mut Vec<Task>,
        temporaries: &[Temporary],
        finish: NativeScopeBodyTasks,
    ) -> Option<()> {
        let Kind::With { path, .. } = &recipe.kind else {
            return None;
        };
        let NativeScopeBodyTasks { body, handler, end } = finish;
        let mut slots = temporaries.iter();
        let name = if let Receiver::Stack(receiver) = &recipe.receiver {
            let slot = slots.next()?;
            tasks.push(Self::native_namespace_word_task(
                &command.words,
                receiver.clone(),
            )?);
            tasks.push(temporary(Op::STORE_SCALAR1, slot));
            Some(slot)
        } else {
            None
        };
        let path_slot = if path.is_empty() {
            None
        } else {
            let slot = slots.next()?;
            for operand in path {
                tasks.push(Self::native_namespace_word_task(
                    &command.words,
                    operand.clone(),
                )?);
            }
            tasks.push(operation(
                Op::LIST,
                &[super::super::bytecode_imm(path.len())],
            ));
            tasks.push(temporary(Op::STORE_SCALAR1, slot));
            tasks.push(operation(Op::POP, &[]));
            Some(slot)
        };
        let keys = slots.next()?;
        if slots.next().is_some() {
            return None;
        }
        tasks.push(match &recipe.receiver {
            Receiver::Local(name) => Task::NamespaceLocalOperation(Op::LOAD_SCALAR1, name.clone()),
            Receiver::Stack(_) => operation(Op::LOAD_STK, &[]),
        });
        path_tasks(tasks, path_slot);
        tasks.push(operation(Op::DICT_EXPAND, &[]));
        tasks.push(temporary(Op::STORE_SCALAR1, keys));
        tasks.push(operation(Op::POP, &[]));
        tasks.push(Task::BeginNativeCatch(handler.clone()));
        tasks.push(body);
        tasks.push(Task::EndNativeCatch);
        recombine(tasks, &recipe.receiver, name, path_slot, keys);
        tasks.push(Task::Operation(
            Op::JUMP4,
            vec![Operand::Label(end.clone())],
        ));
        tasks.push(Task::Label(handler));
        tasks.push(operation(Op::PUSH_RETURN_OPTS, &[]));
        tasks.push(operation(Op::PUSH_RESULT, &[]));
        tasks.push(Task::EndNativeCatchBranch);
        recombine(tasks, &recipe.receiver, name, path_slot, keys);
        tasks.push(operation(Op::RETURN_STK, &[]));
        tasks.push(Task::Label(end));
        Some(())
    }

    pub(super) fn emit_native_dictionary_update_task(
        &mut self,
        op: Op,
        root: &[u8],
        targets: &[Vec<u8>],
    ) {
        let Some(protocol) = self.compiled_variable_protocol else {
            self.refuse_native_dependency();
            return;
        };
        let Some(root) = self.lvt.find_native(protocol, root) else {
            self.refuse_native_dependency();
            return;
        };
        let Some(slots) = targets
            .iter()
            .map(|name| self.lvt.find_native(protocol, name))
            .collect::<Option<Vec<_>>>()
        else {
            self.refuse_native_dependency();
            return;
        };
        let instruction = self.emit(
            op,
            vec![
                Operand::Imm(super::super::bytecode_imm(root)),
                Operand::Imm(0),
            ],
        );
        self.instructions[instruction].dict_vars = Some(slots);
    }
}
