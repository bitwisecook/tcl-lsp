// SPDX-License-Identifier: AGPL-3.0-or-later
//! Ordered compiler visits from authenticated original instruction recipes.

use super::{
    Arc, CommandAllocationSite, CompilerTraversal, SourceCompiledChild, SourceExecutionContext,
    SourceNativeCompilationFailure, WordExpr,
};
use tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand;
use tcl_registry::native_control_compilation::{
    NativeControlOutcome, NativeControlPreparationStep,
};
use tcl_registry::native_instruction_plan::NativeInstructionPlan;

#[derive(Clone, Copy)]
struct OriginalControlPreparations<'a> {
    steps: &'a [NativeControlPreparationStep],
    generic: bool,
    rejection: Option<&'a tcl_registry::native_compilation::NativeCompilationFailure>,
}

impl CompilerTraversal<'_> {
    pub(super) fn structured_compilation(
        &mut self,
        words: &[WordExpr],
        offset: u32,
        recipe: &NativeInstructionPlan,
        context: SourceExecutionContext<'_>,
    ) -> Option<SourceNativeCompilationFailure> {
        let (steps, generic, rejection) = match recipe {
            NativeInstructionPlan::Control(recipe) => (
                recipe.preparations.as_slice(),
                matches!(recipe.outcome, NativeControlOutcome::Generic),
                if let NativeControlOutcome::Rejected(failure) = &recipe.outcome {
                    Some(failure)
                } else {
                    None
                },
            ),
            NativeInstructionPlan::Each(recipe) => (
                recipe.preparations.as_slice(),
                matches!(recipe.outcome, NativeControlOutcome::Generic),
                if let NativeControlOutcome::Rejected(failure) = &recipe.outcome {
                    Some(failure)
                } else {
                    None
                },
            ),
            NativeInstructionPlan::Try(recipe) => (
                recipe.preparations.as_slice(),
                matches!(recipe.outcome, NativeControlOutcome::Generic),
                if let NativeControlOutcome::Rejected(failure) = &recipe.outcome {
                    Some(failure)
                } else {
                    None
                },
            ),
            _ => return self.original_non_control_compilation(words, offset, recipe, context),
        };
        self.control_preparations(
            words,
            offset,
            recipe,
            context,
            OriginalControlPreparations {
                steps,
                generic,
                rejection,
            },
        )
    }

    fn original_non_control_compilation(
        &mut self,
        words: &[WordExpr],
        offset: u32,
        recipe: &NativeInstructionPlan,
        context: SourceExecutionContext<'_>,
    ) -> Option<SourceNativeCompilationFailure> {
        match recipe {
            NativeInstructionPlan::ReturnOptions(recipe) => self.original_operands(
                words,
                recipe
                    .compiler_word_visits()
                    .map(NativeCompilerWordOperand::Original),
                context,
            ),
            NativeInstructionPlan::Concat(
                tcl_registry::native_instruction_plan::NativeConcatInstruction::Operands(operands),
            ) => self.original_operands(words, operands.iter().cloned(), context),
            NativeInstructionPlan::Concat(_) => None,
            NativeInstructionPlan::NamespaceBindings(recipe) => {
                self.original_namespace_preparations(words, recipe, context)
            }
            NativeInstructionPlan::Uplevel(recipe) => self.original_operands(
                words,
                recipe
                    .level_word
                    .into_iter()
                    .chain(recipe.script_words.clone())
                    .map(NativeCompilerWordOperand::Original),
                context,
            ),
            NativeInstructionPlan::Load { target } => {
                let Some(target_word) = words.len().checked_sub(1) else {
                    self.require_provider();
                    return None;
                };
                self.original_variable_operand(words, target, target_word, context)
            }
            NativeInstructionPlan::Store { target, value_word } => {
                self.original_store_preparations(words, target, *value_word, context)
            }
            NativeInstructionPlan::Error(error) => self.original_operands(
                words,
                error.steps.iter().filter_map(|step| {
                    if let tcl_registry::native_error_compilation::NativeErrorStep::Word(operand) =
                        step
                    {
                        Some(operand.clone())
                    } else {
                        None
                    }
                }),
                context,
            ),
            NativeInstructionPlan::DictionaryLookup(dictionary) => {
                self.original_operands(words, dictionary.operands.iter().cloned(), context)
            }
            NativeInstructionPlan::Expression(expression) => {
                if let Some(program) = &expression.program {
                    self.original_expression(
                        program,
                        offset,
                        context,
                        recipe.expression_error_context(&program.operand),
                    )
                } else {
                    self.original_operands(words, expression.operands.iter().cloned(), context)
                }
            }
            NativeInstructionPlan::TclOoHelper(helper) => {
                self.original_tcloo_preparations(words, helper, context)
            }
            NativeInstructionPlan::Unset(recipe) => {
                self.original_unset_preparations(words, recipe, context)
            }
            NativeInstructionPlan::StringMatch(recipe) => self.original_operands(
                words,
                [recipe.pattern.clone(), recipe.subject.clone()],
                context,
            ),
            _ => {
                self.require_provider();
                None
            }
        }
    }

    fn original_tcloo_preparations(
        &mut self,
        words: &[WordExpr],
        helper: &tcl_registry::native_tcloo_compilation::NativeTclOoInstruction,
        context: SourceExecutionContext<'_>,
    ) -> Option<SourceNativeCompilationFailure> {
        use tcl_registry::native_tcloo_compilation::NativeTclOoInstruction;
        match helper {
            NativeTclOoInstruction::ObjectInfo { operand, .. } => {
                self.original_operand(words, operand, context)
            }
            NativeTclOoInstruction::SelfObject | NativeTclOoInstruction::SelfNamespace => None,
            NativeTclOoInstruction::Next {
                words: operands, ..
            } => self.original_operands(words, operands.iter().cloned(), context),
        }
    }

    fn original_unset_preparations(
        &mut self,
        words: &[WordExpr],
        recipe: &tcl_registry::native_unset_compilation::NativeUnsetInstruction,
        context: SourceExecutionContext<'_>,
    ) -> Option<SourceNativeCompilationFailure> {
        use tcl_registry::native_unset_compilation::NativeUnsetReceiver;
        for variable in &recipe.variables {
            if let NativeUnsetReceiver::Original(target) = &variable.receiver {
                let NativeCompilerWordOperand::Original(index) = variable.operand else {
                    self.require_provider();
                    return None;
                };
                if let Some(failure) = self.original_variable_operand(words, target, index, context)
                {
                    return Some(failure);
                }
            }
        }
        None
    }

    fn original_store_preparations(
        &mut self,
        words: &[WordExpr],
        target: &tcl_syntax::native_variable_words::NativeVariableWordOperand,
        value_word: usize,
        context: SourceExecutionContext<'_>,
    ) -> Option<SourceNativeCompilationFailure> {
        let Some(target_word) = value_word.checked_sub(1) else {
            self.require_provider();
            return None;
        };
        if let Some(failure) = self.original_variable_operand(words, target, target_word, context) {
            return Some(failure);
        }
        self.original_operand(
            words,
            &NativeCompilerWordOperand::Original(value_word),
            context,
        )
    }

    fn original_operands(
        &mut self,
        words: &[WordExpr],
        operands: impl IntoIterator<Item = NativeCompilerWordOperand>,
        context: SourceExecutionContext<'_>,
    ) -> Option<SourceNativeCompilationFailure> {
        for operand in operands {
            if let Some(failure) = self.original_operand(words, &operand, context) {
                return Some(failure);
            }
        }
        None
    }

    fn original_namespace_preparations(
        &mut self,
        words: &[WordExpr],
        recipe: &tcl_registry::native_namespace_binding_compilation::NativeNamespaceBindingCompilation,
        context: SourceExecutionContext<'_>,
    ) -> Option<SourceNativeCompilationFailure> {
        for visit in &recipe.visits {
            if let tcl_registry::native_namespace_binding_compilation::NativeNamespaceBindingVisit::Word(operand) = visit
                && let Some(failure) = self.original_operand(words, operand, context)
            {
                return Some(failure);
            }
        }
        if recipe.outcome == tcl_registry::native_namespace_binding_compilation::NativeNamespaceBindingOutcome::Generic {
            self.substitutions(words, context)
        } else { None }
    }

    fn control_preparations(
        &mut self,
        words: &[WordExpr],
        offset: u32,
        recipe: &NativeInstructionPlan,
        context: SourceExecutionContext<'_>,
        preparations: OriginalControlPreparations<'_>,
    ) -> Option<SourceNativeCompilationFailure> {
        let OriginalControlPreparations {
            steps,
            generic,
            rejection,
        } = preparations;
        for step in steps {
            let failure = match step {
                NativeControlPreparationStep::Word(operand) => {
                    self.original_operand(words, operand, context)
                }
                NativeControlPreparationStep::Script {
                    operand,
                    span,
                    context: body_context,
                } => {
                    let compilation = entered_context(*body_context, context.compilation);
                    let Some(compilation) = compilation else {
                        self.require_provider();
                        return None;
                    };
                    self.original_script(
                        words,
                        offset,
                        *span,
                        context,
                        compilation,
                        !generic && rejection.is_none(),
                    )
                    .map(|mut failure| {
                        self.append_body_error_context(&mut failure, recipe, operand, *span);
                        failure
                    })
                }
                NativeControlPreparationStep::SpeculativeScript {
                    span,
                    context: body_context,
                    ..
                } => {
                    let Some(compilation) = entered_context(*body_context, context.compilation)
                    else {
                        self.require_provider();
                        return None;
                    };
                    let children = self.compiled_children.clone();
                    let fallbacks = self.snapshot.generic_fallbacks.clone();
                    let before_context = self.before_next_context.clone();
                    if self
                        .original_script(
                            words,
                            offset,
                            *span,
                            context,
                            compilation,
                            !generic && rejection.is_none(),
                        )
                        .is_some()
                    {
                        self.compiled_children = children;
                        self.snapshot.generic_fallbacks = fallbacks;
                        self.before_next_context = before_context;
                        self.snapshot
                            .generic_fallbacks
                            .insert(CommandAllocationSite {
                                source: Arc::clone(&self.chunk.source),
                                offset,
                            });
                        return self.substitutions(words, context);
                    }
                    None
                }
                NativeControlPreparationStep::Expression(operand) => {
                    let Some(program) = recipe.expression_program(operand) else {
                        self.require_provider();
                        return None;
                    };
                    self.original_expression(
                        program,
                        offset,
                        context,
                        recipe.expression_error_context(&program.operand),
                    )
                }
                NativeControlPreparationStep::DeclareLocal(_)
                | NativeControlPreparationStep::DeclareAnonymousLocal
                | NativeControlPreparationStep::Literal(_)
                | NativeControlPreparationStep::Integer(_)
                | NativeControlPreparationStep::List(_) => None,
            };
            if failure.is_some() {
                return failure;
            }
        }
        if let Some(rejection) = rejection {
            return Some(self.failure_at(offset, rejection.clone(), Vec::new()));
        }
        if generic {
            self.substitutions(words, context)
        } else {
            None
        }
    }

    fn append_body_error_context(
        &self,
        failure: &mut SourceNativeCompilationFailure,
        recipe: &NativeInstructionPlan,
        operand: &NativeCompilerWordOperand,
        span: tcl_lexer::Span,
    ) {
        if let Some(error_context) = recipe.body_error_context(operand)
            && let Some(inner) = failure.contexts.last_mut()
        {
            let line = inner
                .invocation
                .offset
                .checked_sub(span.start())
                .map(|offset| {
                    tcl_lexer::LineIndex::from_bytes(
                        &self.chunk.source.source_image().bytes()[span.as_range()],
                    )
                    .line_at(offset)
                })
                .and_then(|line| line.checked_add(1));
            if let Some(note) = error_context.note(line).filter(|note| !note.is_empty()) {
                inner.after_context.push(note);
            }
        }
    }

    fn original_expression_steps(
        &self,
        program: &tcl_registry::native_expression_program::NativeExpressionProgram,
    ) -> (
        Vec<tcl_registry::native_compilation::NativeExpressionCompilerStep>,
        Option<tcl_runtime_api::native_compilation::NativeMathFunctionPrerequisite>,
    ) {
        use tcl_registry::native_compilation::NativeMathFunctionResolution as Resolution;
        let table = self
            .state
            .baseline
            .native_entry
            .as_ref()
            .and_then(|entry| entry.math_functions.as_ref());
        let mut used_math_table = false;
        let steps = program.compiler_steps(|name| {
            used_math_table = true;
            let Some(table) = table else {
                return Resolution::Unknown;
            };
            match table.lookup(name) {
                tcl_runtime_api::native_compilation::NativeMathFunctionResolution::Present(row) => {
                    row.arity
                        .map_or(Resolution::Unknown, |arity| Resolution::Known { arity })
                }
                tcl_runtime_api::native_compilation::NativeMathFunctionResolution::Absent => {
                    Resolution::Absent
                }
                tcl_runtime_api::native_compilation::NativeMathFunctionResolution::Unknown => {
                    Resolution::Unknown
                }
            }
        });
        let prerequisite = used_math_table
            .then(|| {
                let entry = self.state.baseline.native_entry.as_ref()?;
                Some(
                    tcl_runtime_api::native_compilation::NativeMathFunctionPrerequisite {
                        interpreter: entry.interpreter,
                        table: entry.math_functions.as_ref()?.clone(),
                    },
                )
            })
            .flatten();
        (steps, prerequisite)
    }

    fn original_expression(
        &mut self,
        program: &tcl_registry::native_expression_program::NativeExpressionProgram,
        offset: u32,
        context: SourceExecutionContext<'_>,
        error_context: Option<
            tcl_registry::native_compilation::NativeCompiledExpressionErrorContext,
        >,
    ) -> Option<SourceNativeCompilationFailure> {
        use tcl_registry::native_compilation::NativeExpressionCompilerStep as Step;
        let image = self.chunk.source.source_image().clone();
        if image.bytes().get(program.span.as_range()) != Some(program.source.as_slice()) {
            self.require_provider();
            return None;
        }
        let (steps, prerequisite) = self.original_expression_steps(program);
        self.retain_prefix_math_table(prerequisite.clone());
        for step in steps {
            let relative = match step {
                Step::Unknown => {
                    self.require_provider();
                    return None;
                }
                Step::Failure(failure) => {
                    if let Some(context) = error_context
                        && !context.note().is_empty()
                    {
                        self.before_next_context.push(context.note().to_owned());
                    }
                    let mut failure = self.failure_at(offset, failure, Vec::new());
                    failure.math_table_prerequisite.clone_from(&prerequisite);
                    return Some(failure);
                }
                Step::Script(span) => span,
            };
            let Some(spelling) = program.source.get(relative.as_range()).filter(|spelling| {
                spelling.first() == Some(&b'[')
                    && spelling.last() == Some(&b']')
                    && spelling.len() >= 2
            }) else {
                self.require_provider();
                return None;
            };
            let base = program
                .span
                .start()
                .checked_add(relative.start())?
                .checked_add(1)?;
            let source = tcl_lexer::SourceImage::from_bytes(
                &spelling[1..spelling.len() - 1],
                image.channel(),
            );
            if let Some(mut failure) = self.script(
                &source,
                base,
                SourceExecutionContext {
                    depth: context.depth + 1,
                    ..context
                },
            ) {
                if let Some(prerequisite) = &prerequisite {
                    if failure
                        .math_table_prerequisite
                        .as_ref()
                        .is_some_and(|old| old != prerequisite)
                    {
                        self.require_provider();
                        failure.failure.message = None;
                        failure.failure.error_code = None;
                        failure.failure.error_info = None;
                    } else {
                        failure.math_table_prerequisite = Some(prerequisite.clone());
                    }
                }
                if let Some(context) = error_context
                    && !context.note().is_empty()
                    && let Some(inner) = failure.contexts.last_mut()
                {
                    inner.after_context.push(context.note().to_owned());
                }
                return Some(failure);
            }
        }
        None
    }

    fn original_operand(
        &mut self,
        words: &[WordExpr],
        operand: &NativeCompilerWordOperand,
        context: SourceExecutionContext<'_>,
    ) -> Option<SourceNativeCompilationFailure> {
        match operand {
            NativeCompilerWordOperand::Original(index) => {
                let Some(word) = words.get(*index) else {
                    self.require_provider();
                    return None;
                };
                self.substitutions(std::slice::from_ref(word), context)
            }
            NativeCompilerWordOperand::LiteralExpansion { .. } => None,
        }
    }

    fn original_variable_operand(
        &mut self,
        words: &[WordExpr],
        target: &tcl_syntax::native_variable_words::NativeVariableWordOperand,
        target_word: usize,
        context: SourceExecutionContext<'_>,
    ) -> Option<SourceNativeCompilationFailure> {
        use tcl_syntax::native_variable_words::NativeVariableWordOperand;
        match target {
            NativeVariableWordOperand::Literal { .. } => None,
            NativeVariableWordOperand::DynamicWord => self.original_operand(
                words,
                &NativeCompilerWordOperand::Original(target_word),
                context,
            ),
            NativeVariableWordOperand::CompoundArray { index, .. } => {
                self.original_variable_index(index, context)
            }
        }
    }

    fn original_variable_index(
        &mut self,
        arena: &tcl_lexer::ExecutablePartArena,
        context: SourceExecutionContext<'_>,
    ) -> Option<SourceNativeCompilationFailure> {
        if arena.image() != self.chunk.source.source_image() {
            self.require_provider();
            return None;
        }
        let mut pending = vec![(arena.root(), 0, context.depth)];
        while let Some((list, next, depth)) = pending.pop() {
            if crate::depth_guard::MAX_SOURCE_NEST_DEPTH.exceeded(depth) {
                self.require_provider();
                continue;
            }
            let Some(component) = arena.list(list).get(next) else {
                continue;
            };
            pending.push((list, next + 1, depth));
            match component.part {
                tcl_lexer::ExecutablePart::Command { body } => {
                    let Some(script) = arena.bytes(body) else {
                        self.require_provider();
                        continue;
                    };
                    if let Some(failure) = self.script(
                        &tcl_lexer::SourceImage::from_bytes(script, arena.image().channel()),
                        body.start(),
                        SourceExecutionContext {
                            depth: depth + 1,
                            ..context
                        },
                    ) {
                        return Some(failure);
                    }
                }
                tcl_lexer::ExecutablePart::Variable {
                    index: Some(index), ..
                } => {
                    pending.push((index, 0, depth + 1));
                }
                tcl_lexer::ExecutablePart::Expression { .. }
                | tcl_lexer::ExecutablePart::ParseError(_) => self.possible_error = true,
                _ => {}
            }
        }
        None
    }

    fn original_script(
        &mut self,
        words: &[WordExpr],
        offset: u32,
        span: tcl_lexer::Span,
        context: SourceExecutionContext<'_>,
        compilation: tcl_registry::native_compilation::NativeCompilationContext,
        publish: bool,
    ) -> Option<SourceNativeCompilationFailure> {
        let image = self.chunk.source.source_image().clone();
        let Some(bytes) = image.bytes().get(span.as_range()) else {
            self.require_provider();
            return None;
        };
        let source = tcl_lexer::SourceImage::from_bytes(bytes, image.channel());
        if let Some(failure) = self.script(
            &source,
            span.start(),
            SourceExecutionContext {
                compilation,
                depth: context.depth + 1,
                ..context
            },
        ) {
            return Some(failure);
        }
        if publish && !self.possible_error {
            let selected = super::super::compiled_invocation::select_invocation_boxed(
                words,
                self.state,
                &SourceExecutionContext {
                    compilation_snapshot: Some(self.snapshot),
                    ..context
                },
                offset,
            );
            let Some(parent) = selected.admitted.as_ref().filter(|parent| {
                matches!(
                    parent.as_ref(),
                    super::super::compiled_invocation::SourceNativeCompilerAdmission::Inline(_)
                )
            }) else {
                self.require_provider();
                return None;
            };
            self.compiled_children
                .entry(CommandAllocationSite {
                    source: Arc::clone(&self.chunk.source),
                    offset: span.start(),
                })
                .or_default()
                .push(SourceCompiledChild {
                    enclosing: self.chunk.clone(),
                    table: Arc::clone(&self.snapshot.table),
                    parent: Arc::clone(parent),
                    script: super::super::ExecutedScriptSource {
                        text: source,
                        origin: Arc::clone(&self.chunk.source),
                        mapping: super::super::ExecutedScriptMapping::Contiguous {
                            base: span.start(),
                        },
                    },
                    compilation,
                    parent_namespace_key: context.namespace_identity(),
                    namespace_key: context.namespace_identity(),
                    compiler_visits: Arc::default(),
                });
        }
        None
    }
}

fn entered_context(
    body: tcl_registry::native_compilation::NativeCompiledBodyContext,
    enclosing: tcl_registry::native_compilation::NativeCompilationContext,
) -> Option<tcl_registry::native_compilation::NativeCompilationContext> {
    use tcl_registry::native_compilation::NativeCompiledBodyContext;
    Some(match body {
        NativeCompiledBodyContext::Inherit => enclosing,
        NativeCompiledBodyContext::ExceptionRange => enclosing.with_inline_exception_range(),
        NativeCompiledBodyContext::Loop => {
            tcl_registry::native_compilation::NativeCompilationContext {
                loop_depth: enclosing.loop_depth.checked_add(1)?,
                ..enclosing
            }
        }
    })
}
