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

//! World-state contents/absence lattice and typed dispatch-stability proofs.
//!
//! World-state SSA ([`crate::world_state_ssa`]) versions the mutable
//! interpreter world but deliberately does not model what any version
//! contains.  A registry-resolved call can therefore be referentially
//! transparent with respect to its result and still be observable through
//! Tcl's mutable dispatch machinery: execution traces, `rename`, aliases,
//! namespace imports and ensembles, safe-interpreter policy, and `unknown`
//! handling.  This module is the missing contents half: a forward dataflow
//! analysis over the executable semantic CFG that tracks, per dispatch
//! domain, whether the domain provably still has its declared entry contents
//! and no live observer.
//!
//! Design rules mirrored from the rest of the common compiler:
//!
//! - **No command names.**  Transfer functions consume only typed registry
//!   [`tcl_registry::StateTransition`] facts, ordinary effect-footprint
//!   intents, and structured word shapes.  A command, trace, namespace,
//!   interpreter, or object mutation reaches this pass exclusively as
//!   registry data.
//! - **Fail closed.**  An unresolved head, an undeclared invocation, an
//!   opaque region, a lowered statement whose operand words this pass cannot
//!   prove inert, and every dynamic transition operand widen conservatively.
//! - **Completion-edge fidelity.**  A transition commits on the `TCL_OK`
//!   edge; on an abrupt edge it follows the registry's
//!   [`StateTransitionCommit`] contract, joining changed and unchanged state
//!   exactly as the world-state renamer does.
//!
//! The entry state is a typed contract ([`DispatchEntryAssumption`]), not a
//! heuristic: the driver states what may be assumed about workspace, package,
//! sourced-file, and interpreter history at function entry, and
//! [`DispatchEntryAssumption::UnknownWorld`] fails every proof closed.
//!
//! Cost model: the fixpoint is a worklist over CFG blocks with precomputed
//! predecessor edges — `O(edges × lattice-height)` block applications, one
//! pass on the linear compatibility CFG.  Per-edge states are `Arc`-shared
//! copy-on-write, so an instruction that changes nothing forwards its
//! incoming state without allocating, and every ledger in the abstract state
//! is bounded by a small constant, making one state O(1) to clone, join, and
//! compare.

use std::borrow::Cow;
use std::collections::VecDeque;
use std::sync::Arc;

use rustc_hash::FxHashMap;

use tcl_registry::{
    CallbackKinds, CommandBindingTransition, DispatchDependencies, DispatchDependencyDomain,
    EffectFootprint, InterpreterTransition, NamespaceTransition, NamespaceTransitionTarget,
    ObjectDispatchTarget, ObjectDispatchTransition, Reentrancy, StateTransition,
    StateTransitionCommit, StateTransitionDomain, StateTransitionFact, StateTransitionKnowledge,
    TraceOperation, TraceOperationSet, TraceTarget, TraceTransition, TransitionSubject,
    VariableAliasTarget, WorldStateDomain,
};

use crate::cfg::BlockId;
use crate::effect_ssa::{CallbackSummary, WorldStateIntentKind, project_effect_footprint};
use crate::executable_ir::{
    ExecutableFunction, ExecutableInstruction, ExecutableTerminator, InvocationResolution,
};
use crate::ir::{Statement, WordExpr, WordPart};
use crate::state_ssa::StateSite;
use crate::state_ssa::adapters::{
    WorldInterpreterScope, WorldRegion, WorldRegionKind, WorldSubjectScope,
};
use crate::world_state_ssa::{EdgeCompletion, edge_completion, successors_of};

/// Bound on distinct changed-subject patterns tracked per identity ledger.
const MAX_TRACKED_SUBJECTS: usize = 16;
/// Bound on distinct traced targets tracked per trace ledger.
const MAX_TRACE_CELLS: usize = 8;
/// Bound on distinct registrations tracked per traced target.
const MAX_TRACE_REGISTRATIONS: usize = 8;
/// Bound on tracked variable alias links.
const MAX_ALIAS_LINKS: usize = 8;
/// Saturation point for registration counting.
const COUNT_SATURATION: u8 = 8;
/// Bound on per-block applications relative to block count.  The lattice has
/// constant height, so genuine convergence needs far fewer; exceeding the
/// bound fails closed.
const MAX_BLOCK_APPLICATIONS_PER_BLOCK: usize = 64;

/// What the driver may assume about the interpreter world at function entry.
///
/// This is the typed representation of workspace, package, sourced-file, and
/// interpreter history demanded by the dispatch-stability contract.  It is an
/// explicit input chosen by the analysis driver, never inferred from source
/// text, and it is deliberately extensible: a workspace-aware driver can add
/// a variant carrying aggregated cross-file facts without touching this
/// pass's transfer functions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DispatchEntryAssumption {
    /// Dispatch state at entry is the registry's baseline for the selected
    /// dialect: registry command bindings are intact and no trace, alias,
    /// ensemble, or interpreter-policy observer is live.
    ///
    /// The production driver applies this to a compilation unit's top-level
    /// script, modelling evaluation of the file in a fresh interpreter.  The
    /// resulting O105 findings are advisory (hint severity, no auto-fix); a
    /// stronger action-tier proof must additionally justify this assumption
    /// from workspace facts.
    PristineRegistryWorld,
    /// Nothing may be assumed: every domain starts widened and every site
    /// proof fails closed.
    ///
    /// Applied to procedure, method, and nested body units, whose bodies run
    /// only after arbitrary interposed top-level and cross-file history.
    UnknownWorld,
}

/// One versioned partition of the tracked world, at dispatch-proof
/// granularity.
///
/// Tracks deliberately mirror [`WorldRegionKind`] (minus the per-resource
/// split of external state) so that world-state SSA, effect footprints, and
/// this pass name the same partitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorldTrack {
    /// Name-to-command bindings in the current interpreter.
    CommandBindings,
    /// Namespace lookup, import, export, ensemble, and path state.
    NamespaceLookup,
    /// Namespace-specific `unknown` handler state.
    NamespaceUnknown,
    /// Command execution (enter/leave/step) trace registrations.
    ExecutionTraces,
    /// Command rename/delete trace registrations.
    CommandTraces,
    /// Variable trace registrations.
    VariableTraces,
    /// `TclOO` class, object, and method dispatch state.
    ObjectDispatch,
    /// Safe-interpreter and hidden-command policy of the current interpreter.
    InterpreterPolicy,
    /// Current interpreter result storage.
    InterpreterResult,
    /// Current interpreter completion metadata.
    CompletionState,
    /// Child-interpreter existence and identity.
    InterpreterTopology,
    /// Tcl variable cells.
    VariableStore,
    /// Package-loading state.
    PackageState,
    /// Host-provided capability state.
    HostCapabilities,
    /// Externally visible resources retained from the legacy effect model.
    ExternalState,
}

impl WorldTrack {
    /// Every track, in deterministic declaration order.
    pub const ALL: [Self; 15] = [
        Self::CommandBindings,
        Self::NamespaceLookup,
        Self::NamespaceUnknown,
        Self::ExecutionTraces,
        Self::CommandTraces,
        Self::VariableTraces,
        Self::ObjectDispatch,
        Self::InterpreterPolicy,
        Self::InterpreterTopology,
        Self::VariableStore,
        Self::PackageState,
        Self::HostCapabilities,
        Self::ExternalState,
        Self::InterpreterResult,
        Self::CompletionState,
    ];

    const fn index(self) -> usize {
        match self {
            Self::CommandBindings => 0,
            Self::NamespaceLookup => 1,
            Self::NamespaceUnknown => 2,
            Self::ExecutionTraces => 3,
            Self::CommandTraces => 4,
            Self::VariableTraces => 5,
            Self::ObjectDispatch => 6,
            Self::InterpreterPolicy => 7,
            Self::InterpreterTopology => 8,
            Self::VariableStore => 9,
            Self::PackageState => 10,
            Self::HostCapabilities => 11,
            Self::ExternalState => 12,
            Self::InterpreterResult => 13,
            Self::CompletionState => 14,
        }
    }

    /// Stable label used when a track version participates in a value key.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::CommandBindings => "cmdbind",
            Self::NamespaceLookup => "nslookup",
            Self::NamespaceUnknown => "nsunknown",
            Self::ExecutionTraces => "exectrace",
            Self::CommandTraces => "cmdtrace",
            Self::VariableTraces => "vartrace",
            Self::ObjectDispatch => "oodispatch",
            Self::InterpreterPolicy => "interppolicy",
            Self::InterpreterTopology => "interptopo",
            Self::VariableStore => "varstore",
            Self::PackageState => "package",
            Self::HostCapabilities => "host",
            Self::ExternalState => "external",
            Self::InterpreterResult => "interpresult",
            Self::CompletionState => "completion",
        }
    }

    /// Project a registry versioned-world domain onto its track.
    #[must_use]
    pub const fn from_world_state_domain(domain: WorldStateDomain) -> Self {
        match domain {
            WorldStateDomain::InterpreterTopology => Self::InterpreterTopology,
            WorldStateDomain::CommandBindings => Self::CommandBindings,
            WorldStateDomain::NamespaceLookup => Self::NamespaceLookup,
            WorldStateDomain::NamespaceUnknown => Self::NamespaceUnknown,
            WorldStateDomain::ExecutionTraces => Self::ExecutionTraces,
            WorldStateDomain::VariableTraces => Self::VariableTraces,
            WorldStateDomain::CommandTraces => Self::CommandTraces,
            WorldStateDomain::OoDispatch => Self::ObjectDispatch,
            WorldStateDomain::InterpreterPolicy => Self::InterpreterPolicy,
            WorldStateDomain::InterpreterResult => Self::InterpreterResult,
            WorldStateDomain::CompletionState => Self::CompletionState,
            WorldStateDomain::PackageState => Self::PackageState,
            WorldStateDomain::HostCapabilities => Self::HostCapabilities,
            WorldStateDomain::VariableStore => Self::VariableStore,
            WorldStateDomain::LegacyExternal(_) => Self::ExternalState,
        }
    }

    /// The tracks whose versions key one dispatch dependency domain.
    #[must_use]
    pub const fn for_dispatch_dependency(domain: DispatchDependencyDomain) -> &'static [Self] {
        match domain {
            DispatchDependencyDomain::CommandBinding => &[Self::CommandBindings],
            DispatchDependencyDomain::NamespaceLookup => &[Self::NamespaceLookup],
            DispatchDependencyDomain::CommandTraces => {
                &[Self::ExecutionTraces, Self::CommandTraces]
            }
            DispatchDependencyDomain::InterpreterPolicy => {
                &[Self::InterpreterPolicy, Self::InterpreterTopology]
            }
            DispatchDependencyDomain::ObjectDispatch => &[Self::ObjectDispatch],
            DispatchDependencyDomain::UnknownHandling => {
                &[Self::NamespaceUnknown, Self::CommandBindings]
            }
        }
    }
}

/// A value-numbering token for one track: equal tokens prove equal contents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TrackVersion {
    /// Unchanged since function entry.
    Entry,
    /// Last changed by the instruction at the given coordinates.
    Site {
        /// Executable block index.
        block: u32,
        /// Instruction index within the block.
        instruction: u32,
    },
    /// Joined from differing predecessor versions at the given block.
    Phi {
        /// Executable block index owning the join.
        block: u32,
    },
}

impl TrackVersion {
    fn render(self, out: &mut String) {
        use std::fmt::Write as _;
        match self {
            Self::Entry => out.push('e'),
            Self::Site { block, instruction } => {
                let _ = write!(out, "b{block}i{instruction}");
            }
            Self::Phi { block } => {
                let _ = write!(out, "p{block}");
            }
        }
    }
}

/// A saturating interval of live-registration counts.
///
/// C Tcl appends a fresh trace record for every `trace add` (duplicates
/// included) and `trace remove` deletes the most recent exact match or
/// succeeds as a no-op, so registration liveness is a counting question.
/// The interval stays sound under either duplicate policy: an idempotent
/// add would only make the upper bound conservative.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CountRange {
    lo: u8,
    hi: Option<u8>,
}

impl CountRange {
    const ZERO: Self = Self { lo: 0, hi: Some(0) };

    fn add_one(self) -> Self {
        Self {
            lo: self.lo.saturating_add(1).min(COUNT_SATURATION),
            hi: match self.hi {
                Some(hi) if hi < COUNT_SATURATION => Some(hi + 1),
                _ => None,
            },
        }
    }

    /// Remove one provably matching registration.
    fn remove_exact(self) -> Self {
        Self {
            lo: self.lo.saturating_sub(1),
            hi: self.hi.map(|hi| hi.saturating_sub(1)),
        }
    }

    /// A removal that may or may not have matched: the lower bound weakens,
    /// the upper bound must stay.
    fn remove_maybe(self) -> Self {
        Self {
            lo: self.lo.saturating_sub(1),
            hi: self.hi,
        }
    }

    fn join(self, other: Self) -> Self {
        Self {
            lo: self.lo.min(other.lo),
            hi: match (self.hi, other.hi) {
                (Some(a), Some(b)) => Some(a.max(b)),
                _ => None,
            },
        }
    }

    fn may_be_live(self) -> bool {
        self.hi != Some(0)
    }
}

/// One canonical trace registration identity: operation list plus prefix.
///
/// `None` for the prefix records a registration whose command prefix was a
/// dynamic Tcl value: it still observes, but no literal removal can be
/// proven to match it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct TraceKey {
    operations: Vec<TraceOperation>,
    prefix: Option<String>,
}

/// Live-registration knowledge for one traced target.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct TraceCell {
    unknown: bool,
    registrations: Vec<(TraceKey, CountRange)>,
}

impl TraceCell {
    fn may_be_observed(&self) -> bool {
        self.unknown
            || self
                .registrations
                .iter()
                .any(|(_, range)| range.may_be_live())
    }

    fn add(&mut self, key: TraceKey) {
        if self.unknown {
            return;
        }
        if let Some((_, range)) = self
            .registrations
            .iter_mut()
            .find(|(known, _)| *known == key)
        {
            *range = range.add_one();
            return;
        }
        if self.registrations.len() >= MAX_TRACE_REGISTRATIONS {
            self.unknown = true;
            self.registrations.clear();
            return;
        }
        self.registrations.push((key, CountRange::ZERO.add_one()));
    }

    fn remove(&mut self, key: &TraceKey) {
        if self.unknown {
            return;
        }
        let ambiguous = key.prefix.is_none()
            || self
                .registrations
                .iter()
                .any(|(known, _)| known.prefix.is_none() && known.operations == key.operations);
        if ambiguous {
            self.remove_any();
            return;
        }
        if let Some((_, range)) = self
            .registrations
            .iter_mut()
            .find(|(known, _)| known == key)
        {
            *range = range.remove_exact();
        }
    }

    /// A removal whose match cannot be identified: at most one registration
    /// went away, so only lower bounds weaken.
    fn remove_any(&mut self) {
        for (_, range) in &mut self.registrations {
            *range = range.remove_maybe();
        }
    }

    fn join(&mut self, other: &Self) {
        if self.unknown || other.unknown {
            self.unknown = true;
            self.registrations.clear();
            return;
        }
        for (key, other_range) in &other.registrations {
            if let Some((_, range)) = self
                .registrations
                .iter_mut()
                .find(|(known, _)| known == key)
            {
                *range = range.join(*other_range);
            } else if self.registrations.len() >= MAX_TRACE_REGISTRATIONS {
                self.unknown = true;
                self.registrations.clear();
                return;
            } else {
                self.registrations
                    .push((key.clone(), CountRange::ZERO.join(*other_range)));
            }
        }
        for (key, range) in &mut self.registrations {
            if !other
                .registrations
                .iter()
                .any(|(other_key, _)| other_key == key)
            {
                *range = range.join(CountRange::ZERO);
            }
        }
    }
}

/// How a variable access selects elements of its cell.
enum ElementAccess<'a> {
    /// A scalar access: only whole-variable traces apply.
    Scalar,
    /// One literal array element.
    Known(&'a str),
    /// A computed array element: any element trace of the base applies.
    Dynamic,
}

/// Live-trace knowledge for one trace class (execution, command, variable).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct TraceLedger {
    widened: bool,
    cells: Vec<(String, TraceCell)>,
}

impl TraceLedger {
    fn widen(&mut self) {
        self.widened = true;
        self.cells.clear();
    }

    /// Whether a live registration may carry a step operation.
    ///
    /// A step trace observes every command nested inside its target rather
    /// than only the target itself, so one live step registration anywhere
    /// suppresses reuse everywhere. This is derived from the registrations
    /// rather than latched: a stored flag could only ever be set, so
    /// removing the last `enterstep` registration would leave the hazard
    /// asserted forever and suppress reuse with no observer live.
    fn step_hazard(&self) -> bool {
        self.widened
            || self.cells.iter().any(|(_, cell)| {
                cell.unknown
                    || cell.registrations.iter().any(|(key, range)| {
                        range.may_be_live()
                            && key.operations.iter().any(|operation| {
                                matches!(
                                    operation,
                                    TraceOperation::EnterStep | TraceOperation::LeaveStep
                                )
                            })
                    })
            })
    }

    fn completely_absent(&self) -> bool {
        !self.widened && self.cells.iter().all(|(_, cell)| !cell.may_be_observed())
    }

    fn cell_mut(&mut self, name: &str) -> Option<&mut TraceCell> {
        if self.widened {
            return None;
        }
        if let Some(position) = self.cells.iter().position(|(known, _)| known == name) {
            return Some(&mut self.cells[position].1);
        }
        if self.cells.len() >= MAX_TRACE_CELLS {
            self.widen();
            return None;
        }
        self.cells.push((name.to_owned(), TraceCell::default()));
        let last = self.cells.len() - 1;
        Some(&mut self.cells[last].1)
    }

    fn target_may_be_observed(&self, name: &str) -> bool {
        self.widened
            || self
                .cells
                .iter()
                .any(|(known, cell)| known == name && cell.may_be_observed())
    }

    /// Whether an access to variable `base` may fire a trace: whole-variable
    /// traces always apply, element traces by the element relation.
    fn variable_access_may_observe(&self, base: &str, element: &ElementAccess<'_>) -> bool {
        if self.widened {
            return true;
        }
        self.cells.iter().any(|(known, cell)| {
            if !cell.may_be_observed() {
                return false;
            }
            if known == base {
                return true;
            }
            let Some(rest) = known.strip_prefix(base) else {
                return false;
            };
            if !rest.starts_with('(') {
                return false;
            }
            match element {
                ElementAccess::Scalar => false,
                ElementAccess::Known(element) => {
                    rest.strip_prefix('(')
                        .and_then(|rest| rest.strip_suffix(')'))
                        == Some(*element)
                }
                ElementAccess::Dynamic => true,
            }
        })
    }

    fn join(&mut self, other: &Self) {
        if self.widened || other.widened {
            self.widen();
            return;
        }
        for (name, other_cell) in &other.cells {
            if let Some(position) = self.cells.iter().position(|(known, _)| known == name) {
                self.cells[position].1.join(other_cell);
            } else if self.cells.len() >= MAX_TRACE_CELLS {
                self.widen();
                return;
            } else {
                let mut cell = TraceCell::default();
                cell.join(other_cell);
                self.cells.push((name.clone(), cell));
            }
        }
    }
}

/// A changed-subject pattern in an identity ledger.
#[derive(Debug, Clone, PartialEq, Eq)]
enum SubjectPattern {
    /// One normalised subject name.
    Exact(String),
    /// Every subject inside one normalised namespace (empty = global).
    NamespacePrefix(String),
}

impl SubjectPattern {
    fn matches(&self, candidate: &str) -> bool {
        match self {
            Self::Exact(name) => name == candidate,
            Self::NamespacePrefix(prefix) => {
                prefix.is_empty()
                    || candidate == prefix
                    || tcl_syntax::naming::authored_namespace_hint_tail(candidate, prefix).is_some()
            }
        }
    }
}

/// Identity knowledge for a subject-keyed domain (bindings, object dispatch).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct IdentityLedger {
    widened: bool,
    changed: Vec<SubjectPattern>,
}

impl IdentityLedger {
    fn widen(&mut self) {
        self.widened = true;
        self.changed.clear();
    }

    fn record(&mut self, pattern: SubjectPattern) {
        if self.widened || self.changed.contains(&pattern) {
            return;
        }
        if self.changed.len() >= MAX_TRACKED_SUBJECTS {
            self.widen();
            return;
        }
        self.changed.push(pattern);
    }

    fn record_subject(&mut self, subject: &TransitionSubject) {
        match subject.literal() {
            Some(name) => self.record(SubjectPattern::Exact(normalise_name(name).into_owned())),
            None => self.widen(),
        }
    }

    fn subject_stable(&self, candidate: &str) -> bool {
        !self.widened
            && !self
                .changed
                .iter()
                .any(|pattern| pattern.matches(candidate))
    }

    fn untouched(&self) -> bool {
        !self.widened && self.changed.is_empty()
    }

    fn join(&mut self, other: &Self) {
        if self.widened || other.widened {
            self.widen();
            return;
        }
        for pattern in &other.changed {
            self.record(pattern.clone());
            if self.widened {
                return;
            }
        }
    }
}

/// Known variable-cell alias links established by `upvar`-style transitions.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct AliasLedger {
    unknown: bool,
    links: Vec<(String, Option<String>)>,
}

impl AliasLedger {
    fn widen(&mut self) {
        self.unknown = true;
        self.links.clear();
    }

    fn record(&mut self, local: String, target: Option<String>) {
        if self.unknown {
            return;
        }
        if let Some(position) = self.links.iter().position(|(name, _)| *name == local) {
            self.links[position].1 = target;
            return;
        }
        if self.links.len() >= MAX_ALIAS_LINKS {
            self.widen();
            return;
        }
        self.links.push((local, target));
    }

    fn join(&mut self, other: &Self) {
        if self.unknown || other.unknown {
            self.widen();
            return;
        }
        // A link present on only one predecessor may or may not exist after
        // the join, so its target becomes unknown. This has to run in *both*
        // directions. Weakening only the incoming side leaves a link this
        // side already held pointing at a concrete cell, even though the
        // other path never aliased that local — a variable operand would then
        // be checked against the alias target's traces while a trace on the
        // unaliased local cell went unnoticed, yielding a false proof. It
        // would also make the result depend on predecessor visit order.
        for (name, target) in &mut self.links {
            if !other.links.iter().any(|(known, _)| known == name) {
                *target = None;
            }
        }
        for (name, target) in &other.links {
            if let Some(position) = self.links.iter().position(|(local, _)| local == name) {
                if self.links[position].1 != *target {
                    self.links[position].1 = None;
                }
            } else if self.links.len() >= MAX_ALIAS_LINKS {
                self.widen();
                return;
            } else {
                // Present on one path only: after the join the local name
                // may resolve to an unknown cell.
                self.links.push((name.clone(), None));
            }
        }
    }
}

/// The complete abstract world at one program point.
#[derive(Debug, Clone, PartialEq, Eq)]
struct WorldContents {
    variable_grammar: Option<tcl_lexer::LexerConfig>,
    versions: [TrackVersion; WorldTrack::ALL.len()],
    bindings: IdentityLedger,
    namespace_lookup_stable: bool,
    namespace_unknown_stable: bool,
    execution_traces: TraceLedger,
    command_traces: TraceLedger,
    variable_traces: TraceLedger,
    object_dispatch: IdentityLedger,
    interpreter_policy_stable: bool,
    aliases: AliasLedger,
}

impl WorldContents {
    fn entry(assumption: DispatchEntryAssumption) -> Self {
        let mut state = Self {
            variable_grammar: None,
            versions: [TrackVersion::Entry; WorldTrack::ALL.len()],
            bindings: IdentityLedger::default(),
            namespace_lookup_stable: true,
            namespace_unknown_stable: true,
            execution_traces: TraceLedger::default(),
            command_traces: TraceLedger::default(),
            variable_traces: TraceLedger::default(),
            object_dispatch: IdentityLedger::default(),
            interpreter_policy_stable: true,
            aliases: AliasLedger::default(),
        };
        if assumption == DispatchEntryAssumption::UnknownWorld {
            state.widen_all(TrackVersion::Entry);
        }
        state
    }

    fn bump(&mut self, track: WorldTrack, version: TrackVersion) {
        self.versions[track.index()] = version;
    }

    fn widen_all(&mut self, version: TrackVersion) {
        for track in WorldTrack::ALL {
            self.bump(track, version);
        }
        self.bindings.widen();
        self.namespace_lookup_stable = false;
        self.namespace_unknown_stable = false;
        self.execution_traces.widen();
        self.command_traces.widen();
        self.variable_traces.widen();
        self.object_dispatch.widen();
        self.interpreter_policy_stable = false;
        self.aliases.widen();
    }

    /// Whether every proof-bearing component is already at lattice top, in
    /// which case further transfers cannot change anything but versions.
    fn fully_widened(&self) -> bool {
        self.bindings.widened
            && !self.namespace_lookup_stable
            && !self.namespace_unknown_stable
            && self.execution_traces.widened
            && self.command_traces.widened
            && self.variable_traces.widened
            && self.object_dispatch.widened
            && !self.interpreter_policy_stable
            && self.aliases.unknown
    }

    fn join(&mut self, other: &Self, block: u32) {
        for (index, version) in self.versions.iter_mut().enumerate() {
            if *version != other.versions[index] {
                *version = TrackVersion::Phi { block };
            }
        }
        self.bindings.join(&other.bindings);
        self.namespace_lookup_stable &= other.namespace_lookup_stable;
        self.namespace_unknown_stable &= other.namespace_unknown_stable;
        self.execution_traces.join(&other.execution_traces);
        self.command_traces.join(&other.command_traces);
        self.variable_traces.join(&other.variable_traces);
        self.object_dispatch.join(&other.object_dispatch);
        self.interpreter_policy_stable &= other.interpreter_policy_stable;
        self.aliases.join(&other.aliases);
    }

    /// Whether source substitution may run an observer or an unmodelled
    /// index evaluation, following one alias hop.
    fn variable_access_may_observe(&self, spelling: &str) -> bool {
        let Some(reference) = self
            .variable_grammar
            .and_then(|config| parse_variable_reference(spelling, config))
        else {
            return true;
        };
        matches!(reference.element, ElementAccess::Dynamic)
            || self.resolved_variable_access_may_observe(&reference)
    }

    /// A native name operand and an alias target are already evaluated names.
    /// Their literal sigils and element keys are not substitution syntax.
    fn variable_name_may_observe(&self, name: &str) -> bool {
        self.resolved_variable_access_may_observe(&literal_variable_reference(name))
    }

    fn resolved_variable_access_may_observe(&self, reference: &VariableReference<'_>) -> bool {
        if self.aliases.unknown {
            return !self.variable_traces.completely_absent();
        }
        let base = normalise_name(reference.base);
        match self
            .aliases
            .links
            .iter()
            .find(|(local, _)| *local == base.as_ref())
        {
            None => self
                .variable_traces
                .variable_access_may_observe(&base, &reference.element),
            // The target may itself name an array element, in which case
            // both the element's and the whole array's traces apply.
            Some((_, Some(target))) => {
                let target = literal_variable_reference(target);
                self.variable_traces
                    .variable_access_may_observe(target.base, &target.element)
            }
            Some((_, None)) => !self.variable_traces.completely_absent(),
        }
    }
}

/// A parsed Tcl variable reference: base name plus element selection.
struct VariableReference<'a> {
    base: &'a str,
    element: ElementAccess<'a>,
}

fn literal_variable_reference(name: &str) -> VariableReference<'_> {
    match tcl_syntax::naming::split_element_ref(name) {
        Some((base, element)) => VariableReference {
            base,
            element: ElementAccess::Known(element),
        },
        None => VariableReference {
            base: name,
            element: ElementAccess::Scalar,
        },
    }
}

/// Project a complete substitution under its retained source grammar.
/// Combined braced names remain literal data; separately substituted or
/// escape-decoded indexes retain an unknown element relation.
fn parse_variable_reference(
    spelling: &str,
    config: tcl_lexer::LexerConfig,
) -> Option<VariableReference<'_>> {
    let reference = tcl_lexer::word_parts::whole_var_ref(spelling.as_bytes(), config).ok()??;
    let base = std::str::from_utf8(reference.name).ok()?;
    let Some(index) = reference.index else {
        return Some(literal_variable_reference(base));
    };
    let index = std::str::from_utf8(index).ok()?;
    Some(VariableReference {
        base,
        element: if tcl_syntax::naming::array_key_is_literal(index) {
            ElementAccess::Known(index)
        } else {
            ElementAccess::Dynamic
        },
    })
}

/// Written-name keys for the explicit authored world-ledger model.
/// Native lookup and selected descriptor labels retain their separate owners.
fn normalise_name(name: &str) -> Cow<'_, str> {
    if !name.contains(':') {
        return Cow::Borrowed(name);
    }
    let rooted = tcl_syntax::naming::canonical_written_command(name);
    Cow::Owned(
        tcl_syntax::naming::unroot_rooted_key(&rooted)
            .unwrap_or(&rooted)
            .to_owned(),
    )
}

/// The [`normalise_name`] key of `variable` written in `namespace`. The two
/// written words are qualified separately, so an absolute variable ignores the
/// namespace and a colon-named namespace keeps its own segment.
fn namespace_alias_target(namespace: &str, variable: &str) -> String {
    let namespace = tcl_syntax::naming::qualify_namespace("::", namespace);
    let rooted = tcl_syntax::naming::qualify(&namespace, variable);
    tcl_syntax::naming::unroot_rooted_key(&rooted)
        .unwrap_or(&rooted)
        .to_owned()
}

/// Whether an interpreter-path operand can denote the interpreter this
/// function runs in.  Tcl spells the current interpreter as the empty path.
fn interpreter_may_be_current(subject: &TransitionSubject) -> bool {
    match subject.literal() {
        Some(path) => path.trim().is_empty(),
        None => true,
    }
}

fn scope_may_be_current(scope: &WorldInterpreterScope) -> bool {
    match scope {
        WorldInterpreterScope::Current | WorldInterpreterScope::Any => true,
        WorldInterpreterScope::Named(path) => path.trim().is_empty(),
    }
}

/// Whether a word evaluation may observe or mutate the world: literals never
/// do; a variable read does exactly when a live variable trace can fire on
/// it; command substitutions and opaque words always may.
fn word_world_hazard(state: &WorldContents, word: &WordExpr) -> bool {
    match word {
        WordExpr::Literal { .. } | WordExpr::BracedLiteral { .. } => false,
        WordExpr::Variable { spelling, .. } => state.variable_access_may_observe(spelling),
        WordExpr::CommandSubstitution { .. } | WordExpr::Opaque { .. } => true,
        WordExpr::Template { parts, .. } => parts.iter().any(|part| match part {
            WordPart::Text { .. } => false,
            WordPart::Variable { spelling, .. } => state.variable_access_may_observe(spelling),
            WordPart::CommandSubstitution { .. } | WordPart::Opaque { .. } => true,
        }),
        WordExpr::Expand { word, .. } => word_world_hazard(state, word),
    }
}

/// Whether an expression evaluation may observe or mutate the world.
///
/// Command substitutions and math-function applications dispatch commands
/// (`tcl::mathfunc` resolution), so both fail closed; `Raw` preserves
/// unparseable text and must be treated as opaque.
fn expr_world_hazard(state: &WorldContents, expr: &tcl_syntax::expr::ExprNode) -> bool {
    use tcl_syntax::expr::ExprNode;
    match expr {
        ExprNode::Literal { .. } | ExprNode::CompiledWord { .. } => false,
        // A `"…"` operand substitutes its `$var` and `[cmd]`; a `{…}` one
        // is inert.
        ExprNode::String { text, .. } => {
            tcl_syntax::expr::quoted_string_body(text).is_some_and(|body| body.contains(['$', '[']))
        }
        ExprNode::Var { text, .. } => state.variable_access_may_observe(text),
        ExprNode::Command { .. } | ExprNode::Call { .. } | ExprNode::Raw { .. } => true,
        ExprNode::Unary { operand, .. } => expr_world_hazard(state, operand),
        ExprNode::Binary { left, right, .. } => {
            expr_world_hazard(state, left) || expr_world_hazard(state, right)
        }
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            expr_world_hazard(state, condition)
                || expr_world_hazard(state, true_branch)
                || expr_world_hazard(state, false_branch)
        }
    }
}

/// The typed dispatch-stability proof captured at one invocation site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SiteDispatchProof {
    /// Dependency domains proven stable for this site's resolved command.
    pub covers: DispatchDependencies,
    /// Per-track value-numbering tokens at the site.
    pub versions: [TrackVersion; WorldTrack::ALL.len()],
    /// Whether every operand word of the invocation is provably free of
    /// world observation (no command substitution, no expansion, no variable
    /// read that a live variable trace may observe).
    pub operand_words_admissible: bool,
}

impl SiteDispatchProof {
    /// Whether the proof covers every required dependency domain and the
    /// site's operand words are admissible.
    #[must_use]
    pub fn satisfies(&self, required: DispatchDependencies) -> bool {
        self.operand_words_admissible && required.iter().all(|domain| self.covers.contains(domain))
    }

    /// Render the value-key fragments for the given dependency domains plus
    /// declared versioned-world result domains, deduplicated in track order.
    #[must_use]
    pub fn world_key(
        &self,
        required: DispatchDependencies,
        result_domains: &[WorldStateDomain],
    ) -> Vec<String> {
        let mut selected = [false; WorldTrack::ALL.len()];
        for domain in required.iter() {
            for track in WorldTrack::for_dispatch_dependency(domain) {
                selected[track.index()] = true;
            }
        }
        for domain in result_domains {
            selected[WorldTrack::from_world_state_domain(*domain).index()] = true;
        }
        WorldTrack::ALL
            .into_iter()
            .filter(|track| selected[track.index()])
            .map(|track| {
                let mut fragment = String::from("world:");
                fragment.push_str(track.label());
                fragment.push('@');
                self.versions[track.index()].render(&mut fragment);
                fragment
            })
            .collect()
    }
}

/// Per-function dispatch-stability proofs, keyed by executable coordinates.
#[derive(Debug, Clone, Default)]
pub struct DispatchProofAnalysis {
    sites: FxHashMap<(u32, u32), SiteDispatchProof>,
}

impl DispatchProofAnalysis {
    /// The proof captured for the invocation at the given block and
    /// instruction indices, when the analysis reached it.
    #[must_use]
    pub fn site(&self, block: u32, instruction: u32) -> Option<&SiteDispatchProof> {
        self.sites.get(&(block, instruction))
    }
}

/// Precomputed CFG shape shared by the fixpoint and the capture replay.
struct CfgShape {
    successors: Vec<Vec<usize>>,
    completions: Vec<Vec<EdgeCompletion>>,
    /// `(predecessor, edge index within the predecessor)` per block.
    predecessors: Vec<Vec<(usize, usize)>>,
    reachable: Vec<bool>,
}

impl CfgShape {
    fn of(function: &ExecutableFunction) -> Self {
        let block_count = function.blocks.len();
        let mut successors: Vec<Vec<usize>> = vec![Vec::new(); block_count];
        let mut completions: Vec<Vec<EdgeCompletion>> = vec![Vec::new(); block_count];
        let mut predecessors: Vec<Vec<(usize, usize)>> = vec![Vec::new(); block_count];
        for (index, block) in function.blocks.iter().enumerate() {
            let Some(terminator) = &block.terminator else {
                continue;
            };
            let mut targets = successors_of(terminator);
            targets.sort_unstable();
            targets.dedup();
            completions[index] = targets
                .iter()
                .map(|target| edge_completion(terminator, *target))
                .collect();
            for (edge, target) in targets.iter().enumerate() {
                if *target < block_count {
                    predecessors[*target].push((index, edge));
                }
            }
            successors[index] = targets;
        }
        let mut reachable = vec![false; block_count];
        let mut pending = vec![function.entry.index()];
        while let Some(block) = pending.pop() {
            if block >= block_count || reachable[block] {
                continue;
            }
            reachable[block] = true;
            pending.extend(successors[block].iter().copied());
        }
        Self {
            successors,
            completions,
            predecessors,
            reachable,
        }
    }
}

/// Analyse dispatch stability for one executable function under an explicit
/// entry contract.
///
/// The result is deterministic for an isomorphic CFG.  The worklist fixpoint
/// is bounded; a pathological CFG that fails to stabilise within the bound
/// fails closed by reporting no site proofs at all.
#[must_use]
pub fn analyse_dispatch_stability(
    function: &ExecutableFunction,
    entry: DispatchEntryAssumption,
) -> DispatchProofAnalysis {
    analyse_dispatch_stability_using(function, entry, None, &|_, facts| {
        Some(normalise_name(&facts.canonical_command).into_owned())
    })
}

/// Original source consumer of the same authored contents fixpoint. A Native
/// entry or authored simulation cannot borrow a pristine spelling inventory.
/// Even a selected Native trace-absence receipt would not cover the remaining
/// command/namespace/policy domains tracked here.
pub(crate) fn analyse_original_dispatch_stability(
    function: &ExecutableFunction,
    entry: DispatchEntryAssumption,
    registry: &tcl_registry::CommandRegistry,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
) -> DispatchProofAnalysis {
    analyse_source_dispatch_stability(function, entry, registry, metadata, false)
}

/// The same fixpoint for value reuse over a supplied source. A source runs in
/// a fresh interpreter of its release, whose builtins carry no traces, aliases
/// or imports (tclsh 8.4-9.1), so its authored simulation entry also supplies
/// ledger subjects. Execution entries keep their own Native owner.
pub(crate) fn analyse_reuse_dispatch_stability(
    function: &ExecutableFunction,
    entry: DispatchEntryAssumption,
    registry: &tcl_registry::CommandRegistry,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
) -> DispatchProofAnalysis {
    analyse_source_dispatch_stability(function, entry, registry, metadata, true)
}

fn analyse_source_dispatch_stability(
    function: &ExecutableFunction,
    entry: DispatchEntryAssumption,
    registry: &tcl_registry::CommandRegistry,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    authored_simulation: bool,
) -> DispatchProofAnalysis {
    let grammar = metadata
        .filter(|metadata| metadata.matches_registry(registry))
        .and_then(crate::registry_invocation::InvocationMetadataContext::source_analysis_input)
        .map(crate::analyser::ResolvedAnalysisInput::lexer_config);
    analyse_dispatch_stability_using(function, entry, grammar, &|invoke, facts| {
        source_ledger_subject(invoke, facts, registry, metadata, authored_simulation)
    })
}

#[cfg(test)]
fn original_ledger_subject(
    invoke: &crate::executable_ir::GenericInvoke,
    facts: &tcl_registry::InvocationFacts,
    registry: &tcl_registry::CommandRegistry,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
) -> Option<String> {
    source_ledger_subject(invoke, facts, registry, metadata, false)
}

/// The authentic original call slot supplies this ledger key; a selected
/// descriptor label is checked as data and cannot recreate that slot.
fn source_ledger_subject(
    invoke: &crate::executable_ir::GenericInvoke,
    facts: &tcl_registry::InvocationFacts,
    registry: &tcl_registry::CommandRegistry,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    authored_simulation: bool,
) -> Option<String> {
    let metadata = metadata.filter(|metadata| metadata.matches_registry(registry))?;
    let binding = invoke.source_binding.as_ref()?;
    let (_, mut tokens) = binding.original_recorded_command()?;
    if tokens.words() != invoke.original_words {
        return None;
    }
    tokens.source_binding = Some(binding.clone());
    let purpose = binding.original_invocation_source_purpose(&tokens, metadata, registry)?;
    let admitted = match purpose {
        crate::command_binding::OriginalInvocationSourcePurpose::Logical
        | crate::command_binding::OriginalInvocationSourcePurpose::Standalone => true,
        crate::command_binding::OriginalInvocationSourcePurpose::AuthoredSimulation => {
            authored_simulation
        }
        crate::command_binding::OriginalInvocationSourcePurpose::Execution => false,
    };
    if !admitted {
        return None;
    }
    let selected = crate::registry_invocation::resolved_handler_invocation_with_metadata_context(
        registry,
        Some(metadata),
        &tokens,
    )?;
    if selected.facts.as_ref() != facts {
        return None;
    }
    let reference = binding.evaluated_command_reference()?;
    let crate::command_binding::SourceCommandReferenceBinding::Direct {
        kind: crate::command_binding::BindingKind::Builtin,
        identity,
    } = reference.binding()
    else {
        return None;
    };
    let slot = reference.slot()?;
    if slot != identity.origin
        || !reference.matches_original_invocation_site(binding.invocation_site()?)
    {
        return None;
    }
    tcl_syntax::naming::unroot_rooted_key(slot).map(str::to_owned)
}

fn analyse_dispatch_stability_using(
    function: &ExecutableFunction,
    entry: DispatchEntryAssumption,
    grammar: Option<tcl_lexer::LexerConfig>,
    subject: &dyn Fn(
        &crate::executable_ir::GenericInvoke,
        &tcl_registry::InvocationFacts,
    ) -> Option<String>,
) -> DispatchProofAnalysis {
    let block_count = function.blocks.len();
    if block_count == 0 {
        return DispatchProofAnalysis::default();
    }
    // Widening is absorbing: no transfer ever restores a widened ledger or a
    // cleared stability flag. An unknown-world entry therefore starts at the
    // lattice top and stays there, so every site proof it could produce would
    // fail closed. Walking the CFG to discover that is measurable work on the
    // procedure, method, and body units that carry this contract, and they
    // are the majority of units in ordinary Tcl.
    if entry == DispatchEntryAssumption::UnknownWorld {
        return DispatchProofAnalysis::default();
    }
    let shape = CfgShape::of(function);
    let entry_index = function.entry.index();
    let mut initial = WorldContents::entry(entry);
    initial.variable_grammar = grammar;
    let entry_state = Arc::new(initial);

    let mut edge_out: Vec<Vec<Option<Arc<WorldContents>>>> = shape
        .successors
        .iter()
        .map(|targets| vec![None; targets.len()])
        .collect();
    let mut queued = vec![false; block_count];
    let mut worklist: VecDeque<usize> = VecDeque::new();
    if entry_index < block_count {
        worklist.push_back(entry_index);
        queued[entry_index] = true;
    }
    let mut budget = block_count.saturating_mul(MAX_BLOCK_APPLICATIONS_PER_BLOCK);
    while let Some(block) = worklist.pop_front() {
        queued[block] = false;
        if budget == 0 {
            return DispatchProofAnalysis::default();
        }
        budget -= 1;
        let Some(state) = join_block_entry(block, entry_index, &entry_state, &shape, &edge_out)
        else {
            continue;
        };
        let outgoing = apply_block(function, block, state, &shape);
        let mut changed = false;
        for (edge, next) in outgoing.into_iter().enumerate() {
            let same = match (&edge_out[block][edge], &next) {
                (Some(old), Some(new)) => Arc::ptr_eq(old, new) || old == new,
                (None, None) => true,
                _ => false,
            };
            if !same {
                edge_out[block][edge] = next;
                changed = true;
            }
        }
        if changed {
            for successor in &shape.successors[block] {
                if *successor < block_count && shape.reachable[*successor] && !queued[*successor] {
                    queued[*successor] = true;
                    worklist.push_back(*successor);
                }
            }
        }
    }

    let mut analysis = DispatchProofAnalysis::default();
    for block in 0..block_count {
        if !shape.reachable[block] {
            continue;
        }
        let Some(state) = join_block_entry(block, entry_index, &entry_state, &shape, &edge_out)
        else {
            continue;
        };
        capture_block_sites(function, block, &state, &mut analysis, subject);
    }
    analysis
}

/// Join predecessor edge states into a block-entry state.  `None` means no
/// predecessor state has been computed yet (a cycle edge before its first
/// visit): the caller skips the block this round rather than fabricating
/// contents; at fixpoint every reachable predecessor edge contributes.
fn join_block_entry(
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
    facts: &'f [StateTransitionFact],
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
fn apply_block(
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
fn capture_block_sites(
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

fn apply_transition_fact(
    state: &mut WorldContents,
    transition: &StateTransition,
    site: TrackVersion,
) {
    match transition {
        StateTransition::CommandBinding(transition) => {
            apply_command_binding(state, transition, site);
        }
        StateTransition::Interpreter(transition) => {
            apply_interpreter_transition(state, transition, site);
        }
        StateTransition::VariableCellAlias(alias) => {
            apply_variable_alias(state, alias, site);
        }
        StateTransition::Namespace(transition) => {
            apply_namespace_transition(state, transition, site);
        }
        StateTransition::Trace(transition) => apply_trace_transition(state, transition, site),
        StateTransition::ObjectDispatch(transition) => {
            apply_object_transition(state, transition, site);
        }
        StateTransition::Package(transition) => {
            use tcl_registry::model::binding::PackageTransition;
            let writes = match transition {
                PackageTransition::Provide { version, .. } => version.is_some(),
                PackageTransition::Ifneeded {
                    script_provided, ..
                } => *script_provided,
                PackageTransition::UnknownHandler { handler } => handler.is_some(),
                PackageTransition::Prefer { mode } => mode.is_some(),
                PackageTransition::Require { .. }
                | PackageTransition::Forget { .. }
                | PackageTransition::SourceLoad { .. }
                | PackageTransition::IndexedAutoload { .. }
                | PackageTransition::DiscoveryDependencyChanged { .. } => true,
            };
            if writes {
                state.bump(WorldTrack::PackageState, site);
            }
        }
        StateTransition::Widen(widening) => {
            for domain in &widening.domains {
                apply_domain_widening(state, *domain, site);
            }
        }
    }
}

fn apply_command_binding(
    state: &mut WorldContents,
    transition: &CommandBindingTransition,
    site: TrackVersion,
) {
    match transition {
        CommandBindingTransition::Define { name, .. } => state.bindings.record_subject(name),
        CommandBindingTransition::Move { from, to } => {
            state.bindings.record_subject(from);
            state.bindings.record_subject(to);
        }
        CommandBindingTransition::Delete { interpreter, name } => match interpreter {
            None => state.bindings.record_subject(name),
            Some(interpreter) if interpreter_may_be_current(interpreter) => {
                state.bindings.record_subject(name);
            }
            // A deletion inside a named child interpreter cannot unbind a
            // current-interpreter command.
            Some(_) => return,
        },
        CommandBindingTransition::Alias {
            source_interpreter,
            alias,
            ..
        } => {
            if !interpreter_may_be_current(source_interpreter) {
                return;
            }
            state.bindings.record_subject(alias);
        }
        CommandBindingTransition::Unknown { .. } => state.bindings.widen(),
    }
    state.bump(WorldTrack::CommandBindings, site);
}

fn apply_interpreter_transition(
    state: &mut WorldContents,
    transition: &InterpreterTransition,
    site: TrackVersion,
) {
    match transition {
        InterpreterTransition::Create { interpreter, .. } => {
            state.bump(WorldTrack::InterpreterTopology, site);
            // The child-path command appears in the current interpreter; a
            // generated child name is fresh and cannot collide with an
            // existing resolution.
            if let Some(subject) = interpreter {
                state.bindings.record_subject(subject);
                state.bump(WorldTrack::CommandBindings, site);
            }
        }
        InterpreterTransition::Delete { .. } => {
            // Any surviving interpreter can retain an alias whose target was
            // the deleted child; reverse alias edges are not tracked.
            state.bindings.widen();
            state.bump(WorldTrack::CommandBindings, site);
            state.bump(WorldTrack::InterpreterTopology, site);
        }
        InterpreterTransition::MarkTrusted { interpreter }
        | InterpreterTransition::SetRecursionLimit { interpreter, .. }
        | InterpreterTransition::SetBackgroundError { interpreter, .. } => {
            if interpreter_may_be_current(interpreter) {
                state.interpreter_policy_stable = false;
            }
            state.bump(WorldTrack::InterpreterPolicy, site);
        }
        InterpreterTransition::Hide {
            interpreter,
            visible,
            ..
        }
        | InterpreterTransition::Expose {
            interpreter,
            visible,
            ..
        } => {
            if interpreter_may_be_current(interpreter) {
                state.interpreter_policy_stable = false;
                state.bindings.record_subject(visible);
                state.bump(WorldTrack::CommandBindings, site);
            }
            state.bump(WorldTrack::InterpreterPolicy, site);
        }
    }
}

fn apply_variable_alias(
    state: &mut WorldContents,
    alias: &tcl_registry::VariableCellAliasTransition,
    site: TrackVersion,
) {
    let target = match &alias.target {
        VariableAliasTarget::Global { variable }
        | VariableAliasTarget::CurrentNamespace { variable } => variable
            .literal()
            .map(|name| normalise_name(name).into_owned()),
        VariableAliasTarget::CallerSelectedFrame { .. } => None,
        VariableAliasTarget::Namespace {
            namespace,
            variable,
        } => match (namespace.literal(), variable.literal()) {
            (Some(namespace), Some(variable)) => Some(namespace_alias_target(namespace, variable)),
            _ => None,
        },
    };
    match alias.local.literal() {
        Some(local) => state
            .aliases
            .record(normalise_name(local).into_owned(), target),
        None => state.aliases.widen(),
    }
    state.bump(WorldTrack::VariableStore, site);
}

fn apply_namespace_transition(
    state: &mut WorldContents,
    transition: &NamespaceTransition,
    site: TrackVersion,
) {
    let prefix = |target: &NamespaceTransitionTarget| -> Option<SubjectPattern> {
        match target {
            // Analysed units start in the global namespace, so the current
            // namespace is the global prefix.
            NamespaceTransitionTarget::Current => {
                Some(SubjectPattern::NamespacePrefix(String::new()))
            }
            NamespaceTransitionTarget::Named(subject)
                if subject
                    .literal()
                    .is_some_and(tcl_syntax::naming::is_absolute) =>
            {
                Some(SubjectPattern::NamespacePrefix(
                    normalise_name(subject.literal().expect("qualified namespace value"))
                        .into_owned(),
                ))
            }
            NamespaceTransitionTarget::Named(_) => None,
        }
    };
    match transition {
        NamespaceTransition::Ensure { .. } | NamespaceTransition::Export { .. } => {
            // Creating an empty namespace binds no command; export lists
            // gate future imports.  Neither changes an existing resolution.
            state.bump(WorldTrack::NamespaceLookup, site);
        }
        NamespaceTransition::Delete { namespace } => {
            state.namespace_lookup_stable = false;
            state.namespace_unknown_stable = false;
            if let Some(pattern) = prefix(namespace) {
                state.bindings.record(pattern.clone());
                state.object_dispatch.record(pattern);
            } else {
                state.bindings.widen();
                state.object_dispatch.widen();
            }
            state.bump(WorldTrack::NamespaceLookup, site);
            state.bump(WorldTrack::NamespaceUnknown, site);
            state.bump(WorldTrack::CommandBindings, site);
            state.bump(WorldTrack::ObjectDispatch, site);
            state.bump(WorldTrack::VariableStore, site);
        }
        NamespaceTransition::Import { namespace, .. }
        | NamespaceTransition::Forget { namespace, .. }
        | NamespaceTransition::Ensemble { namespace, .. } => {
            state.namespace_lookup_stable = false;
            match prefix(namespace) {
                Some(pattern) => state.bindings.record(pattern),
                None => state.bindings.widen(),
            }
            state.bump(WorldTrack::NamespaceLookup, site);
            state.bump(WorldTrack::CommandBindings, site);
        }
        NamespaceTransition::SetPath { .. } => {
            state.namespace_lookup_stable = false;
            state.bump(WorldTrack::NamespaceLookup, site);
        }
        NamespaceTransition::SetUnknown { .. } => {
            state.namespace_unknown_stable = false;
            state.bump(WorldTrack::NamespaceUnknown, site);
        }
    }
}

fn apply_trace_transition(
    state: &mut WorldContents,
    transition: &TraceTransition,
    site: TrackVersion,
) {
    let (adding, target, operations, prefix) = match transition {
        TraceTransition::Add {
            target,
            operations,
            prefix,
        } => (true, target, operations, prefix),
        TraceTransition::Remove {
            target,
            operations,
            prefix,
        } => (false, target, operations, prefix),
    };
    let (track, subject) = match target {
        TraceTarget::Variable(subject) => (WorldTrack::VariableTraces, subject),
        TraceTarget::Command(subject) => (WorldTrack::CommandTraces, subject),
        TraceTarget::Execution(subject) => (WorldTrack::ExecutionTraces, subject),
    };
    if matches!(target, TraceTarget::Variable(_)) {
        // Variable trace registration vivifies its cell; removal of the last
        // trace releases the placeholder.
        state.bump(WorldTrack::VariableStore, site);
    }
    state.bump(track, site);
    let ledger = match track {
        WorldTrack::VariableTraces => &mut state.variable_traces,
        WorldTrack::CommandTraces => &mut state.command_traces,
        _ => &mut state.execution_traces,
    };
    let Some(name) = subject.literal() else {
        if adding {
            ledger.widen();
        } else {
            // A dynamic removal target removes at most one registration
            // somewhere: presence upper bounds are unaffected.
            for (_, cell) in &mut ledger.cells {
                cell.remove_any();
            }
        }
        return;
    };
    let name = normalise_name(name);
    let known_operations = match operations {
        TraceOperationSet::Known(operations) => Some(operations.clone()),
        TraceOperationSet::Unknown(_) => None,
    };
    let literal_prefix = prefix.literal().map(str::to_owned);
    let Some(cell) = ledger.cell_mut(&name) else {
        return;
    };
    match (adding, known_operations) {
        (true, Some(operations)) => {
            cell.add(TraceKey {
                operations,
                prefix: literal_prefix,
            });
        }
        (true, None) => cell.unknown = true,
        (false, Some(operations)) => {
            cell.remove(&TraceKey {
                operations,
                prefix: literal_prefix,
            });
        }
        (false, None) => cell.remove_any(),
    }
}

fn apply_object_transition(
    state: &mut WorldContents,
    transition: &ObjectDispatchTransition,
    site: TrackVersion,
) {
    match transition {
        ObjectDispatchTransition::Configure { target, .. } => {
            state.object_dispatch.record_subject(target);
        }
        ObjectDispatchTransition::Destroy { target } => {
            state.object_dispatch.record_subject(target);
            // Destroying an object also deletes its public command.
            state.bindings.record_subject(target);
            state.bump(WorldTrack::CommandBindings, site);
        }
        ObjectDispatchTransition::Create { target, .. }
        | ObjectDispatchTransition::Copy { target, .. } => match target {
            ObjectDispatchTarget::Named(subject) => state.object_dispatch.record_subject(subject),
            // Generated object names are fresh and cannot collide with an
            // existing resolution.
            ObjectDispatchTarget::Fresh => {}
        },
    }
    state.bump(WorldTrack::ObjectDispatch, site);
}

fn apply_domain_widening(
    state: &mut WorldContents,
    domain: StateTransitionDomain,
    site: TrackVersion,
) {
    match domain {
        StateTransitionDomain::CommandBindings => {
            state.bindings.widen();
            state.bump(WorldTrack::CommandBindings, site);
        }
        StateTransitionDomain::VariableCells => {
            if state.variable_traces.completely_absent() && !state.aliases.unknown {
                state.aliases.widen();
                state.bump(WorldTrack::VariableStore, site);
            } else {
                // A write to an unknown cell may fire arbitrary handlers.
                state.widen_all(site);
            }
        }
        StateTransitionDomain::Namespaces | StateTransitionDomain::CommandResolution => {
            state.namespace_lookup_stable = false;
            state.namespace_unknown_stable = false;
            state.bump(WorldTrack::NamespaceLookup, site);
            state.bump(WorldTrack::NamespaceUnknown, site);
        }
        StateTransitionDomain::InterpreterTopology => {
            state.bump(WorldTrack::InterpreterTopology, site);
        }
        StateTransitionDomain::InterpreterPolicy => {
            state.interpreter_policy_stable = false;
            state.bump(WorldTrack::InterpreterPolicy, site);
        }
        StateTransitionDomain::Interpreters => state.widen_all(site),
        StateTransitionDomain::CommandTraces => {
            state.command_traces.widen();
            state.bump(WorldTrack::CommandTraces, site);
        }
        StateTransitionDomain::ExecutionTraces => {
            state.execution_traces.widen();
            state.bump(WorldTrack::ExecutionTraces, site);
        }
        StateTransitionDomain::VariableTraces => {
            state.variable_traces.widen();
            state.bump(WorldTrack::VariableTraces, site);
        }
        StateTransitionDomain::ObjectDispatch => {
            state.object_dispatch.widen();
            state.bump(WorldTrack::ObjectDispatch, site);
        }
        StateTransitionDomain::Packages => state.bump(WorldTrack::PackageState, site),
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod original_ledger_tests;
