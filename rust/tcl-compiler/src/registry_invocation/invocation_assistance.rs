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

//! The registry shape of an invocation and the argument, value and catalogue assistance on it.

use super::body_candidates::body_assistance_context;
use super::effective_words::{
    compose_effective_words, compose_original_effective_words, effective_words_for_target,
    frozen_argument_words, source_effective_words_for_target,
};
use super::handler_invocation::resolved_declared_assistance;
use super::metadata_context::InvocationMetadataContext;
use super::source_taint::original_hosted_source_registry_metadata;
use super::structured_invocation::original_declared_structured_invocation;
use super::word_resolution::{
    EffectiveInvocationWord, RegistryInvocationResolution, effective_invocation_word,
    invocation_word_with_source, resolve_effective_tokens_with_metadata_context,
    resolve_registry_words_in_realm,
};
use super::{
    EffectiveCommandWords, InvocationWordOrigin, RegistryInvocationShape, source_structure,
};
use crate::ir::{CommandTokens, WordExpr};
use tcl_registry::model::semantic::SemanticContext;
use tcl_registry::{CommandRegistry, InvocationFacts, InvocationWords};
use tcl_syntax::word_rules::WordValueRules;

impl RegistryInvocationShape {
    /// Possible authored body evaluation within this shape. This establishes
    /// no entered frame, callback execution or closed invocation alternatives.
    pub(crate) fn may_execute_parameter_body(&self) -> bool {
        self.possible_traits
            .contains(tcl_registry::Traits::PERFORMS_SUBSTITUTION)
            || self.roles.iter().any(|(_, role)| {
                matches!(
                    role,
                    tcl_registry::ArgRole::Body
                        | tcl_registry::ArgRole::LambdaLiteral
                        | tcl_registry::ArgRole::CommandPrefix
                )
            })
    }

    fn from_hosted_source(
        registry: &CommandRegistry,
        context: InvocationMetadataContext<'_>,
        tokens: &CommandTokens,
    ) -> Option<Self> {
        let metadata = original_hosted_source_registry_metadata(registry, context, tokens)?;
        if metadata
            .authored_barriers()
            .has_nominal_command_name_hazard()
        {
            return None;
        }
        let effective = compose_original_effective_words(tokens, metadata.shape().command(), &[])?;
        let mut shape = metadata
            .shape()
            .with_source_schema(registry, context, |schema| {
                Some(Self::from_facts(&schema.facts(), effective))
            })?;
        shape.hosted_source = Some(std::sync::Arc::new(metadata));
        Some(shape)
    }

    pub(super) fn from_facts(facts: &InvocationFacts, effective: EffectiveCommandWords) -> Self {
        Self {
            hosted_source: None,
            command: facts.canonical_command.clone(),
            operation: facts.operation,
            data_collection: facts.data_collection,
            side_switch_target: facts.side_switch_target,
            option_keywords: facts.option_keywords.clone(),
            irules_source_flow_action: facts.irules_source_flow_action,
            source_may_commit_response: facts.source_may_commit_response,
            roles: facts.arg_roles.clone(),
            argument_offset: facts.argument_offset,
            roles_complete: facts.arg_roles_complete,
            possible_traits: facts.traits,
            nominal_return_type: facts.return_type,
            effective,
        }
    }

    /// Query conditional source descriptors at this same retained original point.
    /// The selected schema supplies syntax and warning metadata only, never a
    /// live handler, effects, completed reads or Native compilation authority.
    pub(crate) fn with_source_schema<T>(
        &self,
        registry: &CommandRegistry,
        context: InvocationMetadataContext<'_>,
        tokens: &CommandTokens,
        apply: impl FnOnce(&tcl_registry::ResolvedInvocation<'_, '_>) -> Option<T>,
    ) -> Option<T> {
        if let Some(original) = &self.hosted_source {
            if Self::from_hosted_source(registry, context, tokens).as_ref() != Some(self) {
                return None;
            }
            return original
                .shape()
                .with_source_schema(registry, context, apply);
        }
        self.with_source_schema_alternatives(registry, context, tokens, true, apply)
    }

    /// Readonly values under the shape's retained source purpose. Hosted
    /// literals use their own protocol rather than an evaluated Native argv.
    pub(crate) fn source_argument_words(
        &self,
        registry: &CommandRegistry,
        context: Option<InvocationMetadataContext<'_>>,
        tokens: &CommandTokens,
    ) -> Option<Vec<EffectiveInvocationWord>> {
        let Some(_) = &self.hosted_source else {
            return Some(frozen_argument_words(tokens, &self.effective));
        };
        let context = context?;
        let current = Self::from_hosted_source(registry, context, tokens)?;
        if &current != self {
            return None;
        }
        let input = context.source_analysis_input()?;
        let words = source_structure::original_vendor_registry_words(
            input.borrowed_context_registry(),
            tokens,
            current.hosted_source.as_deref()?.clone(),
        )?;
        Some(words.arguments().to_vec())
    }

    /// Logical candidate unions retain possible schemas alongside unknown
    /// handlers. Other source purposes require closed unanimous alternatives.
    fn with_source_candidate_schema<T>(
        &self,
        registry: &CommandRegistry,
        context: InvocationMetadataContext<'_>,
        tokens: &CommandTokens,
        apply: impl FnOnce(&tcl_registry::ResolvedInvocation<'_, '_>) -> Option<T>,
    ) -> Option<T> {
        if !context.permits_logical_source_names()
            || tokens.source_binding.as_ref().and_then(|binding| {
                binding.original_source_advice_purpose(tokens, context, registry)
            }) != Some(crate::command_binding::OriginalInvocationSourcePurpose::Logical)
        {
            return self.with_source_schema(registry, context, tokens, apply);
        }
        if self.effective.words.is_empty()
            || self.effective.words.len() != self.effective.origins.len()
            || self.effective.origins.iter().any(|origin| {
                matches!(origin, InvocationWordOrigin::BindingPrefix(index)
                    if *index >= self.effective.binding_prefix.len())
            })
        {
            return None;
        }
        self.with_source_schema_alternatives(registry, context, tokens, false, apply)
    }

    /// One possible candidate's own schema, for conditional advice that keeps
    /// lookup closure as an open obligation: an unknown residual or possible
    /// absence does not withdraw a candidate that is itself selected.
    pub(crate) fn with_possible_source_schema<T>(
        &self,
        registry: &CommandRegistry,
        context: InvocationMetadataContext<'_>,
        tokens: &CommandTokens,
        apply: impl FnOnce(&tcl_registry::ResolvedInvocation<'_, '_>) -> Option<T>,
    ) -> Option<T> {
        if self.hosted_source.is_some() {
            return self.with_source_schema(registry, context, tokens, apply);
        }
        // A hosted rule body has no native operand layout; its own hosted
        // catalogue entry describes the same command's schema.
        if context
            .source_analysis_input()
            .is_some_and(crate::analyser::ResolvedAnalysisInput::has_hosted_source_name_context)
            && tokens.source_binding.as_ref().is_some_and(|binding| {
                binding
                    .original_conditional_operand_layout_advice(tokens, registry)
                    .is_none()
            })
        {
            let hosted = Self::from_hosted_source(registry, context, tokens)?;
            if hosted.command != self.command {
                return None;
            }
            return hosted
                .hosted_source
                .as_deref()?
                .shape()
                .with_source_schema(registry, context, apply);
        }
        if self.effective.words.is_empty()
            || self.effective.words.len() != self.effective.origins.len()
            || self.effective.origins.iter().any(|origin| {
                matches!(origin, InvocationWordOrigin::BindingPrefix(index)
                    if *index >= self.effective.binding_prefix.len())
            })
        {
            return None;
        }
        self.with_source_schema_alternatives(registry, context, tokens, false, apply)
    }

    fn with_source_schema_alternatives<T>(
        &self,
        registry: &CommandRegistry,
        context: InvocationMetadataContext<'_>,
        tokens: &CommandTokens,
        require_unanimous: bool,
        apply: impl FnOnce(&tcl_registry::ResolvedInvocation<'_, '_>) -> Option<T>,
    ) -> Option<T> {
        let binding = tokens.source_binding.as_ref()?;
        let point = binding
            .original_source_metadata_for_advice(tokens, registry)
            .ok()??;
        context.source_analysis_input()?;
        point.source_analysis_input()?;
        if !context.matches_registry(registry)
            || point.context() != context.context()
            || point.source_analysis_input() != context.source_analysis_input()
        {
            return None;
        }
        let assistance = original_registry_invocation_assistance_with_metadata_context(
            registry,
            Some(point),
            tokens,
        )?;
        if require_unanimous {
            assistance.unanimous_command_words()?;
        }
        if !assistance.candidates.contains(self) {
            return None;
        }
        let original = binding.original_conditional_operand_layout_advice(tokens, registry)?;
        let config = binding.original_lexer_config_for_tokens(tokens)?;
        let dialect = tcl_registry::InvocationDialect {
            lexer_grammar: config.grammar_over(original.dialect().lexer_grammar),
            word_values: WordValueRules::from_config(&config),
            ..original.dialect()
        };
        let values: Vec<_> = self
            .effective
            .words
            .iter()
            .zip(&self.effective.origins)
            .map(|(word, origin)| match origin {
                InvocationWordOrigin::BindingPrefix(index) => {
                    self.effective.binding_prefix.get(*index).cloned()
                }
                _ => Some(effective_invocation_word(
                    word,
                    config.escapes,
                    dialect.word_values,
                )),
            })
            .collect::<Option<_>>()?;
        let words: Vec<_> = self
            .effective
            .words
            .iter()
            .zip(&values)
            .map(|(word, value)| {
                invocation_word_with_source(word, value, dialect.lexer_grammar.escapes)
            })
            .collect();
        let selected =
            tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
                registry,
                Some(point.context()),
                InvocationWords::structured(*words.first()?, &words[1..]).with_dialect(dialect),
                original.realm(),
            );
        let selected = selected.resolved()?;
        (selected.facts().canonical_command == self.command).then_some(())?;
        apply(&selected)
    }

    /// Literal names this original candidate may link into a method frame.
    /// Exact frozen prefix/argument values stay data; this inventory grants no
    /// Native cell, handler, completed store, observer absence or erasure proof.
    #[must_use]
    pub fn possible_method_instance_variable_names(
        &self,
        registry: &CommandRegistry,
        context: InvocationMetadataContext<'_>,
        tokens: &CommandTokens,
    ) -> Option<Vec<String>> {
        let current = original_registry_invocation_assistance_with_metadata_context(
            registry,
            Some(context),
            tokens,
        )?;
        if current.unknown_residual || current.may_be_absent || !current.candidates.contains(self) {
            return None;
        }
        if self.operation
            != tcl_registry::SemanticOperationId::StructuredLowering(
                tcl_registry::hooks::LoweringHookId::Variable,
            )
        {
            return Some(Vec::new());
        }
        if !self.roles_complete {
            return None;
        }
        let words = frozen_argument_words(tokens, &self.effective);
        self.roles
            .iter()
            .filter(|(_, role)| *role == tcl_registry::ArgRole::VarWrite)
            .map(|(argument, _)| {
                let index = self.argument_offset.checked_add(usize::from(*argument))?;
                match words.get(index)? {
                    EffectiveInvocationWord::Literal(value) => Some(value.clone()),
                    EffectiveInvocationWord::ByteLiteral(value) => {
                        std::str::from_utf8(value).ok().map(str::to_owned)
                    }
                    _ => None,
                }
            })
            .collect()
    }

    /// Original written operands mentioned by this possible candidate.
    /// These roles grant assistance only, not reads, stores or body execution.
    #[must_use]
    pub fn written_argument_roles(&self) -> Vec<(usize, tcl_registry::ArgRole)> {
        mapped_written_argument_roles(&self.effective, self.argument_offset, &self.roles)
    }
}

pub(super) fn mapped_written_argument_roles(
    effective: &EffectiveCommandWords,
    argument_offset: usize,
    roles: &[(u8, tcl_registry::ArgRole)],
) -> Vec<(usize, tcl_registry::ArgRole)> {
    let mut mapped = Vec::new();
    for &(index, role) in roles {
        if let Some(effective_index) = argument_offset.checked_add(usize::from(index))
            && let Some(written) = effective.written_argument(effective_index)
            && !mapped.contains(&(written, role))
        {
            mapped.push((written, role));
        }
    }
    mapped
}

/// Advisory envelope retaining every uncertainty of shared command lookup.
/// Consumers may explain candidates but must not infer stores or optimise.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryInvocationAssistance {
    /// Registry candidates for which nominal metadata is available.
    pub candidates: Vec<RegistryInvocationShape>,
    /// At least one possible implementation has no registry shape.
    pub unknown_residual: bool,
    /// Lookup can fail at runtime.
    pub may_be_absent: bool,
}

/// Command words shared by every closed advisory lookup alternative.
/// This projection grants no execution, role, mutation or completion facts.
#[derive(Debug, Clone, Copy)]
pub struct AdvisoryCommandWords<'a> {
    pub(super) command: &'a str,
    pub(super) effective: &'a EffectiveCommandWords,
}

impl AdvisoryCommandWords<'_> {
    /// Registry command identity shared by the advisory alternatives.
    #[must_use]
    pub fn command(&self) -> &str {
        self.command
    }

    /// Original words, alias prefix and origins shared by all alternatives.
    #[must_use]
    pub fn effective(&self) -> &EffectiveCommandWords {
        self.effective
    }
}

impl RegistryInvocationAssistance {
    /// Project unanimous advisory words without selecting an implementation.
    /// Any absent, unknown or disagreeing lookup alternative withdraws the view.
    #[must_use]
    pub fn unanimous_command_words(&self) -> Option<AdvisoryCommandWords<'_>> {
        if self.unknown_residual || self.may_be_absent {
            return None;
        }
        let first = self.candidates.first()?;
        if first.effective.words.is_empty()
            || first.effective.words.len() != first.effective.origins.len()
            || first.effective.origins.iter().any(|origin| {
                matches!(origin, InvocationWordOrigin::BindingPrefix(index)
                    if *index >= first.effective.binding_prefix.len())
            })
        {
            return None;
        }
        self.candidates
            .iter()
            .all(|candidate| {
                candidate.command == first.command && candidate.effective == first.effective
            })
            .then_some(AdvisoryCommandWords {
                command: &first.command,
                effective: &first.effective,
            })
    }
}

/// A possible value-store shape for assistance. It cannot establish a write,
/// a variable's runtime type, or a constant-propagation fact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdvisoryValueAssignment {
    /// Literal destination mentioned by the candidate operation.
    pub name: String,
    /// Authored or alias-inserted value word.
    pub value: WordExpr,
}

/// Project possible scalar value stores from the shared May envelope. A
/// document procedure named `set` contributes no stock assignment contract.
#[must_use]
pub fn advisory_value_assignments(
    registry: &CommandRegistry,
    tokens: &CommandTokens,
) -> Vec<AdvisoryValueAssignment> {
    advisory_value_assignments_with_metadata_context(registry, None, tokens)
}

/// Possible scalar stores under the complete supplied availability generation.
/// The May envelope supplies no actual write, type or constant value.
#[must_use]
pub fn advisory_value_assignments_in_context(
    context: &tcl_registry::model::ContextRegistry,
    tokens: &CommandTokens,
) -> Vec<AdvisoryValueAssignment> {
    advisory_value_assignments_with_metadata_context(
        context.commands(),
        Some(context.into()),
        tokens,
    )
}

pub(crate) fn advisory_value_assignments_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
) -> Vec<AdvisoryValueAssignment> {
    let Some(assistance) =
        registry_invocation_assistance_with_metadata_context(registry, context, tokens)
    else {
        return Vec::new();
    };
    let dialect = tokens
        .source_binding
        .as_ref()
        .and_then(|binding| binding.variable_context.invocation_dialect)
        .or_else(|| {
            context.map_or_else(
                || {
                    registry
                        .profile()
                        .map(tcl_registry::InvocationDialect::of_profile)
                },
                |context| {
                    context
                        .context()
                        .environment
                        .point()
                        .map(tcl_registry::InvocationDialect::of_point)
                },
            )
        });
    let Some(dialect) = dialect else {
        return Vec::new();
    };
    assistance
        .candidates
        .into_iter()
        .filter_map(|shape| {
            if shape.operation
                != tcl_registry::SemanticOperationId::StructuredLowering(
                    tcl_registry::hooks::LoweringHookId::Set,
                )
                || shape.effective.words.len() != 3
            {
                return None;
            }
            Some(AdvisoryValueAssignment {
                name: shape.effective.argument_literal(
                    0,
                    dialect.lexer_grammar.escapes,
                    dialect.word_values,
                )?,
                value: shape.effective.words.get(2)?.clone(),
            })
        })
        .collect()
}

/// Possible original value stores under the complete selected source schema.
/// No handler, completed write, native cell or value-version fact follows.
pub(crate) fn original_advisory_value_assignments_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
) -> Vec<AdvisoryValueAssignment> {
    let Some(context) = context.filter(|context| context.permits_logical_source_names()) else {
        return Vec::new();
    };
    let Some(assistance) = original_registry_invocation_assistance_with_metadata_context(
        registry,
        Some(context),
        tokens,
    ) else {
        return Vec::new();
    };
    assistance
        .candidates
        .into_iter()
        .filter_map(|candidate| {
            candidate.with_source_candidate_schema(registry, context, tokens, |schema| {
                (schema.facts().operation
                    == tcl_registry::SemanticOperationId::StructuredLowering(
                        tcl_registry::hooks::LoweringHookId::Set,
                    )
                    && candidate.effective.words.len() == 3)
                    .then_some(())?;
                Some(AdvisoryValueAssignment {
                    name: schema
                        .words
                        .arguments()
                        .literal_at(schema.semantics.argument_offset)?
                        .to_owned(),
                    value: candidate.effective.words.get(2)?.clone(),
                })
            })
        })
        .collect()
}

/// Conditional object factory metadata at one original source point.
/// Unknown receiver contents, handler and completion alternatives remain open.
#[derive(Clone, Debug)]
pub(crate) struct AdvisoryObjectFactory {
    pub(crate) publication: Option<(String, &'static str)>,
    pub(crate) result_class: Option<&'static str>,
}

/// Original selected factory source schemas for advisory object-class unions.
/// A named publication requires the descriptor's explicit source layout; an
/// unsupported optional/control layout supplies no name. Callable result class
/// metadata remains independent of that named publication. Neither is a store,
/// allocation, physical read, runtime class, completed handler or dispatch proof.
pub(crate) fn advisory_object_factories_with_metadata_context(
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
) -> Vec<AdvisoryObjectFactory> {
    let Some(assistance) = original_registry_invocation_assistance_with_metadata_context(
        registry,
        Some(context),
        tokens,
    ) else {
        return Vec::new();
    };
    assistance
        .candidates
        .into_iter()
        .filter_map(|candidate| {
            candidate.with_source_candidate_schema(registry, context, tokens, |schema| {
                (schema.facts().arity_accepts_frozen_arguments() == Some(true)).then_some(())?;
                let publication =
                    schema
                        .authored_source_command_publication()
                        .and_then(|publication| {
                            let tcl_registry::AuthoredSourceCommandPublicationKind::Instance {
                                class_name,
                            } = publication.kind
                            else {
                                return None;
                            };
                            Some((
                                schema
                                    .words
                                    .arguments()
                                    .literal_at(publication.argument)?
                                    .to_owned(),
                                class_name,
                            ))
                        });
                let result_class = schema.authored_source_callable_factory_class();
                (publication.is_some() || result_class.is_some()).then_some(AdvisoryObjectFactory {
                    publication,
                    result_class,
                })
            })
        })
        .collect()
}

/// Possible original operands of a default return operation. These are
/// result candidates only: no completion, returned type or reached caller is
/// established. Option-bearing forms require their own return-options recipe.
#[must_use]
pub fn advisory_return_values(registry: &CommandRegistry, tokens: &CommandTokens) -> Vec<WordExpr> {
    advisory_return_values_with_metadata_context(registry, None, tokens)
}

/// Possible return operands selected under the retained availability context.
/// These source candidates provide no completion or returned runtime type.
#[must_use]
pub fn advisory_return_values_in_context(
    context: &tcl_registry::model::ContextRegistry,
    tokens: &CommandTokens,
) -> Vec<WordExpr> {
    advisory_return_values_with_metadata_context(context.commands(), Some(context.into()), tokens)
}

pub(crate) fn advisory_return_values_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
) -> Vec<WordExpr> {
    let mut values = Vec::new();
    for shape in registry_invocation_assistance_with_metadata_context(registry, context, tokens)
        .into_iter()
        .flat_map(|assistance| assistance.candidates)
    {
        if shape.operation
            == tcl_registry::SemanticOperationId::StructuredLowering(
                tcl_registry::hooks::LoweringHookId::Return,
            )
            && shape.effective.words.len() == 2
            && let Some(value) = shape.effective.words.get(1)
            && !values.contains(value)
        {
            values.push(value.clone());
        }
    }
    values
}

/// Nominal catalogue assistance for an unchanged logical lookup slot. The
/// provider need not be loaded; this cannot establish actual result types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogueInvocationAssistance {
    /// Slot applicability established by the source lookup owner.
    pub candidate: crate::command_binding::DeclaredCommandCandidate,
    /// Available nominal metadata, selected under the document's dialect.
    pub shape: RegistryInvocationShape,
}

/// Obtain catalogue assistance separately from execution proofs. A command
/// mutation, document definition or unknown lookup invalidates applicability;
/// catalogue availability alone never restores the live implementation.
#[must_use]
pub fn catalogue_invocation_assistance<'a>(
    registry: &CommandRegistry,
    context: impl Into<InvocationMetadataContext<'a>>,
    tokens: &CommandTokens,
) -> Option<CatalogueInvocationAssistance> {
    let context = context.into();
    if !context.matches_registry(registry) {
        return None;
    }
    if tokens.synthetic.is_some() {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    let candidate = binding.catalogue_command.as_ref()?;
    context.context().resolve_spec(registry, &candidate.name)?;
    let effective = compose_effective_words(tokens, &candidate.name, &[])?;
    let RegistryInvocationResolution::Resolved(facts) =
        resolve_effective_tokens_with_metadata_context(
            registry,
            Some(context),
            tokens,
            &effective,
            binding.variable_context.invocation_dialect,
        )
        .ok()?
    else {
        return None;
    };
    Some(CatalogueInvocationAssistance {
        candidate: candidate.clone(),
        shape: RegistryInvocationShape::from_facts(&facts, effective),
    })
}

/// Query possible registry shapes without upgrading a May binding into Must.
/// Document implementations retain an unknown residual and cannot inherit a
/// stock command contract from their written name.
#[must_use]
pub fn registry_invocation_assistance(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    tokens: &CommandTokens,
) -> Option<RegistryInvocationAssistance> {
    registry_invocation_assistance_with_metadata_context(
        registry,
        context.map(InvocationMetadataContext::from),
        tokens,
    )
}

/// Query possible Registry shapes under the retained availability generation.
/// The original binding supplies candidates and unresolved alternatives; this
/// context neither selects a candidate nor supplies missing execution proof.
#[must_use]
pub fn registry_invocation_assistance_in_context(
    context: &tcl_registry::model::ContextRegistry,
    tokens: &CommandTokens,
) -> Option<RegistryInvocationAssistance> {
    registry_invocation_assistance_with_metadata_context(
        context.commands(),
        Some(context.into()),
        tokens,
    )
}

pub(crate) fn registry_invocation_assistance_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
) -> Option<RegistryInvocationAssistance> {
    registry_invocation_assistance_with_metadata_context_using(
        registry,
        context,
        tokens,
        effective_words_for_target,
    )
}

fn registry_invocation_assistance_with_metadata_context_using(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
    compose: fn(
        &CommandTokens,
        &crate::command_binding::SourceCommandTarget,
    ) -> Option<EffectiveCommandWords>,
) -> Option<RegistryInvocationAssistance> {
    if context.is_some_and(|context| !context.matches_registry(registry)) {
        return None;
    }
    if tokens.synthetic.is_some() {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    let dialect = binding.variable_context.invocation_dialect.or_else(|| {
        registry
            .profile()
            .map(tcl_registry::InvocationDialect::of_profile)
    });
    let mut assistance = RegistryInvocationAssistance {
        candidates: Vec::new(),
        unknown_residual: binding.execution_is_unknown(),
        may_be_absent: binding.execution_may_be_absent(),
    };
    for target in binding
        .execution_targets()
        .chain(binding.proved_handler_target())
    {
        if !target.registry_backed {
            assistance.unknown_residual = true;
            continue;
        }
        let effective = compose(tokens, target)?;
        let Ok(RegistryInvocationResolution::Resolved(facts)) =
            resolve_effective_tokens_with_metadata_context(
                registry, context, tokens, &effective, dialect,
            )
        else {
            assistance.unknown_residual = true;
            continue;
        };
        assistance
            .candidates
            .push(RegistryInvocationShape::from_facts(&facts, effective));
    }
    Some(assistance)
}

/// Readonly Registry metadata at an unchanged original command site.
/// A unanimous retained execution envelope is preferred. Otherwise the genuine
/// original declaration layout may supply conditional command words before
/// operand evaluation, including an uncalled procedure body. Runtime lookup
/// uncertainty, Normal completion, stores and compiler admission are separate;
/// this projection changes none of their receipts.
#[must_use]
pub fn original_registry_invocation_assistance(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    tokens: &CommandTokens,
) -> Option<RegistryInvocationAssistance> {
    original_registry_invocation_assistance_with_metadata_context(
        registry,
        context.map(InvocationMetadataContext::from),
        tokens,
    )
}

/// Read conditional original Registry syntax with the supplied metadata owner.
/// Source consumers retain complete input and reject missing ownership before
/// calling; this readonly advice supplies no entered invocation or edit grant.
#[must_use]
pub fn original_registry_invocation_assistance_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
) -> Option<RegistryInvocationAssistance> {
    // Implementation contract: naming.source.original-registry-header-advice
    // docs/design/analysis/name-resolution-proofs/original-registry-header-advice.md
    tokens
        .source_binding
        .as_ref()?
        .original_lexer_config_for_tokens(tokens)?;
    let reached = registry_invocation_assistance_with_metadata_context_using(
        registry,
        context,
        tokens,
        source_effective_words_for_target,
    );
    if reached
        .as_ref()
        .is_some_and(|advice| advice.unanimous_command_words().is_some())
    {
        return reached;
    }
    if let Some(context) = context
        && context
            .source_analysis_input()
            .is_some_and(crate::analyser::ResolvedAnalysisInput::has_hosted_source_name_context)
    {
        let mut reached = reached?;
        // In the fresh F5 world the original command table answers first;
        // the hosted catalogue is advice only where that lookup has nothing.
        if !reached.candidates.is_empty()
            && tokens.source_binding.as_ref().is_some_and(
                crate::command_binding::SourceInvocationBinding::in_hosted_rule_loader_world,
            )
        {
            return Some(reached);
        }
        let Some(candidate) =
            RegistryInvocationShape::from_hosted_source(registry, context, tokens)
        else {
            return Some(reached);
        };
        if !reached.candidates.contains(&candidate) {
            reached.candidates.push(candidate);
        }
        return Some(reached);
    }
    let context = context
        .or_else(|| body_assistance_context(registry, tokens).map(InvocationMetadataContext::from));
    let declared = context
        .and_then(|context| original_declared_structured_invocation(registry, context, tokens));
    let Some(declared) = declared else {
        return reached;
    };
    Some(RegistryInvocationAssistance {
        candidates: vec![RegistryInvocationShape::from_facts(
            &declared.facts,
            declared.effective,
        )],
        unknown_residual: false,
        may_be_absent: false,
    })
}

/// Possible Registry grammar in the unchanged literal prefix of an open quoted
/// procedure body. The shared source owner retains a distinct incomplete header
/// observation; this creates no complete body, installed definition, formal
/// activation, executable invocation, Normal or native compiler capability.
#[must_use]
pub fn original_incomplete_body_registry_invocation_assistance(
    registry: &CommandRegistry,
    realm: &crate::realm::CommandBindingRealm,
    tokens: &CommandTokens,
) -> Option<RegistryInvocationAssistance> {
    // Implementation contract: naming.source.incomplete-body-header-metadata
    // docs/design/analysis/name-resolution-proofs/incomplete-body-header-metadata.md
    let advice = realm
        .source_bindings_ref()
        .incomplete_body_operand_layout_advice(tokens, registry)?;
    let dialect = advice.dialect();
    let mut candidates = Vec::new();
    for target in advice.targets() {
        let effective = source_effective_words_for_target(tokens, target)?;
        let values: Vec<_> = effective
            .words
            .iter()
            .map(|word| {
                effective_invocation_word(word, dialect.lexer_grammar.escapes, dialect.word_values)
            })
            .collect();
        let words: Vec<_> = values
            .iter()
            .map(EffectiveInvocationWord::as_registry_word)
            .collect();
        let RegistryInvocationResolution::Resolved(facts) =
            resolve_registry_words_in_realm(registry, None, &words, Some(dialect), advice.realm())
                .ok()?
        else {
            return None;
        };
        candidates.push(RegistryInvocationShape::from_facts(&facts, effective));
    }
    (!candidates.is_empty()).then_some(RegistryInvocationAssistance {
        candidates,
        unknown_residual: false,
        may_be_absent: false,
    })
}

/// Source role assistance from retained lookup candidates and applicable
/// declarations. Original operand mapping is owned here; captured alias
/// prefixes and expanded values cannot manufacture editable source indices.
/// This union preserves lookup uncertainty and grants no execution, read,
/// store, completion or optimisation proof.
#[must_use]
pub fn invocation_argument_role_assistance<'a>(
    registry: &CommandRegistry,
    context: impl Into<InvocationMetadataContext<'a>>,
    surface: &tcl_registry::model::DocumentCommandSurface<'_>,
    tokens: &CommandTokens,
) -> Vec<(usize, tcl_registry::ArgRole)> {
    let context = context.into();
    if !context.matches_registry(registry) {
        return Vec::new();
    }
    let mut roles = Vec::new();
    if let Some(assistance) =
        registry_invocation_assistance_with_metadata_context(registry, Some(context), tokens)
    {
        for candidate in assistance.candidates {
            roles.extend(candidate.written_argument_roles());
        }
    }
    if let Some(catalogue) = catalogue_invocation_assistance(registry, context, tokens) {
        roles.extend(catalogue.shape.written_argument_roles());
    }
    if let Some(declared) = resolved_declared_assistance(surface, tokens) {
        roles.extend(declared.roles);
    }
    let mut unique = Vec::new();
    for role in roles {
        if !unique.contains(&role) {
            unique.push(role);
        }
    }
    unique.sort_by_key(|(index, _)| *index);
    unique
}

/// Definite syntactic roles shared by every actual retained implementation.
/// An opaque, absent, incomplete or unaccepted alternative withdraws the
/// entire projection. Catalogue and declared navigation do not establish a
/// grammar obligation. This grants neither execution nor successful effects.
#[must_use]
pub fn invocation_argument_role_consensus<'a>(
    registry: &CommandRegistry,
    context: impl Into<InvocationMetadataContext<'a>>,
    tokens: &CommandTokens,
) -> Vec<(usize, tcl_registry::ArgRole)> {
    let context = context.into();
    if !context.matches_registry(registry) {
        return Vec::new();
    }
    let Some(binding) = tokens.source_binding.as_ref() else {
        return Vec::new();
    };
    if tokens.synthetic.is_some()
        || binding.execution_is_unknown()
        || binding.execution_may_be_absent()
    {
        return Vec::new();
    }
    let dialect = binding.variable_context.invocation_dialect;
    let mut consensus: Option<Vec<(usize, tcl_registry::ArgRole)>> = None;
    for target in binding
        .execution_targets()
        .chain(binding.proved_handler_target())
    {
        if !target.registry_backed {
            return Vec::new();
        }
        let Some(effective) = effective_words_for_target(tokens, target) else {
            return Vec::new();
        };
        let Ok(RegistryInvocationResolution::Resolved(facts)) =
            resolve_effective_tokens_with_metadata_context(
                registry,
                Some(context),
                tokens,
                &effective,
                dialect,
            )
        else {
            return Vec::new();
        };
        if !facts.arg_roles_complete || facts.arity_accepts_frozen_arguments() != Some(true) {
            return Vec::new();
        }
        let roles =
            mapped_written_argument_roles(&effective, facts.argument_offset, &facts.arg_roles);
        if let Some(shared) = &mut consensus {
            shared.retain(|role| roles.contains(role));
        } else {
            consensus = Some(roles);
        }
    }
    consensus.unwrap_or_default()
}
