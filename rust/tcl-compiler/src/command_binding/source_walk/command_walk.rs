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

//! One source command: its argument evaluation, the point it records and its target selection.

use super::super::binding_projection::PointWalkEntry;
use super::super::command_identity::CommandAllocationSite;
use super::super::invocation_binding::SourceObserverExecution;
use super::super::variable_access::SourceVariableEvaluationOwner;
use super::super::{
    ModuleCommandBindings, PointBindingMemo, PreparedSourceArguments, SourceArgumentReceipts,
    SourceBindingPoint, SourceCommandBindings, SourceCommandInput, SourceExecutionContext,
    SourceOutcomes, compiled_invocation, native_result, normal_result, original_name_value,
    pre_handler_failure, receiver_self, root_owner_work, rule_declaration, source_arguments,
};
use super::command_dispatch::retain_source_native_result;
use super::execution_context::{
    source_argument_context_boxed, source_execution_context_boxed,
    source_original_compilation_context_boxed, source_selected_context_boxed,
};
use super::outcomes::{
    boxed_source_branch, boxed_source_outcomes, join_source_procedure_noop,
    opaque_source_invocation, publish_source_branch,
};
use std::sync::{Arc, OnceLock};

/// Build source word carriers in a leaf frame rather than retaining their
/// inline construction temporary through recursive command substitution.
#[inline(never)]
pub(in super::super) fn source_command_tokens_boxed(
    image: &tcl_lexer::SourceImage,
    base: u32,
    config: tcl_lexer::LexerConfig,
    segment: &crate::segmenter::SegmentedCommand,
) -> Box<crate::ir::CommandTokens> {
    #[cfg(any(test, debug_assertions))]
    let _timing = root_owner_work::Timing::at(root_owner_work::Query::SourceWordTokens);
    Box::new(crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::from_image(image).with_base(base, 0, 0),
        config,
        segment,
    ))
}

#[cfg(test)]
fn trace_original_selection(
    words: &[crate::ir::WordExpr],
    site: u32,
    state: &ModuleCommandBindings,
    context: &SourceExecutionContext<'_>,
    compiled: &compiled_invocation::CompiledInvocationSelection,
) {
    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
        let head = original_name_value::original_static_command_head_input(
            words,
            site,
            state,
            context.config,
        );
        eprintln!(
            "ORIGINAL_SELECTION site={} words={} head={:?} mode={:?} admission={:?} live={} unknown={} error={} namespace={:?} name_policy={:?}",
            site,
            words.len(),
            head.as_ref().map(
                super::super::super::signature_scan::name_value::SignatureSourceNameInput::bytes
            ),
            context.compilation().mode,
            compiled.admission,
            compiled.live,
            compiled.unknown,
            compiled.compile_error,
            context.namespace_identity(),
            state.source_variables.execution_name_policy
        );
    }
}

#[cfg(test)]
fn trace_original_arguments(
    words: &[crate::ir::WordExpr],
    site: u32,
    state: &ModuleCommandBindings,
    context: &SourceExecutionContext<'_>,
    arguments: &PreparedSourceArguments,
) {
    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
        let head = original_name_value::original_command_head_input(words, site, state, *context);
        eprintln!(
            "ORIGINAL_ARGV site={} complete={} effective={:?} written={} names={} head={:?}",
            site,
            arguments.complete_normally,
            arguments.effective,
            arguments.written_arguments.len(),
            arguments
                .written_name_values
                .iter()
                .filter(|value| value.is_some())
                .count(),
            head.as_ref().map(
                super::super::super::signature_scan::name_value::SignatureSourceNameInput::bytes
            )
        );
    }
}

fn note_source_command_lookup(
    state: &mut ModuleCommandBindings,
    compiled: &compiled_invocation::CompiledInvocationSelection,
    effective: &[crate::registry_invocation::EffectiveInvocationWord],
) {
    if (compiled.live || compiled.unknown)
        && let (Some(pool), Some(dialect)) =
            (&mut state.ordinary_literal_pool, state.baseline.dialect)
        && let Some(head) = effective
            .first()
            .and_then(|word| word.as_registry_word().literal())
    {
        pool.note_command_lookup(head, dialect);
    }
}

impl PreparedSourceArguments {
    fn finish_normal_completions(&mut self, observed: bool) {
        self.outcomes.normal_completion = self
            .outcomes
            .normal_completion
            .filter(|_| self.complete_normally && !observed);
        self.outcomes.complete_procedure_return = self
            .outcomes
            .complete_procedure_return
            .filter(|_| self.complete_normally && !observed);
    }

    /// Construct the owned preparation only in a nonrecursive leaf frame.
    #[inline(never)]
    pub(in super::super) fn boxed(word_count: usize) -> Box<Self> {
        Box::new(Self {
            effective: Vec::with_capacity(word_count),
            written_arguments: Arc::from([]),
            written_values: Vec::with_capacity(word_count),
            written_name_values: Vec::with_capacity(word_count),
            written_representations: Vec::with_capacity(word_count),
            written_objects: Vec::with_capacity(word_count),
            written_method_prefixes: Vec::with_capacity(word_count),
            written_variable_reads: Vec::with_capacity(word_count),
            outcomes: SourceOutcomes::default(),
            complete_normally: true,
            ready: false,
        })
    }

    #[inline(never)]
    pub(in super::super) fn into_outcomes(self: Box<Self>) -> SourceOutcomes {
        self.outcomes
    }
}

pub(in super::super) fn source_invocation_argument_owner(
    state: &ModuleCommandBindings,
    offset: u32,
    parent: Option<&SourceVariableEvaluationOwner>,
) -> Option<SourceVariableEvaluationOwner> {
    state.current_source_origin.as_ref().map(|source| {
        SourceVariableEvaluationOwner::InvocationArguments {
            invocation: CommandAllocationSite {
                source: Arc::clone(source),
                offset,
            },
            parent: parent.cloned().map(Arc::new),
        }
    })
}

fn empty_source_command_outcomes(
    state: &ModuleCommandBindings,
    earlier: &SourceOutcomes,
) -> SourceOutcomes {
    let mut result = SourceOutcomes::normal(state);
    result.normal_value = Some(Arc::new(native_result::EvaluatedSourceValue {
        text: String::new(),
        representation: tcl_syntax::value::ValueRepresentation::Unknown,
        numeric: None,
    }));
    result.join(earlier);
    result
}

impl SourceCommandBindings {
    pub(super) fn walk_source_command(
        &mut self,
        image: &tcl_lexer::SourceImage,
        base: u32,
        state: &mut ModuleCommandBindings,
        segment: &crate::segmenter::SegmentedCommand,
        context: &SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        let SourceExecutionContext { config, .. } = *context;
        let Ok(source) = image.try_text() else {
            return opaque_source_invocation(state);
        };
        let words = source_command_tokens_boxed(image, base, config, segment);
        self.record_declaration_operand_layout(state, context, segment.span.start(), &words);
        let compiled = compiled_invocation::select_invocation_boxed(
            words.words(),
            state,
            context,
            segment.span.start(),
        );

        #[cfg(test)]
        trace_original_selection(
            words.words(),
            segment.span.start(),
            state,
            context,
            &compiled,
        );
        let original_variable_compilation =
            original_name_value::OriginalSourceVariableCompilation::from_selected(
                &compiled, state, *context,
            );
        let original_context = source_original_compilation_context_boxed(
            context,
            original_variable_compilation.as_ref(),
        );
        let context = original_context.as_ref();
        if let Some(outcomes) = self.walk_original_list_assignment_boxed(
            SourceCommandInput {
                image,
                segment,
                words: words.words(),
                effective: &[],
            },
            base,
            state,
            &compiled,
            context,
        ) {
            return *outcomes;
        }
        // Unknown opcode admission alone does not establish name effects.
        // Unclosed compiler effects can precede the original argv, so their
        // possible world cannot borrow argv-first absence.
        if compiled.unknown && !compiled.compile_error && !compiled.compiler_preserves_names() {
            state.mark_opaque_binding_mutation();
        }
        let argument_entry =
            pre_handler_failure::ArgumentEntry::capture(state, context, segment.span.start());
        let arguments =
            self.prepare_source_arguments(source, base, state, segment, words.words(), context);
        if !arguments.ready {
            self.record_pre_handler_failure(argument_entry, words.words(), &arguments.outcomes);
            return arguments.into_outcomes();
        }
        if arguments.effective.is_empty() {
            return empty_source_command_outcomes(state, &arguments.outcomes);
        }
        note_source_command_lookup(state, &compiled, &arguments.effective);
        self.walk_evaluated_source_command(
            SourceCommandInput {
                image,
                segment,
                words: words.words(),
                effective: &[],
            },
            state,
            &compiled,
            context,
            arguments,
        )
    }

    /// Keep the optional list-assignment result out of recursive source frames.
    #[inline(never)]
    fn walk_original_list_assignment_boxed(
        &mut self,
        input: SourceCommandInput<'_>,
        base: u32,
        state: &mut ModuleCommandBindings,
        compiled: &compiled_invocation::CompiledInvocationSelection,
        context: &SourceExecutionContext<'_>,
    ) -> Option<Box<SourceOutcomes>> {
        self.walk_original_list_assignment(input, base, state, compiled, context)
            .map(Box::new)
    }

    #[inline(never)]
    fn walk_evaluated_source_command(
        &mut self,
        input: SourceCommandInput<'_>,
        state: &mut ModuleCommandBindings,
        compiled: &compiled_invocation::CompiledInvocationSelection,
        context: &SourceExecutionContext<'_>,
        mut arguments: Box<PreparedSourceArguments>,
    ) -> SourceOutcomes {
        let SourceCommandInput {
            image,
            segment,
            words,
            ..
        } = input;
        let argument_context = source_argument_context_boxed(
            context,
            SourceArgumentReceipts {
                arguments: &arguments.written_arguments,
                values: &arguments.written_values,
                name_values: &arguments.written_name_values,
                representations: &arguments.written_representations,
                objects: &arguments.written_objects,
                prefixes: &arguments.written_method_prefixes,
                reads: &arguments.written_variable_reads,
            },
        );
        let context = Box::new(argument_context.context());
        #[cfg(test)]
        trace_original_arguments(words, segment.span.start(), state, &context, &arguments);

        let execution_owner = self.begin_invocation_reads(
            state.current_source_origin.as_ref(),
            segment.span.start(),
            context.variable_read_owner,
        );
        let mut execution_context = source_execution_context_boxed(
            &context,
            execution_owner.as_ref(),
            segment.span.start(),
        );
        let mut observer_scope =
            self.prepare_command_observers(&arguments.effective, state, &execution_context);
        arguments.outcomes.join(&observer_scope.entry);
        let Some(continuing) = observer_scope.entry.normal.take() else {
            self.finish_invocation_reads(execution_owner.as_ref(), &arguments.outcomes);
            arguments.outcomes.publish(state);
            return arguments.into_outcomes();
        };
        // Only abrupt callback paths join the eventual handler completion.
        arguments.outcomes.discard_normal_handler_entry();
        publish_source_branch(state, continuing);
        let input = SourceCommandInput {
            image,
            segment,
            words,
            effective: &arguments.effective,
        };
        self.autoload_dispatch_head(input, state, &execution_context);
        let normal_result_basis = self.record_evaluated_source_point(
            input,
            state,
            compiled,
            SourceObserverExecution::from_observed(observer_scope.was_observed),
            arguments.complete_normally && !observer_scope.was_observed,
            &context,
        );
        execution_context.withdraw_observed_operand_facts(observer_scope.was_observed);
        let dispatched =
            self.walk_selected_source_command(input, state, compiled, &execution_context);
        let dispatched =
            self.finish_command_observer_scope(dispatched, &observer_scope, &execution_context);
        self.complete_normal_variable_continuation(
            state.current_source_origin.as_ref(),
            segment.span.start(),
            &dispatched,
            observer_scope.was_observed,
        );
        arguments.outcomes.join(&dispatched);
        arguments.finish_normal_completions(observer_scope.was_observed);
        self.finish_invocation_reads(execution_owner.as_ref(), &arguments.outcomes);
        self.record_invocation_normal_result(normal_result_basis, &arguments.outcomes);
        arguments.outcomes.publish(state);
        arguments.into_outcomes()
    }

    /// Freeze argv in its own frame before retaining recursive body dispatch.
    #[inline(never)]
    fn prepare_source_arguments(
        &mut self,
        source: &str,
        base: u32,
        state: &mut ModuleCommandBindings,
        segment: &crate::segmenter::SegmentedCommand,
        words: &[crate::ir::WordExpr],
        context: &SourceExecutionContext<'_>,
    ) -> Box<PreparedSourceArguments> {
        #[cfg(any(test, debug_assertions))]
        let _timing = root_owner_work::Timing::at(root_owner_work::Query::SourceArguments);
        self.prepare_source_argument_range(
            source_arguments::SourceArgumentSlice {
                source,
                base,
                words,
                selected: 0..words.len(),
            },
            state,
            segment,
            context,
        )
    }

    /// Capture large point projections before entering any recursive body.
    #[inline(never)]
    pub(in super::super) fn record_evaluated_source_point(
        &mut self,
        input: SourceCommandInput<'_>,
        state: &ModuleCommandBindings,
        compiled: &compiled_invocation::CompiledInvocationSelection,
        observer_execution: SourceObserverExecution,
        arguments_complete_normally: bool,
        context: &SourceExecutionContext<'_>,
    ) -> Box<normal_result::InvocationResultBasis> {
        #[cfg(any(test, debug_assertions))]
        let _timing = root_owner_work::Timing::at(root_owner_work::Query::PointCapture);
        let SourceCommandInput {
            segment,
            words,
            effective,
            ..
        } = input;
        let head_word = words.first();
        let context = *context;
        let head = effective
            .first()
            .and_then(|word| word.as_registry_word().literal());
        let point = SourceBindingPoint {
            declaration_preview: self.declaration_preview_depth != 0,
            walk_entry: PointWalkEntry::at_depth(self.unreached_walk_depth),
            original_arguments_complete_normally: arguments_complete_normally,
            observer_execution,
            realm: context.realm(),
            compilation: context.compilation(),
            compiled_execution: compiled.clone(),
            head: head.map(str::to_owned),
            original_head_input: original_name_value::original_command_head_input(
                words,
                segment.span.start(),
                state,
                context,
            ),
            original_written_words: original_name_value::OriginalWrittenSourceWords::at_dispatch(
                words,
                segment.span.start(),
                state,
                context.config,
            )
            .map(Arc::new),
            evaluated_argument_values: context
                .written_arguments
                .unwrap_or(effective)
                .iter()
                .skip(1)
                .map(|word| word.as_registry_word().literal().map(str::to_owned))
                .collect(),
            evaluated_argument_words: context
                .written_arguments
                .unwrap_or(effective)
                .iter()
                .skip(1)
                .cloned()
                .collect(),
            frozen_written_words: context.written_arguments.map(Arc::from),
            frozen_written_names: context.written_name_values.map(Arc::from),
            frozen_head_object: head_word.and_then(|word| {
                context
                    .written_objects
                    .and_then(|words| words.first())
                    .and_then(Option::as_ref)
                    .map(|object| {
                        Arc::new(receiver_self::FrozenSourceObjectHead {
                            word: word.clone(),
                            object: Arc::clone(object),
                        })
                    })
            }),
            method_prefix_arguments: context
                .written_method_prefixes
                .into_iter()
                .flatten()
                .enumerate()
                .skip(1)
                .filter_map(|(written, prefix)| {
                    prefix
                        .as_ref()
                        .filter(|prefix| prefix.is_current(state))
                        .map(|prefix| (written - 1, Arc::clone(prefix)))
                })
                .collect(),
            offset: segment.span.start(),
            end: segment.span.end(),
            namespace: context.namespace.to_owned(),
            namespace_key: context.namespace_identity(),
            state: state.clone(),
            lookup_snapshot: OnceLock::new(),
            original_binding: PointBindingMemo::default(),
            dispatch: true,
            continuation_of: None,
        };
        let basis = Box::new(normal_result::InvocationResultBasis::of(&point, words));
        self.record_point(point);
        basis
    }

    fn walk_selected_source_command(
        &mut self,
        input: SourceCommandInput<'_>,
        state: &mut ModuleCommandBindings,
        compiled: &compiled_invocation::CompiledInvocationSelection,
        context: &SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        use tcl_registry::native_compilation::NativeCompilationSelection;
        #[cfg(any(test, debug_assertions))]
        let _timing = root_owner_work::Timing::at(root_owner_work::Query::SelectedCommand);
        let SourceCommandInput {
            image,
            segment,
            words,
            effective,
        } = input;
        let own_unknown_head = self.rule_loader_statement == Some((segment.span.start(), true));
        if let Some(rejection) =
            rule_declaration::rule_loader_rejection(effective, state, context, own_unknown_head)
        {
            self.rule_loader_rejection = Some(segment.span.start());
            let mut outcomes = SourceOutcomes::normal(state);
            if rejection == rule_declaration::RuleLoaderRejection::Unknown {
                // What the name would have run stays unanswered.
                outcomes.add_abrupt(
                    tcl_registry::completion_route::InvocationCompletionRoute::UnknownAbrupt,
                    state,
                );
            }
            return outcomes;
        }
        if let Some(outcomes) =
            self.prepare_selected_source_command(input, state, compiled, context)
        {
            return *outcomes;
        }
        if let Some(outcomes) =
            self.walk_logical_called_source(segment, words, effective, compiled, state, context)
        {
            return *outcomes;
        }
        if let Some(outcomes) =
            self.walk_logical_namespace_source(segment, words, effective, compiled, state, context)
        {
            return *outcomes;
        }
        // Called here rather than from the preparation above, so that frame
        // is gone before the survey's conditional descent recurses.
        if let Some(outcomes) =
            self.walk_logical_survey_command(segment, words, effective, compiled, state, context)
        {
            return *outcomes;
        }
        let mut outcomes = boxed_source_outcomes();
        let incoming = boxed_source_branch(state);
        if compiled.live {
            self.join_live_source_command(input, &incoming, context, &mut outcomes);
        }
        for proof in &compiled.proofs {
            if proof.operation
                == tcl_registry::SemanticOperationId::Intrinsic(
                    tcl_registry::IntrinsicId::ProcedureNoOp,
                )
            {
                join_source_procedure_noop(&mut outcomes, &incoming);
                continue;
            }
            let mut native = incoming.clone();
            let selection = NativeCompilationSelection::Inline {
                operation: proof.operation,
                guard: proof.guard,
            };
            let selected_entry =
                if proof.certainty == compiled_invocation::CompiledExecutionCertainty::May {
                    context.conditional()
                } else {
                    *context
                };
            let selected_context = source_selected_context_boxed(&selected_entry, &selection);
            let mut result = self.walk_source_target(
                segment,
                words,
                effective,
                &mut native,
                &proof.target,
                &selected_context,
            );
            retain_source_native_result(&mut result, &incoming, &proof.target, effective, context);
            outcomes.join(&result);
        }
        self.join_other_selected_source_commands(
            SourceCommandInput {
                image,
                segment,
                words,
                effective,
            },
            &incoming,
            compiled,
            context,
            &mut outcomes,
        );
        outcomes.publish(state);
        *outcomes
    }

    pub(in super::super) fn record_point(&mut self, point: SourceBindingPoint) {
        self.invalidate_unpositioned_projections();
        if self.root_origin != point.state.current_source_origin {
            self.record_origin_point(point);
            return;
        }
        let indices = self.dispatch_points.entry(point.offset).or_default();
        if let Some(index) = indices.iter().find(|index| {
            self.points[**index].namespace_key == point.namespace_key
                && self.points[**index].declaration_preview == point.declaration_preview
                && self.points[**index].realm == point.realm
                && self.points[**index].state.variable_frame == point.state.variable_frame
        }) {
            self.points[*index].original_arguments_complete_normally &=
                point.original_arguments_complete_normally;
            if point.observer_execution.observed() {
                self.points[*index].observer_execution = SourceObserverExecution::MayObserved;
            }
            if self.points[*index].head != point.head {
                self.points[*index].head = None;
            }
            if self.points[*index].original_head_input != point.original_head_input {
                self.points[*index].original_head_input = None;
            }
            if self.points[*index].original_written_words != point.original_written_words {
                self.points[*index].original_written_words = None;
            }
            join_evaluated_words(
                &mut self.points[*index].evaluated_argument_values,
                &point.evaluated_argument_values,
            );
            self.points[*index].state.join(&point.state);
            self.points[*index]
                .compiled_execution
                .join(&point.compiled_execution);
            if self.points[*index].frozen_written_words != point.frozen_written_words {
                self.points[*index].frozen_written_words = None;
            }
            if self.points[*index].frozen_written_names != point.frozen_written_names {
                self.points[*index].frozen_written_names = None;
            }
            if self.points[*index].frozen_head_object != point.frozen_head_object {
                self.points[*index].frozen_head_object = None;
            }
            self.points[*index]
                .method_prefix_arguments
                .retain(|prefix| point.method_prefix_arguments.contains(prefix));
            compiled_invocation::join_argument_words(
                &mut self.points[*index].evaluated_argument_words,
                &point.evaluated_argument_words,
            );
            self.points[*index].invalidate_derived();
        } else {
            indices.push(self.points.len());
            self.points.push(point);
        }
    }
}

pub(in super::super) fn join_evaluated_words(
    values: &mut Arc<[Option<String>]>,
    other: &[Option<String>],
) {
    if values.len() != other.len() {
        *values = std::iter::repeat_n(None, values.len().max(other.len())).collect();
        return;
    }
    if values.iter().eq(other) {
        return;
    }
    *values = values
        .iter()
        .zip(other)
        .map(|(value, other)| (value == other).then(|| value.clone()).flatten())
        .collect();
}
