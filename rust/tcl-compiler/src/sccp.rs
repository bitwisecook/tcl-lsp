//! Sparse Conditional Constant Propagation (SCCP).
//!
//! Classic SCCP lattice-based constant propagation: iteratively
//! refine per-SSA-value [`LatticeValue`] facts until a fixed point,
//! using CFG reachability so unreachable branches never drag their
//! targets down to `Overdefined`.
//!
//! This is the first strip of C25:
//! - **C25a** (this file) — predecessor / CFG-order wrappers and the
//!   [`join`] helper for lattice updates.
//! - **C25b** — the `sccp` fixed-point driver that consumes these
//!   helpers plus [`SsaFunction`] phi nodes and branch terminators.
//! - **C25c** and **C25d** add the dataflow-graph extraction that
//!   renders SCCP facts for consumers.
//!
//! Ported from `core/compiler/core_analyses.py::_sccp` and the
//! surrounding lattice-join helpers.

use std::collections::{HashMap, HashSet};

use crate::analyses::{ConstValue, LatticeValue, MAX_CONSTSET_SIZE};
use crate::cfg::Function as CfgFunction;
use crate::ssa::ValueKey;

// ---------------------------------------------------------------------------
// Public aliases (C25a)
// ---------------------------------------------------------------------------

/// Predecessor map: block name → set of block names that branch
/// into it. Thin wrapper around [`CfgFunction::predecessors`] kept
/// in this module so callers can reach it without reaching into
/// the CFG type directly.
#[must_use]
pub fn compute_predecessors(cfg: &CfgFunction) -> HashMap<String, HashSet<String>> {
    cfg.predecessors()
}

/// CFG traversal order used by SCCP — reverse post-order from the
/// entry block. Blocks that the RPO walk cannot reach from `entry`
/// are appended at the end so the driver can still observe them
/// (matching the Python `_cfg_order` behaviour).
#[must_use]
pub fn cfg_order(cfg: &CfgFunction) -> Vec<String> {
    let mut order = cfg.reverse_postorder();
    let seen: HashSet<String> = order.iter().cloned().collect();
    for name in cfg.blocks.keys() {
        if !seen.contains(name) {
            order.push(name.clone());
        }
    }
    order
}

// ---------------------------------------------------------------------------
// Lattice join (C25a)
// ---------------------------------------------------------------------------

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
/// [`MAX_CONSTSET_SIZE`]. Matches `core_analyses.py::_join`:
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cfg::{Block, Function, Terminator};
    use crate::expr_ast::ExprNode;

    fn goto(target: &str) -> Terminator {
        Terminator::Goto {
            target: target.into(),
            span: None,
        }
    }

    fn branch(cond: ExprNode, tt: &str, ft: &str) -> Terminator {
        Terminator::Branch {
            condition: cond,
            true_target: tt.into(),
            false_target: ft.into(),
            span: None,
        }
    }

    fn literal(text: &str) -> ExprNode {
        ExprNode::Literal {
            text: text.into(),
            start: 0,
            end: u32::try_from(text.len()).unwrap_or(0),
        }
    }

    // -- join --

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
        let key: ValueKey = ("x".into(), 1);
        assert!(set_value(
            &mut values,
            key.clone(),
            &LatticeValue::Const(ConstValue::Int(1))
        ));
        assert!(!set_value(
            &mut values,
            key.clone(),
            &LatticeValue::Const(ConstValue::Int(1))
        ));
        assert!(set_value(
            &mut values,
            key,
            &LatticeValue::Const(ConstValue::Int(2))
        ));
    }

    // -- predecessors + cfg_order --

    #[test]
    fn predecessors_simple_chain() {
        let mut f = Function::new("::top", "a");
        f.blocks.insert("b".into(), Block::new("b"));
        f.blocks.get_mut("a").unwrap().terminator = Some(goto("b"));
        f.blocks.get_mut("b").unwrap().terminator = Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        });
        let p = compute_predecessors(&f);
        assert!(p.get("b").unwrap().contains("a"));
        assert!(p.get("a").is_none_or(HashSet::is_empty));
    }

    #[test]
    fn cfg_order_starts_at_entry() {
        let mut f = Function::new("::top", "entry");
        f.blocks.insert("t".into(), Block::new("t"));
        f.blocks.insert("e".into(), Block::new("e"));
        f.blocks.insert("join".into(), Block::new("join"));
        f.blocks.get_mut("entry").unwrap().terminator =
            Some(branch(literal("1"), "t", "e"));
        f.blocks.get_mut("t").unwrap().terminator = Some(goto("join"));
        f.blocks.get_mut("e").unwrap().terminator = Some(goto("join"));
        f.blocks.get_mut("join").unwrap().terminator = Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        });
        let order = cfg_order(&f);
        assert_eq!(order[0], "entry");
        // join must appear after both branches.
        let join_pos = order.iter().position(|b| b == "join").unwrap();
        let t_pos = order.iter().position(|b| b == "t").unwrap();
        let e_pos = order.iter().position(|b| b == "e").unwrap();
        assert!(join_pos > t_pos);
        assert!(join_pos > e_pos);
    }

    #[test]
    fn cfg_order_appends_unreachable_blocks() {
        let mut f = Function::new("::top", "entry");
        f.blocks.insert("dead".into(), Block::new("dead"));
        f.blocks.get_mut("entry").unwrap().terminator = Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        });
        f.blocks.get_mut("dead").unwrap().terminator = Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        });
        let order = cfg_order(&f);
        assert!(order.contains(&"entry".to_string()));
        assert!(order.contains(&"dead".to_string()));
    }
}
