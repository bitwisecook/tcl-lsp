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

//! Expression rewrites proved on original source: gap compaction, inversion and operators.

use super::source_registry_words::source_registry_words;
use super::{OriginalRegistrySource, OriginalRegistryWords, retained_document_image};
use crate::analyser::AnalysisResult;
use tcl_registry::CommandRegistry;

/// Pure original expression gap compaction through the existing Native rewrite
/// owner. Retained source/configuration and binding are checked before its
/// independent handler, observer, reflection and finite-completion admission.
#[must_use]
pub fn original_expression_gap_compaction(
    source: &str,
    analysis: &AnalysisResult,
    segment: &crate::segmenter::SegmentedCommand,
    candidate: &str,
) -> Option<crate::command_binding::OriginalExpressionGapCompaction> {
    let config = analysis.body_lexer_config?;
    let realm = analysis.retained_command_realm()?;
    let image = retained_document_image(source, realm, config)?;
    let input = analysis.resolved_input.as_ref()?;
    if !analysis.matches_original_source_image(&image, config)
        || !realm.matches_resolved_analysis_input(input)
    {
        return None;
    }
    let mut tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::from_image(&image),
        config,
        segment,
    );
    realm.stamp_original_tokens(&mut tokens);
    tokens
        .source_binding
        .as_ref()?
        .original_expression_gap_compaction(&tokens, analysis.resolved_registry()?, candidate)
}

/// Syntax-only whitespace gaps in one whole selected braced Expr operand.
/// Variable, function and child-script tokens stay exact. This does not prove
/// pure evaluation, normal completion, source reflection or alpha renaming.
pub struct OriginalExpressionSyntaxGapCompaction {
    pub(super) original: tcl_lexer::NativeWord,
    pub(super) gaps: Vec<tcl_lexer::Span>,
    pub(super) registry: tcl_registry::RegistrySemanticKey,
}

impl OriginalExpressionSyntaxGapCompaction {
    /// Exact original braced operand under its complete source configuration.
    #[must_use]
    pub fn original_operand(&self) -> &tcl_lexer::NativeWord {
        &self.original
    }

    /// Only configured expression token gaps may disappear.
    #[must_use]
    pub fn removed_gaps(&self) -> &[tcl_lexer::Span] {
        &self.gaps
    }

    /// Original whole source and complete configuration remain current.
    #[must_use]
    pub fn matches_source(
        &self,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
    ) -> bool {
        self.original.image() == image && self.original.config() == config
    }

    /// The selected source schema retains its actual Registry generation.
    #[must_use]
    pub fn matches_registry(&self, registry: &CommandRegistry) -> bool {
        self.registry == registry.snapshot().semantic_key()
    }
}

/// A selected original Expr-role operand whose checked boolean comparison
/// is complemented without changing either operand's lexical receipt/order.
/// This is independent of syntax gaps and constant/normal evaluation proofs.
pub struct OriginalExpressionBooleanInversion {
    pub(super) original: tcl_lexer::NativeWord,
    replacement: String,
    pub(super) input: crate::analyser::ResolvedAnalysisInput,
}

impl OriginalExpressionBooleanInversion {
    /// Exact original whole operand; no new word claims its source receipt.
    #[must_use]
    pub fn original_operand(&self) -> &tcl_lexer::NativeWord {
        &self.original
    }

    /// Separately authored and native-decoding-checked whole braced word.
    #[must_use]
    pub fn replacement_source_word(&self) -> &str {
        &self.replacement
    }

    /// Complete original source/channel and configuration remain unchanged.
    #[must_use]
    pub fn matches_source(
        &self,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
    ) -> bool {
        self.original.image() == image && self.original.config() == config
    }

    /// Every retained profile, registry and expression-input axis matches.
    #[must_use]
    pub fn matches_input(&self, input: &crate::analyser::ResolvedAnalysisInput) -> bool {
        &self.input == input
    }
}

/// Issue an independent boolean inversion only from the same current quiet
/// selected Expr-role owner. Conditional advice supplies no edit authority.
#[must_use]
pub fn original_expression_boolean_comparison_inversion(
    source: &str,
    analysis: &AnalysisResult,
    segment: &crate::segmenter::SegmentedCommand,
    argument: usize,
    candidate: &str,
) -> Option<OriginalExpressionBooleanInversion> {
    let words = source_registry_words(source, analysis, segment)?;
    let input = analysis.resolved_input.as_ref()?;
    let context = input.context_registry();
    if !matches!(words.source(), OriginalRegistrySource::Selected)
        || !words.operands_preserve_source_lookup()
        || !words
            .roles()?
            .contains(&(argument, tcl_registry::ArgRole::Expr))
    {
        return None;
    }
    words.with_source_schema(&context, |_| ())?;
    let original = words.original_argument_word(argument)?.clone();
    let image = tcl_lexer::SourceImage::document(source);
    let config = input.lexer_config();
    let mut tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::from_image(&image),
        config,
        segment,
    );
    analysis
        .retained_command_realm()?
        .stamp_original_tokens(&mut tokens);
    let replacement = tokens
        .source_binding
        .as_ref()?
        .original_expression_boolean_inversion(
            &tokens,
            context.commands(),
            input,
            &original,
            candidate,
        )?;
    Some(OriginalExpressionBooleanInversion {
        original,
        replacement,
        input: input.clone(),
    })
}

/// The explicit editor intent authenticated by the original expression owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OriginalExpressionOperatorRewriteKind {
    /// Preserve boolean truth and left-to-right short-circuit reads.
    DeMorgan,
    /// Deliberately complement equality/membership while retaining reads.
    InvertComparison,
}

/// A checked whole-operand editor proposal. The original source receipt and
/// input remain independent of the newly authored replacement.
pub struct OriginalExpressionOperatorRewrite {
    pub(super) original: tcl_lexer::NativeWord,
    replacement: String,
    pub(super) input: crate::analyser::ResolvedAnalysisInput,
}

impl OriginalExpressionOperatorRewrite {
    /// Exact original source operand under the full caller configuration.
    #[must_use]
    pub fn original_operand(&self) -> &tcl_lexer::NativeWord {
        &self.original
    }

    /// Separately checked and authored complete braced word.
    #[must_use]
    pub fn replacement_source_word(&self) -> &str {
        &self.replacement
    }

    /// Every retained expression, registry, realm and source input axis matches.
    #[must_use]
    pub fn matches_input(&self, input: &crate::analyser::ResolvedAnalysisInput) -> bool {
        &self.input == input
    }
}

const REFLECTION_FREE_BODY_DEPTH: u32 = 16;

/// Every body of `words` runs only registry commands, selected or under a
/// closed conditional lookup, that neither
/// reflect command names nor introspect or call into the current frame, have
/// no command substitution, and whose own bodies hold the same. Such a body
/// cannot observe the written text of the command that encloses it.
fn reflection_free_bodies(
    source: &str,
    analysis: &AnalysisResult,
    words: &OriginalRegistryWords,
    context: &tcl_registry::model::ContextRegistry,
    depth: u32,
) -> bool {
    let Some(input) = analysis.resolved_input.as_ref() else {
        return false;
    };
    let image = tcl_lexer::SourceImage::document(source);
    let Some(bodies) = words.source_script_bodies_for_mutation_coverage(context) else {
        return false;
    };
    bodies.iter().all(|body| {
        if !body.matches_source(&image, input.lexer_config()) || !body.matches_context(context) {
            return false;
        }
        let Some(commands) = body.original_commands(context) else {
            return false;
        };
        if commands.is_empty() {
            return true;
        }
        if depth >= REFLECTION_FREE_BODY_DEPTH {
            return false;
        }
        let span = body.content_span();
        let Some(text) = source.get(span.as_range()) else {
            return false;
        };
        let segments = crate::segmenter::segment_commands_with_offset_and_config(
            text,
            span.start(),
            input.lexer_config(),
        );
        segments.len() == commands.len()
            && segments
                .iter()
                .all(|segment| reflection_free_command(source, analysis, segment, context, depth))
    })
}

fn reflection_free_command(
    source: &str,
    analysis: &AnalysisResult,
    segment: &crate::segmenter::SegmentedCommand,
    context: &tcl_registry::model::ContextRegistry,
    depth: u32,
) -> bool {
    if segment
        .word_fragments
        .iter()
        .flatten()
        .any(|fragment| fragment.token.kind == tcl_lexer::TokenType::Cmd)
    {
        return false;
    }
    let Some(words) = source_registry_words(source, analysis, segment) else {
        return false;
    };
    // Operand effects are already bounded: no command substitution is admitted.
    let closed_lookup = match words.source() {
        OriginalRegistrySource::Selected => true,
        OriginalRegistrySource::Conditional(metadata) => metadata.obligations().is_empty(),
        OriginalRegistrySource::Vendor(metadata) => hosted_body_command_named(metadata),
        _ => false,
    };
    closed_lookup
        && words.with_source_schema(context, |schema| {
            schema.facts().arg_roles_complete
                && !schema.facts().traits.intersects(
                    tcl_registry::Traits::REFLECTS_COMMAND_NAMES
                        | tcl_registry::Traits::CURRENT_FRAME_INTROSPECTION
                        | tcl_registry::Traits::INVOKES_USER_PROC,
                )
        }) == Some(true)
        && reflection_free_bodies(source, analysis, &words, context, depth + 1)
}

/// A hosted schema whose only obligations are that its coordinates are
/// authored and its parent body may not be entered: lookup itself is closed.
fn hosted_lookup_closed(
    metadata: &crate::registry_invocation::OriginalConditionalVendorRegistryMetadata,
) -> bool {
    use crate::command_binding::VendorCatalogueSourceObligation as Obligation;
    let barriers = metadata.authored_barriers();
    !barriers.has_unknown_transitions()
        && !barriers.has_nominal_command_name_hazard()
        && metadata.obligations().iter().all(|obligation| {
            matches!(
                obligation,
                Obligation::AuthoredSourceCoordinates | Obligation::StoredScriptBodyApplicability
            )
        })
}

/// A hosted rule-body command whose name can only reach its registry
/// command: iRules reaches a procedure solely through `call`, so an unknown
/// current table cannot turn a bare builtin name into source-observing code.
/// Binding alternatives and unknown namespaces still refuse.
fn hosted_body_command_named(
    metadata: &crate::registry_invocation::OriginalConditionalVendorRegistryMetadata,
) -> bool {
    use crate::command_binding::VendorCatalogueSourceObligation as Obligation;
    let barriers = metadata.authored_barriers();
    !barriers.has_unknown_transitions()
        && !barriers.has_nominal_command_name_hazard()
        && metadata.obligations().iter().all(|obligation| {
            matches!(
                obligation,
                Obligation::AuthoredSourceCoordinates
                    | Obligation::StoredScriptBodyApplicability
                    | Obligation::UnknownCurrentLookup
            )
        })
}

/// The hosted counterpart of [`original_expression_operator_rewrite`]: the
/// same single Expr operand, reflection-free bodies and operator proof, under
/// a closed hosted lookup instead of a selected native handler.
fn hosted_expression_operator_rewrite(
    source: &str,
    analysis: &AnalysisResult,
    words: &OriginalRegistryWords,
    metadata: &crate::registry_invocation::OriginalConditionalVendorRegistryMetadata,
    context: &tcl_registry::model::ContextRegistry,
    (argument, candidate, kind): (usize, &str, OriginalExpressionOperatorRewriteKind),
) -> Option<OriginalExpressionOperatorRewrite> {
    use tcl_registry::hooks::LoweringHookId;
    let input = analysis.resolved_input.as_ref()?;
    let image = tcl_lexer::SourceImage::document(source);
    let roles = words.roles()?;
    if !metadata.matches_source(&image, input.lexer_config())
        || !metadata.matches_registry(context.commands())
        || !hosted_lookup_closed(metadata)
        || !roles.contains(&(argument, tcl_registry::ArgRole::Expr))
        || roles
            .iter()
            .filter(|(_, role)| *role == tcl_registry::ArgRole::Expr)
            .count()
            != 1
        || candidate
            .bytes()
            .any(|byte| matches!(byte, b'{' | b'}' | b'\\'))
        || !reflection_free_bodies(source, analysis, words, context, 0)
    {
        return None;
    }
    let hook = words.with_source_schema(context, |schema| schema.facts().lowering_hook)??;
    if !matches!(hook, LoweringHookId::If | LoweringHookId::Expr) {
        return None;
    }
    let original = words.original_argument_word(argument)?.clone();
    if original.group().kind != tcl_lexer::WordKind::Braced || original.group().expand {
        return None;
    }
    let body_span = original.content_span().ok()?;
    let bytes = original.image().bytes().get(body_span.as_range())?;
    crate::command_binding::hosted_operator_rewrite_holds(
        bytes,
        candidate.as_bytes(),
        input.unit_profile(),
        input.lexer_config(),
        kind,
    )
    .then(|| OriginalExpressionOperatorRewrite {
        original,
        replacement: format!("{{{candidate}}}"),
        input: input.clone(),
    })
}

/// Authenticate an explicit editor operator intent on one whole original
/// expression. This admits neither arbitrary parsed replacements nor nested
/// body/source reflection, command substitutions or function evaluation.
#[must_use]
pub fn original_expression_operator_rewrite(
    source: &str,
    analysis: &AnalysisResult,
    segment: &crate::segmenter::SegmentedCommand,
    argument: usize,
    candidate: &str,
    kind: OriginalExpressionOperatorRewriteKind,
) -> Option<OriginalExpressionOperatorRewrite> {
    let words = source_registry_words(source, analysis, segment)?;
    let input = analysis.resolved_input.as_ref()?;
    let context = input.context_registry();
    if let OriginalRegistrySource::Vendor(metadata) = words.source() {
        return hosted_expression_operator_rewrite(
            source,
            analysis,
            &words,
            metadata,
            &context,
            (argument, candidate, kind),
        );
    }
    if !matches!(words.source(), OriginalRegistrySource::Selected)
        || !words.operands_preserve_source_lookup()
        || !words
            .roles()?
            .contains(&(argument, tcl_registry::ArgRole::Expr))
        || words
            .roles()?
            .iter()
            .filter(|(_, role)| *role == tcl_registry::ArgRole::Expr)
            .count()
            != 1
    {
        return None;
    }
    if !words.with_source_schema(&context, |schema| {
        schema.authored_source_clause_shape().is_none()
    })? {
        return None;
    }
    if !reflection_free_bodies(source, analysis, &words, &context, 0) {
        return None;
    }
    let original = words.original_argument_word(argument)?.clone();
    let image = tcl_lexer::SourceImage::document(source);
    let mut tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::from_image(&image),
        input.lexer_config(),
        segment,
    );
    analysis
        .retained_command_realm()?
        .stamp_original_tokens(&mut tokens);
    let replacement = tokens
        .source_binding
        .as_ref()?
        .original_expression_operator_rewrite(
            &tokens,
            context.commands(),
            input,
            &original,
            candidate,
            kind,
        )?;
    Some(OriginalExpressionOperatorRewrite {
        original,
        replacement,
        input: input.clone(),
    })
}

/// Issue syntax gaps only for a selected whole original Expr-role operand.
/// Conditional source advice and observed or changed dispatch cannot issue it.
#[must_use]
pub fn original_expression_syntax_gap_compaction(
    source: &str,
    analysis: &AnalysisResult,
    segment: &crate::segmenter::SegmentedCommand,
    argument: usize,
    candidate: &str,
) -> Option<OriginalExpressionSyntaxGapCompaction> {
    let words = source_registry_words(source, analysis, segment)?;
    let input = analysis.resolved_input.as_ref()?;
    let context = input.context_registry();
    if !matches!(words.source(), OriginalRegistrySource::Selected)
        || !words.operands_preserve_source_lookup()
        || !words
            .roles()?
            .contains(&(argument, tcl_registry::ArgRole::Expr))
    {
        return None;
    }
    words.with_source_schema(&context, |_| ())?;
    let original = words.original_argument_word(argument)?.clone();
    let image = tcl_lexer::SourceImage::document(source);
    let config = input.lexer_config();
    let mut tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::from_image(&image),
        config,
        segment,
    );
    analysis
        .retained_command_realm()?
        .stamp_original_tokens(&mut tokens);
    let gaps = tokens
        .source_binding
        .as_ref()?
        .original_expression_syntax_gaps(
            &tokens,
            context.commands(),
            input,
            &original,
            candidate,
        )?;
    Some(OriginalExpressionSyntaxGapCompaction {
        original,
        gaps,
        registry: context.commands().snapshot().semantic_key(),
    })
}
