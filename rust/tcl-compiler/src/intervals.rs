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

//! Integer interval abstract domain over SSA values.
//!
//! A *parallel, non-perturbing* numeric analysis: it runs **after** SCCP and
//! reads its constant lattice, but does not feed back into the SCCP lattice or
//! any existing consumer.  Downstream checks (the dynamic index-bounds pass in
//! [`crate::analyser`]) consult it for the *dynamic* (`$var`) cases the purely
//! syntactic bounds checks skip.
//!
//! Each SSA value `(name, version)` maps to an [`Interval`] `[lo, hi]` over the
//! integers, with `None` meaning unbounded (-inf / +inf).  Values that are not
//! provably integral, or whose range we cannot bound, are `TOP` — always sound.
//! Loop-header phis are **widened** so the fixpoint terminates.  Tightening back
//! is query-driven via [`refine_interval`], which intersects a value's interval
//! with the dominating constant-bound guards at a specific use site.  If the
//! bounded fixpoint does not converge within the iteration cap, the whole result
//! degrades to `TOP` rather than risk returning a still-ascending (unsound,
//! too-narrow) interval.
//!
//! Soundness (never claim a tighter range than
//! reality) matters far more than precision, because the only consumers turn a
//! *proven* fact (index in range) into a diagnostic decision.

use std::collections::{HashMap, HashSet};

use tcl_dialect::NumberSyntax;
use tcl_syntax::expr::ast::{BinOp, ExprNode, UnaryOp};
use tcl_syntax::number::{Number, ParseFlags, parse_whole_with};

use crate::analyses::{ConstValue, LatticeValue};
use crate::cfg::{BlockId, Function as CfgFunction, Terminator};
use crate::depth_guard::MAX_EXPR_NODE_DEPTH;
use crate::ssa::{SsaFunction, ValueKey, Version};

/// A bound is an `i64`, or `None` for an infinity (sign given by position).
pub type Bound = Option<i64>;

/// A closed integer interval `[lo, hi]`; `None` bound = ±infinity.
///
/// `lo is None` → -inf, `hi is None` → +inf.  The empty/unreached value is
/// `BOTTOM` (lo > hi sentinel, detected via [`Interval::is_bottom`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Interval {
    /// Lower bound (`None` = -inf).
    pub lo: Bound,
    /// Upper bound (`None` = +inf).
    pub hi: Bound,
}

/// The unbounded interval `[-inf, +inf]`.
pub const TOP: Interval = Interval { lo: None, hi: None };
/// The canonical empty interval (`lo > hi`).
pub const BOTTOM: Interval = Interval {
    lo: Some(1),
    hi: Some(0),
};

/// The single-point interval `[n, n]`.
#[must_use]
pub fn constant(n: i64) -> Interval {
    Interval {
        lo: Some(n),
        hi: Some(n),
    }
}

impl Interval {
    /// True when both bounds are infinite.
    #[must_use]
    pub fn is_top(self) -> bool {
        self.lo.is_none() && self.hi.is_none()
    }

    /// True when the interval is the empty `lo > hi` sentinel.
    #[must_use]
    pub fn is_bottom(self) -> bool {
        matches!((self.lo, self.hi), (Some(l), Some(h)) if l > h)
    }
}

/// Least upper bound: the smallest interval containing both.
#[must_use]
fn join(a: Interval, b: Interval) -> Interval {
    if a.is_bottom() {
        return b;
    }
    if b.is_bottom() {
        return a;
    }
    // lo = min(lo) treating None as -inf; hi = max(hi) treating None as +inf.
    let lo = match (a.lo, b.lo) {
        (Some(x), Some(y)) => Some(x.min(y)),
        _ => None,
    };
    let hi = match (a.hi, b.hi) {
        (Some(x), Some(y)) => Some(x.max(y)),
        _ => None,
    };
    Interval { lo, hi }
}

/// Standard interval widening: any bound that moved outward jumps to ±inf.
/// Guarantees termination of the loop fixpoint.
#[must_use]
fn widen(old: Interval, new: Interval) -> Interval {
    if old.is_bottom() {
        return new;
    }
    if new.is_bottom() {
        return old;
    }
    let lo = match (old.lo, new.lo) {
        (Some(o), Some(n)) if n >= o => Some(o),
        _ => None,
    };
    let hi = match (old.hi, new.hi) {
        (Some(o), Some(n)) if n <= o => Some(o),
        _ => None,
    };
    Interval { lo, hi }
}

#[must_use]
fn add(a: Interval, b: Interval) -> Interval {
    if a.is_bottom() || b.is_bottom() {
        return BOTTOM;
    }
    let lo = match (a.lo, b.lo) {
        (Some(x), Some(y)) => x.checked_add(y),
        _ => None,
    };
    let hi = match (a.hi, b.hi) {
        (Some(x), Some(y)) => x.checked_add(y),
        _ => None,
    };
    Interval { lo, hi }
}

#[must_use]
fn sub(a: Interval, b: Interval) -> Interval {
    if a.is_bottom() || b.is_bottom() {
        return BOTTOM;
    }
    // a - b = [a.lo - b.hi, a.hi - b.lo]
    let lo = match (a.lo, b.hi) {
        (Some(x), Some(y)) => x.checked_sub(y),
        _ => None,
    };
    let hi = match (a.hi, b.lo) {
        (Some(x), Some(y)) => x.checked_sub(y),
        _ => None,
    };
    Interval { lo, hi }
}

#[must_use]
fn mul(a: Interval, b: Interval) -> Interval {
    if a.is_bottom() || b.is_bottom() {
        return BOTTOM;
    }
    // Only attempt when fully bounded; otherwise TOP (sound).
    let (Some(alo), Some(ahi), Some(blo), Some(bhi)) = (a.lo, a.hi, b.lo, b.hi) else {
        return TOP;
    };
    let corners = [
        alo.checked_mul(blo),
        alo.checked_mul(bhi),
        ahi.checked_mul(blo),
        ahi.checked_mul(bhi),
    ];
    // Any overflow → fall back to TOP (sound).
    if corners.iter().any(Option::is_none) {
        return TOP;
    }
    let vals: Vec<i64> = corners.into_iter().flatten().collect();
    Interval {
        lo: vals.iter().copied().min(),
        hi: vals.iter().copied().max(),
    }
}

#[must_use]
fn negate(a: Interval) -> Interval {
    if a.is_bottom() {
        return BOTTOM;
    }
    Interval {
        lo: a.hi.map(|h| -h),
        hi: a.lo.map(|l| -l),
    }
}

/// Greatest lower bound: the largest interval contained in both.  Here `None`
/// is the *identity* (-inf for a lower bound, +inf for an upper), the opposite
/// of [`join`] where it is absorbing.
#[must_use]
fn intersect(a: Interval, b: Interval) -> Interval {
    if a.is_bottom() || b.is_bottom() {
        return BOTTOM;
    }
    let lo = match (a.lo, b.lo) {
        (None, x) | (x, None) => x,
        (Some(x), Some(y)) => Some(x.max(y)),
    };
    let hi = match (a.hi, b.hi) {
        (None, x) | (x, None) => x,
        (Some(x), Some(y)) => Some(x.min(y)),
    };
    Interval { lo, hi }
}

/// The integer value of an `ExprNode::Literal` (incl. bool keyword) under the
/// target release's numeral grammar, else `None`.
#[must_use]
fn literal_int(node: &ExprNode, numbers: NumberSyntax) -> Option<i64> {
    let ExprNode::Literal { text, .. } = node else {
        return None;
    };
    let t = text.trim();
    // Boolean words resolve by unique prefix in expr context (`tru` is a
    // boolean literal to C Tcl) — the canonical word acceptor.
    if let Some(b) = tcl_syntax::boolean::parse_boolean_word(t) {
        return Some(i64::from(b));
    }
    parse_whole_int(t, numbers)
}

/// The `i64` value of a *complete* Tcl integer numeral under `numbers`, via the
/// one shared numeral facility ([`tcl_syntax::number`]).
///
/// The dialect decides the whole shape, and getting it wrong is not merely
/// imprecise: a bare leading zero is **octal up to 8.6** (`0755` == 493) and
/// **decimal from 9.0** (`0755` == 755), so a version-blind reader hands the
/// interval domain a wrong constant. `0o`/`0b` exist only from 8.5, `0d` and
/// `_` separators only from 9.0; a numeral whose prefix the release lacks, or
/// whose digits are invalid for its radix (`0o8`, `0xZZ`), is not a number at
/// all here — it is a bareword, so `None` (→ `TOP`) is the honest answer.
///
/// `integer_only` mirrors `TCL_PARSE_INTEGER_ONLY`: a fractional part is
/// trailing junk rather than a float, so a non-integral literal yields `None`
/// instead of a truncated integer. A magnitude past a wide comes back as
/// [`Number::Big`] and is likewise `None` — the domain's bounds are `i64`.
///
/// The dialect is an explicit parameter, never the thread-local ambient
/// ([`tcl_syntax::number::set_runtime_syntax`]): the LSP analyses documents of
/// different dialects in one process, so ambient state would let one document's
/// grammar decide another document's constants.
#[must_use]
fn parse_whole_int(t: &str, numbers: NumberSyntax) -> Option<i64> {
    let flags = ParseFlags {
        integer_only: true,
        ..ParseFlags::for_syntax(numbers)
    };
    match parse_whole_with(t, flags)? {
        Number::Int(v) => Some(v),
        Number::Big { .. } | Number::Double(_) | Number::Nan { .. } => None,
    }
}

/// Parse a literal IR value word (an [`crate::ir::Statement::AssignConst`]
/// value, an [`crate::ir::Statement::Incr`] amount) as an int, if it is
/// exactly a **canonical** decimal one.
///
/// Deliberately *not* the full numeral grammar, and deliberately not
/// dialect-aware — because it accepts only the spellings every release reads
/// identically. The accepted shape is an optional sign plus a canonical decimal
/// digit run (`0`, or a run with no leading zero): no radix prefix, no leading
/// zero, no `_` separators. Every such text denotes the same integer under
/// `Tcl84`, `Tcl85` and `Tcl90` alike, so no dialect can change the answer and
/// there is nothing to thread.
///
/// The leading-zero exclusion is the load-bearing part: `"0755"` parses as 755
/// to `i64::from_str` but *means* 493 up to 8.6, so accepting it would seed a
/// wrong point interval. This mirrors SCCP's own
/// [`crate::sccp::parse_literal_value`], which keeps a non-round-tripping
/// leading-zero literal as a `ConstValue::String` for exactly that reason —
/// so this function no longer undoes that abstention. Excluded spellings
/// yield `None` → `TOP`, which is imprecise but always sound.
#[must_use]
fn const_int_from_value(text: &str) -> Option<i64> {
    let t = text.trim();
    let digits = t.strip_prefix(['+', '-']).unwrap_or(t);
    // Canonical decimal: all digits, and no leading zero unless it is `0`
    // itself (`0755` / `08` are dialect-dependent; `00` is not canonical).
    let canonical = !digits.is_empty()
        && digits.bytes().all(|b| b.is_ascii_digit())
        && (digits == "0" || !digits.starts_with('0'));
    if canonical {
        t.parse::<i64>().ok()
    } else {
        None
    }
}

/// Abstract-evaluate `expr` over the current interval environment, reading its
/// numeric literals under `numbers` (the target release's numeral grammar).
#[must_use]
pub(crate) fn eval_expr(
    expr: &ExprNode,
    env: &HashMap<String, Interval>,
    numbers: NumberSyntax,
) -> Interval {
    // Public entry: the top of an expression tree is nesting depth 0 (issue
    // #996 — the recursion cap lives in [`eval_expr_at`]).
    eval_expr_at(expr, env, 0, numbers)
}

#[must_use]
fn eval_expr_at(
    expr: &ExprNode,
    env: &HashMap<String, Interval>,
    depth: u32,
    numbers: NumberSyntax,
) -> Interval {
    // Native-stack safety net (issue #996): walks the `ExprNode` tree, one
    // native frame per level. Past the cap, return `TOP` (the unbounded
    // interval) — the same "know nothing" answer this abstract evaluator
    // already gives for any unsupported node, so the result stays a sound
    // over-approximation, never a crash.
    if MAX_EXPR_NODE_DEPTH.exceeded(depth) {
        return TOP;
    }
    match expr {
        ExprNode::Literal { .. } => literal_int(expr, numbers).map_or(TOP, constant),
        ExprNode::Var { name, .. } => env.get(name).copied().unwrap_or(TOP),
        ExprNode::Unary { op, operand } => {
            let inner = eval_expr_at(operand, env, depth + 1, numbers);
            match op {
                UnaryOp::Neg => negate(inner),
                UnaryOp::Pos => inner,
                _ => TOP,
            }
        }
        ExprNode::Binary { op, left, right } => {
            let la = eval_expr_at(left, env, depth + 1, numbers);
            let ra = eval_expr_at(right, env, depth + 1, numbers);
            match op {
                BinOp::Add => add(la, ra),
                BinOp::Sub => sub(la, ra),
                BinOp::Mul => mul(la, ra),
                // Comparisons / logicals yield a boolean 0/1.
                BinOp::Eq
                | BinOp::Ne
                | BinOp::Lt
                | BinOp::Le
                | BinOp::Gt
                | BinOp::Ge
                | BinOp::And
                | BinOp::Or => Interval {
                    lo: Some(0),
                    hi: Some(1),
                },
                _ => TOP,
            }
        }
        _ => TOP,
    }
}

/// The interval a value satisfies given `value <op> k` is true (or false, when
/// `negate`).  Returns the half-line constraint, or `None` if `op` is not an
/// order/equality comparison.
///
/// The `negate` (false-edge) inversion comes from [`BinOp::inverse`] — the
/// owner table in `tcl_syntax::expr::operators` — rather than a local copy of
/// its rows (issue #1437).  Operators the table inverts but this domain does
/// not model (`eq`/`ne`, `in`/`ni`, the string orderings) invert to another
/// unmodelled operator and fall out of the match below as `None`, exactly as
/// they did when the local table passed them through untouched.
///
/// **NaN and the false edge.**  `BinOp::inverse_needs_non_nan` flags the four
/// ordered rows: `!(x < k)` is *not* `x >= k` when `x` may be NaN, because a
/// NaN operand makes every ordered comparison false, so a NaN `x` takes the
/// false edge of `if {$x < k}` without satisfying `x >= k`.  That is sound here
/// because this is an **integer** abstract domain: an [`Interval`] is only ever
/// read as "*if* this value is an integer, it lies in this range", and NaN — like
/// `1.5` or `abc` — is simply not in the domain.  The lone guard-narrowing
/// consumer ([`crate::interval_bounds`]) acts on the fact only where the value
/// is used as a list index, which must be an integer or the access fails
/// anyway, and it reports only when the *whole* interval is out of range, so a
/// non-integer value can never turn the narrowing into a claim about
/// well-formed code.  The equality rows need no such argument: `!=` is the
/// exact complement of `==` even for NaN.
#[must_use]
fn guard_interval(op: BinOp, k: i64, negate: bool) -> Option<Interval> {
    let op = if negate {
        op.inverse().unwrap_or(op)
    } else {
        op
    };
    match op {
        // Saturate the `±1` at the i64 boundary: a branch literal of `i64::MIN`
        // (for `<`) or `i64::MAX` (for `>`) would overflow an unchecked `k ± 1`
        // and panic in debug/test builds. Saturating keeps a *sound* (if by one
        // value wider) interval — an over-approximation is always safe for the
        // analysis (issue 147).
        BinOp::Lt => Some(Interval {
            lo: None,
            hi: Some(k.saturating_sub(1)),
        }),
        BinOp::Le => Some(Interval {
            lo: None,
            hi: Some(k),
        }),
        BinOp::Gt => Some(Interval {
            lo: Some(k.saturating_add(1)),
            hi: None,
        }),
        BinOp::Ge => Some(Interval {
            lo: Some(k),
            hi: None,
        }),
        BinOp::Eq => Some(constant(k)),
        // NE and non-comparisons give no single interval.
        _ => None,
    }
}

/// If `cond` is `$name <cmp> <int-const>` (or the const on the left), return the
/// interval `name` satisfies when `cond` is true (`negate` for the false edge).
/// Only a top-level comparison against a literal int is handled.
#[must_use]
fn guard_constraint(
    cond: &ExprNode,
    name: &str,
    negate: bool,
    numbers: NumberSyntax,
) -> Option<Interval> {
    let ExprNode::Binary { op, left, right } = cond else {
        return None;
    };
    // $name <op> K
    if let ExprNode::Var { name: vn, .. } = left.as_ref()
        && vn == name
        && let Some(k) = literal_int(right, numbers)
    {
        return guard_interval(*op, k, negate);
    }
    // K <op> $name  → rewrite as $name <flipped-op> K
    if let ExprNode::Var { name: vn, .. } = right.as_ref()
        && vn == name
        && let Some(k) = literal_int(left, numbers)
    {
        let mirror = match op {
            BinOp::Lt => BinOp::Gt,
            BinOp::Le => BinOp::Ge,
            BinOp::Gt => BinOp::Lt,
            BinOp::Ge => BinOp::Le,
            other => *other,
        };
        return guard_interval(mirror, k, negate);
    }
    None
}

/// `(name, version) → [branch-block ids]` index for guard narrowing: for
/// each `Branch` block, the names in its condition resolved against the
/// block's exit version of each.
#[must_use]
pub fn build_guard_index(cfg: &CfgFunction, ssa: &SsaFunction) -> HashMap<ValueKey, Vec<BlockId>> {
    let mut index: HashMap<ValueKey, Vec<BlockId>> = HashMap::new();
    for (dn, dblock) in &cfg.blocks {
        let Some(Terminator::Branch { condition, .. }) = &dblock.terminator else {
            continue;
        };
        let Some(sb) = ssa.blocks.get(dn) else {
            continue;
        };
        for name in condition.vars() {
            // `name` is a raw IR scan; only an interned SSA variable can carry
            // a guard fact (an un-interned name is not a tracked value).
            let Some(sym) = ssa.var_symbol(&name) else {
                continue;
            };
            if let Some(&version) = sb.exit_versions.get(&sym) {
                index.entry((sym, version)).or_default().push(*dn);
            }
        }
    }
    index
}

/// True when `ancestor` dominates `node` (walks `node`'s idom chain).
#[must_use]
fn dominates(ssa: &SsaFunction, ancestor: BlockId, node: BlockId) -> bool {
    if ancestor == node {
        return true;
    }
    let mut curr = node;
    loop {
        match ssa.idom.get(&curr) {
            Some(Some(parent)) => {
                if *parent == ancestor {
                    return true;
                }
                curr = *parent;
            }
            _ => return false,
        }
    }
}

/// The guard-analysis lookup tables [`refine_interval`] consults: the
/// branch-block index (`build_guard_index`), the per-block predecessor
/// count (used for the merge / edge-domination check), and the numeral grammar
/// the branch conditions' literal bounds are read under.  Bundled so the
/// refiner keeps a small argument list.
#[derive(Clone, Copy)]
pub struct GuardTables<'a> {
    /// Branch blocks that constrain each `(sym, version)`.
    pub guard_index: &'a HashMap<ValueKey, Vec<BlockId>>,
    /// Predecessor count per block.
    pub pred_counts: &'a HashMap<BlockId, usize>,
    /// The target release's numeric-literal grammar — a guard's constant bound
    /// (`$i < 0755`) means different numbers in different releases, so the
    /// dialect rides along with the tables rather than being re-derived (or,
    /// worse, read from ambient state) inside the refiner. See
    /// [`parse_whole_int`].
    pub numbers: NumberSyntax,
}

/// Narrow `base[(name, version)]` by the constant-bound guards that hold on
/// every path reaching `block`.
#[must_use]
pub fn refine_interval<S1: std::hash::BuildHasher>(
    base: &HashMap<ValueKey, Interval, S1>,
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    block: BlockId,
    name: &str,
    version: Version,
    guards: GuardTables<'_>,
) -> Interval {
    let GuardTables {
        guard_index,
        pred_counts,
        numbers,
    } = guards;
    // A name that was never interned is not a tracked SSA value: no base
    // interval and no guard fact, so its interval is TOP.
    let Some(sym) = ssa.var_symbol(name) else {
        return TOP;
    };
    let mut iv = base.get(&(sym, version)).copied().unwrap_or(TOP);
    let Some(candidate_blocks) = guard_index.get(&(sym, version)) else {
        return iv;
    };
    for &dn in candidate_blocks {
        if dn == block {
            continue;
        }
        let Some(dblock) = cfg.blocks.get(&dn) else {
            continue;
        };
        let Some(Terminator::Branch {
            condition,
            true_target,
            false_target,
            ..
        }) = &dblock.terminator
        else {
            continue;
        };
        let Some(sb) = ssa.blocks.get(&dn) else {
            continue;
        };
        if sb.exit_versions.get(&sym) != Some(&version) {
            continue;
        }
        let true_dom = dominates(ssa, *true_target, block);
        let false_dom = dominates(ssa, *false_target, block);
        // `negate == false` for the true edge (the condition held), `true` for
        // the false edge (the condition failed).
        let (guard_target, negate) = if true_dom && !false_dom {
            (*true_target, false)
        } else if false_dom && !true_dom {
            (*false_target, true)
        } else {
            continue;
        };
        // The guard holds at `block` only if control *must* have traversed the
        // guarded edge to reach it.  When the guarded target has more than one
        // predecessor it is a merge, so another edge could reach it without the
        // guard.  That is sound *only* when the guard variable is redefined at
        // the merge (a phi for `sym`): the phi gives the guarded edge its own
        // incoming version, so the version being refined is the one carried
        // specifically by that edge (the ordinary rotated-loop-header case,
        // where the loop variable is `incr`-ed).  Without such a phi the *same*
        // version flows in from a non-guarded predecessor — e.g. a `break` into
        // a loop exit that is also the header's false target, with the guard
        // variable un-redefined — where the guard does not hold, so refining
        // would be unsound (issue 148).
        let target_preds = pred_counts.get(&guard_target).copied().unwrap_or(0);
        if target_preds > 1 {
            let redefined_at_target = ssa
                .blocks
                .get(&guard_target)
                .is_some_and(|sb| sb.phis.iter().any(|p| p.name == sym));
            if !redefined_at_target {
                continue;
            }
        }
        if let Some(c) = guard_constraint(condition, name, negate, numbers) {
            iv = intersect(iv, c);
        }
    }
    iv
}

/// Seed a `[c, c]` interval from a constant-integer SCCP value, else `None`.
#[must_use]
fn seed_const<S: std::hash::BuildHasher>(
    key: ValueKey,
    values: &HashMap<ValueKey, LatticeValue, S>,
) -> Option<Interval> {
    match values.get(&key) {
        Some(LatticeValue::Const(ConstValue::Int(n))) => Some(constant(*n)),
        Some(LatticeValue::Const(ConstValue::Bool(b))) => Some(constant(i64::from(*b))),
        _ => None,
    }
}

/// Interval produced by `stmt` for its def `name`.
#[must_use]
fn transfer(
    stmt: &crate::ir::Statement,
    name: &str,
    env: &HashMap<String, Interval>,
    old: Interval,
    numbers: NumberSyntax,
) -> Interval {
    use crate::ir::Statement;
    match stmt {
        Statement::AssignConst { value, .. } => const_int_from_value(value).map_or(TOP, constant),
        Statement::AssignExpr { expr, .. } => eval_expr(expr, env, numbers),
        Statement::Incr { amount, .. } => {
            let mut base = env.get(name).copied().unwrap_or(old);
            if base.is_bottom() {
                base = TOP;
            }
            let amt = match amount {
                None => constant(1),
                Some(a) => const_int_from_value(a).map_or(TOP, constant),
            };
            add(base, amt)
        }
        _ => TOP,
    }
}

const MAX_ITERS: usize = 50;

/// The set of loop-header blocks: a block `v` that is the target of a back edge
/// `u → v` (an edge where `v` dominates `u`).  Loop-header phis are widened.
#[must_use]
fn loop_headers(cfg: &CfgFunction, ssa: &SsaFunction) -> HashSet<BlockId> {
    let mut headers = HashSet::new();
    for (u, block) in &cfg.blocks {
        let Some(term) = &block.terminator else {
            continue;
        };
        for v in term.successors() {
            if dominates(ssa, v, *u) {
                headers.insert(v);
            }
        }
    }
    headers
}

/// The numeric-literal grammar of a lowered module's dialect — the
/// [`crate::ir::Module::dialect`] name resolved through its
/// [`tcl_dialect::DialectProfile`].
///
/// `None` — and an unknown name — resolve through `by_opt_name` to the plain-Tcl
/// profile, whose grammar is `Tcl90`: the same default
/// [`crate::codegen::CodegenCtx::numbers`] takes for its hand-built contexts.
#[must_use]
pub fn numbers_for_dialect(dialect: Option<&str>) -> NumberSyntax {
    tcl_dialect::DialectProfile::by_opt_name(dialect)
        .grammar
        .numbers
}

/// [`compute_intervals_with`] under the Tcl 9.0 numeral grammar.
///
/// **A dialect-blind entry point.** It exists for out-of-crate consumers that
/// do not yet thread a dialect (`tcl-explorer`'s `intervals` view); every
/// in-crate consumer calls [`compute_intervals_with`] with the dialect its
/// caller already holds. `Tcl90` is the crate's established
/// no-dialect-available default (see [`crate::codegen::CodegenCtx::numbers`]),
/// and under it a bare leading zero is decimal — so for an 8.x target this
/// entry reads `0755` as 755 rather than 493. Prefer the `_with` form.
#[must_use]
pub fn compute_intervals<S: std::hash::BuildHasher>(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    values: &HashMap<ValueKey, LatticeValue, S>,
) -> HashMap<ValueKey, Interval> {
    compute_intervals_with(cfg, ssa, values, NumberSyntax::default())
}

/// Forward interval analysis over the SSA form.  `values` is the SCCP lattice;
/// a constant integer seeds `[c, c]`.  Everything else starts `BOTTOM` and is
/// refined to fixpoint with loop-header widening.
///
/// `numbers` is the numeral grammar of the release being analysed *for*, threaded
/// explicitly from the caller (the analyser's dialect, a registry's profile, or
/// [`numbers_for_dialect`] over the module's dialect) — never taken from
/// `tcl_syntax::number`'s thread-local ambient, because one process analyses
/// documents of several dialects at once. It decides the value of every
/// numeric literal the pass reads: `0755` is 493 up to 8.6 and 755 from 9.0.
#[must_use]
pub fn compute_intervals_with<S: std::hash::BuildHasher>(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    values: &HashMap<ValueKey, LatticeValue, S>,
    numbers: NumberSyntax,
) -> HashMap<ValueKey, Interval> {
    let mut result: HashMap<ValueKey, Interval> = HashMap::new();
    let cur = |result: &HashMap<ValueKey, Interval>, key: &ValueKey| -> Interval {
        result.get(key).copied().unwrap_or(BOTTOM)
    };

    let headers = loop_headers(cfg, ssa);
    let order = cfg.reverse_postorder();

    let mut converged = false;
    for _ in 0..MAX_ITERS {
        let mut changed = false;
        for bn in &order {
            let Some(ssa_block) = ssa.blocks.get(bn) else {
                continue;
            };
            let is_loop_header = headers.contains(bn);
            for phi in &ssa_block.phis {
                let key = (phi.name, phi.version);
                let mut merged = BOTTOM;
                for &inc in phi.incoming.values() {
                    merged = if inc > 0 {
                        join(merged, cur(&result, &(phi.name, inc)))
                    } else {
                        join(merged, TOP)
                    };
                }
                if is_loop_header {
                    merged = widen(cur(&result, &key), merged);
                }
                if merged != cur(&result, &key) {
                    result.insert(key, merged);
                    changed = true;
                }
            }
            for s in &ssa_block.statements {
                // `env` is keyed by display name for `transfer` / `eval_expr`
                // (which resolve `ExprNode::Var` by name); resolve each use
                // symbol to its name, looking the interval up by `(sym, ver)`.
                let env: HashMap<String, Interval> = s
                    .uses
                    .iter()
                    .map(|(&sym, &ver)| (ssa.var_name(sym).to_owned(), cur(&result, &(sym, ver))))
                    .collect();
                for (&sym, &ver) in &s.defs {
                    let key = (sym, ver);
                    let nm = ssa.var_name(sym);
                    let val = seed_const(key, values).unwrap_or_else(|| {
                        transfer(&s.statement, nm, &env, cur(&result, &key), numbers)
                    });
                    if val != cur(&result, &key) {
                        result.insert(key, val);
                        changed = true;
                    }
                }
            }
        }
        if !changed {
            converged = true;
            break;
        }
    }

    if !converged {
        // The fixpoint did not stabilise within the cap, so `result` may still
        // be ascending (intervals NARROWER than reality) — reporting a finding
        // from such an under-approximation would be unsound.  Degrade every
        // value to TOP so no consumer can derive a finding from unconverged data.
        for v in result.values_mut() {
            *v = TOP;
        }
        return result;
    }

    // Replace any remaining BOTTOM (unrefined) with TOP so consumers never see
    // the empty sentinel for a reachable value.
    for v in result.values_mut() {
        if v.is_bottom() {
            *v = TOP;
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn join_widen_basic() {
        assert_eq!(
            join(constant(1), constant(3)),
            Interval {
                lo: Some(1),
                hi: Some(3)
            }
        );
        // widening a moving-up bound sends it to +inf.
        let old = Interval {
            lo: Some(0),
            hi: Some(4),
        };
        let new = Interval {
            lo: Some(0),
            hi: Some(8),
        };
        assert_eq!(
            widen(old, new),
            Interval {
                lo: Some(0),
                hi: None
            }
        );
    }

    #[test]
    fn add_sub_intervals() {
        let a = Interval {
            lo: Some(1),
            hi: Some(3),
        };
        let b = Interval {
            lo: Some(10),
            hi: Some(20),
        };
        assert_eq!(
            add(a, b),
            Interval {
                lo: Some(11),
                hi: Some(23)
            }
        );
        assert_eq!(
            sub(b, a),
            Interval {
                lo: Some(7),
                hi: Some(19)
            }
        );
    }

    #[test]
    fn intersect_guard() {
        // value < 10 (true) → [-inf, 9].
        assert_eq!(
            guard_interval(BinOp::Lt, 10, false),
            Some(Interval {
                lo: None,
                hi: Some(9)
            })
        );
        // negated `< 10` → `>= 10` → [10, +inf].
        assert_eq!(
            guard_interval(BinOp::Lt, 10, true),
            Some(Interval {
                lo: Some(10),
                hi: None
            })
        );
    }

    fn iv(lo: i64, hi: i64) -> Interval {
        Interval {
            lo: Some(lo),
            hi: Some(hi),
        }
    }

    #[test]
    fn top_bottom_predicates() {
        assert!(TOP.is_top());
        assert!(!TOP.is_bottom());
        assert!(BOTTOM.is_bottom());
        assert!(!BOTTOM.is_top());
        assert!(!constant(5).is_top());
        assert!(!constant(5).is_bottom());
        // lo > hi is bottom.
        assert!(iv(3, 1).is_bottom());
    }

    #[test]
    fn join_absorbs_bottom_and_widens_to_inf() {
        // BOTTOM is the identity for join.
        assert_eq!(join(BOTTOM, constant(4)), constant(4));
        assert_eq!(join(constant(4), BOTTOM), constant(4));
        // A None (±inf) bound dominates.
        assert_eq!(join(TOP, constant(4)), TOP);
        assert_eq!(
            join(
                iv(0, 5),
                Interval {
                    lo: None,
                    hi: Some(2)
                }
            ),
            Interval {
                lo: None,
                hi: Some(5)
            }
        );
    }

    #[test]
    fn widen_bottom_and_downward_bound() {
        assert_eq!(widen(BOTTOM, constant(3)), constant(3));
        assert_eq!(widen(constant(3), BOTTOM), constant(3));
        // A lower bound moving down jumps to -inf.
        assert_eq!(
            widen(iv(5, 10), iv(2, 10)),
            Interval {
                lo: None,
                hi: Some(10)
            }
        );
    }

    #[test]
    fn arithmetic_bottom_overflow_and_inf() {
        // BOTTOM propagates through add/sub/mul/negate.
        assert_eq!(add(BOTTOM, constant(1)), BOTTOM);
        assert_eq!(sub(constant(1), BOTTOM), BOTTOM);
        assert_eq!(mul(BOTTOM, constant(1)), BOTTOM);
        assert_eq!(negate(BOTTOM), BOTTOM);
        // mul of fully-bounded intervals takes the corner min/max:
        // [2,3]*[4,5] = [8,15] (corners 8,10,12,15) — tclsh: expr {2*4}=8, {3*5}=15.
        assert_eq!(mul(iv(2, 3), iv(4, 5)), iv(8, 15));
        // mul with a negative interval: [-2,3]*[4,5] → corners -10,-8,12,15 → [-10,15].
        assert_eq!(mul(iv(-2, 3), iv(4, 5)), iv(-10, 15));
        // An unbounded operand → TOP (sound).
        assert_eq!(mul(TOP, iv(1, 2)), TOP);
        // Overflow on add → None (±inf) bound, not a wrap.
        assert_eq!(
            add(constant(i64::MAX), constant(1)),
            Interval { lo: None, hi: None }
        );
        // negate flips the bounds: -[2,5] = [-5,-2].
        assert_eq!(negate(iv(2, 5)), iv(-5, -2));
    }

    #[test]
    fn intersect_uses_none_as_identity() {
        // [0,10] ∩ [5,20] = [5,10].
        assert_eq!(intersect(iv(0, 10), iv(5, 20)), iv(5, 10));
        // None is the identity (not absorbing) in intersect.
        assert_eq!(
            intersect(
                iv(0, 10),
                Interval {
                    lo: None,
                    hi: Some(7)
                }
            ),
            iv(0, 7)
        );
        assert_eq!(intersect(BOTTOM, iv(0, 10)), BOTTOM);
    }

    /// A `Literal` node for `t`, read under `numbers`.
    fn lit_at(t: &str, numbers: NumberSyntax) -> Option<i64> {
        literal_int(
            &ExprNode::Literal {
                text: t.to_string(),
                start: 0,
                end: 0,
            },
            numbers,
        )
    }

    #[test]
    fn literal_int_parses_radix_and_bool_keywords() {
        // Verified against tclsh9.0: 0xff=255, 0o17=15, 0b101=5,
        // true/yes/on=1, false/no/off=0, 42=42, -7=-7.
        let lit = |t: &str| lit_at(t, NumberSyntax::Tcl90);
        assert_eq!(lit("0xff"), Some(255));
        assert_eq!(lit("0xFF"), Some(255));
        assert_eq!(lit("0o17"), Some(15));
        assert_eq!(lit("0b101"), Some(5));
        assert_eq!(lit("42"), Some(42));
        assert_eq!(lit("-7"), Some(-7));
        assert_eq!(lit("true"), Some(1));
        assert_eq!(lit("yes"), Some(1));
        assert_eq!(lit("on"), Some(1));
        assert_eq!(lit("false"), Some(0));
        assert_eq!(lit("no"), Some(0));
        assert_eq!(lit("off"), Some(0));
        assert_eq!(lit("notanumber"), None);
        // A non-literal node yields None.
        assert_eq!(
            literal_int(
                &ExprNode::Var {
                    text: "$x".into(),
                    name: "x".into(),
                    start: 0,
                    end: 0
                },
                NumberSyntax::Tcl90
            ),
            None
        );
        // parse_whole_int / const_int_from_value directly.
        assert_eq!(parse_whole_int("0b1111", NumberSyntax::Tcl90), Some(15));
        assert_eq!(parse_whole_int("zzz", NumberSyntax::Tcl90), None);
        assert_eq!(const_int_from_value(" 9 "), Some(9));
        assert_eq!(const_int_from_value("0xff"), None); // const value is canonical decimal only
    }

    /// The numeral grammar is the *target release's*, threaded explicitly.
    ///
    /// A bare leading zero is octal up to 8.6 and decimal from 9.0
    /// (`tcl9.0.3` defines `KILL_OCTAL`), which is the one case where a
    /// version-blind read is *wrong* rather than merely imprecise: tclsh8.6
    /// `expr {0755}` → 493, tclsh9.0 `expr {0755}` → 755.
    #[test]
    fn leading_zero_literal_is_octal_before_9_0() {
        for syntax in [NumberSyntax::Tcl84, NumberSyntax::Tcl85] {
            assert_eq!(lit_at("0755", syntax), Some(493), "{syntax:?}");
            assert_eq!(lit_at("010", syntax), Some(8), "{syntax:?}");
            // An invalid octal digit stops C's scan, so the whole word is not
            // a numeral: tclsh8.6 `expr {08}` → "invalid octal number".
            assert_eq!(lit_at("08", syntax), None, "{syntax:?}");
        }
        assert_eq!(lit_at("0755", NumberSyntax::Tcl90), Some(755));
        assert_eq!(lit_at("010", NumberSyntax::Tcl90), Some(10));
        assert_eq!(lit_at("08", NumberSyntax::Tcl90), Some(8));
        // A point interval built from the literal, end to end.
        let env: HashMap<String, Interval> = HashMap::new();
        assert_eq!(
            eval_expr(&pexpr("0755"), &env, NumberSyntax::Tcl85),
            constant(493)
        );
        assert_eq!(
            eval_expr(&pexpr("0755"), &env, NumberSyntax::Tcl90),
            constant(755)
        );
    }

    /// Radix prefixes appear in the release that added them, `_` separators in
    /// 9.0, and a radix-invalid digit is never a numeral in any release.
    #[test]
    fn radix_prefixes_and_separators_follow_the_release() {
        // `0x` is universal.
        for syntax in [
            NumberSyntax::Tcl84,
            NumberSyntax::Tcl85,
            NumberSyntax::Tcl90,
        ] {
            assert_eq!(lit_at("0x1f", syntax), Some(31), "{syntax:?}");
            // Radix-invalid digits are barewords everywhere.
            assert_eq!(lit_at("0o8", syntax), None, "{syntax:?}");
            assert_eq!(lit_at("0xZZ", syntax), None, "{syntax:?}");
        }
        // `0o` / `0b`: 8.5 onward.
        assert_eq!(lit_at("0o17", NumberSyntax::Tcl84), None);
        assert_eq!(lit_at("0b101", NumberSyntax::Tcl84), None);
        for syntax in [NumberSyntax::Tcl85, NumberSyntax::Tcl90] {
            assert_eq!(lit_at("0o17", syntax), Some(15), "{syntax:?}");
            assert_eq!(lit_at("0b101", syntax), Some(5), "{syntax:?}");
        }
        // `0d` and `_`: 9.0 only.
        for syntax in [NumberSyntax::Tcl84, NumberSyntax::Tcl85] {
            assert_eq!(lit_at("0d99", syntax), None, "{syntax:?}");
            assert_eq!(lit_at("1_000", syntax), None, "{syntax:?}");
        }
        assert_eq!(lit_at("0d99", NumberSyntax::Tcl90), Some(99));
        assert_eq!(lit_at("1_000", NumberSyntax::Tcl90), Some(1000));
        // A signed radix literal (the old prefix-keyed reader missed these).
        assert_eq!(lit_at("-0x10", NumberSyntax::Tcl90), Some(-16));
        // Non-integral and past-a-wide literals give no bound.
        assert_eq!(lit_at("1.5", NumberSyntax::Tcl90), None);
        assert_eq!(lit_at("9223372036854775808", NumberSyntax::Tcl90), None);
    }

    /// The `AssignConst` / `Incr` value reader takes only spellings every
    /// release reads identically, so it needs no dialect.
    #[test]
    fn const_int_from_value_is_canonical_decimal_only() {
        assert_eq!(const_int_from_value("42"), Some(42));
        assert_eq!(const_int_from_value("-7"), Some(-7));
        assert_eq!(const_int_from_value("+5"), Some(5));
        assert_eq!(const_int_from_value("0"), Some(0));
        // Dialect-dependent or non-decimal spellings abstain (→ TOP, sound):
        // `0755` is 493 up to 8.6 and 755 from 9.0, so no single answer is
        // safe here — the same abstention `sccp::parse_literal_value` makes.
        for text in ["0755", "08", "00", "0xff", "0o17", "1_000", "0d99", "1.5"] {
            assert_eq!(const_int_from_value(text), None, "{text}");
        }
    }

    fn pexpr(src: &str) -> ExprNode {
        tcl_syntax::expr::parser::parse_expr(src, None)
    }

    /// Regression coverage for issue #996: `eval_expr` recurses once per
    /// `ExprNode` level with no depth cap before this fix. A tree built
    /// directly is unbounded (the Pratt parser caps its own output at 256)
    /// and empirically overflowed the native stack (SIGABRT) in the low
    /// thousands of levels on a 2 MiB thread. 3000 is past that crash range
    /// and past `MAX_EXPR_NODE_DEPTH` (256); the assertion is that it returns
    /// (the value past the cap is a sound `TOP`).
    #[test]
    fn deeply_nested_eval_expr_survives() {
        let env: HashMap<String, Interval> = HashMap::new();
        let mut node = pexpr("1");
        for _ in 0..3000 {
            node = ExprNode::Unary {
                op: UnaryOp::Neg,
                operand: Box::new(node),
            };
        }
        let _ = eval_expr(&node, &env, NumberSyntax::Tcl90);
    }

    #[test]
    fn eval_expr_abstract_semantics() {
        let mut env: HashMap<String, Interval> = HashMap::new();
        env.insert("x".to_string(), iv(2, 4));
        let ev = |src: &str| eval_expr(&pexpr(src), &env, NumberSyntax::Tcl90);
        // literal → point interval.
        assert_eq!(ev("5"), constant(5));
        // var in env → its interval; unknown var → TOP.
        assert_eq!(ev("$x"), iv(2, 4));
        assert_eq!(ev("$unknown"), TOP);
        // unary neg / pos.
        assert_eq!(ev("-$x"), iv(-4, -2));
        assert_eq!(ev("+$x"), iv(2, 4));
        // binary add/sub/mul over the env.
        assert_eq!(ev("$x + 1"), iv(3, 5));
        assert_eq!(ev("$x - 1"), iv(1, 3));
        assert_eq!(ev("$x * 2"), iv(4, 8));
        // comparison → boolean [0,1].
        assert_eq!(ev("$x < 3"), iv(0, 1));
        // division is unmodelled → TOP (sound).
        assert_eq!(ev("$x / 2"), TOP);
    }

    #[test]
    fn guard_constraint_both_operand_orders() {
        let gc = |src: &str, negate: bool| {
            guard_constraint(&pexpr(src), "x", negate, NumberSyntax::Tcl90)
        };
        // `$x < 5` true → x ∈ [-inf, 4] (tclsh: 4<5=1, 5<5=0).
        assert_eq!(
            gc("$x < 5", false),
            Some(Interval {
                lo: None,
                hi: Some(4)
            })
        );
        // mirrored: `5 < $x` true → x ∈ [6, +inf].
        assert_eq!(
            gc("5 < $x", false),
            Some(Interval {
                lo: Some(6),
                hi: None
            })
        );
        // `$x == 7` → [7,7]; negated → Ne → None.
        assert_eq!(gc("$x == 7", false), Some(constant(7)));
        assert_eq!(gc("$x == 7", true), None);
        // not a comparison against this name → None.
        assert_eq!(gc("$y < 5", false), None);
        assert_eq!(gc("$x + 1", false), None);
    }

    #[test]
    fn guard_interval_all_ops_and_negation() {
        assert_eq!(
            guard_interval(BinOp::Le, 5, false),
            Some(Interval {
                lo: None,
                hi: Some(5)
            })
        );
        assert_eq!(
            guard_interval(BinOp::Gt, 5, false),
            Some(Interval {
                lo: Some(6),
                hi: None
            })
        );
        assert_eq!(
            guard_interval(BinOp::Ge, 5, false),
            Some(Interval {
                lo: Some(5),
                hi: None
            })
        );
        assert_eq!(guard_interval(BinOp::Eq, 5, false), Some(constant(5)));
        // Ne yields no single interval.
        assert_eq!(guard_interval(BinOp::Ne, 5, false), None);
        // Negations flip Le↔Gt, Ge↔Lt, Eq↔Ne.
        assert_eq!(
            guard_interval(BinOp::Le, 5, true),
            Some(Interval {
                lo: Some(6),
                hi: None
            })
        );
        assert_eq!(
            guard_interval(BinOp::Ge, 5, true),
            Some(Interval {
                lo: None,
                hi: Some(4)
            })
        );
    }

    /// The false-edge inversion now comes from the owner table
    /// (`BinOp::inverse`) instead of a local copy, and this pins the whole
    /// negated row set — including the NaN-affected ordered four, whose
    /// narrowing this domain deliberately keeps (issue #1437; see
    /// [`guard_interval`]'s "NaN and the false edge").
    #[test]
    fn guard_interval_negation_matches_the_owner_inverse_table() {
        let neg = |op| guard_interval(op, 5, true);
        let half_from = |lo| {
            Some(Interval {
                lo: Some(lo),
                hi: None,
            })
        };
        let half_to = |hi| {
            Some(Interval {
                lo: None,
                hi: Some(hi),
            })
        };
        // The ordered four invert to each other: `!(x < 5)` narrows to x >= 5.
        assert_eq!(neg(BinOp::Lt), half_from(5));
        assert_eq!(neg(BinOp::Le), half_from(6));
        assert_eq!(neg(BinOp::Gt), half_to(5));
        assert_eq!(neg(BinOp::Ge), half_to(4));
        // `!=` is the exact complement of `==` for every value, NaN included,
        // so the false edge of `if {$x != 5}` really is the point [5,5].
        assert_eq!(neg(BinOp::Ne), Some(constant(5)));
        // `==` inverts to `!=`, which is no single interval.
        assert_eq!(neg(BinOp::Eq), None);
        // Operators the owner table inverts but this domain does not model
        // still yield nothing, exactly as the local pass-through table did.
        for op in [BinOp::StrEq, BinOp::StrNe, BinOp::In, BinOp::Ni] {
            assert_eq!(neg(op), None, "{op:?} is not an integer-domain guard");
        }
        // And an operator with no inverse at all is left alone.
        assert_eq!(neg(BinOp::Add), None);
    }

    #[test]
    fn guard_interval_saturates_at_i64_boundary() {
        // `value < i64::MIN` / `value > i64::MAX` would overflow an unchecked
        // `k ± 1`; saturation keeps a sound (empty-ish, one-value-wide) bound
        // instead of panicking (issue 147).
        assert_eq!(
            guard_interval(BinOp::Lt, i64::MIN, false),
            Some(Interval {
                lo: None,
                hi: Some(i64::MIN)
            })
        );
        assert_eq!(
            guard_interval(BinOp::Gt, i64::MAX, false),
            Some(Interval {
                lo: Some(i64::MAX),
                hi: None
            })
        );
        // Negated forms route through the same arithmetic (`>=` neg → `<`).
        assert_eq!(
            guard_interval(BinOp::Ge, i64::MIN, true),
            Some(Interval {
                lo: None,
                hi: Some(i64::MIN)
            })
        );
    }

    #[test]
    fn compute_intervals_end_to_end() {
        use crate::compilation_unit::CompilationUnit;
        use tcl_registry::cache::registry_for_dialect;
        let registry = registry_for_dialect("tcl8.6");
        // A constant assignment gives a point interval the analysis can infer.
        let cu = CompilationUnit::build_for(
            "proc f {} { set x 5\n incr x\n return $x }",
            registry,
            false,
        );
        let fu = cu.procedures.get("::f").expect("proc");
        let intervals = compute_intervals_with(
            &fu.cfg,
            &fu.ssa,
            &fu.sccp.values,
            numbers_for_dialect(Some("tcl8.6")),
        );
        // The analysis runs and produces a (possibly empty but well-formed) map;
        // every computed interval is a valid lattice element (not the lo>hi
        // sentinel unless explicitly bottom).
        for &i in intervals.values() {
            assert!(
                !i.is_bottom() || i == BOTTOM,
                "computed interval must be a sound lattice element: {i:?}"
            );
        }
    }
}
