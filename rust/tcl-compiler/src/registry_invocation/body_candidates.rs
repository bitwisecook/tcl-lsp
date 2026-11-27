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

//! Preparing and publishing the body candidates of an invocation and their parameter advice.

use super::effective_words::{
    effective_words_for_target, frozen_argument_values, frozen_argument_words,
};
use super::handler_invocation::{
    resolved_declared_assistance, resolved_handler_invocation_with_metadata_context,
};
use super::invocation_assistance::{
    catalogue_invocation_assistance, mapped_written_argument_roles,
};
use super::metadata_context::InvocationMetadataContext;
use super::native_compilation::native_compiler_replay_source;
use super::source_model_operations::resolve_original_declared_layout;
use super::word_resolution::{
    RegistryInvocationResolution, resolve_effective_tokens_with_metadata_context,
};
use super::{
    BodyParameterSourceInvocation, ConditionalBodyInvocation, EffectiveCommandWords,
    InvocationBodyAssistance, InvocationWordOrigin, ParameterRoleAdvice, ReceiverTraitInvocation,
    RegistryInvocationShape, ResolvedStatementInvocation,
};
use crate::ir::CommandTokens;
use tcl_registry::model::semantic::SemanticContext;
use tcl_registry::{CommandRegistry, InvocationFacts};

/// Heap-owned only during this original invocation's preparation. Different
/// May and exact-actual dialect inputs retain independent selected facts.
pub(super) struct PreparedBodyCandidate {
    pub(super) effective: EffectiveCommandWords,
    possible_facts: Box<InvocationFacts>,
    exact_facts: Option<Box<InvocationFacts>>,
    same_dialect: bool,
    selected_execution: bool,
}

impl PreparedBodyCandidate {
    pub(super) fn exact_facts(&self) -> Option<&InvocationFacts> {
        if self.same_dialect {
            Some(&self.possible_facts)
        } else {
            self.exact_facts.as_deref()
        }
    }
}

pub(super) struct PreparedBodyCandidates {
    pub(super) candidates: Vec<PreparedBodyCandidate>,
    pub(super) unknown_residual: bool,
    pub(super) may_be_absent: bool,
}

#[inline(never)]
pub(super) fn prepare_body_candidates<'a>(
    registry: &CommandRegistry,
    context: impl Into<InvocationMetadataContext<'a>>,
    tokens: &CommandTokens,
) -> Option<Box<PreparedBodyCandidates>> {
    let context = context.into();
    if !context.matches_registry(registry) {
        return None;
    }
    if tokens.synthetic.is_some() {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    let actual = binding.variable_context.invocation_dialect;
    let possible = actual.or_else(|| {
        registry
            .profile()
            .map(tcl_registry::InvocationDialect::of_profile)
    });
    let mut prepared = Box::new(PreparedBodyCandidates {
        candidates: Vec::new(),
        unknown_residual: binding.execution_is_unknown(),
        may_be_absent: binding.execution_may_be_absent(),
    });
    for target in binding
        .execution_targets()
        .chain(binding.proved_handler_target())
    {
        if !target.registry_backed {
            prepared.unknown_residual = true;
            continue;
        }
        let effective = effective_words_for_target(tokens, target)?;
        let Ok(RegistryInvocationResolution::Resolved(possible_facts)) =
            resolve_effective_tokens_with_metadata_context(
                registry,
                Some(context),
                tokens,
                &effective,
                possible,
            )
        else {
            prepared.unknown_residual = true;
            continue;
        };
        let same_dialect = actual.is_some() && actual == possible;
        let exact_facts = if same_dialect || actual.is_none() {
            None
        } else if let Ok(RegistryInvocationResolution::Resolved(facts)) =
            resolve_effective_tokens_with_metadata_context(
                registry,
                Some(context),
                tokens,
                &effective,
                actual,
            )
        {
            Some(facts)
        } else {
            None
        };
        prepared.candidates.push(PreparedBodyCandidate {
            effective,
            possible_facts,
            exact_facts,
            same_dialect,
            selected_execution: binding.proved_execution_target() == Some(target),
        });
    }
    Some(prepared)
}

#[inline(never)]
pub(crate) fn invocation_body_assistance<'a>(
    registry: &CommandRegistry,
    context: impl Into<InvocationMetadataContext<'a>>,
    surface: &tcl_registry::model::DocumentCommandSurface<'_>,
    tokens: &CommandTokens,
) -> Box<InvocationBodyAssistance> {
    let context = context.into();
    body_assistance_with_entry(registry, context, surface, tokens, None)
}

#[inline(never)]
fn body_assistance_with_entry<'a>(
    registry: &CommandRegistry,
    context: impl Into<InvocationMetadataContext<'a>>,
    surface: &tcl_registry::model::DocumentCommandSurface<'_>,
    tokens: &CommandTokens,
    entry: Option<std::sync::Arc<crate::command_binding::SourceConditionalBodyEntry>>,
) -> Box<InvocationBodyAssistance> {
    let context = context.into();
    let prepared = prepare_body_candidates(registry, context, tokens);
    let mut result = Box::new(InvocationBodyAssistance {
        caller_template_source: None,
        possible_roles: Vec::new(),
        definite_roles: Vec::new(),
        possible_traits: tcl_registry::Traits::empty(),
        definite_traits: tcl_registry::Traits::empty(),
        definite_scope: None,
        possible_definition_grammars: Vec::new(),
        possible_case_lists: Vec::new(),
        possible_operations: Vec::new(),
        definite_invocation: None,
        conditional_invocation: None,
        receiver_trait_invocation: None,
        logical_trait_invocation: None,
        logical_lambda_roles: None,
        body_parameter_source: None,
        declared_roles: Vec::new(),
        unknown_residual: prepared.as_ref().is_none_or(|view| view.unknown_residual),
        may_be_absent: prepared.as_ref().is_none_or(|view| view.may_be_absent),
        parameter_advice: ParameterRoleAdvice::default(),
    });
    result.parameter_advice.closed = !result.unknown_residual && !result.may_be_absent;
    publish_possible_body_context(
        registry,
        context,
        surface,
        tokens,
        prepared.as_deref(),
        &mut result,
    );
    if result.unknown_residual || result.may_be_absent || tokens.synthetic.is_some() {
        return result;
    }
    let prepared = prepared.expect("closed assistance has prepared candidates");
    publish_unanimous_body_context(registry, context, &prepared, &mut result);
    if let Some(entry) = entry {
        retain_conditional_body_invocation(
            entry,
            registry,
            context,
            tokens,
            &prepared,
            &mut result,
        );
    }
    result.definite_invocation = prepared
        .candidates
        .iter()
        .find(|candidate| candidate.selected_execution)
        .filter(|candidate| {
            candidate.exact_facts().is_some_and(|facts| {
                facts.arg_roles_complete && facts.arity_accepts_frozen_arguments() == Some(true)
            })
        })
        .and_then(|candidate| prepared_body_invocation(registry, context, tokens, candidate));
    result.retain_caller_template_source(tokens);
    result
}

fn publish_possible_body_context(
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    surface: &tcl_registry::model::DocumentCommandSurface<'_>,
    tokens: &CommandTokens,
    prepared: Option<&PreparedBodyCandidates>,
    result: &mut InvocationBodyAssistance,
) {
    if !context.matches_registry(registry) {
        return;
    }
    if let Some(prepared) = prepared {
        for candidate in &prepared.candidates {
            let facts = &candidate.possible_facts;
            let roles = mapped_written_argument_roles(
                &candidate.effective,
                facts.argument_offset,
                &facts.arg_roles,
            );
            result
                .parameter_advice
                .candidates
                .push(RegistryInvocationShape::from_facts(
                    facts,
                    candidate.effective.clone(),
                ));
            result.possible_roles.extend(roles);
            result.possible_traits |= facts.traits;
            collect_body_navigation(
                registry,
                context,
                tokens,
                (
                    &facts.canonical_command,
                    facts.operation,
                    &candidate.effective,
                ),
                result,
            );
        }
    }
    if let Some(catalogue) = catalogue_invocation_assistance(registry, context, tokens) {
        result
            .possible_roles
            .extend(catalogue.shape.written_argument_roles());
        result.possible_traits |= catalogue.shape.possible_traits;
        collect_body_navigation(
            registry,
            context,
            tokens,
            (
                &catalogue.shape.command,
                catalogue.shape.operation,
                &catalogue.shape.effective,
            ),
            result,
        );
    }
    if let Some(declared) = resolved_declared_assistance(surface, tokens) {
        result.declared_roles.clone_from(&declared.roles);
        result.possible_roles.extend(declared.roles);
    }
    let mut unique = Vec::new();
    for role in std::mem::take(&mut result.possible_roles) {
        if !unique.contains(&role) {
            unique.push(role);
        }
    }
    unique.sort_by_key(|(index, _)| *index);
    result.possible_roles = unique;
}

fn publish_unanimous_body_context(
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    prepared: &PreparedBodyCandidates,
    result: &mut InvocationBodyAssistance,
) {
    let mut roles: Option<Vec<(usize, tcl_registry::ArgRole)>> = None;
    let mut traits: Option<tcl_registry::Traits> = None;
    let mut scope: Option<Option<&'static tcl_registry::scoped::ScopedCommandEnv>> = None;
    for candidate in &prepared.candidates {
        let Some(facts) = candidate.exact_facts().filter(|facts| {
            facts.arg_roles_complete && facts.arity_accepts_frozen_arguments() == Some(true)
        }) else {
            return;
        };
        let mapped = mapped_written_argument_roles(
            &candidate.effective,
            facts.argument_offset,
            &facts.arg_roles,
        );
        if let Some(shared) = &mut roles {
            shared.retain(|role| mapped.contains(role));
        } else {
            roles = Some(mapped);
        }
        traits = Some(traits.map_or(facts.traits, |previous| previous.intersection(facts.traits)));
        let selected_scope = context
            .context()
            .resolve_spec(registry, &facts.canonical_command)
            .and_then(|spec| spec.body_scope);
        scope = Some(match scope {
            None => selected_scope,
            Some(Some(previous))
                if selected_scope.is_some_and(|next| std::ptr::eq(previous, next)) =>
            {
                Some(previous)
            }
            _ => None,
        });
    }
    result.definite_roles = roles.unwrap_or_default();
    result.definite_traits = traits.unwrap_or_default();
    result.definite_scope = scope.flatten();
}

pub(super) fn prepared_body_invocation<'a>(
    registry: &CommandRegistry,
    context: impl Into<InvocationMetadataContext<'a>>,
    tokens: &CommandTokens,
    candidate: &PreparedBodyCandidate,
) -> Option<ResolvedStatementInvocation> {
    let context = context.into();
    let effective = candidate.effective.clone();
    let dialect = tokens
        .source_binding
        .as_ref()
        .and_then(|binding| binding.variable_context.invocation_dialect)
        .or_else(|| {
            context
                .context()
                .environment
                .point()
                .map(tcl_registry::InvocationDialect::of_point)
        })
        .or_else(|| {
            registry
                .profile()
                .map(tcl_registry::InvocationDialect::of_profile)
        });
    Some(ResolvedStatementInvocation {
        dialect,
        facts: Box::new(candidate.exact_facts()?.clone()),
        arguments: effective.argument_presentations(tokens.argv_texts.get(1..)?)?,
        evaluated_arguments: frozen_argument_values(tokens, &effective),
        evaluated_words: frozen_argument_words(tokens, &effective),
        effective,
        authored_case_body_selection: None,
    })
}

fn collect_body_navigation(
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
    candidate: (
        &str,
        tcl_registry::SemanticOperationId,
        &EffectiveCommandWords,
    ),
    result: &mut InvocationBodyAssistance,
) {
    let (command, operation, effective) = candidate;
    let operation = (operation, effective.clone());
    if !result.possible_operations.contains(&operation) {
        result.possible_operations.push(operation);
    }
    if let Some(grammar) = context
        .context()
        .resolve_spec(registry, command)
        .and_then(|spec| spec.definition_body)
        && !result
            .possible_definition_grammars
            .iter()
            .any(|previous| std::ptr::eq(*previous, grammar))
    {
        result.possible_definition_grammars.push(grammar);
    }
    let Some(arguments) = effective.argument_spellings(&tokens.argv_texts[1..]) else {
        return;
    };
    let arguments: Vec<_> = arguments.iter().map(String::as_str).collect();
    // CaseInvocation indices are already relative to the complete post-head
    // argv supplied below, unlike selected member ArgRole indices.
    if let Some((spec, invocation)) = registry.case_invocation(
        command,
        &arguments,
        Some(context.context().authoring_query()),
    ) && let Some(index) = invocation
        .clause_list_index
        .and_then(|index| effective.written_argument(index))
    {
        result.possible_case_lists.push((spec, index));
    }
}

/// Use the retained execution point before the catalogue's separate authoring
/// view. The version-to-environment label is owned by the typed Tcl release;
/// an unknown point cannot manufacture a runtime or a default context.
pub(super) fn body_assistance_context(
    registry: &CommandRegistry,
    tokens: &CommandTokens,
) -> Option<SemanticContext> {
    let authoring = registry.profile().map(SemanticContext::for_profile);
    if let Some(packaged) =
        authoring.filter(|context| !context.context().authoring_query().packages.is_empty())
    {
        // Package placements are authoring availability, independently of the
        // actual engine passed to native facts and the retained invocation realm.
        return Some(packaged);
    }
    tokens
        .source_binding
        .as_ref()
        .and_then(|binding| binding.variable_context.invocation_dialect)
        .and_then(tcl_registry::InvocationDialect::execution_point)
        .and_then(|point| {
            point
                .tcl_version()
                .map(|version| SemanticContext::for_environment(version.dialect_profile_name()))
                .or_else(|| {
                    (point.family() == tcl_dialect::model::Family::Jim)
                        .then(|| SemanticContext::for_environment(point.family().name()))
                })
        })
        .or(authoring)
}

/// Reconstruct only original words in this source buffer, then attach the
/// caller's retained source inventory. The affine base is supplied by the
/// source owner; this helper never analyses a detached body as a fresh entry.
pub(crate) fn segmented_body_assistance(
    surface: &tcl_registry::model::DocumentCommandSurface<'_>,
    bindings: &crate::command_binding::SourceCommandBindings,
    source: &str,
    config: tcl_lexer::LexerConfig,
    command: &tcl_source::segment::SegmentedCommand,
    base: u32,
    metadata: Option<&tcl_registry::model::ContextRegistry>,
) -> Option<Box<InvocationBodyAssistance>> {
    let registry = surface.commands();
    let original = match bindings.source_origin()?.kind() {
        crate::command_binding::SourceOriginKind::InheritedPoint { source: text, .. }
        | crate::command_binding::SourceOriginKind::Authored(text)
        | crate::command_binding::SourceOriginKind::Loaded { source: text, .. }
        | crate::command_binding::SourceOriginKind::Derived { source: text, .. }
        | crate::command_binding::SourceOriginKind::ConditionalField { source: text, .. } => text,
    };
    let start = usize::try_from(base).ok()?;
    if original.get(start..start.checked_add(source.len())?) != Some(source.as_bytes()) {
        return None;
    }
    let mut tokens =
        CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(source), config, command);
    crate::lattice_rebase::rebase_command_tokens(&mut tokens, i64::from(base));
    bindings.stamp_original_tokens(&mut tokens);
    bindings.attach_declared_body_assistance(&mut tokens, surface);
    let context = match metadata {
        Some(context) => InvocationMetadataContext::from(context),
        None => body_assistance_context(registry, &tokens)?.into(),
    };
    if !context.matches_registry(registry) {
        return None;
    }
    let mut view = body_assistance_with_entry(registry, context, surface, &tokens, None);
    if let Some(owned) = owned_body_layout_invocation(registry, context, &tokens, bindings) {
        publish_conditional_parameter_advice(owned, &mut view);
    } else if let Some(owned) =
        receiver_source_trait_invocation(registry, context, &tokens, bindings)
    {
        view.parameter_advice = ParameterRoleAdvice {
            candidates: vec![RegistryInvocationShape::from_facts(
                &owned.invocation.facts,
                owned.invocation.effective.clone(),
            )],
            closed: true,
        };
        view.receiver_trait_invocation = Some(owned);
    } else {
        view.body_parameter_source =
            original_body_parameter_source_invocation(registry, context, &tokens, bindings, config);
    }
    view.retain_caller_template_source(&tokens);
    Some(view)
}

/// Parameter usage from an authentic original body/declaration row. The
/// dedicated source facade cannot supply generic closed or executable facts.
pub(super) fn original_body_parameter_source_invocation(
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
    bindings: &crate::command_binding::SourceCommandBindings,
    config: tcl_lexer::LexerConfig,
) -> Option<BodyParameterSourceInvocation> {
    context.matches_registry(registry).then_some(())?;
    let binding = tokens.source_binding.as_ref()?;
    let site = binding.invocation_site()?;
    (bindings.source_origin() == Some(&site.source)).then_some(())?;
    let entry = binding.original_body_parameter_source_scope(tokens)?;
    let layout = entry.layout();
    let metadata = layout
        .original_declaration_metadata_context(registry, tokens)
        .ok()??;
    if metadata.context() != context.context()
        || context
            .source_analysis_input()
            .is_some_and(|input| metadata.source_analysis_input() != Some(input))
    {
        return None;
    }
    let words = entry.original_words(tokens, config, registry)?;
    if words.iter().any(|word| word.group().expand) {
        return None;
    }
    let invocation = resolve_original_declared_layout(registry, metadata, tokens, layout)?;
    if !invocation.facts.arg_roles_complete
        || invocation.facts.arity_accepts_frozen_arguments() != Some(true)
        || invocation.effective.words.len() != words.len()
        || invocation.effective.origins.len() != words.len()
        || invocation.effective.origins.first() != Some(&InvocationWordOrigin::ResolvedHead)
        || invocation
            .effective
            .origins
            .iter()
            .enumerate()
            .skip(1)
            .any(|(ordinal, origin)| *origin != InvocationWordOrigin::Written(ordinal))
    {
        return None;
    }
    let shape =
        RegistryInvocationShape::from_facts(&invocation.facts, invocation.effective.clone());
    Some(BodyParameterSourceInvocation {
        entry,
        words,
        shape,
        invocation,
    })
}

/// Conditional traits from the complete original receiver declaration. Its
/// source namespace does not claim the future receiver's private namespace.
fn receiver_source_trait_invocation<'a>(
    registry: &CommandRegistry,
    context: impl Into<InvocationMetadataContext<'a>>,
    tokens: &CommandTokens,
    bindings: &crate::command_binding::SourceCommandBindings,
) -> Option<ReceiverTraitInvocation> {
    // naming.tcloo.original-declared-receiver-caller-traits
    // docs/design/analysis/name-resolution-proofs/tcloo-original-declared-receiver-caller-traits.md
    let binding = tokens.source_binding.as_ref()?;
    let site = binding.invocation_site()?;
    let entry = bindings.declared_receiver_body_entry_at(&site.source, site.offset)?;
    native_compiler_replay_source(tokens, site)?;
    let advice = binding.declaration_operand_layout_advice(tokens)?;
    #[cfg(debug_assertions)]
    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_CALLER_FRAME").is_some() {
        eprintln!(
            "ORIGINAL_RECEIVER_TRAITS offset={} closed={} original_closed={} opaque={} targets={}",
            site.offset,
            advice.closed_receiver_trait_lookup(&entry),
            advice.closed_lookup(),
            advice.has_opaque_handler_alternatives(),
            advice.targets().len()
        );
    }
    if !advice.closed_receiver_trait_lookup(&entry) {
        return None;
    }
    let invocation = resolve_original_declared_layout(registry, context, tokens, &advice)?;
    if !invocation.facts.arg_roles_complete
        || invocation.facts.arity_accepts_frozen_arguments() != Some(true)
    {
        return None;
    }
    Some(ReceiverTraitInvocation { entry, invocation })
}

/// One declaration's conditional handler layout. Compiled execution residuals
/// remain independent; they cannot erase a separately proved logical handler.
pub(super) fn owned_body_layout_invocation<'a>(
    registry: &CommandRegistry,
    context: impl Into<InvocationMetadataContext<'a>>,
    tokens: &CommandTokens,
    bindings: &crate::command_binding::SourceCommandBindings,
) -> Option<ConditionalBodyInvocation> {
    let context = context.into();
    if !context.matches_registry(registry) {
        return None;
    }
    let site = tokens.source_binding.as_ref()?.invocation_site()?;
    let entry = bindings.conditional_body_entry_at(&site.source, site.offset)?;
    native_compiler_replay_source(tokens, site)?;
    let mut conditional = tokens.clone();
    conditional.source_binding =
        Some(bindings.conditional_invocation_at_source(&entry, site.offset));
    if !entry.owns_invocation(conditional.source_binding.as_ref()?) {
        return None;
    }
    let invocation =
        resolved_handler_invocation_with_metadata_context(registry, Some(context), &conditional)
            .or_else(|| {
                let prepared = prepare_body_candidates(registry, context, &conditional)?;
                if prepared.unknown_residual || prepared.may_be_absent {
                    return None;
                }
                let mut unanimous = None;
                for candidate in &prepared.candidates {
                    let facts = candidate.exact_facts()?;
                    if !facts.arg_roles_complete
                        || facts.arity_accepts_frozen_arguments() != Some(true)
                    {
                        return None;
                    }
                    let selected =
                        prepared_body_invocation(registry, context, &conditional, candidate)?;
                    if unanimous
                        .as_ref()
                        .is_some_and(|previous| previous != &selected)
                    {
                        return None;
                    }
                    unanimous = Some(selected);
                }
                unanimous
            })?;
    if !invocation.facts.arg_roles_complete
        || invocation.facts.arity_accepts_frozen_arguments() != Some(true)
    {
        return None;
    }
    Some(ConditionalBodyInvocation { entry, invocation })
}

fn publish_conditional_parameter_advice(
    owned: ConditionalBodyInvocation,
    view: &mut InvocationBodyAssistance,
) {
    view.parameter_advice = ParameterRoleAdvice {
        candidates: vec![RegistryInvocationShape::from_facts(
            &owned.invocation.facts,
            owned.invocation.effective.clone(),
        )],
        closed: true,
    };
    view.conditional_invocation = Some(owned);
}

/// Build a purpose-restricted layout only from converged original candidates.
/// The declaration recipe is independent of entered-script frame observations.
fn retain_conditional_body_invocation(
    entry: std::sync::Arc<crate::command_binding::SourceConditionalBodyEntry>,
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
    prepared: &PreparedBodyCandidates,
    view: &mut InvocationBodyAssistance,
) {
    let Some(site) = tokens
        .source_binding
        .as_ref()
        .and_then(|binding| binding.invocation_site())
    else {
        return;
    };
    if !entry.owns_source(&site.source, site.offset)
        || !tokens
            .source_binding
            .as_ref()
            .is_some_and(|binding| entry.owns_invocation(binding))
    {
        return;
    }
    let mut unanimous = None;
    for candidate in &prepared.candidates {
        let Some(facts) = candidate.exact_facts() else {
            return;
        };
        if !facts.arg_roles_complete || facts.arity_accepts_frozen_arguments() != Some(true) {
            return;
        }
        let Some(selected) = prepared_body_invocation(registry, context, tokens, candidate) else {
            return;
        };
        if unanimous
            .as_ref()
            .is_some_and(|previous| previous != &selected)
        {
            return;
        }
        unanimous = Some(selected);
    }
    if let Some(invocation) = unanimous {
        publish_conditional_parameter_advice(ConditionalBodyInvocation { entry, invocation }, view);
    }
}
