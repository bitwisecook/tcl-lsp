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

//! Source advice at an offset: publications, procedure calls, Logical candidates and classes.

use super::source_registry_words::source_registry_words_at;
use super::transition_words::source_transition_words_from_advice;
use super::{
    OriginalOperandSource, OriginalRegistrySource, OriginalRegistryWords, first_tail_command,
    original_query_document_image, retained_document_image, trace_jim_source_option,
};
use crate::analyser::AnalysisResult;

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
        return super::super::original_native_compiler_words(
            &image,
            tokens.words(),
            offset,
            config,
        );
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
) -> Option<super::super::OriginalSourceCommandPublication> {
    let words = source_registry_words_at(source, analysis, offset)?;
    let context = analysis.resolved_input.as_ref()?.context_registry();
    let site = || {
        analysis.retained_command_realm().and_then(|realm| {
            realm
                .invocation_at_source("", offset)
                .invocation_site()
                .cloned()
        })
    };
    super::super::source_publication::OriginalSourceCommandPublication::capture(
        words, &context, site,
    )
}

/// Genuine setter operands and complete single-command construction syntax.
/// No value, frame, allocation or successful store is issued.
#[must_use]
pub fn source_handle_construction_at(
    source: &str,
    analysis: &AnalysisResult,
    offset: u32,
) -> Option<super::super::OriginalSourceHandleConstruction> {
    let words = source_registry_words_at(source, analysis, offset)?;
    let context = analysis.resolved_input.as_ref()?.context_registry();
    super::super::source_publication::OriginalSourceHandleConstruction::capture(words, &context)
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

pub(crate) fn source_reached_procedure_call_at(
    source: &str,
    analysis: &AnalysisResult,
    offset: u32,
) -> Option<crate::command_binding::formal_topology::ReachedProcedureCall> {
    let input = analysis.resolved_input.as_ref()?;
    let image = original_query_document_image(source, analysis, input.lexer_config());
    analysis
        .matches_original_source_image(&image, input.lexer_config())
        .then_some(())?;
    let realm = analysis.retained_command_realm()?;
    let binding = realm.invocation_at_source("", offset);
    realm.source_bindings_ref().reached_procedure_call(&binding)
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
) -> Option<super::super::OriginalSourceHandleClassAdvice> {
    let words = source_registry_words_at(source, analysis, offset)?;
    let context = analysis.resolved_input.as_ref()?.context_registry();
    super::super::source_publication::OriginalSourceHandleClassAdvice::capture(
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

pub(super) fn registered_original_words_at<'a>(
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
        super::super::original_native_compiler_words(&image, tokens.words(), offset, config)?
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
        super::super::original_native_compiler_words(&image, tokens.words(), offset, config)?
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
        super::super::original_native_compiler_words(&image, tokens.words(), offset, config)?
    };
    (native.first()?.span().start() == offset).then_some((input, realm, native))
}

/// Authored target metadata of a selected original list builder at its actual
/// source point. Inert list data gains no deferred parent, future lookup,
/// evaluated result object, current receiver or dispatch authority.
#[must_use]
pub fn source_authored_command_prefix(
    source: &str,
    analysis: &AnalysisResult,
    producer: &tcl_source::segment::SegmentedCommand,
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
    let original = super::super::original_native_compiler_words(
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
    producer: &tcl_source::segment::SegmentedCommand,
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
    producer: &tcl_source::segment::SegmentedCommand,
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
    segment: &tcl_source::segment::SegmentedCommand,
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
