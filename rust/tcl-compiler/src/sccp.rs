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

use rustc_hash::FxHashSet;
use tcl_lexer::TokenType;
use tcl_registry::CommandRegistry;
use tcl_registry::value_transfer::{
    BindingKind, DependencyEvidence, DomainFact, ExactValue, Existence, FactDomain, FactView,
};

use crate::analyses::{ConstValue, LatticeValue, MAX_CONSTSET_SIZE};
use crate::cfg::{BlockId, Function as CfgFunction, Terminator};
use crate::codegen::helpers::split_list_values;
use crate::expr_ast::ExprNode;
use crate::ir::Statement;
use crate::ssa::{SsaFunction, SsaStatement, Symbol, ValueKey};
use crate::tcl_expr_eval::FoldPolicy;
use crate::value_transfer::{AnalysisContextKey, LatticeDriver};

mod original_value_facts;
pub(crate) use original_value_facts::DiagnosticValueFacts;
pub use original_value_facts::{
    ExistenceFrame, ExistenceMetadataInputs, ExpressionAnalysis, ExpressionEvaluationPoint,
    SemanticValueFacts, SemanticValueProjection, ValueFactInputs, evaluate_branch, evaluate_def,
    evaluate_def_with_folds, execution_value_facts, existence_constant_branches,
    existence_constant_branches_with_metadata_context, existence_constant_branches_with_ssa,
    existence_constant_branches_with_ssa_with_metadata_context, semantic_value_facts,
};

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

// Lattice join

/// Canonical [`ConstValue`] ordering for deterministic set merges.
///
/// [`ConstValue::Float`] is not [`Eq`], so we rely on byte-level
/// equality of the `to_bits` representation for hashing/sorting.
fn cv_key(v: &ConstValue) -> (u8, String) {
    match v {
        ConstValue::Int(i) => (0, i.to_string()),
        ConstValue::Float(f) => (1, format!("{:016x}", f.to_bits())),
        ConstValue::Bool(b) => (2, b.to_string()),
        ConstValue::String(s) => (3, s.clone()),
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
        if !merged.iter().any(|m| cv_eq(m, v)) {
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

/// Equality check that treats `Float` values via bitwise
/// comparison so NaN sorts deterministically into its own bucket.
fn cv_eq(a: &ConstValue, b: &ConstValue) -> bool {
    match (a, b) {
        (ConstValue::Int(x), ConstValue::Int(y)) => x == y,
        (ConstValue::Float(x), ConstValue::Float(y)) => x.to_bits() == y.to_bits(),
        (ConstValue::Bool(x), ConstValue::Bool(y)) => x == y,
        (ConstValue::String(x), ConstValue::String(y)) => x == y,
        _ => false,
    }
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
/// - Statement handling uses [`evaluate_def`] below, which folds
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

    /// `self` with a callback that writes a computed name read as a trace on
    /// one, which is how it can change any variable.
    fn with_callback_writes_as_traces(self) -> Self {
        Self {
            has_dynamic_variable_trace: self.writes_any_variable(),
            ..self
        }
    }
}

/// What a function's frame binds on entry, for the existence rung
/// (`docs/design/compiler/value-transfers.md` § *Existence*, the entry
/// state): parameters bound as scalars; a `TclOO` method's instance
/// variables and, in an iRules `when` handler, the connection's variables,
/// linked to state another invocation may have bound; in the document's
/// initial global frame the registry's special variables as startup binds
/// them. Every other local enters unbound.
#[derive(Clone, Copy)]
pub struct ExistenceEntry<'a> {
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

/// The blocks executable before the sweep starts: the entry, when the
/// function has one.
fn entry_block_set(cfg: &CfgFunction) -> HashSet<BlockId> {
    cfg.blocks
        .contains_key(&cfg.entry)
        .then_some(cfg.entry)
        .into_iter()
        .collect()
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
    drive(|round| solve(&inputs, round))
}

/// What a run reads of the module beyond one function: its procedures,
/// which a call to one of them reaches, and the names the run owns although
/// the function aliases them.
#[derive(Clone, Copy)]
pub(crate) struct ModuleRun<'a> {
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
    drive(|round| solve(inputs, round))
}

/// The solver's rounds over `solve`, one run of the solver each. A loop the
/// first run proves the start state of is run to its exit, and the run is
/// made again with the state it leaves stated on the edges it leaves by: the
/// values only descend. The second run enumerates nothing. Where its settled
/// values contradict the state some loops left, those loops' states are
/// dropped and the run made again without them; every other loop's state
/// stands. Each such run drops at least one loop, so the rounds end.
fn drive(mut solve: impl FnMut(&Round) -> Solved) -> SccpResult {
    let mut round = Round::First;
    let mut loops = Vec::new();
    loop {
        match solve(&round) {
            Solved::Settled(result) => {
                return SccpResult {
                    loop_enumerations: loops,
                    ..*result
                };
            }
            Solved::Enumerated(enumerated, exits) => {
                loops = enumerated;
                round = Round::Published(exits);
            }
            Solved::Contradicted(ends) => {
                loops.retain(|record| !ends.contains(&record.exit_block));
                if let Round::Published(exits) = &mut round {
                    exits.retain(|refinement| !ends.contains(&refinement.edge.1));
                }
            }
        }
    }
}

/// The inputs of one solver run.
#[derive(Clone, Copy)]
pub(crate) struct SolveInputs<'a> {
    pub(crate) cfg: &'a CfgFunction,
    pub(crate) ssa: &'a SsaFunction,
    pub(crate) param_constants: Option<&'a HashMap<(String, crate::ssa::Version), LatticeValue>>,
    pub(crate) policy: FoldPolicy,
    pub(crate) extra_escaping: &'a HashSet<String>,
    pub(crate) trace: TraceInputs<'a>,
    pub(crate) folds: Option<BuiltinFoldInputs<'a>>,
    pub(crate) module: ModuleRun<'a>,
}

impl<'a> SolveInputs<'a> {
    /// The run's driver over `trace` and the escaping set, reading what the
    /// run reads of the module.
    fn driver(&self, trace: TraceInputs<'a>, escaping: &HashSet<String>) -> LatticeDriver<'a> {
        LatticeDriver::new(trace, self.folds, self.policy, escaping)
            .with_cfg_metadata(&self.cfg.metadata_context)
            .in_module(&self.cfg.name, self.module)
    }
}

/// Which run of the solver this is.
enum Round {
    /// The first: the loops whose start state it proves are enumerated.
    First,
    /// The second, with the refinements the first run's loops published.
    Published(Vec<EdgeRefinement>),
}

/// What a run of the solver answers.
enum Solved {
    /// The settled result.
    Settled(Box<SccpResult>),
    /// The first run enumerated loops whose exits publish state: the loops
    /// and the refinements their exits state, for the second run.
    Enumerated(Vec<EnumeratedLoop>, Vec<EdgeRefinement>),
    /// The second run's settled values contradict the state the loops that
    /// leave to these blocks left.
    Contradicted(HashSet<BlockId>),
}

/// One run of the solver ([`sccp_with_builtin_folds`]).
fn solve(inputs: &SolveInputs<'_>, round: &Round) -> Solved {
    let SolveInputs {
        cfg,
        ssa,
        param_constants,
        policy,
        extra_escaping,
        trace,
        ..
    } = *inputs;
    let trace = trace.with_callback_writes_as_traces();
    let preds = compute_predecessors(cfg);
    let mut values = seeded_values(ssa, param_constants);

    let grammar = trace.source_metadata_input.map_or_else(
        || {
            trace
                .registry
                .profile()
                .map_or_else(tcl_dialect::LexerGrammar::default, |p| p.grammar)
        },
        |input| input.unit_profile().grammar,
    );
    seed_live_in_roots(cfg, ssa, &mut values, grammar);

    let escaping = escaping_names(cfg, &trace, extra_escaping, inputs.module.owned);
    // Every command-specific answer below comes from the registry's
    // declaration for the resolved invocation, through one driver whose
    // context is this run's identity.
    let driver = inputs.driver(trace, &escaping);
    driver.catch_ends(cfg);
    // What every branch edge's condition proves, in every domain, read
    // under the document's grammar.
    let config = trace.existence.map_or_else(
        || {
            trace.source_metadata_input.map_or_else(
                || crate::dynamic_names::lexer_config_for(trace.registry),
                crate::analyser::ResolvedAnalysisInput::lexer_config,
            )
        },
        |entry| entry.config,
    );
    let facts = branch_facts(cfg, (trace.registry, config, policy), &driver);
    // The existence rung runs beside the values, over the same executable
    // blocks and edges, when the caller asks for it.
    let mut existence = trace
        .existence
        .map(|entry| ExistenceRun::new(cfg, ssa, entry, (&escaping, &facts), trace.registry));
    if let Some(run) = &existence {
        driver.existence_places(run.query_only.clone());
        driver.existence_external(run.external.clone());
    }
    // Every refinement, in order: the existence rung's, which it carries
    // itself, and every other domain's, which the sweep reads through the
    // blocks they hold in, the state enumerated loops leave among them.
    let external = external_places((cfg, &trace), (&escaping, config));
    let mut others = value_refinements(&facts, ssa, &external);
    if let Round::Published(exits) = round {
        others.extend_from_slice(exits);
    }
    let refinements = ordered_refinements(existence.as_mut(), others);

    let executable_blocks = entry_block_set(cfg);
    let order = cfg_order(cfg);
    let regions = RegionShape::of(cfg);
    let sweep = SweepContext {
        cfg,
        ssa,
        preds: &preds,
        escaping: &escaping,
        has_dynamic_variable_trace: trace.has_dynamic_variable_trace,
        grammar,
        driver: &driver,
        regions: &regions,
        refinements: &refinements,
        external: &external,
    };
    let mut state = SweepState {
        values,
        executable_blocks,
        executable_edges: HashSet::new(),
        existence,
        refined: RefinementFlow::new(cfg, &refinements),
    };

    sweep.settle(&order, &mut state);

    let SweepState {
        mut values,
        executable_blocks,
        executable_edges,
        existence,
        refined,
    } = state;
    let refinements_at = refined.map(RefinementFlow::settled).unwrap_or_default();
    if matches!(round, Round::Published(_)) {
        let ends = contradicted_exits(&refinements, &refinements_at, &values);
        if !ends.is_empty() {
            return Solved::Contradicted(ends);
        }
    }
    if matches!(round, Round::First) {
        let (loops, exits) = enumerate_loops(
            (cfg, ssa, &preds),
            (&mut values, Narrowing::new(&refinements, &refinements_at)),
            (&executable_blocks, &executable_edges),
            existence.as_ref().map(|run| &run.exits),
            (&driver, &external),
        );
        if !exits.is_empty() {
            return Solved::Enumerated(loops, exits);
        }
    }
    Solved::Settled(Box::new(settled_result(
        (cfg, ssa, &order),
        (&driver, grammar),
        SettledRun {
            values,
            executable_blocks,
            executable_edges,
            existence,
            refinements,
            refinements_at,
        },
    )))
}

/// What a run's sweep settled on, before the post-passes read it.
struct SettledRun {
    values: HashMap<ValueKey, LatticeValue>,
    executable_blocks: HashSet<BlockId>,
    executable_edges: HashSet<(BlockId, BlockId)>,
    existence: Option<ExistenceRun>,
    refinements: Vec<EdgeRefinement>,
    refinements_at: HashMap<BlockId, Vec<usize>>,
}

/// A settled run's result: the constant branches and the post-passes over
/// the settled lattice, each block under the refinements in force there, and
/// every fact the run recorded.
fn settled_result(
    (cfg, ssa, order): (&CfgFunction, &SsaFunction, &[BlockId]),
    (driver, grammar): (&LatticeDriver<'_>, tcl_dialect::LexerGrammar),
    run: SettledRun,
) -> SccpResult {
    let SettledRun {
        mut values,
        executable_blocks,
        executable_edges,
        existence,
        refinements,
        refinements_at,
    } = run;
    let narrowing = Narrowing::new(&refinements, &refinements_at);
    let mut constant_branches = collect_constant_branches(
        cfg,
        ssa,
        (&mut values, narrowing),
        &executable_blocks,
        order,
        BranchFold { grammar, driver },
        existence.as_ref().map(|run| &run.exits),
    );
    let rung = existence.map_or_else(RungResults::default, |run| run.finish(ssa));
    // The post-passes read the settled lattice, each block under the
    // refinements in force there, before it moves in.
    let (template_plans, selections, unreached_arms) = settled_statements(
        (driver, cfg, ssa),
        (&mut values, narrowing),
        &executable_blocks,
    );
    constant_branches.extend(unreached_arms);
    let value_entries = refined_values(&refinements, &refinements_at, &values);
    SccpResult {
        template_plans,
        selections,
        values,
        executable_blocks,
        executable_edges,
        constant_branches,
        existence: rung.versions,
        existence_reads: rung.reads,
        existence_exits: rung.exits,
        existence_entries: rung.entries,
        refinements,
        refinements_at,
        value_entries,
        query_places: rung.query_places,
        existence_guards: rung.guards,
        ..driver.take_run_facts()
    }
}

/// The names whose value is never a compile-time constant.
///
/// Global / namespace / upvar-aliased / traced variables are shared mutable
/// state observable and writable from other scopes, traces, and source
/// files, so folding through one would be unsound across any opaque call
/// (`set ::g 5; mut; expr {$::g + 1}` must NOT fold to 6 — `mut` may have
/// rewritten `::g`). SCCP forces every such definition to `Overdefined` so it
/// never propagates a constant through one; the read is still tracked for
/// liveness. The set is the whole-function (flow-insensitive) view of the
/// `var_observability` alias/trace lattice, widened by any whole-module fact
/// the caller supplies (`extra_escaping`), by the whole-module
/// `traced_variables` fact — which also catches a trace installed by a
/// *called* proc, which the single-`CfgFunction` view cannot see — and by the
/// names a callback script of the module writes (`deferred_writes`), whose
/// write no statement of this function shows. A name the run owns
/// ([`ModuleRun::owned`]) is the run's own local, save one a trace or a
/// callback script may reach.
fn escaping_names(
    cfg: &CfgFunction,
    trace: &TraceInputs<'_>,
    extra_escaping: &HashSet<String>,
    owned: Option<&HashSet<String>>,
) -> HashSet<String> {
    let mut escaping = crate::var_observability::analyse_var_observability(cfg, trace.registry)
        .escaping_var_names();
    if let Some(owned) = owned {
        escaping.retain(|name| !owned.contains(name));
    }
    escaping.extend(extra_escaping.iter().cloned());
    trace.extend_module_escaping(&mut escaping);
    escaping
}

/// The read-only context one solver sweep runs each block under.
struct SweepContext<'a> {
    cfg: &'a CfgFunction,
    ssa: &'a SsaFunction,
    preds: &'a HashMap<BlockId, HashSet<BlockId>>,
    escaping: &'a HashSet<String>,
    has_dynamic_variable_trace: bool,
    grammar: tcl_dialect::LexerGrammar,
    driver: &'a LatticeDriver<'a>,
    /// Where the function's exception edges leave from.
    regions: &'a RegionShape,
    /// The run's refinements, which the refinement flow indexes.
    refinements: &'a [EdgeRefinement],
    /// Which places another actor may write.
    external: &'a dyn Fn(&str) -> bool,
}

/// Where a function's exception edges leave from, which decides what a
/// statement that certainly raises does to its block: inside a handler's
/// region the block's normal way out is closed and the exception edge carries
/// the state the statement left, and outside one nothing is claimed.
struct RegionShape {
    /// The blocks a handler's region holds the commands of: a source of an
    /// exception edge that is no region entry.
    throwing: HashSet<BlockId>,
    /// The region entries, by the block that holds the body's first command.
    entries: HashMap<BlockId, Vec<crate::cfg::RegionEntry>>,
    /// The region entries' edges.
    entry_edges: HashSet<(BlockId, BlockId)>,
}

impl RegionShape {
    fn of(cfg: &CfgFunction) -> Self {
        let entry_edges: HashSet<(BlockId, BlockId)> = cfg
            .region_entries
            .iter()
            .map(|entry| (entry.source, entry.handler))
            .collect();
        let throwing = cfg
            .exception_edges
            .iter()
            .filter(|edge| !entry_edges.contains(edge))
            .map(|&(from, _)| from)
            .collect();
        let mut entries: HashMap<BlockId, Vec<crate::cfg::RegionEntry>> = HashMap::new();
        for entry in &cfg.region_entries {
            entries.entry(entry.first).or_default().push(*entry);
        }
        Self {
            throwing,
            entries,
            entry_edges,
        }
    }
}

/// What a block's statements leave of its way out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BlockExit {
    /// Whether the block can complete normally: no statement of it is
    /// certain to raise.
    completes: bool,
    /// Whether the state before the block is one a throw can leave from:
    /// false where its first statement raises only after a store, so the
    /// handler never sees what held before it.
    entry_throws: bool,
}

impl BlockExit {
    const NORMAL: Self = Self {
        completes: true,
        entry_throws: true,
    };
}

/// What the sweeps advance: the lattice values, the executable blocks and
/// edges, and the existence rung when the run computes it.
struct SweepState {
    values: HashMap<ValueKey, LatticeValue>,
    executable_blocks: HashSet<BlockId>,
    executable_edges: HashSet<(BlockId, BlockId)>,
    existence: Option<ExistenceRun>,
    /// The refinements outside the existence domain as the sweep reads
    /// them, when the function has one.
    refined: Option<RefinementFlow>,
}

impl SweepContext<'_> {
    /// The optimistic fixpoint over the RPO sweep, followed by a finalising
    /// pass that forces both arms for any executable branch still stuck on an
    /// UNKNOWN condition (defensive: a value defined only in unreachable code
    /// could otherwise leave a successor spuriously unreachable).
    /// `finalizing` is monotone, so the outer loop runs at most twice.
    fn settle(&self, order: &[BlockId], state: &mut SweepState) {
        let mut finalizing = false;
        loop {
            let mut changed = true;
            while changed {
                changed = false;
                // Every sweep re-evaluates each executable statement from
                // scratch, so only the settled sweep's route entries — the
                // last one run — should reach the tally. Each sweep is one
                // iteration of the run's request: a tenth of what it has left.
                self.driver.reset_tally_for_sweep();
                self.driver.reset_undecided_for_sweep();
                self.driver.open_iteration();
                for bn in order {
                    if state.executable_blocks.contains(bn) {
                        changed |= self.block(*bn, state, finalizing);
                    }
                }
            }
            if finalizing {
                break;
            }
            finalizing = true;
        }
    }

    /// One executable block of a sweep: its φs, its statements, the
    /// existence rung through them, and its terminator's edges; whether
    /// anything moved.
    fn block(&self, bn: BlockId, state: &mut SweepState, finalizing: bool) -> bool {
        let Some(ssa_block) = self.ssa.blocks.get(&bn) else {
            return false;
        };
        let mut changed = false;
        let incoming_exec: Vec<BlockId> = self
            .preds
            .get(&bn)
            .map(|set| {
                set.iter()
                    .copied()
                    .filter(|p| state.executable_edges.contains(&(*p, bn)))
                    .collect()
            })
            .unwrap_or_default();

        // Phi nodes (not at entry, only when some predecessor is
        // executable).
        if bn != self.cfg.entry {
            changed |= sccp_process_phis(&mut state.values, ssa_block, &incoming_exec);
            record_phi_folded_types(&state.values, ssa_block, &incoming_exec, self.driver);
        }

        // The refinements every executable edge into the block carries
        // narrow the versions they name for its statements and its
        // terminator; the φs above read each version's own value.
        let narrowed = self.narrow(bn, state, &mut changed);

        // The existence rung enters the block with the join of its
        // executable edges, which is each φ's fact.
        let mut at = state.existence.as_mut().map(|run| {
            run.enter(
                self.cfg,
                (bn, ssa_block),
                &incoming_exec,
                self.driver,
                &mut changed,
            )
        });

        // Statements.
        let throwing = self.regions.throwing.contains(&bn);
        self.driver.set_throwing(throwing);
        let run = sccp_process_statements(
            &mut state.values,
            ssa_block,
            self.ssa,
            StatementInputs {
                block: bn,
                escaping: self.escaping,
                has_dynamic_variable_trace: self.has_dynamic_variable_trace,
                clobbers: self.ssa.value_clobbers.get(&bn),
                catches: Some(CatchBodies {
                    bodies: &self.cfg.opaque_catch_bodies,
                    external: self.external,
                }),
            },
            self.driver,
            at.as_mut(),
        );
        self.driver.set_throwing(false);
        changed |= run.changed;
        if let Some(at) = &at {
            at.before_terminator(self.driver);
        }

        // Terminator.
        let inputs = TerminatorInputs {
            cfg: self.cfg,
            ssa: self.ssa,
            values: &state.values,
            grammar: self.grammar,
            driver: self.driver,
            exit: run.exit,
            entry_edges: &self.regions.entry_edges,
        };
        changed |= sccp_process_terminator(
            bn,
            &inputs,
            &mut state.executable_blocks,
            &mut state.executable_edges,
            finalizing,
        );
        changed |= self.open_region_entries(
            bn,
            run.exit,
            (&mut state.executable_blocks, &mut state.executable_edges),
        );
        if let Some(at) = at {
            changed |= at.leave(self.driver, run.exit.entry_throws);
        }
        restore_values(&mut state.values, narrowed);
        changed
    }

    /// Enter `bn` in the refinement flow and narrow, in the lattice, each
    /// version a value-domain refinement in force there names; what each
    /// narrowed version held before, for [`restore_values`]. `changed` is
    /// set when the refinements the edges into `bn` carry moved.
    fn narrow(
        &self,
        bn: BlockId,
        state: &mut SweepState,
        changed: &mut bool,
    ) -> Vec<(ValueKey, Option<LatticeValue>)> {
        let Some(flow) = state.refined.as_mut() else {
            return Vec::new();
        };
        let (moved, in_force) = flow.enter(self.cfg, self.preds, bn, &state.executable_edges);
        *changed |= moved;
        narrow_values(&mut state.values, self.refinements, in_force)
    }

    /// The region entries whose body starts at `bn`: the state before the
    /// body reaches the handler when the body's first command can fail
    /// before it stores anything, and otherwise never.
    fn open_region_entries(
        &self,
        bn: BlockId,
        exit: BlockExit,
        (blocks, edges): (&mut HashSet<BlockId>, &mut HashSet<(BlockId, BlockId)>),
    ) -> bool {
        let mut changed = false;
        if !exit.entry_throws {
            return changed;
        }
        for entry in self.regions.entries.get(&bn).into_iter().flatten() {
            changed |= edges.insert((entry.source, entry.handler));
            if self.cfg.blocks.contains_key(&entry.handler) {
                changed |= blocks.insert(entry.handler);
            }
        }
        changed
    }
}

/// The existence rung at one block during a sweep: the run, the block, and
/// — for a block some handler region holds — the join of the points passed
/// so far.
struct ExistenceAt<'r> {
    run: &'r mut ExistenceRun,
    block: BlockId,
    /// The join of the points passed so far after the block's entry.
    through: Option<Vec<Existence>>,
    /// The state the block entered with, for a region block: a point a
    /// throw leaves from unless the block's first statement raises only
    /// after a store.
    entry: Option<Vec<Existence>>,
}

impl ExistenceAt<'_> {
    /// Record the facts the statement at `index` finds at each place it
    /// reads: the settled sweep's stay.
    fn record_reads(&mut self, index: usize, stmt_ssa: &SsaStatement, driver: &LatticeDriver<'_>) {
        let Ok(index) = u32::try_from(index) else {
            return;
        };
        for &symbol in stmt_ssa.uses.keys() {
            let fact = driver.existence_now(symbol).unwrap_or(Existence::Pending);
            self.run.reads.insert((self.block, index, symbol), fact);
        }
    }

    /// Join the current point into the block's points, for a region block.
    fn note_point(&mut self, driver: &LatticeDriver<'_>) {
        if let Some(points) = self.through.as_mut() {
            for (fact, symbol) in points.iter_mut().zip(0u32..) {
                if let Some(now) = driver.existence_now(Symbol(symbol)) {
                    *fact = fact.join(now);
                }
            }
        }
    }

    /// Apply the clobber the statement at `index` performs, after its own
    /// storage outcomes, and join the point into the block's points.
    fn finish_statement(&mut self, index: usize, driver: &LatticeDriver<'_>) {
        if let Some(clobber) = self.run.clobbers.get(&(self.block, index)) {
            driver.existence_clobber(|fact| clobber.applied(fact));
            for &symbol in &clobber.touched {
                driver.existence_step(
                    symbol,
                    crate::value_transfer::ExistenceStep::Set(Existence::MayBound),
                );
            }
        }
        self.note_point(driver);
    }

    /// A barrier or a static-body `uplevel` at `index`: every place is
    /// may-bound afterwards, its own definitions included; whether a
    /// version's fact moved.
    fn barrier(
        &mut self,
        index: usize,
        stmt_ssa: &SsaStatement,
        driver: &LatticeDriver<'_>,
    ) -> bool {
        self.finish_statement(index, driver);
        let mut changed = false;
        for (&var, &ver) in &stmt_ssa.defs {
            changed |= self.run.record_version((var, ver), Existence::MayBound);
        }
        changed
    }

    /// The step the definition `var` takes: an externally mutable place
    /// stays may-bound, a typed assignment binds, and any other statement's
    /// comes from its evaluation.
    fn step_for(
        &self,
        stmt_ssa: &SsaStatement,
        (var, name, element_write_base): (Symbol, &str, Option<Symbol>),
        driver: &LatticeDriver<'_>,
        (kept, evaluated): (bool, impl FnOnce() -> crate::value_transfer::ExistenceStep),
    ) -> crate::value_transfer::ExistenceStep {
        if self
            .run
            .mutable
            .get(var.0 as usize)
            .copied()
            .unwrap_or(true)
        {
            return crate::value_transfer::ExistenceStep::Set(Existence::MayBound);
        }
        // A statement that raised before it stored leaves a place as it was,
        // a typed assignment included.
        if kept {
            return evaluated();
        }
        let step = assignment_existence(stmt_ssa, var, element_write_base, driver)
            .unwrap_or_else(evaluated);
        // The registry's write class states what a call does to the places
        // it names where its declared transfer states less.
        driver
            .existence_by_write_class(&stmt_ssa.statement, name, step)
            .unwrap_or(step)
    }

    /// Advance the definition `key`'s place by `step` and record the fact
    /// its version is established with; whether it moved.
    fn advance(
        &mut self,
        key: ValueKey,
        step: crate::value_transfer::ExistenceStep,
        driver: &LatticeDriver<'_>,
    ) -> bool {
        let fact = driver
            .existence_step(key.0, step)
            .unwrap_or(Existence::Pending);
        self.run.record_version(key, fact)
    }

    /// Apply a computed name in the terminator's words, and the places a
    /// body nested in one of its substitutions may touch, before the branch
    /// reads its condition.
    fn before_terminator(&self, driver: &LatticeDriver<'_>) {
        if let Some(clobber) = self.run.terminator_clobbers.get(&self.block) {
            driver.existence_clobber(|fact| clobber.applied(fact));
            for &symbol in &clobber.touched {
                driver.existence_step(
                    symbol,
                    crate::value_transfer::ExistenceStep::Set(Existence::MayBound),
                );
            }
        }
    }

    /// Leave the block: record its exit, and for a region block the join
    /// of its points, its entry among them where a throw can leave from it
    /// (`entry_throws`); whether either moved.
    fn leave(self, driver: &LatticeDriver<'_>, entry_throws: bool) -> bool {
        let Some(exit) = driver.existence_leave() else {
            return false;
        };
        let through = self.through.map(|mut points| {
            if let Some(entry) = self.entry.as_deref().filter(|_| entry_throws) {
                join_state(&mut points, entry);
            }
            join_state(&mut points, &exit);
            points
        });
        self.run.finish_block(self.block, exit, through)
    }
}

/// A whole-frame effect on the existence rung that no storage outcome
/// states: a barrier's, a computed name's, or a nested body's.
#[derive(Debug, Clone, Default)]
struct Clobber {
    /// A barrier or a static-body `uplevel`: any place may have been
    /// written or destroyed, so every place is may-bound afterwards.
    all: bool,
    /// A computed name written: an unbound place may be bound afterwards.
    writes: bool,
    /// A computed name destroyed: a bound place may be unbound afterwards.
    destroys: bool,
    /// The places a nested body the statement keeps inline (a non-lowered
    /// `switch`'s arms) may write or destroy.
    touched: Vec<Symbol>,
}

impl Clobber {
    /// The clobber a statement's computed-name facts state.
    fn of_names(barrier: crate::dynamic_names::DynamicNameBarrier) -> Self {
        Self {
            writes: barrier.writes,
            destroys: barrier.destroys,
            ..Self::default()
        }
    }

    /// Whether the clobber changes nothing.
    fn is_empty(&self) -> bool {
        !self.all && !self.writes && !self.destroys && self.touched.is_empty()
    }

    /// The fact a place holds after the clobber, when it held `fact`.
    const fn applied(&self, fact: Existence) -> Existence {
        match fact {
            _ if self.all => Existence::MayBound,
            Existence::Unbound if self.writes => Existence::MayBound,
            Existence::Bound(_) if self.destroys => Existence::MayBound,
            other => other,
        }
    }
}

/// One run's existence rung (`docs/design/compiler/value-transfers.md`
/// § *Existence*): a forward fact per place, flowing through the same
/// executable blocks and edges the value lattice runs over. A block enters
/// with the join of its executable predecessors' exits — an exception edge
/// contributes every point of the region its handler covers, since any
/// command in it may throw — each statement's storage outcomes advance the
/// places it defines, and a barrier, a computed name or an inline nested
/// body clobbers what it may touch. The per-version facts, the facts each
/// statement reads, and each block's exit are the run's answer.
struct ExistenceRun {
    /// The fact each symbol holds on entry to the function.
    entry: Vec<Existence>,
    /// Whether each symbol's place is writable from outside the function —
    /// qualified, aliased, traced, or under a computed trace — and so
    /// may-bound wherever it is read.
    mutable: Vec<bool>,
    /// Whether each slot's place is externally mutable: a `mutable` one, or
    /// one linked to state another invocation, the object or the host
    /// holds ([`linked_elsewhere`]). Such a place is never refined, and an
    /// existence query about one decides nothing: whatever holds it can
    /// change it through any call, not only across a barrier.
    external: Vec<bool>,
    /// The clobber each statement performs, by `(block, index)`.
    clobbers: HashMap<(BlockId, usize), Clobber>,
    /// The clobber each block's terminator performs.
    terminator_clobbers: HashMap<BlockId, Clobber>,
    /// The exception edges that leave from the block before a body: they
    /// carry that block's exit, never the points inside it.
    entry_edges: HashSet<(BlockId, BlockId)>,
    /// Per handler block, the blocks whose every point its exception edges
    /// may leave from.
    handler_regions: HashMap<BlockId, Vec<BlockId>>,
    /// The blocks some handler region holds, whose inner points are kept.
    in_regions: HashSet<BlockId>,
    /// Each executable block's exit state.
    exits: HashMap<BlockId, Vec<Existence>>,
    /// The join of every point in each region block.
    through: HashMap<BlockId, Vec<Existence>>,
    /// The fact each SSA version is established with.
    versions: HashMap<ValueKey, Existence>,
    /// The fact each statement finds at each place it reads.
    reads: HashMap<(BlockId, u32, Symbol), Existence>,
    /// The slots past the SSA's symbols: each place only an existence
    /// query names, which the run carries like any other.
    query_only: HashMap<String, Symbol>,
    /// The guarded edges' refinements, until the run takes them
    /// ([`ordered_refinements`]); the sweep reads `by_edge`.
    refinements: Vec<EdgeRefinement>,
    /// The refinements by edge: each place's slot and its refined fact.
    by_edge: HashMap<(BlockId, BlockId), Vec<(usize, Existence)>>,
    /// Each existence guard of the function: a place a guarded edge proves
    /// bound, and the block the edge enters ([`SccpResult::existence_guards`]).
    guards: Vec<(String, BlockId)>,
    /// Each executable block's entry state.
    entry_states: HashMap<BlockId, Vec<Existence>>,
}

/// What the existence rung leaves in the run's result: the fact each SSA
/// version is established with, the fact each statement finds at each place
/// it reads, each block's exit, the block-qualified facts, the places only
/// an existence query names and the existence guards. Its guarded edges'
/// refinements join the run's before the sweep ([`ordered_refinements`]).
#[derive(Default)]
struct RungResults {
    versions: HashMap<ValueKey, Existence>,
    reads: HashMap<(BlockId, u32, Symbol), Existence>,
    exits: HashMap<BlockId, Vec<Existence>>,
    entries: HashMap<(BlockId, ValueKey), Existence>,
    query_places: Vec<String>,
    /// The existence guards ([`SccpResult::existence_guards`]), which the
    /// rung's run collects, so they exist only where the rung runs: the deep
    /// tier (`AnalysisTier::Deep`). A run without the rung leaves none, and
    /// W210 then has no guard's word — a tier below the deep one that keeps
    /// the dataflow diagnostics must collect them itself
    /// (`existence_guards`) or lose every `info exists` guard.
    guards: Vec<(String, BlockId)>,
}

/// Join `incoming` into `state`, place by place.
fn join_state(state: &mut [Existence], incoming: &[Existence]) {
    for (fact, other) in state.iter_mut().zip(incoming) {
        *fact = fact.join(*other);
    }
}

/// Join `state` into what `states` holds for `block`; whether it moved.
fn join_into(
    states: &mut HashMap<BlockId, Vec<Existence>>,
    block: BlockId,
    state: Vec<Existence>,
) -> bool {
    if let Some(old) = states.get_mut(&block) {
        let before = old.clone();
        join_state(old, &state);
        return *old != before;
    }
    states.insert(block, state);
    true
}

impl ExistenceRun {
    /// The run for `cfg` / `ssa` entered under `entry`; `escaping` is the
    /// value lattice's externally-mutable set, and `facts` what each branch
    /// edge's condition proves, of which the run takes the existence facts.
    fn new(
        cfg: &CfgFunction,
        ssa: &SsaFunction,
        entry: ExistenceEntry<'_>,
        (escaping, facts): (&HashSet<String>, &[BranchFact]),
        registry: &CommandRegistry,
    ) -> Self {
        // The SSA's symbols, then a slot per place only an existence query
        // names, in that order, so a symbol's slot is its index.
        let symbols = ssa.var_names().len();
        let extra = query_only_places(cfg, ssa, registry, entry.config);
        let mut names: Vec<String> = ssa.var_names().to_vec();
        names.extend(extra.iter().cloned());
        let query_only: HashMap<String, Symbol> = extra
            .into_iter()
            .zip(symbols..)
            .filter_map(|(name, slot)| Some((name, Symbol(u32::try_from(slot).ok()?))))
            .collect();
        let mutable: Vec<bool> = names
            .iter()
            .map(|name| {
                entry.dynamic_trace
                    || name.contains("::")
                    || escaping.contains(name.as_str())
                    || escaping.contains(place_base(name))
            })
            .collect();
        let dialect = Some(tcl_registry::special_vars::surface_query_for_profile(
            registry.profile(),
        ));
        let external: Vec<bool> = names
            .iter()
            .zip(&mutable)
            .map(|(name, &mutable)| mutable || linked_elsewhere(name, entry, registry, dialect))
            .collect();
        let caller_frame = Clobber::of_names(cfg.caller_frame_barrier);
        let entry_state: Vec<Existence> = names
            .iter()
            .zip(&mutable)
            .map(|(name, &mutable)| {
                let fact = if mutable {
                    Existence::MayBound
                } else {
                    entry_fact(name, entry, registry, dialect)
                };
                caller_frame.applied(fact)
            })
            .collect();
        let mut clobbers = HashMap::new();
        let mut terminator_clobbers = HashMap::new();
        for (&block_id, block) in &cfg.blocks {
            for (index, statement) in block.statements.iter().enumerate() {
                let clobber = statement_clobber(statement, ssa, cfg, registry, entry.config);
                if !clobber.is_empty() {
                    clobbers.insert((block_id, index), clobber);
                }
            }
            if let Some(terminator) = &block.terminator {
                let clobber = terminator_clobber(terminator, ssa, cfg, registry, entry.config);
                if !clobber.is_empty() {
                    terminator_clobbers.insert(block_id, clobber);
                }
            }
        }
        let refinements = existence_refinements(facts, ssa, (&query_only, &external));
        let guards = existence_guards(facts);
        let mut by_edge: HashMap<(BlockId, BlockId), Vec<(usize, Existence)>> = HashMap::new();
        for refinement in &refinements {
            if let FactView::Domain(DomainFact::Existence(fact)) = &refinement.fact {
                by_edge
                    .entry(refinement.edge)
                    .or_default()
                    .push((refinement.key.0.0 as usize, *fact));
            }
        }
        let entry_edges: HashSet<(BlockId, BlockId)> = cfg
            .region_entries
            .iter()
            .map(|entry| (entry.source, entry.handler))
            .collect();
        let handler_regions = handler_regions(cfg, &entry_edges);
        let in_regions = handler_regions.values().flatten().copied().collect();
        // Version 0 of each SSA place is what the frame enters with; a
        // query-only slot has no version.
        let versions = (0u32..)
            .zip(entry_state.iter().take(symbols))
            .map(|(symbol, fact)| ((Symbol(symbol), 0), *fact))
            .collect();
        Self {
            entry: entry_state,
            mutable,
            external,
            clobbers,
            terminator_clobbers,
            entry_edges,
            handler_regions,
            in_regions,
            exits: HashMap::new(),
            through: HashMap::new(),
            versions,
            reads: HashMap::new(),
            query_only,
            refinements,
            by_edge,
            guards,
            entry_states: HashMap::new(),
        }
    }

    /// What the run leaves in the result once the solver settles.
    fn finish(self, ssa: &SsaFunction) -> RungResults {
        RungResults {
            entries: self.block_qualified(ssa),
            query_places: self.query_places(),
            versions: self.versions,
            reads: self.reads,
            exits: self.exits,
            guards: self.guards,
        }
    }

    /// The places only an existence query names, by their slot past the
    /// SSA's symbols.
    fn query_places(&self) -> Vec<String> {
        let mut places: Vec<(&String, Symbol)> = self
            .query_only
            .iter()
            .map(|(name, &slot)| (name, slot))
            .collect();
        places.sort_by_key(|&(_, slot)| slot);
        places.into_iter().map(|(name, _)| name.clone()).collect()
    }

    /// Enter `block` for a sweep: its entry state, which each φ takes, at
    /// the driver's cursor; `changed` is set when a φ's fact moved.
    fn enter<'r>(
        &'r mut self,
        cfg: &CfgFunction,
        (block, ssa_block): (BlockId, &crate::ssa::SsaBlock),
        incoming: &[BlockId],
        driver: &LatticeDriver<'_>,
        changed: &mut bool,
    ) -> ExistenceAt<'r> {
        let state = self.block_entry(cfg, block, incoming);
        self.entry_states.insert(block, state.clone());
        for phi in &ssa_block.phis {
            let fact = state
                .get(phi.name.0 as usize)
                .copied()
                .unwrap_or(Existence::Pending);
            *changed |= self.record_version((phi.name, phi.version), fact);
        }
        let in_region = self.in_regions.contains(&block);
        let through = in_region.then(|| vec![Existence::Pending; state.len()]);
        let entry = in_region.then(|| state.clone());
        driver.existence_enter(state);
        ExistenceAt {
            run: self,
            block,
            through,
            entry,
        }
    }

    /// The state `block` enters with: the entry rules for the function's
    /// entry, joined with every executable incoming edge's contribution —
    /// a predecessor's exit, or across an exception edge every point of the
    /// handler's region. A predecessor the sweep has not reached yet
    /// contributes nothing.
    fn block_entry(
        &self,
        cfg: &CfgFunction,
        block: BlockId,
        incoming: &[BlockId],
    ) -> Vec<Existence> {
        let mut state = if block == cfg.entry {
            self.entry.clone()
        } else {
            vec![Existence::Pending; self.entry.len()]
        };
        for &pred in incoming {
            let normal = cfg
                .blocks
                .get(&pred)
                .is_some_and(|b| b.successors().contains(&block));
            if normal && let Some(exit) = self.exits.get(&pred) {
                join_state(&mut state, &self.arriving((pred, block), exit));
            }
            if self.entry_edges.contains(&(pred, block)) {
                // The state before the body is the block before it, at its
                // exit.
                if let Some(exit) = self.exits.get(&pred) {
                    join_state(&mut state, exit);
                }
            } else if cfg.exception_edges.contains(&(pred, block)) {
                for region in self.handler_regions.get(&block).into_iter().flatten() {
                    if let Some(points) = self.through.get(region) {
                        join_state(&mut state, points);
                    }
                }
            }
        }
        state
    }

    /// `exit`, the state a predecessor leaves with, as it arrives along
    /// `edge`: each refinement of the edge narrows its place.
    fn arriving<'e>(
        &self,
        edge: (BlockId, BlockId),
        exit: &'e [Existence],
    ) -> std::borrow::Cow<'e, [Existence]> {
        let Some(facts) = self.by_edge.get(&edge) else {
            return std::borrow::Cow::Borrowed(exit);
        };
        let mut state = exit.to_vec();
        for &(slot, fact) in facts {
            if let Some(place) = state.get_mut(slot) {
                *place = narrowed(*place, fact);
            }
        }
        std::borrow::Cow::Owned(state)
    }

    /// The block-qualified facts: for each executable block and the version
    /// of each place live at its entry, the fact the place holds there
    /// where it differs from the fact the version was established with. A
    /// slot past the SSA's symbols has no version, so it has none.
    fn block_qualified(&self, ssa: &SsaFunction) -> HashMap<(BlockId, ValueKey), Existence> {
        let mut out = HashMap::new();
        for (&block, state) in &self.entry_states {
            let live = ssa
                .blocks
                .get(&block)
                .map(|ssa_block| &ssa_block.entry_versions);
            for (slot, &here) in (0u32..).zip(state) {
                let symbol = Symbol(slot);
                let version = live
                    .and_then(|versions| versions.get(&symbol))
                    .copied()
                    .unwrap_or(0);
                let key = (symbol, version);
                if here != Existence::Pending
                    && self.versions.get(&key).is_some_and(|&own| own != here)
                {
                    out.insert((block, key), here);
                }
            }
        }
        out
    }

    /// Record `fact` for `key`, joined with what an earlier sweep recorded;
    /// whether the record moved.
    fn record_version(&mut self, key: ValueKey, fact: Existence) -> bool {
        let old = self
            .versions
            .get(&key)
            .copied()
            .unwrap_or(Existence::Pending);
        let joined = old.join(fact);
        if joined == old && self.versions.contains_key(&key) {
            return false;
        }
        self.versions.insert(key, joined);
        true
    }

    /// Record a block's exit and, for a region block, the join of its
    /// points; whether either moved.
    fn finish_block(
        &mut self,
        block: BlockId,
        exit: Vec<Existence>,
        through: Option<Vec<Existence>>,
    ) -> bool {
        let mut changed = join_into(&mut self.exits, block, exit);
        if let Some(points) = through {
            changed |= join_into(&mut self.through, block, points);
        }
        changed
    }
}

/// The fact a refinement to `fact` leaves at a place that holds `current`:
/// `fact` where it narrows the place — a may-bound place to anything, a
/// place bound as either kind to one kind — and `current` otherwise. A
/// refinement the place contradicts rides an edge the query did not decide
/// although the place has a fact, as for a special variable the host binds,
/// so the place keeps its fact.
const fn narrowed(current: Existence, fact: Existence) -> Existence {
    match (current, fact) {
        (Existence::MayBound, _) | (Existence::Bound(BindingKind::Either), Existence::Bound(_)) => {
            fact
        }
        _ => current,
    }
}

/// One branch edge's fact about one place, from its condition's
/// `Selection` transfer ([`crate::tcl_expr_eval::condition_edge_facts`]).
struct BranchFact {
    /// The branch block and the block the edge enters.
    edge: (BlockId, BlockId),
    /// What the edge proves.
    fact: crate::tcl_expr_eval::PlaceFact,
}

/// What every branch edge of the function proves: the true and the false
/// edge of each branch whose targets differ, each condition read against
/// `registry` under the document's lexer configuration and the target's
/// fold policy, with the commands `driver` trusts.
fn branch_facts(
    cfg: &CfgFunction,
    (registry, config, policy): (&CommandRegistry, tcl_lexer::LexerConfig, FoldPolicy),
    driver: &LatticeDriver<'_>,
) -> Vec<BranchFact> {
    let trusted = |head: &str| driver.trusted(head);
    let mut out = Vec::new();
    for (&block_id, block) in &cfg.blocks {
        let Some(Terminator::Branch {
            condition,
            true_target,
            false_target,
            condition_base,
            ..
        }) = &block.terminator
        else {
            continue;
        };
        if true_target == false_target {
            continue;
        }
        let reading = crate::tcl_expr_eval::ConditionReading {
            registry,
            config,
            policy,
            trusted: &trusted,
            existence: crate::existence_query::QueryReading::for_cfg(
                cfg,
                registry,
                config,
                cfg.source_tokens_at(block_id, usize::MAX),
                *condition_base,
            ),
        };
        let facts = crate::tcl_expr_eval::condition_edge_facts(condition, reading);
        for (target, facts) in [
            (*true_target, facts.on_true),
            (*false_target, facts.on_false),
        ] {
            out.extend(facts.into_iter().map(|fact| BranchFact {
                edge: (block_id, target),
                fact,
            }));
        }
    }
    out
}

/// The version of `symbol` `block` leaves: its exit version, or the entry's
/// for a place the function never defines.
fn exit_version(ssa: &SsaFunction, block: BlockId, symbol: Symbol) -> crate::ssa::Version {
    ssa.blocks
        .get(&block)
        .and_then(|ssa_block| ssa_block.exit_versions.get(&symbol))
        .copied()
        .unwrap_or(0)
}

/// The existence refinements of the function's guarded edges
/// ([`EdgeRefinement`]): each branch edge's existence fact about a place the
/// rung carries refines the place on that edge. An externally mutable place
/// (`external`, by slot) is never refined: a plain call to a procedure the
/// module cannot see, or to a computed head, may write or unset a global, an
/// alias, an instance variable or a connection's name without any barrier or
/// up-frame, so a refinement there would outlive the point at which another
/// actor acts.
fn existence_refinements(
    facts: &[BranchFact],
    ssa: &SsaFunction,
    (query_only, external): (&HashMap<String, Symbol>, &[bool]),
) -> Vec<EdgeRefinement> {
    let mut out = Vec::new();
    for BranchFact { edge, fact } in facts {
        if fact.domain != FactDomain::Existence {
            continue;
        }
        let Some(symbol) = ssa
            .var_symbol(&fact.place)
            .or_else(|| query_only.get(&fact.place).copied())
        else {
            continue;
        };
        if external.get(symbol.0 as usize).copied().unwrap_or(true) {
            continue;
        }
        out.push(EdgeRefinement {
            edge: *edge,
            key: (symbol, exit_version(ssa, edge.0, symbol)),
            domain: fact.domain,
            fact: fact.fact.clone(),
            evidence: DependencyEvidence::default(),
            loop_exit: false,
        });
    }
    out.sort_by_key(|refinement| (refinement.edge.0.0, refinement.edge.1.0, refinement.key.0.0));
    out
}

/// Every place a guarded edge proves bound, with the block the edge enters,
/// whether or not the run refines the place: W210 takes a guard's word for
/// the region its edge enters ([`SccpResult::guarded`]).
fn existence_guards(facts: &[BranchFact]) -> Vec<(String, BlockId)> {
    facts
        .iter()
        .filter(|branch| {
            matches!(
                branch.fact.fact,
                FactView::Domain(DomainFact::Existence(Existence::Bound(_)))
            )
        })
        .map(|branch| (branch.fact.place.clone(), branch.edge.1))
        .collect()
}

/// Whether another actor may write a place, so that no refinement narrows
/// it and no enumerated loop writes it: one externally mutable
/// ([`is_externally_mutable`]) or an element of an escaping array, one
/// linked to state another invocation, the object or the host holds
/// ([`linked_elsewhere`], named where the run computes existence), and every
/// place once a computed name anywhere in the function may write or destroy
/// any.
fn external_places<'a>(
    (cfg, trace): (&'a CfgFunction, &'a TraceInputs<'a>),
    (escaping, config): (&'a HashSet<String>, tcl_lexer::LexerConfig),
) -> impl Fn(&str) -> bool + 'a {
    let dialect = Some(tcl_registry::special_vars::surface_query_for_profile(
        trace.registry.profile(),
    ));
    let computed_names = std::cell::OnceCell::new();
    move |name: &str| {
        is_externally_mutable(name, escaping, trace.has_dynamic_variable_trace)
            || escaping.contains(place_base(name))
            || trace
                .existence
                .is_some_and(|entry| linked_elsewhere(name, entry, trace.registry, dialect))
            || *computed_names.get_or_init(|| {
                let names = crate::dynamic_names::dynamic_name_barrier(cfg, trace.registry, config);
                names.writes || names.destroys
            })
    }
}

/// Run each loop whose start state the settled run proves to its exit
/// ([`crate::static_loops::enumerate_loop`]), and state what it leaves as an
/// exact-value refinement on every executable edge into the block it leaves
/// to, for the version of each place it wrote that is live there and that
/// the loop defines — a φ there, or a definition in its blocks — never the
/// version live where it starts: the loops run, and the refinements their
/// exits publish. A name the loop writes that is dead after it has no φ
/// where it leaves, so the version live there is the one from before the
/// loop, which its state does not describe. A loop that leaves with a
/// completion its plan does not absorb publishes nothing, since what it
/// leaves holds on that path alone, and one that declines is not run.
fn enumerate_loops(
    (cfg, ssa, preds): (
        &CfgFunction,
        &SsaFunction,
        &HashMap<BlockId, HashSet<BlockId>>,
    ),
    (values, narrowing): (&mut HashMap<ValueKey, LatticeValue>, Narrowing<'_>),
    (executable, edges): (&HashSet<BlockId>, &HashSet<(BlockId, BlockId)>),
    existence_exits: Option<&HashMap<BlockId, Vec<Existence>>>,
    (driver, external): (&LatticeDriver<'_>, &dyn Fn(&str) -> bool),
) -> (Vec<EnumeratedLoop>, Vec<EdgeRefinement>) {
    let mut loops: Vec<(BlockId, &crate::cfg::LoopNode)> = cfg
        .loop_nodes
        .iter()
        .map(|(&end, node)| (end, node))
        .filter(|(end, node)| executable.contains(end) && executable.contains(&node.start))
        .collect();
    loops.sort_by_key(|(end, _)| end.0);
    let mut records = Vec::new();
    let mut published = Vec::new();
    for (end, node) in loops {
        let narrowed = narrowing.narrow(values, node.start);
        let state = start_state(
            ssa,
            node.start,
            values,
            existence_exits.and_then(|exits| exits.get(&node.start)),
        );
        restore_values(values, narrowed);
        let Ok(run) = crate::static_loops::enumerate_loop(
            driver,
            &node.statement,
            state,
            (true, crate::static_loops::DEFAULT_MAX_STATIC_LOOP_ITERS),
            external,
        ) else {
            continue;
        };
        let mut left = Vec::new();
        if run.completion == tcl_registry::value_transfer::CompletionOutcome::Normal
            && let Some(entry) = ssa.blocks.get(&end).map(|block| &block.entry_versions)
        {
            let before = ssa
                .blocks
                .get(&node.start)
                .map(|block| &block.exit_versions);
            for (name, slot) in &run.state {
                let crate::static_loops::Slot::Value(value) = slot else {
                    continue;
                };
                let Some(symbol) = ssa.var_symbol(name) else {
                    continue;
                };
                let Some(&version) = entry.get(&symbol) else {
                    continue;
                };
                let live_before = before
                    .and_then(|versions| versions.get(&symbol))
                    .copied()
                    .unwrap_or(0);
                if version == live_before {
                    continue;
                }
                for &pred in preds.get(&end).into_iter().flatten() {
                    if edges.contains(&(pred, end)) {
                        published.push(EdgeRefinement {
                            edge: (pred, end),
                            key: (symbol, version),
                            domain: FactDomain::ExactValue,
                            fact: FactView::Exact(value.clone(), None),
                            evidence: DependencyEvidence::default(),
                            loop_exit: true,
                        });
                    }
                }
                left.push((
                    name.clone(),
                    LatticeValue::Const(crate::value_transfer::exact_to_const(value)),
                ));
            }
        }
        records.push(EnumeratedLoop {
            exit_block: end,
            span: node.span,
            iterations: run.iterations,
            exit: run.exit,
            published: left,
        });
    }
    (records, published)
}

/// The exact state a loop starts from: what the block `start` leaves in each
/// place — the value the settled lattice proves exact at its exit, under the
/// refinements in force there, a scalar of unknown value where the existence
/// rung proves it bound as one, and unbound where the rung proves it so. A
/// place the run proves none of is not held, and a read of it or a store to
/// it declines the enumeration.
fn start_state(
    ssa: &SsaFunction,
    start: BlockId,
    values: &HashMap<ValueKey, LatticeValue>,
    exits: Option<&Vec<Existence>>,
) -> crate::static_loops::LoopState {
    use crate::static_loops::Slot;
    let mut state = crate::static_loops::LoopState::default();
    let Some(block) = ssa.blocks.get(&start) else {
        return state;
    };
    for (slot, name) in ssa.var_names().iter().enumerate() {
        let Ok(index) = u32::try_from(slot) else {
            continue;
        };
        let symbol = Symbol(index);
        let version = block.exit_versions.get(&symbol).copied().unwrap_or(0);
        match values.get(&(symbol, version)) {
            Some(LatticeValue::Const(constant)) => state.set(
                name.clone(),
                Slot::Value(crate::value_transfer::const_to_exact(constant)),
            ),
            _ => {
                if let Some(slot) = exits
                    .and_then(|facts| facts.get(slot))
                    .copied()
                    .and_then(slot_of_existence)
                {
                    state.set(name.clone(), slot);
                }
            }
        }
    }
    state
}

/// What a loop enumeration holds of a place whose value the lattice does not
/// prove exact, from the existence rung's fact: a scalar of unknown value
/// where it is proven bound as one, unbound where it is proven so, and
/// nothing otherwise.
fn slot_of_existence(fact: Existence) -> Option<crate::static_loops::Slot> {
    use crate::static_loops::Slot;
    match fact {
        Existence::Bound(BindingKind::Scalar) => Some(Slot::Scalar),
        Existence::Unbound => Some(Slot::Unbound),
        _ => None,
    }
}

/// Every other domain's refinements: each branch edge's fact about a
/// variable the SSA holds and that is not `external` — no other actor may
/// write it — narrowing the version the branch block leaves it at.
fn value_refinements(
    facts: &[BranchFact],
    ssa: &SsaFunction,
    external: &dyn Fn(&str) -> bool,
) -> Vec<EdgeRefinement> {
    facts
        .iter()
        .filter(|branch| branch.fact.domain != FactDomain::Existence)
        .filter_map(|BranchFact { edge, fact }| {
            let symbol = ssa.var_symbol(&fact.place)?;
            if external(&fact.place) {
                return None;
            }
            Some(EdgeRefinement {
                edge: *edge,
                key: (symbol, exit_version(ssa, edge.0, symbol)),
                domain: fact.domain,
                fact: fact.fact.clone(),
                evidence: DependencyEvidence::default(),
                loop_exit: false,
            })
        })
        .collect()
}

/// Every refinement of the run in one order ([`refinement_order`]): the
/// existence rung's, which it hands over (its sweep reads its own by edge),
/// and every other domain's.
fn ordered_refinements(
    rung: Option<&mut ExistenceRun>,
    others: Vec<EdgeRefinement>,
) -> Vec<EdgeRefinement> {
    let mut refinements = rung.map_or_else(Vec::new, |run| std::mem::take(&mut run.refinements));
    refinements.extend(others);
    refinements.sort_by_key(refinement_order);
    refinements
}

/// The order refinements are kept in: by edge, then place, then domain.
fn refinement_order(refinement: &EdgeRefinement) -> (u32, u32, u32, u8) {
    let rank = match refinement.domain {
        FactDomain::Existence => 0,
        FactDomain::ExactValue => 1,
        FactDomain::Type => 2,
        FactDomain::Range => 3,
        _ => 4,
    };
    (
        refinement.edge.0.0,
        refinement.edge.1.0,
        refinement.key.0.0,
        rank,
    )
}

/// The refinements outside the existence domain as the solver reads them
/// while it sweeps (`docs/design/compiler/value-transfers.md` § *Predicate
/// refinement*): which each branch edge takes, which hold at each block it
/// has entered, and the blocks that hold none.
struct RefinementFlow {
    /// By edge, the indices into the run's refinements of those the edge
    /// takes.
    by_edge: HashMap<(BlockId, BlockId), Vec<usize>>,
    /// The blocks holding a barrier or an up-frame, which may write any
    /// variable without a new version: they hold none and pass none on.
    overriding: HashSet<BlockId>,
    /// Per block the solver has entered, the refinements every executable
    /// edge into it carried when it last did.
    held: HashMap<BlockId, BTreeSet<usize>>,
}

impl RefinementFlow {
    /// The flow over `refinements`, or none when no refinement outside the
    /// existence domain exists. Refinements stating one fact about one
    /// version on several edges — the state an enumerated loop leaves, on
    /// every edge it may leave by — are one fact, so a block every one of
    /// those edges enters holds it; a loop's state and a branch's fact stay
    /// two, since they narrow differently ([`refined_value`]).
    fn new(cfg: &CfgFunction, refinements: &[EdgeRefinement]) -> Option<Self> {
        let mut by_edge: HashMap<(BlockId, BlockId), Vec<usize>> = HashMap::new();
        let mut alike: HashMap<(ValueKey, FactDomain, bool), Vec<usize>> = HashMap::new();
        for (index, refinement) in refinements.iter().enumerate() {
            if refinement.domain == FactDomain::Existence {
                continue;
            }
            let same = alike
                .entry((refinement.key, refinement.domain, refinement.loop_exit))
                .or_default();
            let earlier = same
                .iter()
                .copied()
                .find(|&earlier| refinements[earlier].fact == refinement.fact);
            let fact = earlier.unwrap_or_else(|| {
                same.push(index);
                index
            });
            by_edge.entry(refinement.edge).or_default().push(fact);
        }
        if by_edge.is_empty() {
            return None;
        }
        let overriding = cfg
            .blocks
            .iter()
            .filter(|(_, block)| {
                block.statements.iter().any(|statement| {
                    matches!(
                        statement,
                        Statement::Barrier { .. } | Statement::UpFrame { .. }
                    )
                })
            })
            .map(|(&id, _)| id)
            .collect();
        Some(Self {
            by_edge,
            overriding,
            held: HashMap::new(),
        })
    }

    /// Enter `block`: the refinements every executable edge into it carries
    /// now — a branch edge that takes one, or an edge from a block where it
    /// holds; an exception edge carries none, and a predecessor not entered
    /// yet carries every one, the greatest fixed point the edges into a
    /// cycle narrow — recorded, with whether that moved, and those in force
    /// at its statements and terminator, none where the block overrides.
    fn enter(
        &mut self,
        cfg: &CfgFunction,
        preds: &HashMap<BlockId, HashSet<BlockId>>,
        block: BlockId,
        edges: &HashSet<(BlockId, BlockId)>,
    ) -> (bool, BTreeSet<usize>) {
        let mut entering: Option<BTreeSet<usize>> = (block == cfg.entry).then(BTreeSet::new);
        for &pred in preds.get(&block).into_iter().flatten() {
            if !edges.contains(&(pred, block)) {
                continue;
            }
            let carried = if cfg.exception_edges.contains(&(pred, block)) {
                Some(BTreeSet::new())
            } else {
                self.held.get(&pred).map(|before| {
                    let mut carried = if self.overriding.contains(&pred) {
                        BTreeSet::new()
                    } else {
                        before.clone()
                    };
                    carried.extend(self.by_edge.get(&(pred, block)).into_iter().flatten());
                    carried
                })
            };
            entering = match (entering, carried) {
                (Some(entering), Some(carried)) => {
                    Some(entering.intersection(&carried).copied().collect())
                }
                (entering, carried) => entering.or(carried),
            };
        }
        let entering = entering.unwrap_or_default();
        let moved = self.held.get(&block) != Some(&entering);
        let in_force = if self.overriding.contains(&block) {
            BTreeSet::new()
        } else {
            entering.clone()
        };
        self.held.insert(block, entering);
        (moved, in_force)
    }

    /// Per block, the refinements in force there once the solver settles.
    fn settled(self) -> HashMap<BlockId, Vec<usize>> {
        let overriding = self.overriding;
        self.held
            .into_iter()
            .filter(|(block, held)| !held.is_empty() && !overriding.contains(block))
            .map(|(block, held)| (block, held.into_iter().collect()))
            .collect()
    }
}

/// The refinements outside the existence domain in force at each block once
/// the solver settles, with the run's refinements they index.
#[derive(Clone, Copy)]
struct Narrowing<'a> {
    refinements: &'a [EdgeRefinement],
    at: &'a HashMap<BlockId, Vec<usize>>,
}

impl<'a> Narrowing<'a> {
    /// The narrowing `at` names, over the run's `refinements`.
    fn new(refinements: &'a [EdgeRefinement], at: &'a HashMap<BlockId, Vec<usize>>) -> Self {
        Self { refinements, at }
    }

    /// Narrow, in `values`, each version a value-domain refinement in force
    /// at `block` names ([`narrow_values`]).
    fn narrow(
        self,
        values: &mut HashMap<ValueKey, LatticeValue>,
        block: BlockId,
    ) -> Vec<(ValueKey, Option<LatticeValue>)> {
        narrow_values(
            values,
            self.refinements,
            self.at.get(&block).into_iter().flatten().copied(),
        )
    }
}

/// Narrow, in `values`, each version a value-domain refinement of
/// `in_force` names to what it holds where every such refinement holds
/// ([`refined_value`]); what each narrowed version held before, for
/// [`restore_values`]. A refinement never makes a version: it overrides the
/// version's value for the statements of one block, and the version's own
/// comes back after them.
fn narrow_values(
    values: &mut HashMap<ValueKey, LatticeValue>,
    refinements: &[EdgeRefinement],
    in_force: impl IntoIterator<Item = usize>,
) -> Vec<(ValueKey, Option<LatticeValue>)> {
    let mut by_key: HashMap<ValueKey, Vec<&EdgeRefinement>> = HashMap::new();
    for refinement in in_force
        .into_iter()
        .filter_map(|index| refinements.get(index))
    {
        if refinement.domain == FactDomain::ExactValue {
            by_key.entry(refinement.key).or_default().push(refinement);
        }
    }
    let mut narrowed = Vec::new();
    for (key, facts) in by_key {
        let own = held_value(values, key);
        let refined = refined_value(&own, facts);
        if refined != own {
            narrowed.push((key, values.insert(key, refined)));
        }
    }
    narrowed
}

/// Give back each version [`narrow_values`] narrowed what it held before.
fn restore_values(
    values: &mut HashMap<ValueKey, LatticeValue>,
    narrowed: Vec<(ValueKey, Option<LatticeValue>)>,
) {
    for (key, before) in narrowed {
        match before {
            Some(value) => {
                values.insert(key, value);
            }
            None => {
                values.remove(&key);
            }
        }
    }
}

/// The block-qualified values ([`SccpResult::value_entries`]): at each block,
/// each version a value-domain refinement in force there narrows
/// ([`refined_value`]), where the narrowed value differs from its own.
fn refined_values(
    refinements: &[EdgeRefinement],
    refinements_at: &HashMap<BlockId, Vec<usize>>,
    values: &HashMap<ValueKey, LatticeValue>,
) -> HashMap<(BlockId, ValueKey), LatticeValue> {
    let mut out = HashMap::new();
    for (&block, indices) in refinements_at {
        let mut by_key: HashMap<ValueKey, Vec<&EdgeRefinement>> = HashMap::new();
        for refinement in indices.iter().filter_map(|&index| refinements.get(index)) {
            if refinement.domain == FactDomain::ExactValue {
                by_key.entry(refinement.key).or_default().push(refinement);
            }
        }
        for (key, facts) in by_key {
            let own = held_value(values, key);
            let refined = refined_value(&own, facts);
            if refined != own {
                out.insert((block, key), refined);
            }
        }
    }
    out
}

/// The blocks the loops whose exit state the settled values contradict leave
/// to: a version a loop's state narrows, at a block where that state is in
/// force, settled on a value the state rules out. The enumeration and the
/// lattice disagree about what that loop leaves, and neither is read; every
/// other loop's state stands.
fn contradicted_exits(
    refinements: &[EdgeRefinement],
    refinements_at: &HashMap<BlockId, Vec<usize>>,
    values: &HashMap<ValueKey, LatticeValue>,
) -> HashSet<BlockId> {
    refinements_at
        .values()
        .flatten()
        .filter_map(|&index| refinements.get(index))
        .filter(|refinement| refinement.loop_exit && refinement.domain == FactDomain::ExactValue)
        .filter(|refinement| {
            let own = held_value(values, refinement.key);
            own != LatticeValue::Unknown
                && refined_value(&own, [*refinement]) == LatticeValue::Unknown
        })
        .map(|refinement| refinement.edge.1)
        .collect()
}

/// What a version holding `own` holds where every value-domain refinement of
/// `facts` holds: the members of its finite set the facts allow, or, for a
/// value the lattice cannot pin, the facts' own exact value or finite set.
/// A constant stays itself, as does a set the facts leave no member of — a
/// branch the solver could not decide although its operand is known —
/// and a version the solver has not reached stays the optimistic bottom.
/// The state an enumerated loop leaves holds of the version's settled value
/// exactly, so a constant or a set of its own the fact rules out is a value
/// the solver has not settled yet, and the version stays the optimistic
/// bottom until it has: the narrowed value then only descends, as the
/// solver's values do, and no branch decides on the unsettled one.
fn refined_value<'f>(
    own: &LatticeValue,
    facts: impl IntoIterator<Item = &'f EdgeRefinement>,
) -> LatticeValue {
    let mut allowed: Option<Vec<&'f ExactValue>> = None;
    let mut loop_exit = false;
    for refinement in facts {
        loop_exit |= refinement.loop_exit;
        let members: Vec<&ExactValue> = match &refinement.fact {
            FactView::Exact(value, _) => vec![value],
            FactView::Finite(values, _) => values.iter().collect(),
            _ => continue,
        };
        allowed = Some(match allowed {
            None => members,
            Some(before) => before
                .into_iter()
                .filter(|kept| members.iter().any(|member| member.bytes == kept.bytes))
                .collect(),
        });
    }
    let Some(allowed) = allowed else {
        return own.clone();
    };
    let permits = |constant: &ConstValue| {
        let text = crate::value_transfer::const_to_exact(constant).bytes;
        allowed.iter().any(|member| member.bytes == text)
    };
    match own {
        LatticeValue::ConstSet(members) => {
            let kept: Vec<ConstValue> = members.iter().filter(|c| permits(c)).cloned().collect();
            match kept.as_slice() {
                [] if loop_exit => LatticeValue::Unknown,
                [] => own.clone(),
                [one] => LatticeValue::Const(one.clone()),
                _ => LatticeValue::constset(kept),
            }
        }
        LatticeValue::Const(constant) if loop_exit && !permits(constant) => LatticeValue::Unknown,
        LatticeValue::Overdefined => match allowed.as_slice() {
            [] => LatticeValue::Overdefined,
            [one] => LatticeValue::Const(crate::value_transfer::exact_to_const(one)),
            many => LatticeValue::constset(
                many.iter()
                    .map(|member| crate::value_transfer::exact_to_const(member))
                    .collect(),
            ),
        },
        LatticeValue::Unknown | LatticeValue::Const(_) => own.clone(),
    }
}

/// The places an existence query names that no SSA symbol holds, sorted: a
/// name the function only asks `info exists` or `array exists` about — `if
/// {[info exists x]}` with no other mention of `x` — or the array an
/// element query asks about, which the query reads (an element exists only
/// in an array). Each takes a slot past the SSA's symbols, so the rung
/// carries it from the entry through every clobber like any other place.
fn query_only_places(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
) -> Vec<String> {
    let mut names = Vec::new();
    for (&block_id, block) in &cfg.blocks {
        for (index, statement) in block.statements.iter().enumerate() {
            let parent = cfg.source_tokens_at(block_id, index);
            match statement {
                Statement::AssignExpr {
                    expr, expr_base, ..
                } => {
                    collect_query_names(
                        expr,
                        crate::existence_query::QueryReading::for_cfg(
                            cfg, registry, config, parent, *expr_base,
                        ),
                        &mut names,
                    );
                }
                Statement::AssignValue { value, .. } => {
                    let reading = crate::existence_query::QueryReading::for_cfg(
                        cfg, registry, config, parent, None,
                    );
                    if cfg.metadata_context.is_standalone() {
                        names.extend(reading.in_text(value).map(|query| query.var));
                    } else if let Some(calls) = parent
                        .and_then(|parent| crate::word_subst::checked_lifted_calls(parent, config))
                    {
                        for call in calls {
                            names.extend(
                                call.tokens
                                    .as_ref()
                                    .or(call.words.as_ref())
                                    .and_then(|tokens| reading.in_tokens(tokens))
                                    .map(|query| query.var),
                            );
                        }
                    }
                }
                _ => {}
            }
        }
        if let Some(Terminator::Branch {
            condition,
            condition_base,
            ..
        }) = &block.terminator
        {
            collect_query_names(
                condition,
                crate::existence_query::QueryReading::for_cfg(
                    cfg,
                    registry,
                    config,
                    cfg.source_tokens_at(block_id, usize::MAX),
                    *condition_base,
                ),
                &mut names,
            );
        }
    }
    let mut places: Vec<String> = names
        .iter()
        .map(|name| place_base(name).to_owned())
        .filter(|place| ssa.var_symbol(place).is_none())
        .collect();
    places.sort_unstable();
    places.dedup();
    places
}

/// Keep command leaves attached to the same original expression site while
/// walking its tree. The explicit stack avoids native recursion depth limits.
fn collect_query_names(
    expression: &ExprNode,
    reading: crate::existence_query::QueryReading<'_>,
    names: &mut Vec<String>,
) {
    let mut pending = vec![expression];
    while let Some(node) = pending.pop() {
        match node {
            ExprNode::Command { .. } => {
                names.extend(reading.in_expr(node).map(|query| query.var));
            }
            ExprNode::Binary { left, right, .. } => pending.extend([&**left, &**right]),
            ExprNode::Unary { operand, .. } => pending.push(operand),
            ExprNode::Ternary {
                condition,
                true_branch,
                false_branch,
            } => {
                pending.extend([&**condition, &**true_branch, &**false_branch]);
            }
            ExprNode::Call { args, .. } => pending.extend(args.iter()),
            _ => {}
        }
    }
}

/// The fact a place named `name` enters an unaliased frame with: a
/// parameter bound as a scalar; an element whose array the frame links or
/// holds from elsewhere, a `TclOO` instance variable, a connection-scoped
/// name, and a special variable startup does not bind all may-bound; a
/// special variable startup binds bound as its kind; anything else unbound.
fn entry_fact(
    name: &str,
    entry: ExistenceEntry<'_>,
    registry: &CommandRegistry,
    dialect: Option<tcl_dialect::model::SurfaceQuery<'static>>,
) -> Existence {
    let linked = |name: &str| linked_to_state(name, entry);
    let special = |name: &str| special_at_entry(name, entry, registry, dialect);
    let base = place_base(name);
    if base != name {
        // Which elements an array holds is its own fact: an element of a
        // parameter, a linked array or a special one is not provably
        // absent.
        let held_elsewhere = entry.params.iter().any(|param| param == base)
            || linked(base)
            || special(base).is_some()
            || entry.caller_places.is_some_and(|places| places.links(base));
        return if held_elsewhere {
            Existence::MayBound
        } else {
            Existence::Unbound
        };
    }
    if entry.params.iter().any(|param| param == name) {
        return Existence::Bound(BindingKind::Scalar);
    }
    if let Some(&fact) = entry
        .caller_places
        .and_then(|places| places.entries.get(name))
    {
        return fact;
    }
    if linked(name) {
        return Existence::MayBound;
    }
    if let Some(spec) = special(name) {
        let bound = registry.is_initially_bound(name, dialect);
        return match spec.kind {
            tcl_registry::special_vars::SpecialVarKind::Scalar if bound => {
                Existence::Bound(BindingKind::Scalar)
            }
            tcl_registry::special_vars::SpecialVarKind::Array if bound => {
                Existence::Bound(BindingKind::Array)
            }
            _ => Existence::MayBound,
        };
    }
    Existence::Unbound
}

/// Whether `name` is a `TclOO` instance variable (not shadowed by a
/// parameter) or a name an iRules `when` handler's connection binds: state
/// another invocation holds.
fn linked_to_state(name: &str, entry: ExistenceEntry<'_>) -> bool {
    entry.object_state.is_some_and(|state| state.contains(name))
        && !entry.params.iter().any(|param| param == name)
        || entry
            .connection_scoped
            .is_some_and(|scoped| scoped.holds(name))
}

/// The registry's special variable `name`, when the body is the document's
/// initial global frame, where the host rather than the script binds it.
fn special_at_entry(
    name: &str,
    entry: ExistenceEntry<'_>,
    registry: &CommandRegistry,
    dialect: Option<tcl_dialect::model::SurfaceQuery<'static>>,
) -> Option<&'static tcl_registry::special_vars::SpecialVarSpec> {
    entry
        .initial_global
        .then(|| registry.special_var_in_dialect(name, dialect))
        .flatten()
}

/// Whether the place `name`, or the array it is an element of, is linked
/// to state another invocation, the object or the host holds: a `TclOO`
/// instance variable, a connection-scoped name, or a special variable of
/// the initial global frame. Beside the qualified, aliased and traced
/// places the rung already holds may-bound, these are the externally
/// mutable places a refinement never narrows.
fn linked_elsewhere(
    name: &str,
    entry: ExistenceEntry<'_>,
    registry: &CommandRegistry,
    dialect: Option<tcl_dialect::model::SurfaceQuery<'static>>,
) -> bool {
    let base = place_base(name);
    linked_to_state(base, entry) || special_at_entry(base, entry, registry, dialect).is_some()
}

/// The clobber one statement performs: every place for a barrier or a
/// static-body `uplevel`, the computed-name facts its words raise, and the
/// places a nested body it keeps inline may define or destroy.
fn statement_clobber(
    statement: &Statement,
    ssa: &SsaFunction,
    cfg: &CfgFunction,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
) -> Clobber {
    let mut clobber = Clobber::of_names(crate::dynamic_names::statement_barrier_for_cfg(
        statement, cfg, registry, config,
    ));
    if matches!(
        statement,
        Statement::Barrier { .. } | Statement::UpFrame { .. }
    ) || crate::ssa::is_unseen_call_marker(statement)
    {
        clobber.all = true;
        return clobber;
    }
    let mut touch = BodyTouch::default();
    for script in crate::ir_helpers::nested_bodies(statement) {
        touch.script(script, registry, config, 0);
    }
    touch.statement_words(statement, registry, config, 0);
    clobber.all |= touch.all;
    clobber.touched = touched_symbols(&touch.names, ssa);
    clobber
}

/// The clobber a block's terminator performs: the computed-name facts its
/// condition or returned word raise, and the places a script body nested
/// in one of its command substitutions may define or destroy.
fn terminator_clobber(
    terminator: &Terminator,
    ssa: &SsaFunction,
    cfg: &CfgFunction,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
) -> Clobber {
    let mut clobber = Clobber::of_names(crate::dynamic_names::terminator_barrier_for_cfg(
        terminator, cfg, registry, config,
    ));
    let mut touch = BodyTouch::default();
    match terminator {
        Terminator::Branch { condition, .. } => touch.expr(condition, registry, config, 0),
        Terminator::Return { value, expr, .. } => {
            if let Some(value) = value {
                touch.word(value, registry, config, 0);
            }
            if let Some(expr) = expr {
                touch.expr(expr, registry, config, 0);
            }
        }
        // Complete records an already observed invocation in this block;
        // its operands and effects are accounted for by the original statement.
        Terminator::Goto { .. } | Terminator::Complete { .. } => {}
    }
    clobber.all |= touch.all;
    clobber.touched = touched_symbols(&touch.names, ssa);
    clobber
}

/// The symbols of every place `named` holds: a place and the array it sits
/// in, and every element of an array touched whole.
fn touched_symbols(named: &HashSet<String>, ssa: &SsaFunction) -> Vec<Symbol> {
    if named.is_empty() {
        return Vec::new();
    }
    let bases: HashSet<&str> = named.iter().map(|name| place_base(name)).collect();
    ssa.var_names()
        .iter()
        .enumerate()
        .filter(|(_, name)| {
            named.contains(name.as_str())
                || bases.contains(name.as_str())
                || bases.contains(place_base(name))
        })
        .map(|(index, _)| Symbol(u32::try_from(index).unwrap_or(u32::MAX)))
        .collect()
}

/// What the script bodies a statement runs in this frame may define or
/// destroy: an inline nested body's (a non-lowered `switch`'s arms), and one
/// nested in a command substitution — `[catch {unset x}]`, `[eval {…}]`,
/// `[lmap v {1} {…}]` run their body here, so the rung clobbers every name
/// it defines or destroys (#2231's consequence). A
/// body lowers to the statements the IR builds for it and each is asked
/// what it defines ([`crate::ssa::defs_of_with_registry`]); a barrier or an
/// up-frame among them, or text nested past the depth cap, touches every
/// place.
#[derive(Default)]
struct BodyTouch {
    /// The places the bodies define or destroy.
    names: HashSet<String>,
    /// Whether a body may touch any place.
    all: bool,
}

impl BodyTouch {
    /// Every statement of `script`, however nested: what it defines, and
    /// the substitutions its own words run.
    fn script(
        &mut self,
        script: &crate::ir::Script,
        registry: &CommandRegistry,
        config: tcl_lexer::LexerConfig,
        depth: u32,
    ) {
        crate::ir::for_each_statement(script, &mut |inner| {
            if matches!(inner, Statement::Barrier { .. } | Statement::UpFrame { .. }) {
                self.all = true;
            }
            self.names
                .extend(crate::ssa::defs_of_with_registry(inner, Some(registry)));
            self.statement_words(inner, registry, config, depth);
        });
    }

    /// The command substitutions one statement's words run.
    fn statement_words(
        &mut self,
        statement: &Statement,
        registry: &CommandRegistry,
        config: tcl_lexer::LexerConfig,
        depth: u32,
    ) {
        match statement {
            Statement::Call { args, .. } | Statement::Barrier { args, .. } => {
                for arg in args {
                    self.word(arg, registry, config, depth);
                }
            }
            Statement::AssignConst { value, .. }
            | Statement::AssignValue { value, .. }
            | Statement::Switch { subject: value, .. } => {
                self.word(value, registry, config, depth);
            }
            Statement::AssignExpr { expr, .. } | Statement::ExprEval { expr, .. } => {
                self.expr(expr, registry, config, depth);
            }
            Statement::Return { value, expr, .. } => {
                if let Some(value) = value {
                    self.word(value, registry, config, depth);
                }
                if let Some(expr) = expr {
                    self.expr(expr, registry, config, depth);
                }
            }
            _ => {}
        }
    }

    /// Every `[…]` in a word's spelling.
    fn word(
        &mut self,
        text: &str,
        registry: &CommandRegistry,
        config: tcl_lexer::LexerConfig,
        depth: u32,
    ) {
        for inner in crate::var_refs::command_subst_texts_with_config(text, config) {
            self.substitution(&inner, registry, config, depth + 1);
        }
    }

    /// Every `[…]` in an expression.
    fn expr(
        &mut self,
        expr: &ExprNode,
        registry: &CommandRegistry,
        config: tcl_lexer::LexerConfig,
        depth: u32,
    ) {
        let mut commands = Vec::new();
        crate::ir_helpers::collect_expr_commands(expr, &mut commands);
        for text in &commands {
            let trimmed = text.trim();
            let inner = trimmed
                .strip_prefix('[')
                .and_then(|inner| inner.strip_suffix(']'))
                .unwrap_or(trimmed);
            self.substitution(inner, registry, config, depth + 1);
        }
    }

    /// One substitution's script: the bodies its commands run in this frame
    /// — never its own commands' definitions, which the statements the
    /// lowering places for a substitution already state — and the
    /// substitutions its words nest. A command the lowering keeps opaque
    /// (`[eval $script]`, `[dict with d {…}]`) may touch any place.
    fn substitution(
        &mut self,
        script: &str,
        registry: &CommandRegistry,
        config: tcl_lexer::LexerConfig,
        depth: u32,
    ) {
        if crate::depth_guard::MAX_BRACKET_TEXT_DEPTH.exceeded(depth) {
            self.all = true;
            return;
        }
        // A script with no braced or quoted word holds no body, and one with
        // no bracket nests no substitution.
        if !script.contains(['{', '"', '[']) {
            return;
        }
        let module =
            crate::lowering::lower_to_ir_with_dialect(script, registry, config, registry.profile());
        for statement in &module.top_level.statements {
            if matches!(
                statement,
                Statement::Barrier { .. } | Statement::UpFrame { .. }
            ) {
                self.all = true;
            }
            for body in crate::ir_helpers::nested_bodies(statement) {
                self.script(body, registry, config, depth);
            }
            self.statement_words(statement, registry, config, depth);
        }
    }
}

/// The variable that holds the place `name`: the array for an element
/// `base(key)`, the name itself otherwise.
pub(crate) fn place_base(name: &str) -> &str {
    name.split_once('(')
        .filter(|_| name.ends_with(')'))
        .map_or(name, |(base, _)| base)
}

/// Per handler block, the region its exception edges leave from: every
/// block on a normal path from one of its edges' sources to another — a
/// `try` or `catch` body between its first command and its tail — sources
/// included. Any command in the region may throw, so the handler sees every
/// point of it. The block before the body is no part of it: its edge
/// (`entry_edges`) carries the state at its exit alone.
fn handler_regions(
    cfg: &CfgFunction,
    entry_edges: &HashSet<(BlockId, BlockId)>,
) -> HashMap<BlockId, Vec<BlockId>> {
    let mut sources: HashMap<BlockId, Vec<BlockId>> = HashMap::new();
    for &(from, to) in &cfg.exception_edges {
        if entry_edges.contains(&(from, to)) {
            continue;
        }
        let listed = sources.entry(to).or_default();
        if !listed.contains(&from) {
            listed.push(from);
        }
    }
    if sources.is_empty() {
        return HashMap::new();
    }
    let mut predecessors: HashMap<BlockId, Vec<BlockId>> = HashMap::new();
    for (&id, block) in &cfg.blocks {
        for successor in block.successors() {
            predecessors.entry(successor).or_default().push(id);
        }
    }
    let reach = |starts: &[BlockId], next: &dyn Fn(BlockId) -> Vec<BlockId>| {
        let mut seen: HashSet<BlockId> = starts.iter().copied().collect();
        let mut work: Vec<BlockId> = starts.to_vec();
        while let Some(block) = work.pop() {
            for other in next(block) {
                if seen.insert(other) {
                    work.push(other);
                }
            }
        }
        seen
    };
    sources
        .into_iter()
        .map(|(handler, starts)| {
            let forward = reach(&starts, &|block| {
                cfg.blocks
                    .get(&block)
                    .map(crate::cfg::Block::successors)
                    .unwrap_or_default()
            });
            let backward = reach(&starts, &|block| {
                predecessors.get(&block).cloned().unwrap_or_default()
            });
            let mut region: Vec<BlockId> = forward.intersection(&backward).copied().collect();
            region.sort_unstable_by_key(|block| block.0);
            (handler, region)
        })
        .collect()
}

/// The lattice's starting values: the interprocedural parameter seed. The
/// seed keys on the parameter *name* (a stable, cache-safe identity); each
/// resolves to this build's interned symbol. A parameter never read in the
/// body isn't interned, and its seed slot would never be consulted, so
/// dropping it is behaviour-neutral.
fn seeded_values(
    ssa: &SsaFunction,
    param_constants: Option<&HashMap<(String, crate::ssa::Version), LatticeValue>>,
) -> HashMap<ValueKey, LatticeValue> {
    let mut values = HashMap::new();
    for ((name, version), value) in param_constants.into_iter().flatten() {
        if let Some(sym) = ssa.var_symbol(name) {
            values.insert((sym, *version), value.clone());
        }
    }
    values
}

/// Seed live-in roots to `Overdefined`: a value *used* but never *defined*
/// anywhere in this function (a proc parameter, a global / namespace read, an
/// upvar target, or an undefined-variable read) holds a runtime-unknown value,
/// so it is `Overdefined`, not the `Unknown` join-identity (which would
/// silently vanish from any phi it feeds — folding `join(const, $runtime)` to
/// the constant).
fn seed_live_in_roots<S: std::hash::BuildHasher>(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    values: &mut HashMap<ValueKey, LatticeValue, S>,
    grammar: tcl_dialect::LexerGrammar,
) {
    let mut defined_keys: FxHashSet<ValueKey> = FxHashSet::default();
    let mut used_keys: FxHashSet<ValueKey> = FxHashSet::default();
    for ssa_block in ssa.blocks.values() {
        for phi in &ssa_block.phis {
            defined_keys.insert((phi.name, phi.version));
            for inc in phi.incoming.values() {
                // Record every phi feed, including the version-0 (entry /
                // live-in) incoming: it is never a def, so it drops through
                // to the `used_keys.difference(&defined_keys)` seeding below
                // and is pinned `Overdefined`. This is what lets the phi
                // join at [`sccp_process_phis`] see the caller's value
                // instead of silently dropping it and folding to the
                // defined-arm constant.
                used_keys.insert((phi.name, *inc));
            }
        }
        for s in &ssa_block.statements {
            for (&var, ver) in &s.defs {
                defined_keys.insert((var, *ver));
            }
            for (&var, ver) in &s.uses {
                used_keys.insert((var, *ver));
            }
        }
    }
    // Branch-condition reads, resolved at each block's exit versions.
    for (bn, block) in &cfg.blocks {
        if let Some(Terminator::Branch { condition, .. }) = &block.terminator
            && let Some(sb) = ssa.blocks.get(bn)
        {
            for var in crate::var_refs::vars_in_expr(condition, grammar) {
                let Some(sym) = ssa.var_symbol(&var) else {
                    continue;
                };
                let ver = sb.exit_versions.get(&sym).copied().unwrap_or(0);
                used_keys.insert((sym, ver));
            }
        }
    }
    for key in used_keys.difference(&defined_keys) {
        values.entry(*key).or_insert(LatticeValue::Overdefined);
    }
}

/// Optimistic (Wegman–Zadeck) deferral test for a non-constant branch.
///
/// Returns `true` when the condition *may still fold* on a later sweep:
/// some operand defined in this function (SSA exit-version > 0) is still
/// `Unknown` (not yet computed) and none is `Overdefined`. Such a branch
/// opens neither arm until either the operand resolves to a constant or an
/// `Overdefined` operand proves the condition genuinely non-constant.
///
/// Operands read at version 0 (proc parameters, globals, and other
/// live-in roots) are excluded: [`seed_live_in_roots`] already seeds them
/// `Overdefined`, so they are never "not yet computed".
fn branch_deferrable(
    ssa_block: &crate::ssa::SsaBlock,
    condition: &ExprNode,
    values: &HashMap<ValueKey, LatticeValue>,
    ssa: &SsaFunction,
    grammar: tcl_dialect::LexerGrammar,
) -> bool {
    let mut any_operand = false;
    let mut any_unknown = false;
    for name in crate::var_refs::vars_in_expr(condition, grammar) {
        let Some(sym) = ssa.var_symbol(&name) else {
            continue;
        };
        let ver = ssa_block.exit_versions.get(&sym).copied().unwrap_or(0);
        if ver == 0 {
            continue;
        }
        any_operand = true;
        match values.get(&(sym, ver)) {
            Some(LatticeValue::Overdefined) => return false,
            Some(LatticeValue::Unknown) | None => any_unknown = true,
            _ => {}
        }
    }
    any_operand && any_unknown
}

/// A name is externally mutable (and so never a constant) when it is global /
/// namespace-qualified, escapes via alias / trace *within this function* (or
/// is traced *anywhere in the module* — a `trace add variable` installed by
/// a different proc, unioned into `escaping` by the caller; see [`sccp`]'s
/// docs), or the module installs a variable trace on a non-literal
/// (dynamic) target — in which case *every* name is potentially traced and
/// none can be trusted, mirroring
/// [`crate::gvn::is_pure_command_with_traces`]'s handling of
/// `has_dynamic_trace`.
///
/// `pub(crate)`: also consulted by [`crate::optimiser::propagation`]'s
/// def-use-chain-based load-forwarding (O102), which does not otherwise run
/// through this module's lattice and so needs the same predicate applied
/// directly.
pub(crate) fn is_externally_mutable(
    name: &str,
    escaping: &HashSet<String>,
    has_dynamic_variable_trace: bool,
) -> bool {
    has_dynamic_variable_trace || name.starts_with("::") || escaping.contains(name)
}

/// Join phi values from edge-executable predecessors for one block. Returns
/// `true` if any lattice value changed. Extracted from [`sccp`].
fn sccp_process_phis(
    values: &mut HashMap<ValueKey, LatticeValue>,
    ssa_block: &crate::ssa::SsaBlock,
    incoming_exec: &[BlockId],
) -> bool {
    if incoming_exec.is_empty() {
        return false;
    }
    let mut changed = false;
    for phi in &ssa_block.phis {
        let mut phi_val = LatticeValue::Unknown;
        for pred in incoming_exec {
            let incoming_ver = phi.incoming.get(pred).copied().unwrap_or(0);
            // A version-0 incoming is the entry / live-in root (a proc
            // parameter, global, or other caller-supplied value). Its
            // runtime value is unknown at compile time, so it joins in as
            // `Overdefined` — never skipped. Skipping it would let a phi
            // that merges a live-in with a defined-arm constant fold to
            // that constant, miscompiling any `if {$param} { set x k }`
            // followed by a test on `x`. This mirrors the interval pass,
            // which joins `TOP` for `inc == 0` (intervals.rs:570-574).
            // `seed_live_in_roots` pins `(name, 0)` `Overdefined`; the
            // explicit default keeps the join correct even if a feed was
            // never seeded.
            let key: ValueKey = (phi.name, incoming_ver);
            let candidate = values.get(&key).cloned().unwrap_or(if incoming_ver == 0 {
                LatticeValue::Overdefined
            } else {
                LatticeValue::Unknown
            });
            phi_val = join(&phi_val, &candidate);
        }
        if set_value(values, (phi.name, phi.version), &phi_val) {
            changed = true;
        }
    }
    changed
}

/// Each φ's folded type: what every executable incoming value states alike
/// ([`crate::value_transfer::FoldedType::join_all`]). A live-in root states
/// nothing, so a φ over one states nothing; an incoming value the solver
/// has not reached is skipped, as the value join skips it.
fn record_phi_folded_types(
    values: &HashMap<ValueKey, LatticeValue>,
    ssa_block: &crate::ssa::SsaBlock,
    incoming_exec: &[BlockId],
    driver: &LatticeDriver<'_>,
) {
    if incoming_exec.is_empty() {
        return;
    }
    for phi in &ssa_block.phis {
        let members = incoming_exec.iter().filter_map(|pred| {
            let version = phi.incoming.get(pred).copied().unwrap_or(0);
            if version == 0 {
                return Some(None);
            }
            let key: ValueKey = (phi.name, version);
            match values.get(&key) {
                None | Some(LatticeValue::Unknown) => None,
                Some(_) => Some(driver.folded_of(key)),
            }
        });
        let folded = crate::value_transfer::FoldedType::join_all(members);
        driver.record_folded((phi.name, phi.version), folded);
    }
}

/// What [`sccp_process_statements`] reads beside the block: the block's id,
/// the names that escape the function, whether a dynamic variable trace may
/// fire, and the fresh versions each marker for a call to code the module
/// cannot see in the block gives the names live after it
/// ([`crate::ssa::SsaFunction::value_clobbers`]).
#[derive(Clone, Copy)]
struct StatementInputs<'a> {
    block: BlockId,
    escaping: &'a HashSet<String>,
    has_dynamic_variable_trace: bool,
    clobbers: Option<&'a crate::ssa::BlockValueClobbers>,
    /// The opaque `catch` bodies the enumeration may run, when the run has
    /// them.
    catches: Option<CatchBodies<'a>>,
}

/// The bodies of a function's opaque `catch` calls, by span, and which
/// places another actor may write, which a body the enumeration runs writes
/// none of.
#[derive(Clone, Copy)]
struct CatchBodies<'a> {
    bodies: &'a HashMap<tcl_lexer::Span, crate::ir::Script>,
    external: &'a dyn Fn(&str) -> bool,
}

/// Evaluate each statement's defs for one block, widening across barriers.
/// Returns `true` if any lattice value changed. Extracted from [`sccp`].
///
/// With the existence rung, each statement also records the facts it
/// reads, advances each place it defines by its storage outcome — the
/// externally mutable ones stay may-bound — and applies its clobber.
fn sccp_process_statements(
    values: &mut HashMap<ValueKey, LatticeValue>,
    ssa_block: &crate::ssa::SsaBlock,
    ssa: &SsaFunction,
    inputs: StatementInputs<'_>,
    driver: &LatticeDriver<'_>,
    mut existence: Option<&mut ExistenceAt<'_>>,
) -> StatementsRun {
    let StatementInputs {
        block,
        escaping,
        has_dynamic_variable_trace,
        clobbers,
        catches,
    } = inputs;
    let (mut changed, mut prepared) = (false, None);
    let mut exit = BlockExit::NORMAL;
    // The existence a marker for what a statement's scripts may write finds,
    // which is where those scripts start: the statement after the marker is
    // evaluated over it.
    let mut before_marker = None;
    for (index, stmt_ssa) in ssa_block.statements.iter().enumerate() {
        // The fresh versions a call to code the module cannot see leaves take
        // effect after the inputs and the barrier of the call they stand
        // beside.
        if crate::ssa::is_unseen_call_marker(&stmt_ssa.statement) {
            changed |= clobber_at_unseen_call(
                values,
                (index, clobbers, exit.completes),
                (driver, existence.as_deref_mut()),
            );
            continue;
        }
        // A statement after one that certainly raises never runs: its
        // definitions keep what their places held.
        if !exit.completes {
            changed |= keep_prior(
                values,
                (ssa_block, index),
                stmt_ssa,
                (driver, existence.as_deref_mut()),
            );
            continue;
        }
        if let Some(at) = existence.as_deref_mut() {
            at.record_reads(index, stmt_ssa, driver);
        }
        let scripts_start = before_marker.take();
        if crate::ssa::is_arm_writes_marker(&stmt_ssa.statement) {
            before_marker = driver.existence_state();
        }
        if matches!(
            stmt_ssa.statement,
            Statement::Barrier { .. } | Statement::UpFrame { .. }
        ) {
            // A `return` that leaves the procedure writes nothing, and
            // nothing of the frame runs after it: the values its own words
            // read stay what they are.
            if !stmt_ssa.statement.is_return_barrier() {
                changed |= widen_at_barrier(
                    values,
                    (index, stmt_ssa),
                    (driver, existence.as_deref_mut()),
                );
            }
            continue;
        }
        // An opaque `catch` body the enumeration runs gives the names it
        // writes what it leaves, before the marker that states them widens.
        let body_run = catches.and_then(|catches| {
            catch_body_answer(
                (ssa_block, index),
                (values, ssa),
                (driver, catches),
                before_marker.as_deref(),
            )
        });
        let _entry = driver.body_entry_scope(scripts_start);
        // The statement is evaluated once, when a definition first needs
        // it: a call's ordered stores give each definition its own value.
        let mut evaluated = body_run
            .or_else(|| {
                driver.evaluate_catch_end((ssa_block, index), values, ssa, |entry| {
                    existence
                        .as_deref()
                        .and_then(|at| at.run.exits.get(&entry).cloned())
                })
            })
            .or_else(|| pair_answer(&mut prepared, (ssa_block, index), values, (ssa, driver)))
            .or_else(|| driver.procedure_answer((ssa_block, index), values, ssa));
        // Where a raise ends the path, whether the statement raises is part
        // of what it does, so it is evaluated whatever its definitions need.
        if driver.raises_end_paths() && evaluated.is_none() {
            evaluated = if stmt_ssa.defs.is_empty() {
                driver.probe_completion(stmt_ssa, values, ssa)
            } else {
                Some(evaluate_defs_under(stmt_ssa, values, ssa, driver))
            };
        }
        let raised = raised_writes(evaluated.as_ref());
        driver.record_raised((block, index), raised.is_some());
        let site = StatementSite {
            block: ssa_block,
            index,
            ssa,
            escaping,
            has_dynamic_variable_trace,
            driver,
        };
        changed |= define_values(values, &site, evaluated, existence.as_deref_mut());
        if let Some(written) = raised {
            exit = BlockExit {
                completes: false,
                entry_throws: index > 0 || written == 0,
            };
        }
        if let Some(at) = existence.as_deref_mut() {
            at.finish_statement(index, driver);
        }
    }
    StatementsRun { changed, exit }
}

/// What the names an opaque `catch` body writes hold after it, when the
/// enumeration runs the body to its end over the exact state before it
/// ([`crate::static_loops::enumerate_script`]): the `catch` absorbs however
/// the body completes, so each name holds what the body left — a value, or
/// unbound — and one it never wrote holds what it held. The marker at
/// `index` states the body's writes, reading each name's version before it,
/// and the call it stands ahead of reads the rest of what the body reads;
/// `existence` is the rung's state before the marker. `None` for any other
/// statement and for a body the enumeration does not run.
fn catch_body_answer(
    (ssa_block, index): (&crate::ssa::SsaBlock, usize),
    (values, ssa): (&HashMap<ValueKey, LatticeValue>, &SsaFunction),
    (driver, catches): (&LatticeDriver<'_>, CatchBodies<'_>),
    existence: Option<&[Existence]>,
) -> Option<DefValues> {
    use crate::static_loops::Slot;
    use crate::value_transfer::{DefAnswer, ExistenceStep};
    let marker = ssa_block.statements.get(index)?;
    if !crate::ssa::is_arm_writes_marker(&marker.statement) {
        return None;
    }
    let host = &ssa_block.statements[crate::ssa::arm_writes_host(&ssa_block.statements, index)?];
    let body = catches.bodies.get(&host.statement.span())?;
    let mut state = crate::static_loops::LoopState::default();
    for (&symbol, &version) in marker.uses.iter().chain(&host.uses) {
        let name = ssa.var_name(symbol);
        match values.get(&(symbol, version)) {
            Some(LatticeValue::Const(constant)) => state.set(
                name.to_owned(),
                Slot::Value(crate::value_transfer::const_to_exact(constant)),
            ),
            _ => {
                if let Some(slot) = existence
                    .and_then(|facts| facts.get(symbol.0 as usize))
                    .copied()
                    .and_then(slot_of_existence)
                {
                    state.set(name.to_owned(), slot);
                }
            }
        }
    }
    let run = crate::static_loops::enumerate_script(
        driver,
        body,
        state,
        crate::static_loops::DEFAULT_MAX_STATIC_LOOP_ITERS,
        catches.external,
    )
    .ok()?;
    // The marker states every name the body writes; a write it does not
    // state is one the solver has no definition for.
    let stated: HashSet<&str> = marker
        .defs
        .keys()
        .map(|&symbol| ssa.var_name(symbol))
        .collect();
    if run
        .written
        .iter()
        .any(|name| !stated.contains(name.as_str()))
    {
        return None;
    }
    let answers = marker
        .defs
        .iter()
        .map(|(&symbol, &version)| {
            let key = (symbol, version);
            let name = ssa.var_name(symbol);
            match (run.written.contains(name), run.state.get(name)) {
                (true, Some(Slot::Value(value))) => Some(DefAnswer {
                    stated: true,
                    existence: ExistenceStep::Set(Existence::Bound(BindingKind::Scalar)),
                    ..DefAnswer::untyped(
                        key,
                        LatticeValue::Const(crate::value_transfer::exact_to_const(value)),
                    )
                }),
                (true, Some(Slot::Unbound)) => Some(DefAnswer {
                    stated: true,
                    existence: ExistenceStep::Set(Existence::Unbound),
                    ..DefAnswer::untyped(key, LatticeValue::Overdefined)
                }),
                (true, _) => None,
                (false, _) => {
                    let prior = marker.uses.get(&symbol).copied().unwrap_or(0);
                    Some(DefAnswer {
                        preserved: true,
                        existence: ExistenceStep::PRESERVE,
                        ..DefAnswer::untyped(key, held_value(values, (symbol, prior)))
                    })
                }
            }
        })
        .collect::<Option<Vec<_>>>()?;
    Some(DefValues::PerDef(answers))
}

/// The fresh versions the marker for a call to code the module cannot see at
/// `index` gives the names live after it
/// ([`crate::ssa::SsaFunction::value_clobbers`]): that code may have rewritten
/// each, so each states no value and, with the rest of the frame, may be bound
/// or not — save a parameter of a procedure a caller proved pure, whose seed
/// no such code can rebind. After a statement that certainly raised the
/// marker was never reached, so each keeps what its place held. Whether
/// anything moved.
fn clobber_at_unseen_call(
    values: &mut HashMap<ValueKey, LatticeValue>,
    (index, clobbers, completes): (usize, Option<&crate::ssa::BlockValueClobbers>, bool),
    (driver, existence): (&LatticeDriver<'_>, Option<&mut ExistenceAt<'_>>),
) -> bool {
    let mut changed = false;
    let fresh_versions = clobbers.and_then(|markers| markers.get(&index));
    for (&var, &(prior, fresh)) in fresh_versions.into_iter().flatten() {
        let kept = !completes
            || (driver.proven_pure_parameters()
                && matches!(values.get(&(var, 0)), Some(LatticeValue::Const(_))));
        let value = if kept {
            held_value(values, (var, prior))
        } else {
            LatticeValue::Overdefined
        };
        driver.record_folded(
            (var, fresh),
            kept.then(|| driver.folded_of((var, prior))).flatten(),
        );
        changed |= set_value(values, (var, fresh), &value);
    }
    if let Some(at) = existence {
        if completes {
            at.finish_statement(index, driver);
        }
        let step = if completes {
            crate::value_transfer::ExistenceStep::Set(Existence::MayBound)
        } else {
            crate::value_transfer::ExistenceStep::PRESERVE
        };
        for (&var, &(_, fresh)) in fresh_versions.into_iter().flatten() {
            changed |= at.advance((var, fresh), step, driver);
        }
    }
    changed
}

/// What a barrier or an up-frame statement does to the values: every value
/// is widened and the statement's own definitions are opaque. Whether a value
/// moved.
fn widen_at_barrier(
    values: &mut HashMap<ValueKey, LatticeValue>,
    (index, stmt_ssa): (usize, &SsaStatement),
    (driver, existence): (&LatticeDriver<'_>, Option<&mut ExistenceAt<'_>>),
) -> bool {
    let mut changed = false;
    // Barriers widen all currently-tracked values — EXCEPT
    // version-0 (parameter) seeds, which hold the caller's
    // literal and are immutable across the barrier (a barrier
    // that mutates the var produces a fresh version), so a
    // callee `dict with $param` still sees the interproc
    // literal.
    //
    // `UpFrame` (the CFG shape for a literal-body `uplevel`)
    // shares this treatment: `uplevel 1 {…}` / `uplevel #0 {…}`
    // evaluates its body in a DIFFERENT frame — the caller's, or
    // the absolute global one — so it can reassign any name
    // visible there, exactly like an opaque barrier. Reproduced
    // against tclsh 8.6/9.0: `set n 5; uplevel #0 {set n 99};
    // puts [expr {$n + 1}]` prints `100`; before this widening,
    // the optimiser proposed folding to the stale `6`.
    let keys: Vec<ValueKey> = values.keys().copied().collect();
    for k in keys {
        if k.1 == 0 {
            continue;
        }
        if set_value(values, k, &LatticeValue::Overdefined) {
            changed = true;
        }
    }
    // A widened value states nothing of its type either.
    driver.forget_folded();
    // A barrier also *defines* variables of its own (e.g. `dict for {x
    // y} …` defines `x`/`y`). Those defs are opaque — the barrier can
    // set them to anything — so set each to `Overdefined`. Without this
    // the def key is never inserted (the widen loop above only touches
    // keys already present), so it stays `Unknown` and vanishes from a
    // downstream phi join, letting a phi that merges a barrier-def with
    // a constant fold to that constant and miscompile a following test.
    for (&var, ver) in &stmt_ssa.defs {
        if set_value(values, (var, *ver), &LatticeValue::Overdefined) {
            changed = true;
        }
    }
    // Every place is may-bound after a barrier, as every value is
    // widened.
    if let Some(at) = existence {
        changed |= at.barrier(index, stmt_ssa, driver);
    }
    changed
}

/// The statement a block's sweep is defining the values of, with what it is
/// evaluated under.
struct StatementSite<'a> {
    block: &'a crate::ssa::SsaBlock,
    index: usize,
    ssa: &'a SsaFunction,
    escaping: &'a HashSet<String>,
    has_dynamic_variable_trace: bool,
    driver: &'a LatticeDriver<'a>,
}

/// How many stores ran before the statement certainly raised, when it did.
fn raised_writes(evaluated: Option<&DefValues>) -> Option<usize> {
    match evaluated {
        Some(DefValues::Raised(raised)) => Some(raised.written),
        _ => None,
    }
}

/// Each definition of the statement at `site`: its value, folded type and
/// the version it preserved, and with the existence rung its step. `evaluated`
/// is the statement's evaluation when one was made ahead; a definition that
/// needs one makes it. Whether a value moved.
fn define_values(
    values: &mut HashMap<ValueKey, LatticeValue>,
    site: &StatementSite<'_>,
    mut evaluated: Option<DefValues>,
    mut existence: Option<&mut ExistenceAt<'_>>,
) -> bool {
    let StatementSite {
        block: ssa_block,
        index,
        ssa,
        escaping,
        has_dynamic_variable_trace,
        driver,
    } = *site;
    let stmt_ssa = &ssa_block.statements[index];
    let mut changed = false;
    // An element write's base def carries no scalar value of its own —
    // `set arr(k) 5` / `set arr($i) 5` refresh `arr` for whole-array
    // readers but must never let `$arr` fold to the element's value.
    let element_write_base = match &stmt_ssa.statement {
        Statement::AssignConst { name, .. }
        | Statement::AssignExpr { name, .. }
        | Statement::AssignValue { name, .. }
        | Statement::Incr { name, .. }
            if name.contains('(') =>
        {
            ssa.var_symbol(crate::naming::normalise_var_name(name))
        }
        _ => None,
    };
    let raised = raised_writes(evaluated.as_ref());
    let keeps_all =
        matches!(&evaluated, Some(DefValues::Raised(raised)) if raised.prefix.is_none());
    for (&var, &ver) in &stmt_ssa.defs {
        let mut value_of = |values: &HashMap<ValueKey, LatticeValue>| {
            let evaluated =
                evaluated.get_or_insert_with(|| evaluate_defs_under(stmt_ssa, values, ssa, driver));
            (
                evaluated.of((var, ver)),
                evaluated.folded_of((var, ver)),
                evaluated.stated((var, ver)),
                evaluated.preserved((var, ver)),
                evaluated.existence((var, ver)),
            )
        };

        // A definition's folded type is its own evaluation's: a widened
        // or a joined definition states none. One whose outcome left its
        // place untouched names the version the place held.
        let mut preserved = None;
        let (val, folded) =
            if is_externally_mutable(ssa.var_name(var), escaping, has_dynamic_variable_trace)
                || element_write_base == Some(var)
            {
                (LatticeValue::Overdefined, None)
            } else if keeps_all {
                // The statement raised before it stored anything.
                let prior = prior_version(ssa_block, index, var);
                preserved = Some(prior);
                (held_value(values, (var, prior)), None)
            } else if stmt_ssa.may_defs.contains(&var) {
                // A synthetic array-element may-def: the write may or may
                // not have hit this element, so its value is the JOIN of
                // the prior version (recorded as a use) and the written
                // value — unless the statement's evaluated outcome names
                // the element's place (`array set arr {k v}` writes
                // `arr(k)`), which makes the write definite. The base
                // refresh of an element write carries no prior use — the
                // base holds no value of its own.
                match stmt_ssa.uses.get(&var) {
                    Some(prev_ver) => {
                        let (written, folded, stated, _, _) = value_of(values);
                        if stated {
                            (written, folded)
                        } else {
                            let prev = values
                                .get(&(var, *prev_ver))
                                .cloned()
                                .unwrap_or(LatticeValue::Overdefined);
                            (join(&prev, &written), None)
                        }
                    }
                    None => (LatticeValue::Overdefined, None),
                }
            } else {
                let (value, folded, _, kept, _) = value_of(values);
                if kept {
                    let prior = prior_version(ssa_block, index, var);
                    preserved = Some(prior);
                    // A statement that raised before it reached a place
                    // left what the place held, whether or not it read it.
                    if raised.is_some() {
                        (held_value(values, (var, prior)), folded)
                    } else {
                        (value, folded)
                    }
                } else {
                    (value, folded)
                }
            };
        driver.record_folded((var, ver), folded);
        driver.record_preserved((var, ver), preserved);
        if set_value(values, (var, ver), &val) {
            changed = true;
        }
        if let Some(at) = existence.as_deref_mut() {
            let step = at.step_for(
                stmt_ssa,
                (var, ssa.var_name(var), element_write_base),
                driver,
                (keeps_all, || value_of(values).4),
            );
            changed |= at.advance((var, ver), step, driver);
        }
    }
    changed
}

/// What a block's statements did: whether anything moved, and the block's way
/// out.
struct StatementsRun {
    changed: bool,
    exit: BlockExit,
}

/// The value `key` holds, as a φ reads it: a version no statement has defined
/// yet is the `Unknown` identity, and the entry's own is `Overdefined`.
fn held_value<S: std::hash::BuildHasher>(
    values: &HashMap<ValueKey, LatticeValue, S>,
    key: ValueKey,
) -> LatticeValue {
    values.get(&key).cloned().unwrap_or(if key.1 == 0 {
        LatticeValue::Overdefined
    } else {
        LatticeValue::Unknown
    })
}

/// The definitions of the statement at `index` of a block that never runs
/// it, because an earlier statement certainly raised: each keeps what its
/// place held, and its existence is left as it was. Whether a value moved.
fn keep_prior(
    values: &mut HashMap<ValueKey, LatticeValue>,
    (ssa_block, index): (&crate::ssa::SsaBlock, usize),
    stmt_ssa: &SsaStatement,
    (driver, mut existence): (&LatticeDriver<'_>, Option<&mut ExistenceAt<'_>>),
) -> bool {
    let mut changed = false;
    for (&var, &ver) in &stmt_ssa.defs {
        let prior = prior_version(ssa_block, index, var);
        let held = held_value(values, (var, prior));
        driver.record_folded((var, ver), None);
        driver.record_preserved((var, ver), Some(prior));
        changed |= set_value(values, (var, ver), &held);
        if let Some(at) = existence.as_deref_mut() {
            changed |= at.advance(
                (var, ver),
                crate::value_transfer::ExistenceStep::PRESERVE,
                driver,
            );
        }
    }
    changed
}

/// The existence step a typed assignment (`set`'s lowering) takes on the
/// definition `var`: the written place binds as a scalar, the array an
/// element write refreshes binds as an array, and an element the SSA fans a
/// dynamic-key write over may have been bound. `None` for any other
/// statement, whose step is its evaluation's; a typed assignment whose
/// `set` the module rebinds takes the generic widening.
fn assignment_existence(
    stmt_ssa: &SsaStatement,
    var: Symbol,
    element_write_base: Option<Symbol>,
    driver: &LatticeDriver<'_>,
) -> Option<crate::value_transfer::ExistenceStep> {
    use crate::value_transfer::ExistenceStep;
    if !matches!(
        stmt_ssa.statement,
        Statement::AssignConst { .. }
            | Statement::AssignExpr { .. }
            | Statement::AssignValue { .. }
    ) {
        return None;
    }
    Some(if !driver.typed_assignment_trusted() {
        ExistenceStep::UNKNOWN
    } else if element_write_base == Some(var) {
        ExistenceStep::Set(Existence::Bound(BindingKind::Array))
    } else if stmt_ssa.may_defs.contains(&var) {
        ExistenceStep::Join(Existence::Bound(BindingKind::Scalar))
    } else {
        ExistenceStep::Set(Existence::Bound(BindingKind::Scalar))
    })
}

/// The evaluation the statement at `index` of `block` starts with when it is
/// one of a synthetic embedded call and its host, which are one evaluation: the
/// call's definitions are what the host's words write, and the host's are its
/// result. At the call this evaluates the pair and holds the host's answer in
/// `prepared`; at the host it hands that answer over. `None` for any other
/// statement and wherever the pair declines.
fn pair_answer(
    prepared: &mut Option<(usize, DefValues)>,
    (block, index): (&crate::ssa::SsaBlock, usize),
    values: &HashMap<ValueKey, LatticeValue>,
    (ssa, driver): (&SsaFunction, &LatticeDriver<'_>),
) -> Option<DefValues> {
    let host = prepared.take().filter(|(at, _)| *at == index);
    if let Some((_, answer)) = host {
        return Some(answer);
    }
    let (host, pair) = driver.evaluate_embedded(block, index, values, ssa)?;
    *prepared = Some((host, pair.host));
    Some(match pair.raised {
        Some(written) => DefValues::Raised(Box::new(RaisedDefs {
            prefix: Some(pair.call),
            written,
        })),
        None => DefValues::PerDef(pair.call),
    })
}

/// The version of `var` the statement at `index` of `block` finds in its
/// place: the block's latest earlier definition, else the version the block
/// enters with, else the undefined root.
pub(crate) fn prior_version(
    block: &crate::ssa::SsaBlock,
    index: usize,
    var: Symbol,
) -> crate::ssa::Version {
    block.statements[..index]
        .iter()
        .rev()
        .find_map(|statement| statement.defs.get(&var).copied())
        .or_else(|| block.entry_versions.get(&var).copied())
        .unwrap_or(0)
}

/// Read-only inputs shared by [`sccp_process_terminator`].
struct TerminatorInputs<'a> {
    cfg: &'a CfgFunction,
    ssa: &'a SsaFunction,
    values: &'a HashMap<ValueKey, LatticeValue>,
    /// The document's lexer grammar — the branch condition's `Raw` operand
    /// texts are re-read under it when their variables are collected.
    grammar: tcl_dialect::LexerGrammar,
    /// The run's value-transfer driver: a condition's nested commands and
    /// finite inputs are evaluated through it.
    driver: &'a LatticeDriver<'a>,
    /// The block's way out, from its statements: a block that certainly
    /// raises opens no normal edge.
    exit: BlockExit,
    /// The exception edges that leave from the block before a body, which
    /// open when the body's first command can fail ahead of any store.
    entry_edges: &'a HashSet<(BlockId, BlockId)>,
}

/// Process a block's terminator: mark the matching outgoing edges
/// as executable.  Returns `true` when any new edge / block was
/// added.  Extracted from [`sccp`].
fn sccp_process_terminator(
    bn: BlockId,
    inputs: &TerminatorInputs<'_>,
    executable_blocks: &mut HashSet<BlockId>,
    executable_edges: &mut HashSet<(BlockId, BlockId)>,
    finalizing: bool,
) -> bool {
    let TerminatorInputs {
        cfg,
        ssa,
        values,
        grammar,
        driver,
        exit,
        entry_edges,
    } = *inputs;
    let mut changed = false;
    let Some(block) = cfg.blocks.get(&bn) else {
        return false;
    };
    let Some(term) = &block.terminator else {
        return false;
    };
    // A block that certainly raises never reaches its successors: only its
    // exception edges leave it.
    match term {
        _ if !exit.completes => {}
        Terminator::Goto { target, .. } => {
            let edge = (bn, *target);
            if !executable_edges.contains(&edge) {
                executable_edges.insert(edge);
                changed = true;
            }
            if cfg.blocks.contains_key(target) && executable_blocks.insert(*target) {
                changed = true;
            }
        }
        Terminator::Branch {
            condition,
            true_target,
            false_target,
            span,
            condition_base,
            ..
        } => {
            let Some(ssa_block) = ssa.blocks.get(&bn) else {
                return changed;
            };
            driver.explaining(*span);
            let decision = evaluate_branch_under(
                ssa_block,
                condition,
                values,
                ssa,
                BranchFold { grammar, driver },
                cfg.source_tokens_at(bn, usize::MAX).zip(*condition_base),
            );
            driver.explaining(None);
            let targets: Vec<BlockId> = match decision {
                Some(true) => vec![*true_target],
                Some(false) => vec![*false_target],
                // Optimistic (Wegman–Zadeck): while the condition may still
                // fold on a later sweep (a not-yet-computed operand, no
                // `Overdefined` one), open neither arm and let the fixpoint
                // retry — this is what detects loop-carried constant
                // conditions instead of pessimistically opening both arms
                // forever. The finalising pass forces both arms for any
                // branch still stuck this way.
                None if !finalizing
                    && branch_deferrable(ssa_block, condition, values, ssa, grammar) =>
                {
                    Vec::new()
                }
                None => vec![*true_target, *false_target],
            };
            for tgt in targets {
                let edge = (bn, tgt);
                if !executable_edges.contains(&edge) {
                    executable_edges.insert(edge);
                    changed = true;
                }
                if cfg.blocks.contains_key(&tgt) && executable_blocks.insert(tgt) {
                    changed = true;
                }
            }
        }
        // Observed completion has no normal successor and performs no second
        // operand evaluation. Original block exception edges remain below.
        Terminator::Return { .. } | Terminator::Complete { .. } => {}
    }
    // `try` exception edges sourced at `bn`: when `bn` is executable the
    // handler is reachable (a throw can occur in the body).
    for (from, to) in &cfg.exception_edges {
        // The edge from the block before a body opens with the body's first
        // command, which says whether the state before it can be thrown from.
        if *from != bn || entry_edges.contains(&(bn, *to)) {
            continue;
        }
        let edge = (bn, *to);
        if executable_edges.insert(edge) {
            changed = true;
        }
        if cfg.blocks.contains_key(to) && executable_blocks.insert(*to) {
            changed = true;
        }
    }
    changed
}

/// Post-fixpoint sweep that records every reachable branch whose
/// condition evaluated to a constant lattice value, and the definitions a
/// condition the shared engine decided left untouched
/// ([`record_condition_preserves`]). Extracted from [`sccp`].
fn collect_constant_branches(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    (values, narrowing): (&mut HashMap<ValueKey, LatticeValue>, Narrowing<'_>),
    executable_blocks: &HashSet<BlockId>,
    order: &[BlockId],
    fold: BranchFold<'_>,
    existence_exits: Option<&HashMap<BlockId, Vec<Existence>>>,
) -> Vec<ConstantBranch> {
    let mut constant_branches: Vec<ConstantBranch> = Vec::new();
    for bn in order {
        if !executable_blocks.contains(bn) {
            continue;
        }
        let Some(block) = cfg.blocks.get(bn) else {
            continue;
        };
        let Some(Terminator::Branch {
            condition,
            true_target,
            false_target,
            span: term_span,
            condition_base,
            ..
        }) = &block.terminator
        else {
            continue;
        };
        let Some(ssa_block) = ssa.blocks.get(bn) else {
            continue;
        };
        fold.driver.explaining(*term_span);
        // The condition reads the existence rung at the block's exit.
        if let Some(exit) = existence_exits.and_then(|exits| exits.get(bn)) {
            fold.driver.existence_enter(exit.clone());
        }
        let narrowed = narrowing.narrow(values, *bn);
        let decision = evaluate_branch_under(
            ssa_block,
            condition,
            values,
            ssa,
            fold,
            cfg.source_tokens_at(*bn, usize::MAX).zip(*condition_base),
        );
        restore_values(values, narrowed);
        fold.driver.existence_leave();
        fold.driver.explaining(None);
        if decision.is_some() {
            record_condition_preserves(cfg, ssa, *bn, fold.driver);
        }
        let cond_text = crate::expr_ast::expr_text(condition);
        let (true_name, false_name) = (
            cfg.block_name(*true_target).to_owned(),
            cfg.block_name(*false_target).to_owned(),
        );
        match decision {
            Some(true) => constant_branches.push(ConstantBranch {
                block: cfg.block_name(*bn).to_owned(),
                span: *term_span,
                condition: cond_text,
                value: true,
                taken_target: true_name,
                not_taken_target: false_name,
                kind: BranchFactKind::Applied,
            }),
            Some(false) => constant_branches.push(ConstantBranch {
                block: cfg.block_name(*bn).to_owned(),
                span: *term_span,
                condition: cond_text,
                value: false,
                taken_target: false_name,
                not_taken_target: true_name,
                kind: BranchFactKind::Applied,
            }),
            None => {}
        }
    }
    constant_branches
}

/// The post-passes over the settled lattice, block by block under the
/// refinements in force there: each executable `subst` call's template
/// plan, each opaque case list's selection, and the `Selected` fact of each
/// arm no member of a subject reaches, each in source order.
fn settled_statements(
    (driver, cfg, ssa): (&LatticeDriver<'_>, &CfgFunction, &SsaFunction),
    (values, narrowing): (&mut HashMap<ValueKey, LatticeValue>, Narrowing<'_>),
    executable_blocks: &HashSet<BlockId>,
) -> (
    Vec<TemplatePlanRecord>,
    Vec<SelectionRecord>,
    Vec<ConstantBranch>,
) {
    let (mut plans, mut selections, mut unreached) = (Vec::new(), Vec::new(), Vec::new());
    for (&block_id, block) in &ssa.blocks {
        if !executable_blocks.contains(&block_id) {
            continue;
        }
        let narrowed = narrowing.narrow(values, block_id);
        plans.extend(driver.template_plans_in(ssa, block, values));
        let (records, arms) = driver.selection_facts_in(cfg, ssa, (block_id, block), values);
        selections.extend(records);
        unreached.extend(arms);
        restore_values(values, narrowed);
    }
    plans.sort_by_key(|record: &TemplatePlanRecord| (record.span.start(), record.span.end()));
    selections.sort_by_key(|record: &SelectionRecord| (record.span.start(), record.span.end()));
    unreached.sort_by_key(|fact: &ConstantBranch| fact.span.map(|span| (span.start(), span.end())));
    (plans, selections, unreached)
}

/// The `<cond>` statement before a branch carries what the condition's
/// command substitutions may write ([`crate::ir::SyntheticMarker::Condition`]).
/// When the shared engine decided the condition, every nested command it
/// ran answered without a store — any other declines `StatefulNested` — so
/// each definition the statement reads before it writes (a conditional
/// writer's target, a read-modify-write's) holds the version it read
/// ([`SccpResult::preserved`]). Only the block's last statement, the one
/// right before the branch whose condition it summarises, speaks for it.
fn record_condition_preserves(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    bn: BlockId,
    driver: &LatticeDriver<'_>,
) {
    let (Some(block), Some(ssa_block)) = (cfg.blocks.get(&bn), ssa.blocks.get(&bn)) else {
        return;
    };
    let Some(Terminator::Branch {
        span: Some(branch), ..
    }) = &block.terminator
    else {
        return;
    };
    let Some(last) = ssa_block.statements.last() else {
        return;
    };
    let Statement::Call {
        tokens: Some(tokens),
        span,
        ..
    } = &last.statement
    else {
        return;
    };
    let summarises_branch = tokens.synthetic == Some(crate::ir::SyntheticMarker::Condition)
        && span.start() <= branch.start()
        && branch.end() <= span.end();
    if !summarises_branch {
        return;
    }
    for (&var, &ver) in &last.defs {
        if let Some(&prior) = last.uses.get(&var) {
            driver.record_preserved((var, ver), Some(prior));
        }
    }
}

/// Evaluate the lattice value produced by an SSA statement's
/// defs.
///
/// Focused subset: constant-assignment, expression-assignment via the
/// expression evaluator, the registry-declared value transfer of a typed
/// cell update, a synthetic loop header, or a command substitution, and a
/// conservative `Overdefined` fallback for everything else.
///
/// Holds no whole-module command view, so no resolved command answers: there
/// is no evidence that `[llength …]` or `incr` still means the builtin
/// rather than a user `proc` of that name. A caller with the fact uses
/// [`evaluate_def_with_folds`]; a run over a unit's own registry goes
/// through [`sccp_with_builtin_folds`].

/// [`evaluate_def`] with an optional registry builtin-fold context: when
/// `folds` is supplied, an `AssignValue` command-substitution RHS
/// additionally consults the registry `const_fold` engine — see
/// [`BuiltinFoldInputs`] — and every resolved head answers to the
/// whole-module trust fact. `None` is byte-identical to [`evaluate_def`].
///
/// A statement that defines several variables (`regexp … a b`) answers
/// for the first one its command names; the solver itself reads every
/// definition's own value.

/// What one statement's evaluation leaves in its definitions: the one
/// value a typed statement computes, which every definition takes, or a
/// call's value per definition, from its ordered stores.
pub(crate) enum DefValues {
    /// Every definition takes this value, and the folded type the
    /// evaluation that produced it states.
    Each(LatticeValue, Option<crate::value_transfer::FoldedType>),
    /// Each definition's own answer; a definition absent here widens.
    PerDef(Vec<crate::value_transfer::DefAnswer>),
    /// The statement certainly raises, where a throw leaves from: its
    /// definitions are what the stores that ran left.
    Raised(Box<RaisedDefs>),
}

/// A statement that certainly raises, after the stores that ran.
pub(crate) struct RaisedDefs {
    /// Each definition after those stores; `None` when nothing was stored,
    /// so every definition keeps what its place held.
    pub(crate) prefix: Option<Vec<crate::value_transfer::DefAnswer>>,
    /// How many stores ran before the error.
    pub(crate) written: usize,
}

impl DefValues {
    /// The per-definition answers, for the shapes that have them.
    fn answers(&self) -> Option<&[crate::value_transfer::DefAnswer]> {
        match self {
            Self::Each(..) => None,
            Self::PerDef(answers) => Some(answers),
            Self::Raised(raised) => raised.prefix.as_deref(),
        }
    }

    /// The value definition `key` takes.
    fn of(&self, key: ValueKey) -> LatticeValue {
        match self {
            Self::Each(value, _) => value.clone(),
            Self::PerDef(_) | Self::Raised(_) => self
                .answers()
                .and_then(|answers| answers.iter().find(|answer| answer.key == key))
                .map_or(LatticeValue::Overdefined, |answer| answer.value.clone()),
        }
    }

    /// The folded type definition `key` takes, when its evaluation states
    /// one.
    fn folded_of(&self, key: ValueKey) -> Option<crate::value_transfer::FoldedType> {
        match self {
            Self::Each(_, folded) => folded.clone(),
            Self::PerDef(_) | Self::Raised(_) => self
                .answers()
                .and_then(|answers| answers.iter().find(|answer| answer.key == key))
                .and_then(|answer| answer.folded.clone()),
        }
    }

    /// Whether an evaluated outcome's stores name definition `key`'s place
    /// ([`crate::value_transfer::DefAnswer::stated`]).
    fn stated(&self, key: ValueKey) -> bool {
        self.answers().is_some_and(|answers| {
            answers
                .iter()
                .any(|answer| answer.key == key && answer.stated)
        })
    }

    /// Whether the outcome left definition `key`'s place untouched
    /// ([`crate::value_transfer::DefAnswer::preserved`]).
    fn preserved(&self, key: ValueKey) -> bool {
        self.answers().is_some_and(|answers| {
            answers
                .iter()
                .any(|answer| answer.key == key && answer.preserved)
        })
    }

    /// The existence step definition `key` takes
    /// ([`crate::value_transfer::DefAnswer::existence`]); a statement the
    /// evaluation has no per-definition answer for widens, and one that
    /// raised before it stored anything leaves every place as it was.
    fn existence(&self, key: ValueKey) -> crate::value_transfer::ExistenceStep {
        use crate::value_transfer::ExistenceStep;
        match self {
            Self::Raised(raised) if raised.prefix.is_none() => ExistenceStep::PRESERVE,
            _ => self
                .answers()
                .and_then(|answers| answers.iter().find(|answer| answer.key == key))
                .map_or(ExistenceStep::UNKNOWN, |answer| answer.existence),
        }
    }
}

/// [`evaluate_def_with_folds`] under one run's driver, for every
/// definition of the statement: it is dispatched by its typed shape, and
/// every command-specific answer comes from the registry's declaration for
/// the resolved invocation (`docs/design/compiler/value-transfers.md`). No
/// command is recognised by its spelling here.
fn evaluate_defs_under<S: std::hash::BuildHasher>(
    stmt_ssa: &SsaStatement,
    values: &HashMap<ValueKey, LatticeValue, S>,
    ssa: &SsaFunction,
    driver: &LatticeDriver<'_>,
) -> DefValues {
    driver.explaining(Some(stmt_ssa.statement.span()));
    let value = evaluate_def_dispatch(stmt_ssa, values, ssa, driver);
    driver.explaining(None);
    value
}

/// The typed dispatch of [`evaluate_defs_under`].
fn evaluate_def_dispatch<S: std::hash::BuildHasher>(
    stmt_ssa: &SsaStatement,
    values: &HashMap<ValueKey, LatticeValue, S>,
    ssa: &SsaFunction,
    driver: &LatticeDriver<'_>,
) -> DefValues {
    let value = match &stmt_ssa.statement {
        // A typed assignment is `set`'s lowering, so it means nothing once
        // the module rebinds `set`.
        Statement::AssignConst { .. }
        | Statement::AssignExpr { .. }
        | Statement::AssignValue { .. }
            if !driver.typed_assignment_trusted() =>
        {
            LatticeValue::Overdefined
        }
        // A whole-word command substitution's value comes with the folded
        // type its route states.
        Statement::AssignValue {
            value,
            value_needs_backsubst,
            ..
        } => {
            let (value, folded) = fold_assign_value(
                value,
                *value_needs_backsubst,
                &stmt_ssa.uses,
                values,
                ssa,
                driver,
            );
            return DefValues::Each(value, folded);
        }
        // The value is a braced word's content by construction (the
        // lowering's other arm is a canonical integer), so it is read as
        // Tcl reads a braced word: backslash-newline collapses.
        Statement::AssignConst { value, .. } => driver
            .literal_value(value, TokenType::Str)
            .map_or(LatticeValue::Overdefined, |value| {
                LatticeValue::Const(parse_literal_value(&value))
            }),
        Statement::AssignExpr {
            expr,
            command_binding,
            expr_base,
            ..
        } => {
            let original = driver.original_expression_parent(&stmt_ssa.statement);
            return driver.evaluate_assign_expr(
                expr,
                command_binding.as_ref(),
                original.as_ref().zip(*expr_base),
                &stmt_ssa.uses,
                values,
                ssa,
            );
        }
        Statement::ExprEval {
            expr,
            command_binding,
            expr_base,
            ..
        } => {
            let original = driver.original_expression_parent(&stmt_ssa.statement);
            return driver.evaluate_expr_eval(
                expr,
                command_binding,
                original.as_ref().zip(*expr_base),
                &stmt_ssa.uses,
                values,
                ssa,
            );
        }
        Statement::Call { .. } => {
            return driver.evaluate_call(stmt_ssa, values, ssa, &stmt_ssa.uses);
        }
        Statement::Incr {
            name,
            amount,
            amount_braced,
            ..
        } => {
            return driver.evaluate_incr(
                name,
                amount.as_deref().map(|text| (text, *amount_braced)),
                &crate::value_transfer::named_defs(stmt_ssa, ssa),
                &stmt_ssa.uses,
                values,
                ssa,
            );
        }
        _ => LatticeValue::Overdefined,
    };
    DefValues::Each(value, None)
}

/// Resolve `$var` / `${var}` to a lattice value by looking up the
/// SSA version in `uses` and indexing `values`. Returns None when
/// the text isn't a simple var reference.
fn resolve_simple_var_ref<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
    text: &str,
    uses: &HashMap<Symbol, crate::ssa::Version, S1>,
    values: &HashMap<ValueKey, LatticeValue, S2>,
    ssa: &SsaFunction,
) -> Option<LatticeValue> {
    let key = simple_var_ref_key(text, uses, ssa)?;
    Some(values.get(&key).cloned().unwrap_or(LatticeValue::Unknown))
}

/// The SSA value a `$var` / `${var}` word reads at this statement, or
/// `None` when the text isn't a simple var reference.
fn simple_var_ref_key<S: std::hash::BuildHasher>(
    text: &str,
    uses: &HashMap<Symbol, crate::ssa::Version, S>,
    ssa: &SsaFunction,
) -> Option<ValueKey> {
    let name = if let Some(name) = text.strip_prefix("${").and_then(|s| s.strip_suffix('}')) {
        name
    } else {
        let name = text.strip_prefix('$')?;
        if name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b':')
        {
            name
        } else {
            return None;
        }
    };
    let sym = ssa.var_symbol(name)?;
    let ver = *uses.get(&sym)?;
    Some((sym, ver))
}

/// What a branch fold needs: the grammar its condition text was written
/// under, and the run's driver, whose services (and the target release's
/// fold policy) the condition is evaluated through.
#[derive(Clone, Copy)]
pub(crate) struct BranchFold<'a> {
    grammar: tcl_dialect::LexerGrammar,
    /// The run's value-transfer driver, whose services the condition is
    /// evaluated through.
    driver: &'a LatticeDriver<'a>,
}

/// Evaluate a branch condition.
///
/// The condition runs through the shared engine over the driver's services
/// under the effect-free nested policy, so a nested pure command resolves
/// (`if {[string length $acc] == 6}`), and with exactly one distinct finite
/// SSA value among its reads it runs once per member: every member true is
/// `Some(true)`, every member false `Some(false)`, and a mixed or undecided
/// answer is `None`. Two finite values decline as correlated. The reads
/// are the block's exit versions, and a variable the block never defines
/// — a parameter's caller-provided seed — reads its version 0. A `Raw`
/// operand reads a variable only as [`with_whole_variable_operands`] proves
/// it; any other `Raw` text leaves the condition undecided.
#[must_use]
pub(crate) fn evaluate_branch_under<S: std::hash::BuildHasher>(
    ssa_block: &crate::ssa::SsaBlock,
    condition: &ExprNode,
    values: &HashMap<ValueKey, LatticeValue, S>,
    ssa: &SsaFunction,
    fold: BranchFold<'_>,
    original: Option<(&crate::ir::CommandTokens, u32)>,
) -> Option<bool> {
    let mut uses: HashMap<Symbol, crate::ssa::Version> = ssa_block
        .exit_versions
        .iter()
        .map(|(&sym, &ver)| (sym, ver))
        .collect();
    let config = fold.driver.source_lexer_config();
    for name in condition.vars_element_qualified_with_config(config) {
        if let Some(sym) = ssa.var_symbol(&name) {
            uses.entry(sym).or_insert(0);
        }
    }
    let resolved = with_whole_variable_operands(condition, fold.grammar.braced_var);
    fold.driver.evaluate_condition(
        resolved.as_ref().unwrap_or(condition),
        original,
        &uses,
        values,
        ssa,
    )
}

/// `condition` with every `Raw` *operand* the variable-name owner proves is
/// exactly one variable reference read as that variable — the flattened
/// dispatch's whole-variable subject, which
/// `switch_subject_operand` keeps `Raw` so codegen loads the name intact.
/// `None` when no operand resolves. The proof is
/// [`crate::value_transfer::whole_variable_operand`] under `style`, the
/// document's `${…}` close rule; a root `Raw` is an unparsed condition, not
/// an operand, and stays undecided, as does any `Raw` text the owner does
/// not prove.
fn with_whole_variable_operands(
    condition: &ExprNode,
    style: tcl_dialect::BracedVarStyle,
) -> Option<ExprNode> {
    let operand = |node: &ExprNode| match node {
        ExprNode::Raw { text } => {
            crate::value_transfer::whole_variable_operand(text, style).map(|name| ExprNode::Var {
                text: text.clone(),
                name: name.to_owned(),
                start: 0,
                end: 0,
            })
        }
        other => with_whole_variable_operands(other, style),
    };
    let or_same = |resolved: Option<ExprNode>, node: &ExprNode| {
        Box::new(resolved.unwrap_or_else(|| node.clone()))
    };
    match condition {
        ExprNode::Binary { op, left, right } => {
            let (l, r) = (operand(left), operand(right));
            (l.is_some() || r.is_some()).then(|| ExprNode::Binary {
                op: *op,
                left: or_same(l, left),
                right: or_same(r, right),
            })
        }
        ExprNode::Unary { op, operand: inner } => operand(inner).map(|inner| ExprNode::Unary {
            op: *op,
            operand: Box::new(inner),
        }),
        ExprNode::Ternary {
            condition: test,
            true_branch,
            false_branch,
        } => {
            let (c, t, f) = (operand(test), operand(true_branch), operand(false_branch));
            (c.is_some() || t.is_some() || f.is_some()).then(|| ExprNode::Ternary {
                condition: or_same(c, test),
                true_branch: or_same(t, true_branch),
                false_branch: or_same(f, false_branch),
            })
        }
        ExprNode::Call {
            function,
            args,
            start,
            end,
        } => {
            let resolved: Vec<Option<ExprNode>> = args.iter().map(operand).collect();
            resolved
                .iter()
                .any(Option::is_some)
                .then(|| ExprNode::Call {
                    function: function.clone(),
                    args: resolved
                        .into_iter()
                        .zip(args)
                        .map(|(resolved, arg)| resolved.unwrap_or_else(|| arg.clone()))
                        .collect(),
                    start: *start,
                    end: *end,
                })
        }
        _ => None,
    }
}

/// Extract iteration-variable elements from a foreach list arg
/// that is a literal (no `$` / `[` substitution).
///
/// `list_text` must already be delimiter-stripped by the segmenter (the shape
/// `Statement::Foreach`'s `list_arg` is built in, and the shape every caller
/// of this function must pass) — a second strip here would wrongly peel a
/// single-element list like `{{a b} {c d}}` (segmented to `{a b} {c d}`) down
/// to the elements `a` and `b}`. See `cfg_builder::list_literal_nonempty` and
/// `analyser::bounds_checks` for the same contract.
///
/// Behaviour:
/// - Strip whitespace.
/// - Split on Tcl list semantics (`Tcl_SplitList`), not ASCII whitespace.
/// - Returns `None` for anything that starts with `$` or `[` so
///   callers fall through to
///   [`resolve_foreach_list_via_lattice`].
#[must_use]
pub fn extract_foreach_elements(
    list_text: &str,
    rules: tcl_syntax::word_rules::WordValueRules,
) -> Option<Vec<String>> {
    let stripped = list_text.trim();
    if stripped.is_empty() {
        return Some(Vec::new());
    }
    if stripped.starts_with('[') || stripped.starts_with('$') {
        return None;
    }
    // List-aware split (Tcl_SplitList semantics): a nested-brace list like
    // `a {b c} d` yields the three elements `a`, `b c`, `d` — not the four
    // whitespace runs `a`, `{b`, `c}`, `d` a naive split would produce.
    Some(split_list_values(stripped, rules))
}

/// Resolve `$var` / `${var}` to a `Vec<String>` of list elements
/// via the SCCP lattice. Returns `None` when the operand is not a
/// simple var reference or its lattice value is not a
/// Const(String).
#[must_use]
pub fn resolve_foreach_list_via_lattice<S1, S2>(
    list_text: &str,
    uses: &HashMap<Symbol, crate::ssa::Version, S1>,
    values: &HashMap<ValueKey, LatticeValue, S2>,
    ssa: &SsaFunction,
    rules: tcl_syntax::word_rules::WordValueRules,
) -> Option<Vec<String>>
where
    S1: std::hash::BuildHasher,
    S2: std::hash::BuildHasher,
{
    let stripped = list_text.trim();
    let name = if let Some(name) = stripped
        .strip_prefix("${")
        .and_then(|s| s.strip_suffix('}'))
    {
        name
    } else {
        let name = stripped.strip_prefix('$')?;
        if name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b':')
        {
            name
        } else {
            return None;
        }
    };
    let sym = ssa.var_symbol(name)?;
    let ver = uses.get(&sym).copied()?;
    match values.get(&(sym, ver))? {
        // The lattice value is the variable's runtime string; splitting it as a
        // `foreach` list uses Tcl list semantics (nested-brace aware), not a
        // whitespace split.
        LatticeValue::Const(ConstValue::String(s)) => Some(split_list_values(s, rules)),
        _ => None,
    }
}

// AssignValue folding

/// Fold the RHS of an `AssignValue` statement to a lattice value.
///
/// Covers three tiers:
/// 1. **Plain literal** — no `$` / `[` → `Const(parse_literal_value)` of
///    the word's value, kept exactly: a value is a value, not a source token
///    to trim. The statement carries the word's spelling, so a word the
///    lowering marked `value_needs_backsubst` (a bare or quoted word with an
///    escape) is cooked under the document's escape grammar — `set s
///    "a\tb"` holds three characters — and a spelling that holds a
///    backslash the lowering did not account for has no known value.
/// 2. **Simple var reference** `$x` / `${x}` → lattice lookup.
/// 3. **Command substitution** `[cmd args…]` → the resolved command's
///    declared route through the value-transfer driver
///    ([`crate::value_transfer`]), then — when the driver holds the trust
///    fact — the registry const-fold engine ([`BuiltinFoldInputs`]).
///
/// Anything else widens to `Overdefined`.
fn fold_assign_value<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
    value: &str,
    needs_backsubst: bool,
    uses: &HashMap<Symbol, crate::ssa::Version, S1>,
    values: &HashMap<ValueKey, LatticeValue, S2>,
    ssa: &SsaFunction,
    driver: &LatticeDriver<'_>,
) -> (LatticeValue, Option<crate::value_transfer::FoldedType>) {
    // Plain literal.
    if !value.contains('$') && !value.contains('[') {
        let cooked = if needs_backsubst {
            driver.literal_value(value, TokenType::Esc)
        } else if value.contains('\\') {
            None
        } else {
            Some(std::borrow::Cow::Borrowed(value))
        };
        let value = cooked.map_or(LatticeValue::Overdefined, |value| {
            LatticeValue::Const(parse_literal_value(&value))
        });
        return (value, None);
    }
    // Simple var reference: a copy shares its source's value, and the
    // source's folded type with it.
    if let Some(resolved) = resolve_simple_var_ref(value, uses, values, ssa) {
        let folded = simple_var_ref_key(value, uses, ssa).and_then(|key| driver.folded_of(key));
        return (resolved, folded);
    }
    // Command substitution.
    if value.starts_with('[')
        && value.ends_with(']')
        && let Some(folded) = driver.fold_cmd_subst(value, uses, values, ssa)
    {
        return folded;
    }
    (LatticeValue::Overdefined, None)
}

/// The value ingress for a literal: the text kept exactly, classified as
/// [`ConstValue::Int`] only when the canonical integer spelling round-trips
/// (`str(int(s)) == s`). A leading-zero literal such as `"08"` or `"010"`
/// parses as 8 / 10 but does not round-trip, so it stays a string —
/// preserving the identity SCCP needs to compare it correctly under each
/// dialect's leading-zero rule (octal in tcl8.x, decimal in tcl9.0);
/// `"+5"` and `"-0"` are kept as strings for the same reason. Nothing is
/// trimmed: `set p { again}` holds ` again`, and a rewrite that reads the
/// lattice sees the space (#2052). The classification rule is the
/// registry's ([`ExactValue::from_literal`]); this is its lattice
/// projection.
#[must_use]
pub fn parse_literal_value(text: &str) -> ConstValue {
    crate::value_transfer::exact_to_const(&ExactValue::from_literal(text))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Whitespace inside a Tcl word is data, so the lattice keeps the exact
    /// spelling (#2052).
    ///
    /// Trimming corrupted every consumer: `set p { again}` reached the lattice
    /// as `again`, so `append s $p` was rewritten to `append s again` and the
    /// program printed `helloagain` where tclsh 9.0.4 prints `hello again`.
    /// `string length $p` folded to 5 against the true 6.
    #[test]
    fn a_literal_keeps_its_surrounding_whitespace() {
        assert_eq!(
            parse_literal_value(" again"),
            ConstValue::String(" again".to_owned()),
        );
        assert_eq!(
            parse_literal_value("trailing "),
            ConstValue::String("trailing ".to_owned()),
        );
        // An integer with whitespace around it is not the integer: rendering
        // it back as `5` would drop the space just as surely.
        assert_eq!(
            parse_literal_value(" 5"),
            ConstValue::String(" 5".to_owned()),
        );
        // The bare integer still folds.
        assert_eq!(parse_literal_value("5"), ConstValue::Int(5));
        assert_eq!(parse_literal_value("-17"), ConstValue::Int(-17));
    }
    use crate::cfg::{Block, BlockId, Function, Terminator};
    use crate::expr_ast::ExprNode;

    fn registry() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    /// [`evaluate_def_with_folds`] under a module that mutates no command
    /// binding — the stance the shared per-unit lattice takes for an
    /// untouched file ([`FoldTrust::ObservedBindings`] over
    /// [`crate::command_binding::ModuleCommandMutations`]'s `Default`).
    ///
    /// Plain [`evaluate_def`] holds no whole-module command view at all and
    /// therefore folds no builtin command substitution, so a fold-arm test
    /// has to state the trust fact it folds under.
    fn evaluate_pristine<S: std::hash::BuildHasher>(
        stmt: &SsaStatement,
        values: &HashMap<ValueKey, LatticeValue, S>,
        ssa: &SsaFunction,
        policy: FoldPolicy,
    ) -> LatticeValue {
        evaluate_def_with_folds(
            stmt,
            values,
            ssa,
            policy,
            Some(BuiltinFoldInputs {
                source_metadata_input: None,
                registry: &registry(),
                mutations: &crate::command_binding::ModuleCommandMutations::default(),
                dialect: None,
                defining_class: None,
                registry_engine: false,
                trust: FoldTrust::ObservedBindings,
                proven_pure_parameters: false,
            }),
        )
    }

    /// Convenience wrapper over [`sccp`] for tests with no `Module` in
    /// hand — no traced variables, no dynamic variable trace.
    fn sccp_no_traces(
        cfg: &CfgFunction,
        ssa: &SsaFunction,
        param_constants: Option<&HashMap<(String, crate::ssa::Version), LatticeValue>>,
        policy: FoldPolicy,
    ) -> SccpResult {
        sccp(
            cfg,
            ssa,
            param_constants,
            policy,
            TraceInputs {
                source_metadata_input: None,
                registry: &registry(),
                traced_variables: &BTreeSet::new(),
                has_dynamic_variable_trace: false,
                deferred_writes: &crate::ir::NO_DEFERRED_WRITES,
                analysis_context: None,
                existence: None,
            },
        )
    }

    /// [`sccp_no_traces`] under a module that mutates no command binding —
    /// the shared per-unit lattice's stance for an untouched file, as
    /// [`evaluate_pristine`] states it for one statement. Without the trust
    /// fact no resolved command answers, so a test of a route states it.
    fn sccp_pristine(cfg: &CfgFunction, ssa: &SsaFunction, policy: FoldPolicy) -> SccpResult {
        let registry = registry();
        sccp_with_builtin_folds(
            cfg,
            ssa,
            None,
            policy,
            &HashSet::new(),
            TraceInputs {
                source_metadata_input: None,
                registry: &registry,
                traced_variables: &BTreeSet::new(),
                has_dynamic_variable_trace: false,
                deferred_writes: &crate::ir::NO_DEFERRED_WRITES,
                analysis_context: None,
                existence: None,
            },
            Some(BuiltinFoldInputs {
                source_metadata_input: None,
                registry: &registry,
                mutations: &crate::command_binding::ModuleCommandMutations::default(),
                dialect: None,
                defining_class: None,
                registry_engine: false,
                trust: FoldTrust::ObservedBindings,
                proven_pure_parameters: false,
            }),
        )
    }

    /// Intern `name` and insert a fresh block; returns its [`BlockId`].
    fn block(f: &mut Function, name: &str) -> BlockId {
        let id = f.intern_block(name);
        f.blocks.insert(id, Block::new(name));
        id
    }

    fn id_of(f: &Function, name: &str) -> BlockId {
        f.block_id(name).expect("interned")
    }

    fn goto(target: BlockId) -> Terminator {
        Terminator::Goto { target, span: None }
    }

    fn branch(cond: ExprNode, tt: BlockId, ft: BlockId) -> Terminator {
        Terminator::Branch {
            condition: cond,
            true_target: tt,
            false_target: ft,
            span: None,
            condition_base: None,
        }
    }

    fn literal(text: &str) -> ExprNode {
        ExprNode::Literal {
            text: text.into(),
            start: 0,
            end: u32::try_from(text.len()).unwrap_or(0),
        }
    }

    // Join.

    #[test]
    fn join_unknown_absorbs() {
        let c = LatticeValue::Const(ConstValue::Int(7));
        assert_eq!(join(&LatticeValue::Unknown, &c), c);
        assert_eq!(join(&c, &LatticeValue::Unknown), c);
    }

    #[test]
    fn join_overdefined_is_absorbing() {
        let c = LatticeValue::Const(ConstValue::Int(7));
        assert_eq!(
            join(&LatticeValue::Overdefined, &c),
            LatticeValue::Overdefined
        );
        assert_eq!(
            join(&c, &LatticeValue::Overdefined),
            LatticeValue::Overdefined
        );
    }

    #[test]
    fn join_identical_const_stays_const() {
        let c = LatticeValue::Const(ConstValue::Int(7));
        assert_eq!(join(&c, &c), c);
    }

    #[test]
    fn join_distinct_consts_widens_to_set() {
        let a = LatticeValue::Const(ConstValue::Int(1));
        let b = LatticeValue::Const(ConstValue::Int(2));
        let merged = join(&a, &b);
        match merged {
            LatticeValue::ConstSet(ref vs) => {
                assert_eq!(vs.len(), 2);
            }
            other => panic!("expected ConstSet, got {other:?}"),
        }
    }

    #[test]
    fn join_existing_set_absorbs_member() {
        let a = LatticeValue::ConstSet(vec![ConstValue::Int(1), ConstValue::Int(2)]);
        let b = LatticeValue::Const(ConstValue::Int(1));
        // Adding a member already in the set returns `old` unchanged.
        assert_eq!(join(&a, &b), a);
    }

    #[test]
    fn join_widens_large_sets_to_overdefined() {
        let big: Vec<ConstValue> = (0..MAX_CONSTSET_SIZE)
            .map(|i| ConstValue::Int(i64::try_from(i).unwrap()))
            .collect();
        let a = LatticeValue::ConstSet(big);
        let b = LatticeValue::Const(ConstValue::Int(999));
        assert_eq!(join(&a, &b), LatticeValue::Overdefined);
    }

    #[test]
    fn set_value_tracks_change() {
        let mut values: HashMap<ValueKey, LatticeValue> = HashMap::new();
        let key: ValueKey = (Symbol(0), 1);
        assert!(set_value(
            &mut values,
            key,
            &LatticeValue::Const(ConstValue::Int(1))
        ));
        assert!(!set_value(
            &mut values,
            key,
            &LatticeValue::Const(ConstValue::Int(1))
        ));
        assert!(set_value(
            &mut values,
            key,
            &LatticeValue::Const(ConstValue::Int(2))
        ));
    }

    // Predecessors + cfg_order.

    #[test]
    fn predecessors_simple_chain() {
        let mut f = Function::new("::top", "a");
        let a = f.entry;
        let b = block(&mut f, "b");
        f.blocks.get_mut(&a).unwrap().terminator = Some(goto(b));
        f.blocks.get_mut(&b).unwrap().terminator = Some(Terminator::Return {
            value: None,
            value_word: None,
            span: None,
            expr: None,
            expr_base: None,
            tokens: None,
            braced: false,
        });
        let p = compute_predecessors(&f);
        assert!(p.get(&b).unwrap().contains(&a));
        assert!(p.get(&a).is_none_or(HashSet::is_empty));
    }

    #[test]
    fn cfg_order_starts_at_entry() {
        let mut f = Function::new("::top", "entry");
        let entry = f.entry;
        let t = block(&mut f, "t");
        let e = block(&mut f, "e");
        let join = block(&mut f, "join");
        f.blocks.get_mut(&entry).unwrap().terminator = Some(branch(literal("1"), t, e));
        f.blocks.get_mut(&t).unwrap().terminator = Some(goto(join));
        f.blocks.get_mut(&e).unwrap().terminator = Some(goto(join));
        f.blocks.get_mut(&join).unwrap().terminator = Some(Terminator::Return {
            value: None,
            value_word: None,
            span: None,
            expr: None,
            expr_base: None,
            tokens: None,
            braced: false,
        });
        let order = cfg_order(&f);
        assert_eq!(order[0], entry);
        // join must appear after both branches.
        let join_pos = order.iter().position(|b| *b == join).unwrap();
        let t_pos = order.iter().position(|b| *b == t).unwrap();
        let e_pos = order.iter().position(|b| *b == e).unwrap();
        assert!(join_pos > t_pos);
        assert!(join_pos > e_pos);
    }

    /// A static-body `uplevel 0` is an `UpFrame`, not a generic
    /// barrier, but its body can create a local in the frame whose existence
    /// query follows. The whole-function existence fold must abstain just as
    /// it does for `Barrier`.
    #[test]
    fn existence_fold_abstains_for_a_live_upframe() {
        let registry = CommandRegistry::build_default();
        let cu = crate::compilation_unit::CompilationUnit::build_for(
            "proc f {} { uplevel 0 { set created 1 }; if {[info exists created]} { return yes } else { return no } }",
            &registry,
            false,
        );
        let f = cu.function("::f").expect("procedure analysed");
        assert!(
            f.cfg
                .blocks
                .values()
                .flat_map(|block| block.statements.iter())
                .any(|statement| matches!(statement, Statement::UpFrame { .. })),
            "the regression requires the static uplevel lowering path"
        );
        assert!(
            f.sccp.constant_branches.is_empty(),
            "UpFrame may define `created`, so info exists must not fold: {:?}",
            f.sccp.constant_branches
        );
    }

    /// `set x 1; unset x; info exists x` decides 0 inside the fixed point:
    /// the query reads the rung, the branch is an `Applied` fact,
    /// and its true arm is unreachable. tclsh 8.4 to 9.1 print `no`.
    #[test]
    fn set_unset_info_exists_decides_zero() {
        let registry = CommandRegistry::build_default();
        let cu = crate::compilation_unit::CompilationUnit::build_for(
            "proc f {} { set x 1; unset x; if {[info exists x]} { puts yes } else { puts no } }",
            &registry,
            false,
        );
        let f = cu.function("::f").expect("procedure analysed");
        let decided = f
            .sccp
            .constant_branches
            .iter()
            .find(|branch| branch.condition == "[info exists x]")
            .unwrap_or_else(|| panic!("the query decides: {:?}", f.sccp.constant_branches));
        assert!(!decided.value);
        assert_eq!(decided.kind, BranchFactKind::Applied);
        let dead = f.cfg.block_id(&decided.not_taken_target).expect("the arm");
        assert!(!f.sccp.executable_blocks.contains(&dead));
    }

    /// The existence facts each `puts` reading `name` finds, in source
    /// order.
    fn puts_reads(f: &crate::compilation_unit::FunctionUnit, name: &str) -> Vec<Existence> {
        let symbol = f.ssa.var_symbol(name).expect("the place is an SSA symbol");
        let mut reads: Vec<(u32, Existence)> = f
            .sccp
            .existence_reads
            .iter()
            .filter(|((_, _, read), _)| *read == symbol)
            .filter_map(|(&(block, index, _), &fact)| {
                let statement = f
                    .cfg
                    .blocks
                    .get(&block)?
                    .statements
                    .get(usize::try_from(index).ok()?)?;
                matches!(statement, Statement::Call { command, .. }
                    if command.trim_start_matches("::") == "puts")
                .then(|| (statement.span().start(), fact))
            })
            .collect();
        reads.sort_by_key(|&(start, _)| start);
        reads.into_iter().map(|(_, fact)| fact).collect()
    }

    /// The existence guard refines its edges: on a may-bound place
    /// the true edge of `[info exists x]` carries `Bound(Either)` and the
    /// false edge `Unbound`, `!` swaps the two, and past the merge the
    /// place is may-bound again. A `global` alias is externally mutable and
    /// never refined: both its reads stay may-bound.
    #[test]
    fn the_existence_guard_refines_its_edges() {
        let registry = CommandRegistry::build_default();
        let cu = crate::compilation_unit::CompilationUnit::build_for(
            "proc f {c} { if {$c} { set x 1 }; if {[info exists x]} { puts $x } else { puts none }; puts $x }\n\
             proc g {c} { if {$c} { set x 1 }; if {![info exists x]} { puts none } else { puts $x }; puts $x }\n\
             proc h {} { global x; if {[info exists x]} { puts $x } else { puts none }; puts $x }\n",
            &registry,
            false,
        );
        let h = cu.function("::h").expect("procedure analysed");
        assert!(h.sccp.refinements.is_empty(), "{:?}", h.sccp.refinements);
        assert_eq!(
            puts_reads(h, "x"),
            vec![Existence::MayBound, Existence::MayBound]
        );
        for proc in ["::f", "::g"] {
            let f = cu.function(proc).expect("procedure analysed");
            assert_eq!(
                puts_reads(f, "x"),
                vec![Existence::Bound(BindingKind::Either), Existence::MayBound],
                "{proc}: the guarded read is bound, the read past the merge may-bound"
            );
            assert_eq!(
                f.sccp.refinements.len(),
                2,
                "{proc}: {:?}",
                f.sccp.refinements
            );
            let unbound = f
                .sccp
                .refinements
                .iter()
                .find(|refinement| {
                    refinement.fact == FactView::Domain(DomainFact::Existence(Existence::Unbound))
                })
                .unwrap_or_else(|| panic!("{proc}: {:?}", f.sccp.refinements));
            assert_eq!(
                f.sccp.existence_at(unbound.edge.1, unbound.key),
                Some(Existence::Unbound),
                "{proc}: the other arm holds the place unbound"
            );
        }
    }

    /// `C1 && C2` refines by both answers only when `C2`, which runs after
    /// `C1`, changes no place: `[info exists x] && [info exists y]` binds
    /// both, `[otherproc] && [info exists x]` binds `x`, and `[info exists
    /// x] && [otherproc]` binds nothing — the procedure may unset `x`
    /// through an alias before the edge is taken.
    #[test]
    fn an_impure_operand_ends_the_facts_before_it() {
        let registry = CommandRegistry::build_default();
        let cu = crate::compilation_unit::CompilationUnit::build_for(
            "proc f {c} { if {$c} { set x 1; set y 1 }; if {[info exists x] && [info exists y]} { puts $x$y } }\n\
             proc g {c} { if {$c} { set x 1 }; if {[otherproc] && [info exists x]} { puts $x } }\n\
             proc h {c} { if {$c} { set x 1 }; if {[info exists x] && [otherproc]} { puts $x } }\n",
            &registry,
            false,
        );
        let bound = vec![Existence::Bound(BindingKind::Either)];
        let f = cu.function("::f").expect("procedure analysed");
        assert_eq!(puts_reads(f, "x"), bound, "::f x");
        assert_eq!(puts_reads(f, "y"), bound, "::f y");
        let g = cu.function("::g").expect("procedure analysed");
        assert_eq!(puts_reads(g, "x"), bound, "::g");
        let h = cu.function("::h").expect("procedure analysed");
        assert_eq!(puts_reads(h, "x"), vec![Existence::MayBound], "::h");
    }

    /// An externally mutable place is never refined: the guard on
    /// `::errorInfo`, at the top level and in a procedure, and on a
    /// `TclOO` instance variable's `if {[info exists x]} {return $x}` —
    /// spelled with absolute heads, which keep a method body analysable
    /// (its namespace is the receiver's) — leaves each read may-bound, since
    /// a call to a procedure the module cannot see may unset the global or
    /// the object's variable without any barrier.
    #[test]
    fn an_externally_mutable_place_is_never_refined() {
        let registry = CommandRegistry::build_default();
        let cu = crate::compilation_unit::CompilationUnit::build_for(
            "if {[info exists ::errorInfo]} {puts $::errorInfo}\n\
             proc p {} { if {[info exists ::errorInfo]} {puts $::errorInfo} }\n\
             oo::class create C {\n variable x\n \
             method m {} { ::if {[::info exists x]} {::return $x}; ::return none }\n}\n",
            &registry,
            false,
        );
        for unit in ["::top", "::p"] {
            let f = cu.function(unit).expect("unit analysed");
            assert!(
                f.sccp.refinements.is_empty(),
                "{unit}: {:?}",
                f.sccp.refinements
            );
            assert_eq!(
                puts_reads(f, "::errorInfo"),
                vec![Existence::MayBound],
                "{unit}"
            );
        }
        let m = cu.methods.values().next().expect("the method analysed");
        assert!(m.sccp.refinements.is_empty(), "{:?}", m.sccp.refinements);
        let x = m.ssa.var_symbol("x").expect("x");
        let returned: Vec<Existence> = m
            .cfg
            .blocks
            .iter()
            .filter_map(|(&id, block)| match &block.terminator {
                Some(Terminator::Return {
                    value: Some(value), ..
                }) if value == "${x}" => m.sccp.existence_at_exit(id, x),
                _ => None,
            })
            .collect();
        assert_eq!(returned, vec![Existence::MayBound]);
    }

    /// A `return` that leaves the procedure is a barrier that writes
    /// nothing, so the values its words read keep what the frame proves:
    /// `es` is `abc` at `return -code error -errorinfo $es msg`, where an
    /// opaque barrier, `eval $script`, widens it.
    #[test]
    fn a_return_that_leaves_the_procedure_widens_nothing() {
        let registry = CommandRegistry::build_default();
        let value_of_es = |body: &str| {
            let source = format!("proc r {{script}} {{\n    set es abc\n    {body}\n}}\n");
            let cu = crate::compilation_unit::CompilationUnit::build_for(&source, &registry, false);
            let r = cu.function("::r").expect("the procedure analysed");
            let es = r.ssa.var_symbol("es").expect("the variable");
            r.sccp.values.get(&(es, 1)).cloned()
        };
        assert_eq!(
            value_of_es("return -code error -errorinfo $es msg"),
            Some(LatticeValue::Const(ConstValue::String("abc".to_owned())))
        );
        assert_eq!(
            value_of_es("eval $script; return $es"),
            Some(LatticeValue::Overdefined)
        );
    }

    /// A refinement never survives a barrier: a local its guard
    /// refines bound reads bound before `eval $script`, which lowers to a
    /// barrier, and may-bound after it — the script may have unset it.
    #[test]
    fn a_barrier_ends_the_refinement() {
        let registry = CommandRegistry::build_default();
        let cu = crate::compilation_unit::CompilationUnit::build_for(
            "proc m {script c} { if {$c} {set x 1}; if {[info exists x]} { puts $x; eval $script; puts $x } }\n",
            &registry,
            false,
        );
        let m = cu.function("::m").expect("the procedure analysed");
        assert!(
            m.cfg
                .blocks
                .values()
                .flat_map(|block| block.statements.iter())
                .any(|statement| matches!(statement, Statement::Barrier { .. })),
            "the witness needs `eval $script` to lower to a barrier"
        );
        assert_eq!(
            puts_reads(m, "x"),
            vec![Existence::Bound(BindingKind::Either), Existence::MayBound]
        );
    }

    /// A special variable of the initial global frame is never refined:
    /// at the top level of a Tcl 8.6 script `errorCode` enters
    /// may-bound — startup binds it only under 8.4 — and `if {![info exists
    /// errorCode]}` refines neither edge, since the host, or any command
    /// that raises, may set it without a barrier.
    #[test]
    fn a_negated_guard_never_refines_a_special_variable() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let cu = crate::compilation_unit::CompilationUnit::build_for_dialect(
            "if {![info exists errorCode]} { puts none } else { puts $errorCode }\nputs $errorCode\n",
            registry,
            false,
            "tcl8.6",
        );
        let top = cu.function("::top").expect("top level analysed");
        assert!(
            top.sccp.refinements.is_empty(),
            "{:?}",
            top.sccp.refinements
        );
        assert_eq!(
            puts_reads(top, "errorCode"),
            vec![Existence::MayBound, Existence::MayBound]
        );
    }

    /// Each refinement of `f`, as the edge it sits on (`true` for the
    /// branch's true edge), the place and the fact spelled for comparison.
    fn refinement_rows(f: &crate::compilation_unit::FunctionUnit) -> Vec<(bool, String, String)> {
        f.sccp
            .refinements
            .iter()
            .map(|refinement| {
                let polarity = matches!(
                    f.cfg.blocks.get(&refinement.edge.0).and_then(|block| block.terminator.as_ref()),
                    Some(Terminator::Branch { true_target, .. }) if *true_target == refinement.edge.1
                );
                let text = |value: &ExactValue| String::from_utf8_lossy(&value.bytes).into_owned();
                let fact = match &refinement.fact {
                    FactView::Exact(value, _) => format!("= {}", text(value)),
                    FactView::Finite(values, _) => format!(
                        "in {}",
                        values.iter().map(text).collect::<Vec<_>>().join(" ")
                    ),
                    FactView::Domain(DomainFact::Type {
                        intrep: Some(intrep),
                        ..
                    }) => format!("type {intrep:?}"),
                    FactView::Domain(DomainFact::Range {
                        lo: Some(lo),
                        hi: Some(hi),
                    }) if lo == hi => format!("range {lo}"),
                    FactView::Domain(DomainFact::Range { lo, hi }) => {
                        let bound = |bound: &Option<i64>| bound.map(|n| n.to_string());
                        format!(
                            "range {}..{}",
                            bound(lo).unwrap_or_default(),
                            bound(hi).unwrap_or_default()
                        )
                    }
                    FactView::Domain(DomainFact::Existence(existence)) => {
                        format!("exists {existence:?}")
                    }
                    other => format!("{other:?}"),
                };
                let place = f
                    .sccp
                    .place_name(&f.ssa, refinement.key.0)
                    .unwrap_or("?")
                    .to_owned();
                (polarity, place, fact)
            })
            .collect()
    }

    /// The refinements a case expects: its edge, place and fact.
    type Rows = &'static [(bool, &'static str, &'static str)];

    /// Each case of the per-shape test: the procedure, its source and the
    /// refinements it expects.
    const SHAPE_CASES: &[(&str, &str, Rows)] = &[
        (
            "a1",
            "proc a1 {x} {if {$x eq \"a\"} {puts $x}}",
            &[(true, "x", "= a")],
        ),
        (
            "a2",
            "proc a2 {x} {if {$x ne \"a\"} {puts $x}}",
            &[(false, "x", "= a")],
        ),
        (
            "a3",
            "proc a3 {x} {if {\"a\" eq $x} {puts $x}}",
            &[(true, "x", "= a")],
        ),
        (
            "a4",
            "proc a4 {x} {if {$x == \"abc\"} {puts $x}}",
            &[(true, "x", "= abc")],
        ),
        (
            "a5",
            "proc a5 {x} {if {$x == 1} {puts $x}}",
            &[(true, "x", "type Numeric"), (true, "x", "range 1")],
        ),
        (
            "a6",
            "proc a6 {x} {if {$x != 1.5} {puts $x}}",
            &[(false, "x", "type Numeric")],
        ),
        (
            "a7",
            "proc a7 {x} {if {$x in {a b a}} {puts $x}}",
            &[(true, "x", "in a b")],
        ),
        (
            "a8",
            "proc a8 {x} {if {$x ni {a}} {puts $x}}",
            &[(false, "x", "= a")],
        ),
        (
            "a9",
            "proc a9 {x} {if {[string is integer -strict $x]} {puts $x}}",
            &[(true, "x", "type Int")],
        ),
        (
            "a10",
            "proc a10 {x} {if {[string is integer $x]} {puts $x}}",
            &[],
        ),
        (
            "a11",
            "proc a11 {x} {if {!($x eq \"a\")} {puts $x}}",
            &[(false, "x", "= a")],
        ),
        (
            "a12",
            "proc a12 {x y} {if {$x eq \"a\" && $y eq \"b\"} {puts $x$y}}",
            &[(true, "x", "= a"), (true, "y", "= b")],
        ),
        (
            "a13",
            "proc a13 {x y} {if {$x eq \"a\" || $y eq \"b\"} {puts $x$y}}",
            &[],
        ),
        (
            "a14",
            "proc a14 {x y} {if {$x ne \"a\" || $y ne \"b\"} {puts $x$y}}",
            &[(false, "x", "= a"), (false, "y", "= b")],
        ),
        (
            "a15",
            "proc a15 {x} {if {$x eq \"a\" && [foo]} {puts $x}}",
            &[],
        ),
        (
            "a16",
            "proc a16 {x y} {if {$x} {puts $x}; if {$x eq $y} {puts $x$y}}",
            &[],
        ),
        (
            "a17",
            "proc a17 {x} {switch -- $x {a {puts $x} b {puts $x}}}",
            &[(true, "x", "= a"), (true, "x", "= b")],
        ),
        (
            "a18",
            "proc a18 {x} {switch -glob -- $x {a* {puts $x}}}",
            &[],
        ),
        ("a19", "proc a19 {} {if {$::g eq \"a\"} {puts $::g}}", &[]),
        (
            "a20",
            "proc a20 {} {global g; if {$g eq \"a\"} {puts $g}}",
            &[],
        ),
        (
            "a21",
            "proc a21 {a} {if {$a(k) eq \"z\"} {puts $a(k)}}",
            &[],
        ),
        (
            "a22",
            "proc a22 {x} {if {$x < 5} {puts $x}}",
            &[(true, "x", "range ..4"), (false, "x", "range 5..")],
        ),
        (
            "a23",
            "proc a23 {x} {if {5 <= $x} {puts $x}}",
            &[(true, "x", "range 5.."), (false, "x", "range ..4")],
        ),
        (
            "a24",
            "proc a24 {x} {if {$x > 0x10} {puts $x}}",
            &[(true, "x", "range 17.."), (false, "x", "range ..16")],
        ),
        (
            "a25",
            "proc a25 {x} {if {$x >= 010} {puts $x}}",
            &[(true, "x", "range 8.."), (false, "x", "range ..7")],
        ),
        ("a26", "proc a26 {x} {if {$x < false} {puts $x}}", &[]),
        ("a27", "proc a27 {x} {if {$x <= 2.5} {puts $x}}", &[]),
    ];

    /// The per-shape table, row for row, under Tcl 8.6: `eq` refines its
    /// true edge and `ne` its false one, either side; `==` with a literal no
    /// number spells refines the value, and with a number the type and the
    /// integer point, never the value; an ordered comparison with an integer
    /// the half-line on each edge, either side, `010` octal under 8.6, and
    /// with a boolean word, which compares as a string, or a double nothing;
    /// `in` the finite set and `ni` its false edge; `string is` under
    /// `-strict` the type, without it nothing;
    /// `!` swaps, `&&` keeps both true-edge answers and `||` both
    /// false-edge ones, and neither keeps an answer an impure operand after
    /// it may undo; the flattened exact `switch` refines each arm. `$x`
    /// alone, `$x eq $y`, an opaque `-glob` switch, a qualified name, an
    /// alias and an element refine nothing.
    #[test]
    fn each_shape_refines_the_edge_its_table_row_names() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let cases = SHAPE_CASES;
        let source = cases
            .iter()
            .map(|(_, proc, _)| *proc)
            .collect::<Vec<_>>()
            .join("\n");
        let cu = crate::compilation_unit::CompilationUnit::build_for_dialect(
            &source, registry, false, "tcl8.6",
        );
        for (name, _, expected) in cases {
            let f = cu
                .function(&format!("::{name}"))
                .expect("procedure analysed");
            let mut rows = refinement_rows(f);
            rows.sort();
            let mut expected: Vec<(bool, String, String)> = expected
                .iter()
                .map(|&(polarity, place, fact)| (polarity, place.to_owned(), fact.to_owned()))
                .collect();
            expected.sort();
            assert_eq!(rows, expected, "{name}");
        }
    }

    /// The `puts` statements of `f`, in source order, with the block each
    /// sits in.
    fn puts_blocks(f: &crate::compilation_unit::FunctionUnit) -> Vec<BlockId> {
        let mut found: Vec<(u32, BlockId)> = f
            .cfg
            .blocks
            .iter()
            .flat_map(|(&id, block)| {
                block
                    .statements
                    .iter()
                    .filter(|statement| {
                        matches!(statement, Statement::Call { command, .. } if command == "puts")
                    })
                    .map(move |statement| (statement.span().start(), id))
            })
            .collect();
        found.sort_unstable();
        found.into_iter().map(|(_, id)| id).collect()
    }

    /// A value refinement holds wherever every executable path carries it:
    /// in its arm and past a merge inside the arm, whose two paths both
    /// cross the refined edge, and not past the merge of the refined arm
    /// with the other one, where the version reads as its own value. A
    /// barrier — `time $script` runs the script here, which may set `x`
    /// without a new version — ends it, in its own block and after.
    #[test]
    fn a_value_refinement_holds_where_every_path_carries_it() {
        let registry = CommandRegistry::build_default();
        let cu = crate::compilation_unit::CompilationUnit::build_for(
            "proc p {x c} {if {$x eq \"a\"} {if {$c} {puts 1} else {puts 2}; puts $x}; puts $x}\n\
             proc m {x script} {if {$x eq \"a\"} {puts $x; time $script; puts $x}}\n",
            &registry,
            false,
        );
        let a = LatticeValue::Const(ConstValue::String("a".to_owned()));
        let p = cu.function("::p").expect("procedure analysed");
        let x = (p.ssa.var_symbol("x").expect("x"), 0);
        let held: Vec<Option<&LatticeValue>> = puts_blocks(p)
            .into_iter()
            .map(|block| p.sccp.value_at(block, x))
            .collect();
        assert_eq!(
            held,
            vec![
                Some(&a),
                Some(&a),
                Some(&a),
                Some(&LatticeValue::Overdefined)
            ],
            "both inner arms, the inner merge, then past the outer merge"
        );
        let m = cu.function("::m").expect("procedure analysed");
        let x = (m.ssa.var_symbol("x").expect("x"), 0);
        assert!(
            m.cfg
                .blocks
                .values()
                .flat_map(|block| block.statements.iter())
                .any(|statement| matches!(statement, Statement::Barrier { .. })),
            "the witness needs `time $script` to lower to a barrier"
        );
        assert_eq!(m.sccp.refinements.len(), 1, "{:?}", m.sccp.refinements);
        for block in puts_blocks(m) {
            assert_eq!(m.sccp.value_at(block, x), Some(&LatticeValue::Overdefined));
            assert_eq!(m.sccp.refinements_in(block).count(), 0);
        }
    }

    #[test]
    fn existence_fold_abstains_for_nested_upframe_alias_unset_mutation() {
        // `uplevel 0` evaluates in this procedure's frame. The nested body
        // aliases that frame's parameter and unsets it, so Tcl observes the else
        // branch. The no-uplevel twin proves the branch is otherwise foldable;
        // this is a mutation test of the exact fact the fold must block.
        let registry = CommandRegistry::build_default();
        let stable = crate::compilation_unit::CompilationUnit::build_for(
            "proc f {local} { if {[info exists local]} { return yes } else { return no } }",
            &registry,
            false,
        );
        assert!(
            !stable
                .function("::f")
                .expect("control procedure analysed")
                .sccp
                .constant_branches
                .is_empty(),
            "the unmutated control must be foldable"
        );

        let mutated = crate::compilation_unit::CompilationUnit::build_for(
            "proc f {local} { uplevel 0 { uplevel 0 { upvar 0 local alias; unset alias } }; if {[info exists local]} { return yes } else { return no } }",
            &registry,
            false,
        );
        let f = mutated.function("::f").expect("mutated procedure analysed");
        let outer = f
            .cfg
            .blocks
            .values()
            .flat_map(|block| block.statements.iter())
            .find_map(|statement| match statement {
                Statement::UpFrame { body, .. } => Some(body),
                _ => None,
            })
            .expect("outer literal uplevel must lower to UpFrame");
        assert!(
            outer
                .statements
                .iter()
                .any(|statement| matches!(statement, Statement::UpFrame { .. })),
            "the nested literal uplevel must remain an UpFrame in its parent body"
        );
        assert!(
            f.sccp.constant_branches.is_empty(),
            "nested upframe/upvar/unset may remove local: {:?}",
            f.sccp.constant_branches
        );
    }

    #[test]
    fn upframe_body_models_upvar_alias_unset_in_the_caller_frame() {
        // tclsh: `proc caller {} {set caller_value 1; f; info exists
        // caller_value}; proc f {} {uplevel 1 {upvar 0 caller_value alias;
        // unset alias}}; caller` returns 0. The literal UpFrame body therefore
        // carries a real caller-frame mutation, not merely a control-flow
        // wrapper, and is a separate vector from the nested local-frame test.
        let registry = CommandRegistry::build_default();
        let cu = crate::compilation_unit::CompilationUnit::build_for(
            "proc f {} { uplevel 1 { upvar 0 caller_value alias; unset alias } }",
            &registry,
            false,
        );
        let f = cu.function("::f").expect("procedure analysed");
        let body = f
            .cfg
            .blocks
            .values()
            .flat_map(|block| block.statements.iter())
            .find_map(|statement| match statement {
                Statement::UpFrame { body, .. } => Some(body),
                _ => None,
            })
            .expect("literal caller-frame body must lower to UpFrame");
        assert!(body.statements.iter().any(|statement| {
            matches!(statement, Statement::Call { command, .. } if command == "upvar")
        }));
        assert!(body.statements.iter().any(|statement| {
            matches!(statement, Statement::Call { command, .. } if command == "unset")
        }));
    }

    // Driver.

    use crate::expr_ast::BinOp;
    use crate::ir::Statement;
    use crate::ssa::{SsaBlock, SsaStatement};
    use tcl_lexer::Span;

    /// A block-less SSA function used purely as a variable-name interner for
    /// the hand-built statement / lattice tests.
    fn bare_ssa() -> SsaFunction {
        SsaFunction::trivial("::top", BlockId(0), vec!["entry".into()])
    }

    fn assign_const_stmt(ssa: &mut SsaFunction, name: &str, value: &str, ver: u32) -> SsaStatement {
        let mut defs = HashMap::new();
        defs.insert(ssa.intern_var(name), ver);
        SsaStatement {
            statement: Statement::AssignConst {
                span: Span::new(0, 0),
                name: name.into(),
                name_braced: false,
                value: value.into(),
                value_span: None,
            },
            uses: HashMap::new(),
            defs,
            may_defs: std::collections::HashSet::new(),
            destruction_defs: std::collections::HashSet::new(),
            quoted_uses: std::collections::HashSet::new(),
            name_only_uses: std::collections::HashSet::new(),
        }
    }

    fn empty_ssa_block(name: &str) -> SsaBlock {
        SsaBlock {
            name: name.into(),
            phis: Vec::new(),
            statements: Vec::new(),
            entry_versions: HashMap::new(),
            exit_versions: HashMap::new(),
        }
    }

    /// Build an [`SsaFunction`] over `f`'s interner with the given per-name SSA
    /// blocks; any CFG block not listed gets an empty SSA block.
    fn make_ssa(f: &Function, named: Vec<(&str, SsaBlock)>) -> SsaFunction {
        let mut ssa = SsaFunction::trivial("::top", f.entry, f.block_names().to_vec());
        let mut provided: std::collections::HashSet<BlockId> = std::collections::HashSet::new();
        for (name, blk) in named {
            let id = id_of(f, name);
            provided.insert(id);
            ssa.blocks.insert(id, blk);
        }
        for id in f.blocks.keys() {
            if !provided.contains(id) {
                ssa.blocks.insert(*id, empty_ssa_block(f.block_name(*id)));
            }
        }
        ssa
    }

    #[test]
    fn phi_join_widens_version_zero_livein_to_overdefined() {
        // A phi merging a version-0 live-in (a proc parameter / global /
        // other caller-supplied root) with a defined-arm constant must widen
        // to Overdefined — the caller's value cannot vanish from the merge.
        // Otherwise `if {$c} { set x 5 }; if {$x == 5} {A} else {B}` folds the
        // second test always-true and deletes the else arm. Mirrors the
        // interval pass joining TOP for `inc == 0` (intervals.rs:570-574).
        let mut ssa = bare_ssa();
        let x = ssa.intern_var("x");
        let mut block = empty_ssa_block("merge");
        block.phis.push(crate::ssa::Phi {
            name: x,
            version: 2,
            incoming: HashMap::from([(BlockId(1), 0), (BlockId(2), 1)]),
        });
        // `seed_live_in_roots` pins the version-0 root Overdefined.
        let mut values: HashMap<ValueKey, LatticeValue> = HashMap::new();
        values.insert((x, 0), LatticeValue::Overdefined);
        values.insert((x, 1), LatticeValue::Const(ConstValue::Int(5)));
        assert!(sccp_process_phis(
            &mut values,
            &block,
            &[BlockId(1), BlockId(2)]
        ));
        assert_eq!(values.get(&(x, 2)), Some(&LatticeValue::Overdefined));
    }

    #[test]
    fn barrier_own_defs_become_overdefined() {
        // A barrier (e.g. `dict for {x y} $d {}`) defines its
        // own variables. Those defs must be set Overdefined so they participate
        // in a downstream phi join; otherwise the def key is never inserted and
        // a phi merging the barrier-def with a constant folds to that constant.
        let mut ssa = bare_ssa();
        let x = ssa.intern_var("x");
        let mut block = empty_ssa_block("barrier_block");
        let mut defs: HashMap<Symbol, crate::ssa::Version> = HashMap::new();
        defs.insert(x, 2);
        block.statements.push(SsaStatement {
            statement: Statement::Barrier {
                span: Span::new(0, 0),
                reason: "dict-for".into(),
                command: "::tcl::dict::for".into(),
                canonical_command: None,
                args: vec!["d".into()],
                tokens: None,
            },
            uses: HashMap::new(),
            defs,
            may_defs: std::collections::HashSet::new(),
            destruction_defs: std::collections::HashSet::new(),
            quoted_uses: std::collections::HashSet::new(),
            name_only_uses: std::collections::HashSet::new(),
        });
        let mut values: HashMap<ValueKey, LatticeValue> = HashMap::new();
        let escaping: HashSet<String> = HashSet::new();
        assert!(
            sccp_process_statements(
                &mut values,
                &block,
                &ssa,
                StatementInputs {
                    block: BlockId(0),
                    escaping: &escaping,
                    has_dynamic_variable_trace: false,
                    clobbers: None,
                    catches: None,
                },
                &LatticeDriver::detached(None, FoldPolicy::default()),
                None,
            )
            .changed
        );
        assert_eq!(
            values.get(&(x, 2)),
            Some(&LatticeValue::Overdefined),
            "a barrier-defined variable must be Overdefined, not absent/Unknown"
        );
    }

    #[test]
    fn phi_join_defaults_missing_version_zero_feed_to_overdefined() {
        // Defence in depth: even if a version-0 feed was never seeded, the
        // join must treat it as Overdefined rather than skipping it (which
        // would leave the phi holding the defined-arm constant).
        let mut ssa = bare_ssa();
        let x = ssa.intern_var("x");
        let mut block = empty_ssa_block("merge");
        block.phis.push(crate::ssa::Phi {
            name: x,
            version: 2,
            incoming: HashMap::from([(BlockId(1), 0), (BlockId(2), 1)]),
        });
        let mut values: HashMap<ValueKey, LatticeValue> = HashMap::new();
        values.insert((x, 1), LatticeValue::Const(ConstValue::Int(5)));
        assert!(sccp_process_phis(
            &mut values,
            &block,
            &[BlockId(1), BlockId(2)]
        ));
        assert_eq!(values.get(&(x, 2)), Some(&LatticeValue::Overdefined));
    }

    /// A call to a command the module cannot see may create or destroy a plain
    /// top-level name as it may a `::` one, and a procedure's local through
    /// `upvar 1`, so `[info exists]` after it decides nothing; a call the
    /// module resolves leaves the local's existence decided.
    #[test]
    fn an_unseen_call_leaves_no_name_with_a_settled_existence() {
        let registry = CommandRegistry::build_default();
        for (source, function) in [
            (
                "set x 1\nfoo\nif {[info exists x]} { puts a } else { puts b }\n",
                "::top",
            ),
            (
                "foo\nif {[info exists never_set]} { puts a } else { puts b }\n",
                "::top",
            ),
            (
                "proc p {} { set x 1; foo; if {[info exists x]} { puts a } else { puts b } }\n",
                "::p",
            ),
        ] {
            let cu = crate::compilation_unit::CompilationUnit::build_for(source, &registry, false);
            let f = cu.function(function).expect("function analysed");
            assert!(f.sccp.constant_branches.is_empty(), "{source}");
        }
        let cu = crate::compilation_unit::CompilationUnit::build_for(
            "proc foo {} { puts hi }\nproc p {} { set x 1; foo; if {[info exists x]} { puts a } else { puts b } }\n",
            &registry,
            false,
        );
        let f = cu.function("::p").expect("procedure analysed");
        let decided = f
            .sccp
            .constant_branches
            .iter()
            .find(|branch| branch.condition == "[info exists x]")
            .expect("the local's existence is decided");
        assert!(decided.value);
    }

    #[test]
    fn sccp_marks_entry_executable_and_propagates_const() {
        // entry: set x 42
        let mut f = Function::new("::top", "entry");
        let entry = f.entry;
        f.blocks.get_mut(&entry).unwrap().terminator = Some(Terminator::Return {
            value: None,
            value_word: None,
            span: None,
            expr: None,
            expr_base: None,
            tokens: None,
            braced: false,
        });
        let mut ssa = make_ssa(&f, vec![]);
        let stmt = assign_const_stmt(&mut ssa, "x", "42", 1);
        ssa.blocks.get_mut(&entry).unwrap().statements.push(stmt);
        let x = ssa.var_symbol("x").unwrap();

        let r = sccp_no_traces(&f, &ssa, None, FoldPolicy::default());
        assert!(r.executable_blocks.contains(&entry));
        assert_eq!(
            r.values.get(&(x, 1)),
            Some(&LatticeValue::Const(ConstValue::Int(42)))
        );
    }

    #[test]
    fn sccp_constant_branch_detected_and_taken_target_marked() {
        // entry: branch on literal "1" → true → "t", false → "e"
        let mut f = Function::new("::top", "entry");
        let entry = f.entry;
        let t = block(&mut f, "t");
        let e = block(&mut f, "e");
        f.blocks.get_mut(&entry).unwrap().terminator = Some(branch(literal("1"), t, e));
        f.blocks.get_mut(&t).unwrap().terminator = Some(Terminator::Return {
            value: None,
            value_word: None,
            span: None,
            expr: None,
            expr_base: None,
            tokens: None,
            braced: false,
        });
        f.blocks.get_mut(&e).unwrap().terminator = Some(Terminator::Return {
            value: None,
            value_word: None,
            span: None,
            expr: None,
            expr_base: None,
            tokens: None,
            braced: false,
        });
        let ssa = make_ssa(&f, vec![]);

        let r = sccp_no_traces(&f, &ssa, None, FoldPolicy::default());
        assert!(r.executable_blocks.contains(&t));
        assert!(!r.executable_blocks.contains(&e));
        assert_eq!(r.constant_branches.len(), 1);
        let cb = &r.constant_branches[0];
        assert!(cb.value);
        assert_eq!(cb.taken_target, "t");
        assert_eq!(cb.not_taken_target, "e");
    }

    #[test]
    fn sccp_false_branch_prunes_true_target() {
        let mut f = Function::new("::top", "entry");
        let entry = f.entry;
        let t = block(&mut f, "t");
        let e = block(&mut f, "e");
        f.blocks.get_mut(&entry).unwrap().terminator = Some(branch(literal("0"), t, e));
        f.blocks.get_mut(&t).unwrap().terminator = Some(Terminator::Return {
            value: None,
            value_word: None,
            span: None,
            expr: None,
            expr_base: None,
            tokens: None,
            braced: false,
        });
        f.blocks.get_mut(&e).unwrap().terminator = Some(Terminator::Return {
            value: None,
            value_word: None,
            span: None,
            expr: None,
            expr_base: None,
            tokens: None,
            braced: false,
        });
        let ssa = make_ssa(&f, vec![]);

        let r = sccp_no_traces(&f, &ssa, None, FoldPolicy::default());
        assert!(!r.executable_blocks.contains(&t));
        assert!(r.executable_blocks.contains(&e));
    }

    #[test]
    fn sccp_unknown_branch_executes_both_targets() {
        // Var reference — lattice value defaults to Unknown → decision None.
        let mut f = Function::new("::top", "entry");
        let entry = f.entry;
        let t = block(&mut f, "t");
        let e = block(&mut f, "e");
        let cond = ExprNode::Var {
            text: "$z".into(),
            name: "z".into(),
            start: 0,
            end: 2,
        };
        f.blocks.get_mut(&entry).unwrap().terminator = Some(branch(cond, t, e));
        f.blocks.get_mut(&t).unwrap().terminator = Some(Terminator::Return {
            value: None,
            value_word: None,
            span: None,
            expr: None,
            expr_base: None,
            tokens: None,
            braced: false,
        });
        f.blocks.get_mut(&e).unwrap().terminator = Some(Terminator::Return {
            value: None,
            value_word: None,
            span: None,
            expr: None,
            expr_base: None,
            tokens: None,
            braced: false,
        });
        let ssa = make_ssa(&f, vec![]);

        let r = sccp_no_traces(&f, &ssa, None, FoldPolicy::default());
        assert!(r.executable_blocks.contains(&t));
        assert!(r.executable_blocks.contains(&e));
        assert!(r.constant_branches.is_empty());
    }

    #[test]
    fn evaluate_def_assign_const_produces_int_or_string() {
        let mut ssa = bare_ssa();
        let s_int = assign_const_stmt(&mut ssa, "x", "42", 1);
        assert_eq!(
            evaluate_def(&s_int, &HashMap::new(), &ssa, FoldPolicy::default()),
            LatticeValue::Const(ConstValue::Int(42))
        );
        let s_str = assign_const_stmt(&mut ssa, "x", "hello", 1);
        assert_eq!(
            evaluate_def(&s_str, &HashMap::new(), &ssa, FoldPolicy::default()),
            LatticeValue::Const(ConstValue::String("hello".into()))
        );
    }

    #[test]
    fn evaluate_def_assign_expr_folds_with_lattice() {
        // `set x [expr {$a + 3}]` with $a → Const(2) should fold to 5.
        let mut ssa = bare_ssa();
        let a = ssa.intern_var("a");
        let x = ssa.intern_var("x");
        let mut uses = HashMap::new();
        uses.insert(a, 1);
        let mut defs = HashMap::new();
        defs.insert(x, 1);

        let expr = ExprNode::Binary {
            op: BinOp::Add,
            left: Box::new(ExprNode::Var {
                text: "$a".into(),
                name: "a".into(),
                start: 0,
                end: 2,
            }),
            right: Box::new(ExprNode::Literal {
                text: "3".into(),
                start: 3,
                end: 4,
            }),
        };
        let stmt_ssa = SsaStatement {
            statement: Statement::AssignExpr {
                span: Span::new(0, 0),
                name: "x".into(),
                name_braced: false,
                expr,
                command_binding: Some(tcl_runtime_api::CommandBindingIdentity::new("expr", "expr")),
                expr_base: None,
                fallback_value: "[expr {$a + 3}]".into(),
            },
            uses,
            defs,
            may_defs: std::collections::HashSet::new(),
            destruction_defs: std::collections::HashSet::new(),
            quoted_uses: std::collections::HashSet::new(),
            name_only_uses: std::collections::HashSet::new(),
        };

        let mut values = HashMap::new();
        values.insert((a, 1), LatticeValue::Const(ConstValue::Int(2)));

        assert_eq!(
            evaluate_def(&stmt_ssa, &values, &ssa, FoldPolicy::default()),
            LatticeValue::Const(ConstValue::Int(5))
        );
    }

    /// SCCP folds the iRules word operators when — and only when — the
    /// policy says the dialect has them.
    ///
    /// Evaluating through a dialect-blind entry point (a bare
    /// `octal: Option<bool>`) leaves `FoldOps::is_irules` `false`, so every
    /// word operator silently declines while the `eq` control on the same
    /// shape folds, because plain Tcl shares it.
    #[test]
    fn evaluate_def_folds_irules_word_operator_only_under_an_irules_policy() {
        let mut ssa = bare_ssa();
        let subject = ssa.intern_var("s");
        let out = ssa.intern_var("hit");
        let mut uses = HashMap::new();
        uses.insert(subject, 1);
        let mut defs = HashMap::new();
        defs.insert(out, 1);

        let expr = ExprNode::Binary {
            op: crate::expr_ast::BinOp::Contains,
            left: Box::new(ExprNode::Var {
                text: "$s".into(),
                name: "s".into(),
                start: 0,
                end: 2,
            }),
            right: Box::new(ExprNode::String {
                text: "cd".into(),
                start: 3,
                end: 7,
            }),
        };
        let stmt_ssa = SsaStatement {
            statement: Statement::AssignExpr {
                span: Span::new(0, 0),
                name: "hit".into(),
                name_braced: false,
                expr,
                command_binding: Some(tcl_runtime_api::CommandBindingIdentity::new("expr", "expr")),
                expr_base: None,
                fallback_value: "[expr {$s contains \"cd\"}]".into(),
            },
            uses,
            defs,
            may_defs: std::collections::HashSet::new(),
            destruction_defs: std::collections::HashSet::new(),
            quoted_uses: std::collections::HashSet::new(),
            name_only_uses: std::collections::HashSet::new(),
        };
        let mut values = HashMap::new();
        values.insert(
            (subject, 1),
            LatticeValue::Const(ConstValue::String("abcde".into())),
        );

        let irules =
            FoldPolicy::for_profile(Some(true), Some(tcl_dialect::DialectProfile::irules()));
        assert_eq!(
            evaluate_def(&stmt_ssa, &values, &ssa, irules),
            LatticeValue::Const(ConstValue::Int(1)),
            "`$s contains \"cd\"` with $s = abcde must fold to 1 under an iRules policy"
        );
        // TN: without the dialect fact the fold is declined, not guessed.
        assert_eq!(
            evaluate_def(&stmt_ssa, &values, &ssa, FoldPolicy::default()),
            LatticeValue::Overdefined,
            "a dialect-blind policy must decline the word-operator fold"
        );
    }

    /// A fused `set r [expr {TEXT}]` under `profile`'s expression grammar.
    fn fused_expr_stmt(
        ssa: &mut SsaFunction,
        text: &str,
        profile: Option<&'static tcl_dialect::DialectProfile>,
    ) -> SsaStatement {
        let mut defs = HashMap::new();
        defs.insert(ssa.intern_var("r"), 1);
        SsaStatement {
            statement: Statement::AssignExpr {
                span: Span::new(0, 0),
                name: "r".into(),
                name_braced: false,
                expr: crate::expr_parser::parse_expr_for_profile(text, profile),
                command_binding: Some(tcl_runtime_api::CommandBindingIdentity::new("expr", "expr")),
                expr_base: None,
                fallback_value: format!("[expr {{{text}}}]"),
            },
            uses: HashMap::new(),
            defs,
            may_defs: std::collections::HashSet::new(),
            destruction_defs: std::collections::HashSet::new(),
            quoted_uses: std::collections::HashSet::new(),
            name_only_uses: std::collections::HashSet::new(),
        }
    }

    /// `expr` answers its full value, not only a number: a string result is
    /// the result, and a string that reads as a number under the target's
    /// grammar is that number's canonical form. Oracle, tclsh 8.4 to 9.1:
    /// `expr {"x"}` is `x`, `expr {1 ? "yes" : "no"}` is `yes`, `expr
    /// {"true"}` is `true`, `expr {" 5 "}` is 5, `expr {"0x10"}` is 16,
    /// `expr {"1.50"}` is 1.5 and `expr {1/0}` raises; `expr {"010"}` is 8 up
    /// to 8.6 and 10 from 9.0; `expr {1 << 70}` is
    /// `1180591620717411303424` from 8.5 and 0 under 8.4, so a target that
    /// does not widen declines it.
    #[test]
    fn a_fused_expression_answers_its_full_value() {
        let text = |s: &str| LatticeValue::Const(ConstValue::String(s.to_owned()));
        let int = |i: i64| LatticeValue::Const(ConstValue::Int(i));
        let under = |name: &str| {
            let profile = tcl_dialect::DialectProfile::find(name).expect(name);
            (
                Some(profile),
                FoldPolicy::for_profile(
                    crate::tcl_expr_eval::leading_zero_is_octal(profile),
                    Some(profile),
                ),
            )
        };
        let cases: [(&str, &str, LatticeValue); 17] = [
            ("tcl8.4", r#""x""#, text("x")),
            ("f5-irules", r#""x""#, text("x")),
            ("tcl9.0", r#"1 ? "yes" : "no""#, text("yes")),
            ("tcl8.6", r#""true""#, text("true")),
            ("tcl8.6", r#"" 5 ""#, int(5)),
            ("tcl9.0", r#""0x10""#, int(16)),
            (
                "tcl8.6",
                r#""1.50""#,
                LatticeValue::Const(ConstValue::Float(1.5)),
            ),
            ("tcl8.6", r#""010""#, int(8)),
            ("tcl9.0", r#""010""#, int(10)),
            ("tcl8.6", r#""08""#, text("08")),
            ("tcl9.0", "1/0", LatticeValue::Overdefined),
            ("tcl9.0", "1 << 70", text("1180591620717411303424")),
            ("tcl8.6", "2 ** 64", text("18446744073709551616")),
            ("tcl8.4", "1 << 70", LatticeValue::Overdefined),
            ("f5-irules", "1 << 70", LatticeValue::Overdefined),
            ("tcl9.0", "[string length abc] * 2", int(6)),
            ("tcl9.0", "0 && [error never]", int(0)),
        ];
        for (dialect, expression, want) in cases {
            let (profile, policy) = under(dialect);
            let mut ssa = bare_ssa();
            let stmt = fused_expr_stmt(&mut ssa, expression, profile);
            assert_eq!(
                evaluate_pristine(&stmt, &HashMap::new(), &ssa, policy),
                want,
                "{dialect}: expr {{{expression}}}"
            );
        }
        // No leading-zero rule at all: the two readings of `010` disagree.
        let mut ssa = bare_ssa();
        let stmt = fused_expr_stmt(&mut ssa, r#""010""#, None);
        assert_eq!(
            evaluate_pristine(&stmt, &HashMap::new(), &ssa, FoldPolicy::from_octal(None)),
            LatticeValue::Overdefined
        );
    }

    /// A quoted `expr` word is substituted as text before the expression is
    /// parsed: with `a` holding `1 + 1`, `expr "$a * 2"` is `1 + 1 * 2`, 3
    /// under tclsh 8.4 to 9.1 — the variable's value is re-read as
    /// expression source, where a braced `expr {$a * 2}` raises.
    #[test]
    fn a_quoted_expression_word_is_substituted_before_it_is_parsed() {
        let mut ssa = bare_ssa();
        let mut stmt = assign_value_stmt(&mut ssa, "r", "[expr \"$a * 2\"]", 1);
        let a = ssa.intern_var("a");
        stmt.uses.insert(a, 1);
        let mut values = HashMap::new();
        values.insert(
            (a, 1),
            LatticeValue::Const(ConstValue::String("1 + 1".into())),
        );
        assert_eq!(
            evaluate_pristine(&stmt, &values, &ssa, FoldPolicy::default()),
            LatticeValue::Const(ConstValue::Int(3))
        );
        let mut ssa = bare_ssa();
        let mut stmt = fused_expr_stmt(&mut ssa, "$a * 2", None);
        let a = ssa.intern_var("a");
        stmt.uses.insert(a, 1);
        assert_eq!(
            evaluate_pristine(&stmt, &values, &ssa, FoldPolicy::default()),
            LatticeValue::Overdefined
        );
    }

    // evaluate_def for Incr.

    fn incr_stmt(
        ssa: &mut SsaFunction,
        name: &str,
        amount: Option<&str>,
        old_ver: u32,
        new_ver: u32,
    ) -> SsaStatement {
        let sym = ssa.intern_var(name);
        let mut uses = HashMap::new();
        uses.insert(sym, old_ver);
        let mut defs = HashMap::new();
        defs.insert(sym, new_ver);
        SsaStatement {
            statement: Statement::Incr {
                span: Span::new(0, 0),
                name: name.into(),
                name_braced: false,
                amount: amount.map(String::from),
                amount_braced: false,
                safe_on_uninit: false,
            },
            uses,
            defs,
            may_defs: std::collections::HashSet::new(),
            destruction_defs: std::collections::HashSet::new(),
            quoted_uses: std::collections::HashSet::new(),
            name_only_uses: std::collections::HashSet::new(),
        }
    }

    #[test]
    fn evaluate_def_incr_default_amount() {
        // x@1 = Const(Int(5)); `incr x` → x@2 = Const(Int(6)).
        let mut ssa = bare_ssa();
        let stmt = incr_stmt(&mut ssa, "x", None, 1, 2);
        let x = ssa.var_symbol("x").unwrap();
        let mut values = HashMap::new();
        values.insert((x, 1), LatticeValue::Const(ConstValue::Int(5)));
        assert_eq!(
            evaluate_pristine(&stmt, &values, &ssa, FoldPolicy::default()),
            LatticeValue::Const(ConstValue::Int(6))
        );
    }

    #[test]
    fn evaluate_def_incr_integer_literal_amount() {
        let mut ssa = bare_ssa();
        let stmt = incr_stmt(&mut ssa, "x", Some("10"), 1, 2);
        let x = ssa.var_symbol("x").unwrap();
        let mut values = HashMap::new();
        values.insert((x, 1), LatticeValue::Const(ConstValue::Int(3)));
        assert_eq!(
            evaluate_pristine(&stmt, &values, &ssa, FoldPolicy::default()),
            LatticeValue::Const(ConstValue::Int(13))
        );
    }

    #[test]
    fn evaluate_def_incr_negative_literal_amount() {
        let mut ssa = bare_ssa();
        let stmt = incr_stmt(&mut ssa, "x", Some("-2"), 1, 2);
        let x = ssa.var_symbol("x").unwrap();
        let mut values = HashMap::new();
        values.insert((x, 1), LatticeValue::Const(ConstValue::Int(10)));
        assert_eq!(
            evaluate_pristine(&stmt, &values, &ssa, FoldPolicy::default()),
            LatticeValue::Const(ConstValue::Int(8))
        );
    }

    #[test]
    fn evaluate_def_incr_var_ref_amount() {
        // `incr x $y` where $y resolves to 4.
        let mut ssa = bare_ssa();
        let mut stmt = incr_stmt(&mut ssa, "x", Some("$y"), 1, 2);
        let x = ssa.var_symbol("x").unwrap();
        let y = ssa.intern_var("y");
        stmt.uses.insert(y, 1);
        let mut values = HashMap::new();
        values.insert((x, 1), LatticeValue::Const(ConstValue::Int(6)));
        values.insert((y, 1), LatticeValue::Const(ConstValue::Int(4)));
        assert_eq!(
            evaluate_pristine(&stmt, &values, &ssa, FoldPolicy::default()),
            LatticeValue::Const(ConstValue::Int(10))
        );
    }

    #[test]
    fn evaluate_def_incr_unknown_base_propagates_unknown() {
        let mut ssa = bare_ssa();
        let stmt = incr_stmt(&mut ssa, "x", None, 1, 2);
        let values = HashMap::new();
        // No entry for x@1 → base is Unknown → result Unknown.
        assert_eq!(
            evaluate_pristine(&stmt, &values, &ssa, FoldPolicy::default()),
            LatticeValue::Unknown
        );
    }

    /// Without the whole-module trust fact there is no evidence a head still
    /// denotes its registry command, so no resolved command answers — the
    /// typed `incr` included; the same statement under the pristine stance
    /// folds (#2164).
    #[test]
    fn evaluate_def_without_the_trust_fact_answers_for_no_command() {
        let mut ssa = bare_ssa();
        let stmt = incr_stmt(&mut ssa, "x", None, 1, 2);
        let x = ssa.var_symbol("x").unwrap();
        let mut values = HashMap::new();
        values.insert((x, 1), LatticeValue::Const(ConstValue::Int(5)));
        assert_eq!(
            evaluate_def(&stmt, &values, &ssa, FoldPolicy::default()),
            LatticeValue::Overdefined
        );
        assert_eq!(
            evaluate_pristine(&stmt, &values, &ssa, FoldPolicy::default()),
            LatticeValue::Const(ConstValue::Int(6))
        );
    }

    #[test]
    fn evaluate_def_incr_overdefined_base_widens() {
        let mut ssa = bare_ssa();
        let stmt = incr_stmt(&mut ssa, "x", None, 1, 2);
        let x = ssa.var_symbol("x").unwrap();
        let mut values = HashMap::new();
        values.insert((x, 1), LatticeValue::Overdefined);
        assert_eq!(
            evaluate_pristine(&stmt, &values, &ssa, FoldPolicy::default()),
            LatticeValue::Overdefined
        );
    }

    #[test]
    fn evaluate_def_incr_non_integer_amount_widens() {
        let mut ssa = bare_ssa();
        let stmt = incr_stmt(&mut ssa, "x", Some("2.5"), 1, 2);
        let x = ssa.var_symbol("x").unwrap();
        let mut values = HashMap::new();
        values.insert((x, 1), LatticeValue::Const(ConstValue::Int(1)));
        assert_eq!(
            evaluate_pristine(&stmt, &values, &ssa, FoldPolicy::default()),
            LatticeValue::Overdefined
        );
    }

    /// A `Call` whose resolved plan is a cell update, in the shape the
    /// lowering gives `append` / `lappend`: the target is the first
    /// argument, a def, and a read of its own prior version.
    fn cell_update_call(
        ssa: &mut SsaFunction,
        command: &str,
        args: &[&str],
        old_ver: u32,
        new_ver: u32,
    ) -> SsaStatement {
        let target = ssa.intern_var(args[0]);
        let mut uses = HashMap::new();
        uses.insert(target, old_ver);
        let mut defs = HashMap::new();
        defs.insert(target, new_ver);
        SsaStatement {
            statement: Statement::Call {
                span: Span::new(0, 0),
                command: command.into(),
                canonical_command: None,
                args: args.iter().map(|a| (*a).to_owned()).collect(),
                defs: vec![args[0].to_owned()],
                reads: Vec::new(),
                reads_own_defs: true,
                safe_on_uninit: true,
                tokens: None,
                foreach_groups: None,
            },
            uses,
            defs,
            may_defs: std::collections::HashSet::new(),
            destruction_defs: std::collections::HashSet::new(),
            quoted_uses: std::collections::HashSet::new(),
            name_only_uses: std::collections::HashSet::new(),
        }
    }

    fn folds_for(dialect: &str) -> BuiltinFoldInputs<'static> {
        use std::sync::OnceLock;
        static MUTATIONS: OnceLock<crate::command_binding::ModuleCommandMutations> =
            OnceLock::new();
        let environment = tcl_registry::model::ingress::resolve_environment(dialect);
        BuiltinFoldInputs {
            source_metadata_input: None,
            registry: tcl_registry::model::ingress::static_context_for(dialect).commands(),
            mutations: MUTATIONS.get_or_init(Default::default),
            dialect: Some(environment.analyser_profile()),
            defining_class: None,
            registry_engine: false,
            trust: FoldTrust::ObservedBindings,
            proven_pure_parameters: false,
        }
    }

    /// `append s " world"` over `s@1 = hello`: the registry's cell append
    /// writes `hello world`, byte-exact, and the def takes the store.
    #[test]
    fn evaluate_def_append_call_folds_through_the_cell_update() {
        let mut ssa = bare_ssa();
        let stmt = cell_update_call(&mut ssa, "append", &["s", " world", "!"], 1, 2);
        let s = ssa.var_symbol("s").unwrap();
        let mut values = HashMap::new();
        values.insert(
            (s, 1),
            LatticeValue::Const(ConstValue::String("hello".into())),
        );
        assert_eq!(
            evaluate_pristine(&stmt, &values, &ssa, FoldPolicy::default()),
            LatticeValue::Const(ConstValue::String("hello world!".into()))
        );
    }

    /// `append s $p` reads the piece from the lattice, exactly: `set p
    /// { again}` keeps its leading space (#2052).
    #[test]
    fn evaluate_def_append_var_piece_reads_the_lattice_exactly() {
        let mut ssa = bare_ssa();
        let mut stmt = cell_update_call(&mut ssa, "append", &["s", "$p"], 1, 2);
        let s = ssa.var_symbol("s").unwrap();
        let p = ssa.intern_var("p");
        stmt.uses.insert(p, 1);
        let mut values = HashMap::new();
        values.insert(
            (s, 1),
            LatticeValue::Const(ConstValue::String("hello".into())),
        );
        values.insert(
            (p, 1),
            LatticeValue::Const(ConstValue::String(" again".into())),
        );
        assert_eq!(
            evaluate_pristine(&stmt, &values, &ssa, FoldPolicy::default()),
            LatticeValue::Const(ConstValue::String("hello again".into()))
        );
    }

    /// `lappend l c {d e}` over `l@1 = a b`: the list append renders
    /// canonically; over a value that is not a list it is the program's
    /// error, so the def widens.
    #[test]
    fn evaluate_def_lappend_call_renders_the_canonical_list() {
        let mut ssa = bare_ssa();
        let stmt = cell_update_call(&mut ssa, "lappend", &["l", "c", "d e"], 1, 2);
        let l = ssa.var_symbol("l").unwrap();
        let mut values = HashMap::new();
        values.insert(
            (l, 1),
            LatticeValue::Const(ConstValue::String("a b".into())),
        );
        assert_eq!(
            evaluate_pristine(&stmt, &values, &ssa, FoldPolicy::default()),
            LatticeValue::Const(ConstValue::String("a b c {d e}".into()))
        );
        values.insert((l, 1), LatticeValue::Const(ConstValue::String("{".into())));
        assert_eq!(
            evaluate_pristine(&stmt, &values, &ssa, FoldPolicy::default()),
            LatticeValue::Overdefined
        );
        // An unknown prior stays pending, never a manufactured empty list.
        assert_eq!(
            evaluate_pristine(&stmt, &HashMap::new(), &ssa, FoldPolicy::default()),
            LatticeValue::Unknown
        );
    }

    /// `dict incr d a` over `d@1 = a 1 b 2`: the keyed update rewrites the
    /// dictionary the variable holds, and the call's one definition takes
    /// it (tclsh 8.5 to 9.1 give `a 2 b 2`). A prior the lattice cannot
    /// prove leaves the definition unknown or widened, never an empty
    /// dictionary.
    #[test]
    fn evaluate_def_dict_incr_writes_the_dictionary() {
        let mut ssa = bare_ssa();
        let d = ssa.intern_var("d");
        let mut uses = HashMap::new();
        uses.insert(d, 1);
        let mut defs = HashMap::new();
        defs.insert(d, 2);
        let stmt = SsaStatement {
            statement: Statement::Call {
                span: Span::new(0, 0),
                command: "dict".into(),
                canonical_command: None,
                args: vec!["incr".into(), "d".into(), "a".into()],
                defs: vec!["d".into()],
                reads: Vec::new(),
                reads_own_defs: true,
                safe_on_uninit: true,
                tokens: None,
                foreach_groups: None,
            },
            uses,
            defs,
            may_defs: std::collections::HashSet::new(),
            destruction_defs: std::collections::HashSet::new(),
            quoted_uses: std::collections::HashSet::new(),
            name_only_uses: std::collections::HashSet::new(),
        };
        let mut values = HashMap::new();
        values.insert(
            (d, 1),
            LatticeValue::Const(ConstValue::String("a 1 b 2".into())),
        );
        assert_eq!(
            evaluate_pristine(&stmt, &values, &ssa, FoldPolicy::default()),
            LatticeValue::Const(ConstValue::String("a 2 b 2".into()))
        );
        assert_eq!(
            evaluate_pristine(&stmt, &HashMap::new(), &ssa, FoldPolicy::default()),
            LatticeValue::Unknown
        );
        values.insert((d, 1), LatticeValue::Overdefined);
        assert_eq!(
            evaluate_pristine(&stmt, &values, &ssa, FoldPolicy::default()),
            LatticeValue::Overdefined
        );
    }

    /// A generic call with a def but no declared plan keeps the
    /// conservative answer.
    #[test]
    fn evaluate_def_call_without_a_plan_widens() {
        let mut ssa = bare_ssa();
        let stmt = cell_update_call(&mut ssa, "gets", &["chan", "line"], 1, 2);
        let chan = ssa.var_symbol("chan").unwrap();
        let mut values = HashMap::new();
        values.insert(
            (chan, 1),
            LatticeValue::Const(ConstValue::String("stdin".into())),
        );
        assert_eq!(
            evaluate_pristine(&stmt, &values, &ssa, FoldPolicy::default()),
            LatticeValue::Overdefined
        );
    }

    /// The correlated finite-set limit: one finite input evaluates per
    /// member; two distinct finite inputs decline; two reads of one SSA
    /// value are one distinct input.
    #[test]
    fn evaluate_def_incr_lifts_over_one_finite_input() {
        let mut ssa = bare_ssa();
        let stmt = incr_stmt(&mut ssa, "x", Some("10"), 1, 2);
        let x = ssa.var_symbol("x").unwrap();
        let mut values = HashMap::new();
        values.insert(
            (x, 1),
            LatticeValue::ConstSet(vec![ConstValue::Int(1), ConstValue::Int(2)]),
        );
        assert_eq!(
            evaluate_pristine(&stmt, &values, &ssa, FoldPolicy::default()),
            LatticeValue::ConstSet(vec![ConstValue::Int(11), ConstValue::Int(12)])
        );

        let mut ssa = bare_ssa();
        let mut stmt = incr_stmt(&mut ssa, "x", Some("$a"), 1, 2);
        let x = ssa.var_symbol("x").unwrap();
        let a = ssa.intern_var("a");
        stmt.uses.insert(a, 1);
        let mut values = HashMap::new();
        values.insert(
            (x, 1),
            LatticeValue::ConstSet(vec![ConstValue::Int(1), ConstValue::Int(2)]),
        );
        values.insert(
            (a, 1),
            LatticeValue::ConstSet(vec![ConstValue::Int(10), ConstValue::Int(20)]),
        );
        assert_eq!(
            evaluate_pristine(&stmt, &values, &ssa, FoldPolicy::default()),
            LatticeValue::Overdefined,
            "two distinct finite inputs are correlated"
        );
        // `incr x $x`: the step reads the target's own version.
        let mut ssa = bare_ssa();
        let stmt = incr_stmt(&mut ssa, "x", Some("$x"), 1, 2);
        let x = ssa.var_symbol("x").unwrap();
        let mut values = HashMap::new();
        values.insert(
            (x, 1),
            LatticeValue::ConstSet(vec![ConstValue::Int(1), ConstValue::Int(2)]),
        );
        assert_eq!(
            evaluate_pristine(&stmt, &values, &ssa, FoldPolicy::default()),
            LatticeValue::ConstSet(vec![ConstValue::Int(2), ConstValue::Int(4)])
        );
    }

    /// The release rules reach the lattice through the registry's route:
    /// a leading-zero base reads as octal up to 8.6 and decimal from 9.0,
    /// and declines under a profile naming no release (the lenient `tcl`);
    /// past the wide boundary 8.5 onward widens, 8.4 and an unnamed release
    /// decline. `f5-irules` reads as its declared 8.4 base does (ruling 8):
    /// tclsh 8.4 prints 9 for `set x 010; incr x`, and -8 past the wide
    /// boundary, which no model computes.
    #[test]
    fn evaluate_def_incr_reads_the_base_under_the_targets_release() {
        let mut ssa = bare_ssa();
        let stmt = incr_stmt(&mut ssa, "x", None, 1, 2);
        let x = ssa.var_symbol("x").unwrap();
        let under = |dialect: &str, base: LatticeValue| {
            let folds = folds_for(dialect);
            let mut values = HashMap::new();
            values.insert((x, 1), base);
            evaluate_def_with_folds(
                &stmt,
                &values,
                &ssa,
                FoldPolicy::from_registry(folds.registry),
                Some(folds),
            )
        };
        let zero = || LatticeValue::Const(ConstValue::String("010".into()));
        assert_eq!(
            under("tcl8.4", zero()),
            LatticeValue::Const(ConstValue::Int(9))
        );
        assert_eq!(
            under("tcl8.6", zero()),
            LatticeValue::Const(ConstValue::Int(9))
        );
        assert_eq!(
            under("tcl9.0", zero()),
            LatticeValue::Const(ConstValue::Int(11))
        );
        assert_eq!(
            under("f5-irules", zero()),
            LatticeValue::Const(ConstValue::Int(9))
        );
        assert_eq!(under("tcl", zero()), LatticeValue::Overdefined);

        let max = || LatticeValue::Const(ConstValue::Int(i64::MAX));
        assert_eq!(
            under("tcl8.6", max()),
            LatticeValue::Const(ConstValue::String("9223372036854775808".into()))
        );
        assert_eq!(under("tcl8.4", max()), LatticeValue::Overdefined);
        assert_eq!(under("f5-irules", max()), LatticeValue::Overdefined);
        assert_eq!(under("tcl", max()), LatticeValue::Overdefined);
        let mut values = HashMap::new();
        // A whitespace-padded step is an integer in every release.
        let padded = incr_stmt(&mut ssa, "x", Some(" 5"), 1, 2);
        values.insert((x, 1), LatticeValue::Const(ConstValue::Int(1)));
        assert_eq!(
            evaluate_pristine(&padded, &values, &ssa, FoldPolicy::default()),
            LatticeValue::Const(ConstValue::Int(6))
        );
    }

    /// `set x [incr n]`: the host statement's uses do not hold `n` (its read
    /// sits on the synthetic call ahead of the host), so the prior store is
    /// a permanent miss and the definition widens. Reading version 0 found
    /// no value and answered a pending that never resolved, and a join
    /// then took the other arm's constant for `x` (`f 1` prints 2 under
    /// 8.4 to 9.1, where `tcl opt` had rewritten `puts $x` to `puts 5`).
    #[test]
    fn a_missing_use_declines_the_prior_store() {
        let mut ssa = bare_ssa();
        let n = ssa.intern_var("n");
        let stmt = assign_value_stmt(&mut ssa, "x", "[incr n]", 1);
        assert!(!stmt.uses.contains_key(&n), "the host holds no use of n");
        let mut values = HashMap::new();
        values.insert((n, 1), LatticeValue::Const(ConstValue::Int(1)));
        assert_eq!(
            evaluate_pristine(&stmt, &values, &ssa, FoldPolicy::default()),
            LatticeValue::Overdefined,
            "never the pending bottom a join would launder"
        );
        // The statement-position update holds its target's use and folds.
        let incr = incr_stmt(&mut ssa, "n", None, 1, 2);
        assert_eq!(
            evaluate_pristine(&incr, &values, &ssa, FoldPolicy::default()),
            LatticeValue::Const(ConstValue::Int(2))
        );
    }

    /// `[set x]` in value position reads `x` through the cell-write route;
    /// `[set x 10]` writes storage a value position cannot land
    /// (`EffectFreeOnly`), so its host widens.
    #[test]
    fn evaluate_def_set_read_in_value_position_folds() {
        let mut ssa = bare_ssa();
        let x = ssa.intern_var("x");
        let mut read = assign_value_stmt(&mut ssa, "r", "[set x]", 1);
        read.uses.insert(x, 1);
        let mut write = assign_value_stmt(&mut ssa, "r", "[set x 10]", 1);
        write.uses.insert(x, 1);
        let mut values = HashMap::new();
        values.insert(
            (x, 1),
            LatticeValue::Const(ConstValue::String("hello".into())),
        );
        assert_eq!(
            evaluate_pristine(&read, &values, &ssa, FoldPolicy::default()),
            LatticeValue::Const(ConstValue::String("hello".into()))
        );
        assert_eq!(
            evaluate_pristine(&write, &values, &ssa, FoldPolicy::default()),
            LatticeValue::Overdefined
        );
    }

    /// `[string range "a\tb" 0 1]` in value position reads its words as Tcl
    /// substitutes them: `a` and a tab, where the raw spelling gave `a\`. A
    /// braced word keeps its backslash, and a braced `$x` is text, not a
    /// read (tclsh 8.4 to 9.1 print `a` and a tab, `a\`, and `$x`).
    #[test]
    fn string_range_in_value_position_reads_cooked_escapes() {
        let mut ssa = bare_ssa();
        let xy = ssa.intern_var("xy");
        let quoted = assign_value_stmt(&mut ssa, "r", "[string range \"a\\tb\" 0 1]", 1);
        let braced = assign_value_stmt(&mut ssa, "r", "[string range {a\\tb} 0 1]", 1);
        let mut dollar = assign_value_stmt(&mut ssa, "r", "[string range {$xy} 0 1]", 1);
        dollar.uses.insert(xy, 1);
        let text = |value: &str| LatticeValue::Const(ConstValue::String(value.to_owned()));
        let mut values = HashMap::new();
        values.insert((xy, 1), text("abc"));
        let fold =
            |stmt: &SsaStatement| evaluate_pristine(stmt, &values, &ssa, FoldPolicy::default());
        assert_eq!(fold(&quoted), text("a\t"));
        assert_eq!(fold(&braced), text("a\\"));
        assert_eq!(fold(&dollar), text("$x"), "the braced word is text");
    }

    /// `[string range …]` in value position runs the registry's route: the
    /// index numerals read under the target's grammar. `f5-irules` reads
    /// them as its declared 8.4 base does (ruling 8; tclsh 8.4 prints
    /// `ijkl`), and the lenient `tcl` profile, naming no release, declines.
    #[test]
    fn evaluate_def_assign_value_folds_string_range_under_the_release() {
        let mut ssa = bare_ssa();
        let stmt = assign_value_stmt(&mut ssa, "s", "[string range abcdefghijkl 010 end]", 1);
        let under = |dialect: &str| {
            let folds = folds_for(dialect);
            evaluate_def_with_folds(
                &stmt,
                &HashMap::new(),
                &ssa,
                FoldPolicy::from_registry(folds.registry),
                Some(folds),
            )
        };
        assert_eq!(
            under("tcl8.6"),
            LatticeValue::Const(ConstValue::String("ijkl".into()))
        );
        assert_eq!(
            under("tcl9.0"),
            LatticeValue::Const(ConstValue::String("kl".into()))
        );
        assert_eq!(
            under("f5-irules"),
            LatticeValue::Const(ConstValue::String("ijkl".into()))
        );
        assert_eq!(under("tcl"), LatticeValue::Overdefined);
        let plain = assign_value_stmt(&mut ssa, "t", "[string range { a } 0 end]", 1);
        assert_eq!(
            evaluate_pristine(&plain, &HashMap::new(), &ssa, FoldPolicy::default()),
            LatticeValue::Const(ConstValue::String(" a ".into()))
        );
    }

    /// The run records each statement's route and answer for the
    /// Explorer, at the fixed point.
    #[test]
    fn sccp_records_a_route_explanation_per_statement() {
        let registry = registry();
        let cu = crate::compilation_unit::CompilationUnit::build_for(
            "proc p {} {\n    set n 1\n    incr n\n    append s x\n    set r [string range abc 0 1]\n    return $n\n}\n",
            &registry,
            false,
        );
        let fu = cu.procedures.get("::p").expect("proc");
        let explained: Vec<(String, String)> = fu
            .sccp
            .explanations
            .iter()
            .map(|e| (e.command.clone(), e.answer.clone()))
            .collect();
        assert!(
            explained.contains(&("incr".to_owned(), "evaluated".to_owned())),
            "{explained:?}"
        );
        // The existence rung proves `s` unbound, and `append` creates its
        // cell in every release, so the absent cell evaluates
        // (`the_absent_cell_release_table`: `append s foo` is `foo` under
        // tclsh 8.4 to 9.1).
        assert!(
            explained.contains(&("append".to_owned(), "evaluated".to_owned())),
            "an append over an unbound cell creates it: {explained:?}"
        );
        assert!(
            fu.sccp
                .explanations
                .iter()
                .any(|e| e.command == "string" && e.route.starts_with("direct string-range")),
            "{:?}",
            fu.sccp.explanations
        );
    }

    #[test]
    fn resolve_simple_var_ref_accepts_bare_and_braced() {
        let mut ssa = bare_ssa();
        let x = ssa.intern_var("x");
        let mut uses = HashMap::new();
        uses.insert(x, 1);
        let mut values = HashMap::new();
        values.insert((x, 1), LatticeValue::Const(ConstValue::Int(7)));
        assert_eq!(
            resolve_simple_var_ref("$x", &uses, &values, &ssa),
            Some(LatticeValue::Const(ConstValue::Int(7)))
        );
        assert_eq!(
            resolve_simple_var_ref("${x}", &uses, &values, &ssa),
            Some(LatticeValue::Const(ConstValue::Int(7)))
        );
        assert_eq!(resolve_simple_var_ref("$y", &uses, &values, &ssa), None);
        assert_eq!(resolve_simple_var_ref("plain", &uses, &values, &ssa), None);
    }

    // Foreach constset extraction.

    fn foreach_stmt(ssa: &mut SsaFunction, var: &str, list: &str, new_ver: u32) -> SsaStatement {
        let mut defs = HashMap::new();
        defs.insert(ssa.intern_var(var), new_ver);
        SsaStatement {
            statement: Statement::Call {
                span: Span::new(0, 0),
                command: "foreach".into(),
                canonical_command: None,
                args: vec![list.into()],
                defs: vec![var.into()],
                reads: Vec::new(),
                reads_own_defs: false,
                safe_on_uninit: false,
                tokens: None,
                // The CFG builder marks the synthetic loop header it emits
                // with its iterator-group sizes; the driver keys the header
                // layout on that typed fact, never on the command's name.
                foreach_groups: Some(vec![1]),
            },
            uses: HashMap::new(),
            defs,
            may_defs: std::collections::HashSet::new(),
            destruction_defs: std::collections::HashSet::new(),
            quoted_uses: std::collections::HashSet::new(),
            name_only_uses: std::collections::HashSet::new(),
        }
    }

    #[test]
    fn extract_foreach_elements_literal_list() {
        // `list_text` is already delimiter-stripped by the segmenter, matching
        // the shape `foreach v {a b c}`'s `list_arg` is built in: the word's
        // own `{…}` is gone, leaving plain whitespace-separated text.
        assert_eq!(
            extract_foreach_elements("a b c", tcl_syntax::word_rules::WordValueRules::TCL),
            Some(vec!["a".into(), "b".into(), "c".into()])
        );
    }

    #[test]
    fn extract_foreach_elements_rejects_substitutions() {
        assert_eq!(
            extract_foreach_elements("$lst", tcl_syntax::word_rules::WordValueRules::TCL),
            None
        );
        assert_eq!(
            extract_foreach_elements("[list a b c]", tcl_syntax::word_rules::WordValueRules::TCL),
            None
        );
    }

    #[test]
    fn extract_foreach_elements_splits_nested_braces_as_tcl_list() {
        // `a {b c} d` is a three-element Tcl list — `a`, `b c`, `d` — not the
        // four whitespace runs `a`, `{b`, `c}`, `d`. A naive `split_ascii_whitespace`
        // corrupted the CONSTSET; the list-aware split fixes it.
        assert_eq!(
            extract_foreach_elements("a {b c} d", tcl_syntax::word_rules::WordValueRules::TCL),
            Some(vec!["a".into(), "b c".into(), "d".into()])
        );
        // Backslash-escaped whitespace groups an element too: `a\ b c` is two
        // elements `a b` and `c`.
        assert_eq!(
            extract_foreach_elements("a\\ b c", tcl_syntax::word_rules::WordValueRules::TCL),
            Some(vec!["a b".into(), "c".into()])
        );
    }

    #[test]
    fn extract_foreach_elements_empty_list_returns_empty() {
        assert_eq!(
            extract_foreach_elements("", tcl_syntax::word_rules::WordValueRules::TCL),
            Some(Vec::new())
        );
    }

    // `list_text` already has its outer
    // `foreach`-word delimiter removed by the segmenter (see
    // `Statement::Foreach::list_arg`'s construction in
    // `lowering::structured`), so a value that itself starts with `{` / `"`
    // is a *nested* list element, not a delimiter to peel a second time.
    #[test]
    fn extract_foreach_elements_single_element_braced_list_not_double_stripped() {
        // Source `foreach v {{a b c}}`: the segmenter strips the word's own
        // outer `{…}`, leaving the single-element list `{a b c}` as
        // `list_text`. Splitting it as a Tcl list yields the one element
        // `a b c`, not the three whitespace-separated words `a`, `b`, `c`
        // a second brace-strip-then-split would wrongly produce.
        assert_eq!(
            extract_foreach_elements("{a b c}", tcl_syntax::word_rules::WordValueRules::TCL),
            Some(vec!["a b c".into()])
        );
    }

    #[test]
    fn extract_foreach_elements_two_braced_elements_not_double_stripped() {
        // Source `foreach v {{a b} {c d}}`: `list_text` is `{a b} {c d}`.
        // The pre-fix double-strip peeled the text's own outer `{`/`}` too,
        // yielding the corrupted split `a`, `b}`, `{c`, `d` (lenient split of
        // `a b} {c d`). The correct split is the two nested-list elements.
        assert_eq!(
            extract_foreach_elements("{a b} {c d}", tcl_syntax::word_rules::WordValueRules::TCL),
            Some(vec!["a b".into(), "c d".into()])
        );
    }

    #[test]
    fn extract_foreach_elements_quoted_word_already_delimiter_stripped() {
        // Source `foreach v "a b c"`: the segmenter strips the word's own
        // `"…"` delimiters the same way it strips `{…}`, so `list_text` is
        // plain `a b c` — no quote handling is needed (or wanted) here.
        assert_eq!(
            extract_foreach_elements("a b c", tcl_syntax::word_rules::WordValueRules::TCL),
            Some(vec!["a".into(), "b".into(), "c".into()])
        );
    }

    #[test]
    fn evaluate_def_foreach_literal_list_folds_constset() {
        // `list` mirrors `list_arg` as the segmenter hands it: source
        // `foreach v {1 2 3}`'s outer `{…}` is already stripped.
        let mut ssa = bare_ssa();
        let stmt = foreach_stmt(&mut ssa, "v", "1 2 3", 1);
        let result = evaluate_pristine(&stmt, &HashMap::new(), &ssa, FoldPolicy::default());
        match result {
            LatticeValue::ConstSet(ref vs) => {
                assert_eq!(vs.len(), 3);
                assert!(vs.contains(&ConstValue::Int(1)));
                assert!(vs.contains(&ConstValue::Int(3)));
            }
            other => panic!("expected ConstSet, got {other:?}"),
        }
    }

    #[test]
    fn evaluate_def_foreach_list_cmd_subst_folds_constset() {
        // `foreach v [list a b c]` folds through the declared `list` route to the
        // same element CONSTSET as the braced-literal form.
        let mut ssa = bare_ssa();
        let stmt = foreach_stmt(&mut ssa, "v", "[list a b c]", 1);
        let result = evaluate_pristine(&stmt, &HashMap::new(), &ssa, FoldPolicy::default());
        match result {
            LatticeValue::ConstSet(ref vs) => {
                assert_eq!(vs.len(), 3);
                assert!(vs.contains(&ConstValue::String("a".into())));
                assert!(vs.contains(&ConstValue::String("c".into())));
            }
            other => panic!("expected ConstSet, got {other:?}"),
        }
    }

    #[test]
    fn evaluate_def_foreach_single_element_folds_const() {
        // `list` mirrors `list_arg` for source `foreach v {only}`: the
        // segmenter's delimiter strip leaves the bare word `only`.
        let mut ssa = bare_ssa();
        let stmt = foreach_stmt(&mut ssa, "v", "only", 1);
        assert_eq!(
            evaluate_pristine(&stmt, &HashMap::new(), &ssa, FoldPolicy::default()),
            LatticeValue::Const(ConstValue::String("only".into()))
        );
    }

    #[test]
    fn evaluate_def_foreach_nested_braced_elements_folds_constset() {
        // Source `foreach v {{a b} {c d}}` hands `list_arg` = `{a b} {c d}`
        // (only the word's own outer `{…}` is stripped by the segmenter).
        // Peeling a *second* level of bracing off this already
        // delimiter-stripped text would fold `v` to the corrupted CONSTSET
        // {"a", "b}"} instead of the two list elements.
        let mut ssa = bare_ssa();
        let stmt = foreach_stmt(&mut ssa, "v", "{a b} {c d}", 1);
        let result = evaluate_pristine(&stmt, &HashMap::new(), &ssa, FoldPolicy::default());
        match result {
            LatticeValue::ConstSet(ref vs) => {
                assert_eq!(vs.len(), 2);
                assert!(vs.contains(&ConstValue::String("a b".into())));
                assert!(vs.contains(&ConstValue::String("c d".into())));
                assert!(!vs.contains(&ConstValue::String("b}".into())));
            }
            other => panic!("expected ConstSet, got {other:?}"),
        }
    }

    #[test]
    fn evaluate_def_foreach_via_lattice_var() {
        let mut ssa = bare_ssa();
        let mut stmt = foreach_stmt(&mut ssa, "v", "$lst", 1);
        let lst = ssa.intern_var("lst");
        stmt.uses.insert(lst, 1);
        let mut values = HashMap::new();
        values.insert(
            (lst, 1),
            LatticeValue::Const(ConstValue::String("a b c".into())),
        );
        let result = evaluate_pristine(&stmt, &values, &ssa, FoldPolicy::default());
        match result {
            LatticeValue::ConstSet(ref vs) => assert_eq!(vs.len(), 3),
            other => panic!("expected ConstSet, got {other:?}"),
        }
    }

    #[test]
    fn evaluate_def_foreach_unbound_var_widens() {
        let mut ssa = bare_ssa();
        let mut stmt = foreach_stmt(&mut ssa, "v", "$lst", 1);
        let lst = ssa.intern_var("lst");
        stmt.uses.insert(lst, 1);
        // Empty lattice — var not bound.
        let result = evaluate_pristine(&stmt, &HashMap::new(), &ssa, FoldPolicy::default());
        assert_eq!(result, LatticeValue::Overdefined);
    }

    #[test]
    fn evaluate_def_foreach_multi_var_binds_each_its_elements() {
        // Two binders over one list: each takes the elements it is
        // assigned, so over `a b` the first binder, `v`, holds `a`.
        let mut ssa = bare_ssa();
        let mut stmt = foreach_stmt(&mut ssa, "v", "a b", 1);
        let Statement::Call { defs, .. } = &mut stmt.statement else {
            panic!();
        };
        defs.push("w".into());
        let result = evaluate_pristine(&stmt, &HashMap::new(), &ssa, FoldPolicy::default());
        assert_eq!(result, LatticeValue::Const(ConstValue::String("a".into())));
    }

    // AssignValue + command-substitution folding.

    fn assign_value_stmt(ssa: &mut SsaFunction, name: &str, value: &str, ver: u32) -> SsaStatement {
        let mut defs = HashMap::new();
        defs.insert(ssa.intern_var(name), ver);
        SsaStatement {
            statement: Statement::AssignValue {
                span: Span::new(0, 0),
                name: name.into(),
                name_braced: false,
                value: value.into(),
                value_needs_backsubst: false,
                tokens: None,
            },
            uses: HashMap::new(),
            defs,
            may_defs: std::collections::HashSet::new(),
            destruction_defs: std::collections::HashSet::new(),
            quoted_uses: std::collections::HashSet::new(),
            name_only_uses: std::collections::HashSet::new(),
        }
    }

    #[test]
    fn evaluate_def_assign_value_plain_literal() {
        let mut ssa = bare_ssa();
        let stmt = assign_value_stmt(&mut ssa, "x", "hello", 1);
        assert_eq!(
            evaluate_def(&stmt, &HashMap::new(), &ssa, FoldPolicy::default()),
            LatticeValue::Const(ConstValue::String("hello".into()))
        );
    }

    #[test]
    fn evaluate_def_assign_value_integer_literal() {
        let mut ssa = bare_ssa();
        let stmt = assign_value_stmt(&mut ssa, "x", "42", 1);
        assert_eq!(
            evaluate_def(&stmt, &HashMap::new(), &ssa, FoldPolicy::default()),
            LatticeValue::Const(ConstValue::Int(42))
        );
    }

    #[test]
    fn evaluate_def_assign_value_resolves_var_ref() {
        let mut ssa = bare_ssa();
        let mut stmt = assign_value_stmt(&mut ssa, "y", "$x", 1);
        let x = ssa.intern_var("x");
        stmt.uses.insert(x, 1);
        let mut values = HashMap::new();
        values.insert((x, 1), LatticeValue::Const(ConstValue::Int(7)));
        assert_eq!(
            evaluate_def(&stmt, &values, &ssa, FoldPolicy::default()),
            LatticeValue::Const(ConstValue::Int(7))
        );
    }

    #[test]
    fn evaluate_def_assign_value_folds_list_cmd() {
        let mut ssa = bare_ssa();
        let stmt = assign_value_stmt(&mut ssa, "x", "[list a b c]", 1);
        let result = evaluate_pristine(&stmt, &HashMap::new(), &ssa, FoldPolicy::default());
        match result {
            LatticeValue::Const(ConstValue::String(s)) => assert_eq!(s, "a b c"),
            other => panic!("expected Const(String), got {other:?}"),
        }
    }

    #[test]
    fn evaluate_def_assign_value_folds_llength_literal() {
        let mut ssa = bare_ssa();
        let stmt = assign_value_stmt(&mut ssa, "n", "[llength {a b c d}]", 1);
        assert_eq!(
            evaluate_pristine(&stmt, &HashMap::new(), &ssa, FoldPolicy::default()),
            LatticeValue::Const(ConstValue::Int(4))
        );
    }

    #[test]
    fn evaluate_def_assign_value_folds_string_length() {
        let mut ssa = bare_ssa();
        let stmt = assign_value_stmt(&mut ssa, "n", "[string length \"hello\"]", 1);
        assert_eq!(
            evaluate_pristine(&stmt, &HashMap::new(), &ssa, FoldPolicy::default()),
            LatticeValue::Const(ConstValue::Int(5))
        );
    }

    // The fold arms answer to the command-binding trust fact.

    /// The whole-module mutation summary for `src`.
    fn mutations_for(src: &str) -> crate::command_binding::ModuleCommandMutations {
        let reg = registry();
        let ir = crate::lowering::lower_to_ir(src, &reg);
        crate::command_binding::scan_module_command_mutations(&ir, &reg)
    }

    /// Evaluate `stmt` under the whole-module trust fact `mutations`.
    fn evaluate_under_unit(
        stmt: &SsaStatement,
        ssa: &SsaFunction,
        registry: &CommandRegistry,
        mutations: &crate::command_binding::ModuleCommandMutations,
    ) -> LatticeValue {
        evaluate_def_with_folds(
            stmt,
            &HashMap::new(),
            ssa,
            FoldPolicy::default(),
            Some(BuiltinFoldInputs {
                source_metadata_input: None,
                registry,
                mutations,
                dialect: None,
                defining_class: None,
                registry_engine: false,
                trust: FoldTrust::WholeModule,
                proven_pure_parameters: false,
            }),
        )
    }

    /// The `[list …]` / `[format …]` arms run ahead of the registry engine, so
    /// they must apply its trust gate themselves — otherwise a unit that
    /// renamed `list` still gets the builtin's answer.
    ///
    /// tclsh 8.6.16 / 9.0.4 (identical): after `rename list mylist`, evaluating
    /// `list a b c` raises rather than returning `a b c`.
    #[test]
    fn list_arm_declines_once_the_unit_renames_list() {
        let reg = registry();
        let mut ssa = bare_ssa();
        let stmt = assign_value_stmt(&mut ssa, "x", "[list a b c]", 1);

        let untouched = mutations_for("set y 1\n");
        assert_eq!(
            evaluate_under_unit(&stmt, &ssa, &reg, &untouched),
            LatticeValue::Const(ConstValue::String("a b c".into())),
            "an untouched `list` still folds"
        );

        let renamed = mutations_for("rename list mylist\n");
        assert_eq!(
            evaluate_under_unit(&stmt, &ssa, &reg, &renamed),
            LatticeValue::Overdefined,
            "a renamed `list` must not fold to the builtin's answer"
        );
    }

    /// The same gate, on an arm reached through the head-word check rather
    /// than through its own early return — and per name: renaming `llength`
    /// must not cost `list` its fold.
    #[test]
    fn head_word_arms_decline_only_for_the_name_the_unit_touched() {
        let reg = registry();
        let mut ssa = bare_ssa();
        let count = assign_value_stmt(&mut ssa, "n", "[llength {a b c d}]", 1);
        let listing = assign_value_stmt(&mut ssa, "x", "[list a b c]", 2);

        let renamed = mutations_for("rename llength myll\n");
        assert_eq!(
            evaluate_under_unit(&count, &ssa, &reg, &renamed),
            LatticeValue::Overdefined,
            "a renamed `llength` must not fold to the builtin's answer"
        );
        assert_eq!(
            evaluate_under_unit(&listing, &ssa, &reg, &renamed),
            LatticeValue::Const(ConstValue::String("a b c".into())),
            "`list` is untouched by a `rename llength` and keeps its fold"
        );
    }

    /// Evaluate `stmt` under `mutations` with the shared per-unit lattice's
    /// stance ([`FoldTrust::ObservedBindings`]).
    fn evaluate_under_lattice_stance(
        stmt: &SsaStatement,
        ssa: &SsaFunction,
        registry: &CommandRegistry,
        mutations: &crate::command_binding::ModuleCommandMutations,
    ) -> LatticeValue {
        evaluate_def_with_folds(
            stmt,
            &HashMap::new(),
            ssa,
            FoldPolicy::default(),
            Some(BuiltinFoldInputs {
                source_metadata_input: None,
                registry,
                mutations,
                dialect: None,
                defining_class: None,
                registry_engine: false,
                trust: FoldTrust::ObservedBindings,
                proven_pure_parameters: false,
            }),
        )
    }

    /// A user `proc` shadowing a builtin is a *named* takeover, so both
    /// stances decline it. This is the root of #2164: the shared per-unit
    /// lattice took no trust fact at all and answered `3` for a
    /// `[llength {a b c}]` the module resolves to a proc returning 99, which
    /// the optimiser then published as `return 3` beside its own `set v 99`.
    ///
    /// tclsh 8.4.20 / 8.5.19 / 8.6.18 / 9.0.4 / 9.1b0 (unanimous): with
    /// `proc llength {l} {return 99}` in scope, `[llength {a b c}]` is 99.
    #[test]
    fn both_stances_decline_a_builtin_a_proc_shadows() {
        let reg = registry();
        let mut ssa = bare_ssa();
        let stmt = assign_value_stmt(&mut ssa, "n", "[llength {a b c}]", 1);

        let shadowed = mutations_for("proc llength {l} { return 99 }\n");
        assert_eq!(
            evaluate_under_lattice_stance(&stmt, &ssa, &reg, &shadowed),
            LatticeValue::Overdefined,
            "the lattice must not answer with builtin semantics for a shadowed name"
        );
        assert_eq!(
            evaluate_under_unit(&stmt, &ssa, &reg, &shadowed),
            LatticeValue::Overdefined,
            "nor may the rewrite stance"
        );

        // Positive control: the same statement, the same two stances, a module
        // that shadows nothing — both fold, so the declines above are the
        // shadow's doing and not a dead arm.
        let untouched = mutations_for("set y 1\n");
        assert_eq!(
            evaluate_under_lattice_stance(&stmt, &ssa, &reg, &untouched),
            LatticeValue::Const(ConstValue::Int(3)),
        );
        assert_eq!(
            evaluate_under_unit(&stmt, &ssa, &reg, &untouched),
            LatticeValue::Const(ConstValue::Int(3)),
        );
    }

    /// Where the two stances deliberately differ: one command head the
    /// registry cannot resolve raises the summary's unbounded `dynamic` top,
    /// which names no subject. The rewrite stance declines on it (a rewrite
    /// must be certain); the lattice stance keeps folding, or a single
    /// unknown library call would cost a file every constant it has.
    #[test]
    fn only_the_rewrite_stance_declines_on_the_unbounded_top() {
        let reg = registry();
        let mut ssa = bare_ssa();
        let stmt = assign_value_stmt(&mut ssa, "n", "[llength {a b c}]", 1);

        let opaque = mutations_for("someUnknownLibraryCall x\n");
        assert!(
            opaque.has_dynamic_mutation(),
            "an unresolved head is expected to raise the unbounded top"
        );
        assert!(
            opaque.observed_binding_is_the_builtin("llength"),
            "but it names no subject, so `llength`'s observed binding is intact"
        );
        assert_eq!(
            evaluate_under_lattice_stance(&stmt, &ssa, &reg, &opaque),
            LatticeValue::Const(ConstValue::Int(3)),
        );
        assert_eq!(
            evaluate_under_unit(&stmt, &ssa, &reg, &opaque),
            LatticeValue::Overdefined,
            "a fold that becomes a rewrite declines on the unbounded top"
        );
    }

    /// A `rename` whose **subject** the scan cannot name distrusts every
    /// builtin, where an unresolved command *head* does not (#2168).
    ///
    /// Both raise the unbounded top, which is why gating the lattice on the
    /// whole of it was rejected in #2164 — it would take the fold in
    /// [`only_the_rewrite_stance_declines_on_the_unbounded_top`] with it. The
    /// distinction is whether something was definitely rebound: `rename $a {}`
    /// moved *some* command, so no name is claimable; `someUnknownLibraryCall
    /// x` moved nothing.
    ///
    /// tclsh 8.6.18 and 9.0.4 both print 99 for the repro on #2168, where the
    /// optimiser rewrote the body to `return 3` — and its own output literally
    /// read `rename llength {}`, having constant-propagated the operands in
    /// the same run.
    #[test]
    fn an_unnameable_rename_subject_distrusts_where_an_unknown_head_does_not() {
        let reg = registry();
        let mut ssa = bare_ssa();
        let stmt = assign_value_stmt(&mut ssa, "n", "[llength {a b c}]", 1);

        let computed = mutations_for(
            "set a llength
rename $a {}
",
        );
        assert!(
            computed.has_dynamic_mutation(),
            "a computed rename raises the unbounded top"
        );
        assert!(
            !computed.observed_binding_is_the_builtin("llength"),
            "and, unlike an unknown head, it leaves no name claimable"
        );
        assert_eq!(
            evaluate_under_lattice_stance(&stmt, &ssa, &reg, &computed),
            LatticeValue::Overdefined,
            "the lattice must not fold a builtin a computed rename may have moved"
        );

        // Positive control, and the coverage #2164 measured: an unresolved
        // head still raises the top yet still folds, so this change cannot
        // have been bought by gating on `dynamic`.
        let opaque = mutations_for(
            "someUnknownLibraryCall x
",
        );
        assert!(opaque.has_dynamic_mutation());
        assert_eq!(
            evaluate_under_lattice_stance(&stmt, &ssa, &reg, &opaque),
            LatticeValue::Const(ConstValue::Int(3)),
        );
    }

    /// The same rule for a body the source-recursive walk never sees. A proc
    /// installed through an alias prefix is recovered only by the closed
    /// command lattice, so its unnameable rename subject reaches the optimiser
    /// through `mutation_projection` or not at all (#2168).
    ///
    /// Measured end to end before this was joined: the program below prints
    /// **99**, and `tcl optimise` rewrote it into one printing **3** on tclsh
    /// 8.6.18 and 9.0.4.
    #[test]
    fn an_unnameable_rename_subject_inside_an_alias_defined_body_also_distrusts() {
        let reg = registry();
        let mut ssa = bare_ssa();
        let stmt = assign_value_stmt(&mut ssa, "n", "[llength {a b c}]", 1);

        // `makep {} BODY` is `proc p {} BODY`; BODY is never walked as source.
        let aliased = mutations_for(
            "proc mylen {l} { return 99 }
interp alias {} makep {} proc p
makep {} {set a llength; set b mylen; rename $a {}; rename $b $a}
p
",
        );
        assert!(
            !aliased.observed_binding_is_the_builtin("llength"),
            "the projection is the only witness that the subject was unnameable"
        );
        assert_eq!(
            evaluate_under_lattice_stance(&stmt, &ssa, &reg, &aliased),
            LatticeValue::Overdefined,
            "folding here rewrites a program meaning 99 into one meaning 3"
        );
    }

    #[test]
    fn string_length_fold_counts_in_the_selected_dialects_character_model() {
        // U+1D11E written in a UTF-8 source file: tclsh 9.0 and 9.1 decode
        // the source as UTF-8 and count one scalar; 8.4 to 8.6 decode it in
        // the system encoding and print 4. The registry's route answers
        // under the dialect's release and declines where the source's
        // decoding is ambiguous — 8.x and no selected release.
        let mut ssa = bare_ssa();
        let stmt = assign_value_stmt(&mut ssa, "n", "[string length \"\u{1D11E}\"]", 1);
        let fold = |dialect: &str| {
            let folds = folds_for(dialect);
            evaluate_def_with_folds(
                &stmt,
                &HashMap::new(),
                &ssa,
                FoldPolicy::from_registry(folds.registry),
                Some(folds),
            )
        };
        assert_eq!(fold("tcl9.0"), LatticeValue::Const(ConstValue::Int(1)));
        assert_eq!(fold("tcl8.6"), LatticeValue::Overdefined);
        assert_eq!(
            evaluate_pristine(&stmt, &HashMap::new(), &ssa, FoldPolicy::default()),
            LatticeValue::Overdefined,
            "no selected release leaves the source's decoding ambiguous"
        );

        // A string both models count identically still folds with no selected
        // release — declining those would drop an ordinary optimisation.
        let mut ssa = bare_ssa();
        let ascii = assign_value_stmt(&mut ssa, "n", "[string length \"hello\"]", 1);
        assert_eq!(
            evaluate_pristine(
                &ascii,
                &HashMap::new(),
                &ssa,
                FoldPolicy::for_profile(Some(false), None)
            ),
            LatticeValue::Const(ConstValue::Int(5))
        );
    }

    #[test]
    fn evaluate_def_assign_value_folds_expr_cmd_subst() {
        let mut ssa = bare_ssa();
        let stmt = assign_value_stmt(&mut ssa, "x", "[expr {1 + 2}]", 1);
        assert_eq!(
            evaluate_pristine(&stmt, &HashMap::new(), &ssa, FoldPolicy::default()),
            LatticeValue::Const(ConstValue::Int(3))
        );
    }

    #[test]
    fn evaluate_def_assign_value_folds_format_literal() {
        let mut ssa = bare_ssa();
        let stmt = assign_value_stmt(&mut ssa, "s", "[format \"%d-%d\" 1 2]", 1);
        match evaluate_pristine(&stmt, &HashMap::new(), &ssa, FoldPolicy::default()) {
            LatticeValue::Const(ConstValue::String(s)) => assert_eq!(s, "1-2"),
            other => panic!("expected Const(String), got {other:?}"),
        }
    }

    #[test]
    fn quoted_expr_with_string_var_does_not_fold() {
        // `set r [expr "$a == $b"]` with a="alpha", b="beta": Tcl substitutes
        // the values textually before parsing, so `expr "alpha == beta"`
        // errors (`invalid bareword`). The fold must bail rather than treat
        // the strings as operands and return 0.
        let mut ssa = bare_ssa();
        let mut stmt = assign_value_stmt(&mut ssa, "r", "[expr \"$a == $b\"]", 1);
        let a = ssa.intern_var("a");
        let b = ssa.intern_var("b");
        stmt.uses.insert(a, 1);
        stmt.uses.insert(b, 1);
        let mut values = HashMap::new();
        values.insert(
            (a, 1),
            LatticeValue::Const(ConstValue::String("alpha".into())),
        );
        values.insert(
            (b, 1),
            LatticeValue::Const(ConstValue::String("beta".into())),
        );
        assert_eq!(
            evaluate_def(&stmt, &values, &ssa, FoldPolicy::default()),
            LatticeValue::Overdefined
        );
    }

    #[test]
    fn quoted_expr_with_numeric_var_still_folds() {
        // `set r [expr "$a + $b"]` with numeric a, b is sound: textual
        // substitution yields `3 + 4`, a valid expr → fold to 7.
        let mut ssa = bare_ssa();
        let mut stmt = assign_value_stmt(&mut ssa, "r", "[expr \"$a + $b\"]", 1);
        let a = ssa.intern_var("a");
        let b = ssa.intern_var("b");
        stmt.uses.insert(a, 1);
        stmt.uses.insert(b, 1);
        let mut values = HashMap::new();
        values.insert((a, 1), LatticeValue::Const(ConstValue::Int(3)));
        values.insert((b, 1), LatticeValue::Const(ConstValue::Int(4)));
        assert_eq!(
            evaluate_pristine(&stmt, &values, &ssa, FoldPolicy::default()),
            LatticeValue::Const(ConstValue::Int(7))
        );
    }

    #[test]
    fn braced_expr_with_string_var_folds_as_string_compare() {
        // `set r [expr {$a == $b}]` is braced — expr resolves the vars itself,
        // so a string-valued var is a valid operand and the compare folds.
        let mut ssa = bare_ssa();
        let mut stmt = assign_value_stmt(&mut ssa, "r", "[expr {$a == $b}]", 1);
        let a = ssa.intern_var("a");
        let b = ssa.intern_var("b");
        stmt.uses.insert(a, 1);
        stmt.uses.insert(b, 1);
        let mut values = HashMap::new();
        values.insert(
            (a, 1),
            LatticeValue::Const(ConstValue::String("alpha".into())),
        );
        values.insert(
            (b, 1),
            LatticeValue::Const(ConstValue::String("beta".into())),
        );
        assert_eq!(
            evaluate_pristine(&stmt, &values, &ssa, FoldPolicy::default()),
            LatticeValue::Const(ConstValue::Int(0))
        );
    }

    #[test]
    fn evaluate_def_assign_value_unknown_cmd_widens() {
        let mut ssa = bare_ssa();
        let stmt = assign_value_stmt(&mut ssa, "x", "[nonexistent_fold args]", 1);
        assert_eq!(
            evaluate_pristine(&stmt, &HashMap::new(), &ssa, FoldPolicy::default()),
            LatticeValue::Overdefined
        );
    }

    #[test]
    fn evaluate_def_assign_value_llength_via_lattice_var() {
        let mut ssa = bare_ssa();
        let mut stmt = assign_value_stmt(&mut ssa, "n", "[llength $lst]", 1);
        let lst = ssa.intern_var("lst");
        stmt.uses.insert(lst, 1);
        let mut values = HashMap::new();
        values.insert(
            (lst, 1),
            LatticeValue::Const(ConstValue::String("a b c".into())),
        );
        assert_eq!(
            evaluate_pristine(&stmt, &values, &ssa, FoldPolicy::default()),
            LatticeValue::Const(ConstValue::Int(3))
        );
    }

    #[test]
    fn parse_literal_value_prefers_int() {
        assert_eq!(parse_literal_value("42"), ConstValue::Int(42));
        assert_eq!(parse_literal_value("-5"), ConstValue::Int(-5));
        assert_eq!(parse_literal_value("0"), ConstValue::Int(0));
        assert_eq!(
            parse_literal_value("hello"),
            ConstValue::String("hello".into())
        );
        // Leading-zero and non-canonical integer forms do not round-trip, so
        // they stay strings (lets SCCP apply the per-dialect leading-zero rule).
        assert_eq!(parse_literal_value("08"), ConstValue::String("08".into()));
        assert_eq!(parse_literal_value("010"), ConstValue::String("010".into()));
        assert_eq!(parse_literal_value("+5"), ConstValue::String("+5".into()));
        assert_eq!(parse_literal_value("-0"), ConstValue::String("-0".into()));
    }

    #[test]
    fn cfg_order_appends_unreachable_blocks() {
        let mut f = Function::new("::top", "entry");
        let entry = f.entry;
        let dead = block(&mut f, "dead");
        f.blocks.get_mut(&entry).unwrap().terminator = Some(Terminator::Return {
            value: None,
            value_word: None,
            span: None,
            expr: None,
            expr_base: None,
            tokens: None,
            braced: false,
        });
        f.blocks.get_mut(&dead).unwrap().terminator = Some(Terminator::Return {
            value: None,
            value_word: None,
            span: None,
            expr: None,
            expr_base: None,
            tokens: None,
            braced: false,
        });
        let order = cfg_order(&f);
        assert!(order.contains(&entry));
        assert!(order.contains(&dead));
    }

    fn cu(src: &str) -> crate::compilation_unit::CompilationUnit {
        crate::compilation_unit::CompilationUnit::build_for(
            src,
            &tcl_registry::CommandRegistry::build_default(),
            false,
        )
    }

    /// A condition resolves a nested pure command through the driver's
    /// services: with `acc` the constant `foobar`, `[string length $acc] ==
    /// 6` is true (tclsh 8.4 to 9.1 take the branch); with `acc` a
    /// parameter nothing is known and the branch stays open.
    #[test]
    fn evaluate_branch_resolves_a_nested_command() {
        let known = "proc ::p {} {\n set acc foobar\n if {[string length $acc] == 6} { return 1 } else { return 0 }\n}";
        let unit = cu(known);
        let f = unit.function("::p").unwrap();
        let result = sccp_pristine(&f.cfg, &f.ssa, FoldPolicy::default());
        assert!(
            result
                .constant_branches
                .iter()
                .any(|branch| branch.value && branch.condition.contains("string length")),
            "{:?}",
            result.constant_branches
        );
        let open =
            "proc ::p {acc} {\n if {[string length $acc] == 6} { return 1 } else { return 0 }\n}";
        let unit = cu(open);
        let f = unit.function("::p").unwrap();
        let result = sccp_pristine(&f.cfg, &f.ssa, FoldPolicy::default());
        assert!(
            result.constant_branches.is_empty(),
            "{:?}",
            result.constant_branches
        );
    }

    #[test]
    fn sccp_folds_post_loop_branch_via_static_summary() {
        // After `for {set i 0} {$i < 10} {incr i} {}` tclsh leaves `i == 10`,
        // so the following `if {$i == 10}` is statically true. SCCP cannot fold
        // a loop-carried phi, but the static-loop summary simulates the loop
        // and folds the branch. Verified against tclsh 8.4-9.0 (i == 10, and
        // the accumulator j == 5, hold after the loops).
        let c = cu(
            "proc ::p {} { for {set i 0} {$i < 10} {incr i} {}\n if {$i == 10} { return yes } else { return no } }",
        );
        let fu = c.function("::p").unwrap();
        let r = sccp_no_traces(&fu.cfg, &fu.ssa, None, FoldPolicy::default());
        let cb = r
            .constant_branches
            .iter()
            .find(|cb| cb.condition.contains("$i == 10"))
            .expect("post-loop branch must fold via the static-loop summary");
        assert!(cb.value, "i == 10 after the loop, so the branch is true");

        // Body side effects are simulated too: j accumulates to 5.
        let ca = cu(
            "proc ::a {} { set j 0\n for {set k 5} {$k > 0} {incr k -1} { incr j }\n if {$j == 5} { return yes } else { return no } }",
        );
        let fa = ca.function("::a").unwrap();
        let ra = sccp_no_traces(&fa.cfg, &fa.ssa, None, FoldPolicy::default());
        let cba = ra
            .constant_branches
            .iter()
            .find(|cb| cb.condition.contains("$j == 5"))
            .expect("accumulator branch must fold via the static-loop summary");
        assert!(cba.value, "j == 5 after the loop");

        // A loop with an unknown (parameter) bound cannot be summarised, so the
        // post-loop branch stays unfolded (conservative).
        let cq = cu(
            "proc ::q {n} { for {set i 0} {$i < $n} {incr i} {}\n if {$i == 10} { return yes } else { return no } }",
        );
        let fq = cq.function("::q").unwrap();
        let rq = sccp_no_traces(&fq.cfg, &fq.ssa, None, FoldPolicy::default());
        assert!(
            !rq.constant_branches
                .iter()
                .any(|cb| cb.condition.contains("$i == 10")),
            "an unknown loop bound must not fold the post-loop branch"
        );
    }

    #[test]
    fn sccp_foreach_nested_braced_list_constset_not_corrupted() {
        // End to end: `foreach v {{a b} {c d}}` lowers `list_arg` to the
        // segmenter-stripped text `{a b} {c d}` (only the word's own outer
        // braces are gone). Peeling a *second* level of bracing off this
        // already-stripped text corrupts `v`'s CONSTSET to the two elements
        // `a` and `b}` (from the lenient split of `a b} {c d`) and wrongly
        // proves `if {$v eq "c d"}` false. It must fold to the two correct
        // elements `a b` and `c d` instead.
        let c = cu(
            "proc ::p {} { foreach v {{a b} {c d}} { if {$v eq \"c d\"} { set r yes } else { set r no } } }",
        );
        let fu = c.function("::p").unwrap();
        let r = sccp_pristine(&fu.cfg, &fu.ssa, FoldPolicy::default());
        let v = fu.ssa.var_symbol("v").expect("v must be an SSA symbol");
        let const_sets: Vec<_> = r
            .values
            .iter()
            .filter(|((sym, _), _)| *sym == v)
            .filter_map(|(_, val)| match val {
                LatticeValue::ConstSet(vs) => Some(vs.clone()),
                _ => None,
            })
            .collect();
        assert!(
            !const_sets
                .iter()
                .any(|vs| vs.contains(&ConstValue::String("b}".into()))),
            "v's CONSTSET must not contain the corrupted element \"b}}\": {const_sets:?}"
        );
        assert!(
            const_sets.iter().any(|vs| vs.len() == 2
                && vs.contains(&ConstValue::String("a b".into()))
                && vs.contains(&ConstValue::String("c d".into()))),
            "expected a CONSTSET {{\"a b\", \"c d\"}} among v's lattice values, got {const_sets:?}"
        );
    }

    #[test]
    fn branch_deferrable_optimism() {
        let cond = ExprNode::Var {
            text: "$x".into(),
            name: "x".into(),
            start: 0,
            end: 2,
        };
        let mut ssa = bare_ssa();
        let x = ssa.intern_var("x");
        let mut sb = empty_ssa_block("b");
        sb.exit_versions.insert(x, 1);
        let mut values: HashMap<ValueKey, LatticeValue> = HashMap::new();

        // Defined operand (version 1) not yet computed → defer.
        assert!(branch_deferrable(
            &sb,
            &cond,
            &values,
            &ssa,
            tcl_dialect::LexerGrammar::default()
        ));
        values.insert((x, 1), LatticeValue::Unknown);
        assert!(branch_deferrable(
            &sb,
            &cond,
            &values,
            &ssa,
            tcl_dialect::LexerGrammar::default()
        ));

        // An `Overdefined` operand proves the condition genuinely
        // non-constant → never defer.
        values.insert((x, 1), LatticeValue::Overdefined);
        assert!(!branch_deferrable(
            &sb,
            &cond,
            &values,
            &ssa,
            tcl_dialect::LexerGrammar::default()
        ));

        // A constant operand folds via `evaluate_branch`, so the `None`
        // arm is never reached → not deferrable here.
        values.insert((x, 1), LatticeValue::Const(ConstValue::Int(1)));
        assert!(!branch_deferrable(
            &sb,
            &cond,
            &values,
            &ssa,
            tcl_dialect::LexerGrammar::default()
        ));

        // Version-0 operands (parameters / globals / live-in roots) are
        // already `Overdefined` and excluded from the deferral test.
        let mut sb0 = empty_ssa_block("b");
        sb0.exit_versions.insert(x, 0);
        assert!(!branch_deferrable(
            &sb0,
            &cond,
            &HashMap::new(),
            &ssa,
            tcl_dialect::LexerGrammar::default()
        ));
    }

    #[test]
    fn sccp_widens_global_aliased_var_to_overdefined() {
        // A `global`-aliased variable is shared mutable state: SCCP must not
        // fold a constant through it, so the `if {$g == 5}` branch stays
        // unresolved (both arms executable, no constant branch). The matching
        // *local* program does fold — proving the widening is what makes the
        // difference, not an unrelated failure to evaluate.
        let global_src =
            "proc ::p {} { global g\n set g 5\n if {$g == 5} { return 1 } else { return 0 } }";
        let local_src = "proc ::p {} { set x 5\n if {$x == 5} { return 1 } else { return 0 } }";

        let cg = cu(global_src);
        let fg = cg.function("::p").unwrap();
        let rg = sccp_no_traces(&fg.cfg, &fg.ssa, None, FoldPolicy::default());
        assert!(
            rg.constant_branches.is_empty(),
            "global var must not fold a constant branch"
        );

        let cl = cu(local_src);
        let fl = cl.function("::p").unwrap();
        let rl = sccp_no_traces(&fl.cfg, &fl.ssa, None, FoldPolicy::default());
        assert!(
            !rl.constant_branches.is_empty(),
            "local var should still fold the constant branch"
        );
    }

    /// Regression: a literal-body `uplevel #0 {…}` (the CFG shape `Statement::
    /// UpFrame`) evaluates its body in the absolute global frame, which can
    /// reassign any name visible there — including one with no `global`/
    /// `variable`/`upvar`/`trace` declaration at all. SCCP must widen every
    /// tracked value across it exactly as it already does for a plain
    /// `Statement::Barrier`. Confirmed against tclsh 8.6/9.0: `set n 5;
    /// uplevel #0 {set n 99}; if {$n == 5} {…}` takes the *else* branch
    /// (`n` is 99), so SCCP must not fold this to a constant-true branch.
    #[test]
    fn sccp_widens_across_upframe_from_literal_uplevel() {
        let with_upframe =
            cu("set n 5\nuplevel #0 { set n 99 }\nif {$n == 5} { set r yes } else { set r no }\n");
        let f = with_upframe.function("::top").unwrap();
        let r = sccp_no_traces(&f.cfg, &f.ssa, None, FoldPolicy::default());
        assert!(
            r.constant_branches.is_empty(),
            "a value reachable through an UpFrame must not fold a constant branch, got {:?}",
            r.constant_branches,
        );
    }

    /// The value the solver gives the last definition of `name` in `f`.
    fn last_value(f: &crate::compilation_unit::FunctionUnit, name: &str) -> LatticeValue {
        let symbol = f
            .ssa
            .var_symbol(name)
            .expect("the variable is an SSA symbol");
        f.sccp
            .values
            .iter()
            .filter(|((sym, _), _)| *sym == symbol)
            .max_by_key(|((_, version), _)| *version)
            .map(|(_, value)| value.clone())
            .expect("a definition")
    }

    /// A `switch` the flow graph keeps as one statement leaves each name its
    /// arms write with no constant value, and a name only an arm binds may be
    /// unbound afterwards: tclsh 8.4 to 9.1 print `0` and `1` for `proc f {s} {
    /// set go 1; switch -glob -- $s { q* { set go 0; set fresh 1 } }; puts $go;
    /// puts $fresh }` called with `q1`, and fail on `$fresh` with `zz`.
    #[test]
    fn an_opaque_switch_arm_leaves_what_it_writes_without_a_constant_or_a_settled_existence() {
        let registry = CommandRegistry::build_default();
        for arm in ["-glob", "-nocase", "-regexp"] {
            let cu = crate::compilation_unit::CompilationUnit::build_for(
                &format!(
                    "proc f {{s}} {{ set go 1; switch {arm} -- $s {{ q* {{ set go 0; set fresh 1 }} }}; puts $go; puts $fresh }}"
                ),
                &registry,
                false,
            );
            let f = cu.function("::f").expect("procedure analysed");
            assert_eq!(last_value(f, "go"), LatticeValue::Overdefined, "{arm}");
            assert_eq!(puts_reads(f, "fresh"), [Existence::MayBound], "{arm}");
        }
        // A name no arm writes keeps its constant.
        let cu = crate::compilation_unit::CompilationUnit::build_for(
            "proc f {s} { set go 1; switch -glob -- $s { q* { set other 0 } }; puts $go }",
            &registry,
            false,
        );
        let f = cu.function("::f").expect("procedure analysed");
        assert_eq!(last_value(f, "go"), LatticeValue::Const(ConstValue::Int(1)));
    }

    /// What a command an arm runs does to the frame is as real as a write the
    /// arm makes itself: a callee that writes the caller's name through
    /// `upvar`, and a command that may write any name (`namespace eval`,
    /// `dict with`), leave the name with no constant, while a callee that
    /// writes only a global leaves a local alone.
    #[test]
    fn an_opaque_switch_arm_that_runs_a_writer_or_a_barrier_leaves_the_name_unsettled() {
        let registry = CommandRegistry::build_default();
        for source in [
            "proc zero {v} {upvar 1 $v x; set x 0}\nproc f {s} { set go 1; switch -glob -- $s { q* { zero go } }; puts $go }",
            "proc f {s} { set go 1; switch -glob -- $s { q* { dict with d { set go 0 } } }; puts $go }",
            "proc f {s} { set go 1; switch -glob -- $s { q* { eval $s } }; puts $go }",
        ] {
            let cu = crate::compilation_unit::CompilationUnit::build_for(source, &registry, false);
            let f = cu.function("::f").expect("procedure analysed");
            assert_eq!(last_value(f, "go"), LatticeValue::Overdefined, "{source}");
        }
        let cu = crate::compilation_unit::CompilationUnit::build_for(
            "set go 1\nswitch -glob -- [gets stdin] { q* { namespace eval :: {set go 0} } }\nputs $go\n",
            &registry,
            false,
        );
        let f = cu.function("::top").expect("top level analysed");
        assert_eq!(last_value(f, "go"), LatticeValue::Overdefined);
        let cu = crate::compilation_unit::CompilationUnit::build_for(
            "proc hit {} {global hits; incr hits}\nproc f {s} { set go 1; switch -glob -- $s { q* { hit } }; puts $go }",
            &registry,
            false,
        );
        let f = cu.function("::f").expect("procedure analysed");
        assert_eq!(last_value(f, "go"), LatticeValue::Const(ConstValue::Int(1)));
    }

    /// The names a callback script writes — `after`, `after idle`, a
    /// variable trace's callback, and a procedure named as a callback — are
    /// externally mutable in every function, the top level and a procedure
    /// alike, and a callback that writes a computed name makes every name so.
    #[test]
    fn the_names_a_callback_writes_are_never_constant() {
        let registry = CommandRegistry::build_default();
        for source in [
            "set go 1\nafter idle {set ::go 0}\nputs $go\n",
            "set go 1\nafter 100 { set go 0 }\nputs $go\n",
            "set go 1\ntrace add variable x write { set ::go 0 ;# }\nputs $go\n",
            "proc tick {} { set ::go 0 }\nset go 1\nafter 100 tick\nputs $go\n",
            "set go 1\nafter 100 { set $name 0 }\nputs $go\n",
        ] {
            let cu = crate::compilation_unit::CompilationUnit::build_for(source, &registry, false);
            let f = cu.function("::top").expect("top level analysed");
            assert_eq!(last_value(f, "go"), LatticeValue::Overdefined, "{source}");
        }
        let cu = crate::compilation_unit::CompilationUnit::build_for(
            "after 100 { set go 0 }\nproc p {} { global go; set go 1; puts $go }\n",
            &registry,
            false,
        );
        let f = cu.function("::p").expect("procedure analysed");
        assert_eq!(last_value(f, "go"), LatticeValue::Overdefined);
        let cu = crate::compilation_unit::CompilationUnit::build_for(
            "set unrelated 0\nafter 100 { set go 0 }\nputs $unrelated\n",
            &registry,
            false,
        );
        let f = cu.function("::top").expect("top level analysed");
        assert_eq!(
            last_value(f, "unrelated"),
            LatticeValue::Const(ConstValue::Int(0))
        );
    }

    /// The values the solver gives the last definitions of `names` in the
    /// function `name` of `source`.
    fn last_values(source: &str, name: &str, names: &[&str]) -> Vec<LatticeValue> {
        let registry = CommandRegistry::build_default();
        let cu = crate::compilation_unit::CompilationUnit::build_for(source, &registry, false);
        let f = cu.function(name).expect("function analysed");
        names
            .iter()
            .map(|variable| last_value(f, variable))
            .collect()
    }

    /// An expression, the function that holds it and the final value of each
    /// of two places it leaves.
    type NestedWriteCase = (&'static str, &'static str, [(&'static str, i64); 2]);

    /// A statement's substitutions run in the engine's left-to-right order
    /// under one ordered state, so the writes they make are the definitions
    /// the statement's embedded call makes and the host takes the result:
    /// over `x` = 1, `$x + [incr x] + $x` is 5 and leaves `x` at 2. A branch
    /// the engine never reaches writes nothing, and the place keeps what it
    /// held. The host is an assignment of an expression, an expression
    /// statement, or an assignment of one `expr` substitution, whose quoted
    /// operand is substituted first; at the top level and in a procedure.
    #[test]
    fn a_nested_write_is_the_definition_its_embedded_call_makes_and_the_host_takes_the_result() {
        let int = |n: i64| LatticeValue::Const(ConstValue::Int(n));
        let cases: [NestedWriteCase; 10] = [
            ("$x + [incr x] + $x", "::p", [("r", 5), ("x", 2)]),
            ("$y + [incr x] + $y", "::p", [("r", 12), ("x", 2)]),
            ("[set y] + [incr x]", "::p", [("r", 7), ("x", 2)]),
            ("0 && [incr x]", "::p", [("r", 0), ("x", 1)]),
            ("$x + [set x 10] + $x", "::p", [("r", 21), ("x", 10)]),
            ("[incr x] + [incr x]", "::p", [("r", 5), ("x", 3)]),
            ("$x ? [incr x] : [incr x 10]", "::p", [("r", 2), ("x", 2)]),
            (
                "!$x ? [incr x] : [incr x 10]",
                "::p",
                [("r", 11), ("x", 11)],
            ),
            (
                "[expr {$x + [incr x]}] + [incr x]",
                "::p",
                [("r", 6), ("x", 3)],
            ),
            ("$x + [incr x] + $x", "::top", [("r", 5), ("x", 2)]),
        ];
        for (expr, function, expected) in cases {
            let names = expected.map(|(name, _)| name);
            let want: Vec<LatticeValue> = expected.map(|(_, value)| int(value)).to_vec();
            let source = if function == "::top" {
                format!("set x 1\nset y 5\nset r [expr {{{expr}}}]\n")
            } else {
                format!("proc p {{}} {{set x 1; set y 5; set r [expr {{{expr}}}]; return $x}}\n")
            };
            assert_eq!(last_values(&source, function, &names), want, "{source}");
        }
        let statement = "proc p {} {set x 1; expr {$x + [incr x] + $x}; return $x}\n";
        assert_eq!(
            last_values(statement, "::p", &["x"]),
            [int(2)],
            "{statement}"
        );
        let quoted = "proc p {} {set x 1; set r [expr \"$x + [incr x]\"]; return $x}\n";
        assert_eq!(
            last_values(quoted, "::p", &["r", "x"]),
            [int(3), int(2)],
            "{quoted}"
        );
        let skipped =
            "proc p {} {set x 1; set r [expr {$x + [incr x] + (0 && [foo])}]; return $r}\n";
        assert_eq!(last_values(skipped, "::p", &["r"]), [int(3)], "{skipped}");
        let preserved = "proc p {} {set x 1; set r [expr {0 && [incr x]}]; return $x}\n";
        let registry = CommandRegistry::build_default();
        let cu = crate::compilation_unit::CompilationUnit::build_for(preserved, &registry, false);
        let f = cu.function("::p").expect("analysed");
        let x = f.ssa.var_symbol("x").expect("x");
        assert_eq!(
            f.sccp.preserved.get(&(x, 2)),
            Some(&1),
            "the call leaves `x` as it was where no write ran"
        );
    }

    /// The writes a word's own command makes are the command's, ahead of its
    /// own: `string length [append s bc]` in an expression, or as the value
    /// of the statement, leaves `s` at `abc`, and `incr x [incr x]` reads `x`
    /// after the inner `incr` ran, so over `x` = 1 it is 4.
    #[test]
    fn a_write_a_commands_word_makes_is_a_write_of_the_statement() {
        let text = |value: &str| LatticeValue::Const(ConstValue::String(value.to_owned()));
        let int = |n: i64| LatticeValue::Const(ConstValue::Int(n));
        let appended = "proc p {} {set s a; set r [expr {[string length [append s bc]] + [string length $s]}]; return $s}\n";
        assert_eq!(
            last_values(appended, "::p", &["r", "s"]),
            [int(6), text("abc")]
        );
        let first = "proc p {} {set s a; set r [expr {[string length $s] + [string length [append s bc]]}]; return $s}\n";
        assert_eq!(
            last_values(first, "::p", &["r", "s"]),
            [int(4), text("abc")]
        );
        let command = "proc p {} {set s a; set r [string length [append s bc]]; return $s}\n";
        assert_eq!(
            last_values(command, "::p", &["r", "s"]),
            [int(3), text("abc")]
        );
        let reads_after = "proc p {} {set x 1; set r [expr {[incr x [incr x]]}]; return $x}\n";
        assert_eq!(
            last_values(reads_after, "::p", &["r", "x"]),
            [int(4), int(4)]
        );
        let value = "proc p {} {set x 1; set r [incr x [incr x]]; return $x}\n";
        assert_eq!(last_values(value, "::p", &["r", "x"]), [int(4), int(4)]);
        let braced = "proc p {} {set x 1; set r [expr {$x} + [incr x]]; return $x}\n";
        assert_eq!(last_values(braced, "::p", &["r", "x"]), [int(4), int(2)]);
        let created = "proc p {} {unset -nocomplain u; set r [expr {[string length [append u [set u 3]]]}]; return $u}\n";
        assert_eq!(last_values(created, "::p", &["r", "u"]), [int(2), int(33)]);
        let inside = "proc p {} {set s a; set r [expr {[string length [append s [append s b]]]}]; return $s}\n";
        assert_eq!(
            last_values(inside, "::p", &["r", "s"]),
            [int(4), text("abab")]
        );
    }

    /// A statement the engine evaluates for each of several possible inputs
    /// leaves each place the join of what every member left: `$c ? [incr x] :
    /// 7` writes `x` for the input that takes the branch and leaves it alone
    /// for the other, so `x` is 1 or 2 and the result 7 or 2.
    #[test]
    fn a_write_only_some_inputs_make_is_joined_over_the_members() {
        let source = "proc p {n} {set x 1; if {$n} {set c 0} else {set c 1}; \
                      set r [expr {$c ? [incr x] : 7}]; return $x}\n";
        let ints = |value: &LatticeValue| -> Vec<i64> {
            let LatticeValue::ConstSet(members) = value else {
                panic!("a set of constants: {value:?}");
            };
            let mut ints: Vec<i64> = members
                .iter()
                .map(|member| match member {
                    ConstValue::Int(n) => *n,
                    other => panic!("an integer: {other:?}"),
                })
                .collect();
            ints.sort_unstable();
            ints
        };
        let values = last_values(source, "::p", &["r", "x"]);
        assert_eq!(
            (ints(&values[0]), ints(&values[1])),
            (vec![2, 7], vec![1, 2])
        );
    }

    /// An error in the middle of an expression ends the statement with the
    /// writes the expression's commands had made. Inside the body of a
    /// `catch` the handler is thrown to with them — `x` is 2 after `catch {set
    /// r [expr {[incr x] + [error mid]}]}`, and `r` is never set — and where
    /// no handler is thrown to the statement is not evaluated at all.
    #[test]
    fn an_error_in_the_middle_leaves_the_writes_made_before_it_inside_a_catch() {
        let int = |n: i64| LatticeValue::Const(ConstValue::Int(n));
        let in_proc =
            "proc p {} {set x 1; catch {set r [expr {[incr x] + [error mid]}]}; return $x}\n";
        assert_eq!(last_values(in_proc, "::p", &["x"]), [int(2)]);
        let statement = "proc p {} {set x 1; catch {expr {[incr x] + [error mid]}}; return $x}\n";
        assert_eq!(last_values(statement, "::p", &["x"]), [int(2)]);
    }

    /// What the statement's evaluation cannot own, or is not an evaluation of,
    /// keeps the conservative answer: a write to a global, a command the
    /// module defines, an error in the middle outside any handler, a host that
    /// is a call or a condition, whose substitutions run under the
    /// effect-free policy, and a value that is two substitutions rather than
    /// one command. A loop carries the write round, so neither the first
    /// iteration's value nor the last's is a constant.
    #[test]
    fn what_the_state_cannot_own_keeps_the_conservative_answer() {
        let widened = |source: &str, names: &[&str]| {
            let values = last_values(source, "::p", names);
            assert!(
                values
                    .iter()
                    .all(|value| *value == LatticeValue::Overdefined),
                "{source}: {values:?}"
            );
        };
        widened(
            "proc p {} {set x 1; set r [expr {$x + [incr ::g]}]; return $x}\n",
            &["r"],
        );
        widened(
            "proc bump {} {upvar 1 x x; incr x}\nproc p {} {set x 1; set r [expr {[bump] + $x}]; return $x}\n",
            &["r", "x"],
        );
        widened(
            "proc p {} {set x 1; set r [expr {[incr x] + [error mid]}]; return $x}\n",
            &["x"],
        );
        widened(
            "proc p {} {set x 1; puts [expr {$x + [incr x]}]; return $x}\n",
            &["x"],
        );
        widened(
            "proc p {} {set x 1; if {[incr x] > 1} {puts a}; return $x}\n",
            &["x"],
        );
        widened(
            "proc p {} {set r [expr {[incr x] + 1}]; return $x}\n",
            &["x"],
        );
        // A place no statement reads or defines ahead of the call has no value
        // to keep where the expression leaves it alone.
        widened(
            "proc p {} {set r [expr {0 && [set y 3]}]; return 1}\n",
            &["y"],
        );
        // A module that defines `expr` runs its own.
        for source in [
            "proc expr {args} {return 9}\nproc p {} {set x 1; set r [expr \"$x + [incr x]\"]; return $x}\n",
            "proc expr {args} {return 9}\nproc p {} {set x 1; set r [expr {$x + [incr x]}]; return $x}\n",
            "proc expr {args} {return 9}\nproc p {} {set x 1; expr {$x + [incr x]}; return $x}\n",
        ] {
            widened(source, &["x"]);
        }
        // A value that is two substitutions is not one command.
        widened(
            "proc p {} {set x 1; set r [expr {1}][incr x]; return $x}\n",
            &["x"],
        );
        widened(
            "proc p {} {set x 1; set r [incr x][expr {1}]; return $x}\n",
            &["x"],
        );
        widened(
            "proc p {} {set x 0; for {set i 0} {$i < 3} {incr i} {set r [expr {[incr x] * 2}]}; return $x}\n",
            &["r", "x"],
        );
    }

    /// A call to a command the module cannot see leaves each name live past it
    /// a fresh version with no constant value — a plain top-level name, which
    /// is the global name, and a procedure's local, which the callee reaches
    /// through `upvar 1` — while the version the call holds keeps its value for
    /// a read before the call, and a definition after the call is decided
    /// again. A name the call follows only in a procedure the module defines
    /// keeps its constant.
    #[test]
    fn an_unseen_call_widens_the_versions_it_leaves_and_no_earlier_or_later_one() {
        let registry = CommandRegistry::build_default();
        let cu = crate::compilation_unit::CompilationUnit::build_for(
            "set x 1\nputs $x\nfoo\nputs $x\nset y 2\nset x 3\nputs $x$y\n",
            &registry,
            false,
        );
        let top = cu.function("::top").expect("top level analysed");
        let symbol = top.ssa.var_symbol("x").expect("x");
        assert!(top.ssa.is_observed_by_unseen_call(symbol, 1));
        assert!(!top.ssa.is_observed_by_unseen_call(symbol, 2));
        assert_eq!(top.ssa.binding_version(symbol, 2), 1);
        assert_eq!(
            top.sccp.values.get(&(symbol, 1)),
            Some(&LatticeValue::Const(ConstValue::Int(1)))
        );
        assert_eq!(
            top.sccp.values.get(&(symbol, 2)),
            Some(&LatticeValue::Overdefined)
        );
        assert_eq!(
            last_value(top, "y"),
            LatticeValue::Const(ConstValue::Int(2))
        );
        assert_eq!(
            last_value(top, "x"),
            LatticeValue::Const(ConstValue::Int(3))
        );
        let cu = crate::compilation_unit::CompilationUnit::build_for(
            "proc p {} { set x 1; foo; puts $x }\n",
            &registry,
            false,
        );
        let p = cu.function("::p").expect("procedure analysed");
        assert_eq!(last_value(p, "x"), LatticeValue::Overdefined);
        let cu = crate::compilation_unit::CompilationUnit::build_for(
            "proc foo {} { puts hi }\nset x 1\nfoo\nputs $x\n",
            &registry,
            false,
        );
        let top = cu.function("::top").expect("top level analysed");
        assert_eq!(
            last_value(top, "x"),
            LatticeValue::Const(ConstValue::Int(1))
        );
        // The `::` spelling is externally mutable whatever the call.
        let cu = crate::compilation_unit::CompilationUnit::build_for(
            "set ::x 1\nputs $::x\n",
            &registry,
            false,
        );
        let f = cu.function("::top").expect("top level analysed");
        assert_eq!(last_value(f, "::x"), LatticeValue::Overdefined);
    }

    /// A contradiction drops the state of the loop whose state the settled
    /// run contradicts and keeps every other loop's: the run is made again
    /// with the other loop's refinements alone, and the result records that
    /// loop alone. Two loops leave to blocks 3 and 7; the second run
    /// contradicts the first's state.
    #[test]
    fn a_contradiction_drops_only_its_own_loop() {
        let record = |end: u32| EnumeratedLoop {
            exit_block: BlockId(end),
            span: Span::new(0, 0),
            iterations: 1,
            exit: tcl_registry::value_transfer::ExitRule::Exhaustion,
            published: Vec::new(),
        };
        let refinement = |end: u32| EdgeRefinement {
            edge: (BlockId(end - 1), BlockId(end)),
            key: (Symbol(0), 1),
            domain: FactDomain::ExactValue,
            fact: FactView::Exact(ExactValue::int(1), None),
            evidence: DependencyEvidence::default(),
            loop_exit: true,
        };
        let mut rounds = Vec::new();
        let result = drive(|round| {
            let published = match round {
                Round::First => None,
                Round::Published(exits) => Some(
                    exits
                        .iter()
                        .map(|refinement| refinement.edge.1)
                        .collect::<Vec<_>>(),
                ),
            };
            rounds.push(published.clone());
            match published {
                None => Solved::Enumerated(
                    vec![record(3), record(7)],
                    vec![refinement(3), refinement(7)],
                ),
                Some(ends) if ends.contains(&BlockId(3)) => {
                    Solved::Contradicted(HashSet::from([BlockId(3)]))
                }
                Some(_) => Solved::Settled(Box::default()),
            }
        });
        assert_eq!(
            rounds,
            [
                None,
                Some(vec![BlockId(3), BlockId(7)]),
                Some(vec![BlockId(7)])
            ]
        );
        assert_eq!(
            result
                .loop_enumerations
                .iter()
                .map(|record| record.exit_block)
                .collect::<Vec<_>>(),
            [BlockId(7)]
        );
    }
}
