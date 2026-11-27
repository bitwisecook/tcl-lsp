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

//! Dispatch of a selected command to receiver, named, unknown, resolved and document targets.

use super::super::binding_projection::{source_binding, source_binding_from_original_input_boxed};
use super::super::named_invocation::SourceNamedInvocationProof;
use super::super::{
    DeferredSourceBody, ModuleCommandBindings, SourceCommandBindings, SourceCommandInput,
    SourceCommandTarget, SourceExecutionContext, SourceInvocationBinding, SourceOutcomes,
    compiled_invocation, logical_definition, named_arguments, original_name_value, receiver_self,
};
use super::execution_context::source_selected_context_boxed;
use super::outcomes::{boxed_source_branch, boxed_source_outcomes, opaque_source_invocation};
use std::sync::Arc;
use tcl_registry::CommandRegistry;

/// Clone a binding branch outside recursive frames, retaining it on the heap.
#[inline(never)]
/// The registry entry a literal head names.
pub(in super::super) fn registry_head(
    registry: &CommandRegistry,
    effective: &[crate::registry_invocation::EffectiveInvocationWord],
) -> Option<&'static tcl_registry::CommandSpec> {
    let crate::registry_invocation::EffectiveInvocationWord::Literal(head) = effective.first()?
    else {
        return None;
    };
    registry.get(head)
}

#[cfg(debug_assertions)]
fn trace_original_generic_head(site: u32, state: &ModuleCommandBindings, has_head: bool) {
    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_VARIABLE_TRANSFER").is_some() {
        eprintln!(
            "ORIGINAL_GENERIC_HEAD site={} input={} dynamic={} opaque={}",
            site, has_head, state.source_variables.dynamic_bindings, state.opaque_binding_mutation
        );
    }
}

#[cfg(debug_assertions)]
fn trace_original_generic_binding(site: u32, binding: Option<&SourceInvocationBinding>) {
    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_VARIABLE_TRANSFER").is_some() {
        eprintln!(
            "ORIGINAL_GENERIC_BINDING site={} available={} unknown={} absent={} targets={}",
            site,
            binding.is_some(),
            binding.as_ref().is_some_and(|binding| binding.unknown),
            binding
                .as_ref()
                .is_some_and(|binding| binding.may_be_absent),
            binding.as_ref().map_or(0, |binding| binding.targets.len())
        );
    }
}

#[cfg(debug_assertions)]
fn trace_original_generic_result(site: u32, outcomes: &SourceOutcomes) {
    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_VARIABLE_TRANSFER").is_some() {
        eprintln!(
            "ORIGINAL_GENERIC_RESULT site={} normal={} abrupt={} normal_dynamic={}",
            site,
            outcomes.normal.is_some(),
            outcomes.abrupt.len(),
            outcomes
                .normal
                .as_ref()
                .is_some_and(|normal| normal.source_variables.dynamic_bindings)
        );
    }
}

#[inline(never)]
pub(super) fn retain_source_native_result(
    outcomes: &mut SourceOutcomes,
    incoming: &ModuleCommandBindings,
    target: &SourceCommandTarget,
    arguments: &[crate::registry_invocation::EffectiveInvocationWord],
    context: &SourceExecutionContext<'_>,
) {
    outcomes.retain_native_result(incoming, target, arguments, *context);
}

impl SourceCommandBindings {
    #[inline(never)]
    pub(super) fn prepare_selected_source_command(
        &mut self,
        input: SourceCommandInput<'_>,
        state: &mut ModuleCommandBindings,
        compiled: &compiled_invocation::CompiledInvocationSelection,
        context: &SourceExecutionContext<'_>,
    ) -> Option<Box<SourceOutcomes>> {
        let SourceCommandInput {
            segment,
            words,
            effective,
            ..
        } = input;
        self.retain_rule_loader_events(segment, words, effective, state, *context);
        if let Some(outcomes) =
            logical_definition::transfer_selected(words, segment.span.start(), state, *context)
        {
            return Some(Box::new(outcomes));
        }
        if !compiled.compile_error {
            return self.walk_selected_receiver_command(segment, words, effective, state, context);
        }
        None
    }

    /// Receiver-only temporaries do not remain live through native body descent.
    #[inline(never)]
    fn walk_selected_receiver_command(
        &mut self,
        segment: &tcl_source::segment::SegmentedCommand,
        words: &[tcl_source::words::WordExpr],
        effective: &[crate::registry_invocation::EffectiveInvocationWord],
        state: &mut ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
    ) -> Option<Box<SourceOutcomes>> {
        let context = *context;
        if let Some(result) = Self::retained_receiver_result(effective, state, context) {
            return Some(Box::new(result));
        }
        if let Some(result) = Self::walk_receiver_builtin_links(effective, state, context) {
            return Some(Box::new(result));
        }
        self.walk_retained_receiver_method(segment.span.start(), words, effective, state, context)
            .map(Box::new)
    }

    /// Live lookup owns its preparation only on the live dispatch branch.
    #[inline(never)]
    pub(super) fn join_live_source_command(
        &mut self,
        input: SourceCommandInput<'_>,
        incoming: &ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
        outcomes: &mut SourceOutcomes,
    ) {
        let mut live = boxed_source_branch(incoming);
        let selected_context = source_selected_context_boxed(
            context,
            &tcl_registry::native_compilation::NativeCompilationSelection::Generic,
        );
        let mut live_outcomes = boxed_source_outcomes();
        self.walk_resolved_source_command(input, &mut live, &selected_context, &mut live_outcomes);
        outcomes.join(&live_outcomes);
    }

    /// Named and residual branches follow inline execution without retaining
    /// their construction slots across that execution.
    #[inline(never)]
    pub(super) fn join_other_selected_source_commands(
        &mut self,
        input: SourceCommandInput<'_>,
        incoming: &ModuleCommandBindings,
        compiled: &compiled_invocation::CompiledInvocationSelection,
        context: &SourceExecutionContext<'_>,
        outcomes: &mut SourceOutcomes,
    ) {
        let SourceCommandInput {
            segment,
            words,
            effective,
            ..
        } = input;
        for proof in &compiled.named {
            outcomes
                .join(&self.walk_named_source_command(
                    segment, words, effective, incoming, proof, context,
                ));
        }
        if compiled.unknown {
            outcomes.join(&self.walk_unknown_compiler_handler(
                segment, words, effective, incoming, compiled, context,
            ));
        }
        if compiled.compile_error {
            let mut residual = boxed_source_branch(incoming);
            residual.mark_opaque_binding_mutation();
            outcomes.join(&SourceOutcomes::invocation(
                &residual,
                tcl_registry::completion_route::InvocationCompletionRoute::Unknown,
            ));
        }
    }

    fn walk_unknown_compiler_handler(
        &mut self,
        segment: &tcl_source::segment::SegmentedCommand,
        words: &[tcl_source::words::WordExpr],
        effective: &[crate::registry_invocation::EffectiveInvocationWord],
        incoming: &ModuleCommandBindings,
        compiled: &compiled_invocation::CompiledInvocationSelection,
        context: &SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        use tcl_registry::native_compilation::NativeCompilationSelection;
        let mut outcomes = SourceOutcomes::default();
        let head = original_name_value::original_command_head_input(
            words,
            segment.span.start(),
            incoming,
            *context,
        );
        let loader_binding = (head.is_none()
            && super::super::rule_declaration::is_rule_loader_top_level(context))
        .then(|| source_binding(incoming, segment.name(), &context.namespace_identity()));
        if let Some(target) = compiled
            .stable_handler_after_original_arguments(
                head.as_ref(),
                incoming,
                &context.namespace_identity(),
            )
            .or_else(|| {
                loader_binding
                    .as_ref()
                    .and_then(SourceInvocationBinding::proved_target)
            })
            && let Some(declaration) = self
                .walk_rule_loader_procedure(segment, words, effective, incoming, target, *context)
        {
            return declaration;
        }
        if let Some((target, transfer)) = compiled
            .stable_handler_after_original_arguments(
                head.as_ref(),
                incoming,
                &context.namespace_identity(),
            )
            .and_then(|target| {
                compiled_invocation::successful_handler_transfer(
                    target,
                    effective,
                    incoming,
                    context.registry,
                    context.realm(),
                )
                .map(|transfer| (target, transfer))
            })
        {
            let mut stable = boxed_source_branch(incoming);
            let conditional =
                if transfer == compiled_invocation::HandlerSourceTransfer::PossibleBodies {
                    context.conditional()
                } else {
                    *context
                };
            let selected_context =
                source_selected_context_boxed(&conditional, &NativeCompilationSelection::Unknown);
            let mut result = self.walk_source_target(
                segment,
                words,
                effective,
                &mut stable,
                target,
                &selected_context,
            );
            retain_source_native_result(&mut result, incoming, target, effective, context);
            if transfer == compiled_invocation::HandlerSourceTransfer::PossibleBodies {
                // The inventory is conditional on body entry. Retain the
                // unchanged normal branch so these effects remain May.
                result.join(&SourceOutcomes::normal(incoming));
            }
            outcomes.join(&result);
            outcomes.add_abrupt(
                tcl_registry::completion_route::InvocationCompletionRoute::Tcl(
                    tcl_registry::completion::CompletionCode::Error,
                ),
                incoming,
            );
        } else {
            let mut residual = boxed_source_branch(incoming);
            // A validated loader declaration is inventory, not a command-world
            // mutation; only an unresolved statement stays opaque.
            if !(head.is_none()
                && super::super::rule_declaration::is_rule_loader_top_level(context)
                && super::super::rule_declaration::is_rule_loader_declaration_head(
                    segment, incoming, context,
                ))
            {
                residual.mark_opaque_binding_mutation();
            }
            outcomes.join(&SourceOutcomes::invocation(
                &residual,
                tcl_registry::completion_route::InvocationCompletionRoute::Unknown,
            ));
        }
        outcomes
    }

    fn walk_named_source_command(
        &mut self,
        segment: &tcl_source::segment::SegmentedCommand,
        words: &[tcl_source::words::WordExpr],
        effective: &[crate::registry_invocation::EffectiveInvocationWord],
        incoming: &ModuleCommandBindings,
        selected: &SourceNamedInvocationProof,
        context: &SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        use tcl_registry::completion_route::InvocationCompletionRoute as Route;
        let proof = selected.with_late_lookup(incoming);
        let Ok(projected) = named_arguments::ProjectedNamedArguments::capture(
            words,
            effective,
            incoming,
            context,
            proof.arguments_from,
        ) else {
            let mut residual = boxed_source_branch(incoming);
            residual.mark_opaque_binding_mutation();
            return SourceOutcomes::invocation(&residual, Route::Unknown);
        };
        let mut outcomes = SourceOutcomes::default();
        if proof.unknown {
            let mut residual = boxed_source_branch(incoming);
            residual.mark_opaque_binding_mutation();
            outcomes.join(&SourceOutcomes::invocation(&residual, Route::Unknown));
        }
        if proof.may_be_absent {
            outcomes.add_abrupt(Route::UnknownAbrupt, incoming);
        }
        for target in &proof.targets {
            let mut branch = boxed_source_branch(incoming);
            let selected_context = projected.context(context);
            let conditional = if proof.unknown || proof.may_be_absent || proof.targets.len() != 1 {
                selected_context.conditional()
            } else {
                *selected_context
            };
            let context = &conditional;
            // A registry widget's methods are declared like a registry
            // command's, so its call walks through the registry facts.
            let stock_creation =
                branch.stock_metaclass_creation_target(target, &projected.effective, *context);
            let mut result = if let Some(stock) = &stock_creation {
                let mut result = self.walk_source_target(
                    segment,
                    &projected.words,
                    &projected.effective,
                    &mut branch,
                    stock,
                    context,
                );
                if let Some(normal) = result.normal.as_deref_mut()
                    && normal
                        .mark_custom_metaclass_creation(
                            stock,
                            target,
                            &projected.effective,
                            *context,
                        )
                        .is_none()
                {
                    normal.mark_opaque_binding_mutation();
                }
                result
            } else if target.registry_backed
                || matches!(target.kind, super::super::BindingKind::Instance { .. })
            {
                self.walk_source_target(
                    segment,
                    &projected.words,
                    &projected.effective,
                    &mut branch,
                    target,
                    context,
                )
            } else {
                self.walk_document_target(
                    segment.span.start(),
                    Some(&projected.words),
                    &projected.effective,
                    &mut branch,
                    target,
                    *context,
                )
            };
            retain_source_native_result(
                &mut result,
                incoming,
                target,
                &projected.effective,
                context,
            );
            outcomes.join(&result);
        }
        outcomes
    }

    /// The outcome of a command whose binding is unknown. Kept out of line:
    /// the caller's frame is on the recursive source walk.
    #[inline(never)]
    fn join_unknown_source_binding(
        &self,
        incoming: &ModuleCommandBindings,
        effective: &[crate::registry_invocation::EffectiveInvocationWord],
        context: &SourceExecutionContext<'_>,
        outcomes: &mut SourceOutcomes,
    ) {
        let mut residual = Box::new(incoming.clone());
        if let Some(frame) = incoming.declared_command_frame_effect(context.registry, effective) {
            // A declared command (document stub, sidecar or workspace
            // procedure) is trusted to leave the command world intact; its
            // frame effect bounds what it does to variables.
            if frame != tcl_registry::model::DeclaredFrameEffect::Stated(None) {
                Arc::make_mut(&mut residual.source_variables).widen_keeping_observers();
            }
        } else if let Some(pure) = incoming.receiver_context_head(
            context.registry,
            effective,
            self.declaration_preview_depth != 0,
        ) {
            // A TclOO helper this walk's namespace cannot reach. It reads
            // the call context or dispatches to the receiver's own methods,
            // whose bodies are walked in their own right.
            if !pure {
                Arc::make_mut(&mut residual.source_variables).widen_keeping_observers();
            }
        } else if let Some(crate::registry_invocation::EffectiveInvocationWord::Literal(head)) =
            effective.first()
            && incoming.absent_head_reaches_only_stock_autoload(head, &context.namespace_identity())
        {
            // The stock handler's autoload defines no command the walk cannot
            // see, so the command table stays as it was (#2427); the call's
            // result stays unknown.
        } else {
            // The default `unknown` handler loads at global level, never
            // into a receiver's private namespace; a command the registry
            // knows is reached instead, with its own effects.
            let receiver_opacity = residual.receiver_frame_opacity;
            residual.mark_opaque_binding_mutation();
            if registry_head(context.registry, effective).is_none() {
                residual.receiver_frame_opacity = receiver_opacity;
            }
            residual.loader_handler_unknown = true;
        }
        outcomes.join(&SourceOutcomes::invocation(
            &residual,
            tcl_registry::completion_route::InvocationCompletionRoute::Unknown,
        ));
    }

    fn walk_resolved_source_command(
        &mut self,
        input: SourceCommandInput<'_>,
        state: &mut ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
        outcomes: &mut SourceOutcomes,
    ) {
        let SourceCommandInput {
            segment,
            words,
            effective,
            ..
        } = input;
        let Some(binding) =
            Self::prepare_original_source_binding(words, segment.span.start(), state, context)
        else {
            state.mark_opaque_binding_mutation();
            outcomes.join(&SourceOutcomes::invocation(
                state,
                tcl_registry::completion_route::InvocationCompletionRoute::Unknown,
            ));
            return;
        };
        let incoming = boxed_source_branch(state);
        if binding.unknown {
            self.join_unknown_source_binding(&incoming, effective, context, outcomes);
        }
        if binding.may_be_absent {
            outcomes.add_abrupt(
                tcl_registry::completion_route::InvocationCompletionRoute::UnknownAbrupt,
                &incoming,
            );
        }
        let selected_context =
            if binding.unknown || binding.may_be_absent || binding.targets.len() != 1 {
                context.conditional()
            } else {
                *context
            };
        let context = &selected_context;
        for target in &binding.targets {
            let mut alternative = boxed_source_branch(&incoming);
            let stock_creation =
                alternative.stock_metaclass_creation_target(target, effective, *context);
            // A registry widget's methods are declared like a registry
            // command's, so its call walks through the registry facts.
            let mut result = if let Some(stock) = &stock_creation {
                let mut result = self.walk_source_target(
                    segment,
                    words,
                    effective,
                    &mut alternative,
                    stock,
                    context,
                );
                if let Some(normal) = result.normal.as_deref_mut()
                    && normal
                        .mark_custom_metaclass_creation(stock, target, effective, *context)
                        .is_none()
                {
                    normal.mark_opaque_binding_mutation();
                }
                result
            } else if target.registry_backed
                || matches!(target.kind, super::super::BindingKind::Instance { .. })
            {
                self.walk_source_target(
                    segment,
                    words,
                    effective,
                    &mut alternative,
                    target,
                    context,
                )
            } else {
                self.walk_document_target(
                    segment.span.start(),
                    Some(words),
                    effective,
                    &mut alternative,
                    target,
                    *context,
                )
            };
            retain_source_native_result(&mut result, &incoming, target, effective, context);
            #[cfg(debug_assertions)]
            trace_original_generic_result(segment.span.start(), &result);
            outcomes.join(&result);
        }
        outcomes.publish(state);
    }

    #[inline(never)]
    pub(in super::super) fn prepare_original_source_binding(
        words: &[tcl_source::words::WordExpr],
        offset: u32,
        state: &ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
    ) -> Option<Box<SourceInvocationBinding>> {
        let namespace = context.namespace_identity();
        let head = original_name_value::original_command_head_input(words, offset, state, *context);
        #[cfg(debug_assertions)]
        trace_original_generic_head(offset, state, head.is_some());
        let head = head?;
        let binding = source_binding_from_original_input_boxed(state, &head, &namespace);
        #[cfg(debug_assertions)]
        trace_original_generic_binding(offset, binding.as_deref());
        binding
    }

    pub(in super::super) fn walk_document_target(
        &mut self,
        site: u32,
        original_words: Option<&[tcl_source::words::WordExpr]>,
        effective: &[crate::registry_invocation::EffectiveInvocationWord],
        state: &mut ModuleCommandBindings,
        target: &SourceCommandTarget,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        use tcl_registry::completion_route::InvocationCompletionRoute as Route;
        if self.compilation_scope == tcl_runtime_api::SourceCompilationScope::EnteredSource {
            if let Some(body) = self.deferred.get(&target.implementation_id()).cloned() {
                self.record_unrepresented_called_body(site, state, target, &body);
            }
            return opaque_source_invocation(state);
        }
        if let Some(outcomes) =
            self.walk_ensemble_dispatch(site, original_words, effective, state, target, context)
        {
            return outcomes;
        }
        if state.installed_definition_dispatcher(target).is_some() {
            // No constructor or typemethod execution contract is donated by
            // the definition receipt. Its nominal result query is separate.
            return opaque_source_invocation(state);
        }
        if let Some(result) = original_words.and_then(|words| {
            self.walk_original_constructor(site, words, effective, state, target, context)
        }) {
            return result;
        }
        if let Some(outcomes) = original_words
            .and_then(|words| Self::walk_jim_stock_method(site, words, state, target, context))
        {
            return outcomes;
        }
        let body = self.deferred.get(&target.implementation_id()).cloned();
        let Some(body) = body else {
            state.mark_opaque_binding_mutation();
            return SourceOutcomes::invocation(state, Route::Unknown);
        };
        if self.declaration_preview_depth == 0 {
            self.called_implementations.insert(body.implementation_id());
        }
        let implementation = body.implementation_id();
        if !self.active_calls.insert(implementation.clone()) {
            self.record_unrepresented_called_body(site, state, target, &body);
            if self.recursive_assumption_refused.contains(&implementation) {
                state.mark_opaque_binding_mutation();
            } else {
                // Fixpoint assumption: the recursive call leaves the shared
                // command world as the enclosing activation finds it. Its
                // variable effects stay unknown. Verified when the enclosing
                // activation ends.
                Arc::make_mut(&mut state.source_variables).widen();
                self.recursive_assumptions.insert(implementation);
            }
            return SourceOutcomes::invocation(state, Route::Unknown);
        }
        let entry = boxed_source_branch(state);
        let selected = receiver_self::CalledBodyTarget {
            target,
            receiver: None,
        };
        let formals = self.script_formal_sources.len();
        if let (Some(words), Some(source), Some(grammar)) = (
            original_words,
            state.current_source_origin.as_ref(),
            state
                .source_variables
                .invocation_dialect
                .and_then(tcl_registry::InvocationDialect::parameter_grammar),
        ) {
            let call = super::super::command_identity::CommandAllocationSite {
                source: Arc::clone(source),
                offset: site,
            };
            self.script_formal_sources
                .extend(super::script_formals::script_formal_sources(
                    &call,
                    words,
                    &body,
                    target.prepended.len(),
                    grammar,
                    &state.variable_frame,
                ));
        }
        let mut result = self.walk_called_body(site, effective, state, selected, &body, context);
        self.script_formal_sources.truncate(formals);
        if self.recursive_assumptions.remove(&implementation)
            && (!result.preserves_command_world(&entry)
                || !self.recursive_body_preserves_command_world(
                    site, effective, &entry, selected, &body, context,
                ))
        {
            // The body changes the command world, so the assumption is not a
            // fixpoint: walk this activation again with a widening back-edge.
            // Its records join those of the first walk.
            *state = *entry;
            self.recursive_assumption_refused
                .insert(implementation.clone());
            result = self.walk_called_body(site, effective, state, selected, &body, context);
            self.recursive_assumption_refused.remove(&implementation);
        }
        self.active_calls.remove(&implementation);
        result
    }

    /// The recursive back-edge assumption must hold for every argument
    /// vector a recursive call can pass, not only this activation's: walk
    /// the body once more with every formal, defaulted ones included, bound
    /// to an unknown word.
    fn recursive_body_preserves_command_world(
        &mut self,
        site: u32,
        effective: &[crate::registry_invocation::EffectiveInvocationWord],
        entry: &ModuleCommandBindings,
        selected: receiver_self::CalledBodyTarget<'_>,
        body: &DeferredSourceBody,
        context: SourceExecutionContext<'_>,
    ) -> bool {
        let Some(head) = effective.first() else {
            return false;
        };
        let generic = std::iter::once(head.clone())
            .chain(
                body.parameters
                    .iter()
                    .map(|_| crate::registry_invocation::EffectiveInvocationWord::Dynamic),
            )
            .collect::<Vec<_>>();
        let mut scratch = boxed_source_branch(entry);
        let outcomes = self.walk_called_body(site, &generic, &mut scratch, selected, body, context);
        self.recursive_assumptions.remove(&body.implementation_id());
        outcomes.preserves_command_world(entry)
    }
}
