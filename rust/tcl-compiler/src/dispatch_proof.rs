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

use tcl_registry::{DispatchDependencies, TraceOperation, TransitionSubject, WorldStateDomain};

use crate::executable_ir::ExecutableFunction;
#[cfg(test)]
use crate::executable_ir::{ExecutableInstruction, ExecutableTerminator, InvocationResolution};
use crate::state_ssa::adapters::WorldInterpreterScope;
use crate::world_state_ssa::{EdgeCompletion, edge_completion, successors_of};
use tcl_source::words::WordExpr;
use tcl_source::words::WordPart;

mod block_transfer;
mod state_transitions;
mod world_tracks;

use block_transfer::{apply_block, capture_block_sites, join_block_entry};
pub use world_tracks::{TrackVersion, WorldTrack};

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
            let Some((array, key)) = tcl_syntax::naming::split_element_ref(known) else {
                return false;
            };
            array == base
                && match element {
                    ElementAccess::Scalar => false,
                    ElementAccess::Known(element) => key == *element,
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
        kind: tcl_names::command::BindingKind::Builtin,
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

#[cfg(test)]
mod tests;

#[cfg(test)]
mod original_ledger_tests;
