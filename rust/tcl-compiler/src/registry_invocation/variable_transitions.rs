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

//! Variable-side transitions an invocation may make: namespace directives, aliases and traces.

use super::effective_words::{effective_words_for_target, frozen_argument_words};
use super::handler_invocation::{
    resolved_handler_invocation, resolved_handler_invocation_with_metadata_context,
};
use super::metadata_context::InvocationMetadataContext;
use super::word_resolution::{
    EffectiveInvocationWord, RegistryInvocationResolution,
    resolve_registry_words_in_realm_with_metadata_context,
};
use super::{PossibleVariableAliasTransitions, ResolvedStatementInvocation, VariableNameOperands};
use crate::ir::CommandTokens;
use tcl_registry::model::semantic::SemanticContext;
use tcl_registry::{CommandRegistry, InvocationFacts, InvocationWord};
use tcl_source::words::WordExpr;

/// Namespace import/export directives of one converged handler and frozen argv.
/// This metadata supplies neither a physical mutation nor an execution,
/// compiler, or successful-import proof.
#[derive(Debug)]
pub struct NamespaceDirectiveFootprint {
    transitions: Vec<tcl_registry::NamespaceTransition>,
}

impl NamespaceDirectiveFootprint {
    /// Authored directives, retaining unknown operand values and namespace axes.
    pub fn transitions(&self) -> impl Iterator<Item = &tcl_registry::NamespaceTransition> {
        self.transitions.iter()
    }
}

/// Recover directive metadata independently of native opcode eligibility.
/// A replaced or uncertain handler cannot donate its written command grammar.
#[must_use]
pub fn namespace_directive_footprint(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    tokens: &CommandTokens,
) -> Option<NamespaceDirectiveFootprint> {
    namespace_directive_footprint_for_invocation(&resolved_handler_invocation(
        registry, context, tokens,
    )?)
}

/// Namespace directive candidates under the actual supplied metadata owner.
/// Missing or foreign availability refuses; this grants no namespace effect.
pub(crate) fn namespace_directive_footprint_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
) -> Option<NamespaceDirectiveFootprint> {
    let context = context.filter(|context| context.matches_registry(registry))?;
    namespace_directive_footprint_for_invocation(
        &resolved_handler_invocation_with_metadata_context(registry, Some(context), tokens)?,
    )
}

fn namespace_directive_footprint_for_invocation(
    invocation: &ResolvedStatementInvocation,
) -> Option<NamespaceDirectiveFootprint> {
    if invocation.facts.arity_accepts_frozen_arguments() != Some(true) {
        return None;
    }
    let transitions = invocation
        .facts
        .state_transitions
        .declared()?
        .facts()
        .iter()
        .filter_map(|fact| match &fact.transition {
            tcl_registry::StateTransition::Namespace(
                transition @ (tcl_registry::NamespaceTransition::Import { .. }
                | tcl_registry::NamespaceTransition::Export { .. }),
            ) => Some(transition.clone()),
            _ => None,
        })
        .collect();
    Some(NamespaceDirectiveFootprint { transitions })
}

impl PossibleVariableAliasTransitions {
    /// Candidate declarations whose activation policy does not rule them out.
    pub fn aliases(
        &self,
    ) -> impl Iterator<Item = &tcl_registry::state_transition::VariableCellAliasTransition> {
        self.aliases.iter()
    }

    /// Additional runtime handlers or operand layouts remain unenumerated.
    #[must_use]
    pub const fn unknown_residual(&self) -> bool {
        self.unknown_residual
    }
}

/// Project possible outward alias exposure from exact source candidate tokens.
/// Unknown receiver namespaces retain their lookup residual; catalogue names
/// cannot create candidates, and compiler selection grants no declaration edge.
#[must_use]
pub fn possible_variable_alias_transitions(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    tokens: &CommandTokens,
) -> Option<PossibleVariableAliasTransitions> {
    possible_variable_alias_transitions_with_metadata_context(
        registry,
        context.map(InvocationMetadataContext::from),
        tokens,
    )
}

/// Candidate alias declarations under complete supplied source availability.
/// This grants exposure advice, never a completed link or physical cell.
#[must_use]
pub fn possible_variable_alias_transitions_in_context(
    context: &tcl_registry::model::ContextRegistry,
    tokens: &CommandTokens,
) -> Option<PossibleVariableAliasTransitions> {
    possible_variable_alias_transitions_with_metadata_context(
        context.commands(),
        Some(context.into()),
        tokens,
    )
}

pub(crate) fn possible_variable_alias_transitions_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
) -> Option<PossibleVariableAliasTransitions> {
    if context.is_some_and(|context| !context.matches_registry(registry)) {
        return None;
    }
    if tokens.synthetic.is_some() {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    let dialect = binding.variable_context.invocation_dialect;
    let mut footprint = PossibleVariableAliasTransitions {
        aliases: Vec::new(),
        unknown_residual: binding.execution_is_unknown() || binding.execution_may_be_absent(),
    };
    for target in binding
        .execution_targets()
        .chain(binding.proved_handler_target())
    {
        if !target.registry_backed {
            footprint.unknown_residual = true;
            continue;
        }
        let effective = effective_words_for_target(tokens, target)?;
        let values = frozen_argument_words(tokens, &effective);
        let mut words = vec![InvocationWord::Literal(&target.command)];
        words.extend(values.iter().map(EffectiveInvocationWord::as_registry_word));
        let Ok(RegistryInvocationResolution::Resolved(facts)) =
            resolve_registry_words_in_realm_with_metadata_context(
                registry,
                context,
                &words,
                dialect,
                binding.invocation_realm().unwrap_or_default(),
            )
        else {
            footprint.unknown_residual = true;
            continue;
        };
        for fact in facts
            .state_transitions
            .declared()
            .into_iter()
            .flat_map(tcl_registry::StateTransitions::facts)
        {
            let tcl_registry::StateTransition::VariableCellAlias(alias) = &fact.transition else {
                continue;
            };
            if alias
                .destination
                .is_active_in_frame_with_policy(binding.variable_context.alias_frame(), dialect)
                != Some(false)
                && !footprint.aliases.contains(alias)
            {
                footprint.aliases.push(alias.clone());
            }
        }
    }
    Some(footprint)
}

/// Possible variable-observer registrations from retained handler candidates.
/// This is a May exposure footprint, never a completed trace-table mutation.
#[derive(Debug)]
pub struct PossibleVariableTraceTransitions {
    traces: Vec<tcl_registry::TraceTransition>,
    pub(super) unknown_residual: bool,
    metadata_unavailable: bool,
}

impl PossibleVariableTraceTransitions {
    /// Candidate observer transitions with their frozen variable operands.
    pub fn transitions(&self) -> impl Iterator<Item = &tcl_registry::TraceTransition> {
        self.traces.iter()
    }

    /// Additional handlers or operand layouts remain unenumerated.
    #[must_use]
    pub const fn unknown_residual(&self) -> bool {
        self.unknown_residual
    }

    /// An original candidate has no descriptor in the supplied generation.
    /// The observer inventory cannot be closed from a catalogue fallback.
    #[must_use]
    pub const fn metadata_unavailable(&self) -> bool {
        self.metadata_unavailable
    }
}

/// Retain observer hazards even when receiver lookup cannot prove one handler.
/// Only original candidate identities can supply this metadata.
#[must_use]
pub fn possible_variable_trace_transitions(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    tokens: &CommandTokens,
) -> Option<PossibleVariableTraceTransitions> {
    possible_variable_trace_transitions_with_metadata_context(
        registry,
        context.map(InvocationMetadataContext::from),
        tokens,
    )
}

/// May observer transitions under the complete supplied availability generation.
/// This does not establish a completed registration or physical observer table.
#[must_use]
pub fn possible_variable_trace_transitions_in_context(
    context: &tcl_registry::model::ContextRegistry,
    tokens: &CommandTokens,
) -> Option<PossibleVariableTraceTransitions> {
    possible_variable_trace_transitions_with_metadata_context(
        context.commands(),
        Some(context.into()),
        tokens,
    )
}

pub(crate) fn possible_variable_trace_transitions_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
) -> Option<PossibleVariableTraceTransitions> {
    if context.is_some_and(|context| !context.matches_registry(registry)) {
        return None;
    }
    if tokens.synthetic.is_some() {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    let mut footprint = PossibleVariableTraceTransitions {
        traces: Vec::new(),
        unknown_residual: binding.execution_is_unknown() || binding.execution_may_be_absent(),
        metadata_unavailable: false,
    };
    for target in binding
        .execution_targets()
        .chain(binding.proved_handler_target())
    {
        if !target.registry_backed {
            footprint.unknown_residual = true;
            continue;
        }
        let realm = binding.invocation_realm().unwrap_or_default();
        if context.is_some_and(|context| {
            context
                .context()
                .resolve_spec_in_realm(registry, &target.command, realm)
                .is_none()
        }) {
            footprint.metadata_unavailable = true;
        }
        let effective = effective_words_for_target(tokens, target)?;
        let values = frozen_argument_words(tokens, &effective);
        let mut words = vec![InvocationWord::Literal(&target.command)];
        words.extend(values.iter().map(EffectiveInvocationWord::as_registry_word));
        let Ok(RegistryInvocationResolution::Resolved(facts)) =
            resolve_registry_words_in_realm_with_metadata_context(
                registry,
                context,
                &words,
                binding.variable_context.invocation_dialect,
                binding.invocation_realm().unwrap_or_default(),
            )
        else {
            footprint.unknown_residual = true;
            continue;
        };
        for fact in facts
            .state_transitions
            .declared()
            .into_iter()
            .flat_map(tcl_registry::StateTransitions::facts)
        {
            let tcl_registry::StateTransition::Trace(trace) = &fact.transition else {
                continue;
            };
            if matches!(
                trace,
                tcl_registry::TraceTransition::Add {
                    target: tcl_registry::TraceTarget::Variable(_),
                    ..
                } | tcl_registry::TraceTransition::Remove {
                    target: tcl_registry::TraceTarget::Variable(_),
                    ..
                }
            ) && !footprint.traces.contains(trace)
            {
                footprint.traces.push(trace.clone());
            }
        }
    }
    Some(footprint)
}

/// Possible name accesses of retained execution candidates.
/// These are hazard dependencies only: neither a physical store nor normal
/// continuation, value, compiler eligibility or purity is established.
#[derive(Debug)]
pub struct PossibleVariableNameOperands {
    pub(super) candidates: Vec<VariableNameOperands>,
    pub(super) unknown_residual: bool,
}

impl PossibleVariableNameOperands {
    /// Frozen candidate operands in their actual effective argument positions.
    pub fn operands(
        &self,
    ) -> impl Iterator<Item = (tcl_registry::ArgRole, &EffectiveInvocationWord)> {
        self.candidates
            .iter()
            .flat_map(VariableNameOperands::operands)
    }

    /// Possible contents mutations, excluding proved binding-only operands of
    /// each candidate. This grants no physical write or normal completion.
    pub fn contents_write_operands(&self) -> impl Iterator<Item = &EffectiveInvocationWord> {
        self.candidates
            .iter()
            .flat_map(VariableNameOperands::contents_write_operands)
    }

    /// Possible operand lookups with their authored handler phase. The phase
    /// chooses a context for a hazard query; it cannot establish a store.
    pub fn phased_operands(
        &self,
    ) -> impl Iterator<
        Item = (
            tcl_registry::ArgRole,
            &EffectiveInvocationWord,
            tcl_registry::native_compilation::VariableOperandBindingPhase,
            bool,
        ),
    > {
        self.candidates.iter().flat_map(|candidate| {
            candidate
                .operands()
                .map(|(role, word)| (role, word, candidate.binding_phase, candidate.destroys))
        })
    }

    /// Possible role classes whose exact operand layout remains unresolved.
    pub fn unresolved_roles(&self) -> impl Iterator<Item = tcl_registry::ArgRole> + '_ {
        self.candidates
            .iter()
            .flat_map(|candidate| candidate.unresolved_roles.iter().copied())
    }

    /// Unresolved role classes paired with that candidate's destruction effect.
    /// This preserves both write and destruction hazards when selected handlers
    /// disagree; it grants no operand position or reached mutation proof.
    pub fn phased_unresolved_roles(
        &self,
    ) -> impl Iterator<Item = (tcl_registry::ArgRole, bool)> + '_ {
        self.candidates.iter().flat_map(|candidate| {
            candidate
                .unresolved_roles
                .iter()
                .map(|role| (*role, candidate.destroys))
        })
    }

    /// At least one retained candidate may destroy its selected variable.
    #[must_use]
    pub fn destroys(&self) -> bool {
        self.candidates.iter().any(VariableNameOperands::destroys)
    }

    /// Additional handlers or accesses cannot be enumerated.
    #[must_use]
    pub const fn unknown_residual(&self) -> bool {
        self.unknown_residual
    }
}

/// Union May name-access hazards from authoritative candidate identities.
/// Written command spelling and catalogue declarations cannot donate candidates.
#[must_use]
pub fn possible_variable_name_operands(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    tokens: &CommandTokens,
) -> Option<PossibleVariableNameOperands> {
    possible_variable_name_operands_with_metadata_context(
        registry,
        context.map(InvocationMetadataContext::from),
        tokens,
    )
}

/// Resolve retained possible variable name operands under the complete supplied availability generation.
/// Existing original identity, argument and handler premises remain required.
#[must_use]
pub fn possible_variable_name_operands_in_context(
    context: &tcl_registry::model::ContextRegistry,
    tokens: &CommandTokens,
) -> Option<PossibleVariableNameOperands> {
    possible_variable_name_operands_with_metadata_context(
        context.commands(),
        Some(context.into()),
        tokens,
    )
}

pub(crate) fn possible_variable_name_operands_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
) -> Option<PossibleVariableNameOperands> {
    if context.is_some_and(|context| !context.matches_registry(registry)) {
        return None;
    }

    if !matches!(
        tokens.synthetic,
        None | Some(
            crate::ir::SyntheticMarker::EvaluatedWrapper
                | crate::ir::SyntheticMarker::CapturedCatchOutputs
        )
    ) {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    let dialect = binding.variable_context.invocation_dialect;
    let mut footprint = PossibleVariableNameOperands {
        candidates: Vec::new(),
        unknown_residual: binding.execution_is_unknown() || binding.execution_may_be_absent(),
    };
    for target in binding
        .execution_targets()
        .chain(binding.proved_handler_target())
    {
        let candidate = target
            .registry_backed
            .then(|| {
                let effective = effective_words_for_target(tokens, target)?;
                let values = frozen_argument_words(tokens, &effective);
                let expansion_at = values.iter().position(|word| {
                    matches!(
                        word,
                        EffectiveInvocationWord::Expanded
                            | EffectiveInvocationWord::KnownExpansion(_) | EffectiveInvocationWord::KnownByteExpansion(_)
                    )
                });
                let mut words = vec![InvocationWord::Literal(&target.command)];
                words.extend(values.iter().map(EffectiveInvocationWord::as_registry_word));
                let Ok(RegistryInvocationResolution::Resolved(facts)) =
                    resolve_registry_words_in_realm_with_metadata_context(
                        registry,
                        context,
                        &words,
                        dialect,
                        binding.invocation_realm().unwrap_or_default(),
                    )
                else {
                    return None;
                };
                if facts.argument_offset > values.len()
                    || facts.arity_accepts_frozen_arguments() == Some(false)
                {
                    return None;
                }
                Some(VariableNameOperands {
                    binding_phase: facts.successful_handler.map_or(
                        tcl_registry::native_compilation::VariableOperandBindingPhase::AfterArguments,
                        tcl_registry::native_compilation::SuccessfulHandlerSpec::variable_binding_phase,
                    ),
                    operands: facts
                        .arg_roles
                        .iter()
                        .filter_map(|&(index, role)| {
                            matches!(
                                role,
                                tcl_registry::ArgRole::VarRead | tcl_registry::ArgRole::VarWrite
                            )
                            .then(|| {
                                let argument = facts.argument_offset + usize::from(index);
                                possible_name_operand_word(&values, expansion_at, argument)
                                    .map(|word| (role, word))
                            })
                            .flatten()
                        })
                        .collect(),
                    unresolved_roles: if facts.arg_roles_complete {
                        &[]
                    } else {
                        facts.arg_role_resolver_roles
                    },
                    binding_only_names: binding_only_operand_names(&facts),
                    destroys: facts
                        .traits
                        .contains(tcl_registry::Traits::DESTROYS_VARIABLE),
                })
            })
            .flatten();
        if let Some(candidate) = candidate {
            footprint.candidates.push(candidate);
        } else {
            footprint.unknown_residual = true;
        }
    }
    Some(footprint)
}

fn possible_name_operand_word(
    values: &[EffectiveInvocationWord],
    expansion_at: Option<usize>,
    argument: usize,
) -> Option<EffectiveInvocationWord> {
    // Expansion can move or supply later argv values. Earlier captured name
    // values remain fixed; neither branch grants a normal physical store.
    if expansion_at.is_some_and(|start| argument >= start) {
        Some(EffectiveInvocationWord::Dynamic)
    } else {
        values.get(argument).cloned()
    }
}

impl VariableNameOperands {
    /// Frozen evaluated name shapes, without a physical-store licence.
    pub fn operands(
        &self,
    ) -> impl Iterator<Item = (tcl_registry::ArgRole, &EffectiveInvocationWord)> {
        self.operands.iter().map(|(role, word)| (*role, word))
    }

    /// Candidate value/presence mutations, separately from alias or observer
    /// registration roles. Unknown names remain possible mutations.
    pub fn contents_write_operands(&self) -> impl Iterator<Item = &EffectiveInvocationWord> {
        self.operands.iter().filter_map(|(role, word)| {
            if *role != tcl_registry::ArgRole::VarWrite
                || matches!(word, EffectiveInvocationWord::Literal(name) if self.binding_only_names.contains(name))
            {
                None
            } else {
                Some(word)
            }
        })
    }

    /// Role classes whose exact argument positions remain unresolved.
    /// These denote possible accesses; they do not create definitions.
    #[must_use]
    pub const fn unresolved_roles(&self) -> &'static [tcl_registry::ArgRole] {
        self.unresolved_roles
    }

    /// The authored handler destroys the selected variable rather than stores it.
    #[must_use]
    pub const fn destroys(&self) -> bool {
        self.destroys
    }
}

fn binding_only_operand_names(facts: &InvocationFacts) -> std::collections::HashSet<String> {
    facts
        .state_transitions
        .declared()
        .into_iter()
        .flat_map(tcl_registry::StateTransitions::facts)
        .filter_map(|fact| match &fact.transition {
            tcl_registry::StateTransition::VariableCellAlias(alias) => alias.local.literal(),
            tcl_registry::StateTransition::Trace(
                tcl_registry::TraceTransition::Add {
                    target: tcl_registry::TraceTarget::Variable(target),
                    ..
                }
                | tcl_registry::TraceTransition::Remove {
                    target: tcl_registry::TraceTarget::Variable(target),
                    ..
                },
            ) => target.literal(),
            _ => None,
        })
        .filter(|name| !crate::variable_bindings::contents_write_operand(facts, Some(name)))
        .map(str::to_owned)
        .collect()
}

/// Name-uncertainty projection independent of successful physical transfer.
#[must_use]
pub fn variable_name_operands(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    tokens: &CommandTokens,
) -> Option<VariableNameOperands> {
    if tokens.synthetic.is_some() {
        return None;
    }
    let invocation = resolved_handler_invocation(registry, context, tokens)?;
    if invocation
        .effective
        .words
        .iter()
        .any(|word| matches!(word, WordExpr::Expand { .. }))
    {
        return None;
    }
    let count = invocation.effective.words.len().checked_sub(1)?;
    if invocation.facts.argument_offset > count
        || invocation.facts.arity_accepts_frozen_arguments() != Some(true)
    {
        return None;
    }
    let operands = invocation
        .facts
        .arg_roles
        .iter()
        .filter(|&&(_, role)| {
            matches!(
                role,
                tcl_registry::ArgRole::VarRead | tcl_registry::ArgRole::VarWrite
            )
        })
        .map(|&(index, role)| {
            (
                role,
                invocation.argument_word(invocation.facts.argument_offset + usize::from(index)),
            )
        })
        .collect();
    Some(VariableNameOperands {
        binding_phase: invocation.facts.successful_handler.map_or(
            tcl_registry::native_compilation::VariableOperandBindingPhase::AfterArguments,
            tcl_registry::native_compilation::SuccessfulHandlerSpec::variable_binding_phase,
        ),
        operands,
        unresolved_roles: if invocation.facts.arg_roles_complete {
            &[]
        } else {
            invocation.facts.arg_role_resolver_roles
        },
        binding_only_names: binding_only_operand_names(&invocation.facts),
        destroys: invocation
            .facts
            .traits
            .contains(tcl_registry::Traits::DESTROYS_VARIABLE),
    })
}
