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
use tcl_registry::CommandRegistry;

mod advice_queries;
mod expression_rewrites;
mod module_words;
mod package_references;
mod procedure_arguments;
mod selected_words;
mod source_cases;
mod source_registry_words;
mod transition_words;
mod vendor_words;

use advice_queries::registered_original_words_at;
pub use advice_queries::{
    original_logical_source_words_at, source_authored_command_prefix, source_authored_jim_call_at,
    source_authored_jim_class_at, source_callback_prefix_at, source_callback_procedure_target_at,
    source_class_declaration_at, source_class_instance_words_at, source_class_proc_call_at,
    source_class_publications, source_class_reference_at, source_command_publication_at,
    source_configured_class_at, source_configured_class_words_at, source_constructor_call_at,
    source_handle_class_advice_at, source_handle_construction_at,
    source_logical_absolute_command_candidate_at, source_logical_callback_candidates,
    source_logical_callback_candidates_at, source_logical_command_candidates,
    source_logical_command_candidates_at, source_logical_ensemble_transition_receipts,
    source_logical_name_references, source_logical_procedure_call_at,
    source_logical_procedure_call_for_cursor, source_logical_procedure_calls,
    source_native_procedure_body_commands, source_original_procedure_call_at,
    source_original_procedure_calls, source_procedure_publications,
    source_produced_command_prefix_words, source_registered_factory_at,
    source_registered_handle_binding_at, source_registered_instance_words_at,
};
pub(crate) use advice_queries::{
    source_authored_jim_procedure_members_at, source_produced_command_prefix_words_in,
    source_reached_procedure_call_at,
};
pub use expression_rewrites::{
    OriginalExpressionBooleanInversion, OriginalExpressionOperatorRewrite,
    OriginalExpressionOperatorRewriteKind, OriginalExpressionSyntaxGapCompaction,
    original_expression_boolean_comparison_inversion, original_expression_gap_compaction,
    original_expression_operator_rewrite, original_expression_syntax_gap_compaction,
};
pub use module_words::original_logical_registry_words_for_command;
pub(crate) use module_words::{
    original_declared_words_for_tokens, original_immediate_body_commands_for_module,
    original_logical_registry_words_for_module, original_registry_words_for_module,
    original_registry_words_for_supplied_tokens, original_registry_words_for_tokens,
    original_segment_for_tokens,
};
pub use package_references::{
    OriginalSourcePackageReferenceSelection, original_package_require_insert_offset,
};
pub use procedure_arguments::{OriginalProcedureArguments, original_procedure_arguments};
#[cfg(test)]
use selected_words::conditional_native_case_source_words;
pub(crate) use selected_words::selected_registry_words_in_original_image;
use selected_words::{
    effective_operand_sources, original_source_argument_words, selected_registry_words_from_tokens,
};
pub use selected_words::{
    selected_registry_words, selected_registry_words_in_realm, source_declared_command_words,
};
#[cfg(test)]
use source_cases::original_folded_case_subject;
pub(crate) use source_cases::{
    original_logical_source_case_for_tokens, original_native_source_case_for_statement,
};
use source_registry_words::source_definition_body_grammar;
pub use source_registry_words::{
    source_registry_words, source_registry_words_at, source_registry_words_in_definition_body,
    source_registry_words_in_definition_body_at,
};
pub(super) use source_registry_words::{trace_remote_source_flag, trace_remote_source_option};
pub(super) use transition_words::logical_source_registry_words;
use transition_words::logical_source_registry_words_in_original_image;
pub(crate) use transition_words::{
    conditional_lambda_formatting_words, source_transition_words_from_advice,
};
use vendor_words::conditional_roles;
pub(crate) use vendor_words::original_vendor_registry_words;
pub use vendor_words::selected_vendor_registry_words_at;

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

#[cfg(test)]
mod stock_source_proposal_metadata_tests;

#[cfg(test)]
mod original_query_document_image_tests;
