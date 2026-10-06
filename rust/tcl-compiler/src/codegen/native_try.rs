// SPDX-License-Identifier: AGPL-3.0-or-later
//! Selected Tcl try instructions retain protected bodies, original options and locals.

use super::{CodegenCtx, NativeEmissionTask, NativeNamespaceRollback, Op, Operand};
use tcl_registry::native_compilation::NativeCompiledBodyContext;
use tcl_registry::native_control_compilation::{
    NativeControlCompilation, NativeControlOutcome, NativeControlPreparationStep,
};
use tcl_registry::native_try_compilation::{
    NativeTryCondition, NativeTryHandler, NativeTryInstruction,
};

type Temporary = std::rc::Rc<std::cell::Cell<Option<usize>>>;

fn op(code: Op) -> NativeEmissionTask {
    NativeEmissionTask::Operation(code, vec![])
}
fn imm(code: Op, value: usize) -> NativeEmissionTask {
    NativeEmissionTask::Operation(code, vec![Operand::Imm(super::super::bytecode_imm(value))])
}
fn jump(code: Op, target: &str) -> NativeEmissionTask {
    NativeEmissionTask::Operation(code, vec![Operand::Label(target.to_owned())])
}
fn literal(bytes: &[u8]) -> NativeEmissionTask {
    NativeEmissionTask::Literal(bytes.to_vec())
}
fn local(code: Op, slot: &Temporary) -> NativeEmissionTask {
    NativeEmissionTask::NativeTemporaryOperation(code, std::rc::Rc::clone(slot), vec![])
}

struct TryEmitter<'a, 'b> {
    ctx: &'a mut CodegenCtx<'b>,
    image: tcl_lexer::SourceImage,
    config: tcl_lexer::LexerConfig,
    words: Vec<tcl_lexer::NativeWord>,
    instruction: NativeTryInstruction,
    tasks: Vec<NativeEmissionTask>,
    result: Temporary,
    options: Temporary,
    end: String,
    final_start: String,
    targets: Vec<String>,
}

impl TryEmitter<'_, '_> {
    fn c91(&self) -> bool {
        self.instruction.version >= tcl_dialect::TclVersion::V9_1
    }
    fn load(&self) -> Op {
        if self.c91() {
            Op::LOAD_SCALAR4
        } else {
            Op::LOAD_SCALAR1
        }
    }
    fn store(&self) -> Op {
        if self.c91() {
            Op::STORE_SCALAR4
        } else {
            Op::STORE_SCALAR1
        }
    }
    fn label(&mut self, prefix: &str) -> String {
        self.ctx.fresh_label(prefix)
    }
    fn body(
        &mut self,
        body: tcl_registry::native_control_instructions::NativeControlBody,
        protected: bool,
    ) {
        if body.script.is_some() {
            self.tasks.push(NativeEmissionTask::ControlScript(
                self.image.clone(),
                body,
                if protected {
                    NativeCompiledBodyContext::ExceptionRange
                } else {
                    NativeCompiledBodyContext::Inherit
                },
                self.config,
            ));
        } else {
            let saved = self.ctx.native_compilation;
            let entered = if protected {
                saved.with_inline_exception_range()
            } else {
                saved
            };
            self.tasks.extend([
                NativeEmissionTask::ControlContext(entered),
                CodegenCtx::native_namespace_word_task(&self.words, body.operand)
                    .expect("selected original try operand"),
                op(Op::EVAL_STK),
                NativeEmissionTask::ControlContext(saved),
            ]);
        }
    }
    fn store_result_options(&mut self) {
        self.tasks.extend([
            local(self.store(), &self.options),
            op(Op::POP),
            local(self.store(), &self.result),
            op(Op::POP),
        ]);
    }
    fn bindings(&mut self, handler: &NativeTryHandler) {
        for (name, source) in [
            (&handler.result, &self.result),
            (&handler.options, &self.options),
        ] {
            if let Some(name) = name {
                self.tasks.extend([
                    local(self.load(), source),
                    NativeEmissionTask::NamespaceLocalOperation(self.store(), name.clone()),
                    op(Op::POP),
                ]);
            }
        }
    }
    fn body_capture(&mut self) {
        let failed = self.label("native_try_body_error");
        let options = self.label("native_try_body_options");
        self.tasks
            .push(NativeEmissionTask::BeginNativeCatch(failed.clone()));
        self.body(self.instruction.body.clone(), true);
        if self.instruction.catches_success {
            self.tasks.push(literal(b"0"));
            self.tasks.push(if self.c91() {
                op(Op::SWAP)
            } else {
                imm(Op::REVERSE, 2)
            });
            self.tasks.push(jump(Op::JUMP4, &options));
        } else {
            self.tasks.push(NativeEmissionTask::EndNativeCatch);
            if self.instruction.finally.is_some() {
                self.tasks.extend([
                    local(self.store(), &self.result),
                    op(Op::POP),
                    literal(b"-level 0 -code 0"),
                    local(self.store(), &self.options),
                    op(Op::POP),
                ]);
                self.tasks.push(jump(Op::JUMP4, &self.final_start));
            } else {
                self.tasks.push(jump(Op::JUMP4, &self.end));
            }
        }
        self.tasks.extend([
            NativeEmissionTask::Label(failed),
            op(Op::PUSH_RETURN_CODE),
            op(Op::PUSH_RESULT),
            NativeEmissionTask::Label(options),
            op(Op::PUSH_RETURN_OPTS),
        ]);
        self.tasks.push(if self.instruction.catches_success {
            NativeEmissionTask::EndNativeCatch
        } else {
            NativeEmissionTask::EndNativeCatchBranch
        });
        self.store_result_options();
    }
    fn test_condition(&mut self, handler: &NativeTryHandler, next: &str) {
        self.tasks.push(op(Op::DUP));
        self.tasks.push(if self.c91() {
            NativeEmissionTask::PrivateInteger(i64::from(handler.condition.code()))
        } else {
            literal(handler.condition.code().to_string().as_bytes())
        });
        self.tasks.extend([op(Op::EQ), jump(Op::JUMP_FALSE4, next)]);
        if let NativeTryCondition::Trap(pattern) = &handler.condition {
            self.tasks.extend([
                local(self.load(), &self.options),
                literal(b"-errorcode"),
                imm(Op::DICT_GET, 1),
            ]);
            if self.c91() {
                self.tasks.extend([
                    NativeEmissionTask::PrivateList(
                        pattern.clone(),
                        tcl_syntax::native_string::NativeStringProtocol::C(
                            self.instruction.version,
                        ),
                    ),
                    imm(Op::ERROR_PREFIX_EQ, pattern.len()),
                ]);
            } else {
                self.tasks.push(NativeEmissionTask::Operation(
                    Op::LIST_RANGE_IMM,
                    vec![
                        Operand::Imm(0),
                        Operand::Imm(super::super::bytecode_imm(pattern.len() - 1)),
                    ],
                ));
                self.tasks.push(literal(
                    &tcl_syntax::list_result::NativeListResultSerialization::for_string_protocol(
                        tcl_syntax::native_string::NativeStringProtocol::C(
                            self.instruction.version,
                        ),
                    )
                    .render(pattern),
                ));
                self.tasks.push(op(Op::STR_EQ));
            }
            self.tasks.push(jump(Op::JUMP_FALSE4, next));
        }
        self.tasks.push(op(Op::POP));
    }
    fn no_final_handler(&mut self, index: usize, handler: &NativeTryHandler) {
        self.bindings(handler);
        if handler.body.is_none() {
            self.tasks
                .push(jump(Op::JUMP4, &self.targets[handler.target]));
            return;
        }
        self.tasks
            .push(NativeEmissionTask::Label(self.targets[index].clone()));
        if handler.empty_body {
            self.tasks
                .extend([literal(b""), jump(Op::JUMP4, &self.end)]);
            return;
        }
        let failed = self.label("native_try_handler_error");
        let no_during = self.label("native_try_handler_no_during");
        self.tasks
            .push(NativeEmissionTask::BeginNativeCatch(failed.clone()));
        self.body(handler.body.clone().expect("non-dash handler"), true);
        self.tasks.extend([
            NativeEmissionTask::EndNativeCatch,
            jump(Op::JUMP4, &self.end),
            NativeEmissionTask::Label(failed),
            op(Op::PUSH_RESULT),
            op(Op::PUSH_RETURN_OPTS),
            op(Op::PUSH_RETURN_CODE),
            NativeEmissionTask::EndNativeCatchBranch,
            literal(b"1"),
            op(Op::EQ),
            jump(Op::JUMP_FALSE4, &no_during),
        ]);
        if self.c91() {
            self.tasks.extend([
                literal(b"-during"),
                local(self.load(), &self.options),
                op(Op::DICT_PUT),
            ]);
        } else {
            self.tasks.extend([
                local(self.load(), &self.options),
                imm(Op::REVERSE, 2),
                local(self.store(), &self.options),
                op(Op::POP),
                literal(b"-during"),
                imm(Op::REVERSE, 2),
                NativeEmissionTask::NativeTemporaryOperation(
                    Op::DICT_SET,
                    std::rc::Rc::clone(&self.options),
                    vec![Operand::Imm(1)],
                ),
            ]);
        }
        self.tasks.push(NativeEmissionTask::Label(no_during));
        self.tasks.push(if self.c91() {
            op(Op::SWAP)
        } else {
            imm(Op::REVERSE, 2)
        });
        self.tasks
            .extend([op(Op::RETURN_STK), jump(Op::JUMP4, &self.end)]);
    }
    // Result replacement is inside the selected handler range; finally runs even
    // when an actual result/options setter raises an exception.
    fn final_handler(&mut self, index: usize, handler: &NativeTryHandler) {
        let has_body = handler.body.is_some();
        let binds = handler.result.is_some() || handler.options.is_some();
        if !has_body && !binds {
            self.tasks
                .push(jump(Op::JUMP4, &self.targets[handler.target]));
            return;
        }
        let failed = self.label("native_try_handler_error");
        let settled = self.label("native_try_handler_settled");
        let no_during = self.label("native_try_handler_no_during");
        self.tasks
            .push(NativeEmissionTask::BeginNativeCatch(failed.clone()));
        self.bindings(handler);
        if let Some(body) = handler.body.clone() {
            let direct = self.label("native_try_handler_body");
            // Dash clauses enter the body without the target clause's setters.
            self.tasks.extend([
                jump(Op::JUMP4, &direct),
                NativeEmissionTask::Label(self.targets[index].clone()),
                NativeEmissionTask::BeginNativeCatchBranch(failed.clone()),
                NativeEmissionTask::Label(direct),
            ]);
            self.body(body, true);
            if self.instruction.numeric_handler_table {
                self.tasks.extend([
                    op(Op::PUSH_RETURN_OPTS),
                    NativeEmissionTask::EndNativeCatch,
                    jump(Op::JUMP4, &settled),
                ]);
            } else {
                self.tasks.extend([
                    literal(b"0"),
                    op(Op::PUSH_RETURN_OPTS),
                    imm(Op::REVERSE, 3),
                    jump(Op::JUMP4, &settled),
                ]);
            }
        } else {
            self.tasks.extend([
                NativeEmissionTask::EndNativeCatch,
                jump(Op::JUMP4, &self.targets[handler.target]),
            ]);
        }
        self.tasks.push(NativeEmissionTask::Label(failed));
        self.capture_handler_replacement(has_body, settled, no_during);
        self.tasks.push(jump(Op::JUMP4, &self.final_start));
    }
    fn capture_handler_replacement(&mut self, has_body: bool, settled: String, no_during: String) {
        if self.instruction.numeric_handler_table {
            self.tasks.extend([
                op(Op::PUSH_RESULT),
                op(Op::PUSH_RETURN_OPTS),
                op(Op::PUSH_RETURN_CODE),
                NativeEmissionTask::EndNativeCatchBranch,
                literal(b"1"),
                op(Op::EQ),
                jump(Op::JUMP_FALSE4, &no_during),
                literal(b"-during"),
                local(self.load(), &self.options),
                op(Op::DICT_PUT),
                NativeEmissionTask::Label(no_during),
                NativeEmissionTask::Label(settled),
            ]);
            self.store_result_options();
        } else {
            self.tasks.extend([
                op(Op::PUSH_RETURN_OPTS),
                op(Op::PUSH_RETURN_CODE),
                op(Op::PUSH_RESULT),
                NativeEmissionTask::Label(settled),
            ]);
            self.tasks.push(if has_body {
                NativeEmissionTask::EndNativeCatch
            } else {
                NativeEmissionTask::EndNativeCatchBranch
            });
            self.tasks.extend([
                local(self.store(), &self.result),
                op(Op::POP),
                literal(b"1"),
                op(Op::EQ),
                jump(Op::JUMP_FALSE4, &no_during),
            ]);
            if self.c91() {
                self.tasks.extend([
                    literal(b"-during"),
                    local(self.load(), &self.options),
                    op(Op::DICT_PUT),
                    NativeEmissionTask::Label(no_during),
                    local(self.store(), &self.options),
                    op(Op::POP),
                ]);
            } else {
                let done = self.label("native_try_handler_saved");
                self.tasks.extend([
                    local(self.load(), &self.options),
                    literal(b"-during"),
                    imm(Op::REVERSE, 3),
                    local(self.store(), &self.options),
                    op(Op::POP),
                    NativeEmissionTask::NativeTemporaryOperation(
                        Op::DICT_SET,
                        std::rc::Rc::clone(&self.options),
                        vec![Operand::Imm(1)],
                    ),
                    jump(Op::JUMP4, &done),
                    NativeEmissionTask::Label(no_during),
                    local(self.store(), &self.options),
                    NativeEmissionTask::Label(done),
                    op(Op::POP),
                ]);
            }
        }
    }
    fn handler_table(
        &mut self,
        handlers: &[NativeTryHandler],
        unmatched: &str,
    ) -> Option<Vec<String>> {
        use NativeEmissionTask as Task;
        if self.instruction.numeric_handler_table {
            let labels: Vec<_> = (0..handlers.len())
                .map(|_| self.label("native_try_case"))
                .collect();
            let mut entries = std::collections::HashMap::new();
            for (handler, label) in handlers.iter().zip(&labels) {
                entries
                    .entry(i64::from(handler.condition.code()))
                    .or_insert_with(|| label.clone());
            }
            self.tasks.extend([
                Task::SwitchTable(
                    self.instruction.version,
                    true,
                    std::collections::HashMap::new(),
                    entries,
                ),
                jump(
                    Op::JUMP4,
                    if self.instruction.finally.is_some() {
                        &self.final_start
                    } else {
                        unmatched
                    },
                ),
            ]);
            Some(labels)
        } else {
            None
        }
    }
    fn final_body(&mut self) {
        let Some(body) = self.instruction.finally.clone() else {
            return;
        };
        let failed = self.label("native_try_final_error");
        let no_during = self.label("native_try_final_no_during");
        let done = self.label("native_try_final_saved");
        self.tasks
            .push(NativeEmissionTask::Label(self.final_start.clone()));
        self.tasks
            .push(NativeEmissionTask::BeginNativeCatch(failed.clone()));
        self.body(body, true);
        self.tasks.extend([
            NativeEmissionTask::EndNativeCatch,
            op(Op::POP),
            jump(Op::JUMP4, &done),
            NativeEmissionTask::Label(failed),
            op(Op::PUSH_RESULT),
            op(Op::PUSH_RETURN_OPTS),
            op(Op::PUSH_RETURN_CODE),
            NativeEmissionTask::EndNativeCatchBranch,
            literal(b"1"),
            op(Op::EQ),
            jump(Op::JUMP_FALSE4, &no_during),
        ]);
        if self.c91() {
            self.tasks.extend([
                literal(b"-during"),
                local(self.load(), &self.options),
                op(Op::DICT_PUT),
                NativeEmissionTask::Label(no_during),
                local(self.store(), &self.options),
                op(Op::POP),
            ]);
        } else {
            let save_result = self.label("native_try_final_result");
            self.tasks.extend([
                local(self.load(), &self.options),
                literal(b"-during"),
                imm(Op::REVERSE, 3),
                local(self.store(), &self.options),
                op(Op::POP),
                NativeEmissionTask::NativeTemporaryOperation(
                    Op::DICT_SET,
                    std::rc::Rc::clone(&self.options),
                    vec![Operand::Imm(1)],
                ),
                op(Op::POP),
                jump(Op::JUMP4, &save_result),
                NativeEmissionTask::Label(no_during),
                local(self.store(), &self.options),
                op(Op::POP),
                NativeEmissionTask::Label(save_result),
            ]);
        }
        self.tasks.extend([
            local(self.store(), &self.result),
            op(Op::POP),
            NativeEmissionTask::Label(done),
            local(self.load(), &self.options),
            local(self.load(), &self.result),
            op(Op::RETURN_STK),
        ]);
    }
    fn finally_only(&mut self) {
        let body_error = self.label("native_try_body_error");
        let body_options = self.label("native_try_body_options");
        let final_error = self.label("native_try_final_error");
        let no_during = self.label("native_try_final_no_during");
        let final_ok = self.label("native_try_final_ok");
        self.tasks
            .push(NativeEmissionTask::BeginNativeCatch(body_error.clone()));
        self.body(self.instruction.body.clone(), true);
        self.tasks.extend([
            jump(Op::JUMP4, &body_options),
            NativeEmissionTask::Label(body_error),
            op(Op::PUSH_RESULT),
            NativeEmissionTask::Label(body_options),
            op(Op::PUSH_RETURN_OPTS),
            NativeEmissionTask::EndNativeCatch,
            NativeEmissionTask::BeginNativeCatch(final_error.clone()),
        ]);
        self.body(
            self.instruction.finally.clone().expect("finally-only"),
            true,
        );
        self.tasks
            .extend([NativeEmissionTask::EndNativeCatch, op(Op::POP)]);
        if self.c91() {
            self.tasks.push(op(Op::SWAP));
        }
        self.tasks.extend([
            jump(Op::JUMP4, &final_ok),
            NativeEmissionTask::Label(final_error),
            op(Op::PUSH_RESULT),
            op(Op::PUSH_RETURN_OPTS),
            op(Op::PUSH_RETURN_CODE),
            NativeEmissionTask::EndNativeCatchBranch,
            literal(b"1"),
            op(Op::EQ),
            jump(Op::JUMP_FALSE4, &no_during),
            literal(b"-during"),
            imm(Op::OVER, 3),
        ]);
        if self.c91() {
            self.tasks.push(op(Op::DICT_PUT));
        } else {
            self.tasks.extend([imm(Op::LIST, 2), op(Op::LIST_CONCAT)]);
        }
        self.tasks.extend([
            NativeEmissionTask::Label(no_during),
            imm(Op::REVERSE, 4),
            op(Op::POP),
            op(Op::POP),
        ]);
        if !self.c91() {
            self.tasks.push(jump(Op::JUMP4, &self.end));
        }
        self.tasks.push(NativeEmissionTask::Label(final_ok));
        if !self.c91() {
            self.tasks.push(imm(Op::REVERSE, 2));
            self.tasks.push(NativeEmissionTask::Label(self.end.clone()));
        }
        self.tasks.push(op(Op::RETURN_STK));
    }
}

impl CodegenCtx<'_> {
    fn native_try_decline_tasks(
        &mut self,
        command: &tcl_lexer::NativeScriptCommandWords,
        preparations: Vec<NativeControlPreparationStep>,
    ) -> Option<Vec<NativeEmissionTask>> {
        use NativeEmissionTask as Task;
        let saved = NativeNamespaceRollback {
            instructions: self.instructions.len(),
            labels: self.label_positions.clone(),
            loop_regions: self.inline_loop_regions.len(),
            command_index: self.cmd_index,
        };
        let mut tasks = Vec::new();
        for visit in preparations {
            tasks.push(match visit {
                NativeControlPreparationStep::DeclareLocal(name) => {
                    Task::DeclareNamespaceLocal(name)
                }
                NativeControlPreparationStep::DeclareAnonymousLocal => {
                    Task::DeclareNativeTemporary(std::rc::Rc::new(std::cell::Cell::new(None)))
                }
                NativeControlPreparationStep::Literal(bytes) => Task::Literal(bytes),
                NativeControlPreparationStep::Integer(value) => Task::PrivateInteger(value),
                NativeControlPreparationStep::List(members) => Task::PrivateList(
                    members,
                    tcl_syntax::native_string::NativeStringProtocol::C(
                        self.native_entry?.execution_point?.tcl_version()?,
                    ),
                ),
                _ => return None,
            });
        }
        tasks.push(Task::NamespaceGenericRollback(saved, command.clone()));
        Some(tasks)
    }
    pub(super) fn native_try_tasks(
        &mut self,
        command: &tcl_lexer::NativeScriptCommandWords,
        recipe: NativeControlCompilation<NativeTryInstruction>,
    ) -> Option<Vec<NativeEmissionTask>> {
        use NativeEmissionTask as Task;
        let image = command.words.first()?.image().clone();
        let config = command.words[0].config();
        let NativeControlOutcome::Inline(instruction) = recipe.outcome else {
            return self.native_try_decline_tasks(command, recipe.preparations);
        };
        let end = self.fresh_label("native_try_end");
        let final_start = self.fresh_label("native_try_final");
        let targets = (0..instruction.handlers.len())
            .map(|_| self.fresh_label("native_try_target"))
            .collect();
        let result = std::rc::Rc::new(std::cell::Cell::new(None));
        let options = std::rc::Rc::new(std::cell::Cell::new(None));
        let mut emitter = TryEmitter {
            ctx: self,
            image,
            config,
            words: command.words.clone(),
            instruction,
            tasks: Vec::new(),
            result,
            options,
            end,
            final_start,
            targets,
        };
        for handler in &emitter.instruction.handlers {
            for name in handler.result.iter().chain(handler.options.iter()) {
                emitter
                    .tasks
                    .push(Task::DeclareNamespaceLocal(name.clone()));
            }
        }
        if emitter.instruction.handlers.is_empty() {
            if emitter.instruction.finally.is_some() {
                emitter.finally_only();
            } else {
                emitter.body(emitter.instruction.body.clone(), false);
            }
            return Some(emitter.tasks);
        }
        emitter.tasks.extend([
            Task::DeclareNativeTemporary(std::rc::Rc::clone(&emitter.result)),
            Task::DeclareNativeTemporary(std::rc::Rc::clone(&emitter.options)),
        ]);
        emitter.body_capture();
        let handlers = emitter.instruction.handlers.clone();
        let unmatched = emitter.label("native_try_unmatched");
        let table = emitter.handler_table(&handlers, &unmatched);
        for (index, handler) in handlers.iter().enumerate() {
            let next = emitter.label("native_try_next");
            if let Some(table) = &table {
                emitter.tasks.push(Task::Label(table[index].clone()));
            } else {
                emitter.test_condition(handler, &next);
            }
            if emitter.instruction.finally.is_some() {
                emitter.final_handler(index, handler);
            } else {
                emitter.no_final_handler(index, handler);
            }
            if table.is_none() {
                emitter.tasks.push(Task::Label(next));
            }
        }
        if table.is_none() {
            emitter.tasks.push(op(Op::POP));
        }
        if emitter.instruction.finally.is_some() {
            emitter.final_body();
        } else {
            if table.is_some() {
                emitter.tasks.push(Task::Label(unmatched));
            }
            emitter.tasks.extend([
                local(emitter.load(), &emitter.options),
                local(emitter.load(), &emitter.result),
                op(Op::RETURN_STK),
                Task::Label(emitter.end.clone()),
            ]);
        }
        Some(emitter.tasks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protected_try_emits_native_return_and_real_unnamed_slots() {
        for name in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let profile = tcl_dialect::DialectProfile::find(name).unwrap();
            let registry =
                tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
            let entry = crate::environment_ingress::captured_native_entry(profile);
            let image = tcl_lexer::SourceImage::native(&b"try {return VALUE} on error {message options} {set handled $message} finally {set cleaned 1}"[..]);
            let script = tcl_lexer::native_script_words_in(
                image.clone(),
                tcl_lexer::Span::new(0, u32::try_from(image.len()).unwrap()),
                tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            )
            .unwrap();
            let mut context = CodegenCtx::new(true, &[], &registry);
            context.native_entry = Some(&entry);
            context.invocation_dialect = Some(tcl_registry::InvocationDialect::of_profile(profile));
            context.source_string_protocol = entry.source_string_protocol;
            context.native_compilation.frame =
                tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode;
            context.emit_native_words(&script.commands[0].words);
            assert!(!context.native_dependency_refusal, "{name}");
            assert!(
                context
                    .instructions
                    .iter()
                    .any(|instruction| instruction.op == Op::RETURN_IMM),
                "protected return must not become DONE: {name}"
            );
            assert!(
                context
                    .instructions
                    .iter()
                    .any(|instruction| instruction.op == Op::RETURN_STK)
            );
            assert!(
                !context
                    .instructions
                    .iter()
                    .any(|instruction| matches!(instruction.op, Op::INVOKE_STK1 | Op::INVOKE_STK4)),
                "{name}"
            );
            let names = context.lvt.native_slot_names();
            let slots: Vec<_> = names
                .iter()
                .map(|name| name.as_ref().map(tcl_runtime_api::NameBytes::as_bytes))
                .collect();
            assert_eq!(
                slots,
                [
                    Some(&b"message"[..]),
                    Some(&b"options"[..]),
                    None,
                    None,
                    Some(&b"handled"[..]),
                    Some(&b"cleaned"[..])
                ],
                "{name}"
            );
            assert_eq!(context.catch_depth, 0);
            assert!(
                context
                    .instructions
                    .iter()
                    .filter(|instruction| instruction.op == Op::BEGIN_CATCH4)
                    .all(|instruction| instruction.catch_target.is_some())
            );
            if name == "tcl9.1" {
                assert!(
                    context
                        .instructions
                        .iter()
                        .any(|instruction| instruction.native_switch_integers.is_some())
                );
                assert!(
                    context
                        .instructions
                        .iter()
                        .any(|instruction| instruction.op == Op::DICT_PUT)
                );
            } else {
                assert!(
                    context
                        .instructions
                        .iter()
                        .any(|instruction| instruction.op == Op::DICT_SET)
                );
            }
        }
    }
}
