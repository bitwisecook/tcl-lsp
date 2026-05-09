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

#![allow(clippy::implicit_hasher)]

use std::collections::{HashMap, HashSet};

use crate::analyses::{ConstValue, LatticeValue, MAX_CONSTSET_SIZE};
use crate::cfg::{Function as CfgFunction, Terminator};
use crate::expr_ast::ExprNode;
use crate::ir::Statement;
use crate::ssa::{SsaFunction, SsaStatement, ValueKey};
use crate::tcl_expr_eval::{eval_tcl_expr, Env, EnvValue, TclValue};

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

// ---------------------------------------------------------------------------
// Driver (C25b)
// ---------------------------------------------------------------------------

/// A branch whose condition SCCP determined to be constant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstantBranch {
    /// CFG block containing the branch.
    pub block: String,
    /// Source span of the branch terminator (its condition
    /// expression when known). Used by diagnostic aggregators to
    /// point editors and CLIs at the triggering site.
    pub span: Option<tcl_lexer::Span>,
    /// Condition text for diagnostic reporting.
    pub condition: String,
    /// Evaluated boolean value.
    pub value: bool,
    /// Target reached when the condition holds.
    pub taken_target: String,
    /// Target skipped.
    pub not_taken_target: String,
}

/// Full SCCP result: per-SSA-value lattice entries, the set of
/// reachable blocks, the set of reachable edges, and
/// constant-folded branch annotations for reachable blocks.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SccpResult {
    /// Per-SSA-value lattice entry.
    pub values: HashMap<ValueKey, LatticeValue>,
    /// Blocks reachable from `cfg.entry` under current assumptions.
    pub executable_blocks: HashSet<String>,
    /// `(from_block, to_block)` edges known executable.
    pub executable_edges: HashSet<(String, String)>,
    /// Constant branches detected during propagation.
    pub constant_branches: Vec<ConstantBranch>,
}

/// Sparse Conditional Constant Propagation driver.
///
/// Iterates to a fixed point over the lattice values of every SSA
/// value, using CFG reachability (via `executable_edges`) so that
/// unreachable branches don't widen their targets. `param_constants`
/// lets interprocedural analysis seed the caller-provided argument
/// lattice entries.
///
/// This is a focused port of `core_analyses.py::_sccp`:
///
/// - Phi handling uses the incoming versions for each *executable*
///   predecessor, joining them onto the phi's SSA value.
/// - Statement handling uses [`evaluate_def`] below, which folds
///   [`Statement::AssignConst`] and [`Statement::AssignExpr`] via
///   the C22 evaluator. Other statement kinds and
///   [`Statement::Barrier`] widen their defs to `Overdefined`.
/// - Branch decisions are resolved via [`evaluate_branch`] below,
///   which consults the lattice environment and then the C22
///   evaluator.
#[must_use]
pub fn sccp(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    param_constants: Option<&HashMap<ValueKey, LatticeValue>>,
) -> SccpResult {
    let preds = compute_predecessors(cfg);
    let mut values: HashMap<ValueKey, LatticeValue> = HashMap::new();
    if let Some(seed) = param_constants {
        for (k, v) in seed {
            values.insert(k.clone(), v.clone());
        }
    }
    let mut executable_blocks: HashSet<String> = HashSet::new();
    let mut executable_edges: HashSet<(String, String)> = HashSet::new();
    if cfg.blocks.contains_key(&cfg.entry) {
        executable_blocks.insert(cfg.entry.clone());
    }
    let order = cfg_order(cfg);

    let mut changed = true;
    while changed {
        changed = false;
        for bn in &order {
            if !executable_blocks.contains(bn) {
                continue;
            }
            let Some(ssa_block) = ssa.blocks.get(bn) else {
                continue;
            };

            let incoming_exec: Vec<String> = preds
                .get(bn)
                .map(|set| {
                    set.iter()
                        .filter(|p| executable_edges.contains(&((*p).clone(), bn.clone())))
                        .cloned()
                        .collect()
                })
                .unwrap_or_default();

            // Phi nodes (not at entry, only when some predecessor is
            // executable).
            for phi in &ssa_block.phis {
                if bn == &cfg.entry {
                    continue;
                }
                if incoming_exec.is_empty() {
                    continue;
                }
                let mut phi_val = LatticeValue::Unknown;
                for pred in &incoming_exec {
                    let incoming_ver = phi.incoming.get(pred).copied().unwrap_or(0);
                    if incoming_ver == 0 {
                        continue;
                    }
                    let key: ValueKey = (phi.name.clone(), incoming_ver);
                    let candidate = values.get(&key).cloned().unwrap_or(LatticeValue::Unknown);
                    phi_val = join(&phi_val, &candidate);
                }
                if set_value(&mut values, (phi.name.clone(), phi.version), &phi_val) {
                    changed = true;
                }
            }

            // Statements.
            for stmt_ssa in &ssa_block.statements {
                if matches!(stmt_ssa.statement, Statement::Barrier { .. }) {
                    // Barriers widen all currently-tracked values.
                    let keys: Vec<ValueKey> = values.keys().cloned().collect();
                    for k in keys {
                        if set_value(&mut values, k, &LatticeValue::Overdefined) {
                            changed = true;
                        }
                    }
                    continue;
                }
                for (var, ver) in &stmt_ssa.defs {
                    let val = evaluate_def(stmt_ssa, &values);
                    if set_value(&mut values, (var.clone(), *ver), &val) {
                        changed = true;
                    }
                }
            }

            // Terminator.
            if sccp_process_terminator(
                bn,
                cfg,
                ssa,
                &values,
                &mut executable_blocks,
                &mut executable_edges,
            ) {
                changed = true;
            }
        }
    }

    let constant_branches =
        collect_constant_branches(cfg, ssa, &values, &executable_blocks, &order);

    SccpResult {
        values,
        executable_blocks,
        executable_edges,
        constant_branches,
    }
}

/// Process a block's terminator: mark the matching outgoing edges
/// as executable.  Returns `true` when any new edge / block was
/// added.  Extracted from [`sccp`].
fn sccp_process_terminator(
    bn: &str,
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    values: &HashMap<ValueKey, LatticeValue>,
    executable_blocks: &mut HashSet<String>,
    executable_edges: &mut HashSet<(String, String)>,
) -> bool {
    let mut changed = false;
    let Some(block) = cfg.blocks.get(bn) else {
        return false;
    };
    let Some(term) = &block.terminator else {
        return false;
    };
    match term {
        Terminator::Goto { target, .. } => {
            let edge = (bn.to_owned(), target.clone());
            if !executable_edges.contains(&edge) {
                executable_edges.insert(edge);
                changed = true;
            }
            if cfg.blocks.contains_key(target) && executable_blocks.insert(target.clone()) {
                changed = true;
            }
        }
        Terminator::Branch {
            condition,
            true_target,
            false_target,
            ..
        } => {
            let Some(ssa_block) = ssa.blocks.get(bn) else {
                return changed;
            };
            let decision = evaluate_branch(ssa_block, condition, values);
            let targets: Vec<&str> = match decision {
                Some(true) => vec![true_target.as_str()],
                Some(false) => vec![false_target.as_str()],
                None => vec![true_target.as_str(), false_target.as_str()],
            };
            for tgt in targets {
                let edge = (bn.to_owned(), tgt.to_owned());
                if !executable_edges.contains(&edge) {
                    executable_edges.insert(edge);
                    changed = true;
                }
                if cfg.blocks.contains_key(tgt) && executable_blocks.insert(tgt.to_owned()) {
                    changed = true;
                }
            }
        }
        Terminator::Return { .. } => {}
    }
    changed
}

/// Post-fixpoint sweep that records every reachable branch whose
/// condition evaluated to a constant lattice value.  Extracted
/// from [`sccp`].
fn collect_constant_branches(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    values: &HashMap<ValueKey, LatticeValue>,
    executable_blocks: &HashSet<String>,
    order: &[String],
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
            ..
        }) = &block.terminator
        else {
            continue;
        };
        let Some(ssa_block) = ssa.blocks.get(bn) else {
            continue;
        };
        let decision = evaluate_branch(ssa_block, condition, values);
        let cond_text = crate::expr_ast::expr_text(condition);
        match decision {
            Some(true) => constant_branches.push(ConstantBranch {
                block: bn.clone(),
                span: *term_span,
                condition: cond_text,
                value: true,
                taken_target: true_target.clone(),
                not_taken_target: false_target.clone(),
            }),
            Some(false) => constant_branches.push(ConstantBranch {
                block: bn.clone(),
                span: *term_span,
                condition: cond_text,
                value: false,
                taken_target: false_target.clone(),
                not_taken_target: true_target.clone(),
            }),
            None => {}
        }
    }
    constant_branches
}

/// Evaluate the lattice value produced by an SSA statement's
/// defs.
///
/// Focused subset: constant-assignment, expression-assignment via
/// the C22 evaluator, and a conservative `Overdefined` fallback
/// for everything else.
#[must_use]
pub fn evaluate_def(
    stmt_ssa: &SsaStatement,
    values: &HashMap<ValueKey, LatticeValue>,
) -> LatticeValue {
    match &stmt_ssa.statement {
        Statement::AssignConst { value, .. } => LatticeValue::Const(parse_literal_value(value)),
        Statement::AssignExpr { expr, .. } => {
            let env = env_from_uses(&stmt_ssa.uses, values);
            match eval_tcl_expr(expr, &env) {
                Some(v) => LatticeValue::Const(tcl_value_to_const(v)),
                None => LatticeValue::Overdefined,
            }
        }
        Statement::AssignValue { value, .. } => {
            // C25e4: fold when the RHS is either a plain literal
            // (no command substitution), a simple `$var` that
            // resolves to a lattice Const, or a `[cmd args...]`
            // that try_fold_cmd_subst recognises.
            fold_assign_value(value, &stmt_ssa.uses, values)
        }
        Statement::Call {
            command,
            args,
            defs,
            ..
        } if matches!(command.as_str(), "foreach" | "lmap")
            && defs.len() == 1
            && args.len() == 1 =>
        {
            // C25e2: `foreach v LIST` / `lmap v LIST` folds the
            // iteration variable to the CONSTSET of elements when
            // LIST is a literal or resolves to a Const(String)
            // through the lattice. Multi-variable and multi-list
            // foreaches are left as Overdefined.
            let elements = extract_foreach_elements(&args[0])
                .or_else(|| resolve_foreach_list_via_lattice(&args[0], &stmt_ssa.uses, values));
            match elements {
                Some(items) if items.is_empty() => LatticeValue::Overdefined,
                Some(items) => {
                    let consts: Vec<ConstValue> =
                        items.iter().map(|s| parse_literal_value(s)).collect();
                    if consts.len() == 1 {
                        LatticeValue::Const(consts.into_iter().next().unwrap())
                    } else {
                        LatticeValue::constset(consts)
                    }
                }
                None => LatticeValue::Overdefined,
            }
        }
        Statement::Incr { name, amount, .. } => {
            // C25e1: track `incr NAME ?AMOUNT?` through the lattice
            // when the current value of NAME is a single Const(Int)
            // and AMOUNT is either absent (defaults to 1), a decimal
            // integer literal, or a simple `$var` reference that
            // resolves to Const(Int) via `uses`.
            let ver = stmt_ssa.uses.get(name).copied().unwrap_or(0);
            let base = values
                .get(&(name.clone(), ver))
                .cloned()
                .unwrap_or(LatticeValue::Unknown);
            let base_int = match &base {
                LatticeValue::Const(ConstValue::Int(i)) => *i,
                LatticeValue::Unknown => return LatticeValue::Unknown,
                // Overdefined or a non-integer Const widens.
                _ => return LatticeValue::Overdefined,
            };
            let amt = match amount.as_deref() {
                None => 1,
                Some(text) => {
                    let trimmed = text.trim();
                    if let Ok(v) = trimmed.parse::<i64>() {
                        v
                    } else if let Some(amount) =
                        resolve_simple_var_ref(trimmed, &stmt_ssa.uses, values)
                    {
                        match amount {
                            LatticeValue::Const(ConstValue::Int(i)) => i,
                            LatticeValue::Unknown => return LatticeValue::Unknown,
                            _ => return LatticeValue::Overdefined,
                        }
                    } else {
                        return LatticeValue::Overdefined;
                    }
                }
            };
            base_int
                .checked_add(amt)
                .map_or(LatticeValue::Overdefined, |v| {
                    LatticeValue::Const(ConstValue::Int(v))
                })
        }
        _ => LatticeValue::Overdefined,
    }
}

/// Resolve `$var` / `${var}` to a lattice value by looking up the
/// SSA version in `uses` and indexing `values`. Returns None when
/// the text isn't a simple var reference.
fn resolve_simple_var_ref(
    text: &str,
    uses: &HashMap<String, crate::ssa::Version>,
    values: &HashMap<ValueKey, LatticeValue>,
) -> Option<LatticeValue> {
    let name = if let Some(name) = text.strip_prefix("${").and_then(|s| s.strip_suffix('}')) {
        name
    } else if let Some(name) = text.strip_prefix('$') {
        if name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b':')
        {
            name
        } else {
            return None;
        }
    } else {
        return None;
    };
    let ver = *uses.get(name)?;
    Some(
        values
            .get(&(name.to_owned(), ver))
            .cloned()
            .unwrap_or(LatticeValue::Unknown),
    )
}

/// Evaluate a branch condition.
///
/// Returns `Some(true)` / `Some(false)` when the condition folds to
/// a constant under the current lattice; `None` otherwise.
#[must_use]
pub fn evaluate_branch(
    ssa_block: &crate::ssa::SsaBlock,
    condition: &ExprNode,
    values: &HashMap<ValueKey, LatticeValue>,
) -> Option<bool> {
    let env = env_from_uses(&ssa_block.exit_versions, values);
    let v = eval_tcl_expr(condition, &env)?;
    Some(v.is_truthy())
}

/// Build a [`tcl_expr_eval::Env`] from a `{name → version}` map
/// and the current lattice. Only entries whose lattice value is
/// a single [`LatticeValue::Const`] are bound; anything else
/// leaves the variable unbound so the evaluator returns `None`.
fn env_from_uses(
    uses: &HashMap<String, crate::ssa::Version>,
    values: &HashMap<ValueKey, LatticeValue>,
) -> Env {
    let mut env = Env::new();
    for (name, ver) in uses {
        let key: ValueKey = (name.clone(), *ver);
        if let Some(LatticeValue::Const(c)) = values.get(&key) {
            env.insert(name.clone(), const_to_env_value(c));
        }
    }
    env
}

fn const_to_env_value(c: &ConstValue) -> EnvValue {
    match c {
        ConstValue::Int(i) => EnvValue::Int(*i),
        ConstValue::Float(f) => EnvValue::Float(*f),
        ConstValue::Bool(b) => EnvValue::Int(i64::from(*b)),
        ConstValue::String(s) => EnvValue::Str(s.clone()),
    }
}

fn tcl_value_to_const(v: TclValue) -> ConstValue {
    match v {
        TclValue::Int(i) => ConstValue::Int(i),
        TclValue::Float(f) => ConstValue::Float(f),
    }
}

/// Extract iteration-variable elements from a foreach list arg
/// that is a literal (no `$` / `[` substitution).
///
/// Ported from `core_analyses.py::_extract_foreach_elements`:
/// - Strip whitespace, one level of `{…}` or `"…"` wrapping.
/// - Split on ASCII whitespace.
/// - Returns `None` for anything that starts with `$` or `[` so
///   callers fall through to
///   [`resolve_foreach_list_via_lattice`].
#[must_use]
pub fn extract_foreach_elements(list_text: &str) -> Option<Vec<String>> {
    let stripped = list_text.trim();
    if stripped.is_empty() {
        return Some(Vec::new());
    }
    if stripped.starts_with('[') || stripped.starts_with('$') {
        return None;
    }
    let inner = if (stripped.starts_with('{') && stripped.ends_with('}'))
        || (stripped.starts_with('"') && stripped.ends_with('"'))
    {
        &stripped[1..stripped.len() - 1]
    } else {
        stripped
    };
    Some(inner.split_ascii_whitespace().map(str::to_owned).collect())
}

/// Resolve `$var` / `${var}` to a `Vec<String>` of list elements
/// via the SCCP lattice. Returns `None` when the operand is not a
/// simple var reference or its lattice value is not a
/// Const(String).
#[must_use]
pub fn resolve_foreach_list_via_lattice(
    list_text: &str,
    uses: &HashMap<String, crate::ssa::Version>,
    values: &HashMap<ValueKey, LatticeValue>,
) -> Option<Vec<String>> {
    let stripped = list_text.trim();
    let name = if let Some(name) = stripped
        .strip_prefix("${")
        .and_then(|s| s.strip_suffix('}'))
    {
        name
    } else if let Some(name) = stripped.strip_prefix('$') {
        if name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b':')
        {
            name
        } else {
            return None;
        }
    } else {
        return None;
    };
    let ver = uses.get(name).copied()?;
    match values.get(&(name.to_owned(), ver))? {
        LatticeValue::Const(ConstValue::String(s)) => {
            Some(s.split_ascii_whitespace().map(str::to_owned).collect())
        }
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// AssignValue folding (C25e4)
// ---------------------------------------------------------------------------

/// Fold the RHS of an `AssignValue` statement to a lattice value.
///
/// Covers three tiers:
/// 1. **Plain literal** — no `$` / `[` → `Const(parse_literal_value)`.
/// 2. **Simple var reference** `$x` / `${x}` → lattice lookup.
/// 3. **Command substitution** `[cmd args…]` → delegate to
///    [`try_fold_cmd_subst`].
///
/// Anything else widens to `Overdefined`.
fn fold_assign_value(
    value: &str,
    uses: &HashMap<String, crate::ssa::Version>,
    values: &HashMap<ValueKey, LatticeValue>,
) -> LatticeValue {
    let stripped = value.trim();
    // Plain literal.
    if !stripped.contains('$') && !stripped.contains('[') {
        return LatticeValue::Const(parse_literal_value(stripped));
    }
    // Simple var reference.
    if let Some(resolved) = resolve_simple_var_ref(stripped, uses, values) {
        return resolved;
    }
    // Command substitution.
    if stripped.starts_with('[') && stripped.ends_with(']') {
        if let Some(lv) = try_fold_cmd_subst(stripped, uses, values) {
            return lv;
        }
    }
    LatticeValue::Overdefined
}

/// Try to constant-fold a `[cmd args…]` command substitution.
///
/// Recognised forms:
/// - `[list arg1 arg2 …]` with all-literal args → folded list text.
/// - `[llength {a b c}]` / `[llength "a b c"]` → integer element count.
/// - `[string length "text"]` → integer character count.
/// - `[expr {EXPR}]` — parses the inner expression and folds it
///   under the current lattice (bridges to C22's evaluator).
///
/// Returns `None` for anything else so callers widen to
/// Overdefined.
fn try_fold_cmd_subst(
    value: &str,
    uses: &HashMap<String, crate::ssa::Version>,
    values: &HashMap<ValueKey, LatticeValue>,
) -> Option<LatticeValue> {
    // `[list ...]` — reuse the codegen fold.
    if let Some(folded) = crate::codegen::helpers::fold_list_cmd(value) {
        return Some(LatticeValue::Const(ConstValue::String(folded)));
    }
    // `[format "..." args…]` with literal args.
    if let Some(folded) = crate::codegen::helpers::try_format_fold(value) {
        return Some(LatticeValue::Const(ConstValue::String(folded)));
    }

    let inner = value.strip_prefix('[')?.strip_suffix(']')?;
    let (cmd, rest) = split_head(inner);

    // `[llength LIST]` with a literal or lattice-resolvable list.
    if cmd == "llength" {
        let arg = rest?.trim();
        if let Some(elements) = extract_foreach_elements(arg) {
            let n = i64::try_from(elements.len()).unwrap_or(i64::MAX);
            return Some(LatticeValue::Const(ConstValue::Int(n)));
        }
        if let Some(items) = resolve_foreach_list_via_lattice(arg, uses, values) {
            let n = i64::try_from(items.len()).unwrap_or(i64::MAX);
            return Some(LatticeValue::Const(ConstValue::Int(n)));
        }
        return None;
    }

    // `[string length "text"]` with a literal string operand.
    if cmd == "string" {
        if let Some(after_cmd) = rest {
            let (sub, sub_rest) = split_head(after_cmd.trim());
            if sub == "length" {
                if let Some(arg) = sub_rest.map(|s| strip_one_level(s.trim())) {
                    let len = i64::try_from(arg.chars().count()).unwrap_or(i64::MAX);
                    return Some(LatticeValue::Const(ConstValue::Int(len)));
                }
            }
        }
        return None;
    }

    // `[expr {EXPR}]` — parse + fold under the current lattice.
    if cmd == "expr" {
        let arg = rest?.trim();
        let expr_text = strip_one_level(arg);
        let expr = crate::expr_parser::parse_expr(expr_text, None);
        let env = env_from_uses(uses, values);
        return eval_tcl_expr(&expr, &env).map(|v| LatticeValue::Const(tcl_value_to_const(v)));
    }

    None
}

/// Split a command-substitution body into `(head_word, rest)`.
/// `rest` is `None` if the body is a single word, otherwise the
/// remaining text with the leading whitespace stripped.
fn split_head(text: &str) -> (&str, Option<&str>) {
    let trimmed = text.trim_start();
    let end = trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
    let head = &trimmed[..end];
    if end >= trimmed.len() {
        return (head, None);
    }
    let rest = trimmed[end..].trim_start();
    if rest.is_empty() {
        (head, None)
    } else {
        (head, Some(rest))
    }
}

/// Strip one level of `{…}` or `"…"` wrapping, returning the
/// inside trimmed.
fn strip_one_level(text: &str) -> &str {
    if text.len() >= 2 {
        let bytes = text.as_bytes();
        if (bytes[0] == b'{' && bytes[text.len() - 1] == b'}')
            || (bytes[0] == b'"' && bytes[text.len() - 1] == b'"')
        {
            return text[1..text.len() - 1].trim();
        }
    }
    text
}

/// Parse a literal text as a [`ConstValue`]. Matches Python's
/// `_parse_literal_value`: prefers integer, then string fallback.
#[must_use]
pub fn parse_literal_value(text: &str) -> ConstValue {
    if let Ok(i) = text.parse::<i64>() {
        return ConstValue::Int(i);
    }
    ConstValue::String(text.to_owned())
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
        f.blocks.get_mut("entry").unwrap().terminator = Some(branch(literal("1"), "t", "e"));
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

    // -- C25b: driver --

    use crate::expr_ast::BinOp;
    use crate::ir::Statement;
    use crate::ssa::{SsaBlock, SsaStatement};
    use tcl_lexer::Span;

    fn assign_const_stmt(name: &str, value: &str, ver: u32) -> SsaStatement {
        let mut defs = HashMap::new();
        defs.insert(name.to_string(), ver);
        SsaStatement {
            statement: Statement::AssignConst {
                span: Span::new(0, 0),
                name: name.into(),
                value: value.into(),
            },
            uses: HashMap::new(),
            defs,
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

    #[test]
    fn sccp_marks_entry_executable_and_propagates_const() {
        // entry: set x 42
        let mut f = Function::new("::top", "entry");
        let entry_blk = f.blocks.get_mut("entry").unwrap();
        entry_blk.terminator = Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        });
        let mut ssa_entry = empty_ssa_block("entry");
        ssa_entry.statements.push(assign_const_stmt("x", "42", 1));
        let mut ssa = SsaFunction {
            name: "::top".into(),
            entry: "entry".into(),
            blocks: HashMap::new(),
            idom: HashMap::new(),
            dominance_frontier: HashMap::new(),
            dominator_tree: HashMap::new(),
        };
        ssa.blocks.insert("entry".into(), ssa_entry);

        let r = sccp(&f, &ssa, None);
        assert!(r.executable_blocks.contains("entry"));
        assert_eq!(
            r.values.get(&("x".to_string(), 1)),
            Some(&LatticeValue::Const(ConstValue::Int(42)))
        );
    }

    #[test]
    fn sccp_constant_branch_detected_and_taken_target_marked() {
        // entry: branch on literal "1" → true → "t", false → "e"
        let mut f = Function::new("::top", "entry");
        f.blocks.insert("t".into(), Block::new("t"));
        f.blocks.insert("e".into(), Block::new("e"));
        f.blocks.get_mut("entry").unwrap().terminator = Some(branch(literal("1"), "t", "e"));
        f.blocks.get_mut("t").unwrap().terminator = Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        });
        f.blocks.get_mut("e").unwrap().terminator = Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        });
        let mut ssa = SsaFunction {
            name: "::top".into(),
            entry: "entry".into(),
            blocks: HashMap::new(),
            idom: HashMap::new(),
            dominance_frontier: HashMap::new(),
            dominator_tree: HashMap::new(),
        };
        ssa.blocks.insert("entry".into(), empty_ssa_block("entry"));
        ssa.blocks.insert("t".into(), empty_ssa_block("t"));
        ssa.blocks.insert("e".into(), empty_ssa_block("e"));

        let r = sccp(&f, &ssa, None);
        assert!(r.executable_blocks.contains("t"));
        assert!(!r.executable_blocks.contains("e"));
        assert_eq!(r.constant_branches.len(), 1);
        let cb = &r.constant_branches[0];
        assert!(cb.value);
        assert_eq!(cb.taken_target, "t");
        assert_eq!(cb.not_taken_target, "e");
    }

    #[test]
    fn sccp_false_branch_prunes_true_target() {
        let mut f = Function::new("::top", "entry");
        f.blocks.insert("t".into(), Block::new("t"));
        f.blocks.insert("e".into(), Block::new("e"));
        f.blocks.get_mut("entry").unwrap().terminator = Some(branch(literal("0"), "t", "e"));
        f.blocks.get_mut("t").unwrap().terminator = Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        });
        f.blocks.get_mut("e").unwrap().terminator = Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        });
        let mut ssa = SsaFunction {
            name: "::top".into(),
            entry: "entry".into(),
            blocks: HashMap::new(),
            idom: HashMap::new(),
            dominance_frontier: HashMap::new(),
            dominator_tree: HashMap::new(),
        };
        ssa.blocks.insert("entry".into(), empty_ssa_block("entry"));
        ssa.blocks.insert("t".into(), empty_ssa_block("t"));
        ssa.blocks.insert("e".into(), empty_ssa_block("e"));

        let r = sccp(&f, &ssa, None);
        assert!(!r.executable_blocks.contains("t"));
        assert!(r.executable_blocks.contains("e"));
    }

    #[test]
    fn sccp_unknown_branch_executes_both_targets() {
        // Var reference — lattice value defaults to Unknown → decision None.
        let mut f = Function::new("::top", "entry");
        f.blocks.insert("t".into(), Block::new("t"));
        f.blocks.insert("e".into(), Block::new("e"));
        let cond = ExprNode::Var {
            text: "$z".into(),
            name: "z".into(),
            start: 0,
            end: 2,
        };
        f.blocks.get_mut("entry").unwrap().terminator = Some(branch(cond, "t", "e"));
        f.blocks.get_mut("t").unwrap().terminator = Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        });
        f.blocks.get_mut("e").unwrap().terminator = Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        });
        let mut ssa = SsaFunction {
            name: "::top".into(),
            entry: "entry".into(),
            blocks: HashMap::new(),
            idom: HashMap::new(),
            dominance_frontier: HashMap::new(),
            dominator_tree: HashMap::new(),
        };
        ssa.blocks.insert("entry".into(), empty_ssa_block("entry"));
        ssa.blocks.insert("t".into(), empty_ssa_block("t"));
        ssa.blocks.insert("e".into(), empty_ssa_block("e"));

        let r = sccp(&f, &ssa, None);
        assert!(r.executable_blocks.contains("t"));
        assert!(r.executable_blocks.contains("e"));
        assert!(r.constant_branches.is_empty());
    }

    #[test]
    fn evaluate_def_assign_const_produces_int_or_string() {
        let s_int = assign_const_stmt("x", "42", 1);
        assert_eq!(
            evaluate_def(&s_int, &HashMap::new()),
            LatticeValue::Const(ConstValue::Int(42))
        );
        let s_str = assign_const_stmt("x", "hello", 1);
        assert_eq!(
            evaluate_def(&s_str, &HashMap::new()),
            LatticeValue::Const(ConstValue::String("hello".into()))
        );
    }

    #[test]
    fn evaluate_def_assign_expr_folds_with_lattice() {
        // `set x [expr {$a + 3}]` with $a → Const(2) should fold to 5.
        let mut uses = HashMap::new();
        uses.insert("a".to_string(), 1);
        let mut defs = HashMap::new();
        defs.insert("x".to_string(), 1);

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
                expr,
            },
            uses,
            defs,
        };

        let mut values = HashMap::new();
        values.insert(
            ("a".to_string(), 1),
            LatticeValue::Const(ConstValue::Int(2)),
        );

        assert_eq!(
            evaluate_def(&stmt_ssa, &values),
            LatticeValue::Const(ConstValue::Int(5))
        );
    }

    // -- C25e1: evaluate_def for Incr --

    fn incr_stmt(name: &str, amount: Option<&str>, old_ver: u32, new_ver: u32) -> SsaStatement {
        let mut uses = HashMap::new();
        uses.insert(name.to_string(), old_ver);
        let mut defs = HashMap::new();
        defs.insert(name.to_string(), new_ver);
        SsaStatement {
            statement: Statement::Incr {
                span: Span::new(0, 0),
                name: name.into(),
                amount: amount.map(String::from),
                safe_on_uninit: false,
            },
            uses,
            defs,
        }
    }

    #[test]
    fn evaluate_def_incr_default_amount() {
        // x@1 = Const(Int(5)); `incr x` → x@2 = Const(Int(6)).
        let stmt = incr_stmt("x", None, 1, 2);
        let mut values = HashMap::new();
        values.insert(
            ("x".to_string(), 1),
            LatticeValue::Const(ConstValue::Int(5)),
        );
        assert_eq!(
            evaluate_def(&stmt, &values),
            LatticeValue::Const(ConstValue::Int(6))
        );
    }

    #[test]
    fn evaluate_def_incr_integer_literal_amount() {
        let stmt = incr_stmt("x", Some("10"), 1, 2);
        let mut values = HashMap::new();
        values.insert(
            ("x".to_string(), 1),
            LatticeValue::Const(ConstValue::Int(3)),
        );
        assert_eq!(
            evaluate_def(&stmt, &values),
            LatticeValue::Const(ConstValue::Int(13))
        );
    }

    #[test]
    fn evaluate_def_incr_negative_literal_amount() {
        let stmt = incr_stmt("x", Some("-2"), 1, 2);
        let mut values = HashMap::new();
        values.insert(
            ("x".to_string(), 1),
            LatticeValue::Const(ConstValue::Int(10)),
        );
        assert_eq!(
            evaluate_def(&stmt, &values),
            LatticeValue::Const(ConstValue::Int(8))
        );
    }

    #[test]
    fn evaluate_def_incr_var_ref_amount() {
        // `incr x $y` where $y resolves to 4.
        let mut stmt = incr_stmt("x", Some("$y"), 1, 2);
        stmt.uses.insert("y".to_string(), 1);
        let mut values = HashMap::new();
        values.insert(
            ("x".to_string(), 1),
            LatticeValue::Const(ConstValue::Int(6)),
        );
        values.insert(
            ("y".to_string(), 1),
            LatticeValue::Const(ConstValue::Int(4)),
        );
        assert_eq!(
            evaluate_def(&stmt, &values),
            LatticeValue::Const(ConstValue::Int(10))
        );
    }

    #[test]
    fn evaluate_def_incr_unknown_base_propagates_unknown() {
        let stmt = incr_stmt("x", None, 1, 2);
        let values = HashMap::new();
        // No entry for x@1 → base is Unknown → result Unknown.
        assert_eq!(evaluate_def(&stmt, &values), LatticeValue::Unknown);
    }

    #[test]
    fn evaluate_def_incr_overdefined_base_widens() {
        let stmt = incr_stmt("x", None, 1, 2);
        let mut values = HashMap::new();
        values.insert(("x".to_string(), 1), LatticeValue::Overdefined);
        assert_eq!(evaluate_def(&stmt, &values), LatticeValue::Overdefined);
    }

    #[test]
    fn evaluate_def_incr_non_integer_amount_widens() {
        let stmt = incr_stmt("x", Some("2.5"), 1, 2);
        let mut values = HashMap::new();
        values.insert(
            ("x".to_string(), 1),
            LatticeValue::Const(ConstValue::Int(1)),
        );
        assert_eq!(evaluate_def(&stmt, &values), LatticeValue::Overdefined);
    }

    #[test]
    fn resolve_simple_var_ref_accepts_bare_and_braced() {
        let mut uses = HashMap::new();
        uses.insert("x".to_string(), 1);
        let mut values = HashMap::new();
        values.insert(
            ("x".to_string(), 1),
            LatticeValue::Const(ConstValue::Int(7)),
        );
        assert_eq!(
            resolve_simple_var_ref("$x", &uses, &values),
            Some(LatticeValue::Const(ConstValue::Int(7)))
        );
        assert_eq!(
            resolve_simple_var_ref("${x}", &uses, &values),
            Some(LatticeValue::Const(ConstValue::Int(7)))
        );
        assert_eq!(resolve_simple_var_ref("$y", &uses, &values), None);
        assert_eq!(resolve_simple_var_ref("plain", &uses, &values), None);
    }

    // -- C25e2: foreach constset extraction --

    fn foreach_stmt(var: &str, list: &str, new_ver: u32) -> SsaStatement {
        let mut defs = HashMap::new();
        defs.insert(var.to_string(), new_ver);
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
                foreach_groups: None,
            },
            uses: HashMap::new(),
            defs,
        }
    }

    #[test]
    fn extract_foreach_elements_literal_list() {
        assert_eq!(
            extract_foreach_elements("{a b c}"),
            Some(vec!["a".into(), "b".into(), "c".into()])
        );
        assert_eq!(
            extract_foreach_elements("\"a b c\""),
            Some(vec!["a".into(), "b".into(), "c".into()])
        );
        assert_eq!(
            extract_foreach_elements("a b c"),
            Some(vec!["a".into(), "b".into(), "c".into()])
        );
    }

    #[test]
    fn extract_foreach_elements_rejects_substitutions() {
        assert_eq!(extract_foreach_elements("$lst"), None);
        assert_eq!(extract_foreach_elements("[list a b c]"), None);
    }

    #[test]
    fn extract_foreach_elements_empty_list_returns_empty() {
        assert_eq!(extract_foreach_elements(""), Some(Vec::new()));
        assert_eq!(extract_foreach_elements("{}"), Some(Vec::new()));
    }

    #[test]
    fn evaluate_def_foreach_literal_list_folds_constset() {
        let stmt = foreach_stmt("v", "{1 2 3}", 1);
        let result = evaluate_def(&stmt, &HashMap::new());
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
    fn evaluate_def_foreach_single_element_folds_const() {
        let stmt = foreach_stmt("v", "{only}", 1);
        assert_eq!(
            evaluate_def(&stmt, &HashMap::new()),
            LatticeValue::Const(ConstValue::String("only".into()))
        );
    }

    #[test]
    fn evaluate_def_foreach_via_lattice_var() {
        let mut stmt = foreach_stmt("v", "$lst", 1);
        stmt.uses.insert("lst".to_string(), 1);
        let mut values = HashMap::new();
        values.insert(
            ("lst".to_string(), 1),
            LatticeValue::Const(ConstValue::String("a b c".into())),
        );
        let result = evaluate_def(&stmt, &values);
        match result {
            LatticeValue::ConstSet(ref vs) => assert_eq!(vs.len(), 3),
            other => panic!("expected ConstSet, got {other:?}"),
        }
    }

    #[test]
    fn evaluate_def_foreach_unbound_var_widens() {
        let mut stmt = foreach_stmt("v", "$lst", 1);
        stmt.uses.insert("lst".to_string(), 1);
        // Empty lattice — var not bound.
        let result = evaluate_def(&stmt, &HashMap::new());
        assert_eq!(result, LatticeValue::Overdefined);
    }

    #[test]
    fn evaluate_def_foreach_multi_var_widens() {
        // 2-element defs → no constset extraction.
        let mut stmt = foreach_stmt("v", "{a b}", 1);
        let Statement::Call { defs, .. } = &mut stmt.statement else {
            panic!();
        };
        defs.push("w".into());
        let result = evaluate_def(&stmt, &HashMap::new());
        assert_eq!(result, LatticeValue::Overdefined);
    }

    // -- C25e4: AssignValue + command-substitution folding --

    fn assign_value_stmt(name: &str, value: &str, ver: u32) -> SsaStatement {
        let mut defs = HashMap::new();
        defs.insert(name.to_string(), ver);
        SsaStatement {
            statement: Statement::AssignValue {
                span: Span::new(0, 0),
                name: name.into(),
                value: value.into(),
                value_needs_backsubst: false,
                tokens: None,
            },
            uses: HashMap::new(),
            defs,
        }
    }

    #[test]
    fn evaluate_def_assign_value_plain_literal() {
        let stmt = assign_value_stmt("x", "hello", 1);
        assert_eq!(
            evaluate_def(&stmt, &HashMap::new()),
            LatticeValue::Const(ConstValue::String("hello".into()))
        );
    }

    #[test]
    fn evaluate_def_assign_value_integer_literal() {
        let stmt = assign_value_stmt("x", "42", 1);
        assert_eq!(
            evaluate_def(&stmt, &HashMap::new()),
            LatticeValue::Const(ConstValue::Int(42))
        );
    }

    #[test]
    fn evaluate_def_assign_value_resolves_var_ref() {
        let mut stmt = assign_value_stmt("y", "$x", 1);
        stmt.uses.insert("x".into(), 1);
        let mut values = HashMap::new();
        values.insert(
            ("x".to_string(), 1),
            LatticeValue::Const(ConstValue::Int(7)),
        );
        assert_eq!(
            evaluate_def(&stmt, &values),
            LatticeValue::Const(ConstValue::Int(7))
        );
    }

    #[test]
    fn evaluate_def_assign_value_folds_list_cmd() {
        let stmt = assign_value_stmt("x", "[list a b c]", 1);
        let result = evaluate_def(&stmt, &HashMap::new());
        match result {
            LatticeValue::Const(ConstValue::String(s)) => assert_eq!(s, "a b c"),
            other => panic!("expected Const(String), got {other:?}"),
        }
    }

    #[test]
    fn evaluate_def_assign_value_folds_llength_literal() {
        let stmt = assign_value_stmt("n", "[llength {a b c d}]", 1);
        assert_eq!(
            evaluate_def(&stmt, &HashMap::new()),
            LatticeValue::Const(ConstValue::Int(4))
        );
    }

    #[test]
    fn evaluate_def_assign_value_folds_string_length() {
        let stmt = assign_value_stmt("n", "[string length \"hello\"]", 1);
        assert_eq!(
            evaluate_def(&stmt, &HashMap::new()),
            LatticeValue::Const(ConstValue::Int(5))
        );
    }

    #[test]
    fn evaluate_def_assign_value_folds_expr_cmd_subst() {
        let stmt = assign_value_stmt("x", "[expr {1 + 2}]", 1);
        assert_eq!(
            evaluate_def(&stmt, &HashMap::new()),
            LatticeValue::Const(ConstValue::Int(3))
        );
    }

    #[test]
    fn evaluate_def_assign_value_folds_format_literal() {
        let stmt = assign_value_stmt("s", "[format \"%d-%d\" 1 2]", 1);
        match evaluate_def(&stmt, &HashMap::new()) {
            LatticeValue::Const(ConstValue::String(s)) => assert_eq!(s, "1-2"),
            other => panic!("expected Const(String), got {other:?}"),
        }
    }

    #[test]
    fn evaluate_def_assign_value_unknown_cmd_widens() {
        let stmt = assign_value_stmt("x", "[nonexistent_fold args]", 1);
        assert_eq!(
            evaluate_def(&stmt, &HashMap::new()),
            LatticeValue::Overdefined
        );
    }

    #[test]
    fn evaluate_def_assign_value_llength_via_lattice_var() {
        let mut stmt = assign_value_stmt("n", "[llength $lst]", 1);
        stmt.uses.insert("lst".into(), 1);
        let mut values = HashMap::new();
        values.insert(
            ("lst".to_string(), 1),
            LatticeValue::Const(ConstValue::String("a b c".into())),
        );
        assert_eq!(
            evaluate_def(&stmt, &values),
            LatticeValue::Const(ConstValue::Int(3))
        );
    }

    #[test]
    fn split_head_basic() {
        assert_eq!(split_head("cmd arg1 arg2"), ("cmd", Some("arg1 arg2")));
        assert_eq!(split_head("  cmd"), ("cmd", None));
        assert_eq!(split_head(""), ("", None));
    }

    #[test]
    fn strip_one_level_braces_and_quotes() {
        assert_eq!(strip_one_level("{abc}"), "abc");
        assert_eq!(strip_one_level("\"abc\""), "abc");
        assert_eq!(strip_one_level("bare"), "bare");
        assert_eq!(strip_one_level("{}"), "");
    }

    #[test]
    fn parse_literal_value_prefers_int() {
        assert_eq!(parse_literal_value("42"), ConstValue::Int(42));
        assert_eq!(
            parse_literal_value("hello"),
            ConstValue::String("hello".into())
        );
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
