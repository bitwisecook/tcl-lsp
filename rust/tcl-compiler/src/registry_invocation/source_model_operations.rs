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

//! Original source-model operations: variable footprints, substitution purity and layouts.

use super::effective_words::{
    compose_original_effective_words, original_logical_prefix_for_tokens,
};
use super::handler_invocation::resolved_handler_invocation_with_metadata_context;
use super::metadata_context::InvocationMetadataContext;
use super::structured_invocation::original_declared_structured_invocation;
use super::word_resolution::{
    EffectiveInvocationWord, RegistryInvocationResolution, effective_invocation_word,
    resolve_registry_words_in_realm_with_metadata_context_and_case,
};
use super::{ResolvedStatementInvocation, resolved_tokens_invocation_with_metadata_context};
use crate::ir::CommandTokens;
use tcl_registry::CommandRegistry;

/// Conditional option topology of the retained original invocation. Captured
/// operands keep their effective ordinals, with no handler or Normal grant.
pub(crate) fn conditional_option_arguments_with_metadata_context(
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
) -> Option<(
    ResolvedStatementInvocation,
    tcl_registry::AuthoredSourceOptionArguments,
)> {
    let logical_source = tokens
        .source_binding
        .as_ref()?
        .original_source_advice_purpose(tokens, context, registry)?
        == crate::command_binding::OriginalInvocationSourcePurpose::Logical;
    if !context.matches_registry(registry) || tokens.synthetic.is_some() {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    if let Some(input) = context.source_analysis_input()
        && binding
            .original_lexer_config_for_tokens(tokens)?
            .normalized()
            != input.lexer_config().normalized()
    {
        return None;
    }
    let invocation =
        resolved_tokens_invocation_with_metadata_context(registry, Some(context), tokens)
            .or_else(|| {
                resolved_handler_invocation_with_metadata_context(registry, Some(context), tokens)
            })
            .or_else(|| original_declared_structured_invocation(registry, context, tokens))?;
    let realm = if logical_source {
        binding.original_source_realm_for_input(
            tokens,
            context.source_analysis_input()?,
            registry,
        )?
    } else {
        binding.invocation_realm().or_else(|| {
            binding
                .declaration_operand_layout_advice(tokens)
                .map(|advice| advice.realm())
        })?
    };
    let options = invocation.with_metadata_schema(registry, context, realm, |schema| {
        schema.authored_source_possible_option_arguments()
    })?;
    Some((invocation, options))
}

/// Conditional operation recipe for the exact supplied Logical source model.
/// Original alias captures and source words remain composed by the shared
/// declaration owner. This grants no Native handler, Normal completion,
/// compiler entry, variable allocation or executable erasure permission.
#[doc(hidden)]
pub fn original_logical_operation_invocation_with_metadata_context(
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
) -> Option<ResolvedStatementInvocation> {
    original_source_model_operation_invocation_with_metadata_context(
        registry,
        context,
        tokens,
        crate::command_binding::OriginalInvocationSourcePurpose::Logical,
    )
}

/// The original operation invocation of a source read under `purpose`, a
/// software source model (Logical, or a release's authored simulation). It
/// answers source structure only: no Native entry, dispatch or completion.
#[doc(hidden)]
#[must_use]
pub fn original_source_model_operation_invocation_with_metadata_context(
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
    purpose: crate::command_binding::OriginalInvocationSourcePurpose,
) -> Option<ResolvedStatementInvocation> {
    if !matches!(
        purpose,
        crate::command_binding::OriginalInvocationSourcePurpose::Logical
            | crate::command_binding::OriginalInvocationSourcePurpose::AuthoredSimulation
    ) {
        return None;
    }
    let logical_source = tokens
        .source_binding
        .as_ref()?
        .original_source_advice_purpose(tokens, context, registry)?
        == purpose;
    if !context.matches_registry(registry) || !logical_source || tokens.synthetic.is_some() {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    let context = binding.original_invocation_metadata_for_input(
        tokens,
        context.source_analysis_input()?,
        registry,
    )?;
    if !logical_source {
        return None;
    }
    resolved_handler_invocation_with_metadata_context(registry, Some(context), tokens)
        .or_else(|| original_declared_structured_invocation(registry, context, tokens))
}

/// Conditional completion of the exact original Native source schema.
/// This supplies analysis topology only, without a reached handler or Normal result.
pub(crate) fn original_native_source_conditional_completion(
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
) -> Option<tcl_registry::completion_route::InvocationCompletionRoute> {
    let input = context.source_analysis_input()?;
    if input.has_logical_source_name_context()
        || input.has_hosted_source_name_context()
        || !context.matches_registry(registry)
        || tokens.synthetic.is_some()
    {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    let context = binding.original_invocation_metadata_for_input(tokens, input, registry)?;
    let config = binding.original_lexer_config_for_tokens(tokens)?;
    if config.normalized() != input.lexer_config().normalized() {
        return None;
    }
    if binding.has_original_native_source_command() {
        let words = binding.original_native_source_words(tokens, input, registry)?;
        let dialect = words.dialect()?;
        let arguments = words
            .arguments()
            .iter()
            .map(|word| {
                word.literal_bytes()
                    .and_then(|bytes| std::str::from_utf8(bytes).ok())
                    .map(tcl_registry::InvocationWord::Literal)
            })
            .collect::<Option<Vec<_>>>()?;
        return words.with_source_schema(input.borrowed_context_registry(), |schema| {
            (schema.facts().arity_accepts_frozen_arguments() == Some(true)).then_some(())?;
            registry.invocation_completion_route(
                schema.descriptor_key().label(),
                tcl_registry::InvocationArguments::structured(&arguments).with_dialect(dialect),
                Some(context.context().authoring_query()),
            )
        })?;
    }
    let advice = binding
        .declaration_operand_layout_advice(tokens)
        .or_else(|| binding.original_compilation_lookup_advice(tokens))?;
    if advice.has_unclosed_source_coverage() {
        return None;
    }
    let selected = resolve_original_source_layout(registry, Some(context), tokens, &advice)?;
    selected.dialect?;
    if selected.facts.arity_accepts_frozen_arguments() != Some(true)
        || selected
            .evaluated_words
            .iter()
            .any(|word| word.literal_bytes().is_none())
    {
        return None;
    }
    Some(selected.completion_route(registry))
}

/// Conditional variable destinations and aliases of the exact Logical source.
/// This is a readonly MAY footprint, without Native entry or completed stores.
pub(crate) fn original_logical_variable_footprint_with_metadata_context(
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
) -> Option<(
    tcl_registry::VariableWriteProjection,
    Vec<tcl_registry::VariableCellAliasTransition>,
)> {
    if context.matches_registry(registry)
        && let Some(words) = tokens
            .source_binding
            .as_ref()?
            .original_logical_source_words(tokens, context.source_analysis_input()?, registry)
    {
        return words.with_source_schema(
            context.source_analysis_input()?.borrowed_context_registry(),
            |schema| {
                Some((
                    CommandRegistry::variable_write_projection_for_selected_source(schema),
                    schema.authored_logical_source_variable_aliases()?,
                ))
            },
        )?;
    }
    let binding = tokens.source_binding.as_ref()?;
    let context = binding.original_invocation_metadata_for_input(
        tokens,
        context.source_analysis_input()?,
        registry,
    )?;
    let selected =
        original_logical_operation_invocation_with_metadata_context(registry, context, tokens)?;
    let realm = binding.original_source_realm_for_input(
        tokens,
        context.source_analysis_input()?,
        registry,
    )?;
    selected.with_metadata_schema(registry, context, realm, |schema| {
        let aliases = schema.authored_logical_source_variable_aliases()?;
        Some((
            CommandRegistry::variable_write_projection_for_selected_source(schema),
            aliases,
        ))
    })
}

/// Purity of the selected original substitution implementation. Native
/// dispatch requires a normal handler and closed execution alternatives;
/// the separate Logical branch retains its exact source operation recipe.
/// Neither branch proves operand evaluation, no-raise, observers or erasure.
pub(crate) fn original_substitution_purity_with_metadata_context(
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
) -> Option<bool> {
    let logical_source = tokens
        .source_binding
        .as_ref()?
        .original_source_advice_purpose(tokens, context, registry)?
        == crate::command_binding::OriginalInvocationSourcePurpose::Logical;
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    if !context.matches_registry(registry) || tokens.synthetic.is_some() {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    let config = binding.original_lexer_config_for_tokens(tokens)?;
    let input = context.source_analysis_input()?;
    if config.normalized() != input.lexer_config().normalized() {
        return None;
    }
    let invocation = if logical_source {
        original_logical_operation_invocation_with_metadata_context(registry, context, tokens)?
    } else {
        if binding.execution_is_unknown() || binding.execution_may_be_absent() {
            return None;
        }
        resolved_handler_invocation_with_metadata_context(registry, Some(context), tokens)?
    };
    if invocation.facts.arity_accepts_frozen_arguments() != Some(true)
        || matches!(
            invocation.facts.subcommand,
            tcl_registry::OwnedSubcommandResolution::Unknown { .. }
                | tcl_registry::OwnedSubcommandResolution::Ambiguous { .. }
                | tcl_registry::OwnedSubcommandResolution::Indeterminate { .. }
        )
    {
        return None;
    }
    Some(!invocation.facts.mutator && invocation.facts.evaluates_purely())
}

/// The source-role consumer chooses its purpose-specific closure before this
/// shared full-vector/schema resolution. This helper supplies no completion.
pub(super) fn resolve_original_declared_layout<'a>(
    registry: &CommandRegistry,
    context: impl Into<InvocationMetadataContext<'a>>,
    tokens: &CommandTokens,
    advice: &crate::command_binding::OriginalCompilationLookupAdvice,
) -> Option<ResolvedStatementInvocation> {
    resolve_original_source_layout(registry, Some(context.into()), tokens, advice)
}

pub(super) fn resolve_original_source_layout(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
    advice: &crate::command_binding::OriginalCompilationLookupAdvice,
) -> Option<ResolvedStatementInvocation> {
    if context.is_some_and(|context| !context.matches_registry(registry)) {
        return None;
    }
    let dialect = advice.dialect();
    let mut unanimous = None;
    for target in advice.targets() {
        if !target.registry_backed {
            return None;
        }
        if !target.prepended.is_empty()
            && context.is_some_and(
                crate::registry_invocation::InvocationMetadataContext::permits_logical_source_names,
            )
        {
            let context = context?;
            let purpose = tokens
                .source_binding
                .as_ref()?
                .original_source_advice_purpose(tokens, context, registry)?;
            if purpose == crate::command_binding::OriginalInvocationSourcePurpose::Logical {
                original_logical_prefix_for_tokens(
                    tokens,
                    target,
                    context.source_analysis_input()?,
                )?;
            }
        }
        let effective =
            compose_original_effective_words(tokens, &target.command, &target.prepended)?;
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
        let (resolution, authored_case_body_selection) =
            resolve_registry_words_in_realm_with_metadata_context_and_case(
                registry,
                context,
                &words,
                Some(dialect),
                advice.realm(),
            )
            .ok()?;
        let RegistryInvocationResolution::Resolved(facts) = resolution else {
            return None;
        };
        let evaluated_words = values.get(1..)?.to_vec();
        let selected = ResolvedStatementInvocation {
            dialect: Some(dialect),
            facts,
            arguments: effective.argument_presentations(tokens.argv_texts.get(1..)?)?,
            evaluated_arguments: evaluated_words
                .iter()
                .map(|word| match word {
                    EffectiveInvocationWord::Literal(value) => Some(value.clone()),
                    _ => None,
                })
                .collect(),
            evaluated_words,
            effective,
            authored_case_body_selection: Some(authored_case_body_selection),
        };
        if unanimous
            .as_ref()
            .is_some_and(|previous| previous != &selected)
        {
            return None;
        }
        unanimous = Some(selected);
    }
    unanimous
}
