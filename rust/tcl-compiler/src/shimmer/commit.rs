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

use super::hints::{arg_shimmer_type, is_numeric_compatible, is_pure_intrep};
use super::use_site::foreach_header_expected_type;

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
    pub fn commit(&mut self, t: TclType, span: Span) {
        if self.first_span.is_none() {
            self.first_span = Some(span);
        }
        self.may.clear();
        self.may.push(t);
        self.all_committed = true;
        self.overflowed = false;
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
            ssa: ctx.ssa,
            types: ctx.types,
            values: ctx.values,
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
pub struct CommitCtx<'a> {
    /// Command registry, for `arg_types` shimmer hints and foreach headers.
    pub registry: &'a CommandRegistry,
    /// SSA form, for variable symbols and per-statement use versions.
    pub ssa: &'a SsaFunction,
    /// Per-version inferred types, for each version's initial purity.
    pub types: &'a HashMap<ValueKey, TypeLattice>,
    /// SCCP constants, for the numeric-literal purity distinction.
    pub values: &'a HashMap<ValueKey, LatticeValue>,
}

/// A per-block replay of the commitment transfer function, kept in step with a
/// detector's statement walk: query [`Self::state_of`] *before* checking a
/// statement, then [`Self::step`] past it.
pub struct CommitWalker<'a> {
    state: HashMap<ValueKey, CommitState>,
    registry: &'a CommandRegistry,
    ssa: &'a SsaFunction,
    types: &'a HashMap<ValueKey, TypeLattice>,
    values: &'a HashMap<ValueKey, LatticeValue>,
}

impl CommitWalker<'_> {
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
            ssa: self.ssa,
            types: self.types,
            values: self.values,
        };
        initial_state(&ctx, key).unwrap_or_default()
    }

    /// Advance the state past one statement.
    pub fn step(&mut self, stmt: &Statement, uses: &HashMap<Symbol, u32>) {
        let ctx = CommitCtx {
            registry: self.registry,
            ssa: self.ssa,
            types: self.types,
            values: self.values,
        };
        for read in typed_reads_of_statement(&ctx, stmt, uses) {
            apply_read(&ctx, &mut self.state, read, None);
        }
    }
}

/// The state a version starts in at its def: pure when the producer left the
/// value uncommitted ([`is_pure_intrep`] over the type lattice + SCCP constant
/// — a literal, an interpolation, or a string-command result), committed to
/// the producer's intrep otherwise (`[list …]`, `[dict create …]`, `expr`,
/// `binary format`, …).  `None` when the version has no known type (stays
/// pure-with-unknown: never drives a warning because `must_pay` needs a
/// non-empty may-set).
fn initial_state(ctx: &CommitCtx<'_>, key: ValueKey) -> Option<CommitState> {
    let lattice = ctx.types.get(&key)?;
    if lattice.kind() != TypeKind::Known {
        return Some(CommitState::pure());
    }
    let t = lattice.tcl_type()?;
    Some(if is_pure_intrep(t, ctx.values.get(&key)) {
        CommitState::pure()
    } else {
        CommitState::committed(t, None)
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
            let entry = join_entry(ctx, block_id, &preds, executable_edges, &block_exit);
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
            let s = block_exit
                .get(p)
                .and_then(|m| m.get(&key))
                .cloned()
                .unwrap_or_else(|| initial_state(ctx, key).unwrap_or_default());
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
        for ss in &ssa_block.statements {
            for read in typed_reads_of_statement(ctx, &ss.statement, &ss.uses) {
                apply_read(ctx, &mut state, read, Some(use_commit_types));
            }
        }
        // Branch-condition reads live on the terminator, evaluated after the
        // block's statements with its exit versions.
        if let Some(block) = cfg.blocks.get(&block_id)
            && let Some(Terminator::Branch {
                condition, span, ..
            }) = &block.terminator
        {
            let branch_span = span.unwrap_or_else(|| Span::new(0, 0));
            for read in typed_reads_of_expr(ctx, condition, &ssa_block.exit_versions, branch_span) {
                apply_read(ctx, &mut state, read, Some(use_commit_types));
            }
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
        Statement::Call {
            args,
            foreach_groups,
            ..
        } => {
            let lookup = stmt.canonical_command_or_source();
            let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
            for (i, word) in args.iter().enumerate() {
                let expected = if foreach_groups.is_some() {
                    foreach_header_expected_type(ctx.registry, lookup)
                } else {
                    arg_shimmer_type(ctx.registry, lookup, &arg_refs, i)
                };
                if let Some(expected) = expected {
                    push_var_read(ctx, &mut out, word, expected, stmt.span(), uses);
                }
            }
        }
        Statement::AssignValue { value, .. } => {
            if let Some((command, args)) =
                crate::value_shapes::parse_command_substitution(value.trim())
            {
                let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
                for (i, word) in args.iter().enumerate() {
                    if let Some(expected) = arg_shimmer_type(ctx.registry, &command, &arg_refs, i) {
                        push_var_read(ctx, &mut out, word, expected, stmt.span(), uses);
                    }
                }
            }
        }
        Statement::Incr { name, amount, .. } => {
            push_var_read(
                ctx,
                &mut out,
                &format!("${name}"),
                TclType::Int,
                stmt.span(),
                uses,
            );
            if let Some(amt) = amount.as_deref().map(str::trim)
                && amt.starts_with('$')
            {
                push_var_read(ctx, &mut out, amt, TclType::Int, stmt.span(), uses);
            }
        }
        Statement::AssignExpr { expr, span, .. } | Statement::ExprEval { expr, span, .. } => {
            out.extend(typed_reads_of_expr(ctx, expr, uses, *span));
        }
        _ => {}
    }
    out
}

/// Extract the typed operand reads of one expression AST: arithmetic /
/// bitwise / shift operands read as `Numeric`, `&&`/`||` operands as
/// `Boolean`, and the RIGHT operand of `in`/`ni` as `List`.  String
/// comparisons are dual-ported (no conversion) and comparison operators probe
/// per-value (statically unknowable) — neither commits.
fn typed_reads_of_expr(
    ctx: &CommitCtx<'_>,
    node: &ExprNode,
    uses: &HashMap<Symbol, u32>,
    span: Span,
) -> Vec<TypedRead> {
    let mut out = Vec::new();
    collect_expr_reads(ctx, node, uses, span, &mut out, 0);
    out
}

fn collect_expr_reads(
    ctx: &CommitCtx<'_>,
    node: &ExprNode,
    uses: &HashMap<Symbol, u32>,
    span: Span,
    out: &mut Vec<TypedRead>,
    depth: u32,
) {
    // Native-stack safety net (issue #996): walks the `ExprNode` tree, one
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
            collect_expr_reads(ctx, left, uses, span, out, depth + 1);
            collect_expr_reads(ctx, right, uses, span, out, depth + 1);
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
                    push_expr_var_read(ctx, out, left, TclType::Numeric, span, uses);
                    push_expr_var_read(ctx, out, right, TclType::Numeric, span, uses);
                }
                BinOp::And | BinOp::Or => {
                    push_expr_var_read(ctx, out, left, TclType::Boolean, span, uses);
                    push_expr_var_read(ctx, out, right, TclType::Boolean, span, uses);
                }
                BinOp::In | BinOp::Ni => {
                    push_expr_var_read(ctx, out, right, TclType::List, span, uses);
                }
                _ => {}
            }
        }
        ExprNode::Unary { operand, .. } => {
            collect_expr_reads(ctx, operand, uses, span, out, depth + 1);
        }
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
            ..
        } => {
            collect_expr_reads(ctx, condition, uses, span, out, depth + 1);
            collect_expr_reads(ctx, true_branch, uses, span, out, depth + 1);
            collect_expr_reads(ctx, false_branch, uses, span, out, depth + 1);
        }
        _ => {}
    }
}

/// Push a typed read for a `$var` argument word, resolving its SSA use
/// version; non-variable words (literals, substitutions) commit nothing here —
/// their values are not tracked variables.
fn push_var_read(
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
    let Some(sym) = ctx.ssa.var_symbol(var) else {
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

/// Push a typed read for an expression `Var` leaf.
fn push_expr_var_read(
    ctx: &CommitCtx<'_>,
    out: &mut Vec<TypedRead>,
    node: &ExprNode,
    expected: TclType,
    span: Span,
    uses: &HashMap<Symbol, u32>,
) {
    if let ExprNode::Var { name, .. } = node {
        push_var_read(
            ctx,
            out,
            &format!("${}", name.trim_start_matches('$')),
            expected,
            span,
            uses,
        );
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

    fn facts_for<'a>(
        cu: &'a CompilationUnit,
        registry: &'a CommandRegistry,
        func: &str,
    ) -> (CommitFacts, &'a crate::compilation_unit::FunctionUnit) {
        let fu = cu.function(func).unwrap();
        let ctx = CommitCtx {
            registry,
            ssa: &fu.ssa,
            types: &fu.types,
            values: &fu.sccp.values,
        };
        let facts = compute_commit_facts(
            &fu.cfg,
            &ctx,
            &fu.sccp.executable_blocks,
            &fu.sccp.executable_edges,
        );
        (facts, fu)
    }

    /// Regression coverage for issue #996: `collect_expr_reads` recurses once
    /// per `ExprNode` level with no depth cap before this fix. A tree built
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
            ssa: &fu.ssa,
            types: &fu.types,
            values: &fu.sccp.values,
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
        let uses: HashMap<Symbol, u32> = HashMap::new();
        let _ = typed_reads_of_expr(&ctx, &node, &uses, Span::new(0, 1));
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
            ssa: &fu.ssa,
            types: &fu.types,
            values: &fu.sccp.values,
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
        let r = registry();
        let src = "proc f {c} {\n  set a {1 2}\n  if {$c} { expr {$a + 1} } else { llength $a }\n  dict size $a\n}\n";
        let cu = CompilationUnit::build_for(src, &r, false);
        let (facts, fu) = facts_for(&cu, &r, "::f");
        let ctx = CommitCtx {
            registry: &r,
            ssa: &fu.ssa,
            types: &fu.types,
            values: &fu.sccp.values,
        };
        let sym = fu.ssa.var_symbol("a").unwrap();
        // Find the block holding the `dict size` call and replay to it.
        for (&bid, ssa_block) in &fu.ssa.blocks {
            let mut walker = facts.walker(&ctx, bid);
            for ss in &ssa_block.statements {
                if let Statement::Call { command, args, .. } = &ss.statement
                    && command == "dict"
                    && args.first().map(String::as_str) == Some("size")
                {
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

    /// Def-site pushback: a pure literal whose only typed read is a List read
    /// resolves to `List` in the single-commitments map.
    #[test]
    fn single_commitment_pushback_for_pure_literal() {
        let r = registry();
        let cu = CompilationUnit::build_for("set l {1 2 3}\nllength $l", &r, false);
        let (facts, fu) = facts_for(&cu, &r, "::top");
        let ctx = CommitCtx {
            registry: &r,
            ssa: &fu.ssa,
            types: &fu.types,
            values: &fu.sccp.values,
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
            ssa: &fu.ssa,
            types: &fu.types,
            values: &fu.sccp.values,
        };
        let pushback = facts.single_commitments(&ctx);
        let sym = fu.ssa.var_symbol("d").unwrap();
        assert!(
            !pushback.contains_key(&(sym, 1)),
            "committed producers must not appear in the pushback map: {pushback:?}"
        );
    }
}
