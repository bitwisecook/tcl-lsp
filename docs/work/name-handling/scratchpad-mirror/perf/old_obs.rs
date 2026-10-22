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
        self.matches_original_source_image(image, config)
            .then_some(())?;
        let mut selected = None;
        for (word, _, _, site) in self.original_written_word_observations_at_span(span, config)? {
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
        self.matches_original_source_image(image, config)
            .then_some(())?;
        let mut selected = None;
        for (word, ordinal, _, site) in
            self.original_written_word_observations_at_span(span, config)?
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

    fn original_written_word_observations_at_span(
        &self,
        span: tcl_lexer::Span,
        config: tcl_lexer::LexerConfig,
    ) -> Option<
        Vec<(
            tcl_lexer::NativeWord,
            usize,
            super::SourceInvocationBinding,
            super::CommandAllocationSite,
        )>,
    > {
        let origin = self.root_origin.as_ref()?;
        let mut found = Vec::new();
        let mut offsets = self
            .dispatch_points
            .range(..=span.start())
            .map(|(offset, _)| *offset)
            .collect::<std::collections::BTreeSet<_>>();
        if self.original_logical_source_name_advice_input().is_some() {
            offsets.extend(self.declaration_layouts.iter().filter_map(|(site, _)| {
                (site.source == *origin && site.offset <= span.start()).then_some(site.offset)
            }));
        }
        for offset in offsets.iter().rev() {
            let binding = self.attach_invocation_reads(
                Self::query_dispatch_points(
                    self.dispatch_points_at(*offset)
                        .filter(|point| point.state.current_source_origin.as_ref() == Some(origin)),
                ),
                Some(origin),
                *offset,
            );
            let geometry = if let Some(input) = self.original_logical_source_name_advice_input() {
                binding.original_logical_source_geometry_words(input)
            } else {
                binding.original_recorded_name_words()
            };
            let Some((words, retained_config)) = geometry else {
                continue;
            };
            if retained_config != config
                || !words.iter().any(|word| {
                    word.original_written_span(origin.source_image().bytes())
                        .is_some_and(|written| {
                            written.start() <= span.start() && span.end() <= written.end()
                        })
                })
            {
                continue;
            }
            let original = origin.original_lexical_command(words, *offset, config)?;
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
                continue;
            };
            if ordinals.next().is_some() {
                return None;
            }
            let site = binding.invocation_site()?.clone();
            found.push((native[written].clone(), written, binding, site));
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
}

