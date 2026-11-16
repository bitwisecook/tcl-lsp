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

//! Readonly original Registry source structure shared by editor and host consumers.

mod original_body_syntax;
pub use original_body_syntax::OriginalBodySyntaxInventory;

mod registration_abstention;
pub use registration_abstention::{
    OriginalStandaloneListProjection, OriginalStandaloneSchemaAbstention,
    OriginalStandaloneSchemaAbstentions,
};

mod package_target_survey;
pub use package_target_survey::{
    OriginalPackageIndexAdvertisement, OriginalPackageIndexControl,
    OriginalPackageIndexRetainedBody, OriginalPackageIndexSourceSurvey,
    OriginalPackageIndexTargetGuard,
};

pub use crate::command_binding::{
    OriginalNativeDeferredAutoloadAdvice, OriginalNativeDeferredAutoloadObligation,
    source_native_deferred_autoload_advice_at,
};

use crate::analyser::AnalysisResult;
use tcl_registry::{CommandRegistry, InvocationArguments};

/// Observe an existing source-owner Option without changing evaluation or authority.
pub(crate) fn trace_jim_source_option<T>(stage: &str, value: Option<T>) -> Option<T> {
    #[cfg(debug_assertions)]
    if jim_source_trace_row(stage) {
        eprintln!("JIM_SOURCE_OWNER stage={stage} present={}", value.is_some());
    }
    #[cfg(not(debug_assertions))]
    let _ = stage;
    value
}

/// Share the unchanged flag, filter and total quota with held-data rows.
#[cfg(debug_assertions)]
pub(crate) fn jim_source_trace_row(stage: &str) -> bool {
    use std::sync::atomic::{AtomicU32, Ordering};
    const LIMIT: u32 = 1024;
    static ROWS: AtomicU32 = AtomicU32::new(0);
    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_JIM_SOURCE_OWNER").is_some()
        && jim_source_trace_stage_selected(
            stage,
            tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_JIM_SOURCE_OWNER_FILTER").as_deref(),
        )
    {
        match ROWS.try_update(Ordering::Relaxed, Ordering::Relaxed, |used| {
            if used < LIMIT { Some(used + 1) } else { None }
        }) {
            Ok(index) if index < LIMIT - 1 => return true,
            Ok(_) => {
                eprintln!("JIM_SOURCE_OWNER stage=option_trace_quota state=UNOBSERVED limit=1024");
            }
            Err(_) => {}
        }
    }
    false
}

// Diagnostic selection precedes the unchanged process quota. The absent mode
// preserves every old row; unknown modes cannot masquerade as an observation.
#[cfg(any(debug_assertions, test))]
fn jim_source_trace_stage_selected(stage: &str, filter: Option<&std::ffi::OsStr>) -> bool {
    filter.is_none_or(|filter| {
        (filter == "alias-only-v1" && stage.starts_with("jim.alias."))
            || (filter == "member-only-v1" && stage.starts_with("jim.own_member."))
    })
}

#[cfg(test)]
fn trace_original_child_guard(label: &str, offset: Option<u32>, passed: bool) {
    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_CHILD_GUARDS").is_some() {
        eprintln!("ORIGINAL_CHILD_GUARD label={label} offset={offset:?} passed={passed}");
    }
}

mod source_load;
pub(crate) use source_load::capture_original_project_source_load_at;
pub use source_load::{
    OriginalProjectSourceAssociation, OriginalProjectSourceCurrentCut,
    OriginalProjectSourceFrontier, OriginalProjectSourceLoad, original_project_source_load_at,
};

mod logical_variable_operands;
pub(crate) use logical_variable_operands::OriginalLogicalVariableOperands;

mod project_factory_headers;
pub use project_factory_headers::{
    ForeignFactoryMemberSyntax, ForeignFactorySourceObligation, OriginalForeignFactoryBody,
    OriginalForeignFactoryClassSyntax, OriginalProjectFactoryHeaders,
    original_foreign_factory_bodies,
};
mod project_procedure_roles;
pub use project_procedure_roles::{
    OriginalProjectProcedureArguments, OriginalProjectProcedureRoles,
    original_project_procedure_arguments,
};

mod project_class_cards;
pub use project_class_cards::{
    OriginalProjectClassCards, OriginalProjectReceiverClass, original_project_receiver_class,
    original_project_receiver_class_in_unit, original_project_receiver_class_with_sources,
};
pub(crate) use project_class_cards::{OriginalProjectClassCells, ProjectClassCell};

mod procedure;
pub use procedure::{
    OriginalLogicalProcedureArguments, OriginalProcedureSourceDescriptor,
    original_logical_procedure_arguments, original_procedure_source_descriptor,
};

/// One original whole word containing exactly one complete child command.
/// This readonly lexical projection grants no command selection or execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSingleSourceCommandSubstitution {
    operand: tcl_lexer::NativeWord,
    command: tcl_lexer::NativeScriptCommandWords,
}
impl OriginalSingleSourceCommandSubstitution {
    /// The original containing operand, with its full image and lexer policy.
    #[must_use]
    pub const fn original_operand(&self) -> &tcl_lexer::NativeWord {
        &self.operand
    }
    /// The single complete original child command and its unchanged words.
    #[must_use]
    pub const fn command(&self) -> &tcl_lexer::NativeScriptCommandWords {
        &self.command
    }
}

/// Delegate single-child shape to the existing original executable-word owner.
/// Literal bracket text, concatenations, multiple commands and malformed tails
/// refuse. Selection still requires the retained analysis at the child site.
#[must_use]
pub fn original_single_source_command_substitution(
    operand: &tcl_lexer::NativeWord,
) -> Option<OriginalSingleSourceCommandSubstitution> {
    crate::command_binding::original_single_command_substitution_words(operand).map(|original| {
        OriginalSingleSourceCommandSubstitution {
            operand: original.original_operand().clone(),
            command: original.command().clone(),
        }
    })
}

/// The same original whole-substitution projection reborrowed from its actual
/// recursively checked parent plan. Public plan rows cannot forge ancestry.
/// This supplies lexical words only, never a Native key, execution or Normal.
#[must_use]
pub fn original_single_source_command_substitution_in_plan(
    plan: &tcl_lexer::NativeScriptWordsPlan,
    operand: &tcl_lexer::NativeWord,
) -> Option<OriginalSingleSourceCommandSubstitution> {
    crate::command_binding::original_single_command_substitution_words_in_plan(plan, operand).map(
        |original| OriginalSingleSourceCommandSubstitution {
            operand: original.original_operand().clone(),
            command: original.command().clone(),
        },
    )
}

/// Reuse only immutable document geometry; every caller still performs its
/// original complete input/owner/currentness checks at the original position.
/// A missing or mismatched retained image takes the original construction path.
/// The first command of `tail`, as the whole tail's segmentation gives it.
/// Lexing runs left to right, so once a prefix holds two complete commands
/// before any syntax failure, the first can no longer change; F5's `else`
/// lookahead joins later lines, so that grammar always parses the whole tail.
fn first_tail_command(
    tail: &str,
    offset: u32,
    config: tcl_lexer::LexerConfig,
) -> Option<crate::segmenter::SegmentedCommand> {
    let mut end = 256;
    while !config.brace_line_continuation.continues() && end < tail.len() {
        let mut cut = end;
        while !tail.is_char_boundary(cut) {
            cut -= 1;
        }
        let window = &tail[..cut];
        let settled = crate::segmenter::segment_script_image_with_offset_and_config(
            &tcl_lexer::SourceImage::document(window),
            offset,
            config,
        )
        .is_ok_and(|script| script.commands.len() >= 2);
        if settled {
            return crate::segmenter::segment_commands_with_offset_and_config(
                window, offset, config,
            )
            .into_iter()
            .next();
        }
        end = end.saturating_mul(2);
    }
    crate::segmenter::segment_commands_with_offset_and_config(tail, offset, config)
        .into_iter()
        .next()
}

pub(crate) fn original_query_document_image(
    source: &str,
    analysis: &AnalysisResult,
    config: tcl_lexer::LexerConfig,
) -> tcl_lexer::SourceImage {
    analysis
        .retained_command_realm()
        .and_then(|realm| retained_document_image(source, realm, config))
        .unwrap_or_else(|| tcl_lexer::SourceImage::document(source))
}

fn retained_document_image(
    source: &str,
    realm: &crate::realm::CommandBindingRealm,
    config: tcl_lexer::LexerConfig,
) -> Option<tcl_lexer::SourceImage> {
    let image = matching_document_image(source, realm.original_source_image()?)?;
    realm
        .matches_original_source_image(&image, config)
        .then_some(image)
}

fn matching_document_image(
    source: &str,
    image: &tcl_lexer::SourceImage,
) -> Option<tcl_lexer::SourceImage> {
    // naming.source.original-immutable-image-geometry
    // docs/design/analysis/name-resolution-proofs/original-immutable-image-geometry.md
    (image.channel() == tcl_lexer::SourceChannel::Document && image.has_bytes_of(source))
        .then(|| image.clone())
}

fn trace_original_source_role_option<T>(
    stage: &str,
    offset: Option<u32>,
    value: Option<T>,
) -> Option<T> {
    #[cfg(debug_assertions)]
    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_BODY_SYNTAX").is_some() {
        eprintln!(
            "ORIGINAL_SOURCE_ROLE_ROUTE stage={stage} offset={offset:?} present={}",
            value.is_some()
        );
    }
    #[cfg(not(debug_assertions))]
    let _ = (stage, offset);
    value
}

fn trace_original_source_role_words(
    stage: &str,
    offset: Option<u32>,
    value: Option<OriginalRegistryWords>,
) -> Option<OriginalRegistryWords> {
    #[cfg(debug_assertions)]
    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_BODY_SYNTAX").is_some() {
        if let Some(words) = value.as_ref() {
            let source = match &words.source {
                OriginalRegistrySource::Selected => "Selected",
                OriginalRegistrySource::Scoped(_) => "Scoped",
                OriginalRegistrySource::DefinitionMember(_) => "DefinitionMember",
                OriginalRegistrySource::SourceTransitions(_) => "SourceTransitions",
                OriginalRegistrySource::ProducedPrefix(_) => "ProducedPrefix",
                OriginalRegistrySource::Conditional(_) => "Conditional",
                OriginalRegistrySource::ConditionalNativeSource(_) => "ConditionalNativeSource",
                OriginalRegistrySource::Vendor(_) => "Vendor",
            };
            eprintln!(
                "ORIGINAL_SOURCE_ROLE_ROUTE stage={stage} offset={offset:?} present=true source={source} command={:?} roles={:?} operands_preserve_lookup={}",
                words.command, words.roles, words.operands_preserve_source_lookup
            );
        } else {
            eprintln!("ORIGINAL_SOURCE_ROLE_ROUTE stage={stage} offset={offset:?} present=false");
        }
    }
    #[cfg(not(debug_assertions))]
    let _ = (stage, offset);
    value
}

#[cfg(debug_assertions)]
fn trace_original_source_role_binding(
    stage: &str,
    offset: Option<u32>,
    binding: Option<&crate::command_binding::SourceInvocationBinding>,
) {
    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_BODY_SYNTAX").is_some() {
        if let Some(binding) = binding {
            eprintln!(
                "ORIGINAL_SOURCE_ROLE_BINDING stage={stage} offset={offset:?} targets={:?} unknown={} absent={} runtime_state={} compiler_state={} compiler_admission={:?}",
                binding
                    .targets
                    .iter()
                    .map(|target| (&target.command, target.registry_backed, &target.kind))
                    .collect::<Vec<_>>(),
                binding.unknown,
                binding.may_be_absent,
                binding.lookup_state.is_some(),
                binding.compiler_lookup_state.is_some(),
                binding.native_compilation_admission
            );
        } else {
            eprintln!("ORIGINAL_SOURCE_ROLE_BINDING stage={stage} offset={offset:?} present=false");
        }
    }
}

/// Original document/workspace assistance at the unchanged complete call.
/// Known replacements, unsupported lookup scope and missing full source
/// correspondence decline. This never inherits Registry implementation facts.
#[must_use]
pub fn source_declared_command_words(
    source: &str,
    analysis: &AnalysisResult,
    segment: &crate::segmenter::SegmentedCommand,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OriginalRegistryCapturePurpose {
    General,
    PackageSurvey,
    StandaloneStructure,
}

/// Fresh readonly source grammar with its own complete editing input and
/// original source interpretation. Conditional applicability cannot supply
/// execution, complete reference coverage, native entry or rename permission.
/// Consumers of an existing analysis use `source_registry_words` instead so
/// its retained native entry, hosted ancestry and sealed input remain owners.
#[derive(Debug)]
pub struct OriginalSourceRegistryContext {
    image: tcl_lexer::SourceImage,
    input: crate::analyser::ResolvedAnalysisInput,
    realm: crate::realm::CommandBindingRealm,
    purpose: OriginalRegistryCapturePurpose,
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
        segment: &crate::segmenter::SegmentedCommand,
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
        if tcl_registry::InvocationDialect::of_profile(self.input.unit_profile())
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
        segment: &crate::segmenter::SegmentedCommand,
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
        segment: &crate::segmenter::SegmentedCommand,
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

/// One effective operand's original source geometry. Captured prefix operands
/// lack a written location; expansion children cannot acquire a whole word.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalOperandSource {
    span: tcl_lexer::Span,
    input: Option<crate::signature_scan::scope::SignatureSourceNameInput>,
    word: Option<tcl_lexer::NativeWord>,
}
impl OriginalOperandSource {
    /// Exact written operand or independently retained readonly child extent.
    #[must_use]
    pub const fn span(&self) -> tcl_lexer::Span {
        self.span
    }
    /// Genuine optional naming input, never reconstructed from a label.
    #[must_use]
    pub fn input(&self) -> Option<&crate::signature_scan::scope::SignatureSourceNameInput> {
        self.input.as_ref()
    }
    /// Complete original word, absent for a readonly list expansion child.
    #[must_use]
    pub fn word(&self) -> Option<&tcl_lexer::NativeWord> {
        self.word.as_ref()
    }
}

/// Selection purpose retained independently of the canonical schema label.
/// Conditional applicability remains explicit and cannot supply execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OriginalRegistrySource {
    /// Unanimous original Registry argv/role assistance.
    Selected,
    /// Exact scoped command vocabulary from a retained original body issuer.
    Scoped(std::sync::Arc<super::source_scoped_body::OriginalSourceScopedCommandSchema>),
    /// Closed document member selected under its genuine original parent.
    /// This authoring schema carries no runtime lookup or evaluation purpose.
    DefinitionMember(std::sync::Arc<super::source_scoped_body::OriginalSourceScriptBody>),
    /// Authored source tape retains its naming purpose, original operands,
    /// source order and any alias/move lineage with conditional premises.
    SourceTransitions(
        std::sync::Arc<crate::command_binding::OriginalSourceCommandTransitionAdvice>,
    ),
    /// Future source argv keeps its actual deferred parent and original builder.
    /// This is neither an evaluated list object nor a reached target invocation.
    ProducedPrefix(std::sync::Arc<crate::command_binding::OriginalSourceProducedCommandPrefix>),
    /// Native source schema with unknown and alternative applicability obligations.
    Conditional(std::sync::Arc<crate::registry_invocation::OriginalConditionalRegistryMetadata>),
    /// Possible Native schema retained only for conditional case-body source.
    ConditionalNativeSource(std::sync::Arc<OriginalNativeSourceApplicability>),
    /// Hosted source schema with its independent policy and cell barriers.
    Vendor(std::sync::Arc<crate::registry_invocation::OriginalConditionalVendorRegistryMetadata>),
}

/// Lookup residuals retained with possible Native source words. This does
/// not establish command selection, effects, entered body or edit authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalNativeSourceApplicability {
    unknown_residual: bool,
    may_be_absent: bool,
    source_advice: Option<(
        crate::analyser::ResolvedAnalysisInput,
        std::sync::Arc<crate::command_binding::OriginalSourceCommandTransitionAdvice>,
    )>,
}
impl OriginalNativeSourceApplicability {
    /// An unresolved or non-Registry lookup alternative remains possible.
    #[must_use]
    pub const fn has_unknown_residual(&self) -> bool {
        self.unknown_residual
    }
    /// The source lookup may yield no Registry command.
    #[must_use]
    pub const fn may_be_absent(&self) -> bool {
        self.may_be_absent
    }

    fn matches_source_context(
        &self,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
        context: &tcl_registry::model::ContextRegistry,
    ) -> bool {
        self.source_advice.as_ref().is_none_or(|(input, advice)| {
            input.lexer_config() == config
                && std::ptr::eq(input.borrowed_context_registry(), context)
                && advice.matches_context(context)
                && advice.matches_source(image, config)
        })
    }
}

/// Readonly continuity from invocation entry to one genuine Logical Written
/// operand. Every earlier Written head/argument is an original static whole
/// word; captured alias prefix values remain data from their separate issuer.
/// This closes only current operand syntax, not prior source/observer debts,
/// runtime values, a reached invocation or successful execution.
#[derive(Debug)]
pub struct OriginalLogicalStaticOperandPrefix<'a> {
    invocation: &'a OriginalRegistryWords,
    argument: usize,
    written: &'a [tcl_lexer::NativeWord],
}
impl OriginalLogicalStaticOperandPrefix<'_> {
    /// Complete effective invocation, retaining both source and alias issuers.
    #[must_use]
    pub const fn original_invocation(&self) -> &OriginalRegistryWords {
        self.invocation
    }
    /// Selected effective post-head argument ordinal, without recapturing it.
    #[must_use]
    pub const fn argument(&self) -> usize {
        self.argument
    }
    /// Genuine earlier Written words in their original evaluation order.
    #[must_use]
    pub const fn original_written_prefix(&self) -> &[tcl_lexer::NativeWord] {
        self.written
    }
}

/// Source-only correspondence of two genuinely retained head purposes.
/// The complete issuers remain separate: a selected effective `ResolvedHead`
/// never becomes a Written word, and the Written source head acquires no
/// selected dispatch, entered frame or replacement authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OriginalSourceHeadCorrespondence {
    left: OriginalRegistryWords,
    right: OriginalRegistryWords,
}
impl OriginalSourceHeadCorrespondence {
    pub(crate) fn matches_selected_source(&self, words: &OriginalRegistryWords) -> bool {
        &self.right == words
    }
    #[cfg(test)]
    pub(crate) const fn left(&self) -> &OriginalRegistryWords {
        &self.left
    }
    #[cfg(test)]
    pub(crate) const fn right(&self) -> &OriginalRegistryWords {
        &self.right
    }
}

/// Sealed effective source argv and authored Registry roles. This supplies
/// source structure only: no runtime dispatch, entered frame, cell, compiler,
/// Normal, reflection equivalence or edit permission follows from this view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalRegistryWords {
    command: String,
    dialect: Option<tcl_registry::InvocationDialect>,
    arguments: Vec<crate::registry_invocation::EffectiveInvocationWord>,
    operands: Vec<Option<OriginalOperandSource>>,
    origins: Vec<crate::registry_invocation::InvocationWordOrigin>,
    head: Option<OriginalOperandSource>,
    roles: Option<Vec<(usize, tcl_registry::ArgRole)>>,
    source: OriginalRegistrySource,
    captured_values:
        Option<std::sync::Arc<crate::command_binding::OriginalSourceCommandTransitionAdvice>>,
    operands_preserve_source_lookup: bool,
    image: tcl_lexer::SourceImage,
    config: tcl_lexer::LexerConfig,
    registry: tcl_registry::RegistrySemanticKey,
    context: Option<tcl_registry::model::ResolvedContext>,
    /// Parent source proof for a generic child that has no independent root
    /// command-tape entry. It only revalidates the selected source projection.
    definition_parent: Option<std::sync::Arc<super::source_scoped_body::OriginalSourceScriptBody>>,
}
fn selected_source_package_reference(
    schema: &tcl_registry::ResolvedInvocation<'_, '_>,
) -> Option<tcl_registry::source_navigation::SourcePackageReference> {
    schema.authored_source_package_reference()
}

impl OriginalRegistryWords {
    /// Canonical authored Registry descriptor; this is not a source spelling.
    #[must_use]
    pub fn command(&self) -> &str {
        &self.command
    }
    /// Independently retained invocation dialect, when actual selection owns it.
    #[must_use]
    pub const fn dialect(&self) -> Option<tcl_registry::InvocationDialect> {
        self.dialect
    }
    /// Effective post-head operands, retaining dynamic/opaque/expanded states.
    #[must_use]
    pub fn arguments(&self) -> &[crate::registry_invocation::EffectiveInvocationWord] {
        &self.arguments
    }
    /// Effective post-head source anchors. No captured operand borrows a span.
    #[must_use]
    pub fn operands(&self) -> &[Option<OriginalOperandSource>] {
        &self.operands
    }
    /// Genuine original whole word at this effective operand ordinal.
    /// A captured prefix retains its allocation producer and explicit origin;
    /// it gains no written operand at the current call or editable child word.
    #[must_use]
    pub fn original_argument_word(&self, argument: usize) -> Option<&tcl_lexer::NativeWord> {
        use crate::registry_invocation::InvocationWordOrigin;
        let origin = self.origins.get(argument.checked_add(1)?)?;
        let word = match origin {
            InvocationWordOrigin::Written(_) => self.operands.get(argument)?.as_ref()?.word()?,
            InvocationWordOrigin::BindingPrefix(_) => {
                let advice = match &self.source {
                    OriginalRegistrySource::SourceTransitions(advice) => advice,
                    _ => self.captured_values.as_ref()?,
                };
                advice
                    .matches_source(&self.image, self.config)
                    .then_some(())?;
                let captured = advice.arguments().get(argument)?;
                (captured.origin == *origin).then_some(())?;
                &captured.original
            }
            InvocationWordOrigin::ResolvedHead | InvocationWordOrigin::ExpandedElement { .. } => {
                return None;
            }
        };
        (word.image() == &self.image && word.config() == self.config && !word.group().expand)
            .then_some(word)
    }

    /// Retain a static earlier-Written prefix before lending invocation-entry
    /// source constants to this operand. The exact Logical input/context,
    /// image/configuration, selected schema and full original vector remain
    /// mandatory. Dynamic/expanded prior operands decline, including reads
    /// which could invoke unclosed observers. No executed quietness is issued.
    #[must_use]
    pub fn original_logical_static_operand_prefix_before(
        &self,
        context: &tcl_registry::model::ContextRegistry,
        argument: usize,
    ) -> Option<OriginalLogicalStaticOperandPrefix<'_>> {
        // naming.navigation.retained-path-source-inventory
        // docs/design/analysis/name-resolution-proofs/retained-path-source-inventory.md
        let OriginalRegistrySource::SourceTransitions(advice) = &self.source else {
            return None;
        };
        let input = advice.logical_source_input()?;
        if input.lexer_config() != self.config
            || !std::ptr::eq(input.borrowed_context_registry(), context)
            || !advice.matches_source(&self.image, self.config)
            || !advice.matches_context(context)
        {
            return None;
        }
        self.with_source_schema(context, |_| ())?;
        let crate::registry_invocation::InvocationWordOrigin::Written(written) =
            *self.origins.get(argument.checked_add(1)?)?
        else {
            return None;
        };
        let original = advice.original_words();
        if written == 0 || self.original_argument_word(argument)? != original.get(written)? {
            return None;
        }
        let prefix = original.get(..written)?;
        for word in prefix {
            if word.image() != &self.image
                || word.config() != self.config
                || tcl_syntax::word_rules::original_static_word_source_bytes(word).is_none()
            {
                return None;
            }
        }
        Some(OriginalLogicalStaticOperandPrefix {
            invocation: self,
            argument,
            written: prefix,
        })
    }

    /// Genuine readonly original value input at an effective operand ordinal.
    /// A captured alias prefix retains its separate declaration producer and
    /// conditional source obligations. It gains no call-site word extent,
    /// runtime `AliasPrefix` object, entered frame or invocation authority.
    #[must_use]
    pub fn original_argument_value_input(
        &self,
        argument: usize,
    ) -> Option<&crate::signature_scan::scope::SignatureSourceNameInput> {
        if let Some(input) = self
            .operands
            .get(argument)?
            .as_ref()
            .and_then(OriginalOperandSource::input)
        {
            return Some(input);
        }
        let advice = match &self.source {
            OriginalRegistrySource::SourceTransitions(advice) => advice,
            _ => self.captured_values.as_ref()?,
        };
        advice
            .arguments()
            .get(argument)?
            .input
            .as_ref()?
            .native_input()
    }

    /// Independently retained conditional declaration producers of captured
    /// operands. Selection, written geometry and execution remain separate.
    #[must_use]
    pub fn captured_argument_values(
        &self,
    ) -> Option<&crate::command_binding::OriginalSourceCommandTransitionAdvice> {
        self.captured_values.as_deref()
    }

    /// Actual written source head, independent of an alias's effective target.
    /// Hosted words retain their own source word without borrowing Native input.
    #[must_use]
    pub fn head_source(&self) -> Option<&OriginalOperandSource> {
        self.head.as_ref()
    }
    /// Original effective argv origins, including the head at ordinal zero.
    #[must_use]
    pub fn origins(&self) -> &[crate::registry_invocation::InvocationWordOrigin] {
        &self.origins
    }
    /// Join actual original head words under the same source/config/context
    /// and selected descriptor, without equating their independent purposes.
    /// A differing head origin is admitted only for the existing authored
    /// transition Written(0) and selected effective `ResolvedHead` producers.
    pub(crate) fn original_source_head_correspondence(
        &self,
        other: &Self,
        context: &tcl_registry::model::ContextRegistry,
    ) -> Option<OriginalSourceHeadCorrespondence> {
        // naming.editor.original-method-candidates
        // docs/design/analysis/name-resolution-proofs/original-method-candidates.md
        use crate::registry_invocation::InvocationWordOrigin;
        if self.image != other.image || self.config != other.config {
            return None;
        }
        let left = self.head_source()?.word()?;
        let right = other.head_source()?.word()?;
        if left != right
            || left.image() != &self.image
            || left.config() != self.config
            || left.group().expand
        {
            return None;
        }
        let left_origin = self.origins.first()?;
        let right_origin = other.origins.first()?;
        if left_origin != right_origin {
            if left_origin != &InvocationWordOrigin::Written(0)
                || right_origin != &InvocationWordOrigin::ResolvedHead
            {
                return None;
            }
            let OriginalRegistrySource::SourceTransitions(advice) = self.source() else {
                return None;
            };
            if !matches!(other.source(), OriginalRegistrySource::Selected)
                || !advice.matches_source(&self.image, self.config)
                || !advice.matches_context(context)
                || advice.original_words().first() != Some(left)
            {
                return None;
            }
        }
        let same_descriptor = self.with_source_schema(context, |left| {
            other.with_source_schema(context, |right| {
                std::ptr::eq(
                    left.authored_source_descriptors().command,
                    right.authored_source_descriptors().command,
                ) && left.semantics.argument_offset == right.semantics.argument_offset
            })
        })??;
        same_descriptor.then_some(())?;
        Some(OriginalSourceHeadCorrespondence {
            left: self.clone(),
            right: other.clone(),
        })
    }

    /// Complete unanimous roles at effective post-head ordinals, or unknown.
    #[must_use]
    pub fn roles(&self) -> Option<&[(usize, tcl_registry::ArgRole)]> {
        self.roles.as_deref()
    }
    /// Possible authored roles projected onto genuine whole written arguments.
    /// Captured prefix operands and expanded children provide no whole-word
    /// position; the head is excluded. This supplies no definite role.
    #[must_use]
    pub fn written_argument_roles(&self) -> Vec<(usize, tcl_registry::ArgRole)> {
        let mut roles = Vec::new();
        for &(argument, role) in self.roles().unwrap_or(&[]) {
            let Some(crate::registry_invocation::InvocationWordOrigin::Written(written)) = argument
                .checked_add(1)
                .and_then(|ordinal| self.origins.get(ordinal))
            else {
                continue;
            };
            if let Some(argument) = written.checked_sub(1)
                && !roles.contains(&(argument, role))
            {
                roles.push((argument, role));
            }
        }
        roles.sort_by_key(|(ordinal, _)| *ordinal);
        roles
    }

    /// Original selection/conditional applicability without a runtime grant.
    #[must_use]
    pub const fn source(&self) -> &OriginalRegistrySource {
        &self.source
    }
    /// Independent original operand-effect check for the selected source
    /// lookup. Conditional schema applicability cannot supply this facet.
    /// This is not an execution, compiler, Normal or general edit receipt.
    #[must_use]
    pub const fn operands_preserve_source_lookup(&self) -> bool {
        self.operands_preserve_source_lookup
    }
    /// Exact complete source/channel and full lexer config correspondence.
    #[must_use]
    pub fn matches_source(
        &self,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
    ) -> bool {
        self.image == *image && self.config == config
    }
    /// Actual complete availability context when the source analysis owns it.
    /// A realm-only geometry projection does not manufacture this context.
    #[must_use]
    pub fn context(&self) -> Option<&tcl_registry::model::ResolvedContext> {
        self.context.as_ref()
    }
    /// Authenticated enclosing definition body when this argv was reborrowed
    /// from a complete parent source plan.
    #[must_use]
    pub fn definition_parent(
        &self,
    ) -> Option<&super::source_scoped_body::OriginalSourceScriptBody> {
        self.definition_parent.as_deref()
    }
    /// Borrow the same exact argv's Registry-authored source schema under its
    /// actual retained context. The projection is source advice only; fields
    /// describing possible handlers/effects never establish execution here.
    #[must_use]
    pub fn with_source_schema<'r, T>(
        &'r self,
        context: &'r tcl_registry::model::ContextRegistry,
        project: impl FnOnce(&tcl_registry::ResolvedInvocation<'r, '_>) -> T,
    ) -> Option<T> {
        let arguments = self
            .arguments
            .iter()
            .map(|word| source_schema_word(word.as_registry_word()))
            .collect::<Vec<_>>();
        self.with_source_arguments(context, &arguments, project)
    }

    /// Query the same selected source descriptor with separately supplied
    /// advisory values on ordinary original written operands. Captured words,
    /// expansion cardinality and the original argv stay unchanged. This does
    /// not establish the values, a Native argv, dispatch or completion.
    pub(crate) fn with_source_value_projection<'r, T>(
        &'r self,
        context: &'r tcl_registry::model::ContextRegistry,
        values: &[(usize, &str)],
        project: impl FnOnce(&tcl_registry::ResolvedInvocation<'r, '_>) -> T,
    ) -> Option<T> {
        let mut arguments = self
            .arguments
            .iter()
            .map(|word| source_schema_word(word.as_registry_word()))
            .collect::<Vec<_>>();
        let mut seen = std::collections::HashSet::new();
        for &(ordinal, value) in values {
            if !seen.insert(ordinal)
                || !matches!(self.origins.get(ordinal.checked_add(1)?),
                    Some(crate::registry_invocation::InvocationWordOrigin::Written(written)) if *written > 0)
            {
                return None;
            }
            let word = self.operands.get(ordinal)?.as_ref()?.word()?;
            if word.group().expand {
                return None;
            }
            *arguments.get_mut(ordinal)? = tcl_registry::InvocationWord::Literal(value);
        }
        self.with_source_arguments(context, &arguments, project)
    }

    fn with_source_arguments<'r, T>(
        &'r self,
        context: &'r tcl_registry::model::ContextRegistry,
        arguments: &[tcl_registry::InvocationWord<'_>],
        project: impl FnOnce(&tcl_registry::ResolvedInvocation<'r, '_>) -> T,
    ) -> Option<T> {
        if !self.matches_registry(context.commands())
            || self.context.as_ref() != Some(context.context())
            || matches!(&self.source, OriginalRegistrySource::SourceTransitions(advice) if !advice.matches_context(context))
            || matches!(&self.source, OriginalRegistrySource::ProducedPrefix(prefix) if !prefix.matches_source_context(&self.image, self.config, context))
            || matches!(&self.source, OriginalRegistrySource::ConditionalNativeSource(applicability) if !applicability.matches_source_context(&self.image, self.config, context))
            || self.definition_parent.as_ref().is_some_and(|parent| {
                !parent.matches_source(&self.image, self.config) || !parent.matches_context(context)
            })
        {
            return None;
        }
        let invocation = tcl_registry::InvocationWords::structured(
            tcl_registry::InvocationWord::Literal(&self.command),
            arguments,
        );
        let invocation = self
            .dialect
            .map_or(invocation, |dialect| invocation.with_dialect(dialect));
        if let OriginalRegistrySource::DefinitionMember(parent) = &self.source {
            if !parent.matches_source(&self.image, self.config) || !parent.matches_context(context)
            {
                return None;
            }
            let grammar = parent
                .source_words()
                .with_source_schema(context, source_definition_body_grammar)
                .flatten()?;
            let descriptor = context
                .commands()
                .authored_document_member_descriptor(grammar, &self.command)?;
            let selected = tcl_registry::registry::resolve_source_descriptor_invocation(
                descriptor,
                invocation,
                context.commands().source_descriptor_availability(
                    descriptor,
                    Some(context.context().authoring_query()),
                ),
                self.dialect.map(|dialect| dialect.numbers),
            );
            return Some(project(&selected));
        }
        if let OriginalRegistrySource::Scoped(scoped) = &self.source {
            if scoped.body().context() != context.context()
                || !scoped.body().matches_source(&self.image, self.config)
                || !scoped.body().matches_registry(context.commands())
            {
                return None;
            }
            let selected = tcl_registry::registry::resolve_source_descriptor_invocation(
                scoped.descriptor(),
                invocation,
                context.commands().source_descriptor_availability(
                    scoped.body().parent_descriptor(),
                    Some(context.context().authoring_query()),
                ),
                self.dialect.map(|dialect| dialect.numbers),
            );
            return Some(project(&selected));
        }
        let selected = tcl_registry::model::assembly::resolve_invocation_words_in_context(
            context.commands(),
            Some(context.context()),
            invocation,
        )?;
        Some(project(&selected))
    }

    /// Original script regions from the shared source body/role owner.
    /// Complete context and applicability stay retained; no runtime body entry,
    /// scoped vocabulary, current cell, Normal or writable authority follows.
    #[must_use]
    pub fn source_script_bodies(
        &self,
        context: &tcl_registry::model::ContextRegistry,
    ) -> Vec<super::source_scoped_body::OriginalSourceScriptBody> {
        self.source_script_bodies_for(context, super::OriginalSourceScriptPurpose::Syntax)
    }

    /// The same source inventory under an explicit syntax/evaluation purpose.
    /// Reference-only syntax remains available independently of potential entry.
    #[must_use]
    pub fn source_script_bodies_for(
        &self,
        context: &tcl_registry::model::ContextRegistry,
        purpose: super::OriginalSourceScriptPurpose,
    ) -> Vec<super::source_scoped_body::OriginalSourceScriptBody> {
        super::source_scoped_body::source_script_bodies_for(self, context, purpose)
    }

    /// Possible case-body candidates for source syntax consumers. The separate
    /// projection retains unknown option/subject selection; it does not change
    /// the stronger body inventory or establish entry.
    #[must_use]
    pub(crate) fn conditional_source_case_bodies(
        &self,
        context: &tcl_registry::model::ContextRegistry,
    ) -> Option<super::source_scoped_body::OriginalConditionalSourceCaseBodies> {
        super::source_scoped_body::conditional_source_case_bodies_for(self, context)
    }

    /// Complete original case regions under a separately conditioned normal
    /// source continuation; this supplies no actual Normal or Native entry.
    pub(crate) fn normal_source_case_bodies(
        &self,
        context: &tcl_registry::model::ContextRegistry,
    ) -> Option<super::source_scoped_body::OriginalNormalSourceCaseBodies> {
        super::source_scoped_body::normal_source_case_bodies_for(self, context)
    }

    /// Complete original source-body inventory for a coverage consumer.
    /// Unlike the partial syntax inventory, a missing, dynamic or malformed
    /// potentially evaluated body returns `None`. This certifies source
    /// coverage only, independently of execution, effects, frames or edits.
    #[must_use]
    pub fn source_script_bodies_for_mutation_coverage(
        &self,
        context: &tcl_registry::model::ContextRegistry,
    ) -> Option<Vec<super::source_scoped_body::OriginalSourceScriptBody>> {
        super::source_scoped_body::source_script_bodies_for_mutation_coverage(self, context)
    }

    /// Complete original syntax-body coverage, including reference-only
    /// declarations. Missing/cooked/dynamic/unrepresented bodies remain None.
    /// This covers source syntax only; no entered body or runtime absence follows.
    #[must_use]
    pub fn source_script_bodies_for_syntax_coverage(
        &self,
        context: &tcl_registry::model::ContextRegistry,
    ) -> Option<Vec<super::source_scoped_body::OriginalSourceScriptBody>> {
        super::source_scoped_body::source_script_bodies_for_coverage(
            self,
            context,
            super::OriginalSourceScriptPurpose::Syntax,
        )
    }

    /// Original expression command regions at the retained AST's actual
    /// operand-content base. Unique genuine Expr geometry is required;
    /// unrelated or cooked operands cannot donate a child source span.
    #[must_use]
    pub fn source_expression_script_bodies_at(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        expression_base: u32,
    ) -> Option<Vec<super::OriginalSourceScriptBody>> {
        super::source_scoped_body::source_expression_script_bodies_at(self, input, expression_base)
    }

    /// Registry contents used by the issuer, independent of reporting names.
    #[must_use]
    pub fn matches_registry(&self, registry: &CommandRegistry) -> bool {
        self.registry == registry.snapshot().semantic_key()
    }
    /// Original expression command regions under the caller's retained complete
    /// input. Multiple/cooked operands and unknown expression grammar decline.
    #[must_use]
    pub fn source_expression_script_bodies(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Option<Vec<super::OriginalSourceScriptBody>> {
        super::source_scoped_body::source_expression_script_bodies(self, input)
    }
}

/// Borrow exact text for Registry source controls without changing the retained
/// counted value. Opaque units, raw zero, substitution and expansion retain
/// their independent byte/cardinality facets; no executable argv is issued.
pub(crate) fn source_schema_word(
    word: tcl_registry::InvocationWord<'_>,
) -> tcl_registry::InvocationWord<'_> {
    let tcl_registry::InvocationWord::KnownBytes(bytes) = word else {
        return word;
    };
    if bytes.contains(&0) {
        return word;
    }
    std::str::from_utf8(bytes).map_or(word, tcl_registry::InvocationWord::Literal)
}

/// Unanimous Registry advice retains effective argument values and their own
/// source origins. Captured arguments occupy slots without source anchors;
/// expanded children use only independently retained readonly extents.
pub fn selected_registry_words(
    source: &str,
    analysis: &AnalysisResult,
    segment: &crate::segmenter::SegmentedCommand,
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

fn selected_registry_words_with_context(
    source: &str,
    config: tcl_lexer::LexerConfig,
    realm: &crate::realm::CommandBindingRealm,
    segment: &crate::segmenter::SegmentedCommand,
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
    segment: &crate::segmenter::SegmentedCommand,
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
    segment: &crate::segmenter::SegmentedCommand,
    registry: &CommandRegistry,
) -> Option<OriginalRegistryWords> {
    let image = retained_document_image(source, realm, config)?;
    selected_registry_words_in_original_image(&image, config, realm, segment, registry)
}

pub(crate) fn selected_registry_words_in_original_image(
    image: &tcl_lexer::SourceImage,
    config: tcl_lexer::LexerConfig,
    realm: &crate::realm::CommandBindingRealm,
    segment: &crate::segmenter::SegmentedCommand,
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

fn selected_registry_words_from_tokens(
    image: &tcl_lexer::SourceImage,
    config: tcl_lexer::LexerConfig,
    tokens: &crate::ir::CommandTokens,
    segment: &crate::segmenter::SegmentedCommand,
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
fn conditional_native_case_source_words(
    source: &str,
    config: tcl_lexer::LexerConfig,
    realm: &crate::realm::CommandBindingRealm,
    input: &crate::analyser::ResolvedAnalysisInput,
    context: &tcl_registry::model::ContextRegistry,
    segment: &crate::segmenter::SegmentedCommand,
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
fn original_source_argument_words(
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

fn effective_operand_sources(
    segment: &crate::segmenter::SegmentedCommand,
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

/// Readonly source argument topology of the actual retained procedure
/// implementation. Formal bindings describe argv ordinals only; they do not
/// install a frame, read a cell, prepare a native call or permit an edit.
pub struct OriginalProcedureArguments<'a> {
    declaration: &'a crate::signature_scan::original_name::SourceDeclarationMetadata<
        crate::analyser::ProcDef,
    >,
    formals: crate::signature_scan::formal_parameters::SignatureSourceFormalParameters,
    bindings: Vec<tcl_syntax::formal_params::FormalByteArgumentBinding>,
    arguments: Vec<crate::registry_invocation::EffectiveInvocationWord>,
    operands: Vec<Option<OriginalOperandSource>>,
    image: tcl_lexer::SourceImage,
    config: tcl_lexer::LexerConfig,
    registry: tcl_registry::RegistrySemanticKey,
}

impl<'a> OriginalProcedureArguments<'a> {
    /// Canonical immutable declaration selected by the implementation allocation.
    #[must_use]
    pub const fn declaration(
        &self,
    ) -> &'a crate::signature_scan::original_name::SourceDeclarationMetadata<crate::analyser::ProcDef>
    {
        self.declaration
    }
    /// Actual original `ParamList` and its independently selected source grammar.
    #[must_use]
    pub const fn formals(
        &self,
    ) -> &crate::signature_scan::formal_parameters::SignatureSourceFormalParameters {
        &self.formals
    }
    /// Required/default/rest/caller-link syntax at this exact effective argc.
    #[must_use]
    pub fn bindings(&self) -> &[tcl_syntax::formal_params::FormalByteArgumentBinding] {
        &self.bindings
    }
    /// Effective post-head values, with captured prefix slots preserved.
    #[must_use]
    pub fn arguments(&self) -> &[crate::registry_invocation::EffectiveInvocationWord] {
        &self.arguments
    }
    /// Source anchors of those exact effective values; captured operands have none.
    #[must_use]
    pub fn operands(&self) -> &[Option<OriginalOperandSource>] {
        &self.operands
    }
    /// Exact immutable complete source/channel and lexer configuration.
    #[must_use]
    pub fn matches_source(
        &self,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
    ) -> bool {
        self.image == *image && self.config == config && self.formals.matches_source(image, config)
    }
    /// Registry generation used for the independently selected formal grammar.
    #[must_use]
    pub fn matches_registry(&self, registry: &CommandRegistry) -> bool {
        self.registry == registry.snapshot().semantic_key()
    }
}

/// Match an actual original procedure reference to its same-document immutable
/// declaration and exact effective argv. Alias prefixes and expansion children
/// retain their own origins; unknown lookup, foreign formals, ambiguous records,
/// unavailable child extents and changed source cannot acquire another anchor.
#[must_use]
pub fn original_procedure_arguments<'a>(
    source: &str,
    analysis: &'a AnalysisResult,
    segment: &crate::segmenter::SegmentedCommand,
) -> Option<OriginalProcedureArguments<'a>> {
    // naming.source.original-procedure-argument-topology
    // docs/design/analysis/name-resolution-proofs/original-procedure-argument-topology.md
    let descriptor = original_procedure_source_descriptor(source, analysis, segment)?;
    let words = descriptor
        .arguments
        .iter()
        .map(|word| word.as_registry_word())
        .collect::<Vec<_>>();
    let count = InvocationArguments::structured(&words).exact_argv_len()?;
    let bindings = descriptor.formals.bindings(count).ok()?;
    Some(OriginalProcedureArguments {
        declaration: descriptor.declaration,
        formals: descriptor.formals,
        bindings,
        arguments: descriptor.arguments,
        operands: descriptor.operands,
        image: descriptor.image,
        config: descriptor.config,
        registry: descriptor.registry,
    })
}

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
    original: tcl_lexer::NativeWord,
    gaps: Vec<tcl_lexer::Span>,
    registry: tcl_registry::RegistrySemanticKey,
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
    original: tcl_lexer::NativeWord,
    replacement: String,
    input: crate::analyser::ResolvedAnalysisInput,
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
    original: tcl_lexer::NativeWord,
    replacement: String,
    input: crate::analyser::ResolvedAnalysisInput,
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

/// Source structure from the selected original argv, or the independently
/// retained conditional schema. The latter keeps its applicability obligations
/// and grants no runtime dispatch, entered frame, compiler or edit permission.
pub fn source_registry_words(
    source: &str,
    analysis: &AnalysisResult,
    segment: &crate::segmenter::SegmentedCommand,
) -> Option<OriginalRegistryWords> {
    source_registry_words_with_definition_parent(source, analysis, segment, None)
}

/// Same source schema issuer with the actual enclosing definition-body receipt.
/// The original full argv must reborrow from that parent's complete body plan.
#[must_use]
pub fn source_registry_words_in_definition_body(
    source: &str,
    analysis: &AnalysisResult,
    segment: &crate::segmenter::SegmentedCommand,
    parent: &super::source_scoped_body::OriginalSourceScriptBody,
) -> Option<OriginalRegistryWords> {
    source_registry_words_with_definition_parent(source, analysis, segment, Some(parent))
}

/// Reborrow one exact child command from an authenticated definition-body
/// plan before applying its selected authored source schema. The complete
/// parent command vector remains mandatory; this is not a fresh child entry.
#[must_use]
pub fn source_registry_words_in_definition_body_at(
    source: &str,
    analysis: &AnalysisResult,
    offset: u32,
    parent: &super::source_scoped_body::OriginalSourceScriptBody,
) -> Option<OriginalRegistryWords> {
    use super::source_scoped_body::OriginalSourceDefinitionMemberDisposition as Disposition;
    let config = analysis.body_lexer_config?;
    let input = analysis.resolved_input.as_ref()?;
    (config == input.lexer_config()).then_some(())?;
    let realm = analysis.retained_command_realm()?;
    realm.matches_resolved_analysis_input(input).then_some(())?;
    let image = retained_document_image(source, realm, config)?;
    analysis
        .matches_original_source_image(&image, config)
        .then_some(())?;
    if !parent.matches_source(&image, config) || !parent.matches_context(&input.context_registry())
    {
        return None;
    }
    let segment = if let Some((segment, _)) = realm
        .invocation_at_source("", offset)
        .original_recorded_command()
    {
        segment
    } else if let Some(segment) = analysis
        .resolved_input
        .as_ref()
        .and_then(|input| realm.original_source_catalogue_segment_at(input, offset))
    {
        segment
    } else {
        let tail = source.get(usize::try_from(offset).ok()?..)?;
        first_tail_command(tail, offset, config.nested())?
    };
    (segment.argv.first()?.span.start() == offset).then_some(())?;
    let context = input.context_registry();
    let members = parent.definition_member_region(&context, parent.content_span())?;
    members.complete_original_commands()?;
    let original = members.original_command_words_at(offset)?;
    let mut tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::from_image(&image),
        config,
        &segment,
    );
    realm.stamp_original_tokens(&mut tokens);
    let selected = crate::registry_invocation::original_native_compiler_words(
        &image,
        tokens.words(),
        offset,
        config,
    )?;
    (selected.as_slice() == original).then_some(())?;
    match members.script_body_disposition(&context, original) {
        Disposition::Unavailable => None,
        Disposition::CompleteNonMember => source_registry_words(source, analysis, &segment)
            .or_else(|| generic_definition_child_source_words(source, analysis, &segment, parent)),
        Disposition::CompleteMember(_) => {
            source_registry_words_in_definition_body(source, analysis, &segment, parent)
        }
    }
}

struct DefinitionChildOperands {
    arguments: Vec<crate::registry_invocation::EffectiveInvocationWord>,
    operands: Vec<Option<OriginalOperandSource>>,
    origins: Vec<crate::registry_invocation::InvocationWordOrigin>,
}

fn original_definition_child_operands(
    original: &[tcl_lexer::NativeWord],
) -> DefinitionChildOperands {
    use crate::registry_invocation::{EffectiveInvocationWord, InvocationWordOrigin};
    let mut arguments = Vec::with_capacity(original.len().saturating_sub(1));
    let mut operands = Vec::with_capacity(arguments.capacity());
    let mut origins = vec![InvocationWordOrigin::Written(0)];
    for (index, word) in original.iter().enumerate().skip(1) {
        let value = if word.group().expand {
            EffectiveInvocationWord::Expanded
        } else if let Some(value) = tcl_syntax::word_rules::original_static_word_unicode_value(word)
        {
            EffectiveInvocationWord::ByteLiteral(std::sync::Arc::from(value))
        } else {
            EffectiveInvocationWord::Dynamic
        };
        arguments.push(value);
        operands.push(Some(OriginalOperandSource {
            span: word.span(),
            input: None,
            word: Some(word.clone()),
        }));
        origins.push(InvocationWordOrigin::Written(index));
    }
    DefinitionChildOperands {
        arguments,
        operands,
        origins,
    }
}

fn original_definition_child_head(
    original: &[tcl_lexer::NativeWord],
) -> Option<OriginalOperandSource> {
    Some(OriginalOperandSource {
        span: original.first()?.span(),
        input: None,
        word: Some(original.first()?.clone()),
    })
}

// naming.source.original-generic-definition-child-registry-roles
// docs/design/analysis/name-resolution-proofs/source-original-generic-definition-child-registry-roles.md
fn generic_definition_child_source_words(
    source: &str,
    analysis: &AnalysisResult,
    segment: &crate::segmenter::SegmentedCommand,
    parent: &super::source_scoped_body::OriginalSourceScriptBody,
) -> Option<OriginalRegistryWords> {
    let input = analysis.resolved_input.as_ref()?;
    let config = input.lexer_config();
    let image = retained_document_image(source, analysis.retained_command_realm()?, config)?;
    let context = input.context_registry();
    if analysis.body_lexer_config != Some(config)
        || !analysis.matches_original_source_image(&image, config)
        || !parent.matches_source(&image, config)
        || !parent.matches_context(&context)
    {
        return None;
    }
    let head = segment.argv.first()?;
    let offset = head.span.start();
    let members = parent.definition_member_region(&context, parent.content_span())?;
    let originals = members.complete_original_commands()?;
    let original = members.original_command_words_at(offset)?;
    let tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::from_image(&image),
        config,
        segment,
    );
    let native = crate::registry_invocation::original_native_compiler_words(
        &image,
        tokens.words(),
        offset,
        config,
    )?;
    (native.as_slice() == original && originals.iter().any(|words| words == original))
        .then_some(())?;
    let head_bytes = tcl_syntax::word_rules::original_static_word_unicode_value(original.first()?)?;
    let head_text = std::str::from_utf8(&head_bytes).ok()?;
    let static_values = original
        .iter()
        .skip(1)
        .map(tcl_syntax::word_rules::original_static_word_unicode_value)
        .collect::<Vec<_>>();
    let values = original
        .iter()
        .skip(1)
        .zip(&static_values)
        .map(|(word, value)| {
            if word.group().expand {
                tcl_registry::InvocationWord::Expanded
            } else if let Some(value) = value {
                tcl_registry::InvocationWord::KnownBytes(value.as_slice())
            } else {
                tcl_registry::InvocationWord::Dynamic
            }
        })
        .collect::<Vec<_>>();
    let mut invocation = tcl_registry::InvocationWords::structured(
        tcl_registry::InvocationWord::Literal(head_text),
        &values,
    );
    if let Some(dialect) = parent.source_words().dialect() {
        invocation = invocation.with_dialect(dialect);
    }
    // Registry source roles use the actual retained context's authoring realm;
    // this source-only facet must not manufacture a RuleLoader/runtime realm.
    let selected = tcl_registry::model::assembly::resolve_invocation_words_in_context(
        context.commands(),
        Some(context.context()),
        invocation,
    )?;
    let (selected_roles, complete) = selected.authored_source_argument_roles();
    if !complete {
        return None;
    }
    let roles = selected_roles
        .into_iter()
        .map(|(position, role)| {
            selected
                .semantics
                .argument_offset
                .checked_add(usize::from(position))
                .map(|position| (position, role))
        })
        .collect::<Option<Vec<_>>>()?;
    let DefinitionChildOperands {
        arguments,
        operands,
        origins,
    } = original_definition_child_operands(original);
    let head = original_definition_child_head(original)?;
    Some(OriginalRegistryWords {
        command: selected.canonical_command.to_owned(),
        dialect: parent.source_words().dialect(),
        arguments,
        operands,
        origins,
        head: Some(head),
        roles: Some(roles),
        source: OriginalRegistrySource::Selected,
        captured_values: None,
        operands_preserve_source_lookup: false,
        image,
        config,
        registry: context.commands().snapshot().semantic_key(),
        context: Some(context.context().clone()),
        definition_parent: Some(std::sync::Arc::new(parent.clone())),
    })
}

#[cfg(test)]
fn trace_source_registry_input(
    analysis: &AnalysisResult,
    segment: &crate::segmenter::SegmentedCommand,
) {
    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_CHILD_GUARDS").is_some() {
        let offset = segment.argv.first().map(|word| word.span.start());
        trace_original_child_guard(
            "source.config",
            offset,
            analysis.body_lexer_config.is_some(),
        );
        trace_original_child_guard(
            "source.realm",
            offset,
            analysis.retained_command_realm().is_some(),
        );
        trace_original_child_guard("source.input", offset, analysis.resolved_input.is_some());
        trace_original_child_guard(
            "source.registry",
            offset,
            analysis.resolved_registry().is_some(),
        );
    }
}

fn trace_source_registry_barrier(
    segment: &crate::segmenter::SegmentedCommand,
    tokens: &crate::ir::CommandTokens,
    blocked: bool,
) {
    #[cfg(not(any(debug_assertions, test)))]
    let _ = blocked;
    trace_remote_source_flag(
        "source.binding",
        segment.argv.first().map(|word| word.span.start()),
        tokens.source_binding.is_some(),
    );
    #[cfg(debug_assertions)]
    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_BODY_SYNTAX").is_some() {
        eprintln!(
            "ORIGINAL_SOURCE_ROLE_ROUTE stage=registry.barrier offset={:?} blocked={blocked}",
            segment.argv.first().map(|word| word.span.start())
        );
    }
    #[cfg(test)]
    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_CHILD_GUARDS").is_some() {
        let offset = segment.argv.first().map(|word| word.span.start());
        trace_original_child_guard("source.barrier", offset, !blocked);
        trace_original_child_guard("source.binding", offset, tokens.source_binding.is_some());
    }
}

fn source_vendor_registry_words(
    source: &str,
    analysis: &AnalysisResult,
    segment: &crate::segmenter::SegmentedCommand,
    image: &tcl_lexer::SourceImage,
    config: tcl_lexer::LexerConfig,
    offset: u32,
) -> Option<OriginalRegistryWords> {
    let (metadata, _) = trace_remote_source_option(
        "source.selected_vendor",
        Some(offset),
        selected_vendor_registry_words_at(source, analysis, offset),
    )?;
    let context = analysis.resolved_input.as_ref()?.context_registry();
    let mut tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::from_image(image),
        config,
        segment,
    );
    analysis
        .retained_command_realm()?
        .stamp_original_tokens(&mut tokens);
    // The shared builder authenticates the complete original native
    // vector. Resegmentation's comment trivia is not word identity.
    trace_remote_source_option(
        "source.vendor_words",
        Some(offset),
        original_vendor_registry_words(&context, &tokens, metadata),
    )
}

fn source_native_registry_words(
    source: &str,
    analysis: &AnalysisResult,
    segment: &crate::segmenter::SegmentedCommand,
    geometry: (&tcl_lexer::SourceImage, tcl_lexer::LexerConfig, u32),
    realm: &crate::realm::CommandBindingRealm,
    input: &crate::analyser::ResolvedAnalysisInput,
    tokens: &crate::ir::CommandTokens,
) -> Option<OriginalRegistryWords> {
    let (image, config, offset) = geometry;
    let metadata = analysis
        .original_conditional_registry_metadata_in_source(image, config, offset)
        .cloned();
    let context = analysis.resolved_input.as_ref()?.context_registry();
    trace_original_source_role_words(
        "registry.native",
        Some(offset),
        native_source_registry_words(
            source,
            config,
            analysis.retained_command_realm()?,
            &context,
            segment,
            metadata,
        ),
    )
    .or_else(|| {
        let advice = trace_original_source_role_option(
            "registry.prefix_advice",
            Some(offset),
            realm.original_source_prefix_catalogue_advice(input, tokens),
        )?;
        trace_original_source_role_words(
            "registry.prefix_words",
            Some(offset),
            source_transition_words_with_segment(source, config, &context, segment, advice),
        )
    })
    .or_else(|| {
        let advice = trace_original_source_role_option(
            "registry.member_advice",
            Some(offset),
            realm.original_member_source_catalogue_advice(input, tokens),
        )?;
        trace_original_source_role_words(
            "registry.member_words",
            Some(offset),
            source_transition_words_with_segment(source, config, &context, segment, advice),
        )
    })
}

fn source_registry_words_with_definition_parent(
    source: &str,
    analysis: &AnalysisResult,
    segment: &crate::segmenter::SegmentedCommand,
    parent: Option<&super::source_scoped_body::OriginalSourceScriptBody>,
) -> Option<OriginalRegistryWords> {
    #[cfg(test)]
    trace_source_registry_input(analysis, segment);
    let config = analysis.body_lexer_config?;
    let realm = analysis.retained_command_realm()?;
    let image = trace_remote_source_option(
        "source.image",
        segment.argv.first().map(|word| word.span.start()),
        retained_document_image(source, realm, config),
    );
    #[cfg(test)]
    trace_original_child_guard(
        "source.image",
        segment.argv.first().map(|word| word.span.start()),
        image.is_some(),
    );
    let image = image?;
    let matches_image = trace_remote_source_flag(
        "source.image_config",
        segment.argv.first().map(|word| word.span.start()),
        analysis.matches_original_source_image(&image, config),
    );
    #[cfg(test)]
    trace_original_child_guard(
        "source.image_config",
        segment.argv.first().map(|word| word.span.start()),
        matches_image,
    );
    matches_image.then_some(())?;
    // The image check above proves `source` is the analysed document, so a
    // parentless answer is a function of the segment and the stamped inputs.
    if parent.is_none() {
        if let Some(memoised) = analysis.memoised_source_words(segment) {
            return memoised.0;
        }
        let words = source_registry_words_from_image(
            source, analysis, segment, None, &image, config, realm,
        );
        analysis.memoise_source_words(segment, words.as_ref());
        return words;
    }
    source_registry_words_from_image(source, analysis, segment, parent, &image, config, realm)
}

fn source_registry_words_from_image(
    source: &str,
    analysis: &AnalysisResult,
    segment: &crate::segmenter::SegmentedCommand,
    parent: Option<&super::source_scoped_body::OriginalSourceScriptBody>,
    image: &tcl_lexer::SourceImage,
    config: tcl_lexer::LexerConfig,
    realm: &crate::realm::CommandBindingRealm,
) -> Option<OriginalRegistryWords> {
    let registry = analysis.resolved_registry()?;
    let input = analysis.resolved_input.as_ref()?;
    let mut tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::from_image(image),
        config,
        segment,
    );
    realm.stamp_original_tokens(&mut tokens);
    #[cfg(debug_assertions)]
    trace_original_source_role_binding(
        "registry",
        segment.argv.first().map(|word| word.span.start()),
        tokens.source_binding.as_ref(),
    );
    let blocked = trace_remote_source_flag(
        "source.blocked",
        segment.argv.first().map(|word| word.span.start()),
        realm.original_source_registry_barrier(&input.context_registry(), &tokens),
    );
    trace_source_registry_barrier(segment, &tokens, blocked);
    if blocked {
        return None;
    }
    if let Some(parent) = parent {
        return original_definition_member_registry_words(source, analysis, segment, parent);
    }
    if let Some(scoped) = trace_original_source_role_words(
        "registry.scoped",
        segment.argv.first().map(|word| word.span.start()),
        original_scoped_registry_words(source, analysis, segment),
    ) {
        return Some(scoped);
    }
    if realm
        .source_bindings_ref()
        .original_logical_source_name_advice_input()
        == Some(input)
    {
        return logical_source_registry_words(source, realm, input, segment);
    }
    if !analysis.has_original_vendor_source_names()
        && let Some(selected) = trace_original_source_role_words(
            "registry.selected",
            segment.argv.first().map(|word| word.span.start()),
            selected_registry_words(source, analysis, segment, registry),
        )
    {
        return Some(selected);
    }
    let offset = segment.argv.first()?.span.start();
    if analysis.has_original_vendor_source_names() {
        return source_vendor_registry_words(source, analysis, segment, image, config, offset);
    }
    source_native_registry_words(
        source,
        analysis,
        segment,
        (image, config, offset),
        realm,
        input,
        &tokens,
    )
}

fn source_definition_body_grammar(
    schema: &tcl_registry::ResolvedInvocation<'_, '_>,
) -> Option<&'static tcl_registry::definer::DefinitionBodyGrammar> {
    schema.authored_source_definition_body_grammar()
}

fn original_definition_member_registry_words(
    source: &str,
    analysis: &AnalysisResult,
    segment: &crate::segmenter::SegmentedCommand,
    parent: &super::source_scoped_body::OriginalSourceScriptBody,
) -> Option<OriginalRegistryWords> {
    let config = analysis.body_lexer_config?;
    let input = analysis.resolved_input.as_ref()?;
    if config != input.lexer_config() || segment.is_partial {
        return None;
    }
    let context = input.context_registry();
    let realm = analysis.retained_command_realm()?;
    if !realm.matches_resolved_analysis_input(input) {
        return None;
    }
    let image = retained_document_image(source, realm, config)?;
    if !parent.matches_source(&image, config) || !parent.matches_context(&context) {
        return None;
    }
    let region = parent.definition_member_region(&context, parent.content_span())?;
    region.complete_original_commands()?;
    let offset = segment.argv.first()?.span.start();
    let original = region.original_command_words_at(offset)?;
    let mut tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::from_image(&image),
        config,
        segment,
    );
    realm.stamp_original_tokens(&mut tokens);
    let selected = crate::registry_invocation::original_native_compiler_words(
        &image,
        tokens.words(),
        offset,
        config,
    )?;
    if selected != original {
        return None;
    }
    let name = tcl_syntax::word_rules::original_static_word_ascii_presentation(original.first()?)?;
    let name = std::str::from_utf8(&name).ok()?;
    let grammar = parent
        .source_words()
        .with_source_schema(&context, |schema| {
            schema.authored_source_definition_body_grammar()
        })
        .flatten()?;
    context
        .commands()
        .authored_document_member_descriptor(grammar, name)?;
    original_source_schema_words(
        &tokens,
        original,
        name.to_owned(),
        OriginalRegistrySource::DefinitionMember(std::sync::Arc::new(parent.clone())),
        &context,
    )
}

fn original_scoped_registry_words(
    source: &str,
    analysis: &AnalysisResult,
    segment: &crate::segmenter::SegmentedCommand,
) -> Option<OriginalRegistryWords> {
    let config = analysis.body_lexer_config?;
    let realm = analysis.retained_command_realm()?;
    let image = retained_document_image(source, realm, config)?;
    let offset = segment.argv.first()?.span.start();
    let body = analysis.original_scoped_body_in_source(&image, config, offset)?;
    let context = analysis.resolved_input.as_ref()?.context_registry();
    let mut tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::from_image(&image),
        config,
        segment,
    );
    realm.stamp_original_tokens(&mut tokens);
    let original = crate::registry_invocation::original_native_compiler_words(
        &image,
        tokens.words(),
        offset,
        config,
    )?;
    let first = original.first()?;
    let last = original.last()?;
    let extent = body.content_span();
    if first.span().start() < extent.start() || last.span().end() > extent.end() {
        return None;
    }
    let name = tcl_syntax::word_rules::original_static_word_ascii_presentation(first)?;
    let name = std::str::from_utf8(&name).ok()?;
    let schema = super::source_scoped_body::OriginalSourceScopedCommandSchema::select(body, name)?;
    original_source_schema_words(
        &tokens,
        &original,
        schema.command().name.to_owned(),
        OriginalRegistrySource::Scoped(std::sync::Arc::new(schema)),
        &context,
    )
}

fn original_source_schema_words(
    tokens: &crate::ir::CommandTokens,
    original: &[tcl_lexer::NativeWord],
    command: String,
    source: OriginalRegistrySource,
    context: &tcl_registry::model::ContextRegistry,
) -> Option<OriginalRegistryWords> {
    let first = original.first()?;
    let image = first.image().clone();
    let config = first.config();
    let arguments = original
        .iter()
        .skip(1)
        .map(|word| {
            if word.group().expand {
                crate::registry_invocation::EffectiveInvocationWord::Expanded
            } else {
                let presentation = if matches!(&source, OriginalRegistrySource::DefinitionMember(_))
                {
                    tcl_syntax::word_rules::original_static_word_source_bytes(word)
                } else {
                    tcl_syntax::word_rules::original_static_word_ascii_presentation(word)
                };
                presentation.map_or(
                    crate::registry_invocation::EffectiveInvocationWord::Dynamic,
                    |value| {
                        crate::registry_invocation::EffectiveInvocationWord::ByteLiteral(
                            std::sync::Arc::from(value),
                        )
                    },
                )
            }
        })
        .collect::<Vec<_>>();
    let operands = original
        .iter()
        .skip(1)
        .map(|word| {
            Some(OriginalOperandSource {
                span: word.span(),
                input: None,
                word: Some(word.clone()),
            })
        })
        .collect();
    let head = Some(OriginalOperandSource {
        span: first.span(),
        input: None,
        word: Some(first.clone()),
    });
    let mut words = OriginalRegistryWords {
        command,
        dialect: tokens
            .source_binding
            .as_ref()
            .and_then(|binding| binding.original_source_word_dialect_for_tokens(tokens)),
        arguments,
        operands,
        origins: (0..original.len())
            .map(crate::registry_invocation::InvocationWordOrigin::Written)
            .collect(),
        head,
        roles: None,
        source,
        captured_values: None,
        operands_preserve_source_lookup: false,
        image,
        config,
        registry: context.commands().snapshot().semantic_key(),
        context: Some(context.context().clone()),
        definition_parent: None,
    };
    words.roles = words
        .with_source_schema(context, |schema| {
            let (roles, complete) = schema.authored_source_argument_roles();
            conditional_roles(&roles, schema.semantics.argument_offset, complete)
        })
        .flatten();
    Some(words)
}

/// Source schema at one original invocation offset under the same retained
/// analysis. A stored complete vector is preferred; conditional future/body
/// source advice retains its own full-image issuer through the shared builder.
/// No reporting command string selects the schema or supplies execution.
#[must_use]
pub fn source_registry_words_at(
    source: &str,
    analysis: &AnalysisResult,
    offset: u32,
) -> Option<OriginalRegistryWords> {
    // naming.diagnostic.original-package-source-advice
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-package-source-advice.md
    if let Some(region) = analysis.original_source_region.as_ref()
        && !trace_remote_source_flag(
            "at.region_owns",
            Some(offset),
            region.owns_original_command_at(analysis, offset),
        )
    {
        return None;
    }
    let config = trace_remote_source_option("at.config", Some(offset), analysis.body_lexer_config)?;
    let image = original_query_document_image(source, analysis, config);
    trace_remote_source_flag(
        "at.image_config",
        Some(offset),
        analysis.matches_original_source_image(&image, config),
    )
    .then_some(())?;
    let realm =
        trace_remote_source_option("at.realm", Some(offset), analysis.retained_command_realm())?;
    let binding = realm.invocation_at_source("", offset);
    let segment = if let Some((segment, _)) = binding.original_recorded_command() {
        segment
    } else if let Some(segment) = analysis
        .resolved_input
        .as_ref()
        .and_then(|input| realm.original_source_catalogue_segment_at(input, offset))
    {
        segment
    } else {
        let tail = source.get(usize::try_from(offset).ok()?..)?;
        first_tail_command(tail, offset, config.nested())?
    };
    (segment.argv.first()?.span.start() == offset).then_some(())?;
    trace_remote_source_option(
        "at.source_registry_words",
        Some(offset),
        source_registry_words(source, analysis, &segment),
    )
}

/// Diagnostic identity: reports only the original query result.
#[inline]
pub(super) fn trace_remote_source_option<T>(
    stage: &str,
    offset: Option<u32>,
    value: Option<T>,
) -> Option<T> {
    #[cfg(test)]
    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_REMOTE_FIRST_OPERAND").is_some() {
        eprintln!(
            "REMOTE_SOURCE_WORDS stage={stage} offset={offset:?} present={}",
            value.is_some()
        );
    }
    #[cfg(not(test))]
    let _ = (stage, offset);
    value
}

/// Diagnostic identity: reports only the original predicate result.
#[inline]
pub(super) fn trace_remote_source_flag(stage: &str, offset: Option<u32>, value: bool) -> bool {
    #[cfg(test)]
    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_REMOTE_FIRST_OPERAND").is_some() {
        eprintln!("REMOTE_SOURCE_WORDS stage={stage} offset={offset:?} value={value}");
    }
    #[cfg(not(test))]
    let _ = (stage, offset);
    value
}

/// Original whole words under a positively retained Logical source input.
/// Missing Native or hosted producers cannot enter this compatibility domain.
/// The vector is readonly geometry, without a Native name or runtime grant.
#[must_use]
pub fn original_logical_source_words_at(
    source: &str,
    analysis: &AnalysisResult,
    offset: u32,
) -> Option<Vec<tcl_lexer::NativeWord>> {
    // naming.core.original-package-source-action-context
    // docs/design/analysis/name-resolution-proofs/core-original-package-source-action-context.md
    let config = analysis.body_lexer_config?;
    let image = original_query_document_image(source, analysis, config);
    analysis
        .matches_original_source_image(&image, config)
        .then_some(())?;
    let realm = analysis.retained_command_realm()?;
    let input = analysis.resolved_input.as_ref()?;
    (realm
        .source_bindings_ref()
        .original_logical_source_name_advice_input()
        == Some(input))
    .then_some(())?;
    let binding = realm.invocation_at_source("", offset);
    if let Some((_, tokens)) = binding.original_recorded_command() {
        return super::original_native_compiler_words(&image, tokens.words(), offset, config);
    }
    let original = realm
        .source_bindings_ref()
        .original_logical_source_command_at_in_source(input, &image, offset, config)?;
    Some(original.native_words().to_vec())
}

/// Original Registry-created instance syntax at its actual source occurrence.
/// The full existing Analysis owns both factory and invocation; a source slice
/// is never recaptured as a fresh entry, nor is a reporting class name looked up.
#[must_use]
pub fn source_registered_instance_words_at(
    source: &str,
    analysis: &AnalysisResult,
    offset: u32,
) -> Option<crate::command_binding::OriginalSourceRegisteredInstanceWords> {
    let (input, realm, native) = registered_original_words_at(source, analysis, offset)?;
    realm.original_source_registered_instance_words(input, &native)
}

/// Original whole handle setter and its selected constructor substitution.
/// This is authored source ownership, without cell, allocation or value proof.
#[must_use]
pub fn source_registered_handle_binding_at(
    source: &str,
    analysis: &AnalysisResult,
    offset: u32,
) -> Option<crate::command_binding::OriginalSourceRegisteredHandleBinding> {
    let (input, realm, native) = registered_original_words_at(source, analysis, offset)?;
    realm.original_source_registered_handle_binding(input, &native)
}

/// Original future command naming layout, retaining source applicability only.
#[must_use]
pub fn source_command_publication_at(
    source: &str,
    analysis: &AnalysisResult,
    offset: u32,
) -> Option<super::OriginalSourceCommandPublication> {
    let words = source_registry_words_at(source, analysis, offset)?;
    let context = analysis.resolved_input.as_ref()?.context_registry();
    super::source_publication::OriginalSourceCommandPublication::capture(words, &context)
}

/// Genuine setter operands and complete single-command construction syntax.
/// No value, frame, allocation or successful store is issued.
#[must_use]
pub fn source_handle_construction_at(
    source: &str,
    analysis: &AnalysisResult,
    offset: u32,
) -> Option<super::OriginalSourceHandleConstruction> {
    let words = source_registry_words_at(source, analysis, offset)?;
    let context = analysis.resolved_input.as_ref()?.context_registry();
    super::source_publication::OriginalSourceHandleConstruction::capture(words, &context)
}

/// Same-horizon Logical name references from genuine selected command-name
/// operands. This is conditional source advice, not an executed lookup or edit.
#[must_use]
pub fn source_logical_name_references(
    source: &str,
    analysis: &AnalysisResult,
) -> Option<Vec<crate::command_binding::OriginalLogicalNameReference>> {
    let (input, realm) = logical_source_candidate_context(source, analysis)?;
    realm.original_logical_name_reference_inventory(input)
}

/// Positive original procedure-body commands with their genuine source callers.
/// Missing or unsupported bodies do not establish complete command coverage.
#[must_use]
pub fn source_native_procedure_body_commands(
    source: &str,
    analysis: &AnalysisResult,
) -> Option<Vec<crate::command_binding::OriginalNativeProcedureBodyCommand>> {
    let input = analysis.resolved_input.as_ref()?;
    let image = original_query_document_image(source, analysis, input.lexer_config());
    analysis
        .matches_original_source_image(&image, input.lexer_config())
        .then_some(())?;
    analysis
        .retained_command_realm()?
        .original_native_procedure_body_commands(input)
}

/// Positive conditional source calls retained by the original Native source walk.
/// Missing sites and unresolved body coverage cannot prove negative call closure.
#[must_use]
pub fn source_original_procedure_calls(
    source: &str,
    analysis: &AnalysisResult,
) -> Option<Vec<crate::command_binding::OriginalSourceProcedureCall>> {
    let input = analysis.resolved_input.as_ref()?;
    let image = original_query_document_image(source, analysis, input.lexer_config());
    analysis
        .matches_original_source_image(&image, input.lexer_config())
        .then_some(())?;
    analysis
        .retained_command_realm()?
        .original_source_procedure_calls(input)
}

/// Conditional Native-authored procedure source relation at the actual call horizon.
/// Runtime lookup, body entry and transformation authority remain independent.
#[must_use]
pub fn source_original_procedure_call_at(
    source: &str,
    analysis: &AnalysisResult,
    offset: u32,
) -> Option<crate::command_binding::OriginalSourceProcedureCall> {
    let input = analysis.resolved_input.as_ref()?;
    let image = original_query_document_image(source, analysis, input.lexer_config());
    analysis
        .matches_original_source_image(&image, input.lexer_config())
        .then_some(())?;
    let realm = analysis.retained_command_realm()?;
    realm.original_source_procedure_call_at(input, offset)
}

/// Conditional original procedure header at this exact source call horizon.
/// Alias prefixes and actual definition transitions stay held; missing original
/// owners remain unavailable, never a report-name or Native target fallback.
#[must_use]
pub fn source_logical_procedure_call_at(
    source: &str,
    analysis: &AnalysisResult,
    offset: u32,
) -> Option<crate::command_binding::OriginalLogicalProcedureCall> {
    let (input, realm) = logical_source_candidate_context(source, analysis)?;
    crate::realm::CommandBindingRealm::original_logical_procedure_call_at(realm, input, offset)
}

/// Represented original Logical procedure calls from the retained source tape.
/// Reporting invocation records cannot supply or veto this independent source
/// occurrence owner. Missing candidates remain unavailable, not an empty census.
#[must_use]
pub fn source_logical_procedure_calls(
    source: &str,
    analysis: &AnalysisResult,
) -> Option<Vec<crate::command_binding::OriginalLogicalProcedureCall>> {
    let (input, realm) = logical_source_candidate_context(source, analysis)?;
    realm.original_logical_procedure_calls(input)
}

/// Represented Registry-selected Logical ensemble transition receipts from
/// the exact current source and configuration. The receipt list preserves
/// original operand ownership without analyzer target reports. Other source
/// mutations may be unrepresented, so empty or partial results prove neither
/// an absent ensemble map nor a current dispatch binding.
#[must_use]
pub fn source_logical_ensemble_transition_receipts(
    source: &str,
    analysis: &AnalysisResult,
) -> Option<Vec<crate::command_binding::OriginalSourceCommandTransition>> {
    let (input, realm) = logical_source_candidate_context(source, analysis)?;
    realm.original_logical_ensemble_transition_receipts(input)
}

/// Select only the original head containing the cursor. Whole retained argv,
/// input and definition joins remain owned by the existing source call factory.
#[must_use]
pub fn source_logical_procedure_call_for_cursor(
    source: &str,
    analysis: &AnalysisResult,
    offset: u32,
) -> Option<crate::command_binding::OriginalLogicalProcedureCall> {
    let mut calls = source_logical_procedure_calls(source, analysis)?
        .into_iter()
        .filter(|call| {
            call.original_words()
                .first()
                .is_some_and(|head| head.span().start() <= offset && offset < head.span().end())
        });
    let selected = calls.next()?;
    calls.next().is_none().then_some(selected)
}

/// Ordered readonly Logical candidates at this exact original source command.
/// Caller scope, path and name bytes remain source recipes; candidates prove no
/// installed command, reached declaration, body entry or edit permission.
#[must_use]
pub fn source_logical_command_candidates_at(
    source: &str,
    analysis: &AnalysisResult,
    offset: u32,
) -> Option<crate::command_binding::OriginalLogicalCommandCandidates> {
    let (input, realm) = logical_source_candidate_context(source, analysis)?;
    realm.original_logical_navigation_candidates(input, offset)
}

/// Represented readonly Logical occurrences from the complete current source.
/// Missing frames or grammar contribute no receipt; empty results establish no
/// command census, installed target or absence.
#[must_use]
pub fn source_logical_command_candidates(
    source: &str,
    analysis: &AnalysisResult,
) -> Option<Vec<crate::command_binding::OriginalLogicalCommandCandidates>> {
    let (input, realm) = logical_source_candidate_context(source, analysis)?;
    realm.original_logical_navigation_inventory(input)
}

/// One literal absolute Logical command head from an authentic retained
/// original body. Caller namespace and method frame remain unavailable; this
/// coordinate supplies no installed lookup, call binding or absence proof.
#[must_use]
pub fn source_logical_absolute_command_candidate_at(
    source: &str,
    analysis: &AnalysisResult,
    offset: u32,
) -> Option<crate::command_binding::OriginalLogicalAbsoluteCommandCandidate> {
    let (input, realm) = logical_source_candidate_context(source, analysis)?;
    realm.original_logical_absolute_navigation_candidate(input, offset)
}

/// Ordered readonly Logical candidates for selected prefix roles at this exact original installer.
/// Caller scope, path and name bytes remain source recipes; candidates prove no
/// installed command, reached declaration, body entry or edit permission.
#[must_use]
pub fn source_logical_callback_candidates_at(
    source: &str,
    analysis: &AnalysisResult,
    offset: u32,
) -> Option<Vec<crate::command_binding::OriginalLogicalCallbackCandidates>> {
    let (input, realm) = logical_source_candidate_context(source, analysis)?;
    realm.original_logical_callback_candidates(input, offset)
}

// The immutable tape already owns the complete original vectors and body
// bounds. A source suffix cannot reconstruct that context at a nested closer.
fn logical_source_candidate_context<'a>(
    source: &str,
    analysis: &'a AnalysisResult,
) -> Option<(
    &'a crate::analyser::ResolvedAnalysisInput,
    &'a crate::realm::CommandBindingRealm,
)> {
    analysis.allows_lexical_declaration_advice().then_some(())?;
    let config = analysis.body_lexer_config?;
    let input = analysis.resolved_input.as_ref()?;
    if config != input.lexer_config() {
        return None;
    }
    let image = tcl_lexer::SourceImage::document(source);
    analysis
        .matches_original_source_image(&image, config)
        .then_some(())?;
    let realm = analysis.retained_command_realm()?;
    realm.matches_resolved_analysis_input(input).then_some(())?;
    Some((input, realm))
}

/// Genuine Logical callback prefix roles from the current complete source tape.
/// Missing frames or grammar contribute no candidate. Empty results prove no
/// complete callback census or absence. Native callbacks retain
/// their independent source lookup and installation owners.
#[must_use]
pub fn source_logical_callback_candidates(
    source: &str,
    analysis: &AnalysisResult,
) -> Option<Vec<crate::command_binding::OriginalLogicalCallbackCandidates>> {
    analysis.allows_lexical_declaration_advice().then_some(())?;
    let input = analysis.resolved_input.as_ref()?;
    analysis
        .matches_original_source_image(
            &tcl_lexer::SourceImage::document(source),
            input.lexer_config(),
        )
        .then_some(())?;
    analysis
        .retained_command_realm()?
        .original_logical_callback_inventory(input)
}

/// Original setter and exact factory substitution for source handle-class advice.
/// This supplies no stored handle, variable cell or successful allocation.
#[must_use]
pub fn source_handle_class_advice_at(
    source: &str,
    analysis: &AnalysisResult,
    offset: u32,
) -> Option<super::OriginalSourceHandleClassAdvice> {
    let words = source_registry_words_at(source, analysis, offset)?;
    let context = analysis.resolved_input.as_ref()?.context_registry();
    super::source_publication::OriginalSourceHandleClassAdvice::capture(
        source, analysis, words, &context,
    )
}

/// Original named Registry factory source declaration. This retains actual
/// factory/name words and conditional applicability, without successful creation.
#[must_use]
pub fn source_registered_factory_at(
    source: &str,
    analysis: &AnalysisResult,
    offset: u32,
) -> Option<crate::command_binding::OriginalSourceRegisteredInstance> {
    let (input, realm, native) = registered_original_words_at(source, analysis, offset)?;
    realm.original_source_registered_factory(input, &native)
}

/// Actual whole source constructor call and its original selected class definer.
/// This supplies source advice only, without a class or constructor execution.
#[must_use]
pub fn source_constructor_call_at(
    source: &str,
    analysis: &AnalysisResult,
    offset: u32,
) -> Option<crate::command_binding::OriginalSourceConstructorCall> {
    let (input, realm, native) = trace_jim_source_option(
        "constructor.registered_words",
        registered_original_words_at(source, analysis, offset),
    )?;
    trace_jim_source_option(
        "constructor.realm_receipt",
        realm.original_source_constructor_call(input, &native),
    )
}

/// Actual whole `Class::proc` call and the [incr Tcl] source class its
/// qualifier names. Source advice only: member declaration and execution
/// remain the consumer's facts.
#[must_use]
pub fn source_class_proc_call_at(
    source: &str,
    analysis: &AnalysisResult,
    offset: u32,
) -> Option<crate::command_binding::OriginalSourceClassProcCall> {
    let (input, realm, native) = registered_original_words_at(source, analysis, offset)?;
    realm.original_source_class_proc_call(input, &native)
}

/// Conditional named or symbolic source-class receiver with whole original argv.
/// This supplies no allocation, stored value, Native dispatch or lifetime grant.
#[must_use]
pub fn source_class_instance_words_at(
    source: &str,
    analysis: &AnalysisResult,
    offset: u32,
) -> Option<crate::command_binding::OriginalSourceClassInstanceWords> {
    let (input, realm, native) = registered_original_words_at(source, analysis, offset)?;
    let receipt = realm.original_source_class_instance_words(input, &native)?;
    receipt
        .instance()
        .constructor()
        .constructor_shape(analysis)?;
    Some(receipt)
}

/// Final conditional procedure source slots and authentic declaration ancestry.
/// The represented source graph honours known moves, deletions and replacement;
/// unknown mutations remain obligations. This grants no completed runtime world.
#[must_use]
pub fn source_procedure_publications(
    source: &str,
    analysis: &AnalysisResult,
) -> Option<crate::command_binding::OriginalSourceProcedurePublications> {
    let input = analysis.resolved_input.as_ref()?;
    let realm = analysis.retained_command_realm()?;
    let image = retained_document_image(source, realm, input.lexer_config())?;
    analysis
        .matches_original_source_image(&image, input.lexer_config())
        .then_some(())?;
    realm.original_source_procedure_publications(input)
}

/// Final conditional class source slots and their authentic factory ancestry.
/// Known moves, deletion and replacement withdraw the old source spelling;
/// unknown operations retain applicability obligations, never Native identity.
#[must_use]
pub fn source_class_publications(
    source: &str,
    analysis: &AnalysisResult,
) -> Option<crate::command_binding::OriginalSourceClassPublications> {
    let input = analysis.resolved_input.as_ref()?;
    let realm = analysis.retained_command_realm()?;
    let image = retained_document_image(source, realm, input.lexer_config())?;
    analysis
        .matches_original_source_image(&image, input.lexer_config())
        .then_some(())?;
    realm.original_source_class_publications(input)
}

/// Conditional callback procedure target from the original registration horizon.
/// Captured alias operands and local canonical declaration remain authentic;
/// callback reach, future table, argv, frame and dispatch stay unresolved.
#[must_use]
pub fn source_callback_procedure_target_at(
    source: &str,
    analysis: &AnalysisResult,
    registration_offset: u32,
    prefix: &crate::command_binding::OriginalCallbackPrefix,
) -> Option<crate::command_binding::OriginalSourceCallbackProcedureLookup> {
    let (input, realm, native) =
        registered_original_words_at(source, analysis, registration_offset)?;
    realm.original_source_callback_procedure_target(input, &native, prefix)
}

/// Attach the genuine selected whole source installer to a readonly prefix.
/// The actual consumer site is checked against the source graph's registration;
/// captured operand producer spans cannot replace it. This issues no Native
/// lookup, installed callback, future frame or successful factory capability.
#[must_use]
pub fn source_callback_prefix_at(
    source: &str,
    analysis: &AnalysisResult,
    registration_offset: u32,
    prefix: &crate::command_binding::OriginalCallbackPrefix,
) -> Option<crate::command_binding::OriginalCallbackPrefix> {
    let original =
        source_callback_procedure_target_at(source, analysis, registration_offset, prefix)?;
    prefix.with_source_registration(&original)
}

/// Original readonly class reference selected at its genuine source consumer.
/// Original list children and any earlier procedure producer retain independent
/// source ownership. This supplies no native class token or runtime inheritance.
#[must_use]
pub fn source_class_reference_at(
    source: &str,
    analysis: &AnalysisResult,
    offset: u32,
    name: &crate::signature_scan::scope::SignatureSourceNameInput,
) -> Option<crate::command_binding::OriginalSourceClassReference> {
    let (input, realm, native) = registered_original_words_at(source, analysis, offset)?;
    realm.original_source_class_reference(input, &native, name)
}

/// Whole original class configuration target and its pretransition source
/// declaration. Logical and Native inputs remain separate; this provides no
/// object identity, entered definition worker, method-table closure or edit.
#[must_use]
pub fn source_configured_class_at(
    source: &str,
    analysis: &AnalysisResult,
    offset: u32,
) -> Option<crate::command_binding::OriginalSourceConfiguredClassReference> {
    let (input, realm, native) = registered_original_words_at(source, analysis, offset)?;
    realm.original_source_configured_class(input, &native)
}

/// Source roles for the exact configured-class invocation retained by the
/// transition owner. This keeps alias/move lineage and the selected target
/// schema attached to the original words; a report label cannot select it.
#[must_use]
// naming.source.original-configured-class-transition-roles
// docs/design/analysis/name-resolution-proofs/source-original-configured-class-transition-roles.md
pub fn source_configured_class_words_at(
    source: &str,
    analysis: &AnalysisResult,
    offset: u32,
) -> Option<OriginalRegistryWords> {
    let config = analysis.body_lexer_config?;
    let input = analysis.resolved_input.as_ref()?;
    let context = input.context_registry();
    let reference = source_configured_class_at(source, analysis, offset)?;
    let image = retained_document_image(source, analysis.retained_command_realm()?, config)?;
    if config != input.lexer_config()
        || !analysis.matches_original_source_image(&image, config)
        || !reference.matches_source_context(&image, config, &context)
    {
        return None;
    }
    source_transition_words_from_advice(source, config, &context, reference.consumer().clone())
}

/// Conditional Jim class lists and base providers from the genuine source tape.
/// This is separate from Native class-declaration and allocated receiver authority.
#[must_use]
pub fn source_authored_jim_class_at(
    source: &str,
    analysis: &AnalysisResult,
    offset: u32,
) -> Option<std::sync::Arc<crate::command_binding::OriginalAuthoredJimClass>> {
    let (input, realm, native) = trace_jim_source_option(
        "authored.registered_words",
        registered_original_words_at(source, analysis, offset),
    )?;
    trace_jim_source_option(
        "authored.realm_receipt",
        realm.original_authored_jim_class(input, &native),
    )
}

/// Exact original selected class declaration at its authentic source occurrence.
#[must_use]
pub fn source_class_declaration_at(
    source: &str,
    analysis: &AnalysisResult,
    offset: u32,
) -> Option<crate::command_binding::OriginalSourceClassDeclaration> {
    let (input, realm, native) = registered_original_words_at(source, analysis, offset)?;
    if input.has_logical_source_name_context() {
        let words = source_registry_words_at(source, analysis, offset)?;
        let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
            return None;
        };
        return realm.original_source_class_declaration_for_advice(input, &native, advice);
    }
    realm.original_source_class_declaration(input, &native)
}

fn registered_original_words_at<'a>(
    source: &str,
    analysis: &'a AnalysisResult,
    offset: u32,
) -> Option<(
    &'a crate::analyser::ResolvedAnalysisInput,
    &'a crate::realm::CommandBindingRealm,
    Vec<tcl_lexer::NativeWord>,
)> {
    if let Some(region) = analysis.original_source_region.as_ref()
        && !region.owns_original_command_at(analysis, offset)
    {
        return None;
    }
    let config = analysis.body_lexer_config?;
    let image = original_query_document_image(source, analysis, config);
    analysis
        .matches_original_source_image(&image, config)
        .then_some(())?;
    let input = analysis.resolved_input.as_ref()?;
    let realm = analysis.retained_command_realm()?;
    realm.matches_resolved_analysis_input(input).then_some(())?;
    let binding = realm.invocation_at_source("", offset);
    let native = if let Some((_, tokens)) = binding.original_recorded_command() {
        super::original_native_compiler_words(&image, tokens.words(), offset, config)?
    } else if let Some(original) = realm.original_authored_jim_parent_words(input, offset) {
        original
    } else if let Some(segment) = realm.original_source_catalogue_segment_at(input, offset) {
        // The retained catalogue bounds a command inside a body script, as in
        // `source_registry_words_at`; a parsed suffix would run past its brace.
        let tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(source),
            config,
            &segment,
        );
        super::original_native_compiler_words(&image, tokens.words(), offset, config)?
    } else {
        // The original tape remains the body/source-role authority below.
        // Parsing a suffix only supplies unchanged word geometry for that join.
        let tail = source.get(usize::try_from(offset).ok()?..)?;
        let segment = first_tail_command(tail, offset, config.nested())?;
        let tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(source),
            config,
            &segment,
        );
        super::original_native_compiler_words(&image, tokens.words(), offset, config)?
    };
    (native.first()?.span().start() == offset).then_some((input, realm, native))
}

/// Current root insertion geometry shared by missing-package diagnostics and
/// reviewed package suggestions. Only selected leading root requirements are
/// header commands; nested scripts, data and prefix-like text cannot supply
/// this position. The full original input and availability remain owners.
#[must_use]
pub fn original_package_require_insert_offset(
    source: &str,
    analysis: &AnalysisResult,
) -> Option<u32> {
    // naming.core.original-package-source-action-context
    // docs/design/analysis/name-resolution-proofs/core-original-package-source-action-context.md
    use tcl_registry::source_navigation::SourcePackageReferenceKind as Kind;
    let config = analysis.body_lexer_config?;
    let image = tcl_lexer::SourceImage::document(source);
    analysis
        .matches_original_source_image(&image, config)
        .then_some(())?;
    let context = analysis.resolved_input.as_ref()?.context_registry();
    let source_end = u32::try_from(source.len()).ok()?;
    let original_commands = tcl_lexer::native_script_words_in(
        image.clone(),
        tcl_lexer::Span::new(0, source_end),
        config,
    )
    .ok()?;
    let mut end =
        if config.leading_bom == tcl_lexer::LeadingBom::Skip && source.starts_with('\u{feff}') {
            3
        } else {
            0
        };
    if source.get(end..)?.starts_with("#!") {
        end = source
            .get(end..)?
            .find('\n')
            .map_or(source.len(), |newline| end + newline + 1);
    }
    for (command_index, segment) in
        crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
            .into_iter()
            .enumerate()
    {
        let Some(words) = source_registry_words(source, analysis, &segment) else {
            break;
        };
        let head = words.head_source()?.word()?;
        let start = usize::try_from(head.span().start()).ok()?;
        let original = original_commands.commands.get(command_index)?;
        let source_extent = original.source_extent?;
        if original.span.start() != segment.span.start()
            || original.span.end() < segment.execution_span(source).end()
            || source_extent.start() != segment.span.start()
        {
            return None;
        }
        let insertion_end = match original.terminator {
            Some(terminator) if terminator.start() == source_extent.end() => terminator.end(),
            None if source_extent.end() == source_end => source_extent.end(),
            Some(_) | None => return None,
        };
        if start < end {
            continue;
        }
        if !source.get(end..start)?.trim().is_empty()
            || words
                .package_reference(&context)
                .is_none_or(|reference| reference.kind() != Kind::Require)
        {
            break;
        }
        end = usize::try_from(insertion_end).ok()?;
    }
    u32::try_from(end).ok()
}

/// Genuine source package-reference operand under its selected source schema.
/// Native package purposes retain the original value recipe. Hosted and
/// Logical source operands keep exact ASCII metadata matching independently;
/// neither path claims a runtime package table, loader or successful call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourcePackageReference {
    words: std::sync::Arc<OriginalRegistryWords>,
    reference: tcl_registry::source_navigation::SourcePackageReference,
    literal: Vec<u8>,
    native_key: Option<tcl_registry::native_package::NativePackageNameKey>,
}
impl OriginalSourcePackageReference {
    /// Same selected whole invocation, including its conditional obligations.
    #[must_use]
    pub fn words(&self) -> &OriginalRegistryWords {
        &self.words
    }
    /// Requirement or actual version-supplying source provision.
    #[must_use]
    pub const fn kind(&self) -> tcl_registry::source_navigation::SourcePackageReferenceKind {
        self.reference.kind
    }
    /// Original effective name ordinal; captured arguments gain no call anchor.
    #[must_use]
    pub const fn argument(&self) -> usize {
        self.reference.argument
    }
    /// Compare one explicitly intended Unicode source value with the same
    /// genuine written package word under its complete source configuration.
    /// This purpose does not compare Native package keys or imply installation.
    /// Missing word/configuration/Unicode correspondence is unavailable.
    #[must_use]
    pub fn matches_unicode_source_value(
        &self,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
        value: &str,
    ) -> Option<bool> {
        self.words.matches_source(image, config).then_some(())?;
        let word = self.words.original_argument_word(self.reference.argument)?;
        let original = tcl_syntax::word_rules::original_static_word_unicode_value(word)?;
        Some(original.as_slice() == value.as_bytes())
    }

    /// Match fixed ASCII descriptor metadata under the original package purpose.
    #[must_use]
    pub fn matches_ascii(&self, metadata: &str) -> bool {
        // naming.diagnostic.original-package-source-advice
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-package-source-advice.md
        self.native_key.as_ref().map_or_else(
            || metadata.is_ascii() && self.literal == metadata.as_bytes(),
            |key| key.matches_ascii(metadata),
        )
    }
}
/// Actual selected package reference or explicit source-coverage uncertainty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OriginalSourcePackageReferenceSelection {
    /// Genuine original name under the selected require/provide operation.
    Reference(OriginalSourcePackageReference),
    /// Complete selected operation contributes no package reference.
    NotReference,
    /// Missing schema, dynamic name or unsupported original operand.
    Unavailable,
}

impl OriginalRegistryWords {
    /// Retain the distinction between missing source representation and an
    /// actual selected non-package operation. No package-table absence follows.
    #[must_use]
    pub fn package_reference_selection(
        &self,
        context: &tcl_registry::model::ContextRegistry,
    ) -> OriginalSourcePackageReferenceSelection {
        use tcl_registry::source_navigation::SourcePackageReferenceSelection as Selection;
        let Some(selection) = self.with_source_schema(context, |schema| {
            schema.authored_source_package_reference_selection()
        }) else {
            return OriginalSourcePackageReferenceSelection::Unavailable;
        };
        match selection {
            Selection::NotReference => OriginalSourcePackageReferenceSelection::NotReference,
            Selection::Unavailable => OriginalSourcePackageReferenceSelection::Unavailable,
            Selection::Reference(_) => self.package_reference(context).map_or(
                OriginalSourcePackageReferenceSelection::Unavailable,
                OriginalSourcePackageReferenceSelection::Reference,
            ),
        }
    }

    /// Project a package source reference using the same complete actual
    /// availability context, selected grammar and original effective operand.
    #[must_use]
    pub fn package_reference(
        &self,
        context: &tcl_registry::model::ContextRegistry,
    ) -> Option<OriginalSourcePackageReference> {
        // naming.diagnostic.original-package-source-advice
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-package-source-advice.md
        let reference = self.with_source_schema(context, selected_source_package_reference)??;
        let literal = self
            .arguments
            .get(reference.argument)?
            .literal_bytes()?
            .to_vec();
        let native_key = self
            .original_argument_value_input(reference.argument)
            .map(|input| {
                tcl_registry::native_package::NativePackageNameKey::from_native_units(
                    input.bytes(),
                    input.policy(),
                )
            });
        Some(OriginalSourcePackageReference {
            words: std::sync::Arc::new(self.clone()),
            reference,
            literal,
            native_key,
        })
    }
}

/// Readonly case syntax selected at an unchanged original source point.
/// Effective captured values keep their origins; only a genuine whole word
/// supplies pattern geometry. Possible selection and no-match paths remain
/// explicit, independently of Native handlers and executable IR.
pub(crate) struct OriginalSourceCase {
    layout: tcl_registry::spec::CaseInvocation,
    bodies: tcl_registry::case_bodies::CaseBodyOperands,
    patterns: Vec<String>,
    subject: crate::ir::WordExpr,
    folded_subject: Option<crate::ir::WordExpr>,
    subject_literal: Option<String>,
    scripts: Vec<super::OriginalSourceScriptBody>,
    config: tcl_lexer::LexerConfig,
}

impl OriginalSourceCase {
    pub(crate) fn body_operands(&self) -> &tcl_registry::case_bodies::CaseBodyOperands {
        &self.bodies
    }

    pub(crate) fn patterns(&self) -> &[String] {
        &self.patterns
    }

    pub(crate) const fn mode(&self) -> tcl_registry::spec::CaseMatchMode {
        self.layout.mode
    }

    pub(crate) fn references_parameter(&self, parameter: &str) -> bool {
        original_scalar_reference_is(&self.subject, parameter, self.config)
            || self.folded_subject.as_ref().is_some_and(|word| {
                original_scalar_reference_is(word, parameter, self.config.nested())
            })
    }

    pub(crate) const fn folds_case(&self) -> bool {
        self.layout.nocase || self.folded_subject.is_some()
    }

    /// Literal data or complete scalar syntax from the original subject word.
    pub(crate) fn subject_data(&self) -> Option<&str> {
        self.subject_literal.as_deref()
    }

    pub(crate) fn matches_subject_presentation(&self, subject: &str, braced: bool) -> bool {
        self.subject.legacy_text() == subject
            && matches!(&self.subject, crate::ir::WordExpr::BracedLiteral { .. }) == braced
    }

    pub(crate) fn scalar_subject_name(&self) -> Option<&str> {
        let (reference, source) = self.subject.sole_variable_substitution()?;
        (source.provenance == crate::ir::Provenance::Source).then_some(())?;
        tcl_syntax::naming::whole_scalar_variable_reference_name(reference, self.config)
    }

    /// Match each current IR body to its genuine case-source script extent.
    /// Source correspondence grants no Native body entry or executable erasure.
    pub(crate) fn matches_statement_bodies(
        &self,
        arms: &[crate::ir::SwitchArm],
        default: Option<&crate::ir::Script>,
    ) -> bool {
        let actual = arms
            .iter()
            .filter_map(|arm| arm.body.as_ref())
            .chain(default)
            .collect::<Vec<_>>();
        actual.len() == self.scripts.len()
            && self.scripts.len() == self.bodies.bodies.len()
            && actual.iter().zip(&self.scripts).all(|(script, body)| {
                let Some(executed) = script.executed_source.as_deref() else {
                    return false;
                };
                let crate::command_binding::ExecutedScriptMapping::Contiguous { base } =
                    executed.mapping
                else {
                    return false;
                };
                let span = body.content_span();
                executed.origin.source_image() == body.original_container().image()
                    && base == span.start()
                    && usize::try_from(span.end() - span.start()).ok()
                        == Some(executed.text.bytes().len())
                    && executed.origin.source_image().bytes().get(span.as_range())
                        == Some(executed.text.bytes())
            })
    }
}

fn original_scalar_reference_is(
    word: &crate::ir::WordExpr,
    parameter: &str,
    config: tcl_lexer::LexerConfig,
) -> bool {
    let Some((spelling, source)) = word.sole_variable_substitution() else {
        return false;
    };
    if source.provenance != crate::ir::Provenance::Source {
        return false;
    }
    let Ok(Some(reference)) = tcl_lexer::word_parts::whole_var_ref(spelling.as_bytes(), config)
    else {
        return false;
    };
    reference.index.is_none()
        && tcl_syntax::naming::split_element_ref_bytes(reference.name).is_none()
        && reference.name == parameter.as_bytes()
}

/// Retain the separately selected source-role projection without strengthening
/// `InvocationFacts`. The original vector, full input and selected descriptor
/// must all agree before this facade can describe possible case bodies.
pub(crate) fn original_logical_source_case_for_tokens(
    tokens: &crate::ir::CommandTokens,
    registry: &CommandRegistry,
    metadata: super::InvocationMetadataContext<'_>,
) -> Option<OriginalSourceCase> {
    // naming.compiler.original-unknown-handler-source-context
    // docs/design/analysis/name-resolution-proofs/compiler-original-unknown-handler-source-context.md
    let binding = tokens.source_binding.as_ref()?;
    let metadata = binding.original_invocation_metadata_for_input(
        tokens,
        metadata.source_analysis_input()?,
        registry,
    )?;
    let selected = super::original_logical_operation_invocation_with_metadata_context(
        registry, metadata, tokens,
    )?;
    original_source_case_projection(tokens, registry, metadata, &selected)
}

/// An already typed Native Switch may retain its original readonly case
/// syntax only through its actual normal handler and closed source lookup.
/// A generic Native call cannot acquire that compiler admission here.
pub(crate) fn original_native_source_case_for_statement(
    script: &crate::ir::Script,
    statement: &crate::ir::Statement,
    registry: &CommandRegistry,
    metadata: super::InvocationMetadataContext<'_>,
) -> Option<OriginalSourceCase> {
    matches!(statement, crate::ir::Statement::Switch { .. }).then_some(())?;
    let tokens = script.retained_source_tokens_for_statement(statement)?;
    let binding = tokens.source_binding.as_ref()?;
    let metadata = binding.original_invocation_metadata_for_input(
        tokens,
        metadata.source_analysis_input()?,
        registry,
    )?;
    let selected = original_native_handler_for_case_source(tokens, registry, metadata)?;
    let case = original_source_case_projection(tokens, registry, metadata, &selected)?;
    (!case.body_operands().selection_unknown).then_some(case)
}

fn original_native_handler_for_case_source(
    tokens: &crate::ir::CommandTokens,
    registry: &CommandRegistry,
    metadata: super::InvocationMetadataContext<'_>,
) -> Option<super::ResolvedStatementInvocation> {
    let binding = tokens.source_binding.as_ref()?;
    if binding.original_invocation_source_purpose(tokens, metadata, registry)?
        == crate::command_binding::OriginalInvocationSourcePurpose::Logical
    {
        return None;
    }
    binding
        .original_source_word_dialect_for_tokens(tokens)?
        .native_name_protocol()?;
    if binding.execution_is_unknown() || binding.execution_may_be_absent() {
        return None;
    }
    let selected =
        super::resolved_handler_invocation_with_metadata_context(registry, Some(metadata), tokens)?;
    if !selected.facts.arg_roles_complete
        || selected.facts.arity_accepts_frozen_arguments() != Some(true)
        || matches!(
            selected.facts.subcommand,
            tcl_registry::OwnedSubcommandResolution::Unknown { .. }
                | tcl_registry::OwnedSubcommandResolution::Ambiguous { .. }
                | tcl_registry::OwnedSubcommandResolution::Indeterminate { .. }
        )
    {
        return None;
    }
    Some(selected)
}

fn original_source_case_projection(
    tokens: &crate::ir::CommandTokens,
    registry: &CommandRegistry,
    metadata: super::InvocationMetadataContext<'_>,
    selected: &super::ResolvedStatementInvocation,
) -> Option<OriginalSourceCase> {
    let input = metadata.source_analysis_input()?;
    let binding = tokens.source_binding.as_ref()?;
    let logical_source = binding.original_source_advice_purpose(tokens, metadata, registry)?
        == crate::command_binding::OriginalInvocationSourcePurpose::Logical;
    let config = binding.original_lexer_config_for_tokens(tokens)?;
    let realm = if logical_source {
        binding.original_source_realm_for_input(tokens, input, registry)?
    } else {
        binding.invocation_realm()?
    };
    selected.with_metadata_schema(registry, metadata, realm, |schema| {
        schema.semantics.options.case_list.map(|_| ())
    })?;
    let words = original_case_registry_words(tokens, registry, metadata, selected)?;
    let (case, layout, bodies) =
        words.with_source_schema(input.borrowed_context_registry(), |schema| {
            selected.with_metadata_schema(registry, metadata, realm, |original| {
                let source_descriptors = schema.authored_source_descriptors();
                let selected_descriptors = original.authored_source_descriptors();
                let same_subcommand = match (
                    source_descriptors.subcommand,
                    selected_descriptors.subcommand,
                ) {
                    (Some(source), Some(selected)) => std::ptr::eq(source, selected),
                    (None, None) => true,
                    _ => false,
                };
                // Case grammar belongs directly to the immutable selected
                // command; form selection cannot replace this descriptor.
                // Copied transition values supply no further owner identity.
                if !std::ptr::eq(source_descriptors.command, selected_descriptors.command)
                    || !same_subcommand
                {
                    return None;
                }
                let (case, layout) = schema.authored_source_case_invocation()?;
                let bodies = if logical_source {
                    schema
                        .authored_logical_source_role_projection()?
                        .case_body_operands()?
                        .clone()
                } else {
                    let options = schema.semantics.options.available().collect::<Vec<_>>();
                    case.possible_body_operands(schema.words.arguments(), &options)?
                };
                Some((case, layout, bodies))
            })
        })??;
    let argument = layout.subject_index?;
    let written = selected.effective.written_argument(argument)?;
    let subject = tokens.words().get(written + 1)?.clone();
    let patterns = original_case_patterns(&words, case, &bodies)?;
    let folded_subject = original_folded_case_subject(tokens, &subject, registry, metadata);
    let subject_literal = super::original_literal_argument_data(tokens, written);
    let scripts = words.source_script_bodies_for(
        input.borrowed_context_registry(),
        super::OriginalSourceScriptPurpose::PotentialEvaluation,
    );
    Some(OriginalSourceCase {
        layout,
        bodies,
        patterns,
        subject,
        folded_subject,
        subject_literal,
        scripts,
        config,
    })
}

fn original_case_registry_words(
    tokens: &crate::ir::CommandTokens,
    registry: &CommandRegistry,
    metadata: super::InvocationMetadataContext<'_>,
    selected: &super::ResolvedStatementInvocation,
) -> Option<OriginalRegistryWords> {
    use super::{EffectiveInvocationWord, InvocationWordOrigin};
    let binding = tokens.source_binding.as_ref()?;
    let config = binding.original_lexer_config_for_tokens(tokens)?;
    let (segment, recorded) = binding.original_recorded_command()?;
    (recorded.words() == tokens.words()).then_some(())?;
    let site = binding.invocation_site()?;
    let image = site.source.source_image().clone();
    let native =
        super::original_native_compiler_words(&image, tokens.words(), site.offset, config)?;
    let frozen = super::frozen_argument_words(tokens, &selected.effective);
    let arguments = selected
        .effective
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
                tcl_syntax::word_rules::original_static_word_unicode_value(word)
                    .map_or(EffectiveInvocationWord::Dynamic, |value| {
                        EffectiveInvocationWord::ByteLiteral(std::sync::Arc::from(value))
                    })
            })
        })
        .collect::<Option<Vec<_>>>()?;
    let operands = effective_operand_sources(
        &segment,
        tokens,
        &selected.effective,
        &arguments,
        Some(&native),
    )?;
    Some(OriginalRegistryWords {
        command: selected.facts.canonical_command.clone(),
        dialect: selected.dialect,
        arguments,
        operands,
        origins: selected.effective.origins.clone(),
        head: Some(OriginalOperandSource {
            span: native.first()?.span(),
            input: None,
            word: native.first().cloned(),
        }),
        roles: None,
        source: OriginalRegistrySource::Selected,
        captured_values: None,
        operands_preserve_source_lookup: false,
        image,
        config,
        registry: registry.snapshot().semantic_key(),
        context: Some(metadata.context().clone()),
        definition_parent: None,
    })
}

fn original_case_patterns(
    words: &OriginalRegistryWords,
    case: tcl_registry::CaseListSpec,
    bodies: &tcl_registry::case_bodies::CaseBodyOperands,
) -> Option<Vec<String>> {
    let mut patterns = Vec::new();
    for (index, pattern) in bodies.patterns.iter().enumerate() {
        let original = words.original_argument_word(pattern.argument)?;
        let value = words.arguments.get(pattern.argument)?.literal_bytes()?;
        if tcl_syntax::word_rules::original_static_word_unicode_value(original)?.as_slice() != value
        {
            return None;
        }
        if let Some(element) = pattern.list_element {
            let fields = tcl_syntax::word_rules::original_static_word_list_elements(original)?;
            let field = fields.get(element)?;
            if case.is_keyword_pattern(field.value(), index, bodies.patterns.len()) {
                continue;
            }
            if case.pattern_is_list(field.value()) {
                patterns.extend(
                    field
                        .elements()?
                        .iter()
                        .map(|field| field.value().to_owned()),
                );
            } else {
                patterns.push(field.value().to_owned());
            }
        } else {
            let value = std::str::from_utf8(value).ok()?;
            if !case.is_keyword_pattern(value, index, bodies.patterns.len()) {
                if case.pattern_is_list(value) {
                    patterns.extend(
                        tcl_syntax::word_rules::original_static_word_list_elements(original)?
                            .iter()
                            .map(|field| field.value().to_owned()),
                    );
                } else {
                    patterns.push(value.to_owned());
                }
            }
        }
    }
    Some(patterns)
}

#[cfg(test)]
fn trace_original_folded_case_guard(stage: &str, offset: Option<u32>, passed: bool) {
    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_FACTORY_GUARDS").is_some() {
        eprintln!("ORIGINAL_FACTORY_GUARD stage={stage} offset={offset:?} passed={passed}");
    }
}

fn original_folded_case_source_image(
    parent: &crate::ir::CommandTokens,
) -> Option<(tcl_lexer::LexerConfig, &tcl_lexer::SourceImage)> {
    let config = parent
        .source_binding
        .as_ref()?
        .original_lexer_config_for_tokens(parent)?;
    let image = parent
        .source_binding
        .as_ref()?
        .invocation_site()?
        .source
        .source_image();
    Some((config, image))
}

fn original_folded_case_subject(
    parent: &crate::ir::CommandTokens,
    subject: &crate::ir::WordExpr,
    registry: &CommandRegistry,
    metadata: super::InvocationMetadataContext<'_>,
) -> Option<crate::ir::WordExpr> {
    #[cfg(test)]
    let trace_enabled =
        tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_FACTORY_GUARDS").is_some();
    #[cfg(test)]
    let trace = trace_original_folded_case_guard;
    let substitution = subject.sole_command_substitution();
    #[cfg(test)]
    if trace_enabled {
        trace("case_fold.substitution", None, substitution.is_some());
    }
    substitution?;
    #[cfg(test)]
    let (_, source) = substitution?;
    let input = metadata.source_analysis_input()?;
    let (config, image) = original_folded_case_source_image(parent)?;
    let extent = subject.original_command_substitution_span(image.bytes())?;
    let calls = crate::word_subst::checked_original_lifted_calls_with_metadata_context(
        parent, config, registry, metadata,
    );
    #[cfg(test)]
    if trace_enabled {
        trace(
            "case_fold.complete_children",
            Some(source.span.start()),
            calls.is_some(),
        );
    }
    let calls = calls?;
    let call = calls.iter().find(|call| call.span == extent);
    #[cfg(test)]
    if trace_enabled {
        trace(
            "case_fold.exact_child",
            Some(source.span.start()),
            call.is_some(),
        );
    }
    let call = call?;
    let child = call.tokens.as_ref()?;
    let binding = child.source_binding.as_ref()?;
    let child_metadata = binding.original_invocation_metadata_for_input(child, input, registry);
    #[cfg(test)]
    if trace_enabled {
        trace(
            "case_fold.child_metadata",
            Some(source.span.start()),
            child_metadata.is_some(),
        );
    }
    let child_metadata = child_metadata?;
    let logical_source =
        binding.original_invocation_source_purpose(child, child_metadata, registry)?
            == crate::command_binding::OriginalInvocationSourcePurpose::Logical;
    let selected = if logical_source {
        super::original_logical_operation_invocation_with_metadata_context(
            registry,
            child_metadata,
            child,
        )
    } else {
        original_native_handler_for_case_source(child, registry, child_metadata)
    };
    #[cfg(test)]
    if trace_enabled {
        trace(
            "case_fold.selected_child",
            Some(source.span.start()),
            selected.is_some(),
        );
    }
    let selected = selected?;
    let realm = if logical_source {
        binding.original_source_realm_for_input(child, input, registry)?
    } else {
        binding.invocation_realm()?
    };
    let value = selected.with_metadata_schema(registry, child_metadata, realm, |schema| {
        (schema.semantics.byte_array_effect == tcl_registry::ByteArrayEffect::CaseFolds
            && schema.words.arguments().exact_argv_len()
                == schema.semantics.argument_offset.checked_add(1))
        .then_some(schema.semantics.argument_offset)
    });
    #[cfg(test)]
    if trace_enabled {
        trace(
            "case_fold.selected_descriptor",
            Some(source.span.start()),
            value.is_some(),
        );
    }
    let value = value?;
    let written = selected.effective.written_argument(value)?;
    Some(child.words().get(written + 1)?.clone())
}

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
    let native = super::original_native_compiler_words(
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
    let native = super::original_native_compiler_words(image, tokens.words(), site.offset, config)?;
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
        if let super::InvocationWordOrigin::Written(written) = origin
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
    let input =
        super::InvocationMetadataContext::for_module(registry, module)?.source_analysis_input()?;
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
    super::InvocationMetadataContext::for_module(registry, module)?;
    let realm = crate::realm::document_realm_bindings_for_module(module, registry)?;
    let binding = tokens.source_binding.as_ref()?;
    if binding.has_original_source_layout_owner() {
        return original_registry_words_for_supplied_tokens(&realm, input, tokens);
    }
    let retained = binding.original_logical_source_words(tokens, input, registry)?;
    let offset = tokens.words().first()?.source().span.start();
    let original = super::original_native_compiler_words(
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
            let actual = super::original_native_compiler_words(
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

pub(super) fn logical_source_registry_words(
    source: &str,
    realm: &crate::realm::CommandBindingRealm,
    input: &crate::analyser::ResolvedAnalysisInput,
    segment: &crate::segmenter::SegmentedCommand,
) -> Option<OriginalRegistryWords> {
    if realm
        .source_bindings_ref()
        .original_logical_source_name_advice_input()
        != Some(input)
        || !realm.matches_resolved_analysis_input(input)
    {
        return None;
    }
    // Retain the same complete bytes/channel/configuration check without
    // allocating, hashing and indexing another whole Document image per query.
    let image = retained_document_image(source, realm, input.lexer_config())?;
    let context = input.context_registry();
    let words =
        source_transition_registry_words(source, input.lexer_config(), realm, &context, segment)
            .or_else(|| {
                let mut tokens = crate::ir::CommandTokens::from_segmented(
                    &tcl_lexer::SourceMap::from_image(&image),
                    input.lexer_config(),
                    segment,
                );
                realm.stamp_original_tokens(&mut tokens);
                let advice = realm.original_source_prefix_catalogue_advice(input, &tokens)?;
                source_transition_words_with_segment(
                    source,
                    input.lexer_config(),
                    &context,
                    segment,
                    advice,
                )
            })?;
    let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
        return None;
    };
    (advice.original_head().logical_input().is_some()
        && advice.logical_source_input() == Some(input))
    .then_some(words)
}

fn logical_source_registry_words_in_original_image(
    image: &tcl_lexer::SourceImage,
    realm: &crate::realm::CommandBindingRealm,
    input: &crate::analyser::ResolvedAnalysisInput,
    segment: &crate::segmenter::SegmentedCommand,
) -> Option<OriginalRegistryWords> {
    if realm
        .source_bindings_ref()
        .original_logical_source_name_advice_input()
        != Some(input)
        || !realm.matches_resolved_analysis_input(input)
        || !realm.matches_original_source_image(image, input.lexer_config())
    {
        return None;
    }
    let mut tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::from_image(image),
        input.lexer_config(),
        segment,
    );
    realm.stamp_original_tokens(&mut tokens);
    let context = input.context_registry();
    let advice = realm
        .original_source_transition_advice(&context, &tokens)
        .or_else(|| realm.original_source_prefix_catalogue_advice(input, &tokens))?;
    let words = source_transition_words_with_segment_in_image(
        image,
        input.lexer_config(),
        &context,
        segment,
        advice,
    )?;
    let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
        return None;
    };
    (advice.original_head().logical_input().is_some()
        && advice.logical_source_input() == Some(input))
    .then_some(words)
}

fn native_source_registry_words(
    source: &str,
    config: tcl_lexer::LexerConfig,
    realm: &crate::realm::CommandBindingRealm,
    context: &tcl_registry::model::ContextRegistry,
    segment: &crate::segmenter::SegmentedCommand,
    retained_metadata: Option<crate::registry_invocation::OriginalConditionalRegistryMetadata>,
) -> Option<OriginalRegistryWords> {
    let image = retained_document_image(source, realm, config)?;
    realm
        .matches_original_source_image(&image, config)
        .then_some(())?;
    if let Some(input) = realm
        .source_bindings_ref()
        .original_logical_source_name_advice_input()
    {
        // This domain is selected by the validated full Logical input, never
        // by a missing Native operand or a Registry/profile display label.
        if config != input.lexer_config()
            || !std::ptr::eq(std::sync::Arc::as_ptr(&input.context_registry()), context)
        {
            return None;
        }
        return logical_source_registry_words(source, realm, input, segment);
    }
    let Some(metadata) = trace_original_source_role_option(
        "native.metadata",
        segment.argv.first().map(|word| word.span.start()),
        native_source_metadata(source, config, realm, context, segment, retained_metadata),
    ) else {
        return trace_original_source_role_words(
            "native.transition_fallback",
            segment.argv.first().map(|word| word.span.start()),
            source_transition_registry_words(source, config, realm, context, segment),
        );
    };
    conditional_source_registry_words(source, config, realm, context, segment, metadata)
}

fn native_source_metadata(
    source: &str,
    config: tcl_lexer::LexerConfig,
    realm: &crate::realm::CommandBindingRealm,
    context: &tcl_registry::model::ContextRegistry,
    segment: &crate::segmenter::SegmentedCommand,
    retained_metadata: Option<crate::registry_invocation::OriginalConditionalRegistryMetadata>,
) -> Option<crate::registry_invocation::OriginalConditionalRegistryMetadata> {
    let registry = context.commands();
    let image = retained_document_image(source, realm, config)?;
    let mut tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::from_image(&image),
        config,
        segment,
    );
    realm.stamp_original_tokens(&mut tokens);
    if realm.original_source_registry_barrier(context, &tokens) {
        return None;
    }

    let metadata = if let Some(metadata) = trace_original_source_role_option(
        "metadata.retained",
        segment.argv.first().map(|word| word.span.start()),
        retained_metadata,
    ) {
        metadata
    } else {
        let candidate = tokens
            .source_binding
            .as_ref()
            .and_then(|binding| {
                trace_original_source_role_option(
                    "metadata.point_candidate",
                    segment.argv.first().map(|word| word.span.start()),
                    binding.original_catalogue_source_candidate_from_tokens(&tokens, registry),
                )
            })
            .or_else(|| {
                trace_original_source_role_option(
                    "metadata.future_candidate",
                    segment.argv.first().map(|word| word.span.start()),
                    realm
                        .source_bindings_ref()
                        .original_future_source_catalogue_candidate(
                            &image, config, &tokens, registry,
                        ),
                )
            });
        candidate.and_then(|candidate| {
            super::conditional_metadata::original_catalogue_registry_metadata(context, candidate)
        })?
    };
    Some(metadata)
}

fn conditional_source_values(
    metadata: &crate::registry_invocation::OriginalConditionalRegistryMetadata,
) -> Option<(
    Vec<tcl_lexer::NativeWord>,
    Vec<crate::registry_invocation::EffectiveInvocationWord>,
)> {
    use crate::registry_invocation::EffectiveInvocationWord;
    let native = metadata.original_words().to_vec();
    let captured = tcl_registry::native_compiler_words::NativeCompilerWords::capture(
        &native,
        metadata
            .original_head()
            .name_input()
            .policy()
            .string_protocol(),
    )
    .ok()?;
    let values = native
        .iter()
        .enumerate()
        .skip(1)
        .map(|(ordinal, word)| {
            if word.group().expand {
                EffectiveInvocationWord::Expanded
            } else {
                captured
                    .literal(ordinal)
                    .map_or(EffectiveInvocationWord::Dynamic, |bytes| {
                        EffectiveInvocationWord::ByteLiteral(std::sync::Arc::from(bytes))
                    })
            }
        })
        .collect::<Vec<_>>();
    Some((native, values))
}

fn conditional_source_operands(
    native: &[tcl_lexer::NativeWord],
    segment: &crate::segmenter::SegmentedCommand,
    origin: &OriginalRegistrySource,
) -> Vec<Option<OriginalOperandSource>> {
    native
        .iter()
        .enumerate()
        .skip(1)
        .map(|(ordinal, word)| {
            Some(OriginalOperandSource {
                span: segment.argv.get(ordinal)?.span,
                input: match origin {
                    OriginalRegistrySource::Conditional(metadata) => {
                        let head = metadata.original_head().name_input();
                        crate::signature_scan::scope::SignatureSourceNameKey::from_original_native_word(
                            word, head.word_value_rules(), head.policy())
                            .map(crate::signature_scan::scope::SignatureSourceNameInput::OriginalWord)
                    },
                    OriginalRegistrySource::Selected | OriginalRegistrySource::ConditionalNativeSource(_) | OriginalRegistrySource::Scoped(_) | OriginalRegistrySource::DefinitionMember(_) | OriginalRegistrySource::Vendor(_) | OriginalRegistrySource::SourceTransitions(_) | OriginalRegistrySource::ProducedPrefix(_) => None,
                },
                word: Some(word.clone()),
            })
        })
        .collect()
}

fn conditional_source_registry_words(
    source: &str,
    config: tcl_lexer::LexerConfig,
    realm: &crate::realm::CommandBindingRealm,
    context: &tcl_registry::model::ContextRegistry,
    segment: &crate::segmenter::SegmentedCommand,
    metadata: crate::registry_invocation::OriginalConditionalRegistryMetadata,
) -> Option<OriginalRegistryWords> {
    use crate::registry_invocation::InvocationWordOrigin;
    let registry = context.commands();
    let image = retained_document_image(source, realm, config)?;
    let offset = segment.argv.first()?.span.start();
    let (command, native, values, roles, origin) = {
        let (native, values) = conditional_source_values(&metadata)?;
        let roles = conditional_roles(
            metadata.roles(),
            metadata.argument_offset(),
            metadata.roles_complete(),
        );
        (
            metadata.command().to_owned(),
            native,
            values,
            roles,
            OriginalRegistrySource::Conditional(std::sync::Arc::new(metadata)),
        )
    };
    let mut tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::from_image(&image),
        config,
        segment,
    );
    realm.stamp_original_tokens(&mut tokens);
    let dialect = match &origin {
        OriginalRegistrySource::Scoped(_)
        | OriginalRegistrySource::DefinitionMember(_)
        | OriginalRegistrySource::Vendor(_)
        | OriginalRegistrySource::SourceTransitions(_)
        | OriginalRegistrySource::ProducedPrefix(_) => None,
        OriginalRegistrySource::Conditional(metadata) => metadata.original_source_dialect(),
        OriginalRegistrySource::Selected | OriginalRegistrySource::ConditionalNativeSource(_) => {
            tokens
                .source_binding
                .as_ref()
                .and_then(|binding| binding.original_source_word_dialect_for_tokens(&tokens))
        }
    };
    let original = crate::registry_invocation::original_native_compiler_words(
        &image,
        tokens.words(),
        offset,
        config,
    )?;
    if original != native {
        return None;
    }
    let operands = conditional_source_operands(&native, segment, &origin);
    let head = native.first().map(|word| OriginalOperandSource {
        span: word.span(),
        input: match &origin {
            OriginalRegistrySource::Conditional(metadata) => Some(
                crate::signature_scan::scope::SignatureSourceNameInput::OriginalWord(
                    metadata.original_head().name_input().clone(),
                ),
            ),
            OriginalRegistrySource::Selected
            | OriginalRegistrySource::ConditionalNativeSource(_)
            | OriginalRegistrySource::Scoped(_)
            | OriginalRegistrySource::DefinitionMember(_)
            | OriginalRegistrySource::Vendor(_)
            | OriginalRegistrySource::SourceTransitions(_)
            | OriginalRegistrySource::ProducedPrefix(_) => None,
        },
        word: Some(word.clone()),
    });
    Some(OriginalRegistryWords {
        command,
        dialect,
        arguments: values,
        operands,
        origins: (0..native.len())
            .map(InvocationWordOrigin::Written)
            .collect(),
        roles,
        head,
        source: origin,
        captured_values: None,
        operands_preserve_source_lookup: false,
        image,
        config,
        registry: registry.snapshot().semantic_key(),
        context: Some(context.context().clone()),
        definition_parent: None,
    })
}

/// Conditional root-script aliases and moves share one immutable source tape.
/// Known targets and definite absence do not fall through; an unbounded
/// snapshot retains conditional source applicability without a dispatch grant.
fn source_transition_registry_words(
    source: &str,
    config: tcl_lexer::LexerConfig,
    realm: &crate::realm::CommandBindingRealm,
    context: &tcl_registry::model::ContextRegistry,
    segment: &crate::segmenter::SegmentedCommand,
) -> Option<OriginalRegistryWords> {
    let image = retained_document_image(source, realm, config)?;
    let mut tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::from_image(&image),
        config,
        segment,
    );
    realm.stamp_original_tokens(&mut tokens);
    let advice = realm.original_source_transition_advice(context, &tokens);
    #[cfg(test)]
    trace_original_child_guard(
        "transition.advice",
        segment.argv.first().map(|word| word.span.start()),
        advice.is_some(),
    );
    let result = source_transition_words_with_segment(source, config, context, segment, advice?);
    #[cfg(test)]
    trace_original_child_guard(
        "transition.words",
        segment.argv.first().map(|word| word.span.start()),
        result.is_some(),
    );
    result
}

/// Project a sealed original source receipt through the same role/body owner.
/// The original vector is recaptured in its whole-image address space; a
/// detached script or reconstructed argv cannot supply this correspondence.
pub(crate) fn conditional_lambda_formatting_words(
    source: &str,
    config: tcl_lexer::LexerConfig,
    context: &tcl_registry::model::ContextRegistry,
    segment: &crate::segmenter::SegmentedCommand,
    advice: &crate::command_binding::OriginalSourceCommandTransitionAdvice,
) -> Option<OriginalRegistryWords> {
    let mut words =
        source_transition_words_with_segment(source, config, context, segment, advice.clone())?;
    words.operands_preserve_source_lookup = false;
    Some(words)
}

pub(crate) fn source_transition_words_from_advice(
    source: &str,
    config: tcl_lexer::LexerConfig,
    context: &tcl_registry::model::ContextRegistry,
    advice: crate::command_binding::OriginalSourceCommandTransitionAdvice,
) -> Option<OriginalRegistryWords> {
    let original = advice.original_words();
    let span = tcl_lexer::Span::new(
        original.first()?.span().start(),
        original.last()?.span().end(),
    );
    let mut segments = crate::segmenter::segment_commands_with_offset_and_config(
        source.get(span.as_range())?,
        span.start(),
        config.nested(),
    );
    if segments.len() != 1 {
        return None;
    }
    source_transition_words_with_segment(source, config, context, &segments.pop()?, advice)
}

fn source_transition_words_with_segment(
    source: &str,
    config: tcl_lexer::LexerConfig,
    context: &tcl_registry::model::ContextRegistry,
    segment: &crate::segmenter::SegmentedCommand,
    advice: crate::command_binding::OriginalSourceCommandTransitionAdvice,
) -> Option<OriginalRegistryWords> {
    let image = matching_document_image(source, advice.original_words().first()?.image())?;
    source_transition_words_with_segment_in_image(&image, config, context, segment, advice)
}

// The caller must retain the actual source image; no Document conversion is
// permitted for entered materialised script values.
fn source_transition_words_with_segment_in_image(
    image: &tcl_lexer::SourceImage,
    config: tcl_lexer::LexerConfig,
    context: &tcl_registry::model::ContextRegistry,
    segment: &crate::segmenter::SegmentedCommand,
    advice: crate::command_binding::OriginalSourceCommandTransitionAdvice,
) -> Option<OriginalRegistryWords> {
    use crate::registry_invocation::{EffectiveInvocationWord, InvocationWordOrigin};
    let image = image.clone();
    let tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::from_image(&image),
        config,
        segment,
    );
    if crate::registry_invocation::original_native_compiler_words(
        &image,
        tokens.words(),
        segment.argv.first()?.span.start(),
        config,
    )?
    .as_slice()
        != advice.original_words()
    {
        return None;
    }
    if !advice.matches_context(context) || !advice.matches_source(&image, config) {
        return None;
    }
    let values = advice
        .arguments()
        .iter()
        .map(|argument| {
            if argument.original.group().expand {
                EffectiveInvocationWord::Expanded
            } else {
                argument
                    .value
                    .as_ref()
                    .map_or(EffectiveInvocationWord::Dynamic, |bytes| {
                        EffectiveInvocationWord::ByteLiteral(std::sync::Arc::clone(bytes))
                    })
            }
        })
        .collect();
    let operands = advice
        .arguments()
        .iter()
        .map(|argument| {
            let InvocationWordOrigin::Written(ordinal) = argument.origin else {
                return None;
            };
            let token = segment.argv.get(ordinal)?;
            // The realm authenticated the exact complete native vector.
            // Segmented tokens and grouped original words retain different
            // genuine delimiter extents; both are preserved independently.
            Some(OriginalOperandSource {
                span: token.span,
                input: argument
                    .input
                    .as_ref()
                    .and_then(|input| input.native_input())
                    .cloned(),
                word: Some(argument.original.clone()),
            })
        })
        .collect();
    let mut origins = vec![InvocationWordOrigin::Written(0)];
    origins.extend(advice.arguments().iter().map(|argument| argument.origin));
    let head = advice
        .original_words()
        .first()
        .map(|word| OriginalOperandSource {
            span: word.span(),
            input: advice.original_head().native_input().cloned(),
            word: Some(word.clone()),
        });
    Some(OriginalRegistryWords {
        command: advice.command().to_owned(),
        dialect: Some(advice.dialect()),
        arguments: values,
        operands,
        origins,
        roles: advice.roles().map(<[_]>::to_vec),
        head,
        source: OriginalRegistrySource::SourceTransitions(std::sync::Arc::new(advice)),
        captured_values: None,
        operands_preserve_source_lookup: false,
        image,
        config,
        registry: context.commands().snapshot().semantic_key(),
        context: Some(context.context().clone()),
        definition_parent: None,
    })
}

/// Authored target metadata of a selected original list builder at its actual
/// source point. Inert list data gains no deferred parent, future lookup,
/// evaluated result object, current receiver or dispatch authority.
#[must_use]
pub fn source_authored_command_prefix(
    source: &str,
    analysis: &AnalysisResult,
    producer: &crate::segmenter::SegmentedCommand,
) -> Option<crate::command_binding::OriginalSourceAuthoredCommandPrefix> {
    // naming.source.original-authored-command-prefix-data
    // docs/design/analysis/name-resolution-proofs/source-original-authored-command-prefix-data.md
    let input = analysis.resolved_input.as_ref()?;
    let config = input.lexer_config();
    let image = tcl_lexer::SourceImage::document(source);
    analysis
        .matches_original_source_image(&image, config)
        .then_some(())?;
    (analysis.body_lexer_config == Some(config)).then_some(())?;
    let realm = analysis.retained_command_realm()?;
    realm.matches_resolved_analysis_input(input).then_some(())?;
    let tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::from_image(&image),
        config,
        producer,
    );
    let original = super::original_native_compiler_words(
        &image,
        tokens.words(),
        producer.argv.first()?.span.start(),
        config,
    )?;
    realm.original_authored_command_prefix(input, &original)
}

/// Conditional future source argv built by a selected original list invocation
/// in a retained deferred operand. Every written target operand keeps its own
/// builder source word; captured target-prefix values acquire no source anchor.
/// This receipt cannot supply execution, native values or syntax compaction.
#[must_use]
pub fn source_produced_command_prefix_words(
    source: &str,
    analysis: &AnalysisResult,
    producer: &crate::segmenter::SegmentedCommand,
) -> Option<OriginalRegistryWords> {
    // naming.source.original-produced-command-prefix
    // docs/design/analysis/name-resolution-proofs/original-produced-command-prefix.md
    let input = analysis.resolved_input.as_ref()?;
    analysis
        .matches_original_source_image(
            &tcl_lexer::SourceImage::document(source),
            input.lexer_config(),
        )
        .then_some(())?;
    source_produced_command_prefix_words_in(
        source,
        input,
        analysis.retained_command_realm()?,
        producer,
    )
}

/// The same source producer while an analyser retains the complete input and
/// realm independently of its final result. No fresh interpretation, default
/// context or presentation head replaces those owners.
pub(crate) fn source_produced_command_prefix_words_in(
    source: &str,
    input: &crate::analyser::ResolvedAnalysisInput,
    realm: &crate::realm::CommandBindingRealm,
    producer: &crate::segmenter::SegmentedCommand,
) -> Option<OriginalRegistryWords> {
    let context = input.context_registry();
    let image = tcl_lexer::SourceImage::document(source);
    let config = input.lexer_config();
    (realm.matches_resolved_analysis_input(input)
        && realm.matches_original_source_image(&image, config))
    .then_some(())?;
    let tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::new(source),
        config,
        producer,
    );
    let original = crate::registry_invocation::original_native_compiler_words(
        &image,
        tokens.words(),
        producer.argv.first()?.span.start(),
        config,
    )?;
    let prefix = realm.original_produced_command_prefix(input, &original)?;
    // The genuine deferred parent owns this builder's conditional schema.
    // A standalone nested lookup would require an unrelated entered frame.
    let builder =
        source_transition_words_from_advice(source, config, &context, prefix.producer().clone())?;
    if !builder.with_source_schema(&context, |schema| {
        schema
            .semantics
            .traits
            .contains(tcl_registry::Traits::BUILDS_COMMAND_PREFIX)
            && schema.semantics.native_result
                == Some(
                    tcl_registry::native_result::NativeResultContract::ListArguments { from: 0 },
                )
    })? {
        return None;
    }
    if builder.command() != prefix.producer().command()
        || builder.origins()
            != (0..original.len())
                .map(crate::registry_invocation::InvocationWordOrigin::Written)
                .collect::<Vec<_>>()
    {
        return None;
    }
    produced_prefix_words(source, config, &context, producer, prefix)
}

fn produced_prefix_words(
    source: &str,
    config: tcl_lexer::LexerConfig,
    context: &tcl_registry::model::ContextRegistry,
    segment: &crate::segmenter::SegmentedCommand,
    prefix: crate::command_binding::OriginalSourceProducedCommandPrefix,
) -> Option<OriginalRegistryWords> {
    use crate::registry_invocation::{EffectiveInvocationWord, InvocationWordOrigin};
    let image = tcl_lexer::SourceImage::document(source);
    prefix
        .matches_source_context(&image, config, context)
        .then_some(())?;
    let mut origins = vec![InvocationWordOrigin::Written(1)];
    origins.extend(
        prefix
            .arguments()
            .iter()
            .map(|argument| match argument.origin {
                InvocationWordOrigin::Written(ordinal) => {
                    ordinal.checked_add(1).map(InvocationWordOrigin::Written)
                }
                other => Some(other),
            })
            .collect::<Option<Vec<_>>>()?,
    );
    let operands = prefix
        .arguments()
        .iter()
        .map(|argument| {
            let InvocationWordOrigin::Written(_) = argument.origin else {
                return None;
            };
            let word = &argument.original;
            let token = segment.argv.iter().find(|token| {
                word.tokens()
                    .first()
                    .is_some_and(|first| first.span == token.span)
            })?;
            Some(OriginalOperandSource {
                span: token.span,
                input: argument
                    .input
                    .as_ref()
                    .and_then(|input| input.native_input())
                    .cloned(),
                word: Some(word.clone()),
            })
        })
        .collect();
    let arguments = prefix
        .arguments()
        .iter()
        .map(|argument| {
            argument
                .value
                .as_ref()
                .map_or(EffectiveInvocationWord::Dynamic, |value| {
                    EffectiveInvocationWord::ByteLiteral(std::sync::Arc::clone(value))
                })
        })
        .collect();
    let head = prefix.original_head().original_word()?;
    Some(OriginalRegistryWords {
        command: prefix.command().to_owned(),
        dialect: Some(prefix.producer().dialect()),
        arguments,
        operands,
        origins,
        roles: prefix.roles().map(<[_]>::to_vec),
        head: Some(OriginalOperandSource {
            span: head.span(),
            input: prefix.original_head().native_input().cloned(),
            word: Some(head.clone()),
        }),
        source: OriginalRegistrySource::ProducedPrefix(std::sync::Arc::new(prefix)),
        captured_values: None,
        operands_preserve_source_lookup: false,
        image,
        config,
        registry: context.commands().snapshot().semantic_key(),
        context: Some(context.context().clone()),
        definition_parent: None,
    })
}

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

fn conditional_roles(
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

#[cfg(test)]
mod tests;

#[cfg(test)]
mod original_procedure_argument_tests;

#[cfg(test)]
mod source_lookup_boundary_tests;

#[cfg(test)]
mod source_word_dialect_tests;

#[cfg(test)]
mod retained_image_geometry_tests;

#[cfg(test)]
mod original_selected_command_purpose_tests;

#[cfg(test)]
mod member_source_catalogue_tests;

#[cfg(test)]
mod original_logical_operand_prefix_tests;

#[cfg(test)]
mod conditional_native_case_receipt_tests;

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

#[cfg(test)]
mod stock_source_proposal_metadata_tests;

/// Genuine selected authored Jim class call, conditional metadata only.
#[must_use]
pub fn source_authored_jim_call_at(
    source: &str,
    analysis: &AnalysisResult,
    offset: u32,
) -> Option<crate::command_binding::OriginalAuthoredJimClassCall> {
    let (input, realm, native) = trace_jim_source_option(
        "jim.call.registered_words",
        registered_original_words_at(source, analysis, offset),
    )?;
    trace_jim_source_option(
        "jim.call.realm_receipt",
        realm.original_authored_jim_call(input, &native),
    )
}

/// Own authored Jim member recipes; no generic class or entered body is issued.
#[must_use]
pub(crate) fn source_authored_jim_procedure_members_at(
    source: &str,
    analysis: &AnalysisResult,
    offset: u32,
) -> Option<Vec<crate::command_binding::OriginalAuthoredJimProcedureMember>> {
    let (input, realm, native) = registered_original_words_at(source, analysis, offset)?;
    realm.original_authored_jim_procedure_members(input, &native)
}

#[cfg(test)]
mod original_query_document_image_tests;
