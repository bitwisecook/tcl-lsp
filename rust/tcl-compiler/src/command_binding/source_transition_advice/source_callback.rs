// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original callback targets retain their registration source horizon.

use super::{
    AdviceCell, AdviceGraph, AdviceInvocation, AdviceInvocationContext,
    OriginalSourceCommandTransition, OriginalSourceCommandTransitionAdvice,
    OriginalSourceTransitionAdviceTape, SourceAdviceNameInput, SourceAdviceWord,
    SourceCommandTransitionObligation,
};
use crate::analyser::{AnalysisResult, ProcDef};
use crate::command_binding::OriginalCallbackPrefix;
use crate::registry_invocation::{EffectiveInvocationWord, InvocationWordOrigin};
use crate::signature_scan::original_name::SourceDeclarationMetadata;
use std::sync::Arc;
use tcl_core_types::ByteCommandSlot;
use tcl_lexer::{LexerConfig, NativeWord, SourceImage};
use tcl_registry::{ScriptLookupScope, model::ContextRegistry};
use tcl_syntax::naming::{NativeNameContext, NativeNameProtocol, NativeNameQualification};

/// Whole genuine source installer and its selected command or method schema.
/// This records conditional source registration, not an installed callback,
/// Native lookup, successful factory, future frame or executed argument vector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourceCallbackRegistration {
    source: CallbackRegistrationSource,
    builder: Option<Arc<OriginalSourceCommandTransitionAdvice>>,
    obligations: Vec<SourceCommandTransitionObligation>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
enum CallbackRegistrationSource {
    Command(Arc<OriginalSourceCommandTransitionAdvice>),
    Instance(Arc<super::OriginalSourceRegisteredInstanceWords>),
}
impl OriginalSourceCallbackRegistration {
    /// Actual installer site, independently of captured operand producer sites.
    #[must_use]
    pub fn site(&self) -> &super::super::CommandAllocationSite {
        match &self.source {
            CallbackRegistrationSource::Command(command) => command.site(),
            CallbackRegistrationSource::Instance(instance) => instance.site(),
        }
    }
    /// Complete current written installer vector, including its original head.
    #[must_use]
    pub fn original_words(&self) -> &[NativeWord] {
        match &self.source {
            CallbackRegistrationSource::Command(command) => command.original_words(),
            CallbackRegistrationSource::Instance(instance) => instance.original_words(),
        }
    }
    /// Genuine selected command source schema; method receivers remain separate.
    #[must_use]
    pub fn source_command(&self) -> Option<&OriginalSourceCommandTransitionAdvice> {
        match &self.source {
            CallbackRegistrationSource::Command(command) => Some(command),
            CallbackRegistrationSource::Instance(_) => None,
        }
    }
    /// Genuine source method and its independent factory/handle ancestry.
    #[must_use]
    pub fn source_instance(&self) -> Option<&super::OriginalSourceRegisteredInstanceWords> {
        match &self.source {
            CallbackRegistrationSource::Command(_) => None,
            CallbackRegistrationSource::Instance(instance) => Some(instance),
        }
    }
    /// Independently selected original prefix builder, when the installer
    /// operand is a whole single-command substitution. No returned value or
    /// actual builder execution is supplied by this source receipt.
    #[must_use]
    pub fn source_builder(&self) -> Option<&OriginalSourceCommandTransitionAdvice> {
        self.builder.as_deref()
    }
    /// All source applicability, factory, operand and future-callback premises.
    #[must_use]
    pub fn obligations(&self) -> &[SourceCommandTransitionObligation] {
        &self.obligations
    }
    /// Complete source/configuration and actual selected metadata context agree.
    #[must_use]
    pub fn matches_source_context(
        &self,
        image: &SourceImage,
        config: LexerConfig,
        context: &ContextRegistry,
    ) -> bool {
        self.builder.as_ref().is_none_or(|builder| {
            builder.matches_source(image, config) && builder.matches_context(context)
        }) && match &self.source {
            CallbackRegistrationSource::Command(command) => {
                command.matches_source(image, config) && command.matches_context(context)
            }
            CallbackRegistrationSource::Instance(instance) => {
                instance.matches_source_context(image, config, context)
            }
        }
    }
    fn from_command(command: OriginalSourceCommandTransitionAdvice) -> Self {
        let mut obligations = command.obligations().to_vec();
        obligations.push(SourceCommandTransitionObligation::OriginalCallbackTargetApplicability);
        Self {
            source: CallbackRegistrationSource::Command(Arc::new(command)),
            builder: None,
            obligations,
        }
    }
    fn from_instance(instance: &super::OriginalSourceRegisteredInstanceWords) -> Self {
        let mut obligations = instance.obligations().to_vec();
        obligations.push(SourceCommandTransitionObligation::OriginalCallbackTargetApplicability);
        Self {
            source: CallbackRegistrationSource::Instance(Arc::new(instance.clone())),
            builder: None,
            obligations,
        }
    }
}

/// A source callback lookup retains selected targets and explicit refusal.
/// External headers cannot cross a known source deletion or nonprocedure cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourceCallbackProcedureLookup {
    registration: Arc<OriginalSourceCallbackRegistration>,
    prefix: OriginalCallbackPrefix,
    selection: Option<ByteCommandSlot>,
    outcome: CallbackProcedureOutcome,
}
#[derive(Debug, Clone, PartialEq, Eq)]
enum CallbackProcedureOutcome {
    Target(Box<OriginalSourceCallbackProcedureTarget>),
    Refusal(OriginalSourceCallbackProcedureRefusal),
}
/// Separate source horizons that cannot be reduced to a failed nominal lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OriginalSourceCallbackProcedureRefusal {
    /// No local source target supplies a signature; an independent external
    /// original source header may be considered under genuine lookup geometry.
    UnknownSourceTarget,
    /// The required callback lookup frame has no supported source coordinates.
    UnavailableScope,
    /// A known deletion, replacement, nonprocedure cell or cycle blocks lookup.
    KnownSourceBarrier,
    /// Callback execution belongs to a different or unclassified interpreter.
    ExternalInterpreter,
    /// Genuine scans retain incompatible conditional source target horizons.
    AmbiguousSourceTarget,
}
/// Selected local declaration and held external source name stay distinct.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OriginalSourceCallbackProcedureTargetKind {
    /// An authentic local source procedure declaration is retained.
    LocalProcedure,
    /// Authored alias lineage supplies a held target name without a local body.
    ExternalSourceName,
}
impl OriginalSourceCallbackProcedureLookup {
    /// Complete authentic source registration, independently of callback reach.
    #[must_use]
    pub fn registration(&self) -> &OriginalSourceCallbackRegistration {
        &self.registration
    }
    /// Readonly original callback head and its selected append/scope descriptor.
    #[must_use]
    pub const fn prefix(&self) -> &OriginalCallbackPrefix {
        &self.prefix
    }
    /// Selected conditional target, absent for an explicit refusal.
    #[must_use]
    pub fn target(&self) -> Option<&OriginalSourceCallbackProcedureTarget> {
        match &self.outcome {
            CallbackProcedureOutcome::Target(target) => Some(target),
            CallbackProcedureOutcome::Refusal(_) => None,
        }
    }
    /// Structured source refusal, without reparsing a diagnostic message.
    #[must_use]
    pub const fn refusal(&self) -> Option<OriginalSourceCallbackProcedureRefusal> {
        match self.outcome {
            CallbackProcedureOutcome::Refusal(refusal) => Some(refusal),
            CallbackProcedureOutcome::Target(_) => None,
        }
    }
    /// Only an unknown local source target permits a separate genuine external
    /// source-header lookup. This supplies no external declaration itself.
    #[must_use]
    pub const fn permits_external_signature_lookup(&self) -> bool {
        matches!(
            self.outcome,
            CallbackProcedureOutcome::Refusal(
                OriginalSourceCallbackProcedureRefusal::UnknownSourceTarget
            )
        )
    }
    /// All unresolved registration, target-lineage and future-callback premises.
    #[must_use]
    pub fn obligations(&self) -> &[SourceCommandTransitionObligation] {
        self.target().map_or_else(
            || self.registration.obligations(),
            OriginalSourceCallbackProcedureTarget::obligations,
        )
    }
    /// Full source/configuration/Registry match at the original registration.
    #[must_use]
    pub fn matches_source_context(
        &self,
        image: &SourceImage,
        config: LexerConfig,
        context: &ContextRegistry,
    ) -> bool {
        self.registration
            .matches_source_context(image, config, context)
            && self
                .target()
                .is_none_or(|target| target.matches_source_context(image, config, context))
    }
    pub(crate) fn matches_prefix(&self, prefix: &OriginalCallbackPrefix) -> bool {
        prefix
            .source_registration()
            .is_none_or(|registration| registration == self.registration.as_ref())
            && self.prefix.name_input() == prefix.name_input()
            && self.prefix.scope() == prefix.scope()
            && self.prefix.appended_arity() == prefix.appended_arity()
            && self.prefix.baked_argument_count() == prefix.baked_argument_count()
            && prefix.lookup().is_none_or(|lookup| {
                lookup.site() == self.registration.site()
                    && self.selection.as_ref().is_none_or(|selection| {
                        lookup
                            .candidates()
                            .iter()
                            .all(|path| path.contains(selection))
                    })
            })
    }
}

/// Genuine callback registration and its conditional original procedure target.
/// The selected source horizon supplies no reached callback or future dispatch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourceCallbackProcedureTarget {
    registration: Arc<OriginalSourceCallbackRegistration>,
    prefix: OriginalCallbackPrefix,
    target: SourceAdviceNameInput,
    slot: ByteCommandSlot,
    procedure: Option<Arc<super::source_procedure::OriginalSourceProcedureBody>>,
    captured: Vec<SourceAdviceWord>,
    lineage: Vec<Arc<OriginalSourceCommandTransition>>,
    obligations: Vec<SourceCommandTransitionObligation>,
}
impl OriginalSourceCallbackProcedureTarget {
    /// Explicit local declaration or authored held external target name.
    #[must_use]
    pub const fn kind(&self) -> OriginalSourceCallbackProcedureTargetKind {
        if self.procedure.is_some() {
            OriginalSourceCallbackProcedureTargetKind::LocalProcedure
        } else {
            OriginalSourceCallbackProcedureTargetKind::ExternalSourceName
        }
    }
    /// Whole original registration vector and independently selected schema.
    #[must_use]
    pub fn registration(&self) -> &OriginalSourceCallbackRegistration {
        &self.registration
    }
    /// Authentic readonly callback head, fixed operands and append descriptor.
    #[must_use]
    pub const fn prefix(&self) -> &OriginalCallbackPrefix {
        &self.prefix
    }
    /// Actual original target naming producer, separate from report names.
    #[must_use]
    pub const fn target_input(&self) -> &SourceAdviceNameInput {
        &self.target
    }
    /// Conditional target slot at the registration source horizon.
    #[must_use]
    pub const fn source_slot(&self) -> &ByteCommandSlot {
        &self.slot
    }
    /// Canonical original local procedure declaration, absent for external names.
    #[must_use]
    pub fn declaration(&self) -> Option<&OriginalSourceCommandTransitionAdvice> {
        Some(&self.procedure.as_ref()?.declaration)
    }
    /// Canonical local declaration name, independent of a moved target slot.
    #[must_use]
    pub fn declaration_name_input(&self) -> Option<&SourceAdviceNameInput> {
        Some(&self.procedure.as_ref()?.name)
    }
    /// Exact original declaration slot, independently of later moves. The
    /// returned coordinate indexes conditional source call-graph nodes only.
    #[must_use]
    pub fn declaration_slot(&self) -> Option<&ByteCommandSlot> {
        Some(&self.procedure.as_ref()?.declaration_slot)
    }
    pub(crate) fn original_ir_procedure_name<'a>(
        &self,
        procedures: &'a std::collections::HashMap<String, crate::ir::Procedure>,
    ) -> Option<&'a str> {
        let producer = self.procedure.as_ref()?;
        let body = producer.body.content_span();
        let image = producer.declaration.original_words().first()?.image();
        let bytes = image
            .bytes()
            .get(body.start() as usize..body.end() as usize)?;
        let mut matching = procedures.iter().filter(|(_, procedure)| {
            procedure.span.start() == producer.declaration.site().offset
                && procedure.body_offset == body.start()
                && procedure
                    .body_source
                    .as_ref()
                    .is_some_and(|text| text.as_bytes() == bytes)
        });
        let (name, _) = matching.next()?;
        matching.next().is_none().then_some(name.as_str())
    }
    /// Original alias/move producers supplying the source target and prefix.
    #[must_use]
    pub fn lineage(&self) -> &[Arc<OriginalSourceCommandTransition>] {
        &self.lineage
    }
    /// Captured original alias arguments, preserving unknown expansion length.
    #[must_use]
    pub fn captured_arguments(&self) -> Vec<EffectiveInvocationWord> {
        self.captured
            .iter()
            .map(|word| {
                if word.original.group().expand {
                    EffectiveInvocationWord::Expanded
                } else {
                    word.value.as_deref().map_or(
                        EffectiveInvocationWord::Dynamic,
                        EffectiveInvocationWord::from_bytes,
                    )
                }
            })
            .collect()
    }
    /// Whole original alias operand at the composed prefix ordinal.
    #[must_use]
    pub fn argument_word(&self, ordinal: usize) -> Option<&NativeWord> {
        Some(&self.captured.get(ordinal)?.original)
    }
    /// Original alias-allocation operand origin; composed captures keep their
    /// own producer ordinals rather than claiming one flattened Native argv.
    #[must_use]
    pub fn argument_origin(&self, ordinal: usize) -> Option<&InvocationWordOrigin> {
        Some(&self.captured.get(ordinal)?.origin)
    }
    /// Genuine source naming axis of one original captured alias operand.
    #[must_use]
    pub fn argument_input(&self, ordinal: usize) -> Option<&SourceAdviceNameInput> {
        self.captured.get(ordinal)?.input.as_ref()
    }
    /// Conditional source/execution obligations are retained independently.
    #[must_use]
    pub fn obligations(&self) -> &[SourceCommandTransitionObligation] {
        &self.obligations
    }
    /// Same complete source, lexical configuration and selected Registry.
    #[must_use]
    pub fn matches_source_context(
        &self,
        image: &SourceImage,
        config: LexerConfig,
        context: &ContextRegistry,
    ) -> bool {
        self.registration
            .matches_source_context(image, config, context)
            && self
                .captured
                .iter()
                .all(|word| word.original.image() == image && word.original.config() == config)
            && self
                .target
                .original_word()
                .or_else(|| {
                    Some(
                        self.target
                            .native_input()?
                            .original_static_list_container()?
                            .parent_word(),
                    )
                })
                .is_some_and(|word| word.image() == image && word.config() == config)
            && self.procedure.as_ref().is_none_or(|procedure| {
                procedure.declaration.matches_source(image, config)
                    && procedure.declaration.matches_context(context)
            })
    }
    /// Exact local original procedure metadata. Reporting names and final maps
    /// never replace the genuine declaration site and whole naming word join.
    #[must_use]
    pub fn source_procedure<'a>(
        &self,
        analysis: &'a AnalysisResult,
    ) -> Option<&'a SourceDeclarationMetadata<ProcDef>> {
        let input = analysis.resolved_input.as_ref()?;
        let original = self.declaration_name_input()?.original_word()?;
        self.matches_source_context(
            original.image(),
            input.lexer_config(),
            &input.context_registry(),
        )
        .then_some(())?;
        self.procedure.as_ref()?.source_procedure(analysis)
    }
}

impl OriginalSourceTransitionAdviceTape {
    pub(crate) fn callback_procedure_targets(
        &self,
        offset: u32,
    ) -> Option<Vec<OriginalSourceCallbackProcedureLookup>> {
        Some(
            self.callback_targets
                .get(&offset)?
                .iter()
                .filter_map(|(_, row)| row.clone())
                .collect(),
        )
    }
    pub(crate) fn callback_procedure_target(
        &self,
        offset: u32,
        prefix: &OriginalCallbackPrefix,
    ) -> Option<&OriginalSourceCallbackProcedureLookup> {
        let mut targets = self
            .callback_targets
            .get(&offset)?
            .iter()
            .filter_map(|(_, target)| target.as_ref())
            .filter(|target| target.matches_prefix(prefix));
        let target = targets.next()?;
        targets.next().is_none().then_some(target)
    }
    pub(super) fn retain_callback_target(&mut self, lookup: OriginalSourceCallbackProcedureLookup) {
        self.merge_callback_target(
            lookup.registration.site().offset,
            lookup.prefix.clone(),
            Some(lookup),
        );
    }
    pub(super) fn merge_callback_target(
        &mut self,
        offset: u32,
        prefix: OriginalCallbackPrefix,
        lookup: Option<OriginalSourceCallbackProcedureLookup>,
    ) {
        let row = self.callback_targets.entry(offset).or_default();
        if let Some((_, previous)) = row.iter_mut().find(|(owned, _)| owned == &prefix) {
            if previous != &lookup
                && let Some(retained) = previous
            {
                retained.outcome = CallbackProcedureOutcome::Refusal(
                    OriginalSourceCallbackProcedureRefusal::AmbiguousSourceTarget,
                );
                retained.selection = None;
            }
        } else {
            row.push((prefix, lookup));
        }
    }
}
impl AdviceGraph {
    fn callback_source_selection(
        &self,
        prefix: &OriginalCallbackPrefix,
    ) -> Option<ByteCommandSlot> {
        let policy = self.policy.native()?;
        (policy == prefix.name_input().policy()).then_some(())?;
        match prefix.scope() {
            Some(ScriptLookupScope::InvokingFrame) => {
                self.lookup_head_key(prefix.name_input().bytes())
            }
            Some(ScriptLookupScope::GlobalFrame) => {
                super::lookup_key(&self.policy, prefix.name_input().bytes())
            }
            Some(ScriptLookupScope::TriggerFrame) | None => {
                let recipe = policy.recipe();
                if !matches!(recipe, NativeNameProtocol::C(_))
                    || recipe
                        .command_lookup_input(
                            NativeNameContext::root(),
                            prefix.name_input().bytes(),
                        )
                        .ok()?
                        .qualification()
                        != NativeNameQualification::Absolute
                {
                    return None;
                }
                super::lookup_key(&self.policy, prefix.name_input().bytes())
            }
        }
    }
}
impl AdviceInvocationContext<'_> {
    pub(super) fn retain_source_callback_targets(
        &self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        invocation: AdviceInvocation<'_>,
        graph: &AdviceGraph,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) {
        // naming.source.original-callback-procedure-target
        // docs/design/analysis/name-resolution-proofs/source-original-callback-procedure-target.md
        if self.policy.native().is_none() {
            return;
        }
        let Some(registration) = self
            .schema_advice(invocation, graph, schema)
            .map(OriginalSourceCallbackRegistration::from_command)
            .map(Arc::new)
        else {
            return;
        };
        let interpreter = schema.semantics.body_interpreter.resolve_with(|ordinal| {
            std::str::from_utf8(invocation.arguments.get(ordinal)?.value.as_deref()?).ok()
        });
        for (ordinal, word) in invocation.arguments.iter().enumerate() {
            if (schema.facts().script_lookup_scope(ordinal).is_none()
                && schema.facts().command_prefix_arity(ordinal).is_none())
                || word.original.group().expand
            {
                continue;
            }
            let Some((prefix, owned_registration)) = self.source_callback_prefix(
                &word.original,
                &schema.facts(),
                ordinal,
                graph,
                &registration,
            ) else {
                continue;
            };
            tape.retain_callback_target(Self::source_callback_lookup(
                graph,
                owned_registration,
                prefix,
                &interpreter,
            ));
        }
    }
    pub(super) fn retain_registered_callback_targets(
        &self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        graph: &AdviceGraph,
        instance: &super::OriginalSourceRegisteredInstanceWords,
    ) {
        if self.policy.native().is_none() {
            return;
        }
        let registration = Arc::new(OriginalSourceCallbackRegistration::from_instance(instance));
        let _ = instance.with_source_schema(self.context, |schema| {
            let facts = schema.facts();
            let interpreter = schema.semantics.body_interpreter.resolve_with(|ordinal| {
                std::str::from_utf8(instance.argument_bytes(ordinal)?).ok()
            });
            for ordinal in 0..schema.words.arguments().len() {
                if facts.script_lookup_scope(ordinal).is_none()
                    && facts.command_prefix_arity(ordinal).is_none()
                {
                    continue;
                }
                let Some(word) = instance
                    .argument_word(ordinal)
                    .filter(|word| !word.group().expand)
                else {
                    continue;
                };
                let Some((prefix, owned_registration)) =
                    self.source_callback_prefix(word, &facts, ordinal, graph, &registration)
                else {
                    continue;
                };
                tape.retain_callback_target(Self::source_callback_lookup(
                    graph,
                    owned_registration,
                    prefix,
                    &interpreter,
                ));
            }
        });
    }
    fn source_callback_prefix(
        &self,
        word: &NativeWord,
        facts: &tcl_registry::InvocationFacts,
        ordinal: usize,
        graph: &AdviceGraph,
        registration: &Arc<OriginalSourceCallbackRegistration>,
    ) -> Option<(
        OriginalCallbackPrefix,
        Arc<OriginalSourceCallbackRegistration>,
    )> {
        if let Some(prefix) =
            OriginalCallbackPrefix::from_original_static_operand(word, facts, ordinal, self.dialect)
        {
            return Some((prefix, Arc::clone(registration)));
        }
        // naming.source.original-callback-procedure-target
        // docs/design/analysis/name-resolution-proofs/source-original-callback-procedure-target.md
        let substitution =
            crate::command_binding::original_single_command_substitution_words(word)?;
        let native = &substitution.command().words;
        let selected = self.selected_prefix_words(native, graph)?;
        let values = super::registry_words(&selected.arguments);
        let resolution =
            tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
                self.context.commands(),
                Some(self.context.context()),
                tcl_registry::InvocationWords::structured(
                    tcl_registry::InvocationWord::Literal(&selected.command),
                    &values,
                )
                .with_dialect(self.dialect),
                tcl_dialect::model::InvocationRealm::RuleLoader,
            );
        let schema = resolution.resolved()?;
        if !schema
            .semantics
            .traits
            .contains(tcl_registry::Traits::BUILDS_COMMAND_PREFIX)
            || schema.semantics.native_result
                != Some(
                    tcl_registry::native_result::NativeResultContract::ListArguments { from: 0 },
                )
            || schema.facts().arity_accepts_frozen_arguments() != Some(true)
        {
            return None;
        }
        let input = selected
            .arguments
            .first()?
            .input
            .as_ref()?
            .native_input()?
            .clone();
        let prefix = OriginalCallbackPrefix::from_original_builder_head(
            input,
            facts,
            ordinal,
            selected.arguments.len().checked_sub(1)?,
        )?;
        let builder = self.schema_advice(
            AdviceInvocation {
                native,
                head: &selected.head,
                arguments: &selected.arguments,
                lineage: &selected.lineage,
            },
            graph,
            &schema,
        )?;
        let mut retained = registration.as_ref().clone();
        retained
            .obligations
            .extend(builder.obligations().iter().cloned());
        retained
            .obligations
            .push(SourceCommandTransitionObligation::ProducedCommandPrefixApplicability);
        retained.builder = Some(Arc::new(builder));
        Some((prefix, Arc::new(retained)))
    }
    fn source_callback_lookup(
        graph: &AdviceGraph,
        mut registration: Arc<OriginalSourceCallbackRegistration>,
        prefix: OriginalCallbackPrefix,
        interpreter: &tcl_registry::world_effect::InterpreterScope,
    ) -> OriginalSourceCallbackProcedureLookup {
        if prefix.scope().is_none() {
            Arc::make_mut(&mut registration)
                .obligations
                .push(SourceCommandTransitionObligation::UnavailableCallbackLookupFrame);
        }
        let selection = graph.callback_source_selection(&prefix);
        let outcome = if interpreter != &tcl_registry::world_effect::InterpreterScope::Current {
            CallbackProcedureOutcome::Refusal(
                OriginalSourceCallbackProcedureRefusal::ExternalInterpreter,
            )
        } else if let Some(slot) = &selection {
            match Self::source_callback_target(
                graph,
                Arc::clone(&registration),
                prefix.clone(),
                slot.clone(),
            ) {
                Ok(target) => CallbackProcedureOutcome::Target(Box::new(target)),
                Err(refusal) => CallbackProcedureOutcome::Refusal(refusal),
            }
        } else {
            CallbackProcedureOutcome::Refusal(
                OriginalSourceCallbackProcedureRefusal::UnavailableScope,
            )
        };
        OriginalSourceCallbackProcedureLookup {
            registration,
            prefix,
            selection,
            outcome,
        }
    }
    fn source_callback_target(
        graph: &AdviceGraph,
        registration: Arc<OriginalSourceCallbackRegistration>,
        prefix: OriginalCallbackPrefix,
        mut key: ByteCommandSlot,
    ) -> Result<OriginalSourceCallbackProcedureTarget, OriginalSourceCallbackProcedureRefusal> {
        let mut target = SourceAdviceNameInput::Native(prefix.name_input().clone());
        let mut visited = Vec::new();
        let mut captured = Vec::new();
        let mut lineage = Vec::new();
        loop {
            if visited.contains(&key) {
                return Err(OriginalSourceCallbackProcedureRefusal::KnownSourceBarrier);
            }
            visited.push(key.clone());
            let procedure = match graph.cells.get(&key) {
                Some(AdviceCell::SourceProcedure(procedure)) => Some(Arc::clone(procedure)),
                Some(AdviceCell::Alias {
                    target: next,
                    prefix,
                    lookup,
                    lineage: steps,
                }) => {
                    let mut composed = prefix.clone();
                    composed.extend(captured);
                    captured = composed;
                    lineage.extend(steps.iter().cloned());
                    target = next.as_ref().clone();
                    key = graph
                        .alias_target_key(next, *lookup)
                        .ok_or(OriginalSourceCallbackProcedureRefusal::UnavailableScope)?;
                    continue;
                }
                None if !lineage.is_empty() => None,
                None => return Err(OriginalSourceCallbackProcedureRefusal::UnknownSourceTarget),
                _ => return Err(OriginalSourceCallbackProcedureRefusal::KnownSourceBarrier),
            };
            if let Some(procedure) = &procedure {
                lineage.extend(procedure.lineage.iter().cloned());
            }
            let mut obligations = registration.obligations().to_vec();
            if !lineage.is_empty()
                && !obligations
                    .contains(&SourceCommandTransitionObligation::WrittenTransitionApplicability)
            {
                obligations.push(SourceCommandTransitionObligation::WrittenTransitionApplicability);
            }
            return Ok(OriginalSourceCallbackProcedureTarget {
                registration,
                prefix,
                target,
                slot: key,
                procedure,
                captured,
                lineage,
                obligations,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        OriginalSourceCallbackProcedureRefusal, OriginalSourceCallbackProcedureTargetKind,
    };
    use crate::analyser::{Analyser, AnalysisResult};
    use crate::command_binding::{OriginalCallbackPrefix, SourceCommandTransitionObligation};
    use crate::registry_invocation::source_structure::source_callback_procedure_target_at;

    fn callback_prefix(analysis: &AnalysisResult, head: &[u8]) -> OriginalCallbackPrefix {
        analysis
            .command_invocations
            .iter()
            .filter_map(|invocation| invocation.original_callback_prefix.as_deref())
            .find(|prefix| prefix.name_input().bytes() == head)
            .expect("genuine original callback prefix")
            .clone()
    }
    fn at(source: &str, marker: &str) -> u32 {
        u32::try_from(source.find(marker).unwrap()).unwrap()
    }

    #[test]
    fn original_callback_builder_keeps_whole_producers_without_execution_or_lookup() {
        // naming.source.original-callback-procedure-target
        // docs/design/analysis/name-resolution-proofs/source-original-callback-procedure-target.md
        for setup in ["", "rename list make", "interp alias {} make {} list cb"] {
            let operand = if setup.is_empty() {
                "[list cb $x]"
            } else if setup.starts_with("rename") {
                "[make cb $x]"
            } else {
                "[make $x]"
            };
            let source = format!("proc cb args {{}}; {setup}; lsort -command {operand} {{3 1 2}}");
            let analysis = Analyser::new().analyse(&source, "tcl8.6");
            let input = analysis.resolved_input.as_ref().unwrap();
            let context = input.context_registry();
            let module = crate::lowering::lower_to_ir(&source, context.commands());
            let tokens = module
                .top_level
                .statements
                .last()
                .unwrap()
                .tokens()
                .unwrap();
            let rows = analysis
                .retained_command_realm()
                .unwrap()
                .original_source_callback_targets_for_tokens(input, tokens)
                .unwrap();
            let row = rows.first().unwrap();
            assert_eq!(row.prefix().name_input().bytes(), b"cb");
            assert_eq!(row.prefix().baked_argument_count(), 1);
            assert!(row.prefix().lookup().is_none());
            let builder = row.registration().source_builder().unwrap();
            assert_eq!(builder.command(), "list");
            assert_eq!(
                row.registration().original_words()[0].bytes(),
                b"lsort"
            );
            assert!(builder.original_words()[0].span().start() > row.registration().site().offset);
            assert!(row.target().unwrap().source_procedure(&analysis).is_some());
            assert!(
                row.obligations().contains(
                    &SourceCommandTransitionObligation::ProducedCommandPrefixApplicability
                )
            );
        }
    }

    #[test]
    fn original_callback_source_target_keeps_alias_capture_order_and_canonical_local_declaration() {
        // naming.source.original-callback-procedure-target
        // docs/design/analysis/name-resolution-proofs/source-original-callback-procedure-target.md
        let source = "proc target {a b c d e} {}; interp alias {} inner {} target INNER; interp alias {} cb {} inner OUTER; lsort -command {cb BAKED} {3 1 2}";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let prefix = callback_prefix(&analysis, b"cb");
        let lookup = source_callback_procedure_target_at(
            source,
            &analysis,
            at(source, "lsort -command"),
            &prefix,
        )
        .expect("original registration horizon");
        let target = lookup.target().expect("original local procedure");
        assert_eq!(
            target.kind(),
            OriginalSourceCallbackProcedureTargetKind::LocalProcedure
        );
        assert_eq!(
            target
                .captured_arguments()
                .iter()
                .map(|word| word.literal_bytes().unwrap())
                .collect::<Vec<_>>(),
            vec![b"INNER".as_slice(), b"OUTER".as_slice()]
        );
        assert_eq!(prefix.baked_argument_count(), 1);
        assert_eq!(
            target.argument_input(0).unwrap().original_word(),
            target.argument_word(0)
        );
        assert!(
            target.argument_word(0).unwrap().span().start() < lookup.registration().site().offset
        );
        assert_eq!(
            target
                .source_procedure(&analysis)
                .unwrap()
                .name_input()
                .bytes(),
            b"target"
        );
        assert_eq!(target.lineage().len(), 2);
        assert!(target.obligations().contains(
            &super::SourceCommandTransitionObligation::OriginalCallbackTargetApplicability
        ));
        assert!(!lookup.permits_external_signature_lookup());
        assert!(
            source_callback_procedure_target_at(
                &source.replace("BAKED", "OTHER"),
                &analysis,
                at(source, "lsort -command"),
                &prefix
            )
            .is_none()
        );
    }

    #[test]
    fn original_callback_source_target_keeps_alias_move_and_direct_target_canonical_name() {
        // naming.source.original-callback-procedure-target
        // docs/design/analysis/name-resolution-proofs/source-original-callback-procedure-target.md
        for (source, head) in [
            (
                "proc target {a b c} {}; interp alias {} cb {} target FIXED; rename cb held; lsort -command held {3 1 2}",
                b"held".as_slice(),
            ),
            (
                "proc target {a b} {}; rename target held; lsort -command held {3 1 2}",
                b"held".as_slice(),
            ),
        ] {
            let analysis = Analyser::new().analyse(source, "tcl8.6");
            let prefix = callback_prefix(&analysis, head);
            let lookup = source_callback_procedure_target_at(
                source,
                &analysis,
                at(source, "lsort -command"),
                &prefix,
            )
            .unwrap();
            let target = lookup.target().expect(source);
            assert_eq!(
                target
                    .source_procedure(&analysis)
                    .unwrap()
                    .name_input()
                    .bytes(),
                b"target"
            );
            assert_eq!(target.declaration_name_input().unwrap().bytes(), b"target");
            assert_eq!(
                target.target_input().bytes(),
                if source.contains("rename cb") {
                    b"target".as_slice()
                } else {
                    b"held".as_slice()
                }
            );
            assert_eq!(
                target.source_slot().simple.as_bytes(),
                if source.contains("rename cb") {
                    b"target".as_slice()
                } else {
                    b"held".as_slice()
                }
            );
        }
    }

    #[test]
    fn original_callback_source_target_keeps_held_target_name_and_known_refusal_boundary() {
        // naming.source.original-callback-procedure-target
        // docs/design/analysis/name-resolution-proofs/source-original-callback-procedure-target.md
        for source in [
            "proc target {a b c} {}; interp alias {} cb {} target FIXED; rename target held; lsort -command cb {3 1 2}",
            "proc target {a b c} {}; interp alias {} cb {} target FIXED; rename target {}; lsort -command cb {3 1 2}",
            "proc cb {a b c} {}; rename cb {}; lsort -command cb {3 1 2}",
            "interp alias {} a {} b; interp alias {} b {} a; lsort -command a {3 1 2}",
            "proc cb {a b c} {}; interp alias {} cb child target FIXED; lsort -command cb {3 1 2}",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl8.6");
            let head = if source.contains("-command a ") {
                b"a".as_slice()
            } else {
                b"cb".as_slice()
            };
            let prefix = callback_prefix(&analysis, head);
            let lookup = source_callback_procedure_target_at(
                source,
                &analysis,
                at(source, "lsort -command"),
                &prefix,
            )
            .unwrap_or_else(|| {
                let input = analysis.resolved_input.as_ref().unwrap();
                let tape = analysis
                    .retained_command_realm()
                    .unwrap()
                    .source_bindings()
                    .original_source_transition_advice_tape(&input.context_registry())
                    .unwrap();
                panic!(
                    "{source}: prefix={prefix:?}; retained={:?}",
                    tape.callback_targets.get(&at(source, "lsort -command"))
                );
            });
            assert_eq!(
                lookup.refusal(),
                Some(OriginalSourceCallbackProcedureRefusal::KnownSourceBarrier),
                "{source}"
            );
            assert!(!lookup.permits_external_signature_lookup());
        }
        let source = "proc target {a b c} {}; interp alias {} cb {} target FIXED; rename target held; proc target {a b c d} {}; lsort -command {cb BAKED} {3 1 2}";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let prefix = callback_prefix(&analysis, b"cb");
        let lookup = source_callback_procedure_target_at(
            source,
            &analysis,
            at(source, "lsort -command"),
            &prefix,
        )
        .unwrap();
        let target = lookup
            .target()
            .expect("held alias name joins the genuine replacement declaration");
        assert_eq!(
            target
                .source_procedure(&analysis)
                .unwrap()
                .declaration_site()
                .offset,
            at(source, "proc target {a b c d}")
        );
        assert_eq!(
            target.captured_arguments()[0].literal_bytes(),
            Some(b"FIXED".as_slice())
        );
        assert_eq!(prefix.baked_argument_count(), 1);
        assert!(
            target
                .obligations()
                .contains(&super::SourceCommandTransitionObligation::UnknownEarlierMutation)
        );
    }

    #[test]
    fn original_callback_source_target_distinguishes_alias_external_name_from_missing_direct_target()
     {
        // naming.source.original-callback-procedure-target
        // docs/design/analysis/name-resolution-proofs/source-original-callback-procedure-target.md
        let source = "interp alias {} cb {} unseen FIXED; lsort -command cb {3 1 2}";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let prefix = callback_prefix(&analysis, b"cb");
        let lookup = source_callback_procedure_target_at(
            source,
            &analysis,
            at(source, "lsort -command"),
            &prefix,
        )
        .unwrap();
        let target = lookup.target().expect("genuine held external target name");
        assert_eq!(
            target.kind(),
            OriginalSourceCallbackProcedureTargetKind::ExternalSourceName
        );
        assert_eq!(target.target_input().bytes(), b"unseen");
        assert_eq!(target.source_slot().simple.as_bytes(), b"unseen");
        assert!(target.source_procedure(&analysis).is_none());
        let source = "lsort -command unseen {3 1 2}";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let prefix = callback_prefix(&analysis, b"unseen");
        let lookup = source_callback_procedure_target_at(source, &analysis, 0, &prefix).unwrap();
        assert_eq!(
            lookup.refusal(),
            Some(OriginalSourceCallbackProcedureRefusal::UnknownSourceTarget)
        );
        assert!(lookup.permits_external_signature_lookup());
    }

    #[test]
    fn original_callback_source_target_refuses_relative_trigger_frame_and_keeps_absolute_scope_recipe()
     {
        // naming.source.original-callback-procedure-target
        // docs/design/analysis/name-resolution-proofs/source-original-callback-procedure-target.md
        for (head, refusal) in [("cb", true), ("::cb", false)] {
            let source =
                format!("proc cb {{a b c}} {{}}; trace add variable value read {{{head}}}");
            let analysis = Analyser::new().analyse(&source, "tcl8.6");
            let prefix = callback_prefix(&analysis, head.as_bytes());
            let lookup = source_callback_procedure_target_at(
                &source,
                &analysis,
                at(&source, "trace add"),
                &prefix,
            )
            .unwrap();
            assert_eq!(
                lookup.refusal() == Some(OriginalSourceCallbackProcedureRefusal::UnavailableScope),
                refusal
            );
            if refusal {
                assert!(!lookup.permits_external_signature_lookup());
            } else {
                assert!(
                    lookup
                        .target()
                        .unwrap()
                        .source_procedure(&analysis)
                        .is_some()
                );
            }
        }
    }
    #[test]
    fn original_callback_registration_retains_actual_installer_and_captured_operand_producer() {
        // naming.source.original-callback-procedure-target
        // docs/design/analysis/name-resolution-proofs/source-original-callback-procedure-target.md
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
            let installer = if dialect == "jimtcl" {
                "alias sortWith lsort -command {cb BAKED}"
            } else {
                "interp alias {} sortWith {} lsort -command {cb BAKED}"
            };
            let source = format!("proc cb {{a b c d}} {{}}; {installer}; sortWith {{2 1}}");
            let analysis = Analyser::new().analyse(&source, dialect);
            let prefix = callback_prefix(&analysis, b"cb");
            let registration = prefix.source_registration().expect(dialect);
            assert_eq!(
                registration.site().offset,
                at(&source, "sortWith {2 1}"),
                "{dialect}"
            );
            let producer = prefix
                .name_input()
                .original_static_list_container()
                .unwrap()
                .parent_word();
            assert!(producer.span().start() < registration.site().offset);
            assert_eq!(
                registration.original_words()[0].span().start(),
                registration.site().offset
            );
            assert!(registration.source_command().is_some());
            assert!(registration.source_instance().is_none());
            assert!(
                prefix.lookup().is_none(),
                "captured readonly prefix invents no Native lookup: {dialect}"
            );
            let signature = analysis
                .command_invocations
                .iter()
                .filter_map(|invocation| invocation.original_callback_signature_lookup.as_ref())
                .find(|lookup| lookup.prefix().name_input() == prefix.name_input())
                .expect(dialect);
            assert_eq!(signature.original().registration(), registration);
            assert_eq!(
                signature
                    .declaration()
                    .unwrap()
                    .name()
                    .slot()
                    .simple
                    .as_bytes(),
                b"cb"
            );
            assert!(registration.obligations().contains(
                &super::SourceCommandTransitionObligation::OriginalCallbackTargetApplicability
            ));
            assert!(
                source_callback_procedure_target_at(
                    &source,
                    &analysis,
                    producer.span().start(),
                    &prefix
                )
                .is_none()
            );
            assert!(
                source_callback_procedure_target_at(
                    &format!("{source}\n"),
                    &analysis,
                    registration.site().offset,
                    &prefix
                )
                .is_none()
            );
        }
    }

    #[test]
    fn original_callback_registration_retains_factory_method_and_known_target_barrier() {
        // naming.source.original-callback-procedure-target
        // docs/design/analysis/name-resolution-proofs/source-original-callback-procedure-target.md
        for (factory, receiver) in [
            ("struct::graph graph", "graph walk root -command ::cb"),
            (
                "set graph [struct::graph]",
                "$graph walk root -command ::cb",
            ),
        ] {
            for removed in [false, true] {
                let delete = if removed { "rename cb {}; " } else { "" };
                let source = format!(
                    "# tcl-lsp: requires struct::graph\npackage require struct::graph; proc cb {{a b}} {{}}; {factory}; {delete}{receiver}"
                );
                let analysis = Analyser::new().analyse(&source, "tcl9.0");
                let prefix = callback_prefix(&analysis, b"::cb");
                let registration = prefix
                    .source_registration()
                    .expect("genuine registered method installer");
                assert_eq!(registration.site().offset, at(&source, receiver));
                assert!(registration.source_command().is_none());
                let instance = registration.source_instance().unwrap();
                assert_eq!(instance.instance().factory().command(), "struct::graph");
                assert_eq!(
                    instance.handle_binding().is_some(),
                    factory.starts_with("set ")
                );
                assert!(prefix.lookup().is_none());
                assert_eq!(
                    prefix.scope(),
                    None,
                    "the package method frame stays unclassified"
                );
                assert!(registration.obligations().contains(
                    &super::SourceCommandTransitionObligation::UnavailableCallbackLookupFrame
                ));
                let lookup = source_callback_procedure_target_at(
                    &source,
                    &analysis,
                    registration.site().offset,
                    &prefix,
                )
                .unwrap();
                assert_eq!(lookup.registration(), registration);
                if removed {
                    assert_eq!(
                        lookup.refusal(),
                        Some(OriginalSourceCallbackProcedureRefusal::KnownSourceBarrier)
                    );
                    assert!(!lookup.permits_external_signature_lookup());
                } else {
                    assert_eq!(
                        lookup
                            .target()
                            .unwrap()
                            .source_procedure(&analysis)
                            .unwrap()
                            .name_input()
                            .bytes(),
                        b"cb"
                    );
                }
            }
        }
    }
    #[test]
    fn original_callback_unknown_scope_retains_prefix_without_relative_frame_donation() {
        // naming.source.original-callback-procedure-target
        // docs/design/analysis/name-resolution-proofs/source-original-callback-procedure-target.md
        for head in ["cb", "::cb"] {
            let source = format!(
                "# tcl-lsp: requires smtp\npackage require smtp; proc cb {{a}} {{}}; smtp::sendmessage $t -tlspolicy {head}"
            );
            let analysis = Analyser::new().analyse(&source, "tcl9.0");
            let prefix = callback_prefix(&analysis, head.as_bytes());
            assert_eq!(prefix.scope(), None);
            assert!(prefix.lookup().is_none());
            let registration = prefix
                .source_registration()
                .expect("whole source installer");
            assert!(registration.obligations().contains(
                &super::SourceCommandTransitionObligation::UnavailableCallbackLookupFrame
            ));
            let lookup = source_callback_procedure_target_at(
                &source,
                &analysis,
                registration.site().offset,
                &prefix,
            )
            .unwrap();
            if head == "cb" {
                assert_eq!(
                    lookup.refusal(),
                    Some(OriginalSourceCallbackProcedureRefusal::UnavailableScope)
                );
                assert!(!lookup.permits_external_signature_lookup());
            } else {
                assert_eq!(
                    lookup
                        .target()
                        .unwrap()
                        .source_procedure(&analysis)
                        .unwrap()
                        .name_input()
                        .bytes(),
                    b"cb"
                );
            }
        }
    }
    #[test]
    fn original_package_callback_receipts_require_selected_package_availability() {
        // naming.source.original-callback-procedure-target
        // docs/design/analysis/name-resolution-proofs/source-original-callback-procedure-target.md
        for source in [
            "proc cb {a b} {}; struct::graph graph; graph walk root -command ::cb",
            "proc cb {a} {}; smtp::sendmessage message -tlspolicy ::cb",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl9.0");
            assert!(
                !analysis.command_invocations.iter().any(|invocation| {
                    invocation
                        .original_callback_prefix
                        .as_ref()
                        .is_some_and(|prefix| prefix.name_input().bytes() == b"::cb")
                }),
                "unavailable schema cannot issue an original package callback: {source}"
            );
        }
    }
}
