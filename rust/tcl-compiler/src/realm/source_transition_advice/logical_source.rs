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

//! Advice over the Logical source model: variable scopes, procedure calls and navigation.

use crate::command_binding::OriginalSourceTransitionAdviceTape;
use std::sync::Arc;

impl super::super::CommandBindingRealm {
    pub(crate) fn original_conditional_lambda_formatting_body(
        &self,
        word: &tcl_lexer::NativeWord,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Option<Arc<crate::command_binding::OriginalConditionalLambdaFormattingBody>> {
        if !input.has_logical_source_name_context()
            || !self.matches_resolved_analysis_input(input)
            || !self.matches_original_source_image(word.image(), input.lexer_config())
            || self.bindings.original_logical_source_name_advice_input() != Some(input)
        {
            return None;
        }
        let context = input.borrowed_context_registry();
        let prepared = self.prepared_prefix_tape(context);
        crate::command_binding::OriginalSourceTransitionAdviceTape::decoded_lambda_formatting_phase(
            prepared.as_ref().as_ref()?,
            word,
            input,
        )
    }

    pub(crate) fn original_logical_variable_scope_at(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        image: &tcl_lexer::SourceImage,
        span: tcl_lexer::Span,
        name: &crate::signature_scan::scope::SignatureSourceNameInput,
    ) -> Option<crate::command_binding::OriginalLogicalVariableScope> {
        if !input.has_logical_source_name_context()
            || self.bindings.original_logical_source_name_advice_input() != Some(input)
            || !self.matches_resolved_analysis_input(input)
            || !self.matches_original_source_image(image, input.lexer_config())
        {
            return None;
        }
        let context = input.borrowed_context_registry();
        let prepared = self.prepared_prefix_tape(context);
        prepared
            .as_ref()
            .as_ref()?
            .logical_variable_scope_at(input, image, span, name)
    }

    pub(crate) fn original_logical_variable_formal_occurrences(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Vec<crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence> {
        use crate::command_binding::OriginalLogicalVariableScope;
        use crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence;
        if !input.has_logical_source_name_context()
            || self.bindings.original_logical_source_name_advice_input() != Some(input)
            || !self.matches_resolved_analysis_input(input)
        {
            return Vec::new();
        }
        let Some(image) = self.original_source_image() else {
            return Vec::new();
        };
        if !self.matches_original_source_image(image, input.lexer_config()) {
            return Vec::new();
        }
        let mut result = Vec::new();
        for card in self
            .original_logical_procedure_body_cards(input)
            .into_iter()
            .flatten()
        {
            let Some(fields) = card
                .formal_topology()
                .and_then(super::super::super::command_binding::formal_topology::OriginalFormalTopology::original_name_fields)
            else {
                continue;
            };
            for field in fields {
                let Some(scope) = OriginalLogicalVariableScope::formal(
                    input,
                    Arc::clone(&card),
                    field.span,
                    &field.input,
                ) else {
                    continue;
                };
                if let Some(occurrence) =
                    SignatureSourceVariableOccurrence::from_logical_formal_input(
                        field.span,
                        field.input,
                        field.name,
                        field.recipe,
                        scope,
                    )
                {
                    result.push(occurrence);
                }
            }
        }
        let context = input.borrowed_context_registry();
        let prepared = self.prepared_prefix_tape(context);
        if let Some(tape) = prepared.as_ref().as_ref() {
            result.extend(tape.logical_lambda_formal_occurrences(input, image));
        }
        result
    }

    pub(crate) fn original_logical_variable_frame_at_offset(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        image: &tcl_lexer::SourceImage,
        offset: u32,
    ) -> Option<crate::command_binding::SourceOriginalVariableFrame> {
        if !input.has_logical_source_name_context()
            || self.bindings.original_logical_source_name_advice_input() != Some(input)
            || !self.matches_resolved_analysis_input(input)
            || !self.matches_original_source_image(image, input.lexer_config())
        {
            return None;
        }
        let context = input.borrowed_context_registry();
        let prepared = self.prepared_prefix_tape(context);
        prepared
            .as_ref()
            .as_ref()?
            .logical_variable_frame_at_offset(input, image, offset)
    }

    pub(crate) fn original_logical_variable_formal_name_is_unrepresented(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        symbol: &crate::signature_scan::variable_symbol::SignatureSourceVariableSymbol,
    ) -> bool {
        use crate::signature_scan::variable_symbol::SignatureSourceVariableSlot;
        let SignatureSourceVariableSlot::Local { frame, simple } = symbol.slot() else {
            return false;
        };
        let Some(cards) = self.original_logical_procedure_body_cards(input) else {
            return true;
        };
        let represented = self.original_logical_variable_formal_occurrences(input);
        let mut found = false;
        for card in cards {
            if crate::command_binding::SourceOriginalVariableFrame::from_logical_procedure_source(
                &card, input,
            )
            .as_ref()
                != Some(frame)
            {
                continue;
            }
            found = true;
            let Some(topology) = card.formal_topology() else {
                return true;
            };
            if topology
                .parameters()
                .iter()
                .any(|parameter| parameter.name.as_slice() == simple.as_bytes())
                && !represented
                    .iter()
                    .any(|occurrence| occurrence.symbol() == symbol)
            {
                return true;
            }
        }
        !found
    }

    /// Reborrow one exact Logical source occurrence from the immutable syntax catalogue.
    /// The retained candidate coordinates cannot supply an installed target or call edge.
    pub(crate) fn original_logical_math_function_at(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        offset: u32,
    ) -> Option<crate::command_binding::OriginalLogicalMathFunctionOccurrence> {
        let image = self.original_source_image()?;
        if !input.has_logical_source_name_context()
            || self.bindings.original_logical_source_name_advice_input() != Some(input)
            || !self.matches_resolved_analysis_input(input)
            || !self.matches_original_source_image(image, input.lexer_config())
        {
            return None;
        }
        let context = input.borrowed_context_registry();
        let prepared = self.prepared_prefix_tape(context);
        OriginalSourceTransitionAdviceTape::logical_math_at(
            prepared.as_ref().as_ref()?,
            input,
            image,
            offset,
        )
    }

    pub(crate) fn original_logical_procedure_calls(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Option<Vec<crate::command_binding::OriginalLogicalProcedureCall>> {
        let image = self.original_source_image()?;
        if !input.has_logical_source_name_context()
            || self.bindings.original_logical_source_name_advice_input() != Some(input)
            || !self.matches_resolved_analysis_input(input)
            || !self.matches_original_source_image(image, input.lexer_config())
        {
            return None;
        }
        let context = input.borrowed_context_registry();
        let prepared = self.prepared_prefix_tape(context);
        Some(
            prepared
                .as_ref()
                .as_ref()?
                .logical_procedure_call_inventory(input, image),
        )
    }

    /// Positive represented Logical ensemble transition receipts. Complete
    /// source parsing and input ownership are checked, but unrepresented
    /// mutations may still exist; an empty list does not prove absence.
    pub(crate) fn original_logical_ensemble_transition_receipts(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Option<Vec<crate::command_binding::OriginalSourceCommandTransition>> {
        let image = self.original_source_image()?;
        if !input.has_logical_source_name_context()
            || self.bindings.original_logical_source_name_advice_input() != Some(input)
            || !self.matches_resolved_analysis_input(input)
            || !self.matches_original_source_image(image, input.lexer_config())
        {
            return None;
        }
        let context = input.borrowed_context_registry();
        let purpose = crate::command_binding::OriginalSourceCataloguePurpose::CompleteProgram;
        let prepared = self
            .source_advice
            .get_or_prepare_shared_for(context, purpose, || {
                self.bindings
                    .original_source_transition_advice_tape_for_shared(context, purpose)
            });
        Some(
            prepared
                .as_ref()
                .as_ref()?
                .logical_ensemble_transition_receipts()?
                .into_iter()
                .map(|operation| (*operation).clone())
                .collect(),
        )
    }

    pub(crate) fn original_logical_procedure_call_at(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        offset: u32,
    ) -> Option<crate::command_binding::OriginalLogicalProcedureCall> {
        let image = self.original_source_image()?;
        if !input.has_logical_source_name_context()
            || self.bindings.original_logical_source_name_advice_input() != Some(input)
            || !self.matches_resolved_analysis_input(input)
            || !self.matches_original_source_image(image, input.lexer_config())
        {
            return None;
        }
        let context = input.borrowed_context_registry();
        let prepared = self.prepared_prefix_tape(context);
        OriginalSourceTransitionAdviceTape::logical_procedure_call_at(
            prepared.as_ref().as_ref()?,
            input,
            image,
            offset,
        )
    }

    pub(crate) fn original_logical_navigation_candidates(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        offset: u32,
    ) -> Option<crate::command_binding::OriginalLogicalCommandCandidates> {
        let image = self.original_source_image()?;
        if !input.has_logical_source_name_context()
            || self.bindings.original_logical_source_name_advice_input() != Some(input)
            || !self.matches_resolved_analysis_input(input)
            || !self.matches_original_source_image(image, input.lexer_config())
        {
            return None;
        }
        let context = input.borrowed_context_registry();
        let prepared = self.prepared_prefix_tape(context);
        prepared
            .as_ref()
            .as_ref()?
            .logical_navigation_at(input, image, offset)
    }

    /// Reborrow represented original Logical occurrences from the current tape.
    /// This partial inventory supplies no command census or absence proof.
    pub(crate) fn original_logical_navigation_inventory(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Option<Vec<crate::command_binding::OriginalLogicalCommandCandidates>> {
        let image = self.original_source_image()?;
        if !input.has_logical_source_name_context()
            || self.bindings.original_logical_source_name_advice_input() != Some(input)
            || !self.matches_resolved_analysis_input(input)
            || !self.matches_original_source_image(image, input.lexer_config())
        {
            return None;
        }
        let context = input.borrowed_context_registry();
        let prepared = self.prepared_prefix_tape(context);
        Some(
            prepared
                .as_ref()
                .as_ref()?
                .logical_navigation_inventory(input, image),
        )
    }

    /// Absolute source syntax from a genuine original body, independently of
    /// the body frame's unavailable namespace recipe or reached handler.
    pub(crate) fn original_logical_absolute_navigation_candidate(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        offset: u32,
    ) -> Option<crate::command_binding::OriginalLogicalAbsoluteCommandCandidate> {
        let image = self.original_source_image()?;
        if !input.has_logical_source_name_context()
            || self.bindings.original_logical_source_name_advice_input() != Some(input)
            || !self.matches_resolved_analysis_input(input)
            || !self.matches_original_source_image(image, input.lexer_config())
        {
            return None;
        }
        let context = input.borrowed_context_registry();
        let prepared = self.prepared_prefix_tape(context);
        prepared
            .as_ref()
            .as_ref()?
            .logical_absolute_navigation_at(input, image, offset)
    }

    /// Reborrow actual Logical callback roles at one original installer from the immutable syntax catalogue.
    /// The retained callback candidate coordinates cannot supply an installed target or call edge.
    pub(crate) fn original_logical_callback_candidates(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        offset: u32,
    ) -> Option<Vec<crate::command_binding::OriginalLogicalCallbackCandidates>> {
        let image = self.original_source_image()?;
        if !input.has_logical_source_name_context()
            || self.bindings.original_logical_source_name_advice_input() != Some(input)
            || !self.matches_resolved_analysis_input(input)
            || !self.matches_original_source_image(image, input.lexer_config())
        {
            return None;
        }
        let context = input.borrowed_context_registry();
        let prepared = self.prepared_prefix_tape(context);
        prepared
            .as_ref()
            .as_ref()?
            .logical_callbacks_at(input, image, offset)
    }

    /// Reborrow actual Logical callback roles at one original installer from the immutable syntax catalogue.
    /// The retained callback candidate coordinates cannot supply an installed target or call edge.
    pub(crate) fn original_logical_callback_inventory(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Option<Vec<crate::command_binding::OriginalLogicalCallbackCandidates>> {
        let image = self.original_source_image()?;
        if !input.has_logical_source_name_context()
            || self.bindings.original_logical_source_name_advice_input() != Some(input)
            || !self.matches_resolved_analysis_input(input)
            || !self.matches_original_source_image(image, input.lexer_config())
        {
            return None;
        }
        let context = input.borrowed_context_registry();
        let prepared = self.prepared_prefix_tape(context);
        Some(
            prepared
                .as_ref()
                .as_ref()?
                .logical_callback_inventory(input, image),
        )
    }

    /// Retained selected Logical name operands are source references, not calls.
    pub(crate) fn original_logical_name_reference_inventory(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Option<Vec<crate::command_binding::OriginalLogicalNameReference>> {
        let image = self.original_source_image()?;
        if !input.has_logical_source_name_context()
            || self.bindings.original_logical_source_name_advice_input() != Some(input)
            || !self.matches_resolved_analysis_input(input)
            || !self.matches_original_source_image(image, input.lexer_config())
        {
            return None;
        }
        let context = input.borrowed_context_registry();
        let prepared = self.prepared_prefix_tape(context);
        Some(
            prepared
                .as_ref()
                .as_ref()?
                .logical_name_reference_inventory(input, image),
        )
    }

    /// Reborrow genuine header cards from this realm's immutable source catalogue.
    /// Current complete source/input remain independent of each consumer's purpose.
    pub(crate) fn original_logical_procedure_body_cards(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Option<Vec<Arc<crate::command_binding::OriginalLogicalProcedureBody>>> {
        let image = self.original_source_image()?;
        if !input.has_logical_source_name_context()
            || self.bindings.original_logical_source_name_advice_input() != Some(input)
            || !self.matches_resolved_analysis_input(input)
            || !self.matches_original_source_image(image, input.lexer_config())
        {
            return None;
        }
        let context = input.borrowed_context_registry();
        let prepared = self.prepared_prefix_tape(context);
        Some(prepared.as_ref().as_ref()?.logical_procedure_body_cards())
    }

    /// Checked source-header catalogue; an original refused card is terminal.
    /// Current complete source/input remain independent of each consumer's purpose.
    pub(crate) fn checked_original_logical_procedure_body_cards(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Option<Vec<Arc<crate::command_binding::OriginalLogicalProcedureBody>>> {
        let image = self.original_source_image()?;
        if !input.has_logical_source_name_context()
            || self.bindings.original_logical_source_name_advice_input() != Some(input)
            || !self.matches_resolved_analysis_input(input)
            || !self.matches_original_source_image(image, input.lexer_config())
        {
            return None;
        }
        let context = input.borrowed_context_registry();
        let prepared = self.prepared_prefix_tape(context);
        prepared
            .as_ref()
            .as_ref()?
            .checked_logical_procedure_body_cards()
    }

    pub(in super::super) fn original_logical_catalogue_word_at_span(
        &self,
        image: &tcl_lexer::SourceImage,
        span: tcl_lexer::Span,
        config: tcl_lexer::LexerConfig,
    ) -> Option<(tcl_lexer::NativeWord, u32)> {
        let input = self.source_input.as_ref()?;
        if !input.has_logical_source_name_context()
            || self.bindings.original_logical_source_name_advice_input() != Some(input)
            || !self.matches_resolved_analysis_input(input)
            || config != input.lexer_config()
            || !self.matches_original_source_image(image, config)
        {
            return None;
        }
        let context = input.borrowed_context_registry();
        let prepared = self.prepared_prefix_tape(context);
        prepared.as_ref().as_ref()?.logical_original_word_at_span(
            self.bindings.source_origin()?,
            input,
            span,
        )
    }
}
