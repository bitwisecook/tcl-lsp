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

//! Registry words of vendor sources, conditional roles and stock source command proposals.

use super::source_registry_words::{trace_remote_source_flag, trace_remote_source_option};
use super::{OriginalOperandSource, OriginalRegistrySource, OriginalRegistryWords};
use crate::analyser::AnalysisResult;

/// Same sealed hosted source argv for pre-dispatch lexical inventory and
/// current Analysis consumers. Metadata, complete original words, context and
/// the authentic token carrier must agree; no handler or body entry follows.
pub(crate) fn original_vendor_registry_words(
    context: &tcl_registry::model::ContextRegistry,
    tokens: &crate::ir::CommandTokens,
    metadata: crate::registry_invocation::OriginalConditionalVendorRegistryMetadata,
) -> Option<OriginalRegistryWords> {
    use crate::registry_invocation::{EffectiveInvocationWord, InvocationWordOrigin};
    let shape = metadata.shape();
    let image = shape.original_head().source_image().clone();
    let config = shape.original_head().lexer_config();
    if shape.context() != context.context()
        || shape.registry() != &context.commands().snapshot().semantic_key()
        || !metadata.matches_source(&image, config)
        || !metadata.catalogue().matches_invocation(tokens)
    {
        return None;
    }
    let offset = tokens.argv.first()?.start();
    let native = crate::registry_invocation::original_native_compiler_words(
        &image,
        tokens.words(),
        offset,
        config,
    )?;
    if native.as_slice() != shape.original_words() || native.len() != tokens.argv.len() {
        return None;
    }
    let arguments = native
        .iter()
        .skip(1)
        .map(|word| {
            if word.group().expand {
                EffectiveInvocationWord::Expanded
            } else {
                tcl_syntax::naming::vendor_source_literal_units(
                    shape.original_head().policy(),
                    word,
                    tcl_syntax::naming::VendorSourceNamePurpose::SourceName,
                )
                .map_or(EffectiveInvocationWord::Dynamic, |bytes| {
                    EffectiveInvocationWord::ByteLiteral(std::sync::Arc::from(bytes))
                })
            }
        })
        .collect();
    let operands = native
        .iter()
        .enumerate()
        .skip(1)
        .map(|(ordinal, word)| {
            Some(OriginalOperandSource {
                span: *tokens.argv.get(ordinal)?,
                input: None,
                word: Some(word.clone()),
            })
        })
        .collect();
    let head = native.first().map(|word| OriginalOperandSource {
        span: word.span(),
        input: None,
        word: Some(word.clone()),
    });
    Some(OriginalRegistryWords {
        command: shape.command().to_owned(),
        dialect: None,
        arguments,
        operands,
        origins: (0..native.len())
            .map(InvocationWordOrigin::Written)
            .collect(),
        roles: conditional_roles(
            shape.roles(),
            shape.argument_offset(),
            shape.roles_complete(),
        ),
        head,
        source: OriginalRegistrySource::Vendor(std::sync::Arc::new(metadata)),
        captured_values: None,
        operands_preserve_source_lookup: false,
        image,
        config,
        registry: context.commands().snapshot().semantic_key(),
        context: Some(context.context().clone()),
        definition_parent: None,
    })
}

pub(super) fn conditional_roles(
    roles: &[(u8, tcl_registry::ArgRole)],
    offset: usize,
    complete: bool,
) -> Option<Vec<(usize, tcl_registry::ArgRole)>> {
    if !complete {
        return None;
    }
    roles
        .iter()
        .map(|&(ordinal, role)| {
            offset
                .checked_add(usize::from(ordinal))
                .map(|index| (index, role))
        })
        .collect()
}

/// Hosted source candidates use their own context/policy and complete retained
/// native vector. This returns conditional Registry syntax only, without a
/// C/Jim name recipe or actual handler/table grant.
pub fn selected_vendor_registry_words_at(
    source: &str,
    analysis: &AnalysisResult,
    cursor: u32,
) -> Option<(
    crate::registry_invocation::OriginalConditionalVendorRegistryMetadata,
    crate::segmenter::SegmentedCommand,
)> {
    let config = analysis.body_lexer_config?;
    analysis
        .matches_original_source_image(&tcl_lexer::SourceImage::document(source), config)
        .then_some(())?;
    let context = analysis.resolved_input.as_ref()?.context_registry();
    let realm = analysis.retained_command_realm()?;
    let mut selected = None;
    let mut selected_width = u32::MAX;
    for occurrence in analysis.original_vendor_source_names() {
        let words = occurrence.original_words();
        if words.first()? != occurrence.name_input().original_word() {
            continue;
        }
        let start = words.first()?.span().start();
        let end = words.last()?.span().end();
        let immediate_next = cursor > end
            && source
                .get(end as usize..cursor as usize)
                .is_some_and(|text| {
                    !text.is_empty() && text.chars().all(|c| c == ' ' || c == '\t')
                });
        if cursor < start || (cursor > end && !immediate_next) {
            continue;
        }
        let width = end.checked_sub(start)?;
        if width < selected_width {
            selected_width = width;
            selected = Some(occurrence);
        } else if width == selected_width && selected != Some(occurrence) {
            return None;
        }
    }
    // Choose the actual innermost original command before querying metadata.
    // Missing/blocked inner metadata cannot borrow an outer script's roles.
    let occurrence =
        trace_remote_source_option("vendor.selected_occurrence", Some(cursor), selected)?;
    let words = occurrence.original_words();
    let start = words.first()?.span().start();
    let end = words.last()?.span().end();
    let text = source.get(start as usize..end as usize)?;
    let mut commands =
        crate::segmenter::segment_commands_with_offset_and_config(text, start, config.nested());
    if !trace_remote_source_flag("vendor.single_command", Some(cursor), commands.len() == 1) {
        return None;
    }
    let command = commands.pop()?;
    let mut tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::new(source),
        config,
        &command,
    );
    realm.stamp_original_tokens(&mut tokens);
    let metadata = trace_remote_source_option(
        "vendor.occurrence_metadata",
        Some(cursor),
        crate::registry_invocation::original_vendor_occurrence_registry_metadata(
            &context, &tokens, occurrence,
        ),
    )?;
    Some((metadata, command))
}

impl OriginalRegistryWords {
    /// Project Registry metadata for a deliberately different, complete stock
    /// source-command proposal under this original source owner's full context.
    /// The original schema must remain authenticated first. Proposed argv owns
    /// a separate complete image/configuration and must pass the canonical word
    /// inventory. It never becomes an original invocation or a detached body's
    /// entry. Stock lookup, operand materialisation, effects and equivalence
    /// remain independent review premises; no Native purpose is supplied.
    pub fn with_stock_source_command_proposal<'r, T>(
        &'r self,
        context: &'r tcl_registry::model::ContextRegistry,
        proposed: &[tcl_lexer::NativeWord],
        project: impl FnOnce(&tcl_registry::ResolvedInvocation<'r, '_>) -> T,
    ) -> Option<T> {
        self.with_source_schema(context, |_| ())?;
        let first = proposed.first()?;
        let image = first.image();
        if image.channel() != tcl_lexer::SourceChannel::Document
            || proposed.iter().any(|word| {
                word.image() != image || word.config() != self.config || word.group().expand
            })
        {
            return None;
        }
        let end = u32::try_from(image.bytes().len()).ok()?;
        let plan = tcl_lexer::native_script_words_in_with_purpose(
            image.clone(),
            tcl_lexer::Span::new(0, end),
            self.config,
            tcl_lexer::SourceSegmentationPurpose::WholeInput,
        )
        .ok()?;
        let [command] = plan.commands.as_slice() else {
            return None;
        };
        if plan.fatal_tail.is_some()
            || command.words.as_slice() != proposed
            || first.span().start() != 0
            || proposed.last()?.span().end() != end
        {
            return None;
        }
        let head_value = tcl_syntax::word_rules::original_static_word_unicode_value(first)?;
        let head = std::str::from_utf8(&head_value).ok()?;
        let values = proposed[1..]
            .iter()
            .map(|word| {
                tcl_syntax::word_rules::original_static_word_unicode_value(word)
                    .and_then(|value| String::from_utf8(value).ok())
            })
            .collect::<Vec<_>>();
        let arguments = values
            .iter()
            .map(|value| {
                value.as_deref().map_or(
                    tcl_registry::InvocationWord::Dynamic,
                    tcl_registry::InvocationWord::Literal,
                )
            })
            .collect::<Vec<_>>();
        let invocation = tcl_registry::InvocationWords::structured(
            tcl_registry::InvocationWord::Literal(head),
            &arguments,
        );
        let invocation = self
            .dialect
            .map_or(invocation, |dialect| invocation.with_dialect(dialect));
        let selected = tcl_registry::model::assembly::resolve_invocation_words_in_context(
            context.commands(),
            Some(context.context()),
            invocation,
        )?;
        Some(project(&selected))
    }
}
