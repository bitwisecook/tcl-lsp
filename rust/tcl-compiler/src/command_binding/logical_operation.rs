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

//! Quiet original operations on the independently selected Logical source model.

use super::{
    Arc, BindingKind, CommandBindingTransition, MayBinding, ModuleCommandBindings,
    SourceCommandTarget, SourceExecutionContext, SourceNamespaceKey, SourceOutcomes,
    StateTransition, apply_may_binding_transition, apply_namespace_transition,
    source_binding_projection,
};
use tcl_registry::{
    ArgRole, InvocationFacts, SemanticOperationId, StateTransitions,
    hooks::{AnalyserHookId, LoweringHookId},
    world_effect::{CallbackKinds, EffectAccessMode, InterpreterScope, WorldStateDomain},
};

struct LogicalOriginalInvocation {
    words: Vec<tcl_lexer::NativeWord>,
    arguments: Vec<String>,
    /// A hosted store's substituted value word follows `arguments`.
    value_unknown: bool,
    target: SourceCommandTarget,
    namespace: SourceNamespaceKey,
}

pub(super) fn transfer(
    words: &[crate::ir::WordExpr],
    target: &SourceCommandTarget,
    offset: u32,
    state: &mut ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> Option<SourceOutcomes> {
    // naming.source.logical-original-operation-transfer
    // docs/design/analysis/name-resolution-proofs/logical-original-operation-transfer.md
    let original =
        LogicalOriginalInvocation::capture(words, target, offset, state, context, false)?;
    let input = super::logical_definition::logical_entry(state, context)?.clone();
    let actual = input.context_registry();
    let arguments = original
        .arguments
        .iter()
        .map(|value| tcl_registry::InvocationWord::Literal(value))
        .collect::<Vec<_>>();
    let resolution =
        tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
            actual.commands(),
            Some(actual.context()),
            tcl_registry::InvocationWords::structured(
                tcl_registry::InvocationWord::Literal(&original.target.command),
                &arguments,
            )
            .with_dialect(super::source_analysis_entry::source_input_dialect(&input)),
            context.realm(),
        );
    let schema = resolution.resolved()?;
    let facts = schema.facts();
    if facts.arity_accepts_frozen_arguments() != Some(true) || facts.frame_effect.is_some() {
        return None;
    }
    let transitions = schema.state_transitions();
    if facts.operation == SemanticOperationId::StructuredLowering(LoweringHookId::Set) {
        variable_continuation(&original, &facts, &transitions, state, offset, context)?;
    } else if matches!(
        facts.analyser_hook,
        Some(AnalyserHookId::Rename | AnalyserHookId::InterpAlias)
    ) {
        command_transfer(&original, &facts, &transitions, state, offset, context)?;
    } else if let Some(phase) = schema.authored_dictionary_scope_source_phase() {
        empty_dictionary_scope_continuation(&original, &schema, &phase, state, offset, context)?;
    } else {
        variable_continuation(&original, &facts, &transitions, state, offset, context)?;
    }
    // A continuation of this authored model has no Native handler, command
    // publication, compiler, frame, argv or Normal completion certificate.
    Some(SourceOutcomes::normal(state))
}

/// A hosted F5 rule's advisory walk of a handler body: a literal `set` is the
/// quiet scalar store of the authored Tcl 8.4 simulation, in the handler's own
/// event frame. Like [`transfer`], this issues no Native handler, completion
/// or entered-execution certificate.
pub(super) fn hosted_scalar_transfer(
    words: &[crate::ir::WordExpr],
    target: &SourceCommandTarget,
    offset: u32,
    state: &mut ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> Option<SourceOutcomes> {
    let original = LogicalOriginalInvocation::capture(words, target, offset, state, context, true)?;
    let input = super::logical_definition::hosted_entry(state, context)?.clone();
    let actual = input.context_registry();
    let mut arguments = original
        .arguments
        .iter()
        .map(|value| tcl_registry::InvocationWord::Literal(value))
        .collect::<Vec<_>>();
    if original.value_unknown {
        arguments.push(tcl_registry::InvocationWord::Dynamic);
    }
    let resolution =
        tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
            actual.commands(),
            Some(actual.context()),
            tcl_registry::InvocationWords::structured(
                tcl_registry::InvocationWord::Literal(&original.target.command),
                &arguments,
            )
            .with_dialect(super::source_analysis_entry::source_input_dialect(&input)),
            context.realm(),
        );
    let facts = resolution.resolved()?.facts();
    if facts.operation != SemanticOperationId::StructuredLowering(LoweringHookId::Set)
        || facts.arity_accepts_frozen_arguments() != Some(true)
        || facts.frame_effect.is_some()
    {
        return None;
    }
    scalar_store(&original, &facts, state, offset, context, true)?;
    Some(SourceOutcomes::normal(state))
}

/// The quiet scalar-store facts of a source-model `set`: the Logical
/// operation's, or for a hosted F5 rule the hosted metadata's.
fn source_model_store_facts(
    hosted: bool,
    registry: &tcl_registry::CommandRegistry,
    metadata: crate::registry_invocation::InvocationMetadataContext<'_>,
    tokens: &crate::ir::CommandTokens,
) -> Option<InvocationFacts> {
    let facts = if hosted {
        crate::registry_invocation::original_hosted_scalar_store_facts(registry, metadata, tokens)?
    } else {
        *crate::registry_invocation::original_logical_operation_invocation_with_metadata_context(
            registry, metadata, tokens,
        )?
        .facts
    };
    (facts.operation == SemanticOperationId::StructuredLowering(LoweringHookId::Set)
        && facts.arity_accepts_frozen_arguments() == Some(true)
        && facts.frame_effect.is_none())
    .then_some(facts)
}

impl super::SourceInvocationBinding {
    /// Quiet Logical store from its exact retained original source phase.
    /// A public address-phase report cannot replace this snapshot. All scalar,
    /// contents-provenance, input and observer checks remain in the store owner.
    pub(crate) fn original_conditional_logical_scalar_store(
        &self,
        tokens: &crate::ir::CommandTokens,
        metadata: crate::registry_invocation::InvocationMetadataContext<'_>,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<crate::variable_bindings::ConditionalLogicalScalarStore> {
        let original = self.original_conditional_operand_layout_advice(tokens, registry)?;
        self.conditional_logical_scalar_store(
            tokens,
            original.original_variable_context(),
            metadata,
            registry,
        )
    }

    /// Quiet scalar contents in the exact authored source continuation. This
    /// receipt does not prove a Native store, completed handler or Normal exit.
    pub(crate) fn conditional_logical_scalar_store(
        &self,
        tokens: &crate::ir::CommandTokens,
        before: &crate::var_resolve::ResolveContext,
        metadata: crate::registry_invocation::InvocationMetadataContext<'_>,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<crate::variable_bindings::ConditionalLogicalScalarStore> {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        use crate::command_binding::OriginalInvocationSourcePurpose as Purpose;
        let input = metadata.source_analysis_input()?;
        let purpose = self.original_invocation_source_purpose(tokens, metadata, registry)?;
        // A hosted F5 rule's handler bodies are advice under the authored
        // simulation; their stores take the same receipt as Logical ones.
        let hosted = match purpose {
            Purpose::Logical => false,
            Purpose::AuthoredSimulation if input.has_hosted_source_name_context() => true,
            _ => return None,
        };
        let metadata = self.original_invocation_metadata_for_input(tokens, input, registry)?;
        (self.original_invocation_source_purpose(tokens, metadata, registry)? == purpose)
            .then_some(())?;
        let point_input = metadata.source_analysis_input()?;
        let facts = source_model_store_facts(hosted, registry, metadata, tokens)?;
        let advice = self.original_conditional_operand_layout_advice(tokens, registry)?;
        let (closed, advised_input) = if hosted {
            (
                advice.closed_hosted_source_lookup(),
                advice.snapshot.state.baseline.vendor_source_input.as_ref(),
            )
        } else {
            (
                advice.closed_logical_source_lookup(),
                advice.snapshot.state.logical_source_name_advice_input(),
            )
        };
        if !closed
            || advised_input != Some(point_input)
            || advice.snapshot.state.source_variables.as_ref() != before
        {
            return None;
        }
        let [target] = advice.targets.as_slice() else {
            return None;
        };
        let hosted_entry = if hosted {
            Some(super::SourceAnalysisEntry::for_fresh_hosted_source(
                point_input,
            )?)
        } else {
            None
        };
        let mut options = match &hosted_entry {
            Some(entry) => entry.options(),
            None => super::SourceAnalysisOptions::for_logical_source(point_input)?,
        };
        options.invocation_realm = advice.realm();
        let frame = advice.snapshot.state.variable_frame.clone();
        let namespace = before.namespace.clone();
        let mut context = super::root_source_execution_context(
            &frame,
            &namespace,
            &advice.namespace,
            advice.config,
            registry,
            options,
            tcl_lexer::SourceSegmentationPurpose::NestedScript,
        );
        context.invocation_offset = advice.site.offset;
        let mut state = advice.snapshot.state.clone();
        let original = LogicalOriginalInvocation::capture(
            &advice.words,
            target,
            advice.site.offset,
            &state,
            context,
            hosted,
        )?;
        let (name, value) = original.store_operands()?;
        let stored = (name.to_owned(), value.map(str::to_owned));
        Arc::make_mut(&mut state.source_variables)
            .set_contents_write_source(Some(Arc::clone(&advice.site.source)));
        scalar_store(
            &original,
            &facts,
            &mut state,
            advice.site.offset,
            context,
            hosted,
        )?;
        crate::variable_bindings::ConditionalLogicalScalarStore::retain(
            before,
            state.source_variables.as_ref(),
            input,
            advice.words,
            advice.site,
            stored,
            registry,
        )
    }
}

impl LogicalOriginalInvocation {
    fn capture(
        words: &[crate::ir::WordExpr],
        target: &SourceCommandTarget,
        offset: u32,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
        hosted: bool,
    ) -> Option<Self> {
        if !target.registry_backed
            || !matches!(target.kind, BindingKind::Builtin | BindingKind::Alias)
        {
            return None;
        }
        let (original, value_unknown) = if hosted {
            super::logical_definition::original_hosted_static_invocation_words(
                words, offset, state, context,
            )
            .map(|original| (original, false))
            .or_else(|| {
                super::logical_definition::original_hosted_store_invocation_words(
                    words, offset, state, context,
                )
                .map(|original| (original, true))
            })
        } else {
            super::logical_definition::original_static_invocation_words(
                words, offset, state, context,
            )
            .map(|original| (original, false))
        }?;
        let values = original
            .iter()
            .take(original.len() - usize::from(value_unknown))
            .map(super::logical_definition::static_value)
            .collect::<Option<Vec<_>>>()?;
        let namespace = context.namespace_identity();
        let selected = source_binding_projection(state, values.first()?, &namespace);
        if selected.unknown
            || selected.may_be_absent
            || selected.targets.as_slice() != std::slice::from_ref(target)
        {
            return None;
        }
        let mut arguments = if target.prepended.is_empty() {
            Vec::new()
        } else {
            let prefix = target.logical_original_prefix.as_ref()?;
            prefix.matches_state(state).then_some(())?;
            let input = state.logical_source_name_advice_input()?;
            let captured = target.original_logical_prepended_source_inputs(
                input,
                state.current_source_origin.as_ref()?.source_image(),
                context.config,
            )?;
            captured
                .iter()
                .map(|input| std::str::from_utf8(input.bytes()).ok().map(str::to_owned))
                .collect::<Option<Vec<_>>>()?
        };
        arguments.extend(values.into_iter().skip(1));
        Some(Self {
            words: original,
            arguments,
            value_unknown,
            target: target.clone(),
            namespace,
        })
    }

    /// The name of a scalar store and its literal value; a hosted store's
    /// substituted value word has none.
    fn store_operands(&self) -> Option<(&str, Option<&str>)> {
        match (self.arguments.as_slice(), self.value_unknown) {
            ([name, value], false) => Some((name, Some(value))),
            ([name], true) => Some((name, None)),
            _ => None,
        }
    }

    fn is_current(
        &self,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> bool {
        let Some(origin) = state.current_source_origin.as_ref() else {
            return false;
        };
        self.namespace == context.namespace_identity()
            && self.words.iter().all(|word| {
                word.image() == origin.source_image() && word.config() == context.config
            })
            && self
                .words
                .first()
                .and_then(super::logical_definition::static_value)
                .is_some_and(|head| {
                    let selected = source_binding_projection(state, &head, &self.namespace);
                    !selected.unknown
                        && !selected.may_be_absent
                        && selected.targets.as_slice() == std::slice::from_ref(&self.target)
                })
    }
}

fn bounded_effects(facts: &InvocationFacts, domains: &[WorldStateDomain]) -> bool {
    matches!(
        facts.effects.callback().kinds,
        CallbackKinds::NONE | CallbackKinds::TRACE
    ) && facts.effects.accesses().iter().all(|access| {
        access.interpreter == InterpreterScope::Current
            && access.mode != EffectAccessMode::Clobber
            && (domains.contains(&access.domain)
                || (access.domain == WorldStateDomain::InterpreterPolicy
                    && access.mode == EffectAccessMode::Read))
    })
}

fn readonly_continuation(
    original: &LogicalOriginalInvocation,
    facts: &InvocationFacts,
    transitions: &StateTransitions,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> bool {
    if !original.is_current(state, context)
        || !transitions.facts().is_empty()
        || !facts.arg_roles_complete
        || state.source_variables.dynamic_bindings
        || state.source_variables.dynamic_traces
        || !bounded_effects(
            facts,
            &[
                WorldStateDomain::VariableStore,
                WorldStateDomain::VariableTraces,
            ],
        )
        || facts
            .effects
            .accesses()
            .iter()
            .any(|access| access.mode != EffectAccessMode::Read)
        || facts
            .arg_roles
            .iter()
            .any(|(_, role)| *role != ArgRole::VarRead)
        || facts.arg_roles.is_empty()
    {
        return false;
    }
    facts.arg_roles.iter().all(|(ordinal, _)| {
        let Some(name) = original
            .arguments
            .get(facts.argument_offset + usize::from(*ordinal))
        else {
            return false;
        };
        let access = crate::var_resolve::resolve_literal_access(
            name,
            &state.source_variables,
            false,
            context.registry,
            tcl_registry::TraceOperation::Read,
        );
        !access.dynamic && !access.observed && logical_cell_is_local(&access, state, context)
    })
}

fn variable_continuation(
    original: &LogicalOriginalInvocation,
    facts: &InvocationFacts,
    transitions: &StateTransitions,
    state: &mut ModuleCommandBindings,
    offset: u32,
    context: SourceExecutionContext<'_>,
) -> Option<()> {
    if (facts.operation == SemanticOperationId::StructuredLowering(LoweringHookId::Set)
        && scalar_store(original, facts, state, offset, context, false).is_some())
        || readonly_continuation(original, facts, transitions, state, context)
    {
        return Some(());
    }
    scalar_write_continuation(original, facts, transitions, state, offset, context)
}

fn scalar_write_roles(facts: &InvocationFacts, transitions: &StateTransitions) -> bool {
    transitions.facts().is_empty()
        && facts.arg_roles_complete
        && bounded_effects(
            facts,
            &[
                WorldStateDomain::VariableStore,
                WorldStateDomain::VariableTraces,
            ],
        )
        && facts
            .arg_roles
            .iter()
            .any(|(_, role)| *role == ArgRole::VarWrite)
        && facts
            .arg_roles
            .iter()
            .all(|(_, role)| matches!(role, ArgRole::VarRead | ArgRole::VarWrite))
        && facts.effects.accesses().iter().all(|access| {
            access.domain == WorldStateDomain::VariableStore
                || access.mode == EffectAccessMode::Read
        })
        && facts.effects.accesses().iter().any(|access| {
            access.domain == WorldStateDomain::VariableStore
                && matches!(
                    access.mode,
                    EffectAccessMode::Write | EffectAccessMode::ReadWrite
                )
        })
}

fn scalar_write_continuation(
    original: &LogicalOriginalInvocation,
    facts: &InvocationFacts,
    transitions: &StateTransitions,
    state: &mut ModuleCommandBindings,
    offset: u32,
    context: SourceExecutionContext<'_>,
) -> Option<()> {
    if !original.is_current(state, context)
        || !scalar_write_roles(facts, transitions)
        || state.source_variables.dynamic_bindings
        || state.source_variables.dynamic_traces
    {
        return None;
    }
    let mut writes = Vec::new();
    for (ordinal, role) in &facts.arg_roles {
        let name = original
            .arguments
            .get(facts.argument_offset + usize::from(*ordinal))?;
        let read = crate::var_resolve::resolve_literal_access(
            name,
            &state.source_variables,
            false,
            context.registry,
            tcl_registry::TraceOperation::Read,
        );
        if read.dynamic || read.observed || !logical_cell_is_local(&read, state, context) {
            return None;
        }
        if *role == ArgRole::VarWrite {
            let write = crate::var_resolve::resolve_literal_access(
                name,
                &state.source_variables,
                false,
                context.registry,
                tcl_registry::TraceOperation::Write,
            );
            if write.dynamic
                || write.observed
                || write.kind != crate::place::PlaceKind::Scalar
                || write.index.is_some()
                || !logical_cell_is_local(&write, state, context)
                || state.source_variables.root_contents_kind(&write)
                    == Some(crate::var_resolve::RootContentsKind::Array)
                || state.source_variables.store_would_error(&write)
            {
                return None;
            }
            writes.push(write);
        }
    }
    // Conditional footprints withdraw values without publishing a successful store.
    let variables = Arc::make_mut(&mut state.source_variables);
    for write in writes {
        variables.record_contents_write(&write, offset, true);
    }
    Some(())
}

fn original_empty_dictionary_body(
    original: &LogicalOriginalInvocation,
    schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    phase: &tcl_registry::dictionary_scope::AuthoredDictionaryScopeSourcePhase,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> Option<()> {
    let facts = schema.facts();
    if !original.is_current(state, context)
        || !schema.state_transitions().facts().is_empty()
        || !facts.arg_roles_complete
        || !matches!(
            phase.plan().bindings,
            tcl_registry::dictionary_scope::DictionaryScopeBindings::AllKeys(_)
        )
        || schema.semantics.body_kind != tcl_registry::BodyKind::Plain
        || schema.semantics.body_interpreter != tcl_registry::BodyInterpreter::Current
        || schema.authored_source_script_timing_at(phase.plan().body_argument)
            != Some(tcl_registry::ScriptTiming::SameInvocation)
        || state.source_variables.dynamic_bindings
        || state.source_variables.dynamic_traces
        || !state.source_variables.traced.is_empty()
        || !matches!(
            phase.effects().callback().kinds,
            CallbackKinds::NONE | CallbackKinds::TRACE
        )
        || phase
            .additional_effects()
            .accesses()
            .iter()
            .any(|access| access.mode != EffectAccessMode::Read)
        || phase.effects().callback().reentrancy
            == tcl_registry::world_effect::Reentrancy::AnyInterpreter
        || phase.effects().accesses().iter().any(|access| {
            access.interpreter != InterpreterScope::Current
                || access.mode == EffectAccessMode::Clobber
                || (access.domain != WorldStateDomain::VariableStore
                    && access.mode != EffectAccessMode::Read)
                || !(matches!(
                    access.domain,
                    WorldStateDomain::VariableStore | WorldStateDomain::VariableTraces
                ) || (access.domain == WorldStateDomain::InterpreterPolicy
                    && access.mode == EffectAccessMode::Read))
        })
    {
        return None;
    }
    let roles = facts
        .arg_roles
        .iter()
        .map(|(ordinal, role)| (facts.argument_offset + usize::from(*ordinal), *role))
        .collect::<Vec<_>>();
    if roles.len() != 3
        || ![
            (phase.plan().dictionary_argument, ArgRole::VarRead),
            (phase.plan().dictionary_argument, ArgRole::VarWrite),
            (phase.plan().body_argument, ArgRole::Body),
        ]
        .iter()
        .all(|expected| roles.contains(expected))
    {
        return None;
    }
    let written = phase
        .plan()
        .body_argument
        .checked_sub(original.target.prepended.len())?
        .checked_add(1)?;
    let body = original.words.get(written)?;
    if !crate::registry_invocation::original_written_script_body_is_empty(body)? {
        return None;
    }
    Some(())
}

fn empty_dictionary_scope_continuation(
    original: &LogicalOriginalInvocation,
    schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    phase: &tcl_registry::dictionary_scope::AuthoredDictionaryScopeSourcePhase,
    state: &mut ModuleCommandBindings,
    offset: u32,
    context: SourceExecutionContext<'_>,
) -> Option<()> {
    original_empty_dictionary_body(original, schema, phase, state, context)?;
    let name = original.arguments.get(phase.plan().dictionary_argument)?;
    let write = crate::var_resolve::resolve_literal_access(
        name,
        &state.source_variables,
        false,
        context.registry,
        tcl_registry::TraceOperation::Write,
    );
    let read = crate::var_resolve::resolve_literal_access(
        name,
        &state.source_variables,
        false,
        context.registry,
        tcl_registry::TraceOperation::Read,
    );
    if write.dynamic
        || write.observed
        || read.dynamic
        || read.observed
        || write.kind != crate::place::PlaceKind::Scalar
        || write.index.is_some()
        || !logical_cell_is_local(&write, state, context)
        || !logical_cell_is_local(&read, state, context)
        || state.source_variables.store_would_error(&write)
    {
        return None;
    }
    let arguments = original
        .arguments
        .iter()
        .map(|value| tcl_registry::InvocationWord::Literal(value))
        .collect::<Vec<_>>();
    let input = state.logical_source_name_advice_input()?;
    let arguments = tcl_registry::InvocationArguments::structured(&arguments)
        .with_dialect(super::source_analysis_entry::source_input_dialect(input));
    let footprint = crate::dictionary_bindings::original_dictionary_scope_may_footprint(
        &state.source_variables,
        phase,
        arguments,
        context.registry,
    )?;
    // Possible source writes only withdraw contents; no reached scope is installed.
    let variables = Arc::make_mut(&mut state.source_variables);
    for place in footprint.clobbers.iter().chain(std::iter::once(&write)) {
        variables.record_contents_write(place, offset, true);
    }
    Some(())
}

fn scalar_store(
    original: &LogicalOriginalInvocation,
    facts: &InvocationFacts,
    state: &mut ModuleCommandBindings,
    offset: u32,
    context: SourceExecutionContext<'_>,
    hosted: bool,
) -> Option<()> {
    let (name, value) = original.store_operands()?;
    if !original.is_current(state, context)
        || !facts.arg_roles_complete
        || facts.argument_offset != 0
        || facts.arg_roles.as_slice() != [(0, ArgRole::VarWrite)]
        || !bounded_effects(
            facts,
            &[
                WorldStateDomain::VariableStore,
                WorldStateDomain::VariableTraces,
            ],
        )
        || state.source_variables.dynamic_bindings
        || state.source_variables.dynamic_traces
    {
        return None;
    }
    let access = crate::var_resolve::resolve_literal_access(
        name,
        &state.source_variables,
        false,
        context.registry,
        tcl_registry::TraceOperation::Write,
    );
    if access.dynamic
        || access.observed
        || access.kind != crate::place::PlaceKind::Scalar
        || access.index.is_some()
        || !(if hosted {
            hosted_cell_is_local(&access, state, context)
        } else {
            logical_cell_is_local(&access, state, context)
        })
        || state.source_variables.root_contents_kind(&access)
            == Some(crate::var_resolve::RootContentsKind::Array)
        || state.source_variables.store_would_error(&access)
    {
        return None;
    }
    let variables = Arc::make_mut(&mut state.source_variables);
    variables.record_contents_write(&access, offset, false);
    match value {
        Some(value) => variables.define_literal(name, value, context.registry),
        None => variables.define_stored_contents(name, context.registry),
    }
    Some(())
}

/// Check an authored continuation's cell coordinate, independently of the
/// storage-domain facet. This never selects a physical namespace or store.
pub(super) fn logical_cell_is_local(
    access: &crate::place::Place,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> bool {
    use crate::place::CellOwner;
    use tcl_registry::f5::VariableStorageDomain;
    if super::logical_definition::logical_entry(state, context).is_none() {
        return false;
    }
    let variables = &state.source_variables;
    let namespace = context.namespace_identity();
    let Some(cell) = access.cell.as_ref() else {
        return false;
    };
    if !matches!(namespace, SourceNamespaceKey::Authored(_))
        || variables.namespace_identity.as_ref() != Some(&namespace)
        || variables.interpreter.is_some()
        || variables.execution.is_some()
        || variables.hosted_execution_context.is_some()
        || variables.execution_name_policy.is_some()
        || cell.interpreter.is_some()
        || cell.execution.is_some()
        || cell.generation == crate::place::CellGeneration::Unknown
        || cell.generation
            != variables
                .generations
                .get(&crate::var_resolve::cell_key(access))
                .copied()
                .unwrap_or_default()
    {
        return false;
    }
    match &cell.owner {
        CellOwner::Namespace(_) | CellOwner::NamespaceIdentity(_) => {
            variables.namespace_footprint(access) == Some(namespace)
                && matches!(
                    cell.storage_domain,
                    None | Some(VariableStorageDomain::InterpreterNamespace)
                )
        }
        CellOwner::Activation(activation) => {
            variables.activation.as_ref() == Some(activation) && cell.storage_domain.is_none()
        }
        _ => false,
    }
}

/// [`logical_cell_is_local`] in a hosted rule's advisory walk: only the
/// handler's own event-frame activation cell, under the authored simulation.
pub(super) fn hosted_cell_is_local(
    access: &crate::place::Place,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> bool {
    use crate::place::CellOwner;
    if super::logical_definition::hosted_entry(state, context).is_none() {
        return false;
    }
    let variables = &state.source_variables;
    let namespace = context.namespace_identity();
    let Some(cell) = access.cell.as_ref() else {
        return false;
    };
    if !matches!(namespace, SourceNamespaceKey::Authored(_))
        || variables.namespace_identity.as_ref() != Some(&namespace)
        || variables.interpreter.is_some()
        || variables.execution.is_some()
        || cell.interpreter.is_some()
        || cell.execution.is_some()
        || cell.storage_domain.is_some()
        || cell.generation == crate::place::CellGeneration::Unknown
        || cell.generation
            != variables
                .generations
                .get(&crate::var_resolve::cell_key(access))
                .copied()
                .unwrap_or_default()
    {
        return false;
    }
    // A handler with no activation of its own runs at the global level
    // (`RULE_INIT`: `namespace current` is `::`, BIGIP-NAME-050), so its
    // bare name is a cell of that namespace. Any handler may also write its
    // worker's own `::static` cells.
    match &cell.owner {
        CellOwner::NamespaceIdentity(_)
            if crate::variable_bindings::is_hosted_worker_namespace_cell(cell, variables) =>
        {
            true
        }
        CellOwner::Namespace(_) | CellOwner::NamespaceIdentity(_) => {
            variables.activation.is_none()
                && variables.namespace_footprint(access) == Some(namespace)
        }
        CellOwner::Activation(activation) => variables.activation.as_ref() == Some(activation),
        _ => false,
    }
}

fn command_transfer(
    original: &LogicalOriginalInvocation,
    facts: &InvocationFacts,
    transitions: &StateTransitions,
    state: &mut ModuleCommandBindings,
    offset: u32,
    context: SourceExecutionContext<'_>,
) -> Option<()> {
    if !original.is_current(state, context)
        || !bounded_effects(
            facts,
            &[
                WorldStateDomain::CommandBindings,
                WorldStateDomain::CommandTraces,
                WorldStateDomain::NamespaceLookup,
            ],
        )
        || transitions.facts().is_empty()
    {
        return None;
    }
    let namespace =
        crate::ir_helpers::ExecutionNamespace::SourceContext(original.namespace.clone());
    let mut candidate = state.clone();
    for fact in transitions.facts() {
        // A quiet authored continuation interprets the selected transition
        // conditionally. Native abrupt-edge commit classification remains on
        // the descriptor; it is neither changed nor a Normal certificate.
        match &fact.transition {
            StateTransition::Namespace(
                transition @ tcl_registry::NamespaceTransition::Ensure { .. },
            ) => {
                apply_namespace_transition(&mut candidate, transition, &namespace, offset, None);
            }
            StateTransition::CommandBinding(transition) => {
                admissible_transition(&candidate, transition, &original.namespace)?;
                apply_may_binding_transition(
                    &mut candidate,
                    transition,
                    &namespace,
                    None,
                    offset,
                    None,
                    None,
                );
                if let CommandBindingTransition::Alias { alias, .. } = transition {
                    let selected = source_binding_projection(
                        &candidate,
                        alias.literal()?,
                        &SourceNamespaceKey::authored("::"),
                    );
                    if selected.unknown {
                        return None;
                    }
                    retain_logical_alias_prefix(
                        original,
                        transition,
                        &mut candidate,
                        offset,
                        context,
                    )?;
                }
            }
            _ => return None,
        }
    }
    if candidate.has_opaque_domain() {
        return None;
    }
    *state = candidate;
    Some(())
}

/// Optional authored-key capture is stronger than original source arguments;
/// refusing this projection must not erase their independently retained words.
fn authored_logical_alias_prefix_inputs(
    original: &LogicalOriginalInvocation,
    arguments: &[tcl_registry::TransitionSubject],
    input: &crate::analyser::ResolvedAnalysisInput,
    config: tcl_lexer::LexerConfig,
) -> Option<Vec<crate::signature_scan::scope::SignatureSourceNameInput>> {
    let policy = tcl_registry::InvocationDialect::of_profile(input.analyser_profile())
        .authored_name_policy()?;
    (tcl_registry::InvocationDialect::of_profile(input.unit_profile()).authored_name_policy()
        == Some(policy))
    .then_some(())?;
    let purpose =
        crate::signature_scan::original_name::OriginalSourceNamePurpose::for_analysis_input(
            input, None, policy,
        );
    let captured = arguments
        .iter()
        .map(|subject| {
            let effective = subject.argument_index()?;
            let value = original.arguments.get(effective)?;
            (subject.literal() == Some(value.as_str())).then_some(())?;
            let input = if effective < original.target.prepended.len() {
                original
                    .target
                    .logical_original_prefix
                    .as_ref()?
                    .inputs()?
                    .get(effective)?
                    .clone()
            } else {
                let written = effective
                    .checked_sub(original.target.prepended.len())?
                    .checked_add(1)?;
                let word = original.words.get(written)?;
                crate::signature_scan::scope::SignatureSourceNameInput::OriginalWord(purpose.key(
                    word,
                    tcl_syntax::word_rules::WordValueRules::from_config(&config),
                )?)
            };
            (input.bytes() == value.as_bytes() && input.policy() == policy).then_some(input)
        })
        .collect::<Option<Vec<_>>>()?;
    Some(captured)
}

fn retain_logical_alias_prefix(
    original: &LogicalOriginalInvocation,
    transition: &CommandBindingTransition,
    state: &mut ModuleCommandBindings,
    offset: u32,
    context: SourceExecutionContext<'_>,
) -> Option<()> {
    let CommandBindingTransition::Alias {
        alias, arguments, ..
    } = transition
    else {
        return None;
    };
    let input = super::logical_definition::logical_entry(state, context)?.clone();
    let site = super::CommandAllocationSite {
        source: Arc::clone(state.current_source_origin.as_ref()?),
        offset,
    };
    (original.words.first()?.span().start() == offset).then_some(())?;
    let source_arguments = arguments
        .iter()
        .map(|subject| {
            let effective = subject.argument_index()?;
            let value = original.arguments.get(effective)?;
            (subject.literal() == Some(value.as_str())).then_some(())?;
            let source = if effective < original.target.prepended.len() {
                original
                    .target
                    .logical_original_prefix
                    .as_ref()?
                    .source_inputs()
                    .get(effective)?
                    .clone()
            } else {
                let written = effective
                    .checked_sub(original.target.prepended.len())?
                    .checked_add(1)?;
                super::OriginalLogicalSourceNameInput::from_original_source_word(
                    original.words.get(written)?,
                    &input,
                )?
            };
            (source.bytes() == value.as_bytes()).then_some(source)
        })
        .collect::<Option<Vec<_>>>()?;
    // The stronger authored-key projection remains independently unavailable
    // when this Logical source purpose has no naming recipe.
    let captured =
        authored_logical_alias_prefix_inputs(original, arguments, &input, context.config);
    let key = state.publication_key_at(
        &SourceNamespaceKey::authored("::"),
        alias.literal()?,
        super::namespace_slots::PublicationPurpose::Alias,
    )?;
    let bindings = state.binding_alternatives(&key);
    let mut targets = bindings.iter().filter_map(|binding| match binding {
        MayBinding::Target(target) if !target.terminal && target.kind == BindingKind::Alias => {
            Some(target)
        }
        _ => None,
    });
    let target = targets.next()?.clone();
    (targets.next().is_none()
        && bindings.len() == 1
        && target.prepended.len() == source_arguments.len())
    .then_some(())?;
    let receipt = super::logical_prefix::LogicalAliasPrefix {
        input,
        site,
        words: original.words.clone(),
        arguments: captured,
        source_arguments,
        parent: original.target.logical_original_prefix.clone(),
    };
    Arc::make_mut(&mut state.logical_alias_prefixes).insert(target, Some(Arc::new(receipt)));
    Some(())
}

fn admissible_transition(
    state: &ModuleCommandBindings,
    transition: &CommandBindingTransition,
    namespace: &SourceNamespaceKey,
) -> Option<()> {
    match transition {
        CommandBindingTransition::Move { from, to } => {
            known_source(state, from.literal()?, namespace)?;
            let key = state.publication_key_at(
                namespace,
                to.literal()?,
                super::namespace_slots::PublicationPurpose::Rename,
            )?;
            if !state.namespaces.contains(key.holder().as_ref())
                || state.binding_alternatives(&key)
                    != std::collections::BTreeSet::from([MayBinding::Missing])
            {
                return None;
            }
        }
        CommandBindingTransition::Delete { interpreter, name } => {
            let namespace = match interpreter {
                None => namespace.clone(),
                Some(path) if path.literal() == Some("") => SourceNamespaceKey::authored("::"),
                Some(_) => return None,
            };
            known_source(state, name.literal()?, &namespace)?;
        }
        CommandBindingTransition::Alias {
            source_interpreter,
            alias,
            target_interpreter,
            target,
            arguments,
            ..
        } => {
            if source_interpreter.literal() != Some("")
                || target_interpreter.literal() != Some("")
                || target.literal()?.is_empty()
                || arguments.iter().any(|value| value.literal().is_none())
            {
                return None;
            }
            let key = state.publication_key_at(
                &SourceNamespaceKey::authored("::"),
                alias.literal()?,
                super::namespace_slots::PublicationPurpose::Alias,
            )?;
            if key.authored_spelling()?.is_empty()
                || !state.namespaces.contains(key.holder().as_ref())
            {
                return None;
            }
        }
        CommandBindingTransition::Define { .. } | CommandBindingTransition::Unknown { .. } => {
            return None;
        }
    }
    Some(())
}

fn known_source(
    state: &ModuleCommandBindings,
    name: &str,
    namespace: &SourceNamespaceKey,
) -> Option<()> {
    let keys = state.source_keys_checked(name, namespace).ok()?;
    let [key] = keys.as_slice() else {
        return None;
    };
    let alternatives = state.binding_alternatives(key);
    if alternatives.len() != 1 || !matches!(alternatives.first(), Some(MayBinding::Target(_))) {
        return None;
    }
    Some(())
}

#[cfg(test)]
mod tests;
