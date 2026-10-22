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

    /// Genuine ordinal-zero original whole head at this source occurrence.
    /// Token/content extents do not identify the head's ordinal or site. This
    /// geometry grants no handler, lookup, value, class or edit authority.
    #[must_use]
    pub fn original_written_head_at_span_in_source(
        &self,
        image: &tcl_lexer::SourceImage,
        span: tcl_lexer::Span,
        config: tcl_lexer::LexerConfig,
    ) -> Option<(tcl_lexer::NativeWord, super::CommandAllocationSite)> {
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
    ) -> Option<(tcl_lexer::NativeWord, super::CommandAllocationSite)> {
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

    /// Original callee caller-alias arguments, sharing the realm's per-callee
    /// survey. See [`Self::declaration_caller_alias_arguments`].
    pub(crate) fn declaration_caller_alias_arguments_memo(
        &self,
        invocation: &crate::ir::CommandTokens,
        registry: &tcl_registry::CommandRegistry,
        owner: &crate::registry_invocation::OwnedInvocationMetadataContext,
        memo: &OriginalRealmQueryMemo,
    ) -> Option<Vec<String>> {
        self.declaration_caller_alias_arguments_with(
            invocation,
            registry,
            owner,
            Some(&memo.caller_alias),
        )
    }

    fn original_written_word_observations_at_span(
        &self,
        span: tcl_lexer::Span,
        config: tcl_lexer::LexerConfig,
    ) -> Option<Vec<OriginalWrittenWordObservation>> {
        self.original_written_word_observations_at_span_with(span, config, None)
    }

    /// Every root-origin site at or before `span`, latest first. Only a site
    /// whose retained geometry covers `span` can contribute or refuse; the
    /// memo pre-computes each site's binding and covering extents once.
    fn original_written_word_observations_at_span_with(
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
                        found.push((word, written, site.binding.clone(), call));
                    }
                }
            }
        } else {
            for offset in self.original_written_word_site_offsets(origin, Some(span.start())).iter().rev() {
                let binding = self.original_written_word_site_binding(origin, *offset);
                match self.original_written_word_site_step(origin, *offset, &binding, span, config) {
                    OriginalWrittenWordSiteStep::Skip => {}
                    OriginalWrittenWordSiteStep::Refuse => return None,
                    OriginalWrittenWordSiteStep::Found(word, written, call) => {
                        found.push((word, written, binding, call));
                    }
                }
            }
        }
        if found.is_empty() {
            if let Some(input) = self.original_logical_source_name_advice_input() {
                if config != input.lexer_config() {
                    return None;
                }
                let tape = self.original_source_transition_advice_tape_for(
                    input.borrowed_context_registry(),
                    super::OriginalSourceCataloguePurpose::CompletePrefix,
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
        }
        (!found.is_empty()).then_some(found)
    }

    /// Root-origin dispatch offsets, plus Logical declaration-layout sites,
    /// at or before `limit` (every one when `None`), ascending.
    fn original_written_word_site_offsets(
        &self,
        origin: &Arc<super::SourceOriginId>,
        limit: Option<u32>,
    ) -> std::collections::BTreeSet<u32> {
        let limit = limit.unwrap_or(u32::MAX);
        let mut offsets = self
            .dispatch_points
            .range(..=limit)
            .map(|(offset, _)| *offset)
            .collect::<std::collections::BTreeSet<_>>();
        if self.original_logical_source_name_advice_input().is_some() {
            offsets.extend(self.declaration_layouts.iter().filter_map(|(site, _)| {
                (site.source == *origin && site.offset <= limit).then_some(site.offset)
            }));
        }
        offsets
    }

    fn original_written_word_site_binding(
        &self,
        origin: &Arc<super::SourceOriginId>,
        offset: u32,
    ) -> super::SourceInvocationBinding {
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
        binding: &'b super::SourceInvocationBinding,
    ) -> Option<(&'b [crate::ir::WordExpr], tcl_lexer::LexerConfig)> {
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
            sites.push(OriginalWrittenWordSite {
                offset,
                config,
                written,
                binding,
            });
        }
        Some(sites)
    }

    /// One site's contribution to a span query, exactly as the unmemoised
    /// reverse scan interprets it.
    fn original_written_word_site_step(
        &self,
        origin: &Arc<super::SourceOriginId>,
        offset: u32,
        binding: &super::SourceInvocationBinding,
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
        OriginalWrittenWordSiteStep::Found(native[written].clone(), written, site.clone())
    }
}

type OriginalWrittenWordObservation = (
    tcl_lexer::NativeWord,
    usize,
    super::SourceInvocationBinding,
    super::CommandAllocationSite,
);

enum OriginalWrittenWordSiteStep {
    Skip,
    Refuse,
    Found(tcl_lexer::NativeWord, usize, super::CommandAllocationSite),
}

/// One root-origin site's span-independent written geometry.
#[derive(Debug)]
struct OriginalWrittenWordSite {
    offset: u32,
    config: tcl_lexer::LexerConfig,
    written: Vec<tcl_lexer::Span>,
    binding: super::SourceInvocationBinding,
}

/// Span-independent query products of one immutable realm's bindings. Clones
/// of that realm share them; a replacement realm starts empty. Equality and
/// every answer stay independent of cache warmth.
#[derive(Debug, Clone, Default)]
pub(crate) struct OriginalRealmQueryMemo {
    written_word_sites: Arc<std::sync::OnceLock<Option<Vec<OriginalWrittenWordSite>>>>,
    caller_alias: super::formal_call_advice::DeclarationCallerAliasMemo,
}

