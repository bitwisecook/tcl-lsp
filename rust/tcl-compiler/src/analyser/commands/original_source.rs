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

//! The original written source of a command: static names and vendor vectors for naming, and the segments diagnostics anchor on.

use super::super::state::Analyser;
use super::{DispatchSite, original_document_source_image};
use std::sync::Arc;
use tcl_lexer::{Span, Token, TokenType};
use tcl_registry::ArgRole;
use tcl_source::cst::descend::descend_token;
use tcl_source::cst::segment::segments_from_tree;
use tcl_source::segment::SegmentedCommand;

/// A readonly mini-language template from an original whole written word.
/// Registry selection owns its effective ordinal; the operand owns its value
/// and source anchor. Captured prefixes cannot acquire a call-site span.
pub(in crate::analyser) struct OriginalFormatTemplate {
    pub(in crate::analyser) format: tcl_registry::FormatStringArg,
    pub(in crate::analyser) bytes: Vec<u8>,
    pub(in crate::analyser) span: Span,
}

impl Analyser {
    pub(super) fn retain_original_static_source_names(&mut self, argument_tokens: &[Token]) {
        let Some(first) = argument_tokens.first() else {
            return;
        };
        let Some(tokens) = self.retained_invocation_tokens(first.span.start(), argument_tokens)
        else {
            return;
        };
        let bindings = self.head_identities.source_bindings_ref();
        let Some(origin) = bindings.source_origin() else {
            return;
        };
        let image = self.command_document_source_image();
        let config = self.file_lexer_config();
        if !bindings.matches_original_source_image(&image, config)
            || origin.source_image() != &image
            || tokens.argv.len() != argument_tokens.len()
            || tokens
                .argv
                .iter()
                .zip(argument_tokens)
                .any(|(span, token)| *span != token.span)
        {
            return;
        }
        let Some(native) = crate::registry_invocation::original_native_compiler_words(
            &image,
            tokens.words(),
            first.span.start(),
            config,
        ) else {
            return;
        };
        // Only the authoritative executable arena selects substitution
        // scripts. Inert braced text cannot become a lexical child command.
        // Iteration and exact original body spans keep the walk independent
        // of diagnostics, reached-parent lookup and entered runtime frames.
        let mut pending = vec![(native, None)];
        let mut visited = std::collections::HashSet::new();
        while let Some((native, body_origin)) = pending.pop() {
            // Re-entering the same lexical vector through an ordinary handler
            // walk must preserve its already sealed full source ancestry. This
            // is source membership only; no parent lookup is transplanted.
            let body_origin =
                self.retained_original_source_body_origin(&native, body_origin, &image, config);
            self.retain_original_static_source_vector(&native, body_origin.as_ref());
            if let Some(words) =
                self.original_readonly_vendor_source_vector(&native, body_origin.as_ref())
            {
                let context = self.analysis_context();
                let syntax = words.source_script_bodies_for(
                    &context,
                    crate::registry_invocation::OriginalSourceScriptPurpose::Syntax,
                );
                let expressions = self
                    .resolved_input
                    .as_ref()
                    .or(self.result.resolved_input.as_ref())
                    .and_then(|input| words.source_expression_script_bodies(input))
                    .unwrap_or_default();
                for body in syntax.into_iter().chain(expressions) {
                    if !body.matches_source(&image, config)
                        || !body.matches_context(&context)
                        || !visited.insert(body.content_span())
                    {
                        continue;
                    }
                    let Some(origin) = body.vendor_origin() else {
                        continue;
                    };
                    if let Ok(plan) = tcl_lexer::native_script_words_in(
                        image.clone(),
                        body.content_span(),
                        config,
                    ) {
                        pending.extend(
                            plan.commands
                                .into_iter()
                                .map(|command| (command.words, Some(origin.clone()))),
                        );
                    }
                }
            }
            for word in &native {
                for part in word.executable_parts().all_parts() {
                    let tcl_lexer::ExecutablePart::Command { body } = part.part else {
                        continue;
                    };
                    if !visited.insert(body) {
                        continue;
                    }
                    let Ok(plan) = tcl_lexer::native_script_words_in(image.clone(), body, config)
                    else {
                        continue;
                    };
                    // A malformed tail supplies no complete vector; any
                    // independently complete prefix is still readonly syntax.
                    pending.extend(
                        plan.commands
                            .into_iter()
                            .map(|command| (command.words, body_origin.clone())),
                    );
                }
            }
        }
    }

    fn retained_original_source_body_origin(
        &self,
        native: &[tcl_lexer::NativeWord],
        body_origin: Option<Arc<crate::registry_invocation::OriginalSourceScriptBodyOrigin>>,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
    ) -> Option<Arc<crate::registry_invocation::OriginalSourceScriptBodyOrigin>> {
        body_origin.or_else(|| {
            let head = native.first()?;
            let occurrence = self
                .result
                .original_vendor_source_names
                .get(&head.span())?
                .as_ref()?;
            (occurrence.original_words() == native
                && occurrence.name_input().original_word() == head
                && occurrence.site().source.source_image() == image
                && occurrence.name_input().lexer_config() == config)
                .then(|| occurrence.retained_body_origin())
                .flatten()
        })
    }

    /// Seal authored hosted roles for a genuine lexical vector. Unreached
    /// script children may retain source applicability, never parent lookup.
    fn original_readonly_vendor_source_vector(
        &self,
        native: &[tcl_lexer::NativeWord],
        body_origin: Option<
            &std::sync::Arc<crate::registry_invocation::OriginalSourceScriptBodyOrigin>,
        >,
    ) -> Option<crate::registry_invocation::source_structure::OriginalRegistryWords> {
        let first = native.first()?;
        let last = native.last()?;
        let config = self.file_lexer_config();
        let text = self
            .source
            .get(first.span().start() as usize..last.span().end() as usize)?;
        let mut commands = tcl_source::segment::segment_commands_with_offset_and_config(
            text,
            first.span().start(),
            config.nested(),
        );
        if commands.len() != 1 {
            return None;
        }
        let segment = commands.pop()?;
        let mut tokens = crate::ir::CommandTokens::from_segmented(
            &Self::source_map(
                &self.source,
                &self.cached_line_index,
                self.cached_line_index_source_len,
            ),
            config,
            &segment,
        );
        self.head_identities.stamp_original_tokens(&mut tokens);
        let site = crate::command_binding::CommandAllocationSite {
            source: std::sync::Arc::clone(
                self.head_identities.source_bindings_ref().source_origin()?,
            ),
            offset: first.span().start(),
        };
        let occurrence = crate::signature_scan::vendor_name::VendorSourceNameOccurrence::new(
            &site,
            first,
            self.vendor_source_name_policy()?,
            std::sync::Arc::from(native),
        )?
        .with_body_origin(body_origin);
        let context = self.analysis_context();
        let metadata = crate::registry_invocation::original_vendor_occurrence_registry_metadata(
            &context,
            &tokens,
            &occurrence,
        );
        crate::registry_invocation::source_structure::original_vendor_registry_words(
            &context, &tokens, metadata?,
        )
    }

    fn retain_original_static_source_vector(
        &mut self,
        native: &[tcl_lexer::NativeWord],
        body_origin: Option<
            &std::sync::Arc<crate::registry_invocation::OriginalSourceScriptBodyOrigin>,
        >,
    ) {
        // Implementation contract: naming.vendor.original-source-context-input
        // docs/design/analysis/name-resolution-proofs/vendor-original-source-context-input.md
        let bindings = self.head_identities.source_bindings_ref();
        let Some(origin) = bindings.source_origin() else {
            return;
        };
        let image = self.command_document_source_image();
        let config = self.file_lexer_config();
        if !bindings.matches_original_source_image(&image, config)
            || origin.source_image() != &image
            || native
                .iter()
                .any(|word| word.image() != &image || word.config() != config)
        {
            return;
        }
        let Some(head) = native.first() else {
            return;
        };
        // This site retains the complete lexical source vector only. A
        // reached invocation, selected role and installation are independent.
        let site = crate::command_binding::CommandAllocationSite {
            source: std::sync::Arc::clone(origin),
            offset: head.group().span.start(),
        };
        if let Some(policy) = self.vendor_source_name_policy() {
            let original_words: std::sync::Arc<[tcl_lexer::NativeWord]> =
                std::sync::Arc::from(native);
            for word in native {
                let Some(occurrence) =
                    crate::signature_scan::vendor_name::VendorSourceNameOccurrence::new(
                        &site,
                        word,
                        policy,
                        std::sync::Arc::clone(&original_words),
                    )
                else {
                    continue;
                };
                let occurrence = occurrence.with_body_origin(body_origin);
                for span in [
                    Some(word.span()),
                    word.tokens().first().map(|token| token.span),
                ]
                .into_iter()
                .flatten()
                {
                    match self.result.original_vendor_source_names.entry(span) {
                        std::collections::hash_map::Entry::Vacant(entry) => {
                            entry.insert(Some(occurrence.clone()));
                        }
                        std::collections::hash_map::Entry::Occupied(mut entry) => {
                            let combined = entry
                                .get()
                                .as_ref()
                                .and_then(|existing| existing.merge_source_origin(&occurrence));
                            entry.insert(combined);
                            self.result.source_words_memo.clear();
                        }
                    }
                }
            }
        }
        let Some(policy) = self.declaration_name_policy() else {
            return;
        };
        let rules = self.word_rules();
        let purpose =
            crate::signature_scan::original_name::OriginalSourceNamePurpose::for_analysis_input(
                &self.resolved_analysis_input(),
                self.source_analysis_entry.as_deref(),
                policy,
            );
        for word in native {
            let key = purpose.key(word, rules);
            let Some(key) = key else {
                continue;
            };
            let Some(occurrence) =
                crate::signature_scan::original_name::SourceOriginalNameOccurrence::new(&site, key)
            else {
                continue;
            };
            for span in [
                Some(word.span()),
                word.tokens().first().map(|token| token.span),
            ]
            .into_iter()
            .flatten()
            {
                match self.original_static_source_names.entry(span) {
                    std::collections::hash_map::Entry::Vacant(entry) => {
                        entry.insert(Some(occurrence.clone()));
                    }
                    std::collections::hash_map::Entry::Occupied(mut entry) => {
                        if entry.get().as_ref() != Some(&occurrence) {
                            entry.insert(None);
                        }
                    }
                }
            }
        }
    }

    pub(in super::super) fn vendor_source_name_policy(
        &self,
    ) -> Option<tcl_syntax::naming::VendorSourceNamePolicy> {
        let policy = if let Some(policy) = self
            .resolved_input
            .as_ref()
            .and_then(super::super::input::ResolvedAnalysisInput::vendor_source_policy)
        {
            policy
        } else {
            let generation = self.analysis_context();
            let context = tcl_dialect::model::bigip_execution_context::BigIpExecutionContext::for_environment(
                &generation.context().environment.id)?;
            tcl_syntax::naming::VendorSourceNamePolicy::authored(context)?
        };
        match self
            .source_analysis_entry
            .as_deref()
            .and_then(|entry| entry.execution_name_policy)
        {
            Some(tcl_syntax::naming::ExecutionNamePolicy::ObservedBigIp(observed)) => (policy
                .context()
                == tcl_dialect::model::bigip_execution_context::BigIpExecutionContext::TmmIRule)
                .then(|| tcl_syntax::naming::VendorSourceNamePolicy::observed(observed)),
            _ => Some(policy),
        }
    }

    /// Original static syntax occurrence captured from this walk's exact
    /// complete command vector. It supplies no Registry role or execution.
    pub(in super::super) fn original_static_source_name_at_span(
        &self,
        span: Span,
    ) -> Option<&crate::signature_scan::original_name::SourceOriginalNameOccurrence> {
        self.original_static_source_names.get(&span)?.as_ref()
    }

    pub(super) fn retained_argument_role_assistance(
        &self,
        invocation_offset: u32,
        argument_tokens: &[Token],
    ) -> Vec<(usize, ArgRole)> {
        let generation = self.analysis_context();
        let registry = generation.commands();
        let Some(tokens) = self.retained_invocation_tokens(invocation_offset, argument_tokens)
        else {
            return Vec::new();
        };
        let mut roles = crate::registry_invocation::invocation_argument_role_assistance(
            registry,
            generation.as_ref(),
            &self.command_surface(registry),
            &tokens,
        );
        // naming.source.native-baseline-conditional-source-roles
        // docs/design/analysis/name-resolution-proofs/native-baseline-conditional-source-roles.md
        let context = generation;
        if let Some(advice) = self
            .head_identities
            .original_source_transition_advice(&context, &tokens)
            && let Some(words) =
                crate::registry_invocation::source_structure::source_transition_words_from_advice(
                    &self.source,
                    self.file_lexer_config(),
                    &context,
                    advice,
                )
        {
            for role in words.written_argument_roles() {
                if !roles.contains(&role) {
                    roles.push(role);
                }
            }
        }
        roles.sort_by_key(|(ordinal, _)| *ordinal);
        roles
    }

    pub(super) fn retained_argument_role_consensus(
        &self,
        invocation_offset: u32,
        argument_tokens: &[Token],
    ) -> Vec<(usize, ArgRole)> {
        let generation = self.analysis_context();
        let registry = generation.commands();
        let Some(tokens) = self.retained_invocation_tokens(invocation_offset, argument_tokens)
        else {
            return Vec::new();
        };
        crate::registry_invocation::invocation_argument_role_consensus(
            registry,
            self.analysis_context().as_ref(),
            &tokens,
        )
    }

    fn original_diagnostic_segment(
        &self,
        site: &DispatchSite<'_>,
    ) -> Option<(SegmentedCommand, tcl_lexer::NativeWord)> {
        self.original_source_segment(
            site.cmd_tok.span.start(),
            site.arg_tokens,
            site.presubstituted_args,
        )
    }

    /// Complete source geometry is shared by naming producers and diagnostics.
    /// A produced argv never acquires the source vector of its building call.
    pub(super) fn original_source_segment(
        &self,
        offset: u32,
        argument_tokens: &[Token],
        presubstituted: bool,
    ) -> Option<(SegmentedCommand, tcl_lexer::NativeWord)> {
        if presubstituted {
            return None;
        }
        let tokens = self.retained_invocation_tokens(offset, argument_tokens)?;
        let image = original_document_source_image(
            &self.source,
            self.head_identities.original_source_image(),
        );
        let config = self.file_lexer_config();
        // Reuse lexical geometry only. Every downstream binding, input and
        // declaration guard still runs against its current original inventory.
        let prepared = if let Some(origin) = self
            .head_identities
            .source_bindings_ref()
            .source_origin()
            .filter(|origin| origin.source_image() == &image)
        {
            Some(origin.original_lexical_command(tokens.words(), offset, config)?)
        } else {
            None
        };
        let native: std::borrow::Cow<'_, [tcl_lexer::NativeWord]> =
            if let Some(prepared) = prepared.as_ref() {
                std::borrow::Cow::Borrowed(prepared.native_words())
            } else {
                std::borrow::Cow::Owned(crate::registry_invocation::original_native_compiler_words(
                    &image,
                    tokens.words(),
                    offset,
                    config,
                )?)
            };
        let first = native.first()?;
        let last = native.last()?;
        let source = self
            .source
            .get(first.span().start() as usize..last.span().end() as usize)?;
        if let Some(prepared) = prepared.as_ref() {
            // Capture already parsed this exact region and configured complete
            // vector. Its immutable CST carries no lookup or Native admission.
            return Some((prepared.command().clone(), first.clone()));
        }
        let mut segments = tcl_source::segment::segment_commands_with_offset_and_config(
            source,
            first.span().start(),
            config.nested(),
        );
        if segments.len() != 1 {
            return None;
        }
        let segment = segments.pop()?;
        Some((segment, first.clone()))
    }

    /// Selected original Registry or declared syntax under the actual input.
    /// This is a source naming purpose; it supplies no successful dispatch.
    pub(super) fn original_name_source(
        &self,
        offset: u32,
        argument_tokens: &[Token],
        presubstituted: bool,
    ) -> Option<super::super::diagnostic_registry::OriginalDiagnosticSource> {
        use super::super::diagnostic_registry::{
            OriginalDiagnosticInvocation, OriginalDiagnosticSource,
        };
        let (segment, first) =
            self.original_source_segment(offset, argument_tokens, presubstituted)?;
        if let Some(words) = crate::registry_invocation::source_structure::source_registry_words(
            &self.source,
            &self.result,
            &segment,
        ) {
            if words.head_source()?.word()? != &first {
                return None;
            }
            return OriginalDiagnosticInvocation::new(words, self.analysis_context())
                .map(OriginalDiagnosticSource::Registry);
        }
        let declared = crate::registry_invocation::source_structure::source_declared_command_words(
            &self.source,
            &self.result,
            &segment,
        )?;
        (declared.original_words().first()? == &first)
            .then(|| OriginalDiagnosticSource::Declared(Arc::new(declared)))
    }

    /// Selected original Registry source syntax under the actual full context.
    pub(super) fn original_diagnostic_invocation(
        &self,
        site: &DispatchSite<'_>,
    ) -> Option<super::super::diagnostic_registry::OriginalDiagnosticInvocation> {
        let (segment, first) = self.original_diagnostic_segment(site)?;
        let words = crate::registry_invocation::source_structure::source_registry_words(
            &self.source,
            &self.result,
            &segment,
        )?;
        if words.head_source()?.word()? != &first {
            return None;
        }
        super::super::diagnostic_registry::OriginalDiagnosticInvocation::new(
            words,
            self.analysis_context(),
        )
    }

    pub(in crate::analyser) fn original_diagnostic_call_at(
        &self,
        cmd_tok: Token,
        argument_tokens: &[Token],
    ) -> Option<super::super::diagnostic_registry::OriginalDiagnosticInvocation> {
        match self.original_name_source(cmd_tok.span.start(), argument_tokens, false)? {
            super::super::diagnostic_registry::OriginalDiagnosticSource::Registry(original) => {
                Some(original)
            }
            super::super::diagnostic_registry::OriginalDiagnosticSource::Declared(_) => None,
        }
    }

    pub(in crate::analyser) fn original_substitution_call(
        &self,
        word: &tcl_lexer::NativeWord,
    ) -> Option<super::super::diagnostic_registry::OriginalDiagnosticInvocation> {
        let [token] = word.tokens() else {
            return None;
        };
        if token.kind != TokenType::Cmd || word.image().bytes() != self.source.as_bytes() {
            return None;
        }
        let sm = Self::source_map(
            &self.source,
            &self.cached_line_index,
            self.cached_line_index_source_len,
        );
        let descended = descend_token(&sm, *token, word.config());
        let mut segments = segments_from_tree(descended.tree(), &sm);
        if segments.len() != 1 {
            return None;
        }
        let segment = segments.pop()?;
        let selected = crate::registry_invocation::source_structure::source_registry_words(
            &self.source,
            &self.result,
            &segment,
        )?;
        super::super::diagnostic_registry::OriginalDiagnosticInvocation::new(
            selected,
            self.analysis_context(),
        )
    }

    pub(super) fn original_declared_diagnostic_invocation(
        &self,
        site: &DispatchSite<'_>,
    ) -> Option<crate::command_binding::OriginalDeclaredCommandWords> {
        let (segment, first) = self.original_diagnostic_segment(site)?;
        let words = crate::registry_invocation::source_structure::source_declared_command_words(
            &self.source,
            &self.result,
            &segment,
        )?;
        (words.original_words().first()? == &first).then_some(words)
    }

    pub(super) fn original_diagnostic_source(
        &self,
        site: &DispatchSite<'_>,
    ) -> Option<super::super::diagnostic_registry::OriginalDiagnosticSource> {
        use super::super::diagnostic_registry::OriginalDiagnosticSource;
        self.original_diagnostic_invocation(site)
            .map(OriginalDiagnosticSource::Registry)
            .or_else(|| {
                self.original_declared_diagnostic_invocation(site)
                    .map(|words| OriginalDiagnosticSource::Declared(std::sync::Arc::new(words)))
            })
    }

    pub(in super::super) fn original_diagnostic_source_for_segment(
        &self,
        segment: &SegmentedCommand,
    ) -> Option<super::super::diagnostic_registry::OriginalDiagnosticSource> {
        let command_token = *segment.argv.first()?;
        self.original_diagnostic_source(&DispatchSite {
            cmd_name: segment.texts.first()?,
            args: segment.texts.get(1..)?,
            arg_tokens: segment.argv.get(1..)?,
            arg_single: segment.single_token_word.get(1..)?,
            arg_expand_in: segment.expand_word.as_deref().unwrap_or(&[]),
            cmd_tok: command_token,
            scope_path: &[],
            presubstituted_args: self.presubstituted_args,
        })
    }

    pub(in crate::analyser) fn original_format_templates(
        original: Option<&super::super::diagnostic_registry::OriginalDiagnosticInvocation>,
    ) -> Vec<OriginalFormatTemplate> {
        original.map_or_else(
            Vec::new,
            super::super::diagnostic_registry::OriginalDiagnosticInvocation::format_templates,
        )
    }
}
