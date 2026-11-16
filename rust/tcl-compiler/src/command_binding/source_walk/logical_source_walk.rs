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

//! Quiet Logical source continuations through a called procedure body or a namespace body.

use super::super::binding_projection::source_binding_projection;
use super::super::module_bindings::apply_namespace_transition;
use super::super::source_origin::source_activation_name;
use super::super::{
    BindingKind, ModuleCommandBindings, SourceCommandBindings, SourceCommandTarget,
    SourceExecutionContext, SourceOutcomes, SourceScriptOperands, compiled_invocation,
    declaration_preview, logical_definition, source_analysis_entry,
};
use super::command_walk::source_command_tokens_boxed;
use super::execution_context::{source_body_context_boxed, source_namespace_context_boxed};
use super::target_walk::prepare_source_native_invocation;
use std::sync::Arc;
use tcl_registry::InvocationFacts;

impl SourceCommandBindings {
    fn logical_selected_source_facts(
        values: &[String],
        target: &SourceCommandTarget,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
        delegated_script: bool,
    ) -> Option<(
        InvocationFacts,
        tcl_registry::world_effect::EffectFootprint,
        tcl_registry::StateTransitions,
        tcl_registry::registry::InvocationCompletion,
    )> {
        let input = logical_definition::logical_entry(state, context)?;
        let actual = input.context_registry();
        let arguments = values
            .iter()
            .skip(1)
            .map(|value| tcl_registry::InvocationWord::Literal(value))
            .collect::<Vec<_>>();
        let resolution =
            tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
                actual.commands(),
                Some(actual.context()),
                tcl_registry::InvocationWords::structured(
                    tcl_registry::InvocationWord::Literal(&target.command),
                    &arguments,
                )
                .with_dialect(source_analysis_entry::source_input_dialect(input)),
                context.realm(),
            );
        let schema = resolution.resolved()?;
        let dialect = source_analysis_entry::source_input_dialect(input);
        let (effects, transitions) = if delegated_script {
            schema.authored_source_delegated_script_effects(dialect)?
        } else {
            schema.authored_source_effects_and_state_transitions(dialect)
        };
        Some((
            schema.facts(),
            effects,
            transitions,
            schema.authored_source_invocation_completion(),
        ))
    }

    /// Retain a quiet conditional source continuation for a current Logical
    /// procedure with one statically selected return. This does not establish
    /// an actual activation or a Native completion.
    fn logical_called_source_body(
        &self,
        segment: &crate::segmenter::SegmentedCommand,
        words: &[crate::ir::WordExpr],
        effective: &[crate::registry_invocation::EffectiveInvocationWord],
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> Option<(declaration_preview::DeclaredProcedureBody, String)> {
        let original = logical_definition::original_static_invocation_words(
            words,
            segment.span.start(),
            state,
            context,
        )?;
        let values = original
            .iter()
            .map(logical_definition::static_value)
            .collect::<Option<Vec<_>>>()?;
        if effective.len() != values.len()
            || effective
                .iter()
                .zip(&values)
                .any(|(word, value)| word.as_registry_word().literal() != Some(value.as_str()))
        {
            return None;
        }
        let selected =
            source_binding_projection(state, values.first()?, &context.namespace_identity());
        if selected.unknown || selected.may_be_absent {
            return None;
        }
        let [target] = selected.targets.as_slice() else {
            return None;
        };
        let body = self.logical_called_procedure_body(target, state, context.registry)?;
        body.source.try_text().ok()?;
        let namespace = body.namespace.display()?;
        let actual_count = effective.len().checked_sub(1)?;
        let tcl = tcl_syntax::formal_params::bind_formal_arguments(
            &body.parameters,
            actual_count,
            tcl_dialect::ParameterGrammar::Tcl,
        )
        .ok()?;
        let jim = tcl_syntax::formal_params::bind_formal_arguments(
            &body.parameters,
            actual_count,
            tcl_dialect::ParameterGrammar::Jim,
        )
        .ok()?;
        if tcl != jim
            || tcl.iter().any(|binding| {
                matches!(
                    binding,
                    tcl_syntax::formal_params::FormalArgumentBinding::CallerLink { .. }
                )
            })
        {
            return None;
        }
        Some((body, namespace))
    }

    fn logical_called_return_is_safe(
        facts: &InvocationFacts,
        effects: &tcl_registry::world_effect::EffectFootprint,
        transitions: &tcl_registry::StateTransitions,
        completion: &tcl_registry::registry::InvocationCompletion,
    ) -> bool {
        use tcl_registry::{
            hooks::LoweringHookId,
            world_effect::{
                CallbackKinds, EffectAccessMode, InterpreterScope, Reentrancy, WorldStateDomain,
            },
        };
        facts.operation
            == tcl_registry::SemanticOperationId::StructuredLowering(LoweringHookId::Return)
            && facts.arity_accepts_frozen_arguments() == Some(true)
            && facts.frame_effect.is_none()
            && facts.body_execution.is_none()
            && effects.callback().kinds == CallbackKinds::NONE
            && effects.callback().reentrancy == Reentrancy::Never
            && transitions.facts().is_empty()
            && !effects.accesses().iter().any(|access| {
                access.interpreter != InterpreterScope::Current
                    || access.mode == EffectAccessMode::Clobber
                    || (access.mode != EffectAccessMode::Read
                        && !matches!(
                            access.domain,
                            WorldStateDomain::InterpreterResult | WorldStateDomain::CompletionState
                        ))
            })
            && matches!(
                completion,
                tcl_registry::registry::InvocationCompletion::ReturnsResult(_)
            )
    }

    #[inline(never)]
    pub(super) fn walk_logical_called_source(
        &mut self,
        segment: &crate::segmenter::SegmentedCommand,
        words: &[crate::ir::WordExpr],
        effective: &[crate::registry_invocation::EffectiveInvocationWord],
        compiled: &compiled_invocation::CompiledInvocationSelection,
        state: &mut ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
    ) -> Option<Box<SourceOutcomes>> {
        if compiled.compile_error || compiled.unknown {
            return None;
        }
        let (body, namespace) =
            self.logical_called_source_body(segment, words, effective, state, *context)?;
        let mut branch = state.clone();
        let variables = branch
            .source_variables
            .enter_called_frame(&body.frame)
            .with_namespace_identity(body.namespace.clone());
        branch.source_variables = Arc::new(variables);
        branch.variable_frame = body.frame.clone();
        branch.current_source_origin = Some(Arc::clone(&body.source.origin));
        let segments = crate::segmenter::segment_commands_image_with_offset_and_config(
            &body.source.text,
            body.source.base(),
            context.config,
        )?;
        let [command] = segments.as_slice() else {
            return None;
        };
        if command.is_partial {
            return None;
        }
        let tokens = source_command_tokens_boxed(
            &body.source.text,
            body.source.base(),
            context.config,
            command,
        );
        let mut body_context = *source_body_context_boxed(context, &namespace, &body.frame);
        body_context.namespace_key = Some(&body.namespace);
        body_context.invocation_offset = command.span.start();
        let original = logical_definition::original_static_invocation_words(
            tokens.words(),
            command.span.start(),
            &branch,
            body_context,
        )?;
        let values = original
            .iter()
            .map(logical_definition::static_value)
            .collect::<Option<Vec<_>>>()?;
        let selected = source_binding_projection(&branch, values.first()?, &body.namespace);
        if selected.unknown || selected.may_be_absent {
            return None;
        }
        let [return_target] = selected.targets.as_slice() else {
            return None;
        };
        if !return_target.registry_backed
            || return_target.kind != BindingKind::Builtin
            || !return_target.prepended.is_empty()
            || return_target.implementation_generation != 0
            || return_target.runtime_implementation_generation.is_some()
            || !branch.retained_target_is_current(return_target)
        {
            return None;
        }
        let (facts, effects, transitions, completion) = Self::logical_selected_source_facts(
            &values,
            return_target,
            &branch,
            body_context,
            false,
        )?;
        if !Self::logical_called_return_is_safe(&facts, &effects, &transitions, &completion) {
            return None;
        }
        Some(Box::new(SourceOutcomes::normal(state)))
    }

    /// Descend a single authored namespace body under the Logical source owner.
    /// This supplies a source continuation only, never a Native handler receipt.
    fn logical_namespace_source_target(
        segment: &crate::segmenter::SegmentedCommand,
        words: &[crate::ir::WordExpr],
        effective: &[crate::registry_invocation::EffectiveInvocationWord],
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> Option<(Vec<String>, SourceCommandTarget)> {
        let original = logical_definition::original_static_invocation_words(
            words,
            segment.span.start(),
            state,
            context,
        )?;
        let values = original
            .iter()
            .map(logical_definition::static_value)
            .collect::<Option<Vec<_>>>()?;
        let head = values.first()?;
        let current = context.namespace_identity();
        let selected = source_binding_projection(state, head, &current);
        if selected.unknown || selected.may_be_absent {
            return None;
        }
        let [target] = selected.targets.as_slice() else {
            return None;
        };
        if !target.registry_backed
            || target.kind != BindingKind::Builtin
            || !target.prepended.is_empty()
            || target.implementation_generation != 0
            || target.runtime_implementation_generation.is_some()
            || !state.retained_target_is_current(target)
        {
            return None;
        }
        if effective.len() != values.len()
            || effective
                .iter()
                .zip(&values)
                .any(|(word, value)| word.as_registry_word().literal() != Some(value.as_str()))
        {
            return None;
        }
        Some((values, target.clone()))
    }

    fn logical_namespace_source_transition(
        facts: &InvocationFacts,
        effects: &tcl_registry::world_effect::EffectFootprint,
        transitions: &tcl_registry::StateTransitions,
    ) -> Option<(
        tcl_registry::NamespaceTransition,
        tcl_registry::NamespaceTransitionTarget,
    )> {
        use tcl_registry::{
            hooks::{AnalyserHookId, LoweringHookId},
            world_effect::{
                CallbackKinds, EffectAccessMode, InterpreterScope, Reentrancy, WorldStateDomain,
            },
        };
        if facts.operation
            != tcl_registry::SemanticOperationId::StructuredLowering(LoweringHookId::NamespaceEval)
            || facts.analyser_hook != Some(AnalyserHookId::NamespaceEval)
            || facts.arity_accepts_frozen_arguments() != Some(true)
            || facts.frame_effect.is_some()
            || effects.callback().kinds != CallbackKinds::SCRIPT
            || effects.callback().reentrancy != Reentrancy::CurrentInterpreter
            || effects.accesses().iter().any(|access| {
                access.interpreter != InterpreterScope::Current
                    || access.mode == EffectAccessMode::Clobber
                    || (access.mode != EffectAccessMode::Read
                        && !matches!(
                            access.domain,
                            WorldStateDomain::NamespaceLookup
                                | WorldStateDomain::InterpreterResult
                                | WorldStateDomain::CompletionState
                        ))
            })
        {
            return None;
        }
        let [fact] = transitions.facts() else {
            return None;
        };
        let tcl_registry::StateTransition::Namespace(
            transition @ tcl_registry::NamespaceTransition::Ensure { namespace },
        ) = &fact.transition
        else {
            return None;
        };
        Some((transition.clone(), namespace.clone()))
    }

    #[inline(never)]
    pub(super) fn walk_logical_namespace_source(
        &mut self,
        segment: &crate::segmenter::SegmentedCommand,
        words: &[crate::ir::WordExpr],
        effective: &[crate::registry_invocation::EffectiveInvocationWord],
        compiled: &compiled_invocation::CompiledInvocationSelection,
        state: &mut ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
    ) -> Option<Box<SourceOutcomes>> {
        use tcl_registry::script_body_flow::ScriptBodyFlow;
        if compiled.compile_error || compiled.unknown {
            return None;
        }
        let (values, target) =
            Self::logical_namespace_source_target(segment, words, effective, state, *context)?;
        let current = context.namespace_identity();
        let (facts, effects, transitions, _) =
            Self::logical_selected_source_facts(&values, &target, state, *context, true)?;
        let prepared = prepare_source_native_invocation(
            &target,
            effective,
            state,
            context.registry,
            context.realm(),
        )
        .ok()?;
        let invocation = prepared.invocation_boxed(&target.command);
        let (transition, namespace) =
            Self::logical_namespace_source_transition(&facts, &effects, &transitions)?;
        let ScriptBodyFlow::ConcatenatedScript { argument_offset } =
            tcl_registry::case_bodies::script_body_flow_in_registry(
                context.registry,
                &facts,
                invocation.arguments(),
            )
        else {
            return None;
        };
        if invocation.arguments().exact_argv_len()? != argument_offset.checked_add(1)? {
            return None;
        }
        let namespace_key = state.namespace_target_key_at(&namespace, &current)?;
        let namespace_name = namespace_key.display()?;
        let previous_variables = Arc::clone(&state.source_variables);
        let previous_frame = state.variable_frame.clone();
        let mut entered = state.clone();
        apply_namespace_transition(
            &mut entered,
            &transition,
            &crate::ir_helpers::ExecutionNamespace::SourceContext(current),
            segment.span.start(),
            None,
        );
        if !entered.namespaces.contains(&namespace_key) || entered.has_opaque_domain() {
            return None;
        }
        *state = entered;
        let frame = crate::var_resolve::VariableExecutionFrame::NamespaceActivation {
            namespace: namespace_name.clone(),
            identity: source_activation_name(
                state.current_source_origin.as_ref(),
                "logical-namespace",
                segment.span.start(),
            ),
        };
        let variables = state
            .source_variables
            .enter_called_frame(&frame)
            .with_namespace_identity(namespace_key.clone());
        state.source_variables = Arc::new(variables);
        state.variable_frame = frame.clone();
        let logical_context = context.conditional();
        let body_context = source_body_context_boxed(&logical_context, &namespace_name, &frame);
        let body_context = source_namespace_context_boxed(&body_context, &namespace_key);
        let selection = tcl_registry::native_compilation::NativeCompilationSelection::Generic;
        let operands = SourceScriptOperands {
            compilation_spec: facts.native_compilation.as_ref(),
            compilation_selection: &selection,
            words,
            written_arguments: context.written_arguments,
            target: &target,
            arguments: invocation.arguments_ref(),
        };
        let mut outcomes = self.walk_body_operand(argument_offset, operands, state, &body_context);
        outcomes.restore_frame(&previous_variables, &previous_frame);
        outcomes.publish(state);
        Some(Box::new(outcomes))
    }
}
