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

//! The registry words of a source command: definition bodies, vendor and native sources.

use super::selected_words::selected_registry_words;
use super::transition_words::{
    logical_source_registry_words, native_source_registry_words,
    source_transition_words_with_segment,
};
use super::vendor_words::{
    conditional_roles, original_vendor_registry_words, selected_vendor_registry_words_at,
};
use super::{
    OriginalOperandSource, OriginalRegistrySource, OriginalRegistryWords, first_tail_command,
    original_query_document_image, retained_document_image, trace_original_source_role_binding,
    trace_original_source_role_option, trace_original_source_role_words,
};
use crate::analyser::AnalysisResult;
#[cfg(test)]
use crate::registry_invocation::source_structure::trace_original_child_guard;

/// Source structure from the selected original argv, or the independently
/// retained conditional schema. The latter keeps its applicability obligations
/// and grants no runtime dispatch, entered frame, compiler or edit permission.
pub fn source_registry_words(
    source: &str,
    analysis: &AnalysisResult,
    segment: &crate::segmenter::SegmentedCommand,
) -> Option<OriginalRegistryWords> {
    source_registry_words_with_definition_parent(source, analysis, segment, None)
}

/// Same source schema issuer with the actual enclosing definition-body receipt.
/// The original full argv must reborrow from that parent's complete body plan.
#[must_use]
pub fn source_registry_words_in_definition_body(
    source: &str,
    analysis: &AnalysisResult,
    segment: &crate::segmenter::SegmentedCommand,
    parent: &super::super::source_scoped_body::OriginalSourceScriptBody,
) -> Option<OriginalRegistryWords> {
    source_registry_words_with_definition_parent(source, analysis, segment, Some(parent))
}

/// Reborrow one exact child command from an authenticated definition-body
/// plan before applying its selected authored source schema. The complete
/// parent command vector remains mandatory; this is not a fresh child entry.
#[must_use]
pub fn source_registry_words_in_definition_body_at(
    source: &str,
    analysis: &AnalysisResult,
    offset: u32,
    parent: &super::super::source_scoped_body::OriginalSourceScriptBody,
) -> Option<OriginalRegistryWords> {
    use super::super::source_scoped_body::OriginalSourceDefinitionMemberDisposition as Disposition;
    let config = analysis.body_lexer_config?;
    let input = analysis.resolved_input.as_ref()?;
    (config == input.lexer_config()).then_some(())?;
    let realm = analysis.retained_command_realm()?;
    realm.matches_resolved_analysis_input(input).then_some(())?;
    let image = retained_document_image(source, realm, config)?;
    analysis
        .matches_original_source_image(&image, config)
        .then_some(())?;
    if !parent.matches_source(&image, config) || !parent.matches_context(&input.context_registry())
    {
        return None;
    }
    let segment = if let Some((segment, _)) = realm
        .invocation_at_source("", offset)
        .original_recorded_command()
    {
        segment
    } else if let Some(segment) = analysis
        .resolved_input
        .as_ref()
        .and_then(|input| realm.original_source_catalogue_segment_at(input, offset))
    {
        segment
    } else {
        let tail = source.get(usize::try_from(offset).ok()?..)?;
        first_tail_command(tail, offset, config.nested())?
    };
    (segment.argv.first()?.span.start() == offset).then_some(())?;
    let context = input.context_registry();
    let members = parent.definition_member_region(&context, parent.content_span())?;
    members.complete_original_commands()?;
    let original = members.original_command_words_at(offset)?;
    let mut tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::from_image(&image),
        config,
        &segment,
    );
    realm.stamp_original_tokens(&mut tokens);
    let selected = crate::registry_invocation::original_native_compiler_words(
        &image,
        tokens.words(),
        offset,
        config,
    )?;
    (selected.as_slice() == original).then_some(())?;
    match members.script_body_disposition(&context, original) {
        Disposition::Unavailable => None,
        Disposition::CompleteNonMember => source_registry_words(source, analysis, &segment)
            .or_else(|| generic_definition_child_source_words(source, analysis, &segment, parent)),
        Disposition::CompleteMember(_) => {
            source_registry_words_in_definition_body(source, analysis, &segment, parent)
        }
    }
}

struct DefinitionChildOperands {
    pub(super) arguments: Vec<crate::registry_invocation::EffectiveInvocationWord>,
    pub(super) operands: Vec<Option<OriginalOperandSource>>,
    pub(super) origins: Vec<crate::registry_invocation::InvocationWordOrigin>,
}

fn original_definition_child_operands(
    original: &[tcl_lexer::NativeWord],
) -> DefinitionChildOperands {
    use crate::registry_invocation::{EffectiveInvocationWord, InvocationWordOrigin};
    let mut arguments = Vec::with_capacity(original.len().saturating_sub(1));
    let mut operands = Vec::with_capacity(arguments.capacity());
    let mut origins = vec![InvocationWordOrigin::Written(0)];
    for (index, word) in original.iter().enumerate().skip(1) {
        let value = if word.group().expand {
            EffectiveInvocationWord::Expanded
        } else if let Some(value) = tcl_syntax::word_rules::original_static_word_unicode_value(word)
        {
            EffectiveInvocationWord::ByteLiteral(std::sync::Arc::from(value))
        } else {
            EffectiveInvocationWord::Dynamic
        };
        arguments.push(value);
        operands.push(Some(OriginalOperandSource {
            span: word.span(),
            input: None,
            word: Some(word.clone()),
        }));
        origins.push(InvocationWordOrigin::Written(index));
    }
    DefinitionChildOperands {
        arguments,
        operands,
        origins,
    }
}

fn original_definition_child_head(
    original: &[tcl_lexer::NativeWord],
) -> Option<OriginalOperandSource> {
    Some(OriginalOperandSource {
        span: original.first()?.span(),
        input: None,
        word: Some(original.first()?.clone()),
    })
}

// naming.source.original-generic-definition-child-registry-roles
// docs/design/analysis/name-resolution-proofs/source-original-generic-definition-child-registry-roles.md
fn generic_definition_child_source_words(
    source: &str,
    analysis: &AnalysisResult,
    segment: &crate::segmenter::SegmentedCommand,
    parent: &super::super::source_scoped_body::OriginalSourceScriptBody,
) -> Option<OriginalRegistryWords> {
    let input = analysis.resolved_input.as_ref()?;
    let config = input.lexer_config();
    let image = retained_document_image(source, analysis.retained_command_realm()?, config)?;
    let context = input.context_registry();
    if analysis.body_lexer_config != Some(config)
        || !analysis.matches_original_source_image(&image, config)
        || !parent.matches_source(&image, config)
        || !parent.matches_context(&context)
    {
        return None;
    }
    let head = segment.argv.first()?;
    let offset = head.span.start();
    let members = parent.definition_member_region(&context, parent.content_span())?;
    let originals = members.complete_original_commands()?;
    let original = members.original_command_words_at(offset)?;
    let tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::from_image(&image),
        config,
        segment,
    );
    let native = crate::registry_invocation::original_native_compiler_words(
        &image,
        tokens.words(),
        offset,
        config,
    )?;
    (native.as_slice() == original && originals.iter().any(|words| words == original))
        .then_some(())?;
    let head_bytes = tcl_syntax::word_rules::original_static_word_unicode_value(original.first()?)?;
    let head_text = std::str::from_utf8(&head_bytes).ok()?;
    let static_values = original
        .iter()
        .skip(1)
        .map(tcl_syntax::word_rules::original_static_word_unicode_value)
        .collect::<Vec<_>>();
    let values = original
        .iter()
        .skip(1)
        .zip(&static_values)
        .map(|(word, value)| {
            if word.group().expand {
                tcl_registry::InvocationWord::Expanded
            } else if let Some(value) = value {
                tcl_registry::InvocationWord::KnownBytes(value.as_slice())
            } else {
                tcl_registry::InvocationWord::Dynamic
            }
        })
        .collect::<Vec<_>>();
    let mut invocation = tcl_registry::InvocationWords::structured(
        tcl_registry::InvocationWord::Literal(head_text),
        &values,
    );
    if let Some(dialect) = parent.source_words().dialect() {
        invocation = invocation.with_dialect(dialect);
    }
    // Registry source roles use the actual retained context's authoring realm;
    // this source-only facet must not manufacture a RuleLoader/runtime realm.
    let selected = tcl_registry::model::assembly::resolve_invocation_words_in_context(
        context.commands(),
        Some(context.context()),
        invocation,
    )?;
    let (selected_roles, complete) = selected.authored_source_argument_roles();
    if !complete {
        return None;
    }
    let roles = selected_roles
        .into_iter()
        .map(|(position, role)| {
            selected
                .semantics
                .argument_offset
                .checked_add(usize::from(position))
                .map(|position| (position, role))
        })
        .collect::<Option<Vec<_>>>()?;
    let DefinitionChildOperands {
        arguments,
        operands,
        origins,
    } = original_definition_child_operands(original);
    let head = original_definition_child_head(original)?;
    Some(OriginalRegistryWords {
        command: selected.canonical_command.to_owned(),
        dialect: parent.source_words().dialect(),
        arguments,
        operands,
        origins,
        head: Some(head),
        roles: Some(roles),
        source: OriginalRegistrySource::Selected,
        captured_values: None,
        operands_preserve_source_lookup: false,
        image,
        config,
        registry: context.commands().snapshot().semantic_key(),
        context: Some(context.context().clone()),
        definition_parent: Some(std::sync::Arc::new(parent.clone())),
    })
}

#[cfg(test)]
fn trace_source_registry_input(
    analysis: &AnalysisResult,
    segment: &crate::segmenter::SegmentedCommand,
) {
    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_CHILD_GUARDS").is_some() {
        let offset = segment.argv.first().map(|word| word.span.start());
        trace_original_child_guard(
            "source.config",
            offset,
            analysis.body_lexer_config.is_some(),
        );
        trace_original_child_guard(
            "source.realm",
            offset,
            analysis.retained_command_realm().is_some(),
        );
        trace_original_child_guard("source.input", offset, analysis.resolved_input.is_some());
        trace_original_child_guard(
            "source.registry",
            offset,
            analysis.resolved_registry().is_some(),
        );
    }
}

fn trace_source_registry_barrier(
    segment: &crate::segmenter::SegmentedCommand,
    tokens: &crate::ir::CommandTokens,
    blocked: bool,
) {
    #[cfg(not(any(debug_assertions, test)))]
    let _ = blocked;
    trace_remote_source_flag(
        "source.binding",
        segment.argv.first().map(|word| word.span.start()),
        tokens.source_binding.is_some(),
    );
    #[cfg(debug_assertions)]
    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_BODY_SYNTAX").is_some() {
        eprintln!(
            "ORIGINAL_SOURCE_ROLE_ROUTE stage=registry.barrier offset={:?} blocked={blocked}",
            segment.argv.first().map(|word| word.span.start())
        );
    }
    #[cfg(test)]
    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_CHILD_GUARDS").is_some() {
        let offset = segment.argv.first().map(|word| word.span.start());
        trace_original_child_guard("source.barrier", offset, !blocked);
        trace_original_child_guard("source.binding", offset, tokens.source_binding.is_some());
    }
}

fn source_vendor_registry_words(
    source: &str,
    analysis: &AnalysisResult,
    segment: &crate::segmenter::SegmentedCommand,
    image: &tcl_lexer::SourceImage,
    config: tcl_lexer::LexerConfig,
    offset: u32,
) -> Option<OriginalRegistryWords> {
    let (metadata, _) = trace_remote_source_option(
        "source.selected_vendor",
        Some(offset),
        selected_vendor_registry_words_at(source, analysis, offset),
    )?;
    let context = analysis.resolved_input.as_ref()?.context_registry();
    let mut tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::from_image(image),
        config,
        segment,
    );
    analysis
        .retained_command_realm()?
        .stamp_original_tokens(&mut tokens);
    // The shared builder authenticates the complete original native
    // vector. Resegmentation's comment trivia is not word identity.
    trace_remote_source_option(
        "source.vendor_words",
        Some(offset),
        original_vendor_registry_words(&context, &tokens, metadata),
    )
}

fn source_native_registry_words(
    source: &str,
    analysis: &AnalysisResult,
    segment: &crate::segmenter::SegmentedCommand,
    geometry: (&tcl_lexer::SourceImage, tcl_lexer::LexerConfig, u32),
    realm: &crate::realm::CommandBindingRealm,
    input: &crate::analyser::ResolvedAnalysisInput,
    tokens: &crate::ir::CommandTokens,
) -> Option<OriginalRegistryWords> {
    let (image, config, offset) = geometry;
    let metadata = analysis
        .original_conditional_registry_metadata_in_source(image, config, offset)
        .cloned();
    let context = analysis.resolved_input.as_ref()?.context_registry();
    trace_original_source_role_words(
        "registry.native",
        Some(offset),
        native_source_registry_words(
            source,
            config,
            analysis.retained_command_realm()?,
            &context,
            segment,
            metadata,
        ),
    )
    .or_else(|| {
        let advice = trace_original_source_role_option(
            "registry.prefix_advice",
            Some(offset),
            realm.original_source_prefix_catalogue_advice(input, tokens),
        )?;
        trace_original_source_role_words(
            "registry.prefix_words",
            Some(offset),
            source_transition_words_with_segment(source, config, &context, segment, advice),
        )
    })
    .or_else(|| {
        let advice = trace_original_source_role_option(
            "registry.member_advice",
            Some(offset),
            realm.original_member_source_catalogue_advice(input, tokens),
        )?;
        trace_original_source_role_words(
            "registry.member_words",
            Some(offset),
            source_transition_words_with_segment(source, config, &context, segment, advice),
        )
    })
}

fn source_registry_words_with_definition_parent(
    source: &str,
    analysis: &AnalysisResult,
    segment: &crate::segmenter::SegmentedCommand,
    parent: Option<&super::super::source_scoped_body::OriginalSourceScriptBody>,
) -> Option<OriginalRegistryWords> {
    #[cfg(test)]
    trace_source_registry_input(analysis, segment);
    let config = analysis.body_lexer_config?;
    let realm = analysis.retained_command_realm()?;
    let image = trace_remote_source_option(
        "source.image",
        segment.argv.first().map(|word| word.span.start()),
        retained_document_image(source, realm, config),
    );
    #[cfg(test)]
    trace_original_child_guard(
        "source.image",
        segment.argv.first().map(|word| word.span.start()),
        image.is_some(),
    );
    let image = image?;
    let matches_image = trace_remote_source_flag(
        "source.image_config",
        segment.argv.first().map(|word| word.span.start()),
        analysis.matches_original_source_image(&image, config),
    );
    #[cfg(test)]
    trace_original_child_guard(
        "source.image_config",
        segment.argv.first().map(|word| word.span.start()),
        matches_image,
    );
    matches_image.then_some(())?;
    // The image check above proves `source` is the analysed document, so a
    // parentless answer is a function of the segment and the stamped inputs.
    if parent.is_none() {
        if let Some(memoised) = analysis.memoised_source_words(segment) {
            return memoised.0;
        }
        let words = source_registry_words_from_image(
            source, analysis, segment, None, &image, config, realm,
        );
        analysis.memoise_source_words(segment, words.as_ref());
        return words;
    }
    source_registry_words_from_image(source, analysis, segment, parent, &image, config, realm)
}

fn source_registry_words_from_image(
    source: &str,
    analysis: &AnalysisResult,
    segment: &crate::segmenter::SegmentedCommand,
    parent: Option<&super::super::source_scoped_body::OriginalSourceScriptBody>,
    image: &tcl_lexer::SourceImage,
    config: tcl_lexer::LexerConfig,
    realm: &crate::realm::CommandBindingRealm,
) -> Option<OriginalRegistryWords> {
    let registry = analysis.resolved_registry()?;
    let input = analysis.resolved_input.as_ref()?;
    let mut tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::from_image(image),
        config,
        segment,
    );
    realm.stamp_original_tokens(&mut tokens);
    #[cfg(debug_assertions)]
    trace_original_source_role_binding(
        "registry",
        segment.argv.first().map(|word| word.span.start()),
        tokens.source_binding.as_ref(),
    );
    let blocked = trace_remote_source_flag(
        "source.blocked",
        segment.argv.first().map(|word| word.span.start()),
        realm.original_source_registry_barrier(&input.context_registry(), &tokens),
    );
    trace_source_registry_barrier(segment, &tokens, blocked);
    if blocked {
        return None;
    }
    if let Some(parent) = parent {
        return original_definition_member_registry_words(source, analysis, segment, parent);
    }
    if let Some(scoped) = trace_original_source_role_words(
        "registry.scoped",
        segment.argv.first().map(|word| word.span.start()),
        original_scoped_registry_words(source, analysis, segment),
    ) {
        return Some(scoped);
    }
    if realm
        .source_bindings_ref()
        .original_logical_source_name_advice_input()
        == Some(input)
    {
        return logical_source_registry_words(source, realm, input, segment);
    }
    if !analysis.has_original_vendor_source_names()
        && let Some(selected) = trace_original_source_role_words(
            "registry.selected",
            segment.argv.first().map(|word| word.span.start()),
            selected_registry_words(source, analysis, segment, registry),
        )
    {
        return Some(selected);
    }
    let offset = segment.argv.first()?.span.start();
    if analysis.has_original_vendor_source_names() {
        return source_vendor_registry_words(source, analysis, segment, image, config, offset);
    }
    source_native_registry_words(
        source,
        analysis,
        segment,
        (image, config, offset),
        realm,
        input,
        &tokens,
    )
}

pub(super) fn source_definition_body_grammar(
    schema: &tcl_registry::ResolvedInvocation<'_, '_>,
) -> Option<&'static tcl_registry::definer::DefinitionBodyGrammar> {
    schema.authored_source_definition_body_grammar()
}

fn original_definition_member_registry_words(
    source: &str,
    analysis: &AnalysisResult,
    segment: &crate::segmenter::SegmentedCommand,
    parent: &super::super::source_scoped_body::OriginalSourceScriptBody,
) -> Option<OriginalRegistryWords> {
    let config = analysis.body_lexer_config?;
    let input = analysis.resolved_input.as_ref()?;
    if config != input.lexer_config() || segment.is_partial {
        return None;
    }
    let context = input.context_registry();
    let realm = analysis.retained_command_realm()?;
    if !realm.matches_resolved_analysis_input(input) {
        return None;
    }
    let image = retained_document_image(source, realm, config)?;
    if !parent.matches_source(&image, config) || !parent.matches_context(&context) {
        return None;
    }
    let region = parent.definition_member_region(&context, parent.content_span())?;
    region.complete_original_commands()?;
    let offset = segment.argv.first()?.span.start();
    let original = region.original_command_words_at(offset)?;
    let mut tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::from_image(&image),
        config,
        segment,
    );
    realm.stamp_original_tokens(&mut tokens);
    let selected = crate::registry_invocation::original_native_compiler_words(
        &image,
        tokens.words(),
        offset,
        config,
    )?;
    if selected != original {
        return None;
    }
    let name = tcl_syntax::word_rules::original_static_word_ascii_presentation(original.first()?)?;
    let name = std::str::from_utf8(&name).ok()?;
    let grammar = parent
        .source_words()
        .with_source_schema(&context, |schema| {
            schema.authored_source_definition_body_grammar()
        })
        .flatten()?;
    context
        .commands()
        .authored_document_member_descriptor(grammar, name)?;
    original_source_schema_words(
        &tokens,
        original,
        name.to_owned(),
        OriginalRegistrySource::DefinitionMember(std::sync::Arc::new(parent.clone())),
        &context,
    )
}

fn original_scoped_registry_words(
    source: &str,
    analysis: &AnalysisResult,
    segment: &crate::segmenter::SegmentedCommand,
) -> Option<OriginalRegistryWords> {
    let config = analysis.body_lexer_config?;
    let realm = analysis.retained_command_realm()?;
    let image = retained_document_image(source, realm, config)?;
    let offset = segment.argv.first()?.span.start();
    let body = analysis.original_scoped_body_in_source(&image, config, offset)?;
    let context = analysis.resolved_input.as_ref()?.context_registry();
    let mut tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::from_image(&image),
        config,
        segment,
    );
    realm.stamp_original_tokens(&mut tokens);
    let original = crate::registry_invocation::original_native_compiler_words(
        &image,
        tokens.words(),
        offset,
        config,
    )?;
    let first = original.first()?;
    let last = original.last()?;
    let extent = body.content_span();
    if first.span().start() < extent.start() || last.span().end() > extent.end() {
        return None;
    }
    let name = tcl_syntax::word_rules::original_static_word_ascii_presentation(first)?;
    let name = std::str::from_utf8(&name).ok()?;
    let schema =
        super::super::source_scoped_body::OriginalSourceScopedCommandSchema::select(body, name)?;
    original_source_schema_words(
        &tokens,
        &original,
        schema.command().name.to_owned(),
        OriginalRegistrySource::Scoped(std::sync::Arc::new(schema)),
        &context,
    )
}

fn original_source_schema_words(
    tokens: &crate::ir::CommandTokens,
    original: &[tcl_lexer::NativeWord],
    command: String,
    source: OriginalRegistrySource,
    context: &tcl_registry::model::ContextRegistry,
) -> Option<OriginalRegistryWords> {
    let first = original.first()?;
    let image = first.image().clone();
    let config = first.config();
    let arguments = original
        .iter()
        .skip(1)
        .map(|word| {
            if word.group().expand {
                crate::registry_invocation::EffectiveInvocationWord::Expanded
            } else {
                let presentation = if matches!(&source, OriginalRegistrySource::DefinitionMember(_))
                {
                    tcl_syntax::word_rules::original_static_word_source_bytes(word)
                } else {
                    tcl_syntax::word_rules::original_static_word_ascii_presentation(word)
                };
                presentation.map_or(
                    crate::registry_invocation::EffectiveInvocationWord::Dynamic,
                    |value| {
                        crate::registry_invocation::EffectiveInvocationWord::ByteLiteral(
                            std::sync::Arc::from(value),
                        )
                    },
                )
            }
        })
        .collect::<Vec<_>>();
    let operands = original
        .iter()
        .skip(1)
        .map(|word| {
            Some(OriginalOperandSource {
                span: word.span(),
                input: None,
                word: Some(word.clone()),
            })
        })
        .collect();
    let head = Some(OriginalOperandSource {
        span: first.span(),
        input: None,
        word: Some(first.clone()),
    });
    let mut words = OriginalRegistryWords {
        command,
        dialect: tokens
            .source_binding
            .as_ref()
            .and_then(|binding| binding.original_source_word_dialect_for_tokens(tokens)),
        arguments,
        operands,
        origins: (0..original.len())
            .map(crate::registry_invocation::InvocationWordOrigin::Written)
            .collect(),
        head,
        roles: None,
        source,
        captured_values: None,
        operands_preserve_source_lookup: false,
        image,
        config,
        registry: context.commands().snapshot().semantic_key(),
        context: Some(context.context().clone()),
        definition_parent: None,
    };
    words.roles = words
        .with_source_schema(context, |schema| {
            let (roles, complete) = schema.authored_source_argument_roles();
            conditional_roles(&roles, schema.semantics.argument_offset, complete)
        })
        .flatten();
    Some(words)
}

/// Source schema at one original invocation offset under the same retained
/// analysis. A stored complete vector is preferred; conditional future/body
/// source advice retains its own full-image issuer through the shared builder.
/// No reporting command string selects the schema or supplies execution.
#[must_use]
pub fn source_registry_words_at(
    source: &str,
    analysis: &AnalysisResult,
    offset: u32,
) -> Option<OriginalRegistryWords> {
    // naming.diagnostic.original-package-source-advice
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-package-source-advice.md
    if let Some(region) = analysis.original_source_region.as_ref()
        && !trace_remote_source_flag(
            "at.region_owns",
            Some(offset),
            region.owns_original_command_at(analysis, offset),
        )
    {
        return None;
    }
    let config = trace_remote_source_option("at.config", Some(offset), analysis.body_lexer_config)?;
    let image = original_query_document_image(source, analysis, config);
    trace_remote_source_flag(
        "at.image_config",
        Some(offset),
        analysis.matches_original_source_image(&image, config),
    )
    .then_some(())?;
    let realm =
        trace_remote_source_option("at.realm", Some(offset), analysis.retained_command_realm())?;
    let binding = realm.invocation_at_source("", offset);
    let segment = if let Some((segment, _)) = binding.original_recorded_command() {
        segment
    } else if let Some(segment) = analysis
        .resolved_input
        .as_ref()
        .and_then(|input| realm.original_source_catalogue_segment_at(input, offset))
    {
        segment
    } else {
        let tail = source.get(usize::try_from(offset).ok()?..)?;
        first_tail_command(tail, offset, config.nested())?
    };
    (segment.argv.first()?.span.start() == offset).then_some(())?;
    trace_remote_source_option(
        "at.source_registry_words",
        Some(offset),
        source_registry_words(source, analysis, &segment),
    )
}

/// Diagnostic identity: reports only the original query result.
#[inline]
pub(in super::super) fn trace_remote_source_option<T>(
    stage: &str,
    offset: Option<u32>,
    value: Option<T>,
) -> Option<T> {
    #[cfg(test)]
    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_REMOTE_FIRST_OPERAND").is_some() {
        eprintln!(
            "REMOTE_SOURCE_WORDS stage={stage} offset={offset:?} present={}",
            value.is_some()
        );
    }
    #[cfg(not(test))]
    let _ = (stage, offset);
    value
}

/// Diagnostic identity: reports only the original predicate result.
#[inline]
pub(in super::super) fn trace_remote_source_flag(
    stage: &str,
    offset: Option<u32>,
    value: bool,
) -> bool {
    #[cfg(test)]
    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_REMOTE_FIRST_OPERAND").is_some() {
        eprintln!("REMOTE_SOURCE_WORDS stage={stage} offset={offset:?} value={value}");
    }
    #[cfg(not(test))]
    let _ = (stage, offset);
    value
}
