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
use tcl_dialect::StringCharacterModel;
use tcl_registry::CommandRegistry;

use crate::analyses::{ConstValue, LatticeValue, MAX_CONSTSET_SIZE};
use crate::cfg::{BlockId, Function as CfgFunction, Terminator};
use crate::codegen::helpers::split_list_values;
use crate::expr_ast::ExprNode;
use crate::ir::Statement;
use crate::ssa::{SsaFunction, SsaStatement, Symbol, ValueKey};
use crate::tcl_expr_eval::{Env, EnvValue, FoldPolicy, TclValue, eval_tcl_expr_with_policy};

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
    pub executable_blocks: HashSet<BlockId>,
    /// `(from_block, to_block)` edges known executable.
    pub executable_edges: HashSet<(BlockId, BlockId)>,
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
/// evaluation** (issue #1134): the registry `const_fold` callbacks are pure
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
/// Only supplied by callers holding the whole-module command-mutation trust
/// fact ([`crate::command_binding::ModuleCommandMutations`]) — a renamed /
/// aliased / shadowed head must never fold with builtin semantics. The
/// shared per-unit lattice (`crate::compilation_unit`) is built **without**
/// it (its memoisation key does not carry the mutation fact); the optimiser
/// propagation pass re-runs SCCP with it when a function contains a
/// command-substitution assignment (see
/// `crate::optimiser::propagation`).
#[derive(Clone, Copy)]
pub struct BuiltinFoldInputs<'a> {
    /// Command / subcommand specs — the fold callbacks live here. Carried
    /// here (as well as on [`TraceInputs`]) so the statement-evaluation
    /// helpers need only this one bundle.
    pub registry: &'a CommandRegistry,
    /// Whole-module `rename` / `interp alias` / shadowing-`proc` trust scan.
    pub mutations: &'a crate::command_binding::ModuleCommandMutations,
    /// Resolved dialect profile for versioned folds; `None` when unavailable.
    pub dialect: Option<&'a tcl_dialect::DialectProfile>,
    /// Proven defining class of the enclosing `TclOO` instance-method frame
    /// (enables `[self class]`-style frame-fact folds); `None` elsewhere.
    pub defining_class: Option<&'a str>,
}

/// Registry-driven whole-module trace facts [`sccp`] /
/// [`sccp_with_extra_escaping`] consult when widening their escaping-set,
/// bundled into one `Copy` struct to keep those functions' argument count
/// under the clippy `too_many_arguments` ceiling.
#[derive(Clone, Copy)]
pub struct TraceInputs<'a> {
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

/// Like [`sccp_with_extra_escaping`] but additionally folds pure-builtin
/// command substitutions during lattice evaluation via the registry
/// `const_fold` callbacks — see [`BuiltinFoldInputs`] (issue #1134). Passing
/// `None` is byte-identical to [`sccp_with_extra_escaping`].
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
    let preds = compute_predecessors(cfg);
    let mut values: HashMap<ValueKey, LatticeValue> = HashMap::new();
    if let Some(seed) = param_constants {
        // The interprocedural seed keys on the parameter *name* (a stable,
        // cache-safe identity); resolve each to this build's interned symbol.
        // A param never read in the body isn't interned, and its seed slot
        // would never be consulted, so dropping it is behaviour-neutral.
        for ((name, version), v) in seed {
            if let Some(sym) = ssa.var_symbol(name) {
                values.insert((sym, *version), v.clone());
            }
        }
    }

    seed_live_in_roots(cfg, ssa, &mut values);

    // Global / namespace / upvar-aliased / traced variables are shared mutable
    // state observable and writable from other scopes, traces, and source
    // files. Their value is therefore never a compile-time constant: folding
    // through one would be unsound across any opaque call (`set ::g 5; mut;
    // expr {$::g + 1}` must NOT fold to 6 — `mut` may have rewritten `::g`).
    // Force every such definition to OVERDEFINED so SCCP never propagates a
    // constant through it; the read is still tracked for liveness. The check
    // consults the whole-function (flow-insensitive) view of the
    // `var_observability` alias/trace lattice, widened by any whole-module
    // fact the caller supplies (`extra_escaping`) and by the whole-module
    // `traced_variables` fact — the latter also catches a trace installed by
    // a *called* proc, which the single-`CfgFunction` view here cannot see.
    let mut escaping = crate::var_observability::analyse_var_observability(cfg, trace.registry)
        .escaping_var_names();
    escaping.extend(extra_escaping.iter().cloned());
    escaping.extend(trace.traced_variables.iter().cloned());

    let mut executable_blocks: HashSet<BlockId> = HashSet::new();
    let mut executable_edges: HashSet<(BlockId, BlockId)> = HashSet::new();
    if cfg.blocks.contains_key(&cfg.entry) {
        executable_blocks.insert(cfg.entry);
    }
    let order = cfg_order(cfg);

    // Optimistic fixpoint over the RPO sweep, followed by a finalising pass
    // that forces both arms for any executable branch still stuck on an UNKNOWN
    // condition (defensive: a value defined only in unreachable code could
    // otherwise leave a successor spuriously unreachable). `finalizing` is
    // monotone, so the outer loop runs at most twice.
    let mut finalizing = false;
    loop {
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

                let incoming_exec: Vec<BlockId> = preds
                    .get(bn)
                    .map(|set| {
                        set.iter()
                            .copied()
                            .filter(|p| executable_edges.contains(&(*p, *bn)))
                            .collect()
                    })
                    .unwrap_or_default();

                // Phi nodes (not at entry, only when some predecessor is
                // executable).
                if bn != &cfg.entry {
                    changed |= sccp_process_phis(&mut values, ssa_block, &incoming_exec);
                }

                // Statements.
                changed |= sccp_process_statements(
                    &mut values,
                    ssa_block,
                    ssa,
                    &escaping,
                    policy,
                    trace.has_dynamic_variable_trace,
                    folds,
                );

                // Terminator.
                let inputs = TerminatorInputs {
                    cfg,
                    ssa,
                    values: &values,
                    policy,
                };
                if sccp_process_terminator(
                    *bn,
                    &inputs,
                    &mut executable_blocks,
                    &mut executable_edges,
                    finalizing,
                ) {
                    changed = true;
                }
            }
        }
        if finalizing {
            break;
        }
        finalizing = true;
    }

    let constant_branches =
        collect_constant_branches(cfg, ssa, &values, &executable_blocks, &order, policy);

    SccpResult {
        values,
        executable_blocks,
        executable_edges,
        constant_branches,
    }
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
            for var in crate::var_refs::vars_in_expr(condition) {
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
) -> bool {
    let mut any_operand = false;
    let mut any_unknown = false;
    for name in crate::var_refs::vars_in_expr(condition) {
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

/// Evaluate each statement's defs for one block, widening across barriers.
/// Returns `true` if any lattice value changed. Extracted from [`sccp`].
fn sccp_process_statements(
    values: &mut HashMap<ValueKey, LatticeValue>,
    ssa_block: &crate::ssa::SsaBlock,
    ssa: &SsaFunction,
    escaping: &HashSet<String>,
    policy: FoldPolicy,
    has_dynamic_variable_trace: bool,
    folds: Option<BuiltinFoldInputs<'_>>,
) -> bool {
    let mut changed = false;
    for stmt_ssa in &ssa_block.statements {
        if matches!(
            stmt_ssa.statement,
            Statement::Barrier { .. } | Statement::UpFrame { .. }
        ) {
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
            continue;
        }
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
        for (&var, ver) in &stmt_ssa.defs {
            let val =
                if is_externally_mutable(ssa.var_name(var), escaping, has_dynamic_variable_trace)
                    || element_write_base == Some(var)
                {
                    LatticeValue::Overdefined
                } else if stmt_ssa.may_defs.contains(&var) {
                    // A synthetic array-element may-def: the write may or may
                    // not have hit this element, so its value is the JOIN of
                    // the prior version (recorded as a use) and the written
                    // value. The base refresh of an element write carries no
                    // prior use — the base holds no value of its own.
                    match stmt_ssa.uses.get(&var) {
                        Some(prev_ver) => {
                            let prev = values
                                .get(&(var, *prev_ver))
                                .cloned()
                                .unwrap_or(LatticeValue::Overdefined);
                            join(
                                &prev,
                                &evaluate_def_with_folds(stmt_ssa, &*values, ssa, policy, folds),
                            )
                        }
                        None => LatticeValue::Overdefined,
                    }
                } else {
                    evaluate_def_with_folds(stmt_ssa, &*values, ssa, policy, folds)
                };
            if set_value(values, (var, *ver), &val) {
                changed = true;
            }
        }
    }
    changed
}

/// Read-only inputs shared by [`sccp_process_terminator`].
struct TerminatorInputs<'a> {
    cfg: &'a CfgFunction,
    ssa: &'a SsaFunction,
    values: &'a HashMap<ValueKey, LatticeValue>,
    policy: FoldPolicy,
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
        policy,
    } = *inputs;
    let mut changed = false;
    let Some(block) = cfg.blocks.get(&bn) else {
        return false;
    };
    let Some(term) = &block.terminator else {
        return false;
    };
    match term {
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
            ..
        } => {
            let Some(ssa_block) = ssa.blocks.get(&bn) else {
                return changed;
            };
            let decision = branch_decision(cfg, ssa, bn, ssa_block, condition, values, policy);
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
                None if !finalizing && branch_deferrable(ssa_block, condition, values, ssa) => {
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
        Terminator::Return { .. } => {}
    }
    // `try` exception edges sourced at `bn`: when `bn` is executable the
    // handler is reachable (a throw can occur in the body).
    for (from, to) in &cfg.exception_edges {
        if *from != bn {
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
/// condition evaluated to a constant lattice value.  Extracted
/// from [`sccp`].
fn collect_constant_branches(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    values: &HashMap<ValueKey, LatticeValue>,
    executable_blocks: &HashSet<BlockId>,
    order: &[BlockId],
    policy: FoldPolicy,
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
        let decision = branch_decision(cfg, ssa, *bn, ssa_block, condition, values, policy);
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
            }),
            Some(false) => constant_branches.push(ConstantBranch {
                block: cfg.block_name(*bn).to_owned(),
                span: *term_span,
                condition: cond_text,
                value: false,
                taken_target: false_name,
                not_taken_target: true_name,
            }),
            None => {}
        }
    }
    constant_branches
}

/// Every variable name the function assigns, and every name it `unset`s by
/// literal — the two whole-body facts [`existence_constant_branches`] folds
/// against.  A `Call`'s `defs` cover the commands that define a name without
/// an assignment statement (`global` / `variable` / `upvar`, `regexp -inline`
/// match vars, …).
fn scan_defined_and_unset(cfg: &CfgFunction) -> (FxHashSet<String>, FxHashSet<&str>) {
    let mut defined: FxHashSet<String> = FxHashSet::default();
    let mut unset: FxHashSet<&str> = FxHashSet::default();
    for block in cfg.blocks.values() {
        for stmt in &block.statements {
            match stmt {
                Statement::AssignConst { name, .. }
                | Statement::AssignExpr { name, .. }
                | Statement::AssignValue { name, .. }
                | Statement::Incr { name, .. } => {
                    let n = crate::naming::normalise_var_name(name);
                    if !n.is_empty() {
                        defined.insert(n.to_string());
                    }
                }
                Statement::Call {
                    command,
                    args,
                    defs,
                    ..
                } => {
                    for d in defs {
                        defined.insert(d.clone());
                    }
                    if command == "unset" {
                        unset.extend(args.iter().map(String::as_str));
                    }
                }
                _ => {}
            }
        }
    }
    (defined, unset)
}

/// The entry facts one function frame contributes to the existence fold
/// ([`existence_constant_branches`]), sourced from the typed IR for whichever
/// kind of body it is — a [`crate::ir::Procedure`], a
/// [`crate::ir::MethodDef`], or neither (the top level, a lambda).
///
/// Bundled rather than passed positionally so a new fact reaches both
/// consumers of the fold — the analyser's I230 and the optimiser's O101 — by
/// construction: the two build the same struct from the same IR, so they
/// cannot drift (they did, on method parameters — issue #1129).
#[derive(Clone, Copy, Default)]
pub struct ExistenceFrame<'a> {
    /// The body's formal parameter names: bound on entry as scalars, so
    /// they exist for `info exists` and never for `array exists`.
    /// Empty for the top level and for any body with no parameter list.
    pub params: &'a [String],
    /// Names auto-bound to out-of-frame *object* storage on entry — a
    /// `TclOO` method body's [`crate::ir::MethodDef::instance_vars`].
    /// `None` for every body kind that has none.
    pub object_state: Option<&'a HashSet<String>>,
    /// Whether this body is the document's **initial global frame** (the
    /// compilation unit's top level).  Only there does the frame share the
    /// interpreter's own globals, so only there must the fold abstain on the
    /// registry's special variables — a procedure-local `argv` is an ordinary
    /// fresh Tcl name and keeps folding.
    pub initial_global: bool,
}

/// The array base name of an existence query written as an element guard —
/// `Some("Params")` for `Params(key)` (any element spelling, including a
/// dynamic `Params($k)`), `None` for every other shape.  Only a simple local
/// base qualifies: a namespaced array (`::env(PATH)`) may be populated
/// outside the function's view.
fn array_element_base(var: &str) -> Option<&str> {
    let (base, rest) = var.split_once('(')?;
    if base.is_empty() || !rest.ends_with(')') {
        return None;
    }
    base.bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'_')
        .then_some(base)
}

/// Fold `[info exists X]` / `[array exists X]`
/// if-conditions into [`ConstantBranch`] entries for the
/// false-positive-free cases — a parameter always exists, as a **scalar**
/// (`info exists` → `true`, `array exists` → `false`, issue #1239); a
/// never-defined non-parameter never exists (`false`); an element guard
/// `X(elem)` on an array this body never touches never exists (`false`,
/// issue #1173 — the guard is decided on the *array* name, with the same
/// abstentions as a simple name, so the element key may even be dynamic).
/// `![info exists X]` flips the value.
///
/// SCCP itself can't fold these (the predicate is an opaque
/// `ExprNode::Command`, and SCCP has neither parameter nor existence
/// facts), so this runs as a post-pass with the frame's own facts.  The
/// result feeds both the analyser's I230 (constant condition) and the
/// optimiser's O101 (constant-branch fold / DCE).  Only simple local
/// names are folded, and only in functions free of opaque barriers (an
/// unknown command could `unset` or `upvar`-define the variable).
/// Scope-alias locals (`global` / `variable` / `upvar` / `namespace
/// upvar` bindings) are never folded — their existence tracks the
/// linked out-of-frame variable.  In the **initial global frame**
/// ([`ExistenceFrame::initial_global`]) the registry's special variables join
/// them for the same reason: that frame is the interpreter's own global
/// namespace, whose startup bindings and runtime-materialised entries the
/// body's assignment scan cannot see (issue #1557).
///
/// `dynamic_names` carries the function's
/// [dynamic-name barrier](crate::dynamic_names) and gates each direction
/// independently (issue #923 audit idx 1):
///
/// - a **dynamic write** (`set $switch {}`) can define *any* name, so the
///   "never defined here, therefore absent" fold is no longer provable;
/// - a **dynamic destroy** (`unset $n`) can remove *any* name, so even the
///   "it's a parameter, therefore present" fold is no longer provable.
///
/// Both abstain by declining the fold, which silences I230 and leaves O101
/// with nothing to fold — say less rather than say something wrong.
///
/// [`ExistenceFrame::object_state`] carries the frame's *auto-bound*
/// out-of-frame names — a `TclOO` method body's
/// [`crate::ir::MethodDef::instance_vars`] (issue
/// #1129).  A class-level `variable x` declaration binds `x` in **every**
/// method's frame with no `variable` statement in the body itself, so
/// [`crate::optimiser::elimination::scan_scope_aliases`] (which only sees the
/// body's own commands) cannot find it — the name looks like a never-defined
/// local and the fold used to call it "always absent".  It is not: existence
/// is per-instance runtime state, set by whichever method or constructor
/// assigned it first.  tclsh 9.0.4 and 8.6.14 agree:
///
/// ```tcl
/// oo::class create C { variable x; constructor {} { set x 1 }
///                      method m {} { info exists x } }   ;# [C new] m → 1
/// oo::class create D { variable x
///                      method m {} { info exists x } }   ;# [D new] m → 0
/// oo::class create F { variable x; method setit {} { set x 42 }
///                      method m {} { info exists x } }
/// set f [F new]; $f m   ;# → 0
/// $f setit; $f m        ;# → 1
/// ```
///
/// The declaration alone does not create the variable, but *any earlier call
/// on the same instance* may have — a dynamic fact no per-method analysis can
/// decide, so these names join `aliased` and never fold either way.
///
/// A method parameter that *collides* with an instance-variable name is the
/// exception, and still folds `true`: the parameter shadows the class-level
/// declaration completely, and writes through it never reach object state
/// (again identical on 9.0.4 and 8.6.14).
///
/// ```tcl
/// oo::class create A { variable x; constructor {} { set x 42 }
///                      method m {x} { set r [info exists x]; set x 9; return $r }
///                      method peek {} { return $x } }
/// set a [A new]; $a m hello   ;# → 1
/// $a peek                     ;# → 42, untouched by the method's `set x 9`
/// ```
///
/// This also holds for a *defaulted* parameter called with no argument
/// (`method m {{x def}} …` → `info exists x` is 1 and `$x` is `def`), and
/// when the instance variable was never assigned at all.
#[must_use]
pub fn existence_constant_branches(
    cfg: &CfgFunction,
    frame: ExistenceFrame<'_>,
    registry: &tcl_registry::CommandRegistry,
    dynamic_names: crate::dynamic_names::DynamicNameBarrier,
) -> Vec<ConstantBranch> {
    let mut out = Vec::new();
    if cfg.blocks.values().any(|b| {
        b.statements
            .iter()
            .any(|s| matches!(s, Statement::Barrier { .. } | Statement::UpFrame { .. }))
    }) {
        return out;
    }
    let (defined, unset) = scan_defined_and_unset(cfg);
    // Locals bound to out-of-frame storage (`global` / `variable` / `upvar` /
    // `namespace upvar`): whether such a name exists depends on the *linked*
    // variable, which this function cannot see, so its existence query must
    // never fold either way.  `global` / `variable` / `upvar` escape via
    // `defined` already (their `Call::defs` carry the alias local), but
    // `namespace upvar` lowers with empty defs — tclsh 8.6:
    // `namespace eval ns {variable s ok}; proc t {} {namespace upvar ns s a;
    // info exists a}; t` → 1 (and → 0 when `ns::s` is unset), the exact
    // `::safe::CheckInterp` guard shape (safe.tcl:109).  The scanner also
    // returns `trace` targets, which only widens the skip — conservative,
    // never a false fold.
    let mut aliased = crate::optimiser::elimination::scan_scope_aliases(cfg, registry);
    // Object state is aliased the same way, minus a visible binding command:
    // `TclOO` links every class-level `variable` declaration into each method
    // frame at entry, so the body's own command scan cannot see it (#1129).
    //
    // A formal parameter of the same name is the one exception: it shadows the
    // class-level declaration outright, so the name is an ordinary local that
    // always exists and must keep folding `true`.  tclsh 9.0.4 / 8.6.14 agree
    // — the parameter wins completely, and writes to it do **not** reach
    // object state:
    //
    //   oo::class create A { variable x; constructor {} { set x 42 }
    //                        method m {x} { set r [info exists x]  ;# → 1
    //                                       set x 9; return $r }
    //                        method peek {} { return $x } }
    //   set a [A new]; $a m hello   ;# → 1  ($x inside m is "hello")
    //   $a peek                     ;# → 42, unchanged by `set x 9`
    //
    // Only the *object-state* half yields to parameters.  The
    // command-derived aliases keep full precedence: an explicit `global` /
    // `variable` / `upvar` / `namespace upvar` / `my variable` on a name that
    // is already a parameter is a runtime error on both runtimes (`variable
    // "x" already exists`), so it never legally co-occurs, while a variable
    // *trace* on a parameter does — and a trace callback can unset its own
    // target, which is exactly why `scan_scope_aliases` includes trace
    // targets and why they must go on abstaining.
    if let Some(instance_vars) = frame.object_state {
        aliased.extend(
            instance_vars
                .iter()
                .filter(|name| !frame.params.iter().any(|p| p == *name))
                .cloned(),
        );
    }
    // The document's initial global frame *is* the interpreter's global
    // namespace, so every name the special-variable registry recognises there
    // is out-of-frame runtime state exactly like object state above (issue
    // #1557).  Some are bound before user code (`argv`, `env`, `tcl_platform`,
    // `auto_path`), some are materialised by a later runtime event this body
    // cannot see (`errorInfo` after a `catch`, `auto_index` after an
    // auto-load), and some by a read trace (`tcl_precision` on Tcl 8.x) — none
    // is provably absent merely because the body never assigned it, and
    // tclsh 8.4.20 / 8.5.19 / 8.6.14 / 9.0.4 / 9.1b0 all answer
    // `info exists argv` → 1 at the top level.  Folding them "always absent"
    // produced a false I230 and, worse, an O101 rewrite of
    // `if {[info exists argv]} …` to `if {0} …`.
    //
    // The set is dialect-versioned registry data, so a release that drops a
    // variable (`tcl_precision` in Tcl 9) or a dialect that never had one
    // (iRules has no `argv`) keeps folding it.  Inside a procedure the name is
    // an ordinary local and still folds; an explicit `global argv` there is
    // already covered by the scope-alias skip above.
    if frame.initial_global {
        aliased.extend(
            tcl_registry::special_vars::special_vars_for_dialect(
                tcl_registry::special_vars::dialect_set_for_profile(registry.profile()),
            )
            .map(|spec| spec.name.to_owned()),
        );
    }
    for block in cfg.blocks.values() {
        let Some(Terminator::Branch {
            condition,
            true_target,
            false_target,
            span: Some(span),
            ..
        }) = &block.terminator
        else {
            continue;
        };
        let Some(crate::expr_ast::ExistenceQuery {
            var,
            negated,
            command,
        }) = crate::expr_ast::existence_query_var(condition)
        else {
            continue;
        };
        let exists = if let Some(base) = array_element_base(&var) {
            // An array-element guard on a never-touched array is provably
            // false (issue #1173): no element of `a` can exist when nothing
            // in this barrier-free body ever created `a` — tclsh 9.0.4 /
            // 8.6.16: `proc f {} { info exists Params(key) }` → 0.  The
            // decision is about the *array* name alone, so a dynamic element
            // key (`Params($k)`) folds just as well, and the base takes the
            // same abstentions as a simple name: scope-alias / instance-state
            // (`aliased`), a dynamic write that may have created any name,
            // and any touch of the base — a `set a(x) …` element write, an
            // `array set` / `upvar`-style whole-array def, either spelling.
            // A parameter base abstains outright: the parameter itself is a
            // scalar, and the fold stays strictly one-sided here rather than
            // reason about unset-and-remake shapes.
            if aliased.contains(base)
                || frame.params.iter().any(|p| p == base)
                || dynamic_names.writes
                || defined
                    .iter()
                    .any(|d| d == base || d.strip_prefix(base).is_some_and(|r| r.starts_with('(')))
            {
                continue;
            }
            false
        } else {
            // Namespaced globals may be populated outside the function's
            // view — only fold simple local names.
            if var.is_empty() || !var.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
                continue;
            }
            // A scope-alias local's existence tracks the linked variable —
            // never fold it (see the `aliased` collection above).
            if aliased.contains(var.as_str()) {
                continue;
            }
            if frame.params.iter().any(|p| p == &var) {
                // A literal `unset x` already blocks this; a computed
                // `unset $n` can name the parameter just as well
                // (tclsh 9.0.4 / 8.6.14: `proc f {p n} {unset $n; info exists p}`
                // → `0` for `f hello p`), so the barrier blocks it too.
                if unset.contains(var.as_str()) || dynamic_names.destroys {
                    continue;
                }
                // Which constant depends on the spelling (issue #1239).  A
                // parameter is bound as a *scalar* on entry — Tcl has no
                // pass-an-array-by-value — so `array exists PARAM` is
                // provably **false** where `info exists PARAM` is true.
                // Nothing in a barrier-free body can turn the parameter into
                // an array without first removing the scalar binding, and a
                // literal `unset` / a dynamic destroy already abstained above
                // (`set p(k) …` and `array set p …` on a live scalar are
                // runtime errors, not conversions).
                //
                // tclsh-proof (8.6.16 / 9.0.4):
                //   proc f {a} { if {[array exists a]} { puts yes } else { puts no } }
                //   f 1                                        ;# → no
                //   proc g {a} { array set a {x 1} }
                //   g 1  ;# → can't set "a(x)": variable isn't array
                matches!(command, crate::expr_ast::ExistenceCommand::Info)
            } else if !defined.contains(&var) {
                // `set $switch {}` may have defined exactly this name — the
                // argparse idiom the fold used to call unreachable.
                if dynamic_names.writes {
                    continue;
                }
                false
            } else {
                continue;
            }
        };
        let value = exists ^ negated;
        let (true_name, false_name) = (
            cfg.block_name(*true_target).to_owned(),
            cfg.block_name(*false_target).to_owned(),
        );
        let (taken, not_taken) = if value {
            (true_name, false_name)
        } else {
            (false_name, true_name)
        };
        out.push(ConstantBranch {
            block: block.name.clone(),
            span: Some(*span),
            condition: crate::expr_ast::expr_text(condition),
            value,
            taken_target: taken,
            not_taken_target: not_taken,
        });
    }
    out
}

/// Evaluate the lattice value produced by an SSA statement's
/// defs.
///
/// Focused subset: constant-assignment, expression-assignment via
/// the expression evaluator, and a conservative `Overdefined` fallback
/// for everything else.
#[must_use]
pub fn evaluate_def<S: std::hash::BuildHasher>(
    stmt_ssa: &SsaStatement,
    values: &HashMap<ValueKey, LatticeValue, S>,
    ssa: &SsaFunction,
    policy: FoldPolicy,
) -> LatticeValue {
    evaluate_def_with_folds(stmt_ssa, values, ssa, policy, None)
}

/// [`evaluate_def`] with an optional registry builtin-fold context (issue
/// #1134): when `folds` is supplied, an `AssignValue` command-substitution
/// RHS additionally consults the registry `const_fold` engine — see
/// [`BuiltinFoldInputs`]. `None` is byte-identical to [`evaluate_def`].
#[must_use]
pub fn evaluate_def_with_folds<S: std::hash::BuildHasher>(
    stmt_ssa: &SsaStatement,
    values: &HashMap<ValueKey, LatticeValue, S>,
    ssa: &SsaFunction,
    policy: FoldPolicy,
    folds: Option<BuiltinFoldInputs<'_>>,
) -> LatticeValue {
    match &stmt_ssa.statement {
        Statement::AssignConst { value, .. } => LatticeValue::Const(parse_literal_value(value)),
        Statement::AssignExpr { expr, .. } => {
            let env = env_from_uses(&stmt_ssa.uses, values, ssa);
            match eval_tcl_expr_with_policy(expr, &env, policy) {
                Some(v) => LatticeValue::Const(tcl_value_to_const(v)),
                None => LatticeValue::Overdefined,
            }
        }
        Statement::AssignValue { value, .. } => {
            // Fold when the RHS is either a plain literal
            // (no command substitution), a simple `$var` that
            // resolves to a lattice Const, or a `[cmd args...]`
            // that try_fold_cmd_subst (or, under `folds`, the registry
            // const-fold engine) recognises.
            fold_assign_value(value, &stmt_ssa.uses, values, ssa, policy, folds)
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
            // `foreach v LIST` / `lmap v LIST` folds the
            // iteration variable to the CONSTSET of elements when
            // LIST is a literal, resolves to a Const(String)
            // through the lattice, or is a command substitution
            // (`[list a b c]`, `[format …]`) that folds to a
            // constant list. Multi-variable and multi-list
            // foreaches are left as Overdefined.
            let elements = extract_foreach_elements(&args[0])
                .or_else(|| resolve_foreach_list_via_lattice(&args[0], &stmt_ssa.uses, values, ssa))
                .or_else(|| {
                    // `foreach v [list a b c]` — fold the command substitution
                    // to a constant list string, then split into elements.
                    let arg = args[0].trim();
                    if arg.starts_with('[')
                        && arg.ends_with(']')
                        && let Some(LatticeValue::Const(ConstValue::String(s))) =
                            try_fold_cmd_subst(arg, &stmt_ssa.uses, values, ssa, policy, folds)
                    {
                        return Some(split_list_values(&s));
                    }
                    None
                });
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
            // Track `incr NAME ?AMOUNT?` through the lattice
            // when the current value of NAME is a single Const(Int)
            // and AMOUNT is either absent (defaults to 1), a decimal
            // integer literal, or a simple `$var` reference that
            // resolves to Const(Int) via `uses`.
            // A dynamic-key target (`incr a($i)`) never interns a symbol —
            // that miss is permanent, so it must widen: returning Unknown
            // would launder a fanned element's stale constant through
            // `join(prev, Unknown) = prev`.
            let Some(sym) = ssa.var_symbol(crate::naming::element_var_name(name)) else {
                return LatticeValue::Overdefined;
            };
            let ver = stmt_ssa.uses.get(&sym).copied().unwrap_or(0);
            let base = values
                .get(&(sym, ver))
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
                        resolve_simple_var_ref(trimmed, &stmt_ssa.uses, values, ssa)
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
fn resolve_simple_var_ref<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
    text: &str,
    uses: &HashMap<Symbol, crate::ssa::Version, S1>,
    values: &HashMap<ValueKey, LatticeValue, S2>,
    ssa: &SsaFunction,
) -> Option<LatticeValue> {
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
    Some(
        values
            .get(&(sym, ver))
            .cloned()
            .unwrap_or(LatticeValue::Unknown),
    )
}

/// Resolve a branch decision, preferring a *static-loop summary* when the
/// branch's block is the exit of a bounded `for` loop.
///
/// SCCP alone cannot fold a branch that reads a loop-carried variable *after*
/// the loop — the variable's post-loop phi is a CONSTSET or `Overdefined`, not
/// a single constant. Simulating the loop instead yields its exact final
/// values (`for {set i 0} {$i < 10} {incr i} {}` leaves `i == 10`), so a
/// following `if {$i == 10}` folds. The summary is conservative: it bails to
/// `None` on non-constant bounds, side effects, or runaway iteration, falling
/// back to the lattice fold.
fn branch_decision(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    bn: BlockId,
    ssa_block: &crate::ssa::SsaBlock,
    condition: &ExprNode,
    values: &HashMap<ValueKey, LatticeValue>,
    policy: FoldPolicy,
) -> Option<bool> {
    loop_summary_decision(cfg, ssa, bn, condition, values, policy)
        .or_else(|| evaluate_branch(ssa_block, condition, values, policy, ssa))
}

/// Convert an SCCP [`ConstValue`] to the static simulator's
/// [`crate::static_loops::StaticValue`].
fn const_to_static(c: &ConstValue) -> crate::static_loops::StaticValue {
    use crate::static_loops::StaticValue;
    match c {
        ConstValue::Int(i) => StaticValue::Int(*i),
        ConstValue::Float(f) => StaticValue::Float(*f),
        ConstValue::Bool(b) => StaticValue::Bool(*b),
        ConstValue::String(s) => StaticValue::Str(s.clone()),
    }
}

/// Fold `condition` via a static summary of the `for` loop whose exit block is
/// `bn`, or `None` when `bn` is not a loop exit or the loop cannot be
/// summarised. The simulation is seeded with the constants known at the
/// pre-loop block's exit and run by [`crate::static_loops::summarise_for_statement`].
fn loop_summary_decision(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    bn: BlockId,
    condition: &ExprNode,
    values: &HashMap<ValueKey, LatticeValue>,
    policy: FoldPolicy,
) -> Option<bool> {
    let node = cfg.loop_nodes.get(&bn)?;
    let start_ssa = ssa.blocks.get(&node.entry_block)?;
    let mut start_env = crate::static_loops::StaticEnv::new();
    for (&sym, &ver) in &start_ssa.exit_versions {
        if let Some(LatticeValue::Const(c)) = values.get(&(sym, ver)) {
            start_env.insert(ssa.var_name(sym).to_owned(), const_to_static(c));
        }
    }
    let summarised = crate::static_loops::summarise_for_statement(
        &node.for_stmt,
        &start_env,
        crate::static_loops::DEFAULT_MAX_STATIC_LOOP_ITERS,
        policy,
    )?;
    let v = crate::static_loops::evaluate_expr_with_constants(condition, &summarised, policy)?;
    Some(v != 0)
}

/// Evaluate a branch condition.
///
/// Returns `Some(true)` / `Some(false)` when the condition folds to
/// a constant under the current lattice; `None` otherwise.
#[must_use]
pub fn evaluate_branch<S: std::hash::BuildHasher>(
    ssa_block: &crate::ssa::SsaBlock,
    condition: &ExprNode,
    values: &HashMap<ValueKey, LatticeValue, S>,
    policy: FoldPolicy,
    ssa: &SsaFunction,
) -> Option<bool> {
    let mut env = env_from_uses(&ssa_block.exit_versions, values, ssa);
    // A parameter read in a branch condition without a local redefinition
    // isn't in `exit_versions` (those carry defined-in-block versions), so
    // its caller-provided version-0 seed never reaches the fold. Bind it
    // here — but only when version 0 is still live (the param is not
    // redefined to another value before the branch).
    for name in crate::var_refs::vars_in_expr(condition) {
        if env.contains_key(&name) {
            continue;
        }
        let Some(sym) = ssa.var_symbol(&name) else {
            continue;
        };
        let v0_live = ssa_block.exit_versions.get(&sym).copied().unwrap_or(0) == 0;
        if v0_live && let Some(LatticeValue::Const(c)) = values.get(&(sym, 0)) {
            env.insert(name, const_to_env_value(c));
        }
    }
    let v = eval_tcl_expr_with_policy(condition, &env, policy)?;
    // A NaN condition is C's "floating point value is Not a Number" runtime
    // error, not a truth value — folding either way would delete a branch
    // that must raise. Decline.
    if matches!(&v, crate::tcl_expr_eval::TclValue::Float(f) if f.is_nan()) {
        return None;
    }
    Some(v.is_truthy())
}

/// Build a [`tcl_expr_eval::Env`] from a `{symbol → version}` map
/// and the current lattice. Only entries whose lattice value is
/// a single [`LatticeValue::Const`] are bound; anything else
/// leaves the variable unbound so the evaluator returns `None`.
fn env_from_uses<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
    uses: &HashMap<Symbol, crate::ssa::Version, S1>,
    values: &HashMap<ValueKey, LatticeValue, S2>,
    ssa: &SsaFunction,
) -> Env {
    let mut env = Env::new();
    for (&sym, &ver) in uses {
        if let Some(LatticeValue::Const(c)) = values.get(&(sym, ver)) {
            env.insert(ssa.var_name(sym).to_owned(), const_to_env_value(c));
        }
    }
    env
}

/// Like [`env_from_uses`] but includes only variables whose lattice value is
/// *numeric* (int / float / bool). Used for folding a quoted / bare
/// `expr "…"`, where Tcl substitutes the variable's value textually before
/// parsing: a non-numeric value becomes an invalid bareword, so leaving it
/// unbound makes the fold bail (matching Tcl's runtime error).
fn env_from_uses_numeric<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
    uses: &HashMap<Symbol, crate::ssa::Version, S1>,
    values: &HashMap<ValueKey, LatticeValue, S2>,
    ssa: &SsaFunction,
) -> Env {
    let mut env = Env::new();
    for (&sym, &ver) in uses {
        if let Some(LatticeValue::Const(c)) = values.get(&(sym, ver))
            && matches!(
                c,
                ConstValue::Int(_) | ConstValue::Float(_) | ConstValue::Bool(_)
            )
        {
            env.insert(ssa.var_name(sym).to_owned(), const_to_env_value(c));
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

pub(crate) fn tcl_value_to_const(v: TclValue) -> ConstValue {
    match v {
        TclValue::Int(i) => ConstValue::Int(i),
        TclValue::Float(f) => ConstValue::Float(f),
        // A beyond-wide integer's lattice form is its canonical decimal
        // string — the value's one true rep, which downstream folds re-parse
        // exactly (`set big [expr {2**64}]; expr {$big + 1}` chains).
        TclValue::Big(b) => ConstValue::String(b.to_string()),
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
pub fn extract_foreach_elements(list_text: &str) -> Option<Vec<String>> {
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
    Some(split_list_values(stripped))
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
        LatticeValue::Const(ConstValue::String(s)) => Some(split_list_values(s)),
        _ => None,
    }
}

// AssignValue folding

/// Fold the RHS of an `AssignValue` statement to a lattice value.
///
/// Covers three tiers:
/// 1. **Plain literal** — no `$` / `[` → `Const(parse_literal_value)`.
/// 2. **Simple var reference** `$x` / `${x}` → lattice lookup.
/// 3. **Command substitution** `[cmd args…]` → delegate to
///    [`try_fold_cmd_subst`], then (when `folds` is supplied) to the
///    registry const-fold engine ([`BuiltinFoldInputs`], issue #1134).
///
/// Anything else widens to `Overdefined`.
fn fold_assign_value<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
    value: &str,
    uses: &HashMap<Symbol, crate::ssa::Version, S1>,
    values: &HashMap<ValueKey, LatticeValue, S2>,
    ssa: &SsaFunction,
    policy: FoldPolicy,
    folds: Option<BuiltinFoldInputs<'_>>,
) -> LatticeValue {
    let stripped = value.trim();
    // Plain literal.
    if !stripped.contains('$') && !stripped.contains('[') {
        return LatticeValue::Const(parse_literal_value(stripped));
    }
    // Simple var reference.
    if let Some(resolved) = resolve_simple_var_ref(stripped, uses, values, ssa) {
        return resolved;
    }
    // Command substitution.
    if stripped.starts_with('[') && stripped.ends_with(']') {
        if let Some(lv) = try_fold_cmd_subst(stripped, uses, values, ssa, policy, folds) {
            return lv;
        }
        // Registry const-fold fallback (issue #1134): the fold's `$var`
        // words resolve at this statement's use versions, so a folded
        // value re-enters the lattice and downstream statements see it —
        // the multi-hop chain the hardcoded arms above cannot close.
        // Checked AFTER them so single-hop results stay byte-identical.
        if let Some(f) = folds {
            let trusts = |name: &str| f.mutations.trusts(name);
            let lookup = |name: &str| lattice_const_text(name, uses, values, ssa);
            if let Some(folded) = (crate::const_subst::ConstSubstCtx {
                registry: f.registry,
                version: f
                    .dialect
                    .and_then(tcl_dialect::DialectProfile::const_fold_version),
                defining_class: f.defining_class,
                trusts: &trusts,
                lookup_var: &lookup,
            })
            .fold_cmd_subst(&stripped[1..stripped.len() - 1])
            {
                return LatticeValue::Const(parse_literal_value(&folded));
            }
        }
    }
    LatticeValue::Overdefined
}

/// Resolve `name` to the textual form of its lattice constant at this
/// statement's use version, or `None` when it is not a single `Const` —
/// the variable-lookup the registry const-fold engine runs under (see
/// [`fold_assign_value`]).
fn lattice_const_text<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
    name: &str,
    uses: &HashMap<Symbol, crate::ssa::Version, S1>,
    values: &HashMap<ValueKey, LatticeValue, S2>,
    ssa: &SsaFunction,
) -> Option<String> {
    let sym = ssa.var_symbol(name)?;
    let ver = uses.get(&sym)?;
    match values.get(&(sym, *ver))? {
        LatticeValue::Const(ConstValue::String(s)) => Some(s.clone()),
        LatticeValue::Const(ConstValue::Int(i)) => Some(i.to_string()),
        LatticeValue::Const(ConstValue::Bool(b)) => Some(if *b { "1" } else { "0" }.to_owned()),
        LatticeValue::Const(ConstValue::Float(f)) => Some(f.to_string()),
        _ => None,
    }
}

/// Try to constant-fold a `[cmd args…]` command substitution.
///
/// Recognised forms:
/// - `[list arg1 arg2 …]` with all-literal args → folded list text.
/// - `[llength {a b c}]` / `[llength "a b c"]` → integer element count.
/// - `[string length "text"]` → integer character count.
/// - `[expr {EXPR}]` — parses the inner expression and folds it
///   under the current lattice (bridges to the expression evaluator).
///
/// Returns `None` for anything else so callers widen to
/// Overdefined.
/// Resolve a single command operand to its constant string value: a literal
/// word (optionally brace/quote wrapped), or a pure `$var` / `${var}` whose
/// SCCP lattice value is a constant. Returns `None` for anything that isn't a
/// compile-time constant (array refs, command substitutions, unknown vars),
/// so the caller skips folding.
fn resolve_const_string<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
    arg: &str,
    uses: &HashMap<Symbol, crate::ssa::Version, S1>,
    values: &HashMap<ValueKey, LatticeValue, S2>,
    ssa: &SsaFunction,
) -> Option<String> {
    let arg = arg.trim();
    if let Some(rest) = arg.strip_prefix('$') {
        // `$name` or `${name}` — reject compound refs (array element,
        // nested substitution, multiple words).
        let name = rest
            .strip_prefix('{')
            .and_then(|r| r.strip_suffix('}'))
            .unwrap_or(rest);
        if name.is_empty()
            || name.contains(|c: char| {
                c.is_whitespace() || c == '(' || c == '[' || c == '$' || c == '"'
            })
        {
            return None;
        }
        let sym = ssa.var_symbol(name)?;
        let ver = uses.get(&sym)?;
        return match values.get(&(sym, *ver))? {
            LatticeValue::Const(ConstValue::String(s)) => Some(s.clone()),
            LatticeValue::Const(ConstValue::Int(i)) => Some(i.to_string()),
            LatticeValue::Const(ConstValue::Bool(b)) => Some(if *b { "1" } else { "0" }.to_owned()),
            LatticeValue::Const(ConstValue::Float(f)) => Some(f.to_string()),
            _ => None,
        };
    }
    // A literal word with no interpolation or command substitution.
    if !arg.contains('$') && !arg.contains('[') {
        return Some(strip_one_level(arg).to_owned());
    }
    None
}

// The per-command fold arms below (`list` / `format` / `llength` / `string
// length` / `expr`) are name-keyed on purpose: each arm IS that command's
// fold semantics (what a constant call evaluates to), not a membership
// test a registry trait could answer — the same irreducible-fold rationale
// as `chain_fold`'s per-command arms.
fn try_fold_cmd_subst<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
    value: &str,
    uses: &HashMap<Symbol, crate::ssa::Version, S1>,
    values: &HashMap<ValueKey, LatticeValue, S2>,
    ssa: &SsaFunction,
    policy: FoldPolicy,
    folds: Option<BuiltinFoldInputs<'_>>,
) -> Option<LatticeValue> {
    // Each arm below *is* a builtin's semantics, so it may only run while
    // that name still denotes the builtin: after `rename list mylist` or a
    // shadowing `proc format …` anywhere in the unit, `[list a 1 a 2]` is a
    // call to something else entirely (issue #1585). This is the same trust
    // fact the registry-driven engine below already consults; the arms here
    // ran ahead of it and never asked.
    //
    // `folds == None` is the mutation-fact-free shared per-unit lattice (see
    // [`BuiltinFoldInputs`]), which no rewrite lands from: the optimiser
    // re-runs SCCP *with* the fact before propagating a command-substitution
    // assignment, and that run is the one whose constants reach codegen.
    let trusted = |name: &str| folds.is_none_or(|f| f.mutations.trusts(name));

    // `[list ...]` — reuse the codegen fold.
    if trusted("list")
        && let Some(folded) = crate::codegen::helpers::fold_list_cmd(value)
    {
        return Some(LatticeValue::Const(ConstValue::String(folded)));
    }
    // `[format "..." args…]` with literal args.
    if trusted("format")
        && let Some(folded) = crate::codegen::helpers::try_format_fold(value)
    {
        return Some(LatticeValue::Const(ConstValue::String(folded)));
    }

    let inner = value.strip_prefix('[')?.strip_suffix(']')?;
    let (cmd, rest) = split_head(inner);
    if !trusted(cmd) {
        return None;
    }

    // `[llength LIST]` with a literal or lattice-resolvable list.
    if cmd == "llength" {
        let arg = rest?.trim();
        // Unlike `foreach`'s `list_arg` (already delimiter-stripped by the
        // segmenter), `arg` here is raw source text straight out of the
        // `[...]` command substitution, so it still carries its own
        // `{…}`/`"…"` wrapping that `extract_foreach_elements` no longer
        // strips — peel exactly one level before splitting.
        if let Some(elements) = extract_foreach_elements(strip_one_level(arg)) {
            let n = i64::try_from(elements.len()).unwrap_or(i64::MAX);
            return Some(LatticeValue::Const(ConstValue::Int(n)));
        }
        if let Some(items) = resolve_foreach_list_via_lattice(arg, uses, values, ssa) {
            let n = i64::try_from(items.len()).unwrap_or(i64::MAX);
            return Some(LatticeValue::Const(ConstValue::Int(n)));
        }
        return None;
    }

    // `[string length OPERAND]` where OPERAND resolves to a constant
    // string — a literal word, or a `$var` whose lattice value is known.
    // (Counting the chars of the *unresolved* operand text would mis-fold
    // `string length $s` to the length of "$s".)
    if cmd == "string" {
        if let Some(after_cmd) = rest {
            let (sub, sub_rest) = split_head(after_cmd.trim());
            if sub == "length"
                && let Some(raw) = sub_rest
                && let Some(s) = resolve_const_string(raw.trim(), uses, values, ssa)
            {
                // `string length` counts UTF-16 code units on Tcl 8 and
                // Unicode scalars on Tcl 9, so the fold uses the selected
                // dialect's model. With no selected release the count survives
                // only where both models agree, which is every string with no
                // supplementary character.
                let count = StringCharacterModel::count_for(policy.characters, &s)?;
                let len = i64::try_from(count).unwrap_or(i64::MAX);
                return Some(LatticeValue::Const(ConstValue::Int(len)));
            }
        }
        return None;
    }

    // `[expr {EXPR}]` — parse + fold under the current lattice.
    if cmd == "expr" {
        let arg = rest?.trim();
        // Braced (`expr {…}`) vs quoted / bare (`expr "…"`, `expr …`) changes
        // the substitution model. In a braced expr the `$var` references are
        // resolved by *expr* itself, so a string-valued var is a valid string
        // operand (`expr {$a == $b}` with a="alpha" → string compare → 0). In
        // a quoted / bare expr Tcl substitutes the variable *values* textually
        // *before* parsing, so a non-numeric value becomes an invalid bareword
        // and the whole expr errors at runtime (`expr "$a == $b"` →
        // `expr "alpha == beta"` → `invalid bareword "alpha"`). Folding that
        // to `0` would turn an erroring program into a silent value.
        //
        // Numeric values survive textual substitution as valid expr tokens,
        // so for the non-braced form restrict the env to numeric constants:
        // a string-valued var is then left unbound and the fold bails,
        // matching Tcl.
        let braced = arg.starts_with('{');
        let expr_text = strip_one_level(arg);
        let expr = crate::expr_parser::parse_expr(expr_text, None);
        let env = if braced {
            env_from_uses(uses, values, ssa)
        } else {
            env_from_uses_numeric(uses, values, ssa)
        };
        return eval_tcl_expr_with_policy(&expr, &env, policy)
            .map(|v| LatticeValue::Const(tcl_value_to_const(v)));
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

/// Parse a literal text as a [`ConstValue`]: prefers integer, then string fallback.
///
/// Only collapses to [`ConstValue::Int`] when the canonical integer text
/// round-trips (`str(int(s)) == s`).  A leading-zero literal such as `"08"` or
/// `"010"` parses as 8 / 10 but does *not* round-trip, so it is kept as a
/// string — preserving the identity SCCP needs to compare it correctly under
/// each dialect's leading-zero rule (octal in tcl8.x, decimal in tcl9.0).
/// Likewise `"+5"` / `"-0"` are kept as strings (they don't round-trip).
#[must_use]
pub fn parse_literal_value(text: &str) -> ConstValue {
    let stripped = text.trim();
    // Decimal integer grammar `[+-]?[0-9]+`.
    let digits = stripped.strip_prefix(['+', '-']).unwrap_or(stripped);
    let is_decimal_int = !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit());
    if is_decimal_int
        && let Ok(i) = stripped.parse::<i64>()
        && i.to_string() == stripped
    {
        return ConstValue::Int(i);
    }
    ConstValue::String(stripped.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cfg::{Block, BlockId, Function, Terminator};
    use crate::expr_ast::ExprNode;

    fn registry() -> CommandRegistry {
        CommandRegistry::build_default()
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
                registry: &registry(),
                traced_variables: &BTreeSet::new(),
                has_dynamic_variable_trace: false,
            },
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

    // -- predecessors + cfg_order --

    #[test]
    fn predecessors_simple_chain() {
        let mut f = Function::new("::top", "a");
        let a = f.entry;
        let b = block(&mut f, "b");
        f.blocks.get_mut(&a).unwrap().terminator = Some(goto(b));
        f.blocks.get_mut(&b).unwrap().terminator = Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
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
            span: None,
            expr: None,
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

    /// Issue #1409 — a static-body `uplevel 0` is an `UpFrame`, not a generic
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

    #[test]
    fn existence_fold_abstains_for_nested_upframe_alias_unset_mutation() {
        // `uplevel 0` evaluates in this procedure's frame. The nested body
        // aliases that frame's parameter and unsets it, so Tcl observes the else
        // branch. The no-uplevel twin proves the branch is otherwise foldable;
        // this is a mutation test of the exact fact #1409 must block.
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

    // -- driver --

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
            quoted_uses: std::collections::HashSet::new(),
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
            quoted_uses: std::collections::HashSet::new(),
        });
        let mut values: HashMap<ValueKey, LatticeValue> = HashMap::new();
        let escaping: HashSet<String> = HashSet::new();
        assert!(sccp_process_statements(
            &mut values,
            &block,
            &ssa,
            &escaping,
            FoldPolicy::default(),
            false,
            None
        ));
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

    #[test]
    fn sccp_marks_entry_executable_and_propagates_const() {
        // entry: set x 42
        let mut f = Function::new("::top", "entry");
        let entry = f.entry;
        f.blocks.get_mut(&entry).unwrap().terminator = Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
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
            span: None,
            expr: None,
            braced: false,
        });
        f.blocks.get_mut(&e).unwrap().terminator = Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
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
            span: None,
            expr: None,
            braced: false,
        });
        f.blocks.get_mut(&e).unwrap().terminator = Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
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
            span: None,
            expr: None,
            braced: false,
        });
        f.blocks.get_mut(&e).unwrap().terminator = Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
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
                expr_base: None,
            },
            uses,
            defs,
            may_defs: std::collections::HashSet::new(),
            quoted_uses: std::collections::HashSet::new(),
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
    /// Regression for the Codex #1046-5 / soundness-review finding: SCCP
    /// took a bare `octal: Option<bool>` and evaluated through the
    /// dialect-blind entry point, so `FoldOps::is_irules` was always
    /// `false` here and every word operator silently declined — while the
    /// `eq` control on the same shape folded, because plain Tcl shares it.
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
                expr_base: None,
            },
            uses,
            defs,
            may_defs: std::collections::HashSet::new(),
            quoted_uses: std::collections::HashSet::new(),
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

    // -- evaluate_def for Incr --

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
                safe_on_uninit: false,
            },
            uses,
            defs,
            may_defs: std::collections::HashSet::new(),
            quoted_uses: std::collections::HashSet::new(),
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
            evaluate_def(&stmt, &values, &ssa, FoldPolicy::default()),
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
            evaluate_def(&stmt, &values, &ssa, FoldPolicy::default()),
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
            evaluate_def(&stmt, &values, &ssa, FoldPolicy::default()),
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
            evaluate_def(&stmt, &values, &ssa, FoldPolicy::default()),
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
            evaluate_def(&stmt, &values, &ssa, FoldPolicy::default()),
            LatticeValue::Unknown
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
            evaluate_def(&stmt, &values, &ssa, FoldPolicy::default()),
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
            evaluate_def(&stmt, &values, &ssa, FoldPolicy::default()),
            LatticeValue::Overdefined
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

    // -- foreach constset extraction --

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
                foreach_groups: None,
            },
            uses: HashMap::new(),
            defs,
            may_defs: std::collections::HashSet::new(),
            quoted_uses: std::collections::HashSet::new(),
        }
    }

    #[test]
    fn extract_foreach_elements_literal_list() {
        // `list_text` is already delimiter-stripped by the segmenter, matching
        // the shape `foreach v {a b c}`'s `list_arg` is built in: the word's
        // own `{…}` is gone, leaving plain whitespace-separated text.
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
    fn extract_foreach_elements_splits_nested_braces_as_tcl_list() {
        // `a {b c} d` is a three-element Tcl list — `a`, `b c`, `d` — not the
        // four whitespace runs `a`, `{b`, `c}`, `d`. A naive `split_ascii_whitespace`
        // corrupted the CONSTSET; the list-aware split fixes it.
        assert_eq!(
            extract_foreach_elements("a {b c} d"),
            Some(vec!["a".into(), "b c".into(), "d".into()])
        );
        // Backslash-escaped whitespace groups an element too: `a\ b c` is two
        // elements `a b` and `c`.
        assert_eq!(
            extract_foreach_elements("a\\ b c"),
            Some(vec!["a b".into(), "c".into()])
        );
    }

    #[test]
    fn extract_foreach_elements_empty_list_returns_empty() {
        assert_eq!(extract_foreach_elements(""), Some(Vec::new()));
    }

    // Regression tests for issue #1433: `list_text` already has its outer
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
            extract_foreach_elements("{a b c}"),
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
            extract_foreach_elements("{a b} {c d}"),
            Some(vec!["a b".into(), "c d".into()])
        );
    }

    #[test]
    fn extract_foreach_elements_quoted_word_already_delimiter_stripped() {
        // Source `foreach v "a b c"`: the segmenter strips the word's own
        // `"…"` delimiters the same way it strips `{…}`, so `list_text` is
        // plain `a b c` — no quote handling is needed (or wanted) here.
        assert_eq!(
            extract_foreach_elements("a b c"),
            Some(vec!["a".into(), "b".into(), "c".into()])
        );
    }

    #[test]
    fn evaluate_def_foreach_literal_list_folds_constset() {
        // `list` mirrors `list_arg` as the segmenter hands it: source
        // `foreach v {1 2 3}`'s outer `{…}` is already stripped.
        let mut ssa = bare_ssa();
        let stmt = foreach_stmt(&mut ssa, "v", "1 2 3", 1);
        let result = evaluate_def(&stmt, &HashMap::new(), &ssa, FoldPolicy::default());
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
        // `foreach v [list a b c]` folds through `try_fold_cmd_subst` to the
        // same element CONSTSET as the braced-literal form (issue #777).
        let mut ssa = bare_ssa();
        let stmt = foreach_stmt(&mut ssa, "v", "[list a b c]", 1);
        let result = evaluate_def(&stmt, &HashMap::new(), &ssa, FoldPolicy::default());
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
            evaluate_def(&stmt, &HashMap::new(), &ssa, FoldPolicy::default()),
            LatticeValue::Const(ConstValue::String("only".into()))
        );
    }

    #[test]
    fn evaluate_def_foreach_nested_braced_elements_folds_constset() {
        // Regression test for issue #1433: source `foreach v {{a b} {c d}}`
        // hands `list_arg` = `{a b} {c d}` (only the word's own outer `{…}`
        // is stripped by the segmenter). Before the fix, `evaluate_def`
        // wrongly peeled a *second* level of bracing off this already
        // delimiter-stripped text, folding `v` to the corrupted CONSTSET
        // {"a", "b}"} instead of the two list elements.
        let mut ssa = bare_ssa();
        let stmt = foreach_stmt(&mut ssa, "v", "{a b} {c d}", 1);
        let result = evaluate_def(&stmt, &HashMap::new(), &ssa, FoldPolicy::default());
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
        let result = evaluate_def(&stmt, &values, &ssa, FoldPolicy::default());
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
        let result = evaluate_def(&stmt, &HashMap::new(), &ssa, FoldPolicy::default());
        assert_eq!(result, LatticeValue::Overdefined);
    }

    #[test]
    fn evaluate_def_foreach_multi_var_widens() {
        // 2-element defs → no constset extraction.
        let mut ssa = bare_ssa();
        let mut stmt = foreach_stmt(&mut ssa, "v", "a b", 1);
        let Statement::Call { defs, .. } = &mut stmt.statement else {
            panic!();
        };
        defs.push("w".into());
        let result = evaluate_def(&stmt, &HashMap::new(), &ssa, FoldPolicy::default());
        assert_eq!(result, LatticeValue::Overdefined);
    }

    // -- AssignValue + command-substitution folding --

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
            quoted_uses: std::collections::HashSet::new(),
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
        let result = evaluate_def(&stmt, &HashMap::new(), &ssa, FoldPolicy::default());
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
            evaluate_def(&stmt, &HashMap::new(), &ssa, FoldPolicy::default()),
            LatticeValue::Const(ConstValue::Int(4))
        );
    }

    #[test]
    fn evaluate_def_assign_value_folds_string_length() {
        let mut ssa = bare_ssa();
        let stmt = assign_value_stmt(&mut ssa, "n", "[string length \"hello\"]", 1);
        assert_eq!(
            evaluate_def(&stmt, &HashMap::new(), &ssa, FoldPolicy::default()),
            LatticeValue::Const(ConstValue::Int(5))
        );
    }

    // -- the fold arms answer to the command-binding trust fact (issue #1585) --

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
                registry,
                mutations,
                dialect: None,
                defining_class: None,
            }),
        )
    }

    /// The `[list …]` / `[format …]` arms run ahead of the registry engine and
    /// used to skip its trust gate entirely, so a unit that renamed `list`
    /// still got the builtin's answer (issue #1585).
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

    #[test]
    fn string_length_fold_counts_in_the_selected_dialects_character_model() {
        // U+1D11E is one Tcl 9 scalar but two Tcl 8 `Tcl_UniChar` units, so the
        // compile-time fold must answer as the selected runtime would — and
        // decline when no release is selected, leaving the width ambiguous.
        let mut ssa = bare_ssa();
        let stmt = assign_value_stmt(&mut ssa, "n", "[string length \"\u{1D11E}\"]", 1);
        let fold = |dialect: Option<&tcl_dialect::DialectProfile>| {
            evaluate_def(
                &stmt,
                &HashMap::new(),
                &ssa,
                FoldPolicy::for_profile(Some(false), dialect),
            )
        };
        assert_eq!(
            fold(Some(tcl_dialect::DialectProfile::by_name("tcl9.0"))),
            LatticeValue::Const(ConstValue::Int(1))
        );
        assert_eq!(
            fold(Some(tcl_dialect::DialectProfile::by_name("tcl8.6"))),
            LatticeValue::Const(ConstValue::Int(2))
        );
        assert_eq!(
            fold(None),
            LatticeValue::Overdefined,
            "no selected release leaves a supplementary width ambiguous"
        );

        // A string both models count identically still folds with no selected
        // release — declining those would drop an ordinary optimisation.
        let mut ssa = bare_ssa();
        let ascii = assign_value_stmt(&mut ssa, "n", "[string length \"hello\"]", 1);
        assert_eq!(
            evaluate_def(
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
            evaluate_def(&stmt, &HashMap::new(), &ssa, FoldPolicy::default()),
            LatticeValue::Const(ConstValue::Int(3))
        );
    }

    #[test]
    fn evaluate_def_assign_value_folds_format_literal() {
        let mut ssa = bare_ssa();
        let stmt = assign_value_stmt(&mut ssa, "s", "[format \"%d-%d\" 1 2]", 1);
        match evaluate_def(&stmt, &HashMap::new(), &ssa, FoldPolicy::default()) {
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
            evaluate_def(&stmt, &values, &ssa, FoldPolicy::default()),
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
            evaluate_def(&stmt, &values, &ssa, FoldPolicy::default()),
            LatticeValue::Const(ConstValue::Int(0))
        );
    }

    #[test]
    fn evaluate_def_assign_value_unknown_cmd_widens() {
        let mut ssa = bare_ssa();
        let stmt = assign_value_stmt(&mut ssa, "x", "[nonexistent_fold args]", 1);
        assert_eq!(
            evaluate_def(&stmt, &HashMap::new(), &ssa, FoldPolicy::default()),
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
            evaluate_def(&stmt, &values, &ssa, FoldPolicy::default()),
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
            span: None,
            expr: None,
            braced: false,
        });
        f.blocks.get_mut(&dead).unwrap().terminator = Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
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
        // End-to-end regression test for issue #1433. `foreach v {{a b} {c d}}`
        // lowers `list_arg` to the segmenter-stripped text `{a b} {c d}`
        // (only the word's own outer braces are gone). Before the fix, SCCP's
        // foreach constset extraction peeled a *second* level of bracing off
        // this already-stripped text, corrupting `v`'s CONSTSET to the two
        // elements `a` and `b}` (from the lenient split of `a b} {c d`) and
        // wrongly proving `if {$v eq "c d"}` false. It must fold to the two
        // correct elements `a b` and `c d` instead.
        let c = cu(
            "proc ::p {} { foreach v {{a b} {c d}} { if {$v eq \"c d\"} { set r yes } else { set r no } } }",
        );
        let fu = c.function("::p").unwrap();
        let r = sccp_no_traces(&fu.cfg, &fu.ssa, None, FoldPolicy::default());
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
        assert!(branch_deferrable(&sb, &cond, &values, &ssa));
        values.insert((x, 1), LatticeValue::Unknown);
        assert!(branch_deferrable(&sb, &cond, &values, &ssa));

        // An `Overdefined` operand proves the condition genuinely
        // non-constant → never defer.
        values.insert((x, 1), LatticeValue::Overdefined);
        assert!(!branch_deferrable(&sb, &cond, &values, &ssa));

        // A constant operand folds via `evaluate_branch`, so the `None`
        // arm is never reached → not deferrable here.
        values.insert((x, 1), LatticeValue::Const(ConstValue::Int(1)));
        assert!(!branch_deferrable(&sb, &cond, &values, &ssa));

        // Version-0 operands (parameters / globals / live-in roots) are
        // already `Overdefined` and excluded from the deferral test.
        let mut sb0 = empty_ssa_block("b");
        sb0.exit_versions.insert(x, 0);
        assert!(!branch_deferrable(&sb0, &cond, &HashMap::new(), &ssa));
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
}
