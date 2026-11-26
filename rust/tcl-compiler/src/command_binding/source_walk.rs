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

//! The recursive interpretation of source text: scripts, commands, substitutions and bodies.

use super::binding_projection::PointWalkEntry;
use super::command_identity::CommandAllocationSite;
use super::compiled_invocation::NativeCompilationSnapshot;
use super::invocation_binding::SourceObserverExecution;
#[cfg(any(test, debug_assertions))]
use super::root_owner_work;
#[cfg(debug_assertions)]
use super::source_world_observer;
use super::{
    ModuleCommandBindings, PointBindingMemo, SourceBindingPoint, SourceCommandBindings,
    SourceExecutionContext, SourceOutcomes, compiled_invocation, compiled_preflight,
    rule_declaration,
};
use std::sync::{Arc, OnceLock};

mod body_flow;
mod called_bodies;
mod command_dispatch;
mod command_walk;
mod completion_receipts;
mod execution_context;
mod expression_walk;
mod lifecycle_walk;
mod logical_source_walk;
mod native_definition;
mod native_target;
mod native_transfer;
mod outcomes;
mod script_formals;
mod substitution_walk;
mod target_walk;

pub(super) use body_flow::{evaluated_script_realm, retained_script_operand};
pub(super) use called_bodies::called_body_namespace;
pub(super) use command_dispatch::registry_head;
pub(super) use command_walk::{
    join_evaluated_words, source_command_tokens_boxed, source_invocation_argument_owner,
};
pub(super) use completion_receipts::requires_reentry_guard;
pub use completion_receipts::undeclared_transitions_leave_command_world_closed;
pub(super) use execution_context::{
    SourceBodyEntryPurpose, source_argument_context_boxed, source_body_context_boxed,
    source_context_clone_boxed, source_context_conditional_boxed,
};
use execution_context::{
    source_chunk_context_boxed, source_frame_context_boxed, source_parent_frame_boxed,
    source_selected_frame_boxed,
};
pub use expression_walk::SourceImplicitMathInvocation;
pub(super) use expression_walk::{
    parse_source_expression, source_effective_words, source_operand_truth,
};
pub use lifecycle_walk::{SourceBodyPhase, SourceBodyPhaseReachability};
pub(super) use native_definition::{
    closed_procedure_definition, native_procedure_parameters, original_procedure_publication_key,
};
pub(super) use native_transfer::select_contents_write_source;
pub(super) use outcomes::{
    SourceConditionalHandlerNormal, SourceNativeNormalCompletion, boxed_source_branch,
    opaque_source_invocation, publish_source_branch,
};
pub(super) use script_formals::ScriptFormalSource;
#[cfg(test)]
pub(super) use substitution_walk::{original_variable_arena, original_variable_source};
pub(super) use target_walk::{
    PreparedSourceNativeInvocation, prepare_source_native_invocation,
    resolve_source_invocation_facts,
};

/// Keep physical variable-frame construction out of recursive script frames.
#[inline(never)]
pub(super) fn source_variables_in_frame(
    variables: &Arc<crate::var_resolve::ResolveContext>,
    frame: &crate::var_resolve::VariableExecutionFrame,
) -> Arc<crate::var_resolve::ResolveContext> {
    Arc::new(variables.in_frame(frame))
}

#[derive(Default)]
struct PreparedSourceChunk {
    pub(super) snapshot: Option<NativeCompilationSnapshot>,
    entry_error: Option<Box<ModuleCommandBindings>>,
    failed_outcome: Option<Box<SourceOutcomes>>,
}

fn retain_source_namespace_world(
    state: &mut ModuleCommandBindings,
    context: &SourceExecutionContext<'_>,
) {
    let policy = match state.baseline.native_entry.as_ref() {
        Some(entry) => entry.execution_name_policy(),
        None => state.baseline.execution_name_policy,
    };
    let variables = Arc::make_mut(&mut state.source_variables);
    variables.execution_name_policy = policy;
    variables.retain_namespace_world(
        context.namespace_identity(),
        state.namespaces.iter().cloned(),
        policy
            .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
            .map(tcl_syntax::naming::NamePolicyProtocol::recipe),
    );
}

impl SourceCommandBindings {
    pub(super) fn walk_source(
        &mut self,
        source: &str,
        base: u32,
        state: &mut ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        let channel = state
            .current_source_origin
            .as_ref()
            .map_or(tcl_lexer::SourceChannel::Document, |origin| {
                origin.source_image().channel()
            });
        let image = tcl_lexer::SourceImage::from_bytes(source.as_bytes(), channel);
        self.walk_source_image(&image, base, state, context)
    }

    pub(super) fn walk_source_image(
        &mut self,
        image: &tcl_lexer::SourceImage,
        base: u32,
        state: &mut ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        let Ok(source) = image.try_text() else {
            self.record_unrepresented_source_entry(state, context);
            return opaque_source_invocation(state);
        };
        let entry_context = source_chunk_context_boxed(context, None);
        let context = entry_context.as_ref();
        let config = context.source_purpose().config(context.config);
        // Physical Direct parsing is independent of lexical prefix recovery and
        // unentered declaration previews. Only the actual baseline entry can
        // select it; explicitly authored advice remains its separate route.
        if let Some(failed) =
            self.prepare_direct_source_parse(image, base, state, context, config, source.len())
        {
            return *failed;
        }
        let depth = context.depth;
        let mut prepared = Box::<PreparedSourceChunk>::default();
        self.prepare_source_chunk(image, base, state, context, &mut prepared);
        if let Some(failed) = prepared.failed_outcome.take() {
            self.record_incomplete_body_header(image, base, state, context);
            self.record_unrepresented_source_entry(state, context);
            self.record_runtime_coverage(
                state.current_source_origin.as_ref(),
                base,
                source.len(),
                &failed,
            );
            return *failed;
        }
        let prepared_context = source_chunk_context_boxed(context, prepared.snapshot.as_ref());
        let context = prepared_context.as_ref();
        if crate::depth_guard::MAX_SOURCE_NEST_DEPTH.exceeded(depth) {
            self.record_unrepresented_source_entry(state, context);
            state.mark_opaque_binding_mutation();
            return SourceOutcomes::invocation(
                state,
                tcl_registry::completion_route::InvocationCompletionRoute::Unknown,
            );
        }
        let segments = {
            #[cfg(any(test, debug_assertions))]
            let _timing = root_owner_work::Timing::at(root_owner_work::Query::SourceSegmentation);
            crate::segmenter::segment_commands_image_with_offset_and_config(image, base, config)
        };
        let Some(segments) = segments else {
            self.record_unrepresented_source_entry(state, context);
            return opaque_source_invocation(state);
        };
        let namespace_known = context.namespace_key.is_some()
            || context.frame.namespace_identity().is_some()
            || !matches!(
                context.frame.layout(),
                crate::var_resolve::VariableExecutionFrame::ReceiverMethod { .. }
                    | crate::var_resolve::VariableExecutionFrame::Unknown
                    | crate::var_resolve::VariableExecutionFrame::Selected {
                        namespace: None,
                        ..
                    }
            );
        let selected_frame = source_selected_frame_boxed(context, namespace_known);
        let selected_context = source_frame_context_boxed(context, &selected_frame);
        let context = selected_context.as_ref();
        let previous_frame = source_parent_frame_boxed(&state.variable_frame);
        let parent =
            (previous_frame.as_ref() != context.frame).then(|| Arc::clone(&state.source_variables));
        if parent.is_some() {
            state.source_variables =
                source_variables_in_frame(&state.source_variables, context.frame);
            state.variable_frame = context.frame.clone();
        }
        if namespace_known {
            retain_source_namespace_world(state, context);
        }
        self.retain_original_script_activation(image, base, state, context);
        let mut outcomes = self.walk_source_segments(
            image,
            base,
            &segments,
            state,
            context,
            prepared.entry_error.as_deref(),
        );
        {
            #[cfg(any(test, debug_assertions))]
            let _timing = root_owner_work::Timing::at(root_owner_work::Query::SourcePublication);
            if let Some(parent) = parent {
                outcomes.restore_frame(&parent, &previous_frame);
            }
            outcomes.publish(state);
        }
        self.record_runtime_coverage(
            state.current_source_origin.as_ref(),
            base,
            source.len(),
            &outcomes,
        );
        outcomes
    }

    #[inline(never)]
    fn prepare_direct_source_parse(
        &mut self,
        image: &tcl_lexer::SourceImage,
        base: u32,
        state: &mut ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
        config: tcl_lexer::LexerConfig,
        source_len: usize,
    ) -> Option<Box<SourceOutcomes>> {
        if self.declaration_preview_depth == 0
            && context.compilation().mode
                == tcl_registry::native_compilation::NativeCompilationMode::Direct
            && let Some(entry) = state.baseline.native_entry.as_deref()
        {
            use tcl_registry::native_compilation::NativeDirectScriptParseEntry;
            match context.compilation().direct_script_parse_entry(entry) {
                Some(NativeDirectScriptParseEntry::CommandAtTime) => {}
                Some(NativeDirectScriptParseEntry::WholeScript) => {
                    match tcl_lexer::jim_script_syntax_tokens(image, config) {
                        Ok(syntax) if syntax.missing.is_none() => {}
                        Ok(_syntax_error) => {
                            compiled_preflight::materialise_error_storage(state, context.registry);
                            let failed = SourceOutcomes::invocation(
                                state,
                                tcl_registry::completion_route::InvocationCompletionRoute::Tcl(
                                    tcl_registry::completion::CompletionCode::Error,
                                ),
                            );
                            self.record_unrepresented_source_entry(state, context);
                            self.record_runtime_coverage(
                                state.current_source_origin.as_ref(),
                                base,
                                source_len,
                                &failed,
                            );
                            return Some(Box::new(failed));
                        }
                        Err(_) => {
                            self.record_unrepresented_source_entry(state, context);
                            return Some(Box::new(opaque_source_invocation(state)));
                        }
                    }
                }
                None => {
                    self.record_unrepresented_source_entry(state, context);
                    return Some(Box::new(opaque_source_invocation(state)));
                }
            }
        }
        None
    }

    pub(super) fn walk_source_segments(
        &mut self,
        image: &tcl_lexer::SourceImage,
        base: u32,
        segments: &[crate::segmenter::SegmentedCommand],
        state: &mut ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
        entry_error: Option<&ModuleCommandBindings>,
    ) -> SourceOutcomes {
        self.conditional_source
            .enter_script(image, base, segments, state, context);
        self.retain_original_argument_syntax_commands(base, segments, state, context);
        let mut outcomes = SourceOutcomes::normal(state);
        outcomes.retain_complete_normal_evaluation();
        #[cfg(debug_assertions)]
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
            eprintln!(
                "ORIGINAL_NAMESPACE_CHILD_ENTRY base={base} entry_error={} segments={}",
                entry_error.is_some(),
                segments.len()
            );
        }
        if let Some(abrupt) = entry_error {
            outcomes.add_abrupt(
                tcl_registry::completion_route::InvocationCompletionRoute::Tcl(
                    tcl_registry::completion::CompletionCode::Error,
                ),
                abrupt,
            );
        }
        #[cfg(debug_assertions)]
        outcomes.trace_original_namespace_child("script_entry", base);
        for (index, segment) in segments.iter().enumerate() {
            if crate::analysis_control::checkpoint(
                crate::analysis_control::AnalysisCheckpoint::Command,
            ) {
                break;
            }
            let Some(mut continuing) = outcomes.normal.take() else {
                if !outcomes.abrupt.is_empty() {
                    let mut declaration_state = boxed_source_branch(state);
                    outcomes.publish(&mut declaration_state);
                    self.record_unentered_declaration_suffix(
                        image,
                        base,
                        &segments[index..],
                        &declaration_state,
                        context,
                    );
                }
                break;
            };
            self.conditional_source.retain_command_plan(
                index,
                segment,
                &continuing,
                context,
                outcomes.normal_completion.is_some(),
            );
            let prefix_complete = outcomes.normal_completion;
            let rejected_load = (base == 0
                && self.root_origin == continuing.current_source_origin
                && rule_declaration::is_rule_loader_top_level(context))
            .then(|| continuing.clone());
            if rejected_load.is_some() {
                self.rule_loader_statement = Some((
                    segment.span.start(),
                    rule_declaration::has_unknown_literal_head(segment, &continuing, context),
                ));
            }
            let command = self.walk_source_command(image, base, &mut continuing, segment, context);
            #[cfg(debug_assertions)]
            source_world_observer::observe_command(segment, &command, prefix_complete.is_some());
            #[cfg(debug_assertions)]
            command.trace_original_namespace_child("command_return", segment.span.start());
            outcomes.join(&command);
            outcomes.normal_completion = outcomes
                .normal_completion
                .filter(|_| prefix_complete.is_some());
            outcomes.complete_procedure_return = outcomes
                .complete_procedure_return
                .filter(|_| prefix_complete.is_some());
            #[cfg(debug_assertions)]
            outcomes.trace_original_namespace_child("script_join", segment.span.start());
            let rejected_head = rejected_load.is_some()
                && self.rule_loader_rejection.take() == Some(segment.span.start());
            if rejected_load.is_some() {
                self.rule_loader_statement = None;
            }
            if let Some(before) = rejected_load
                && (rejected_head
                    || (outcomes.normal.is_none()
                        && rule_declaration::is_rule_loader_declaration_head(
                            segment, &before, context,
                        )))
            {
                // The rule loader rejects such a statement, or a malformed
                // declaration, on its own; the declarations after it are still
                // read from the unchanged world.
                outcomes.normal = Some(before);
            }
            let Some(continuing) = &outcomes.normal else {
                continue;
            };
            self.record_continuation_source_point(segment, continuing, context);
        }
        self.conditional_source.leave_script();
        outcomes
    }

    /// Resolve the chunk entry outside recursive script frames. Failure
    /// carriers and compilation snapshots remain on the heap during descent.
    #[inline(never)]
    fn prepare_source_chunk(
        &mut self,
        image: &tcl_lexer::SourceImage,
        base: u32,
        state: &mut ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
        prepared: &mut PreparedSourceChunk,
    ) {
        // A source-point interpretation has no physical compiler ingress.
        // Its positive lexer/body grammar still diagnoses source syntax; a
        // missing physical provider cannot become a guest Error alternative.
        if state.baseline.source_point_entry.is_some() {
            return;
        }
        let context = *context;
        prepared.snapshot = (context.compilation_snapshot.is_none()
            && context.compilation().mode
                != tcl_registry::native_compilation::NativeCompilationMode::Direct)
            .then(|| state.native_compilation_snapshot_in_realm(context.realm()));
        let Some(snapshot) = &mut prepared.snapshot else {
            return;
        };
        #[cfg(any(test, debug_assertions))]
        let preflight_timing = root_owner_work::Timing::at(root_owner_work::Query::Preflight);
        let assessment = compiled_preflight::preflight_image(image, base, state, context, snapshot);
        #[cfg(debug_assertions)]
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
            eprintln!(
                "ORIGINAL_NAMESPACE_CHILD_PREFLIGHT base={base} bytes={} failure={} possible_error={} native_entry_unavailable={} compiler_sites={} child_regions={} preview_depth={}",
                image.len(),
                assessment.failure.is_some(),
                assessment.possible_error,
                assessment.native_entry_unavailable,
                assessment.compiler_invocations.len(),
                assessment.compiled_children.len(),
                self.declaration_preview_depth,
            );
        }
        #[cfg(any(test, debug_assertions))]
        drop(preflight_timing);
        #[cfg(any(test, debug_assertions))]
        let _timing = root_owner_work::Timing::at(root_owner_work::Query::PreflightPublication);
        // A declaration template interprets possible compilation locally. It
        // never publishes an actual compiler visit or chunk-entry obligation.
        if self.declaration_preview_depth == 0 {
            self.record_compilation_source(image, base, state);
            self.retain_compiler_invocations(&assessment.compiler_invocations);
            for (site, children) in &assessment.compiled_children {
                let retained = self.compiled_children.entry(site.clone()).or_default();
                for child in children {
                    if !retained.contains(child) {
                        retained.push(child.clone());
                    }
                }
            }
            for (site, children) in &assessment.expression_syntax_children {
                let retained = self
                    .expression_syntax_children
                    .entry(site.clone())
                    .or_default();
                for child in children {
                    if !retained.contains(child) {
                        retained.push(child.clone());
                    }
                }
            }
            self.record_compilation_boundary(base, state, assessment.failure.as_ref());
            if (assessment.possible_error || assessment.native_entry_unavailable)
                && let Some(origin) = &state.current_source_origin
            {
                self.compilation_provider_required
                    .insert(CommandAllocationSite {
                        source: Arc::clone(origin),
                        offset: base,
                    });
            }
        }
        if let Some(failure) = assessment.failure {
            if self.declaration_preview_depth == 0 && !self.compilation_failures.contains(&failure)
            {
                self.compilation_failures.push(failure);
            }
            compiled_preflight::materialise_error_storage(state, context.registry);
            prepared.failed_outcome = Some(Box::new(SourceOutcomes::invocation(
                state,
                tcl_registry::completion_route::InvocationCompletionRoute::Tcl(
                    tcl_registry::completion::CompletionCode::Error,
                ),
            )));
        } else if assessment.possible_error && self.declaration_preview_depth == 0 {
            // An unentered declaration preview supplies conditional source
            // advice. Missing compiler evidence is an execution obligation,
            // not a guest error state owned by that declaration.
            let mut abrupt = boxed_source_branch(state);
            compiled_preflight::materialise_error_storage(&mut abrupt, context.registry);
            prepared.entry_error = Some(abrupt);
        }
    }

    /// Materialise a continuation snapshot outside the recursive script frame.
    #[inline(never)]
    fn record_continuation_source_point(
        &mut self,
        segment: &crate::segmenter::SegmentedCommand,
        state: &ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
    ) {
        #[cfg(any(test, debug_assertions))]
        let _timing = root_owner_work::Timing::at(root_owner_work::Query::ContinuationCapture);
        let context = *context;
        if self.root_origin != state.current_source_origin {
            return;
        }
        self.invalidate_unpositioned_projections();
        self.points.push(SourceBindingPoint {
            declaration_preview: self.declaration_preview_depth != 0,
            walk_entry: PointWalkEntry::at_depth(self.unreached_walk_depth),
            original_arguments_complete_normally: false,
            observer_execution: SourceObserverExecution::Unobserved,
            realm: context.realm(),
            compilation: context.compilation(),
            compiled_execution: compiled_invocation::CompiledInvocationSelection::default(),
            head: None,
            original_head_input: None,
            original_written_words: None,
            evaluated_argument_values: Arc::default(),
            evaluated_argument_words: Arc::default(),
            frozen_written_words: None,
            frozen_written_names: None,
            frozen_head_object: None,
            method_prefix_arguments: Vec::new(),
            offset: segment.span.end().saturating_add(1),
            end: u32::MAX,
            namespace: context.namespace.to_owned(),
            namespace_key: context.namespace_identity(),
            state: state.clone(),
            lookup_snapshot: OnceLock::new(),
            original_binding: PointBindingMemo::default(),
            dispatch: false,
            continuation_of: state.current_source_origin.as_ref().map(|source| {
                CommandAllocationSite {
                    source: Arc::clone(source),
                    offset: segment.span.start(),
                }
            }),
        });
    }
}
