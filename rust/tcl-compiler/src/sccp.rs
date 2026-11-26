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

//! Sparse Conditional Constant Propagation (SCCP).
//!
//! Classic SCCP lattice-based constant propagation: iteratively
//! refine per-SSA-value [`LatticeValue`] facts until a fixed point,
//! using CFG reachability so unreachable branches never drag their
//! targets down to `Overdefined`.

use std::collections::{BTreeSet, HashMap, HashSet};

use tcl_registry::CommandRegistry;
use tcl_registry::value_transfer::Existence;
#[cfg(test)]
use tcl_registry::value_transfer::{
    BindingKind, DependencyEvidence, DomainFact, ExactValue, FactDomain, FactView,
};

use crate::analyses::{ConstValue, LatticeValue, MAX_CONSTSET_SIZE};
use crate::cfg::{BlockId, Function as CfgFunction, Terminator};
use crate::expr_ast::ExprNode;
use crate::ssa::{SsaBlock, SsaFunction, SsaSourceView, Symbol, ValueKey};
use crate::tcl_expr_eval::FoldPolicy;
use crate::value_transfer::{AnalysisContextKey, LatticeDriver};

mod original_value_facts;
pub(crate) use original_value_facts::DiagnosticValueFacts;
pub use original_value_facts::{
    ExistenceFrame, ExistenceMetadataInputs, ExpressionAnalysis, ExpressionEvaluationPoint,
    SemanticValueFacts, SemanticValueProjection, ValueFactInputs, evaluate_branch,
    execution_value_facts, existence_constant_branches,
    existence_constant_branches_with_metadata_context, existence_constant_branches_with_ssa,
    existence_constant_branches_with_ssa_with_metadata_context, semantic_value_facts,
};
#[cfg(test)]
use original_value_facts::{evaluate_fixture_def, evaluate_fixture_def_with_folds};

mod block_sweep;
mod evaluation;
mod solver;
mod statement_transfer;

#[cfg(test)]
use block_sweep::sccp_process_phis;
#[cfg(test)]
use evaluation::resolve_simple_var_ref;
pub(crate) use evaluation::{DefValues, RaisedDefs, with_whole_variable_operands};
pub use evaluation::{
    extract_foreach_elements, parse_literal_value, resolve_foreach_list_via_lattice,
};
#[cfg(test)]
use solver::{Round, Solved, drive, typed_element_write_base};
pub(crate) use solver::{SolveInputs, place_base, prior_version};
#[cfg(test)]
use solver::{StatementInputs, branch_deferrable, sccp_process_statements};
use solver::{callback_writes_any, drive_escaping};

// Public aliases

/// Predecessor map: block → set of blocks that branch into it. Thin
/// wrapper around [`CfgFunction::predecessors`] kept in this module so
/// callers can reach it without reaching into the CFG type directly.
#[must_use]
pub fn compute_predecessors(cfg: &CfgFunction) -> HashMap<BlockId, HashSet<BlockId>> {
    cfg.predecessors()
}

/// CFG traversal order used by SCCP — reverse post-order from the
/// entry block. Blocks that the RPO walk cannot reach from `entry`
/// are appended at the end so the driver can still observe them.
#[must_use]
pub fn cfg_order(cfg: &CfgFunction) -> Vec<BlockId> {
    let mut order = cfg.reverse_postorder();
    let seen: HashSet<BlockId> = order.iter().copied().collect();
    for id in cfg.blocks.keys() {
        if !seen.contains(id) {
            order.push(*id);
        }
    }
    order
}

enum BranchSelection {
    Taken(bool),
    Both,
    Deferred,
}

impl BranchSelection {
    /// An undecided condition may gain a value during the optimistic sweep.
    /// The finalising sweep opens both arms of every remaining undecided branch.
    fn of(decision: Option<bool>, finalizing: bool, pending: impl FnOnce() -> bool) -> Self {
        match decision {
            Some(taken) => Self::Taken(taken),
            None if !finalizing && pending() => Self::Deferred,
            None => Self::Both,
        }
    }
}

/// Test whether a local operand is still pending and none is overdefined.
/// For a positioned read, a missing cell or version cannot justify waiting.
fn branch_has_pending_operands(
    block: &SsaBlock,
    condition: &ExprNode,
    values: &HashMap<ValueKey, LatticeValue>,
    source: SsaSourceView<'_>,
    grammar: tcl_dialect::LexerGrammar,
) -> bool {
    let mut pending = false;
    for name in crate::var_refs::vars_in_expr(condition, grammar) {
        let (symbol, version) = if source.is_positioned() {
            let Some(read) = source.read_spelling(&name) else {
                return false;
            };
            let Some(version) = read.version else {
                return false;
            };
            (read.symbol, version)
        } else {
            let Some(symbol) = source.symbol(&name) else {
                continue;
            };
            (
                symbol,
                block.exit_versions.get(&symbol).copied().unwrap_or(0),
            )
        };
        // Version-zero live-ins were settled when the solver entered the function.
        if version == 0 {
            continue;
        }
        match values.get(&(symbol, version)) {
            Some(LatticeValue::Overdefined) => return false,
            Some(LatticeValue::Unknown) | None => pending = true,
            _ => {}
        }
    }
    pending
}

/// Open the normal successors and the purpose's admitted additional edges.
/// The evaluator supplies the branch choice, or declines the block when its
/// branch has no matching SSA block.
fn transfer_terminator<'a>(
    cfg: &CfgFunction,
    from: BlockId,
    (blocks, edges): (&mut HashSet<BlockId>, &mut HashSet<(BlockId, BlockId)>),
    completes: bool,
    additional: impl Iterator<Item = &'a (BlockId, BlockId)>,
    branch: impl FnOnce(&ExprNode, Option<tcl_lexer::Span>, Option<u32>) -> Option<BranchSelection>,
) -> bool {
    let Some(terminator) = cfg
        .blocks
        .get(&from)
        .and_then(|block| block.terminator.as_ref())
    else {
        return false;
    };
    let mut open = |target| {
        let new_edge = edges.insert((from, target));
        let new_block = cfg.blocks.contains_key(&target) && blocks.insert(target);
        new_edge || new_block
    };
    let mut changed = false;
    if completes {
        match terminator {
            Terminator::Goto { target, .. } => changed |= open(*target),
            Terminator::Branch {
                condition,
                true_target,
                false_target,
                span,
                condition_base,
                ..
            } => match branch(condition, *span, *condition_base) {
                Some(BranchSelection::Taken(taken)) => {
                    changed |= open(if taken { *true_target } else { *false_target });
                }
                Some(BranchSelection::Both) => {
                    changed |= open(*true_target);
                    changed |= open(*false_target);
                }
                Some(BranchSelection::Deferred) => {}
                None => return false,
            },
            Terminator::Return { .. } | Terminator::Complete { .. } => {}
        }
    }
    for &(source, target) in additional {
        if source == from {
            changed |= open(target);
        }
    }
    changed
}

// Lattice join

/// Canonical [`ConstValue`] ordering for deterministic set merges.
///
/// Use the double's bits to agree with lattice equality.
fn cv_key(v: &ConstValue) -> (u8, String) {
    match v {
        ConstValue::Int(i) => (0, i.to_string()),
        ConstValue::Float(f) => (1, format!("{:016x}", f.to_bits())),
        ConstValue::Bool(b) => (2, b.to_string()),
        ConstValue::String(s) => (3, s.as_str().to_owned()),
    }
}

/// Collect the set of possible [`ConstValue`]s represented by `lv`.
/// Returns `None` for `Unknown` / `Overdefined`.
fn to_set(lv: &LatticeValue) -> Option<Vec<ConstValue>> {
    match lv {
        LatticeValue::Const(v) => Some(vec![v.clone()]),
        LatticeValue::ConstSet(vs) => Some(vs.clone()),
        _ => None,
    }
}

/// Join two lattice values, widening to `Overdefined` when either
/// side is `Overdefined` or when the union exceeds
/// [`MAX_CONSTSET_SIZE`]. Behaviour:
///
/// - `Unknown` is absorbed (takes the non-unknown side).
/// - `Overdefined` is absorbing (either side forces the result).
/// - Otherwise the two value sets are unioned. A union whose size
///   drops to 1 collapses to a [`LatticeValue::Const`]; larger
///   unions yield [`LatticeValue::ConstSet`]; widening past the
///   cap yields [`LatticeValue::Overdefined`].
/// - When the merged set is identical to `old`'s set the result
///   is `old` (pointer-equality isn't tracked, but the caller's
///   change-detection code can rely on value-equality).
#[must_use]
pub fn join(old: &LatticeValue, new: &LatticeValue) -> LatticeValue {
    if matches!(new, LatticeValue::Unknown) {
        return old.clone();
    }
    if matches!(old, LatticeValue::Unknown) {
        return new.clone();
    }
    if matches!(old, LatticeValue::Overdefined) || matches!(new, LatticeValue::Overdefined) {
        return LatticeValue::Overdefined;
    }
    let old_set = to_set(old).unwrap_or_default();
    let new_set = to_set(new).unwrap_or_default();
    let mut merged: Vec<ConstValue> = old_set.clone();
    for v in &new_set {
        if !merged.contains(v) {
            merged.push(v.clone());
        }
    }
    if merged.is_empty() {
        return LatticeValue::Overdefined;
    }
    // Sort for deterministic equality checks.
    merged.sort_by_key(cv_key);
    let mut old_sorted = old_set;
    old_sorted.sort_by_key(cv_key);
    if merged == old_sorted {
        return old.clone();
    }
    if merged.len() == 1 {
        return LatticeValue::Const(merged.remove(0));
    }
    if merged.len() > MAX_CONSTSET_SIZE {
        return LatticeValue::Overdefined;
    }
    LatticeValue::ConstSet(merged)
}

/// Update `values[key]` by joining the existing entry with
/// `candidate`. Returns `true` when the stored value changed
/// (signalling that the SCCP worklist should be repopulated).
pub fn set_value<S: std::hash::BuildHasher>(
    values: &mut HashMap<ValueKey, LatticeValue, S>,
    key: ValueKey,
    candidate: &LatticeValue,
) -> bool {
    let old = values.get(&key).cloned().unwrap_or(LatticeValue::Unknown);
    let merged = join(&old, candidate);
    if merged == old {
        return false;
    }
    values.insert(key, merged);
    true
}

// Driver

/// Which of the three branch facts a [`ConstantBranch`] is
/// (`docs/design/compiler/value-transfers.md` § *Branch facts*): a proven
/// condition, a selected arm with no CFG edge of its own, or applied
/// reachability. A consumer reads the kind stored with the fact and never
/// reruns the proof to learn it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BranchFactKind {
    /// The condition is proven, and reachability was not updated from it.
    /// The solver decides an existence query inside the fixed point, as an
    /// `Applied` fact, so no producer states this kind.
    Proven,
    /// A statement selects among arms the CFG has no edge of its own for (an
    /// opaque `switch`): the fact names an arm no member of the subject
    /// reaches, and applies no reachability — no block is dropped for it.
    Selected,
    /// The solver decided the branch and applied it to the executable
    /// blocks and edges.
    Applied,
}

impl BranchFactKind {
    /// The kind's name, as the Explorer states it.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Proven => "proven",
            Self::Selected => "selected",
            Self::Applied => "applied",
        }
    }
}

/// A branch whose condition SCCP determined to be constant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstantBranch {
    /// CFG block containing the branch — for a [`BranchFactKind::Selected`]
    /// fact, the block holding the statement.
    pub block: String,
    /// Source span of the branch terminator (its condition
    /// expression when known) — a `Selected` fact's is its arm's pattern.
    /// Used by diagnostic aggregators to point editors and CLIs at the
    /// triggering site.
    pub span: Option<tcl_lexer::Span>,
    /// Condition text for diagnostic reporting; a `Selected` fact's is the
    /// arm's pattern.
    pub condition: String,
    /// Evaluated boolean value; a `Selected` fact is always `false` — the
    /// arm is not selected.
    pub value: bool,
    /// Target reached when the condition holds; empty for a `Selected`
    /// fact, since no block stands for an arm.
    pub taken_target: String,
    /// Target skipped; empty for a `Selected` fact.
    pub not_taken_target: String,
    /// Which branch fact this is.
    pub kind: BranchFactKind,
}

/// The template-word plan one executable `subst` call declares
/// (`docs/design/compiler/value-transfers.md` § *The template-word plan*),
/// read over the settled lattice, so a switch's proven value reads as its
/// spelling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplatePlanRecord {
    /// The template word's token span in the document — for a braced word
    /// from its `{` to its content's end; the plan's own spans are offsets
    /// from its start (a braced template's content from 1).
    pub span: tcl_lexer::Span,
    /// The command as the call spells it.
    pub command: String,
    /// The spelling each switch reads as, when the lattice proves every one
    /// exactly — what W102's narrowing advice reads.
    pub switches: Option<Vec<String>>,
    /// The plan.
    pub plan: tcl_registry::value_transfer::TemplateWordPlan,
}

/// The selection one executable opaque case-list statement makes
/// (`docs/design/compiler/value-transfers.md` § *`switch`*, step 2): the arm
/// each member of its proven subject selects, recorded against the arms'
/// pattern spans. Never applied reachability — the statement's arms have no
/// blocks of their own — and never a guess: the fact is the command's own
/// `Selection` transfer over the settled lattice, and a statement whose
/// transfer declines records nothing.
///
/// The fact counts the command's pattern and body pairs in order
/// ([`SelectionFact`](tcl_registry::value_transfer::SelectionFact));
/// `arm_pattern_spans` holds the pattern span of each arm the statement
/// keeps, in the same order. The statement keeps a final `default` pair as
/// its default body rather than an arm, so an arm index equal to
/// `arm_pattern_spans.len()` names that default ([`Self::is_default`]).
#[derive(Debug, Clone, PartialEq)]
pub struct SelectionRecord {
    /// The statement's span.
    pub span: tcl_lexer::Span,
    /// Each kept arm's pattern span, in order.
    pub arm_pattern_spans: Vec<tcl_lexer::Span>,
    /// The selection.
    pub fact: tcl_registry::value_transfer::SelectionFact,
}

impl SelectionRecord {
    /// Whether arm index `arm` names the final `default` pair — the
    /// statement's default body, which has no pattern span of its own.
    #[must_use]
    pub fn is_default(&self, arm: usize) -> bool {
        arm == self.arm_pattern_spans.len()
    }
}

/// Full SCCP result: per-SSA-value lattice entries, the set of
/// reachable blocks, the set of reachable edges, and
/// constant-folded branch annotations for reachable blocks.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SccpResult {
    /// Original native math dependencies, independent of numerical knowledge.
    pub required_math_invocations: Vec<crate::command_binding::SourceMathInvocation>,
    /// Actual expression entry preparations retained by producer proofs.
    pub required_expression_preparations: Vec<crate::command_binding::SourceExpressionPreparation>,
    /// Per-SSA-value lattice entry.
    pub values: HashMap<ValueKey, LatticeValue>,
    /// Blocks reachable from `cfg.entry` under current assumptions.
    pub executable_blocks: HashSet<BlockId>,
    /// `(from_block, to_block)` edges known executable.
    pub executable_edges: HashSet<(BlockId, BlockId)>,
    /// Constant branches detected during propagation.
    pub constant_branches: Vec<ConstantBranch>,
    /// Per statement, the value-transfer route the resolved invocation
    /// declared and how it answered at the fixed point — what the
    /// Explorer's `sccp` view renders beside the lattice.
    pub explanations: Vec<crate::value_transfer::RouteExplanation>,
    /// How many times the run dispatched to each route family, nested
    /// entries included.
    pub route_tally: crate::value_transfer::RouteTally,
    /// Per SSA value, the semantic type, shape and representation evidence
    /// the evaluation that produced it states — kept apart from the exact
    /// value in [`Self::values`], so a consumer asking whether a use
    /// converts reads the representation, never the type alone. A φ keeps
    /// what every executable incoming value states alike; a definition a
    /// barrier widened, or one no evaluation produced, has none.
    pub folded_types: HashMap<ValueKey, crate::value_transfer::FoldedType>,
    /// Per SSA value its statement left untouched — every store an
    /// evaluated outcome makes to its place a `Preserve` (a `regexp` that
    /// did not match, a `scan` whose input ran out, a pack command's
    /// declared preserve), or a condition's substitution the shared engine
    /// ran without a store ([`record_condition_preserves`]) — the version
    /// the place held before the statement. The definition holds that
    /// version's value and exists exactly when it does, so a read of one
    /// whose prior version is unset is a read before set.
    pub preserved: HashMap<ValueKey, crate::ssa::Version>,
    /// The statements the solver proved certainly raise where a handler is
    /// thrown to, as `(block, index)` (`DefValues::Raised`): such a
    /// statement's definitions hold what the stores that ran left, and the
    /// statement's effect is the raise itself, so no consumer may take it for
    /// a store that evaluated cleanly.
    pub raised: HashSet<(BlockId, usize)>,
    /// Each executable `subst` call's template-word plan, in source order —
    /// what W102, the template folders, extract-proc and the dynamic-name
    /// barrier read instead of walking the template themselves.
    pub template_plans: Vec<TemplatePlanRecord>,
    /// Each executable opaque case-list statement's selection over the
    /// settled lattice, in source order.
    pub selections: Vec<SelectionRecord>,
    /// Per SSA value, the existence rung
    /// (`docs/design/compiler/value-transfers.md` § *Existence*): whether
    /// the place is bound where the version is established — by its
    /// definition's storage outcome, by the join a φ takes over the
    /// executable edges, or by the frame's entry rules for version 0. A
    /// version the solver never reached, and every version of a run that
    /// computes no existence (a re-run, a guarded function), has no entry,
    /// which a consumer reads as unavailable — never as unbound.
    pub existence: HashMap<ValueKey, Existence>,
    /// Per statement and variable it reads, the existence fact the place
    /// holds just before the statement runs — after every barrier and
    /// computed name between the version's definition and the statement,
    /// which the per-version fact does not see. Keyed `(block, statement
    /// index, symbol)`.
    pub existence_reads: HashMap<(BlockId, u32, Symbol), Existence>,
    /// Per executable block, the existence fact of every symbol at its
    /// exit, indexed by the symbol: what its terminator reads.
    pub existence_exits: HashMap<BlockId, Vec<Existence>>,
    /// The block-qualified existence facts: for a block and the version of
    /// a place live at its entry, the fact the place holds there where it
    /// differs from the version's own — a guard's refinement, or a clobber
    /// since the definition. [`Self::existence_at`] reads it before the
    /// per-version map.
    pub existence_entries: HashMap<(BlockId, ValueKey), Existence>,
    /// The refinements of the function's branch edges, every domain's, one
    /// per edge, place and fact: the existence rung's where the run computes
    /// it, and the others in every run.
    pub refinements: Vec<EdgeRefinement>,
    /// Per executable block, the refinements outside the existence domain
    /// in force at its statements and its terminator, as indices into
    /// [`Self::refinements`]: those every executable path into the block
    /// carries since a branch edge took them, and that no barrier or
    /// up-frame since — nor in the block itself — may have overridden.
    pub refinements_at: HashMap<BlockId, Vec<usize>>,
    /// The block-qualified values: for a block and a version a refinement
    /// in force there narrows, the value the version holds at the block's
    /// statements where it differs from the version's own.
    /// [`Self::value_at`] reads it before [`Self::values`].
    pub value_entries: HashMap<(BlockId, ValueKey), LatticeValue>,
    /// The places only an existence query names, by their slot past the
    /// SSA's symbols, which a key of the existence rung may hold.
    pub query_places: Vec<String>,
    /// Each existence guard, where the run computes existence: a place a
    /// branch edge's condition proves bound (`[info exists x]`'s true edge,
    /// `![info exists x]`'s false one) and the block the edge enters,
    /// whether or not the rung refines the place.
    pub existence_guards: Vec<(String, BlockId)>,
    /// Each loop the solver ran to its exit over the exact state it starts
    /// from (`docs/design/compiler/value-transfers.md` § *Bounded-loop
    /// enumeration*), in the order of the blocks they leave to.
    pub loop_enumerations: Vec<EnumeratedLoop>,
    /// Whether the run read the module's procedures — a parameter default
    /// `info default` names, or a callee's transfer summary — or would have
    /// read them had its caller held them: a lattice built without the
    /// module's procedures then answers less than one built with them.
    pub reads_module: bool,
    /// What a run made to read how the procedure completes found
    /// ([`ModuleRun::reads_exits`]): where it raises, and whether a call
    /// it made has a completion no re-run decided.
    pub completion: RunCompletion,
}

/// How a run made to read a procedure's exits found it may complete beside
/// its exits ([`SccpResult::completion`]); empty for every other run.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RunCompletion {
    /// The statements that certainly raise where no handler takes the
    /// throw, as `(block, index)`: the procedure raises there, and its
    /// block's own way out is never reached.
    pub raises: HashSet<(BlockId, usize)>,
    /// Whether a call to a procedure of the module has a completion the
    /// run could not decide: no re-run of the callee was made, or one made
    /// with a seed that is not exact, or one that itself held such a call
    /// or a raise beside its normal exits.
    pub undecided: bool,
}

impl RunCompletion {
    /// Whether a call that reaches the run's exits is one that completes
    /// there: no statement the run evaluated certainly raises, and every
    /// call it applied had its completion decided.
    #[must_use]
    pub fn is_decided(&self) -> bool {
        self.raises.is_empty() && !self.undecided
    }

    /// Whether `block` holds a statement that certainly raises, so its
    /// terminator is never reached.
    #[must_use]
    pub fn raises_in(&self, block: BlockId) -> bool {
        self.raises.iter().any(|&(at, _)| at == block)
    }
}

/// A loop the solver ran to its exit over the exact state it starts from,
/// and what that published.
#[derive(Debug, Clone, PartialEq)]
pub struct EnumeratedLoop {
    /// The block the loop leaves to.
    pub exit_block: BlockId,
    /// The loop statement's span.
    pub span: tcl_lexer::Span,
    /// The passes it ran.
    pub iterations: u64,
    /// How it left.
    pub exit: tcl_registry::value_transfer::ExitRule,
    /// The value each place it wrote holds where it leaves, exactly, which
    /// the solver states on every executable edge into [`Self::exit_block`]
    /// for the version live there; none when it left with a completion its
    /// plan does not absorb, whose state holds on that path alone.
    pub published: Vec<(String, LatticeValue)>,
}

/// A fact that holds on one CFG edge, and in the blocks every path into
/// which crosses that edge, for one SSA version, without a new definition
/// (`docs/design/compiler/value-transfers.md` § *Predicate refinement*):
/// what a taken comparison, `switch` arm, type test or existence guard
/// proves about its operand. The existence rung carries its own domain's
/// facts from the edge until a barrier, an up-frame or a write clobbers the
/// place; every other domain's hold in the blocks
/// [`SccpResult::refinements_in`] names. The record carries no span.
#[derive(Debug, Clone, PartialEq)]
pub struct EdgeRefinement {
    /// The guarded edge: the branch block and the block the edge enters.
    pub edge: (BlockId, BlockId),
    /// The version the refinement narrows: the place's version at the
    /// branch block's exit, or, for the state an enumerated loop leaves, the
    /// version live where it leaves to.
    pub key: ValueKey,
    /// The domain it narrows.
    pub domain: tcl_registry::value_transfer::FactDomain,
    /// The narrowed fact, in the domain's view: an exact value or a finite
    /// set of them for the value domain, a domain fact for every other.
    pub fact: tcl_registry::value_transfer::FactView,
    /// What the refinement rests on.
    pub evidence: tcl_registry::value_transfer::DependencyEvidence,
    /// Whether the fact is the state an enumerated loop leaves, which holds
    /// of the version whatever its own value: a value of its own that the
    /// fact rules out is one the solver has not settled yet.
    pub loop_exit: bool,
}

impl SccpResult {
    /// The existence fact the statement at `index` of `block` finds at
    /// `symbol`'s place, when the statement reads it and the run computed
    /// existence; `None` is unavailable, which is neither bound nor
    /// unbound.
    #[must_use]
    pub fn existence_before(
        &self,
        block: BlockId,
        index: usize,
        symbol: Symbol,
    ) -> Option<Existence> {
        let index = u32::try_from(index).ok()?;
        self.existence_reads.get(&(block, index, symbol)).copied()
    }

    /// The existence fact the version `key` holds at `block`'s entry, for a
    /// version live there: the block-qualified fact where a guard's
    /// refinement or a clobber since the definition changed it, else the
    /// version's own; `None` is unavailable.
    #[must_use]
    pub fn existence_at(&self, block: BlockId, key: ValueKey) -> Option<Existence> {
        self.existence_entries
            .get(&(block, key))
            .or_else(|| self.existence.get(&key))
            .copied()
    }

    /// The value the version `key` holds at `block`'s statements and its
    /// terminator: the block-qualified value where a refinement in force
    /// there narrows it, else the version's own. A refinement never makes a
    /// version, and at a merge only what every incoming path carries holds,
    /// so two arms refining `x` to `a` and to `b` meet as `x`'s own value.
    #[must_use]
    pub fn value_at(&self, block: BlockId, key: ValueKey) -> Option<&LatticeValue> {
        self.value_entries
            .get(&(block, key))
            .or_else(|| self.values.get(&key))
    }

    /// The refinements outside the existence domain in force at `block`'s
    /// statements and its terminator ([`Self::refinements_at`]).
    pub fn refinements_in(&self, block: BlockId) -> impl Iterator<Item = &EdgeRefinement> {
        self.refinements_at
            .get(&block)
            .into_iter()
            .flatten()
            .filter_map(|&index| self.refinements.get(index))
    }

    /// Whether an existence guard proves `name` bound at `block`: the block a
    /// guarded edge enters dominates it. The rung refines no place another
    /// actor may write, so for such a place this is the one word W210 has
    /// that the guard ran; it reads the place's name, as the guard does, and
    /// leaves to the rung's fact a read past the guard's region.
    #[must_use]
    pub fn guarded(&self, ssa: &SsaFunction, name: &str, block: BlockId) -> bool {
        self.existence_guards
            .iter()
            .any(|(place, entered)| place == name && crate::loops::dominates(ssa, *entered, block))
    }

    /// The name of the place a key's symbol stands for: an SSA variable, or
    /// a place only an existence query names ([`Self::query_places`]).
    #[must_use]
    pub fn place_name<'s>(&'s self, ssa: &'s SsaFunction, symbol: Symbol) -> Option<&'s str> {
        let slot = symbol.0 as usize;
        let symbols = ssa.var_names();
        symbols.get(slot).map_or_else(
            || {
                self.query_places
                    .get(slot.checked_sub(symbols.len())?)
                    .map(String::as_str)
            },
            |name| Some(name.as_str()),
        )
    }

    /// The existence fact `symbol`'s place holds at `block`'s exit, where
    /// its terminator reads, when the run computed existence.
    #[must_use]
    pub fn existence_at_exit(&self, block: BlockId, symbol: Symbol) -> Option<Existence> {
        self.existence_exits
            .get(&block)
            .and_then(|state| state.get(symbol.0 as usize).copied())
    }

    /// Whether the constant definition `key` holds may be written into
    /// source as a literal. A value a route constructed as a byte array has
    /// no lossless source spelling (`docs/design/compiler/value-transfers.md`
    /// § *Exact values, types, and representation*: a computed `binary
    /// format` needs a lossless materialisation contract before it is
    /// emitted anywhere), so no rewrite materialises one; the analyses still
    /// read its value.
    #[must_use]
    pub fn materialises(&self, key: ValueKey) -> bool {
        self.folded_types.get(&key).is_none_or(|folded| {
            folded.representation
                != tcl_registry::value_transfer::RepresentationEvidence::Constructed(
                    tcl_registry::TclType::ByteArray,
                )
        })
    }
}

/// Sparse Conditional Constant Propagation driver.
///
/// Iterates to a fixed point over the lattice values of every SSA
/// value, using CFG reachability (via `executable_edges`) so that
/// unreachable branches don't widen their targets. `param_constants`
/// lets interprocedural analysis seed the caller-provided argument
/// lattice entries.
///
/// Behaviour:
///
/// - Phi handling uses the incoming versions for each *executable*
///   predecessor, joining them onto the phi's SSA value.
/// - Statement handling uses [`evaluate_defs_under`], which folds
///   [`Statement::AssignConst`] and [`Statement::AssignExpr`] via
///   the expression evaluator. Other statement kinds and
///   [`Statement::Barrier`] widen their defs to `Overdefined`.
/// - Branch decisions are resolved via [`evaluate_branch`] below,
///   which consults the lattice environment and then the expression
///   evaluator.
///
/// `policy` carries the two dialect facts every fold on this pass needs —
/// see [`FoldPolicy`].  Its `octal` half controls how a bare leading-zero
/// string literal (`"08"`, `"010"`) is interpreted when folding `==` /
/// `!=`: `Some(true)` for the tcl8.x octal rule (`"08"` is an invalid octal
/// → string, `"010"` → 8), `Some(false)` for the tcl9.0 decimal rule
/// (`"08"` → 8, `"010"` → 10), and `None` to decline folding such ambiguous
/// operands (the safe default for callers without dialect context).  Its
/// `is_irules` half enables the iRules word operators (`contains`,
/// `starts_with`, …), so `if {$x contains "cd"}` with a known-constant `$x`
/// folds through SCCP under `f5-irules` exactly as the `eq` control does.
///
/// `trace` bundles the registry-driven whole-module trace facts this
/// function consults in addition to its own intra-procedural
/// [`crate::var_observability`] lattice — see [`TraceInputs`]. A caller
/// with no `Module` in hand (a standalone unit test) passes an empty
/// `BTreeSet` and `false`, behaviourally identical to "nothing is traced".
#[must_use]
// `implicit_hasher`: `param_constants` is an `Option<&HashMap>` that almost
// every caller passes as `None` (only the interprocedural seed passes `Some`).
// Generalising over `BuildHasher` makes `S` un-inferable at every `None` call
// site — including out-of-subsystem callers (shimmer, dataflow tests) that
// cannot be annotated from here — so the concrete default hasher is required.
#[allow(clippy::implicit_hasher)]
pub fn sccp(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    param_constants: Option<&HashMap<(String, crate::ssa::Version), LatticeValue>>,
    policy: FoldPolicy,
    trace: TraceInputs<'_>,
) -> SccpResult {
    sccp_with_extra_escaping(cfg, ssa, param_constants, policy, &HashSet::new(), trace)
}

/// Inputs for folding a pure-builtin command substitution **during lattice
/// evaluation**: the registry `const_fold` callbacks are pure
/// functions of constant argument words, so an `AssignValue` RHS like
/// `[namespace qualifiers $base]` whose `$base` is a lattice constant folds
/// to a lattice constant itself — the folded value re-enters the lattice and
/// multi-statement chains (`set base [self class]; set ns [namespace
/// qualifiers $base]`) close under SCCP's ordinary fixpoint.
///
/// Termination is SCCP's own: the fold is a deterministic function of the
/// use versions' lattice values, which only ever move down the lattice
/// (`Unknown → Const → Overdefined`), and nested-substitution recursion is
/// bounded by the engine's structural depth cap
/// (`crate::const_subst`).
///
/// Carries the whole-module command-mutation trust fact
/// ([`crate::command_binding::ModuleCommandMutations`]) — a renamed /
/// aliased / shadowed head must never fold with builtin semantics — and is
/// therefore what *every* resolved head's declared value transfer is gated
/// on (`crate::value_transfer`'s driver, under the stance [`Self::trust`]
/// names), and the registry `const_fold` engine besides. A caller that
/// passes `None` holds no whole-module view and gets no builtin answer at
/// all; `registry_engine` then selects whether a caller that does hold one
/// also gets the registry `const_fold` engine.
#[derive(Clone, Copy)]
pub struct BuiltinFoldInputs<'a> {
    /// Actual supplied source availability; absent input cannot recreate it.
    pub source_metadata_input: Option<&'a crate::analyser::ResolvedAnalysisInput>,
    /// Command / subcommand specs — the fold callbacks live here. Carried
    /// here (as well as on [`TraceInputs`]) so the statement-evaluation
    /// helpers need only this one bundle.
    pub registry: &'a CommandRegistry,
    /// Whole-module `rename` / `interp alias` / shadowing-`proc` trust scan.
    pub mutations: &'a crate::command_binding::ModuleCommandMutations,
    /// Resolved dialect profile for versioned folds; `None` when unavailable.
    pub dialect: Option<&'static tcl_dialect::DialectProfile>,
    /// Proven defining class of the enclosing `TclOO` instance-method frame
    /// (enables `[self class]`-style frame-fact folds); `None` elsewhere.
    pub defining_class: Option<&'a str>,
    /// Whether the registry `const_fold` engine may run after the resolved
    /// head's declared route declines. The shared per-unit lattice
    /// ([`crate::compilation_unit::FunctionUnit`]) supplies the trust fact
    /// with the engine off, which gates its declared routes without widening
    /// what it folds; the optimiser's own re-run turns it on. It never gates
    /// a declared route.
    pub registry_engine: bool,
    /// Which half of `mutations` gates the declared routes — see
    /// [`FoldTrust`].
    pub trust: FoldTrust,
    /// A caller has proved this procedure pure before evaluating it with
    /// constant arguments. Its caller-bound parameter roots cannot be mutated
    /// by a handler; ordinary analyses must leave this false.
    pub proven_pure_parameters: bool,
}

/// How much of the whole-module mutation summary gates a builtin fold.
///
/// The two answers differ only on
/// [`crate::command_binding::ModuleCommandMutations`]'s unbounded `dynamic`
/// top; on every *named* subject — a shadowing `proc`, a `rename`, an alias,
/// an opaque import — they agree, which is what keeps one call from carrying
/// two answers within one statement (#2164).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FoldTrust {
    /// Everything
    /// [`crate::command_binding::ModuleCommandMutations::trusts`] clears,
    /// the `dynamic` top included: nothing folds once *any* binding
    /// transition in the module is unbounded. The stance for a fold that
    /// becomes a source rewrite.
    WholeModule,
    /// What the module's own observed bindings clear
    /// ([`crate::command_binding::ModuleCommandMutations::observed_binding_is_the_builtin`]).
    /// The stance for the shared per-unit value lattice, which answers "what
    /// does this call evaluate to given the definitions this module holds"
    /// and must not lose every fold to one unresolved command head.
    ObservedBindings,
}

/// Registry-driven whole-module trace facts [`sccp`] /
/// [`sccp_with_extra_escaping`] consult when widening their escaping-set,
/// bundled into one `Copy` struct to keep those functions' argument count
/// under the clippy `too_many_arguments` ceiling.
#[derive(Clone, Copy)]
pub struct TraceInputs<'a> {
    /// Actual supplied source availability and lexer axes.
    pub source_metadata_input: Option<&'a crate::analyser::ResolvedAnalysisInput>,
    /// Resolves the variable-trace grammar for the intra-procedural
    /// [`crate::var_observability`] lattice `sccp` builds internally.
    pub registry: &'a CommandRegistry,
    /// [`crate::ir::Module::traced_variables`] — every literal variable
    /// name targeted anywhere in the module by a
    /// `Traits::ESTABLISHES_VARIABLE_TRACE` subcommand; also catches a
    /// trace installed by a *called* proc, invisible to the
    /// single-`CfgFunction` `var_observability` view.
    pub traced_variables: &'a BTreeSet<String>,
    /// [`crate::ir::Module::has_dynamic_variable_trace`] — set when any
    /// such subcommand targets a non-literal (dynamic) name, in which case
    /// *every* variable is potentially traced.
    pub has_dynamic_variable_trace: bool,
    /// [`crate::ir::Module::deferred_writes`] — the names the callback
    /// scripts of the module write. Each is externally mutable in every
    /// function, as a traced name is, and a callback that writes a computed
    /// name makes every variable so.
    pub deferred_writes: &'a crate::ir::DeferredWrites,
    /// The analysis context's memo identity for this run — the registry
    /// and overlay generations, the module's command-binding evidence, the
    /// tier, and the evaluator revision — when the caller carries one; a
    /// caller with no module view passes `None` and runs detached.
    pub analysis_context: Option<&'a AnalysisContextKey>,
    /// The frame facts the existence rung enters the function with, when
    /// the caller asks for it; `None` computes no existence, and every
    /// existence read of the run is unavailable.
    pub existence: Option<ExistenceEntry<'a>>,
}

impl TraceInputs<'_> {
    /// Add the names the module makes externally mutable in every function:
    /// those a variable trace targets and those a callback script writes.
    pub(crate) fn extend_module_escaping(&self, escaping: &mut HashSet<String>) {
        escaping.extend(self.traced_variables.iter().cloned());
        escaping.extend(self.deferred_writes.names.iter().cloned());
    }

    /// Whether any variable may change at any time: the module traces a
    /// computed name, or a callback script writes one.
    pub(crate) fn writes_any_variable(&self) -> bool {
        self.has_dynamic_variable_trace || self.deferred_writes.any
    }

    /// [`Self::writes_any_variable`] for `cfg`, leaving out callbacks
    /// registered only at its sites in blocks `executable` rejects, which
    /// never run.
    pub(crate) fn writes_any_variable_in(
        &self,
        cfg: &crate::cfg::Function,
        executable: &dyn Fn(&crate::cfg::BlockId) -> bool,
    ) -> bool {
        self.has_dynamic_variable_trace
            || callback_writes_any(cfg, self.deferred_writes, executable)
    }

    /// `self` with a callback that writes a computed name read as a trace on
    /// one, which is how it can change any variable, except one registered
    /// only by statements at sites [`DeferredWrites::any_sites`] names: those
    /// [`RunHazards`] charges to the blocks that hold them.
    fn with_callback_writes_as_traces(self) -> Self {
        Self {
            has_dynamic_variable_trace: self.has_dynamic_variable_trace
                || self.deferred_writes.any && self.deferred_writes.any_sites.is_empty(),
            ..self
        }
    }
}

/// Initial variable presence, independently of a place's external mutability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExistenceInitialState {
    /// A genuine declared frame supplies its parameter and linked-state facts.
    DeclaredFrame,
    /// A conditional child may inherit earlier caller writes or destruction.
    InheritedFrame,
}

/// What a function's frame binds on entry, for the existence rung
/// (`docs/design/compiler/value-transfers.md` § *Existence*, the entry
/// state): parameters bound as scalars; a `TclOO` method's instance
/// variables and, in an iRules `when` handler, the connection's variables,
/// linked to state another invocation may have bound; in the document's
/// initial global frame the registry's special variables as startup binds
/// them. Other locals in a declared frame enter unbound; an inherited frame
/// supplies no incoming absence and starts may-bound.
#[derive(Clone, Copy)]
pub struct ExistenceEntry<'a> {
    /// Whether ordinary places begin in a new declared or inherited frame.
    pub initial_state: ExistenceInitialState,
    /// The formal parameter names: bound as scalars on entry.
    pub params: &'a [String],
    /// A `TclOO` method body's instance variables
    /// ([`crate::ir::MethodDef::instance_vars`]): linked on entry, bound
    /// exactly when object state holds them.
    pub object_state: Option<&'a HashSet<String>>,
    /// Whether the body is the document's initial global frame, the
    /// interpreter's own globals.
    pub initial_global: bool,
    /// The names an iRules `when` handler may find bound on entry
    /// ([`crate::value_transfer::AnalysisContextKey::connection_scoped`]);
    /// `None` for every other body.
    pub connection_scoped: Option<&'a crate::value_transfer::ConnectionScoped>,
    /// Whether the module traces a computed variable name, so a trace may
    /// create or destroy any place at any time.
    pub dynamic_trace: bool,
    /// The document's lexer configuration, under which the per-statement
    /// computed-name scan re-reads the words the lowering read.
    pub config: tcl_lexer::LexerConfig,
    /// The locals a procedure links to its caller's places, with the fact
    /// they enter with: set only for the runs that compute a procedure's
    /// transfer summary and for a re-run of it under one call
    /// ([`CallerPlaces`]).
    pub caller_places: Option<&'a CallerPlaces>,
}

/// The locals a procedure links to its caller's places, and the fact each
/// enters with: in a run of its transfer summary every one bound, then every
/// one unbound; in a re-run under one call, what the call's places hold.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CallerPlaces {
    /// Each linked local, with the fact it holds once linked.
    pub entries: HashMap<String, Existence>,
}

impl CallerPlaces {
    /// Every one of `names` entering as `entry`.
    #[must_use]
    pub fn each(names: &HashSet<String>, entry: Existence) -> Self {
        Self {
            entries: names.iter().map(|name| (name.clone(), entry)).collect(),
        }
    }

    /// Whether `name` is a linked local.
    #[must_use]
    pub fn links(&self, name: &str) -> bool {
        self.entries.contains_key(name)
    }
}

/// Like [`sccp`] but additionally forces every name in `extra_escaping` to
/// `Overdefined`, the same treatment [`is_externally_mutable`] already gives
/// a name this *function's own* `global`/`variable`/`upvar`/`trace`
/// declares.
///
/// Needed for the *top-level* script specifically: top-level names already
/// live in the global frame (there is no separate local frame for them to
/// shadow), so a name the top-level body never mentions via `global` can
/// still be reassigned mid-run by any *other* procedure's own `global NAME;
/// set NAME …` — a plain call, with nothing textually resembling an alias
/// from the top level's point of view, and therefore invisible to the
/// per-function [`crate::var_observability`] scan `sccp` runs internally.
/// [`crate::var_observability::scan_module_global_names`] computes the
/// whole-module fact this closes the gap with; every other caller passes an
/// empty set (via plain [`sccp`]) and gets identical behaviour to before.
#[must_use]
#[allow(clippy::implicit_hasher)]
pub fn sccp_with_extra_escaping(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    param_constants: Option<&HashMap<(String, crate::ssa::Version), LatticeValue>>,
    policy: FoldPolicy,
    extra_escaping: &HashSet<String>,
    trace: TraceInputs<'_>,
) -> SccpResult {
    sccp_with_builtin_folds(
        cfg,
        ssa,
        param_constants,
        policy,
        extra_escaping,
        trace,
        None,
    )
}

/// Like [`sccp_with_extra_escaping`] but with the whole-module command-trust
/// fact in hand, so a resolved command answers at all — its declared value
/// transfer always, and the registry `const_fold` engine when
/// [`BuiltinFoldInputs::registry_engine`] is set. Passing `None` is
/// byte-identical to [`sccp_with_extra_escaping`], which answers for no
/// resolved command for want of that fact.
#[must_use]
#[allow(clippy::implicit_hasher)]
pub fn sccp_with_builtin_folds(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    param_constants: Option<&HashMap<(String, crate::ssa::Version), LatticeValue>>,
    policy: FoldPolicy,
    extra_escaping: &HashSet<String>,
    trace: TraceInputs<'_>,
    folds: Option<BuiltinFoldInputs<'_>>,
) -> SccpResult {
    let inputs = SolveInputs {
        cfg,
        ssa,
        param_constants,
        policy,
        extra_escaping,
        trace,
        folds,
        module: ModuleRun::NONE,
    };
    drive_escaping(&inputs)
}

/// A known analysis body; an outer `None` retains unavailable body identity.
#[derive(Clone, Copy)]
pub(crate) enum AnalysisBody {
    TopLevel,
    Procedure(crate::ir::ProcedureId),
}

impl AnalysisBody {
    pub(crate) const fn procedure_id(self) -> Option<crate::ir::ProcedureId> {
        match self {
            Self::TopLevel => None,
            Self::Procedure(id) => Some(id),
        }
    }
}

/// What a run reads of the module beyond one function: its procedures,
/// which a call to one of them reaches, and the names the run owns although
/// the function aliases them.
#[derive(Clone, Copy)]
pub(crate) struct ModuleRun<'a> {
    pub(crate) analysis_body: Option<crate::sccp::AnalysisBody>,
    /// The module's procedures, where the caller holds them.
    pub(crate) procedures: Option<&'a crate::interprocedural::ModuleProcedures<'a>>,
    /// Names the function links to a caller's place which the run treats as
    /// its own locals: a procedure's run for its transfer summary, which
    /// states what it does to the place. A name a trace or a callback script
    /// may reach stays externally mutable.
    pub(crate) owned: Option<&'a HashSet<String>>,
    /// What a call to a procedure of the module gives the run.
    pub(crate) level: ModuleLevel,
    /// Whether the run is made to read how the procedure completes — a
    /// callee's re-run, a summary's own run, a seedless return run, O103's
    /// argument-sensitive re-run: a statement that certainly raises ends its
    /// path in every block, where a unit's lattice ends it only where a
    /// handler is thrown to, so that no code is proved unreachable by an
    /// error the procedure raises itself.
    pub(crate) reads_exits: bool,
}

/// What a run takes from a call to a procedure of the module, as the
/// callee's transfer summary says ([`crate::interprocedural::ModuleProcedures`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ModuleLevel {
    /// The existence the summary states for each place the call names: a
    /// summary's own run, which composes its callees' summaries.
    Outcomes,
    /// Each such place's existence and value, the value where a re-run of
    /// the callee under the call's arguments and the places' facts proves
    /// one: a unit's lattice and a seedless run.
    Places,
    /// Those, and the call's result where the re-run proves it: a re-run's
    /// own run, the argument-sensitive one O103 makes among them.
    Results,
}

impl<'a> ModuleRun<'a> {
    /// A run that reads nothing of the module.
    pub(crate) const NONE: Self = Self {
        procedures: None,
        analysis_body: None,
        owned: None,
        level: ModuleLevel::Places,
        reads_exits: false,
    };

    /// A run that reads the module's procedures where they are given, and
    /// owns no aliased name.
    pub(crate) const fn reading(
        procedures: Option<&'a crate::interprocedural::ModuleProcedures<'a>>,
    ) -> Self {
        Self {
            procedures,
            analysis_body: None,
            owned: None,
            level: ModuleLevel::Places,
            reads_exits: false,
        }
    }
}

/// [`sccp_with_builtin_folds`] over `inputs`, which say what the run reads
/// of the module ([`ModuleRun`]).
#[must_use]
pub(crate) fn sccp_in_module(inputs: &SolveInputs<'_>) -> SccpResult {
    drive_escaping(inputs)
}

/// The branch's driver retains its actual source configuration and services.
#[derive(Clone, Copy)]
pub(crate) struct BranchFold<'a> {
    /// The run's value-transfer driver, whose services the condition is
    /// evaluated through.
    driver: &'a LatticeDriver<'a>,
}

#[cfg(test)]
mod tests;
