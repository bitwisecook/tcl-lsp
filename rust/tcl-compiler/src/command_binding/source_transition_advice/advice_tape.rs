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

//! The advice tape: lookups, Logical original commands and the inventories it extends.

use super::interpreter_handle::OriginalSourceInterpreterHandleBody;
use super::interpreter_path::OriginalSourceInterpreterPathBinding;
use super::logical_body::OriginalLogicalProcedureBody;
use super::source_procedure::OriginalSourceProcedureCall;
use super::{
    AdviceGraph, OriginalLogicalSummaryScope, OriginalSourceArityPrefix,
    OriginalSourceCommandTransition, OriginalSourceCommandTransitionAdvice,
    OriginalSourceTransitionAdviceTape, source_class,
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use tcl_lexer::{NativeWord, SourceImage};

/// A represented refusal is authoritative for this conditional source purpose.
/// Only an unrepresented site can consult a separate declaration-source owner.
#[derive(Debug, Clone, Copy)]
pub(crate) enum OriginalSourceTransitionAdviceLookup<'a> {
    Advice(&'a OriginalSourceCommandTransitionAdvice),
    Refused,
    Unrepresented,
}

impl OriginalSourceTransitionAdviceTape {
    pub(crate) fn native_procedure_calls(
        &self,
    ) -> impl Iterator<Item = &OriginalSourceProcedureCall> {
        self.native_procedure_calls
            .values()
            .filter_map(Option::as_ref)
    }

    pub(crate) fn native_procedure_call_at(
        &self,
        offset: u32,
    ) -> Option<&OriginalSourceProcedureCall> {
        self.native_procedure_calls.get(&offset)?.as_ref()
    }

    pub(crate) fn lookup(&self, offset: u32) -> OriginalSourceTransitionAdviceLookup<'_> {
        if let Some(advice) = self.advice.get(&offset) {
            OriginalSourceTransitionAdviceLookup::Advice(advice)
        } else if self.represented.contains(&offset) {
            OriginalSourceTransitionAdviceLookup::Refused
        } else {
            OriginalSourceTransitionAdviceLookup::Unrepresented
        }
    }

    pub(crate) fn prefix_catalogue_advice(
        &self,
        offset: u32,
    ) -> Option<&OriginalSourceCommandTransitionAdvice> {
        if self.blocks_registry_source(offset) {
            return None;
        }
        if let Some(advice) = self.body_catalogue(offset) {
            return Some(advice);
        }
        match self.lookup(offset) {
            OriginalSourceTransitionAdviceLookup::Advice(advice) => Some(advice),
            OriginalSourceTransitionAdviceLookup::Refused
            | OriginalSourceTransitionAdviceLookup::Unrepresented => None,
        }
    }

    pub(crate) fn logical_procedure_body_cards(&self) -> Vec<Arc<OriginalLogicalProcedureBody>> {
        self.logical_procedure_bodies
            .values()
            .flatten()
            .cloned()
            .collect()
    }

    /// Represented source ensemble operations in original order. Each receipt
    /// keeps its complete argv, Registry transition recipe, input ordinals, and
    /// alias/move lineage. This is a positive receipt list, not a complete
    /// census: empty or partial results prove neither absence nor current state.
    pub(crate) fn logical_ensemble_transition_receipts(
        &self,
    ) -> Option<Vec<Arc<OriginalSourceCommandTransition>>> {
        self.logical_ensemble_transition_receipts
            .values()
            .cloned()
            .collect()
    }

    pub(crate) fn checked_logical_procedure_body_cards(
        &self,
    ) -> Option<Vec<Arc<OriginalLogicalProcedureBody>>> {
        if !self.logical_procedure_catalogue_refusals.is_empty() {
            return None;
        }
        self.logical_procedure_bodies.values().cloned().collect()
    }

    /// Genuine original receiver-body vectors, without receiver lookup or entry.
    /// Conflicting source receipts withdraw this catalogue as a whole.
    pub(in super::super) fn logical_receiver_source_commands(
        &self,
        origin: &Arc<super::super::SourceOriginId>,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Option<
        Vec<(
            Arc<crate::registry_invocation::OriginalSourceDefinitionMemberScriptBody>,
            Arc<crate::registry_invocation::OriginalSourceCommandProjection>,
        )>,
    > {
        input.has_logical_source_name_context().then_some(())?;
        let mut commands = Vec::new();
        for (&offset, receipt) in &self.logical_body_words {
            let receipt = receipt.as_ref()?;
            receipt.original_words(origin, input)?;
            let Some(body) = receipt.original_member_body(origin, input) else {
                continue;
            };
            let original = self.logical_original_command_at(origin, input, offset)?;
            commands.push((body, original));
        }
        Some(commands)
    }

    /// Whole original syntax from independent Logical body/header owners.
    /// The selected command/schema may be unavailable; this grants geometry only.
    pub(crate) fn logical_original_command_at(
        &self,
        origin: &Arc<super::super::SourceOriginId>,
        input: &crate::analyser::ResolvedAnalysisInput,
        offset: u32,
    ) -> Option<Arc<crate::registry_invocation::OriginalSourceCommandProjection>> {
        input.has_logical_source_name_context().then_some(())?;
        let image = origin.source_image();
        let config = input.lexer_config();
        let mut selected: Option<&[NativeWord]> = None;
        if let Some(receipt) = self.logical_body_words.get(&offset) {
            selected = Some(receipt.as_ref()?.original_words(origin, input)?);
        }
        for advice in [
            self.advice.get(&offset),
            self.body_catalogue.get(&offset).and_then(Option::as_ref),
            self.member_catalogue.get(&offset).and_then(Option::as_ref),
        ]
        .into_iter()
        .flatten()
        {
            if !Arc::ptr_eq(&advice.site().source, origin)
                || advice.site().offset != offset
                || advice.logical_source_input() != Some(input)
                || !advice.matches_source(image, config)
                || !advice.matches_context(input.borrowed_context_registry())
            {
                return None;
            }
            if selected.is_some_and(|previous| previous != advice.original_words()) {
                return None;
            }
            selected = Some(advice.original_words());
        }
        let original = selected?;
        if original.first()?.span().start() != offset
            || original
                .iter()
                .any(|word| word.image() != image || word.config() != config)
        {
            return None;
        }
        // Reuse the existing complete-vector projection owner. Original body
        // extents select parser input; reporting strings select nothing.
        let region = tcl_lexer::Span::new(offset, original.last()?.span().end());
        let body = SourceImage::from_bytes(image.bytes().get(region.as_range())?, image.channel());
        let segments = crate::segmenter::segment_commands_image_with_offset_and_config(
            &body,
            offset,
            config.nested(),
        )?;
        let [command] = segments.as_slice() else {
            return None;
        };
        if command.is_partial {
            return None;
        }
        let tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::from_image(image),
            config,
            command,
        );
        let projection = origin.original_lexical_command(tokens.words(), offset, config)?;
        (projection.native_words() == original).then_some(projection)
    }

    pub(crate) fn logical_original_word_observations_at_span(
        &self,
        origin: &Arc<super::super::SourceOriginId>,
        input: &crate::analyser::ResolvedAnalysisInput,
        span: tcl_lexer::Span,
    ) -> Option<Vec<(NativeWord, usize, super::super::CommandAllocationSite)>> {
        input.has_logical_source_name_context().then_some(())?;
        if self.logical_body_words.values().any(Option::is_none) {
            return None;
        }
        let offsets = self
            .logical_body_words
            .keys()
            .chain(self.advice.keys())
            .chain(self.body_catalogue.keys())
            .chain(self.member_catalogue.keys())
            .copied()
            .filter(|offset| *offset <= span.start())
            .collect::<BTreeSet<_>>();
        let mut observations = Vec::new();
        for offset in offsets {
            let Some(projection) = self.logical_original_command_at(origin, input, offset) else {
                continue;
            };
            let mut words = projection
                .native_words()
                .iter()
                .enumerate()
                .filter(|(_, word)| {
                    word.span() == span
                        || word.content_span().ok() == Some(span)
                        || word
                            .tokens()
                            .first()
                            .is_some_and(|token| token.span == span)
                });
            let Some((written, word)) = words.next() else {
                continue;
            };
            if words.next().is_some() {
                return None;
            }
            observations.push((
                word.clone(),
                written,
                super::super::CommandAllocationSite {
                    source: Arc::clone(origin),
                    offset,
                },
            ));
        }
        Some(observations)
    }

    /// Exact Logical catalogue words retain geometry without an entered invocation.
    pub(crate) fn logical_original_word_at_span(
        &self,
        origin: &Arc<super::super::SourceOriginId>,
        input: &crate::analyser::ResolvedAnalysisInput,
        span: tcl_lexer::Span,
    ) -> Option<(NativeWord, u32)> {
        input.has_logical_source_name_context().then_some(())?;
        let image = origin.source_image();
        let config = input.lexer_config();
        let context = input.borrowed_context_registry();
        let offsets = self
            .advice
            .keys()
            .chain(self.body_catalogue.keys())
            .copied()
            .collect::<BTreeSet<_>>();
        if self.logical_body_words.values().any(Option::is_none) {
            return None;
        }
        let mut selected = self.logical_body_original_word_at_span(origin, input, span);
        // Declared Logical bodies retain whole vectors independently of a
        // selected Registry handler or ordinary invocation.
        for (&offset, receipt) in self.logical_variable_words.range(..=span.start()) {
            let Some(receipt) = receipt else {
                continue;
            };
            let Some(current) = receipt.original_word_at_span(image, input, span) else {
                continue;
            };
            if current.1 != offset
                || selected
                    .as_ref()
                    .is_some_and(|previous| previous != &current)
            {
                return None;
            }
            selected = Some(current);
        }
        for offset in offsets {
            if self.blocks_registry_source(offset) {
                continue;
            }
            let advice = if let Some(advice) = self.body_catalogue(offset) {
                advice
            } else {
                match self.lookup(offset) {
                    OriginalSourceTransitionAdviceLookup::Advice(advice) => advice,
                    OriginalSourceTransitionAdviceLookup::Refused
                    | OriginalSourceTransitionAdviceLookup::Unrepresented => continue,
                }
            };
            if advice.site().offset != offset
                || advice.original_words().first()?.span().start() != offset
                || advice.site().source != *origin
                || advice.logical_source_input() != Some(input)
                || !advice.matches_source(image, config)
                || !advice.matches_context(context)
            {
                continue;
            }
            let mut words = advice.original_words().iter().filter(|word| {
                word.span() == span
                    || word
                        .tokens()
                        .first()
                        .is_some_and(|token| token.span == span)
            });
            let Some(word) = words.next() else {
                continue;
            };
            if words.next().is_some() {
                return None;
            }
            if let OriginalSourceTransitionAdviceLookup::Advice(original) = self.lookup(offset)
                && original.original_words() != advice.original_words()
            {
                return None;
            }
            let current = (word.clone(), offset);
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

    pub(crate) fn declared_selection(
        &self,
        offset: u32,
    ) -> Option<&super::super::DeclaredSourceSelection> {
        self.declared.get(&offset)?.as_ref()
    }

    pub(super) fn retain_declared(&mut self, selection: super::super::DeclaredSourceSelection) {
        let offset = selection.site().offset;
        self.declared
            .entry(offset)
            .and_modify(|previous| {
                if previous.as_ref() != Some(&selection) {
                    *previous = None;
                }
            })
            .or_insert(Some(selection));
    }

    pub(crate) fn interpreter_body(
        &self,
        offset: u32,
    ) -> Option<&OriginalSourceInterpreterHandleBody> {
        self.interpreter_bodies.get(&offset)
    }

    pub(crate) fn interpreter_paths(
        &self,
        offset: u32,
    ) -> Option<&[OriginalSourceInterpreterPathBinding]> {
        self.interpreter_paths.get(&offset).map(Vec::as_slice)
    }

    pub(crate) fn blocks_registry_source(&self, offset: u32) -> bool {
        self.registry_barriers.contains(&offset)
    }

    pub(crate) fn member_catalogue(
        &self,
        offset: u32,
    ) -> Option<&OriginalSourceCommandTransitionAdvice> {
        self.member_catalogue.get(&offset)?.as_ref()
    }

    pub(crate) fn original_definition_member_source_catalogue(
        &self,
    ) -> impl Iterator<Item = &OriginalSourceCommandTransitionAdvice> {
        self.member_catalogue.values().filter_map(Option::as_ref)
    }

    pub(crate) fn body_catalogue(
        &self,
        offset: u32,
    ) -> Option<&OriginalSourceCommandTransitionAdvice> {
        self.body_catalogue.get(&offset)?.as_ref()
    }

    pub(crate) fn conditional_case_syntax(
        &self,
        offset: u32,
    ) -> Option<&OriginalSourceCommandTransitionAdvice> {
        self.conditional_case_syntax.get(&offset)?.as_ref()
    }

    pub(in super::super) fn retain_logical_summary_scope(
        &mut self,
        original: &[NativeWord],
        scope: &Arc<super::super::OriginalDeclaredLogicalBodyContext>,
    ) {
        let Some(head) = original.first() else {
            return;
        };
        source_class::merge_receipt(
            &mut self.logical_summary_scopes,
            head.span().start(),
            Some(OriginalLogicalSummaryScope {
                original: Arc::from(original),
                scope: Arc::clone(scope),
            }),
        );
    }

    pub(super) fn retain_arity_prefix(&mut self, graph: &AdviceGraph, original: &[NativeWord]) {
        let Some(head) = original.first() else {
            return;
        };
        source_class::merge_receipt(
            &mut self.arity_prefixes,
            head.span().start(),
            Some(OriginalSourceArityPrefix {
                original: Arc::from(original),
                debts: graph.arity_debts.clone(),
            }),
        );
    }

    fn extend_prefix_inventory(&mut self, other: &mut Self) {
        merge_receipt_inventory(
            &mut self.conditional_procedure_declarations,
            std::mem::take(&mut other.conditional_procedure_declarations),
        );
        merge_receipt_inventory(
            &mut self.decoded_lambda_phases,
            std::mem::take(&mut other.decoded_lambda_phases),
        );
        merge_receipt_inventory(
            &mut self.native_procedure_calls,
            std::mem::take(&mut other.native_procedure_calls),
        );
        merge_receipt_inventory(
            &mut self.logical_procedure_calls,
            std::mem::take(&mut other.logical_procedure_calls),
        );
        merge_receipt_inventory(
            &mut self.logical_name_references,
            std::mem::take(&mut other.logical_name_references),
        );
        merge_receipt_inventory(
            &mut self.logical_ensemble_transition_receipts,
            std::mem::take(&mut other.logical_ensemble_transition_receipts),
        );
        merge_receipt_inventory(
            &mut self.arity_prefixes,
            std::mem::take(&mut other.arity_prefixes),
        );
        self.logical_procedure_catalogue_refusals
            .extend(std::mem::take(
                &mut other.logical_procedure_catalogue_refusals,
            ));
        merge_receipt_inventory(
            &mut self.native_procedure_may_commands,
            std::mem::take(&mut other.native_procedure_may_commands),
        );
        merge_receipt_inventory(
            &mut self.native_deferred_autoload_prefixes,
            std::mem::take(&mut other.native_deferred_autoload_prefixes),
        );
        merge_receipt_inventory(
            &mut self.native_deferred_autoload_horizons,
            std::mem::take(&mut other.native_deferred_autoload_horizons),
        );
        merge_receipt_inventory(
            &mut self.logical_callbacks,
            std::mem::take(&mut other.logical_callbacks),
        );
        merge_receipt_inventory(
            &mut self.logical_variable_words,
            std::mem::take(&mut other.logical_variable_words),
        );
        merge_receipt_inventory(
            &mut self.logical_math,
            std::mem::take(&mut other.logical_math),
        );
    }

    pub(super) fn extend_inventory(&mut self, mut other: Self) {
        self.extend_prefix_inventory(&mut other);
        for (offset, candidate) in other.logical_navigation {
            self.merge_logical_navigation(offset, candidate);
        }
        merge_receipt_inventory(
            &mut self.logical_absolute_navigation,
            other.logical_absolute_navigation,
        );
        merge_receipt_inventory(&mut self.logical_body_words, other.logical_body_words);
        merge_receipt_inventory(
            &mut self.logical_summary_scopes,
            other.logical_summary_scopes,
        );
        merge_receipt_inventory(
            &mut self.logical_procedure_bodies,
            other.logical_procedure_bodies,
        );
        merge_receipt_inventory(
            &mut self.logical_variable_scopes,
            other.logical_variable_scopes,
        );
        merge_receipt_inventory(
            &mut self.conditional_case_syntax,
            other.conditional_case_syntax,
        );
        merge_receipt_inventory(&mut self.body_catalogue, other.body_catalogue);
        merge_receipt_inventory(&mut self.member_catalogue, other.member_catalogue);
        self.scanned_procedure_declarations
            .extend(other.scanned_procedure_declarations);
        merge_receipt_inventory(&mut self.class_instance_calls, other.class_instance_calls);
        for (offset, targets) in other.callback_targets {
            for (prefix, target) in targets {
                self.merge_callback_target(offset, prefix, target);
            }
        }
        for (offset, references) in other.authored_jim_procedure_members {
            for (input, recipe) in references {
                self.merge_authored_jim_procedure_member(offset, input, recipe);
            }
        }
        for (offset, references) in other.class_references {
            for (input, receipt) in references {
                self.merge_class_reference(offset, input, receipt);
            }
        }
        merge_receipt_inventory(&mut self.configured_classes, other.configured_classes);
        self.advice.extend(other.advice);
        self.represented.extend(other.represented);
        self.registry_barriers.extend(other.registry_barriers);
        self.interpreter_bodies.extend(other.interpreter_bodies);
        self.interpreter_paths.extend(other.interpreter_paths);
        #[cfg(debug_assertions)]
        self.trace_authored_jim_inventory(&other.authored_jim_classes);
        merge_receipt_inventory(&mut self.authored_jim_classes, other.authored_jim_classes);
        merge_receipt_inventory(&mut self.authored_jim_calls, other.authored_jim_calls);
        self.extend_class_inventory(other.class_declarations, other.constructor_calls);
        self.extend_class_proc_calls(other.class_proc_calls);
        for (_, facets) in other.class_declaration_facets {
            for facet in facets {
                self.retain_class_declaration_facet(facet);
            }
        }
        merge_receipt_inventory(&mut self.registered_factories, other.registered_factories);
        for (offset, receipt) in other.registered_instances {
            self.registered_instances
                .entry(offset)
                .and_modify(|previous| {
                    if previous != &receipt {
                        *previous = None;
                    }
                })
                .or_insert(receipt);
        }
        for (offset, receipt) in other.registered_handles {
            self.registered_handles
                .entry(offset)
                .and_modify(|previous| {
                    if previous != &receipt {
                        *previous = None;
                    }
                })
                .or_insert(receipt);
        }
        for (offset, prefix) in other.authored_prefixes {
            self.merge_authored_prefix(offset, prefix);
        }
        for (offset, prefix) in other.produced_prefixes {
            self.merge_produced_prefix(offset, prefix);
        }
        for (offset, selection) in other.declared {
            if let Some(selection) = selection {
                self.retain_declared(selection);
            } else {
                self.declared.insert(offset, None);
            }
        }
    }
}

fn merge_receipt_inventory<K: Ord, T: PartialEq>(
    entries: &mut BTreeMap<K, Option<T>>,
    other: BTreeMap<K, Option<T>>,
) {
    for (offset, receipt) in other {
        source_class::merge_receipt(entries, offset, receipt);
    }
}
