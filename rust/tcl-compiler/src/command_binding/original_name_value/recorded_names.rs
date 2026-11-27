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

//! The recorded original words of an invocation: lexer configuration, written names and head input.

use super::{OriginalHeadInputDecline, OriginalRecordedNamePurpose, OriginalWordSourcePurpose};
use crate::command_binding::SourceOriginLexical;
#[cfg(test)]
use crate::command_binding::original_name_value::trace_original_word_config;
use crate::signature_scan::scope::SignatureSourceNameInput;
use tcl_syntax::naming::NamePolicyProtocol;

/// Static pre-argv head with its complete original source-vector owner. This
/// value supplies lookup bytes, independently of selected compiler recipes.
pub(in super::super) fn original_static_command_head_input(
    words: &[tcl_source::words::WordExpr],
    offset: u32,
    state: &super::super::ModuleCommandBindings,
    config: tcl_lexer::LexerConfig,
) -> Option<SignatureSourceNameInput> {
    let original = state
        .current_source_origin
        .as_ref()?
        .original_lexical_command(words, offset, config)?;
    let native = original.native_words();
    let policy = state
        .source_variables
        .execution_name_policy?
        .native_recipe()?;
    tcl_names::word::SignatureSourceNameKey::from_original_native_word(
        native.first()?,
        tcl_syntax::word_rules::WordValueRules::from_config(&config),
        policy,
    )
    .map(SignatureSourceNameInput::OriginalWord)
}

/// Actual effective head after the complete written argv has been frozen.
/// Expanded heads retain their native list child; a captured byte value never
/// borrows a complete static word or a pre-argument compiler coordinate.
pub(in super::super) fn original_command_head_input(
    words: &[tcl_source::words::WordExpr],
    offset: u32,
    state: &super::super::ModuleCommandBindings,
    context: super::super::SourceExecutionContext<'_>,
) -> Option<SignatureSourceNameInput> {
    let written = context.written_arguments?;
    (written.len() == words.len()).then_some(())?;
    let policy = state
        .source_variables
        .execution_name_policy?
        .native_recipe()?;
    let origin = state.current_source_origin.as_ref()?;
    let image = origin.source_image();
    let owned;
    let native = if let Some(original) = context.original_written_projection {
        original.words_for(image, words, context.config)?
    } else {
        owned = origin.original_lexical_command(words, offset, context.config)?;
        owned.native_words()
    };
    // The authenticated written extent includes any leading expansion marker.
    (native.first()?.span().start() == offset).then_some(())?;
    for (ordinal, word) in written.iter().enumerate() {
        use crate::registry_invocation::EffectiveInvocationWord;
        if word.expansion_len() == Some(0) {
            continue;
        }
        if matches!(word, EffectiveInvocationWord::Expanded) {
            return None;
        }
        let original = native.get(ordinal)?;
        let rules = tcl_syntax::word_rules::WordValueRules::from_config(&context.config);
        let input = tcl_names::word::SignatureSourceNameKey::from_original_native_word(original, rules, policy)
            .map(SignatureSourceNameInput::OriginalWord)
            .or_else(|| crate::signature_scan::scope::SignatureSourceNameValue::from_original_static_word(original, rules, policy)
                .map(SignatureSourceNameInput::OriginalValue))
            .or_else(|| {
                let frozen = context.written_name_values?;
                (frozen.len() == words.len()).then_some(())?;
                let value = frozen.get(ordinal)?.as_deref()?;
                value.is_current(&state.source_variables).then(|| SignatureSourceNameInput::OriginalValue(
                    crate::signature_scan::scope::SignatureSourceNameValue::from_original_produced_value(value),
                ))
            })?;
        return match word.expansion_len() {
            Some(elements) => {
                let children = input.original_list_elements()?;
                (children.len() == elements).then_some(())?;
                children.into_iter().next()
            }
            None => Some(input),
        };
    }
    None
}

impl super::super::SourceInvocationBinding {
    /// Reborrow the complete original command through its retained vector.
    /// This projects tokens only and supplies no dispatch or completion grant.
    pub(crate) fn original_recorded_command_tokens(&self) -> Option<crate::ir::CommandTokens> {
        self.with_original_recorded_command(|_, tokens| tokens)
    }

    pub(crate) fn original_recorded_command(
        &self,
    ) -> Option<(
        tcl_source::segment::SegmentedCommand,
        crate::ir::CommandTokens,
    )> {
        self.with_original_recorded_command(|command, tokens| (command.clone(), tokens))
    }

    /// Share the unchanged original-vector gates before choosing owned syntax
    /// or tokens alone. The immutable lexical owner outlives the projection.
    fn with_original_recorded_command<T>(
        &self,
        project: impl FnOnce(&tcl_source::segment::SegmentedCommand, crate::ir::CommandTokens) -> T,
    ) -> Option<T> {
        let site = self.invocation_site()?;
        let (words, config) = self.original_recorded_name_words()?;
        let image = site.source.source_image();
        let original = site
            .source
            .original_lexical_command(words, site.offset, config)?;
        let command = original.command();
        let tokens = crate::ir::CommandTokens::from_segmented_reusing_words(
            &tcl_lexer::SourceMap::from_image(image),
            config,
            command,
            Some(words),
        );
        (tokens.words() == words).then(|| project(command, tokens))
    }

    /// Complete syntax configuration from this original word-vector owner.
    /// It grants syntax only, independently of any Native-purpose admission.
    #[doc(hidden)]
    #[must_use]
    pub fn original_lexer_config_for_tokens(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<tcl_lexer::LexerConfig> {
        if !OriginalWordSourcePurpose::Invocation.accepts(tokens) {
            return None;
        }
        self.original_word_vector_config(tokens)
    }

    /// Original word geometry for argument evaluation alone. An argv boundary
    /// retains its producer's vector, but never selects the original handler.
    pub(crate) fn original_argument_lexer_config(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<tcl_lexer::LexerConfig> {
        if !OriginalWordSourcePurpose::ArgumentEvaluation.accepts(tokens) {
            return None;
        }
        self.original_word_vector_config(tokens)
    }

    /// The same checked original argument vector's retained bytes and channel.
    /// Compiler-only syntax keeps its own image without issuing an invocation.
    pub(crate) fn original_argument_source_image(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<&tcl_lexer::SourceImage> {
        Some(self.original_argument_source(tokens)?.source_image())
    }

    /// The source origin [`Self::original_argument_source_image`] reads,
    /// whose lexical memo serves every capture of its commands.
    pub(crate) fn original_argument_source(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<&tcl_names::origin::SourceOriginId> {
        self.original_argument_lexer_config(tokens)?;
        let source = if let Some(original) = &self.original_written_words {
            &original.site.source
        } else if let Some(original) = &self.original_compiler_words {
            &original.site.source
        } else {
            &self.invocation_site()?.source
        };
        Some(source)
    }

    fn original_word_vector_config(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<tcl_lexer::LexerConfig> {
        #[cfg(test)]
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_CHILD_GUARDS").is_some() {
            eprintln!(
                "ORIGINAL_WORD_CONFIG stage=request offset={:?} words={} synthetic={:?} written={} compiler={} declaration={}",
                tokens
                    .words()
                    .first()
                    .map(|word| word.source().span.start()),
                tokens.words().len(),
                tokens.synthetic,
                self.original_written_words.is_some(),
                self.original_compiler_words.is_some(),
                self.declaration_layout_observations.is_some(),
            );
        }
        if !tokens.original_words_match_argv_text() {
            #[cfg(test)]
            trace_original_word_config("compatibility_vector", tokens, false);
            return None;
        }
        #[cfg(test)]
        trace_original_word_config("compatibility_vector", tokens, true);
        if let Some(original) = &self.original_written_words {
            #[cfg(test)]
            if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_CHILD_GUARDS").is_some() {
                trace_original_word_config(
                    "written_site",
                    tokens,
                    self.invocation_site()
                        .is_some_and(|site| original.site == *site),
                );
                trace_original_word_config(
                    "written_vector",
                    tokens,
                    original.words.as_ref() == tokens.words(),
                );
            }
            return (original.site == *self.invocation_site()?
                && original.words.as_ref() == tokens.words())
            .then_some(original.config);
        }
        if self.original_compiler_words.is_some() {
            return self.original_compiler_word_vector_config(tokens);
        }
        #[cfg(test)]
        trace_original_word_config("declaration_site", tokens, self.invocation_site().is_some());
        let site = self.invocation_site()?;
        let rows = super::super::declaration_layout::original_declaration_layouts(
            self.declaration_layout_observations.as_deref()?,
        )?;
        #[cfg(test)]
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_CHILD_GUARDS").is_some() {
            trace_original_word_config("declaration_rows", tokens, rows.clone().next().is_some());
        }
        let first = rows.clone().next()?;
        #[cfg(test)]
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_CHILD_GUARDS").is_some() {
            trace_original_word_config(
                "declaration_vector",
                tokens,
                first.words.as_ref() == tokens.words(),
            );
            trace_original_word_config(
                "declaration_origin",
                tokens,
                first.snapshot.state.current_source_origin.as_ref() == Some(&site.source),
            );
            trace_original_word_config(
                "declaration_consensus",
                tokens,
                rows.clone()
                    .all(|row| row.words == first.words && row.config == first.config),
            );
        }
        (first.words.as_ref() == tokens.words()
            && first.snapshot.state.current_source_origin.as_ref() == Some(&site.source)
            && rows
                .clone()
                .all(|row| row.words == first.words && row.config == first.config))
        .then_some(first.config)
    }
    fn original_compiler_word_vector_config(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<tcl_lexer::LexerConfig> {
        let original = self.original_compiler_words.as_ref()?;
        // A compiler visit can own this vector after an abrupt runtime
        // prefix. Its exact snapshot supplies syntax, never invocation.
        #[cfg(test)]
        trace_original_word_config(
            "compiler_snapshot",
            tokens,
            self.compiler_lookup_state.is_some(),
        );
        let snapshot = self.compiler_lookup_state.as_ref()?;
        #[cfg(test)]
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_CHILD_GUARDS").is_some() {
            trace_original_word_config(
                "compiler_origin",
                tokens,
                snapshot.state.current_source_origin.as_ref() == Some(&original.site.source),
            );
            trace_original_word_config(
                "compiler_site",
                tokens,
                self.invocation_site()
                    .is_none_or(|site| site == &original.site),
            );
            trace_original_word_config(
                "compiler_head",
                tokens,
                tokens
                    .words()
                    .first()
                    .is_some_and(|word| word.source().span.start() == original.site.offset),
            );
            trace_original_word_config(
                "compiler_vector",
                tokens,
                original.words.as_ref() == tokens.words(),
            );
        }
        (snapshot.state.current_source_origin.as_ref() == Some(&original.site.source)
            && self
                .invocation_site()
                .is_none_or(|site| site == &original.site)
            && tokens.words().first()?.source().span.start() == original.site.offset
            && original.words.as_ref() == tokens.words())
        .then_some(original.config)
    }

    /// Same-vector source grammar from its original dispatch, compiler or
    /// declaration owner. This does not select a physical compiler engine.
    pub(crate) fn original_source_word_dialect_for_tokens(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<tcl_native_model::InvocationDialect> {
        self.original_lexer_config_for_tokens(tokens)?;
        if let Some(original) = &self.original_written_words {
            return original.dialect;
        }
        if self.original_compiler_words.is_some() {
            return self.compiler_word_dialect();
        }
        let site = self.invocation_site()?;
        let rows = super::super::declaration_layout::original_declaration_layouts(
            self.declaration_layout_observations.as_deref()?,
        )?;
        let dialect = rows.clone().next()?.snapshot.state.baseline.dialect?;
        rows.clone()
            .all(|row| {
                row.snapshot.state.baseline.dialect == Some(dialect)
                    && row.snapshot.state.current_source_origin.as_ref() == Some(&site.source)
            })
            .then_some(dialect)
    }
    /// Exact original written operand, independently of effective alias or
    /// expansion ordinals. Readonly evaluated values retain their frozen graph
    /// and cannot become editable words, compiler operands or native argv.
    #[must_use]
    pub fn original_written_name_input(
        &self,
        tokens: &crate::ir::CommandTokens,
        written: usize,
    ) -> Option<SignatureSourceNameInput> {
        if tokens.synthetic.is_some() || !tokens.original_words_match_argv_text() {
            return None;
        }
        let (words, config) = self.original_recorded_name_words()?;
        (words == tokens.words()).then_some(())?;
        self.original_recorded_written_name_input(written, words, config)
    }

    /// Explicit retained Native/authored variable-name policy for the exact
    /// original vector. Logical source syntax supplies no Native name recipe.
    pub(crate) fn original_variable_reference_policy(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<NamePolicyProtocol> {
        let config = self.original_lexer_config_for_tokens(tokens)?;
        let (words, original_config) = self.original_recorded_name_words()?;
        (words == tokens.words() && original_config == config).then_some(())?;
        matches!(
            self.original_recorded_name_purpose(words, config)?,
            OriginalRecordedNamePurpose::Native
        )
        .then_some(())?;
        self.variable_context.execution_name_policy?.native_recipe()
    }

    pub(crate) fn original_compound_array_argument(
        &self,
        tokens: &crate::ir::CommandTokens,
        written: usize,
        escapes: tcl_dialect::EscapeSyntax,
        rules: tcl_syntax::word_rules::WordValueRules,
    ) -> Option<crate::registry_invocation::EffectiveInvocationWord> {
        let policy = self.original_variable_reference_policy(tokens)?;
        let config = self.original_argument_lexer_config(tokens)?;
        if config.escapes != escapes
            || rules != tcl_syntax::word_rules::WordValueRules::from_config(&config)
        {
            return None;
        }
        let site = self.invocation_site()?;
        let original = site
            .source
            .original_lexical_command(tokens.words(), site.offset, config)?;
        crate::registry_invocation::original_array_element_word(
            original.native_words().get(written)?,
            policy,
        )
    }

    /// Readonly input at one exact extent in this invocation's original full
    /// vector. The source channel, complete config and current retained value
    /// are checked before the ordinary original-input factory is reused.
    pub(crate) fn original_written_name_input_at_invocation_span(
        &self,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
        span: tcl_lexer::Span,
    ) -> Option<SignatureSourceNameInput> {
        // Implementation contract: naming.source.original-point-operand-projection
        // docs/design/analysis/name-resolution-proofs/original-point-operand-projection.md
        let site = self.invocation_site()?;
        (site.source.source_image() == image).then_some(())?;
        let (words, retained_config) = self.original_recorded_name_words()?;
        (retained_config == config).then_some(())?;
        let original = site
            .source
            .original_lexical_command(words, site.offset, config)?;
        let native = original.native_words();
        let mut ordinals =
            words
                .iter()
                .zip(native)
                .enumerate()
                .filter_map(|(index, (word, native))| {
                    (word.source().span == span
                        || native
                            .tokens()
                            .first()
                            .is_some_and(|token| token.span == span))
                    .then_some(index)
                });
        let written = ordinals.next()?;
        if ordinals.next().is_some() {
            return None;
        }
        self.original_recorded_written_name_input(written, words, config)
    }

    /// Original written operand from this binding's retained complete vector.
    /// No consumer token reconstruction or reporting spelling is accepted.
    #[must_use]
    pub fn original_retained_written_name_input(
        &self,
        written: usize,
    ) -> Option<SignatureSourceNameInput> {
        let (words, config) = self.original_recorded_name_words()?;
        self.original_recorded_written_name_input(written, words, config)
    }

    pub(super) fn original_recorded_name_words(
        &self,
    ) -> Option<(&[tcl_source::words::WordExpr], tcl_lexer::LexerConfig)> {
        let site = self.invocation_site()?;
        if let Some(original) = &self.original_written_words {
            return (original.site == *site).then_some((original.words.as_ref(), original.config));
        }
        if let Some(original) = &self.original_compiler_words {
            (original.site == *site).then_some(())?;
            Some((&original.words, original.config))
        } else {
            let rows = super::super::declaration_layout::original_declaration_layouts(
                self.declaration_layout_observations.as_deref()?,
            )?;
            let first = rows.clone().next()?;
            (first.snapshot.state.current_source_origin.as_ref() == Some(&site.source)
                && rows
                    .clone()
                    .all(|row| row.words == first.words && row.config == first.config))
            .then_some(())?;
            Some((&first.words, first.config))
        }
    }

    /// Conditional receiver rows retain source geometry without original invocation authority.
    pub(super) fn original_logical_source_geometry_words(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Option<(&[tcl_source::words::WordExpr], tcl_lexer::LexerConfig)> {
        input.has_logical_source_name_context().then_some(())?;
        if self.original_written_words.is_some() || self.original_compiler_words.is_some() {
            return self.original_recorded_name_words();
        }
        let site = self.invocation_site()?;
        let rows = super::super::declaration_layout::original_source_catalogue_layouts(
            self.declaration_layout_observations.as_deref()?,
        )?;
        let first = rows.clone().next()?;
        let registry = input.borrowed_context_registry().commands();
        for row in rows {
            if !row.entry.owns_source(&site.source, site.offset)
                || row.snapshot.state.current_source_origin.as_ref() != Some(&site.source)
                || row.config != input.lexer_config()
                || row.words != first.words
            {
                return None;
            }
            let owner = row
                .snapshot
                .state
                .baseline
                .metadata_context
                .source_analysis_input()?;
            if owner != input && owner != &input.for_nested_source() {
                return None;
            }
            let metadata = row
                .snapshot
                .state
                .baseline
                .metadata_context
                .metadata_context_for_source(registry, row.config, Some(owner.unit_profile()))?;
            if row
                .snapshot
                .state
                .retained_original_source_purpose(metadata, registry)?
                != super::super::OriginalInvocationSourcePurpose::Logical
            {
                return None;
            }
        }
        Some((&first.words, first.config))
    }

    /// Static naming units from a retained Logical source vector, independently
    /// of original invocation or Native value/currentness authority.
    pub(super) fn original_logical_source_written_name_input(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        written: usize,
    ) -> Option<SignatureSourceNameInput> {
        if self.original_written_words.is_some() || self.original_compiler_words.is_some() {
            return None;
        }
        let (words, config) = self.original_logical_source_geometry_words(input)?;
        let site = self.invocation_site()?;
        let original = site
            .source
            .original_lexical_command(words, site.offset, config)?;
        let policy = tcl_native_model::InvocationDialect::of_profile(input.analyser_profile())
            .authored_name_policy()?;
        if tcl_native_model::InvocationDialect::of_profile(input.unit_profile())
            .authored_name_policy()
            != Some(policy)
        {
            return None;
        }
        let purpose =
            crate::signature_scan::original_name::OriginalSourceNamePurpose::for_analysis_input(
                input, None, policy,
            );
        if !matches!(
            purpose,
            crate::signature_scan::original_name::OriginalSourceNamePurpose::Logical(_, _)
        ) {
            return None;
        }
        purpose
            .key(
                original.native_words().get(written)?,
                tcl_syntax::word_rules::WordValueRules::from_config(&config),
            )
            .map(SignatureSourceNameInput::OriginalWord)
    }

    pub(super) fn original_recorded_written_name_input(
        &self,
        written: usize,
        words: &[tcl_source::words::WordExpr],
        config: tcl_lexer::LexerConfig,
    ) -> Option<SignatureSourceNameInput> {
        let site = self.invocation_site()?;
        let original = site
            .source
            .original_lexical_command(words, site.offset, config)?;
        let purpose = self.original_recorded_name_purpose(words, config)?;
        self.recorded_written_name_input_in(
            written,
            words,
            config,
            original.native_words(),
            &purpose,
        )
    }

    /// [`Self::original_written_name_input`] for every written word of
    /// `tokens`, recapturing the command and its name purpose once rather
    /// than once per word, which made a long literal command quadratic
    /// (#2422).
    pub(crate) fn original_written_name_inputs(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<Vec<Option<SignatureSourceNameInput>>> {
        if tokens.synthetic.is_some() || !tokens.original_words_match_argv_text() {
            return None;
        }
        let (words, config) = self.original_recorded_name_words()?;
        (words == tokens.words()).then_some(())?;
        let site = self.invocation_site()?;
        let original = site
            .source
            .original_lexical_command(words, site.offset, config)?;
        let native = original.native_words();
        let purpose = self.original_recorded_name_purpose(words, config);
        Some(
            (0..native.len())
                .map(|written| {
                    self.recorded_written_name_input_in(
                        written,
                        words,
                        config,
                        native,
                        purpose.as_ref()?,
                    )
                })
                .collect(),
        )
    }

    fn recorded_written_name_input_in(
        &self,
        written: usize,
        words: &[tcl_source::words::WordExpr],
        config: tcl_lexer::LexerConfig,
        native: &[tcl_lexer::NativeWord],
        purpose: &OriginalRecordedNamePurpose,
    ) -> Option<SignatureSourceNameInput> {
        use crate::signature_scan::scope::SignatureSourceNameValue;
        use tcl_names::word::SignatureSourceNameKey;
        let word = native.get(written)?;
        let rules = tcl_syntax::word_rules::WordValueRules::from_config(&config);
        match purpose {
            OriginalRecordedNamePurpose::Logical(purpose) => {
                return purpose
                    .key(word, rules)
                    .map(SignatureSourceNameInput::OriginalWord);
            }
            OriginalRecordedNamePurpose::Native => {}
        }
        let policy = self
            .variable_context
            .execution_name_policy?
            .native_recipe()?;
        let rules = tcl_syntax::word_rules::WordValueRules::from_config(&config);
        if let Some(key) = SignatureSourceNameKey::from_original_native_word(word, rules, policy) {
            return Some(SignatureSourceNameInput::OriginalWord(key));
        }
        if let Some(value) =
            SignatureSourceNameValue::from_original_static_word(word, rules, policy)
        {
            return Some(SignatureSourceNameInput::OriginalValue(value));
        }
        let values = self.frozen_written_names.as_deref()?;
        let frozen = self.frozen_written_words.as_deref()?;
        (values.len() == words.len() && frozen.len() == values.len()).then_some(())?;
        let value = values.get(written)?.as_deref()?;
        value.is_current(&self.variable_context).then(|| {
            SignatureSourceNameInput::OriginalValue(
                SignatureSourceNameValue::from_original_produced_value(value),
            )
        })
    }

    fn original_recorded_name_purpose(
        &self,
        words: &[tcl_source::words::WordExpr],
        config: tcl_lexer::LexerConfig,
    ) -> Option<OriginalRecordedNamePurpose> {
        let candidate = if let Some(point) = &self.lookup_state {
            point
                .state
                .baseline
                .metadata_context
                .source_analysis_input()
        } else {
            self.declaration_layout_observations
                .as_deref()
                .and_then(super::super::declaration_layout::original_declaration_layouts)
                .and_then(|mut rows| rows.next())
                .and_then(|row| {
                    row.snapshot
                        .state
                        .baseline
                        .metadata_context
                        .source_analysis_input()
                })
        };
        let Some(candidate) = candidate.filter(|input| input.has_logical_source_name_context())
        else {
            return Some(OriginalRecordedNamePurpose::Native);
        };
        let (_, mut tokens) = self.original_recorded_command()?;
        tokens.source_binding = Some(self.clone());
        (tokens.words() == words && self.original_lexer_config_for_tokens(&tokens)? == config)
            .then_some(())?;
        let registry = candidate.borrowed_context_registry().commands();
        let metadata = self
            .original_source_metadata_for_advice(&tokens, registry)
            .ok()??;
        let input = metadata.source_analysis_input()?;
        input.has_logical_source_name_context().then_some(())?;
        match self.original_invocation_source_purpose(&tokens, metadata, registry)? {
            super::super::OriginalInvocationSourcePurpose::Logical => {}
            super::super::OriginalInvocationSourcePurpose::Execution
            | super::super::OriginalInvocationSourcePurpose::Standalone
            | super::super::OriginalInvocationSourcePurpose::AuthoredSimulation => {
                return Some(OriginalRecordedNamePurpose::Native);
            }
        }
        let policy = tcl_native_model::InvocationDialect::of_profile(input.analyser_profile())
            .authored_name_policy()?;
        (tcl_native_model::InvocationDialect::of_profile(input.unit_profile())
            .authored_name_policy()
            == Some(policy))
        .then_some(())?;
        Some(OriginalRecordedNamePurpose::Logical(
            crate::signature_scan::original_name::OriginalSourceNamePurpose::for_analysis_input(
                input, None, policy,
            ),
        ))
    }

    /// Original head producer with the same full-vector correspondence as an
    /// operand. Equal command spelling supplies no substitute for this owner.
    #[must_use]
    pub fn original_head_name_input(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<SignatureSourceNameInput> {
        if tokens.synthetic.is_some() || !tokens.original_words_match_argv_text() {
            return None;
        }
        let (words, _) = self.original_recorded_name_words()?;
        (words == tokens.words()).then_some(())?;
        self.original_recorded_head_name_input()
    }

    /// Diagnostic facade for the same original-head query. The static stage
    /// describes a refusal only; it supplies no name, owner or selection proof.
    #[must_use]
    pub fn original_head_name_input_with_decline_stage(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> (Option<SignatureSourceNameInput>, &'static str) {
        let mut stage = OriginalHeadInputDecline::Tokens;
        let input = (|| {
            if tokens.synthetic.is_some() || !tokens.original_words_match_argv_text() {
                return None;
            }
            stage = OriginalHeadInputDecline::RecordedWords;
            let (words, _) = self.original_recorded_name_words()?;
            stage = OriginalHeadInputDecline::WordCorrespondence;
            (words == tokens.words()).then_some(())?;
            self.original_recorded_head_name_input_with_stage(&mut stage)
        })();
        (input, stage.stage())
    }

    fn original_recorded_head_name_input_with_stage(
        &self,
        stage: &mut OriginalHeadInputDecline,
    ) -> Option<SignatureSourceNameInput> {
        *stage = OriginalHeadInputDecline::HeadWords;
        let (words, config) = self.original_recorded_name_words()?;
        let Some(frozen) = self.frozen_written_words.as_deref() else {
            *stage = OriginalHeadInputDecline::WrittenInput;
            return self.original_recorded_written_name_input(0, words, config);
        };
        *stage = OriginalHeadInputDecline::FrozenLength;
        (frozen.len() == words.len()).then_some(())?;
        for (written, word) in frozen.iter().enumerate() {
            use crate::registry_invocation::EffectiveInvocationWord;
            if word.expansion_len() == Some(0) {
                continue;
            }
            if matches!(word, EffectiveInvocationWord::Expanded) {
                *stage = OriginalHeadInputDecline::Expanded;
                return None;
            }
            *stage = OriginalHeadInputDecline::WrittenInput;
            let input = self.original_recorded_written_name_input(written, words, config)?;
            return match word.expansion_len() {
                Some(elements) => {
                    *stage = OriginalHeadInputDecline::ListInput;
                    let children = input.original_list_elements()?;
                    *stage = OriginalHeadInputDecline::ListLength;
                    (children.len() == elements).then_some(())?;
                    *stage = OriginalHeadInputDecline::ListFirst;
                    children.into_iter().next()
                }
                None => Some(input),
            };
        }
        *stage = OriginalHeadInputDecline::NoHead;
        None
    }

    /// Actual head producer from this binding's independently retained full
    /// original vector. This facade creates no command tokens or written word
    /// from a reporting name; readonly expanded and frozen values stay readonly.
    #[must_use]
    pub fn original_recorded_head_name_input(&self) -> Option<SignatureSourceNameInput> {
        let (words, config) = self.original_recorded_name_words()?;
        let Some(frozen) = self.frozen_written_words.as_deref() else {
            return self.original_recorded_written_name_input(0, words, config);
        };
        (frozen.len() == words.len()).then_some(())?;
        for (written, word) in frozen.iter().enumerate() {
            use crate::registry_invocation::EffectiveInvocationWord;
            if word.expansion_len() == Some(0) {
                continue;
            }
            if matches!(word, EffectiveInvocationWord::Expanded) {
                return None;
            }
            let input = self.original_recorded_written_name_input(written, words, config)?;
            return match word.expansion_len() {
                Some(elements) => {
                    let children = input.original_list_elements()?;
                    (children.len() == elements).then_some(())?;
                    children.into_iter().next()
                }
                None => Some(input),
            };
        }
        None
    }
}
