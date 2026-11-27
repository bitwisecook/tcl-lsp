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

//! Registry words a captured context selects: declared commands, original tokens and native cases.

#[cfg(debug_assertions)]
use super::trace_original_source_role_binding;
use super::transition_words::{native_source_registry_words, source_transition_words_with_segment};
use super::vendor_words::conditional_roles;
use super::{
    OriginalNativeSourceApplicability, OriginalOperandSource, OriginalRegistryCapturePurpose,
    OriginalRegistrySource, OriginalRegistryWords, OriginalSourceRegistryContext,
    first_tail_command, original_query_document_image, retained_document_image,
    trace_original_source_role_option,
};
use crate::analyser::AnalysisResult;
use tcl_registry::{CommandRegistry, InvocationArguments};

/// Original document/workspace assistance at the unchanged complete call.
/// Known replacements, unsupported lookup scope and missing full source
/// correspondence decline. This never inherits Registry implementation facts.
#[must_use]
pub fn source_declared_command_words(
    source: &str,
    analysis: &AnalysisResult,
    segment: &tcl_source::segment::SegmentedCommand,
) -> Option<crate::command_binding::OriginalDeclaredCommandWords> {
    // naming.source.original-declared-command-word-contract
    // docs/design/analysis/name-resolution-proofs/source-original-declared-command-word-contract.md
    let input = analysis.resolved_input.as_ref()?;
    let config = input.lexer_config();
    let realm = analysis.retained_command_realm()?;
    let image = retained_document_image(source, realm, config)?;
    if !analysis.matches_original_source_image(&image, config) {
        return None;
    }
    if !realm.matches_resolved_analysis_input(input) {
        return None;
    }
    let mut tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::from_image(&image),
        config,
        segment,
    );
    realm.stamp_original_tokens(&mut tokens);
    #[cfg(debug_assertions)]
    trace_original_source_role_binding(
        "declared",
        segment.argv.first().map(|word| word.span.start()),
        tokens.source_binding.as_ref(),
    );
    let offset = segment.argv.first()?.span.start();
    let original = crate::registry_invocation::original_native_compiler_words(
        &image,
        tokens.words(),
        offset,
        config,
    )?;
    let binding = tokens.source_binding.as_ref()?;
    let selected = trace_original_source_role_option(
        "declared.point",
        Some(offset),
        binding.original_declared_source_selection(&tokens, input, &original),
    )
    .or_else(|| {
        trace_original_source_role_option(
            "declared.authored",
            Some(offset),
            realm.original_authored_declared_source_selection(input, &tokens, &original),
        )
    })?;
    trace_original_source_role_option(
        "declared.words",
        Some(offset),
        crate::command_binding::OriginalDeclaredCommandWords::from_selected(selected, input),
    )
}

impl OriginalSourceRegistryContext {
    /// Capture an independently selected fresh source context. This never
    /// analyses a detached body as a fresh source or invokes diagnostics.
    #[must_use]
    pub fn capture(source: &str, input: crate::analyser::ResolvedAnalysisInput) -> Self {
        let (realm, input) =
            crate::analyser::Analyser::readonly_source_registry_realm(source, input);
        Self {
            image: tcl_lexer::SourceImage::document(source),
            input,
            realm,
            purpose: OriginalRegistryCapturePurpose::General,
        }
    }

    /// Capture the ordinary standalone structural consumer's exact pre-walk
    /// input, settings and immutable realm, without producing an `AnalysisResult`.
    #[must_use]
    pub fn capture_standalone_structure(
        source: &str,
        input: crate::analyser::ResolvedAnalysisInput,
    ) -> Self {
        let (realm, input) =
            crate::analyser::Analyser::readonly_standalone_structure_realm(source, input);
        Self {
            image: tcl_lexer::SourceImage::document(source),
            input,
            realm,
            purpose: OriginalRegistryCapturePurpose::StandaloneStructure,
        }
    }

    /// Capture the fixed package-index survey's source context. Its realm
    /// alone carries the survey-scoped conditional source control.
    pub(crate) fn capture_package_index_survey(
        source: &str,
        input: crate::analyser::ResolvedAnalysisInput,
    ) -> Self {
        let (realm, input) =
            crate::analyser::Analyser::readonly_package_index_survey_realm(source, input);
        Self {
            image: tcl_lexer::SourceImage::document(source),
            input,
            realm,
            purpose: OriginalRegistryCapturePurpose::PackageSurvey,
        }
    }

    pub(crate) fn retained_source_realm(&self) -> &crate::realm::CommandBindingRealm {
        &self.realm
    }

    /// Exact immutable availability and full source grammar selected at entry.
    #[must_use]
    pub fn editing_input(&self) -> &crate::analyser::ResolvedAnalysisInput {
        &self.input
    }

    /// Actual complete source channel; no source substring is its own entry.
    #[must_use]
    pub fn source_image(&self) -> &tcl_lexer::SourceImage {
        &self.image
    }

    /// Conditional source roles at the unchanged complete original vector.
    /// Missing positioned lookup retains its own source/future obligations;
    /// known shadowing or absence cannot fall through to a nominal descriptor.
    #[must_use]
    pub fn words(
        &self,
        segment: &tcl_source::segment::SegmentedCommand,
    ) -> Option<OriginalRegistryWords> {
        self.realm
            .matches_resolved_analysis_input(&self.input)
            .then_some(())?;
        let source = self.image.try_text().ok()?;
        let config = self.input.lexer_config();
        let context = self.input.context_registry();
        // Hosted source children require their independently retained body
        // occurrence owner; a fresh Native grammar cannot supply that issuer.
        if self.input.has_hosted_source_name_context() {
            return None;
        }
        // This facade owns a fresh source interpretation only. A genuine
        // positioned selection may project Registry syntax, but cannot issue
        // lookup-preserving rewrites or execution authority to this consumer.
        if tcl_native_model::InvocationDialect::of_profile(self.input.unit_profile())
            .authored_name_policy()
            .is_some()
            && let Some(mut selected) =
                selected_registry_words_with_context(source, config, &self.realm, segment, &context)
        {
            selected.operands_preserve_source_lookup = false;
            return Some(selected);
        }
        native_source_registry_words(source, config, &self.realm, &context, segment, None)
    }

    /// Conditional registration vocabulary retains the complete source's
    /// known shadow/delete barrier before projecting a Registry descriptor.
    /// This does not issue execution, Native dispatch or an analysis result.
    #[must_use]
    pub fn registration_words(
        &self,
        segment: &tcl_source::segment::SegmentedCommand,
    ) -> Option<OriginalRegistryWords> {
        let mut tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::from_image(&self.image),
            self.input.lexer_config(),
            segment,
        );
        self.realm.stamp_original_tokens(&mut tokens);
        if self
            .realm
            .original_source_registry_barrier(self.input.borrowed_context_registry(), &tokens)
        {
            return None;
        }
        self.words(segment)
    }

    /// The unchanged complete original invocation at a document offset.
    /// Stored source geometry is preferred; a parsed tail still has to match
    /// this context's complete image, realm, input and configured original argv.
    #[must_use]
    pub fn registration_words_at(&self, offset: u32) -> Option<OriginalRegistryWords> {
        let config = self.input.lexer_config();
        let binding = self.realm.invocation_at_source("", offset);
        let segment = if let Some((segment, _)) = binding.original_recorded_command() {
            segment
        } else if let Some(segment) = self
            .realm
            .original_source_catalogue_segment_at(&self.input, offset)
        {
            segment
        } else {
            let source = self.image.try_text().ok()?;
            let tail = source.get(usize::try_from(offset).ok()?..)?;
            first_tail_command(tail, offset, config.nested())?
        };
        (segment.argv.first()?.span.start() == offset).then_some(())?;
        self.registration_words(&segment)
    }

    /// Conditional Native source words for the syntax-only case-body walker.
    /// This retains command-lookup residuals and grants no
    /// generic body role, source effect, command selection or edit authority.
    pub(crate) fn conditional_native_case_words(
        &self,
        segment: &tcl_source::segment::SegmentedCommand,
    ) -> Option<OriginalRegistryWords> {
        let source = self.image.try_text().ok()?;
        let context = self.input.context_registry();
        conditional_native_case_source_words(
            source,
            self.input.lexer_config(),
            &self.realm,
            &self.input,
            &context,
            segment,
        )
    }
}

/// Unanimous Registry advice retains effective argument values and their own
/// source origins. Captured arguments occupy slots without source anchors;
/// expanded children use only independently retained readonly extents.
pub fn selected_registry_words(
    source: &str,
    analysis: &AnalysisResult,
    segment: &tcl_source::segment::SegmentedCommand,
    registry: &CommandRegistry,
) -> Option<OriginalRegistryWords> {
    let config = analysis.body_lexer_config?;
    if !analysis.matches_original_source_image(
        &original_query_document_image(source, analysis, config),
        config,
    ) {
        return None;
    }
    let actual_registry = analysis.resolved_registry()?;
    if actual_registry.snapshot().semantic_key() != registry.snapshot().semantic_key() {
        return None;
    }
    let context = analysis.resolved_input.as_ref()?.context_registry();
    selected_registry_words_with_context(
        source,
        config,
        analysis.retained_command_realm()?,
        segment,
        &context,
    )
}

pub(super) fn selected_registry_words_with_context(
    source: &str,
    config: tcl_lexer::LexerConfig,
    realm: &crate::realm::CommandBindingRealm,
    segment: &tcl_source::segment::SegmentedCommand,
    context: &tcl_registry::model::ContextRegistry,
) -> Option<OriginalRegistryWords> {
    let mut selected =
        selected_registry_words_in_realm(source, config, realm, segment, context.commands())?;
    // Implementation contract: naming.source.authored-registry-role-projection
    // docs/design/analysis/name-resolution-proofs/authored-registry-role-projection.md
    selected.context = Some(context.context().clone());
    selected.roles = selected
        .with_source_schema(context, |schema| {
            let (roles, complete) = schema.authored_source_argument_roles();
            conditional_roles(&roles, schema.semantics.argument_offset, complete)
        })
        .flatten();
    selected.captured_values =
        selected_captured_argument_values(source, config, realm, segment, context, &selected);
    Some(selected)
}

/// A selected call does not inherit the source tape's command selection.
/// Matching its entire effective vector retains only independently authored
/// captured value producers and their conditional transition obligations.
fn selected_captured_argument_values(
    source: &str,
    config: tcl_lexer::LexerConfig,
    realm: &crate::realm::CommandBindingRealm,
    segment: &tcl_source::segment::SegmentedCommand,
    context: &tcl_registry::model::ContextRegistry,
    selected: &OriginalRegistryWords,
) -> Option<std::sync::Arc<crate::command_binding::OriginalSourceCommandTransitionAdvice>> {
    use crate::registry_invocation::InvocationWordOrigin;
    if !selected
        .origins
        .iter()
        .any(|origin| matches!(origin, InvocationWordOrigin::BindingPrefix(_)))
    {
        return None;
    }
    let image = retained_document_image(source, realm, config)?;
    let mut tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::from_image(&image),
        config,
        segment,
    );
    realm.stamp_original_tokens(&mut tokens);
    let advice = realm.original_source_transition_value_advice(context, &tokens, false)?;
    let values = source_transition_words_with_segment(source, config, context, segment, advice)?;
    if values.command != selected.command
        || values.dialect != selected.dialect
        || values.origins.get(1..) != selected.origins.get(1..)
        || values.arguments.len() != selected.arguments.len()
        || !values
            .arguments
            .iter()
            .zip(&selected.arguments)
            .all(
                |(source, actual)| match (source.literal_bytes(), actual.literal_bytes()) {
                    (Some(source), Some(actual)) => source == actual,
                    _ => source == actual,
                },
            )
    {
        return None;
    }
    let OriginalRegistrySource::SourceTransitions(advice) = values.source else {
        return None;
    };
    Some(advice)
}

/// Same selected original argv/role owner for a consumer with an independently
/// retained realm, complete source and full config. No nominal-head fallback.
pub fn selected_registry_words_in_realm(
    source: &str,
    config: tcl_lexer::LexerConfig,
    realm: &crate::realm::CommandBindingRealm,
    segment: &tcl_source::segment::SegmentedCommand,
    registry: &CommandRegistry,
) -> Option<OriginalRegistryWords> {
    let image = retained_document_image(source, realm, config)?;
    selected_registry_words_in_original_image(&image, config, realm, segment, registry)
}

pub(crate) fn selected_registry_words_in_original_image(
    image: &tcl_lexer::SourceImage,
    config: tcl_lexer::LexerConfig,
    realm: &crate::realm::CommandBindingRealm,
    segment: &tcl_source::segment::SegmentedCommand,
    registry: &CommandRegistry,
) -> Option<OriginalRegistryWords> {
    realm
        .matches_original_source_image(image, config)
        .then_some(())?;
    let image = image.clone();
    let mut tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::from_image(&image),
        config,
        segment,
    );
    realm.stamp_original_tokens(&mut tokens);
    let assistance = crate::registry_invocation::original_registry_invocation_assistance(
        registry, None, &tokens,
    )
    .or_else(|| {
        crate::registry_invocation::original_incomplete_body_registry_invocation_assistance(
            registry, realm, &tokens,
        )
    })?;
    selected_registry_words_from_tokens(&image, config, &tokens, segment, registry, &assistance)
}

pub(super) fn selected_registry_words_from_tokens(
    image: &tcl_lexer::SourceImage,
    config: tcl_lexer::LexerConfig,
    tokens: &crate::ir::CommandTokens,
    segment: &tcl_source::segment::SegmentedCommand,
    registry: &CommandRegistry,
    assistance: &crate::registry_invocation::RegistryInvocationAssistance,
) -> Option<OriginalRegistryWords> {
    let image = image.clone();
    let selected = assistance.unanimous_command_words()?;
    let roles = assistance.candidates.first().and_then(|first| {
        let roles = first
            .roles
            .iter()
            .map(|&(index, role)| {
                first
                    .argument_offset
                    .checked_add(usize::from(index))
                    .map(|index| (index, role))
            })
            .collect::<Option<Vec<_>>>()?;
        assistance
            .candidates
            .iter()
            .all(|candidate| {
                candidate.roles_complete
                    && candidate
                        .roles
                        .iter()
                        .map(|&(index, role)| {
                            candidate
                                .argument_offset
                                .checked_add(usize::from(index))
                                .map(|index| (index, role))
                        })
                        .collect::<Option<Vec<_>>>()
                        .as_ref()
                        == Some(&roles)
            })
            .then_some(roles)
    });
    let effective = selected.effective();
    let binding = tokens.source_binding.as_ref()?;
    let native = crate::registry_invocation::original_native_compiler_words(
        &image,
        tokens.words(),
        segment.argv.first()?.span.start(),
        config,
    );
    let dialect = binding.original_source_word_dialect_for_tokens(tokens)?;
    let protocol = dialect.native_source_string_protocol()?;
    let arguments =
        original_source_argument_words(tokens, effective, native.as_deref()?, protocol)?;
    let words: Vec<_> = arguments
        .iter()
        .map(|word| word.as_registry_word())
        .collect();
    InvocationArguments::structured(&words).exact_argv_len()?;
    let operands =
        effective_operand_sources(segment, tokens, effective, &arguments, native.as_deref())?;
    let head = native.as_deref().and_then(|words| {
        Some(OriginalOperandSource {
            span: words.first()?.span(),
            input: binding.original_written_name_input(tokens, 0),
            word: words.first().cloned(),
        })
    });
    Some(OriginalRegistryWords {
        command: selected.command().to_owned(),
        dialect: Some(dialect),
        arguments,
        operands,
        origins: effective.origins.clone(),
        head,
        roles,
        source: OriginalRegistrySource::Selected,
        captured_values: None,
        operands_preserve_source_lookup: binding
            .original_source_operands_preserve_lookup(tokens, registry),
        image,
        config,
        registry: registry.snapshot().semantic_key(),
        context: None,
        definition_parent: None,
    })
}

fn conditional_native_case_head(
    native: &[tcl_lexer::NativeWord],
    binding: &crate::command_binding::SourceInvocationBinding,
    tokens: &crate::ir::CommandTokens,
) -> Option<OriginalOperandSource> {
    Some(OriginalOperandSource {
        span: native.first()?.span(),
        input: binding.original_written_name_input(tokens, 0),
        word: native.first().cloned(),
    })
}

/// Conditional original Native argv for the narrow possible-case-body
/// syntax consumer. The exact full input, image/configuration, stamped source
/// binding, native vector and all Registry lookup residuals remain retained.
/// Only one identical possible shape is admitted; this does not select it.
pub(super) fn conditional_native_case_source_words(
    source: &str,
    config: tcl_lexer::LexerConfig,
    realm: &crate::realm::CommandBindingRealm,
    input: &crate::analyser::ResolvedAnalysisInput,
    context: &tcl_registry::model::ContextRegistry,
    segment: &tcl_source::segment::SegmentedCommand,
) -> Option<OriginalRegistryWords> {
    if input.has_hosted_source_name_context()
        || !realm.matches_resolved_analysis_input(input)
        || config != input.lexer_config()
        || !std::ptr::eq(std::sync::Arc::as_ptr(&input.context_registry()), context)
    {
        return None;
    }
    let image = retained_document_image(source, realm, config)?;
    let registry = context.commands();
    let mut tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::from_image(&image),
        config,
        segment,
    );
    realm.stamp_original_tokens(&mut tokens);
    if realm.original_source_registry_barrier(context, &tokens) {
        return None;
    }
    let metadata = crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
        context.commands(),
        input,
    )?;
    if let Some(advice) = realm.original_conditional_case_syntax_advice(input, &tokens) {
        let mut words =
            source_transition_words_with_segment(source, config, context, segment, advice)?;
        let OriginalRegistrySource::SourceTransitions(advice) = words.source else {
            return None;
        };
        words.roles = None;
        words.source = OriginalRegistrySource::ConditionalNativeSource(std::sync::Arc::new(
            OriginalNativeSourceApplicability {
                // This source issuer has no reached lookup or absence proof.
                unknown_residual: true,
                may_be_absent: true,
                source_advice: Some((input.clone(), advice)),
            },
        ));
        return Some(words);
    }
    let assistance =
        crate::registry_invocation::original_registry_invocation_assistance_with_metadata_context(
            context.commands(),
            Some(metadata),
            &tokens,
        )?;
    let shape = assistance.candidates.first()?;
    if assistance
        .candidates
        .iter()
        .any(|candidate| candidate != shape)
    {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    let native = crate::registry_invocation::original_native_compiler_words(
        &image,
        tokens.words(),
        segment.argv.first()?.span.start(),
        config,
    )?;
    let dialect = binding.original_source_word_dialect_for_tokens(&tokens)?;
    let protocol = dialect.native_source_string_protocol()?;
    let arguments = original_source_argument_words(&tokens, &shape.effective, &native, protocol)?;
    let registry_words: Vec<_> = arguments
        .iter()
        .map(|word| word.as_registry_word())
        .collect();
    InvocationArguments::structured(&registry_words).exact_argv_len()?;
    let operands = effective_operand_sources(
        segment,
        &tokens,
        &shape.effective,
        &arguments,
        Some(&native),
    )?;
    let head = Some(conditional_native_case_head(&native, binding, &tokens)?);
    Some(OriginalRegistryWords {
        command: shape.command.clone(),
        dialect: Some(dialect),
        arguments,
        operands,
        origins: shape.effective.origins.clone(),
        head,
        // Conditional source words do not supply definite authored roles.
        roles: None,
        source: OriginalRegistrySource::ConditionalNativeSource(std::sync::Arc::new(
            OriginalNativeSourceApplicability {
                unknown_residual: assistance.unknown_residual,
                may_be_absent: assistance.may_be_absent,
                source_advice: None,
            },
        )),
        captured_values: None,
        operands_preserve_source_lookup: false,
        image,
        config,
        registry: registry.snapshot().semantic_key(),
        context: Some(context.context().clone()),
        definition_parent: None,
    })
}

/// Original lexical values have their own capture purpose. Positioned frozen
/// runtime values cannot replace static source escapes or promote a written
/// substitution to a literal. Captured argv and proved expansion children keep
/// their independent exact bytes and effective cardinality.
pub(super) fn original_source_argument_words(
    tokens: &crate::ir::CommandTokens,
    effective: &crate::registry_invocation::EffectiveCommandWords,
    native: &[tcl_lexer::NativeWord],
    protocol: tcl_syntax::native_string::NativeStringProtocol,
) -> Option<Vec<crate::registry_invocation::EffectiveInvocationWord>> {
    use crate::registry_invocation::{EffectiveInvocationWord, InvocationWordOrigin};
    let captured =
        tcl_registry::native_compiler_words::NativeCompilerWords::capture(native, protocol).ok()?;
    let frozen = crate::registry_invocation::frozen_argument_words(tokens, effective);
    effective
        .origins
        .iter()
        .skip(1)
        .zip(frozen)
        .map(|(origin, frozen)| {
            let InvocationWordOrigin::Written(ordinal) = origin else {
                return Some(frozen);
            };
            let word = native.get(*ordinal)?;
            Some(if word.group().expand {
                EffectiveInvocationWord::Expanded
            } else {
                captured
                    .literal(*ordinal)
                    .map_or(EffectiveInvocationWord::Dynamic, |bytes| {
                        EffectiveInvocationWord::ByteLiteral(std::sync::Arc::from(bytes))
                    })
            })
        })
        .collect()
}

pub(super) fn effective_operand_sources(
    segment: &tcl_source::segment::SegmentedCommand,
    tokens: &crate::ir::CommandTokens,
    effective: &crate::registry_invocation::EffectiveCommandWords,
    arguments: &[crate::registry_invocation::EffectiveInvocationWord],
    native: Option<&[tcl_lexer::NativeWord]>,
) -> Option<Vec<Option<OriginalOperandSource>>> {
    use crate::registry_invocation::InvocationWordOrigin;
    if effective.origins.len() != arguments.len().checked_add(1)?
        || effective.words.len() != effective.origins.len()
    {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    // Every written word's name input at once: one per operand recaptured
    // the whole command each time (#2422).
    let inputs = binding.original_written_name_inputs(tokens);
    let input = |written: usize| inputs.as_ref()?.get(written)?.clone();
    Some(
        effective
            .origins
            .iter()
            .skip(1)
            .zip(arguments)
            .map(|(origin, value)| match origin {
                InvocationWordOrigin::Written(written) => Some(OriginalOperandSource {
                    span: segment.argv.get(*written)?.span,
                    input: input(*written),
                    word: native.and_then(|words| words.get(*written)).cloned(),
                }),
                InvocationWordOrigin::ExpandedElement { written, element } => {
                    let parent = input(*written)?;
                    let mut children = parent.original_list_elements_with_source_spans()?;
                    let (input, span) = children.get_mut(*element)?;
                    (value.literal_bytes()? == input.bytes()).then_some(())?;
                    let span = (*span)?;
                    let parent = native?.get(*written)?.span();
                    if span.start() < parent.start() || span.end() > parent.end() {
                        return None;
                    }
                    Some(OriginalOperandSource {
                        span,
                        input: Some(input.clone()),
                        word: None,
                    })
                }
                InvocationWordOrigin::BindingPrefix(_) | InvocationWordOrigin::ResolvedHead => None,
            })
            .collect(),
    )
}
