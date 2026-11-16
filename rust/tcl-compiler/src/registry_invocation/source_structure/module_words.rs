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

//! Registry words for the tokens of a module, its immediate body commands and Logical commands.

use super::selected_words::{selected_registry_words_with_context, source_declared_command_words};
use super::source_registry_words::source_registry_words;
use super::transition_words::{
    logical_source_registry_words, native_source_registry_words,
    source_transition_words_from_advice,
};
use super::{OriginalRegistrySource, OriginalRegistryWords, original_query_document_image};
use crate::analyser::AnalysisResult;
use tcl_registry::CommandRegistry;

/// Source schema for one retained original complete invocation vector.
/// The current Analysis remains the owner; no fresh source analysis is made.
pub(crate) fn original_segment_for_tokens(
    source: &str,
    analysis: &AnalysisResult,
    tokens: &crate::ir::CommandTokens,
) -> Option<(
    crate::segmenter::SegmentedCommand,
    Vec<tcl_lexer::NativeWord>,
)> {
    let config = analysis.body_lexer_config?;
    let image = original_query_document_image(source, analysis, config);
    if !analysis.matches_original_source_image(&image, config) {
        return None;
    }
    let native = super::super::original_native_compiler_words(
        &image,
        tokens.words(),
        tokens.argv.first()?.start(),
        config,
    )?;
    let start = native.first()?.span().start();
    let end = native.last()?.span().end();
    let text = source.get(usize::try_from(start).ok()?..usize::try_from(end).ok()?)?;
    let mut segments =
        crate::segmenter::segment_commands_with_offset_and_config(text, start, config.nested());
    if segments.len() != 1 {
        return None;
    }
    Some((segments.pop()?, native))
}

/// Selected Registry metadata at the unchanged original complete invocation.
/// This preserves the retained analysis rather than analysing detached text.
pub(crate) fn original_registry_words_for_tokens(
    source: &str,
    analysis: &AnalysisResult,
    tokens: &crate::ir::CommandTokens,
) -> Option<OriginalRegistryWords> {
    let (segment, native) = original_segment_for_tokens(source, analysis, tokens)?;
    let words = source_registry_words(source, analysis, &segment)?;
    (words.head_source()?.word() == native.first()).then_some(words)
}

/// Original source schema for a supplied compiler consumer. The retained
/// realm, full input and exact recorded invocation must agree; a report name
/// or an equal-text parse cannot replace a missing original vector or site.
/// This delegates the same selected/conditional builders used by editor
/// source structure. Source applicability never grants entered dispatch,
/// physical frames, Native arguments, normal results or parameter seeds.
pub(crate) fn original_registry_words_for_supplied_tokens(
    realm: &crate::realm::CommandBindingRealm,
    input: &crate::analyser::ResolvedAnalysisInput,
    tokens: &crate::ir::CommandTokens,
) -> Option<OriginalRegistryWords> {
    let registry = input.borrowed_context_registry().commands();
    let binding = tokens.source_binding.as_ref()?;
    let metadata = binding.original_invocation_metadata_for_input(tokens, input, registry)?;
    let source_purpose = binding.original_source_advice_purpose(tokens, metadata, registry)?;
    let site = binding.invocation_site()?;
    let config = binding.original_lexer_config_for_tokens(tokens)?;
    let image = site.source.source_image();
    if config != input.lexer_config()
        || !realm.matches_resolved_analysis_input(input)
        || !realm.matches_original_source_image(image, config)
    {
        return None;
    }
    let (segment, recorded) = binding.original_recorded_command()?;
    if recorded.words() != tokens.words() || segment.argv.first()?.span.start() != site.offset {
        return None;
    }
    let mut retained_tokens = recorded;
    realm.stamp_original_tokens(&mut retained_tokens);
    let retained = retained_tokens.source_binding.as_ref()?;
    retained.original_invocation_metadata_for_input(&retained_tokens, input, registry)?;
    if retained.invocation_site() != Some(site) || retained_tokens.words() != tokens.words() {
        return None;
    }
    let native =
        super::super::original_native_compiler_words(image, tokens.words(), site.offset, config)?;
    let source = image.try_text().ok()?;
    let context = input.context_registry();
    if realm.original_source_registry_barrier(&context, tokens) {
        return None;
    }
    let selected =
        if source_purpose == crate::command_binding::OriginalInvocationSourcePurpose::Logical {
            logical_source_registry_words(source, realm, input, &segment)
        } else {
            selected_registry_words_with_context(source, config, realm, &segment, &context).or_else(
                || native_source_registry_words(source, config, realm, &context, &segment, None),
            )
        }?;
    if selected.head_source()?.word() != native.first() {
        return None;
    }
    for (argument, origin) in selected.origins().iter().enumerate().skip(1) {
        if let super::super::InvocationWordOrigin::Written(written) = origin
            && selected.operands().get(argument - 1)?.as_ref()?.word() != native.get(*written)
        {
            return None;
        }
    }
    Some(selected)
}

/// Conditional original Registry words under the retained complete Module.
/// Current source attachments and supplied input remain independent of Normal
/// completion, entered handlers, native arguments and executable body topology.
pub(crate) fn original_registry_words_for_module(
    module: &crate::ir::Module,
    registry: &CommandRegistry,
    tokens: &crate::ir::CommandTokens,
) -> Option<OriginalRegistryWords> {
    let input = super::super::InvocationMetadataContext::for_module(registry, module)?
        .source_analysis_input()?;
    let retained = module.retained_source_bindings.as_deref()?;
    retained.positioned_for_module(module, registry)?;
    if tokens.synthetic.is_some()
        || !tokens.original_words_match_argv_text()
        || !retained.owns_original_analysis_attachment(tokens)
        || module.native_lexer_config() != input.lexer_config()
    {
        return None;
    }
    let realm = crate::realm::document_realm_bindings_for_module(module, registry)?;
    original_registry_words_for_supplied_tokens(&realm, input, tokens)
}

/// Conditional source argv under the complete current Logical Module.
/// The retained tape owns deferred child syntax independently of dispatch.
pub(crate) fn original_logical_registry_words_for_module(
    module: &crate::ir::Module,
    registry: &CommandRegistry,
    tokens: &crate::ir::CommandTokens,
) -> Option<OriginalRegistryWords> {
    let input = module.source_metadata_input.as_ref()?;
    if !input.has_logical_source_name_context()
        || module.source_entry.native_entry.is_some()
        || module.source_entry.hosted_execution_context.is_some()
        || module.source_entry.unknown_entry
        || tokens.synthetic.is_some()
        || !tokens.original_words_match_argv_text()
        || module.native_lexer_config() != input.lexer_config()
    {
        return None;
    }
    super::super::InvocationMetadataContext::for_module(registry, module)?;
    let realm = crate::realm::document_realm_bindings_for_module(module, registry)?;
    let binding = tokens.source_binding.as_ref()?;
    if binding.has_original_source_layout_owner() {
        return original_registry_words_for_supplied_tokens(&realm, input, tokens);
    }
    let retained = binding.original_logical_source_words(tokens, input, registry)?;
    let offset = tokens.words().first()?.source().span.start();
    let original = super::super::original_native_compiler_words(
        &module.source,
        tokens.words(),
        offset,
        input.lexer_config(),
    )?;
    let context = input.borrowed_context_registry();
    if realm.original_source_registry_barrier(context, tokens) {
        return None;
    }
    let advice = realm.original_source_transition_advice(context, tokens)?;
    if advice.logical_source_input() != Some(input)
        || advice.original_words() != original.as_slice()
        || !advice.matches_source(&module.source, input.lexer_config())
        || !advice.matches_context(context)
    {
        return None;
    }
    let current = source_transition_words_from_advice(
        module.source.try_text().ok()?,
        input.lexer_config(),
        context,
        advice,
    )?;
    (current == retained).then_some(current)
}

/// Complete immediate Plain-body commands under the unchanged Module owner.
/// Each child retains its own original source point; no body entry is issued.
pub(crate) fn original_immediate_body_commands_for_module(
    module: &crate::ir::Module,
    registry: &CommandRegistry,
    parent: &crate::ir::CommandTokens,
) -> Option<Vec<crate::ir::CommandTokens>> {
    let input = module.source_metadata_input.as_ref()?;
    input.has_logical_source_name_context().then_some(())?;
    let words = original_logical_registry_words_for_module(module, registry, parent)?;
    let context = input.borrowed_context_registry();
    let no_script_body = words.with_source_schema(context, |schema| {
        let (roles, complete) = schema.authored_logical_source_argument_roles();
        complete
            && !roles
                .iter()
                .any(|(_, role)| *role == tcl_registry::ArgRole::Body)
            && schema.authored_source_case_invocation().is_none()
    })?;
    if no_script_body {
        return Some(Vec::new());
    }
    let realm = crate::realm::document_realm_bindings_for_module(module, registry)?;
    let immediate = words.with_source_schema(context, |schema| {
        let mut selected = Vec::new();
        for &(argument, role) in words.roles()? {
            if role != tcl_registry::ArgRole::Body {
                continue;
            }
            if schema.authored_source_script_timing_at(argument)?
                != tcl_registry::ScriptTiming::SameInvocation
            {
                continue;
            }
            if schema.semantics.body_kind != tcl_registry::BodyKind::Plain
                || schema.semantics.body_interpreter != tcl_registry::BodyInterpreter::Current
            {
                return None;
            }
            selected.push(words.operands().get(argument)?.as_ref()?.word()?.span());
        }
        Some(selected)
    })??;
    if immediate.is_empty() {
        return Some(Vec::new());
    }
    let bodies = words.source_script_bodies_for_mutation_coverage(context)?;
    if immediate.iter().any(|span| {
        !bodies.iter().any(|body| {
            span.start() <= body.container_extent_span().start()
                && body.container_extent_span().end() <= span.end()
        })
    }) {
        return None;
    }
    let source = module.source.try_text().ok()?;
    let map = tcl_lexer::SourceMap::from_image(&module.source);
    let mut commands = Vec::new();
    for body in bodies {
        if !immediate.iter().any(|span| {
            span.start() <= body.container_extent_span().start()
                && body.container_extent_span().end() <= span.end()
        }) {
            continue;
        }
        if !body.matches_source(&module.source, input.lexer_config())
            || !body.matches_context(context)
        {
            return None;
        }
        for original in body.original_commands(context)? {
            let segments = crate::segmenter::segment_commands_with_offset_and_config(
                source.get(original.span.as_range())?,
                original.span.start(),
                input.lexer_config(),
            );
            if segments.len() != 1 {
                return None;
            }
            let mut child =
                crate::ir::CommandTokens::from_segmented(&map, input.lexer_config(), &segments[0]);
            realm.stamp_original_tokens(&mut child);
            let selected = original_logical_registry_words_for_module(module, registry, &child)?;
            let offset = selected.head_source()?.word()?.span().start();
            let actual = super::super::original_native_compiler_words(
                &module.source,
                child.words(),
                offset,
                input.lexer_config(),
            )?;
            if actual != original.words {
                return None;
            }
            commands.push(child);
        }
    }
    Some(commands)
}

/// Authored declaration roles at the unchanged complete invocation, with
/// their original declaration owner and no inherited Registry implementation.
pub(crate) fn original_declared_words_for_tokens(
    source: &str,
    analysis: &AnalysisResult,
    tokens: &crate::ir::CommandTokens,
) -> Option<crate::command_binding::OriginalDeclaredCommandWords> {
    let (segment, native) = original_segment_for_tokens(source, analysis, tokens)?;
    let words = source_declared_command_words(source, analysis, &segment)?;
    (words.original_words() == native).then_some(words)
}

/// Positioned Logical source schema from the retained immutable Realm and
/// the exact original command vector. Dynamic values remain unknown; this
/// supplies no Native naming input, dispatch, Normal or lookup preservation.
#[must_use]
pub fn original_logical_registry_words_for_command(
    realm: &crate::realm::CommandBindingRealm,
    input: &crate::analyser::ResolvedAnalysisInput,
    command: &tcl_lexer::NativeScriptCommandWords,
) -> Option<OriginalRegistryWords> {
    // naming.minifier.complete-logical-metadata
    // docs/design/analysis/name-resolution-proofs/minifier-complete-logical-metadata.md
    let image = command.words.first()?.image();
    let source = image.try_text().ok()?;
    let config = input.lexer_config();
    if command
        .words
        .iter()
        .any(|word| word.image() != image || word.config() != config)
    {
        return None;
    }
    let text = source.get(command.span.as_range())?;
    let mut segments = crate::segmenter::segment_commands_with_offset_and_config(
        text,
        command.span.start(),
        config.nested(),
    );
    if segments.len() != 1 {
        return None;
    }
    let segment = segments.pop()?;
    let words = logical_source_registry_words(source, realm, input, &segment)?;
    let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
        return None;
    };
    (advice.original_words() == command.words.as_slice()).then_some(words)
}
