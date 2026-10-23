// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! First-operand hosted remote-result/setter/read source associations.
//! Associations end at an opaque operation before a later source read.
//! These receipts issue no runtime cell, handle, handler, RPC or Normal result.

use super::source_structure::{OriginalRegistrySource, OriginalRegistryWords};
use crate::analyser::{AnalysisResult, ResolvedAnalysisInput};
use crate::signature_scan::vendor_variable::{
    VendorSourceVariableAdvice, VendorSourceVariableBody, VendorSourceVariableBodyKind,
    vendor_variable_source_reference,
};
use std::sync::Arc;
use tcl_lexer::{ExecutablePart, LexerConfig, NativeWord, SourceImage};
use tcl_registry::handle_binding::{HandleBindingSpec, HandleClassSource, HandleName};
use tcl_registry::model::ContextRegistry;
use tcl_registry::remote_method::{RemoteCallSpec, RemoteHandleSpec, RemoteMethodRole};

/// Unresolved premises of this source-only association, never discharged here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteResultSourceObligation {
    /// The selected source schemas need not be the entered implementations.
    HandlerApplicability,
    /// The constructor's successful result and remote extension are unavailable.
    ConstructorResultUnavailable,
    /// Written scalar correspondence issues no stored cell, read or observers.
    VariableCellAndObserversUnavailable,
    /// The retained lexical body supplies no actual activation or lifetime.
    ConditionalBodyApplicability,
    /// Unknown constructor effects remain an unresolved premise, never quiet.
    ConstructorEffectsUnavailable,
}

/// Complete original constructor and selected setter in one genuine lexical body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourceRemoteResultAssociation {
    input: ResolvedAnalysisInput,
    analysis_owner: Arc<crate::command_binding::OriginalSourceAnalysisOwner>,
    body: VendorSourceVariableBody,
    variable: VendorSourceVariableAdvice,
    setter: OriginalRegistryWords,
    setter_layout: HandleBindingSpec,
    operand: NativeWord,
    constructor: OriginalRegistryWords,
    constructor_layout: RemoteHandleSpec,
}

impl OriginalSourceRemoteResultAssociation {
    /// Actual whole source analysis input retained without a physical entry.
    #[must_use]
    pub const fn source_analysis_input(&self) -> &ResolvedAnalysisInput {
        &self.input
    }
    /// Reborrow the same original source/input/body/advice; reporting maps do
    /// not supply this owner, and changed original carriers cannot reuse it.
    #[must_use]
    pub fn matches_source_analysis(&self, source: &str, analysis: &AnalysisResult) -> bool {
        let Some((image, config)) = crate::source_graph::current_analysis(source, analysis) else {
            return false;
        };
        let Some(realm) = analysis.retained_command_realm() else {
            return false;
        };
        let bindings = realm.source_bindings_ref();
        if !bindings.original_analysis_owner().is_some_and(|owner| {
            Arc::ptr_eq(owner, &self.analysis_owner) && owner.matches(bindings, &self.input)
        }) {
            return false;
        }
        let context = self.input.context_registry();
        analysis.resolved_input.as_ref() == Some(&self.input)
            && self.body.matches_source(&image, config)
            && self.body.metadata().context() == context.context()
            && self.body.metadata().registry() == &context.commands().snapshot().semantic_key()
            && self.setter.matches_source(&image, config)
            && self.constructor.matches_source(&image, config)
            && self.setter.matches_registry(context.commands())
            && self.constructor.matches_registry(context.commands())
            && analysis
                .original_vendor_variable_advice()
                .any(|advice| advice == &self.variable)
    }

    /// Genuine lexical declaration/body owner, independently of entered frames.
    #[must_use]
    pub const fn source_body(&self) -> &VendorSourceVariableBody {
        &self.body
    }
    /// Selected setter's supported source scalar operand; no cell identity.
    #[must_use]
    pub const fn variable(&self) -> &VendorSourceVariableAdvice {
        &self.variable
    }
    /// Complete original setter source schema and applicability obligations.
    #[must_use]
    pub const fn setter(&self) -> &OriginalRegistryWords {
        &self.setter
    }
    /// Genuine selected setter result/receiver layout.
    #[must_use]
    pub const fn setter_layout(&self) -> HandleBindingSpec {
        self.setter_layout
    }
    /// Whole original single-command RHS container.
    #[must_use]
    pub const fn original_operand(&self) -> &NativeWord {
        &self.operand
    }
    /// Complete original remote constructor, never an evaluated handle.
    #[must_use]
    pub const fn constructor(&self) -> &OriginalRegistryWords {
        &self.constructor
    }
    /// Selected remote constructor's own label coordinates.
    #[must_use]
    pub const fn constructor_layout(&self) -> RemoteHandleSpec {
        self.constructor_layout
    }
}

/// Later selected remote invocation joined to its original source assignment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourceRemoteFirstOperandAssociation {
    invocation: OriginalRegistryWords,
    layout: RemoteCallSpec,
    handle_operand: NativeWord,
    binding: Arc<OriginalSourceRemoteResultAssociation>,
    body_path: Vec<OriginalRegistryWords>,
}

impl OriginalSourceRemoteFirstOperandAssociation {
    /// Same supplied source/input and original lexical body at this call.
    /// This is correspondence only, independently of runtime handle contents.
    #[must_use]
    pub fn matches_source_analysis(&self, source: &str, analysis: &AnalysisResult) -> bool {
        if !self.binding.matches_source_analysis(source, analysis) {
            return false;
        }
        let Some((image, config)) = crate::source_graph::current_analysis(source, analysis) else {
            return false;
        };
        self.body_path.iter().all(|parent| {
            parent.matches_source(&image, config)
                && parent.matches_registry(self.binding.input.context_registry().commands())
        }) && self.invocation.matches_source(&image, config)
            && self
                .invocation
                .matches_registry(self.binding.input.context_registry().commands())
            && analysis.original_vendor_variable_body_in_source(
                &image,
                config,
                self.handle_operand.span().start(),
            ) == Some(&self.binding.body)
    }

    /// Complete later original selected source invocation.
    #[must_use]
    pub const fn invocation(&self) -> &OriginalRegistryWords {
        &self.invocation
    }
    /// The later source descriptor's handle/method/dispatch coordinates.
    #[must_use]
    pub const fn layout(&self) -> RemoteCallSpec {
        self.layout
    }
    /// Exact whole scalar substitution operand in the later call.
    #[must_use]
    pub const fn handle_operand(&self) -> &NativeWord {
        &self.handle_operand
    }
    /// Compiler-owned source constructor/setter correspondence.
    #[must_use]
    pub fn handle_binding(&self) -> &OriginalSourceRemoteResultAssociation {
        &self.binding
    }
    /// Applicability debts remain explicit; no runtime target follows.
    #[must_use]
    pub const fn obligations(&self) -> &'static [RemoteResultSourceObligation] {
        &[
            RemoteResultSourceObligation::HandlerApplicability,
            RemoteResultSourceObligation::ConstructorResultUnavailable,
            RemoteResultSourceObligation::VariableCellAndObserversUnavailable,
            RemoteResultSourceObligation::ConditionalBodyApplicability,
            RemoteResultSourceObligation::ConstructorEffectsUnavailable,
        ]
    }
}

struct RemoteBodyScan<'a> {
    source: &'a str,
    analysis: &'a AnalysisResult,
    input: &'a ResolvedAnalysisInput,
    image: &'a SourceImage,
    config: LexerConfig,
    context: &'a ContextRegistry,
    body: &'a VendorSourceVariableBody,
}

type SourceBindings = Vec<(String, Arc<OriginalSourceRemoteResultAssociation>)>;

macro_rules! remote_source_trace {
    ($($argument:tt)*) => {
        #[cfg(test)]
        if std::env::var_os("TCL_LSP_TRACE_REMOTE_FIRST_OPERAND").is_some() {
            eprintln!($($argument)*);
        }
    };
}

macro_rules! remote_source_guard {
    ($value:expr, $stage:literal, $offset:expr) => {{
        let value = $value;
        if value.is_none() {
            remote_source_trace!("REMOTE_FIRST_OPERAND stage={} offset={:?}", $stage, $offset);
        }
        value
    }};
}

/// Current supplied hosted analysis only. Standalone source capture is not a
/// substitute for this issuer. Unknown body coverage refuses the whole body;
/// unknown intervening operations withdraw its reaching source association.
/// `Some` authenticates ingress only: refused bodies are omitted, so this is a
/// bounded set of positive candidates and never a complete absence proof.
#[must_use]
pub fn source_remote_first_operand_associations(
    source: &str,
    analysis: &AnalysisResult,
) -> Option<Vec<OriginalSourceRemoteFirstOperandAssociation>> {
    let (image, config) = crate::source_graph::current_analysis(source, analysis)?;
    let input = analysis.resolved_input.as_ref()?;
    if !input.has_hosted_source_name_context() || input.lexer_config() != config {
        return None;
    }
    let context = input.context_registry();
    let mut bodies = Vec::new();
    for advice in analysis.original_vendor_variable_advice() {
        let Some(body) = advice.home() else { continue };
        if matches!(
            body.kind(),
            VendorSourceVariableBodyKind::Procedure | VendorSourceVariableBodyKind::Event
        ) && body.matches_source(&image, config)
            && body.metadata().context() == context.context()
            && body.metadata().registry() == &context.commands().snapshot().semantic_key()
            && !bodies.contains(&body)
        {
            bodies.push(body);
        }
    }
    remote_source_trace!(
        "REMOTE_FIRST_OPERAND stage=body-inventory count={}",
        bodies.len()
    );
    let mut calls = Vec::new();
    for body in bodies {
        let scan = RemoteBodyScan {
            source,
            analysis,
            input,
            image: &image,
            config,
            context: &context,
            body,
        };
        // Missing original body coverage never becomes a complete empty scan.
        if let Some(body_calls) = scan.scan() {
            calls.extend(body_calls);
        }
    }
    Some(calls)
}

impl RemoteBodyScan<'_> {
    fn scan(&self) -> Option<Vec<OriginalSourceRemoteFirstOperandAssociation>> {
        let mut bindings = SourceBindings::new();
        let mut calls = Vec::new();
        self.scan_commands(self.body.span(), &mut bindings, &mut calls, &[], 0)?;
        Some(calls)
    }

    fn scan_commands(
        &self,
        extent: tcl_lexer::Span,
        bindings: &mut SourceBindings,
        calls: &mut Vec<OriginalSourceRemoteFirstOperandAssociation>,
        body_path: &[OriginalRegistryWords],
        depth: usize,
    ) -> Option<()> {
        if depth > 32 {
            return None;
        }
        let plan = tcl_lexer::native_script_words_in_with_purpose(
            self.image.clone(),
            extent,
            self.config.nested(),
            tcl_lexer::SourceSegmentationPurpose::NestedScript,
        )
        .ok()?;
        if plan.fatal_tail.is_some() {
            return None;
        }
        for command in plan.commands {
            let offset = command.words.first()?.span().start();
            if self.analysis.original_vendor_variable_body_in_source(
                self.image,
                self.config,
                offset,
            ) != Some(self.body)
            {
                remote_source_trace!("REMOTE_FIRST_OPERAND stage=body-owner offset={offset}");
                break;
            }
            let Some(words) = self.words_at(offset, &command.words) else {
                remote_source_trace!("REMOTE_FIRST_OPERAND stage=command-words offset={offset}");
                break;
            };
            let Some(summary) = self.summary(&words) else {
                remote_source_trace!("REMOTE_FIRST_OPERAND stage=command-summary offset={offset}");
                break;
            };
            // The query is at this exact handle operand, before later operands
            // and before this handler. Its association never survives the call.
            if let Some(layout) = summary.remote.and_then(|role| role.calls_method().copied()) {
                if let Some(call) = self.first_remote_operand(&words, layout, bindings, body_path) {
                    calls.push(call);
                }
                remote_source_trace!(
                    "REMOTE_FIRST_OPERAND stage=remote-barrier offset={offset} calls={} bindings={}",
                    calls.len(),
                    bindings.len()
                );
                bindings.clear();
                break;
            }
            // Unknown constructor effects remain explicit source debts. Only
            // the exact sole constructor RHS is exempted from continuity here;
            // it does not make the constructor or its result quiet/current.
            if let Some((key, pending)) = self.binding(&words) {
                if self.setter_operands_refuse(&summary, &key, &pending, &command.words) {
                    remote_source_trace!(
                        "REMOTE_FIRST_OPERAND stage=setter-eligibility offset={offset} admits={} opaque={} writes={}",
                        summary.admits_source_setter,
                        summary.writes.opaque_variable_frame,
                        summary.writes.literal_names.len()
                    );
                    break;
                }
                // This store may invoke observers: no prior association is
                // preserved through it. The new result association retains debt.
                bindings.clear();
                bindings.push((key, Arc::new(pending)));
                continue;
            }
            // A setter dispatch occurs after its RHS substitution. Query the
            // genuine nested call's first operand before the outer write, then
            // stop: neither that call nor the setter preserves later bindings.
            if let Some(body) = self.selected_setter_rhs_script(&words, &summary) {
                let mut path = body_path.to_vec();
                path.push(words.clone());
                self.scan_commands(body, bindings, calls, &path, depth + 1)?;
                bindings.clear();
                break;
            }
            if let Some(body) = self.selected_true_same_frame_body(&words) {
                let mut path = body_path.to_vec();
                path.push(words.clone());
                self.scan_commands(body, bindings, calls, &path, depth + 1)?;
                // No completion or post-body preservation is inferred.
                bindings.clear();
                break;
            }
            if !summary.preserves_source_associations
                || summary.writes.opaque_variable_frame
                || !self.operands_preserve_source_associations(&command.words, 0)
            {
                remote_source_trace!(
                    "REMOTE_FIRST_OPERAND stage=continuity offset={offset} preserves={} opaque={} writes={}",
                    summary.preserves_source_associations,
                    summary.writes.opaque_variable_frame,
                    summary.writes.literal_names.len()
                );
                break;
            }
            for name in summary.writes.literal_names {
                let Some(key) = self.key(name.as_bytes()) else {
                    bindings.clear();
                    return Some(());
                };
                bindings.retain(|(owned, _)| owned != &key);
            }
        }
        Some(())
    }

    fn first_remote_operand(
        &self,
        words: &OriginalRegistryWords,
        layout: RemoteCallSpec,
        bindings: &[(String, Arc<OriginalSourceRemoteResultAssociation>)],
        body_path: &[OriginalRegistryWords],
    ) -> Option<OriginalSourceRemoteFirstOperandAssociation> {
        let handle_at = usize::from(layout.handle_arg);
        if words
            .arguments()
            .iter()
            .take(handle_at)
            .all(|word| word.literal_bytes().is_some())
            && let Some(operand) = words.original_argument_word(handle_at)
            && let Some(key) = self.read_key(operand)
            && let Some((_, binding)) = bindings.iter().find(|(owned, _)| *owned == key)
            && binding.constructor_layout.family == layout.family
            && words.with_source_schema(self.context, |schema| {
                schema.facts().arity_accepts_frozen_arguments()
            }) == Some(Some(true))
        {
            return Some(OriginalSourceRemoteFirstOperandAssociation {
                invocation: words.clone(),
                layout,
                handle_operand: operand.clone(),
                binding: Arc::clone(binding),
                body_path: body_path.to_vec(),
            });
        }
        None
    }

    fn setter_keyword_matches(
        setter: &OriginalRegistryWords,
        layout: &HandleBindingSpec,
    ) -> Option<()> {
        if let Some(keyword) = layout.keyword
            && setter
                .arguments()
                .get(usize::from(keyword.at))?
                .literal_bytes()?
                != keyword.word.as_bytes()
        {
            return None;
        }
        Some(())
    }

    fn constructor_operands_match(
        &self,
        original: &[NativeWord],
        constructor: &OriginalRegistryWords,
        constructor_layout: RemoteHandleSpec,
    ) -> Option<()> {
        if original.len().checked_sub(1)? != usize::from(constructor_layout.exact_argc)
            || constructor.with_source_schema(self.context, |schema| {
                schema.facts().arity_accepts_frozen_arguments()
            }) != Some(Some(true))
            || original.iter().any(|word| word.group().expand)
            || constructor
                .arguments()
                .iter()
                .any(|argument| argument.literal_bytes().is_none())
        {
            return None;
        }
        for ordinal in [
            constructor_layout.scope_arg,
            constructor_layout.extension_arg,
        ] {
            let value = constructor
                .arguments()
                .get(usize::from(ordinal))?
                .literal_bytes()?;
            if value.is_empty() || value.contains(&0) || std::str::from_utf8(value).is_err() {
                return None;
            }
        }
        Some(())
    }

    fn setter_operands_refuse(
        &self,
        summary: &SourceSummary,
        key: &str,
        pending: &OriginalSourceRemoteResultAssociation,
        original: &[NativeWord],
    ) -> bool {
        !summary.admits_source_setter
            || summary.writes.opaque_variable_frame
            || summary.writes.literal_names.len() != 1
            || self
                .key(summary.writes.literal_names[0].as_bytes())
                .as_deref()
                != Some(key)
            || original.iter().any(|word| {
                word != &pending.operand
                    && !word
                        .executable_parts()
                        .all_parts()
                        .all(|part| matches!(part.part, ExecutablePart::Text(_)))
            })
    }

    fn single_constructor_plan(
        &self,
        body: tcl_lexer::Span,
    ) -> Option<tcl_lexer::NativeScriptWordsPlan> {
        let plan = tcl_lexer::native_script_words_in_with_purpose(
            self.image.clone(),
            body,
            self.config.nested(),
            tcl_lexer::SourceSegmentationPurpose::NestedScript,
        )
        .ok()?;
        if plan.fatal_tail.is_some() || plan.commands.len() != 1 {
            return None;
        }
        Some(plan)
    }

    fn selected_setter_rhs_script(
        &self,
        words: &OriginalRegistryWords,
        summary: &SourceSummary,
    ) -> Option<tcl_lexer::Span> {
        if !summary.admits_source_setter
            || summary.writes.opaque_variable_frame
            || summary.writes.literal_names.len() != 1
            || words.arguments().iter().enumerate().any(|(index, _)| {
                words
                    .original_argument_word(index)
                    .is_none_or(|word| word.group().expand)
            })
        {
            return None;
        }
        let layout = words.with_source_schema(self.context, |schema| {
            schema
                .authored_source_descriptors()
                .command
                .binds_handle
                .copied()
        })??;
        let HandleName::Word(name_at) = layout.name_from else {
            return None;
        };
        let HandleClassSource::ConstructionValue(value_at) = layout.class_from else {
            return None;
        };
        if let Some(keyword) = layout.keyword
            && words
                .arguments()
                .get(usize::from(keyword.at))?
                .literal_bytes()?
                != keyword.word.as_bytes()
        {
            return None;
        }
        let receiver = words
            .arguments()
            .get(usize::from(name_at))?
            .literal_bytes()?;
        if self.key(receiver)? != self.key(summary.writes.literal_names[0].as_bytes())? {
            return None;
        }
        let operand = words.original_argument_word(usize::from(value_at))?;
        if words.arguments().iter().enumerate().any(|(index, _)| {
            words.original_argument_word(index).is_none_or(|word| {
                word != operand
                    && !word
                        .executable_parts()
                        .all_parts()
                        .all(|part| matches!(part.part, ExecutablePart::Text(_)))
            })
        }) {
            return None;
        }
        let arena = operand.executable_parts();
        let [part] = arena.list(arena.root()) else {
            return None;
        };
        let ExecutablePart::Command { body } = part.part else {
            return None;
        };
        let plan = tcl_lexer::native_script_words_in_with_purpose(
            self.image.clone(),
            body,
            self.config.nested(),
            tcl_lexer::SourceSegmentationPurpose::NestedScript,
        )
        .ok()?;
        (plan.fatal_tail.is_none() && plan.commands.len() == 1).then_some(body)
    }

    fn selected_true_same_frame_body(
        &self,
        words: &OriginalRegistryWords,
    ) -> Option<tcl_lexer::Span> {
        words
            .with_source_schema(self.context, |schema| {
                use tcl_registry::script_body_flow::{
                    ScriptBodyFlow, script_body_flow_for_invocation,
                };
                let facts = schema.facts();
                if facts.operation
                    != tcl_registry::SemanticOperationId::StructuredLowering(
                        tcl_registry::hooks::LoweringHookId::If,
                    )
                    || !intrinsic_conditional_has_no_authored_overrides(
                        &schema.semantics.world_effects,
                        schema.semantics.state_transitions,
                    )
                    || facts.traits.intersects(
                        tcl_registry::Traits::CREATES_SCOPE_ALIAS
                            | tcl_registry::Traits::DESTROYS_VARIABLE
                            | tcl_registry::Traits::ESTABLISHES_VARIABLE_TRACE,
                    )
                    || schema.semantics.frame_effect.is_some()
                    || schema.semantics.body_kind != tcl_registry::BodyKind::Plain
                    || schema.semantics.body_interpreter != tcl_registry::BodyInterpreter::Current
                    || facts.arity_accepts_frozen_arguments() != Some(true)
                    || facts.source_callback_retention()
                        != tcl_registry::resolved_invocation::SourceCallbackRetention::Absent
                {
                    return None;
                }
                let ScriptBodyFlow::Conditional(branches) =
                    script_body_flow_for_invocation(&facts, schema.words.arguments())
                else {
                    return None;
                };
                let [(Some(condition), body)] = branches.as_slice() else {
                    return None;
                };
                // Deliberately bounded positive: the selected conditional grammar
                // owns this literal constant and sole caller-frame body. No command
                // name dispatch, arbitrary boolean evaluation or branch join occurs.
                if words.arguments().get(*condition)?.literal_bytes()? != b"1" {
                    return None;
                }
                if schema.authored_source_script_timing_at(*body)?
                    != tcl_registry::ScriptTiming::SameInvocation
                    || words
                        .arguments()
                        .iter()
                        .enumerate()
                        .any(|(argument, word)| argument != *body && word.literal_bytes().is_none())
                {
                    return None;
                }
                let original = words.original_argument_word(*body)?;
                if original.group().expand || original.group().kind != tcl_lexer::WordKind::Braced {
                    return None;
                }
                original.content_span().ok()
            })
            .flatten()
    }

    fn words_at(&self, offset: u32, original: &[NativeWord]) -> Option<OriginalRegistryWords> {
        let words = remote_source_guard!(
            super::source_structure::source_registry_words_at(self.source, self.analysis, offset),
            "registry-words",
            Some(offset)
        )?;
        let OriginalRegistrySource::Vendor(metadata) = words.source() else {
            remote_source_trace!("REMOTE_FIRST_OPERAND stage=vendor-source-kind offset={offset}");
            return None;
        };
        let shape = metadata.shape();
        if !words.matches_source(self.image, self.config)
            || !words.matches_registry(self.context.commands())
            || words.context() != Some(self.context.context())
            || metadata
                .authored_barriers()
                .has_nominal_command_name_hazard()
            || shape.original_words().len() != original.len()
            || !shape
                .original_words()
                .iter()
                .zip(original)
                .all(|(owned, lexical)| {
                    owned.image() == lexical.image()
                        && owned.span() == lexical.span()
                        && owned.config().normalized() == lexical.config().normalized()
                        && owned.try_text().ok() == lexical.try_text().ok()
                })
        {
            remote_source_trace!(
                "REMOTE_FIRST_OPERAND stage=vendor-word-currentness offset={offset}"
            );
            return None;
        }
        Some(words)
    }

    fn key(&self, name: &[u8]) -> Option<String> {
        tcl_syntax::naming::vendor_scalar_source_reference(
            self.body.body_input().name_input().policy(),
            name,
            self.config,
        )
    }

    fn read_key(&self, word: &NativeWord) -> Option<String> {
        if word.group().expand {
            return None;
        }
        let arena = word.executable_parts();
        let [part] = arena.list(arena.root()) else {
            return None;
        };
        let ExecutablePart::Variable { name, index: None } = part.part else {
            return None;
        };
        self.key(arena.bytes(name)?)
    }

    fn binding(
        &self,
        setter: &OriginalRegistryWords,
    ) -> Option<(String, OriginalSourceRemoteResultAssociation)> {
        let layout = setter.with_source_schema(self.context, |schema| {
            schema
                .authored_source_descriptors()
                .command
                .binds_handle
                .copied()
        })??;
        let HandleName::Word(variable_at) = layout.name_from else {
            return None;
        };
        let HandleClassSource::ConstructionValue(value_at) = layout.class_from else {
            return None;
        };
        Self::setter_keyword_matches(setter, &layout)?;
        let variable_word = setter.original_argument_word(usize::from(variable_at))?;
        let mut advice = self
            .analysis
            .original_vendor_variable_advice()
            .filter(|advice| {
                advice.home() == Some(self.body)
                    && advice
                        .input()
                        .original_occurrence()
                        .name_input()
                        .original_word()
                        == variable_word
                    && advice.metadata().context() == self.context.context()
                    && advice.metadata().registry()
                        == &self.context.commands().snapshot().semantic_key()
            });
        let variable = remote_source_guard!(
            advice.next(),
            "setter-variable-owner",
            Some(variable_word.span().start())
        )?;
        if advice.next().is_some() {
            remote_source_trace!(
                "REMOTE_FIRST_OPERAND stage=ambiguous-setter-variable offset={}",
                variable_word.span().start()
            );
            return None;
        }
        let key = remote_source_guard!(
            vendor_variable_source_reference(variable),
            "setter-variable-key",
            Some(variable_word.span().start())
        )?;
        let operand = setter.original_argument_word(usize::from(value_at))?;
        let arena = operand.executable_parts();
        let [part] = arena.list(arena.root()) else {
            return None;
        };
        let ExecutablePart::Command { body } = part.part else {
            return None;
        };
        let plan = self.single_constructor_plan(body)?;
        let original = &plan.commands.first()?.words;
        let constructor = remote_source_guard!(
            self.words_at(original.first()?.span().start(), original),
            "constructor-words",
            Some(body.start())
        )?;
        let selected = remote_source_guard!(
            self.summary(&constructor),
            "constructor-summary",
            Some(body.start())
        )?;
        let RemoteMethodRole::OpensHandle(constructor_layout) =
            remote_source_guard!(selected.remote, "constructor-role", Some(body.start()))?
        else {
            remote_source_trace!(
                "REMOTE_FIRST_OPERAND stage=constructor-not-opener offset={}",
                body.start()
            );
            return None;
        };
        self.constructor_operands_match(original, &constructor, constructor_layout)?;
        Some((
            key,
            OriginalSourceRemoteResultAssociation {
                input: self.input.clone(),
                analysis_owner: Arc::clone(
                    self.analysis
                        .retained_command_realm()?
                        .source_bindings_ref()
                        .original_analysis_owner()?,
                ),
                body: self.body.clone(),
                variable: variable.clone(),
                setter: setter.clone(),
                setter_layout: layout,
                operand: operand.clone(),
                constructor,
                constructor_layout,
            },
        ))
    }

    fn operands_preserve_source_associations(&self, original: &[NativeWord], depth: usize) -> bool {
        if depth > 32 {
            return false;
        }
        for word in original {
            if word.group().expand {
                return false;
            }
            let arena = word.executable_parts();
            for part in arena.all_parts() {
                match part.part {
                    ExecutablePart::Text(_) => {}
                    ExecutablePart::Variable { .. } => return false,
                    ExecutablePart::Expression { .. } | ExecutablePart::ParseError(_) => {
                        return false;
                    }
                    ExecutablePart::Command { body } => {
                        let Ok(plan) = tcl_lexer::native_script_words_in_with_purpose(
                            self.image.clone(),
                            body,
                            self.config.nested(),
                            tcl_lexer::SourceSegmentationPurpose::NestedScript,
                        ) else {
                            return false;
                        };
                        if plan.fatal_tail.is_some() {
                            return false;
                        }
                        for child in plan.commands {
                            let Some(offset) = child.words.first().map(|word| word.span().start())
                            else {
                                return false;
                            };
                            let Some(words) = self.words_at(offset, &child.words) else {
                                return false;
                            };
                            let Some(summary) = self.summary(&words) else {
                                return false;
                            };
                            if !summary.preserves_source_associations
                                || summary.writes.opaque_variable_frame
                                || !summary.writes.literal_names.is_empty()
                                || !self
                                    .operands_preserve_source_associations(&child.words, depth + 1)
                            {
                                return false;
                            }
                        }
                    }
                }
            }
        }
        true
    }

    fn summary(&self, words: &OriginalRegistryWords) -> Option<SourceSummary> {
        words.with_source_schema(self.context, |schema| {
            use tcl_registry::resolved_invocation::SourceCallbackRetention;
            use tcl_registry::world_effect::{CallbackKinds, EffectAccessMode, InterpreterScope, Reentrancy, WorldStateDomain};
            let facts = schema.facts();
            let writes = tcl_registry::CommandRegistry::variable_write_projection_for_selected_source(schema);
            // Vendor words deliberately carry no Native invocation dialect.
            // This source-only effect projection uses the same complete input
            // already retained by the current body and invocation owners.
            let mut source_dialect = tcl_registry::InvocationDialect::of_profile(self.input.unit_profile());
            source_dialect.lexer_grammar = self.config.grammar_over(source_dialect.lexer_grammar);
            source_dialect.word_values = tcl_syntax::word_rules::WordValueRules::from_config(&self.config);
            let (effects, transitions) = schema.authored_source_effects_and_state_transitions(source_dialect);
            let callback = effects.callback();
            let callbacks_supported = (callback.kinds == CallbackKinds::NONE && callback.reentrancy == Reentrancy::Never)
                || (callback.kinds == CallbackKinds::TRACE && callback.reentrancy == Reentrancy::CurrentInterpreter);
            let source_domains_supported = effects.accesses().iter().all(|access| {
                access.interpreter == InterpreterScope::Current
                    && (access.mode == EffectAccessMode::Read
                    || (access.mode != EffectAccessMode::Clobber
                        && matches!(access.domain,
                            WorldStateDomain::InterpreterResult
                            | WorldStateDomain::VariableStore
                            | WorldStateDomain::LegacyExternal(tcl_registry::side_effects::SideEffectTarget::NetworkIo))))
            });
            let admits_source_setter = schema.semantics.world_effects.is_declared()
                && source_domains_supported
                && source_effect_chain_has_no_unclassified_override(&schema.semantics.world_effects)
                && !schema.semantics.world_effects.has_uncovered_variable_role_writes(
                    schema.words.arguments(), !writes.literal_names.is_empty() || writes.opaque_variable_frame)
                && schema.authored_source_invocation_completion()
                    == tcl_registry::registry::InvocationCompletion::FallsThrough
                && matches!(tcl_registry::script_body_flow::script_body_flow_for_invocation(&facts, schema.words.arguments()),
                    tcl_registry::script_body_flow::ScriptBodyFlow::None)
                && schema.semantics.body_kind == tcl_registry::BodyKind::Plain
                && schema.semantics.body_interpreter == tcl_registry::BodyInterpreter::Current
                && facts.arity_accepts_frozen_arguments() == Some(true)
                && !facts.traits.intersects(
                    tcl_registry::Traits::CREATES_SCOPE_ALIAS
                        | tcl_registry::Traits::DESTROYS_VARIABLE
                        | tcl_registry::Traits::ESTABLISHES_VARIABLE_TRACE
                        | tcl_registry::Traits::CONTROL_FLOW,
                )
                && facts.source_callback_retention() == SourceCallbackRetention::Absent
                && callbacks_supported
                && !effects.legacy().command_table_mutation
                && !transitions.widens(tcl_registry::StateTransitionDomain::CommandBindings)
                && !transitions.widens(tcl_registry::StateTransitionDomain::CommandResolution)
                && transitions.facts().is_empty()
                && schema.semantics.frame_effect.is_none()
                && schema.authored_source_executable_value_capability()
                    == tcl_registry::resolved_invocation::SourceExecutableValueCapability::Absent;
            let preserves_source_associations = admits_source_setter
                && callback.kinds == CallbackKinds::NONE
                && callback.reentrancy == Reentrancy::Never;
            Some(SourceSummary {
                admits_source_setter,
                preserves_source_associations,
                writes,
                remote: schema.authored_source_descriptors().command.remote_method.copied(),
            })
        }).flatten()
    }
}

// Intrinsic conditional source ordering owns the selected literal condition
// and body. Authored effect/transition overrides require a separate pre-body
// contract; the full invocation footprint cannot subtract body effects.
fn intrinsic_conditional_has_no_authored_overrides(
    effects: &tcl_registry::world_effect::ResolvedWorldEffects,
    transitions: tcl_registry::state_transition::ResolvedStateTransitions,
) -> bool {
    source_effect_chain_has_no_unclassified_override(effects)
        && effects.command.is_none()
        && effects.subcommand.is_none()
        && effects.form.is_none()
        && !transitions.is_declared()
}

fn source_effect_chain_has_no_unclassified_override(
    effects: &tcl_registry::world_effect::ResolvedWorldEffects,
) -> bool {
    // The ordinary uncovered-role projection does not inspect source_resolver.
    // Until its owner supplies source-specific coverage, refuse these chains,
    // including a source override shadowed by a more-specific replacement.
    [effects.command, effects.subcommand, effects.form]
        .into_iter()
        .flatten()
        .all(|descriptor| descriptor.source_resolver.is_none())
}

struct SourceSummary {
    admits_source_setter: bool,
    preserves_source_associations: bool,
    writes: tcl_registry::VariableWriteProjection,
    remote: Option<RemoteMethodRole>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn analyse(source: &str) -> AnalysisResult {
        crate::analyser::Analyser::new().analyse(source, "f5-irules")
    }

    #[test]
    fn vendor_source_effect_summary_keeps_native_dialect_unavailable() {
        let source = "when HTTP_REQUEST {set h [ILX::init p e]; ILX::call $h m; ILX::notify $h n}";
        let analysis = analyse(source);
        let words = super::super::source_structure::source_registry_words_at(source, &analysis, 19)
            .expect("actual original source setter");
        assert!(matches!(words.source(), OriginalRegistrySource::Vendor(_)));
        assert!(
            words.dialect().is_none(),
            "source profile is not a Native word dialect"
        );
        let calls = source_remote_first_operand_associations(source, &analysis).unwrap();
        assert_eq!(calls.len(), 1, "the later opaque notify is never rescued");
        assert!(calls[0].matches_source_analysis(source, &analysis));
        let mut changed_config = analysis.clone();
        let mut config = analysis.body_lexer_config.unwrap();
        config.irules_brace_separator = !config.irules_brace_separator;
        changed_config.body_lexer_config = Some(config);
        assert!(!calls[0].matches_source_analysis(source, &changed_config));
        assert!(source_remote_first_operand_associations(source, &changed_config).is_none());
        let foreign = analyse(source);
        assert!(!calls[0].matches_source_analysis(source, &foreign));
        let stale = source.replace(" p e", " q e");
        assert!(!calls[0].matches_source_analysis(&stale, &analysis));
        assert!(source_remote_first_operand_associations(&stale, &analysis).is_none());
        let plain = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        assert!(!calls[0].matches_source_analysis(source, &plain));
        assert!(source_remote_first_operand_associations(source, &plain).is_none());
        let mut mixed = analysis.clone();
        mixed.resolved_input = plain.resolved_input.clone();
        assert!(!calls[0].matches_source_analysis(source, &mixed));
        assert!(source_remote_first_operand_associations(source, &mixed).is_none());
    }

    #[test]
    fn intrinsic_conditional_refuses_source_effect_and_transition_overrides() {
        use tcl_registry::state_transition::{ResolvedStateTransitions, StateTransitionDescriptor};
        use tcl_registry::world_effect::{
            EffectAccess, EffectAccessMode, EffectFootprint, ResolvedWorldEffects,
            SourceWorldEffectResolver, WorldEffectComposition, WorldEffectDescriptor,
        };
        fn source_write(_: tcl_registry::InvocationArguments<'_>) -> EffectFootprint {
            let mut effects = EffectFootprint::default();
            effects.add_access(EffectAccess::variable_store_wildcard(
                EffectAccessMode::Write,
            ));
            effects
        }
        let descriptor = WorldEffectDescriptor {
            source_resolver: Some(SourceWorldEffectResolver::Effects(source_write)),
            ..WorldEffectDescriptor::EMPTY
        };
        assert!(
            !source_write(tcl_registry::InvocationArguments::structured(&[]))
                .accesses()
                .is_empty()
        );
        assert!(intrinsic_conditional_has_no_authored_overrides(
            &ResolvedWorldEffects::default(),
            ResolvedStateTransitions::default()
        ));
        for effects in [
            ResolvedWorldEffects {
                command: Some(descriptor),
                ..Default::default()
            },
            ResolvedWorldEffects {
                subcommand: Some(descriptor),
                ..Default::default()
            },
            ResolvedWorldEffects {
                form: Some(descriptor),
                ..Default::default()
            },
            ResolvedWorldEffects {
                command: Some(descriptor),
                form: Some(WorldEffectDescriptor {
                    composition: WorldEffectComposition::Replace,
                    ..WorldEffectDescriptor::EMPTY
                }),
                ..Default::default()
            },
            ResolvedWorldEffects {
                command: Some(WorldEffectDescriptor::EMPTY),
                ..Default::default()
            },
        ] {
            assert!(!intrinsic_conditional_has_no_authored_overrides(
                &effects,
                ResolvedStateTransitions::default()
            ));
        }
        for transitions in [
            ResolvedStateTransitions {
                command: Some(StateTransitionDescriptor::EMPTY),
                ..Default::default()
            },
            ResolvedStateTransitions {
                subcommand: Some(StateTransitionDescriptor::EMPTY),
                ..Default::default()
            },
            ResolvedStateTransitions {
                form: Some(StateTransitionDescriptor::EMPTY),
                ..Default::default()
            },
        ] {
            assert!(!intrinsic_conditional_has_no_authored_overrides(
                &ResolvedWorldEffects::default(),
                transitions
            ));
        }
    }

    #[test]
    fn first_remote_operand_retains_exact_source_owners_and_stops_at_the_call() {
        let source =
            "when HTTP_REQUEST {set h [ILX::init p e]; ILX::call $h m; ILX::notify ${h} n}";
        let analysis = analyse(source);
        let calls = source_remote_first_operand_associations(source, &analysis).unwrap();
        assert_eq!(calls.len(), 1);
        let call = &calls[0];
        assert!(call.matches_source_analysis(source, &analysis));
        assert!(!call.matches_source_analysis(source, &analyse(source)));
        let binding = call.handle_binding();
        assert_eq!(
            binding.source_analysis_input(),
            analysis.resolved_input.as_ref().unwrap()
        );
        assert_eq!(
            binding.constructor().arguments()[0].literal_bytes(),
            Some(b"p".as_slice())
        );
        assert_eq!(
            binding.constructor().arguments()[1].literal_bytes(),
            Some(b"e".as_slice())
        );
        assert_eq!(binding.variable().home(), Some(binding.source_body()));
        assert_eq!(
            binding.original_operand().try_text().unwrap(),
            "[ILX::init p e]"
        );
        assert!(
            binding
                .source_body()
                .contains_cursor(call.handle_operand().span().start())
        );
        assert!(
            call.obligations()
                .contains(&RemoteResultSourceObligation::ConstructorEffectsUnavailable)
        );
        assert!(
            call.obligations()
                .contains(&RemoteResultSourceObligation::VariableCellAndObserversUnavailable)
        );
        assert!(call.invocation().original_argument_word(0).is_some());
        let mut unavailable = analysis.clone();
        unavailable.resolved_input = None;
        assert!(!call.matches_source_analysis(source, &unavailable));
        assert!(source_remote_first_operand_associations(source, &unavailable).is_none());
        assert!(
            source_remote_first_operand_associations(&source.replace(" p e", " q e"), &analysis)
                .is_none()
        );
    }

    #[test]
    fn first_remote_operand_in_setter_rhs_precedes_outer_store_and_later_opaque_call() {
        for first in [
            "ILX::call $h m",
            "set reply [ILX::call $h m]",
            "ILX::call $h m [HTTP::uri]",
            "set reply [ILX::call $h m [HTTP::uri]]",
        ] {
            let source = format!(
                "when HTTP_REQUEST {{set h [ILX::init p e]; {first}; ILX::notify $h later}}"
            );
            let analysis = analyse(&source);
            let calls = source_remote_first_operand_associations(&source, &analysis).unwrap();
            assert_eq!(calls.len(), 1, "{first}");
            let first_offset = u32::try_from(source.find("ILX::call").unwrap()).unwrap();
            let current = super::super::source_structure::source_registry_words_at(
                &source,
                &analysis,
                first_offset,
            )
            .unwrap();
            assert_eq!(calls[0].invocation(), &current);
            assert!(calls[0].matches_source_analysis(&source, &analysis));
            assert!(
                calls[0].handle_operand().span().start()
                    < u32::try_from(source.find("ILX::notify").unwrap()).unwrap()
            );
            assert_eq!(calls[0].obligations().len(), 5);
        }
        for middle in [
            "unknown_command; set reply [ILX::call $h m]",
            "set reply prefix[ILX::call $h m]",
            "set reply [unknown_command; ILX::call $h m]",
            "set $receiver [ILX::call $h m]",
            "set reply {*}[ILX::call $h m]",
        ] {
            let source = format!("when HTTP_REQUEST {{set h [ILX::init p e]; {middle}}}");
            let analysis = analyse(&source);
            assert!(
                source_remote_first_operand_associations(&source, &analysis)
                    .unwrap()
                    .is_empty(),
                "{middle}"
            );
        }
    }

    #[test]
    fn first_remote_operand_preserves_supported_true_same_frame_source_body() {
        for source in [
            "when HTTP_REQUEST {set h [ILX::init p e]; if {1} {ILX::call $h m}}",
            "when HTTP_REQUEST {if {1} {set h [ILX::init p e]; ILX::notify $h m}}",
            "proc invoke {} {set h [ILX::init p e]; ILX::call $h m}",
        ] {
            let analysis = analyse(source);
            let calls = source_remote_first_operand_associations(source, &analysis).unwrap();
            assert_eq!(calls.len(), 1);
            assert!(calls[0].matches_source_analysis(source, &analysis));
            if !source.starts_with("proc ") {
                assert!(!calls[0].body_path.is_empty());
            }
        }
    }

    #[test]
    fn first_remote_operand_withdraws_opaque_observer_write_and_foreign_body_paths() {
        for middle in [
            "ILX::call $h earlier;",
            "ILX::notify $h earlier;",
            "unknown_command;",
            "return;",
            "break;",
            "continue;",
            "set h other;",
            "unset h;",
            "incr h;",
            "set $which other;",
            "trace add variable h read hook;",
            "upvar 0 other h;",
            "if {$condition} {set h other};",
            "set other [unknown_command];",
            "rename ILX::call other;",
            "string length $trigger;",
        ] {
            let source =
                format!("when HTTP_REQUEST {{set h [ILX::init p e]; {middle} ILX::call $h m}}");
            let calls =
                source_remote_first_operand_associations(&source, &analyse(&source)).unwrap();
            if middle.starts_with("ILX::") {
                assert_eq!(calls.len(), 1);
                assert!(
                    calls[0].handle_operand().span().start()
                        < u32::try_from(source.rfind("ILX::call $h m").unwrap()).unwrap()
                );
            } else {
                assert!(calls.is_empty(), "{middle}");
            }
        }
        for source in [
            "when HTTP_REQUEST {unknown_command; set h [ILX::init p e]; ILX::call $h m}",
            "when HTTP_REQUEST {trace add variable h write hook; set h [ILX::init p e]; ILX::call $h m}",
            "when RULE_INIT {set h [ILX::init p e]}; when HTTP_REQUEST {ILX::call $h m}",
            "set h [ILX::init p e]; when HTTP_REQUEST {ILX::call $h m}",
            "when HTTP_REQUEST {set h prefix[ILX::init p e]; ILX::call $h m}",
            "when HTTP_REQUEST {set h [ILX::init p e; unknown_command]; ILX::call $h m}",
            "when HTTP_REQUEST {set h [ILX::init {*}$labels]; ILX::call $h m}",
            "when HTTP_REQUEST {set h [ILX::init $plugin e]; ILX::call $h m}",
            "when HTTP_REQUEST {set h [ILX::init p e]; ILX::call $a(h) m}",
            "when HTTP_REQUEST {set a(h) [ILX::init p e]; ILX::call $a(h) m}",
            "when HTTP_REQUEST {set h [ILX::init p e]; ILX::call prefix$h m}",
        ] {
            assert!(
                source_remote_first_operand_associations(source, &analyse(source))
                    .unwrap()
                    .is_empty()
            );
        }
    }
}
