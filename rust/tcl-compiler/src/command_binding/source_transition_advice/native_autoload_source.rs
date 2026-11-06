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

//! Conditional external source headers under an authentic deferred body horizon.

use super::{
    AdviceCell, AdviceGraph, AdviceInvocationContext, OriginalNativeProcedureBodyCommand,
    OriginalSourceCommandTransitionAdvice, OriginalSourceTransitionAdviceTape, SourceAdviceWord,
};
use crate::analyser::AnalysisResult;
use crate::signature_scan::scope::SignatureSourceNameKey;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use tcl_core_types::{ByteCommandSlot, ByteNamespacePath, NameBytes};
use tcl_lexer::NativeWord;
use tcl_registry::world_effect::CallbackKinds;
use tcl_registry::{CommandRegistry, InvocationFacts};
use tcl_syntax::naming::{NamePolicyProtocol, NativeNameContext};

/// Runtime applicability remains independent of a conditional library header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OriginalNativeDeferredAutoloadObligation {
    /// The original procedure body has no observed activation.
    BodyEntry,
    /// No actual command lookup or autoload invocation selected the header.
    ActualLookup,
    /// Variable, execution and host observers remain unresolved.
    ObserverApplicability,
    /// Earlier completion and control flow do not prove this head is reached.
    DeferredReachability,
    /// Earlier original operations retain their possible lookup effects.
    UnknownEarlierMutation,
    /// The source graph did not close the future namespace search path.
    UnknownLookupCoordinates,
    /// A prior original schema retains an unaccepted source arity.
    UnacceptedSourceArity,
    /// Earlier channel write is conditional on actual selection of that source operation.
    PriorChannelWriteApplicability,
    /// Earlier message/option interpretation or I/O failure may stop continuation.
    PriorChannelWriteNormalContinuation,
    /// Earlier channel transforms/reflection may run arbitrary callbacks and alter lookup.
    UnresolvedChannelCallbacks,
    /// No post-callback command world or lookup coordinates were observed.
    PostChannelCallbackLookup,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct NativeAutoloadRegisteredPrefix {
    facts: InvocationFacts,
    observer: Option<tcl_registry::source_observer::ConditionalSourceObserverEnvelope>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum NativeAutoloadPrefix {
    Registered(Box<NativeAutoloadRegisteredPrefix>),
    SourceProcedure {
        declaration: Arc<super::source_procedure::OriginalSourceProcedureBody>,
        body_facts: Vec<NativeAutoloadRegisteredPrefix>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct NativeAutoloadRecordedPrefix {
    pub(super) original: Arc<[NativeWord]>,
    pub(super) body: Arc<crate::registry_invocation::OriginalSourceScriptBody>,
    prefix: NativeAutoloadPrefix,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct NativeAutoloadRecordedHorizon {
    pub(super) original: Arc<[NativeWord]>,
    pub(super) body: Arc<crate::registry_invocation::OriginalSourceScriptBody>,
    pub(super) horizon: NativeAutoloadHorizon,
}

/// The name-related graph is frozen at the actual ordered operand horizon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct NativeAutoloadHorizon {
    name: SignatureSourceNameKey,
    namespace: ByteNamespacePath,
    command_candidates: Vec<ByteCommandSlot>,
    library_candidates: Vec<NameBytes>,
    library_slots: Vec<ByteCommandSlot>,
    cells: super::super::SharedSourceInventory<rustc_hash::FxHashMap<ByteCommandSlot, AdviceCell>>,
    possible_occupants: BTreeSet<ByteCommandSlot>,
    namespace_paths: BTreeMap<ByteNamespacePath, Vec<ByteNamespacePath>>,
    unknown_paths: BTreeSet<ByteNamespacePath>,
    uncertain_operations: Vec<Arc<[NativeWord]>>,
    arity_debts: Vec<Arc<OriginalSourceCommandTransitionAdvice>>,
    prefixes: Vec<NativeAutoloadPrefix>,
    body: Arc<crate::registry_invocation::OriginalSourceScriptBody>,
    declaration_path: Vec<crate::command_binding::CommandAllocationSite>,
    registry: tcl_registry::RegistrySemanticKey,
    obligations: Vec<OriginalNativeDeferredAutoloadObligation>,
}

impl AdviceInvocationContext<'_> {
    pub(super) fn retain_native_autoload_source(
        &self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        graph: &AdviceGraph,
        original: &[NativeWord],
        written: &[SourceAdviceWord],
    ) {
        if !matches!(
            self.query,
            Some(super::SourceAdviceQuery::NativeDeferredAutoload)
        ) {
            return;
        }
        let (Some(body), Some(head)) = (graph.procedure_body.as_ref(), original.first()) else {
            return;
        };
        let at = head.span().start();
        let horizon = self
            .native_autoload_horizon(tape, graph, original, written)
            .map(|horizon| NativeAutoloadRecordedHorizon {
                original: Arc::from(original),
                body: Arc::clone(body),
                horizon,
            });
        let prefix = self
            .native_autoload_prefix(graph, original, written)
            .map(|prefix| NativeAutoloadRecordedPrefix {
                original: Arc::from(original),
                body: Arc::clone(body),
                prefix,
            });
        super::source_class::merge_receipt(
            &mut tape.native_deferred_autoload_horizons,
            at,
            horizon,
        );
        super::source_class::merge_receipt(&mut tape.native_deferred_autoload_prefixes, at, prefix);
    }

    pub(super) fn native_autoload_prefix(
        &self,
        graph: &AdviceGraph,
        original: &[NativeWord],
        written: &[SourceAdviceWord],
    ) -> Option<NativeAutoloadPrefix> {
        let head = written.first()?.input.as_ref()?;
        if let Some(procedure) = graph.original_procedure(head) {
            if !procedure.lineage.is_empty()
                || !procedure.body.matches_context(self.context)
                || !procedure.declaration.matches_context(self.context)
            {
                return None;
            }
            let image = procedure.declaration.site().source.source_image();
            let config = procedure.declaration.config;
            if !procedure.body.matches_source(image, config) {
                return None;
            }
            let plan = tcl_lexer::native_script_words_in(
                image.clone(),
                procedure.body.content_span(),
                config,
            )
            .ok()?;
            if plan.fatal_tail.is_some() || plan.commands.len() > 64 {
                return None;
            }
            let mut caller = graph.clone();
            caller.procedure_namespace = Some(procedure.slot.namespace.clone());
            let mut body_facts = Vec::new();
            for command in plan.commands {
                if command.words.iter().any(|word| {
                    word.group().expand
                        || word.executable_parts().all_parts().any(|part| {
                            matches!(
                                part.part,
                                tcl_lexer::ExecutablePart::Command { .. }
                                    | tcl_lexer::ExecutablePart::Expression { .. }
                                    | tcl_lexer::ExecutablePart::ParseError(_)
                            )
                        })
                }) {
                    return None;
                }
                let written = super::original_words(&command.words, self.policy)?;
                body_facts.push(self.native_autoload_registered_prefix(&caller, &written)?);
            }
            return Some(NativeAutoloadPrefix::SourceProcedure {
                declaration: procedure,
                body_facts,
            });
        }
        (!original.iter().any(|word| word.group().expand)).then_some(())?;
        self.native_autoload_registered_prefix(graph, written)
            .map(Box::new)
            .map(NativeAutoloadPrefix::Registered)
    }

    fn native_autoload_registered_prefix(
        &self,
        graph: &AdviceGraph,
        written: &[SourceAdviceWord],
    ) -> Option<NativeAutoloadRegisteredPrefix> {
        let (command, prefix, lineage) = graph.resolve(written.first()?.input.as_ref()?)?;
        if !prefix.is_empty() || !lineage.is_empty() {
            return None;
        }
        let words = super::registry_words(&written[1..]);
        let resolution =
            tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
                self.context.commands(),
                Some(self.context.context()),
                tcl_registry::InvocationWords::structured(
                    tcl_registry::InvocationWord::Literal(&command),
                    &words,
                )
                .with_dialect(self.dialect),
                tcl_dialect::model::InvocationRealm::RuleLoader,
            );
        let schema = resolution.resolved()?;
        let facts = schema.facts();
        let observer = schema.conditional_source_observer_envelope(self.dialect);
        let callback_unavailable = facts.effects.callback().kinds.is_unknown()
            || facts
                .effects
                .callback()
                .kinds
                .contains(CallbackKinds::SCRIPT)
            || facts
                .effects
                .callback()
                .kinds
                .contains(CallbackKinds::COMMAND_PREFIX);
        if facts.arity_accepts_frozen_arguments() != Some(true)
            || (callback_unavailable && observer.is_none())
            || facts
                .traits
                .intersects(tcl_registry::traits::FRAME_REACH_TRAITS)
            || facts.arg_roles.iter().any(|(_, role)| {
                matches!(
                    role,
                    tcl_registry::ArgRole::Body | tcl_registry::ArgRole::Expr
                )
            })
            || !crate::registry_invocation::source_advice_has_no_declared_lookup_mutation(&facts)
        {
            return None;
        }
        Some(NativeAutoloadRegisteredPrefix { facts, observer })
    }

    pub(super) fn native_autoload_horizon(
        &self,
        tape: &OriginalSourceTransitionAdviceTape,
        graph: &AdviceGraph,
        original: &[NativeWord],
        written: &[SourceAdviceWord],
    ) -> Option<NativeAutoloadHorizon> {
        let namespace = graph.c_source_lookup_namespace()?.clone();
        let name = written
            .first()?
            .input
            .as_ref()?
            .native_input()?
            .original_word_key()?
            .clone();
        let captured = tcl_registry::native_compiler_words::NativeCompilerWords::capture(
            original,
            name.policy().string_protocol(),
        )
        .ok()?;
        if original
            .iter()
            .enumerate()
            .any(|(index, word)| word.group().expand || captured.literal(index).is_none())
        {
            return None;
        }
        let body = graph.procedure_body.as_ref()?;
        let library_candidates = tcl_syntax::naming::native_autoload_command_candidates(
            name.policy().string_protocol(),
            name.bytes(),
            &tcl_syntax::naming::native_namespace_full_name_bytes(&namespace),
        )?
        .into_iter()
        .map(NameBytes::from)
        .collect::<Vec<_>>();
        let library_slots = library_candidates
            .iter()
            .map(|candidate| {
                name.policy()
                    .recipe()
                    .command_lookup_slot(NativeNameContext::root(), candidate.as_bytes())
                    .ok()
            })
            .collect::<Option<Vec<_>>>()?;
        let command_candidates = graph.head_lookup_candidates(name.bytes())?;
        let prefixes = NativeAutoloadHorizon::capture_prefixes(tape, graph, body)?;
        let obligations = NativeAutoloadHorizon::capture_obligations(graph, &namespace, &prefixes);
        let result = NativeAutoloadHorizon {
            name,
            namespace,
            command_candidates,
            library_candidates,
            library_slots,
            cells: graph.cells.clone(),
            possible_occupants: graph.possible_occupants.clone(),
            namespace_paths: graph.namespace_paths.clone(),
            unknown_paths: graph.unknown_paths.clone(),
            uncertain_operations: graph.uncertain_operations.clone(),
            arity_debts: graph.arity_debts.clone(),
            prefixes,
            body: Arc::clone(body),
            declaration_path: graph.procedure_path.clone(),
            registry: self.context.commands().snapshot().semantic_key(),
            obligations,
        };
        result.known_slots_are_unoccupied().then_some(result)
    }
}

impl NativeAutoloadHorizon {
    fn capture_prefixes(
        tape: &OriginalSourceTransitionAdviceTape,
        graph: &AdviceGraph,
        body: &crate::registry_invocation::OriginalSourceScriptBody,
    ) -> Option<Vec<NativeAutoloadPrefix>> {
        let mut prefixes = Vec::new();
        for operation in &graph.uncertain_operations {
            let at = operation.first()?.span().start();
            let prior = tape.native_deferred_autoload_prefixes.get(&at)?.as_ref()?;
            if prior.original.as_ref() != operation.as_ref() || prior.body.as_ref() != body {
                return None;
            }
            prefixes.push(prior.prefix.clone());
        }
        Some(prefixes)
    }

    fn capture_obligations(
        graph: &AdviceGraph,
        namespace: &ByteNamespacePath,
        prefixes: &[NativeAutoloadPrefix],
    ) -> Vec<OriginalNativeDeferredAutoloadObligation> {
        let mut obligations = vec![
            OriginalNativeDeferredAutoloadObligation::BodyEntry,
            OriginalNativeDeferredAutoloadObligation::ActualLookup,
            OriginalNativeDeferredAutoloadObligation::ObserverApplicability,
            OriginalNativeDeferredAutoloadObligation::DeferredReachability,
        ];
        if !graph.uncertain_operations.is_empty() {
            obligations.push(OriginalNativeDeferredAutoloadObligation::UnknownEarlierMutation);
        }
        if graph.unknown_paths.contains(namespace) {
            obligations.push(OriginalNativeDeferredAutoloadObligation::UnknownLookupCoordinates);
        }
        if !graph.arity_debts.is_empty() {
            obligations.push(OriginalNativeDeferredAutoloadObligation::UnacceptedSourceArity);
        }
        if prefixes.iter().any(|prefix| match prefix {
            NativeAutoloadPrefix::Registered(prefix) => prefix.observer.is_some(),
            NativeAutoloadPrefix::SourceProcedure { body_facts, .. } => {
                body_facts.iter().any(|facts| facts.observer.is_some())
            }
        }) {
            obligations.extend([
                OriginalNativeDeferredAutoloadObligation::PriorChannelWriteApplicability,
                OriginalNativeDeferredAutoloadObligation::PriorChannelWriteNormalContinuation,
                OriginalNativeDeferredAutoloadObligation::UnresolvedChannelCallbacks,
                OriginalNativeDeferredAutoloadObligation::PostChannelCallbackLookup,
            ]);
        }
        obligations
    }

    fn known_slots_are_unoccupied(&self) -> bool {
        self.command_candidates
            .iter()
            .chain(&self.library_slots)
            .all(|slot| !self.cells.contains_key(slot) && !self.possible_occupants.contains(slot))
    }
}

/// A possible external SOURCE header, never a resolved invocation or absence.
/// Original body reachability, lookup and observers remain explicit obligations.
#[derive(Clone)]
pub struct OriginalNativeDeferredAutoloadAdvice {
    command: OriginalNativeProcedureBodyCommand,
    horizon: NativeAutoloadHorizon,
}

impl std::fmt::Debug for OriginalNativeDeferredAutoloadAdvice {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("OriginalNativeDeferredAutoloadAdvice")
            .field("head_span", &self.name().original_word().span())
            .field("policy", &self.name().policy())
            .field("candidate_count", &self.candidates().len())
            .field("obligations", &self.obligations())
            .field("prior_operation_count", &self.uncertain_operations().len())
            .finish_non_exhaustive()
    }
}

impl PartialEq for OriginalNativeDeferredAutoloadAdvice {
    fn eq(&self, other: &Self) -> bool {
        self.command.same_deferred_source_owner(&other.command) && self.horizon == other.horizon
    }
}
impl Eq for OriginalNativeDeferredAutoloadAdvice {}

impl OriginalNativeDeferredAutoloadAdvice {
    pub(super) fn capture(
        command: OriginalNativeProcedureBodyCommand,
        horizon: NativeAutoloadHorizon,
    ) -> Option<Self> {
        (command.original_words().first()? == horizon.name.original_word()
            && horizon.known_slots_are_unoccupied())
        .then_some(Self { command, horizon })
    }

    /// Exact retained declaration/source/input owner, not an equal text reanalysis.
    #[must_use]
    pub fn is_for_analysis(&self, analysis: &AnalysisResult) -> bool {
        self.command.caller_declaration(analysis).is_some()
            && analysis
                .resolved_registry()
                .is_some_and(|registry| registry.snapshot().semantic_key() == self.horizon.registry)
    }

    /// The actual declaring procedure that owns this deferred source head.
    #[must_use]
    pub fn caller_declaration<'a>(
        &self,
        analysis: &'a AnalysisResult,
    ) -> Option<
        &'a crate::signature_scan::original_name::SourceDeclarationMetadata<
            crate::analyser::ProcDef,
        >,
    > {
        self.is_for_analysis(analysis).then_some(())?;
        self.command.caller_declaration(analysis)
    }

    /// Complete original static head and its independently selected C policy.
    #[must_use]
    pub const fn name(&self) -> &SignatureSourceNameKey {
        &self.horizon.name
    }

    /// C library `auto_qualify` keys, retaining their canonical order.
    #[must_use]
    pub fn candidates(&self) -> &[NameBytes] {
        &self.horizon.library_candidates
    }

    /// Unproved source applicability; callers must display this as conditional.
    #[must_use]
    pub fn obligations(&self) -> &[OriginalNativeDeferredAutoloadObligation] {
        &self.horizon.obligations
    }

    /// Every prior uncertain original operation remains attached to this recipe.
    #[must_use]
    pub fn uncertain_operations(&self) -> &[Arc<[NativeWord]>] {
        &self.horizon.uncertain_operations
    }

    /// Match external source declarations only, preserving every known barrier.
    /// This supplies no present binding, actual autoload or runtime absence.
    #[must_use]
    pub fn matching_external_source_slots<'a, T: Clone + Eq>(
        &self,
        registry: &CommandRegistry,
        declarations: impl IntoIterator<Item = (&'a ByteCommandSlot, NamePolicyProtocol, T)>,
    ) -> Option<Vec<T>> {
        if registry.snapshot().semantic_key() != self.horizon.registry
            || !self.horizon.known_slots_are_unoccupied()
        {
            return None;
        }
        Some(crate::signature_scan::scope::first_matching_byte_slots(
            self.name().policy(),
            &self.horizon.library_slots,
            declarations,
        ))
    }
}

/// Conditional external SOURCE advice at one unique original deferred head.
/// Arguments, foreign source/input, dynamic heads and ambiguous owners refuse.
#[must_use]
pub fn source_native_deferred_autoload_advice_at(
    source: &str,
    analysis: &AnalysisResult,
    offset: u32,
) -> Option<OriginalNativeDeferredAutoloadAdvice> {
    let input = analysis.resolved_input.as_ref()?;
    let realm = analysis.retained_command_realm()?;
    let image = realm.original_source_image()?;
    if input.has_logical_source_name_context()
        || input.has_hosted_source_name_context()
        || image.bytes() != source.as_bytes()
        || !analysis.matches_original_source_image(image, input.lexer_config())
        || !realm.matches_resolved_analysis_input(input)
    {
        return None;
    }
    let commands = realm.original_native_deferred_autoload_body_commands(input)?;
    let mut selected = commands.into_iter().filter(|command| {
        command
            .original_words()
            .first()
            .is_some_and(|head| head.span().start() <= offset && offset < head.span().end())
    });
    let command = selected.next()?;
    selected.next().is_none().then_some(())?;
    command.deferred_autoload_advice(analysis)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::Analyser;

    const BROWSER_SOURCE: &str = "source lib/helpers.tcl\n\nproc run {} {\n    puts [greet_helper world]\n    puts [mypkg_autoloaded]\n}\n";
    const BROWSER_CHILD: &str = "proc greet_helper {who} {\n    return \"hi $who\"\n}\n";

    fn suffix(source: &str, child_source: &str, dialect: &str) -> AnalysisResult {
        let prefix = Analyser::new().analyse(source, dialect);
        let load = crate::registry_invocation::source_structure::original_project_source_load_at(
            source, &prefix, 0,
        )
        .expect("actual source Registry role at this first original vector");
        let point = load.source_point_entry(&prefix).unwrap();
        let child = Analyser::new()
            .analyse_with_source_point_entry(child_source, dialect, &point)
            .unwrap();
        let regions = load
            .conditional_continuation(&prefix, &child)
            .unwrap()
            .source_regions(&prefix, &child)
            .unwrap();
        assert_eq!(regions.regions().len(), 1);
        let suffix = Analyser::new()
            .analyse_source_region(&regions.regions()[0], &prefix, &child)
            .unwrap();
        assert!(
            regions
                .normal_outcome()
                .unwrap()
                .single_source_cut_survey(&prefix, &[&child, &suffix])
                .is_some()
        );
        suffix
    }

    fn at(source: &str, name: &str) -> u32 {
        u32::try_from(source.rfind(name).unwrap() + 2).unwrap()
    }

    fn assert_browser_source_obligations(advice: &OriginalNativeDeferredAutoloadAdvice) {
        for obligation in [
            OriginalNativeDeferredAutoloadObligation::BodyEntry,
            OriginalNativeDeferredAutoloadObligation::ActualLookup,
            OriginalNativeDeferredAutoloadObligation::ObserverApplicability,
            OriginalNativeDeferredAutoloadObligation::DeferredReachability,
            OriginalNativeDeferredAutoloadObligation::UnknownEarlierMutation,
        ] {
            assert!(advice.obligations().contains(&obligation));
        }
        assert!(advice.uncertain_operations().iter().any(|operation| {
            operation
                .first()
                .is_some_and(|head| &BROWSER_SOURCE[head.span().as_range()] == "greet_helper")
        }));
    }

    #[test]
    fn deferred_autoload_source_keeps_real_browser_cut_and_uncertain_prefix() {
        let analysis = suffix(BROWSER_SOURCE, BROWSER_CHILD, "tcl8.6");
        let realm = analysis.retained_command_realm().unwrap();
        let position = at(BROWSER_SOURCE, "mypkg_autoloaded");
        let head = u32::try_from(BROWSER_SOURCE.rfind("mypkg_autoloaded").unwrap()).unwrap();
        let before = realm.invocation_at_source("", head);
        let advice = source_native_deferred_autoload_advice_at(BROWSER_SOURCE, &analysis, position)
            .expect("first conditional external-source request under the genuine Browser cut");
        assert_eq!(advice.name().bytes(), b"mypkg_autoloaded");
        assert_eq!(advice.candidates(), [NameBytes::from("mypkg_autoloaded")]);
        assert!(advice.is_for_analysis(&analysis));
        assert_eq!(
            advice
                .caller_declaration(&analysis)
                .unwrap()
                .declaration_site()
                .offset,
            u32::try_from(BROWSER_SOURCE.find("proc run").unwrap()).unwrap()
        );
        assert_browser_source_obligations(&advice);
        assert!(analysis.original_completed_command_world().is_none());
        assert_eq!(before, realm.invocation_at_source("", head));
        assert!(!before.unobserved_native_dispatch());
        let repeated =
            source_native_deferred_autoload_advice_at(BROWSER_SOURCE, &analysis, position).unwrap();
        assert_eq!(
            advice, repeated,
            "fresh source tapes retain the same nominal owner and graph"
        );
        let foreign = suffix(BROWSER_SOURCE, BROWSER_CHILD, "tcl8.6");
        assert!(!advice.is_for_analysis(&foreign));
        assert_ne!(
            advice,
            source_native_deferred_autoload_advice_at(BROWSER_SOURCE, &foreign, position).unwrap()
        );
        assert!(
            source_native_deferred_autoload_advice_at(
                &(BROWSER_SOURCE.to_owned() + "\n"),
                &analysis,
                position
            )
            .is_none()
        );
        assert!(
            source_native_deferred_autoload_advice_at(
                BROWSER_SOURCE,
                &analysis,
                at(BROWSER_SOURCE, "world")
            )
            .is_none()
        );
        let external =
            Analyser::new().analyse("proc mypkg_autoloaded {} {return LIBRARY}", "tcl8.6");
        let declaration = external.original_procedure_declarations().next().unwrap();
        assert_eq!(
            advice.matching_external_source_slots(
                analysis.resolved_registry().unwrap(),
                [(declaration.name().slot(), declaration.name().policy(), 1_u8)]
            ),
            Some(vec![1])
        );
    }

    #[test]
    fn deferred_autoload_source_transports_parent_prefixes_between_real_brackets() {
        let source = "source lib/helpers.tcl; proc run {} {set first [greet_helper world]; set second [mypkg_autoloaded]}";
        let analysis = suffix(source, BROWSER_CHILD, "tcl8.6");
        let offset = at(source, "mypkg_autoloaded");
        let head = u32::try_from(source.rfind("mypkg_autoloaded").unwrap()).unwrap();
        let realm = analysis.retained_command_realm().unwrap();
        let before = realm.invocation_at_source("", head);
        let advice = source_native_deferred_autoload_advice_at(source, &analysis, offset)
            .expect("the second genuine bracket retains its earlier source-operation prefix");
        assert_eq!(advice.name().bytes(), b"mypkg_autoloaded");
        assert!(advice.is_for_analysis(&analysis));
        assert!(
            advice
                .obligations()
                .contains(&OriginalNativeDeferredAutoloadObligation::UnknownEarlierMutation)
        );
        assert!(
            advice
                .obligations()
                .contains(&OriginalNativeDeferredAutoloadObligation::ObserverApplicability)
        );
        assert!(
            advice
                .horizon
                .prefixes
                .iter()
                .any(|prefix| matches!(prefix, NativeAutoloadPrefix::SourceProcedure { .. }))
        );
        assert!(analysis.original_completed_command_world().is_none());
        assert_eq!(before, realm.invocation_at_source("", head));
        assert!(!before.unobserved_native_dispatch());
        let input = analysis.resolved_input.as_ref().unwrap();
        let ordinary = realm
            .original_native_procedure_body_commands(input)
            .unwrap();
        let mut same_head = ordinary.iter().filter(|command| {
            command
                .original_words()
                .first()
                .is_some_and(|word| word.span().start() == head)
        });
        let phase = same_head
            .next()
            .expect("ordinary source phase remains genuinely represented");
        assert!(same_head.next().is_none());
        assert!(phase.caller_declaration(&analysis).is_some());
        assert!(
            phase.deferred_autoload_advice(&analysis).is_none(),
            "an ordinary-purpose tape has no deferred-autoload prefix authority"
        );
        let foreign = suffix(source, BROWSER_CHILD, "tcl8.6");
        assert!(!advice.is_for_analysis(&foreign));
        assert!(advice.caller_declaration(&foreign).is_none());
        assert!(
            source_native_deferred_autoload_advice_at(
                &(source.to_owned() + "\n"),
                &analysis,
                offset
            )
            .is_none()
        );
    }

    #[test]
    fn deferred_autoload_parent_prefix_transport_preserves_real_unknown_and_mutation_refusals() {
        let controls = [
            (
                "set first [mystery]; set second [mypkg_autoloaded]",
                BROWSER_CHILD,
            ),
            (
                "set first [greet_helper world]; set second [mypkg_autoloaded]",
                "proc greet_helper {who} {rename mypkg_autoloaded moved}",
            ),
            (
                "set first [greet_helper world]; set second [mypkg_autoloaded]",
                "proc greet_helper {who} {return [unknown]}",
            ),
            (
                "set first [greet_helper world]; rename greet_helper mypkg_autoloaded; set second [mypkg_autoloaded]",
                BROWSER_CHILD,
            ),
        ];
        for (body, child) in controls {
            let source = format!("source lib/helpers.tcl; proc run {{}} {{{body}}}");
            let analysis = suffix(&source, child, "tcl8.6");
            assert!(
                source_native_deferred_autoload_advice_at(
                    &source,
                    &analysis,
                    at(&source, "mypkg_autoloaded")
                )
                .is_none(),
                "unavailable real prefix stays terminal: {body}"
            );
        }
    }

    #[test]
    fn deferred_autoload_browser_prefix_keeps_unresolved_channel_callbacks_and_ordinary_unknowns() {
        let analysis = suffix(BROWSER_SOURCE, BROWSER_CHILD, "tcl8.6");
        let advice = source_native_deferred_autoload_advice_at(
            BROWSER_SOURCE,
            &analysis,
            at(BROWSER_SOURCE, "mypkg_autoloaded"),
        )
        .expect("the unchanged Browser head has conditional external source metadata");
        for debt in [
            OriginalNativeDeferredAutoloadObligation::PriorChannelWriteApplicability,
            OriginalNativeDeferredAutoloadObligation::PriorChannelWriteNormalContinuation,
            OriginalNativeDeferredAutoloadObligation::UnresolvedChannelCallbacks,
            OriginalNativeDeferredAutoloadObligation::PostChannelCallbackLookup,
        ] {
            assert!(advice.obligations().contains(&debt));
        }
        let observers = advice
            .horizon
            .prefixes
            .iter()
            .filter_map(|prefix| match prefix {
                NativeAutoloadPrefix::Registered(prefix) => prefix
                    .observer
                    .as_ref()
                    .map(|observer| (&prefix.facts, observer)),
                NativeAutoloadPrefix::SourceProcedure { .. } => None,
            })
            .collect::<Vec<_>>();
        assert_ne!(observers.len(), 0);
        for (facts, observer) in observers {
            assert!(facts.effects.callback().kinds.is_unknown());
            assert!(facts.effects.requires_world_barrier());
            assert_eq!(observer.ordinary_effects(), &facts.effects);
            assert!(observer.source_effects().callback().kinds.is_unknown());
            assert!(observer.obligations().contains(&tcl_registry::source_observer::ConditionalSourceObserverObligation::LookupWorldAfterCallbacks));
        }
        assert!(analysis.original_completed_command_world().is_none());
        let head = u32::try_from(BROWSER_SOURCE.rfind("mypkg_autoloaded").unwrap()).unwrap();
        assert!(
            !analysis
                .retained_command_realm()
                .unwrap()
                .invocation_at_source("", head)
                .unobserved_native_dispatch()
        );
        let foreign = suffix(BROWSER_SOURCE, BROWSER_CHILD, "tcl8.6");
        assert!(!advice.is_for_analysis(&foreign));
        assert!(advice.caller_declaration(&foreign).is_none());
        assert!(
            source_native_deferred_autoload_advice_at(
                &(BROWSER_SOURCE.to_owned() + "\n"),
                &analysis,
                at(BROWSER_SOURCE, "mypkg_autoloaded")
            )
            .is_none()
        );
    }

    #[test]
    fn deferred_autoload_channel_envelope_keeps_unknown_script_dynamic_and_occupied_prefixes_terminal()
     {
        for body in [
            "puts [mystery]; puts [mypkg_autoloaded]",
            "eval $script; puts [mypkg_autoloaded]",
            "$callback; puts [mypkg_autoloaded]",
            "puts $channel DATA; puts [mypkg_autoloaded]",
            "puts -nonewline DATA; puts [mypkg_autoloaded]",
            "puts {*}$args; puts [mypkg_autoloaded]",
            "rename greet_helper mypkg_autoloaded; puts [mypkg_autoloaded]",
        ] {
            let source = format!(
                "source lib/helpers.tcl; proc run {{script callback channel args}} {{{body}}}"
            );
            let analysis = suffix(&source, BROWSER_CHILD, "tcl8.6");
            assert!(
                source_native_deferred_autoload_advice_at(
                    &source,
                    &analysis,
                    at(&source, "mypkg_autoloaded")
                )
                .is_none(),
                "unowned callback/input remains terminal: {body}"
            );
        }
    }

    #[test]
    fn deferred_autoload_source_keeps_real_declaring_namespace_and_release_policy() {
        let source = "source lib/helpers.tcl; proc ::ns::run {} {puts [mypkg_autoloaded]}";
        for version in tcl_dialect::TclVersion::ALL {
            let dialect = version.dialect_name();
            let analysis = suffix(source, "namespace eval ::ns {}", dialect);
            let advice = source_native_deferred_autoload_advice_at(
                source,
                &analysis,
                at(source, "mypkg_autoloaded"),
            )
            .expect("actual declared source namespace without an entered Native frame");
            assert_eq!(
                advice.candidates(),
                [
                    NameBytes::from("::ns::mypkg_autoloaded"),
                    NameBytes::from("mypkg_autoloaded")
                ]
            );
            assert!(advice.is_for_analysis(&analysis));
            assert_eq!(
                advice.name().policy().string_protocol(),
                tcl_syntax::native_string::NativeStringProtocol::C(version)
            );
        }
    }

    #[test]
    fn deferred_autoload_source_refuses_unknown_mutating_and_dynamic_body_horizons() {
        let cases = [
            (
                "puts [mystery]; puts [mypkg_autoloaded]",
                "mypkg_autoloaded",
            ),
            (
                "proc mypkg_autoloaded {} {}; puts [mypkg_autoloaded]",
                "mypkg_autoloaded",
            ),
            (
                "rename greet_helper mypkg_autoloaded; puts [mypkg_autoloaded]",
                "mypkg_autoloaded",
            ),
            (
                "rename greet_helper mypkg_autoloaded; rename mypkg_autoloaded {}; puts [mypkg_autoloaded]",
                "mypkg_autoloaded",
            ),
            ("puts [return VALUE]", "return"),
            ("puts [$name]", "$name"),
            ("puts [mypkg_autoloaded $name]", "mypkg_autoloaded"),
            ("puts [mypkg_autoloaded {*}{arg}]", "mypkg_autoloaded"),
            (
                "namespace eval $name {mypkg_autoloaded}",
                "mypkg_autoloaded",
            ),
        ];
        for (body, name) in cases {
            let source = format!("source lib/helpers.tcl; proc run {{name}} {{{body}}}");
            let analysis = suffix(&source, BROWSER_CHILD, "tcl8.6");
            assert!(
                analysis
                    .original_procedure_declarations()
                    .any(|declaration| declaration.declaration_site().offset
                        == u32::try_from(source.find("proc run").unwrap()).unwrap())
            );
            assert!(
                source_native_deferred_autoload_advice_at(&source, &analysis, at(&source, name))
                    .is_none(),
                "unclosed or occupied body horizon: {body}"
            );
        }
        for child in [
            "proc greet_helper {who} {rename mypkg_autoloaded moved}",
            "proc greet_helper {who} {mystery}",
            "proc greet_helper {who} {return [unknown]}",
        ] {
            let analysis = suffix(BROWSER_SOURCE, child, "tcl8.6");
            assert!(
                source_native_deferred_autoload_advice_at(
                    BROWSER_SOURCE,
                    &analysis,
                    at(BROWSER_SOURCE, "mypkg_autoloaded")
                )
                .is_none(),
                "a source procedure name alone cannot prove its lookup effects: {child}"
            );
        }
    }

    #[test]
    fn deferred_autoload_source_refuses_genuine_child_occupants_and_foreign_context() {
        let child = "proc greet_helper {who} {return $who}; proc mypkg_autoloaded {} {}";
        let analysis = suffix(BROWSER_SOURCE, child, "tcl8.6");
        assert!(analysis.original_completed_command_world().is_none());
        assert!(
            source_native_deferred_autoload_advice_at(
                BROWSER_SOURCE,
                &analysis,
                at(BROWSER_SOURCE, "mypkg_autoloaded")
            )
            .is_none()
        );
        let symbolic = Analyser::new().analyse(BROWSER_SOURCE, "tcl");
        assert!(
            source_native_deferred_autoload_advice_at(
                BROWSER_SOURCE,
                &symbolic,
                at(BROWSER_SOURCE, "mypkg_autoloaded")
            )
            .is_none()
        );
    }

    #[test]
    fn deferred_autoload_source_alias_child_requires_real_continuation() {
        const CHILD: &str = "proc greet_helper {who} {return $who}; interp alias {} mypkg_autoloaded {} greet_helper world";
        let prefix = Analyser::new().analyse(BROWSER_SOURCE, "tcl8.6");
        let load = crate::registry_invocation::source_structure::original_project_source_load_at(
            BROWSER_SOURCE,
            &prefix,
            0,
        )
        .expect("actual source Registry role before the alias child");
        let point = load
            .source_point_entry(&prefix)
            .expect("actual source point");
        let child = Analyser::new()
            .analyse_with_source_point_entry(CHILD, "tcl8.6", &point)
            .expect("actual inherited child input");
        assert_eq!(child.original_procedure_declarations().count(), 1);
        assert_eq!(
            child
                .original_procedure_declarations()
                .next()
                .unwrap()
                .name()
                .slot()
                .simple
                .as_bytes(),
            b"greet_helper",
        );
        assert_eq!(
            child
                .retained_command_realm()
                .unwrap()
                .original_source_image()
                .unwrap()
                .bytes(),
            CHILD.as_bytes(),
        );
        assert!(child.original_completed_command_world().is_none());
        assert!(
            load.conditional_continuation(&prefix, &child).is_none(),
            "this alias child has no genuine closed continuation for a suffix advice request"
        );
    }
}
