// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Conditional suffixes consume the genuine preceding outcome in stack order.

use super::{
    Arc, ModuleCommandBindings, OriginalSourceConditionalContinuation,
    SourceConditionalContinuationPayload, SourceConditionalScriptFrame, SourceOutcomes,
};
use crate::command_binding::{
    OriginalSourcePointEntry, SourceCommandBindings, SourceNamespaceKey, SourceOriginId,
    SourceOriginKind,
};
use std::hash::{Hash, Hasher};
use std::sync::{Mutex, OnceLock, Weak};
use tcl_registry::model::ContextRegistry;

type AdviceGraph = crate::command_binding::source_transition_advice::OriginalSourcePointAdviceGraph;

/// An actual region entry retains its immediate predecessor, never a fresh
/// analysis of the prefix. Equality is the identity of this issued attachment.
#[derive(Clone)]
pub(in crate::command_binding) struct SourceConditionalRegionEntry(Arc<RegionEntryPayload>);
struct RegionEntryPayload {
    continuation: Arc<SourceConditionalContinuationPayload>,
    predecessor: Option<OriginalSourceConditionalRegion>,
    frame_index: usize,
    incoming: Arc<SourceOutcomes>,
    graph: Arc<AdviceGraph>,
}
impl std::fmt::Debug for SourceConditionalRegionEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SourceConditionalRegionEntry")
            .field("frame_index", &self.0.frame_index)
            .field("has_predecessor", &self.0.predecessor.is_some())
            .finish_non_exhaustive()
    }
}
impl PartialEq for SourceConditionalRegionEntry {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for SourceConditionalRegionEntry {}
impl Hash for SourceConditionalRegionEntry {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Arc::as_ptr(&self.0).hash(state);
    }
}
impl SourceConditionalRegionEntry {
    pub(in crate::command_binding) fn owns_procedure_declaration(
        &self,
        declaration: &Arc<
            crate::command_binding::source_transition_advice::OriginalSourceCommandTransitionAdvice,
        >,
    ) -> bool {
        self.0.graph.owns_procedure_declaration(declaration)
    }

    fn frame(&self) -> &SourceConditionalScriptFrame {
        &self.0.continuation.plan.frames[self.0.frame_index]
    }

    pub(in crate::command_binding) fn matches_predecessor(
        &self,
        point: &OriginalSourcePointEntry,
        incoming: &ModuleCommandBindings,
        namespace: &SourceNamespaceKey,
    ) -> bool {
        point == &self.0.continuation.point
            && namespace == &self.frame().context.namespace_key
            && self.0.incoming.normal.as_deref() == Some(incoming)
            && self.0.incoming.normal_completion.is_some()
            && self.0.incoming.abrupt.is_empty()
            && incoming.variable_frame == self.frame().context.frame
    }

    pub(in crate::command_binding) fn incoming_graph(
        &self,
        context: &ContextRegistry,
    ) -> Option<Arc<AdviceGraph>> {
        let expected = self
            .0
            .continuation
            .point
            .input()
            .borrowed_context_registry();
        (context.context() == expected.context()
            && context.commands().snapshot() == expected.commands().snapshot())
        .then(|| Arc::clone(&self.0.graph))
    }

    /// Parse only the actual captured script frame; select only its retained
    /// remaining commands. The image or an arbitrary span cannot issue this plan.
    pub(in crate::command_binding) fn original_plan(
        &self,
        origin: &Arc<SourceOriginId>,
        config: tcl_lexer::LexerConfig,
    ) -> Option<tcl_lexer::NativeScriptWordsPlan> {
        let frame = self.frame();
        if config != frame.context.config {
            return None;
        }
        let end = frame
            .base
            .checked_add(u32::try_from(frame.image.bytes().len()).ok()?)?;
        let mut plan = origin
            .original_script_words_in(
                tcl_lexer::Span::new(frame.base, end),
                config,
                frame.context.source_purpose,
            )
            .ok()?;
        if plan.fatal_tail.is_some() {
            return None;
        }
        let commands = frame.commands.get(frame.next..)?;
        let mut selected = Vec::with_capacity(commands.len());
        for command in commands {
            if command.is_partial {
                return None;
            }
            let mut matches = plan.commands.iter().filter(|original| {
                original
                    .words
                    .first()
                    .is_some_and(|word| word.span().start() == command.span.start())
                    && original.span == command.span
            });
            let original = matches.next()?.clone();
            if matches.next().is_some() {
                return None;
            }
            selected.push(original);
        }
        plan.commands = selected;
        Some(plan)
    }
}

/// One issuer-owned conditional region. This carrier deliberately exposes no
/// whole-document analysis/CU or Native/compiler-purpose conversion.
#[derive(Clone)]
pub struct OriginalSourceConditionalRegion(Arc<RegionPayload>);
struct RegionPayload {
    point: OriginalSourcePointEntry,
    entry: SourceConditionalRegionEntry,
    bindings: Arc<SourceCommandBindings>,
    outcomes: Arc<SourceOutcomes>,
    graph: Option<Arc<AdviceGraph>>,
}
impl std::fmt::Debug for OriginalSourceConditionalRegion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OriginalSourceConditionalRegion")
            .field("frame_index", &self.0.entry.0.frame_index)
            .finish_non_exhaustive()
    }
}
impl PartialEq for OriginalSourceConditionalRegion {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for OriginalSourceConditionalRegion {}

/// A retained ordered chain. Unavailable successors stay explicit: a produced
/// inner prefix never certifies an unproduced outer suffix or whole file.
#[derive(Clone)]
pub struct OriginalSourceConditionalRegionSet(Arc<RegionSetPayload>);
struct RegionSetPayload {
    continuation: Arc<SourceConditionalContinuationPayload>,
    regions: Vec<OriginalSourceConditionalRegion>,
    complete: bool,
    normal: Option<(Arc<SourceOutcomes>, Arc<AdviceGraph>)>,
}
impl std::fmt::Debug for OriginalSourceConditionalRegionSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OriginalSourceConditionalRegionSet")
            .field("region_count", &self.0.regions.len())
            .field("complete", &self.0.complete)
            .finish_non_exhaustive()
    }
}
impl PartialEq for OriginalSourceConditionalRegionSet {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for OriginalSourceConditionalRegionSet {}
impl OriginalSourceConditionalRegionSet {
    /// Actual issued views in inner-to-outer continuation order. An incomplete
    /// set retains only produced views, without a whole-file inventory.
    #[must_use]
    pub fn regions(&self) -> &[OriginalSourceConditionalRegion] {
        &self.0.regions
    }
    /// Whether every saved continuation frame has a normal conditional outcome.
    /// This does not assert physical source-load completion or whole-file entry.
    #[must_use]
    pub fn complete(&self) -> bool {
        self.0.complete
    }
    /// Normal conditional result of this exact ordered chain, not the prefix's
    /// original root result or a physical source-load completion.
    #[must_use]
    pub fn normal_outcome(&self) -> Option<OriginalSourceConditionalOutcome> {
        self.0.normal.as_ref()?;
        Some(OriginalSourceConditionalOutcome(
            ConditionalOutcomeKind::Regions(Arc::clone(&self.0)),
        ))
    }
}

/// Source-only normal outcome of one exact retained continuation chain.
/// Its predecessor and region owners supply no physical source-load completion,
/// runtime entry or compiler visit.
#[derive(Clone)]
pub struct OriginalSourceConditionalOutcome(ConditionalOutcomeKind);
#[derive(Clone)]
enum ConditionalOutcomeKind {
    Regions(Arc<RegionSetPayload>),
    ThroughRegion(Arc<ThroughRegionOutcome>),
}
struct ThroughRegionOutcome {
    region: OriginalSourceConditionalRegion,
    completed: OriginalSourceConditionalOutcome,
    outcomes: Arc<SourceOutcomes>,
    graph: Arc<AdviceGraph>,
}
impl std::fmt::Debug for OriginalSourceConditionalOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OriginalSourceConditionalOutcome")
            .field(
                "through_region",
                &matches!(self.0, ConditionalOutcomeKind::ThroughRegion(_)),
            )
            .finish_non_exhaustive()
    }
}
impl PartialEq for OriginalSourceConditionalOutcome {
    fn eq(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (ConditionalOutcomeKind::Regions(a), ConditionalOutcomeKind::Regions(b)) => {
                Arc::ptr_eq(a, b)
            }
            (
                ConditionalOutcomeKind::ThroughRegion(a),
                ConditionalOutcomeKind::ThroughRegion(b),
            ) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
}
impl Eq for OriginalSourceConditionalOutcome {}
impl OriginalSourceConditionalOutcome {
    fn prefix_point(&self) -> &OriginalSourcePointEntry {
        match &self.0 {
            ConditionalOutcomeKind::Regions(set) => &set.continuation.point,
            ConditionalOutcomeKind::ThroughRegion(outcome) => {
                &outcome.region.0.entry.0.continuation.point
            }
        }
    }
    fn entry_plan(&self) -> &super::SourceConditionalPlan {
        match &self.0 {
            ConditionalOutcomeKind::Regions(set) => &set.continuation.plan,
            ConditionalOutcomeKind::ThroughRegion(outcome) => {
                &outcome.region.0.entry.0.continuation.plan
            }
        }
    }
    /// Rejoin this outcome to the same still-current original prefix analysis.
    /// Host publication and ancestor retirement checks remain independent.
    #[must_use]
    pub fn matches_prefix_analysis(&self, prefix: &crate::analyser::AnalysisResult) -> bool {
        self.prefix_point().matches_parent_analysis(prefix)
            && match &self.0 {
                ConditionalOutcomeKind::Regions(set) => set.complete && set.normal.is_some(),
                ConditionalOutcomeKind::ThroughRegion(_) => true,
            }
    }
    pub(super) fn outcomes_for(
        &self,
        prefix: &crate::analyser::AnalysisResult,
    ) -> Option<Arc<SourceOutcomes>> {
        self.matches_prefix_analysis(prefix).then_some(())?;
        Some(match &self.0 {
            ConditionalOutcomeKind::Regions(set) => Arc::clone(&set.normal.as_ref()?.0),
            ConditionalOutcomeKind::ThroughRegion(outcome) => Arc::clone(&outcome.outcomes),
        })
    }
    pub(super) fn graph_for(
        &self,
        prefix: &crate::analyser::AnalysisResult,
    ) -> Option<Arc<AdviceGraph>> {
        self.matches_prefix_analysis(prefix).then_some(())?;
        Some(match &self.0 {
            ConditionalOutcomeKind::Regions(set) => Arc::clone(&set.normal.as_ref()?.1),
            ConditionalOutcomeKind::ThroughRegion(outcome) => Arc::clone(&outcome.graph),
        })
    }
}

impl OriginalSourceConditionalContinuation {
    /// Continue only the kernel's authentic remaining root/namespace stack.
    /// Host publication/decode applicability remains separate and conditional.
    pub fn source_regions(
        &self,
        parent: &crate::analyser::AnalysisResult,
        child: &crate::analyser::AnalysisResult,
    ) -> Option<OriginalSourceConditionalRegionSet> {
        if !self.matches_parent_analysis(parent) || !self.matches_child_analysis(child) {
            return None;
        }
        static MEMO: OnceLock<Mutex<Vec<Weak<RegionSetPayload>>>> = OnceLock::new();
        let memo = MEMO.get_or_init(|| Mutex::new(Vec::new()));
        let find = |entries: &mut Vec<Weak<RegionSetPayload>>| {
            entries.retain(|entry| entry.strong_count() != 0);
            entries
                .iter()
                .filter_map(Weak::upgrade)
                .find(|entry| Arc::ptr_eq(&entry.continuation, &self.0))
        };
        if let Some(retained) = find(
            &mut memo
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        ) {
            return Some(OriginalSourceConditionalRegionSet(retained));
        }
        // Never hold the interner lock while running the shared source kernel.
        let result = Arc::new(self.replay_region_chain()?);
        let mut memo = memo
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(retained) = find(&mut memo) {
            return Some(OriginalSourceConditionalRegionSet(retained));
        }
        memo.push(Arc::downgrade(&result));
        Some(OriginalSourceConditionalRegionSet(result))
    }

    fn replay_region_chain(&self) -> Option<RegionSetPayload> {
        let context = self.0.point.input().borrowed_context_registry();
        let mut incoming = Arc::clone(&self.0.child_outcomes);
        let mut graph = Arc::clone(&self.0.child_graph);
        let mut regions = Vec::<OriginalSourceConditionalRegion>::new();
        for index in (0..self.0.plan.frames.len()).rev() {
            let frame = &self.0.plan.frames[index];
            if incoming.normal_completion.is_none() || !incoming.abrupt.is_empty() {
                return Some(RegionSetPayload {
                    continuation: Arc::clone(&self.0),
                    regions,
                    complete: false,
                    normal: None,
                });
            }
            let state = incoming.normal.as_deref()?.clone();
            // Each selected child exits in the same source frame as its load.
            // An outer frame is entered only after the previous restoration.
            if state.variable_frame != frame.context.frame {
                return None;
            }
            graph = graph.in_conditional_namespace(&frame.context.namespace_key)?;
            let entry = SourceConditionalRegionEntry(Arc::new(RegionEntryPayload {
                continuation: Arc::clone(&self.0),
                predecessor: regions.last().cloned(),
                frame_index: index,
                incoming: Arc::clone(&incoming),
                graph: Arc::clone(&graph),
            }));
            let RegionWalk {
                region,
                mut outcomes,
                graph: normal_graph,
            } = walk_region(entry, context)?;
            regions.push(region);
            // Completion remains conditional. Restore only this captured,
            // positively selected namespace's real parent activation, in order.
            if outcomes.normal_completion.is_none()
                || outcomes.normal.is_none()
                || !outcomes.abrupt.is_empty()
                || normal_graph.is_none()
            {
                return Some(RegionSetPayload {
                    continuation: Arc::clone(&self.0),
                    regions,
                    complete: false,
                    normal: None,
                });
            }
            if let Some(exit) = &frame.exit {
                if outcomes.normal.as_ref()?.variable_frame != exit.entered {
                    return None;
                }
                outcomes.restore_frame(&exit.parent_variables, &exit.parent);
            } else if index != 0 {
                return None;
            }
            incoming = Arc::new(outcomes);
            graph = normal_graph?;
        }
        let mut normal = (*incoming).clone();
        normal.restore_source_origin(Some(&self.0.point.allocation_site().source));
        Some(RegionSetPayload {
            continuation: Arc::clone(&self.0),
            regions,
            complete: true,
            normal: Some((Arc::new(normal), graph)),
        })
    }
}

struct RegionWalk {
    region: OriginalSourceConditionalRegion,
    outcomes: SourceOutcomes,
    graph: Option<Arc<AdviceGraph>>,
}
fn walk_region(
    entry: SourceConditionalRegionEntry,
    context: &ContextRegistry,
) -> Option<RegionWalk> {
    static MEMO: OnceLock<Mutex<Vec<Weak<RegionPayload>>>> = OnceLock::new();
    let memo = MEMO.get_or_init(|| Mutex::new(Vec::new()));
    let find = |entries: &mut Vec<Weak<RegionPayload>>| {
        entries.retain(|candidate| candidate.strong_count() != 0);
        entries.iter().filter_map(Weak::upgrade).find(|candidate| {
            let previous = &candidate.entry.0;
            Arc::ptr_eq(&previous.continuation, &entry.0.continuation)
                && previous.predecessor == entry.0.predecessor
                && previous.frame_index == entry.0.frame_index
                && previous.incoming == entry.0.incoming
        })
    };
    if let Some(retained) = find(
        &mut memo
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner),
    ) {
        return Some(region_walk(retained));
    }
    let result = Arc::new(walk_region_uncached(entry.clone(), context)?);
    let mut memo = memo
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(retained) = find(&mut memo) {
        return Some(region_walk(retained));
    }
    memo.push(Arc::downgrade(&result));
    Some(region_walk(result))
}

fn region_walk(region: Arc<RegionPayload>) -> RegionWalk {
    RegionWalk {
        outcomes: (*region.outcomes).clone(),
        graph: region.graph.clone(),
        region: OriginalSourceConditionalRegion(region),
    }
}

fn walk_region_uncached(
    entry: SourceConditionalRegionEntry,
    context: &ContextRegistry,
) -> Option<RegionPayload> {
    let frame = entry.frame().clone();
    let point = entry.0.continuation.point.for_conditional_region(
        entry.clone(),
        (**entry.0.incoming.normal.as_ref()?).clone(),
        frame.context.namespace_key.clone(),
    )?;
    let supplied = point.internal_region_entry()?;
    let options = supplied.options();
    let mut state = point.child_state(context.commands(), options, Some(frame.context.config))?;
    let origin = Arc::new(SourceOriginId::from_kind(
        SourceOriginKind::InheritedPoint {
            parent: point.source_origin(),
            source: entry
                .0
                .continuation
                .point
                .allocation_site()
                .source
                .source_image()
                .clone(),
        },
    ));
    state.current_source_origin = Some(Arc::clone(&origin));
    let mut bindings = SourceCommandBindings {
        lexer_config: Some(frame.context.config),
        compilation_scope: supplied.compilation_scope,
        analyser_source_settings: Some(point.analyser_source_settings()),
        root_origin: Some(Arc::clone(&origin)),
        original_analysis_owner: Some(Arc::new(
            crate::command_binding::OriginalSourceAnalysisOwner {
                source: Arc::clone(&origin),
                metadata: state.baseline.metadata_context.clone(),
                config: frame.context.config,
            },
        )),
        ..SourceCommandBindings::default()
    };
    // Reinstall the actual outer stack and exact ordinal; no whole-file
    // source preparation or byte-equivalent prefix reanalysis occurs.
    bindings.conditional_source.frames = entry.0.continuation.plan.frames[..entry.0.frame_index]
        .iter()
        .cloned()
        .map(|mut saved| {
            saved.source = Arc::clone(&origin);
            Some(saved)
        })
        .collect();
    let mut current = frame.clone();
    current.source = Arc::clone(&origin);
    current.ordinal_start = frame.next;
    bindings.conditional_source.resume = Some(current);
    let outcomes = bindings.walk_source_segments(
        &frame.image,
        frame.base,
        frame.commands.get(frame.next..)?,
        &mut state,
        &frame.context.borrowed(context.commands()),
        None,
    );
    bindings.conditional_source.root_outcomes = Some(Arc::new(outcomes.clone()));
    bindings.original_completed_root_state = None;
    if let Some(normal) = &outcomes.normal {
        state = (**normal).clone();
    }
    bindings.final_state = Arc::new(state);
    bindings.invalidate_unpositioned_projections();
    let normal_graph = bindings
        .original_source_transition_seed()?
        .normal_source_outcome_graph(context);
    Some(RegionPayload {
        point,
        entry,
        bindings: Arc::new(bindings),
        outcomes: Arc::new(outcomes),
        graph: normal_graph,
    })
}

impl OriginalSourceConditionalRegion {
    /// Positive original calls from this exact issued region. This supplies no
    /// whole-file catalogue or absence outside its authenticated commands.
    #[must_use]
    pub fn original_procedure_calls(
        &self,
        analysis: &crate::analyser::AnalysisResult,
    ) -> Option<Vec<crate::command_binding::OriginalSourceProcedureCall>> {
        self.matches_region_analysis(analysis).then_some(())?;
        let bindings = self.bindings();
        let owner = bindings.original_analysis_owner()?;
        owner.matches(bindings, self.input()).then_some(())?;
        let context = self.input().context_registry();
        let tape = bindings.original_source_transition_advice_tape_for(
            &context,
            crate::command_binding::source_transition_advice::OriginalSourceCataloguePurpose::ConditionalRegion,
        )?;
        Some(
            tape.native_procedure_calls()
                .filter_map(|call| {
                    let call = call.clone().with_input(
                        self.input(),
                        Arc::clone(owner),
                        bindings.original_inherited_source_entry(),
                    );
                    let head = call.original_words().first()?;
                    (self.owns_original_command_at(analysis, head.span().start())
                        && call.matches_source_context(
                            self.source_image(),
                            self.input().lexer_config(),
                            &context,
                        ))
                    .then_some(call)
                })
                .collect(),
        )
    }

    pub(in crate::command_binding) fn matches_declaration_dependency(
        &self,
        analysis: &crate::analyser::AnalysisResult,
    ) -> bool {
        enum Dependency<'a> {
            Region(&'a OriginalSourceConditionalRegion),
            Continuation(&'a Arc<SourceConditionalContinuationPayload>),
            Outcome(&'a OriginalSourceConditionalOutcome),
        }
        let mut pending = vec![Dependency::Region(self)];
        for _ in 0..4096 {
            let Some(dependency) = pending.pop() else {
                return false;
            };
            match dependency {
                Dependency::Region(region) => {
                    if region.matches_region_analysis(analysis) {
                        return true;
                    }
                    pending.push(Dependency::Continuation(&region.0.entry.0.continuation));
                    if let Some(predecessor) = &region.0.entry.0.predecessor {
                        pending.push(Dependency::Region(predecessor));
                    }
                }
                Dependency::Continuation(payload) => {
                    let continuation = OriginalSourceConditionalContinuation(Arc::clone(payload));
                    if payload.point.matches_ancestor_analysis(analysis)
                        || continuation.matches_child_analysis(analysis)
                    {
                        return true;
                    }
                    if let Some(outcome) = &payload.conditional_outcome {
                        pending.push(Dependency::Outcome(outcome));
                    }
                }
                Dependency::Outcome(outcome) => match &outcome.0 {
                    ConditionalOutcomeKind::Regions(set) => {
                        pending.push(Dependency::Continuation(&set.continuation));
                        pending.extend(set.regions.iter().map(Dependency::Region));
                    }
                    ConditionalOutcomeKind::ThroughRegion(completed) => {
                        pending.push(Dependency::Region(&completed.region));
                        pending.push(Dependency::Outcome(&completed.completed));
                    }
                },
            }
        }
        false
    }

    /// Rejoin the exact parent and opaque child prefix retained by this region.
    /// The host must separately validate physical document and publication leases.
    #[must_use]
    pub fn matches_dependencies(
        &self,
        parent: &crate::analyser::AnalysisResult,
        child_prefix: &crate::analyser::AnalysisResult,
    ) -> bool {
        let continuation =
            OriginalSourceConditionalContinuation(Arc::clone(&self.0.entry.0.continuation));
        continuation.matches_parent_analysis(parent)
            && continuation.matches_child_analysis(child_prefix)
    }
    /// Exact same issuer-owned partial analysis. Host publication and ancestor
    /// leases remain separate; equal bytes or a whole-image match cannot renew it.
    #[must_use]
    pub fn matches_region_analysis(&self, analysis: &crate::analyser::AnalysisResult) -> bool {
        if analysis.original_source_region.as_ref() != Some(self)
            || analysis.analysis_context_unavailable.is_some()
            || analysis.resolved_input.as_ref() != Some(self.input())
            || analysis.body_lexer_config != Some(self.input().lexer_config())
        {
            return false;
        }
        let Some(realm) = analysis.retained_command_realm() else {
            return false;
        };
        let bindings = realm.source_bindings_ref();
        realm.matches_resolved_analysis_input(self.input())
            && realm.matches_original_source_image(self.source_image(), self.input().lexer_config())
            && bindings
                .original_analysis_owner()
                .zip(self.0.bindings.original_analysis_owner())
                .is_some_and(|(actual, expected)| Arc::ptr_eq(actual, expected))
            && bindings.original_inherited_source_entry() == Some(&self.0.point)
    }
    pub(crate) fn input(&self) -> &crate::analyser::ResolvedAnalysisInput {
        self.0.point.input()
    }
    pub(crate) fn source_image(&self) -> &tcl_lexer::SourceImage {
        self.0
            .entry
            .0
            .continuation
            .point
            .allocation_site()
            .source
            .source_image()
    }
    pub(crate) fn commands(&self) -> &[crate::segmenter::SegmentedCommand] {
        let frame = self.0.entry.frame();
        &frame.commands[frame.next..]
    }
    pub(crate) fn bindings(&self) -> &SourceCommandBindings {
        &self.0.bindings
    }
    pub(crate) fn entry(&self) -> Option<crate::command_binding::SourceAnalysisEntry> {
        self.0.point.internal_region_entry()
    }
    pub(crate) fn configure_analyser(&self, analyser: &mut crate::analyser::Analyser) {
        self.0.point.analyser_source_settings().apply(analyser);
    }
    pub(crate) fn accepts_analyser(&self, analyser: &crate::analyser::Analyser) -> bool {
        self.0.point.accepts_analyser_source_settings(analyser)
    }
}

#[cfg(test)]
mod controls {
    use super::*;
    use crate::registry_invocation::source_structure::original_project_source_load_at;

    fn source_load(
        source: &str,
        analysis: &crate::analyser::AnalysisResult,
        text: &str,
    ) -> crate::registry_invocation::source_structure::OriginalProjectSourceLoad {
        original_project_source_load_at(
            source,
            analysis,
            u32::try_from(source.find(text).unwrap()).unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn suffix_view_reuses_the_same_kernel_and_refuses_whole_document_or_foreign_parent() {
        let source = "source first.tcl; set after ONE; source second.tcl";
        let parent = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        let load = source_load(source, &parent, "source first");
        let point = load.source_point_entry(&parent).unwrap();
        let child = crate::analyser::Analyser::new()
            .analyse_with_source_point_entry("", "tcl8.6", &point)
            .unwrap();
        let continuation = load.conditional_continuation(&parent, &child).unwrap();
        let regions = continuation.source_regions(&parent, &child).unwrap();
        assert_eq!(regions.regions().len(), 1);
        assert!(!regions.complete()); // second source remains unavailable
        assert!(regions.normal_outcome().is_none());
        assert_eq!(
            continuation.source_regions(&parent, &child),
            Some(regions.clone())
        );
        let retained_region = regions.regions()[0].clone();
        drop(regions);
        let regions = continuation.source_regions(&parent, &child).unwrap();
        assert_eq!(regions.regions().first(), Some(&retained_region));
        let region = &regions.regions()[0];
        assert_eq!(
            region
                .commands()
                .iter()
                .map(|command| command.texts[0].as_str())
                .collect::<Vec<_>>(),
            ["set", "source"]
        );
        let view = crate::analyser::Analyser::new()
            .analyse_source_region(region, &parent, &child)
            .unwrap();
        assert!(region.matches_region_analysis(&view));
        assert_eq!(view.original_source_region(), Some(region));
        assert!(crate::source_graph::current_analysis(source, &view).is_none());
        let registry = region.input().borrowed_context_registry().commands();
        assert!(
            view.original_optimiser_source_unit(source, registry, None)
                .is_none()
        );
        assert!(
            view.original_source_analysis_entry(source, registry, None)
                .is_none()
        );
        let actual = view.retained_command_realm().unwrap().source_bindings_ref();
        assert!(Arc::ptr_eq(
            actual.original_analysis_owner().unwrap(),
            region.bindings().original_analysis_owner().unwrap()
        ));
        assert!(actual.invocation_unpositioned("set").unknown);
        let mut actual_tokens = crate::ir::CommandTokens::from_segmented(
            &region.source_image().source_map(),
            region.input().lexer_config(),
            &region.commands()[0],
        );
        actual.stamp_original_tokens(&mut actual_tokens);
        let actual_binding = actual_tokens.source_binding.as_ref().unwrap();
        assert!(actual_binding.original_analysis_owner.is_some());
        assert!(
            actual_binding
                .original_source_argument_syntax(
                    &actual_tokens,
                    region.input(),
                    region.input().borrowed_context_registry().commands(),
                )
                .is_none()
        );
        assert!(
            actual
                .conditional_source
                .root_outcomes
                .as_ref()
                .unwrap()
                .native_normal_completion
                .is_none()
        );
        assert!(actual.final_state.baseline.native_entry.is_none());
        assert_eq!(
            actual.final_state.baseline.native_compilation,
            Default::default()
        );
        assert!(actual.original_completed_command_world().is_none());
        assert!(actual.projection_at_source("set", 0).unknown);
        assert!(!region.owns_original_command_at(&view, 0));
        assert!(region.owns_original_command_at(
            &view,
            u32::try_from(source.find("set after").unwrap()).unwrap()
        ));
        assert!(
            !region.owns_original_command_at(
                &view,
                u32::try_from(source.find("ONE").unwrap()).unwrap()
            )
        );
        assert!(
            source_load(source, &view, "source second")
                .source_point_entry(&view)
                .is_some()
        );
        let foreign_parent = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        assert!(
            crate::analyser::Analyser::new()
                .analyse_source_region(region, &foreign_parent, &child)
                .is_none()
        );
        let mut unavailable = view.clone();
        unavailable.body_lexer_config = None;
        assert!(!region.matches_region_analysis(&unavailable));
        assert!(
            region
                .0
                .point
                .source_analysis_entry(registry, region.input())
                .is_none()
        );
    }

    #[test]
    fn ordered_namespace_suffixes_restore_the_actual_inner_frame_before_outer_walk() {
        let source = "namespace eval app {source child.tcl; set inner INSIDE}; set outer OUTSIDE";
        let parent = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        let load = source_load(source, &parent, "source child");
        let point = load.source_point_entry(&parent).unwrap();
        let child = crate::analyser::Analyser::new()
            .analyse_with_source_point_entry("", "tcl8.6", &point)
            .unwrap();
        let regions = load
            .conditional_continuation(&parent, &child)
            .unwrap()
            .source_regions(&parent, &child)
            .unwrap();
        assert_eq!(regions.regions().len(), 2);
        assert!(regions.complete());
        let inner = &regions.regions()[0];
        let outer = &regions.regions()[1];
        assert_eq!(
            inner
                .0
                .entry
                .frame()
                .context
                .namespace_key
                .display()
                .as_deref(),
            Some("::app")
        );
        assert_eq!(
            outer
                .0
                .entry
                .frame()
                .context
                .namespace_key
                .display()
                .as_deref(),
            Some("::")
        );
        assert_eq!(outer.0.entry.0.predecessor.as_ref(), Some(inner));
        assert_eq!(
            outer
                .0
                .entry
                .0
                .incoming
                .normal
                .as_ref()
                .unwrap()
                .variable_frame,
            outer.0.entry.frame().context.frame
        );
        assert!(
            regions
                .normal_outcome()
                .unwrap()
                .matches_prefix_analysis(&parent)
        );
        let inner_view = crate::analyser::Analyser::new()
            .analyse_source_region(inner, &parent, &child)
            .unwrap();
        let outer_view = crate::analyser::Analyser::new()
            .analyse_source_region(outer, &parent, &child)
            .unwrap();
        assert!(inner.matches_region_analysis(&inner_view));
        assert!(!inner.matches_region_analysis(&outer_view));
        let mut reused = crate::analyser::Analyser::new();
        let same_inner = reused
            .analyse_source_region(inner, &parent, &child)
            .unwrap();
        assert!(inner.matches_region_analysis(&same_inner));
        let fresh = reused.analyse("proc fresh {} {}", "tcl8.6");
        assert!(fresh.original_source_region().is_none());
        assert!(fresh.all_procs.contains_key("::fresh"));
        assert!(!fresh.all_procs.contains_key("::app::fresh"));
    }

    #[test]
    fn deep_return_uses_the_completed_chain_without_renewing_the_opaque_child_prefix() {
        let source = "source child.tcl; set resumed YES";
        let child_source = "source leaf.tcl";
        let parent = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        let load = source_load(source, &parent, "source child");
        let point = load.source_point_entry(&parent).unwrap();
        let child = crate::analyser::Analyser::new()
            .analyse_with_source_point_entry(child_source, "tcl8.6", &point)
            .unwrap();
        assert!(load.conditional_continuation(&parent, &child).is_none());
        let inner_load = source_load(child_source, &child, "source leaf");
        let leaf_point = inner_load.source_point_entry(&child).unwrap();
        let leaf = crate::analyser::Analyser::new()
            .analyse_with_source_point_entry("", "tcl8.6", &leaf_point)
            .unwrap();
        let inner = inner_load
            .conditional_continuation(&child, &leaf)
            .unwrap()
            .source_regions(&child, &leaf)
            .unwrap();
        let outcome = inner.normal_outcome().unwrap();
        assert!(outcome.matches_prefix_analysis(&child));
        let resumed = load
            .conditional_continuation_with_outcome(&parent, &child, &outcome)
            .unwrap();
        let complete = resumed.source_regions(&parent, &child).unwrap();
        assert!(complete.complete());
        assert!(
            complete
                .normal_outcome()
                .unwrap()
                .matches_prefix_analysis(&parent)
        );
        // Ordinary child root result stays unavailable, even after a complete chain.
        assert!(load.conditional_continuation(&parent, &child).is_none());
        let foreign_child = crate::analyser::Analyser::new()
            .analyse_with_source_point_entry(child_source, "tcl8.6", &point)
            .unwrap();
        assert!(!outcome.matches_prefix_analysis(&foreign_child));
        assert!(
            load.conditional_continuation_with_outcome(&parent, &foreign_child, &outcome)
                .is_none()
        );
    }
}

impl OriginalSourceConditionalRegion {
    /// Original captured segments for this exact frame, not a whole-image scan.
    pub(crate) fn original_path_commands(
        &self,
        analysis: &crate::analyser::AnalysisResult,
    ) -> Option<(Vec<crate::segmenter::SegmentedCommand>, SourceNamespaceKey)> {
        self.matches_region_analysis(analysis).then_some(())?;
        let frame = self.0.entry.frame();
        let origin = self.0.bindings.source_origin()?;
        self.0
            .entry
            .original_plan(origin, self.input().lexer_config())?;
        Some((
            frame.commands.get(frame.next..)?.to_vec(),
            frame.context.namespace_key.clone(),
        ))
    }

    /// A replay join retains the exact immediate prefix and the actual issued
    /// frame inventory. A dependency ancestor or matching image is insufficient.
    pub(crate) fn reissues_original_source_command(
        &self,
        prefix: &crate::analyser::AnalysisResult,
        analysis: &crate::analyser::AnalysisResult,
        words: &[tcl_lexer::NativeWord],
    ) -> bool {
        let continuation =
            OriginalSourceConditionalContinuation(Arc::clone(&self.0.entry.0.continuation));
        if !continuation.matches_parent_analysis(prefix) || !self.matches_region_analysis(analysis)
        {
            return false;
        }
        let Some(head) = words.first() else {
            return false;
        };
        self.original_vectors().is_some_and(|vectors| {
            vectors.get(&head.span().start()).and_then(Option::as_deref) == Some(words)
        })
    }
}

impl OriginalSourceConditionalRegion {
    /// Actual command-vector membership, not span containment. Quoted/braced
    /// data cannot create an executable vector. Script-body owners remain a
    /// separate registry-selected purpose and are not inferred from braces.
    #[must_use]
    pub fn owns_original_command_at(
        &self,
        analysis: &crate::analyser::AnalysisResult,
        offset: u32,
    ) -> bool {
        if !self.matches_region_analysis(analysis) {
            return false;
        }
        // Actual selected body descent has its own private kernel point owner;
        // lexical braces alone never supply this alternative.
        let binding = self.0.bindings.invocation_at_source("", offset);
        let selected_body_point = binding.invocation_site().is_some_and(|site| {
            self.0.bindings.source_origin() == Some(&site.source) && site.offset == offset
        }) && self
            .0
            .bindings
            .dispatch_points_at(offset)
            .any(|point| !point.declaration_preview)
            && binding.original_recorded_command().is_some();
        selected_body_point
            || self
                .original_vectors()
                .is_some_and(|vectors| vectors.get(&offset).is_some_and(Option::is_some))
    }

    fn original_vectors(
        &self,
    ) -> Option<std::collections::BTreeMap<u32, Option<Vec<tcl_lexer::NativeWord>>>> {
        let origin = self.0.bindings.source_origin()?;
        let plan = self
            .0
            .point
            .original_region_plan(origin, self.input().lexer_config())?;
        let mut vectors = std::collections::BTreeMap::new();
        retain_original_vectors(&plan, 0, &mut vectors)?;
        Some(vectors)
    }
}

fn retain_original_vectors(
    plan: &tcl_lexer::NativeScriptWordsPlan,
    depth: u32,
    vectors: &mut std::collections::BTreeMap<u32, Option<Vec<tcl_lexer::NativeWord>>>,
) -> Option<()> {
    if crate::depth_guard::MAX_SOURCE_NEST_DEPTH.exceeded(depth) || plan.fatal_tail.is_some() {
        return None;
    }
    for command in &plan.commands {
        let offset = command.words.first()?.span().start();
        vectors
            .entry(offset)
            .and_modify(|previous| {
                if previous.as_ref() != Some(&command.words) {
                    *previous = None;
                }
            })
            .or_insert_with(|| Some(command.words.clone()));
        for word in &command.words {
            for part in word.executable_parts().all_parts() {
                if let tcl_lexer::ExecutablePart::Command { body } = part.part {
                    let nested = plan.reborrow_command_substitution_plan(word, body).ok()?;
                    retain_original_vectors(&nested, depth.checked_add(1)?, vectors)?;
                }
            }
        }
    }
    Some(())
}

impl OriginalSourceConditionalRegion {
    /// Rejoin a completed sequential tail with its original unchanged prefix.
    /// The exact issued region and saved outer frames remain in the receipt;
    /// neither a fresh prefix nor URI/text equality can supply this join.
    pub fn return_outcome(
        &self,
        parent_prefix: &crate::analyser::AnalysisResult,
        first_child_prefix: &crate::analyser::AnalysisResult,
        region_analysis: &crate::analyser::AnalysisResult,
        completed: &OriginalSourceConditionalOutcome,
    ) -> Option<OriginalSourceConditionalOutcome> {
        if !self.matches_dependencies(parent_prefix, first_child_prefix)
            || !self.matches_region_analysis(region_analysis)
            || !completed.matches_prefix_analysis(region_analysis)
            || !self.matches_remaining_stack(completed.entry_plan())
        {
            return None;
        }
        static MEMO: OnceLock<Mutex<Vec<Weak<ThroughRegionOutcome>>>> = OnceLock::new();
        let memo = MEMO.get_or_init(|| Mutex::new(Vec::new()));
        let find = |entries: &mut Vec<Weak<ThroughRegionOutcome>>| {
            entries.retain(|entry| entry.strong_count() != 0);
            entries
                .iter()
                .filter_map(Weak::upgrade)
                .find(|entry| entry.region == *self && entry.completed == *completed)
        };
        if let Some(retained) = find(
            &mut memo
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        ) {
            return Some(OriginalSourceConditionalOutcome(
                ConditionalOutcomeKind::ThroughRegion(retained),
            ));
        }
        let outcome = Arc::new(self.project_completed_tail(region_analysis, completed)?);
        let mut memo = memo
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(retained) = find(&mut memo) {
            return Some(OriginalSourceConditionalOutcome(
                ConditionalOutcomeKind::ThroughRegion(retained),
            ));
        }
        memo.push(Arc::downgrade(&outcome));
        Some(OriginalSourceConditionalOutcome(
            ConditionalOutcomeKind::ThroughRegion(outcome),
        ))
    }

    fn matches_remaining_stack(&self, actual: &super::SourceConditionalPlan) -> bool {
        let entry = &self.0.entry.0;
        let prefix = &entry.continuation.plan.frames[..entry.frame_index];
        let Some(current) = actual.frames.get(entry.frame_index) else {
            return false;
        };
        let Some(origin) = self.0.bindings.source_origin() else {
            return false;
        };
        for (expected, actual) in prefix.iter().zip(actual.frames.iter()) {
            let mut expected = expected.clone();
            expected.source = Arc::clone(origin);
            if &expected != actual {
                return false;
            }
        }
        let mut expected = self.0.entry.frame().clone();
        if current.next <= expected.next || current.next > expected.commands.len() {
            return false;
        }
        expected.source = Arc::clone(origin);
        expected.ordinal_start = expected.next;
        expected.next = current.next;
        &expected == current
    }

    fn project_completed_tail(
        &self,
        region_analysis: &crate::analyser::AnalysisResult,
        completed: &OriginalSourceConditionalOutcome,
    ) -> Option<ThroughRegionOutcome> {
        let mut outcomes = (*completed.outcomes_for(region_analysis)?).clone();
        let root = self.0.entry.0.continuation.plan.frames.first()?;
        let normal = outcomes.normal.as_ref()?;
        if outcomes.normal_completion.is_none()
            || !outcomes.abrupt.is_empty()
            || outcomes.native_normal_completion.is_some()
            || normal.variable_frame != root.context.frame
        {
            return None;
        }
        let graph = completed
            .graph_for(region_analysis)?
            .in_conditional_namespace(&root.context.namespace_key)?;
        outcomes.restore_source_origin(Some(
            &self.0.entry.0.continuation.point.allocation_site().source,
        ));
        Some(ThroughRegionOutcome {
            region: self.clone(),
            completed: completed.clone(),
            outcomes: Arc::new(outcomes),
            graph,
        })
    }
}

#[cfg(test)]
mod sequential_controls {
    use crate::registry_invocation::source_structure::original_project_source_load_at;

    #[test]
    fn sequential_tail_returns_to_the_same_original_prefix_without_reanalysis() {
        let source = "source first.tcl; source second.tcl; set after DONE";
        let parent = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        let first = original_project_source_load_at(source, &parent, 0).unwrap();
        let point = first.source_point_entry(&parent).unwrap();
        let child = crate::analyser::Analyser::new()
            .analyse_with_source_point_entry("", "tcl8.6", &point)
            .unwrap();
        let pending = first
            .conditional_continuation(&parent, &child)
            .unwrap()
            .source_regions(&parent, &child)
            .unwrap();
        assert!(!pending.complete());
        let region = &pending.regions()[0];
        let region_view = crate::analyser::Analyser::new()
            .analyse_source_region(region, &parent, &child)
            .unwrap();
        let offset = u32::try_from(source.find("source second").unwrap()).unwrap();
        let second = original_project_source_load_at(source, &region_view, offset).unwrap();
        let second_point = second.source_point_entry(&region_view).unwrap();
        let second_child = crate::analyser::Analyser::new()
            .analyse_with_source_point_entry("", "tcl8.6", &second_point)
            .unwrap();
        let resumed = second
            .conditional_continuation(&region_view, &second_child)
            .unwrap()
            .source_regions(&region_view, &second_child)
            .unwrap();
        assert!(resumed.complete());
        let completed = resumed.normal_outcome().unwrap();
        assert!(completed.matches_prefix_analysis(&region_view));
        assert!(!completed.matches_prefix_analysis(&parent));
        let returned = region
            .return_outcome(&parent, &child, &region_view, &completed)
            .unwrap();
        assert!(returned.matches_prefix_analysis(&parent));
        assert_eq!(
            region.return_outcome(&parent, &child, &region_view, &completed),
            Some(returned)
        );
        let foreign = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        assert!(
            region
                .return_outcome(&foreign, &child, &region_view, &completed)
                .is_none()
        );
        assert!(
            first
                .conditional_continuation(&parent, &child)
                .unwrap()
                .source_regions(&parent, &child)
                .unwrap()
                .normal_outcome()
                .is_none()
        );
    }

    #[test]
    fn twelve_literal_source_hops_return_only_through_the_original_live_chain() {
        let mut prefixes = Vec::new();
        let mut source = "source hop0.tcl".to_owned();
        let mut analysis = crate::analyser::Analyser::new().analyse(&source, "tcl8.6");
        for index in 0..12 {
            let load = original_project_source_load_at(&source, &analysis, 0).unwrap();
            let point = load.source_point_entry(&analysis).unwrap();
            let next = if index == 11 {
                String::new()
            } else {
                format!("source hop{}.tcl", index + 1)
            };
            let mut analyser = crate::analyser::Analyser::new();
            assert!(point.configure_analyser(&analysis, &mut analyser));
            let child = analyser
                .analyse_with_source_point_entry(&next, "tcl8.6", &point)
                .unwrap();
            prefixes.push((source, analysis, load));
            source = next;
            analysis = child;
        }
        let mut completed = None;
        for (_, prefix, load) in prefixes.iter().rev() {
            let continuation = match completed.as_ref() {
                Some(outcome) => {
                    load.conditional_continuation_with_outcome(prefix, &analysis, outcome)
                }
                None => load.conditional_continuation(prefix, &analysis),
            }
            .unwrap();
            let regions = continuation.source_regions(prefix, &analysis).unwrap();
            assert!(regions.complete());
            completed = regions.normal_outcome();
            assert!(completed.as_ref().unwrap().matches_prefix_analysis(prefix));
            analysis = prefix.clone();
        }
        let (_, root, _) = &prefixes[0];
        assert!(completed.unwrap().matches_prefix_analysis(root));
        assert!(root.original_source_region().is_none());
    }
}

#[cfg(test)]
mod declaration_dependency_controls {
    #[test]
    fn region_call_joins_only_the_actual_completed_child_declaration_owner() {
        use crate::analyser::Analyser;
        use crate::registry_invocation::source_structure::original_project_source_load_at;
        let source = "source child.tcl; helper";
        let parent = Analyser::new().analyse(source, "tcl8.6");
        let load = original_project_source_load_at(source, &parent, 0).unwrap();
        let point = load.source_point_entry(&parent).unwrap();
        let child_source = "proc helper {} {}";
        let child = Analyser::new()
            .analyse_with_source_point_entry(child_source, "tcl8.6", &point)
            .unwrap();
        let continuation = load.conditional_continuation(&parent, &child).unwrap();
        let regions = continuation.source_regions(&parent, &child).unwrap();
        assert_eq!(regions.regions().len(), 1);
        let region = &regions.regions()[0];
        let caller = Analyser::new()
            .analyse_source_region(region, &parent, &child)
            .unwrap();
        assert!(region.matches_region_analysis(&caller));
        let offset = u32::try_from(source.rfind("helper").unwrap()).unwrap();
        let calls = region.original_procedure_calls(&caller).unwrap();
        let call = calls
            .iter()
            .find(|call| call.original_words()[0].span().start() == offset)
            .unwrap();
        assert!(
            crate::registry_invocation::source_structure::source_original_procedure_call_at(
                source, &caller, offset
            )
            .is_none()
        );
        let declaration = call
            .declaration_in_region_dependency(&caller, &child)
            .unwrap();
        assert_eq!(declaration.metadata().name, "helper");
        assert!(call.declaration_in_parent(&caller, &child).is_none());
        assert!(call.declaration(&caller).is_none());

        let foreign = Analyser::new()
            .analyse_with_source_point_entry(child_source, "tcl8.6", &point)
            .unwrap();
        assert!(point.matches_child_analysis(&foreign));
        assert!(!continuation.matches_child_analysis(&foreign));
        assert!(
            call.declaration_in_region_dependency(&caller, &foreign)
                .is_none()
        );
        assert!(
            call.declaration_in_region_dependency(&caller, &parent)
                .is_none()
        );
        let mut unavailable = caller.clone();
        unavailable.analysis_context_unavailable = Some(tcl_registry::model::OverlayMiss {
            environment: "tcl8.6".to_owned(),
            overlay: 1,
        });
        assert!(
            call.declaration_in_region_dependency(&unavailable, &child)
                .is_none()
        );
        let mut wrong_config = caller.clone();
        wrong_config.body_lexer_config = None;
        assert!(
            call.declaration_in_region_dependency(&wrong_config, &child)
                .is_none()
        );
        let mut unavailable_child = child.clone();
        unavailable_child.body_lexer_config = None;
        assert!(
            call.declaration_in_region_dependency(&caller, &unavailable_child)
                .is_none()
        );
    }
}

#[cfg(test)]
mod foreign_completed_region_controls {
    use super::*;
    use crate::analyser::{Analyser, AnalysisResult};
    use crate::registry_invocation::source_structure::{
        OriginalProjectSourceLoad, original_project_source_load_at,
    };

    fn completed_tail(
        source: &str,
        parent: &AnalysisResult,
        first: &OriginalProjectSourceLoad,
        child: &AnalysisResult,
    ) -> (
        OriginalSourceConditionalRegion,
        AnalysisResult,
        OriginalSourceConditionalOutcome,
    ) {
        let pending = first
            .conditional_continuation(parent, child)
            .unwrap()
            .source_regions(parent, child)
            .unwrap();
        assert!(!pending.complete());
        assert_eq!(pending.regions().len(), 1);
        let region = pending.regions()[0].clone();
        let view = Analyser::new()
            .analyse_source_region(&region, parent, child)
            .unwrap();
        assert!(region.matches_region_analysis(&view));
        let offset = u32::try_from(source.find("source second").unwrap()).unwrap();
        let second = original_project_source_load_at(source, &view, offset).unwrap();
        let point = second.source_point_entry(&view).unwrap();
        let second_child = Analyser::new()
            .analyse_with_source_point_entry("", "tcl8.6", &point)
            .unwrap();
        let complete = second
            .conditional_continuation(&view, &second_child)
            .unwrap()
            .source_regions(&view, &second_child)
            .unwrap();
        assert!(complete.complete());
        let outcome = complete.normal_outcome().unwrap();
        assert!(outcome.matches_prefix_analysis(&view));
        (region, view, outcome)
    }

    #[test]
    fn return_outcome_refuses_a_byte_equal_foreign_completed_region_owner() {
        let source = "source first.tcl; source second.tcl; set after DONE";
        let parent = Analyser::new().analyse(source, "tcl8.6");
        let first = original_project_source_load_at(source, &parent, 0).unwrap();
        let point = first.source_point_entry(&parent).unwrap();
        let child = Analyser::new()
            .analyse_with_source_point_entry("", "tcl8.6", &point)
            .unwrap();
        let (region, view, completed) = completed_tail(source, &parent, &first, &child);
        let returned = region
            .return_outcome(&parent, &child, &view, &completed)
            .unwrap();
        assert!(returned.matches_prefix_analysis(&parent));

        // Keep the same parent and source point. A genuine second child analysis
        // issues an independent attachment, despite its identical source bytes.
        let other_child = Analyser::new()
            .analyse_with_source_point_entry("", "tcl8.6", &point)
            .unwrap();
        let (other_region, other_view, other_completed) =
            completed_tail(source, &parent, &first, &other_child);
        assert_ne!(region, other_region);
        assert!(!other_region.matches_region_analysis(&view));
        assert!(other_completed.matches_prefix_analysis(&other_view));
        assert!(!other_completed.matches_prefix_analysis(&view));
        assert!(
            other_region
                .return_outcome(&parent, &other_child, &other_view, &other_completed)
                .is_some()
        );
        assert!(
            region
                .return_outcome(&parent, &child, &view, &other_completed)
                .is_none()
        );
    }
}
