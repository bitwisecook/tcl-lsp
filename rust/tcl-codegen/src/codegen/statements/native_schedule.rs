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

//! Scheduling one original command onto the work list as a generic, named or registered-compiler invocation, with its replay boundary.

use super::super::{CodegenCtx, Op, Operand};
use super::{
    NativeEmissionSource, NativeEmissionTask, NativeRegisteredBoundary, PreparedNativeRegistered,
};

impl CodegenCtx<'_> {
    /// A reached native script parse failure uses the same original-byte
    /// presentation in script, bracket, and structured-body compilation.
    pub(in super::super) fn emit_script_parse_error(&mut self, message: &[u8]) {
        self.push_lit_bytes_exact(message);
        self.push_lit_bytes_exact(b"-errorcode NONE");
        self.emit(Op::SYNTAX, vec![Operand::Imm(1), Operand::Imm(0)]);
    }

    /// A parser-issued source extent is separate from a vector's represented
    /// word envelope. Only the actual C source-log recipe uses trailing trivia;
    /// Jim retains its own command presentation. Vector-only replay has no
    /// containing-region extent to promote.
    pub(super) fn native_command_source_span(
        &self,
        command: &tcl_lexer::NativeScriptCommandWords,
    ) -> tcl_lexer::Span {
        match (
            self.invocation_dialect
                .and_then(tcl_registry::InvocationDialect::native_error_log_protocol),
            command.source_extent,
        ) {
            (Some(_), Some(extent)) => extent,
            _ => command.span,
        }
    }

    pub(super) fn schedule_native_command(
        &mut self,
        command: tcl_lexer::NativeScriptCommandWords,
        pending: &mut Vec<NativeEmissionTask>,
    ) {
        use tcl_compiler::native_byte_compilation::{
            NativeByteCommandPlan, native_byte_command_plan,
        };
        let Some(first) = command.words.first() else {
            return;
        };
        let image = first.image().clone();
        let source_span = self.native_command_source_span(&command);
        let Some(bytes) = image.bytes().get(source_span.as_range()) else {
            self.refuse_native_dependency();
            return;
        };
        let exact = tcl_lexer::SourceImage::from_bytes(bytes, image.channel());
        pending.push(NativeEmissionTask::RestoreSource(NativeEmissionSource {
            image: self.source.clone(),
            lines: self.line_index.clone(),
            span: self.current_span,
            command: self.current_span_is_command,
            exact_command: self.exact_command_source.clone(),
            namespace: self.current_command_namespace.clone(),
            namespace_context: self.current_command_namespace_context.clone(),
        }));
        if self.source != image {
            self.line_index = Some(tcl_lexer::LineIndex::from_bytes(image.bytes()));
            self.source = image;
        }
        self.set_command_source_span(source_span);
        self.exact_command_source = Some(exact);
        if let Some(namespace) = self.native_entry.and_then(|entry| {
            entry
                .namespaces
                .iter()
                .find(|namespace| namespace.token == entry.current_namespace)
        }) {
            self.current_command_namespace = namespace.path.clone();
            self.current_command_namespace_context = self.native_entry.and_then(|entry| {
                entry
                    .retained_namespace_context(namespace.token)
                    .ok()
                    .map(tcl_runtime_api::CompiledNamespaceContext::Native)
            });
        }
        // Plain dispatch invokes every command, nested substitutions
        // included, without native compiler selection.
        let plan = if self.plain_command_dispatch
            || self
                .invocation_dialect
                .is_some_and(|dialect| dialect.family() == Some(tcl_dialect::model::Family::Jim))
        {
            Ok(NativeByteCommandPlan::Generic)
        } else if let Some(entry) = self.native_entry {
            native_byte_command_plan(
                &command.words,
                entry,
                self.registry,
                self.native_compilation,
            )
        } else {
            self.refuse_native_dependency();
            Ok(NativeByteCommandPlan::Generic)
        };
        match plan {
            Ok(NativeByteCommandPlan::Named(plan)) => {
                if !self.schedule_native_named(&command, &plan, pending) {
                    self.refuse_native_dependency();
                    pending.push(NativeEmissionTask::Literal(Vec::new()));
                }
            }
            Ok(NativeByteCommandPlan::Registered(plan)) => {
                if !self.schedule_native_registered(&command, &plan, pending) {
                    self.refuse_native_dependency();
                    pending.push(NativeEmissionTask::Literal(Vec::new()));
                }
            }
            Ok(NativeByteCommandPlan::Generic) => {
                self.schedule_native_generic(command, pending);
            }
            Err(_) => {
                self.refuse_native_dependency();
                pending.push(NativeEmissionTask::Literal(Vec::new()));
            }
        }
    }

    pub(super) fn schedule_native_generic(
        &mut self,
        command: tcl_lexer::NativeScriptCommandWords,
        pending: &mut Vec<NativeEmissionTask>,
    ) {
        let boundary = self.emit(Op::NOP, vec![]);
        self.instructions[boundary].source_command_boundary =
            tcl_bytecode::SourceCommandBoundary::Start;
        self.instructions[boundary].no_fold = true;
        let expanded = command.words.iter().any(|word| word.group().expand);
        let count = super::super::bytecode_imm(command.words.len());
        pending.push(NativeEmissionTask::Operation(
            if expanded {
                Op::INVOKE_EXPANDED
            } else if count < 256 {
                Op::INVOKE_STK1
            } else {
                Op::INVOKE_STK4
            },
            if expanded {
                vec![]
            } else {
                vec![Operand::Imm(count)]
            },
        ));
        let command_head = self.intern_original_native_command_literal(&command.words);
        for (index, word) in command.words.into_iter().enumerate().rev() {
            if expanded && word.group().expand {
                pending.push(NativeEmissionTask::Operation(
                    Op::EXPAND_STKTOP,
                    vec![Operand::Imm(super::super::bytecode_imm(index + 1))],
                ));
            }
            if index == 0
                && let Some(head) = command_head
            {
                pending.push(NativeEmissionTask::PoolLiteral(head));
            } else {
                pending.push(NativeEmissionTask::Word(word));
            }
        }
        if expanded {
            self.emit(Op::EXPAND_START, vec![]);
        }
    }

    fn schedule_native_named(
        &mut self,
        command: &tcl_lexer::NativeScriptCommandWords,
        plan: &tcl_compiler::native_byte_compilation::NativeByteNamedInvocation,
        pending: &mut Vec<NativeEmissionTask>,
    ) -> bool {
        use tcl_registry::native_compilation::NativeNamedInvocationProtocol;
        use tcl_registry::native_instruction_plan::NativeNamedInvocationWord;
        let Some(entry) = self.native_entry else {
            return false;
        };
        let (Some(point), Some(names)) = (entry.execution_point, entry.command_name_policy())
        else {
            return false;
        };
        let Some(namespace) = entry
            .namespaces
            .iter()
            .find(|namespace| namespace.token == entry.current_namespace)
        else {
            return false;
        };
        let context = tcl_runtime_api::native_command_name::NativeLiteralContext {
            interpreter: entry.interpreter,
            namespace_token: entry.current_namespace,
            entry_epoch: entry.epoch,
            namespace_path: namespace.path.clone(),
        };
        let Ok(literal) = tcl_registry::native_command_literal::native_compiled_selected_command_name_literal_from_lookup(
            point, names, context, &plan.recipe.name, plan.prerequisite.clone(),
        ) else { return false; };
        let rewrite = plan.recipe.protocol == NativeNamedInvocationProtocol::EnsembleRewrite;
        let Some(mut operations) =
            Self::native_preparation_receipt_tasks(&plan.recipe.preparations)
        else {
            return false;
        };
        if !rewrite {
            operations.push(NativeEmissionTask::NativeCommandLiteral(Box::new(
                literal.clone(),
            )));
        }
        for word in &plan.recipe.words {
            operations.push(match word {
                NativeNamedInvocationWord::Original(operand) => {
                    let Some(task) =
                        Self::native_namespace_word_task(&command.words, operand.clone())
                    else {
                        return false;
                    };
                    task
                }
                NativeNamedInvocationWord::Replacement(bytes) => {
                    NativeEmissionTask::Literal(bytes.clone())
                }
            });
        }
        if rewrite {
            operations.push(NativeEmissionTask::NativeCommandLiteral(Box::new(literal)));
            operations.push(NativeEmissionTask::Operation(
                Op::INVOKE_REPLACE,
                vec![
                    Operand::Imm(super::super::bytecode_imm(plan.recipe.words.len())),
                    Operand::Imm(super::super::bytecode_imm(plan.recipe.arguments_from + 1)),
                ],
            ));
        } else {
            let count = super::super::bytecode_imm(plan.recipe.words.len() + 1);
            operations.push(NativeEmissionTask::Operation(
                if count < 256 {
                    Op::INVOKE_STK1
                } else {
                    Op::INVOKE_STK4
                },
                vec![Operand::Imm(count)],
            ));
        }
        let end = self.begin_original_compiler_selection(
            command,
            namespace.path.clone(),
            tcl_runtime_api::native_compilation::NativeCompilerSelectionPrerequisite::from_command_registration(
                std::sync::Arc::new(plan.prerequisite.clone()),
            ),
        );
        pending.push(NativeEmissionTask::Label(end));
        pending.extend(operations.into_iter().rev());
        true
    }

    /// Retain the actual compiler token before its original operands execute.
    /// Handler identity and value/cell selection remain independent obligations.
    fn schedule_native_registered(
        &mut self,
        command: &tcl_lexer::NativeScriptCommandWords,
        plan: &tcl_compiler::native_byte_compilation::NativeByteRegisteredCommand,
        pending: &mut Vec<NativeEmissionTask>,
    ) -> bool {
        let Some(prepared) = self.prepare_native_registered(command, plan) else {
            return false;
        };
        let mut operations = Vec::new();
        if !self.append_native_registered_tasks(
            command,
            prepared.version,
            prepared.instruction,
            &mut operations,
        ) {
            return false;
        }
        self.install_native_registered_boundary(
            command,
            plan,
            prepared.boundary,
            pending,
            operations,
        )
    }

    fn prepare_native_registered(
        &self,
        command: &tcl_lexer::NativeScriptCommandWords,
        plan: &tcl_compiler::native_byte_compilation::NativeByteRegisteredCommand,
    ) -> Option<PreparedNativeRegistered> {
        use tcl_registry::native_compiler_words::NativeCompilerWords;
        let entry = self.native_entry?;
        let version = entry
            .execution_point
            .and_then(tcl_dialect::model::DialectPoint::tcl_version)?;
        let protocol = self.source_string_protocol?;
        let words = NativeCompilerWords::capture(&command.words, protocol).ok()?;
        let guard = Self::native_registered_guard(plan)?;
        let head = words.literal(0)?;
        let compiler = plan.binding.compiler.clone()?;
        if compiler.ensemble.is_some() || !plan.dependencies.is_empty() {
            return None;
        }
        let namespace = entry
            .namespaces
            .iter()
            .find(|namespace| namespace.token == entry.current_namespace)?;
        let prerequisite = plan.prerequisite.clone().unwrap_or_else(|| {
            tcl_runtime_api::native_compilation::NativeCommandCompilerPrerequisite {
                interpreter: entry.interpreter,
                lookup_namespace_token: entry.current_namespace,
                invocation_word: tcl_runtime_api::NameBytes::from(head),
                slot: plan.binding.slot.clone(),
                namespace_token: plan.binding.namespace_token,
                token: plan.binding.token,
                implementation_generation: plan.binding.implementation_generation,
                compiler,
                selected_worker: None,
                nested_compilers: Vec::new(),
                guard: tcl_compiler::registry_invocation::native_command_binding_guard(guard),
            }
        });
        let instruction = plan.original_instruction(&command.words)?.clone();
        Some(PreparedNativeRegistered {
            version,
            instruction,
            boundary: NativeRegisteredBoundary {
                namespace: namespace.path.clone(),
                prerequisite,
            },
        })
    }

    /// `TclCompileSubstCmd`: the simple template word compiled in place.
    pub(super) fn append_native_substitution_tasks(
        &self,
        command: &tcl_lexer::NativeScriptCommandWords,
        recipe: tcl_registry::substitution::NativeSubstitutionInstruction,
        operations: &mut Vec<NativeEmissionTask>,
    ) -> bool {
        let Some(word) = command.words.get(recipe.template) else {
            return false;
        };
        let flags = tcl_lexer::word_parts::SubstFlags {
            vars: recipe.kinds.variables,
            cmds: recipe.kinds.commands,
            backslashes: recipe.kinds.backslashes,
            ..Default::default()
        };
        let Ok(arena) = tcl_lexer::ExecutablePartArena::decompose_template(
            word.image().clone(),
            recipe.span,
            flags,
            self.lexer_config(),
            tcl_lexer::word_parts::TemplateVariableSyntax::CTcl,
        ) else {
            return false;
        };
        operations.push(NativeEmissionTask::Substitution(std::rc::Rc::new(arena)));
        true
    }

    fn native_registered_guard(
        plan: &tcl_compiler::native_byte_compilation::NativeByteRegisteredCommand,
    ) -> Option<tcl_registry::native_compilation::NativeCompilationGuard> {
        use tcl_registry::native_compilation::NativeCompilationSelection;
        let guard =
            match plan.selection {
                NativeCompilationSelection::Inline { guard, .. } => guard,
                NativeCompilationSelection::Generic
                    if plan.spec.namespace_binding_kind().is_some()
                        || plan.has_generic_preparation()
                        || matches!(
                            plan.spec.grammar,
                            tcl_registry::native_compilation::NativeCompilationGrammar::Array { .. }
                        )
                        || matches!(plan.spec.grammar,
                tcl_registry::native_compilation::NativeCompilationGrammar::Conditional |
                tcl_registry::native_compilation::NativeCompilationGrammar::ForLoop |
                tcl_registry::native_compilation::NativeCompilationGrammar::Catch |
                tcl_registry::native_compilation::NativeCompilationGrammar::Try |
                tcl_registry::native_compilation::NativeCompilationGrammar::Foreach |
                tcl_registry::native_compilation::NativeCompilationGrammar::WhileLoop) =>
                {
                    tcl_registry::native_compilation::NativeCompilationGuard::BeforeArguments
                }
                _ => return None,
            };
        Some(guard)
    }

    fn install_native_registered_boundary(
        &mut self,
        command: &tcl_lexer::NativeScriptCommandWords,
        plan: &tcl_compiler::native_byte_compilation::NativeByteRegisteredCommand,
        boundary: NativeRegisteredBoundary,
        pending: &mut Vec<NativeEmissionTask>,
        operations: Vec<NativeEmissionTask>,
    ) -> bool {
        use NativeEmissionTask as Task;
        let prerequisite = if plan.prerequisite.is_some() {
            tcl_runtime_api::native_compilation::NativeCompilerSelectionPrerequisite::Ensemble(
                std::sync::Arc::new(boundary.prerequisite),
            )
        } else {
            tcl_runtime_api::native_compilation::NativeCompilerSelectionPrerequisite::Command(
                std::sync::Arc::new(boundary.prerequisite),
            )
        };
        let end = self.begin_original_compiler_selection(command, boundary.namespace, prerequisite);
        pending.push(Task::Label(end));
        pending.extend(operations.into_iter().rev());
        true
    }

    /// All selected original compilers share one exact source replay boundary.
    pub(super) fn begin_original_compiler_selection(
        &mut self,
        command: &tcl_lexer::NativeScriptCommandWords,
        namespace: tcl_runtime_api::ByteNamespacePath,
        prerequisite: tcl_runtime_api::native_compilation::NativeCompilerSelectionPrerequisite,
    ) -> String {
        if let tcl_runtime_api::native_compilation::NativeCompilerSelectionPrerequisite::Command(
            required,
        )
        | tcl_runtime_api::native_compilation::NativeCompilerSelectionPrerequisite::Ensemble(
            required,
        ) = &prerequisite
        {
            self.retain_entry_named_compiler_prerequisite(required);
        }
        let source_span = self.native_command_source_span(command);
        let end = self.fresh_label("original_compiler_selection_end");
        let start = self.emit(Op::NOP, vec![]);
        let instruction = &mut self.instructions[start];
        instruction.source_cmd_text = tcl_lexer::SourceImage::from_bytes(
            &command.words[0].image().bytes()[source_span.as_range()],
            command.words[0].image().channel(),
        );
        instruction.source_command_namespace = namespace;
        instruction
            .source_command_namespace_context
            .clone_from(&self.current_command_namespace_context);
        instruction.source_command_boundary = tcl_bytecode::SourceCommandBoundary::Start;
        instruction.source_span = Some(source_span);
        instruction.no_fold = true;
        instruction.native_compiler_selection = Some(tcl_bytecode::NativeCompilerSelectionSite {
            prerequisite,
            end: end.clone(),
        });
        end
    }
}
