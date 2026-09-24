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
use tcl_registry::value_transfer::ExactValue;

use crate::analyses::{ConstValue, LatticeValue, MAX_CONSTSET_SIZE};
use crate::cfg::{BlockId, Function as CfgFunction, Terminator};
use crate::codegen::helpers::split_list_values;
use crate::expr_ast::ExprNode;
use crate::ir::Statement;
use crate::ssa::{SsaFunction, SsaStatement, Symbol, ValueKey};
use crate::tcl_expr_eval::FoldPolicy;
use crate::value_transfer::{AnalysisContextKey, LatticeDriver};

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
    /// The condition is proven, and reachability was not updated from it:
    /// the existence post-pass's `[info exists X]` / `[array exists X]`
    /// folds, which `executable_blocks` does not reflect.
    Proven,
    /// An arm is selected that the CFG has no edge of its own for.
    Selected,
    /// The solver decided the branch and applied it to the executable
    /// blocks and edges.
    Applied,
}

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
    /// Which branch fact this is.
    pub kind: BranchFactKind,
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
}

impl SccpResult {
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
    /// The analysis context's memo identity for this run — the registry
    /// and overlay generations, the module's command-binding evidence, the
    /// tier, and the evaluator revision — when the caller carries one; a
    /// caller with no module view passes `None` and runs detached.
    pub analysis_context: Option<&'a AnalysisContextKey>,
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
    let preds = compute_predecessors(cfg);
    let mut values = seeded_values(ssa, param_constants);

    let grammar = trace
        .registry
        .profile()
        .map_or_else(tcl_dialect::LexerGrammar::default, |p| p.grammar);
    seed_live_in_roots(cfg, ssa, &mut values, grammar);

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
    // Every command-specific answer below comes from the registry's
    // declaration for the resolved invocation, through one driver whose
    // context is this run's identity.
    let driver = LatticeDriver::new(trace, folds, policy, &escaping);

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
            // Every sweep re-evaluates each executable statement from
            // scratch, so only the settled sweep's route entries — the
            // last one run — should reach the tally. Each sweep is one
            // iteration of the run's request: a tenth of what it has left.
            driver.reset_tally_for_sweep();
            driver.open_iteration();
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
                    record_phi_folded_types(&values, ssa_block, &incoming_exec, &driver);
                }

                // Statements.
                changed |= sccp_process_statements(
                    &mut values,
                    ssa_block,
                    ssa,
                    &escaping,
                    trace.has_dynamic_variable_trace,
                    &driver,
                );

                // Terminator.
                let inputs = TerminatorInputs {
                    cfg,
                    ssa,
                    values: &values,
                    policy,
                    grammar,
                    registry: trace.registry,
                    driver: &driver,
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

    let constant_branches = collect_constant_branches(
        cfg,
        ssa,
        &values,
        &executable_blocks,
        &order,
        BranchFold {
            policy,
            grammar,
            registry: trace.registry,
            driver: &driver,
        },
    );

    SccpResult {
        values,
        executable_blocks,
        executable_edges,
        constant_branches,
        ..driver.take_run_facts()
    }
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

/// Evaluate each statement's defs for one block, widening across barriers.
/// Returns `true` if any lattice value changed. Extracted from [`sccp`].
fn sccp_process_statements(
    values: &mut HashMap<ValueKey, LatticeValue>,
    ssa_block: &crate::ssa::SsaBlock,
    ssa: &SsaFunction,
    escaping: &HashSet<String>,
    has_dynamic_variable_trace: bool,
    driver: &LatticeDriver<'_>,
) -> bool {
    let mut changed = false;
    for (index, stmt_ssa) in ssa_block.statements.iter().enumerate() {
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
        // The statement is evaluated once, when a definition first needs
        // it: a call's ordered stores give each definition its own value.
        let mut evaluated: Option<DefValues> = None;
        for (&var, &ver) in &stmt_ssa.defs {
            let mut value_of = |values: &HashMap<ValueKey, LatticeValue>| {
                let evaluated = evaluated
                    .get_or_insert_with(|| evaluate_defs_under(stmt_ssa, values, ssa, driver));
                (
                    evaluated.of((var, ver)),
                    evaluated.folded_of((var, ver)),
                    evaluated.stated((var, ver)),
                    evaluated.preserved((var, ver)),
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
                            let (written, folded, stated, _) = value_of(values);
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
                    let (value, folded, _, kept) = value_of(values);
                    if kept {
                        preserved = Some(prior_version(ssa_block, index, var));
                    }
                    (value, folded)
                };
            driver.record_folded((var, ver), folded);
            driver.record_preserved((var, ver), preserved);
            if set_value(values, (var, ver), &val) {
                changed = true;
            }
        }
    }
    changed
}

/// The version of `var` the statement at `index` of `block` finds in its
/// place: the block's latest earlier definition, else the version the block
/// enters with, else the undefined root.
fn prior_version(block: &crate::ssa::SsaBlock, index: usize, var: Symbol) -> crate::ssa::Version {
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
    policy: FoldPolicy,
    /// The document's lexer grammar — the branch condition's `Raw` operand
    /// texts are re-read under it when their variables are collected.
    grammar: tcl_dialect::LexerGrammar,
    /// The registry the bounded-loop simulator resolves against.
    registry: &'a CommandRegistry,
    /// The run's value-transfer driver: a condition's nested commands and
    /// finite inputs are evaluated through it.
    driver: &'a LatticeDriver<'a>,
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
        grammar,
        registry,
        driver,
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
            span,
            ..
        } => {
            let Some(ssa_block) = ssa.blocks.get(&bn) else {
                return changed;
            };
            driver.explaining(*span);
            let decision = branch_decision(
                cfg,
                ssa,
                bn,
                ssa_block,
                condition,
                values,
                BranchFold {
                    policy,
                    grammar,
                    registry,
                    driver,
                },
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
/// condition evaluated to a constant lattice value, and the definitions a
/// condition the shared engine decided left untouched
/// ([`record_condition_preserves`]; a `for` loop's static summary is not
/// the engine's). Extracted from [`sccp`].
fn collect_constant_branches(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    values: &HashMap<ValueKey, LatticeValue>,
    executable_blocks: &HashSet<BlockId>,
    order: &[BlockId],
    fold: BranchFold<'_>,
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
        fold.driver.explaining(*term_span);
        let decision = branch_decision(cfg, ssa, *bn, ssa_block, condition, values, fold);
        fold.driver.explaining(None);
        if decision.is_some() && !cfg.loop_nodes.contains_key(bn) {
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

/// Every variable name the function assigns, and every name a call unbinds
/// by literal — the two whole-body facts [`existence_constant_branches`]
/// folds against. A `Call`'s `defs` cover the commands that define a name
/// without an assignment statement (`global` / `variable` / `upvar`,
/// `regexp -inline` match vars, …); the unbind fact is each call's resolved
/// existence transfer ([`crate::value_transfer::unbound_names`]), so no
/// command is recognised by its spelling here.
fn scan_defined_and_unbound(
    cfg: &CfgFunction,
    registry: &tcl_registry::CommandRegistry,
) -> (FxHashSet<String>, FxHashSet<String>) {
    let mut defined: FxHashSet<String> = FxHashSet::default();
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
                Statement::Call { defs, .. } => {
                    for d in defs {
                        defined.insert(d.clone());
                    }
                }
                _ => {}
            }
        }
    }
    (defined, crate::value_transfer::unbound_names(cfg, registry))
}

/// The entry facts one function frame contributes to the existence fold
/// ([`existence_constant_branches`]), sourced from the typed IR for whichever
/// kind of body it is — a [`crate::ir::Procedure`], a
/// [`crate::ir::MethodDef`], or neither (the top level, a lambda).
///
/// Bundled rather than passed positionally so a new fact reaches both
/// consumers of the fold — the analyser's I230 and the optimiser's O101 — by
/// construction: the two build the same struct from the same IR, so they
/// cannot drift on, say, method parameters.
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
///
/// Deliberately **not**
/// [`split_element_ref`](tcl_syntax::naming::split_element_ref):
/// this is a narrower *fold-safety* predicate, and its extra tests — non-empty
/// base, bareword base — are the point. The owner admits the zero-length array
/// name `(k)` that `TclObjLookupVarEx` admits, which is not a name this fold
/// may reason about.
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
/// (`info exists` → `true`, `array exists` → `false`); a never-defined
/// non-parameter never exists (`false`); an element guard `X(elem)` on an
/// array this body never touches never exists (`false` — the guard is decided
/// on the *array* name, with the same abstentions as a simple name, so the
/// element key may even be dynamic).
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
/// body's assignment scan cannot see.
///
/// `dynamic_names` carries the function's
/// [dynamic-name barrier](crate::dynamic_names) and gates each direction
/// independently:
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
/// [`crate::ir::MethodDef::instance_vars`].
/// A class-level `variable x` declaration binds `x` in **every**
/// method's frame with no `variable` statement in the body itself, so
/// [`crate::optimiser::elimination::scan_scope_aliases`] (which only sees the
/// body's own commands) cannot find it — the name looks like a never-defined
/// local, which the fold would otherwise call "always absent".  It is not:
/// existence is per-instance runtime state, set by whichever method or
/// constructor assigned it first.  tclsh 9.0.4 and 8.6.14 agree:
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
    config: tcl_lexer::LexerConfig,
) -> Vec<ConstantBranch> {
    let mut out = Vec::new();
    if cfg.blocks.values().any(|b| {
        b.statements
            .iter()
            .any(|s| matches!(s, Statement::Barrier { .. } | Statement::UpFrame { .. }))
    }) {
        return out;
    }
    let (defined, unset) = scan_defined_and_unbound(cfg, registry);
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
    // frame at entry, so the body's own command scan cannot see it.
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
    // is out-of-frame runtime state exactly like object state above.
    // Some are bound before user code (`argv`, `env`, `tcl_platform`,
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
            tcl_registry::special_vars::special_vars_for_dialect(Some(
                tcl_registry::special_vars::surface_query_for_profile(registry.profile()),
            ))
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
        let Some(crate::existence_query::ExistenceQuery { var, negated, kind }) =
            crate::existence_query::in_expr(condition, registry, config)
        else {
            continue;
        };
        let exists = if let Some(base) = array_element_base(&var) {
            // An array-element guard on a never-touched array is provably
            // false: no element of `a` can exist when nothing
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
                // Which constant depends on the spelling.  A
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
                matches!(kind, crate::existence_query::ExistenceKind::AnyVariable)
            } else if !defined.contains(&var) {
                // `set $switch {}` may have defined exactly this name — the
                // argparse idiom.
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
            kind: BranchFactKind::Proven,
        });
    }
    out
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
#[must_use]
pub fn evaluate_def<S: std::hash::BuildHasher>(
    stmt_ssa: &SsaStatement,
    values: &HashMap<ValueKey, LatticeValue, S>,
    ssa: &SsaFunction,
    policy: FoldPolicy,
) -> LatticeValue {
    evaluate_def_with_folds(stmt_ssa, values, ssa, policy, None)
}

/// [`evaluate_def`] with an optional registry builtin-fold context: when
/// `folds` is supplied, an `AssignValue` command-substitution RHS
/// additionally consults the registry `const_fold` engine — see
/// [`BuiltinFoldInputs`] — and every resolved head answers to the
/// whole-module trust fact. `None` is byte-identical to [`evaluate_def`].
///
/// A statement that defines several variables (`regexp … a b`) answers
/// for the first one its command names; the solver itself reads every
/// definition's own value.
#[must_use]
pub fn evaluate_def_with_folds<S: std::hash::BuildHasher>(
    stmt_ssa: &SsaStatement,
    values: &HashMap<ValueKey, LatticeValue, S>,
    ssa: &SsaFunction,
    policy: FoldPolicy,
    folds: Option<BuiltinFoldInputs<'_>>,
) -> LatticeValue {
    let driver = LatticeDriver::detached(folds, policy);
    evaluate_defs_under(stmt_ssa, values, ssa, &driver).primary(stmt_ssa, ssa)
}

/// What one statement's evaluation leaves in its definitions: the one
/// value a typed statement computes, which every definition takes, or a
/// call's value per definition, from its ordered stores.
pub(crate) enum DefValues {
    /// Every definition takes this value, and the folded type the
    /// evaluation that produced it states.
    Each(LatticeValue, Option<crate::value_transfer::FoldedType>),
    /// Each definition's own answer; a definition absent here widens.
    PerDef(Vec<crate::value_transfer::DefAnswer>),
}

impl DefValues {
    /// The value definition `key` takes.
    fn of(&self, key: ValueKey) -> LatticeValue {
        match self {
            Self::Each(value, _) => value.clone(),
            Self::PerDef(answers) => answers
                .iter()
                .find(|answer| answer.key == key)
                .map_or(LatticeValue::Overdefined, |answer| answer.value.clone()),
        }
    }

    /// The folded type definition `key` takes, when its evaluation states
    /// one.
    fn folded_of(&self, key: ValueKey) -> Option<crate::value_transfer::FoldedType> {
        match self {
            Self::Each(_, folded) => folded.clone(),
            Self::PerDef(answers) => answers
                .iter()
                .find(|answer| answer.key == key)
                .and_then(|answer| answer.folded.clone()),
        }
    }

    /// Whether an evaluated outcome's stores name definition `key`'s place
    /// ([`crate::value_transfer::DefAnswer::stated`]).
    fn stated(&self, key: ValueKey) -> bool {
        match self {
            Self::Each(..) => false,
            Self::PerDef(answers) => answers
                .iter()
                .any(|answer| answer.key == key && answer.stated),
        }
    }

    /// Whether the outcome left definition `key`'s place untouched
    /// ([`crate::value_transfer::DefAnswer::preserved`]).
    fn preserved(&self, key: ValueKey) -> bool {
        match self {
            Self::Each(..) => false,
            Self::PerDef(answers) => answers
                .iter()
                .any(|answer| answer.key == key && answer.preserved),
        }
    }

    /// The value of the statement's first named definition: the only one,
    /// or the first variable a call's command names.
    fn primary(&self, stmt_ssa: &SsaStatement, ssa: &SsaFunction) -> LatticeValue {
        let key = if stmt_ssa.defs.len() == 1 {
            stmt_ssa.defs.iter().next().map(|(&sym, &ver)| (sym, ver))
        } else if let Statement::Call { defs, .. } = &stmt_ssa.statement {
            defs.first()
                .and_then(|name| ssa.var_symbol(crate::naming::normalise_var_name(name)))
                .and_then(|sym| stmt_ssa.defs.get(&sym).map(|&ver| (sym, ver)))
        } else {
            None
        };
        match (self, key) {
            (Self::Each(value, _), _) => value.clone(),
            (Self::PerDef(_), Some(key)) => self.of(key),
            (Self::PerDef(_), None) => LatticeValue::Overdefined,
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
            ..
        } => {
            driver.evaluate_assign_expr(expr, command_binding.as_ref(), &stmt_ssa.uses, values, ssa)
        }
        Statement::Call { .. } => {
            return DefValues::PerDef(driver.evaluate_call(stmt_ssa, values, ssa, &stmt_ssa.uses));
        }
        Statement::Incr {
            name,
            amount,
            amount_braced,
            ..
        } => {
            return DefValues::PerDef(driver.evaluate_incr(
                name,
                amount.as_deref().map(|text| (text, *amount_braced)),
                &crate::value_transfer::named_defs(stmt_ssa, ssa),
                &stmt_ssa.uses,
                values,
                ssa,
            ));
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
/// The two dialect facts a branch fold needs: what the target release folds
/// (`policy`) and the grammar its condition text was written under
/// (`grammar`). Bundled so the fold entry points stay inside clippy's
/// argument budget and neither fact can be threaded without the other.
#[derive(Clone, Copy)]
pub(crate) struct BranchFold<'a> {
    policy: FoldPolicy,
    grammar: tcl_dialect::LexerGrammar,
    /// The registry the bounded-loop simulator resolves its cell updates
    /// against.
    registry: &'a CommandRegistry,
    /// The run's value-transfer driver, whose services the condition is
    /// evaluated through.
    driver: &'a LatticeDriver<'a>,
}

fn branch_decision(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    bn: BlockId,
    ssa_block: &crate::ssa::SsaBlock,
    condition: &ExprNode,
    values: &HashMap<ValueKey, LatticeValue>,
    fold: BranchFold<'_>,
) -> Option<bool> {
    loop_summary_decision(cfg, ssa, bn, condition, values, fold.policy, fold.registry)
        .or_else(|| evaluate_branch(ssa_block, condition, values, ssa, fold))
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
    registry: &CommandRegistry,
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
        crate::static_loops::LoopSemantics { policy, registry },
    )?;
    let v = crate::static_loops::evaluate_expr_with_constants(condition, &summarised, policy)?;
    Some(v != 0)
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
/// — a parameter's caller-provided seed — reads its version 0.
#[must_use]
pub(crate) fn evaluate_branch<S: std::hash::BuildHasher>(
    ssa_block: &crate::ssa::SsaBlock,
    condition: &ExprNode,
    values: &HashMap<ValueKey, LatticeValue, S>,
    ssa: &SsaFunction,
    fold: BranchFold<'_>,
) -> Option<bool> {
    let mut uses: HashMap<Symbol, crate::ssa::Version> = ssa_block
        .exit_versions
        .iter()
        .map(|(&sym, &ver)| (sym, ver))
        .collect();
    let config = tcl_lexer::LexerConfig::from_grammar(fold.grammar);
    for name in condition.vars_element_qualified_with_config(config) {
        if let Some(sym) = ssa.var_symbol(&name) {
            uses.entry(sym).or_insert(0);
        }
    }
    fold.driver
        .evaluate_condition(condition, &uses, values, ssa)
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
                registry: &registry(),
                mutations: &crate::command_binding::ModuleCommandMutations::default(),
                dialect: None,
                defining_class: None,
                registry_engine: false,
                trust: FoldTrust::ObservedBindings,
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
                registry: &registry(),
                traced_variables: &BTreeSet::new(),
                has_dynamic_variable_trace: false,
                analysis_context: None,
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
                registry: &registry,
                traced_variables: &BTreeSet::new(),
                has_dynamic_variable_trace: false,
                analysis_context: None,
            },
            Some(BuiltinFoldInputs {
                registry: &registry,
                mutations: &crate::command_binding::ModuleCommandMutations::default(),
                dialect: None,
                defining_class: None,
                registry_engine: false,
                trust: FoldTrust::ObservedBindings,
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
            quoted_uses: std::collections::HashSet::new(),
            name_only_uses: std::collections::HashSet::new(),
        });
        let mut values: HashMap<ValueKey, LatticeValue> = HashMap::new();
        let escaping: HashSet<String> = HashSet::new();
        assert!(sccp_process_statements(
            &mut values,
            &block,
            &ssa,
            &escaping,
            false,
            &LatticeDriver::detached(None, FoldPolicy::default()),
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
            value_word: None,
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
            value_word: None,
            span: None,
            expr: None,
            braced: false,
        });
        f.blocks.get_mut(&e).unwrap().terminator = Some(Terminator::Return {
            value: None,
            value_word: None,
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
            value_word: None,
            span: None,
            expr: None,
            braced: false,
        });
        f.blocks.get_mut(&e).unwrap().terminator = Some(Terminator::Return {
            value: None,
            value_word: None,
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
            value_word: None,
            span: None,
            expr: None,
            braced: false,
        });
        f.blocks.get_mut(&e).unwrap().terminator = Some(Terminator::Return {
            value: None,
            value_word: None,
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
                command_binding: Some(tcl_runtime_api::CommandBindingIdentity::new("expr", "expr")),
                expr_base: None,
                fallback_value: "[expr {$a + 3}]".into(),
            },
            uses,
            defs,
            may_defs: std::collections::HashSet::new(),
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
            registry: tcl_registry::model::ingress::static_context_for(dialect).commands(),
            mutations: MUTATIONS.get_or_init(Default::default),
            dialect: Some(environment.analyser_profile()),
            defining_class: None,
            registry_engine: false,
            trust: FoldTrust::ObservedBindings,
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
        assert!(
            explained
                .iter()
                .any(|(command, answer)| command == "append" && answer != "evaluated"),
            "an append over an unbound cell is pending or declined: {explained:?}"
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
        // Two binders over one list (VT5.7): each takes the elements it is
        // assigned, so over `a b` the first binder, `v`, holds `a` (it had
        // widened while the plan answered one binder only).
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
                registry,
                mutations,
                dialect: None,
                defining_class: None,
                registry_engine: false,
                trust: FoldTrust::WholeModule,
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
                registry,
                mutations,
                dialect: None,
                defining_class: None,
                registry_engine: false,
                trust: FoldTrust::ObservedBindings,
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
            braced: false,
        });
        f.blocks.get_mut(&dead).unwrap().terminator = Some(Terminator::Return {
            value: None,
            value_word: None,
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
}
