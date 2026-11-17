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

//! Source-coverage queries: quiet stores, source loads, completions and source-cut declarations.

use super::effective_words::compose_original_effective_words;
use super::metadata_context::InvocationMetadataContext;
use super::native_compilation_source::original_native_compiler_words;
use super::source_cut_body_layout::OriginalSourceCutBodyLayout;
use super::source_scoped_body::OriginalSourceScriptBody;
use super::structured_invocation::{
    original_declared_structured_invocation, original_structured_operands_are_supported,
};
use super::word_resolution::EffectiveInvocationWord;
use super::{
    InvocationWordOrigin, OriginalAuthoredResultCompletionSourceCoverage,
    OriginalConditionalSourceLoadCoverage, OriginalSourceCutProcedureDeclaration,
    RegistryInvocationShape, ResolvedStatementInvocation, SourceCutDeclarationWordCorrespondence,
    source_schema_queries, source_structure,
};
use crate::ir::{CommandTokens, WordExpr};
use tcl_registry::{CommandRegistry, InvocationFacts};

/// Same-owner quiet scalar-store source coverage, not registry transition facts.
/// Only the original full-input/point/operand and quiet-write kernel below can
/// issue this receipt. It grants no Native, physical frame or Normal capability.
pub struct OriginalQuietScalarStoreSourceCoverage<'a> {
    pub(super) selected: ResolvedStatementInvocation,
    pub(super) analysis: &'a crate::analyser::AnalysisResult,
}

impl OriginalQuietScalarStoreSourceCoverage<'_> {
    /// The exact invocation whose source obligations the issuer checked.
    #[must_use]
    pub const fn invocation(&self) -> &ResolvedStatementInvocation {
        &self.selected
    }

    /// A receipt cannot discharge obligations for another analysis owner.
    #[must_use]
    pub fn is_for_analysis(&self, analysis: &crate::analyser::AnalysisResult) -> bool {
        std::ptr::eq(self.analysis, analysis)
    }
}

/// Coverage of one actual completed quiet scalar store in authored source.
/// Original point, operands, receiver and handler completion stay independent;
/// no compiler visit or descriptor callback absence supplies their evidence.
#[must_use]
pub fn original_authored_quiet_store_for_source_coverage<'a>(
    source: &str,
    analysis: &'a crate::analyser::AnalysisResult,
    tokens: &CommandTokens,
) -> Option<OriginalQuietScalarStoreSourceCoverage<'a>> {
    let input = analysis.resolved_input.as_ref()?;
    let config = input.lexer_config();
    let image = tcl_lexer::SourceImage::document(source);
    if !analysis.matches_original_source_image(&image, config) {
        return None;
    }
    let realm = analysis.retained_command_realm()?;
    if !realm.matches_resolved_analysis_input(input) {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    let offset = tokens.argv.first()?.start();
    let site = binding.invocation_site()?;
    let origin = realm.source_bindings_ref().source_origin()?;
    if site.offset != offset || !std::sync::Arc::ptr_eq(&site.source, origin) {
        return None;
    }
    let expected = realm.retained_original_tokens_for_command(&image, config, offset, tokens)?;
    if expected.source_binding.as_ref() != Some(binding)
        || !binding.original_invocation_completes_normally(tokens)
    {
        return None;
    }
    let registry = analysis.resolved_registry()?;
    let context = InvocationMetadataContext::for_source_input(
        registry,
        input,
        config,
        analysis.resolved_profile(),
    )?;
    if binding.original_source_advice_purpose(tokens, context, registry)?
        != crate::command_binding::OriginalInvocationSourcePurpose::AuthoredSimulation
    {
        return None;
    }
    let selected = original_declared_structured_invocation(registry, context, tokens)?;
    original_structured_operands_are_supported(registry, &selected, None, tokens)?;
    let native = original_native_compiler_words(&image, tokens.words(), offset, config)?;
    realm
        .source_bindings_ref()
        .original_quiet_scalar_store_for_source_coverage(&native, &selected.facts, registry)
        .then_some(OriginalQuietScalarStoreSourceCoverage { selected, analysis })
}

impl OriginalConditionalSourceLoadCoverage<'_> {
    /// Only this selected invocation may consume the source-load coverage.
    #[must_use]
    pub const fn invocation(&self) -> &ResolvedStatementInvocation {
        &self.selected
    }
    /// Never attach this proof to another current analysis allocation.
    #[must_use]
    pub fn is_for_analysis(&self, analysis: &crate::analyser::AnalysisResult) -> bool {
        std::ptr::eq(self.analysis, analysis) && self.outcome.matches_prefix_analysis(analysis)
    }
    /// Exact original path ordinal of the registered load transition.
    #[must_use]
    pub const fn path_argument(&self) -> usize {
        self.load.path_argument()
    }
}

/// Rejoin the exact current static load with its compiler-issued finite survey.
/// This grants no physical loading, Native invocation or operation Normal fact.
#[must_use]
pub fn original_conditional_source_load_for_coverage<'a>(
    source: &str,
    analysis: &'a crate::analyser::AnalysisResult,
    tokens: &CommandTokens,
    survey: &crate::command_binding::OriginalSingleSourceCutSurvey<'a>,
) -> Option<OriginalConditionalSourceLoadCoverage<'a>> {
    let input = analysis.resolved_input.as_ref()?;
    let image = tcl_lexer::SourceImage::document(source);
    let config = input.lexer_config();
    let realm = analysis.retained_command_realm()?;
    if !analysis.matches_original_source_image(&image, config)
        || !realm.matches_resolved_analysis_input(input)
    {
        return None;
    }
    let offset = tokens.argv.first()?.start();
    let binding = tokens.source_binding.as_ref()?;
    let expected = realm.retained_original_tokens_for_command(&image, config, offset, tokens)?;
    if expected.source_binding.as_ref() != Some(binding) {
        return None;
    }
    let load = source_structure::original_project_source_load_at(source, analysis, offset)?;
    load.literal_path()?;
    if !survey.matches_load(analysis, &load) {
        return None;
    }
    let registry = analysis.resolved_registry()?;
    let context = InvocationMetadataContext::for_source_input(
        registry,
        input,
        config,
        analysis.resolved_profile(),
    )?;
    let selected = original_declared_structured_invocation(registry, context, tokens)?;
    original_structured_operands_are_supported(registry, &selected, None, tokens)?;
    let native = original_native_compiler_words(&image, tokens.words(), offset, config)?;
    let name = binding.original_head_name_input(tokens)?;
    let captured = tcl_registry::native_compiler_words::NativeCompilerWords::capture(
        &native,
        name.policy().string_protocol(),
    )
    .ok()?;
    if native.len() != 2 || (0..native.len()).any(|ordinal| captured.literal(ordinal).is_none()) {
        return None;
    }
    Some(OriginalConditionalSourceLoadCoverage {
        selected,
        analysis,
        outcome: survey.outcome(),
        load,
    })
}

impl OriginalAuthoredResultCompletionSourceCoverage<'_> {
    /// Return the selected statement invocation retained by this source-only coverage.
    #[must_use]
    pub const fn invocation(&self) -> &ResolvedStatementInvocation {
        &self.selected
    }

    /// Check whether this coverage borrows the same analysis allocation.
    #[must_use]
    pub fn is_for_analysis(&self, analysis: &crate::analyser::AnalysisResult) -> bool {
        std::ptr::eq(self.analysis, analysis)
    }

    /// All original executable command arguments, never Result text as script.
    #[must_use]
    pub fn argument_regions(&self) -> &[tcl_lexer::Span] {
        &self.regions
    }
}

fn registered_source_completion_resolvers(
    schema: &tcl_registry::ResolvedInvocation<'_, '_>,
) -> bool {
    use tcl_registry::state_transition::LogicalSourceStateTransitionResolver;
    use tcl_registry::world_effect::SourceWorldEffectResolver;
    let effects = schema.semantics.world_effects;
    let transitions = schema.semantics.state_transitions;
    effects.is_declared()
        && transitions.is_declared()
        && effects.stated_layers().all(|descriptor| {
            matches!(
                descriptor.source_resolver,
                Some(SourceWorldEffectResolver::Effects(_))
            )
        })
        && [
            transitions.command,
            transitions.subcommand,
            transitions.form,
        ]
        .into_iter()
        .flatten()
        .all(|descriptor| {
            matches!(
                descriptor.logical_source_resolver,
                Some(LogicalSourceStateTransitionResolver::CompletionState(_))
            )
        })
}

pub(super) fn registered_result_completion_schema_is_supported(
    selected: &ResolvedStatementInvocation,
    input: &crate::analyser::ResolvedAnalysisInput,
    schema: &tcl_registry::ResolvedInvocation<'_, '_>,
) -> bool {
    use tcl_registry::world_effect::{CallbackKinds, Reentrancy, WorldStateDomain};
    let mut dialect = tcl_registry::InvocationDialect::of_profile(input.unit_profile());
    dialect.lexer_grammar = input.lexer_config().grammar_over(dialect.lexer_grammar);
    dialect.word_values =
        tcl_syntax::word_rules::WordValueRules::from_config(&input.lexer_config());
    // Proof comes only from registered source resolvers, never a
    // Native resolver fallback or the metadata family's Native label.
    if !registered_source_completion_resolvers(schema) {
        return false;
    }
    let facts = schema.facts();
    let (roles, complete) = schema.authored_source_argument_roles();
    let (effects, transitions) = schema.authored_source_effects_and_state_transitions(dialect);

    facts.descriptor_key() == selected.facts.descriptor_key()
        && schema.semantics.operation
            == tcl_registry::SemanticOperationId::StructuredLowering(
                tcl_registry::hooks::LoweringHookId::Return,
            )
        && schema.semantics.argument_offset == 0
        && facts.native_result
            == Some(tcl_registry::native_result::NativeResultContract::ReturnResult)
        && complete
        && match schema.words.arguments().exact_argv_len() {
            Some(0) => roles.is_empty(),
            Some(1) => roles.as_slice() == [(0, tcl_registry::ArgRole::Result)],
            _ => false,
        }
        && schema
            .words
            .arguments()
            .exact_argv_len()
            .is_some_and(|count| count <= 1)
        && facts.arity_accepts_frozen_arguments() == Some(true)
        && schema.semantics.body_kind == tcl_registry::BodyKind::Plain
        && schema.semantics.body_interpreter == tcl_registry::BodyInterpreter::Current
        && schema.semantics.body_execution.is_none()
        && schema.semantics.frame_effect.is_none()
        && !facts.traits.intersects(
            tcl_registry::FRAME_REACH_TRAITS
                | tcl_registry::Traits::INTROSPECTS_BY_NAME
                | tcl_registry::Traits::TARGETS_VARIABLE_BY_NAME
                | tcl_registry::Traits::REFLECTS_COMMAND_NAMES
                | tcl_registry::Traits::CREATES_SCOPE_ALIAS
                | tcl_registry::Traits::CREATES_DYNAMIC_BARRIER
                | tcl_registry::Traits::DEFERS_BODY
                | tcl_registry::Traits::PERFORMS_SUBSTITUTION,
        )
        && schema.semantics.world_effects.is_declared()
        && schema.semantics.state_transitions.is_declared()
        && effects.callback().kinds == CallbackKinds::NONE
        && effects.callback().reentrancy == Reentrancy::Never
        && effects.accesses().iter().all(|access| {
            matches!(
                access.domain,
                WorldStateDomain::InterpreterResult
                    | WorldStateDomain::CompletionState
                    | WorldStateDomain::LegacyExternal(
                        tcl_registry::side_effects::SideEffectTarget::EventControl
                    )
            )
        })
        && transitions.facts().is_empty()
        && transitions.command_resolution_impact() == tcl_registry::CommandResolutionImpact::None
}

fn authored_result_completion_effects_are_supported(
    selected: &ResolvedStatementInvocation,
    input: &crate::analyser::ResolvedAnalysisInput,
    context: InvocationMetadataContext<'_>,
    registry: &CommandRegistry,
    tokens: &CommandTokens,
) -> bool {
    RegistryInvocationShape::from_facts(&selected.facts, selected.effective.clone())
        .with_source_schema(registry, context, tokens, |schema| {
            Some(registered_result_completion_schema_is_supported(
                selected, input, schema,
            ))
        })
        .is_some_and(|closed| closed)
}

/// Source-only result completion under the actual current authored schema.
/// Arguments retain their independent conditional closure and nested commands;
/// the operation's abrupt completion is never converted to Normal.
#[must_use]
pub fn original_authored_result_completion_for_source_coverage<'a>(
    source: &str,
    analysis: &'a crate::analyser::AnalysisResult,
    tokens: &CommandTokens,
) -> Option<OriginalAuthoredResultCompletionSourceCoverage<'a>> {
    let input = analysis.resolved_input.as_ref()?;
    let config = input.lexer_config();
    let image = tcl_lexer::SourceImage::document(source);
    if !analysis.matches_original_source_image(&image, config) {
        return None;
    }
    let realm = analysis.retained_command_realm()?;
    if !realm.matches_resolved_analysis_input(input) {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    let offset = tokens.argv.first()?.start();
    let site = binding.invocation_site()?;
    let origin = realm.source_bindings_ref().source_origin()?;
    if site.offset != offset || !std::sync::Arc::ptr_eq(&site.source, origin) {
        return None;
    }
    let expected = realm.retained_original_tokens_for_command(&image, config, offset, tokens)?;
    if expected.source_binding.as_ref() != Some(binding) {
        return None;
    }
    let registry = analysis.resolved_registry()?;
    let context = InvocationMetadataContext::for_source_input(
        registry,
        input,
        config,
        analysis.resolved_profile(),
    )?;
    if binding.original_source_advice_purpose(tokens, context, registry)?
        != crate::command_binding::OriginalInvocationSourcePurpose::AuthoredSimulation
    {
        return None;
    }
    let selected = original_declared_structured_invocation(registry, context, tokens)?;
    original_structured_operands_are_supported(registry, &selected, None, tokens)?;
    if selected.effective.origins.first() != Some(&InvocationWordOrigin::ResolvedHead)
        || selected
            .effective
            .origins
            .iter()
            .enumerate()
            .skip(1)
            .any(|(index, origin)| *origin != InvocationWordOrigin::Written(index))
    {
        return None;
    }
    let native = original_native_compiler_words(&image, tokens.words(), offset, config)?;
    if !realm
        .source_bindings_ref()
        .original_conditional_arguments_for_source_coverage(&native, registry)
        || !authored_result_completion_effects_are_supported(
            &selected, input, context, registry, tokens,
        )
    {
        return None;
    }
    let mut regions = std::collections::BTreeSet::new();
    for word in &native {
        for part in word.executable_parts().all_parts() {
            match part.part {
                tcl_lexer::ExecutablePart::Command { body } => {
                    regions.insert((body.start(), body.end()));
                }
                tcl_lexer::ExecutablePart::ParseError(_)
                | tcl_lexer::ExecutablePart::Expression { .. } => return None,
                _ => {}
            }
        }
    }
    Some(OriginalAuthoredResultCompletionSourceCoverage {
        selected,
        analysis,
        regions: regions
            .into_iter()
            .map(|(a, b)| tcl_lexer::Span::new(a, b))
            .collect(),
    })
}

impl<'a> OriginalSourceCutProcedureDeclaration<'a> {
    /// Same original selected owner, never an equal-text replacement analysis.
    #[must_use]
    pub fn is_for_analysis(&self, analysis: &crate::analyser::AnalysisResult) -> bool {
        std::ptr::eq(self.analysis, analysis)
            && self
                .head_correspondence
                .matches_selected_source(self.body.source_words())
    }
    /// Source-only facts for the independent command-link coverage checks.
    #[must_use]
    pub const fn invocation(&self) -> &ResolvedStatementInvocation {
        &self.selected
    }
    /// Retain this same declaration and completed cut for conditional body syntax.
    /// No alternate header/body or execution entry can replace a refused owner.
    #[must_use]
    pub fn into_body_layout(
        self,
        survey: &crate::command_binding::OriginalSingleSourceCutSurvey<'_>,
    ) -> Option<OriginalSourceCutBodyLayout<'a>> {
        OriginalSourceCutBodyLayout::capture(self, survey)
    }

    /// Exact complete original deferred body, still requiring its own survey.
    #[must_use]
    pub const fn body(&self) -> &OriginalSourceScriptBody {
        &self.body
    }
}

/// Reborrow declaration syntax from the actual child or issued suffix owner.
/// The ordinary implicit source-purpose and execution consumers are unchanged.
#[must_use]
pub fn original_source_cut_procedure_declaration_for_coverage<'a>(
    source: &str,
    analysis: &'a crate::analyser::AnalysisResult,
    command: &crate::segmenter::SegmentedCommand,
    survey: &crate::command_binding::OriginalSingleSourceCutSurvey<'_>,
) -> Option<OriginalSourceCutProcedureDeclaration<'a>> {
    if source_cut_refuses(
        "survey-or-partial",
        !survey.owns_declaration_command(analysis, command) || command.is_partial,
    ) {
        return None;
    }
    let words = source_cut_stage(
        "registry-words",
        source_structure::source_registry_words(source, analysis, command),
    )?;
    let input = source_cut_stage("resolved-input", analysis.resolved_input.as_ref())?;
    let context = input.borrowed_context_registry();
    let realm = source_cut_stage("retained-realm", analysis.retained_command_realm())?;
    let image = source_cut_stage("source-image", realm.original_source_image())?;
    if source_cut_refuses(
        "image-words-context",
        source_cut_image_words_refused(&words, image, source, input, context),
    ) {
        return None;
    }
    let mut tokens = CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::from_image(image),
        input.lexer_config(),
        command,
    );
    realm.stamp_original_tokens(&mut tokens);
    if source_cut_refuses(
        "static-four-words",
        tokens.words().len() != 4
            || tokens.words().iter().any(|word| {
                !matches!(
                    word,
                    WordExpr::Literal { .. } | WordExpr::BracedLiteral { .. }
                )
            }),
    ) {
        return None;
    }
    let native = source_cut_stage(
        "native-words",
        original_native_compiler_words(
            image,
            tokens.words(),
            source_cut_stage("head-argv", command.argv.first())?
                .span
                .start(),
            input.lexer_config(),
        ),
    )?;
    if source_cut_refuses(
        "native-word-correspondence",
        words.origins().len() != native.len()
            || source_cut_stage("head-source", words.head_source())?.word() != native.first()
            || (0..3)
                .any(|argument| words.original_argument_word(argument) != native.get(argument + 1)),
    ) {
        return None;
    }
    let head_correspondence = source_cut_head(
        source,
        (input.lexer_config(), input, realm),
        context,
        &tokens,
        &words,
    )?;
    let facts = source_cut_stage(
        "schema-facts",
        words.with_source_schema(context, source_schema_queries::facts),
    )?;
    let selected = source_cut_stage(
        "selection",
        source_cut_declaration_selection(&tokens, &words, facts),
    )?;
    let mut bodies = source_cut_stage(
        "script-bodies",
        words.source_script_bodies_for_syntax_coverage(context),
    )?;
    if source_cut_refuses("single-body-count", bodies.len() != 1) {
        return None;
    }
    let body = source_cut_stage("one-body", bodies.pop())?;
    let original = source_cut_stage("body-word", words.original_argument_word(2))?;
    if source_cut_refuses(
        "body-source-context",
        source_cut_body_refused(original, &body, image, input, context),
    ) {
        return None;
    }
    source_cut_stage("body-commands", body.original_commands(context))?;
    Some(OriginalSourceCutProcedureDeclaration {
        analysis,
        selected,
        body,
        head_correspondence,
        header: tokens,
        command: command.clone(),
    })
}

fn source_cut_body_refused(
    original: &tcl_lexer::NativeWord,
    body: &OriginalSourceScriptBody,
    image: &tcl_lexer::SourceImage,
    input: &crate::analyser::ResolvedAnalysisInput,
    context: &tcl_registry::model::ContextRegistry,
) -> bool {
    original.group().kind != tcl_lexer::WordKind::Braced
        || original.group().expand
        || body.original_container() != original
        || !body.matches_source(image, input.lexer_config())
        || !body.matches_context(context)
}

impl SourceCutDeclarationWordCorrespondence {
    pub(super) fn matches_selected_source(
        &self,
        words: &source_structure::OriginalRegistryWords,
    ) -> bool {
        self.heads.matches_selected_source(words)
    }
    #[cfg(test)]
    pub(super) fn left(&self) -> &source_structure::OriginalRegistryWords {
        self.heads.left()
    }
    #[cfg(test)]
    pub(super) fn right(&self) -> &source_structure::OriginalRegistryWords {
        self.heads.right()
    }
}

pub(super) enum SourceCutStoredHead {
    Written,
    Resolved(Box<SourceCutDeclarationWordCorrespondence>),
}

impl SourceCutStoredHead {
    pub(super) fn matches_selected_source(
        &self,
        words: &source_structure::OriginalRegistryWords,
    ) -> bool {
        match self {
            Self::Written => true,
            Self::Resolved(head) => head.matches_selected_source(words),
        }
    }
}

fn source_cut_head(
    source: &str,
    current: (
        tcl_lexer::LexerConfig,
        &crate::analyser::ResolvedAnalysisInput,
        &crate::realm::CommandBindingRealm,
    ),
    context: &tcl_registry::model::ContextRegistry,
    tokens: &CommandTokens,
    words: &source_structure::OriginalRegistryWords,
) -> Option<SourceCutStoredHead> {
    let (config, input, realm) = current;
    match words.origins().first()? {
        InvocationWordOrigin::Written(0) => Some(SourceCutStoredHead::Written),
        InvocationWordOrigin::ResolvedHead => Some(SourceCutStoredHead::Resolved(Box::new(
            source_cut_resolved_head_correspondence(
                source, config, input, realm, context, tokens, words,
            )?,
        ))),
        _ => None,
    }
}

fn source_cut_image_words_refused(
    words: &source_structure::OriginalRegistryWords,
    image: &tcl_lexer::SourceImage,
    source: &str,
    input: &crate::analyser::ResolvedAnalysisInput,
    context: &tcl_registry::model::ContextRegistry,
) -> bool {
    image.channel() != tcl_lexer::SourceChannel::Document
        || image.bytes() != source.as_bytes()
        || !words.matches_source(image, input.lexer_config())
        || words.context() != Some(context.context())
        || !source_cut_declaration_origins(words)
}

pub(crate) fn source_cut_declaration_origins(
    words: &source_structure::OriginalRegistryWords,
) -> bool {
    matches!(
        words.origins().first(),
        Some(InvocationWordOrigin::Written(0) | InvocationWordOrigin::ResolvedHead)
    ) && words
        .origins()
        .iter()
        .enumerate()
        .skip(1)
        .all(|(index, origin)| *origin == InvocationWordOrigin::Written(index))
}

pub(super) fn source_cut_resolved_head_correspondence(
    source: &str,
    config: tcl_lexer::LexerConfig,
    input: &crate::analyser::ResolvedAnalysisInput,
    realm: &crate::realm::CommandBindingRealm,
    context: &tcl_registry::model::ContextRegistry,
    tokens: &CommandTokens,
    words: &source_structure::OriginalRegistryWords,
) -> Option<SourceCutDeclarationWordCorrespondence> {
    let image = source_cut_stage("pair.source-image", realm.original_source_image())?;
    let offset = source_cut_stage("pair.argv", tokens.argv.first())?.start();
    let expected = source_cut_stage(
        "pair.expected-tokens",
        realm.retained_original_tokens_for_command(image, config, offset, tokens),
    )?;
    if source_cut_refuses(
        "pair.binding",
        expected.source_binding != tokens.source_binding,
    ) {
        return None;
    }
    let values = source_cut_stage(
        "pair.primary-declaration-values",
        realm.original_source_procedure_declaration_values(input, context, tokens),
    )?;
    values
        .head_correspondence(source, config, input, words)
        .map(|heads| SourceCutDeclarationWordCorrespondence { heads })
}

pub(crate) fn source_cut_stage<T>(stage: &'static str, value: Option<T>) -> Option<T> {
    source_cut_refuses(stage, value.is_none());
    value
}

pub(crate) fn source_cut_refuses(stage: &'static str, refused: bool) -> bool {
    if refused && source_cut_trace_admitted() {
        eprintln!("SOURCE_CUT_DECLARATION first_refusal={stage}");
    }
    refused
}

/// Observe an already-computed source-load predicate with the existing shared quota.
pub(crate) fn load_trace_flag(stage: &'static str, offset: Option<u32>, value: bool) -> bool {
    if source_cut_trace_admitted() {
        eprintln!("SOURCE_LOAD_ISSUER stage={stage} offset={offset:?} value={value}");
    }
    value
}

/// Preserve the original source-load result and its exact ownership.
pub(crate) fn load_trace_option<T>(
    stage: &'static str,
    offset: Option<u32>,
    value: Option<T>,
) -> Option<T> {
    if source_cut_trace_admitted() {
        eprintln!(
            "SOURCE_LOAD_ISSUER stage={stage} offset={offset:?} present={}",
            value.is_some()
        );
    }
    value
}

pub(crate) fn source_cut_trace_admitted() -> bool {
    static ROWS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    option_env!("TCL_LSP_TRACE_COARSE_SOURCE") == Some("1")
        && ROWS
            .try_update(
                std::sync::atomic::Ordering::Relaxed,
                std::sync::atomic::Ordering::Relaxed,
                |rows| (rows < 64).then_some(rows + 1),
            )
            .is_ok()
}

fn source_cut_declaration_selection(
    tokens: &CommandTokens,
    words: &source_structure::OriginalRegistryWords,
    facts: InvocationFacts,
) -> Option<ResolvedStatementInvocation> {
    if facts.operation
        != tcl_registry::SemanticOperationId::StructuredLowering(
            tcl_registry::hooks::LoweringHookId::Proc,
        )
        || facts.body_kind != tcl_registry::BodyKind::Structural
        || !facts.traits.contains(
            tcl_registry::Traits::DEFERS_BODY | tcl_registry::Traits::BODY_RUNS_IN_OWN_FRAME,
        )
        || !facts.arg_roles_complete
        || facts.arity_accepts_frozen_arguments() != Some(true)
    {
        return None;
    }
    let effective = compose_original_effective_words(tokens, words.command(), &[])?;
    let selected = ResolvedStatementInvocation {
        dialect: words.dialect(),
        facts: Box::new(facts),
        arguments: effective.argument_presentations(tokens.argv_texts.get(1..)?)?,
        effective,
        evaluated_arguments: words
            .arguments()
            .iter()
            .map(|word| match word {
                EffectiveInvocationWord::Literal(value) => Some(value.clone()),
                EffectiveInvocationWord::ByteLiteral(value) => {
                    std::str::from_utf8(value).ok().map(str::to_owned)
                }
                _ => None,
            })
            .collect(),
        evaluated_words: words.arguments().to_vec(),
        authored_case_body_selection: None,
    };
    (selected.written_argument_roles()
        == vec![
            (0, tcl_registry::ArgRole::Name),
            (1, tcl_registry::ArgRole::ParamList),
            (2, tcl_registry::ArgRole::Body),
        ])
    .then_some(selected)
}

/// Original deferred procedure declaration grammar for source coverage only.
/// This retains the exact authored source lookup and direct static operands;
/// it grants neither body entry nor Native frame or Normal completion.
#[must_use]
pub fn original_authored_procedure_declaration_for_source_coverage(
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
) -> Option<ResolvedStatementInvocation> {
    if tokens
        .source_binding
        .as_ref()?
        .original_source_advice_purpose(tokens, context, registry)?
        != crate::command_binding::OriginalInvocationSourcePurpose::AuthoredSimulation
        || tokens.words().iter().enumerate().any(|(ordinal, word)| {
            !(matches!(
                word,
                WordExpr::Literal { .. } | WordExpr::BracedLiteral { .. }
            ) || ordinal == 1
                && tokens.source_binding.as_ref().is_some_and(|binding| {
                    binding.original_completed_static_declaration_name(tokens)
                }))
        })
    {
        return None;
    }
    let selected = original_declared_structured_invocation(registry, context, tokens)?;
    original_structured_operands_are_supported(registry, &selected, None, tokens)?;
    if selected.facts.operation
        != tcl_registry::SemanticOperationId::StructuredLowering(
            tcl_registry::hooks::LoweringHookId::Proc,
        )
        || selected.facts.body_kind != tcl_registry::BodyKind::Structural
        || !selected.facts.traits.contains(
            tcl_registry::Traits::DEFERS_BODY | tcl_registry::Traits::BODY_RUNS_IN_OWN_FRAME,
        )
        || selected.written_argument_roles()
            != vec![
                (0, tcl_registry::ArgRole::Name),
                (1, tcl_registry::ArgRole::ParamList),
                (2, tcl_registry::ArgRole::Body),
            ]
    {
        return None;
    }
    Some(selected)
}
