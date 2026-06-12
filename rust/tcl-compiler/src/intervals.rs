//! Integer interval abstract domain over SSA values (Phase 3).
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
//! Port of `compiler/intervals.py`.  Soundness (never claim a tighter range than
//! reality) matters far more than precision, because the only consumers turn a
//! *proven* fact (index in range) into a diagnostic decision.

use std::collections::{HashMap, HashSet};

use tcl_syntax::expr::ast::{BinOp, ExprNode, UnaryOp};

use crate::analyses::{ConstValue, LatticeValue};
use crate::cfg::{Function as CfgFunction, Terminator};
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

/// The integer value of an `ExprNode::Literal` (incl. bool keyword), else
/// `None`.  Mirrors `_literal_int`.
#[must_use]
fn literal_int(node: &ExprNode) -> Option<i64> {
    let ExprNode::Literal { text, .. } = node else {
        return None;
    };
    let t = text.trim();
    match t {
        "true" | "yes" | "on" => return Some(1),
        "false" | "no" | "off" => return Some(0),
        _ => {}
    }
    parse_radix_int(t)
}

/// Parse a literal int the way Python's `int(t, 0) if t[:2] in {0x,0o,0b} else
/// int(t)` does: a `0x`/`0o`/`0b` prefix selects the radix (positive only,
/// matching the `t[:2]` prefix test), otherwise plain decimal (signed).
#[must_use]
fn parse_radix_int(t: &str) -> Option<i64> {
    let prefix = t.get(..2).map(str::to_ascii_lowercase);
    match prefix.as_deref() {
        Some("0x") => i64::from_str_radix(&t[2..], 16).ok(),
        Some("0o") => i64::from_str_radix(&t[2..], 8).ok(),
        Some("0b") => i64::from_str_radix(&t[2..], 2).ok(),
        _ => t.parse::<i64>().ok(),
    }
}

/// Parse a literal IR value word as an int, if it is exactly one.  Mirrors
/// `_const_int_from_value` (plain `int(s)`, no radix prefixes).
#[must_use]
fn const_int_from_value(text: &str) -> Option<i64> {
    text.trim().parse::<i64>().ok()
}

/// Abstract-evaluate `expr` over the current interval environment.  Mirrors
/// `_eval_expr`.
#[must_use]
fn eval_expr(expr: &ExprNode, env: &HashMap<String, Interval>) -> Interval {
    match expr {
        ExprNode::Literal { .. } => literal_int(expr).map_or(TOP, constant),
        ExprNode::Var { name, .. } => env.get(name).copied().unwrap_or(TOP),
        ExprNode::Unary { op, operand } => {
            let inner = eval_expr(operand, env);
            match op {
                UnaryOp::Neg => negate(inner),
                UnaryOp::Pos => inner,
                _ => TOP,
            }
        }
        ExprNode::Binary { op, left, right } => {
            let la = eval_expr(left, env);
            let ra = eval_expr(right, env);
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
/// order/equality comparison.  Mirrors `_guard_interval`.
#[must_use]
fn guard_interval(op: BinOp, k: i64, negate: bool) -> Option<Interval> {
    let op = if negate {
        match op {
            BinOp::Lt => BinOp::Ge,
            BinOp::Le => BinOp::Gt,
            BinOp::Gt => BinOp::Le,
            BinOp::Ge => BinOp::Lt,
            BinOp::Eq => BinOp::Ne,
            BinOp::Ne => BinOp::Eq,
            other => other,
        }
    } else {
        op
    };
    match op {
        BinOp::Lt => Some(Interval {
            lo: None,
            hi: Some(k - 1),
        }),
        BinOp::Le => Some(Interval {
            lo: None,
            hi: Some(k),
        }),
        BinOp::Gt => Some(Interval {
            lo: Some(k + 1),
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
/// Only a top-level comparison against a literal int is handled.  Mirrors
/// `_guard_constraint`.
#[must_use]
fn guard_constraint(cond: &ExprNode, name: &str, negate: bool) -> Option<Interval> {
    let ExprNode::Binary { op, left, right } = cond else {
        return None;
    };
    // $name <op> K
    if let ExprNode::Var { name: vn, .. } = left.as_ref() {
        if vn == name {
            if let Some(k) = literal_int(right) {
                return guard_interval(*op, k, negate);
            }
        }
    }
    // K <op> $name  → rewrite as $name <flipped-op> K
    if let ExprNode::Var { name: vn, .. } = right.as_ref() {
        if vn == name {
            if let Some(k) = literal_int(left) {
                let mirror = match op {
                    BinOp::Lt => BinOp::Gt,
                    BinOp::Le => BinOp::Ge,
                    BinOp::Gt => BinOp::Lt,
                    BinOp::Ge => BinOp::Le,
                    other => *other,
                };
                return guard_interval(mirror, k, negate);
            }
        }
    }
    None
}

/// `(name, version) → [branch-block names]` index for guard narrowing.  Mirrors
/// `build_guard_index`: for each `Branch` block, the names in its condition
/// against the block's exit version of each.
#[must_use]
#[allow(clippy::implicit_hasher)]
pub fn build_guard_index(cfg: &CfgFunction, ssa: &SsaFunction) -> HashMap<ValueKey, Vec<String>> {
    let mut index: HashMap<ValueKey, Vec<String>> = HashMap::new();
    for (dn, dblock) in &cfg.blocks {
        let Some(Terminator::Branch { condition, .. }) = &dblock.terminator else {
            continue;
        };
        let Some(sb) = ssa.blocks.get(dn) else {
            continue;
        };
        for name in condition.vars() {
            if let Some(&version) = sb.exit_versions.get(&name) {
                index.entry((name, version)).or_default().push(dn.clone());
            }
        }
    }
    index
}

/// True when `ancestor` dominates `node` (walks `node`'s idom chain).
#[must_use]
fn dominates(ssa: &SsaFunction, ancestor: &str, node: &str) -> bool {
    if ancestor == node {
        return true;
    }
    let mut curr = node.to_owned();
    loop {
        match ssa.idom.get(&curr) {
            Some(Some(parent)) => {
                if parent == ancestor {
                    return true;
                }
                curr = parent.clone();
            }
            _ => return false,
        }
    }
}

/// Narrow `base[(name, version)]` by the constant-bound guards that hold on
/// every path reaching `block`.  Mirrors `refine_interval`.
#[must_use]
#[allow(clippy::implicit_hasher)]
pub fn refine_interval(
    base: &HashMap<ValueKey, Interval>,
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    block: &str,
    name: &str,
    version: Version,
    guard_index: &HashMap<ValueKey, Vec<String>>,
) -> Interval {
    let mut iv = base
        .get(&(name.to_owned(), version))
        .copied()
        .unwrap_or(TOP);
    let Some(candidate_blocks) = guard_index.get(&(name.to_owned(), version)) else {
        return iv;
    };
    for dn in candidate_blocks {
        if dn == block {
            continue;
        }
        let Some(dblock) = cfg.blocks.get(dn) else {
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
        let Some(sb) = ssa.blocks.get(dn) else {
            continue;
        };
        if sb.exit_versions.get(name) != Some(&version) {
            continue;
        }
        let true_dom = dominates(ssa, true_target.as_str(), block);
        let false_dom = dominates(ssa, false_target.as_str(), block);
        let c = if true_dom && !false_dom {
            guard_constraint(condition, name, false)
        } else if false_dom && !true_dom {
            guard_constraint(condition, name, true)
        } else {
            continue;
        };
        if let Some(c) = c {
            iv = intersect(iv, c);
        }
    }
    iv
}

/// Seed a `[c, c]` interval from a constant-integer SCCP value, else `None`.
#[must_use]
fn seed_const(key: &ValueKey, values: &HashMap<ValueKey, LatticeValue>) -> Option<Interval> {
    match values.get(key) {
        Some(LatticeValue::Const(ConstValue::Int(n))) => Some(constant(*n)),
        Some(LatticeValue::Const(ConstValue::Bool(b))) => Some(constant(i64::from(*b))),
        _ => None,
    }
}

/// Interval produced by `stmt` for its def `name`.  Mirrors `_transfer`.
#[must_use]
fn transfer(
    stmt: &crate::ir::Statement,
    name: &str,
    env: &HashMap<String, Interval>,
    old: Interval,
) -> Interval {
    use crate::ir::Statement;
    match stmt {
        Statement::AssignConst { value, .. } => const_int_from_value(value).map_or(TOP, constant),
        Statement::AssignExpr { expr, .. } => eval_expr(expr, env),
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
fn loop_headers(cfg: &CfgFunction, ssa: &SsaFunction) -> HashSet<String> {
    let mut headers = HashSet::new();
    for (u, block) in &cfg.blocks {
        let Some(term) = &block.terminator else {
            continue;
        };
        for v in term.successors() {
            if dominates(ssa, v, u) {
                headers.insert(v.to_owned());
            }
        }
    }
    headers
}

/// Forward interval analysis over the SSA form.  `values` is the SCCP lattice;
/// a constant integer seeds `[c, c]`.  Everything else starts `BOTTOM` and is
/// refined to fixpoint with loop-header widening.  Mirrors `compute_intervals`.
#[must_use]
#[allow(clippy::implicit_hasher)]
pub fn compute_intervals(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    values: &HashMap<ValueKey, LatticeValue>,
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
                let key = (phi.name.clone(), phi.version);
                let mut merged = BOTTOM;
                for &inc in phi.incoming.values() {
                    merged = if inc > 0 {
                        join(merged, cur(&result, &(phi.name.clone(), inc)))
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
                let env: HashMap<String, Interval> = s
                    .uses
                    .iter()
                    .map(|(nm, &ver)| (nm.clone(), cur(&result, &(nm.clone(), ver))))
                    .collect();
                for (nm, &ver) in &s.defs {
                    let key = (nm.clone(), ver);
                    let val = seed_const(&key, values)
                        .unwrap_or_else(|| transfer(&s.statement, nm, &env, cur(&result, &key)));
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
}
