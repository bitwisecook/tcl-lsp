// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Source correspondence for analysis data without an execution licence.

use super::{
    Arc, BodyProofScope, ModuleCommandBindings, Script, SourceInvocationBinding, SourceOriginId,
};
use crate::registry_invocation::{OwnedInvocationMetadataContext, RegistryInvocationShape};
use tcl_registry::CommandRegistry;

#[derive(Debug, Clone, PartialEq, Eq)]
struct OriginalRegistryDependency {
    offset: u32,
    namespace: super::SourceNamespaceKey,
    dialect: tcl_registry::InvocationDialect,
    realm: tcl_dialect::model::InvocationRealm,
    targets: Vec<super::SourceCommandTarget>,
    shapes: Vec<RegistryInvocationShape>,
    native_source_uncertainty: Option<(bool, bool, bool, bool)>,
    native_observers: Option<Arc<super::command_observers::SourceCommandObservers>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum AnalyticalCommandDependency {
    Positioned(OriginalRegistryDependency),
    NativeSchema(NativeSourceSchemaDependency),
}
impl AnalyticalCommandDependency {
    fn offset(&self) -> u32 {
        match self {
            Self::Positioned(value) => value.offset,
            Self::NativeSchema(value) => value.offset,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeSourceSchemaDependency {
    offset: u32,
    command: String,
    dialect: tcl_registry::InvocationDialect,
    arguments: Vec<crate::registry_invocation::EffectiveInvocationWord>,
    origins: Vec<crate::registry_invocation::InvocationWordOrigin>,
    roles: Vec<(usize, tcl_registry::ArgRole)>,
    namespace: Option<tcl_core_types::ByteNamespacePath>,
    path: Option<Vec<tcl_core_types::ByteNamespacePath>>,
    obligations: Vec<super::SourceCommandTransitionObligation>,
    observers: Arc<super::command_observers::SourceCommandObservers>,
    source_step_observed: bool,
}

/// A private comparison value; its rewritten carriers never leave this owner.
#[derive(Debug, Clone)]
pub(crate) struct AnalyticalBodyCorrespondence {
    pub(crate) body: Script,
    pub(crate) relocation: crate::var_resolve::VariableProofRelocation,
    dependencies: Vec<AnalyticalCommandDependency>,
    purpose: CorrespondencePurpose,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CorrespondencePurpose {
    Logical,
    NativeSource {
        input: crate::analyser::ResolvedAnalysisInput,
        entry: super::SourceAnalysisEntry,
    },
}

impl PartialEq for AnalyticalBodyCorrespondence {
    fn eq(&self, other: &Self) -> bool {
        self.purpose == other.purpose
            && self.body == other.body
            && self.dependencies == other.dependencies
    }
}

impl ModuleCommandBindings {
    pub(crate) fn prepare_analytical_body_correspondence(
        &self,
        body: &Script,
        scope: BodyProofScope<'_>,
        registry: &CommandRegistry,
    ) -> Option<AnalyticalBodyCorrespondence> {
        let OwnedInvocationMetadataContext::SuppliedSource(input) = &self.baseline.metadata_context
        else {
            return None;
        };
        let OwnedInvocationMetadataContext::SuppliedSource(request) = scope.metadata.retain()
        else {
            return None;
        };
        if request.as_ref() != input.as_ref() && request.as_ref() != &input.for_nested_source() {
            return None;
        }
        if self.baseline.logical_source_input.as_ref() != Some(input.as_ref())
            || self.baseline.vendor_source_input.is_some()
            || self.baseline.registry_snapshot.as_ref() != Some(&registry.snapshot().semantic_key())
            || scope.config.nested().normalized() != request.lexer_config().nested().normalized()
        {
            return None;
        }
        let source = body.executed_source.as_ref()?;
        if source.try_text().ok()? != scope.body_source
            || !matches!(source.origin.kind(), super::SourceOriginKind::Authored(image) if image.bytes() == scope.source.as_bytes())
            || self.current_source_origin.as_ref() != Some(&source.origin)
        {
            return None;
        }
        let start = usize::try_from(scope.executable_body_offset).ok()?;
        let end = start.checked_add(scope.body_source.len())?;
        if scope.source.get(start..end) != Some(scope.body_source) {
            return None;
        }
        let mut projection = body.clone();
        let mut relocation = crate::var_resolve::VariableProofRelocation::default();
        let mut dependencies = std::collections::BTreeMap::new();
        let mut valid = true;
        crate::ir::for_each_script_mut(&mut projection, &mut |script| {
            valid &= script.implicit_math_invocations.is_empty()
                && script.expression_preparations.is_empty()
                && script.native_compilation_failure.is_none();
            for statement in &script.statements {
                let tokens = statement.tokens().or_else(|| {
                    script
                        .command_binding_sites
                        .iter()
                        .find(|site| site.span.start() == statement.span().start())
                        .and_then(|site| site.source_tokens.as_deref())
                });
                let Some(tokens) = tokens else {
                    valid = false;
                    continue;
                };
                valid &= collect_registry_dependencies(
                    tokens,
                    scope,
                    self,
                    registry,
                    &mut dependencies,
                    &mut relocation,
                )
                .is_some();
            }
            for site in script.command_binding_sites.iter() {
                if let Some(tokens) = &site.source_tokens {
                    valid &= collect_registry_dependencies(
                        tokens,
                        scope,
                        self,
                        registry,
                        &mut dependencies,
                        &mut relocation,
                    )
                    .is_some();
                } else {
                    valid = false;
                }
                super::body_template::collect_body_offset(
                    &mut relocation,
                    scope,
                    site.span.start(),
                );
                if let Some(context) = &site.variable_context {
                    super::body_template::collect_body_activation(&mut relocation, context);
                }
            }
        });
        if !valid || relocation.activations.len() > 1 || relocation.inverse().is_none() {
            return None;
        }
        let prefix = usize::try_from(
            scope
                .executable_body_offset
                .checked_sub(scope.original_body_offset)?,
        )
        .ok()?;
        let mut text = " ".repeat(prefix);
        text.push_str(scope.body_source);
        let origin = Arc::new(SourceOriginId::authored(&Arc::from(text)));
        relocation.source_origins.insert(
            Arc::new(SourceOriginId::authored(&Arc::from(scope.source))),
            Arc::clone(&origin),
        );
        crate::ir::for_each_script_mut(&mut projection, &mut |script| {
            valid &=
                super::body_template::normalize_script_source(script, scope, &origin).is_some();
            for site in script.command_binding_sites.iter_mut() {
                site.variable_frame = site
                    .variable_frame
                    .as_ref()
                    .map(|frame| relocation.frame(frame));
                site.variable_context = site
                    .variable_context
                    .as_ref()
                    .map(|context| Arc::new(context.relocated(&relocation)));
                if let Some(tokens) = &mut site.source_tokens {
                    normalize_analysis_tokens(tokens, scope, &origin, &relocation);
                }
            }
            for statement in &mut script.statements {
                if let Some(tokens) = statement.tokens_mut() {
                    normalize_analysis_tokens(tokens, scope, &origin, &relocation);
                }
            }
        });
        valid.then(|| AnalyticalBodyCorrespondence {
            body: projection,
            relocation,
            dependencies: dependencies.into_values().collect(),
            purpose: CorrespondencePurpose::Logical,
        })
    }
    pub(crate) fn prepare_native_source_body_correspondence(
        &self,
        body: &Script,
        scope: BodyProofScope<'_>,
        module: &crate::ir::Module,
        id: crate::ir::ProcedureId,
        registry: &CommandRegistry,
    ) -> Option<AnalyticalBodyCorrespondence> {
        let input = module.source_metadata_input.as_ref()?;
        let card = module.original_native_declaration_body_units.get(&id)?;
        let procedure = module.body_units.get(&id)?;
        let request = scope.metadata.retain();
        let OwnedInvocationMetadataContext::SuppliedSource(request) = request else {
            return None;
        };
        if !module.procedure_ids.owns(id)
            || !card.matches_procedure(module, procedure)
            || scope.source != module.source.try_text().ok()?
            || scope.body_source != card.body_text()?
            || scope.original_body_offset != procedure.span.start()
            || scope.executable_body_offset != procedure.body_offset
            || request.as_ref() != input && request.as_ref() != &input.for_nested_source()
            || self.baseline.metadata_context.source_analysis_input() != Some(input)
            || self.baseline.logical_source_input.is_some()
            || self.baseline.vendor_source_input.is_some()
            || self.baseline.native_entry.is_some()
            || self.baseline.registry_snapshot.as_ref() != Some(&registry.snapshot().semantic_key())
            || scope.config.nested().normalized() != request.lexer_config().nested().normalized()
            || body.executed_source.is_some()
            || body.native_compilation_admission.is_some()
        {
            return None;
        }
        let mut current = procedure.body.clone();
        crate::lattice_rebase::rebase_script(&mut current, -i64::from(scope.original_body_offset));
        if &current != body {
            return None;
        }
        let mut projection = body.clone();
        let mut relocation = crate::var_resolve::VariableProofRelocation::default();
        let mut dependencies = std::collections::BTreeMap::new();
        let mut valid = true;
        crate::ir::for_each_script_mut(&mut projection, &mut |script| {
            valid &= script.implicit_math_invocations.is_empty()
                && script.expression_preparations.is_empty()
                && script.native_compilation_failure.is_none();
            for statement in &script.statements {
                let tokens = statement.tokens().or_else(|| {
                    script
                        .command_binding_sites
                        .iter()
                        .find(|site| site.span.start() == statement.span().start())
                        .and_then(|site| site.source_tokens.as_deref())
                });
                let Some(tokens) = tokens else {
                    valid = false;
                    continue;
                };
                valid &= collect_native_source_registry_dependencies(
                    tokens,
                    scope,
                    self,
                    module,
                    id,
                    registry,
                    &mut dependencies,
                    &mut relocation,
                )
                .is_some();
            }
            for site in script.command_binding_sites.iter() {
                if let Some(tokens) = &site.source_tokens {
                    valid &= collect_native_source_registry_dependencies(
                        tokens,
                        scope,
                        self,
                        module,
                        id,
                        registry,
                        &mut dependencies,
                        &mut relocation,
                    )
                    .is_some();
                } else {
                    valid = false;
                }
                super::body_template::collect_body_offset(
                    &mut relocation,
                    scope,
                    site.span.start(),
                );
                if let Some(context) = &site.variable_context {
                    super::body_template::collect_body_activation(&mut relocation, context);
                }
            }
        });
        if !valid || relocation.activations.len() > 1 || relocation.inverse().is_none() {
            return None;
        }
        let prefix = usize::try_from(
            scope
                .executable_body_offset
                .checked_sub(scope.original_body_offset)?,
        )
        .ok()?;
        let mut text = " ".repeat(prefix);
        text.push_str(scope.body_source);
        let origin = Arc::new(SourceOriginId::authored(&Arc::from(text)));
        relocation.source_origins.insert(
            Arc::new(SourceOriginId::authored(&Arc::from(scope.source))),
            Arc::clone(&origin),
        );
        crate::ir::for_each_script_mut(&mut projection, &mut |script| {
            valid &=
                script.executed_source.is_none() && script.native_compilation_admission.is_none();
            for site in script.command_binding_sites.iter_mut() {
                site.variable_frame = site
                    .variable_frame
                    .as_ref()
                    .map(|frame| relocation.frame(frame));
                site.variable_context = site
                    .variable_context
                    .as_ref()
                    .map(|context| Arc::new(context.relocated(&relocation)));
                if let Some(tokens) = &mut site.source_tokens {
                    normalize_analysis_tokens(tokens, scope, &origin, &relocation);
                }
            }
            for statement in &mut script.statements {
                if let Some(tokens) = statement.tokens_mut() {
                    normalize_analysis_tokens(tokens, scope, &origin, &relocation);
                }
            }
        });
        valid.then(|| AnalyticalBodyCorrespondence {
            body: projection,
            relocation,
            dependencies: dependencies.into_values().collect(),
            purpose: CorrespondencePurpose::NativeSource {
                input: input.clone(),
                entry: module.source_entry.clone(),
            },
        })
    }
}

fn collect_registry_dependencies(
    tokens: &crate::ir::CommandTokens,
    scope: BodyProofScope<'_>,
    bindings: &ModuleCommandBindings,
    registry: &CommandRegistry,
    dependencies: &mut std::collections::BTreeMap<u32, AnalyticalCommandDependency>,
    relocation: &mut crate::var_resolve::VariableProofRelocation,
) -> Option<()> {
    let mut original = tokens.clone();
    crate::lattice_rebase::rebase_command_tokens(
        &mut original,
        i64::from(scope.original_body_offset),
    );
    let mut calls = vec![original.clone()];
    let children = crate::word_subst::checked_original_lifted_calls_with_metadata_context(
        &original,
        scope.config,
        registry,
        bindings
            .baseline
            .metadata_context
            .metadata_context(registry)??,
    )?;
    for child in children {
        calls.push(child.tokens?);
    }
    for (_, proof) in &original.nested_bindings {
        let (_, mut recorded) = proof.original_recorded_command()?;
        recorded.source_binding = Some(proof.clone());
        calls.push(recorded);
    }
    for call in calls {
        let dependency = AnalyticalCommandDependency::Positioned(original_registry_dependency(
            &call, scope, bindings, registry,
        )?);
        let offset = dependency.offset();
        if dependencies
            .get(&offset)
            .is_some_and(|old| old != &dependency)
        {
            return None;
        }
        dependencies.insert(offset, dependency);
        super::body_template::collect_token_activations(relocation, &call);
        super::body_template::collect_body_offset(relocation, scope, offset);
    }
    Some(())
}

fn original_registry_dependency(
    tokens: &crate::ir::CommandTokens,
    scope: BodyProofScope<'_>,
    bindings: &ModuleCommandBindings,
    registry: &CommandRegistry,
) -> Option<OriginalRegistryDependency> {
    let proof = tokens.source_binding.as_ref()?;
    let site = proof.invocation_site()?;
    let end =
        u64::from(scope.executable_body_offset) + u64::try_from(scope.body_source.len()).ok()?;
    if !matches!(site.source.kind(), super::SourceOriginKind::Authored(image) if image.bytes() == scope.source.as_bytes())
        || site.offset < scope.executable_body_offset
        || end <= u64::from(site.offset)
    {
        return None;
    }
    if !analysis_carriers_supported(proof)
        || tokens.evaluated_body.is_some()
        || tokens.synthetic.is_some()
    {
        return None;
    }
    let metadata = proof
        .original_invocation_metadata_for_owner(
            tokens,
            &bindings.baseline.metadata_context,
            registry,
        )
        .ok()??;
    let advice = proof.original_conditional_operand_layout_advice(tokens, registry)?;
    if !advice.closed_logical_source_lookup()
        || advice.has_opaque_handler_alternatives()
        || advice.snapshot.state.baseline.as_ref() != bindings.baseline.as_ref()
        || !advice.snapshot.state.command_observers.is_quiet()
        || advice.snapshot.state.source_step_observed()
        || advice.targets().iter().any(|target| {
            !target.registry_backed
                || !target.prepended.is_empty()
                || target.implementation_allocation.is_some()
                || target
                    .identity
                    .as_ref()
                    .is_some_and(|identity| identity.allocation.is_some())
        })
    {
        return None;
    }
    let mut assistance =
        crate::registry_invocation::original_registry_invocation_assistance_with_metadata_context(
            registry,
            Some(metadata),
            tokens,
        )?;
    assistance.unanimous_command_words()?;
    for shape in &mut assistance.candidates {
        shape.with_source_schema(registry, metadata, tokens, |schema| {
            let effects = schema.effects();
            use tcl_registry::world_effect::{CallbackKinds, WorldStateDomain};
            let callbacks = effects.callback().kinds;
            let trace_free = !proof.variable_context.dynamic_traces
                && proof.variable_context.traced.is_empty()
                && proof.variable_context.trace_registrations.is_empty()
                && proof.variable_context.untracked_traces.is_empty();
            let permitted = (callbacks == CallbackKinds::NONE
                || (callbacks == CallbackKinds::TRACE && trace_free))
                && schema.semantics.body_execution.is_none()
                && schema.semantics.frame_effect.is_none()
                && schema.state_transitions().facts().is_empty()
                && effects.accesses().iter().all(|access| {
                    matches!(
                        access.domain,
                        WorldStateDomain::VariableStore
                            | WorldStateDomain::VariableTraces
                            | WorldStateDomain::InterpreterResult
                            | WorldStateDomain::CompletionState
                            | WorldStateDomain::LegacyExternal(
                                tcl_registry::side_effects::SideEffectTarget::ChannelIo
                                    | tcl_registry::side_effects::SideEffectTarget::FileIo
                                    | tcl_registry::side_effects::SideEffectTarget::LogIo
                            )
                    )
                });
            permitted.then_some(())
        })?;
        let mut effective = tokens.clone();
        effective.word_exprs.clone_from(&shape.effective.words);
        crate::lattice_rebase::rebase_command_tokens(
            &mut effective,
            -i64::from(scope.original_body_offset),
        );
        shape.effective.words = effective.word_exprs;
    }
    Some(OriginalRegistryDependency {
        offset: proof
            .invocation_site()?
            .offset
            .checked_sub(scope.original_body_offset)?,
        namespace: advice.namespace().clone(),
        dialect: advice.dialect(),
        realm: advice.realm(),
        targets: advice.targets().to_vec(),
        shapes: assistance.candidates,
        native_source_uncertainty: None,
        native_observers: None,
    })
}

fn analysis_carriers_supported(proof: &SourceInvocationBinding) -> bool {
    proof.compiled_candidates.is_empty()
        && proof.compiled_named_candidates.is_empty()
        && proof.native_compilation_admission.is_none()
        && proof.native_compiler_admission.is_none()
        && proof.native_structured_preparation.is_none()
        && proof.native_switch_preparation.is_none()
        && proof.native_operand_layout.is_none()
        && proof.frozen_head_object.is_none()
        && proof.method_prefix_arguments.is_empty()
        && !proof.original_argument_completion.is_normal()
        && proof.normal_result_observations.is_none()
        && proof.rhs_read_store_observations.is_none()
        && proof.original_variable_operands.is_none()
        && proof.object_callback_effects.is_none()
        && proof.conditional_expression_evaluations.is_none()
}

fn normalize_analysis_tokens(
    tokens: &mut crate::ir::CommandTokens,
    scope: BodyProofScope<'_>,
    origin: &Arc<SourceOriginId>,
    relocation: &crate::var_resolve::VariableProofRelocation,
) {
    super::body_template::normalize_native_tokens(tokens, scope, origin, relocation);
    for proof in tokens
        .source_binding
        .iter_mut()
        .chain(tokens.nested_bindings.iter_mut().map(|(_, proof)| proof))
    {
        // Exact conditional lookup, source vectors and complete metadata are
        // retained in the private dependency value before these issuers move.
        proof.declaration_layout_observations = None;
        proof.declaration_flow_inventory = None;
        proof.original_logical_declaration_targets = None;
        proof.original_written_words = None;
        proof.original_compiler_words = None;
        proof.original_native_source_command = None;
    }
}

fn collect_native_source_registry_dependencies(
    tokens: &crate::ir::CommandTokens,
    scope: BodyProofScope<'_>,
    bindings: &ModuleCommandBindings,
    module: &crate::ir::Module,
    body: crate::ir::ProcedureId,
    registry: &CommandRegistry,
    dependencies: &mut std::collections::BTreeMap<u32, AnalyticalCommandDependency>,
    relocation: &mut crate::var_resolve::VariableProofRelocation,
) -> Option<()> {
    let mut original = tokens.clone();
    crate::lattice_rebase::rebase_command_tokens(
        &mut original,
        i64::from(scope.original_body_offset),
    );
    let mut calls = vec![original.clone()];
    let children = crate::word_subst::checked_original_lifted_calls_with_metadata_context(
        &original,
        scope.config,
        registry,
        bindings
            .baseline
            .metadata_context
            .metadata_context(registry)??,
    )?;
    for child in children {
        calls.push(child.tokens?);
    }
    for (_, proof) in &original.nested_bindings {
        let (_, mut recorded) = proof.original_recorded_command()?;
        recorded.source_binding = Some(proof.clone());
        calls.push(recorded);
    }
    for call in calls {
        let dependency = if call
            .source_binding
            .as_ref()?
            .has_original_native_source_command()
        {
            AnalyticalCommandDependency::NativeSchema(original_native_source_schema_dependency(
                &call, scope, bindings, module, body, registry,
            )?)
        } else {
            AnalyticalCommandDependency::Positioned(original_native_source_registry_dependency(
                &call, scope, bindings, registry,
            )?)
        };
        let offset = dependency.offset();
        if dependencies
            .get(&offset)
            .is_some_and(|old| old != &dependency)
        {
            return None;
        }
        dependencies.insert(offset, dependency);
        super::body_template::collect_token_activations(relocation, &call);
        super::body_template::collect_body_offset(relocation, scope, offset);
    }
    Some(())
}

fn original_native_source_registry_dependency(
    tokens: &crate::ir::CommandTokens,
    scope: BodyProofScope<'_>,
    bindings: &ModuleCommandBindings,
    registry: &CommandRegistry,
) -> Option<OriginalRegistryDependency> {
    let proof = tokens.source_binding.as_ref()?;
    let site = proof.invocation_site()?;
    let end =
        u64::from(scope.executable_body_offset) + u64::try_from(scope.body_source.len()).ok()?;
    if !matches!(site.source.kind(), super::SourceOriginKind::Authored(image) if image.bytes() == scope.source.as_bytes())
        || site.offset < scope.executable_body_offset
        || end <= u64::from(site.offset)
    {
        return None;
    }
    if !analysis_carriers_supported(proof)
        || tokens.evaluated_body.is_some()
        || tokens.synthetic.is_some()
    {
        return None;
    }
    let metadata = proof
        .original_invocation_metadata_for_owner(
            tokens,
            &bindings.baseline.metadata_context,
            registry,
        )
        .ok()??;
    let advice = proof.original_conditional_operand_layout_advice(tokens, registry)?;
    if proof.original_source_advice_purpose(tokens, metadata, registry)?
        != super::OriginalInvocationSourcePurpose::AuthoredSimulation
        || advice.snapshot.state.baseline.as_ref() != bindings.baseline.as_ref()
        || advice.snapshot.state.source_step_observed()
        || advice.targets().iter().any(|target| {
            !target.registry_backed
                || !target.prepended.is_empty()
                || target.implementation_allocation.is_some()
                || target
                    .identity
                    .as_ref()
                    .is_some_and(|identity| identity.allocation.is_some())
        })
    {
        return None;
    }
    let mut assistance =
        crate::registry_invocation::original_registry_invocation_assistance_with_metadata_context(
            registry,
            Some(metadata),
            tokens,
        )?;
    assistance.unanimous_command_words()?;
    for shape in &mut assistance.candidates {
        shape.with_source_schema(registry, metadata, tokens, |schema| {
            let effects = schema.effects();
            use tcl_registry::world_effect::{CallbackKinds, WorldStateDomain};
            let callbacks = effects.callback().kinds;
            let trace_inventory_empty = proof.variable_context.traced.is_empty()
                && proof.variable_context.trace_registrations.is_empty()
                && proof.variable_context.untracked_traces.is_empty();
            let permitted = (callbacks == CallbackKinds::NONE
                || (callbacks == CallbackKinds::TRACE && trace_inventory_empty))
                && schema.semantics.body_execution.is_none()
                && schema.semantics.frame_effect.is_none()
                && schema.state_transitions().facts().is_empty()
                && effects.accesses().iter().all(|access| {
                    matches!(
                        access.domain,
                        WorldStateDomain::VariableStore
                            | WorldStateDomain::VariableTraces
                            | WorldStateDomain::InterpreterResult
                            | WorldStateDomain::CompletionState
                            | WorldStateDomain::LegacyExternal(
                                tcl_registry::side_effects::SideEffectTarget::ChannelIo
                                    | tcl_registry::side_effects::SideEffectTarget::FileIo
                                    | tcl_registry::side_effects::SideEffectTarget::LogIo
                            )
                    )
                });
            permitted.then_some(())
        })?;
        let mut effective = tokens.clone();
        effective.word_exprs.clone_from(&shape.effective.words);
        crate::lattice_rebase::rebase_command_tokens(
            &mut effective,
            -i64::from(scope.original_body_offset),
        );
        shape.effective.words = effective.word_exprs;
    }
    Some(OriginalRegistryDependency {
        offset: proof
            .invocation_site()?
            .offset
            .checked_sub(scope.original_body_offset)?,
        namespace: advice.namespace().clone(),
        dialect: advice.dialect(),
        realm: advice.realm(),
        targets: advice.targets().to_vec(),
        shapes: assistance.candidates,
        native_observers: Some(Arc::clone(&advice.snapshot.state.command_observers)),
        native_source_uncertainty: Some((
            advice.unknown,
            advice.may_be_absent,
            advice.has_opaque_handler_alternatives(),
            proof.variable_context.dynamic_traces,
        )),
    })
}

fn original_native_source_schema_dependency(
    tokens: &crate::ir::CommandTokens,
    scope: BodyProofScope<'_>,
    bindings: &ModuleCommandBindings,
    module: &crate::ir::Module,
    body: crate::ir::ProcedureId,
    registry: &CommandRegistry,
) -> Option<NativeSourceSchemaDependency> {
    let proof = tokens.source_binding.as_ref()?;
    if !proof.original_native_source_owner_for_body(tokens, module, body, registry)
        || !analysis_carriers_supported(proof)
        || tokens.evaluated_body.is_some()
        || tokens.synthetic.is_some()
    {
        return None;
    }
    let input = module.source_metadata_input.as_ref()?;
    let words = proof.original_native_source_words(tokens, input, registry)?;
    let crate::registry_invocation::source_structure::OriginalRegistrySource::SourceTransitions(
        advice,
    ) = words.source()
    else {
        return None;
    };
    let offset = advice.site().offset;
    let end =
        u64::from(scope.executable_body_offset) + u64::try_from(scope.body_source.len()).ok()?;
    if advice.site().source.source_image() != &module.source
        || offset < scope.executable_body_offset
        || u64::from(offset) >= end
        || !advice.lineage().is_empty()
        || !advice.uncertain_operations().is_empty()
        || advice
            .original_words()
            .iter()
            .any(|word| word.group().expand)
        || words.origins().iter().enumerate().any(|(index, origin)| {
            *origin != crate::registry_invocation::InvocationWordOrigin::Written(index)
        })
    {
        return None;
    }
    words.with_source_schema(input.borrowed_context_registry(), |schema| {
        use tcl_registry::world_effect::{CallbackKinds, WorldStateDomain};
        let effects = schema.effects();
        let callbacks = effects.callback().kinds;
        let trace_inventory_empty = proof.variable_context.traced.is_empty()
            && proof.variable_context.trace_registrations.is_empty()
            && proof.variable_context.untracked_traces.is_empty();
        ((callbacks == CallbackKinds::NONE
            || (callbacks == CallbackKinds::TRACE && trace_inventory_empty))
            && schema.semantics.body_execution.is_none()
            && schema.semantics.frame_effect.is_none()
            && schema.state_transitions().facts().is_empty()
            && effects.accesses().iter().all(|access| {
                matches!(
                    access.domain,
                    WorldStateDomain::VariableStore
                        | WorldStateDomain::VariableTraces
                        | WorldStateDomain::InterpreterResult
                        | WorldStateDomain::CompletionState
                        | WorldStateDomain::LegacyExternal(
                            tcl_registry::side_effects::SideEffectTarget::ChannelIo
                                | tcl_registry::side_effects::SideEffectTarget::FileIo
                                | tcl_registry::side_effects::SideEffectTarget::LogIo
                        )
                )
            }))
        .then_some(())
    })??;
    Some(NativeSourceSchemaDependency {
        offset: offset.checked_sub(scope.original_body_offset)?,
        command: words.command().to_owned(),
        dialect: words.dialect()?,
        arguments: words.arguments().to_vec(),
        origins: words.origins().to_vec(),
        roles: words.roles()?.to_vec(),
        namespace: advice.conditional_c_source_namespace().cloned(),
        path: advice.conditional_c_source_path().map(<[_]>::to_vec),
        obligations: advice.obligations().to_vec(),
        observers: Arc::clone(&bindings.command_observers),
        source_step_observed: bindings.source_step_observed(),
    })
}
