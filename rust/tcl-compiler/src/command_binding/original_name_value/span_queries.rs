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

//! Original written words and names at a source span of a completed interpretation.

use crate::signature_scan::scope::{SignatureSourceNameInput, SignatureSourceNameValue};
use std::sync::Arc;

impl super::super::SourceCommandBindings {
    pub(crate) fn retain_original_registered_instance_source(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        receipt: super::super::OriginalSourceRegisteredInstanceWords,
    ) -> Option<()> {
        let owner = self.original_analysis_owner.as_ref()?;
        if !owner.matches(self, input)
            || !std::sync::Arc::ptr_eq(&receipt.site().source, &owner.source)
            || !receipt.matches_source_context(
                owner.source.source_image(),
                owner.config,
                input.borrowed_context_registry(),
            )
        {
            return None;
        }
        let cache = &self.original_registered_instance_sources;
        let memo = owner.derived_memo();
        let linked = memo
            .registered_instance_sources
            .get_or_init(|| std::sync::Arc::downgrade(cache));
        if !std::sync::Weak::ptr_eq(linked, &std::sync::Arc::downgrade(cache)) {
            return None;
        }
        let mut entries = cache
            .entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match entries.entry(receipt.site().offset) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(Some(std::sync::Arc::new(receipt)));
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                if entry.get().as_deref() != Some(&receipt) {
                    entry.insert(None);
                }
            }
        }
        Some(())
    }

    /// Whether the retained root analysis has this exact source image, input
    /// channel and complete lexer configuration. This is correspondence only;
    /// it supplies no completed-source, current value or invocation receipt.
    #[must_use]
    pub fn matches_original_source_image(
        &self,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
    ) -> bool {
        self.lexer_config == Some(config)
            && self
                .root_origin
                .as_ref()
                .is_some_and(|origin| origin.source_image() == image)
    }

    /// One complete retained Logical command vector, including independent
    /// receiver source catalogues. The existing lexical owner reauthenticates
    /// every original word and the exact source/configuration/site. This grants
    /// readonly source geometry, without declaration, dispatch or Native entry.
    pub(crate) fn original_logical_source_command_at_in_source(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        image: &tcl_lexer::SourceImage,
        offset: u32,
        config: tcl_lexer::LexerConfig,
    ) -> Option<std::sync::Arc<crate::registry_invocation::OriginalSourceCommandProjection>> {
        if !input.has_logical_source_name_context()
            || self.original_logical_source_name_advice_input() != Some(input)
            || input.lexer_config() != config
            || !self.matches_original_source_image(image, config)
        {
            return None;
        }
        let origin = self.source_origin()?;
        let binding = self.invocation_at_source("", offset);
        if binding.original_written_words.is_some()
            || binding.original_compiler_words.is_some()
            || binding.declaration_layout_observations.is_some()
        {
            let site = binding.invocation_site()?;
            if !std::sync::Arc::ptr_eq(&site.source, origin) || site.offset != offset {
                return None;
            }
            let (words, retained_config) = binding.original_logical_source_geometry_words(input)?;
            (retained_config == config).then_some(())?;
            return origin.original_lexical_command(words, offset, config);
        }
        self.original_source_transition_advice_tape_for(
            input.borrowed_context_registry(),
            super::super::OriginalSourceCataloguePurpose::CompletePrefix,
        )?
        .logical_original_command_at(origin, input, offset)
    }

    /// Require the complete caller image, including its input channel, before
    /// projecting a retained written value at a source extent.
    #[must_use]
    pub fn original_written_name_input_at_span_in_source(
        &self,
        image: &tcl_lexer::SourceImage,
        span: tcl_lexer::Span,
        config: tcl_lexer::LexerConfig,
    ) -> Option<SignatureSourceNameInput> {
        self.matches_original_source_image(image, config)
            .then_some(())?;
        self.original_written_name_input_at_span(span, config)
    }

    /// Original written input at an exact retained word or first-token extent.
    /// The authentic dispatch point supplies the complete frozen argv and
    /// current value context; a span alone cannot create a producer or word.
    #[must_use]
    pub fn original_written_name_input_at_span(
        &self,
        span: tcl_lexer::Span,
        config: tcl_lexer::LexerConfig,
    ) -> Option<SignatureSourceNameInput> {
        self.original_written_name_input_at_span_with(span, config, None)
    }

    /// [`Self::original_written_name_input_at_span`] reading the owning realm's
    /// site memo, so each site's binding is joined once rather than per query.
    pub(crate) fn original_written_name_input_at_span_with(
        &self,
        span: tcl_lexer::Span,
        config: tcl_lexer::LexerConfig,
        memo: Option<&OriginalRealmQueryMemo>,
    ) -> Option<SignatureSourceNameInput> {
        let mut found = None;
        for (input, _, _) in
            self.original_written_name_observations_at_span_with(span, config, memo)?
        {
            if found.as_ref().is_some_and(|previous| previous != &input) {
                return None;
            }
            found = Some(input);
        }
        found
    }

    /// Original operand and the actual caller's namespace geometry at the same
    /// complete retained invocation. This is readonly source correspondence;
    /// it grants no selected argument role, namespace existence or completion.
    #[must_use]
    pub fn original_namespace_operand_at_span_in_source(
        &self,
        image: &tcl_lexer::SourceImage,
        span: tcl_lexer::Span,
        config: tcl_lexer::LexerConfig,
    ) -> Option<crate::signature_scan::original_name::SourceOriginalNamespaceOperand> {
        self.original_namespace_operand_at_span_in_source_with(image, span, config, None)
    }

    /// As [`Self::original_namespace_operand_at_span_in_source`], sharing the
    /// per-site geometry of one immutable realm through `memo`.
    pub(crate) fn original_namespace_operand_at_span_in_source_with(
        &self,
        image: &tcl_lexer::SourceImage,
        span: tcl_lexer::Span,
        config: tcl_lexer::LexerConfig,
        memo: Option<&OriginalRealmQueryMemo>,
    ) -> Option<crate::signature_scan::original_name::SourceOriginalNamespaceOperand> {
        self.matches_original_source_image(image, config)
            .then_some(())?;
        let logical = self.original_logical_source_name_advice_input();
        let tape = match logical {
            Some(input) => Some(self.original_source_transition_advice_tape_for(
                input.borrowed_context_registry(),
                super::super::OriginalSourceCataloguePurpose::CompletePrefix,
            )?),
            None => None,
        };
        let mut found = None;
        for (input, binding, site) in
            self.original_written_name_observations_at_span_with(span, config, memo)?
        {
            let scope = match (logical, tape.as_ref()) {
                (Some(owner), Some(tape)) => tape
                    .logical_variable_scope_at(owner, image, span, &input)?
                    .namespace(),
                (None, None) => binding.original_operand_naming_scope(&input)?,
                _ => return None,
            };
            let operand = crate::signature_scan::original_name::SourceOriginalNamespaceOperand::new(
                input.clone(),
                scope,
                site,
            );
            if found.as_ref().is_some_and(|previous| previous != &operand) {
                return None;
            }
            found = Some(operand);
        }
        found
    }

    fn original_written_name_observations_at_span_with(
        &self,
        span: tcl_lexer::Span,
        config: tcl_lexer::LexerConfig,
        memo: Option<&OriginalRealmQueryMemo>,
    ) -> Option<
        Vec<(
            SignatureSourceNameInput,
            super::super::SourceInvocationBinding,
            super::super::CommandAllocationSite,
        )>,
    > {
        self.original_written_word_observations_at_span_with(span, config, memo)?
            .into_iter()
            .map(|(word, written, binding, site)| {
                let input = if let Some((words, retained_config)) =
                    binding.original_recorded_name_words()
                {
                    binding.original_recorded_written_name_input(written, words, retained_config)?
                } else if binding.declaration_layout_observations.is_some() {
                    binding.original_logical_source_written_name_input(
                        self.original_logical_source_name_advice_input()?, written,
                    )?
                } else {
                    let owner = self.original_logical_source_name_advice_input()?;
                    let policy = tcl_native_model::InvocationDialect::of_profile(owner.analyser_profile())
                        .authored_name_policy()?;
                    if tcl_native_model::InvocationDialect::of_profile(owner.unit_profile())
                        .authored_name_policy() != Some(policy) { return None; }
                    let purpose = crate::signature_scan::original_name::OriginalSourceNamePurpose::for_analysis_input(
                        owner, None, policy,
                    );
                    if !matches!(purpose, crate::signature_scan::original_name::OriginalSourceNamePurpose::Logical(_, _)) {
                        return None;
                    }
                    SignatureSourceNameInput::OriginalWord(purpose.key(
                        &word, tcl_syntax::word_rules::WordValueRules::from_config(&config),
                    )?)
                };
                Some((input, binding, site))
            })
            .collect()
    }

    /// Original C array-name root with independently unknown compound index.
    /// Complete image/configuration/argv correspondence is retained; this
    /// projection supplies no whole argv value, element cell or compiler gate.
    #[must_use]
    pub fn original_c_array_operand_root_at_span_in_source(
        &self,
        image: &tcl_lexer::SourceImage,
        span: tcl_lexer::Span,
        config: tcl_lexer::LexerConfig,
        policy: tcl_syntax::naming::NamePolicyProtocol,
    ) -> Option<SignatureSourceNameInput> {
        self.original_c_array_operand_root_at_span_in_source_with(image, span, config, policy, None)
    }

    /// As [`Self::original_c_array_operand_root_at_span_in_source`], sharing
    /// the per-site geometry of one immutable realm through `memo`.
    pub(crate) fn original_c_array_operand_root_at_span_in_source_with(
        &self,
        image: &tcl_lexer::SourceImage,
        span: tcl_lexer::Span,
        config: tcl_lexer::LexerConfig,
        policy: tcl_syntax::naming::NamePolicyProtocol,
        memo: Option<&OriginalRealmQueryMemo>,
    ) -> Option<SignatureSourceNameInput> {
        self.matches_original_source_image(image, config)
            .then_some(())?;
        let mut selected = None;
        for (word, _, binding, _) in
            self.original_written_word_observations_at_span_with(span, config, memo)?
        {
            if binding
                .variable_context
                .execution_name_policy
                .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
                != Some(policy)
            {
                return None;
            }
            let input = SignatureSourceNameInput::OriginalValue(
                SignatureSourceNameValue::from_original_variable_operand_root(
                    &word,
                    tcl_syntax::word_rules::WordValueRules::from_config(&config),
                    policy,
                )?,
            );
            if selected.as_ref().is_some_and(|previous| previous != &input) {
                return None;
            }
            selected = Some(input);
        }
        selected
    }

    /// Unanimous genuine whole source word and its original parent head.
    /// Complete source/channel/configuration correspondence is required. This
    /// geometry supplies no value, selected schema, name, cell or execution.
    #[must_use]
    pub fn original_written_word_at_span_in_source(
        &self,
        image: &tcl_lexer::SourceImage,
        span: tcl_lexer::Span,
        config: tcl_lexer::LexerConfig,
    ) -> Option<(tcl_lexer::NativeWord, u32)> {
        self.original_written_word_at_span_in_source_with(image, span, config, None)
    }

    /// As [`Self::original_written_word_at_span_in_source`], sharing the
    /// per-site geometry of one immutable realm through `memo`.
    pub(crate) fn original_written_word_at_span_in_source_with(
        &self,
        image: &tcl_lexer::SourceImage,
        span: tcl_lexer::Span,
        config: tcl_lexer::LexerConfig,
        memo: Option<&OriginalRealmQueryMemo>,
    ) -> Option<(tcl_lexer::NativeWord, u32)> {
        self.matches_original_source_image(image, config)
            .then_some(())?;
        let mut selected = None;
        for (word, _, _, site) in
            self.original_written_word_observations_at_span_with(span, config, memo)?
        {
            (site.source.source_image() == image).then_some(())?;
            let current = (word, site.offset);
            if selected
                .as_ref()
                .is_some_and(|previous| previous != &current)
            {
                return None;
            }
            selected = Some(current);
        }
        selected
    }

    /// Contents the interpretation froze for the whole written word at `span`,
    /// when every dispatch that reached its command agrees on them. This is
    /// advice for declaration records and diagnostics: it grants no dispatch,
    /// store or edit authority.
    pub(crate) fn frozen_written_word_value_at_span_in_source_with(
        &self,
        image: &tcl_lexer::SourceImage,
        span: tcl_lexer::Span,
        config: tcl_lexer::LexerConfig,
        memo: Option<&OriginalRealmQueryMemo>,
    ) -> Option<String> {
        self.matches_original_source_image(image, config)
            .then_some(())?;
        let mut selected: Option<String> = None;
        for (_, ordinal, binding, site) in
            self.original_written_word_observations_at_span_with(span, config, memo)?
        {
            (site.source.source_image() == image).then_some(())?;
            // A declaration preview runs a body no call has entered yet, under
            // the command world of its declaration rather than of a call.
            self.dispatch_points_at(site.offset)
                .any(|point| !point.declaration_preview)
                .then_some(())?;
            let value = match ordinal.checked_sub(1) {
                Some(argument) => binding.evaluated_written_argument_value(argument),
                None => binding.evaluated_command_word(),
            }?;
            if selected
                .as_deref()
                .is_some_and(|previous| previous != value)
            {
                return None;
            }
            selected = Some(value.to_owned());
        }
        selected
    }

    /// Genuine ordinal-zero original whole head at this source occurrence.
    /// Token/content extents do not identify the head's ordinal or site. This
    /// geometry grants no handler, lookup, value, class or edit authority.
    #[must_use]
    pub fn original_written_head_at_span_in_source(
        &self,
        image: &tcl_lexer::SourceImage,
        span: tcl_lexer::Span,
        config: tcl_lexer::LexerConfig,
    ) -> Option<(tcl_lexer::NativeWord, super::super::CommandAllocationSite)> {
        self.original_written_head_at_span_in_source_with(image, span, config, None)
    }

    /// As [`Self::original_written_head_at_span_in_source`], sharing the
    /// per-site geometry of one immutable realm through `memo`.
    pub(crate) fn original_written_head_at_span_in_source_with(
        &self,
        image: &tcl_lexer::SourceImage,
        span: tcl_lexer::Span,
        config: tcl_lexer::LexerConfig,
        memo: Option<&OriginalRealmQueryMemo>,
    ) -> Option<(tcl_lexer::NativeWord, super::super::CommandAllocationSite)> {
        self.matches_original_source_image(image, config)
            .then_some(())?;
        let mut selected = None;
        for (word, ordinal, _, site) in
            self.original_written_word_observations_at_span_with(span, config, memo)?
        {
            if ordinal != 0 {
                continue;
            }
            (site.source.source_image() == image).then_some(())?;
            let current = (word, site);
            if selected
                .as_ref()
                .is_some_and(|previous| previous != &current)
            {
                return None;
            }
            selected = Some(current);
        }
        selected
    }

    /// Every root-origin site at or before `span`, latest first. Only a site
    /// whose retained geometry covers `span` can contribute or refuse; the
    /// memo pre-computes each site's binding and covering extents once.
    pub(super) fn original_written_word_observations_at_span_with(
        &self,
        span: tcl_lexer::Span,
        config: tcl_lexer::LexerConfig,
        memo: Option<&OriginalRealmQueryMemo>,
    ) -> Option<Vec<OriginalWrittenWordObservation>> {
        let origin = self.root_origin.as_ref()?;
        let mut found = Vec::new();
        if let Some(memo) = memo {
            let sites = memo
                .written_word_sites
                .get_or_init(|| self.original_written_word_sites())
                .as_ref()?;
            let upper = sites.partition_point(|site| site.offset <= span.start());
            for site in sites[..upper].iter().rev() {
                if site.written_end_through < span.end() {
                    break;
                }
                if site.config != config
                    || !site.written.iter().any(|written| {
                        written.start() <= span.start() && span.end() <= written.end()
                    })
                {
                    continue;
                }
                match self.original_written_word_site_step(
                    origin,
                    site.offset,
                    &site.binding,
                    span,
                    config,
                ) {
                    OriginalWrittenWordSiteStep::Skip => {}
                    OriginalWrittenWordSiteStep::Refuse => return None,
                    OriginalWrittenWordSiteStep::Found(word, written, call) => {
                        found.push((*word, written, site.binding.clone(), call));
                    }
                }
            }
        } else {
            for offset in self
                .original_written_word_site_offsets(origin, Some(span.start()))
                .iter()
                .rev()
            {
                let binding = self.original_written_word_site_binding(origin, *offset);
                match self.original_written_word_site_step(origin, *offset, &binding, span, config)
                {
                    OriginalWrittenWordSiteStep::Skip => {}
                    OriginalWrittenWordSiteStep::Refuse => return None,
                    OriginalWrittenWordSiteStep::Found(word, written, call) => {
                        found.push((*word, written, binding, call));
                    }
                }
            }
        }
        if found.is_empty()
            && let Some(input) = self.original_logical_source_name_advice_input()
        {
            if config != input.lexer_config() {
                return None;
            }
            let tape = self.original_source_transition_advice_tape_for(
                input.borrowed_context_registry(),
                super::super::OriginalSourceCataloguePurpose::CompletePrefix,
            )?;
            for (word, written, site) in
                tape.logical_original_word_observations_at_span(origin, input, span)?
            {
                let binding = self.invocation_at_source("", site.offset);
                // A present original owner stays terminal even when its checks
                // refuse. Independent body geometry only fills an absent facet.
                if binding.original_written_words.is_some()
                    || binding.original_compiler_words.is_some()
                    || binding.declaration_layout_observations.is_some()
                {
                    continue;
                }
                found.push((word, written, binding, site));
            }
        }
        (!found.is_empty()).then_some(found)
    }

    /// Root-origin dispatch offsets, plus Logical declaration-layout sites,
    /// at or before `limit` (every one when `None`), ascending.
    fn original_written_word_site_offsets(
        &self,
        origin: &Arc<super::super::SourceOriginId>,
        limit: Option<u32>,
    ) -> std::collections::BTreeSet<u32> {
        let limit = limit.unwrap_or(u32::MAX);
        let mut offsets = self
            .dispatch_points
            .range(..=limit)
            .map(|(offset, _)| *offset)
            .collect::<std::collections::BTreeSet<_>>();
        if self.original_logical_source_name_advice_input().is_some() {
            offsets.extend(self.declaration_layouts.keys().filter_map(|site| {
                (site.source == *origin && site.offset <= limit).then_some(site.offset)
            }));
        }
        offsets
    }

    fn original_written_word_site_binding(
        &self,
        origin: &Arc<super::super::SourceOriginId>,
        offset: u32,
    ) -> super::super::SourceInvocationBinding {
        self.attach_invocation_reads(
            Self::query_dispatch_points(
                self.dispatch_points_at(offset)
                    .filter(|point| point.state.current_source_origin.as_ref() == Some(origin)),
            ),
            Some(origin),
            offset,
        )
    }

    fn original_written_word_site_geometry<'b>(
        &self,
        binding: &'b super::super::SourceInvocationBinding,
    ) -> Option<(&'b [tcl_source::words::WordExpr], tcl_lexer::LexerConfig)> {
        if let Some(input) = self.original_logical_source_name_advice_input() {
            binding.original_logical_source_geometry_words(input)
        } else {
            binding.original_recorded_name_words()
        }
    }

    /// Every site's binding and covering written extents, independent of any
    /// queried span. A site without retained geometry never contributes.
    fn original_written_word_sites(&self) -> Option<Vec<OriginalWrittenWordSite>> {
        let origin = self.root_origin.as_ref()?;
        let bytes = origin.source_image().bytes();
        let mut sites = Vec::new();
        for offset in self.original_written_word_site_offsets(origin, None) {
            let binding = self.original_written_word_site_binding(origin, offset);
            let Some((words, config)) = self.original_written_word_site_geometry(&binding) else {
                continue;
            };
            let written = words
                .iter()
                .filter_map(|word| word.original_written_span(bytes))
                .collect::<Vec<_>>();
            let written_end_through = sites
                .last()
                .map_or(0, |site: &OriginalWrittenWordSite| site.written_end_through)
                .max(written.iter().map(|span| span.end()).max().unwrap_or(0));
            sites.push(OriginalWrittenWordSite {
                offset,
                config,
                written,
                written_end_through,
                binding,
            });
        }
        Some(sites)
    }

    /// One site's contribution to a span query, exactly as the unmemoised
    /// reverse scan interprets it.
    fn original_written_word_site_step(
        &self,
        origin: &Arc<super::super::SourceOriginId>,
        offset: u32,
        binding: &super::super::SourceInvocationBinding,
        span: tcl_lexer::Span,
        config: tcl_lexer::LexerConfig,
    ) -> OriginalWrittenWordSiteStep {
        let Some((words, retained_config)) = self.original_written_word_site_geometry(binding)
        else {
            return OriginalWrittenWordSiteStep::Skip;
        };
        if retained_config != config
            || !words.iter().any(|word| {
                word.original_written_span(origin.source_image().bytes())
                    .is_some_and(|written| {
                        written.start() <= span.start() && span.end() <= written.end()
                    })
            })
        {
            return OriginalWrittenWordSiteStep::Skip;
        }
        let Some(original) = origin.original_lexical_command(words, offset, config) else {
            return OriginalWrittenWordSiteStep::Refuse;
        };
        let native = original.native_words();
        let mut ordinals =
            words
                .iter()
                .zip(native)
                .enumerate()
                .filter_map(|(index, (word, native))| {
                    (word.source().span == span
                        || native.span() == span
                        || native.content_span().ok() == Some(span)
                        || native
                            .tokens()
                            .first()
                            .is_some_and(|token| token.span == span))
                    .then_some(index)
                });
        let Some(written) = ordinals.next() else {
            return OriginalWrittenWordSiteStep::Skip;
        };
        if ordinals.next().is_some() {
            return OriginalWrittenWordSiteStep::Refuse;
        }
        let Some(site) = binding.invocation_site() else {
            return OriginalWrittenWordSiteStep::Refuse;
        };
        OriginalWrittenWordSiteStep::Found(Box::new(native[written].clone()), written, site.clone())
    }
}

pub(super) type OriginalWrittenWordObservation = (
    tcl_lexer::NativeWord,
    usize,
    super::super::SourceInvocationBinding,
    super::super::CommandAllocationSite,
);

enum OriginalWrittenWordSiteStep {
    Skip,
    Refuse,
    Found(
        Box<tcl_lexer::NativeWord>,
        usize,
        super::super::CommandAllocationSite,
    ),
}

/// One root-origin site's span-independent written geometry.
#[derive(Debug)]
pub(super) struct OriginalWrittenWordSite {
    pub(super) offset: u32,
    pub(super) config: tcl_lexer::LexerConfig,
    pub(super) written: Vec<tcl_lexer::Span>,
    /// The furthest written-word end of this and every earlier site, so a
    /// reverse scan stops once no earlier site can contain the queried span.
    written_end_through: u32,
    binding: super::super::SourceInvocationBinding,
}

/// Span-independent query products of one immutable realm's bindings. Clones
/// of that realm share them; a replacement realm starts empty. Equality and
/// every answer stay independent of cache warmth.
#[derive(Debug, Clone, Default)]
pub(crate) struct OriginalRealmQueryMemo {
    written_word_sites: Arc<std::sync::OnceLock<Option<Vec<OriginalWrittenWordSite>>>>,
    pub(in super::super) caller_alias: super::super::formal_call_advice::DeclarationCallerAliasMemo,
}
