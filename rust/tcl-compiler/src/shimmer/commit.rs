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

//! Committed-intrep forward dataflow (the "first-use commit" pass).
//!
//! C Tcl values are pure strings (`typePtr == NULL`) until an operation reads
//! them as a typed value; that first read **commits** an intrep in place, and
//! only a *later* read as a different type genuinely re-represents (shimmers).
//! The type lattice cannot carry this — it is keyed per SSA version, and
//! commitment happens at *uses* (a value is pure before `lindex $x 0` and a
//! list after it, with no new version in between) — so this pass tracks it
//! flow-sensitively per program point.
//!
//! Per `(symbol, version)` the state is a **must/may** pair (see
//! [`CommitState`]): the bounded set of intreps the value *may* have committed
//! on some path, and whether *every* path has committed one.  A mismatching use
//! is a genuine, every-execution shimmer precisely when the value is committed
//! on all paths and the required intrep is not among the possible ones — the
//! "multiple different dominator types" analysis: at a merge of two branches
//! that committed `Int` and `List`, a later use as `Dict` pays on both paths
//! (fires), a use as `List` pays only on one (stays silent).
//!
//! Outputs:
//! - [`CommitFacts`] — per-block entry states, consumed by the use-site / expr
//!   shimmer detectors via a per-block [`CommitWalker`] that replays the same
//!   transfer function statement by statement.
//! - [`CommitFacts::single_commitments`] — the def-site pushback map: versions
//!   whose *every* executable typed read committed the same intrep, for
//!   surfacing "first used as: list" at the creation site (hover).

use std::collections::{HashMap, HashSet};

use tcl_lexer::Span;
use tcl_registry::{CommandRegistry, TclType};

use crate::analyses::LatticeValue;
use crate::cfg::{BlockId, Function as CfgFunction, Terminator};
use crate::depth_guard::MAX_EXPR_NODE_DEPTH;
use crate::expr_ast::{BinOp, ExprNode};
use crate::ir::Statement;
use crate::naming::normalise_var_name;
use crate::sccp::cfg_order;
use crate::ssa::{SsaFunction, Symbol, ValueKey};
use crate::types::{TypeKind, TypeLattice};
use crate::value_shapes::is_pure_var_ref;

use super::hints::is_pure_value;
use super::hints::{inert_effective_args, is_numeric_compatible};
use crate::value_transfer::FoldedType;
use tcl_registry::value_transfer::RepresentationEvidence;

/// Upper bound on the tracked may-set — a value committed to more than this
/// many distinct intreps across paths widens to "unknown" (never fires).
const MAX_MAY_TYPES: usize = 3;

/// The commitment state of one SSA value at one program point.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CommitState {
    /// Intreps the value may have committed on **some** path here (bounded by
    /// [`MAX_MAY_TYPES`]; empty ⇒ still pure on every path).
    may: Vec<TclType>,
    /// True when **every** executable path here has committed some intrep.
    all_committed: bool,
    /// True when the may-set overflowed — the state is unknown and must never
    /// drive a warning.
    overflowed: bool,
    /// Earliest site that committed an intrep, for "first converted here"
    /// related info.
    first_span: Option<Span>,
}

impl CommitState {
    /// Reconcile replayed commitment with the actual captured physical read.
    /// A current native representation supersedes a version's earlier use;
    /// equal bytes and an unchanged SSA version do not preserve its intrep.
    /// Closed container possibilities remain cost-only May facts. Missing
    /// physical evidence cannot recover a stale container commitment.
    fn at_captured_representation(
        mut self,
        representation: Option<tcl_syntax::value::ValueRepresentation>,
        alternatives: Option<crate::native_numeric::ClosedContainerRepresentations>,
        already_numeric: bool,
    ) -> Self {
        use tcl_syntax::value::ValueRepresentation;
        let current = match representation {
            Some(ValueRepresentation::List) => Some(TclType::List),
            Some(ValueRepresentation::Dict) => Some(TclType::Dict),
            Some(ValueRepresentation::String) => return Self::pure(),
            None | Some(ValueRepresentation::Unknown) => None,
        };
        if let Some(current) = current {
            let first_span = (self.single_committed() == Some(current))
                .then_some(self.first_span)
                .flatten();
            return Self::committed(current, first_span);
        }
        if let Some(alternatives) = alternatives {
            return Self {
                may: [
                    (ValueRepresentation::List, TclType::List),
                    (ValueRepresentation::Dict, TclType::Dict),
                ]
                .into_iter()
                .filter_map(|(representation, ty)| {
                    alternatives.contains(representation).then_some(ty)
                })
                .collect(),
                all_committed: false,
                overflowed: false,
                first_span: None,
            };
        }
        if self
            .may
            .iter()
            .any(|ty| matches!(ty, TclType::List | TclType::Dict))
        {
            self.all_committed = false;
            self.may
                .retain(|ty| !matches!(ty, TclType::List | TclType::Dict));
            self.first_span = None;
        }
        if !already_numeric
            && self.may.iter().any(|ty| {
                matches!(
                    ty,
                    TclType::Int | TclType::Double | TclType::Numeric | TclType::Boolean
                )
            })
        {
            self.all_committed = false;
            self.may.retain(|ty| {
                !matches!(
                    ty,
                    TclType::Int | TclType::Double | TclType::Numeric | TclType::Boolean
                )
            });
            self.first_span = None;
        }
        self
    }

    /// The pure (uncommitted-on-every-path) state.
    #[must_use]
    pub fn pure() -> Self {
        Self::default()
    }

    /// A state committed to `t` on every path.
    #[must_use]
    pub fn committed(t: TclType, span: Option<Span>) -> Self {
        Self {
            may: vec![t],
            all_committed: true,
            overflowed: false,
            first_span: span,
        }
    }

    /// Record a read of this value at a position requiring intrep `t`: the
    /// runtime converts (or the command errors and execution stops), so the
    /// post-state is committed to exactly `t` on the paths through here.
    ///
    /// **Unless the value already satisfies the read.** A numeric-family
    /// intrep is read in place by the whole `Tcl_Get*FromObj` family, so a
    /// compatible read installs nothing and the cached representation
    /// survives it — verified on tclsh 8.6.16: after
    /// `set d [expr {1.0 + 1.5}]`, `expr {$d && 1}` leaves `d` holding the
    /// very same `tclDoubleType` intrep, and a later `incr d` therefore still
    /// raises `expected integer but got "2.5"`. Overwriting the state with
    /// the *expectation* rather than keeping the *representation* lost that:
    /// the pair was recorded as Boolean, which `must_pay(Int)` then found
    /// integer-compatible, and the genuine `Double` → `Int` mismatch went
    /// unreported.
    pub fn commit(&mut self, t: TclType, span: Span) {
        if self.first_span.is_none() {
            self.first_span = Some(span);
        }
        if self.satisfies(t) {
            return;
        }
        self.may.clear();
        self.may.push(t);
        self.all_committed = true;
        self.overflowed = false;
    }

    /// Whether every intrep this value may hold already answers a read
    /// requiring `t` without converting — the condition under which
    /// [`Self::commit`] leaves the state alone. The inverse of
    /// [`Self::must_pay`] over the *whole* may-set rather than its negation:
    /// `must_pay` asks "does every path pay?", this asks "does no path pay?",
    /// and a mixed state answers neither.
    fn satisfies(&self, t: TclType) -> bool {
        self.all_committed
            && !self.overflowed
            && !self.may.is_empty()
            && self
                .may
                .iter()
                .all(|&c| c == t || is_numeric_compatible(c, t))
    }

    /// Whether **every** executable path to this point pays a conversion for a
    /// read requiring `expected` — committed on all paths, and no possible
    /// committed intrep satisfies the requirement (numeric-family intreps are
    /// interchangeable, mirroring the detectors' compatibility rule).
    #[must_use]
    pub fn must_pay(&self, expected: TclType) -> bool {
        self.all_committed
            && !self.overflowed
            && !self.may.is_empty()
            && !self
                .may
                .iter()
                .any(|&t| t == expected || is_numeric_compatible(t, expected))
    }

    /// The single committed intrep when exactly one is possible — the precise
    /// `from_type` for a fired warning.
    #[must_use]
    pub fn single_committed(&self) -> Option<TclType> {
        (self.all_committed && !self.overflowed && self.may.len() == 1).then(|| self.may[0])
    }

    /// The single intrep the value *may* have committed, even when some path
    /// is still pure — the steady-state rep of a loop-carried value whose
    /// entry re-joins the pure preheader each iteration (`llength $l` /
    /// `dict size $l` alternating: at the `llength`, `may = {Dict}` from the
    /// back edge though the first iteration arrives pure).
    #[must_use]
    pub fn single_may(&self) -> Option<TclType> {
        (!self.overflowed && self.may.len() == 1).then(|| self.may[0])
    }

    /// The committed intreps possibly held here, for the path-dependent
    /// wording of a merge-of-different-commitments warning.
    #[must_use]
    pub fn may_types(&self) -> &[TclType] {
        if self.overflowed { &[] } else { &self.may }
    }

    /// The site that first committed an intrep, when known.
    #[must_use]
    pub fn first_span(&self) -> Option<Span> {
        self.first_span
    }

    /// Join with another predecessor's exit state (control-flow merge): the
    /// may-sets union (bounded), and the value is all-paths-committed only when
    /// both sides are.
    fn join(&mut self, other: &Self) {
        self.all_committed &= other.all_committed;
        self.overflowed |= other.overflowed;
        for &t in &other.may {
            if !self.may.contains(&t) {
                if self.may.len() >= MAX_MAY_TYPES {
                    self.overflowed = true;
                    break;
                }
                self.may.push(t);
            }
        }
        self.first_span = match (self.first_span, other.first_span) {
            (Some(a), Some(b)) => Some(if b.start() < a.start() { b } else { a }),
            (a, b) => a.or(b),
        };
    }
}

/// One typed read extracted from a statement or terminator: variable `sym` is
/// read at a position that installs intrep `expected`.
#[derive(Debug, Clone, Copy)]
struct TypedRead {
    sym: Symbol,
    ver: u32,
    expected: TclType,
    span: Span,
}

/// The fixpoint result: per-block entry commitment states.
pub struct CommitFacts {
    block_entry: HashMap<BlockId, HashMap<ValueKey, CommitState>>,
    /// Per-version set of intreps committed by any executable typed read
    /// (bounded like the may-set), for the def-site pushback.
    use_commit_types: HashMap<ValueKey, Vec<TclType>>,
}

impl CommitFacts {
    /// A per-block walker seeded with the block's entry state, replaying the
    /// transfer function statement by statement in step with a detector walk.
    #[must_use]
    pub fn walker<'a>(&self, ctx: &CommitCtx<'a>, block_id: BlockId) -> CommitWalker<'a> {
        CommitWalker {
            state: self.block_entry.get(&block_id).cloned().unwrap_or_default(),
            registry: ctx.registry,
            context: ctx.context,
            ssa: ctx.ssa,
            types: ctx.types,
            values: ctx.values,
            block: block_id,
            index: 0,
            folded: ctx.folded,
        }
    }

    /// The def-site pushback map: versions that start **pure** and whose every
    /// executable typed read committed the **same** intrep resolve to that
    /// intrep — the "first used as" fact a hover can surface at the creation
    /// site. Versions with conflicting or no typed reads are absent.
    #[must_use]
    pub fn single_commitments(&self, ctx: &CommitCtx<'_>) -> HashMap<ValueKey, TclType> {
        self.use_commit_types
            .iter()
            .filter(|(key, committed)| {
                committed.len() == 1
                    && initial_state(ctx, **key).is_some_and(|s| s == CommitState::pure())
            })
            .map(|(key, committed)| (*key, committed[0]))
            .collect()
    }
}

/// Read-only inputs threaded through the fixpoint and the per-block walkers.
#[derive(Clone, Copy)]
pub struct CommitCtx<'a> {
    /// Command registry, for `arg_types` shimmer hints and foreach headers.
    pub registry: &'a CommandRegistry,
    /// Retained metadata and lexical policy for every replayed read.
    pub context: super::ShimmerContext<'a>,
    /// SSA form, for variable symbols and per-statement use versions.
    pub ssa: &'a SsaFunction,
    /// Source-name projection at the actual read; value keys remain cell identities.
    pub source: crate::ssa::SsaSourceView<'a>,
    /// Per-version inferred types, for each version's initial purity.
    pub types: &'a HashMap<ValueKey, TypeLattice>,
    /// SCCP constants, for the numeric-literal purity distinction.
    pub values: &'a HashMap<ValueKey, LatticeValue>,
    /// The folded types SCCP's evaluations state
    /// ([`crate::sccp::SccpResult::folded_types`]): a computed value's
    /// representation decides its purity before its constant does.
    pub folded: &'a HashMap<ValueKey, FoldedType>,
}

impl CommitCtx<'_> {
    /// The numeral grammar of the release being analysed, from the
    /// retained compilation profile.
    ///
    /// Whether a constant is a valid instance of a numeric type is
    /// release-dependent — `08` and `1_0` are numbers from 9.0 and not before —
    /// so a shimmer hint keyed on it has to ask the document's own release
    /// rather than whatever grammar this process was built for.
    #[must_use]
    pub fn numbers(&self) -> tcl_syntax::number::NumberSyntax {
        self.context.numbers()
    }

    /// The document's word-value rules, from the retained lexical configuration — whether a constant is a valid list/dict is a
    /// list-grammar question, and Jim's parser answers it differently.
    #[must_use]
    pub fn word_rules(&self) -> tcl_syntax::word_rules::WordValueRules {
        self.context.word_rules()
    }
}

/// A per-block replay of the commitment transfer function, kept in step with a
/// detector's statement walk: query [`Self::state_of`] *before* checking a
/// statement, then [`Self::step`] past it.
pub struct CommitWalker<'a> {
    state: HashMap<ValueKey, CommitState>,
    registry: &'a CommandRegistry,
    context: super::ShimmerContext<'a>,
    ssa: &'a SsaFunction,
    types: &'a HashMap<ValueKey, TypeLattice>,
    values: &'a HashMap<ValueKey, LatticeValue>,
    block: BlockId,
    index: usize,
    folded: &'a HashMap<ValueKey, FoldedType>,
}

/// Conversion-cost advice reconciled with the exact captured read. This is
/// neither a constant-value nor an operation-erasure proof.
pub(super) struct RepresentationCost {
    pub current: TclType,
    pub commitment: CommitState,
}

impl CommitWalker<'_> {
    /// Convert actual physical read evidence and semantic contents into one
    /// cost projection. All detectors share this reconciliation; replayed
    /// commitment alone cannot override a callback's current representation.
    fn cost_at_read(
        &self,
        read: crate::ssa::SsaReadReference,
        semantic: TclType,
        expected: TclType,
        advice: crate::ssa::SsaReadRepresentationAdvice,
    ) -> Option<RepresentationCost> {
        let semantic = if advice.already_numeric {
            advice.numeric_category.unwrap_or(TclType::Numeric)
        } else {
            semantic
        };
        let current = super::hints::representation_cost_type(
            semantic,
            advice.representation,
            advice.container_alternatives,
            expected,
        )?;
        let mut commitment = self
            .state_of(read.symbol, read.version?)
            .at_captured_representation(
                advice.representation,
                advice.container_alternatives,
                advice.already_numeric,
            );
        if advice.already_numeric {
            // Reused object coercions can change its subtype without changing
            // its bytes or represented contents version. Current production
            // therefore supersedes replay even when replay has a single type.
            let first_span = (commitment.single_committed() == Some(semantic))
                .then_some(commitment.first_span())
                .flatten();
            commitment = CommitState::committed(semantic, first_span);
        }
        Some(RepresentationCost {
            current,
            commitment,
        })
    }

    pub(super) fn cost_for_word(
        &self,
        source: crate::ssa::SsaSourceView<'_>,
        word: &crate::ir::WordExpr,
        read: crate::ssa::SsaReadReference,
        semantic: TclType,
        expected: TclType,
    ) -> Option<RepresentationCost> {
        self.cost_at_read(
            read,
            semantic,
            expected,
            source.read_word_representation_advice(word, self.registry),
        )
    }

    pub(super) fn cost_for_expression(
        &self,
        source: crate::ssa::SsaSourceView<'_>,
        node: &ExprNode,
        origin: ExpressionCostOrigin<'_>,
        read: crate::ssa::SsaReadReference,
        semantic: TclType,
        expected: TclType,
    ) -> Option<RepresentationCost> {
        let advice = source.read_expression_representation_advice(
            node,
            origin.base,
            origin.executed,
            self.registry,
        );
        self.cost_at_read(read, semantic, expected, advice)
    }

    pub(super) fn cost_for_native_read(
        &self,
        source: crate::ssa::SsaSourceView<'_>,
        read: crate::ssa::SsaReadReference,
        semantic: TclType,
        expected: TclType,
    ) -> Option<RepresentationCost> {
        self.cost_at_read(
            read,
            semantic,
            expected,
            source.native_read_representation_advice(read.symbol, self.registry),
        )
    }

    /// The numeral grammar of the release being analysed — see
    /// [`CommitCtx::numbers`].
    #[must_use]
    pub fn numbers(&self) -> tcl_syntax::number::NumberSyntax {
        self.context.numbers()
    }

    /// The document's word-value rules, from the retained lexical configuration — whether a constant is a valid list/dict is a
    /// list-grammar question, and Jim's parser answers it differently.
    #[must_use]
    pub fn word_rules(&self) -> tcl_syntax::word_rules::WordValueRules {
        self.context.word_rules()
    }

    pub(super) fn config(&self) -> tcl_lexer::LexerConfig {
        self.context.config()
    }

    /// Raw replay state, for transfer and commitment reporting. Detectors must
    /// use the captured-read cost projections to reconcile physical effects.
    /// The commitment state of `(sym, ver)` at the current point — versions
    /// not yet touched start at their def's initial state ([`initial_state`]).
    #[must_use]
    pub fn state_of(&self, sym: Symbol, ver: u32) -> CommitState {
        let key = (sym, ver);
        if let Some(s) = self.state.get(&key) {
            return s.clone();
        }
        let ctx = CommitCtx {
            registry: self.registry,
            context: self.context,
            ssa: self.ssa,
            source: crate::ssa::SsaSourceView::unpositioned(self.ssa),
            types: self.types,
            values: self.values,
            folded: self.folded,
        };
        initial_state(&ctx, key).unwrap_or_default()
    }

    /// Advance the state past one statement.
    pub fn step(&mut self, stmt: &Statement, uses: &HashMap<Symbol, u32>) {
        let ctx = CommitCtx {
            registry: self.registry,
            context: self.context,
            ssa: self.ssa,
            source: crate::ssa::SsaSourceView::at_statement(self.ssa, self.block, self.index),
            types: self.types,
            values: self.values,
            folded: self.folded,
        };
        self.index += 1;
        for read in typed_reads_of_statement(&ctx, stmt, uses) {
            apply_read(&ctx, &mut self.state, read, None);
        }
    }
}

/// Original expression extent or actually evaluated operand origins.
#[derive(Clone, Copy)]
pub(super) struct ExpressionCostOrigin<'a> {
    pub base: Option<u32>,
    pub executed: Option<&'a crate::command_binding::ExecutedExpressionSource>,
}

/// The state a version starts in at its def: pure when the producer left the
/// value uncommitted ([`is_pure_value`]: the representation its evaluation
/// states, else the type lattice + SCCP constant — a literal, an
/// interpolation, or a string-command result), committed to the producer's
/// intrep otherwise (`[list …]`, `[dict create …]`, `expr`, `binary format`,
/// a computed `[string length $s]`, …) — the intrep the route constructed
/// when it says, else the type lattice's.  `None` when the version has no
/// known type (stays pure-with-unknown: never drives a warning because
/// `must_pay` needs a non-empty may-set).
fn initial_state(ctx: &CommitCtx<'_>, key: ValueKey) -> Option<CommitState> {
    let lattice = ctx.types.get(&key)?;
    if lattice.kind() != TypeKind::Known {
        return Some(CommitState::pure());
    }
    let t = lattice.tcl_type()?;
    let folded = ctx.folded.get(&key);
    let representation = folded.map_or(RepresentationEvidence::Unknown, |f| f.representation);
    Some(if is_pure_value(t, ctx.values.get(&key), representation) {
        CommitState::pure()
    } else {
        let built = folded.and_then(FoldedType::constructed_intrep);
        CommitState::committed(built.unwrap_or(t), None)
    })
}

/// Apply one typed read to the state map, recording the commitment (and, when
/// `sink` is provided, accumulating the per-version committed-type set for the
/// def-site pushback).
fn apply_read(
    ctx: &CommitCtx<'_>,
    state: &mut HashMap<ValueKey, CommitState>,
    read: TypedRead,
    sink: Option<&mut HashMap<ValueKey, Vec<TclType>>>,
) {
    let key = (read.sym, read.ver);
    let entry = state
        .entry(key)
        .or_insert_with(|| initial_state(ctx, key).unwrap_or_default());
    entry.commit(read.expected, read.span);
    if let Some(sink) = sink {
        let committed = sink.entry(key).or_default();
        if !committed.contains(&read.expected) && committed.len() <= MAX_MAY_TYPES {
            committed.push(read.expected);
        }
    }
}

/// Compute the per-block entry commitment states for one function by forward
/// fixpoint over the SCCP-executable blocks (must/may join at merges).
#[must_use]
pub fn compute_commit_facts<S: std::hash::BuildHasher, E: std::hash::BuildHasher>(
    cfg: &CfgFunction,
    ctx: &CommitCtx<'_>,
    executable_blocks: &HashSet<BlockId, S>,
    executable_edges: &HashSet<(BlockId, BlockId), E>,
) -> CommitFacts {
    let order = cfg_order(cfg);
    let preds = cfg.predecessors();
    let mut block_entry: HashMap<BlockId, HashMap<ValueKey, CommitState>> = HashMap::new();
    let mut block_exit: HashMap<BlockId, HashMap<ValueKey, CommitState>> = HashMap::new();
    let mut use_commit_types: HashMap<ValueKey, Vec<TclType>> = HashMap::new();

    let mut changed = true;
    while changed {
        changed = false;
        for &block_id in &order {
            if !executable_blocks.contains(&block_id) {
                continue;
            }
            let entry = join_entry(ctx, cfg, block_id, &preds, executable_edges, &block_exit);
            let exit = transfer_block(ctx, cfg, block_id, entry.clone(), &mut use_commit_types);
            if block_entry.get(&block_id) != Some(&entry) {
                block_entry.insert(block_id, entry);
                changed = true;
            }
            if block_exit.get(&block_id) != Some(&exit) {
                block_exit.insert(block_id, exit);
                changed = true;
            }
        }
    }

    CommitFacts {
        block_entry,
        use_commit_types,
    }
}

/// Join the exit states of a block's executable predecessors into its entry
/// state.  A version absent from a predecessor's exit map is at that
/// predecessor's *initial* state along that path, so the join seeds from the
/// initial state rather than skipping the key (skipping would claim
/// all-paths-committed off a single committing path).
fn join_entry<E: std::hash::BuildHasher>(
    ctx: &CommitCtx<'_>,
    cfg: &CfgFunction,
    block_id: BlockId,
    preds: &HashMap<BlockId, HashSet<BlockId>>,
    executable_edges: &HashSet<(BlockId, BlockId), E>,
    block_exit: &HashMap<BlockId, HashMap<ValueKey, CommitState>>,
) -> HashMap<ValueKey, CommitState> {
    let mut exec_preds: Vec<BlockId> = preds
        .get(&block_id)
        .map(|ps| {
            ps.iter()
                .filter(|p| executable_edges.contains(&(**p, block_id)))
                .copied()
                .collect()
        })
        .unwrap_or_default();
    exec_preds.sort_unstable();

    let mut entry: HashMap<ValueKey, CommitState> = HashMap::new();
    let keys: HashSet<ValueKey> = exec_preds
        .iter()
        .filter_map(|p| block_exit.get(p))
        .flat_map(|m| m.keys().copied())
        .collect();
    for key in keys {
        let mut acc: Option<CommitState> = None;
        for p in &exec_preds {
            let s = if cfg.exception_edges.contains(&(*p, block_id)) {
                // Normal operand conversion promises do not describe an error
                // partway through a command's reads. An exceptional edge can
                // retain any prior/partially converted representation.
                CommitState {
                    overflowed: true,
                    ..CommitState::default()
                }
            } else {
                block_exit
                    .get(p)
                    .and_then(|m| m.get(&key))
                    .cloned()
                    .unwrap_or_else(|| initial_state(ctx, key).unwrap_or_default())
            };
            match &mut acc {
                None => acc = Some(s),
                Some(a) => a.join(&s),
            }
        }
        if let Some(a) = acc {
            entry.insert(key, a);
        }
    }
    entry
}

/// Run the transfer function over one block's statements and terminator.
fn transfer_block(
    ctx: &CommitCtx<'_>,
    cfg: &CfgFunction,
    block_id: BlockId,
    mut state: HashMap<ValueKey, CommitState>,
    use_commit_types: &mut HashMap<ValueKey, Vec<TclType>>,
) -> HashMap<ValueKey, CommitState> {
    if let Some(ssa_block) = ctx.ssa.blocks.get(&block_id) {
        for (index, ss) in ssa_block.statements.iter().enumerate() {
            let positioned = CommitCtx {
                source: crate::ssa::SsaSourceView::at_statement(ctx.ssa, block_id, index),
                ..*ctx
            };
            for read in typed_reads_of_statement(&positioned, &ss.statement, &ss.uses) {
                apply_read(ctx, &mut state, read, Some(use_commit_types));
            }
        }
        let positioned = CommitCtx {
            source: crate::ssa::SsaSourceView::at_terminator(ctx.ssa, block_id),
            ..*ctx
        };
        // Branch-condition and return-expression reads live on the
        // terminator, evaluated after the block's statements with its exit
        // versions. Both must move the state for the same reason: the runtime
        // converts there, so a later read in a successor block sees the
        // converted representation.
        match cfg
            .blocks
            .get(&block_id)
            .and_then(|b| b.terminator.as_ref())
        {
            Some(Terminator::Branch {
                condition,
                span,
                condition_base,
                ..
            }) => {
                let branch_span = span.unwrap_or_else(|| Span::new(0, 0));
                for read in
                    typed_reads_of_expr(&positioned, condition, *condition_base, branch_span)
                {
                    apply_read(ctx, &mut state, read, Some(use_commit_types));
                }
            }
            Some(Terminator::Return {
                expr: Some(expr),
                span,
                ..
            }) => {
                let return_span = span.unwrap_or_else(|| Span::new(0, 0));
                for read in typed_reads_of_expr(&positioned, expr, None, return_span) {
                    apply_read(ctx, &mut state, read, Some(use_commit_types));
                }
            }
            _ => {}
        }
    }
    state
}

/// Extract every `$var` read of `stmt` at a position that installs a specific
/// intrep — the transfer function's alphabet.  Mirrors the read extraction of
/// the use-site and expr detectors (registry `arg_types` / foreach headers /
/// `incr` / expr operand contexts), so the state the detectors consult moves
/// exactly when the runtime converts.
fn typed_reads_of_statement(
    ctx: &CommitCtx<'_>,
    stmt: &Statement,
    uses: &HashMap<Symbol, u32>,
) -> Vec<TypedRead> {
    let mut out = Vec::new();
    match stmt {
        Statement::Call { tokens, .. } => {
            // Tcl evaluates the words — running any `[cmd …]` in them — before
            // it invokes the outer command, so those reads land first. Without
            // them the state is stale for every later read of the same
            // variable: `puts [lindex $x 0]` converts `x` to a list just as
            // surely as a bare `lindex $x 0` does.
            push_lifted_reads(ctx, &mut out, tokens.as_ref(), stmt.span());
            let Some(invocation) = ctx.context.statement(stmt) else {
                return out;
            };
            if let Some(tokens) = tokens
                && let Some(expression) = ctx.context.expression(tokens, stmt.span())
            {
                out.extend(typed_reads_of_executed_expr(ctx, &expression, stmt.span()));
                return out;
            }
            let argument_count = invocation.argument_count();

            let inert = inert_effective_args(ctx.registry, &invocation);
            for i in 0..argument_count {
                if inert.contains(&i) {
                    continue;
                }
                let expected = super::hints::invocation_shimmer_expectation(&invocation, i)
                    .map(|hint| hint.expected);
                if let Some(expected) = expected
                    && let Some(word) = invocation.effective_words().words.get(i + 1)
                {
                    push_invocation_var_read(
                        ctx,
                        &mut out,
                        (&invocation, i),
                        word,
                        expected,
                        stmt.span(),
                    );
                }
            }
        }
        Statement::AssignValue { tokens, .. } => {
            // An assignment's value word substitutes exactly as a call's
            // words do, so its reads land the same way and through the same
            // lift — `set r [list [lindex $x 0]]` converts `x` to a list just
            // as `set r [lindex $x 0]` does, and the outermost `[cmd …]` is
            // only the depth-zero case of that walk.
            push_lifted_reads(ctx, &mut out, tokens.as_ref(), stmt.span());
        }
        Statement::Incr { name, amount, .. } => {
            push_named_target_read(
                ctx,
                &mut out,
                &format!("${name}"),
                TclType::Int,
                stmt.span(),
                uses,
            );
            if let Some(amt) = amount.as_deref().map(str::trim)
                && amt.starts_with('$')
                && let Some(word) = ctx
                    .source
                    .source_tokens()
                    .and_then(|tokens| tokens.words().get(2))
            {
                push_var_read(ctx, &mut out, word, TclType::Int, stmt.span());
            }
        }
        Statement::AssignExpr {
            expr,
            span,
            expr_base,
            ..
        }
        | Statement::ExprEval {
            expr,
            span,
            expr_base,
            ..
        } => {
            out.extend(typed_reads_of_expr(ctx, expr, *expr_base, *span));
        }
        _ => {}
    }
    out
}

/// Push the typed reads of every `[cmd …]` nested in `tokens`' words.
///
/// Each substitution reads its arguments exactly as the same command written
/// on its own line would, so this mirrors the direct-call arm — with one extra
/// case: a nested `[expr …]` carries its reads in an expression, not in
/// registry argument roles, so its operands are extracted through
/// [`typed_reads_of_expr`] instead.
///
/// The reads are appended innermost-first, which is the order Tcl evaluates
/// them in.
fn push_lifted_reads(
    ctx: &CommitCtx<'_>,
    out: &mut Vec<TypedRead>,
    tokens: Option<&crate::ir::CommandTokens>,
    span: Span,
) {
    let config = ctx.context.config();
    for lifted in crate::word_subst::lifted_calls(tokens, config) {
        let Some(tokens) = &lifted.tokens else {
            continue;
        };
        let Some(invocation) = ctx.context.invocation(tokens) else {
            continue;
        };
        if let Some(expression) = ctx.context.expression(tokens, lifted.span) {
            out.extend(typed_reads_of_executed_expr(ctx, &expression, span));
            continue;
        }
        let inert = inert_effective_args(ctx.registry, &invocation);

        for i in 0..invocation.argument_count() {
            if inert.contains(&i) {
                continue;
            }
            if let Some(expected) = super::hints::invocation_shimmer_expectation(&invocation, i)
                .map(|hint| hint.expected)
                && let Some(word) = invocation.effective_words().words.get(i + 1)
            {
                push_invocation_var_read(ctx, out, (&invocation, i), word, expected, span);
            }
        }
    }
}

/// Extract the typed operand reads of one expression AST: arithmetic /
/// bitwise / shift operands read as `Numeric`, `&&`/`||` operands as
/// `Boolean`, and the RIGHT operand of `in`/`ni` as `List`.  String
/// comparisons are dual-ported (no conversion) and comparison operators probe
/// per-value (statically unknowable) — neither commits.
fn typed_reads_of_expr(
    ctx: &CommitCtx<'_>,
    node: &ExprNode,
    expr_base: Option<u32>,
    span: Span,
) -> Vec<TypedRead> {
    let mut out = Vec::new();
    collect_expr_reads(
        ctx,
        node,
        ExpressionReads {
            base: expr_base,
            executed: None,
        },
        span,
        &mut out,
        0,
    );
    out
}

#[derive(Clone, Copy)]
struct ExpressionReads<'a> {
    base: Option<u32>,
    executed: Option<&'a crate::command_binding::ExecutedExpressionSource>,
}

fn typed_reads_of_executed_expr(
    ctx: &CommitCtx<'_>,
    expression: &crate::word_subst::LiftedSourceExpression,
    span: Span,
) -> Vec<TypedRead> {
    let mut out = Vec::new();
    let reads = ExpressionReads {
        base: expression.expression_base,
        executed: expression.executed_source.as_ref(),
    };
    collect_expr_reads(ctx, &expression.expression, reads, span, &mut out, 0);
    out
}

fn collect_expr_reads(
    ctx: &CommitCtx<'_>,
    node: &ExprNode,
    expr_base: ExpressionReads<'_>,
    span: Span,
    out: &mut Vec<TypedRead>,
    depth: u32,
) {
    // Native-stack safety net: walks the `ExprNode` tree, one
    // native frame per level. Past the cap, stop descending — a collector
    // that returns the typed reads gathered so far is the safe fallback
    // (reads buried deeper than the cap are not committed; never a crash).
    if MAX_EXPR_NODE_DEPTH.exceeded(depth) {
        return;
    }
    match node {
        ExprNode::Binary {
            op, left, right, ..
        } => {
            collect_expr_reads(ctx, left, expr_base, span, out, depth + 1);
            collect_expr_reads(ctx, right, expr_base, span, out, depth + 1);
            match op {
                BinOp::Add
                | BinOp::Sub
                | BinOp::Mul
                | BinOp::Div
                | BinOp::Mod
                | BinOp::Pow
                | BinOp::LShift
                | BinOp::RShift
                | BinOp::BitAnd
                | BinOp::BitOr
                | BinOp::BitXor => {
                    push_expr_var_read(ctx, out, left, TclType::Numeric, span, expr_base);
                    push_expr_var_read(ctx, out, right, TclType::Numeric, span, expr_base);
                }
                BinOp::And | BinOp::Or => {
                    push_expr_var_read(ctx, out, left, TclType::Boolean, span, expr_base);
                    push_expr_var_read(ctx, out, right, TclType::Boolean, span, expr_base);
                }
                BinOp::In | BinOp::Ni => {
                    push_expr_var_read(ctx, out, right, TclType::List, span, expr_base);
                }
                _ => {}
            }
        }
        ExprNode::Unary { operand, .. } => {
            collect_expr_reads(ctx, operand, expr_base, span, out, depth + 1);
        }
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
            ..
        } => {
            collect_expr_reads(ctx, condition, expr_base, span, out, depth + 1);
            collect_expr_reads(ctx, true_branch, expr_base, span, out, depth + 1);
            collect_expr_reads(ctx, false_branch, expr_base, span, out, depth + 1);
        }
        _ => {}
    }
}

/// Push a typed read for a `$var` argument word, resolving its SSA use
/// version; non-variable words (literals, substitutions) commit nothing here —
/// their values are not tracked variables.
fn push_named_target_read(
    ctx: &CommitCtx<'_>,
    out: &mut Vec<TypedRead>,
    word: &str,
    expected: TclType,
    span: Span,
    uses: &HashMap<Symbol, u32>,
) {
    let stripped = word.trim();
    if !is_pure_var_ref(stripped) {
        return;
    }
    let var = normalise_var_name(stripped);
    let Some(sym) = ctx.source.symbol(var) else {
        return;
    };
    let Some(&ver) = uses.get(&sym) else {
        return;
    };
    if ver == 0 {
        return;
    }
    out.push(TypedRead {
        sym,
        ver,
        expected,
        span,
    });
}

/// Push a retained word read using the actual reference's cell and version.
fn push_invocation_var_read(
    ctx: &CommitCtx<'_>,
    out: &mut Vec<TypedRead>,
    selected: (
        &crate::registry_invocation::NormalRepresentationInvocation,
        usize,
    ),
    word: &crate::ir::WordExpr,
    expected: TclType,
    span: Span,
) {
    if !super::hints::operand_preserves_captured_cache(
        selected.0,
        selected.1,
        ctx.source
            .read_word_representation_advice(word, ctx.registry),
    ) {
        push_var_read(ctx, out, word, expected, span);
    }
}

/// Push an operation's actual conversion using the retained cell and version.
fn push_var_read(
    ctx: &CommitCtx<'_>,
    out: &mut Vec<TypedRead>,
    word: &crate::ir::WordExpr,
    expected: TclType,
    span: Span,
) {
    if let Some(read) = ctx.source.read_word(word) {
        push_positioned_read(out, read, expected, span);
    }
}

fn push_positioned_read(
    out: &mut Vec<TypedRead>,
    read: crate::ssa::SsaReadReference,
    expected: TclType,
    span: Span,
) {
    if let Some(ver) = read.version.filter(|ver| *ver != 0) {
        out.push(TypedRead {
            sym: read.symbol,
            ver,
            expected,
            span,
        });
    }
}

/// Push a native expression read at the parser's original source base.
fn push_expr_var_read(
    ctx: &CommitCtx<'_>,
    out: &mut Vec<TypedRead>,
    node: &ExprNode,
    expected: TclType,
    span: Span,
    expr_base: ExpressionReads<'_>,
) {
    let read = match expr_base.executed {
        Some(source) => ctx.source.read_executed_expression_variable(node, source),
        None => ctx.source.read_expression_variable(node, expr_base.base),
    };
    if let Some(read) = read {
        push_positioned_read(out, read, expected, span);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compilation_unit::CompilationUnit;
    use tcl_registry::CommandRegistry;

    fn registry() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    /// Real callback coercion can leave equal bytes in the same SSA version
    /// with a different intrep. Closed cost alternatives must also never
    /// recover an every-path commitment from that earlier use.
    #[test]
    fn captured_physical_representation_supersedes_replayed_commitment() {
        use tcl_syntax::value::ValueRepresentation;
        let old_span = Span::new(0, 4);
        let earlier = CommitState::committed(TclType::Dict, Some(old_span));
        let current = earlier.clone().at_captured_representation(
            Some(ValueRepresentation::List),
            None,
            false,
        );
        assert_eq!(current.single_committed(), Some(TclType::List));
        assert!(!current.must_pay(TclType::List));
        assert_eq!(current.first_span(), None);
        let pure = earlier.clone().at_captured_representation(
            Some(ValueRepresentation::String),
            None,
            false,
        );
        assert_eq!(pure, CommitState::pure());
        let alternatives =
            crate::native_numeric::ClosedContainerRepresentations::of(ValueRepresentation::List)
                .unwrap()
                .joined(
                    crate::native_numeric::ClosedContainerRepresentations::of(
                        ValueRepresentation::Dict,
                    )
                    .unwrap(),
                );
        let possible = earlier
            .clone()
            .at_captured_representation(None, Some(alternatives), false);
        assert_eq!(possible.may_types(), &[TclType::List, TclType::Dict]);
        assert!(!possible.must_pay(TclType::Int));
        assert_eq!(possible.single_committed(), None);
        assert_eq!(possible.first_span(), None);
        let unknown = earlier.at_captured_representation(None, None, false);
        assert!(!unknown.must_pay(TclType::List));
        assert!(unknown.may_types().is_empty());
    }

    #[test]
    fn numeric_commitment_is_separate_from_unknown_container_projection() {
        let numeric = CommitState::committed(TclType::Double, None);
        assert_eq!(
            numeric.clone().at_captured_representation(None, None, true),
            numeric
        );
        assert_eq!(
            numeric.clone().at_captured_representation(
                Some(tcl_syntax::value::ValueRepresentation::String),
                None,
                false,
            ),
            CommitState::pure()
        );
        assert!(
            !numeric
                .at_captured_representation(None, None, false)
                .must_pay(TclType::List)
        );
    }

    fn facts_for<'a>(
        cu: &'a CompilationUnit,
        registry: &'a CommandRegistry,
        func: &str,
    ) -> (CommitFacts, &'a crate::compilation_unit::FunctionUnit) {
        let fu = cu.function(func).unwrap();
        let ctx = CommitCtx {
            registry,
            context: crate::shimmer::ShimmerContext::standalone(registry),
            ssa: &fu.ssa,
            source: crate::ssa::SsaSourceView::unpositioned(&fu.ssa),
            types: &fu.types,
            values: &fu.sccp.values,
            folded: &fu.sccp.folded_types,
        };
        let facts = compute_commit_facts(
            &fu.cfg,
            &ctx,
            &fu.sccp.executable_blocks,
            &fu.sccp.executable_edges,
        );
        (facts, fu)
    }

    /// `collect_expr_reads` recurses once
    /// per `ExprNode` level, so it needs a depth cap. A tree built
    /// directly is unbounded (the Pratt parser caps its own output at 256)
    /// and empirically overflowed the native stack (SIGABRT) in the low
    /// thousands of levels on a 2 MiB thread. 3000 is past that crash range
    /// and past `MAX_EXPR_NODE_DEPTH` (256); the assertion is that
    /// `typed_reads_of_expr` (which drives `collect_expr_reads`) returns at
    /// all.
    #[test]
    fn deeply_nested_collect_expr_reads_survives() {
        use crate::expr_ast::{ExprNode, UnaryOp};
        use tcl_lexer::Span;
        let r = registry();
        let cu = CompilationUnit::build_for("set v 5", &r, false);
        let fu = cu.function("::top").unwrap();
        let ctx = CommitCtx {
            registry: &r,
            context: crate::shimmer::ShimmerContext::standalone(&r),
            ssa: &fu.ssa,
            source: crate::ssa::SsaSourceView::unpositioned(&fu.ssa),
            types: &fu.types,
            values: &fu.sccp.values,
            folded: &fu.sccp.folded_types,
        };
        let mut node = ExprNode::Var {
            text: "$v".into(),
            name: "v".into(),
            start: 0,
            end: 2,
        };
        for _ in 0..3000 {
            node = ExprNode::Unary {
                op: UnaryOp::Not,
                operand: Box::new(node),
            };
        }
        let _ = typed_reads_of_expr(&ctx, &node, None, Span::new(0, 1));
    }

    /// A read the committed representation already satisfies must not replace
    /// it. `set d [expr {sqrt($x)}]; expr {$d && 1}`
    /// leaves `d` a double on tclsh (the boolean read installs nothing), so a
    /// later `incr d` is still the `Double` -> `Int` mismatch it was — which
    /// recording the *expectation* instead of the *representation* hid.
    #[test]
    fn a_compatible_read_keeps_the_committed_representation() {
        let span = Span::new(0, 1);
        let mut state = CommitState::committed(TclType::Double, Some(span));

        state.commit(TclType::Boolean, span);
        assert_eq!(
            state.single_committed(),
            Some(TclType::Double),
            "a boolean read of a double installs nothing, so the double stands"
        );
        state.commit(TclType::Numeric, span);
        assert_eq!(state.single_committed(), Some(TclType::Double));
        assert!(
            state.must_pay(TclType::Int),
            "`incr` on that double is still a genuine mismatch"
        );

        // An incompatible read still re-represents, as before.
        state.commit(TclType::List, span);
        assert_eq!(state.single_committed(), Some(TclType::List));
    }

    #[test]
    fn exceptional_edge_does_not_inherit_normal_conversion_promise() {
        let r = registry();
        let cu = CompilationUnit::build_for("set v 5", &r, false);
        let fu = cu.function("::top").unwrap();
        let ctx = CommitCtx {
            registry: &r,
            context: crate::shimmer::ShimmerContext::standalone(&r),
            ssa: &fu.ssa,
            source: crate::ssa::SsaSourceView::unpositioned(&fu.ssa),
            types: &fu.types,
            values: &fu.sccp.values,
        };
        let key = (fu.ssa.var_symbol("v").unwrap(), 1);
        let source = fu.cfg.entry;
        let target = BlockId(99);
        let predecessors = HashMap::from([(target, HashSet::from([source]))]);
        let executable = HashSet::from([(source, target)]);
        let exits = HashMap::from([(
            source,
            HashMap::from([(key, CommitState::committed(TclType::List, None))]),
        )]);
        let mut cfg = fu.cfg.clone();
        let normal = join_entry(&ctx, &cfg, target, &predecessors, &executable, &exits);
        assert_eq!(normal[&key].single_committed(), Some(TclType::List));
        cfg.exception_edges.push((source, target));
        let exceptional = join_entry(&ctx, &cfg, target, &predecessors, &executable, &exits);
        assert_eq!(exceptional[&key].single_committed(), None);
        assert!(!exceptional[&key].must_pay(TclType::Dict));
    }

    /// Straight-line: `expr` commits Numeric; the state at the following
    /// statement must-pays a List read (the classic second-conversion FN).
    #[test]
    fn straight_line_expr_commit_then_list_must_pay() {
        let r = registry();
        let cu = CompilationUnit::build_for("set v 5\nexpr {$v + 1}\nlindex $v 0", &r, false);
        let (facts, fu) = facts_for(&cu, &r, "::top");
        let ctx = CommitCtx {
            registry: &r,
            context: crate::shimmer::ShimmerContext::standalone(&r),
            ssa: &fu.ssa,
            source: crate::ssa::SsaSourceView::unpositioned(&fu.ssa),
            types: &fu.types,
            values: &fu.sccp.values,
            folded: &fu.sccp.folded_types,
        };
        let entry = fu.cfg.entry;
        let mut walker = facts.walker(&ctx, entry);
        let sym = fu.ssa.var_symbol("v").unwrap();
        let ssa_block = fu.ssa.blocks.get(&entry).unwrap();
        // Replay to just before the lindex call.
        for ss in &ssa_block.statements {
            if let Statement::Call { command, .. } = &ss.statement
                && command == "lindex"
            {
                let ver = ss.uses[&sym];
                let state = walker.state_of(sym, ver);
                assert!(
                    state.must_pay(TclType::List),
                    "expr committed Numeric; lindex must pay: {state:?}"
                );
                assert_eq!(state.single_committed(), Some(TclType::Numeric));
                return;
            }
            walker.step(&ss.statement, &ss.uses);
        }
        panic!("lindex statement not found");
    }

    /// Branch case: one arm commits Numeric (expr), the other List (llength).
    /// After the merge a Dict read pays on both paths (fires); a List read
    /// pays on only one (stays silent).
    #[test]
    fn merge_of_two_commitments_is_must_only_off_both() {
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let r = tcl_registry::model::ingress::static_context_for_profile(profile).commands();
        // One numeric element reaches the merge from both arms. A two-element
        // value makes the arithmetic arm error before the following read.
        let src = "proc f {c} {\n  set a {1}\n  if {$c} { expr {$a + 1} } else { llength $a }\n  dict size $a\n}\n";
        let cu = CompilationUnit::build_for_profile(src, r, false, profile);
        let (facts, fu) = facts_for(&cu, r, "::f");
        let ctx = CommitCtx {
            registry: r,
            context: crate::shimmer::ShimmerContext::standalone(r),
            ssa: &fu.ssa,
            source: crate::ssa::SsaSourceView::unpositioned(&fu.ssa),
            types: &fu.types,
            values: &fu.sccp.values,
            folded: &fu.sccp.folded_types,
        };
        // Find the block holding the `dict size` call and replay to it.
        for (&bid, ssa_block) in &fu.ssa.blocks {
            let mut walker = facts.walker(&ctx, bid);
            for (index, ss) in ssa_block.statements.iter().enumerate() {
                if let Statement::Call { command, args, .. } = &ss.statement
                    && command == "dict"
                    && args.first().map(String::as_str) == Some("size")
                {
                    let source = crate::ssa::SsaSourceView::at_statement(&fu.ssa, bid, index);
                    let sym = source.symbol("a").unwrap();
                    let ver = ss.uses[&sym];
                    let state = walker.state_of(sym, ver);
                    assert!(
                        state.must_pay(TclType::Dict),
                        "both arms committed non-Dict; dict read must pay: {state:?}"
                    );
                    assert!(
                        !state.must_pay(TclType::List),
                        "one arm committed List; a List read pays on one path only: {state:?}"
                    );
                    return;
                }
                walker.step(&ss.statement, &ss.uses);
            }
        }
        panic!("dict size statement not found");
    }

    #[test]
    fn current_numeric_category_supersedes_semantic_labels_and_replay() {
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let r = tcl_registry::model::ingress::static_context_for_profile(profile).commands();
        // The reached increment creates the numeric store. A constant
        // expression may instead return a shared compiler-pool object.
        let src = "set x 0; incr x; llength $x";
        let last_read = u32::try_from(src.rfind("llength").unwrap()).unwrap();
        let cu = CompilationUnit::build_for_profile(src, r, false, profile);
        let (facts, fu) = facts_for(&cu, r, "::top");
        let ctx = CommitCtx {
            registry: r,
            context: crate::shimmer::ShimmerContext::standalone(r),
            ssa: &fu.ssa,
            source: crate::ssa::SsaSourceView::unpositioned(&fu.ssa),
            types: &fu.types,
            values: &fu.sccp.values,
        };
        for (&block, body) in &fu.ssa.blocks {
            let mut walker = facts.walker(&ctx, block);
            for (index, statement) in body.statements.iter().enumerate() {
                if statement.statement.span().start() == last_read {
                    let source = crate::ssa::SsaSourceView::at_statement(&fu.ssa, block, index);
                    let word = &statement.statement.tokens().unwrap().words()[1];
                    let read = source.read_word(word).expect("original numeric read");
                    // Neither an incompatible contents label nor an older
                    // replayed subtype may replace the current native object.
                    walker.state.insert(
                        (read.symbol, read.version.unwrap()),
                        CommitState::committed(TclType::Double, None),
                    );
                    let cost = walker
                        .cost_for_word(source, word, read, TclType::Double, TclType::List)
                        .expect("current native integer representation");
                    assert_eq!(cost.current, TclType::Int);
                    assert_eq!(cost.commitment.single_committed(), Some(TclType::Int));
                    assert!(cost.commitment.must_pay(TclType::List));
                    return;
                }
                walker.step(&statement.statement, &statement.uses);
            }
        }
        panic!("original numeric read not found");
    }

    #[test]
    fn selected_length_replay_preserves_only_the_actual_native_numeric_cache() {
        let source = "set x 0; incr x; llength $x; incr x";
        let last = u32::try_from(source.rfind("incr").unwrap()).unwrap();
        for name in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let profile = tcl_dialect::DialectProfile::find(name).unwrap();
            let registry =
                tcl_registry::model::ingress::static_context_for_profile(profile).commands();
            let unit = CompilationUnit::build_for_profile(source, registry, false, profile);
            let (facts, function) = facts_for(&unit, registry, "::top");
            let context = CommitCtx {
                registry,
                context: crate::shimmer::ShimmerContext::standalone(registry),
                ssa: &function.ssa,
                source: crate::ssa::SsaSourceView::unpositioned(&function.ssa),
                types: &function.types,
                values: &function.sccp.values,
            };
            let mut found = false;
            for (&block, body) in &function.ssa.blocks {
                let mut walker = facts.walker(&context, block);
                for (index, statement) in body.statements.iter().enumerate() {
                    if statement.statement.span().start() == last {
                        let point =
                            crate::ssa::SsaSourceView::at_statement(&function.ssa, block, index);
                        let read = point
                            .reaching_binding("x", registry)
                            .expect("original increment binding");
                        assert_eq!(
                            walker
                                .state_of(read.symbol, read.version.unwrap())
                                .single_committed(),
                            Some(if name == "tcl8.6" {
                                TclType::List
                            } else {
                                TclType::Int
                            }),
                            "{name}"
                        );
                        found = true;
                    }
                    walker.step(&statement.statement, &statement.uses);
                }
            }
            assert!(found, "original increment must be retained: {name}");
        }
    }

    #[test]
    fn an_erroring_conversion_arm_cannot_commit_the_normal_merge() {
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let r = tcl_registry::model::ingress::static_context_for_profile(profile).commands();
        // An actual two-element native list supplies independent physical
        // representation evidence; its arithmetic conversion still errors.
        let src = "proc f {c} {\n set a [list 1 2]\n\
                   if {$c} {expr {$a + 1}} else {llength $a}\n llength $a\n}";
        let last_read = u32::try_from(src.rfind("llength").unwrap()).unwrap();
        let cu = CompilationUnit::build_for_profile(src, r, false, profile);
        let (facts, fu) = facts_for(&cu, r, "::f");
        let ctx = CommitCtx {
            registry: r,
            context: crate::shimmer::ShimmerContext::standalone(r),
            ssa: &fu.ssa,
            source: crate::ssa::SsaSourceView::unpositioned(&fu.ssa),
            types: &fu.types,
            values: &fu.sccp.values,
        };
        for (&block, body) in &fu.ssa.blocks {
            let mut walker = facts.walker(&ctx, block);
            for (index, statement) in body.statements.iter().enumerate() {
                if statement.statement.span().start() == last_read
                    && matches!(&statement.statement, Statement::Call { command, .. } if command == "llength")
                {
                    let source = crate::ssa::SsaSourceView::at_statement(&fu.ssa, block, index);
                    let word = &statement
                        .statement
                        .tokens()
                        .expect("original command")
                        .words()[1];
                    let read = source.read_word(word).expect("captured normal a read");
                    // CFG replay conservatively keeps the erroring arm. Cost
                    // advice must reconcile it with this actually reached read.
                    let cost = walker
                        .cost_for_word(source, word, read, TclType::String, TclType::List)
                        .expect("normal list read has current representation advice");
                    let state = cost.commitment;
                    assert_eq!(cost.current, TclType::List);
                    assert_eq!(
                        state.single_committed(),
                        Some(TclType::List),
                        "only the successful list conversion reaches this read: {state:?}"
                    );
                    assert!(!state.must_pay(TclType::List));
                    return;
                }
                walker.step(&statement.statement, &statement.uses);
            }
        }
        panic!("normal merge list read not found");
    }

    /// Def-site pushback: a pure literal whose only typed read is a List read
    /// resolves to `List` in the single-commitments map.
    #[test]
    fn single_commitment_pushback_for_pure_literal() {
        let r = registry();
        let cu = CompilationUnit::build_for("set l {1 2 3}\nllength $l", &r, false);
        let (facts, fu) = facts_for(&cu, &r, "::top");
        let ctx = CommitCtx {
            registry: &r,
            context: crate::shimmer::ShimmerContext::standalone(&r),
            ssa: &fu.ssa,
            source: crate::ssa::SsaSourceView::unpositioned(&fu.ssa),
            types: &fu.types,
            values: &fu.sccp.values,
            folded: &fu.sccp.folded_types,
        };
        let pushback = facts.single_commitments(&ctx);
        let sym = fu.ssa.var_symbol("l").unwrap();
        assert_eq!(
            pushback.get(&(sym, 1)),
            Some(&TclType::List),
            "pure literal first-used-as-list must push back List: {pushback:?}"
        );
    }

    /// A committed producer (`[dict create]`) is not "first used as" anything —
    /// the pushback map only covers pure defs.
    #[test]
    fn no_pushback_for_committed_producer() {
        let r = registry();
        let cu = CompilationUnit::build_for("set d [dict create a 1]\nllength $d", &r, false);
        let (facts, fu) = facts_for(&cu, &r, "::top");
        let ctx = CommitCtx {
            registry: &r,
            context: crate::shimmer::ShimmerContext::standalone(&r),
            ssa: &fu.ssa,
            source: crate::ssa::SsaSourceView::unpositioned(&fu.ssa),
            types: &fu.types,
            values: &fu.sccp.values,
            folded: &fu.sccp.folded_types,
        };
        let pushback = facts.single_commitments(&ctx);
        let sym = fu.ssa.var_symbol("d").unwrap();
        assert!(
            !pushback.contains_key(&(sym, 1)),
            "committed producers must not appear in the pushback map: {pushback:?}"
        );
    }
}
