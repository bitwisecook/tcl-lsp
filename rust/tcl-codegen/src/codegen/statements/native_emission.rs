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

//! The native emission work list: its entry points for original words and expressions, and the emitter of each task kind.

use super::super::{CodegenCtx, Op, Operand};
use super::{NativeEmissionTask, authored_name_policy};

impl CodegenCtx<'_> {
    /// Use retained original words when the selected compiler owns preparation
    /// before handler dispatch, including its local and literal allocations.
    pub(in super::super) fn emit_retained_native_expression(&mut self) -> bool {
        let tokens = self.invocation_tokens.clone();
        let Some(binding) = tokens
            .as_ref()
            .and_then(|tokens| tokens.source_binding.as_ref())
        else {
            return false;
        };
        if self.plain_command_dispatch {
            return false;
        }
        let Some((image, words, offset)) = binding.retained_original_compiler_source() else {
            return false;
        };
        let Some(dialect) = binding.compiler_word_dialect() else {
            return false;
        };
        let Some(original) = tcl_compiler::registry_invocation::original_native_compiler_words(
            image,
            words,
            offset,
            tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
        ) else {
            return false;
        };
        let Some(entry) = self.native_entry else {
            return false;
        };
        if self.emit_authored_original_expression(binding, &original) {
            return true;
        }
        let Ok(tcl_compiler::native_byte_compilation::NativeByteCommandPlan::Registered(plan)) =
            tcl_compiler::native_byte_compilation::native_byte_command_plan(
                &original,
                entry,
                self.registry,
                self.native_compilation,
            )
        else {
            return false;
        };
        if plan.spec.grammar
            != tcl_registry::native_compilation::NativeCompilationGrammar::Expression
        {
            return false;
        }
        self.emit_native_words(&original);
        true
    }

    /// Authored evaluation consumes unchanged source under its explicit policy;
    /// the original compiler registration still owns before-argument selection.
    fn emit_authored_original_expression(
        &mut self,
        binding: &tcl_compiler::command_binding::SourceInvocationBinding,
        original: &[tcl_lexer::NativeWord],
    ) -> bool {
        use tcl_registry::native_compiler_words::NativeCompilerWords;
        use tcl_registry::native_expression_program::{
            ExpressionProgramEmission, expression_program_emission, native_expression_instruction,
        };
        let Some(proof) = binding.admitted_inline_invocation() else {
            return false;
        };
        if proof.operation
            != tcl_registry::SemanticOperationId::StructuredLowering(
                tcl_registry::hooks::LoweringHookId::Expr,
            )
        {
            return false;
        }
        let Some(required) = proof.compiler_prerequisite.clone() else {
            return false;
        };
        let Some(entry) = self.native_entry else {
            return false;
        };
        let (Some(protocol), Some(point)) = (entry.source_string_protocol, entry.execution_point)
        else {
            return false;
        };
        let Ok(words) = NativeCompilerWords::capture(original, protocol) else {
            return false;
        };
        let Ok(recipe) = native_expression_instruction(
            &words,
            1,
            tcl_registry::InvocationDialect::of_point(point),
        ) else {
            return false;
        };
        let Some(program) = recipe.program.filter(|program| {
            expression_program_emission(program, entry) == ExpressionProgramEmission::AuthoredSource
        }) else {
            return false;
        };
        let Some(namespace) = entry
            .namespaces
            .iter()
            .find(|namespace| namespace.token == required.lookup_namespace_token)
        else {
            return false;
        };
        let (Some(first), Some(last)) = (original.first(), original.last()) else {
            return false;
        };
        let command = tcl_lexer::NativeScriptCommandWords {
            span: tcl_lexer::Span::new(first.span().start(), last.span().end()),
            source_extent: None,
            terminator: None,
            words: original.to_vec(),
        };
        let end = self.begin_original_compiler_selection(
            &command,
            namespace.path.clone(),
            tcl_runtime_api::native_compilation::NativeCompilerSelectionPrerequisite::from_command_registration(required),
        );
        self.push_lit_bytes_exact(&program.source);
        self.emit(Op::EXPR_STK, vec![]);
        self.place_label(&end);
        true
    }

    pub(in super::super) fn emit_native_original_preparation_invocation(
        &mut self,
        tokens: Option<&tcl_compiler::ir::CommandTokens>,
    ) -> bool {
        if self.plain_command_dispatch {
            return false;
        }
        let original = self.invocation_tokens.clone();
        let Some(tokens) = tokens.or(original.as_deref()) else {
            return false;
        };
        let Some(binding) = tokens.source_binding.as_ref() else {
            return false;
        };
        let Some((image, words, offset)) = binding.original_compiler_source(tokens) else {
            return false;
        };
        let Some(dialect) = binding.compiler_word_dialect() else {
            return false;
        };
        let Some(original) = tcl_compiler::registry_invocation::original_native_compiler_words(
            image,
            words,
            offset,
            tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
        ) else {
            return false;
        };
        let Some(entry) = self.native_entry else {
            return false;
        };
        let Ok(tcl_compiler::native_byte_compilation::NativeByteCommandPlan::Registered(plan)) =
            tcl_compiler::native_byte_compilation::native_byte_command_plan(
                &original,
                entry,
                self.registry,
                self.native_compilation,
            )
        else {
            return false;
        };
        if !plan.spec.requires_original_word_preparation_of(
            original
                .get(plan.operand_from..)
                .is_some_and(|words| words.iter().any(|word| word.group().expand)),
        ) {
            return false;
        }
        self.emit_native_words(&original);
        true
    }

    /// Emit one original command through the selected native compiler owner.
    pub(in super::super) fn emit_native_words(&mut self, words: &[tcl_lexer::NativeWord]) {
        let Some(first) = words.first() else {
            return;
        };
        let command = tcl_lexer::NativeScriptCommandWords {
            span: tcl_lexer::Span::new(first.span().start(), words.last().unwrap().span().end()),
            source_extent: None,
            terminator: None,
            words: words.to_vec(),
        };
        self.run_native_emission(vec![NativeEmissionTask::Command(command)]);
    }

    pub(in super::super) fn emit_executable_arena(
        &mut self,
        arena: &tcl_lexer::ExecutablePartArena,
    ) {
        let arena = std::rc::Rc::new(arena.clone());
        let root = arena.root();
        self.run_native_emission(vec![NativeEmissionTask::List(arena, root, 0)]);
    }

    pub(in super::super) fn emit_substitution_component(
        &mut self,
        arena: &std::rc::Rc<tcl_lexer::ExecutablePartArena>,
        position: usize,
    ) {
        self.run_native_emission(vec![NativeEmissionTask::Part(
            std::rc::Rc::clone(arena),
            arena.root(),
            position,
        )]);
    }

    /// Emit one original lexical word through the same heap work list.
    pub(in super::super) fn emit_original_native_word(&mut self, word: &tcl_lexer::NativeWord) {
        self.run_native_emission(vec![NativeEmissionTask::Word(word.clone())]);
    }

    pub(in super::super) fn emit_native_original_preparation_receipt(
        &mut self,
        visits: &[tcl_registry::native_control_compilation::NativeControlPreparationStep],
    ) -> bool {
        let Some(tasks) = Self::native_preparation_receipt_tasks(visits) else {
            return false;
        };
        self.run_native_emission(tasks.into_iter().rev().collect());
        true
    }

    pub(super) fn run_native_emission(&mut self, mut pending: Vec<NativeEmissionTask>) {
        use NativeEmissionTask as Task;
        while let Some(task) = pending.pop() {
            match task {
                selected @ (Task::PrivateExpressionNumber(..)
                | Task::PrivateLogicalBoolean85(..)
                | Task::NativeCommandLiteral(..)
                | Task::RegisteredExpressionNumber(..)
                | Task::RegisteredExpressionBoolean84(..)
                | Task::FixedMathCall(..)
                | Task::PrivateInteger(..)
                | Task::PrivateList(..)
                | Task::ErrorReturn
                | Task::Operation(..)
                | Task::NativeListIndex(..)
                | Task::NativeListRange(..)
                | Task::NativeListInsertion(..)
                | Task::Literal(..)
                | Task::NativePrivateFormatLiteral(..)
                | Task::PrivateReturnOptions(..)
                | Task::PoolLiteral(..)
                | Task::Label(..)) => self.emit_native_literal_task(selected),
                selected @ (Task::ControlContext(..)
                | Task::ControlScript(..)
                | Task::NativeLoopTargets(..)
                | Task::LeaveNativeLoop
                | Task::NativeLoopExit(..)
                | Task::BeginNativeCatch(..)
                | Task::BeginNativeCatchAt(..)
                | Task::BeginSpeculativeNativeCompilation(..)
                | Task::FinishSpeculativeNativeCatch(..)
                | Task::EndNativeCatch
                | Task::BeginNativeCatchBranch(..)
                | Task::EndNativeCatchBranch) => {
                    self.emit_native_control_task(selected, &mut pending);
                }
                selected @ (Task::NativePreparationLiteral(..)
                | Task::NativeDictionaryUpdateOperation(..)
                | Task::NativeArrayEachStart(..)
                | Task::NativeEachStart(..)
                | Task::RestoreSource(..)
                | Task::DeclareNativeTemporary(..)
                | Task::NativeTemporaryOperation(..)
                | Task::DeclareNamespaceLocal(..)
                | Task::NamespaceLocalOperation(..)
                | Task::NamespaceGenericRollback(..)) => {
                    self.emit_native_binding_task(selected, &mut pending);
                }
                selected @ (Task::NativeStringIndex(..)
                | Task::SwitchOperation(..)
                | Task::SwitchTable(..)) => {
                    self.emit_native_switch_task(selected);
                }
                selected @ (Task::ExpressionSyntax(..) | Task::Syntax(..)) => {
                    self.emit_native_syntax_task(selected, &mut pending);
                }
                selected @ (Task::Word(..)
                | Task::List(..)
                | Task::Script(..)
                | Task::Command(..)) => self.emit_native_source_task(selected, &mut pending),
                selected @ Task::Part(..) => self.emit_native_part_task(selected, &mut pending),
                Task::Substitution(arena) => self.emit_substitution(&arena),
                Task::StampSubstitution(start, text) => {
                    if super::super::cmd_subst::has_command_separator_with_config(
                        &text,
                        self.lexer_config(),
                    ) {
                        self.stamp_substitution_script(start);
                    } else {
                        self.stamp_substitution_error_info(start, &format!("[{text}]"), None);
                    }
                }
                selected @ Task::ExpressionNode(..) => {
                    self.emit_native_expression_task(selected, &mut pending);
                }
            }
        }
    }

    fn emit_native_literal_task(&mut self, task: NativeEmissionTask) {
        use NativeEmissionTask as Task;
        match task {
            Task::PrivateExpressionNumber(version, value) => {
                let index = self
                    .literals
                    .register_private_expression_number(version, value);
                self.push_native_pool_literal(index);
            }
            Task::PrivateLogicalBoolean85(value) => {
                let index = self.literals.register_private_logical_boolean85(value);
                self.push_native_pool_literal(index);
            }
            Task::NativeCommandLiteral(recipe) => {
                let index = self.intern_native_command_recipe(*recipe);
                self.push_native_pool_literal(index);
            }
            Task::RegisteredExpressionNumber(version, bytes, value) => {
                let index = self
                    .literals
                    .intern_expression_number(&bytes, version, value);
                self.push_native_pool_literal(index);
            }
            Task::RegisteredExpressionBoolean84(bytes) => {
                let index = self.literals.intern_expression_boolean84(&bytes);
                self.push_native_pool_literal(index);
            }
            Task::FixedMathCall(binding, argc) => {
                let index = self.emit(Op::CALL_FUNC1, vec![Operand::Imm(i32::from(argc))]);
                self.instructions[index].native_fixed_math_call = Some(binding);
            }
            Task::PrivateInteger(value) => {
                let slot = self.literals.register_private_integer(value);
                self.push_native_pool_literal(slot);
            }
            Task::PrivateList(members, protocol) => {
                let slot = self
                    .literals
                    .register_private_constant_list(&members, protocol);
                self.push_native_pool_literal(slot);
            }
            Task::ErrorReturn => self.emit_native_error_return(""),
            Task::Operation(op, operands) => {
                self.emit(op, operands);
            }
            Task::NativeListInsertion(recipe, count) => {
                use tcl_registry::native_list_operations_compilation::NativeListInsertionInstruction;
                let opcode = match recipe {
                    NativeListInsertionInstruction::ReplaceFour => Op::LREPLACE4,
                    NativeListInsertionInstruction::Replace => Op::LREPLACE,
                };
                let index = self.emit(
                    opcode,
                    vec![
                        Operand::Imm(super::super::bytecode_imm(count)),
                        Operand::Imm(2),
                    ],
                );
                self.instructions[index].native_list_insertion = Some(recipe);
            }
            Task::NativeListRange(range) => {
                let instruction = self.emit(
                    Op::LIST_RANGE_IMM,
                    vec![
                        Operand::Imm(range.first.encoded()),
                        Operand::Imm(range.last.encoded()),
                    ],
                );
                self.instructions[instruction].native_list_range = Some(range);
            }
            Task::NativeListIndex(index) => {
                let instruction =
                    self.emit(Op::LIST_INDEX_IMM, vec![Operand::Imm(index.encoded())]);
                self.instructions[instruction].native_list_index = Some(index);
            }
            Task::NativePrivateFormatLiteral(bytes) => {
                let index = self.literals.register_private_concat_string(&bytes);
                self.push_native_pool_literal(index);
            }
            Task::Literal(bytes) => {
                self.push_lit_bytes_exact(&bytes);
            }
            Task::PrivateReturnOptions(recipe) => {
                let index = self.literals.register_private_return_options(recipe);
                self.push_native_pool_literal(index);
            }
            Task::PoolLiteral(index) => {
                self.push_native_pool_literal(index);
            }
            Task::Label(label) => self.place_label(&label),
            _ => unreachable!("native literal task dispatch"),
        }
    }

    /// `TclCompileBreakCmd` and `TclCompileContinueCmd` from 8.6: inside the
    /// loop's own exception range, at the body's stack depth, the command is a
    /// `jump4` to the loop's target. Elsewhere, and in 8.5, it is the
    /// instruction the enclosing range catches.
    fn emit_native_loop_exit(&mut self, is_break: bool, version: tcl_dialect::TclVersion) {
        let target = (version >= tcl_dialect::TclVersion::V8_6)
            .then(|| self.native_loop_targets.last().cloned().flatten())
            .flatten()
            .and_then(|(break_target, continue_target)| {
                if is_break {
                    Some(break_target)
                } else {
                    continue_target
                }
            });
        match target {
            // The comment keeps the four-byte form, which C never narrows
            // for a break or continue jump.
            Some(label) => {
                self.emit_comment(
                    Op::JUMP4,
                    vec![Operand::Label(label)],
                    if is_break { "break" } else { "continue" },
                );
            }
            None if is_break => {
                self.emit(Op::BREAK, vec![]);
            }
            None => {
                self.emit(Op::CONTINUE, vec![]);
            }
        }
    }

    fn emit_native_control_task(
        &mut self,
        task: NativeEmissionTask,
        pending: &mut Vec<NativeEmissionTask>,
    ) {
        use NativeEmissionTask as Task;
        match task {
            Task::ControlContext(context) => self.native_compilation = context,
            Task::NativeLoopTargets(break_target, continue_target) => {
                self.pending_native_loop = Some((break_target, continue_target));
            }
            Task::LeaveNativeLoop => {
                self.native_loop_targets.pop();
            }
            Task::NativeLoopExit(is_break, version) => {
                self.emit_native_loop_exit(is_break, version);
            }
            Task::ControlScript(image, body, context, config) => {
                // A body keeps the loop in reach only where C keeps the loop's
                // range innermost: the loop's own body, and a body that
                // inherits its context (`if`). Every other body masks it.
                let targets = self.pending_native_loop.take();
                if context != tcl_registry::native_compilation::NativeCompiledBodyContext::Inherit {
                    self.native_loop_targets.push(targets);
                    pending.push(Task::LeaveNativeLoop);
                }
                let saved = self.native_compilation;
                let entered = match context {
                    tcl_registry::native_compilation::NativeCompiledBodyContext::Inherit => saved,
                    tcl_registry::native_compilation::NativeCompiledBodyContext::Loop => {
                        tcl_registry::native_compilation::NativeCompilationContext {
                            loop_depth: saved.loop_depth + 1,
                            ..saved
                        }
                    }
                    tcl_registry::native_compilation::NativeCompiledBodyContext::ExceptionRange => {
                        saved.with_inline_exception_range()
                    }
                    tcl_registry::native_compilation::NativeCompiledBodyContext::ProtectedLoop => {
                        tcl_registry::native_compilation::NativeCompilationContext {
                            loop_depth: saved.loop_depth + 1,
                            ..saved.with_inline_exception_range()
                        }
                    }
                };
                self.native_compilation = entered;
                pending.push(Task::ControlContext(saved));
                if let Some(span) = body.script {
                    pending.push(Task::Script(image, span, config));
                } else {
                    self.refuse_native_dependency();
                }
            }
            Task::BeginNativeCatch(handler) => {
                let index = self.emit(Op::BEGIN_CATCH4, vec![Operand::Imm(0)]);
                self.instructions[index].catch_target = Some(handler);
                self.catch_depth += 1;
            }
            Task::BeginNativeCatchAt(handler, start, end) => {
                let index = self.emit(Op::BEGIN_CATCH4, vec![Operand::Imm(0)]);
                self.instructions[index].catch_target = Some(handler);
                self.instructions[index].catch_start = Some(start);
                self.instructions[index].catch_end = Some(end);
                self.catch_depth += 1;
            }
            Task::BeginSpeculativeNativeCompilation(scope) => {
                self.native_speculative_compilations.push(scope);
            }
            Task::FinishSpeculativeNativeCatch(saved, command, scope, context, catch_depth) => {
                let active = self
                    .native_speculative_compilations
                    .pop()
                    .expect("active speculative catch compiler");
                assert!(std::rc::Rc::ptr_eq(&active, &scope));
                self.native_compilation = context;
                self.catch_depth = catch_depth;
                if scope.get() {
                    self.instructions.truncate(saved.instructions);
                    self.label_positions = saved.labels;
                    self.inline_loop_regions.truncate(saved.loop_regions);
                    self.cmd_index = saved.command_index;
                    self.schedule_native_generic(command, pending);
                }
            }
            Task::EndNativeCatch => {
                self.emit(Op::END_CATCH, vec![]);
                self.catch_depth -= 1;
            }
            Task::BeginNativeCatchBranch(handler) => {
                let index = self.emit(Op::BEGIN_CATCH4, vec![Operand::Imm(0)]);
                self.instructions[index].catch_target = Some(handler);
            }
            Task::EndNativeCatchBranch => {
                self.emit(Op::END_CATCH, vec![]);
            }
            _ => unreachable!("native control task dispatch"),
        }
    }

    fn emit_native_binding_task(
        &mut self,
        task: NativeEmissionTask,
        pending: &mut Vec<NativeEmissionTask>,
    ) {
        use NativeEmissionTask as Task;
        match task {
            selected @ (Task::NativeArrayEachStart(..) | Task::NativeEachStart(..)) => {
                self.emit_native_each_binding_task(selected);
            }
            Task::RestoreSource(source) => {
                self.source = source.image;
                self.line_index = source.lines;
                self.current_span = source.span;
                self.current_span_is_command = source.command;
                self.exact_command_source = source.exact_command;
                self.current_command_namespace = source.namespace;
                self.current_command_namespace_context = source.namespace_context;
            }
            Task::NativePreparationLiteral(bytes) => {
                self.literals.intern_bytes(&bytes);
            }
            Task::NativeDictionaryUpdateOperation(op, root, targets) => {
                self.emit_native_dictionary_update_task(op, &root, &targets);
            }
            Task::DeclareNativeTemporary(slot) => {
                slot.set(Some(self.lvt.intern_anonymous()));
            }
            Task::NativeTemporaryOperation(op, slot, mut operands) => {
                if let Some(slot) = slot.get() {
                    let op = match (op, slot >= 256) {
                        (Op::LOAD_SCALAR1, true) => Op::LOAD_SCALAR4,
                        (Op::STORE_SCALAR1, true) => Op::STORE_SCALAR4,
                        _ => op,
                    };
                    operands.push(Operand::Imm(super::super::bytecode_imm(slot)));
                    self.emit(op, operands);
                } else {
                    self.refuse_native_dependency();
                }
            }
            Task::DeclareNamespaceLocal(name) => {
                if self.command_variable_slot(&name).is_none() {
                    self.refuse_native_dependency();
                }
            }
            Task::NamespaceLocalOperation(op, name) => {
                let slot = self
                    .compiled_variable_protocol
                    .and_then(|protocol| self.lvt.find_native(protocol, &name));
                if let Some(slot) = slot {
                    let op = match (op, slot >= 256) {
                        (Op::LOAD_SCALAR1, true) => Op::LOAD_SCALAR4,
                        (Op::STORE_SCALAR1, true) => Op::STORE_SCALAR4,
                        _ => op,
                    };
                    self.emit(op, vec![Operand::Imm(super::super::bytecode_imm(slot))]);
                } else {
                    self.refuse_native_dependency();
                }
            }
            Task::NamespaceGenericRollback(saved, command) => {
                self.instructions.truncate(saved.instructions);
                self.label_positions = saved.labels;
                self.inline_loop_regions.truncate(saved.loop_regions);
                self.cmd_index = saved.command_index;
                self.schedule_native_generic(command, pending);
            }
            _ => unreachable!("native binding task dispatch"),
        }
    }

    fn emit_native_each_binding_task(&mut self, task: NativeEmissionTask) {
        use NativeEmissionTask as Task;
        match task {
            Task::NativeArrayEachStart(version, variables) => {
                let Some(variables) = variables
                    .iter()
                    .map(|slot| slot.get())
                    .collect::<Option<Vec<_>>>()
                else {
                    self.refuse_native_dependency();
                    return;
                };
                let index = self.emit(Op::FOREACH_START, vec![Operand::Imm(0)]);
                self.instructions[index].native_each =
                    Some(std::sync::Arc::new(tcl_bytecode::NativeEachAuxiliary {
                        version,
                        variables: vec![variables],
                        temporaries: Vec::new(),
                    }));
            }
            Task::NativeEachStart(version, names, temporaries, collect) => {
                let variables = names
                    .iter()
                    .map(|group| {
                        group
                            .iter()
                            .map(|name| {
                                self.compiled_variable_protocol
                                    .and_then(|protocol| self.lvt.find_native(protocol, name))
                            })
                            .collect::<Option<Vec<_>>>()
                    })
                    .collect::<Option<Vec<_>>>();
                let temporaries = temporaries
                    .iter()
                    .map(|slot| slot.get())
                    .collect::<Option<Vec<_>>>();
                if let (Some(variables), Some(temporaries)) = (variables, temporaries) {
                    let index = self.emit(Op::FOREACH_START, vec![Operand::Imm(0)]);
                    self.instructions[index].native_each =
                        Some(std::sync::Arc::new(tcl_bytecode::NativeEachAuxiliary {
                            version,
                            variables,
                            temporaries,
                        }));
                    self.instructions[index].foreach_collect = collect;
                } else {
                    self.refuse_native_dependency();
                }
            }
            _ => unreachable!("native each binding task dispatch"),
        }
    }

    fn emit_native_switch_task(&mut self, task: NativeEmissionTask) {
        use NativeEmissionTask as Task;
        match task {
            Task::NativeStringIndex(version, purpose) => {
                let index = self.emit(Op::STR_INDEX, Vec::new());
                self.instructions[index].native_switch_version = Some(version);
                self.instructions[index].native_string_index_purpose = Some(purpose);
            }
            Task::SwitchOperation(op, operands, version) => {
                let index = self.emit(op, operands);
                self.instructions[index].native_switch_version = Some(version);
            }
            Task::SwitchTable(version, numeric, bytes, integers) => {
                let index = self.emit(Op::JUMP_TABLE, vec![Operand::Imm(0)]);
                let instruction = &mut self.instructions[index];
                instruction.native_switch_version = Some(version);
                if numeric {
                    instruction.native_switch_integers = Some(integers);
                } else {
                    instruction.native_switch_bytes = Some(bytes);
                }
            }
            _ => unreachable!("native switch task dispatch"),
        }
    }

    fn emit_native_syntax_task(
        &mut self,
        task: NativeEmissionTask,
        pending: &mut Vec<NativeEmissionTask>,
    ) {
        use NativeEmissionTask as Task;
        match task {
            Task::ExpressionSyntax(message, error_code) => {
                if let Some(failed) = self.native_speculative_compilations.last() {
                    failed.set(true);
                    while !pending.last().is_some_and(|task| {
                        matches!(task,
                        Task::FinishSpeculativeNativeCatch(_, _, scope, _, _)
                            if std::rc::Rc::ptr_eq(scope, failed))
                    }) {
                        if pending.pop().is_none() {
                            break;
                        }
                    }
                    return;
                }
                let syntax = self.invocation_dialect.and_then(|dialect| {
                    dialect.native_return_options_application(
                        tcl_registry::native_return_options::NativeReturnOptionsApplication::Syntax,
                    )
                });
                let message_index =
                    if syntax.is_some_and(tcl_registry::native_return_options::NativeReturnOptionsApplicationProtocol::syntax_options_share_message) {
                        // TclAddLiteralObj keeps the original error message out
                        // of C9.1's global literal registration table.
                        self.literals.register_unshared(&message)
                    } else {
                        self.literals.intern_bytes(&message)
                    };
                self.push_native_pool_literal(message_index);
                let protocol = self
                    .invocation_dialect
                    .and_then(tcl_registry::InvocationDialect::native_string_protocol);
                if let Some(protocol) = protocol {
                    use tcl_runtime_api::native_return_literal::NativeKnownWordLiteral;
                    let word = |bytes: Vec<u8>| NativeKnownWordLiteral {
                        pieces: vec![bytes],
                        composite: false,
                    };
                    // C's compiler seeds `-errorinfo` from the folded error; an
                    // expression the release evaluates directly is compiled
                    // without folding and raises the error as it runs.
                    let direct = self.direct_statement.is_some() || self.in_direct_statement();
                    let shares_message = syntax.is_some_and(tcl_registry::native_return_options::NativeReturnOptionsApplicationProtocol::syntax_options_share_message);
                    // `TclCompileSyntaxError` stores `Tcl_GetReturnOptions(interp,
                    // TCL_ERROR)`: `-code 1 -level 0 -errorcode .. -errorinfo ..
                    // -errorline 1` in 8.5-9.1. 9.1 shares the message object as
                    // `-errorinfo`; this placeholder holds the key's position.
                    let mut words = vec![
                        word(b"-code".to_vec()),
                        word(b"1".to_vec()),
                        word(b"-level".to_vec()),
                        word(b"0".to_vec()),
                        word(b"-errorcode".to_vec()),
                        word(error_code.unwrap_or_else(|| b"NONE".to_vec())),
                    ];
                    if !direct && syntax.is_some() {
                        words.extend([
                            word(b"-errorinfo".to_vec()),
                            word(message.clone()),
                            word(b"-errorline".to_vec()),
                            word(b"1".to_vec()),
                        ]);
                    }
                    let size = words.len() / 2 - 2;
                    let options =
                        tcl_runtime_api::native_return_literal::NativeReturnOptionsLiteral {
                            protocol,
                            words,
                            code: 1,
                            level: 0,
                            size,
                            retains_controls: true,
                        };
                    let index = self.literals.register_private_return_options(options);
                    if !direct && shares_message {
                        self.literals.retain_syntax_error_info(index, message_index);
                    }
                    self.push_native_pool_literal(index);
                    self.emit(Op::SYNTAX, vec![Operand::Imm(1), Operand::Imm(0)]);
                } else {
                    self.refuse_native_dependency();
                }
            }
            Task::Syntax(message) => {
                if let Some(failed) = self.native_speculative_compilations.last() {
                    failed.set(true);
                    while !pending.last().is_some_and(|task| {
                        matches!(task,
                        Task::FinishSpeculativeNativeCatch(_, _, scope, _, _)
                            if std::rc::Rc::ptr_eq(scope, failed))
                    }) {
                        if pending.pop().is_none() {
                            break;
                        }
                    }
                } else {
                    self.emit_script_parse_error(message.as_bytes());
                }
            }
            _ => unreachable!("native syntax task dispatch"),
        }
    }

    fn emit_native_source_task(
        &mut self,
        task: NativeEmissionTask,
        pending: &mut Vec<NativeEmissionTask>,
    ) {
        use NativeEmissionTask as Task;
        match task {
            Task::Word(word) => {
                if word.group().kind == tcl_lexer::WordKind::Braced {
                    let content = word.content_span().expect("validated native word geometry");
                    let bytes = self.source_string_protocol.and_then(|protocol| {
                        tcl_syntax::backslash::native_source_braced_word_bytes(
                            &word.image().bytes()[content.as_range()],
                            word.image().channel(),
                            word.config().brace_backslash_newline,
                            protocol,
                        )
                        .ok()
                    });
                    if let Some(bytes) = bytes {
                        self.push_lit_bytes_exact(&bytes);
                    } else {
                        self.refuse_native_dependency();
                        self.push_lit_bytes_exact(b"");
                    }
                } else {
                    let arena = std::rc::Rc::new(word.executable_parts().clone());
                    let root = arena.root();
                    pending.push(Task::List(arena, root, 0));
                }
            }
            Task::List(arena, id, position) => {
                let parts = arena.list(id);
                if parts.is_empty() {
                    self.push_lit_bytes_exact(b"");
                    return;
                }
                if position + 1 < parts.len() {
                    pending.push(Task::List(std::rc::Rc::clone(&arena), id, position + 1));
                }
                if position > 0 {
                    pending.push(Task::Operation(Op::STR_CONCAT1, vec![Operand::Imm(2)]));
                }
                pending.push(Task::Part(arena, id, position));
            }
            Task::Script(image, region, config) => {
                // Jim interprets bracket scripts; C compiles their original
                // words within this unit and shares its local allocation.
                if self.invocation_dialect.is_some_and(|dialect| {
                    dialect.family() == Some(tcl_dialect::model::Family::Jim)
                }) {
                    self.push_lit_bytes_exact(&image.bytes()[region.as_range()]);
                    self.emit(Op::EVAL_STK, vec![]);
                    return;
                }
                // An authored naming simulation never matches a native compiler
                // recipe, so its bracket scripts compile as top-level commands
                // under that entry do: generic dispatch under the VM's policy.
                if self.native_entry.is_none_or(authored_name_policy) {
                    if let Ok(text) = std::str::from_utf8(&image.bytes()[region.as_range()]) {
                        self.emit_inline_cmd_subst(text);
                    } else {
                        self.refuse_native_dependency();
                        self.push_lit_bytes_exact(b"");
                    }
                    return;
                }
                let Ok(plan) = tcl_lexer::native_script_words_in(image, region, config) else {
                    self.refuse_native_dependency();
                    self.push_lit_bytes_exact(b"");
                    return;
                };
                if let Some(tail) = plan.fatal_tail {
                    pending.push(Task::Syntax(tail.cut.message));
                } else if plan.commands.is_empty() {
                    self.push_lit_bytes_exact(b"");
                }
                let count = plan.commands.len();
                for (index, command) in plan.commands.into_iter().enumerate().rev() {
                    if index + 1 < count || plan.fatal_tail.is_some() {
                        pending.push(Task::Operation(Op::POP, vec![]));
                    }
                    pending.push(Task::Command(command));
                }
            }
            Task::Command(command) => self.schedule_native_command(command, pending),
            _ => unreachable!("native source task dispatch"),
        }
    }

    fn emit_native_part_task(
        &mut self,
        task: NativeEmissionTask,
        pending: &mut Vec<NativeEmissionTask>,
    ) {
        use NativeEmissionTask as Task;
        use tcl_lexer::{ExecutablePart, ExecutableText};
        match task {
            Task::Part(arena, id, position) => {
                let part = &arena.list(id)[position];
                match &part.part {
                    ExecutablePart::Text(ExecutableText::Original | ExecutableText::Decoded(_)) => {
                        let bytes = self.source_string_protocol.and_then(|protocol| {
                            tcl_syntax::backslash::native_arena_text(
                                &arena,
                                part,
                                self.lexer_config().escapes,
                                protocol,
                            )
                            .ok()
                        });
                        if let Some(bytes) = bytes {
                            self.push_lit_bytes_exact(&bytes);
                        } else {
                            self.refuse_native_dependency();
                            self.push_lit_bytes_exact(b"");
                        }
                    }
                    ExecutablePart::Variable { name, index } => {
                        let name = self.source_string_protocol.and_then(|protocol| {
                            tcl_syntax::backslash::native_source_literal_bytes(
                                arena.bytes(*name).expect("original arena variable name"),
                                arena.image().channel(),
                                protocol,
                            )
                            .ok()
                        });
                        let Some(name) = name else {
                            self.refuse_native_dependency();
                            self.push_lit_bytes_exact(b"");
                            return;
                        };
                        let (op, operands) =
                            self.prepare_original_variable_load(&name, index.is_some());
                        if let Some(index) = index {
                            pending.push(Task::Operation(op, operands));
                            pending.push(Task::List(std::rc::Rc::clone(&arena), *index, 0));
                        } else {
                            self.emit(op, operands);
                        }
                    }
                    ExecutablePart::Command { body } => {
                        // A substituted command runs above the loop body's
                        // stack depth, so the loop is out of its reach.
                        self.native_loop_targets.push(None);
                        pending.push(Task::LeaveNativeLoop);
                        // Runs once the script's commands are all emitted.
                        pending.push(Task::StampSubstitution(
                            self.instructions.len(),
                            String::from_utf8_lossy(
                                arena.bytes(*body).expect("native arena command"),
                            )
                            .into_owned(),
                        ));
                        pending.push(Task::Script(
                            arena.image().clone(),
                            *body,
                            self.lexer_config(),
                        ));
                    }
                    ExecutablePart::Expression { expression } => {
                        self.push_lit_bytes_exact(
                            arena.bytes(*expression).expect("native arena expression"),
                        );
                        self.emit(Op::EXPR_STK, vec![]);
                    }
                    ExecutablePart::ParseError(message) => pending.push(Task::Syntax(message)),
                }
            }
            _ => unreachable!("native part task dispatch"),
        }
    }

    fn emit_native_expression_task(
        &mut self,
        task: NativeEmissionTask,
        pending: &mut Vec<NativeEmissionTask>,
    ) {
        use NativeEmissionTask as Task;
        match task {
            Task::ExpressionNode(program, node) => {
                self.schedule_native_expression_node(program, node, pending);
            }
            _ => unreachable!("native expression task dispatch"),
        }
    }
}
