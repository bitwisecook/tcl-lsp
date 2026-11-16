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

//! Procedure calls and publications, callback targets and command prefixes.

use crate::command_binding::OriginalSourceTransitionAdviceTape;
use std::sync::Arc;
use tcl_registry::model::ContextRegistry;

impl super::super::CommandBindingRealm {
    pub(crate) fn original_native_procedure_body_commands(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Option<Vec<crate::command_binding::OriginalNativeProcedureBodyCommand>> {
        self.matches_resolved_analysis_input(input).then_some(())?;
        self.bindings.original_native_procedure_body_commands(input)
    }

    pub(crate) fn original_native_deferred_autoload_body_commands(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Option<Vec<crate::command_binding::OriginalNativeProcedureBodyCommand>> {
        self.matches_resolved_analysis_input(input).then_some(())?;
        self.bindings
            .original_native_deferred_autoload_body_commands(input)
    }

    pub(super) fn prepared_source_procedure_advice(
        &self,
        context: &ContextRegistry,
    ) -> Arc<Option<OriginalSourceTransitionAdviceTape>> {
        // The separate call cache still prefers its genuine Normal query.
        // Only the identical ordinary fallback shares this realm's tape.
        self.source_call_advice.get_or_prepare_with_shared_fallback(
            context,
            || {
                self.bindings
                    .original_normal_source_procedure_advice_tape(context)
            },
            || {
                self.source_advice.get_or_prepare_shared_for(
                    context,
                    crate::command_binding::OriginalSourceCataloguePurpose::CompleteProgram,
                    || {
                        self.bindings
                            .original_source_transition_advice_tape_shared(context)
                    },
                )
            },
        )
    }

    pub(crate) fn original_source_procedure_calls(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Option<Vec<crate::command_binding::OriginalSourceProcedureCall>> {
        self.matches_resolved_analysis_input(input).then_some(())?;
        let owner = self.bindings.original_analysis_owner()?;
        owner.matches(&self.bindings, input).then_some(())?;
        let image = self.original_source_image()?;
        self.matches_original_source_image(image, input.lexer_config())
            .then_some(())?;
        let context = input.context_registry();
        let prepared = self.prepared_source_procedure_advice(&context);
        Some(
            prepared
                .as_ref()
                .as_ref()?
                .native_procedure_calls()
                .map(|call| {
                    call.clone().with_input(
                        input,
                        Arc::clone(owner),
                        self.bindings.original_inherited_source_entry(),
                    )
                })
                .filter(|call| call.matches_source_context(image, input.lexer_config(), &context))
                .collect(),
        )
    }

    pub(crate) fn original_source_procedure_call_at(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        offset: u32,
    ) -> Option<crate::command_binding::OriginalSourceProcedureCall> {
        self.matches_resolved_analysis_input(input).then_some(())?;
        let owner = self.bindings.original_analysis_owner()?;
        owner.matches(&self.bindings, input).then_some(())?;
        let image = self.original_source_image()?;
        self.matches_original_source_image(image, input.lexer_config())
            .then_some(())?;
        let context = input.context_registry();
        let prepared = self.prepared_source_procedure_advice(&context);
        let call = prepared
            .as_ref()
            .as_ref()?
            .native_procedure_call_at(offset)?
            .clone()
            .with_input(
                input,
                Arc::clone(owner),
                self.bindings.original_inherited_source_entry(),
            );
        call.matches_source_context(image, input.lexer_config(), &context)
            .then_some(call)
    }

    /// Final conditional root procedure publications, not a completed runtime world.
    pub(crate) fn original_source_procedure_publications(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Option<crate::command_binding::OriginalSourceProcedurePublications> {
        self.matches_resolved_analysis_input(input).then_some(())?;
        let image = self.original_source_image()?;
        let context = input.context_registry();
        let prepared = self.source_advice.get_or_prepare_shared_for(
            &context,
            crate::command_binding::OriginalSourceCataloguePurpose::CompleteProgram,
            || {
                self.bindings
                    .original_source_transition_advice_tape_shared(&context)
            },
        );
        let inventory = prepared.as_ref().as_ref()?.procedure_publications()?;
        inventory
            .matches_source_context(image, input.lexer_config(), &context)
            .then(|| inventory.clone())
    }

    /// Complete positioned source callback inventory at one genuine installer.
    /// Target rows retain explicit barriers and conditional future applicability;
    /// no command-word presentation or registration-frame fallback is accepted.
    pub(crate) fn original_source_callback_targets_for_tokens(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<Vec<crate::command_binding::OriginalSourceCallbackProcedureLookup>> {
        if tokens.synthetic.is_some() || !self.matches_resolved_analysis_input(input) {
            return None;
        }
        let image = self.original_source_image()?;
        let offset = tokens.words().first()?.source().span.start();
        let original = crate::registry_invocation::original_native_compiler_words(
            image,
            tokens.words(),
            offset,
            input.lexer_config(),
        )?;
        let context = input.context_registry();
        let prepared = self.source_advice.get_or_prepare_shared_for(
            &context,
            crate::command_binding::OriginalSourceCataloguePurpose::CompleteProgram,
            || {
                self.bindings
                    .original_source_transition_advice_tape_shared(&context)
            },
        );
        let rows = prepared
            .as_ref()
            .as_ref()?
            .callback_procedure_targets(offset)?;
        rows.iter()
            .all(|row| {
                row.registration().original_words() == original.as_slice()
                    && row.matches_source_context(image, input.lexer_config(), &context)
            })
            .then_some(rows)
    }

    /// Callback target source ancestry at its genuine registration occurrence.
    pub(crate) fn original_source_callback_procedure_target(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        original: &[tcl_lexer::NativeWord],
        prefix: &crate::command_binding::OriginalCallbackPrefix,
    ) -> Option<crate::command_binding::OriginalSourceCallbackProcedureLookup> {
        self.matches_resolved_analysis_input(input).then_some(())?;
        let image = self.original_source_image()?;
        let context = input.context_registry();
        let prepared = self.source_advice.get_or_prepare_shared_for(
            &context,
            crate::command_binding::OriginalSourceCataloguePurpose::CompleteProgram,
            || {
                self.bindings
                    .original_source_transition_advice_tape_shared(&context)
            },
        );
        let target = prepared
            .as_ref()
            .as_ref()?
            .callback_procedure_target(original.first()?.span().start(), prefix)?;
        (target.matches_source_context(image, input.lexer_config(), &context)
            && target.registration().original_words() == original)
            .then(|| target.clone())
    }

    /// The same original complete setter and selected factory substitution.
    /// The source binding does not assert that a variable cell holds a value.
    pub(crate) fn original_source_registered_handle_binding(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        original: &[tcl_lexer::NativeWord],
    ) -> Option<crate::command_binding::OriginalSourceRegisteredHandleBinding> {
        self.matches_resolved_analysis_input(input).then_some(())?;
        let image = self.original_source_image()?;
        let config = input.lexer_config();
        self.matches_original_source_image(image, config)
            .then_some(())?;
        let context = input.context_registry();
        let prepared = self.source_advice.get_or_prepare_shared_for(
            &context,
            crate::command_binding::OriginalSourceCataloguePurpose::CompleteProgram,
            || {
                self.bindings
                    .original_source_transition_advice_tape_shared(&context)
            },
        );
        let receipt = prepared
            .as_ref()
            .as_ref()?
            .registered_handle(original.first()?.span().start())?;
        (receipt.matches_source_context(image, config, &context)
            && receipt.setter().original_words() == original)
            .then(|| receipt.clone())
    }

    /// Original list-built future syntax under the same retained whole-source
    /// input and context. No command tokens are made for the produced prefix.
    pub(crate) fn original_produced_command_prefix(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        original: &[tcl_lexer::NativeWord],
    ) -> Option<crate::command_binding::OriginalSourceProducedCommandPrefix> {
        self.matches_resolved_analysis_input(input).then_some(())?;
        let image = self.original_source_image()?;
        let config = input.lexer_config();
        self.matches_original_source_image(image, config)
            .then_some(())?;
        let context = input.context_registry();
        let prepared = self.source_advice.get_or_prepare_shared_for(
            &context,
            crate::command_binding::OriginalSourceCataloguePurpose::CompleteProgram,
            || {
                self.bindings
                    .original_source_transition_advice_tape_shared(&context)
            },
        );
        let prefix = prepared
            .as_ref()
            .as_ref()?
            .produced_prefix(original.first()?.span().start())?;
        (prefix.matches_source_context(image, config, &context)
            && prefix.producer().original_words() == original)
            .then(|| prefix.clone())
    }

    /// Authored list data retains its exact builder point, not a deferred parent.
    pub(crate) fn original_authored_command_prefix(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        original: &[tcl_lexer::NativeWord],
    ) -> Option<crate::command_binding::OriginalSourceAuthoredCommandPrefix> {
        self.matches_resolved_analysis_input(input).then_some(())?;
        let image = self.original_source_image()?;
        let config = input.lexer_config();
        self.matches_original_source_image(image, config)
            .then_some(())?;
        let context = input.context_registry();
        let prepared = self.source_advice.get_or_prepare_shared_for(
            &context,
            crate::command_binding::OriginalSourceCataloguePurpose::CompleteProgram,
            || {
                self.bindings
                    .original_source_transition_advice_tape_shared(&context)
            },
        );
        let prefix = prepared
            .as_ref()
            .as_ref()?
            .authored_prefix(original.first()?.span().start())?;
        (prefix.matches_source_context(image, config, &context)
            && prefix.producer().original_words() == original)
            .then(|| prefix.clone())
    }
}
