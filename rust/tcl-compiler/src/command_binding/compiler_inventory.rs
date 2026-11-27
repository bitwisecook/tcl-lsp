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

//! Compiler visitation and runtime entry are independent source inventories.

use super::{Arc, SourceCommandBindings, SourceInvocationBinding};
use crate::command_binding::SourceOriginLexical;
use std::collections::BTreeMap;
use tcl_names::command::CommandAllocationSite;

fn compiler_syntax_trace_enabled() -> bool {
    static ROWS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    if !(cfg!(test)
        || cfg!(debug_assertions)
        || matches!(option_env!("TCL_LSP_TRACE_COARSE_SOURCE"), Some("1")))
        || tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_BODY_SYNTAX").is_none()
    {
        return false;
    }
    ROWS.try_update(
        std::sync::atomic::Ordering::Relaxed,
        std::sync::atomic::Ordering::Relaxed,
        |rows| (rows < 1024).then_some(rows + 1),
    )
    .is_ok()
}

pub(super) fn trace_compiler_syntax_option<T>(
    stage: &str,
    offset: Option<u32>,
    value: Option<T>,
) -> Option<T> {
    if compiler_syntax_trace_enabled() {
        eprintln!(
            "ORIGINAL_COMPILER_SYNTAX_GATE stage={stage} offset={offset:?} present={}",
            value.is_some()
        );
    }
    value
}

pub(super) fn trace_compiler_syntax_flag(stage: &str, offset: Option<u32>, value: bool) -> bool {
    if compiler_syntax_trace_enabled() {
        eprintln!("ORIGINAL_COMPILER_SYNTAX_GATE stage={stage} offset={offset:?} value={value}");
    }
    value
}

/// Runtime entry knowledge for one exact original command allocation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum SourceRuntimeReachability {
    /// An original declaration/template was interpreted conditionally. This
    /// retains operand layout and potential effects, never actual entry.
    Conditional,
    /// At least one retained traversal reached this command's dispatch.
    Reached,
    /// Complete bounded traversals covered the source but never reached dispatch.
    NotEntered,
    /// Source coverage, suspension or an opaque execution remains unresolved.
    #[default]
    Unknown,
}

/// Exact original words retained by an actual compiler observation. The
/// snapshot keeps its compiler world and descriptive frame; it never grants a
/// runtime lookup, variable cell, entered body or successful invocation.
pub(crate) struct OriginalCompilerArgumentSyntax<'a> {
    metadata: crate::registry_invocation::InvocationMetadataContext<'a>,
    original: &'a super::compiled_invocation::SourceOriginalCompilerWords,
    snapshot: &'a Arc<super::SourceLookupSnapshot>,
    namespace: &'a tcl_names::namespace::SourceNamespaceKey,
    dialect: Option<tcl_native_model::InvocationDialect>,
    analysis_owner: &'a Arc<super::OriginalSourceAnalysisOwner>,
}

impl<'a> OriginalCompilerArgumentSyntax<'a> {
    pub(crate) fn metadata(&self) -> crate::registry_invocation::InvocationMetadataContext<'a> {
        self.metadata
    }

    pub(crate) fn site(&self) -> &'a CommandAllocationSite {
        &self.original.site
    }

    pub(crate) fn image(&self) -> &'a tcl_lexer::SourceImage {
        self.original.site.source.source_image()
    }

    pub(crate) fn words(&self) -> &'a [tcl_source::words::WordExpr] {
        &self.original.words
    }

    pub(crate) fn config(&self) -> tcl_lexer::LexerConfig {
        self.original.config
    }

    pub(crate) fn realm(&self) -> tcl_dialect::model::InvocationRealm {
        self.snapshot.realm
    }

    pub(crate) fn source_dialect(&self) -> Option<tcl_native_model::InvocationDialect> {
        let mut dialect = self.dialect?;
        dialect.lexer_grammar = self.config().grammar_over(dialect.lexer_grammar);
        dialect.word_values = tcl_syntax::word_rules::WordValueRules::from_config(&self.config());
        Some(dialect)
    }

    pub(crate) fn source_lookup_advice(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<super::OriginalCompilationLookupAdvice> {
        super::original_site_operand_layout_advice(
            self.site(),
            tokens,
            self.snapshot,
            self.namespace,
            self.config(),
        )
    }

    pub(crate) fn same_analysis(&self, other: &Self) -> bool {
        Arc::ptr_eq(self.analysis_owner, other.analysis_owner)
    }
}

impl SourceInvocationBinding {
    /// Trace existing compiler/source/runtime syntax refusals without changing them.
    #[cfg(debug_assertions)]
    pub(crate) fn trace_original_body_syntax(
        &self,
        tokens: &crate::ir::CommandTokens,
        input: &crate::analyser::ResolvedAnalysisInput,
        registry: &tcl_registry::CommandRegistry,
        attachment: bool,
    ) {
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_BODY_SYNTAX").is_none() {
            return;
        }
        let syntax = self.original_compiler_argument_syntax(tokens, input, registry);
        let metadata = syntax
            .as_ref()
            .map(OriginalCompilerArgumentSyntax::metadata);
        let compiler = self.original_compilation_lookup_advice(tokens);
        let source_purpose = metadata
            .and_then(|metadata| self.original_source_advice_purpose(tokens, metadata, registry));
        let compiler_purpose = compiler
            .as_ref()
            .zip(metadata)
            .and_then(|(advice, metadata)| advice.original_source_purpose(metadata, registry));
        let compiler_closed = compiler
            .as_ref()
            .zip(metadata)
            .is_some_and(|(advice, metadata)| {
                advice.closed_authored_simulation_source_lookup(tokens, metadata, registry)
            });
        eprintln!(
            "ORIGINAL_BODY_SYNTAX offset={:?} head={:?} words={} attachment={} syntax={} compiler_words={} compiler_state={} owner={} dialect={} protocol={} source_purpose={source_purpose:?} compiler_purpose={compiler_purpose:?} compiler_source_closed={compiler_closed}",
            tokens
                .words()
                .first()
                .map(|word| word.source().span.start()),
            tokens.argv_texts.first(),
            tokens.words().len(),
            attachment,
            syntax.is_some(),
            self.original_compiler_words.is_some(),
            self.compiler_lookup_state.is_some(),
            self.original_analysis_owner.is_some(),
            self.native_compiler_dialect().is_some(),
            self.compiler_source_protocol().is_some(),
        );
        eprintln!(
            "ORIGINAL_BODY_SYNTAX_RUNTIME offset={:?} lookup={} target={} handler={} unobserved={} names={} closed={} evaluated_body={}",
            tokens
                .words()
                .first()
                .map(|word| word.source().span.start()),
            self.lookup_state.is_some(),
            self.proved_target().is_some(),
            self.proved_handler_target().is_some(),
            self.unobserved_native_dispatch(),
            self.original_compiler_names_preserved_for_tokens(tokens),
            self.original_registry_syntax_is_closed(tokens, registry),
            tokens.evaluated_body.is_some(),
        );
        if let Some(advice) = compiler {
            let state = &advice.snapshot.state;
            eprintln!(
                "ORIGINAL_BODY_SYNTAX_COMPILER offset={} opaque={} unclosed={} step={} observers_quiet={} targets={} unknown={} may_absent={}",
                advice.site.offset,
                state.has_opaque_domain(),
                state.unclosed_command_world,
                state.source_step_observed(),
                state.command_observers.is_quiet(),
                advice.targets.len(),
                advice.unknown,
                advice.may_be_absent,
            );
        }
    }

    /// Syntax from the compiler's own full-input snapshot. This is a distinct
    /// purpose from `original_argument_metadata_for_input`: an abrupt argv read
    /// can prevent a runtime point while the compiler still visited the words.
    /// Source-only inherited entries have no compiler policy/word observation
    /// and cannot manufacture this carrier from their source naming recipe.
    pub(crate) fn original_compiler_argument_syntax<'a>(
        &'a self,
        tokens: &crate::ir::CommandTokens,
        input: &crate::analyser::ResolvedAnalysisInput,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<OriginalCompilerArgumentSyntax<'a>> {
        if tokens.source_binding.as_ref() != Some(self)
            || tokens.synthetic.is_some()
            || !tokens.original_words_match_argv_text()
            || input.has_logical_source_name_context()
            || input.has_hosted_source_name_context()
        {
            return None;
        }
        self.native_compiler_dialect()?;
        self.compiler_source_protocol()?;
        let original = self.original_compiler_words.as_ref()?;
        let snapshot = self.compiler_lookup_state.as_ref()?;
        let analysis_owner = self.original_analysis_owner.as_ref()?;
        if original.words.as_ref() != tokens.words()
            || original.config != self.original_argument_lexer_config(tokens)?
            || snapshot.state.current_source_origin.as_ref() != Some(&original.site.source)
            || snapshot.state.baseline.registry_snapshot != Some(registry.snapshot().semantic_key())
            || self.compiler_invocation_realm() != Some(snapshot.realm)
            || !Arc::ptr_eq(&analysis_owner.source, &original.site.source)
            || analysis_owner.config != original.config
        {
            return None;
        }
        original.site.source.original_lexical_command(
            &original.words,
            original.site.offset,
            original.config,
        )?;
        let point = snapshot
            .state
            .baseline
            .metadata_context
            .source_analysis_input()?;
        let owner_input = analysis_owner.metadata.source_analysis_input()?;
        // The caller supplies the actual retained whole-input attachment. Only
        // that issuer may project the compiler point to nested-source rules;
        // normalising an arbitrary caller would erase changed whole-file axes.
        if owner_input != input
            || (point != owner_input && point != &owner_input.for_nested_source())
        {
            return None;
        }
        let metadata = crate::registry_invocation::InvocationMetadataContext::for_source_input(
            registry,
            point,
            original.config,
            Some(point.unit_profile()),
        )?;
        Some(OriginalCompilerArgumentSyntax {
            metadata,
            original,
            snapshot,
            namespace: &self.lookup_namespace_key,
            dialect: self.compiler_word_dialect(),
            analysis_owner,
        })
    }
}

impl SourceCommandBindings {
    /// Read every retained compiler row without assigning a body or selecting it.
    #[cfg(debug_assertions)]
    pub(super) fn trace_original_body_compiler_census(
        &self,
        body: crate::ir::ProcedureId,
        span: tcl_lexer::Span,
        input: &crate::analyser::ResolvedAnalysisInput,
        registry: &tcl_registry::CommandRegistry,
    ) {
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_BODY_SYNTAX").is_none() {
            return;
        }
        let in_span =
            |site: &CommandAllocationSite| span.start() <= site.offset && site.offset < span.end();
        let trace_visit = |kind: &str,
                           site: &CommandAllocationSite,
                           ordinal: usize,
                           visit: &SourceCompilerInvocation| {
            let original = visit.original_words.as_ref();
            let policy = visit.policy.as_ref();
            let point = visit
                .table
                .state
                .baseline
                .metadata_context
                .source_analysis_input();
            eprintln!(
                "ORIGINAL_BODY_COMPILER_ROW body={body:?} kind={kind} offset={} ordinal={ordinal} head={:?} root={} numeric_in_body={} selection={:?} admitted={} structured={} words={} word_count={:?} word_offset={:?} config={:?} config_matches={} source_site={} table_source={} registry={} metadata_matches={} compiler={} dialect={} protocol={} realm={}",
                site.offset,
                visit.head,
                self.root_origin
                    .as_ref()
                    .is_some_and(|root| Arc::ptr_eq(root, &site.source)),
                in_span(site),
                visit.selection,
                visit.admitted.is_some(),
                visit.structured.is_some(),
                original.is_some(),
                original.map(|words| words.words.len()),
                original
                    .and_then(|words| words.words.first().map(|word| word.source().span.start())),
                original.map(|words| words.config),
                original.is_some_and(|words| words.config == input.lexer_config()),
                original.is_some_and(|words| words.site == *site),
                visit.table.state.current_source_origin.as_ref() == Some(&site.source),
                visit.table.state.baseline.registry_snapshot
                    == Some(registry.snapshot().semantic_key()),
                point == Some(input),
                policy.is_some_and(|policy| policy.compiler.is_some()),
                policy.is_some_and(|policy| policy.words.is_some()),
                policy.is_some_and(|policy| policy.source_protocol.is_some()),
                policy.is_some_and(|policy| policy.realm == visit.table.realm),
            );
        };
        eprintln!(
            "ORIGINAL_BODY_COMPILER_CENSUS body={body:?} span={span:?} direct_sites={} child_regions={} analysis_owner={}",
            self.compiler_invocations.len(),
            self.compiled_children.len(),
            self.original_analysis_owner.is_some(),
        );
        for (site, visits) in &self.compiler_invocations {
            for (ordinal, visit) in visits.iter().enumerate() {
                trace_visit("direct", site, ordinal, visit);
            }
        }
        for (site, children) in &self.compiled_children {
            for (ordinal, child) in children.iter().enumerate() {
                let parent_current = match child.parent.as_ref() {
                    super::compiled_invocation::SourceNativeCompilerAdmission::Inline(parent) => {
                        self.compiler_invocations
                            .get(&parent.compilation_site)
                            .is_some_and(|parents| {
                                !parents.is_empty()
                                    && parents.iter().all(|visit| {
                                        visit.admitted.as_deref() == Some(child.parent.as_ref())
                                            && visit.namespace_key == child.parent_namespace_key
                                            && visit.table == child.table
                                    })
                            })
                    }
                    super::compiled_invocation::SourceNativeCompilerAdmission::Named(_) => false,
                };
                self.trace_original_body_compiler_child(body, site, ordinal, child, parent_current);
                for (command, visits) in child.compiler_visits.iter() {
                    for (ordinal, visit) in visits.iter().enumerate() {
                        trace_visit("raw-child", command, ordinal, visit);
                    }
                }
            }
        }
    }

    #[cfg(debug_assertions)]
    fn trace_original_body_compiler_child(
        &self,
        body: crate::ir::ProcedureId,
        site: &CommandAllocationSite,
        ordinal: usize,
        child: &super::compiled_preflight::SourceCompiledChild,
        parent_current: bool,
    ) {
        eprintln!(
            "ORIGINAL_BODY_COMPILER_CHILD body={body:?} offset={} ordinal={ordinal} source_root={} enclosing_offset={} enclosing_root={} parent_offset={:?} parent_root={} base={} bytes={} mapping={:?} script_root={} origin_matches_site={} provider_required={} parent_current={parent_current} sites={}",
            site.offset,
            self.root_origin
                .as_ref()
                .is_some_and(|root| Arc::ptr_eq(root, &site.source)),
            child.enclosing.offset,
            self.root_origin
                .as_ref()
                .is_some_and(|root| Arc::ptr_eq(root, &child.enclosing.source)),
            match child.parent.as_ref() {
                super::compiled_invocation::SourceNativeCompilerAdmission::Inline(parent) =>
                    Some(parent.compilation_site.offset),
                super::compiled_invocation::SourceNativeCompilerAdmission::Named(_) => None,
            },
            match child.parent.as_ref() {
                super::compiled_invocation::SourceNativeCompilerAdmission::Inline(parent) => self
                    .root_origin
                    .as_ref()
                    .is_some_and(|root| Arc::ptr_eq(root, &parent.compilation_site.source)),
                super::compiled_invocation::SourceNativeCompilerAdmission::Named(_) => false,
            },
            child.script.base(),
            child.script.text.len(),
            child.script.mapping,
            self.root_origin
                .as_ref()
                .is_some_and(|root| Arc::ptr_eq(root, &child.script.origin)),
            child.script.origin == site.source,
            self.compilation_provider_required
                .contains(&child.enclosing),
            child.compiler_visits.len(),
        );
    }

    #[cfg(debug_assertions)]
    fn trace_original_compiler_syntax_query(
        offset: Option<u32>,
        tokens: &crate::ir::CommandTokens,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) {
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_BODY_SYNTAX").is_some() {
            eprintln!(
                "ORIGINAL_COMPILER_SYNTAX_QUERY offset={offset:?} words={:?} argv_texts={:?} input_config={:?}",
                tokens.words(),
                tokens.argv_texts,
                input.lexer_config()
            );
        }
    }

    /// Every actual compiler observation retains its own immutable table and
    /// namespace. Missing unanimous Native attachment cannot select one table.
    pub(crate) fn original_compiler_syntax_alternatives<'a>(
        &'a self,
        tokens: &crate::ir::CommandTokens,
        input: &crate::analyser::ResolvedAnalysisInput,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<Vec<OriginalCompilerArgumentSyntax<'a>>> {
        let offset = tokens
            .words()
            .first()
            .map(|word| word.source().span.start());
        #[cfg(debug_assertions)]
        Self::trace_original_compiler_syntax_query(offset, tokens, input);
        let owner = trace_compiler_syntax_option(
            "analysis_owner",
            offset,
            self.original_analysis_owner.as_ref(),
        )?;
        let binding =
            trace_compiler_syntax_option("source_binding", offset, tokens.source_binding.as_ref())?;
        if trace_compiler_syntax_flag("synthetic", offset, tokens.synthetic.is_some())
            || trace_compiler_syntax_flag(
                "word_text_mismatch",
                offset,
                !tokens.original_words_match_argv_text(),
            )
            || trace_compiler_syntax_flag(
                "logical_context",
                offset,
                input.has_logical_source_name_context(),
            )
            || trace_compiler_syntax_flag(
                "hosted_context",
                offset,
                input.has_hosted_source_name_context(),
            )
            || trace_compiler_syntax_flag(
                "binding_owner_mismatch",
                offset,
                !binding
                    .original_analysis_owner
                    .as_ref()
                    .is_some_and(|actual| Arc::ptr_eq(actual, owner)),
            )
            || trace_compiler_syntax_flag(
                "owner_input_mismatch",
                offset,
                owner.metadata.source_analysis_input() != Some(input),
            )
            || trace_compiler_syntax_flag(
                "source_image_mismatch",
                offset,
                !self.matches_original_source_image(
                    owner.source.source_image(),
                    input.lexer_config(),
                ),
            )
        {
            return None;
        }
        self.original_compiler_syntax_visits_for_owner(tokens, input, registry, owner, offset)
    }

    /// Shared actual-visit validation. The ordinary consumer first checks its
    /// attached owner; restoration instead authenticates the current recorded
    /// child at the source producer before publishing an attachment.
    pub(super) fn original_compiler_syntax_visits_for_owner<'a>(
        &'a self,
        tokens: &crate::ir::CommandTokens,
        input: &crate::analyser::ResolvedAnalysisInput,
        registry: &tcl_registry::CommandRegistry,
        owner: &'a Arc<super::OriginalSourceAnalysisOwner>,
        offset: Option<u32>,
    ) -> Option<Vec<OriginalCompilerArgumentSyntax<'a>>> {
        if !self
            .original_analysis_owner
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(current, owner))
        {
            return None;
        }
        let site = CommandAllocationSite {
            source: Arc::clone(&owner.source),
            offset: trace_compiler_syntax_option("head_word", offset, tokens.words().first())?
                .source()
                .span
                .start(),
        };
        let mut visits = self
            .compiler_invocations
            .get(&site)
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        #[cfg(debug_assertions)]
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_BODY_SYNTAX").is_some() {
            eprintln!(
                "ORIGINAL_COMPILER_SYNTAX_DIRECT offset={offset:?} visits={}",
                visits.len()
            );
        }
        if let Some(children) = trace_compiler_syntax_option(
            "child_visits",
            offset,
            self.original_child_compiler_visits(&site),
        ) {
            visits.extend(children);
        }
        visits.extend(self.original_expression_compiler_syntax_visits(&site));
        if trace_compiler_syntax_flag("visits_empty", offset, visits.is_empty()) {
            return None;
        }
        visits
            .into_iter()
            .enumerate()
            .map(|(ordinal, visit)| {
                #[cfg(debug_assertions)]
                if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_BODY_SYNTAX").is_some() {
                    eprintln!("ORIGINAL_COMPILER_SYNTAX_VISIT offset={offset:?} ordinal={ordinal} words={:?}", visit.original_words.as_ref().map(|words| &words.words));
                }
                #[cfg(not(debug_assertions))]
                let _ = ordinal;
                trace_compiler_syntax_option("alternative", offset, original_compiler_syntax_alternative(visit, owner, tokens, input, registry))
            })
            .collect()
    }
}

fn original_compiler_syntax_alternative<'a>(
    visit: &'a SourceCompilerInvocation,
    owner: &'a Arc<super::OriginalSourceAnalysisOwner>,
    tokens: &crate::ir::CommandTokens,
    input: &crate::analyser::ResolvedAnalysisInput,
    registry: &tcl_registry::CommandRegistry,
) -> Option<OriginalCompilerArgumentSyntax<'a>> {
    let offset = tokens
        .words()
        .first()
        .map(|word| word.source().span.start());
    let original = trace_compiler_syntax_option(
        "visit.original_words",
        offset,
        visit.original_words.as_ref(),
    )?;
    let policy = trace_compiler_syntax_option("visit.policy", offset, visit.policy.as_ref())?;
    trace_compiler_syntax_option("visit.compiler", offset, policy.compiler)?;
    trace_compiler_syntax_option("visit.source_protocol", offset, policy.source_protocol)?;
    let dialect = trace_compiler_syntax_option("visit.word_dialect", offset, policy.words)?;
    let snapshot = &visit.table;
    if trace_compiler_syntax_flag(
        "visit.word_vector_mismatch",
        offset,
        original.words.as_ref() != tokens.words(),
    ) || trace_compiler_syntax_flag(
        "visit.input_config_mismatch",
        offset,
        original.config != input.lexer_config(),
    ) || trace_compiler_syntax_flag(
        "visit.owner_config_mismatch",
        offset,
        owner.config != original.config,
    ) || trace_compiler_syntax_flag(
        "visit.origin_mismatch",
        offset,
        !Arc::ptr_eq(&owner.source, &original.site.source),
    ) || trace_compiler_syntax_flag(
        "visit.snapshot_origin_mismatch",
        offset,
        snapshot.state.current_source_origin.as_ref() != Some(&original.site.source),
    ) || trace_compiler_syntax_flag(
        "visit.registry_mismatch",
        offset,
        snapshot.state.baseline.registry_snapshot != Some(registry.snapshot().semantic_key()),
    ) || trace_compiler_syntax_flag(
        "visit.realm_mismatch",
        offset,
        policy.realm != snapshot.realm,
    ) {
        return None;
    }
    trace_compiler_syntax_option(
        "visit.original_lexical_command",
        offset,
        original.site.source.original_lexical_command(
            &original.words,
            original.site.offset,
            original.config,
        ),
    )?;
    let point = trace_compiler_syntax_option(
        "visit.point_input",
        offset,
        snapshot
            .state
            .baseline
            .metadata_context
            .source_analysis_input(),
    )?;
    if trace_compiler_syntax_flag(
        "visit.point_input_mismatch",
        offset,
        point != input && point != &input.for_nested_source(),
    ) {
        return None;
    }
    let metadata = trace_compiler_syntax_option(
        "visit.metadata",
        offset,
        crate::registry_invocation::InvocationMetadataContext::for_source_input(
            registry,
            point,
            original.config,
            Some(point.unit_profile()),
        ),
    )?;
    Some(OriginalCompilerArgumentSyntax {
        metadata,
        original,
        snapshot,
        namespace: &visit.namespace_key,
        dialect: Some(dialect),
        analysis_owner: owner,
    })
}

#[cfg(test)]
mod source_syntax_alternative_tests;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[doc(hidden)]
pub struct SourceCompilerInvocation {
    pub source_locals:
        Option<Arc<super::compiled_preflight::ordered_locals::SourceCompilerLocalInventory>>,
    pub selection: Option<tcl_registry::native_compilation::NativeCompilationSelection>,
    pub admitted: Option<Arc<super::compiled_invocation::SourceNativeCompilerAdmission>>,
    pub operand_layout: Option<Arc<super::SourceNativeOperandLayoutProof>>,
    pub namespace_bindings:
        Option<Arc<super::compiled_invocation::SourceNativeNamespaceBindingPreparation>>,
    pub switch: Option<Arc<super::compiled_invocation::SourceNativeSwitchPreparation>>,
    pub structured: Option<Arc<super::compiled_invocation::SourceNativeStructuredPreparation>>,
    pub original_words: Option<Arc<super::compiled_invocation::SourceOriginalCompilerWords>>,
    pub policy: Option<Arc<SourceNativeCompilerPolicy>>,
    pub namespace: String,
    pub namespace_key: tcl_names::namespace::SourceNamespaceKey,
    pub head: Option<String>,
    // Derived equality compares in field order: the whole table last.
    pub table: Arc<super::SourceLookupSnapshot>,
}

pub(super) type SourceCompilerVisits =
    BTreeMap<CommandAllocationSite, Vec<SourceCompilerInvocation>>;

/// Policies of original compiler observations, independent of their changing
/// lookup/variable worlds. Missing axes remain missing and all observations
/// must agree; this carrier contains no implementation or handler authority.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[doc(hidden)]
pub struct SourceNativeCompilerPolicy {
    words: Option<tcl_native_model::InvocationDialect>,
    source_protocol: Option<tcl_syntax::native_string::NativeStringProtocol>,
    compiler: Option<tcl_native_model::InvocationDialect>,
    realm: tcl_dialect::model::InvocationRealm,
}

impl SourceNativeCompilerPolicy {
    pub(super) fn of_compilation(
        state: &super::ModuleCommandBindings,
        realm: tcl_dialect::model::InvocationRealm,
    ) -> Self {
        Self {
            words: state.baseline.dialect,
            source_protocol: Self::source_protocol_of(state),
            compiler: state.baseline.compilation_dialect(),
            realm,
        }
    }

    pub(super) fn source_protocol_of(
        state: &super::ModuleCommandBindings,
    ) -> Option<tcl_syntax::native_string::NativeStringProtocol> {
        match state.baseline.native_entry.as_deref() {
            Some(entry) => entry.source_string_protocol,
            None => state.baseline.dialect?.native_source_string_protocol(),
        }
    }
}

#[cfg(test)]
mod policy_tests;

impl SourceInvocationBinding {
    /// Exact original structured compiler visits, including Generic and
    /// Rejected selections. Changed source vectors or compiler policy abstain;
    /// this receipt grants neither handler entry nor normal completion.
    #[must_use]
    pub fn original_structured_compilation(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<&super::compiled_invocation::SourceNativeStructuredPreparation> {
        if !tokens.original_words_match_argv_text() {
            return None;
        }
        self.original_compiler_source(tokens)?;
        self.native_compiler_dialect()?;
        self.compiler_source_protocol()?;
        let original = self.original_compiler_words.as_ref()?;
        let preparation = self.native_structured_preparation.as_deref()?;
        (preparation.compilation_site == original.site
            && self.native_compilation_admission_selection()
                != tcl_registry::native_compilation::NativeCompilationSelection::Unknown)
            .then_some(preparation)
    }

    /// Original Inline Control structure for a genuine retained compiler entry.
    /// Generic/rejected recipes and absent physical compiler registrations
    /// cannot supply this source purpose. Runtime entry remains independent.
    pub(crate) fn original_inline_control_compilation(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<&super::compiled_invocation::SourceNativeStructuredPreparation> {
        use tcl_registry::native_compilation::NativeCompilationSelection;
        use tcl_registry::native_control_compilation::NativeControlOutcome;
        use tcl_registry::native_instruction_plan::NativeInstructionPlan;
        let preparation = self.original_structured_compilation(tokens)?;
        if !matches!(
            self.native_compilation_admission_selection(),
            NativeCompilationSelection::Inline { .. }
        ) || !matches!(&preparation.recipe, NativeInstructionPlan::Control(control)
                if matches!(&control.outcome, NativeControlOutcome::Inline(_)))
        {
            return None;
        }
        let original = self.original_compiler_words.as_ref()?;
        let snapshot = self.compiler_lookup_state.as_ref()?;
        let entry = snapshot.state.baseline.native_entry.as_deref()?;
        let prerequisite = preparation.dependency.compiler_prerequisite.as_deref()?;
        let policy = self.compiler_policy.as_deref()?;
        let input = snapshot
            .state
            .baseline
            .metadata_context
            .source_analysis_input()?;
        if prerequisite.interpreter != entry.interpreter
            || snapshot.state.current_source_origin.as_ref() != Some(&original.site.source)
            || preparation.compilation_site != original.site
            || self.original_lexer_config_for_tokens(tokens)? != original.config
            || original.config != input.lexer_config()
            || policy.words != snapshot.state.baseline.dialect
            || policy.compiler != snapshot.state.baseline.compilation_dialect()
            || policy.source_protocol != entry.source_string_protocol
            || policy.realm != snapshot.realm
        {
            return None;
        }
        Some(preparation)
    }

    /// Join compiler-only source structure to the caller's complete input.
    /// The retained compiler point supplies metadata, never a new handler,
    /// Normal completion, native execution entry or body activation.
    pub(crate) fn original_inline_control_metadata_for_input<'a>(
        &'a self,
        tokens: &crate::ir::CommandTokens,
        input: &crate::analyser::ResolvedAnalysisInput,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<crate::registry_invocation::InvocationMetadataContext<'a>> {
        self.original_inline_control_compilation(tokens)?;
        let snapshot = self.compiler_lookup_state.as_ref()?;
        if snapshot.state.baseline.registry_snapshot.as_ref()
            != Some(&registry.snapshot().semantic_key())
        {
            return None;
        }
        let original = self.original_compiler_words.as_ref()?;
        let point = snapshot
            .state
            .baseline
            .metadata_context
            .source_analysis_input()?;
        if point != input {
            return None;
        }
        crate::registry_invocation::InvocationMetadataContext::for_source_input(
            registry,
            point,
            original.config,
            Some(point.unit_profile()),
        )
    }

    /// Original switch compiler body visits, independent of runtime arm entry.
    /// Changed source vectors or an unknown joined admission retain no recipe.
    #[must_use]
    pub fn original_switch_compilation(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<&tcl_registry::native_switch_compilation::NativeSwitchInstruction> {
        if !tokens.original_words_match_argv_text() {
            return None;
        }
        self.original_compiler_source(tokens)?;
        self.native_compiler_dialect()?;
        self.compiler_source_protocol()?;
        let original = self.original_compiler_words.as_ref()?;
        let preparation = self.native_switch_preparation.as_deref()?;
        (preparation.compilation_site == original.site
            && self.native_compilation_admission_selection()
                != tcl_registry::native_compilation::NativeCompilationSelection::Unknown)
            .then_some(&preparation.recipe)
    }

    /// Authenticate the complete original compiler vector, including source
    /// channel and delimiters. A transformed or truncated token vector abstains.
    #[doc(hidden)]
    #[must_use]
    pub fn original_compiler_source<'a>(
        &'a self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<(
        &'a tcl_lexer::SourceImage,
        &'a [tcl_source::words::WordExpr],
        u32,
    )> {
        let original = self.original_compiler_words.as_ref()?;
        (tokens.synthetic.is_none() && tokens.words() == original.words.as_ref()).then(|| {
            (
                original.site.source.source_image(),
                original.words.as_ref(),
                original.site.offset,
            )
        })
    }

    /// Borrow the original compiler receipt retained before IR expression
    /// transformations. Admission and emitted operation guards remain required.
    #[doc(hidden)]
    #[must_use]
    pub fn retained_original_compiler_source(
        &self,
    ) -> Option<(&tcl_lexer::SourceImage, &[tcl_source::words::WordExpr], u32)> {
        let original = self.original_compiler_words.as_ref()?;
        Some((
            original.site.source.source_image(),
            original.words.as_ref(),
            original.site.offset,
        ))
    }

    /// Runtime entry is not implied by an admitted native compiler recipe.
    #[must_use]
    pub fn runtime_reachability(&self) -> SourceRuntimeReachability {
        self.runtime_reachability
    }

    /// Original word-value policy retained at compiler visitation. This is
    /// source decoding evidence, not a compiler engine or normal-handler proof.
    #[doc(hidden)]
    #[must_use]
    pub fn compiler_word_dialect(&self) -> Option<tcl_native_model::InvocationDialect> {
        self.compiler_policy.as_ref()?.words
    }

    /// Original source-string issuer, independent of the installed compiler.
    #[doc(hidden)]
    #[must_use]
    pub fn compiler_source_protocol(
        &self,
    ) -> Option<tcl_syntax::native_string::NativeStringProtocol> {
        self.compiler_policy.as_ref()?.source_protocol
    }

    /// Native dialect retained at compiler visitation, without runtime facts.
    #[must_use]
    pub fn native_compiler_dialect(&self) -> Option<tcl_native_model::InvocationDialect> {
        self.compiler_policy.as_ref()?.compiler
    }

    pub(crate) fn compiler_invocation_realm(&self) -> Option<tcl_dialect::model::InvocationRealm> {
        Some(self.compiler_policy.as_ref()?.realm)
    }
}

impl SourceCommandBindings {
    /// Original structured compiler recipe whose source, frame and registration
    /// are unanimous across all retained observations of this exact allocation.
    /// The recipe supplies compiler visits, not runtime body or value authority.
    #[must_use]
    pub fn structured_compilation_at(
        &self,
        site: &CommandAllocationSite,
    ) -> Option<&super::compiled_invocation::SourceNativeStructuredPreparation> {
        let visits = self
            .original_child_compiler_visits(site)
            .unwrap_or_else(|| {
                self.compiler_invocations
                    .get(site)
                    .map_or_else(Vec::new, |visits| visits.iter().collect())
            });
        let first = *visits.first()?;
        let preparation = first.structured.as_deref()?;
        (preparation.compilation_site == *site
            && visits.iter().all(|visit| {
                visit.structured == first.structured
                    && visit.policy == first.policy
                    && visit.namespace_key == first.namespace_key
                    && visit.table == first.table
                    && visit.original_words == first.original_words
            }))
        .then_some(preparation)
    }

    /// Exact original compiler preparation, without runtime alias/store facts.
    /// All retained compiler observations must agree on recipe and guards.
    #[must_use]
    pub fn namespace_binding_preparation_at(
        &self,
        site: &CommandAllocationSite,
    ) -> Option<&super::compiled_invocation::SourceNativeNamespaceBindingPreparation> {
        let visits = self
            .original_child_compiler_visits(site)
            .unwrap_or_else(|| {
                self.compiler_invocations
                    .get(site)
                    .map_or_else(Vec::new, |visits| visits.iter().collect())
            });
        let first = *visits.first()?;
        let preparation = first.namespace_bindings.as_deref()?;
        (preparation.compilation_site() == site
            && visits.iter().all(|visit| {
                visit.namespace_bindings == first.namespace_bindings
                    && visit.policy == first.policy
                    && visit.namespace_key == first.namespace_key
                    && visit.table == first.table
            }))
        .then_some(preparation)
    }

    /// Exact original switch compiler arms, with unanimous source and guards.
    /// This grants body-compilation geometry, not runtime arm selection.
    #[must_use]
    pub fn switch_compilation_at(
        &self,
        site: &CommandAllocationSite,
    ) -> Option<&tcl_registry::native_switch_compilation::NativeSwitchInstruction> {
        let visits = self
            .original_child_compiler_visits(site)
            .unwrap_or_else(|| {
                self.compiler_invocations
                    .get(site)
                    .map_or_else(Vec::new, |visits| visits.iter().collect())
            });
        let first = *visits.first()?;
        let preparation = first.switch.as_deref()?;
        (preparation.compilation_site == *site
            && visits.iter().all(|visit| {
                visit.switch == first.switch
                    && visit.policy == first.policy
                    && visit.namespace_key == first.namespace_key
                    && visit.table == first.table
                    && visit.original_words == first.original_words
            }))
        .then_some(&preparation.recipe)
    }

    pub(super) fn retain_compiler_invocations(
        &mut self,
        incoming: &BTreeMap<CommandAllocationSite, Vec<SourceCompilerInvocation>>,
    ) {
        for (site, invocations) in incoming {
            let retained = self.compiler_invocations.entry(site.clone()).or_default();
            for invocation in invocations {
                if !retained.contains(invocation) {
                    retained.push(invocation.clone());
                }
            }
        }
    }

    pub(super) fn attach_compiler_invocation(
        &self,
        mut binding: SourceInvocationBinding,
        origin: Option<&Arc<tcl_names::origin::SourceOriginId>>,
        offset: u32,
    ) -> SourceInvocationBinding {
        let Some(origin) = origin else {
            return binding;
        };
        let site = CommandAllocationSite {
            source: Arc::clone(origin),
            offset,
        };
        let reached = binding.runtime_reachability == SourceRuntimeReachability::Reached;
        let original = self.original_child_compiler_visits(&site);
        if reached && original.is_none() {
            if tcl_dialect::trace_env::var_os("TCL_LSP_NATIVE_CODEGEN_DIAGNOSTIC").is_some() {
                eprintln!(
                    "NATIVE_COMPILER_ATTACHMENT offset={offset} reason=reached-without-child selection={:?}",
                    binding.native_compilation_admission
                );
            }
            return binding;
        }
        if !reached && binding.runtime_reachability != SourceRuntimeReachability::Conditional {
            binding.runtime_reachability = self.missing_runtime_reachability(&site);
        }
        let invocations = original.unwrap_or_else(|| {
            self.compiler_invocations
                .get(&site)
                .map_or_else(Vec::new, |visits| visits.iter().collect())
        });
        let Some(first) = invocations.first() else {
            if tcl_dialect::trace_env::var_os("TCL_LSP_NATIVE_CODEGEN_DIAGNOSTIC").is_some() {
                eprintln!(
                    "NATIVE_COMPILER_ATTACHMENT offset={offset} reason=no-visits reachability={:?} selection={:?}",
                    binding.runtime_reachability, binding.native_compilation_admission
                );
            }
            return binding;
        };
        if reached && first.namespace_key != binding.lookup_namespace_key {
            if tcl_dialect::trace_env::var_os("TCL_LSP_NATIVE_CODEGEN_DIAGNOSTIC").is_some() {
                eprintln!(
                    "NATIVE_COMPILER_ATTACHMENT offset={offset} reason=namespace-mismatch head={:?} visits={} selection={:?}",
                    first.head,
                    invocations.len(),
                    binding.native_compilation_admission
                );
            }
            return binding;
        }
        attach_unanimous_original_compiler_metadata(&mut binding, first, &invocations, reached);
        if tcl_dialect::trace_env::var_os("TCL_LSP_NATIVE_CODEGEN_DIAGNOSTIC").is_some() {
            eprintln!(
                "NATIVE_COMPILER_ATTACHMENT offset={offset} head={:?} visits={} selections={:?} final={:?} original_words={} policy={} table={}",
                first.head,
                invocations.len(),
                invocations
                    .iter()
                    .map(|visit| visit.selection)
                    .collect::<Vec<_>>(),
                binding.native_compilation_admission,
                binding.original_compiler_words.is_some(),
                binding.compiler_policy.is_some(),
                binding.compiler_lookup_state.is_some()
            );
        }
        binding
    }

    /// An original child compiler visit selects its namespace independently
    /// of whether any runtime branch enters the child. The same parent
    /// admission, source image and entry prerequisites must still be retained.
    pub(crate) fn compiled_script_namespace_context(
        &self,
        source: &tcl_names::origin::ExecutedScriptSource,
    ) -> Option<tcl_names::namespace::SourceNamespaceKey> {
        let mut contexts = self.compiled_children.iter().flat_map(|(site, children)| {
            children.iter().filter_map(move |child| {
                if &child.script != source {
                    return None;
                }
                let current = child.script.origin == site.source
                    && child.script.base() == site.offset
                    && !self
                        .compilation_provider_required
                        .contains(&child.enclosing)
                    && matches!(
                        child.parent.as_ref(),
                        super::compiled_invocation::SourceNativeCompilerAdmission::Inline(_)
                    )
                    && self
                        .compiler_invocations
                        .get(&match child.parent.as_ref() {
                            super::compiled_invocation::SourceNativeCompilerAdmission::Inline(
                                parent,
                            ) => parent.compilation_site.clone(),
                            super::compiled_invocation::SourceNativeCompilerAdmission::Named(_) => {
                                unreachable!("checked inline child")
                            }
                        })
                        .is_some_and(|parents| {
                            !parents.is_empty()
                                && parents.iter().all(|visit| {
                                    visit.admitted.as_deref() == Some(child.parent.as_ref())
                                        && visit.namespace_key == child.parent_namespace_key
                                        && visit.table == child.table
                                })
                        });
                Some(current.then_some(&child.namespace_key))
            })
        });
        let first = contexts.next()??;
        contexts
            .all(|key| key == Some(first))
            .then(|| first.clone())
    }

    /// A prospective runtime preview does not replace the visits owned by an
    /// admitted original parent. A changed parent compilation does withdraw them.
    fn original_child_compiler_visits(
        &self,
        site: &CommandAllocationSite,
    ) -> Option<Vec<&SourceCompilerInvocation>> {
        let mut visits = Vec::new();
        for (child_site, children) in &self.compiled_children {
            if trace_compiler_syntax_flag(
                "child.site-filter",
                Some(site.offset),
                child_site.source != site.source || child_site.offset > site.offset,
            ) {
                continue;
            }
            for child in children {
                if trace_compiler_syntax_flag(
                    "child.region-or-provider",
                    Some(site.offset),
                    trace_compiler_syntax_flag(
                        "child.relative-region",
                        Some(site.offset),
                        site.offset
                            .checked_sub(child_site.offset)
                            .is_none_or(|offset| (offset as usize) >= child.script.text.len()),
                    ) || trace_compiler_syntax_flag(
                        "child.source-origin",
                        Some(site.offset),
                        child.script.origin != child_site.source,
                    ) || trace_compiler_syntax_flag(
                        "child.source-base",
                        Some(site.offset),
                        child.script.base() != child_site.offset,
                    ) || trace_compiler_syntax_flag(
                        "child.provider-required",
                        Some(site.offset),
                        self.compilation_provider_required
                            .contains(&child.enclosing),
                    ),
                ) {
                    continue;
                }
                let super::compiled_invocation::SourceNativeCompilerAdmission::Inline(parent) =
                    child.parent.as_ref()
                else {
                    trace_compiler_syntax_flag("child.parent.inline", Some(site.offset), false);
                    continue;
                };
                trace_compiler_syntax_flag("child.parent.inline", Some(site.offset), true);
                if trace_compiler_syntax_flag(
                    "child.parent-current",
                    Some(site.offset),
                    !self
                        .compiler_invocations
                        .get(&parent.compilation_site)
                        .is_some_and(|parents| {
                            !parents.is_empty()
                                && parents.iter().all(|visit| {
                                    visit.admitted.as_deref() == Some(child.parent.as_ref())
                                        && visit.namespace_key == child.parent_namespace_key
                                        && visit.table == child.table
                                })
                        }),
                ) {
                    continue;
                }
                if let Some(original) = trace_compiler_syntax_option(
                    "child.recorded-visits",
                    Some(site.offset),
                    child.compiler_visits.get(site),
                ) {
                    visits.extend(original);
                }
            }
        }
        trace_compiler_syntax_flag("child.visits-seen", Some(site.offset), !visits.is_empty())
            .then_some(visits)
    }

    fn missing_runtime_reachability(
        &self,
        site: &CommandAllocationSite,
    ) -> SourceRuntimeReachability {
        let covering = self
            .runtime_coverage
            .iter()
            .filter(|(chunk, (length, _))| {
                chunk.source == site.source
                    && chunk.offset <= site.offset
                    && u64::from(site.offset) < u64::from(chunk.offset) + *length as u64
            })
            .map(|(_, (_, closed))| *closed)
            .collect::<Vec<_>>();
        if !covering.is_empty() && covering.iter().all(|closed| *closed) {
            SourceRuntimeReachability::NotEntered
        } else {
            SourceRuntimeReachability::Unknown
        }
    }

    pub(super) fn record_runtime_coverage(
        &mut self,
        origin: Option<&Arc<tcl_names::origin::SourceOriginId>>,
        base: u32,
        length: usize,
        outcomes: &super::SourceOutcomes,
    ) {
        use tcl_registry::completion::CompletionCode;
        use tcl_registry::completion_route::InvocationCompletionRoute as Route;
        let Some(origin) = origin else {
            return;
        };
        let closed = outcomes
            .normal
            .as_ref()
            .is_none_or(|state| !state.opaque_domain)
            && outcomes.abrupt.iter().all(|(route, state)| {
                !state.opaque_domain
                    && !matches!(
                        route,
                        Route::Unknown
                            | Route::UnknownAbrupt
                            | Route::Tcl(CompletionCode::Other(_))
                    )
            });
        self.runtime_coverage
            .entry(CommandAllocationSite {
                source: Arc::clone(origin),
                offset: base,
            })
            .and_modify(|previous| {
                previous.1 &= closed && previous.0 == length;
            })
            .or_insert((length, closed));
    }
}

impl SourceInvocationBinding {
    /// Original static naming word selected by its exact retained source site.
    /// A first representative token may select its complete grouped word;
    /// arbitrary substrings, transformed vectors and conflicting configs abstain.
    #[must_use]
    pub fn original_source_name_key_at_span(
        &self,
        span: tcl_lexer::Span,
        config: tcl_lexer::LexerConfig,
        rules: tcl_syntax::word_rules::WordValueRules,
        policy: tcl_syntax::naming::NamePolicyProtocol,
    ) -> Option<tcl_names::word::SignatureSourceNameKey> {
        self.original_source_name_word_at_span(span, config, rules, policy)
            .map(|(_, key)| key)
    }

    pub(crate) fn original_source_name_word_at_span(
        &self,
        span: tcl_lexer::Span,
        config: tcl_lexer::LexerConfig,
        rules: tcl_syntax::word_rules::WordValueRules,
        policy: tcl_syntax::naming::NamePolicyProtocol,
    ) -> Option<(
        &tcl_source::words::WordExpr,
        tcl_names::word::SignatureSourceNameKey,
    )> {
        let site = self.invocation_site()?;
        let (image, words) =
            if let Some(observations) = self.declaration_layout_observations.as_deref() {
                let first = super::declaration_layout::original_declaration_layouts(observations)?
                    .next()?;
                if first.config != config || first.entry.source().origin != site.source {
                    return None;
                }
                (site.source.source_image(), first.words.as_ref())
            } else {
                let original = self.original_compiler_words.as_ref()?;
                if original.site != *site || original.config != config {
                    return None;
                }
                (original.site.source.source_image(), original.words.as_ref())
            };
        original_name_word_from_layout(site, image, words, span, config, rules, policy)
    }
}

fn original_name_word_from_layout<'a>(
    site: &CommandAllocationSite,
    image: &tcl_lexer::SourceImage,
    words: &'a [tcl_source::words::WordExpr],
    span: tcl_lexer::Span,
    config: tcl_lexer::LexerConfig,
    rules: tcl_syntax::word_rules::WordValueRules,
    policy: tcl_syntax::naming::NamePolicyProtocol,
) -> Option<(
    &'a tcl_source::words::WordExpr,
    tcl_names::word::SignatureSourceNameKey,
)> {
    let native = crate::registry_invocation::original_native_compiler_words(
        image,
        words,
        site.offset,
        config,
    )?;
    let mut selected = words.iter().zip(&native).filter(|(word, native)| {
        word.source().span == span
            || native
                .tokens()
                .first()
                .is_some_and(|token| token.span == span)
    });
    let (word, native) = selected.next()?;
    if selected.next().is_some() {
        return None;
    }
    let key =
        tcl_names::word::SignatureSourceNameKey::from_original_native_word(native, rules, policy)?;
    Some((word, key))
}

impl SourceCommandBindings {
    /// Issue an original naming occurrence from the existing retained command
    /// layout inventory. A token selects only its complete canonical word;
    /// transformed, materialised or conflicting owners cannot supply one.
    #[must_use]
    pub fn original_source_name_at_span(
        &self,
        span: tcl_lexer::Span,
        config: tcl_lexer::LexerConfig,
        rules: tcl_syntax::word_rules::WordValueRules,
        policy: tcl_syntax::naming::NamePolicyProtocol,
    ) -> Option<crate::signature_scan::original_name::SourceOriginalNameOccurrence> {
        let origin = self.root_origin.as_ref()?;
        let mut found = None;
        // A selected word starts at `span`, so its site reaches that offset.
        for (site, observations) in self
            .root_layouts_spanning(span.start(), span.start())
            .into_iter()
            .rev()
        {
            if &site.source != origin || site.offset > span.start() {
                continue;
            }
            let Some(original) =
                super::declaration_layout::original_declaration_layouts(observations)
            else {
                continue;
            };
            let first = original.clone().next()?;
            if first.config != config || first.entry.source().origin != *origin {
                continue;
            }
            // A cheap rejection only; the shared native vector below owns the
            // exact complete-word and first-token correspondence.
            if !first
                .words
                .iter()
                .any(|word| word.source().span.start() == span.start())
            {
                continue;
            }
            let (_, input) = original_name_word_from_layout(
                site,
                origin.source_image(),
                &first.words,
                span,
                config,
                rules,
                policy,
            )?;
            let occurrence =
                crate::signature_scan::original_name::SourceOriginalNameOccurrence::new(
                    site, input,
                )?;
            if found
                .as_ref()
                .is_some_and(|previous| previous != &occurrence)
            {
                return None;
            }
            found = Some(occurrence);
        }
        found
    }
}

fn attach_unanimous_original_compiler_metadata(
    binding: &mut SourceInvocationBinding,
    first: &SourceCompilerInvocation,
    invocations: &[&SourceCompilerInvocation],
    reached: bool,
) {
    // Only compiler metadata is attached. Actual handler identities, contents,
    // evaluated argv and successful transfer remain absent/unknown.
    binding.native_compilation_admission = first.selection;
    binding
        .native_operand_layout
        .clone_from(&first.operand_layout);
    binding
        .native_compiler_admission
        .clone_from(&first.admitted);
    binding
        .native_structured_preparation
        .clone_from(&first.structured);
    binding.native_switch_preparation.clone_from(&first.switch);
    binding
        .original_compiler_words
        .clone_from(&first.original_words);
    if !reached {
        binding.lookup_namespace.clone_from(&first.namespace);
        binding
            .lookup_namespace_key
            .clone_from(&first.namespace_key);
        binding.lookup_word.clone_from(&first.head);
    }
    binding.compiler_lookup_state = Some(Arc::clone(&first.table));
    binding.compiler_policy = first.policy.clone().filter(|policy| {
        invocations
            .iter()
            .all(|visit| visit.policy.as_ref() == Some(policy))
    });
    for alternative in &invocations[1..] {
        if alternative.structured != first.structured {
            binding.native_structured_preparation = None;
        }
        if alternative.switch != first.switch {
            binding.native_switch_preparation = None;
        }
        if alternative.original_words != first.original_words {
            binding.original_compiler_words = None;
        }
        if alternative.operand_layout != first.operand_layout {
            binding.native_operand_layout = None;
        }
        if alternative.selection != first.selection {
            binding.native_compilation_admission =
                Some(tcl_registry::native_compilation::NativeCompilationSelection::Unknown);
        }
        if alternative.admitted != first.admitted {
            binding.native_compiler_admission = None;
        }
        if alternative.namespace_key != first.namespace_key || alternative.table != first.table {
            binding.compiler_lookup_state = None;
            binding.native_structured_preparation = None;
            binding.native_switch_preparation = None;
        }
    }
}

#[cfg(test)]
mod original_inline_control_source_tests;
