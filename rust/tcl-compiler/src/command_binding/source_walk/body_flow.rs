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

//! Body operands of a native handler: sequences, loops, conditionals and materialised scripts.

use super::super::command_identity::CommandAllocationSite;
use super::super::executed_script_source::{ExecutedScriptMapping, ExecutedScriptSource};
use super::super::source_origin::{MaterialisedSourceKind, SourceOriginId};
use super::super::variable_access::SourceVariableEvaluationOwner;
use super::super::{
    ModuleCommandBindings, SourceCommandBindings, SourceExecutionContext, SourceLoopOperands,
    SourceNativeInvocation, SourceOutcomes, SourceScriptOperands, compiled_invocation,
    compiler_inventory,
};
use super::execution_context::source_loop_context_boxed;
use super::expression_walk::source_operand_truth;
use super::outcomes::{boxed_source_branch, opaque_source_invocation};
use super::target_walk::resolve_source_invocation_facts;
use std::sync::Arc;
use tcl_registry::CommandRegistry;

impl<'a> SourceNativeInvocation<'a> {
    pub(in super::super) fn script_operands(self) -> SourceScriptOperands<'a> {
        SourceScriptOperands {
            compilation_spec: self.compilation_spec,
            compilation_selection: self.compilation_selection,
            words: self.words,
            written_arguments: self.written_arguments,
            target: self.target,
            arguments: self.invocation.arguments_ref(),
        }
    }
}

pub(super) fn frozen_script_operands(
    arguments: tcl_registry::InvocationArguments<'_>,
    from: usize,
) -> Option<Vec<&str>> {
    let count = arguments.exact_argv_len()?;
    (from..count)
        .map(|index| arguments.literal_at(index))
        .collect()
}

/// Preserve the enclosing admission realm only for literal source operands.
/// Decoding literal escapes does not turn authored code into a runtime script;
/// substitution and alias-prefix values do, independently of source mapping.
pub(in super::super) fn evaluated_script_realm(
    arguments: std::ops::Range<usize>,
    operands: SourceScriptOperands<'_>,
    context: SourceExecutionContext<'_>,
) -> tcl_dialect::model::InvocationRealm {
    let rules = tcl_syntax::word_rules::WordValueRules::from_config(&context.config);
    if arguments.into_iter().all(|index| {
        operands.written_word(index).is_some_and(|word| {
            matches!(
                crate::registry_invocation::effective_invocation_word(
                    word,
                    context.config.escapes,
                    rules,
                ),
                crate::registry_invocation::EffectiveInvocationWord::Literal(_)
            )
        })
    }) {
        context.realm()
    } else {
        tcl_dialect::model::InvocationRealm::InterpreterRuntime
    }
}

/// Retain the derived body context on the heap while its script descends.
/// Its native realm and compiler inheritance are selected before entry.
#[inline(never)]
fn boxed_body_operand_context<'a>(
    index: usize,
    operands: SourceScriptOperands<'_>,
    state: &ModuleCommandBindings,
    context: &SourceExecutionContext<'a>,
) -> Box<SourceExecutionContext<'a>> {
    let realm = evaluated_script_realm(index..index + 1, operands, *context);
    let context = compiled_invocation::body_context(index, operands, state, *context);
    Box::new(SourceExecutionContext {
        control: (context).control.with_realm(realm),
        ..context
    })
}

pub(super) fn authored_braced_operand<'a>(
    native: SourceNativeInvocation<'a>,
    argument: usize,
    value: &str,
) -> Option<&'a tcl_source::words::SourceSite> {
    match native.script_operands().written_word(argument)? {
        tcl_source::words::WordExpr::BracedLiteral { text, source } if text == value => {
            Some(source)
        }
        _ => None,
    }
}

pub(in super::super) fn retained_script_operand(
    argument: usize,
    operands: SourceScriptOperands<'_>,
    state: &ModuleCommandBindings,
    invocation: u32,
    config: tcl_lexer::LexerConfig,
) -> Option<ExecutedScriptSource> {
    let parent = CommandAllocationSite {
        source: Arc::clone(state.current_source_origin.as_ref()?),
        offset: invocation,
    };
    if let Some(bytes) = operands.arguments.native_bytes_at(argument) {
        if let Some(crate::registry_invocation::InvocationWordOrigin::Written(written)) =
            operands.written_origin(argument)
            && let Some(protocol) =
                compiler_inventory::SourceNativeCompilerPolicy::source_protocol_of(state)
            && let Some(original) =
                parent
                    .source
                    .original_lexical_command(operands.words, invocation, config)
            && let Some(word) = original.native_words().get(written)
        {
            return Some(ExecutedScriptSource::from_original_word_value(
                parent, argument, word, bytes, protocol,
            ));
        }
        return Some(ExecutedScriptSource::materialised_image(
            parent,
            vec![argument],
            tcl_lexer::SourceImage::native(Arc::<[u8]>::from(bytes)),
        ));
    }
    let value = operands.arguments.literal_at(argument)?;
    let script = operands.written_word(argument).map_or_else(
        || ExecutedScriptSource::materialised(parent.clone(), vec![argument], value),
        |word| ExecutedScriptSource::from_word(parent.clone(), argument, word, value, config),
    );
    Some(script)
}

#[derive(Clone, Copy)]
struct SourceLoopBodyIndices<'a> {
    pub(in super::super) initial: &'a [usize],
    pub(in super::super) conditions: &'a [usize],
    pub(in super::super) repeated: &'a [usize],
    pub(in super::super) continued: &'a [usize],
}

fn source_iteration_variable_lists(
    native: SourceNativeInvocation<'_>,
    registry: &CommandRegistry,
    realm: tcl_dialect::model::InvocationRealm,
) -> Vec<usize> {
    resolve_source_invocation_facts(registry, None, *native.invocation, realm).map_or_default(
        |facts| {
            facts
                .arg_roles
                .iter()
                .filter_map(|&(index, role)| {
                    (role == tcl_registry::ArgRole::LoopVarList)
                        .then_some(facts.argument_offset + usize::from(index))
                })
                .collect()
        },
    )
}

impl SourceCommandBindings {
    pub(in super::super) fn walk_expression_sequence(
        &mut self,
        indices: &[usize],
        operands: SourceScriptOperands<'_>,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        let mut evaluated = SourceOutcomes::normal(state);
        for &index in indices {
            let Some(mut normal) = evaluated.normal.take() else {
                break;
            };
            evaluated.join(&self.walk_expression_operand(index, operands, &mut normal, context));
        }
        evaluated
    }

    pub(super) fn walk_body_flow(
        &mut self,
        flow: &tcl_registry::script_body_flow::ScriptBodyFlow,
        native: SourceNativeInvocation<'_>,
        state: &mut ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
        list_tail: bool,
    ) -> SourceOutcomes {
        use tcl_registry::script_body_flow::ScriptBodyFlow;
        let outcomes = match flow {
            ScriptBodyFlow::Conditional(branches) => {
                self.walk_conditional(branches, native.script_operands(), state, *context)
            }
            ScriptBodyFlow::Loop {
                initial,
                conditions,
                repeated,
                continued,
            } => self.walk_loop_body_flow(
                SourceLoopBodyIndices {
                    initial,
                    conditions,
                    repeated,
                    continued,
                },
                native,
                state,
                context,
            ),
            other => self.walk_non_loop_body_flow(other, native, state, context, list_tail),
        };
        outcomes.publish(state);
        outcomes
    }

    fn walk_loop_body_flow(
        &mut self,
        indices: SourceLoopBodyIndices<'_>,
        native: SourceNativeInvocation<'_>,
        state: &mut ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        let variable_lists =
            source_iteration_variable_lists(native, context.registry, context.realm());
        let facts = resolve_source_invocation_facts(
            context.registry,
            None,
            *native.invocation,
            context.realm(),
        );
        let entry = facts.as_ref().map_or(
            tcl_registry::iteration_entry::IterationEntry::Unknown,
            |facts| facts.iteration_entry(native.invocation.arguments()),
        );
        let finite = facts
            .as_ref()
            .and_then(|facts| facts.finite_iteration(native.invocation.arguments()));
        let loop_context = source_loop_context_boxed(context);
        self.walk_source_loop(
            SourceLoopOperands {
                entry,
                finite: finite.as_ref(),
                variable_lists: &variable_lists,
                initial: indices.initial,
                conditions: indices.conditions,
                repeated: indices.repeated,
                continued: indices.continued,
            },
            native.script_operands(),
            state,
            &loop_context,
        )
    }

    /// Borrow the selected grammar across recursive bodies. Owning a clone of
    /// the largest selection would charge every nested sequence for its stack.
    fn walk_non_loop_body_flow(
        &mut self,
        flow: &tcl_registry::script_body_flow::ScriptBodyFlow,
        native: SourceNativeInvocation<'_>,
        state: &mut ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
        list_tail: bool,
    ) -> SourceOutcomes {
        use tcl_registry::script_body_flow::ScriptBodyFlow;
        match flow {
            ScriptBodyFlow::Deferred | ScriptBodyFlow::None | ScriptBodyFlow::CapturedLifecycle => {
                SourceOutcomes::normal(state)
            }
            ScriptBodyFlow::Sequence(indices) => {
                self.walk_body_sequence(indices, native.script_operands(), state, context)
            }
            ScriptBodyFlow::ConcatenatedScript { argument_offset } => {
                self.walk_concatenated_script(*argument_offset, native, state, context, list_tail)
            }
            other => self.walk_other_body_flow(other, native, state, context),
        }
    }

    /// Other grammar temporaries stay outside source-sequence recursion.
    #[inline(never)]
    fn walk_other_body_flow(
        &mut self,
        flow: &tcl_registry::script_body_flow::ScriptBodyFlow,
        native: SourceNativeInvocation<'_>,
        state: &mut ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        use tcl_registry::script_body_flow::ScriptBodyFlow;
        match flow {
            ScriptBodyFlow::ConcatenatedExpression { argument_offset } => {
                self.walk_concatenated_expression(*argument_offset, native, state, *context)
            }
            ScriptBodyFlow::Lambda(_)
            | ScriptBodyFlow::DictionaryScope(_)
            | ScriptBodyFlow::Capture(_) => {
                self.walk_owned_body_selection(flow, native, state, *context)
            }
            ScriptBodyFlow::Expressions(indices) => {
                self.walk_expression_sequence(indices, native.script_operands(), state, *context)
            }
            ScriptBodyFlow::Try(selection) => {
                self.walk_try_bodies(selection.as_ref(), native, state, *context)
            }
            ScriptBodyFlow::Unknown(indices) => {
                state.mark_opaque_binding_mutation();
                self.walk_possible_body_alternatives(indices, native, state, *context, false)
            }
            ScriptBodyFlow::CaseBodies(selection) => {
                self.walk_case_bodies(selection, native, state, *context)
            }
            ScriptBodyFlow::Alternatives(indices) => {
                self.walk_possible_body_alternatives(indices, native, state, *context, true)
            }
            ScriptBodyFlow::Conditional(_) => {
                unreachable!("conditional flow uses its ordered condition handler")
            }
            ScriptBodyFlow::Loop { .. } => unreachable!("loop flow uses its iteration handler"),
            ScriptBodyFlow::Deferred
            | ScriptBodyFlow::None
            | ScriptBodyFlow::CapturedLifecycle
            | ScriptBodyFlow::Sequence(_)
            | ScriptBodyFlow::ConcatenatedScript { .. } => {
                unreachable!("source sequences use their borrowed context")
            }
        }
    }

    /// Only these handlers consume a selection. Their owned temporaries stay
    /// outside the common sequence/namespace recursion path.
    #[inline(never)]
    fn walk_owned_body_selection(
        &mut self,
        flow: &tcl_registry::script_body_flow::ScriptBodyFlow,
        native: SourceNativeInvocation<'_>,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        use tcl_registry::script_body_flow::ScriptBodyFlow;
        match flow {
            ScriptBodyFlow::Lambda(selection) => {
                self.walk_lambda_invocation(selection.clone(), native, state, context)
            }
            ScriptBodyFlow::DictionaryScope(selection) => {
                self.walk_dictionary_scope(selection.clone(), native, state, context)
            }
            ScriptBodyFlow::Capture(selection) => {
                self.walk_captured_operand(*selection, native.script_operands(), state, context)
            }
            _ => unreachable!("only owned selections enter this handler"),
        }
    }

    fn walk_possible_body_alternatives(
        &mut self,
        indices: &[usize],
        native: SourceNativeInvocation<'_>,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
        retain_unchanged: bool,
    ) -> SourceOutcomes {
        let mut possible = if retain_unchanged {
            SourceOutcomes::normal(state)
        } else {
            SourceOutcomes::invocation(
                state,
                tcl_registry::completion_route::InvocationCompletionRoute::Unknown,
            )
        };
        let incoming = boxed_source_branch(state);
        let context = context.conditional();
        for &index in indices {
            let mut branch = boxed_source_branch(&incoming);
            possible.join(&self.walk_body_operand(
                index,
                native.script_operands(),
                &mut branch,
                &context,
            ));
        }
        possible
    }

    fn walk_captured_operand(
        &mut self,
        selection: tcl_registry::catch_invocation::CatchInvocationSelection,
        operands: SourceScriptOperands<'_>,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        use tcl_registry::catch_invocation::CatchInvocationSelection;
        use tcl_registry::completion_route::InvocationCompletionRoute as Route;
        match selection {
            CatchInvocationSelection::Valid(selected) => self
                .walk_body_operand(selected.script_at, operands, state, &context)
                .capture_with(selected),
            CatchInvocationSelection::Invalid => SourceOutcomes::invocation(
                state,
                Route::Tcl(tcl_registry::completion::CompletionCode::Error),
            ),
            CatchInvocationSelection::Unknown => {
                state.mark_opaque_binding_mutation();
                SourceOutcomes::invocation(state, Route::Unknown)
            }
        }
    }

    pub(in super::super) fn walk_body_sequence(
        &mut self,
        indices: &[usize],
        operands: SourceScriptOperands<'_>,
        state: &mut ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        let mut outcomes = SourceOutcomes::normal(state);
        for index in indices {
            let Some(mut normal) = outcomes.normal.take() else {
                break;
            };
            outcomes.join(&self.walk_body_operand(*index, operands, &mut normal, context));
        }
        outcomes.publish(state);
        outcomes
    }

    fn walk_concatenated_script(
        &mut self,
        argument_offset: usize,
        native: SourceNativeInvocation<'_>,
        state: &mut ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
        list_tail: bool,
    ) -> SourceOutcomes {
        if let Some(count) = native.invocation.arguments().exact_argv_len()
            && argument_offset.checked_add(1) == Some(count)
        {
            return self.walk_body_operand(
                argument_offset,
                native.script_operands(),
                state,
                context,
            );
        }
        self.walk_materialised_concatenated_script(
            argument_offset,
            native,
            state,
            context,
            list_tail,
        )
    }

    /// Materialisation temporaries are absent from an original single-body descent.
    #[inline(never)]
    fn walk_materialised_concatenated_script(
        &mut self,
        argument_offset: usize,
        native: SourceNativeInvocation<'_>,
        state: &mut ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
        list_tail: bool,
    ) -> SourceOutcomes {
        let Some(values) = frozen_script_operands(native.invocation.arguments(), argument_offset)
        else {
            return opaque_source_invocation(state);
        };
        let script = if list_tail {
            let Some(script) = super::super::original_delegated_script::append_list_tail(&values)
            else {
                return opaque_source_invocation(state);
            };
            script
        } else if values.len() == 1 {
            values[0].to_owned()
        } else if native
            .invocation
            .arguments()
            .dialect()
            .is_some_and(|dialect| {
                matches!(
                    dialect.concat_policy(),
                    Some(tcl_dialect::ConcatPolicy::Tcl(_))
                )
            })
        {
            tcl_syntax::list::concat_values(values.iter().copied())
        } else {
            return opaque_source_invocation(state);
        };
        let realm = evaluated_script_realm(
            argument_offset..argument_offset + values.len(),
            native.script_operands(),
            *context,
        );
        let mut context = compiled_invocation::body_context(
            argument_offset,
            native.script_operands(),
            state,
            *context,
        );
        if list_tail
            && native
                .invocation
                .arguments()
                .dialect()
                .and_then(tcl_registry::InvocationDialect::native_eval_object_protocol)
                .is_some_and(|protocol| {
                    !protocol.compiles_source(
                        tcl_registry::native_eval_object::EvalObjectPurpose::NamespaceConcat,
                    )
                })
        {
            context.control = context.control.with_compilation(
                tcl_registry::native_compilation::NativeCompilationContext {
                    mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                    ..context.compilation()
                },
            );
        }
        self.walk_materialised_source(
            &script,
            argument_offset..argument_offset + values.len(),
            MaterialisedSourceKind::Script,
            state,
            SourceExecutionContext {
                control: (context).control.with_realm(realm),
                ..context
            },
        )
    }

    pub(super) fn walk_materialised_source(
        &mut self,
        source: &str,
        arguments: std::ops::Range<usize>,
        kind: MaterialisedSourceKind,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        let Some(origin) = state.current_source_origin.clone() else {
            return opaque_source_invocation(state);
        };
        let expression_owner = SourceVariableEvaluationOwner::NativeExpression {
            invocation: CommandAllocationSite {
                source: Arc::clone(&origin),
                offset: context.invocation_offset,
            },
            parent: context.variable_read_owner.cloned().map(Arc::new),
        };
        let indices = arguments.collect::<Vec<_>>();
        let parent = CommandAllocationSite {
            source: Arc::clone(&origin),
            offset: context.invocation_offset,
        };
        let derived = Arc::new(SourceOriginId::derived(
            parent.clone(),
            indices.clone(),
            &Arc::from(source),
            kind,
        ));
        if kind == MaterialisedSourceKind::Script {
            let script = ExecutedScriptSource {
                text: tcl_lexer::SourceImage::native(Arc::<[u8]>::from(source.as_bytes())),
                origin: Arc::clone(&derived),
                mapping: ExecutedScriptMapping::Materialised,
            };
            for argument in indices {
                self.record_executed_script(
                    parent.clone(),
                    argument,
                    script.clone(),
                    context.entered_frame(),
                    state.source_variables.namespace_identity.as_ref(),
                );
            }
        }
        state.current_source_origin = Some(Arc::clone(&derived));
        let mut outcomes = if kind == MaterialisedSourceKind::Expression {
            self.walk_prepared_expression(
                parent,
                ExecutedScriptSource {
                    text: tcl_lexer::SourceImage::native(Arc::<[u8]>::from(source.as_bytes())),
                    origin: derived,
                    mapping: ExecutedScriptMapping::Materialised,
                },
                state,
                SourceExecutionContext {
                    variable_read_owner: Some(&expression_owner),
                    ..context
                },
            )
        } else {
            self.walk_source(
                source,
                0,
                state,
                &SourceExecutionContext {
                    control: (context)
                        .control
                        .with_source_purpose(tcl_lexer::SourceSegmentationPurpose::NestedScript),
                    ..context
                },
            )
        };
        outcomes.restore_source_origin(Some(&origin));
        outcomes.publish(state);
        outcomes
    }

    pub(in super::super) fn walk_conditional(
        &mut self,
        branches: &[(Option<usize>, usize)],
        operands: SourceScriptOperands<'_>,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        let mut outcomes = SourceOutcomes::default();
        let mut fallthrough = Some(boxed_source_branch(state));
        let mut branch_context = context;
        let mut selected = None;
        for (position, (condition, body)) in branches.iter().enumerate() {
            let Some(mut incoming) = fallthrough.take() else {
                if let Some(selected) = selected {
                    self.walk_unreached_branches(
                        &branches[position..],
                        operands,
                        selected,
                        context,
                    );
                }
                break;
            };
            if let Some(condition) = condition {
                let evaluated = self.walk_expression_operand(
                    *condition,
                    operands,
                    &mut incoming,
                    branch_context,
                );
                for (route, state) in &evaluated.abrupt {
                    outcomes.add_abrupt(*route, state);
                }
                let Some(normal) = evaluated.normal else {
                    break;
                };
                incoming = normal;
                let truth = source_operand_truth(*condition, operands, &incoming, context.registry);
                if truth.is_none() {
                    branch_context = branch_context.conditional();
                }
                if truth != Some(true) {
                    fallthrough = Some(incoming.clone());
                }
                if truth == Some(false) {
                    self.walk_unreached_body(*body, operands, &incoming, context);
                    continue;
                }
                if truth == Some(true) {
                    selected = Some(incoming.clone());
                }
            }
            outcomes.join(&self.walk_body_operand(*body, operands, &mut incoming, &branch_context));
        }
        if let Some(fallthrough) = fallthrough {
            outcomes.join(&SourceOutcomes::normal(&fallthrough));
        }
        outcomes.publish(state);
        outcomes
    }

    /// The original compiler still compiles a body the walk proves unreached
    /// when its condition is not a compile-time constant. Its commands bind in
    /// the incoming command world as never entered; no outcome reaches the
    /// successor.
    fn walk_unreached_body(
        &mut self,
        body: usize,
        operands: SourceScriptOperands<'_>,
        incoming: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) {
        let mut unreached = boxed_source_branch(incoming);
        self.unreached_walk_depth += 1;
        drop(self.walk_body_operand(body, operands, &mut unreached, &context.conditional()));
        self.unreached_walk_depth -= 1;
    }

    /// Branches after a proved-true condition are compiled but never entered.
    fn walk_unreached_branches(
        &mut self,
        branches: &[(Option<usize>, usize)],
        operands: SourceScriptOperands<'_>,
        incoming: Box<ModuleCommandBindings>,
        context: SourceExecutionContext<'_>,
    ) {
        let context = context.conditional();
        let mut unreached = incoming;
        for (condition, body) in branches {
            if let Some(condition) = condition {
                self.unreached_walk_depth += 1;
                let evaluated =
                    self.walk_expression_operand(*condition, operands, &mut unreached, context);
                self.unreached_walk_depth -= 1;
                let Some(normal) = evaluated.normal else {
                    return;
                };
                unreached = normal;
            }
            self.walk_unreached_body(*body, operands, &unreached, context);
        }
    }

    pub(in super::super) fn walk_body_operand(
        &mut self,
        index: usize,
        operands: SourceScriptOperands<'_>,
        state: &mut ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        let context = boxed_body_operand_context(index, operands, state, context);
        let formal = self.script_formal_operand(
            state.current_source_origin.as_ref(),
            operands.written_word(index),
            operands.arguments.literal_at(index),
            context.config,
        );
        // `uplevel 1` in the entered body selects the activation whose
        // literal this is, which the walk knows by name.
        let caller_frame = formal
            .as_ref()
            .filter(|_| {
                matches!(
                    context.frame,
                    crate::var_resolve::VariableExecutionFrame::Selected {
                        selector: tcl_registry::FrameLevel::Relative(1),
                        ..
                    }
                )
            })
            .map(|(_, frame)| frame.clone());
        let context = match &caller_frame {
            Some(frame) => Box::new(SourceExecutionContext { frame, ..*context }),
            None => context,
        };
        let Some(script) = formal.map(|(script, _)| script).or_else(|| {
            retained_script_operand(
                index,
                operands,
                state,
                context.invocation_offset,
                context.config,
            )
        }) else {
            return opaque_source_invocation(state);
        };
        let Some(origin) = &state.current_source_origin else {
            return opaque_source_invocation(state);
        };
        let parent = CommandAllocationSite {
            source: Arc::clone(origin),
            offset: context.invocation_offset,
        };
        if let Some(word) = operands.written_word(index) {
            self.record_executed_word(&parent.source, word.source().span, None, script.clone());
        }
        if context.entered_frame().is_some() {
            self.record_executed_script(
                parent,
                index,
                script.clone(),
                context.entered_frame(),
                state.source_variables.namespace_identity.as_ref(),
            );
        } else {
            // The selected body context names the namespace it would run in.
            self.record_conditional_in_frame_script(
                parent,
                index,
                script.clone(),
                Some(&context.namespace_identity()),
            );
        }
        let previous_origin = state
            .current_source_origin
            .replace(Arc::clone(&script.origin));
        let mut outcomes = self.walk_source_image(&script.text, script.base(), state, &context);
        outcomes.restore_source_origin(previous_origin.as_ref());
        outcomes.publish(state);
        outcomes
    }
}
