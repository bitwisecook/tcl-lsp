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

//! Applying one block to the world contents: instruction and region-write effects, held completion-edge transitions and the site proofs captured on replay.

use super::state_transitions::apply_transition_fact;
use super::world_tracks::{TrackVersion, WorldTrack};
use super::{
    CfgShape, DispatchProofAnalysis, IdentityLedger, SiteDispatchProof, SubjectPattern,
    TraceLedger, WorldContents, expr_world_hazard, normalise_name, scope_may_be_current,
    word_world_hazard,
};
use crate::cfg::BlockId;
use crate::effect_ssa::{CallbackSummary, WorldStateIntentKind, project_effect_footprint};
use crate::executable_ir::{
    ExecutableFunction, ExecutableInstruction, ExecutableTerminator, InvocationResolution,
};
use crate::ir::{Statement, WordExpr};
use crate::state_ssa::StateSite;
use crate::state_ssa::adapters::{WorldRegion, WorldRegionKind, WorldSubjectScope};
use crate::world_state_ssa::EdgeCompletion;
use std::sync::Arc;
use tcl_registry::{
    CallbackKinds, CommandBindingTransition, DispatchDependencies, DispatchDependencyDomain,
    EffectFootprint, Reentrancy, StateTransition, StateTransitionCommit, StateTransitionFact,
    StateTransitionKnowledge, TransitionSubject,
};

/// Join predecessor edge states into a block-entry state.  `None` means no
/// predecessor state has been computed yet (a cycle edge before its first
/// visit): the caller skips the block this round rather than fabricating
/// contents; at fixpoint every reachable predecessor edge contributes.
pub(super) fn join_block_entry(
    block: usize,
    entry_index: usize,
    entry_state: &Arc<WorldContents>,
    shape: &CfgShape,
    edge_out: &[Vec<Option<Arc<WorldContents>>>],
) -> Option<Arc<WorldContents>> {
    let block_id = u32::try_from(block).unwrap_or(u32::MAX);
    let mut joined: Option<Arc<WorldContents>> =
        (block == entry_index).then(|| Arc::clone(entry_state));
    for (predecessor, edge) in &shape.predecessors[block] {
        if !shape.reachable[*predecessor] {
            continue;
        }
        let Some(incoming) = edge_out[*predecessor][*edge].as_ref() else {
            continue;
        };
        match &mut joined {
            None => joined = Some(Arc::clone(incoming)),
            Some(current) => {
                if !Arc::ptr_eq(current, incoming) && **current != **incoming {
                    Arc::make_mut(current).join(incoming, block_id);
                }
            }
        }
    }
    joined
}

/// Transition facts held for completion-edge application.
struct HeldTransitions<'f> {
    pub(super) facts: &'f [StateTransitionFact],
    site: TrackVersion,
}

impl HeldTransitions<'_> {
    fn fold_unconditionally(&self, state: &mut Arc<WorldContents>) {
        let mut committed = (**state).clone();
        for fact in self.facts {
            apply_transition_fact(&mut committed, &fact.transition, self.site);
        }
        if committed != **state {
            let block = self.site_block();
            Arc::make_mut(state).join(&committed, block);
        }
    }

    fn apply_for_edge(
        &self,
        state: &Arc<WorldContents>,
        edge: EdgeCompletion,
    ) -> Arc<WorldContents> {
        if edge.ok && !edge.abrupt {
            let mut committed = (**state).clone();
            for fact in self.facts {
                apply_transition_fact(&mut committed, &fact.transition, self.site);
            }
            return Arc::new(committed);
        }
        let applicable: Vec<&StateTransitionFact> = self
            .facts
            .iter()
            .filter(|fact| {
                edge.ok || fact.commit == StateTransitionCommit::MayCommitBeforeAbruptCompletion
            })
            .collect();
        if applicable.is_empty() {
            return Arc::clone(state);
        }
        let mut committed = (**state).clone();
        for fact in applicable {
            apply_transition_fact(&mut committed, &fact.transition, self.site);
        }
        if committed == **state {
            return Arc::clone(state);
        }
        let mut joined = (**state).clone();
        joined.join(&committed, self.site_block());
        Arc::new(joined)
    }

    fn site_block(&self) -> u32 {
        match self.site {
            TrackVersion::Site { block, .. } | TrackVersion::Phi { block } => block,
            TrackVersion::Entry => 0,
        }
    }
}

/// Apply one block's instructions and produce its per-edge outgoing states.
pub(super) fn apply_block(
    function: &ExecutableFunction,
    block: usize,
    mut state: Arc<WorldContents>,
    shape: &CfgShape,
) -> Vec<Option<Arc<WorldContents>>> {
    let held = apply_block_instructions(function, block, &mut state);
    let dispatched = held.is_some()
        && matches!(
            function.blocks[block].terminator,
            Some(ExecutableTerminator::CompletionSwitch { .. })
        );
    if let Some(held) = &held
        && !dispatched
    {
        held.fold_unconditionally(&mut state);
    }
    shape.successors[block]
        .iter()
        .enumerate()
        .map(|(edge, _)| {
            let out = match (&held, dispatched) {
                (Some(held), true) => held.apply_for_edge(&state, shape.completions[block][edge]),
                _ => Arc::clone(&state),
            };
            Some(out)
        })
        .collect()
}

/// Walk one block's instructions, mutating `state` copy-on-write, and return
/// the trailing transition-bearing invocation's facts for edge application.
fn apply_block_instructions<'f>(
    function: &'f ExecutableFunction,
    block: usize,
    state: &mut Arc<WorldContents>,
) -> Option<HeldTransitions<'f>> {
    let block_id = u32::try_from(block).unwrap_or(u32::MAX);
    let mut held: Option<HeldTransitions<'f>> = None;
    for (index, instruction) in function.blocks[block].instructions.iter().enumerate() {
        if let Some(previous) = held.take() {
            // A transition-bearing invocation not dispatched by this block's
            // terminator loses edge precision: fold both outcomes.
            previous.fold_unconditionally(state);
        }
        let site = TrackVersion::Site {
            block: block_id,
            instruction: u32::try_from(index).unwrap_or(u32::MAX),
        };
        held = apply_instruction(state, instruction, block, index, site);
    }
    held
}

/// Replay one block from its joined entry state, capturing invocation-site
/// proofs from the state as it stands when dispatch begins.
pub(super) fn capture_block_sites(
    function: &ExecutableFunction,
    block: usize,
    entry: &Arc<WorldContents>,
    analysis: &mut DispatchProofAnalysis,
    subject: &dyn Fn(
        &crate::executable_ir::GenericInvoke,
        &tcl_registry::InvocationFacts,
    ) -> Option<String>,
) {
    let block_id = u32::try_from(block).unwrap_or(u32::MAX);
    let mut state = Arc::clone(entry);
    let mut held: Option<HeldTransitions<'_>> = None;
    for (index, instruction) in function.blocks[block].instructions.iter().enumerate() {
        if let Some(previous) = held.take() {
            previous.fold_unconditionally(&mut state);
        }
        if let ExecutableInstruction::Invoke(invoke) = instruction
            && let InvocationResolution::Resolved(resolved) = &invoke.resolution
        {
            let words = invoke
                .effective_words
                .as_ref()
                .map_or(invoke.original_words.as_slice(), |words| {
                    words.words.as_slice()
                });
            if let Some(subject) = subject(invoke, resolved) {
                let proof = capture_site_proof(&state, &subject, words);
                analysis
                    .sites
                    .insert((block_id, u32::try_from(index).unwrap_or(u32::MAX)), proof);
            }
        }
        let site = TrackVersion::Site {
            block: block_id,
            instruction: u32::try_from(index).unwrap_or(u32::MAX),
        };
        held = apply_instruction(&mut state, instruction, block, index, site);
    }
}

fn capture_site_proof(
    state: &WorldContents,
    subject: &str,
    words: &[WordExpr],
) -> SiteDispatchProof {
    let mut covers = DispatchDependencies::NONE;
    for domain in [
        DispatchDependencyDomain::CommandBinding,
        DispatchDependencyDomain::NamespaceLookup,
        DispatchDependencyDomain::CommandTraces,
        DispatchDependencyDomain::InterpreterPolicy,
        DispatchDependencyDomain::ObjectDispatch,
        DispatchDependencyDomain::UnknownHandling,
    ] {
        if domain_stable_for_subject(state, domain, subject) {
            covers = covers.union(DispatchDependencies::one(domain));
        }
    }
    let operand_words_admissible = words
        .iter()
        .skip(1)
        .all(|word| word_admissible(state, word));
    SiteDispatchProof {
        covers,
        versions: state.versions,
        operand_words_admissible,
    }
}

fn domain_stable_for_subject(
    state: &WorldContents,
    domain: DispatchDependencyDomain,
    subject: &str,
) -> bool {
    match domain {
        DispatchDependencyDomain::CommandBinding => state.bindings.subject_stable(subject),
        DispatchDependencyDomain::NamespaceLookup => state.namespace_lookup_stable,
        DispatchDependencyDomain::CommandTraces => {
            !state.execution_traces.step_hazard()
                && !state.execution_traces.target_may_be_observed(subject)
                && !state.command_traces.target_may_be_observed(subject)
        }
        DispatchDependencyDomain::InterpreterPolicy => state.interpreter_policy_stable,
        DispatchDependencyDomain::ObjectDispatch => state.object_dispatch.subject_stable(subject),
        DispatchDependencyDomain::UnknownHandling => {
            state.namespace_unknown_stable && state.bindings.untouched()
        }
    }
}

/// Whether one operand word is admissible for value reuse: its evaluation
/// must be provably free of world observation, and expansion is declined
/// because it changes the argv shape the registry facts were resolved for.
fn word_admissible(state: &WorldContents, word: &WordExpr) -> bool {
    match word {
        WordExpr::Expand { .. } => false,
        _ => !word_world_hazard(state, word),
    }
}

/// Apply one instruction's unconditional effects; return transition facts
/// that must be applied per completion edge.
fn apply_instruction<'f>(
    state: &mut Arc<WorldContents>,
    instruction: &'f ExecutableInstruction,
    block: usize,
    index: usize,
    site: TrackVersion,
) -> Option<HeldTransitions<'f>> {
    match instruction {
        ExecutableInstruction::EvaluateWord { word, .. } => {
            if word_world_hazard(state, word) && !state.fully_widened() {
                Arc::make_mut(state).widen_all(site);
            }
            None
        }
        // Neither these nor the structured-control operations below carry a
        // command dispatch of their own; a body statement that does is a
        // separate instruction on the same graph.
        ExecutableInstruction::ExpandWord { .. }
        | ExecutableInstruction::BuildArgv { .. }
        | ExecutableInstruction::MatchPattern { .. }
        | ExecutableInstruction::IterateLists { .. }
        | ExecutableInstruction::JoinCompletion { .. }
        | ExecutableInstruction::WriteCompletionCell { .. }
        | ExecutableInstruction::CompleteStructuredRegion(_) => None,
        ExecutableInstruction::Invoke(invoke) => {
            apply_invoke(state, &invoke.resolution, block, index, site)
        }
        ExecutableInstruction::ExecuteLowered(operation) => {
            apply_lowered_statement(state, &operation.statement, site);
            None
        }
        ExecutableInstruction::ExecuteOpaqueRegion(_) => {
            if !state.fully_widened() {
                Arc::make_mut(state).widen_all(site);
            }
            None
        }
        ExecutableInstruction::CompleteEvaluatedRegion(completed) => {
            let footprint = completed.region.residual_effects.resolve();
            let state_site = StateSite::statement(
                BlockId(u32::try_from(block).unwrap_or(u32::MAX)),
                u32::try_from(index).unwrap_or(u32::MAX),
                0,
            );
            match project_effect_footprint(&footprint, &state_site, None) {
                Ok(intents) => {
                    for intent in intents {
                        if intent.kind != WorldStateIntentKind::Use {
                            apply_region_write(Arc::make_mut(state), &intent.location, site);
                        }
                    }
                }
                Err(_) => Arc::make_mut(state).widen_all(site),
            }
            None
        }
        // A condition or operand containing a command substitution still
        // widens, exactly as a word evaluation does.
        ExecutableInstruction::EvaluateExpr { expr, .. } => {
            if executable_expr_world_hazard(expr) && !state.fully_widened() {
                Arc::make_mut(state).widen_all(site);
            }
            None
        }
    }
}

/// Whether evaluating a structured-control operand can run commands that
/// change mutable world contents.
fn executable_expr_world_hazard(expr: &crate::executable_ir::ExecutableExpr) -> bool {
    use crate::executable_ir::ExecutableExpr;
    match expr {
        ExecutableExpr::Condition { expr, .. } => !expr.command_texts().is_empty(),
        ExecutableExpr::Operand { text, braced } => !*braced && text.contains('['),
        ExecutableExpr::TrapPrefix { .. } => false,
    }
}

fn apply_invoke<'f>(
    state: &mut Arc<WorldContents>,
    resolution: &'f InvocationResolution,
    block: usize,
    index: usize,
    site: TrackVersion,
) -> Option<HeldTransitions<'f>> {
    let InvocationResolution::Resolved(resolved) = resolution else {
        if !state.fully_widened() {
            Arc::make_mut(state).widen_all(site);
        }
        return None;
    };
    let footprint = resolved.world_state_effects();
    if !footprint.accesses().is_empty() || footprint.requires_world_barrier() {
        let state_site = StateSite::statement(
            BlockId(u32::try_from(block).unwrap_or(u32::MAX)),
            u32::try_from(index).unwrap_or(u32::MAX),
            0,
        );
        // A registry-declared trace-only callback barrier is vacuous when
        // the contents lattice proves no trace registration is live: no
        // handler exists for the operation to fire.
        let empty_callback_footprint = EffectFootprint::default();
        let summary = trace_only_barrier_discharged(state, resolved)
            .then(|| CallbackSummary::closed(&empty_callback_footprint));
        match project_effect_footprint(footprint, &state_site, summary) {
            Ok(intents) => {
                for intent in intents {
                    if intent.kind != WorldStateIntentKind::Use {
                        apply_region_write(Arc::make_mut(state), &intent.location, site);
                    }
                }
            }
            Err(_) => Arc::make_mut(state).widen_all(site),
        }
    }
    match &resolved.state_transitions {
        StateTransitionKnowledge::Declared(transitions) => {
            let facts = transitions.facts();
            (!facts.is_empty()).then_some(HeldTransitions { facts, site })
        }
        StateTransitionKnowledge::UnknownInvocation => {
            if !state.fully_widened() {
                Arc::make_mut(state).widen_all(site);
            }
            None
        }
    }
}

/// Whether a world barrier is registry-declared to consist solely of
/// current-interpreter trace callbacks that the contents lattice proves
/// cannot fire.
///
/// The registry's [`CallbackKinds`] vocabulary is the authority: a `TRACE`
/// callback fires only through a live trace registration, so proven absence
/// across every trace class makes the barrier vacuous.  Script, command
/// prefix, host, and unknown callback kinds are never dischargeable here,
/// and any transition reaching beyond the current interpreter keeps the
/// barrier because foreign trace registrations are not tracked.
fn trace_only_barrier_discharged(
    state: &WorldContents,
    resolved: &tcl_registry::InvocationFacts,
) -> bool {
    let callback = resolved.world_state_effects().callback();
    if callback.kinds != CallbackKinds::TRACE
        || callback.reentrancy != Reentrancy::CurrentInterpreter
    {
        return false;
    }
    let StateTransitionKnowledge::Declared(transitions) = &resolved.state_transitions else {
        return false;
    };
    transitions
        .facts()
        .iter()
        .all(|fact| transition_confined_to_current_interpreter(&fact.transition))
        && state.execution_traces.completely_absent()
        && state.command_traces.completely_absent()
        && state.variable_traces.completely_absent()
}

fn transition_confined_to_current_interpreter(transition: &StateTransition) -> bool {
    fn definitely_current(subject: &TransitionSubject) -> bool {
        subject.literal().is_some_and(|path| path.trim().is_empty())
    }
    match transition {
        StateTransition::CommandBinding(transition) => match transition {
            CommandBindingTransition::Define { .. } | CommandBindingTransition::Move { .. } => true,
            CommandBindingTransition::Delete { interpreter, .. } => {
                interpreter.as_ref().is_none_or(definitely_current)
            }
            CommandBindingTransition::Alias {
                source_interpreter,
                target_interpreter,
                ..
            } => definitely_current(source_interpreter) && definitely_current(target_interpreter),
            CommandBindingTransition::Unknown { .. } => false,
        },
        StateTransition::Interpreter(_) | StateTransition::Widen(_) => false,
        StateTransition::VariableCellAlias(_)
        | StateTransition::Namespace(_)
        | StateTransition::Trace(_)
        | StateTransition::Package(_)
        | StateTransition::ObjectDispatch(_) => true,
    }
}

/// Conservative ledger damage for an ordinary (non-transition) region write.
fn apply_region_write(state: &mut WorldContents, region: &WorldRegion, site: TrackVersion) {
    match region {
        WorldRegion::Any => state.widen_all(site),
        WorldRegion::InterpreterWildcard { interpreter } => {
            if scope_may_be_current(interpreter) {
                state.widen_all(site);
            }
        }
        WorldRegion::NamespaceLineage { .. } => {
            state.bump(WorldTrack::NamespaceLookup, site);
        }
        WorldRegion::NamespaceSubtree { interpreter, .. } => {
            if scope_may_be_current(interpreter) {
                state.namespace_lookup_stable = false;
                state.bindings.widen();
                state.object_dispatch.widen();
                state.bump(WorldTrack::NamespaceLookup, site);
                state.bump(WorldTrack::CommandBindings, site);
                state.bump(WorldTrack::ObjectDispatch, site);
                state.bump(WorldTrack::VariableStore, site);
            }
        }
        WorldRegion::Scoped {
            kind,
            interpreter,
            subject,
            ..
        } => {
            if scope_may_be_current(interpreter) {
                apply_scoped_region_write(state, *kind, subject, site);
            }
        }
    }
}

fn apply_scoped_region_write(
    state: &mut WorldContents,
    kind: WorldRegionKind,
    subject: &WorldSubjectScope,
    site: TrackVersion,
) {
    match kind {
        WorldRegionKind::CommandBindings => {
            record_subject_scope(&mut state.bindings, subject);
            state.bump(WorldTrack::CommandBindings, site);
        }
        WorldRegionKind::NamespaceLookup => {
            state.namespace_lookup_stable = false;
            state.bump(WorldTrack::NamespaceLookup, site);
        }
        WorldRegionKind::NamespaceUnknown => {
            state.namespace_unknown_stable = false;
            state.bump(WorldTrack::NamespaceUnknown, site);
        }
        WorldRegionKind::ExecutionTraces => {
            trace_region_damage(&mut state.execution_traces, subject);
            state.bump(WorldTrack::ExecutionTraces, site);
        }
        WorldRegionKind::CommandTraces => {
            trace_region_damage(&mut state.command_traces, subject);
            state.bump(WorldTrack::CommandTraces, site);
        }
        WorldRegionKind::VariableTraces => {
            trace_region_damage(&mut state.variable_traces, subject);
            state.bump(WorldTrack::VariableTraces, site);
        }
        WorldRegionKind::ObjectDispatch => {
            record_subject_scope(&mut state.object_dispatch, subject);
            state.bump(WorldTrack::ObjectDispatch, site);
        }
        WorldRegionKind::InterpreterPolicy => {
            state.interpreter_policy_stable = false;
            state.bump(WorldTrack::InterpreterPolicy, site);
        }
        WorldRegionKind::InterpreterResult => state.bump(WorldTrack::InterpreterResult, site),
        WorldRegionKind::CompletionState => state.bump(WorldTrack::CompletionState, site),
        WorldRegionKind::InterpreterTopology => {
            state.bump(WorldTrack::InterpreterTopology, site);
        }
        WorldRegionKind::VariableStore => {
            // Writing a variable cell fires its write traces, which run
            // arbitrary re-entrant Tcl.
            let hazard = match subject {
                WorldSubjectScope::Named(name) => state.variable_name_may_observe(name),
                WorldSubjectScope::Wildcard => !state.variable_traces.completely_absent(),
            };
            if hazard {
                state.widen_all(site);
            } else {
                state.bump(WorldTrack::VariableStore, site);
            }
        }
        WorldRegionKind::PackageState => state.bump(WorldTrack::PackageState, site),
        WorldRegionKind::HostCapabilities => state.bump(WorldTrack::HostCapabilities, site),
        WorldRegionKind::ExternalResource(_) => state.bump(WorldTrack::ExternalState, site),
    }
}

fn record_subject_scope(ledger: &mut IdentityLedger, subject: &WorldSubjectScope) {
    match subject {
        WorldSubjectScope::Named(name) => {
            ledger.record(SubjectPattern::Exact(normalise_name(name).into_owned()));
        }
        WorldSubjectScope::Wildcard => ledger.widen(),
    }
}

fn trace_region_damage(ledger: &mut TraceLedger, subject: &WorldSubjectScope) {
    match subject {
        WorldSubjectScope::Named(name) => {
            let name = normalise_name(name);
            if let Some(cell) = ledger.cell_mut(&name) {
                // An unknown cell may hold a step registration, which
                // `TraceLedger::step_hazard` derives from this flag.
                cell.unknown = true;
            }
        }
        WorldSubjectScope::Wildcard => ledger.widen(),
    }
}

fn apply_lowered_statement(
    state: &mut Arc<WorldContents>,
    statement: &Statement,
    site: TrackVersion,
) {
    let widen = |state: &mut Arc<WorldContents>| {
        if !state.fully_widened() {
            Arc::make_mut(state).widen_all(site);
        }
    };
    match statement {
        Statement::AssignConst { name, .. } => {
            apply_variable_write(state, name, site);
        }
        Statement::AssignValue {
            name,
            value,
            tokens,
            ..
        } => {
            let value_hazard = match tokens.as_ref().and_then(|tokens| tokens.words().get(2)) {
                Some(word) => word_world_hazard(state, word),
                None => !value_safe_by_text(state, value),
            };
            if value_hazard {
                widen(state);
                return;
            }
            apply_variable_write(state, name, site);
        }
        Statement::AssignExpr { name, expr, .. } => {
            if expr_world_hazard(state, expr) {
                widen(state);
                return;
            }
            apply_variable_write(state, name, site);
        }
        Statement::Incr { name, amount, .. } => {
            if amount
                .as_ref()
                .is_some_and(|amount| !value_safe_by_text(state, amount))
            {
                widen(state);
                return;
            }
            // `incr` both reads and writes the cell; either access may fire
            // a trace.
            apply_variable_write(state, name, site);
        }
        Statement::ExprEval { expr, .. } => {
            if expr_world_hazard(state, expr) {
                widen(state);
            }
        }
        Statement::Return {
            value,
            expr,
            braced,
            ..
        } => {
            let value_hazard = !braced
                && value
                    .as_ref()
                    .is_some_and(|value| !value_safe_by_text(state, value));
            let expr_hazard = expr
                .as_ref()
                .is_some_and(|expr| expr_world_hazard(state, expr));
            if value_hazard || expr_hazard {
                widen(state);
            }
        }
        _ => widen(state),
    }
}

/// Whether raw statement text may perform any substitution at all.
fn text_may_substitute(text: &str) -> bool {
    text.contains('[') || text.contains('$') || text.contains('\\')
}

/// Fallback safety for raw value text: no substitution syntax, or only
/// variable reads that provably cannot fire a trace.
fn value_safe_by_text(state: &WorldContents, text: &str) -> bool {
    if !text_may_substitute(text) {
        return true;
    }
    if text.contains('[') || text.contains('\\') {
        return false;
    }
    // Variable substitutions only: safe exactly when no variable trace can
    // fire at all, since the referenced names are not structurally known.
    state.variable_traces.completely_absent() && !state.aliases.unknown
}

fn apply_variable_write(state: &mut Arc<WorldContents>, name: &str, site: TrackVersion) {
    let dynamic_target = text_may_substitute(name);
    let hazard = if dynamic_target {
        !state.variable_traces.completely_absent() || state.aliases.unknown
    } else {
        state.variable_name_may_observe(name)
    };
    if hazard {
        if !state.fully_widened() {
            Arc::make_mut(state).widen_all(site);
        }
        return;
    }
    Arc::make_mut(state).bump(WorldTrack::VariableStore, site);
}
