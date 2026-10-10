// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Pure authored-transition tapes shared by one immutable source realm.

use crate::command_binding::{
    OriginalSourceCommandTransitionAdvice, OriginalSourceTransitionAdviceLookup,
    OriginalSourceTransitionAdviceTape,
};
use std::sync::{Arc, Mutex};
use tcl_registry::model::{ContextRegistry, ResolvedContext};

type SourceAdviceCacheEntry = (
    ResolvedContext,
    Arc<Option<OriginalSourceTransitionAdviceTape>>,
);

#[derive(Debug, Clone, Default)]
pub(super) struct OriginalSourceTransitionAdviceCache {
    entries: Arc<Mutex<Vec<SourceAdviceCacheEntry>>>,
}
impl OriginalSourceTransitionAdviceCache {
    fn get_or_prepare(
        &self,
        context: &ContextRegistry,
        prepare: impl FnOnce() -> Option<OriginalSourceTransitionAdviceTape>,
    ) -> Arc<Option<OriginalSourceTransitionAdviceTape>> {
        let find = |entries: &[SourceAdviceCacheEntry]| {
            entries
                .iter()
                .find(|(owned, _)| owned == context.context())
                .map(|(_, value)| Arc::clone(value))
        };
        if let Some(value) = find(
            &self
                .entries
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        ) {
            return value;
        }
        let prepared = Arc::new(prepare());
        let mut entries = self
            .entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(value) = find(&entries) {
            return value;
        }
        if entries.len() < 4 {
            entries.push((context.context().clone(), Arc::clone(&prepared)));
        }
        prepared
    }
}
impl super::CommandBindingRealm {
    /// A genuine authored shadow/delete/alias loop bars conditional Registry
    /// source fallback. Missing or unsupported catalogue coordinates do not.
    pub(crate) fn original_source_registry_barrier(
        &self,
        context: &ContextRegistry,
        tokens: &crate::ir::CommandTokens,
    ) -> bool {
        let Some(config) = self.bindings.original_source_lexer_config() else {
            return false;
        };
        let Some(image) = self.original_source_image() else {
            return false;
        };
        let Some(offset) = tokens
            .words()
            .first()
            .map(|word| word.source().span.start())
        else {
            return false;
        };
        if tokens.synthetic.is_some()
            || !self.matches_original_source_image(image, config)
            || crate::registry_invocation::original_native_compiler_words(
                image,
                tokens.words(),
                offset,
                config,
            )
            .is_none()
        {
            return false;
        }
        self.source_advice
            .get_or_prepare(context, || {
                self.bindings
                    .original_source_transition_advice_tape(context)
            })
            .as_ref()
            .as_ref()
            .is_some_and(|tape| tape.blocks_registry_source(offset))
    }

    /// Canonical created-child eval source body, separate from selected
    /// dispatch. The same full input/context/source/config and exact original
    /// vectors guard the cached source-order recipe.
    pub(crate) fn original_source_interpreter_handle_body(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        original: &[tcl_lexer::NativeWord],
    ) -> Option<crate::command_binding::OriginalSourceInterpreterHandleBody> {
        self.matches_resolved_analysis_input(input).then_some(())?;
        let image = self.original_source_image()?;
        let config = input.lexer_config();
        self.matches_original_source_image(image, config)
            .then_some(())?;
        let context = input.context_registry();
        let prepared = self.source_advice.get_or_prepare(&context, || {
            self.bindings
                .original_source_transition_advice_tape(&context)
        });
        let receipt = prepared
            .as_ref()
            .as_ref()?
            .interpreter_body(original.first()?.span().start())?;
        (receipt.matches_source_context(image, config, &context)
            && receipt.original_words() == original)
            .then(|| receipt.clone())
    }

    /// Original selected child path, distinct from the moved parent command.
    /// Complete creation and invocation vectors guard the conditional binding.
    pub(crate) fn original_source_interpreter_path_binding(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        original: &[tcl_lexer::NativeWord],
        path: &crate::signature_scan::scope::SignatureSourceNameInput,
    ) -> Option<crate::command_binding::OriginalSourceInterpreterPathBinding> {
        self.matches_resolved_analysis_input(input).then_some(())?;
        let image = self.original_source_image()?;
        let config = input.lexer_config();
        self.matches_original_source_image(image, config)
            .then_some(())?;
        let context = input.context_registry();
        let prepared = self.source_advice.get_or_prepare(&context, || {
            self.bindings
                .original_source_transition_advice_tape(&context)
        });
        let mut matching = prepared
            .as_ref()
            .as_ref()?
            .interpreter_paths(original.first()?.span().start())?
            .iter()
            .filter(|binding| {
                binding.path_input() == path
                    && binding.original_words() == original
                    && binding.matches_source_context(image, config, &context)
            });
        let selected = matching.next()?;
        matching
            .all(|other| other == selected)
            .then(|| selected.clone())
    }

    pub(crate) fn original_authored_declared_source_selection(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        tokens: &crate::ir::CommandTokens,
        original: &[tcl_lexer::NativeWord],
    ) -> Option<crate::command_binding::DeclaredSourceSelection> {
        // naming.source.original-declared-command-word-contract
        // docs/design/analysis/name-resolution-proofs/source-original-declared-command-word-contract.md
        let config = input.lexer_config();
        let image = self.original_source_image()?;
        if tokens.synthetic.is_some()
            || !self.matches_resolved_analysis_input(input)
            || !self.matches_original_source_image(image, config)
        {
            return None;
        }
        let binding = tokens.source_binding.as_ref()?;
        if !binding.targets.is_empty() {
            return None;
        }
        let context = input.context_registry();
        let offset = original.first()?.span().start();
        let prepared = self.source_advice.get_or_prepare(&context, || {
            self.bindings
                .original_source_transition_advice_tape(&context)
        });
        let selected = prepared.as_ref().as_ref()?.declared_selection(offset)?;
        if crate::registry_invocation::original_native_compiler_words(
            image,
            tokens.words(),
            offset,
            config,
        )?
        .as_slice()
            != original
            || selected.original_words() != original
        {
            return None;
        }
        Some(selected.clone())
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
        let prepared = self.source_advice.get_or_prepare(&context, || {
            self.bindings
                .original_source_transition_advice_tape(&context)
        });
        let receipt = prepared
            .as_ref()
            .as_ref()?
            .registered_instance(original.first()?.span().start())?;
        (receipt.matches_source_context(image, config, &context)
            && receipt.original_words() == original)
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
        let prepared = self.source_advice.get_or_prepare(&context, || {
            self.bindings
                .original_source_transition_advice_tape(&context)
        });
        let receipt = prepared
            .as_ref()
            .as_ref()?
            .registered_factory(original.first()?.span().start())?;
        (receipt.factory().matches_source(image, config)
            && receipt.factory().matches_context(&context)
            && receipt.factory().original_words() == original)
            .then(|| receipt.clone())
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
        let prepared = self.source_advice.get_or_prepare(&context, || {
            self.bindings
                .original_source_transition_advice_tape(&context)
        });
        let receipt = prepared
            .as_ref()
            .as_ref()?
            .source_class(original.first()?.span().start())?;
        (receipt.factory().matches_source(image, config)
            && receipt.factory().matches_context(&context)
            && receipt.factory().original_words() == original)
            .then(|| receipt.clone())
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
        let prepared = self.source_advice.get_or_prepare(&context, || {
            self.bindings
                .original_source_transition_advice_tape(&context)
        });
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
        let prepared = self.source_advice.get_or_prepare(&context, || {
            self.bindings
                .original_source_transition_advice_tape(&context)
        });
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
        self.matches_resolved_analysis_input(input).then_some(())?;
        let image = self.original_source_image()?;
        let config = input.lexer_config();
        self.matches_original_source_image(image, config)
            .then_some(())?;
        let context = input.context_registry();
        let prepared = self.source_advice.get_or_prepare(&context, || {
            self.bindings
                .original_source_transition_advice_tape(&context)
        });
        let receipt = prepared
            .as_ref()
            .as_ref()?
            .constructor_call(original.first()?.span().start())?;
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
        let prepared = self.source_advice.get_or_prepare(&context, || {
            self.bindings
                .original_source_transition_advice_tape(&context)
        });
        let receipt = prepared
            .as_ref()
            .as_ref()?
            .class_instance_words(original.first()?.span().start())?;
        (receipt.matches_source_context(image, input.lexer_config(), &context)
            && receipt.original_words() == original)
            .then(|| receipt.clone())
    }

    /// Final conditional root procedure publications, not a completed runtime world.
    pub(crate) fn original_source_procedure_publications(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Option<crate::command_binding::OriginalSourceProcedurePublications> {
        self.matches_resolved_analysis_input(input).then_some(())?;
        let image = self.original_source_image()?;
        let context = input.context_registry();
        let prepared = self.source_advice.get_or_prepare(&context, || {
            self.bindings
                .original_source_transition_advice_tape(&context)
        });
        let inventory = prepared.as_ref().as_ref()?.procedure_publications()?;
        inventory
            .matches_source_context(image, input.lexer_config(), &context)
            .then(|| inventory.clone())
    }

    /// Final conditional root class slots, with no allocated class or runtime world.
    pub(crate) fn original_source_class_publications(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Option<crate::command_binding::OriginalSourceClassPublications> {
        self.matches_resolved_analysis_input(input).then_some(())?;
        let image = self.original_source_image()?;
        let context = input.context_registry();
        let prepared = self.source_advice.get_or_prepare(&context, || {
            self.bindings
                .original_source_transition_advice_tape(&context)
        });
        let inventory = prepared.as_ref().as_ref()?.class_publications()?;
        inventory
            .matches_source_context(image, input.lexer_config(), &context)
            .then(|| inventory.with_retained_input(input))
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
        let prepared = self.source_advice.get_or_prepare(&context, || {
            self.bindings
                .original_source_transition_advice_tape(&context)
        });
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
        let prepared = self.source_advice.get_or_prepare(&context, || {
            self.bindings
                .original_source_transition_advice_tape(&context)
        });
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
        let prepared = self.source_advice.get_or_prepare(&context, || {
            self.bindings
                .original_source_transition_advice_tape(&context)
        });
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
        let prepared = self.source_advice.get_or_prepare(&context, || {
            self.bindings
                .original_source_transition_advice_tape(&context)
        });
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
        let prepared = self.source_advice.get_or_prepare(&context, || {
            self.bindings
                .original_source_transition_advice_tape(&context)
        });
        let prefix = prepared
            .as_ref()
            .as_ref()?
            .authored_prefix(original.first()?.span().start())?;
        (prefix.matches_source_context(image, config, &context)
            && prefix.producer().original_words() == original)
            .then(|| prefix.clone())
    }

    /// Source-only aliases and moves do not fill absent execution lookup.
    /// Full context and immutable source/Registry/configuration guard the tape.
    pub(crate) fn original_source_transition_advice(
        &self,
        context: &ContextRegistry,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<OriginalSourceCommandTransitionAdvice> {
        let binding = tokens.source_binding.as_ref()?;
        // A snapshot is not itself a selected command. Retained unbounded
        // lookup may receive conditional source advice; a known target or
        // definite absence remains authoritative and cannot fall through.
        let logical = self.bindings.original_logical_source_name_advice_input();

        if tokens.synthetic.is_some() {
            return None;
        }
        if let Some(input) = logical
            && (!self.matches_resolved_analysis_input(input)
                || !std::ptr::eq(Arc::as_ptr(&input.context_registry()), context))
        {
            return None;
        }
        let advice = self.retained_source_transition_value_advice(context, tokens)?;
        if logical.is_none()
            && binding.lookup_state.is_some()
            && (!binding.unknown || !binding.targets.is_empty())
        {
            let original_operand_uncertainty = advice.obligations().contains(
                &crate::command_binding::SourceCommandTransitionObligation::NativeBaselineSourceApplicability,
            ) && advice.uncertain_operations().iter().any(|operation| {
                operation.as_ref() == advice.original_words()
            });
            if !original_operand_uncertainty {
                return None;
            }
        }
        Some(advice)
    }

    /// Original source operand producers are a separate purpose from command
    /// selection. A known target may retain declaration values, but the caller
    /// must independently match its effective argv, target and dialect.
    pub(crate) fn original_source_transition_value_advice(
        &self,
        context: &ContextRegistry,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<OriginalSourceCommandTransitionAdvice> {
        tokens.source_binding.as_ref()?;
        if tokens.synthetic.is_some() {
            return None;
        }
        self.retained_source_transition_value_advice(context, tokens)
    }

    fn retained_source_transition_value_advice(
        &self,
        context: &ContextRegistry,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<OriginalSourceCommandTransitionAdvice> {
        let binding = tokens.source_binding.as_ref()?;
        let logical = self.bindings.original_logical_source_name_advice_input();
        if let Some(input) = logical
            && (!self.matches_resolved_analysis_input(input)
                || !std::ptr::eq(Arc::as_ptr(&input.context_registry()), context))
        {
            return None;
        }
        let config = self.bindings.original_source_lexer_config()?;
        let image = self.original_source_image()?;
        self.matches_original_source_image(image, config)
            .then_some(())?;
        let offset = tokens.words().first()?.source().span.start();
        let prepared = self.source_advice.get_or_prepare(context, || {
            self.bindings
                .original_source_transition_advice_tape(context)
        });
        let original = crate::registry_invocation::original_native_compiler_words(
            image,
            tokens.words(),
            offset,
            config,
        )?;
        let advice = match prepared.as_ref().as_ref()?.lookup(offset) {
            OriginalSourceTransitionAdviceLookup::Advice(advice) => advice.clone(),
            OriginalSourceTransitionAdviceLookup::Refused => return None,
            OriginalSourceTransitionAdviceLookup::Unrepresented => {
                logical?;
                self.bindings
                    .original_logical_declaration_advice(context, &original)?
            }
        };
        if !advice.matches_context(context)
            || !advice.matches_source(image, config)
            || crate::registry_invocation::original_native_compiler_words(
                image,
                tokens.words(),
                offset,
                config,
            )?
            .as_slice()
                != advice.original_words()
        {
            return None;
        }
        if logical.is_some() && !logical_targets_match_advice(context, binding, &advice) {
            return None;
        }
        Some(advice)
    }
}

/// Conditional Logical advice cannot override an actually selected target.
fn logical_targets_match_advice(
    context: &ContextRegistry,
    binding: &crate::command_binding::SourceInvocationBinding,
    advice: &OriginalSourceCommandTransitionAdvice,
) -> bool {
    if binding.lookup_state.is_some() && !binding.unknown && binding.targets.is_empty() {
        return false;
    }
    let prefix = advice
        .arguments()
        .iter()
        .filter(|word| {
            matches!(
                word.origin,
                crate::registry_invocation::InvocationWordOrigin::BindingPrefix(_)
            )
        })
        .map(|word| word.value.as_deref())
        .collect::<Option<Vec<_>>>();
    let Some(prefix) = prefix else {
        return false;
    };
    let Some(descriptor) = context
        .context()
        .resolve_spec(context.commands(), advice.command())
    else {
        return false;
    };
    if binding.targets.iter().any(|target| {
        !target.registry_backed
            || !context
                .context()
                .resolve_spec(context.commands(), &target.command)
                .is_some_and(|selected| std::ptr::eq(selected, descriptor))
            || target
                .prepended
                .iter()
                .map(crate::registry_invocation::EffectiveInvocationWord::literal_bytes)
                .collect::<Option<Vec<_>>>()
                .as_deref()
                != Some(prefix.as_slice())
    }) {
        return false;
    }
    true
}
