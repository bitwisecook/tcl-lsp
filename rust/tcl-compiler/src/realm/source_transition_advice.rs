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

#[cfg(test)]
fn trace_original_child_guard(label: &str, offset: Option<u32>, passed: bool) {
    if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_CHILD_GUARDS").is_some() {
        eprintln!("ORIGINAL_CHILD_GUARD label={label} offset={offset:?} passed={passed}");
    }
}

fn cut_value_stage<T>(trace: bool, stage: &'static str, value: Option<T>) -> Option<T> {
    if trace {
        crate::registry_invocation::source_cut_stage(stage, value)
    } else {
        value
    }
}
fn cut_value_refuses(trace: bool, stage: &'static str, refused: bool) -> bool {
    if trace {
        crate::registry_invocation::source_cut_refuses(stage, refused)
    } else {
        refused
    }
}

fn observe_cut_value_refusal(
    trace: bool,
    binding: &crate::command_binding::SourceInvocationBinding,
    image: &tcl_lexer::SourceImage,
    offset: u32,
) {
    cut_value_refuses(trace, "value.represented-refusal", true);
    if trace && crate::registry_invocation::source_cut_trace_admitted() {
        let origin = binding
            .dispatch_site
            .as_ref()
            .map(|site| Arc::as_ptr(&site.source));
        eprintln!(
            "SOURCE_CUT_VALUE_REFUSED origin={origin:?} data={:p} bytes={} offset={offset}",
            image.bytes().as_ptr(),
            image.len(),
        );
    }
}

type SourceAdviceCacheEntry = (
    ResolvedContext,
    tcl_registry::RegistrySemanticKey,
    crate::command_binding::OriginalSourceCataloguePurpose,
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
        self.get_or_prepare_for(
            context,
            crate::command_binding::OriginalSourceCataloguePurpose::CompleteProgram,
            prepare,
        )
    }

    fn get_or_prepare_for(
        &self,
        context: &ContextRegistry,
        purpose: crate::command_binding::OriginalSourceCataloguePurpose,
        prepare: impl FnOnce() -> Option<OriginalSourceTransitionAdviceTape>,
    ) -> Arc<Option<OriginalSourceTransitionAdviceTape>> {
        self.get_or_prepare_shared_for(context, purpose, || Arc::new(prepare()))
    }

    fn get_or_prepare_with_shared_fallback(
        &self,
        context: &ContextRegistry,
        prepare: impl FnOnce() -> Option<OriginalSourceTransitionAdviceTape>,
        fallback: impl FnOnce() -> Arc<Option<OriginalSourceTransitionAdviceTape>>,
    ) -> Arc<Option<OriginalSourceTransitionAdviceTape>> {
        self.get_or_prepare_shared_for(
            context,
            crate::command_binding::OriginalSourceCataloguePurpose::CompleteProgram,
            || match prepare() {
                Some(tape) => Arc::new(Some(tape)),
                None => fallback(),
            },
        )
    }

    fn get_or_prepare_shared_for(
        &self,
        context: &ContextRegistry,
        purpose: crate::command_binding::OriginalSourceCataloguePurpose,
        prepare: impl FnOnce() -> Arc<Option<OriginalSourceTransitionAdviceTape>>,
    ) -> Arc<Option<OriginalSourceTransitionAdviceTape>> {
        let registry = context.commands().snapshot().semantic_key();
        let find = |entries: &[SourceAdviceCacheEntry]| {
            entries
                .iter()
                .find(|(owned, commands, source_purpose, _)| {
                    owned == context.context()
                        && commands == &registry
                        && *source_purpose == purpose
                })
                .map(|(_, _, _, value)| Arc::clone(value))
        };
        if let Some(value) = find(
            &self
                .entries
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        ) {
            return value;
        }
        let prepared = prepare();
        let mut entries = self
            .entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(value) = find(&entries) {
            return value;
        }
        if entries.len() < 4 {
            entries.push((
                context.context().clone(),
                registry,
                purpose,
                Arc::clone(&prepared),
            ));
        }
        prepared
    }
}
impl super::CommandBindingRealm {
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
        let purpose = crate::command_binding::OriginalSourceCataloguePurpose::CompletePrefix;
        let prepared = self.source_advice.get_or_prepare_for(context, purpose, || {
            self.bindings
                .original_source_transition_advice_tape_for(context, purpose)
        });
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
        let purpose = crate::command_binding::OriginalSourceCataloguePurpose::CompletePrefix;
        let prepared = self.source_advice.get_or_prepare_for(context, purpose, || {
            self.bindings
                .original_source_transition_advice_tape_for(context, purpose)
        });
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
                .and_then(super::super::command_binding::formal_topology::OriginalFormalTopology::original_name_fields)
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
        let purpose = crate::command_binding::OriginalSourceCataloguePurpose::CompletePrefix;
        let prepared = self.source_advice.get_or_prepare_for(context, purpose, || {
            self.bindings
                .original_source_transition_advice_tape_for(context, purpose)
        });
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
        let purpose = crate::command_binding::OriginalSourceCataloguePurpose::CompletePrefix;
        let prepared = self.source_advice.get_or_prepare_for(context, purpose, || {
            self.bindings
                .original_source_transition_advice_tape_for(context, purpose)
        });
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
        let prepared = self.source_advice.get_or_prepare(&context, || {
            self.bindings
                .original_source_transition_advice_tape(&context)
        });
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
        if tcl_registry::InvocationDialect::of_profile(input.unit_profile()).family()
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
        let prepared = self.source_advice.get_or_prepare(&context, || {
            self.bindings
                .original_source_transition_advice_tape(&context)
        });
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
        let prepared = self.source_advice.get_or_prepare(&context, || {
            self.bindings
                .original_source_transition_advice_tape(&context)
        });
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
        let prepared = self.source_advice.get_or_prepare(&context, || {
            self.bindings
                .original_source_transition_advice_tape(&context)
        });
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
        let prepared = self.source_advice.get_or_prepare(&context, || {
            self.bindings
                .original_source_transition_advice_tape(&context)
        });
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
        let prepared = self.source_advice.get_or_prepare(&context, || {
            self.bindings
                .original_source_transition_advice_tape(&context)
        });
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
        let prepared = self.source_advice.get_or_prepare(&context, || {
            self.bindings
                .original_source_transition_advice_tape(&context)
        });
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
        let prepared = self.source_advice.get_or_prepare(&context, || {
            self.bindings
                .original_source_transition_advice_tape(&context)
        });
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

    pub(crate) fn original_native_procedure_body_commands(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Option<Vec<crate::command_binding::OriginalNativeProcedureBodyCommand>> {
        self.matches_resolved_analysis_input(input).then_some(())?;
        self.bindings.original_native_procedure_body_commands(input)
    }

    fn prepared_source_procedure_advice(
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
                self.source_advice.get_or_prepare(context, || {
                    self.bindings
                        .original_source_transition_advice_tape(context)
                })
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

    /// Absolute member-body source catalogue retains its original issuer.
    /// The returned schema cannot provide selected dispatch or an entered frame.
    pub(crate) fn original_member_source_catalogue_advice(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<OriginalSourceCommandTransitionAdvice> {
        self.original_catalogue_advice(input, tokens, true)
    }

    /// Reborrow one actual Member catalogue row for readonly source reporting.
    /// This retains the original body issuer and all applicability debts. It
    /// supplies no physical receiver lookup, command entry or edit inventory.
    pub(crate) fn original_member_reference_source_catalogue_advice(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        original: &[tcl_lexer::NativeWord],
    ) -> Option<OriginalSourceCommandTransitionAdvice> {
        // naming.editor.original-method-candidates
        // docs/design/analysis/name-resolution-proofs/original-method-candidates.md
        let image = self.original_source_image()?;
        let config = input.lexer_config();
        if !self.matches_resolved_analysis_input(input)
            || !self.matches_original_source_image(image, config)
            || original.is_empty()
            || original
                .iter()
                .any(|word| word.image() != image || word.config() != config)
        {
            return None;
        }
        let context = input.borrowed_context_registry();
        let purpose = crate::command_binding::OriginalSourceCataloguePurpose::CompleteProgram;
        let prepared = self.source_advice.get_or_prepare_for(context, purpose, || {
            self.bindings
                .original_source_transition_advice_tape_for(context, purpose)
        });
        let tape = prepared.as_ref().as_ref()?;
        let offset = original.first()?.span().start();
        if tape.blocks_registry_source(offset) {
            return None;
        }
        let advice = tape.member_catalogue(offset)?;
        let body = advice.original_definition_member_source_body()?;
        let extent = tcl_lexer::Span::new(offset, original.last()?.span().end());
        if !advice.matches_context(context)
            || !advice.matches_source(image, config)
            || advice.original_words() != original
            || !body.matches_context(context)
            || !body.matches_source(image, config)
            || extent.start() < body.content_span().start()
            || extent.end() > body.content_span().end()
        {
            return None;
        }
        Some(advice.clone())
    }

    /// Readonly schema at a complete original command before a distinct parse tail.
    /// This grants no partial command, body entry, current cell or execution.
    pub(crate) fn original_source_prefix_catalogue_advice(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<OriginalSourceCommandTransitionAdvice> {
        self.original_catalogue_advice(input, tokens, false)
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
        let purpose = crate::command_binding::OriginalSourceCataloguePurpose::CompletePrefix;
        let prepared = self.source_advice.get_or_prepare_for(context, purpose, || {
            self.bindings
                .original_source_transition_advice_tape_for(context, purpose)
        });
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
        let purpose = crate::command_binding::OriginalSourceCataloguePurpose::CompletePrefix;
        let prepared = self.source_advice.get_or_prepare_for(context, purpose, || {
            self.bindings
                .original_source_transition_advice_tape_for(context, purpose)
        });
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
        let prepared = self.source_advice.get_or_prepare_for(context, purpose, || {
            self.bindings
                .original_source_transition_advice_tape_for(context, purpose)
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
        let purpose = crate::command_binding::OriginalSourceCataloguePurpose::CompletePrefix;
        let prepared = self.source_advice.get_or_prepare_for(context, purpose, || {
            self.bindings
                .original_source_transition_advice_tape_for(context, purpose)
        });
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
        let purpose = crate::command_binding::OriginalSourceCataloguePurpose::CompletePrefix;
        let prepared = self.source_advice.get_or_prepare_for(context, purpose, || {
            self.bindings
                .original_source_transition_advice_tape_for(context, purpose)
        });
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
        let purpose = crate::command_binding::OriginalSourceCataloguePurpose::CompletePrefix;
        let prepared = self.source_advice.get_or_prepare_for(context, purpose, || {
            self.bindings
                .original_source_transition_advice_tape_for(context, purpose)
        });
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
        let purpose = crate::command_binding::OriginalSourceCataloguePurpose::CompletePrefix;
        let prepared = self.source_advice.get_or_prepare_for(context, purpose, || {
            self.bindings
                .original_source_transition_advice_tape_for(context, purpose)
        });
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
        let purpose = crate::command_binding::OriginalSourceCataloguePurpose::CompletePrefix;
        let prepared = self.source_advice.get_or_prepare_for(context, purpose, || {
            self.bindings
                .original_source_transition_advice_tape_for(context, purpose)
        });
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
        let purpose = crate::command_binding::OriginalSourceCataloguePurpose::CompletePrefix;
        let prepared = self.source_advice.get_or_prepare_for(context, purpose, || {
            self.bindings
                .original_source_transition_advice_tape_for(context, purpose)
        });
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
        let purpose = crate::command_binding::OriginalSourceCataloguePurpose::CompletePrefix;
        let prepared = self.source_advice.get_or_prepare_for(context, purpose, || {
            self.bindings
                .original_source_transition_advice_tape_for(context, purpose)
        });
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
        let purpose = crate::command_binding::OriginalSourceCataloguePurpose::CompletePrefix;
        let prepared = self.source_advice.get_or_prepare_for(context, purpose, || {
            self.bindings
                .original_source_transition_advice_tape_for(context, purpose)
        });
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
        let purpose = crate::command_binding::OriginalSourceCataloguePurpose::CompletePrefix;
        let prepared = self.source_advice.get_or_prepare_for(context, purpose, || {
            self.bindings
                .original_source_transition_advice_tape_for(context, purpose)
        });
        prepared
            .as_ref()
            .as_ref()?
            .checked_logical_procedure_body_cards()
    }

    pub(super) fn original_logical_catalogue_word_at_span(
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
        let purpose = crate::command_binding::OriginalSourceCataloguePurpose::CompletePrefix;
        let prepared = self.source_advice.get_or_prepare_for(context, purpose, || {
            self.bindings
                .original_source_transition_advice_tape_for(context, purpose)
        });
        prepared.as_ref().as_ref()?.logical_original_word_at_span(
            self.bindings.source_origin()?,
            input,
            span,
        )
    }

    /// Exact original command extent from the shared source issuer. The caller
    /// still checks current point/target metadata before using its schema.
    pub(crate) fn original_source_catalogue_segment_at(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        offset: u32,
    ) -> Option<crate::segmenter::SegmentedCommand> {
        self.matches_resolved_analysis_input(input).then_some(())?;
        let image = self.original_source_image()?;
        let config = input.lexer_config();
        self.matches_original_source_image(image, config)
            .then_some(())?;
        let context = input.borrowed_context_registry();
        let purpose = crate::command_binding::OriginalSourceCataloguePurpose::CompletePrefix;
        let prepared = self.source_advice.get_or_prepare_for(context, purpose, || {
            self.bindings
                .original_source_transition_advice_tape_for(context, purpose)
        });
        let tape = prepared.as_ref().as_ref()?;
        if tape.blocks_registry_source(offset) {
            return None;
        }
        let advice = if let Some(advice) = tape.body_catalogue(offset) {
            advice
        } else {
            match tape.lookup(offset) {
                OriginalSourceTransitionAdviceLookup::Advice(advice) => advice,
                OriginalSourceTransitionAdviceLookup::Refused
                | OriginalSourceTransitionAdviceLookup::Unrepresented => return None,
            }
        };
        if !advice.matches_context(context) || !advice.matches_source(image, config) {
            return None;
        }
        if input.has_logical_source_name_context() {
            return Some(
                tape.logical_original_command_at(self.bindings.source_origin()?, input, offset)?
                    .command()
                    .clone(),
            );
        }
        let span = tcl_lexer::Span::new(
            advice.original_words().first()?.span().start(),
            advice.original_words().last()?.span().end(),
        );
        if span.start() != offset {
            return None;
        }
        let mut segments = crate::segmenter::segment_commands_with_offset_and_config(
            image.try_text().ok()?.get(span.as_range())?,
            span.start(),
            config.nested(),
        );
        (segments.len() == 1).then_some(())?;
        let segment = segments.pop()?;
        let tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::from_image(image),
            config,
            &segment,
        );
        (crate::registry_invocation::original_native_compiler_words(
            image,
            tokens.words(),
            offset,
            config,
        )?
        .as_slice()
            == advice.original_words())
        .then_some(segment)
    }

    /// Possible original case syntax retains source uncertainty rather than
    /// borrowing a reached handler or an ordinary invocation word vector.
    pub(crate) fn original_conditional_case_syntax_advice(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<OriginalSourceCommandTransitionAdvice> {
        self.matches_resolved_analysis_input(input).then_some(())?;
        if tokens.synthetic.is_some()
            || input.has_hosted_source_name_context()
            || input.has_logical_source_name_context()
        {
            return None;
        }
        let context = input.borrowed_context_registry();
        let image = self.original_source_image()?;
        let config = input.lexer_config();
        self.matches_original_source_image(image, config)
            .then_some(())?;
        let binding = tokens.source_binding.as_ref()?;
        if binding.lookup_state.is_some() && !binding.unknown && binding.targets.is_empty() {
            return None;
        }
        let offset = tokens.words().first()?.source().span.start();
        let original = crate::registry_invocation::original_native_compiler_words(
            image,
            tokens.words(),
            offset,
            config,
        )?;
        let purpose = crate::command_binding::OriginalSourceCataloguePurpose::CompletePrefix;
        let prepared = self.source_advice.get_or_prepare_for(context, purpose, || {
            self.bindings
                .original_source_transition_advice_tape_for(context, purpose)
        });
        let tape = prepared.as_ref().as_ref()?;
        if tape.blocks_registry_source(offset) {
            return None;
        }
        let advice = tape.conditional_case_syntax(offset)?;
        if !advice.matches_context(context)
            || !advice.matches_source(image, config)
            || original.as_slice() != advice.original_words()
        {
            return None;
        }
        if binding.lookup_state.is_some() {
            binding
                .original_invocation_metadata_at_point(tokens, context.commands())
                .ok()?;
        }
        crate::command_binding::source_targets_match_advice(context, binding, advice)
            .then(|| advice.clone())
    }

    fn original_catalogue_advice(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        tokens: &crate::ir::CommandTokens,
        member: bool,
    ) -> Option<OriginalSourceCommandTransitionAdvice> {
        self.matches_resolved_analysis_input(input).then_some(())?;
        if tokens.synthetic.is_some() {
            return None;
        }
        let context = input.borrowed_context_registry();
        let image = self.original_source_image()?;
        let config = input.lexer_config();
        self.matches_original_source_image(image, config)
            .then_some(())?;
        let binding = tokens.source_binding.as_ref()?;
        if binding.lookup_state.is_some() && !binding.unknown && binding.targets.is_empty() {
            return None;
        }
        let offset = tokens.words().first()?.source().span.start();
        let original = crate::registry_invocation::original_native_compiler_words(
            image,
            tokens.words(),
            offset,
            config,
        )?;
        let purpose = if member {
            crate::command_binding::OriginalSourceCataloguePurpose::CompleteProgram
        } else {
            crate::command_binding::OriginalSourceCataloguePurpose::CompletePrefix
        };
        let prepared = self.source_advice.get_or_prepare_for(context, purpose, || {
            self.bindings
                .original_source_transition_advice_tape_for(context, purpose)
        });
        let tape = prepared.as_ref().as_ref()?;
        if tape.blocks_registry_source(offset) {
            return None;
        }
        let advice = if member {
            tape.member_catalogue(offset)?
        } else {
            OriginalSourceTransitionAdviceTape::prefix_catalogue_advice(tape, offset)?
        };
        if !advice.matches_context(context)
            || !advice.matches_source(image, config)
            || original.as_slice() != advice.original_words()
        {
            return None;
        }
        if binding.lookup_state.is_some() {
            binding
                .original_invocation_metadata_at_point(tokens, context.commands())
                .ok()?;
        }
        if !crate::command_binding::source_targets_match_advice(context, binding, advice) {
            return None;
        }
        Some(advice.clone())
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
        let advice = self.retained_source_transition_value_advice(context, tokens, false)?;
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
        trace_cut: bool,
    ) -> Option<OriginalSourceCommandTransitionAdvice> {
        cut_value_stage(trace_cut, "value.binding", tokens.source_binding.as_ref())?;
        if cut_value_refuses(trace_cut, "value.synthetic", tokens.synthetic.is_some()) {
            return None;
        }
        self.retained_source_transition_value_advice(context, tokens, trace_cut)
    }

    #[cfg(test)]
    fn trace_retained_source_input(
        &self,
        context: &ContextRegistry,
        tokens: &crate::ir::CommandTokens,
        binding: &crate::command_binding::SourceInvocationBinding,
        logical: Option<&crate::analyser::ResolvedAnalysisInput>,
    ) {
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_CHILD_GUARDS").is_some() {
            let offset = tokens
                .words()
                .first()
                .map(|word| word.source().span.start());
            trace_original_child_guard("retained.logical", offset, logical.is_some());
            trace_original_child_guard(
                "retained.input_context",
                offset,
                logical.is_none_or(|input| {
                    self.matches_resolved_analysis_input(input)
                        && std::ptr::eq(Arc::as_ptr(&input.context_registry()), context)
                }),
            );
            trace_original_child_guard(
                "retained.reached_lookup",
                offset,
                binding.lookup_state.is_some(),
            );
            trace_original_child_guard("retained.unknown", offset, binding.unknown);
            trace_original_child_guard("retained.targets", offset, !binding.targets.is_empty());
        }
    }

    #[cfg(test)]
    fn trace_retained_source_vector(
        original: &[tcl_lexer::NativeWord],
        advice: &OriginalSourceCommandTransitionAdvice,
        offset: u32,
    ) {
        if original != advice.original_words() {
            eprintln!(
                "ORIGINAL_ADVICE_VECTOR offset={offset} original_count={} retained_count={}",
                original.len(),
                advice.original_words().len(),
            );
            for (index, (actual, retained)) in original
                .iter()
                .zip(advice.original_words())
                .enumerate()
                .take(16)
            {
                if actual != retained {
                    eprintln!(
                        "ORIGINAL_ADVICE_VECTOR offset={offset} index={index} image={} config={} group={} tokens={} arena={} written={} operand={} original_start={} original_end={} retained_start={} retained_end={} original_tokens={} retained_tokens={}",
                        actual.image() == retained.image(),
                        actual.config() == retained.config(),
                        actual.group() == retained.group(),
                        actual.tokens() == retained.tokens(),
                        actual.executable_parts() == retained.executable_parts(),
                        actual.span() == retained.span(),
                        actual.word_span() == retained.word_span(),
                        actual.span().start(),
                        actual.span().end(),
                        retained.span().start(),
                        retained.span().end(),
                        actual.tokens().len(),
                        retained.tokens().len(),
                    );
                }
            }
        }
    }

    #[cfg(test)]
    fn trace_retained_source_value(
        context: &ContextRegistry,
        binding: &crate::command_binding::SourceInvocationBinding,
        logical: Option<&crate::analyser::ResolvedAnalysisInput>,
        source: (&tcl_lexer::SourceImage, tcl_lexer::LexerConfig, u32),
        original: &[tcl_lexer::NativeWord],
        advice: &OriginalSourceCommandTransitionAdvice,
    ) {
        let (image, config, offset) = source;
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_CHILD_GUARDS").is_some() {
            trace_original_child_guard(
                "retained.advice_context",
                Some(offset),
                advice.matches_context(context),
            );
            trace_original_child_guard(
                "retained.advice_source",
                Some(offset),
                advice.matches_source(image, config),
            );
            trace_original_child_guard(
                "retained.advice_vector",
                Some(offset),
                original == advice.original_words(),
            );
            Self::trace_retained_source_vector(original, advice, offset);
            trace_original_child_guard(
                "retained.target_match",
                Some(offset),
                logical.is_none()
                    || crate::command_binding::source_targets_match_advice(
                        context, binding, advice,
                    ),
            );
        }
    }

    fn retained_source_value_matches(
        context: &ContextRegistry,
        tokens: &crate::ir::CommandTokens,
        binding: &crate::command_binding::SourceInvocationBinding,
        logical: Option<&crate::analyser::ResolvedAnalysisInput>,
        source: (&tcl_lexer::SourceImage, tcl_lexer::LexerConfig, u32),
        advice: &OriginalSourceCommandTransitionAdvice,
        trace_cut: bool,
    ) -> Option<()> {
        let (image, config, offset) = source;
        if cut_value_refuses(
            trace_cut,
            "value.advice-context",
            !advice.matches_context(context),
        ) || cut_value_refuses(
            trace_cut,
            "value.advice-source",
            !advice.matches_source(image, config),
        ) || cut_value_refuses(
            trace_cut,
            "value.advice-vector",
            cut_value_stage(
                trace_cut,
                "value.final-native-vector",
                crate::registry_invocation::original_native_compiler_words(
                    image,
                    tokens.words(),
                    offset,
                    config,
                ),
            )?
            .as_slice()
                != advice.original_words(),
        ) {
            return None;
        }
        if logical.is_some()
            && cut_value_refuses(
                trace_cut,
                "value.logical-target",
                !crate::command_binding::source_targets_match_advice(context, binding, advice),
            )
        {
            return None;
        }
        Some(())
    }

    fn retained_source_transition_value_advice(
        &self,
        context: &ContextRegistry,
        tokens: &crate::ir::CommandTokens,
        trace_cut: bool,
    ) -> Option<OriginalSourceCommandTransitionAdvice> {
        let binding = tokens.source_binding.as_ref()?;
        let logical = self.bindings.original_logical_source_name_advice_input();
        #[cfg(test)]
        self.trace_retained_source_input(context, tokens, binding, logical);
        if let Some(input) = logical
            && (cut_value_refuses(
                trace_cut,
                "value.logical-owner",
                !self.matches_resolved_analysis_input(input),
            ) || cut_value_refuses(
                trace_cut,
                "value.logical-context",
                !std::ptr::eq(Arc::as_ptr(&input.context_registry()), context),
            ))
        {
            return None;
        }
        let config = cut_value_stage(
            trace_cut,
            "value.lexer",
            self.bindings.original_source_lexer_config(),
        )?;
        let image = cut_value_stage(trace_cut, "value.image", self.original_source_image())?;
        cut_value_stage(
            trace_cut,
            "value.image-current",
            self.matches_original_source_image(image, config)
                .then_some(()),
        )?;
        let offset = cut_value_stage(trace_cut, "value.head", tokens.words().first())?
            .source()
            .span
            .start();
        let prepared = self.source_advice.get_or_prepare(context, || {
            self.bindings
                .original_source_transition_advice_tape(context)
        });
        let original = crate::registry_invocation::original_native_compiler_words(
            image,
            tokens.words(),
            offset,
            config,
        );
        #[cfg(test)]
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_CHILD_GUARDS").is_some() {
            trace_original_child_guard(
                "retained.tape",
                Some(offset),
                prepared.as_ref().as_ref().is_some(),
            );
            trace_original_child_guard("retained.native_vector", Some(offset), original.is_some());
        }
        let original = cut_value_stage(trace_cut, "value.native-vector", original)?;
        let advice = match cut_value_stage(trace_cut, "value.tape", prepared.as_ref().as_ref())?
            .lookup(offset)
        {
            OriginalSourceTransitionAdviceLookup::Advice(advice) => {
                #[cfg(test)]
                trace_original_child_guard("retained.represented_advice", Some(offset), true);
                advice.clone()
            }
            OriginalSourceTransitionAdviceLookup::Refused => {
                observe_cut_value_refusal(trace_cut, binding, image, offset);
                #[cfg(test)]
                trace_original_child_guard("retained.represented_refusal", Some(offset), false);
                return None;
            }
            OriginalSourceTransitionAdviceLookup::Unrepresented => {
                #[cfg(test)]
                trace_original_child_guard("retained.unrepresented", Some(offset), false);
                cut_value_stage(trace_cut, "value.unrepresented-logical", logical)?;
                cut_value_stage(
                    trace_cut,
                    "value.logical-declaration",
                    self.bindings
                        .original_logical_declaration_advice(context, &original),
                )?
            }
        };
        #[cfg(test)]
        Self::trace_retained_source_value(
            context,
            binding,
            logical,
            (image, config, offset),
            &original,
            &advice,
        );
        Self::retained_source_value_matches(
            context,
            tokens,
            binding,
            logical,
            (image, config, offset),
            &advice,
            trace_cut,
        )?;
        Some(advice)
    }
}

impl super::CommandBindingRealm {
    pub(crate) fn original_class_display_publications(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Option<crate::registry_invocation::source_structure::OriginalProjectClassCells> {
        self.matches_resolved_analysis_input(input).then_some(())?;
        let image = self.original_source_image()?;
        let context = input.context_registry();
        let prepared = self.source_advice.get_or_prepare(&context, || {
            self.bindings
                .original_source_transition_advice_tape(&context)
        });
        let inventory = prepared.as_ref().as_ref()?.class_display_publications()?;
        inventory.with_retained_input(image, input)
    }
}

#[cfg(test)]
mod shared_fallback_tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn shared_fallback_retains_the_same_unavailable_value_without_repreparing() {
        let context = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let ordinary = OriginalSourceTransitionAdviceCache::default();
        let calls = OriginalSourceTransitionAdviceCache::default();
        let prepared = ordinary.get_or_prepare(context, || None);
        let attempts = Cell::new(0);
        let selected = calls.get_or_prepare_with_shared_fallback(
            context,
            || {
                attempts.set(attempts.get() + 1);
                None
            },
            || ordinary.get_or_prepare(context, || panic!("ordinary value already retained")),
        );
        assert!(Arc::ptr_eq(&prepared, &selected));
        assert!(selected.is_none());
        let repeated = calls.get_or_prepare_with_shared_fallback(
            context,
            || panic!("call cache already retained"),
            || panic!("call cache already retained"),
        );
        assert!(Arc::ptr_eq(&selected, &repeated));
        assert_eq!(attempts.get(), 1);
    }

    #[test]
    fn preferred_preparation_never_consults_the_shared_fallback() {
        // Cache mechanics only: the empty test tape asserts no source outcome.
        let context = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let calls = OriginalSourceTransitionAdviceCache::default();
        let prepared = calls.get_or_prepare_with_shared_fallback(
            context,
            || Some(OriginalSourceTransitionAdviceTape::default()),
            || panic!("a present preferred tape must not consult the fallback"),
        );
        assert!(prepared.is_some());
        let repeated = calls.get_or_prepare_with_shared_fallback(
            context,
            || panic!("preferred tape already retained"),
            || panic!("preferred tape already retained"),
        );
        assert!(Arc::ptr_eq(&prepared, &repeated));
    }

    #[test]
    fn non_normal_source_reuses_its_actual_ordinary_tape() {
        let analysis =
            crate::analyser::Analyser::new().analyse("error source_tape_cache_control", "tcl8.6");
        let input = analysis.resolved_input.as_ref().unwrap();
        let context = input.context_registry();
        let realm = analysis.retained_command_realm().unwrap();
        assert!(
            realm
                .bindings
                .original_normal_source_procedure_advice_tape(&context)
                .is_none()
        );
        let ordinary = realm.source_advice.get_or_prepare(&context, || {
            realm
                .bindings
                .original_source_transition_advice_tape(&context)
        });
        assert!(ordinary.is_some());
        let selected = realm.prepared_source_procedure_advice(&context);
        assert!(Arc::ptr_eq(&ordinary, &selected));
        assert!(
            realm
                .bindings
                .original_normal_source_procedure_advice_tape(&context)
                .is_none()
        );
    }
}
