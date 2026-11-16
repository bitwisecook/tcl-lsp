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

//! Pure authored-transition tapes shared by one immutable source realm.

use crate::command_binding::{
    OriginalSourceCommandTransitionAdvice, OriginalSourceTransitionAdviceLookup,
    OriginalSourceTransitionAdviceTape,
};
use std::sync::{Arc, Mutex};
use tcl_registry::model::{ContextRegistry, ResolvedContext};

mod class_advice;
mod logical_source;
mod procedure_advice;

#[cfg(test)]
fn trace_original_child_guard(label: &str, offset: Option<u32>, passed: bool) {
    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_CHILD_GUARDS").is_some() {
        eprintln!("ORIGINAL_CHILD_GUARD label={label} offset={offset:?} passed={passed}");
    }
}

#[cfg(test)]
fn trace_retained_source_preparation(offset: u32, tape: bool, native_vector: bool) {
    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_CHILD_GUARDS").is_some() {
        trace_original_child_guard("retained.tape", Some(offset), tape);
        trace_original_child_guard("retained.native_vector", Some(offset), native_vector);
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
        let origin = binding.source_cut_trace_dispatch_origin();
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
    #[cfg(test)]
    fn get_or_prepare(
        &self,
        context: &ContextRegistry,
        prepare: impl FnOnce() -> Option<OriginalSourceTransitionAdviceTape>,
    ) -> Arc<Option<OriginalSourceTransitionAdviceTape>> {
        self.get_or_prepare_shared_for(
            context,
            crate::command_binding::OriginalSourceCataloguePurpose::CompleteProgram,
            || Arc::new(prepare()),
        )
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
            .get_or_prepare_shared_for(
                context,
                crate::command_binding::OriginalSourceCataloguePurpose::CompleteProgram,
                || {
                    self.bindings
                        .original_source_transition_advice_tape_shared(context)
                },
            )
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
        let prepared = self.source_advice.get_or_prepare_shared_for(
            &context,
            crate::command_binding::OriginalSourceCataloguePurpose::CompleteProgram,
            || {
                self.bindings
                    .original_source_transition_advice_tape_shared(&context)
            },
        );
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
        let prepared = self.source_advice.get_or_prepare_shared_for(
            &context,
            crate::command_binding::OriginalSourceCataloguePurpose::CompleteProgram,
            || {
                self.bindings
                    .original_source_transition_advice_tape_shared(&context)
            },
        );
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
        let prepared = self
            .source_advice
            .get_or_prepare_shared_for(context, purpose, || {
                self.bindings
                    .original_source_transition_advice_tape_for_shared(context, purpose)
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
        let prepared = self.prepared_prefix_tape(context);
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
        let prepared = self.prepared_prefix_tape(context);
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

    fn prepared_tape_for(
        &self,
        context: &ContextRegistry,
        purpose: crate::command_binding::OriginalSourceCataloguePurpose,
    ) -> Arc<Option<OriginalSourceTransitionAdviceTape>> {
        self.source_advice
            .get_or_prepare_shared_for(context, purpose, || {
                self.bindings
                    .original_source_transition_advice_tape_for_shared(context, purpose)
            })
    }

    /// The complete-prefix tape. A complete-program tape exists only when the
    /// whole image has no fatal tail and no conditional region, and then the
    /// prefix tape is the identical computation, so it is reused rather than
    /// preparing the whole source a second time.
    fn prepared_prefix_tape(
        &self,
        context: &ContextRegistry,
    ) -> Arc<Option<OriginalSourceTransitionAdviceTape>> {
        use crate::command_binding::OriginalSourceCataloguePurpose::{
            CompletePrefix, CompleteProgram,
        };
        let program = self.prepared_tape_for(context, CompleteProgram);
        if program.is_some() {
            program
        } else {
            self.prepared_tape_for(context, CompletePrefix)
        }
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
        let prepared = if member {
            self.prepared_tape_for(
                context,
                crate::command_binding::OriginalSourceCataloguePurpose::CompleteProgram,
            )
        } else {
            self.prepared_prefix_tape(context)
        };
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

    /// Read primary declaration values from this same held tape, never lookup fallback.
    pub(crate) fn original_source_procedure_declaration_values(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        context: &ContextRegistry,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<crate::command_binding::OriginalSourceProcedureDeclarationValues> {
        if cut_value_refuses(
            true,
            "declaration-values.full-input",
            !self.matches_resolved_analysis_input(input)
                || !std::ptr::eq(Arc::as_ptr(&input.context_registry()), context)
                || tokens.synthetic.is_some()
                || !self.bindings.owns_original_analysis_attachment(tokens),
        ) {
            return None;
        }
        let binding = cut_value_stage(
            true,
            "declaration-values.binding",
            tokens.source_binding.as_ref(),
        )?;
        let origin = cut_value_stage(
            true,
            "declaration-values.origin",
            self.bindings.source_origin(),
        )?;
        let owner = cut_value_stage(
            true,
            "declaration-values.analysis-owner",
            self.bindings.original_analysis_owner(),
        )?;
        let image = origin.source_image();
        let config = input.lexer_config();
        if cut_value_refuses(
            true,
            "declaration-values.current-image",
            self.bindings.original_source_lexer_config() != Some(config)
                || !self.matches_original_source_image(image, config),
        ) {
            return None;
        }
        let offset = cut_value_stage(true, "declaration-values.head", tokens.words().first())?
            .source()
            .span
            .start();
        let original = cut_value_stage(
            true,
            "declaration-values.native-vector",
            crate::registry_invocation::original_native_compiler_words(
                image,
                tokens.words(),
                offset,
                config,
            ),
        )?;
        let purpose = if self.bindings.has_conditional_source_region() {
            crate::command_binding::OriginalSourceCataloguePurpose::ConditionalRegion
        } else {
            crate::command_binding::OriginalSourceCataloguePurpose::CompleteProgram
        };
        let prepared = self
            .source_advice
            .get_or_prepare_shared_for(context, purpose, || {
                self.bindings
                    .original_source_transition_advice_tape_for_shared(context, purpose)
            });
        let tape = cut_value_stage(true, "declaration-values.tape", prepared.as_ref().as_ref())?;
        tape.procedure_declaration_values(owner, input, offset, &original, binding)
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
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_CHILD_GUARDS").is_some() {
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
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_CHILD_GUARDS").is_some() {
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
        let prepared = self.source_advice.get_or_prepare_shared_for(
            context,
            crate::command_binding::OriginalSourceCataloguePurpose::CompleteProgram,
            || {
                self.bindings
                    .original_source_transition_advice_tape_shared(context)
            },
        );
        let original = crate::registry_invocation::original_native_compiler_words(
            image,
            tokens.words(),
            offset,
            config,
        );
        #[cfg(test)]
        trace_retained_source_preparation(
            offset,
            prepared.as_ref().as_ref().is_some(),
            original.is_some(),
        );
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

#[cfg(test)]
mod shared_fallback_tests;
