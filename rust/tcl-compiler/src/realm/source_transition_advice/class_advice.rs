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

//! Class advice: Jim classes, class declarations, references, constructors and instances.

use crate::command_binding::SourceOriginLexical;
use std::sync::Arc;

impl super::super::CommandBindingRealm {
    pub(super) fn registered_factory_provider_is_proved(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        instance: &crate::command_binding::OriginalSourceRegisteredInstance,
    ) -> bool {
        let context = input.borrowed_context_registry();
        let Some(spec) = instance.descriptor(context) else {
            return false;
        };
        self.bindings
            .package_factory_is_proved_at(Some(instance.factory().site()), spec)
    }

    /// Conditional instance method source syntax, with its own actual factory.
    /// Neither an entered callable nor a runtime object is supplied.
    pub(crate) fn original_source_registered_instance_words(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        original: &[tcl_lexer::NativeWord],
    ) -> Option<crate::command_binding::OriginalSourceRegisteredInstanceWords> {
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
            .registered_instance(original.first()?.span().start())?;
        (receipt.matches_source_context(image, config, &context)
            && receipt.original_words() == original
            && self.registered_factory_provider_is_proved(input, receipt.instance()))
        .then(|| receipt.clone())
    }

    /// Original selected named Registry factory, without successful allocation.
    pub(crate) fn original_source_registered_factory(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        original: &[tcl_lexer::NativeWord],
    ) -> Option<crate::command_binding::OriginalSourceRegisteredInstance> {
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
            .registered_factory(original.first()?.span().start())?;
        (receipt.factory().matches_source(image, config)
            && receipt.factory().matches_context(&context)
            && receipt.factory().original_words() == original
            && self.registered_factory_provider_is_proved(input, receipt))
        .then(|| receipt.clone())
    }

    /// Distinct conditional Jim source header and parent-bound base providers.
    /// Reborrowing this source tape issues no Native class, argument or receiver.
    pub(crate) fn original_authored_jim_call(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        original: &[tcl_lexer::NativeWord],
    ) -> Option<crate::command_binding::OriginalAuthoredJimClassCall> {
        crate::registry_invocation::source_structure::trace_jim_source_option(
            "jim.call.input",
            self.matches_resolved_analysis_input(input).then_some(()),
        )?;
        let image = crate::registry_invocation::source_structure::trace_jim_source_option(
            "jim.call.image",
            self.original_source_image(),
        )?;
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
        let tape = crate::registry_invocation::source_structure::trace_jim_source_option(
            "jim.call.tape",
            prepared.as_ref().as_ref(),
        )?;
        let receipt = crate::registry_invocation::source_structure::trace_jim_source_option(
            "jim.call.tape_receipt",
            tape.authored_jim_call(original.first()?.span().start()),
        )?;
        crate::registry_invocation::source_structure::trace_jim_source_option(
            "jim.call.exact_rejoin",
            (receipt.matches_source_context(image, config, &context)
                && receipt.original_words() == original)
                .then(|| receipt.clone()),
        )
    }

    /// Whole original class words from its genuine script parent. This is
    /// source geometry only; it does not enter the body or settle its lookup.
    pub(crate) fn original_authored_jim_parent_words(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        offset: u32,
    ) -> Option<Vec<tcl_lexer::NativeWord>> {
        if tcl_native_model::InvocationDialect::of_profile(input.unit_profile()).family()
            != Some(tcl_dialect::model::Family::Jim)
        {
            return None;
        }
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
        let receipt = prepared.as_ref().as_ref()?.authored_jim_class(offset)?;
        let parent = receipt.parent_body()?;
        (receipt.factory().site().offset == offset
            && receipt.matches_source_context(image, config, &context))
        .then_some(())?;
        let plan = receipt
            .factory()
            .site()
            .source
            .original_script_words_in(
                parent.content_span(),
                config,
                tcl_lexer::SourceSegmentationPurpose::NestedScript,
            )
            .ok()?;
        plan.commands
            .iter()
            .find(|command| {
                command.words == receipt.factory().original_words()
                    && command
                        .words
                        .first()
                        .is_some_and(|word| word.span().start() == offset)
            })
            .map(|command| command.words.clone())
    }

    pub(crate) fn original_authored_jim_class(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        original: &[tcl_lexer::NativeWord],
    ) -> Option<Arc<crate::command_binding::OriginalAuthoredJimClass>> {
        crate::registry_invocation::source_structure::trace_jim_source_option(
            "authored.input",
            self.matches_resolved_analysis_input(input).then_some(()),
        )?;
        let image = crate::registry_invocation::source_structure::trace_jim_source_option(
            "authored.image",
            self.original_source_image(),
        )?;
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
        let tape = crate::registry_invocation::source_structure::trace_jim_source_option(
            "authored.tape",
            prepared.as_ref().as_ref(),
        )?;
        let receipt = crate::registry_invocation::source_structure::trace_jim_source_option(
            "authored.tape_receipt",
            tape.authored_jim_class(original.first()?.span().start()),
        )?;
        (crate::registry_invocation::source_structure::trace_jim_source_option(
            "authored.canonical_context",
            receipt
                .matches_source_context(image, config, &context)
                .then_some(()),
        )
        .is_some()
            && crate::registry_invocation::source_structure::trace_jim_source_option(
                "authored.canonical_words",
                (receipt.factory().original_words() == original).then_some(()),
            )
            .is_some())
        .then(|| Arc::clone(receipt))
    }

    /// Exact source class factory selected independently of class report maps.
    pub(crate) fn original_source_class_declaration(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        original: &[tcl_lexer::NativeWord],
    ) -> Option<crate::command_binding::OriginalSourceClassDeclaration> {
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
            .source_class(original.first()?.span().start())?;
        (receipt.factory().matches_source(image, config)
            && receipt.factory().matches_context(&context)
            && receipt.factory().original_words() == original)
            .then(|| receipt.clone())
    }

    /// One authentic Logical declaration facet under its actual source schema.
    /// Generic inventory conflicts and Native occupation remain independent.
    pub(crate) fn original_source_class_declaration_for_advice(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        original: &[tcl_lexer::NativeWord],
        advice: &crate::command_binding::OriginalSourceCommandTransitionAdvice,
    ) -> Option<crate::command_binding::OriginalSourceClassDeclaration> {
        self.matches_resolved_analysis_input(input).then_some(())?;
        let image = self.original_source_image()?;
        let config = input.lexer_config();
        self.matches_original_source_image(image, config)
            .then_some(())?;
        let context = input.context_registry();
        if advice.logical_source_input() != Some(input)
            || !advice.matches_source(image, config)
            || !advice.matches_context(&context)
            || advice.original_words() != original
        {
            return None;
        }
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
            .source_class_for_advice(advice)?;
        Some(receipt.clone())
    }

    /// Own Jim source members from the exact original consumer vector and realm.
    pub(crate) fn original_authored_jim_procedure_members(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        original: &[tcl_lexer::NativeWord],
    ) -> Option<Vec<crate::command_binding::OriginalAuthoredJimProcedureMember>> {
        self.matches_resolved_analysis_input(input).then_some(())?;
        let image = self.original_source_image()?;
        self.matches_original_source_image(image, input.lexer_config())
            .then_some(())?;
        let context = input.context_registry();
        let owner = self.bindings.original_analysis_owner()?;
        let prepared = self.source_advice.get_or_prepare_shared_for(
            &context,
            crate::command_binding::OriginalSourceCataloguePurpose::CompleteProgram,
            || {
                self.bindings
                    .original_source_transition_advice_tape_shared(&context)
            },
        );
        Some(
            prepared
                .as_ref()
                .as_ref()?
                .authored_jim_procedure_members(original, owner),
        )
    }

    pub(crate) fn authenticates_authored_jim_procedure_member(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        receipt: &crate::command_binding::OriginalAuthoredJimProcedureMember,
    ) -> bool {
        if !self.matches_resolved_analysis_input(input) {
            return false;
        }
        let context = input.context_registry();
        let prepared = self.source_advice.get_or_prepare_shared_for(
            &context,
            crate::command_binding::OriginalSourceCataloguePurpose::CompleteProgram,
            || {
                self.bindings
                    .original_source_transition_advice_tape_shared(&context)
            },
        );
        prepared
            .as_ref()
            .as_ref()
            .is_some_and(|tape| tape.authenticates_authored_jim_procedure_member(receipt))
    }

    /// Conditional original list-child class reference at its actual consumer.
    pub(crate) fn original_source_class_reference(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        original: &[tcl_lexer::NativeWord],
        name: &crate::signature_scan::scope::SignatureSourceNameInput,
    ) -> Option<crate::command_binding::OriginalSourceClassReference> {
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
        let receipt = prepared
            .as_ref()
            .as_ref()?
            .class_reference(original.first()?.span().start(), name)?;
        (receipt.matches_source_context(image, input.lexer_config(), &context)
            && receipt.consumer().original_words() == original)
            .then(|| receipt.clone())
    }

    /// Selected class configuration operand before its authored mutation.
    pub(crate) fn original_source_configured_class(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        original: &[tcl_lexer::NativeWord],
    ) -> Option<crate::command_binding::OriginalSourceConfiguredClassReference> {
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
            .configured_class(original.first()?.span().start())?;
        (receipt.matches_source_context(image, config, &context)
            && receipt.consumer().original_words() == original)
            .then(|| receipt.clone())
    }

    /// Whole constructor source words joined to their authentic class definer.
    pub(crate) fn original_source_constructor_call(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        original: &[tcl_lexer::NativeWord],
    ) -> Option<crate::command_binding::OriginalSourceConstructorCall> {
        crate::registry_invocation::source_structure::trace_jim_source_option(
            "constructor.input",
            self.matches_resolved_analysis_input(input).then_some(()),
        )?;
        let image = crate::registry_invocation::source_structure::trace_jim_source_option(
            "constructor.image",
            self.original_source_image(),
        )?;
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
        let tape = crate::registry_invocation::source_structure::trace_jim_source_option(
            "constructor.tape",
            prepared.as_ref().as_ref(),
        )?;
        let receipt = crate::registry_invocation::source_structure::trace_jim_source_option(
            "constructor.tape_receipt",
            tape.constructor_call(original.first()?.span().start()),
        )?;
        (receipt.matches_source_context(image, config, &context)
            && receipt.original_words() == original)
            .then(|| receipt.clone())
    }

    /// Whole original `Class::proc` call joined to its [incr Tcl] source class.
    pub(crate) fn original_source_class_proc_call(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        original: &[tcl_lexer::NativeWord],
    ) -> Option<crate::command_binding::OriginalSourceClassProcCall> {
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
            .class_proc_call(original.first()?.span().start())?;
        (receipt.matches_source_context(image, config, &context)
            && receipt.original_words() == original)
            .then(|| receipt.clone())
    }

    /// Whole immutable source-class receiver and original constructor lineage.
    pub(crate) fn original_source_class_instance_words(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        original: &[tcl_lexer::NativeWord],
    ) -> Option<crate::command_binding::OriginalSourceClassInstanceWords> {
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
        let receipt = prepared
            .as_ref()
            .as_ref()?
            .class_instance_words(original.first()?.span().start())?;
        (receipt.matches_source_context(image, input.lexer_config(), &context)
            && receipt.original_words() == original)
            .then(|| receipt.clone())
    }

    /// Final conditional root class slots, with no allocated class or runtime world.
    pub(crate) fn original_source_class_publications(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Option<crate::command_binding::OriginalSourceClassPublications> {
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
        let inventory = prepared.as_ref().as_ref()?.class_publications()?;
        inventory
            .matches_source_context(image, input.lexer_config(), &context)
            .then(|| inventory.with_retained_input(input))
    }
}

impl super::super::CommandBindingRealm {
    pub(crate) fn original_class_display_publications(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Option<crate::registry_invocation::source_structure::OriginalProjectClassCells> {
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
        let inventory = prepared.as_ref().as_ref()?.class_display_publications()?;
        inventory.with_retained_input(image, input)
    }
}
