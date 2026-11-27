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

//! Registry words built from source transition advice: Logical, native and conditional sources.

use super::vendor_words::conditional_roles;
use super::{
    OriginalOperandSource, OriginalRegistrySource, OriginalRegistryWords, matching_document_image,
    retained_document_image, trace_original_source_role_option, trace_original_source_role_words,
};
#[cfg(test)]
use crate::registry_invocation::source_structure::trace_original_child_guard;

pub(in super::super) fn logical_source_registry_words(
    source: &str,
    realm: &crate::realm::CommandBindingRealm,
    input: &crate::analyser::ResolvedAnalysisInput,
    segment: &tcl_source::segment::SegmentedCommand,
) -> Option<OriginalRegistryWords> {
    if realm
        .source_bindings_ref()
        .original_logical_source_name_advice_input()
        != Some(input)
        || !realm.matches_resolved_analysis_input(input)
    {
        return None;
    }
    // Retain the same complete bytes/channel/configuration check without
    // allocating, hashing and indexing another whole Document image per query.
    let image = retained_document_image(source, realm, input.lexer_config())?;
    let context = input.context_registry();
    let words =
        source_transition_registry_words(source, input.lexer_config(), realm, &context, segment)
            .or_else(|| {
                let mut tokens = crate::ir::CommandTokens::from_segmented(
                    &tcl_lexer::SourceMap::from_image(&image),
                    input.lexer_config(),
                    segment,
                );
                realm.stamp_original_tokens(&mut tokens);
                let advice = realm.original_source_prefix_catalogue_advice(input, &tokens)?;
                source_transition_words_with_segment(
                    source,
                    input.lexer_config(),
                    &context,
                    segment,
                    advice,
                )
            })?;
    let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
        return None;
    };
    (advice.original_head().logical_input().is_some()
        && advice.logical_source_input() == Some(input))
    .then_some(words)
}

pub(super) fn logical_source_registry_words_in_original_image(
    image: &tcl_lexer::SourceImage,
    realm: &crate::realm::CommandBindingRealm,
    input: &crate::analyser::ResolvedAnalysisInput,
    segment: &tcl_source::segment::SegmentedCommand,
) -> Option<OriginalRegistryWords> {
    if realm
        .source_bindings_ref()
        .original_logical_source_name_advice_input()
        != Some(input)
        || !realm.matches_resolved_analysis_input(input)
        || !realm.matches_original_source_image(image, input.lexer_config())
    {
        return None;
    }
    let mut tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::from_image(image),
        input.lexer_config(),
        segment,
    );
    realm.stamp_original_tokens(&mut tokens);
    let context = input.context_registry();
    let advice = realm
        .original_source_transition_advice(&context, &tokens)
        .or_else(|| realm.original_source_prefix_catalogue_advice(input, &tokens))?;
    let words = source_transition_words_with_segment_in_image(
        image,
        input.lexer_config(),
        &context,
        segment,
        advice,
    )?;
    let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
        return None;
    };
    (advice.original_head().logical_input().is_some()
        && advice.logical_source_input() == Some(input))
    .then_some(words)
}

pub(super) fn native_source_registry_words(
    source: &str,
    config: tcl_lexer::LexerConfig,
    realm: &crate::realm::CommandBindingRealm,
    context: &tcl_registry::model::ContextRegistry,
    segment: &tcl_source::segment::SegmentedCommand,
    retained_metadata: Option<crate::registry_invocation::OriginalConditionalRegistryMetadata>,
) -> Option<OriginalRegistryWords> {
    let image = retained_document_image(source, realm, config)?;
    realm
        .matches_original_source_image(&image, config)
        .then_some(())?;
    if let Some(input) = realm
        .source_bindings_ref()
        .original_logical_source_name_advice_input()
    {
        // This domain is selected by the validated full Logical input, never
        // by a missing Native operand or a Registry/profile display label.
        if config != input.lexer_config()
            || !std::ptr::eq(std::sync::Arc::as_ptr(&input.context_registry()), context)
        {
            return None;
        }
        return logical_source_registry_words(source, realm, input, segment);
    }
    let Some(metadata) = trace_original_source_role_option(
        "native.metadata",
        segment.argv.first().map(|word| word.span.start()),
        native_source_metadata(source, config, realm, context, segment, retained_metadata),
    ) else {
        return trace_original_source_role_words(
            "native.transition_fallback",
            segment.argv.first().map(|word| word.span.start()),
            source_transition_registry_words(source, config, realm, context, segment),
        );
    };
    conditional_source_registry_words(source, config, realm, context, segment, metadata)
}

fn native_source_metadata(
    source: &str,
    config: tcl_lexer::LexerConfig,
    realm: &crate::realm::CommandBindingRealm,
    context: &tcl_registry::model::ContextRegistry,
    segment: &tcl_source::segment::SegmentedCommand,
    retained_metadata: Option<crate::registry_invocation::OriginalConditionalRegistryMetadata>,
) -> Option<crate::registry_invocation::OriginalConditionalRegistryMetadata> {
    let registry = context.commands();
    let image = retained_document_image(source, realm, config)?;
    let mut tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::from_image(&image),
        config,
        segment,
    );
    realm.stamp_original_tokens(&mut tokens);
    if realm.original_source_registry_barrier(context, &tokens) {
        return None;
    }

    let metadata = if let Some(metadata) = trace_original_source_role_option(
        "metadata.retained",
        segment.argv.first().map(|word| word.span.start()),
        retained_metadata,
    ) {
        metadata
    } else {
        let candidate = tokens
            .source_binding
            .as_ref()
            .and_then(|binding| {
                trace_original_source_role_option(
                    "metadata.point_candidate",
                    segment.argv.first().map(|word| word.span.start()),
                    binding.original_catalogue_source_candidate_from_tokens(&tokens, registry),
                )
            })
            .or_else(|| {
                trace_original_source_role_option(
                    "metadata.future_candidate",
                    segment.argv.first().map(|word| word.span.start()),
                    realm
                        .source_bindings_ref()
                        .original_future_source_catalogue_candidate(
                            &image, config, &tokens, registry,
                        ),
                )
            });
        candidate.and_then(|candidate| {
            super::super::conditional_metadata::original_catalogue_registry_metadata(
                context, candidate,
            )
        })?
    };
    Some(metadata)
}

fn conditional_source_values(
    metadata: &crate::registry_invocation::OriginalConditionalRegistryMetadata,
) -> Option<(
    Vec<tcl_lexer::NativeWord>,
    Vec<crate::registry_invocation::EffectiveInvocationWord>,
)> {
    use crate::registry_invocation::EffectiveInvocationWord;
    let native = metadata.original_words().to_vec();
    let captured = tcl_registry::native_compiler_words::NativeCompilerWords::capture(
        &native,
        metadata
            .original_head()
            .name_input()
            .policy()
            .string_protocol(),
    )
    .ok()?;
    let values = native
        .iter()
        .enumerate()
        .skip(1)
        .map(|(ordinal, word)| {
            if word.group().expand {
                EffectiveInvocationWord::Expanded
            } else {
                captured
                    .literal(ordinal)
                    .map_or(EffectiveInvocationWord::Dynamic, |bytes| {
                        EffectiveInvocationWord::ByteLiteral(std::sync::Arc::from(bytes))
                    })
            }
        })
        .collect::<Vec<_>>();
    Some((native, values))
}

fn conditional_source_operands(
    native: &[tcl_lexer::NativeWord],
    segment: &tcl_source::segment::SegmentedCommand,
    origin: &OriginalRegistrySource,
) -> Vec<Option<OriginalOperandSource>> {
    native
        .iter()
        .enumerate()
        .skip(1)
        .map(|(ordinal, word)| {
            Some(OriginalOperandSource {
                span: segment.argv.get(ordinal)?.span,
                input: match origin {
                    OriginalRegistrySource::Conditional(metadata) => {
                        let head = metadata.original_head().name_input();
                        tcl_names::word::SignatureSourceNameKey::from_original_native_word(
                            word,
                            head.word_value_rules(),
                            head.policy(),
                        )
                        .map(crate::signature_scan::scope::SignatureSourceNameInput::OriginalWord)
                    }
                    OriginalRegistrySource::Selected
                    | OriginalRegistrySource::ConditionalNativeSource(_)
                    | OriginalRegistrySource::Scoped(_)
                    | OriginalRegistrySource::DefinitionMember(_)
                    | OriginalRegistrySource::Vendor(_)
                    | OriginalRegistrySource::SourceTransitions(_)
                    | OriginalRegistrySource::ProducedPrefix(_) => None,
                },
                word: Some(word.clone()),
            })
        })
        .collect()
}

fn conditional_source_registry_words(
    source: &str,
    config: tcl_lexer::LexerConfig,
    realm: &crate::realm::CommandBindingRealm,
    context: &tcl_registry::model::ContextRegistry,
    segment: &tcl_source::segment::SegmentedCommand,
    metadata: crate::registry_invocation::OriginalConditionalRegistryMetadata,
) -> Option<OriginalRegistryWords> {
    use crate::registry_invocation::InvocationWordOrigin;
    let registry = context.commands();
    let image = retained_document_image(source, realm, config)?;
    let offset = segment.argv.first()?.span.start();
    let (command, native, values, roles, origin) = {
        let (native, values) = conditional_source_values(&metadata)?;
        let roles = conditional_roles(
            metadata.roles(),
            metadata.argument_offset(),
            metadata.roles_complete(),
        );
        (
            metadata.command().to_owned(),
            native,
            values,
            roles,
            OriginalRegistrySource::Conditional(std::sync::Arc::new(metadata)),
        )
    };
    let mut tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::from_image(&image),
        config,
        segment,
    );
    realm.stamp_original_tokens(&mut tokens);
    let dialect = match &origin {
        OriginalRegistrySource::Scoped(_)
        | OriginalRegistrySource::DefinitionMember(_)
        | OriginalRegistrySource::Vendor(_)
        | OriginalRegistrySource::SourceTransitions(_)
        | OriginalRegistrySource::ProducedPrefix(_) => None,
        OriginalRegistrySource::Conditional(metadata) => metadata.original_source_dialect(),
        OriginalRegistrySource::Selected | OriginalRegistrySource::ConditionalNativeSource(_) => {
            tokens
                .source_binding
                .as_ref()
                .and_then(|binding| binding.original_source_word_dialect_for_tokens(&tokens))
        }
    };
    let original = crate::registry_invocation::original_native_compiler_words(
        &image,
        tokens.words(),
        offset,
        config,
    )?;
    if original != native {
        return None;
    }
    let operands = conditional_source_operands(&native, segment, &origin);
    let head = native.first().map(|word| OriginalOperandSource {
        span: word.span(),
        input: match &origin {
            OriginalRegistrySource::Conditional(metadata) => Some(
                crate::signature_scan::scope::SignatureSourceNameInput::OriginalWord(
                    metadata.original_head().name_input().clone(),
                ),
            ),
            OriginalRegistrySource::Selected
            | OriginalRegistrySource::ConditionalNativeSource(_)
            | OriginalRegistrySource::Scoped(_)
            | OriginalRegistrySource::DefinitionMember(_)
            | OriginalRegistrySource::Vendor(_)
            | OriginalRegistrySource::SourceTransitions(_)
            | OriginalRegistrySource::ProducedPrefix(_) => None,
        },
        word: Some(word.clone()),
    });
    Some(OriginalRegistryWords {
        command,
        dialect,
        arguments: values,
        operands,
        origins: (0..native.len())
            .map(InvocationWordOrigin::Written)
            .collect(),
        roles,
        head,
        source: origin,
        captured_values: None,
        operands_preserve_source_lookup: false,
        image,
        config,
        registry: registry.snapshot().semantic_key(),
        context: Some(context.context().clone()),
        definition_parent: None,
    })
}

/// Conditional root-script aliases and moves share one immutable source tape.
/// Known targets and definite absence do not fall through; an unbounded
/// snapshot retains conditional source applicability without a dispatch grant.
fn source_transition_registry_words(
    source: &str,
    config: tcl_lexer::LexerConfig,
    realm: &crate::realm::CommandBindingRealm,
    context: &tcl_registry::model::ContextRegistry,
    segment: &tcl_source::segment::SegmentedCommand,
) -> Option<OriginalRegistryWords> {
    let image = retained_document_image(source, realm, config)?;
    let mut tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::from_image(&image),
        config,
        segment,
    );
    realm.stamp_original_tokens(&mut tokens);
    let advice = realm.original_source_transition_advice(context, &tokens);
    #[cfg(test)]
    trace_original_child_guard(
        "transition.advice",
        segment.argv.first().map(|word| word.span.start()),
        advice.is_some(),
    );
    let result = source_transition_words_with_segment(source, config, context, segment, advice?);
    #[cfg(test)]
    trace_original_child_guard(
        "transition.words",
        segment.argv.first().map(|word| word.span.start()),
        result.is_some(),
    );
    result
}

/// Project a sealed original source receipt through the same role/body owner.
/// The original vector is recaptured in its whole-image address space; a
/// detached script or reconstructed argv cannot supply this correspondence.
pub(crate) fn conditional_lambda_formatting_words(
    source: &str,
    config: tcl_lexer::LexerConfig,
    context: &tcl_registry::model::ContextRegistry,
    segment: &tcl_source::segment::SegmentedCommand,
    advice: &crate::command_binding::OriginalSourceCommandTransitionAdvice,
) -> Option<OriginalRegistryWords> {
    let mut words =
        source_transition_words_with_segment(source, config, context, segment, advice.clone())?;
    words.operands_preserve_source_lookup = false;
    Some(words)
}

pub(crate) fn source_transition_words_from_advice(
    source: &str,
    config: tcl_lexer::LexerConfig,
    context: &tcl_registry::model::ContextRegistry,
    advice: crate::command_binding::OriginalSourceCommandTransitionAdvice,
) -> Option<OriginalRegistryWords> {
    let original = advice.original_words();
    let span = tcl_lexer::Span::new(
        original.first()?.span().start(),
        original.last()?.span().end(),
    );
    let mut segments = tcl_source::segment::segment_commands_with_offset_and_config(
        source.get(span.as_range())?,
        span.start(),
        config.nested(),
    );
    if segments.len() != 1 {
        return None;
    }
    source_transition_words_with_segment(source, config, context, &segments.pop()?, advice)
}

pub(super) fn source_transition_words_with_segment(
    source: &str,
    config: tcl_lexer::LexerConfig,
    context: &tcl_registry::model::ContextRegistry,
    segment: &tcl_source::segment::SegmentedCommand,
    advice: crate::command_binding::OriginalSourceCommandTransitionAdvice,
) -> Option<OriginalRegistryWords> {
    let image = matching_document_image(source, advice.original_words().first()?.image())?;
    source_transition_words_with_segment_in_image(&image, config, context, segment, advice)
}

// The caller must retain the actual source image; no Document conversion is
// permitted for entered materialised script values.
fn source_transition_words_with_segment_in_image(
    image: &tcl_lexer::SourceImage,
    config: tcl_lexer::LexerConfig,
    context: &tcl_registry::model::ContextRegistry,
    segment: &tcl_source::segment::SegmentedCommand,
    advice: crate::command_binding::OriginalSourceCommandTransitionAdvice,
) -> Option<OriginalRegistryWords> {
    use crate::registry_invocation::{EffectiveInvocationWord, InvocationWordOrigin};
    let image = image.clone();
    let tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::from_image(&image),
        config,
        segment,
    );
    if crate::registry_invocation::original_native_compiler_words(
        &image,
        tokens.words(),
        segment.argv.first()?.span.start(),
        config,
    )?
    .as_slice()
        != advice.original_words()
    {
        return None;
    }
    if !advice.matches_context(context) || !advice.matches_source(&image, config) {
        return None;
    }
    let values = advice
        .arguments()
        .iter()
        .map(|argument| {
            if argument.original.group().expand {
                EffectiveInvocationWord::Expanded
            } else {
                argument
                    .value
                    .as_ref()
                    .map_or(EffectiveInvocationWord::Dynamic, |bytes| {
                        EffectiveInvocationWord::ByteLiteral(std::sync::Arc::clone(bytes))
                    })
            }
        })
        .collect();
    let operands = advice
        .arguments()
        .iter()
        .map(|argument| {
            let InvocationWordOrigin::Written(ordinal) = argument.origin else {
                return None;
            };
            let token = segment.argv.get(ordinal)?;
            // The realm authenticated the exact complete native vector.
            // Segmented tokens and grouped original words retain different
            // genuine delimiter extents; both are preserved independently.
            Some(OriginalOperandSource {
                span: token.span,
                input: argument
                    .input
                    .as_ref()
                    .and_then(|input| input.native_input())
                    .cloned(),
                word: Some(argument.original.clone()),
            })
        })
        .collect();
    let mut origins = vec![InvocationWordOrigin::Written(0)];
    origins.extend(advice.arguments().iter().map(|argument| argument.origin));
    let head = advice
        .original_words()
        .first()
        .map(|word| OriginalOperandSource {
            span: word.span(),
            input: advice.original_head().native_input().cloned(),
            word: Some(word.clone()),
        });
    Some(OriginalRegistryWords {
        command: advice.command().to_owned(),
        dialect: Some(advice.dialect()),
        arguments: values,
        operands,
        origins,
        roles: advice.roles().map(<[_]>::to_vec),
        head,
        source: OriginalRegistrySource::SourceTransitions(std::sync::Arc::new(advice)),
        captured_values: None,
        operands_preserve_source_lookup: false,
        image,
        config,
        registry: context.commands().snapshot().semantic_key(),
        context: Some(context.context().clone()),
        definition_parent: None,
    })
}
