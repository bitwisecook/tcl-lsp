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

//! Preparing the advice tape of a source: its seed, the initial graph and the entry points.

use super::advice_graph::lookup_key;
use super::catalogue_seed::AdviceCatalogueSeedMemo;
use super::logical_body::OriginalLogicalProcedureBody;
use super::{
    AdviceCell, AdviceGraph, AdviceInvocationContext, AdviceNamingPolicy, AdviceTapeInputs,
    LogicalProcedureCatalogueRefusal, OriginalSourceCommandTransitionAdvice,
    OriginalSourceTransitionAdviceTape, SourceAdviceQuery, SourceCommandTransitionObligation,
    logical_navigation,
};
use crate::command_binding::SourceOriginLexical;
use std::sync::Arc;
use tcl_lexer::{LexerConfig, NativeWord};
use tcl_registry::model::ContextRegistry;

impl super::super::SourceCommandBindings {
    /// Ordinary selected Registry-schema arity debts in this exact source prefix.
    /// Receiver, registered and source-header alternatives retain independent
    /// owners. An empty list proves no complete invocation acceptance, reached
    /// lookup, successful call or Normal result.
    pub(crate) fn original_source_prefix_arity_debts(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        original: &[NativeWord],
    ) -> Option<Vec<Arc<OriginalSourceCommandTransitionAdvice>>> {
        if self.source_metadata_owner().source_analysis_input() != Some(input) {
            return None;
        }
        let origin = self.source_origin()?;
        if original.is_empty()
            || original.iter().any(|word| {
                word.image() != origin.source_image() || word.config() != input.lexer_config()
            })
            || !self.matches_original_source_image(origin.source_image(), input.lexer_config())
        {
            return None;
        }
        let tape = self.original_source_transition_advice_tape_for(
            input.borrowed_context_registry(),
            OriginalSourceCataloguePurpose::CompletePrefix,
        )?;
        let prefix = tape
            .arity_prefixes
            .get(&original[0].span().start())?
            .as_ref()?;
        if prefix.original.as_ref() != original
            || prefix.debts.iter().any(|debt| {
                !Arc::ptr_eq(&debt.site.source, origin)
                    || !debt.matches_source(origin.source_image(), input.lexer_config())
                    || !debt.matches_context(input.borrowed_context_registry())
                    || !debt
                        .obligations
                        .contains(&SourceCommandTransitionObligation::UnacceptedSourceArity)
            })
        {
            return None;
        }
        Some(prefix.debts.clone())
    }

    pub(crate) const fn original_source_lexer_config(&self) -> Option<LexerConfig> {
        self.lexer_config
    }

    pub(crate) fn original_logical_source_name_advice_input(
        &self,
    ) -> Option<&crate::analyser::ResolvedAnalysisInput> {
        self.final_state.logical_source_name_advice_input()
    }

    #[cfg(test)]
    pub(super) fn prepare_source_advice_tape(
        &self,
        context: &ContextRegistry,
    ) -> Option<AdviceTapeInputs<'_>> {
        let seed = self.original_source_transition_seed()?;
        let prepared = seed
            .prepare_source_advice_tape(context, OriginalSourceCataloguePurpose::CompleteProgram)?;
        Some(AdviceTapeInputs {
            origin: self.source_origin()?,
            config: prepared.config,
            policy: prepared.policy,
            dialect: prepared.dialect,
            plan: prepared.plan,
        })
    }

    #[cfg(test)]
    pub(super) fn initial_source_advice_graph(
        &self,
        context: &ContextRegistry,
        policy: &AdviceNamingPolicy,
    ) -> Option<AdviceGraph> {
        self.original_source_transition_seed()?
            .initial_source_advice_graph(context, policy)
    }

    pub(in super::super) fn original_source_transition_seed(
        &self,
    ) -> Option<OriginalSourceTransitionSeed> {
        // An unentered interpretation offers no advice to re-interpret from.
        if self.unentered {
            return None;
        }
        if self.final_state.baseline.source_point_entry.is_some()
            && self.original_inherited_source_entry().is_none()
        {
            return None;
        }
        let origin = self.source_origin()?;
        let config = self.lexer_config?;
        self.matches_original_source_image(origin.source_image(), config)
            .then_some(())?;
        Some(OriginalSourceTransitionSeed {
            analysis_owner: self.original_analysis_owner().cloned(),
            origin: Arc::clone(origin),
            config,
            baseline: Arc::clone(&self.final_state.baseline),
            logical_input: self.final_state.logical_source_name_advice_input().cloned(),
            stock_source_effects: self.original_jim_stock_source_effects(),
        })
    }

    /// Independently interpreted authored root-script transitions. This owner
    /// never mutates, publishes or borrows the retained runtime command state.
    pub(crate) fn original_logical_procedure_body_cards(
        &self,
    ) -> Option<Vec<Arc<OriginalLogicalProcedureBody>>> {
        let seed = self.original_source_transition_seed()?;
        if seed.baseline.native_entry.is_some() {
            return None;
        }
        let input = seed.logical_input.as_ref()?;
        input.has_logical_source_name_context().then_some(())?;
        let context = seed.source_context(input.borrowed_context_registry().commands())?;
        let tape = seed.prepare_for(&context, OriginalSourceCataloguePurpose::CompletePrefix)?;
        Some(
            tape.logical_procedure_bodies
                .into_values()
                .flatten()
                .collect(),
        )
    }

    /// Exact deferred Logical command scope for MAY body dependencies only.
    pub(crate) fn original_logical_summary_scope(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        original: &[NativeWord],
    ) -> Option<Arc<super::super::OriginalDeclaredLogicalBodyContext>> {
        if self.original_logical_source_name_advice_input() != Some(input)
            || self.source_metadata_owner().source_analysis_input() != Some(input)
        {
            return None;
        }
        let tape =
            self.original_source_transition_advice_tape(input.borrowed_context_registry())?;
        let record = tape
            .logical_summary_scopes
            .get(&original.first()?.span().start())?
            .as_ref()?;
        (record.original.as_ref() == original
            && record.scope.matches_input(input)
            && record.scope.owns_words(original))
        .then(|| Arc::clone(&record.scope))
    }

    pub(crate) fn original_source_transition_advice_tape(
        &self,
        context: &ContextRegistry,
    ) -> Option<OriginalSourceTransitionAdviceTape> {
        self.original_source_transition_seed()?.prepare(context)
    }

    pub(crate) fn original_normal_source_procedure_advice_tape(
        &self,
        context: &ContextRegistry,
    ) -> Option<OriginalSourceTransitionAdviceTape> {
        self.original_source_transition_seed()?
            .normal_source_outcome_advice(context, self)
            .map(|(_, tape)| tape)
    }

    pub(crate) fn original_source_transition_advice_tape_for(
        &self,
        context: &ContextRegistry,
        purpose: OriginalSourceCataloguePurpose,
    ) -> Option<OriginalSourceTransitionAdviceTape> {
        self.original_source_transition_seed()?
            .prepare_for(context, purpose)
    }

    /// [`Self::original_source_transition_advice_tape`], shared with every
    /// other realm and bindings copy that holds an equal seed.
    pub(crate) fn original_source_transition_advice_tape_shared(
        &self,
        context: &ContextRegistry,
    ) -> Arc<Option<OriginalSourceTransitionAdviceTape>> {
        self.original_source_transition_advice_tape_for_shared(
            context,
            OriginalSourceCataloguePurpose::CompleteProgram,
        )
    }

    /// [`Self::original_source_transition_advice_tape_for`], shared.
    pub(crate) fn original_source_transition_advice_tape_for_shared(
        &self,
        context: &ContextRegistry,
        purpose: OriginalSourceCataloguePurpose,
    ) -> Arc<Option<OriginalSourceTransitionAdviceTape>> {
        match self.original_source_transition_seed() {
            Some(seed) => seed.prepared_shared(context, purpose),
            None => Arc::new(None),
        }
    }
}

impl OriginalSourceTransitionSeed {
    /// A fresh tape for the realm caches. Not shared across realms: receipts
    /// derived from a tape are compared by identity, and a process-wide memo
    /// made those comparisons depend on what was analysed earlier.
    pub(in super::super) fn prepared_shared(
        &self,
        context: &ContextRegistry,
        purpose: OriginalSourceCataloguePurpose,
    ) -> Arc<Option<OriginalSourceTransitionAdviceTape>> {
        Arc::new(self.prepare_for(context, purpose))
    }
}

/// Immutable whole-source issuer shared by independently requested purposes.
/// The baseline retains its real metadata and entry; no body becomes a new entry.
#[derive(Debug, Clone)]
pub(in super::super) struct OriginalSourceTransitionSeed {
    // Only the Normal-source query uses this attachment. Source syntax reuse
    // keeps its existing four-field seed equality below.
    pub(in super::super) analysis_owner: Option<Arc<super::super::OriginalSourceAnalysisOwner>>,
    pub(in super::super) origin: Arc<tcl_names::origin::SourceOriginId>,
    pub(in super::super) config: LexerConfig,
    pub(in super::super) baseline: Arc<super::super::BindingBaseline>,
    pub(in super::super) logical_input: Option<crate::analyser::ResolvedAnalysisInput>,
    pub(in super::super) stock_source_effects:
        Vec<super::super::jim_stock_class::OriginalJimStockSourceEffects>,
}

impl PartialEq for OriginalSourceTransitionSeed {
    fn eq(&self, other: &Self) -> bool {
        self.origin == other.origin
            && self.config == other.config
            && self.baseline == other.baseline
            && self.logical_input == other.logical_input
            && self.stock_source_effects == other.stock_source_effects
    }
}

impl Eq for OriginalSourceTransitionSeed {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OriginalSourceCataloguePurpose {
    CompleteProgram,
    /// Authentic complete commands before the separately retained parse cut.
    CompletePrefix,
    /// Exact sealed region plan, without whole-image coverage or absence.
    ConditionalRegion,
}

impl OriginalSourceTransitionSeed {
    pub(in super::super) fn matches_registry(
        &self,
        registry: &tcl_registry::CommandRegistry,
    ) -> bool {
        self.baseline.registry_snapshot.as_ref() == Some(&registry.snapshot().semantic_key())
    }

    pub(in super::super) fn source_context(
        &self,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<Arc<ContextRegistry>> {
        if !self.matches_registry(registry) {
            return None;
        }
        match &self.baseline.metadata_context {
            crate::registry_invocation::OwnedInvocationMetadataContext::SuppliedSource(input) => {
                crate::registry_invocation::InvocationMetadataContext::for_source_input(
                    registry,
                    input,
                    self.config,
                    Some(input.unit_profile()),
                )?;
                Some(input.context_registry())
            }
            crate::registry_invocation::OwnedInvocationMetadataContext::Standalone => {
                let profile = registry.profile()?;
                // Explicit scalar compatibility selects static availability only.
                // The exact frozen store keeps every override and generation.
                Some(Arc::new(
                    tcl_registry::model::ingress::context_for_profile(profile)
                        .with_command_store(registry.snapshot().shared_registry()),
                ))
            }
            crate::registry_invocation::OwnedInvocationMetadataContext::Supplied(_)
            | crate::registry_invocation::OwnedInvocationMetadataContext::Unavailable => None,
        }
    }

    pub(super) fn prepare_source_advice_tape(
        &self,
        context: &ContextRegistry,
        purpose: OriginalSourceCataloguePurpose,
    ) -> Option<AdviceTapeInputs<'_>> {
        let origin = &self.origin;
        let config = self.config;
        let baseline = &self.baseline;
        let policy = if let Some(native) = baseline
            .execution_name_policy
            .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
        {
            AdviceNamingPolicy::Native(native)
        } else {
            AdviceNamingPolicy::Logical(self.logical_input.as_ref()?.clone())
        };
        let dialect = baseline.dialect?;
        if baseline.registry_snapshot.as_ref()
            != Some(&context.commands().snapshot().semantic_key())
        {
            return None;
        }
        if purpose == OriginalSourceCataloguePurpose::ConditionalRegion
            && baseline
                .source_point_entry
                .as_ref()
                .is_none_or(|entry| entry.conditional_region().is_none())
        {
            return None;
        }
        let plan = match baseline
            .source_point_entry
            .as_ref()
            .filter(|entry| entry.conditional_region().is_some())
        {
            Some(entry) => {
                // Region catalogues are positive prefix views only. They may
                // never certify a whole-file inventory or missing occurrence.
                if purpose == OriginalSourceCataloguePurpose::CompleteProgram {
                    return None;
                }
                entry.original_region_plan(origin, config)?
            }
            None => origin
                .original_script_words_in(
                    tcl_lexer::Span::new(
                        0,
                        u32::try_from(origin.source_image().bytes().len()).ok()?,
                    ),
                    config,
                    tcl_lexer::SourceSegmentationPurpose::WholeInput,
                )
                .ok()?,
        };
        if plan.fatal_tail.is_some() && purpose == OriginalSourceCataloguePurpose::CompleteProgram {
            return None;
        }
        Some(AdviceTapeInputs {
            origin,
            config,
            policy,
            dialect,
            plan,
        })
    }

    pub(super) fn initial_source_advice_graph(
        &self,
        context: &ContextRegistry,
        policy: &AdviceNamingPolicy,
    ) -> Option<AdviceGraph> {
        let baseline = &self.baseline;
        if let Some(entry) = &baseline.source_point_entry {
            return entry.source_advice_graph(context)?.child_graph(policy);
        }
        let dialect = baseline.dialect?;
        let seed = AdviceCatalogueSeedMemo::shared().get(context, policy, dialect)?;
        let mut graph = AdviceGraph::from_catalogue_seed(policy, dialect, &seed);
        if let Some(entry) = &baseline.native_entry {
            graph.retain_native_baseline(context, entry)?;
        }
        for command in baseline.declared_commands.keys() {
            if let Some(key) = lookup_key(policy, command.as_bytes()) {
                let surface = baseline
                    .declared_surface
                    .as_ref()?
                    .document_surface(context.commands());
                let name = baseline.declared_commands.get(command)?;
                let declaration = surface.declared_command(name)?;
                graph
                    .cells
                    .insert(key, AdviceCell::Declared(Arc::new(declaration.clone())));
            }
        }
        Some(graph)
    }

    pub(in super::super) fn prepare(
        &self,
        context: &ContextRegistry,
    ) -> Option<OriginalSourceTransitionAdviceTape> {
        self.prepare_for(context, OriginalSourceCataloguePurpose::CompleteProgram)
    }

    pub(in super::super) fn prepare_for(
        &self,
        context: &ContextRegistry,
        purpose: OriginalSourceCataloguePurpose,
    ) -> Option<OriginalSourceTransitionAdviceTape> {
        self.prepare_for_query(context, purpose, None)
    }

    pub(super) fn prepare_for_query(
        &self,
        context: &ContextRegistry,
        purpose: OriginalSourceCataloguePurpose,
        query: Option<SourceAdviceQuery<'_>>,
    ) -> Option<OriginalSourceTransitionAdviceTape> {
        let AdviceTapeInputs {
            origin,
            config,
            policy,
            dialect,
            plan,
        } = self.prepare_source_advice_tape(context, purpose)?;
        let mut graph = self.initial_source_advice_graph(context, &policy)?;
        graph.stock_source_effects = self
            .stock_source_effects
            .iter()
            .filter(|effect| {
                effect.matches_seed(self.analysis_owner.as_ref(), origin, &self.baseline)
                    && plan
                        .commands
                        .iter()
                        .any(|command| command.words == effect.original())
            })
            .cloned()
            .collect();
        let mut tape = OriginalSourceTransitionAdviceTape::default();
        let partial_tail = plan.fatal_tail;
        if matches!(policy, AdviceNamingPolicy::Logical(_))
            && let Some(tail) = plan.fatal_tail
        {
            // CompletePrefix retains positive source cards, but its malformed
            // remainder cannot certify a whole-document catalogue or emptiness.
            tape.logical_procedure_catalogue_refusals.insert(
                LogicalProcedureCatalogueRefusal::RootLexicalTailUnavailable(tail.command_start),
            );
        }
        let advice_context = AdviceInvocationContext {
            origin,
            context,
            config,
            dialect,
            policy: &policy,
            query,
        };
        advice_context.observe_publication_proposal(&graph);
        for invocation in plan.commands {
            // A cooperative stop point only: a cancelled host read unwinds here.
            crate::analysis_control::unwind_point(
                crate::analysis_control::AnalysisCheckpoint::Body,
            );
            if let AdviceNamingPolicy::Logical(input) = &policy
                && self.baseline.metadata_context.source_analysis_input() == Some(input)
                && self.baseline.logical_source_input.as_ref() == Some(input)
                && self.baseline.native_entry.is_none()
                && self.baseline.hosted_execution_context.is_none()
                && self.baseline.vendor_source_input.is_none()
                && !self.baseline.unknown_entry
            {
                // Only the whole source seed's original root command traversal
                // supplies this conditional Document recipe. Nested bodies keep
                // their independent source scope, and no runtime frame follows.
                tape.retain_logical_document_navigation(&graph, &invocation.words, origin);
            }
            advice_context.retain_original_invocation(
                &mut graph,
                &mut tape,
                &invocation,
                logical_navigation::LogicalCallbackScope::Document,
            )?;
        }
        if purpose == OriginalSourceCataloguePurpose::CompletePrefix
            && self.baseline.source_point_entry.is_none()
            && self.baseline.hosted_execution_context.is_none()
            && self.baseline.vendor_source_input.is_none()
            && !self.baseline.unknown_entry
            && let Some(tail) = partial_tail
        {
            tape.partial_expression_span = advice_context.partial_expression_span(&graph, tail);
        }
        advice_context.finish_normal_source_graph(&graph);
        advice_context.retain_future_original_procedure_bodies(&mut tape, &graph);
        advice_context.retain_final_original_procedure_publications(&mut tape, &graph);
        advice_context.retain_final_original_class_publications(&mut tape, &graph);
        advice_context.retain_final_class_display_publications(&mut tape, &graph);
        Some(tape)
    }
}
